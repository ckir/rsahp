# rsahp Desktop Packaging Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship rsahp as a double-clickable installed desktop app on Windows (Inno Setup installer) and Linux (Flatpak), via a single-binary `rsahp-desktop` wrapper that embeds the axum backend in a background thread and the egui frontend on the main thread, preserving the localhost HTTP boundary.

**Architecture:** Four sequential phases. **P1** adds a tested data-dir resolution module to `common` (no behavior change to existing bins). **P2** factors `run_server`/`run_gui` out of the two `main.rs` files into their lib crates, wires `use_gpu` live (default OFF), and adds the `rsahp-desktop` wrapper crate (ephemeral port + ready-signal channel + graceful shutdown + single-instance lock). **P3** adds the Windows Inno installer + local build task. **P4** adds the Linux Flatpak. CI release wiring (gated on tests) lands in P3/P4.

**Tech Stack:** Rust 2024 workspace; axum 0.8 + sea-orm 1.1 (SQLite); eframe/egui 0.34; `directories` (data dir), `percent-encoding` (URI-safe DB path), `fd-lock` (single-instance lock), `rfd` (error dialogs); Inno Setup (`iscc`); flatpak-builder + `flatpak-cargo-generator.py`; GitHub Actions.

---

## Review status

**Round 1** (solo panel + agy escalation) → **RED** → all 17 findings folded (10 solo F1–F10 + 7 agy A1–A7; A1 downgraded to optional hardening): percent-encoded SQLite URL + real sea-orm integration test (F2/A4); `fd-lock` not `fs4` (F3); lock-before-seed (A3); bounded-exit watchdog (A2); `config.save()` path fix (F1); Flatpak `skip:` list (A5); `cargo pkgid` version (A6); test-gated release (A7).

**Round 2** (agy escalation, fresh seats: Resource Vampire / Blindspot Auditor / Mechanism Gamer) → **RED** → 2 real findings folded, 2 rejected by measurement:
- **R2-3 (FOLDED)** — the exit watchdog was spawned unconditionally, so on a GUI error its 3s `exit(0)` would race and kill the modal error dialog with a success code. Now the watchdog guards ONLY the clean path (Task 11 step 10).
- **R2-4 (FOLDED)** — `std::process::exit` skips the `WorkerGuard` drop, losing buffered logs behind a "see logs" prompt. Now `drop(log_guard)` flushes before every error-path exit (Task 11 steps 5/8/10).
- **R2-1 (REJECTED)** — claimed `if let … && let …` is unstable; refuted by measurement — `frontend/src/config.rs:63-65` and `backend/src/config.rs:48-50` already use let-chains and the project compiles/CI is green (stabilized Rust 1.88, edition 2024). Instance of agy's known version-stale-syntax tendency.
- **R2-2 (REJECTED)** — claimed the wrapper reads `database_url`/`port` off the frontend `AppConfig`; refuted by the plan text — Task 11 builds `db_url` from `AppPaths::database_url()` and binds an ephemeral port; it never reads those fields.

---

## Reconciliation notes (spec vs. actual code — READ FIRST)

The design spec (`docs/superpowers/specs/2026-07-14-desktop-packaging-design.md`) was written before a full code read. Recon (2026-07-14) found three facts that change the plan; these are **binding decisions**, not open questions:

- **R1 — No shared config struct.** `common` holds only DTOs. `backend/src/config.rs::AppConfig` reads only `{database_url, port}`; `frontend/src/config.rs::AppConfig` reads only `{api_url, zoom_scale}`. We KEEP the two separate structs (each deserializes its own subset of the shared `config.json`). We do NOT unify them.
- **R2 — `use_gpu` is dead code, and its default flips.** It is read by neither struct today; the frontend chose GPU via `--disable-gpu` (opt-out, default ON). **User decision (2026-07-14):** GPU default becomes **OFF**; the flag becomes **`--enable-gpu`** (opt-in); config `use_gpu: true` also enables it (OR semantics). This is the "wire it live" option (agy + controller recommended, user-approved).
- **R3 — Standalone bins stay cwd-relative.** The spec body of Phase 1 ("Both the backend AND the frontend resolve config.json from the data dir") is superseded by fold-ledger #12 and core-interface line 34 ("standalone bins default to cwd; the wrapper owns the data-dir default"). This is what preserves `start.ps1` / `verify.ps1` / dev behavior. **Only `rsahp-desktop` uses the per-user data dir.** The data-dir module (P1) is therefore a self-contained, tested library that P2's wrapper consumes — it does not alter the existing bins.

**Oracle for behavioral correctness:** `verify.ps1` (the CI contract at `.github/workflows/ci.yml:82`) launches `cargo run --bin backend -- --config ../config.json` and polls `http://127.0.0.1:<port>/` for the exact body `"rsahp backend running"`. Any change that breaks this string, the `/` route, the `--config` flag, the root-relative config path, or the static-port bind for the standalone backend is WRONG — surface the conflict, do not "adapt".

**Every implementer task below MUST begin with Step 0 (state-verification):** open the cited file(s) and confirm the pasted "current code" matches reality at the cited lines. If it differs, STOP and report `STATE_MISMATCH: <what>` — do not adapt. If making code compile would change the shape/type/encoding of any value (a config key's type, the sqlite URL string form, a channel's message type), STOP and report `[original] -> [yours] because <reason>`.

---

## File Structure

**Phase 1 (data-dir module):** `common/Cargo.toml` (add deps) · `common/src/datadir.rs` (new) · `common/src/lib.rs` (module decl) · `backend/tests/datadir_url.rs` (new integration test).

**Phase 2 (wrapper + seam refactor):** `backend/src/lib.rs` (`run_server`) · `backend/src/main.rs` (thin) · `frontend/src/lib.rs` (`run_gui`) · `frontend/src/config.rs` (`use_gpu`, `enable_gpu`, `load_from`, save-path fix) · `frontend/src/main.rs` (thin) · `rsahp-desktop/` (new crate) · root `Cargo.toml` (member).

**Phase 3 (Windows):** `packaging/windows/rsahp.iss` · `packaging/windows/rsahp.ico` · `xtask/src/main.rs` (local build action) · `.github/workflows/release.yml`.

**Phase 4 (Linux):** `packaging/linux/io.github.ckir.rsahp.{yml,desktop,metainfo.xml,png}` · `packaging/linux/generate-sources.sh` · `.github/workflows/release.yml` (Flatpak leg).

---

# Phase 1 — Data-dir resolution module (`common::datadir`)

**Independently mergeable. Changes no existing binary's behavior.** Delivers tested path/URL/seed helpers.

### Task 1: Add `directories` + `percent-encoding` to `common`

**Files:** Modify `common/Cargo.toml`.

- [ ] **Step 0: Verify state.** Open `common/Cargo.toml`. Confirm `[dependencies]` currently holds only `chrono` and `serde`; no `directories`/`percent-encoding`. Else report `STATE_MISMATCH`.

- [ ] **Step 1: Add deps.** Under `[dependencies]`:

```toml
directories = "5.0"
percent-encoding = "2.3"
```

Also ensure a dev-dependency for the seed test (add if absent):

```toml
[dev-dependencies]
serde_json = "1.0"
```

- [ ] **Step 2: Verify resolution.** Run: `cargo fetch -p common`
  Expected: no error; `cargo tree -p common -i directories` shows `directories v5.x`.

- [ ] **Step 3: Commit.**

```bash
git add common/Cargo.toml Cargo.lock
git commit -m "build(common): add directories + percent-encoding for data-dir resolution"
```

### Task 2: `AppPaths` + `resolve()` + percent-encoded SQLite URL

**Files:** Create `common/src/datadir.rs`; modify `common/src/lib.rs`.

- [ ] **Step 0: Verify state.** Open `common/src/lib.rs`; confirm it declares only DTO structs, no `pub mod datadir`. Grep the repo for `ProjectDirs|data_local_dir` → zero real hits. Else `STATE_MISMATCH`.

- [ ] **Step 1: Create the module with tests first.** Create `common/src/datadir.rs`:

