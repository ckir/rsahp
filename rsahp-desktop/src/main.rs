//! Single-binary desktop wrapper: embeds the axum backend (background tokio thread) and
//! the egui frontend (main thread), preserving the localhost HTTP boundary.
//!
//! Order (matters): resolve paths → create data_dir → acquire single-instance lock →
//! seed config (lock-winner only) → logging → start backend on an EPHEMERAL port in a bg
//! thread → block for its bound address (or failure) → run GUI on main → on close, fire
//! graceful shutdown and exit with a watchdog so a hung drain can never zombie the app.

use std::io::ErrorKind;
use std::net::SocketAddr;
use std::time::Duration;

use common::datadir;
use fd_lock::RwLock as FdRwLock;
use frontend::config::AppConfig;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

fn fatal(title: &str, msg: &str) -> ! {
    rfd::MessageDialog::new()
        .set_level(rfd::MessageLevel::Error)
        .set_title(title)
        .set_description(msg)
        .show();
    eprintln!("{title}: {msg}");
    std::process::exit(1);
}

fn main() {
    // 1. Resolve per-user local data dir + derived paths.
    let paths = datadir::resolve()
        .unwrap_or_else(|| fatal("rsahp", "Could not determine a user data directory."));

    // 2. Create data_dir FIRST (the lock file lives in it) — but do NOT seed config yet.
    if let Err(e) = std::fs::create_dir_all(&paths.data_dir) {
        fatal("rsahp", &format!("Failed to create data directory: {e}"));
    }

    // 3. Single-instance guard: hold an exclusive advisory lock for the whole process.
    //    Acquire it BEFORE seeding config so two concurrent first-launches cannot race to
    //    write config.json (a Windows sharing-violation crash). fd-lock: Err(WouldBlock)
    //    ⇒ already held ⇒ another instance is running.
    let lock_path = paths.data_dir.join("rsahp.lock");
    let lock_file = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(&lock_path)
        .unwrap_or_else(|e| fatal("rsahp", &format!("Failed to open lock file: {e}")));
    let mut instance_lock = FdRwLock::new(lock_file);
    let _lock_guard = match instance_lock.try_write() {
        Ok(guard) => guard,
        Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
            rfd::MessageDialog::new()
                .set_level(rfd::MessageLevel::Info)
                .set_title("rsahp")
                .set_description("rsahp is already running.")
                .show();
            std::process::exit(0);
        }
        Err(e) => fatal(
            "rsahp",
            &format!("Failed to acquire single-instance lock: {e}"),
        ),
    };

    // 4. Now (lock-winner only) create logs + seed config.json.
    if let Err(e) =
        datadir::ensure_dirs_and_seed(&paths.data_dir, &paths.logs_dir, &paths.config_path)
    {
        fatal(
            "rsahp",
            &format!("Failed to initialize data directory: {e}"),
        );
    }

    // 5. Logging → <data_dir>/logs.
    let file_appender =
        RollingFileAppender::new(Rotation::DAILY, &paths.logs_dir, "rsahp_desktop.log");
    // `log_guard` flushes buffered logs when dropped. `std::process::exit` SKIPS Drop, so
    // we drop it explicitly before every error-path exit (below) or the "see logs" prompt
    // points at an empty file.
    let (non_blocking, log_guard) = tracing_appender::non_blocking(file_appender);
    tracing_subscriber::registry()
        .with(EnvFilter::new("info"))
        .with(fmt::layer().json().with_writer(std::io::stdout))
        .with(fmt::layer().json().with_writer(non_blocking))
        .init();

    // 6. Frontend config (use_gpu, zoom_scale) from the data dir; db_url from resolved path.
    let config = AppConfig::load_from(&paths.config_path);
    let db_url = paths.database_url();

    // 7. Channels + backend on a bg thread bound to an EPHEMERAL port. `done_tx` is moved
    //    into the thread and dropped when it finishes; the main thread waits on `done_rx`
    //    with a timeout so it (the owner of `log_guard`) always flushes logs before exit.
    let (ready_tx, ready_rx) = tokio::sync::oneshot::channel::<SocketAddr>();
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
    let (done_tx, done_rx) = std::sync::mpsc::channel::<()>();

    let _server_thread = std::thread::spawn(move || {
        // Dropped when this thread (and its run_server) finishes → disconnects `done_rx`.
        let _done_tx = done_tx;
        let rt = match tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
        {
            Ok(rt) => rt,
            Err(e) => {
                tracing::error!("failed to build tokio runtime: {e}");
                return; // ready_tx drops → main surfaces the failure
            }
        };
        let bind_addr: SocketAddr = "127.0.0.1:0".parse().expect("valid loopback addr");
        if let Err(e) = rt.block_on(backend::run_server(
            db_url,
            bind_addr,
            ready_tx,
            shutdown_rx,
        )) {
            tracing::error!("backend server failed: {e}");
        }
    });

    // 8. Block for the real bound address (deterministic). Sender dropped ⇒ startup failed.
    let addr = match ready_rx.blocking_recv() {
        Ok(addr) => addr,
        Err(_) => {
            drop(log_guard); // flush the backend's error log before we exit
            fatal(
                "rsahp",
                "The backend service failed to start (see logs). The application cannot continue.",
            );
        }
    };

    // 9. GUI on the main thread. The `/api/documents` suffix is load-bearing (the admin
    //    panel derives its base via `.replace("/documents","")`).
    let api_base = format!("http://{addr}/api/documents");
    let gui_result = frontend::run_gui(api_base, config);

    // 10. Window closed → graceful shutdown → bounded, log-flushing exit. We wait up to 3s
    //     for the backend thread to finish its graceful drain (signaled by `done_tx` being
    //     dropped when that thread exits). Whether it drains in time (Disconnected) or hangs
    //     past the deadline (Timeout), the MAIN thread — which owns `log_guard` — is always
    //     the one that runs `drop(log_guard)` before exiting, so buffered logs are never lost
    //     (no detached watchdog thread that could exit while the guard is still held).
    let _ = shutdown_tx.send(());
    match gui_result {
        Ok(()) => {
            let _ = done_rx.recv_timeout(Duration::from_secs(3));
            drop(log_guard); // flush buffered logs before exiting (process::exit skips Drop)
            std::process::exit(0);
        }
        Err(e) => {
            // fatal() exits the process (which reaps the still-running server thread); flush
            // logs first so the "see logs" prompt isn't pointing at an empty file.
            drop(log_guard);
            fatal("rsahp", &format!("GUI error: {e}"));
        }
    }
}

// A1 (hardening, deferred/optional): if the egui update loop PANICS, the main thread
// unwinds without running step 10, so the graceful-shutdown signal never fires. This is
// NOT a resource leak — the process exits and the OS reaps the background thread and
// releases the advisory lock (SQLite is durable across abrupt termination). If clean
// shutdown-on-panic is later desired, wrap step 9 in std::panic::catch_unwind or move
// `shutdown_tx` into a Drop guard. Left out here to keep the flow simple.
