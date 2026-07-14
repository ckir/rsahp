//! Application entry point.
//! This module contains the main execution logic to start the server,
//! initialize tracing/logging, load configuration, and set up the database connection.

use sea_orm::DbErr;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

use backend::config;

/// Application main function.
/// It is asynchronous and uses the `tokio` runtime.
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

/// Listens for graceful shutdown signals (Ctrl+C, SIGTERM, or Windows-specific signals).
#[cfg(unix)]
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
    tracing::info!("Shutdown signal received, starting graceful shutdown...");
}

#[cfg(windows)]
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    let ctrl_close = async {
        tokio::signal::windows::ctrl_close()
            .expect("failed to install Ctrl+Close handler")
            .recv()
            .await;
    };

    let ctrl_break = async {
        tokio::signal::windows::ctrl_break()
            .expect("failed to install Ctrl+Break handler")
            .recv()
            .await;
    };

    let ctrl_shutdown = async {
        tokio::signal::windows::ctrl_shutdown()
            .expect("failed to install Ctrl+Shutdown handler")
            .recv()
            .await;
    };

    tokio::select! {
        _ = ctrl_c => {},
        _ = ctrl_close => {},
        _ = ctrl_break => {},
        _ = ctrl_shutdown => {},
    }
    tracing::info!("Shutdown signal received, starting graceful shutdown...");
}

#[cfg(not(any(unix, windows)))]
async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("failed to install Ctrl+C handler");
    tracing::info!("Shutdown signal received, starting graceful shutdown...");
}