```rust
//! Per-user data directory resolution for the packaged desktop app.
//!
//! Only the `rsahp-desktop` wrapper uses this. The standalone `backend`/`frontend`
//! binaries keep their cwd-relative config/DB/logs behavior (dev unchanged).

use percent_encoding::{AsciiSet, CONTROLS, utf8_percent_encode};
use std::path::{Path, PathBuf};

/// Characters we percent-encode in a filesystem path before embedding it in a
/// `sqlite://` URL. Beyond CONTROLS: a SPACE (common in Windows usernames like
/// `C:\Users\John Doe\...`) and the URL-structural chars that would otherwise be
/// misparsed. We deliberately do NOT encode `/`, `:`, `.`, `-`, `_`, `~` (needed
/// intact for drive letters and path separators).
const PATH_ENCODE_SET: &AsciiSet = &CONTROLS
    .add(b' ')
    .add(b'"')
    .add(b'<')
    .add(b'>')
    .add(b'`')
    .add(b'#')
    .add(b'?')
    .add(b'{')
    .add(b'}')
    .add(b'%')
    .add(b'|')
    .add(b'^');

/// Resolved, absolute, per-user paths for the packaged app, under the OS-specific
/// **local** (non-roaming) data directory.
#[derive(Debug, Clone)]
pub struct AppPaths {
    pub data_dir: PathBuf,
    pub config_path: PathBuf,
    pub db_path: PathBuf,
    pub logs_dir: PathBuf,
}

impl AppPaths {
    #[must_use]
    pub fn database_url(&self) -> String {
        database_url_from_path(&self.db_path)
    }
}

/// Builds a URI-safe sea-orm/sqlx SQLite URL from an absolute filesystem path.
///
/// Backslashes → forward slashes; a leading `/` is guaranteed (empty-authority
/// absolute-path form); unsafe chars (esp. SPACE) are percent-encoded. Yields
/// `sqlite:///C:/Users/John%20Doe/.../rsahp.db?mode=rwc` on Windows and
/// `sqlite:///home/.../rsahp.db?mode=rwc` on Linux.
#[must_use]
pub fn database_url_from_path(db_path: &Path) -> String {
    let mut s = db_path.to_string_lossy().replace('\\', "/");
    if !s.starts_with('/') {
        s.insert(0, '/');
    }
    let encoded = utf8_percent_encode(&s, PATH_ENCODE_SET).to_string();
    format!("sqlite://{encoded}?mode=rwc")
}

/// Resolves the per-user local data dir and derived paths. `None` only if the OS
/// cannot supply a home/data directory.
#[must_use]
pub fn resolve() -> Option<AppPaths> {
    // Empty qualifier/org → `<LocalAppData>\rsahp\...` / `~/.local/share/rsahp`, no org
    // segment. data_local_dir() is LOCAL (non-roaming) — a SQLite DB must never roam.
    let dirs = directories::ProjectDirs::from("", "", "rsahp")?;
    let data_dir = dirs.data_local_dir().to_path_buf();
    let config_path = data_dir.join("config.json");
    let db_path = data_dir.join("rsahp.db");
    let logs_dir = data_dir.join("logs");
    Some(AppPaths { data_dir, config_path, db_path, logs_dir })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_to_local_non_roaming_dir_containing_rsahp() {
        let paths = resolve().expect("data dir should resolve on a normal machine");
        let s = paths.data_dir.to_string_lossy().to_lowercase();
        assert!(s.contains("rsahp"), "data_dir should contain 'rsahp': {s}");

        #[cfg(windows)]
        {
            let local = std::env::var("LOCALAPPDATA").expect("LOCALAPPDATA set");
            assert!(paths.data_dir.starts_with(&local), "must be under %LocalAppData%: {s}");
        }
        #[cfg(target_os = "linux")]
        {
            let home = std::env::var("HOME").expect("HOME set");
            let xdg = std::env::var("XDG_DATA_HOME").unwrap_or_else(|_| format!("{home}/.local/share"));
            assert!(paths.data_dir.starts_with(&xdg), "must be under XDG data home: {s}");
        }

        assert_eq!(paths.config_path, paths.data_dir.join("config.json"));
        assert_eq!(paths.db_path, paths.data_dir.join("rsahp.db"));
        assert_eq!(paths.logs_dir, paths.data_dir.join("logs"));
    }

    #[test]
    fn database_url_unix_absolute() {
        let unix = database_url_from_path(Path::new("/home/u/.local/share/rsahp/rsahp.db"));
        assert_eq!(unix, "sqlite:///home/u/.local/share/rsahp/rsahp.db?mode=rwc");
    }

    #[test]
    fn database_url_windows_encodes_space_in_username() {
        // The critical case: a Windows username with a space must be percent-encoded.
        let win = database_url_from_path(Path::new(r"C:\Users\John Doe\AppData\Local\rsahp\rsahp.db"));
        assert_eq!(win, "sqlite:///C:/Users/John%20Doe/AppData/Local/rsahp/rsahp.db?mode=rwc");
    }
}
```

- [ ] **Step 2: Wire the module.** In `common/src/lib.rs`, after the module doc comment (before the DTOs), add: `pub mod datadir;`

- [ ] **Step 3: Run tests.** Run: `cargo nextest run -p common`
  Expected: PASS — all three `datadir` tests green (the space-encoding test is the key one).

- [ ] **Step 4: fmt/clippy.** Run: `cargo fmt --all -- --check` && `cargo clippy -p common -- -D warnings`
  Expected: exit 0.

- [ ] **Step 5: Commit.**

```bash
git add common/src/datadir.rs common/src/lib.rs
git commit -m "feat(common): data-dir resolution + percent-encoded sqlite URL builder"
```

### Task 3: Seed helper + first-run directory creation

**Files:** Modify `common/src/datadir.rs`.

- [ ] **Step 0: Verify state.** Confirm Task 2 landed. Open repo-root `config.json`; confirm exactly the four keys `api_url`, `use_gpu`, `database_url`, `port`. Else `STATE_MISMATCH`.

- [ ] **Step 1: Add the seed constant + helper (module level, above `#[cfg(test)]`).**

```rust
/// Default `config.json` seeded on first run — **every key both binaries read** across
/// their separate `AppConfig` structs (backend: `database_url`, `port`; frontend:
/// `api_url`, `use_gpu`). `use_gpu: false` = safe default (GPU init can crash on faulty
/// drivers; capable machines opt in). Written verbatim (neither struct owns all keys).
pub const DEFAULT_CONFIG_JSON: &str = r#"{
  "api_url": "http://127.0.0.1:4002/api/documents",
  "use_gpu": false,
  "database_url": "sqlite://rsahp.db?mode=rwc",
  "port": 4002
}
"#;

/// Ensures `data_dir` + `logs_dir` exist and seeds `config_path` with
/// [`DEFAULT_CONFIG_JSON`] **only if it does not already exist** (idempotent — never
/// clobbers a user-edited config). NOTE: the wrapper creates `data_dir` separately
/// *before* acquiring the single-instance lock (the lock file lives in it); this call
/// runs *after* the lock so only the lock-winner seeds the config (no first-run race).
pub fn ensure_dirs_and_seed(
    data_dir: &Path,
    logs_dir: &Path,
    config_path: &Path,
) -> std::io::Result<()> {
    std::fs::create_dir_all(data_dir)?;
    std::fs::create_dir_all(logs_dir)?;
    if !config_path.exists() {
        std::fs::write(config_path, DEFAULT_CONFIG_JSON)?;
    }
    Ok(())
}
```

- [ ] **Step 2: Add the test (append to `tests` mod).**

