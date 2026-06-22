//! Application entry point.
//! This module contains the main execution logic to start the server,
//! initialize tracing/logging, load configuration, and set up the database connection.

use sea_orm::{Database, DbErr};
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

use backend::{config, create_router, setup_schema};

/// Application main function.
/// It is asynchronous and uses the `tokio` runtime.
#[tokio::main]
async fn main() -> Result<(), DbErr> {
    // Load application configuration
    let config = config::AppConfig::load();
    // Resolve port to use
    let port = config.port.unwrap_or(3001);
    // Resolve database URL to connect to
    let db_url = config
        .database_url
        .unwrap_or_else(|| "sqlite://rsahp.db?mode=rwc".to_string());

    // Note on "rotational log files 10mb/date":
    // tracing_appender natively supports date-based rotation (Rotation::DAILY).
    // Set up a rolling file appender for logs
    let file_appender = RollingFileAppender::new(Rotation::DAILY, "logs", "rsahp_backend.log");
    // Ensure file logging happens non-blocking
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    // Initialize the tracing subscriber registry
    tracing_subscriber::registry()
        // Configure logging level via environment filter
        .with(EnvFilter::new("info"))
        // Output logs to stdout in JSON format
        .with(fmt::layer().json().with_writer(std::io::stdout))
        // Output logs to file in JSON format
        .with(fmt::layer().json().with_writer(non_blocking))
        .init();

    // Log the server startup
    tracing::info!("Starting AHP Backend Server...");

    // Setup DB connection
    let db = Database::connect(&db_url).await?;
    // Log successful database connection
    tracing::info!("Connected to database: {}", db_url);
    // Setup and migrate the database schema
    setup_schema(&db).await?;

    // Create the Axum application router
    let app = create_router(db);

    // --- Start Server ---
    // Define the bind address
    let bind_addr = format!("127.0.0.1:{}", port);
    // Bind a TCP listener
    let listener = tokio::net::TcpListener::bind(&bind_addr).await.unwrap();
    // Log the bound address
    tracing::info!("Listening on {}", bind_addr);
    // Serve the Axum application
    axum::serve(listener, app).await.unwrap();

    // Return success
    Ok(())
}
