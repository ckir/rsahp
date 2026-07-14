//! The main entry point for the frontend application.

#![allow(warnings)]
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

use frontend::config::AppConfig;

/// The main function of the application.
fn main() -> Result<(), eframe::Error> {
    // Create a rolling file appender for daily log rotation.
    let file_appender = RollingFileAppender::new(Rotation::DAILY, "logs", "rsahp_frontend.log");

    // Create a non-blocking writer for the file appender.
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    // Initialize the tracing subscriber with EnvFilter and formatting layers.
    tracing_subscriber::registry()
        // Set the log filter level to "info".
        .with(EnvFilter::new("info"))
        // Add a JSON formatting layer writing to standard output.
        .with(fmt::layer().json().with_writer(std::io::stdout))
        // Add a JSON formatting layer writing to the log file.
        .with(fmt::layer().json().with_writer(non_blocking))
        .init();

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