```rust
    #[test]
    fn seed_writes_all_keys_and_is_idempotent() {
        let tmp = std::env::temp_dir().join(format!("rsahp_seed_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        let cfg = tmp.join("config.json");

        ensure_dirs_and_seed(&tmp, &tmp.join("logs"), &cfg).expect("seed ok");
        let v: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&cfg).unwrap()).unwrap();
        for key in ["api_url", "use_gpu", "database_url", "port"] {
            assert!(v.get(key).is_some(), "missing key: {key}");
        }
        assert!(tmp.join("logs").is_dir());

        std::fs::write(&cfg, r#"{"api_url":"http://custom"}"#).unwrap();
        ensure_dirs_and_seed(&tmp, &tmp.join("logs"), &cfg).expect("second seed ok");
        assert_eq!(std::fs::read_to_string(&cfg).unwrap(), r#"{"api_url":"http://custom"}"#);

        let _ = std::fs::remove_dir_all(&tmp);
    }
```

- [ ] **Step 3: Run + lint.** Run: `cargo nextest run -p common` then `cargo clippy -p common -- -D warnings`
  Expected: PASS; exit 0.

- [ ] **Step 4: Commit.**

```bash
git add common/src/datadir.rs
git commit -m "feat(common): first-run config seed + data-dir creation (idempotent)"
```

### Task 4: Integration test — sea-orm connects to the resolved absolute URL (spaced path)

**Rationale (folds F2 + verifies A4):** the unit test only checks the URL *string*. This test proves sea-orm/sqlx actually CONNECTS and migrates against the percent-encoded absolute URL, on the host OS, using a temp dir path that **contains a space** — the exact `John Doe` failure mode.

**Files:** Create `backend/tests/datadir_url.rs`.

- [ ] **Step 0: Verify state.** Open `backend/Cargo.toml`; confirm `common = { path = "../common" }` is a normal dep, `tokio` has `features = ["full"]`, and `setup_schema` is `pub` in `backend/src/lib.rs` (line 17). Else `STATE_MISMATCH`. Note: a package's regular deps ARE nameable from its `tests/` integration tests — but if `use common::…` fails to resolve at Step 2, add `common = { path = "../common" }` to backend `[dev-dependencies]` (harmless duplicate) OR move this test into a `#[cfg(test)] mod` inside `backend/src/lib.rs` (which unambiguously sees normal deps).

- [ ] **Step 1: Write the test.** Create `backend/tests/datadir_url.rs`:

```rust
//! Proves the percent-encoded absolute SQLite URL from `common::datadir` actually
//! connects + migrates via sea-orm on the host OS, including a path with a SPACE.

use common::datadir::database_url_from_path;

#[tokio::test]
async fn sea_orm_connects_and_migrates_via_encoded_absolute_url_with_space() {
    // Deliberately include a space to mimic `C:\Users\John Doe\...`.
    let dir = std::env::temp_dir()
        .join("rsahp url test with space")
        .join(format!("run_{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let db_path = dir.join("rsahp.db");

    let url = database_url_from_path(&db_path);
    assert!(url.contains("%20"), "spaced path must be percent-encoded: {url}");

    let db = sea_orm::Database::connect(&url)
        .await
        .expect("sea-orm must connect to the encoded absolute sqlite URL");
    backend::setup_schema(&db)
        .await
        .expect("migrations must run against the resolved URL");

    drop(db);
    let _ = std::fs::remove_dir_all(&dir);
}
```

- [ ] **Step 2: Run it.** Run: `cargo nextest run -p backend datadir_url`
  Expected: PASS on the host OS. If it FAILS at `connect` on Windows, the URL slash-count or encoding is wrong — report the exact error (this is the definitive gate for the sqlite URL form; do NOT proceed with a broken URL). On Linux the same test proves the Unix form.

- [ ] **Step 3: Commit.**

```bash
git add backend/tests/datadir_url.rs
git commit -m "test(backend): sea-orm connects+migrates via encoded absolute URL (spaced path)"
```

### Task 5: Phase 1 gate

- [ ] **Step 1:** Run: `cargo build --workspace --exclude xtask` && `cargo nextest run -p common -p backend`
  Expected: build ok; all tests pass.
- [ ] **Step 2:** Run: `pwsh ./verify.ps1`
  Expected: `Configuration verified successfully!` — proves the standalone backend is untouched by Phase 1.

---

# Phase 2 — `rsahp-desktop` wrapper + seam refactor

### Task 6: Add `backend::run_server`

**Files:** Modify `backend/src/lib.rs`.

- [ ] **Step 0: Verify state.** Confirm line 13 is `use sea_orm::DbErr;`, `setup_schema` at 17, `create_router` at 120, `/` route returns `"rsahp backend running"` (126). Else `STATE_MISMATCH`.

- [ ] **Step 1: Extend imports.** Replace line 13 (`use sea_orm::DbErr;`) with:

```rust
use sea_orm::DbErr;
use std::net::SocketAddr;
use tokio::sync::oneshot;
```

- [ ] **Step 2: Add `run_server` immediately AFTER `create_router`'s closing `}` (after line 147), before the `#[cfg(test)]` items.**

```rust
/// Connects the DB, applies migrations, and serves the axum app on `bind_addr`.
///
/// Sends the **actually-bound** [`SocketAddr`] over `ready_tx` (pass `127.0.0.1:0` for
/// an ephemeral port and learn the real one — no blind retry). Graceful shutdown is
/// driven by `shutdown_rx`. On any startup failure (connect / migrate / bind) this
/// returns `Err` and drops `ready_tx`, so a caller blocked on the ready channel observes
/// the failure instead of hanging.
pub async fn run_server(
    db_url: String,
    bind_addr: SocketAddr,
    ready_tx: oneshot::Sender<SocketAddr>,
    shutdown_rx: oneshot::Receiver<()>,
) -> Result<(), DbErr> {
    let db = sea_orm::Database::connect(&db_url).await?;
    tracing::info!("Connected to database: {}", db_url);
    setup_schema(&db).await?;

    let app = create_router(db);

    let listener = tokio::net::TcpListener::bind(bind_addr)
        .await
        .map_err(|e| DbErr::Custom(format!("failed to bind {bind_addr}: {e}")))?;
    let local_addr = listener
        .local_addr()
        .map_err(|e| DbErr::Custom(format!("failed to read local addr: {e}")))?;
    tracing::info!("Listening on {}", local_addr);

    let _ = ready_tx.send(local_addr);

    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            let _ = shutdown_rx.await;
            tracing::info!("Shutdown signal received, starting graceful shutdown...");
        })
        .await
        .map_err(|e| DbErr::Custom(format!("server error: {e}")))?;

    Ok(())
}
```

- [ ] **Step 3: Build.** Run: `cargo build -p backend`. Expected: compiles (error propagation, no `.unwrap()`).
- [ ] **Step 4: Commit.**

```bash
git add backend/src/lib.rs
git commit -m "feat(backend): add run_server (ephemeral bind + ready-signal + graceful shutdown)"
```

### Task 7: Refactor `backend/src/main.rs` to call `run_server`

**Files:** Modify `backend/src/main.rs`. This task gives the **complete resulting `main()` body** — do not do piecemeal line surgery; verify the current file then replace `main()` wholesale.

- [ ] **Step 0: Verify state.** Confirm current `main()` is lines 13–69 (config load 16, port 18, db_url 20–22, tracing 27–39, `Database::connect`/`setup_schema`/`create_router` 45–52, bind/serve 54–65) and `shutdown_signal()` variants at 72–138. Else `STATE_MISMATCH`.

- [ ] **Step 1: Replace the entire `main()` function (lines 13–69) with:**

```rust
#[tokio::main]
async fn main() -> Result<(), DbErr> {
    // Load application configuration
    let config = config::AppConfig::load();
    let port = config.port.unwrap_or(3001);
    let db_url = config
        .database_url
        .unwrap_or_else(|| "sqlite://rsahp.db?mode=rwc".to_string());

    // Rolling daily file log (cwd-relative — dev behavior unchanged).
    let file_appender = RollingFileAppender::new(Rotation::DAILY, "logs", "rsahp_backend.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    tracing_subscriber::registry()
        .with(EnvFilter::new("info"))
        .with(fmt::layer().json().with_writer(std::io::stdout))
        .with(fmt::layer().json().with_writer(non_blocking))
        .init();

    tracing::info!("Starting AHP Backend Server...");

    // Bridge the OS signal handler into the oneshot run_server expects.
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
    tokio::spawn(async move {
        shutdown_signal().await;
        let _ = shutdown_tx.send(());
    });

    // Standalone bin binds the STATIC port from config (dev/verify.ps1 unchanged). The
    // bound-addr report is unused here (run_server logs it), so drop the receiver.
    let bind_addr: std::net::SocketAddr = format!("127.0.0.1:{}", port)
        .parse()
        .expect("valid socket address");
    let (ready_tx, _ready_rx) = tokio::sync::oneshot::channel::<std::net::SocketAddr>();

    backend::run_server(db_url, bind_addr, ready_tx, shutdown_rx).await?;

    Ok(())
}
```

