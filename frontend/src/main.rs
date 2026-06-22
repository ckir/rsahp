#![allow(warnings)]
use eframe::egui;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

mod config;
mod ui;

fn main() -> Result<(), eframe::Error> {
    let file_appender = RollingFileAppender::new(Rotation::DAILY, "logs", "rsahp_frontend.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::registry()
        .with(EnvFilter::new("info"))
        .with(fmt::layer().with_writer(std::io::stdout))
        .with(fmt::layer().with_writer(non_blocking))
        .init();

    let config = config::AppConfig::load();
    let use_gpu = config.use_gpu.unwrap_or(false);

    let hardware_acceleration = if use_gpu {
        eframe::HardwareAcceleration::Preferred
    } else {
        eframe::HardwareAcceleration::Off
    };

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_maximized(true),
        hardware_acceleration,
        ..Default::default()
    };

    eframe::run_native(
        "AHP Group Decision System",
        options,
        Box::new(move |_cc| Ok(Box::new(ui::RsahpApp::new(config)))),
    )
}
