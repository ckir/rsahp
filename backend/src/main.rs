use axum::{routing::get, Router};
use sea_orm::{ConnectionTrait, Database, DbErr, Schema};
use std::net::SocketAddr;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use backend::{config, create_router, setup_schema};

#[tokio::main]
async fn main() -> Result<(), DbErr> {
    let config = config::AppConfig::load();
    let port = config.port.unwrap_or(3001);
    let db_url = config.database_url.unwrap_or_else(|| "sqlite://rsahp.db?mode=rwc".to_string());

    // Note on "rotational log files 10mb/date":
    // tracing_appender natively supports date-based rotation (Rotation::DAILY).
    let file_appender = RollingFileAppender::new(Rotation::DAILY, "logs", "rsahp_backend.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::registry()
        .with(EnvFilter::new("info"))
        .with(fmt::layer().with_writer(std::io::stdout))
        .with(fmt::layer().with_writer(non_blocking))
        .init();

    tracing::info!("Starting AHP Backend Server...");

    // Setup DB
    let db = Database::connect(&db_url).await?;
    tracing::info!("Connected to database: {}", db_url);
    setup_schema(&db).await?;

    let app = create_router(db);

    // --- Start Server ---
    let bind_addr = format!("127.0.0.1:{}", port);
    let listener = tokio::net::TcpListener::bind(&bind_addr).await.unwrap();
    tracing::info!("Listening on {}", bind_addr);
    axum::serve(listener, app).await.unwrap();

    Ok(())
}