- [ ] **Step 2: Fix imports.** Line 5 `use sea_orm::{Database, DbErr};` → `use sea_orm::DbErr;` (drop `Database`). Line 9 `use backend::{config, create_router, setup_schema};` → `use backend::config;`. Leave the `tracing_appender`/`tracing_subscriber` imports (6–7) and the `shutdown_signal()` definitions (72–138) untouched.

- [ ] **Step 3: Build with zero unused-import warnings.** Run: `cargo build -p backend 2>&1`
  Expected: compiles; NO unused-import warnings for `Database`, `create_router`, `setup_schema`.

- [ ] **Step 4: Oracle — the standalone contract.** Run: `pwsh ./verify.ps1`
  Expected: `Configuration verified successfully! Backend booted and bound to port 4002.`

- [ ] **Step 5: Backend tests.** Run: `cargo nextest run -p backend`. Expected: PASS.
- [ ] **Step 6: Commit.**

```bash
git add backend/src/main.rs
git commit -m "refactor(backend): main.rs delegates to run_server via signal→oneshot bridge"
```

### Task 8: Frontend config — live `use_gpu`, `--enable-gpu`, `load_from`, and the `save()` path fix

**Files:** Modify `frontend/src/config.rs`. Gives complete resulting items. **Folds F1** (`save()` must write to the resolved path, not re-parse CLI into cwd).

- [ ] **Step 0: Verify state.** Confirm: `CliArgs` (8–22) has `pub disable_gpu: bool` (19–21); `AppConfig` (25–31) = `api_url` + `zoom_scale`; `Default` (34–44); `load()` (49–77) uses `CliArgs::parse()`; `save()` (80–92) re-parses CLI for the path. Then run `git grep -n "disable_gpu"` — expect ONLY `frontend/src/config.rs:21` and `frontend/src/main.rs:33` (confirmed 2026-07-14). Any other hit (a test/script/CI) → update it in this task or Task 9; an unexplained hit → `STATE_MISMATCH`. Also `git grep -n "config.save\|\.save()"` — expect `frontend/src/ui/taskbar.rs:120` (persists `zoom_scale`); confirm that call site.

- [ ] **Step 1: Update imports.** At the top of `frontend/src/config.rs`, add: `use std::path::{Path, PathBuf};`

- [ ] **Step 2: Rename the CLI flag (opt-out → opt-in).** Replace lines 19–21:

```rust
    /// Enable GPU hardware acceleration (OFF by default; equivalent to config `use_gpu: true`).
    #[arg(long)]
    pub enable_gpu: bool,
```

- [ ] **Step 3: Replace the whole `AppConfig` struct + `Default` impl (lines 25–44) with:**

```rust
/// The main configuration structure for the application.
#[derive(Serialize, Deserialize, Debug)]
pub struct AppConfig {
    /// The URL of the API server.
    pub api_url: Option<String>,
    /// The UI zoom scale factor.
    pub zoom_scale: Option<f32>,
    /// Request GPU hardware acceleration. `None`/`Some(false)` → CPU (safe default);
    /// `Some(true)` → GPU. Also settable at runtime via `--enable-gpu`.
    pub use_gpu: Option<bool>,
    /// The path this config was loaded from, so `save()` writes back to the SAME file
    /// (not a re-parsed CWD default). Not serialized.
    #[serde(skip)]
    config_path: Option<PathBuf>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            api_url: None,
            zoom_scale: Some(1.25),
            use_gpu: None,
            config_path: None,
        }
    }
}
```

- [ ] **Step 4: Replace the `impl AppConfig` block's `load()` and `save()` (lines 47–92) with the following, adding `load_from`:**

```rust
impl AppConfig {
    /// Loads config from `config.json` (or `--config <path>`), then applies CLI overrides.
    pub fn load() -> (Self, CliArgs) {
        let cli = CliArgs::parse();
        let config_path = cli
            .config
            .clone()
            .unwrap_or_else(|| "config.json".to_string());
        let mut config = AppConfig::load_from(Path::new(&config_path));

        if let Some(url) = cli.api_url.clone() {
            config.api_url = Some(url);
        }
        (config, cli)
    }

    /// Loads config from an explicit path (no CLI parsing) — used by `rsahp-desktop`.
    /// Records the path so `save()` writes back to it.
    #[must_use]
    pub fn load_from(path: &Path) -> Self {
        let mut config = AppConfig::default();
        if let Ok(content) = fs::read_to_string(path)
            && let Ok(parsed) = serde_json::from_str::<AppConfig>(&content)
        {
            config = parsed;
        }
        config.config_path = Some(path.to_path_buf());
        config
    }

    /// Saves the current configuration back to the path it was loaded from (falling back
    /// to `config.json` in the CWD only if unknown). Does NOT re-parse CLI args — in the
    /// packaged wrapper that would target a read-only install dir and silently fail.
    pub fn save(&self) {
        let path = self
            .config_path
            .clone()
            .unwrap_or_else(|| PathBuf::from("config.json"));
        if let Ok(content) = serde_json::to_string_pretty(self) {
            let _ = fs::write(path, content);
        }
    }
}
```

- [ ] **Step 5: Build `config.rs` in isolation.** Run: `cargo build -p frontend 2>&1`
  Expected: `config.rs` compiles. A remaining error naming `disable_gpu` in `main.rs` is fixed in Task 9 — but if `config.rs` itself errors, report it. (Do NOT commit yet — the crate is non-compiling until Task 9; commit there.)

### Task 9: Add `frontend::run_gui` and thin out `frontend/src/main.rs`

**Files:** Modify `frontend/src/lib.rs` and `frontend/src/main.rs`.

- [ ] **Step 0: Verify state.** `frontend/src/lib.rs` exposes `pub mod config; pub mod ui;`. `frontend/src/main.rs` lines 30–56 (GPU decision on `cli_args.disable_gpu`, `run_native("AHP Group Decision System", ...)`), with `#![allow(warnings)]` at line 3. Else `STATE_MISMATCH`.

- [ ] **Step 1: Append `run_gui` to `frontend/src/lib.rs`.**

```rust
use crate::config::AppConfig;
use crate::ui::RsahpApp;

/// Runs the eframe GUI on the **calling (main) thread** (eframe requires it), pointed at
/// `api_base` (e.g. `http://127.0.0.1:PORT/api/documents`). GPU is requested only when
/// `config.use_gpu == Some(true)` — default is CPU.
///
/// # Errors
/// Returns any `eframe::Error` from `run_native`.
pub fn run_gui(api_base: String, mut config: AppConfig) -> Result<(), eframe::Error> {
    config.api_url = Some(api_base);

    let hardware_acceleration = if config.use_gpu == Some(true) {
        eframe::HardwareAcceleration::Preferred
    } else {
        eframe::HardwareAcceleration::Off
    };

    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_maximized(true),
        hardware_acceleration,
        ..Default::default()
    };

    eframe::run_native(
        "AHP Group Decision System",
        options,
        Box::new(move |_cc| Ok(Box::new(RsahpApp::new(config)))),
    )
}
```

- [ ] **Step 2: Replace `frontend/src/main.rs` lines 29–56 (config load through the end of `main`) with:**

```rust
    // Load the application configuration.
    let (mut config, cli_args) = AppConfig::load();

    // `--enable-gpu` is a dev opt-in equivalent to config `use_gpu: true` (OR semantics).
    if cli_args.enable_gpu {
        config.use_gpu = Some(true);
    }

    let api_base = config
        .api_url
        .clone()
        .unwrap_or_else(|| "http://127.0.0.1:4002/api/documents".to_string());

    frontend::run_gui(api_base, config)
}
```

If `cargo build` warns `unused import: eframe::egui` at line 4, delete line 4 (the `#![allow(warnings)]` may suppress it — either way, harmless).

