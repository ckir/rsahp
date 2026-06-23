//! The main entry point for the frontend application.

#![allow(warnings)]
use eframe::egui;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

use frontend::config::AppConfig;
use frontend::ui::RsahpApp;

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
    let (config, cli_args) = AppConfig::load();

    // Determine the hardware acceleration preference based on the CLI flag.
    let hardware_acceleration = if cli_args.disable_gpu {
        eframe::HardwareAcceleration::Off
    } else {
        eframe::HardwareAcceleration::Preferred
    };

    // Configure the native options for the eframe application window.
    let options = eframe::NativeOptions {
        // Build the viewport with initial size and maximized state.
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_maximized(true),
        // Set the hardware acceleration option.
        hardware_acceleration,
        ..Default::default()
    };

    // Run the native eframe application with the configured options and UI instance.
    eframe::run_native(
        "AHP Group Decision System",
        options,
        Box::new(move |_cc| Ok(Box::new(RsahpApp::new(config)))),
    )
}