- [ ] **Step 3: Build the crate cleanly.** Run: `cargo build -p frontend 2>&1`. Expected: compiles, no `disable_gpu` errors.
- [ ] **Step 4: Frontend tests.** Run: `cargo nextest run -p frontend`. Expected: PASS.
- [ ] **Step 5: Commit config + run_gui together.**

```bash
git add frontend/src/config.rs frontend/src/lib.rs frontend/src/main.rs
git commit -m "feat(frontend): live use_gpu (default OFF, --enable-gpu), run_gui seam, save() path fix"
```

### Task 10: Scaffold the `rsahp-desktop` crate

**Files:** Create `rsahp-desktop/Cargo.toml`, `rsahp-desktop/src/main.rs`; modify root `Cargo.toml`.

- [ ] **Step 0: Verify state.** Root `Cargo.toml` `members = ["frontend","backend","common","xtask","migration"]`, `resolver = "2"`; no `rsahp-desktop` dir. Else `STATE_MISMATCH`.

- [ ] **Step 1: Add member.** Root `Cargo.toml`:

```toml
members = [
    "frontend",
    "backend",
    "common",
    "xtask",
    "migration",
    "rsahp-desktop"
]
```

- [ ] **Step 2: Create `rsahp-desktop/Cargo.toml`.**

```toml
[package]
name = "rsahp-desktop"
version = "0.1.0"
publish = false
edition = "2024"

[[bin]]
name = "rsahp-desktop"
path = "src/main.rs"

[dependencies]
backend = { version = "0.1.0", path = "../backend" }
frontend = { version = "0.1.0", path = "../frontend" }
common = { version = "0.1.0", path = "../common" }
tokio = { version = "1.52.3", features = ["rt-multi-thread", "sync"] }
tracing = "0.1.44"
tracing-appender = "0.2.5"
tracing-subscriber = { version = "0.3.23", features = ["env-filter", "json"] }
fd-lock = "4"
rfd = "0.17.2"
```

Note: no `[lints] workspace = true` (matches sibling app crates `backend`/`frontend`). `fd-lock` (used by rustup) has an unambiguous `try_write() -> io::Result<guard>` API — `Err(WouldBlock)` means held (this is why we chose it over `fs4`, whose return type varies by version).

- [ ] **Step 3: Compiling stub.** `rsahp-desktop/src/main.rs`:

```rust
//! Single-binary desktop wrapper (backend on a bg thread, GUI on the main thread).
fn main() {
    println!("rsahp-desktop stub");
}
```

- [ ] **Step 4: Build.** Run: `cargo build -p rsahp-desktop`. Expected: compiles; `fd-lock`, `rfd` resolve.
- [ ] **Step 5: Commit.**

```bash
git add Cargo.toml Cargo.lock rsahp-desktop/Cargo.toml rsahp-desktop/src/main.rs
git commit -m "feat(rsahp-desktop): scaffold wrapper crate + workspace member"
```

### Task 11: Implement the wrapper lifecycle

**Files:** Modify `rsahp-desktop/src/main.rs`. **Folds A2** (bounded exit, no zombie), **A3** (create dir → lock → seed), **F3** (`fd-lock`), **A1** (documented hardening).

- [ ] **Step 0: Verify state.** Confirm `common::datadir::{resolve, ensure_dirs_and_seed, AppPaths}`, `backend::run_server`, `frontend::{run_gui, config::AppConfig}` exist with the signatures from Tasks 2/3/6/8/9. Confirm `fd-lock v4` exposes `RwLock::<File>::try_write(&mut self) -> io::Result<RwLockWriteGuard>` (`cargo doc -p fd-lock --no-deps`); if the API differs at the resolved version, adapt and note it. Else `STATE_MISMATCH`.

- [ ] **Step 1: Replace `rsahp-desktop/src/main.rs` entirely.**

```rust
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
        Err(e) => fatal("rsahp", &format!("Failed to acquire single-instance lock: {e}")),
    };

    // 4. Now (lock-winner only) create logs + seed config.json.
    if let Err(e) =
        datadir::ensure_dirs_and_seed(&paths.data_dir, &paths.logs_dir, &paths.config_path)
    {
        fatal("rsahp", &format!("Failed to initialize data directory: {e}"));
    }

    // 5. Logging → <data_dir>/logs.
    let file_appender = RollingFileAppender::new(Rotation::DAILY, &paths.logs_dir, "rsahp_desktop.log");
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

    // 7. Channels + backend on a bg thread bound to an EPHEMERAL port.
    let (ready_tx, ready_rx) = tokio::sync::oneshot::channel::<SocketAddr>();
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

    let server_thread = std::thread::spawn(move || {
        let rt = match tokio::runtime::Builder::new_multi_thread().enable_all().build() {
            Ok(rt) => rt,
            Err(e) => {
                tracing::error!("failed to build tokio runtime: {e}");
                return; // ready_tx drops → main surfaces the failure
            }
        };
        let bind_addr: SocketAddr = "127.0.0.1:0".parse().expect("valid loopback addr");
        if let Err(e) = rt.block_on(backend::run_server(db_url, bind_addr, ready_tx, shutdown_rx)) {
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

    // 10. Window closed → graceful shutdown → bounded exit. The watchdog guards ONLY the
    //     clean path (a hung connection-drain must never zombie us). It is NOT spawned on
    //     the error path — otherwise its 3s exit(0) would race the modal error dialog,
    //     killing it with a SUCCESS code and masking the failure.
    let _ = shutdown_tx.send(());
    match gui_result {
        Ok(()) => {
            std::thread::spawn(|| {
                std::thread::sleep(Duration::from_secs(3));
                std::process::exit(0);
            });
            let _ = server_thread.join();
            drop(log_guard); // flush buffered logs before exiting (process::exit skips Drop)
            std::process::exit(0);
        }
        Err(e) => {
            // No watchdog here: let the user acknowledge the dialog. fatal() exits the
            // process (which reaps the still-running server thread); flush logs first.
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
```

- [ ] **Step 2: Build.** Run: `cargo build -p rsahp-desktop 2>&1`. Expected: compiles. If `fd-lock`'s guard lifetime complains, ensure `instance_lock` is `let mut` and outlives `_lock_guard` (both in `main` scope).

- [ ] **Step 3: Smoke test (local, has display).** Run: `cargo run -p rsahp-desktop`
  Expected: window "AHP Group Decision System" appears; `%LocalAppData%\rsahp\{config.json,rsahp.db,rsahp.lock,logs\rsahp_desktop.log}` created; login modal renders (backend reachable on ephemeral port). Close → process exits within ~3s; `Get-Process rsahp-desktop -ErrorAction SilentlyContinue` → empty.

- [ ] **Step 4: Single-instance test.** Launch twice (second while first is up). Expected: second shows "rsahp is already running." and exits; first unaffected.

- [ ] **Step 5: Backend-failure test.** Temporarily change `"127.0.0.1:0"` to a port you already occupy; run; confirm the "backend service failed to start" dialog + non-zero exit; then REVERT.

- [ ] **Step 6: Commit.**

```bash
git add rsahp-desktop/src/main.rs
git commit -m "feat(rsahp-desktop): lifecycle — lock-before-seed, ephemeral port, bounded graceful exit"
```

### Task 12: Phase 2 gate

- [ ] **Step 1:** Run: `cargo build --workspace` && `cargo nextest run`. Expected: all green.
- [ ] **Step 2:** Run: `pwsh ./verify.ps1`, then `pwsh ./start.ps1` (Windows) → confirm two separate `backend`/`frontend` processes launch from `target\debug` against root `config.json`; stop them (`Stop-Process -Name backend,frontend -Force`).
- [ ] **Step 3:** Run: `cargo fmt --all -- --check` && `cargo clippy -- -D warnings`. Expected: exit 0 (repo's exact gate — no stricter flags).

---

# Phase 3 — Windows Inno Setup installer

### Task 13: Placeholder app icon (both `.ico` and `.png`)

**Files:** Create `packaging/windows/rsahp.ico` and `packaging/linux/io.github.ckir.rsahp.png`. **Folds F5** (generator must not join the workspace) + **F9** (always produce both formats).

- [ ] **Step 0: Verify state.** No `packaging/` dir yet. Check ImageMagick: `magick -version` (or `convert -version`).

- [ ] **Step 1a (if ImageMagick present): produce BOTH formats.**

```bash
mkdir -p packaging/windows packaging/linux
magick -size 256x256 xc:"#3B3B98" -gravity center -pointsize 96 -fill "#F5F5F5" -annotate 0 "AHP" packaging/windows/rsahp.ico
magick -size 256x256 xc:"#3B3B98" -gravity center -pointsize 96 -fill "#F5F5F5" -annotate 0 "AHP" packaging/linux/io.github.ckir.rsahp.png
```

- [ ] **Step 1b (fallback, no ImageMagick): generate via a throwaway crate OUTSIDE the repo tree** (so it never becomes an un-excluded workspace member that breaks `cargo build --workspace`). Use the scratchpad dir. Create `<SCRATCH>/icongen/Cargo.toml`:

```toml
[package]
name = "icongen"
version = "0.0.0"
publish = false
edition = "2021"

[dependencies]
image = "0.25"
ico = "0.3"
```

`<SCRATCH>/icongen/src/main.rs`:

```rust
use image::{Rgba, RgbaImage};
use std::path::Path;

fn main() {
    let out = std::env::args().nth(1).expect("pass the repo root as arg 1");
    let root = Path::new(&out);
    let size = 256u32;
    let mut img = RgbaImage::from_pixel(size, size, Rgba([59, 59, 152, 255]));
    for x in 0..size {
        for y in 0..size {
            if x < 8 || y < 8 || x >= size - 8 || y >= size - 8 {
                img.put_pixel(x, y, Rgba([245, 245, 245, 255]));
            }
        }
    }
    std::fs::create_dir_all(root.join("packaging/windows")).unwrap();
    std::fs::create_dir_all(root.join("packaging/linux")).unwrap();
    img.clone()
        .save(root.join("packaging/linux/io.github.ckir.rsahp.png"))
        .unwrap();

    let mut dir = ico::IconDir::new(ico::ResourceType::Icon);
    let icon = ico::IconImage::from_rgba_data(size, size, img.into_raw());
    dir.add_entry(ico::IconDirEntry::encode(&icon).unwrap());
    dir.write(std::fs::File::create(root.join("packaging/windows/rsahp.ico")).unwrap())
        .unwrap();
    println!("wrote rsahp.ico + io.github.ckir.rsahp.png under {out}");
}
```

Run it, passing the repo root, then it needs no cleanup inside the repo:

```bash
cargo run --manifest-path "$SCRATCH/icongen/Cargo.toml" -- "$(pwd)"
```

(`$SCRATCH` = the session scratchpad dir. The generator lives outside the repo, so `cargo build --workspace` is never affected.)

- [ ] **Step 2: Verify both exist.** Run: `ls -la packaging/windows/rsahp.ico packaging/linux/io.github.ckir.rsahp.png`. Expected: two files > 0 bytes.
- [ ] **Step 3: Confirm workspace still builds (no stray member).** Run: `cargo build --workspace --exclude xtask`. Expected: ok.
- [ ] **Step 4: Commit.**

```bash
git add packaging/windows/rsahp.ico packaging/linux/io.github.ckir.rsahp.png
git commit -m "feat(packaging): placeholder app icon (.ico + .png)"
```

### Task 14: Inno Setup script + local build xtask action

**Files:** Create `packaging/windows/rsahp.iss`; modify `xtask/src/main.rs`. **Folds F4** (correct arm index) + **A6** (`cargo pkgid`, not JSON string-slicing).

- [ ] **Step 0: Verify state.** Open `xtask/src/main.rs`; confirm the `selections` array (lines 44–53, 8 entries `[1]`…`[0] Quit`) and the `match selection { 0 => quick … 6 => version_bump, 7 => break, _ => unreachable!() }` (lines 66–140). Confirm `use xshell::{cmd, Shell};` is imported. Confirm a repo-root `LICENSE` file exists (if not, omit the `LICENSE` line in the `.iss` `[Files]`). Else `STATE_MISMATCH`.

- [ ] **Step 1: Create `packaging/windows/rsahp.iss`.**

```inno
; rsahp Windows installer (per-user, no admin). Version injected via /DMyAppVersion.
#ifndef MyAppVersion
  #define MyAppVersion "0.0.0"
#endif
#define MyAppName "rsahp"
#define MyAppExeName "rsahp-desktop.exe"

[Setup]
AppId={{A9F3C2E1-7B4D-4E8A-9C1F-000000000001}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
DefaultDirName={localappdata}\Programs\rsahp
DefaultGroupName=rsahp
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
OutputDir=..\..\dist
OutputBaseFilename=rsahp-setup-{#MyAppVersion}
Compression=lzma
SolidCompression=yes
WizardStyle=modern
UninstallDisplayIcon={app}\{#MyAppExeName}

[Files]
Source: "..\..\target\release\{#MyAppExeName}"; DestDir: "{app}"; Flags: ignoreversion
Source: "rsahp.ico"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\LICENSE"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\rsahp"; Filename: "{app}\{#MyAppExeName}"; IconFilename: "{app}\rsahp.ico"
Name: "{userdesktop}\rsahp"; Filename: "{app}\{#MyAppExeName}"; IconFilename: "{app}\rsahp.ico"; Tasks: desktopicon

[Tasks]
Name: "desktopicon"; Description: "Create a &desktop shortcut"; GroupDescription: "Additional icons:"; Flags: unchecked

[Run]
Filename: "{app}\{#MyAppExeName}"; Description: "Launch rsahp"; Flags: nowait postinstall skipifsilent
```

No bundled DB/config — seeded in `%LocalAppData%\rsahp\` on first run. **Deferred (F8):** the spec's opt-in "remove user data on uninstall" checkbox is NOT implemented here; user data is always preserved. This is a deliberate, documented simplification — flag it to the user; add an `[UninstallDelete]` + a Tasks-gated checkbox later if wanted.

- [ ] **Step 2: Add the xtask menu entry.** In `selections` (lines 44–53), insert immediately BEFORE the `"[0] Quit"` line:

```rust
        "[8] SHIP & RELEASE: Build Windows Installer (local)",
```

This new entry takes array index **7**, so the existing `"[0] Quit"` shifts to array index **8**.

- [ ] **Step 3: Update the match arms accordingly.** In the `match selection` block: the existing quit arm `7 => break,` must become `8 => break,`, and add the new arm at index 7:

```rust
        7 => build_windows_installer(&sh)?,
        8 => break,
```

(Confirm by reading the surrounding arms that the pre-existing quit arm was `7 => break` and is the one you renumber to `8`. Do NOT leave two arms matching the same index.)

- [ ] **Step 4: Implement the action (A6: robust version via `cargo pkgid`).** Add near `version_bump`:

```rust
/// Builds `rsahp-desktop` (release) and runs Inno Setup's `iscc`, injecting the version
/// from `cargo pkgid` (robust — no JSON string-slicing). Windows-only.
fn build_windows_installer(sh: &Shell) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(not(windows))]
    {
        let _ = sh;
        println!("Windows installer build is only supported on Windows.");
        Ok(())
    }
    #[cfg(windows)]
    {
        cmd!(sh, "cargo build --release -p rsahp-desktop").run()?;

        // `cargo pkgid -p rsahp-desktop` → e.g. "path+file:///…#rsahp-desktop@0.1.0"
        // (or "…#0.1.0"). Take the version after the last '@', else after the last '#'.
        let pkgid = cmd!(sh, "cargo pkgid -p rsahp-desktop").read()?;
        let pkgid = pkgid.trim();
        let version = pkgid
            .rsplit_once('@')
            .map(|(_, v)| v)
            .or_else(|| pkgid.rsplit_once('#').map(|(_, v)| v))
            .ok_or("could not parse version from cargo pkgid")?
            .to_string();
        println!("Building installer for rsahp-desktop v{version}");

        let def = format!("/DMyAppVersion={version}");
        cmd!(sh, "iscc {def} packaging/windows/rsahp.iss").run()?;
        println!("Installer written to dist/rsahp-setup-{version}.exe");
        Ok(())
    }
}
```

- [ ] **Step 5: Build xtask.** Run: `cargo build -p xtask`. Expected: compiles on all platforms.
- [ ] **Step 6: Local installer (Windows only; `iscc` on PATH via `choco install innosetup`).** Run: `cargo build --release -p rsahp-desktop` then `iscc /DMyAppVersion=0.1.0 packaging/windows/rsahp.iss`.
  Expected: `dist/rsahp-setup-0.1.0.exe`. Install (no admin) → Start-menu launch works → uninstall removes program files (user data preserved).
- [ ] **Step 7: gitignore dist.** Add `dist/` to `.gitignore` if absent.
- [ ] **Step 8: Commit.**

```bash
git add packaging/windows/rsahp.iss xtask/src/main.rs .gitignore
git commit -m "feat(packaging): Inno script + local Windows installer xtask action"
```

### Task 15: Release workflow — gating test job + Windows leg

**Files:** Create `.github/workflows/release.yml`. **Folds A7** (release gated on tests).

- [ ] **Step 0: Verify state.** `.github/workflows/ci.yml` triggers on push/PR to `master`, single matrix job. No `release.yml` yet. Else `STATE_MISMATCH`.

- [ ] **Step 1: Create `.github/workflows/release.yml`.**

```yaml
name: Release

on:
  push:
    tags:
      - "v*.*.*"   # pinned pattern (not bare v*) — only semver tags trigger a release
  workflow_dispatch: {}

permissions:
  contents: write

jobs:
  # A7: gate the release on the test suite. A tag on a broken commit must NOT publish.
  test:
    name: Test gate
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Linux GUI/audio build deps
        run: |
          sudo apt-get update
          sudo apt-get install -y libasound2-dev libudev-dev libxkbcommon-dev libwayland-dev libx11-xcb-dev
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy
      - uses: Swatinem/rust-cache@v2
      - uses: taiki-e/install-action@v2
        with:
          tool: cargo-nextest
      - run: cargo fmt --all -- --check
      - run: cargo clippy -- -D warnings
      - run: cargo nextest run

  windows-installer:
    name: Windows installer
    needs: test
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - name: Install Inno Setup
        run: choco install innosetup --no-progress -y
      - name: Build wrapper (release)
        run: cargo build --release -p rsahp-desktop
      - name: Resolve version
        id: ver
        shell: pwsh
        run: |
          $v = (cargo metadata --format-version 1 --no-deps | ConvertFrom-Json).packages `
               | Where-Object { $_.name -eq 'rsahp-desktop' } | Select-Object -First 1 -ExpandProperty version
          "version=$v" >> $env:GITHUB_OUTPUT
      - name: Build installer
        run: iscc /DMyAppVersion=${{ steps.ver.outputs.version }} packaging/windows/rsahp.iss
      - name: Upload installer artifact
        uses: actions/upload-artifact@v4
        with:
          name: rsahp-setup-windows
          path: dist/rsahp-setup-*.exe
          if-no-files-found: error
      - name: Attach to release
        if: startsWith(github.ref, 'refs/tags/')
        uses: softprops/action-gh-release@v2
        with:
          files: dist/rsahp-setup-*.exe
```

- [ ] **Step 2: Validate YAML.** Run: `yq '.' .github/workflows/release.yml > /dev/null && echo OK` (or visual check). Expected: `OK`.
- [ ] **Step 3: Commit.**

```bash
git add .github/workflows/release.yml
git commit -m "ci: test-gated release workflow with Windows installer leg"
```

Note: `windows-latest` has no display; the job only BUILDS the installer. Launch/uninstall is the manual Task 14 Step 6.

---

# Phase 4 — Linux Flatpak

### Task 16: Flatpak metadata (.desktop, metainfo, icon)

**Files:** Create `packaging/linux/io.github.ckir.rsahp.{desktop,metainfo.xml}` (icon PNG from Task 13).

- [ ] **Step 0: Verify state.** App-id `io.github.ckir.rsahp`; `packaging/linux/io.github.ckir.rsahp.png` exists (Task 13). All filenames MUST carry the app-id prefix.

- [ ] **Step 1: `.desktop`.**

```ini
[Desktop Entry]
Type=Application
Name=rsahp
Comment=AHP Group Decision System
Exec=rsahp-desktop
Icon=io.github.ckir.rsahp
Terminal=false
Categories=Office;Utility;
```

- [ ] **Step 2: metainfo.** `packaging/linux/io.github.ckir.rsahp.metainfo.xml`:

```xml
<?xml version="1.0" encoding="UTF-8"?>
<component type="desktop-application">
  <id>io.github.ckir.rsahp</id>
  <metadata_license>CC0-1.0</metadata_license>
  <project_license>LicenseRef-proprietary</project_license>
  <name>rsahp</name>
  <summary>AHP Group Decision System</summary>
  <description>
    <p>rsahp is a desktop application for Analytic Hierarchy Process (AHP) group
       decision-making. It bundles a local backend service and a native GUI.</p>
  </description>
  <launchable type="desktop-id">io.github.ckir.rsahp.desktop</launchable>
  <content_rating type="oars-1.1" />
  <releases>
    <release version="0.1.0" date="2026-07-14" />
  </releases>
</component>
```

Set `<project_license>` to the repo's real SPDX id if a `LICENSE` exists. **F10:** the `<release>` version/date are hardcoded — extend the `version_bump` xtask action (see final notes) to also update this file, so it does not go stale on a bump.

- [ ] **Step 3: Validate (if `appstreamcli` available).** Run: `appstreamcli validate packaging/linux/io.github.ckir.rsahp.metainfo.xml`. Expected: `Validation successful` (screenshot warnings OK; ERRORS must be fixed — flatpak-builder runs this and fails on errors). Re-checked in CI (Task 18) if unavailable locally.
- [ ] **Step 4: Commit.**

```bash
git add packaging/linux/io.github.ckir.rsahp.desktop packaging/linux/io.github.ckir.rsahp.metainfo.xml
git commit -m "feat(packaging): Flatpak desktop entry + metainfo"
```

### Task 17: Flatpak manifest + offline sources

**Files:** Create `packaging/linux/io.github.ckir.rsahp.yml`, `packaging/linux/generate-sources.sh`. **Folds A5** (`skip:` list) + **F6** (pin the generator).

- [ ] **Step 0: Verify state.** `Cargo.lock` at repo root exists. Pinned runtime = `24.08` (or the current freedesktop stable at impl time — pin an explicit version, never "latest"; record it).

- [ ] **Step 1: Sources helper (F6: pin the generator, don't track master).** `packaging/linux/generate-sources.sh`:

```bash
#!/usr/bin/env bash
# Generates cargo-sources.json from Cargo.lock so flatpak-builder can build offline.
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
repo_root="$(cd "$here/../.." && pwd)"

# Pin flatpak-cargo-generator to a specific commit (NOT master) for reproducibility.
# Obtain the current HEAD SHA of flatpak/flatpak-builder-tools, pin it here, and record it.
GEN_REF="${FLATPAK_CARGO_GEN_REF:-PIN_A_COMMIT_SHA_HERE}"
gen="$here/flatpak-cargo-generator.py"
if [[ ! -f "$gen" ]]; then
  curl -fsSL -o "$gen" \
    "https://raw.githubusercontent.com/flatpak/flatpak-builder-tools/${GEN_REF}/cargo/flatpak-cargo-generator.py"
fi

python3 "$gen" "$repo_root/Cargo.lock" -o "$here/cargo-sources.json"
echo "wrote $here/cargo-sources.json (generator ref: $GEN_REF)"
```

Replace `PIN_A_COMMIT_SHA_HERE` with a real commit SHA at implementation time (this is external data to fetch, not a lazy placeholder — record the SHA in the commit message). `chmod +x packaging/linux/generate-sources.sh`. Add `packaging/linux/cargo-sources.json` and `packaging/linux/flatpak-cargo-generator.py` to `.gitignore`.

- [ ] **Step 2: Manifest (A5: `skip:` prevents copying target/.git/build outputs into the sandbox).** `packaging/linux/io.github.ckir.rsahp.yml`:

```yaml
app-id: io.github.ckir.rsahp
runtime: org.freedesktop.Platform
runtime-version: "24.08"
sdk: org.freedesktop.Sdk
sdk-extensions:
  - org.freedesktop.Sdk.Extension.rust-stable
command: rsahp-desktop

finish-args:
  - --share=network
  - --socket=wayland
  - --socket=fallback-x11
  - --device=dri

build-options:
  append-path: /usr/lib/sdk/rust-stable/bin
  env:
    CARGO_HOME: /run/build/rsahp/cargo

modules:
  - name: rsahp
    buildsystem: simple
    build-commands:
      - cargo --offline build --release -p rsahp-desktop
      - install -Dm755 target/release/rsahp-desktop /app/bin/rsahp-desktop
      - install -Dm644 packaging/linux/io.github.ckir.rsahp.desktop /app/share/applications/io.github.ckir.rsahp.desktop
      - install -Dm644 packaging/linux/io.github.ckir.rsahp.metainfo.xml /app/share/metainfo/io.github.ckir.rsahp.metainfo.xml
      - install -Dm644 packaging/linux/io.github.ckir.rsahp.png /app/share/icons/hicolor/256x256/apps/io.github.ckir.rsahp.png
    sources:
      - type: dir
        path: ../..
        skip:
          - target
          - .git
          - build-dir
          - repo
          - dist
          - .flatpak-builder
      - cargo-sources.json
```

Each `install -D...` MUST be a single physical line (verify). The `skip:` list is load-bearing — without it flatpak-builder copies `target/`, `.git/`, and its own outputs into the sandbox, bloating/recursing the build.

- [ ] **Step 3: Local build (Linux; flatpak + flatpak-builder + runtime).**

```bash
flatpak remote-add --if-not-exists --user flathub https://flathub.org/repo/flathub.flatpakrepo
flatpak install --user -y flathub org.freedesktop.Platform//24.08 org.freedesktop.Sdk//24.08 org.freedesktop.Sdk.Extension.rust-stable//24.08
./packaging/linux/generate-sources.sh
flatpak-builder --user --force-clean --install build-dir packaging/linux/io.github.ckir.rsahp.yml
flatpak run io.github.ckir.rsahp
```

Expected: offline build succeeds; app launches; login modal renders (backend reachable in-sandbox on the ephemeral loopback port); `~/.var/app/io.github.ckir.rsahp/data/rsahp/rsahp.db` persists across restarts.

- [ ] **Step 4: Bundle.**

```bash
flatpak-builder --user --force-clean --repo=repo build-dir packaging/linux/io.github.ckir.rsahp.yml
flatpak build-bundle repo dist/rsahp-0.1.0.flatpak io.github.ckir.rsahp
```

Expected: `dist/rsahp-0.1.0.flatpak`; `flatpak install --user dist/rsahp-0.1.0.flatpak` works.

- [ ] **Step 5: Commit.**

```bash
git add packaging/linux/io.github.ckir.rsahp.yml packaging/linux/generate-sources.sh .gitignore
git commit -m "feat(packaging): Flatpak manifest (skip list) + pinned offline sources generator"
```

### Task 18: Release workflow — Flatpak leg (gated)

**Files:** Modify `.github/workflows/release.yml`.

- [ ] **Step 0: Verify state.** `release.yml` has `test` + `windows-installer` jobs; no `flatpak-bundle` yet.

- [ ] **Step 1: Add the job (gated on `test`).**

```yaml
  flatpak-bundle:
    name: Flatpak bundle
    needs: test
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - name: Install flatpak toolchain
        run: |
          sudo apt-get update
          sudo apt-get install -y flatpak flatpak-builder python3 appstream
      - name: Add flathub + install runtime/SDK
        run: |
          flatpak remote-add --if-not-exists --user flathub https://flathub.org/repo/flathub.flatpakrepo
          flatpak install --user -y flathub org.freedesktop.Platform//24.08 org.freedesktop.Sdk//24.08 org.freedesktop.Sdk.Extension.rust-stable//24.08
      - name: Validate metainfo
        run: appstreamcli validate packaging/linux/io.github.ckir.rsahp.metainfo.xml
      - name: Generate offline cargo sources
        run: ./packaging/linux/generate-sources.sh
      - name: Build + bundle
        run: |
          mkdir -p dist
          flatpak-builder --user --force-clean --repo=repo build-dir packaging/linux/io.github.ckir.rsahp.yml
          VERSION=$(cargo metadata --format-version 1 --no-deps | python3 -c "import sys,json;print(next(p['version'] for p in json.load(sys.stdin)['packages'] if p['name']=='rsahp-desktop'))")
          flatpak build-bundle repo dist/rsahp-$VERSION.flatpak io.github.ckir.rsahp
      - name: Upload bundle artifact
        uses: actions/upload-artifact@v4
        with:
          name: rsahp-flatpak
          path: dist/rsahp-*.flatpak
          if-no-files-found: error
      - name: Attach to release
        if: startsWith(github.ref, 'refs/tags/')
        uses: softprops/action-gh-release@v2
        with:
          files: dist/rsahp-*.flatpak
```

Ensure `generate-sources.sh` has a pinned `GEN_REF` (or pass `FLATPAK_CARGO_GEN_REF` as an env in this step) before the tagged run.

- [ ] **Step 2: Validate YAML + commit.**

```bash
git add .github/workflows/release.yml
git commit -m "ci: add gated Flatpak build+bundle job"
```

### Task 19: Final integration gate

- [ ] **Step 1:** Run: `cargo build --workspace` && `cargo nextest run` && `cargo fmt --all -- --check` && `cargo clippy -- -D warnings`. Expected: all green (no stricter flags than the repo uses).
- [ ] **Step 2:** Run: `pwsh ./verify.ps1`. Expected: green (standalone backend contract intact end-to-end).
- [ ] **Step 3:** Confirm `release.yml` triggers only on `v*.*.*` and every packaging job `needs: test`. Do NOT push a tag — releasing is user-gated. Report: "Push a `vX.Y.Z` tag to produce both installers."
- [ ] **Step 4:** Dispatch a final code reviewer over the whole branch diff.

---

## Notes for the executor

- **Version source of truth:** installer/bundle version = `rsahp-desktop`'s package `version`. Extend the existing `version_bump` xtask action to ALSO bump `rsahp-desktop/Cargo.toml` AND the metainfo `<release>` (F10) if lockstep versioning is desired (optional; flagged, not required).
- **`use_gpu` recap:** default OFF; enabled by config `use_gpu: true` OR the dev-only `--enable-gpu` flag. The packaged wrapper has no CLI, so config is the only knob — the packaged-user GPU escape hatch.
- **Do not push tags or create GitHub Releases** without explicit user instruction.
- **Panel-folded high-severity items** (do not silently drop when implementing): percent-encoded DB path + spaced-path integration test (F2/A4); `fd-lock` not `fs4` (F3); create-dir→lock→seed ordering (A3); bounded-exit watchdog on the clean path ONLY (A2/R2-3); `save()` writes to the loaded path (F1); `drop(log_guard)` flush before error-path exits (R2-4); Flatpak `skip:` (A5); `cargo pkgid` version (A6); test-gated release (A7).
```
