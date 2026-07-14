// SPDX-License-Identifier: LicenseRef-PolyForm-Noncommercial-1.0.0
//! The main entry point for the frontend application.

#![allow(warnings)]
use eframe::egui;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

pub mod config;
pub mod ui;

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
        // Start maximized. Do NOT also set `with_inner_size` here: on eframe 0.34 / winit,
        // passing an explicit inner size alongside `with_maximized(true)` causes the
        // maximized flag to be dropped, and the window opens at the inner size instead
        // (verified via UI automation — it opened at ~1200x800 rather than maximized).
        // With only the maximized flag, the window manager assigns a native restore size
        // and eframe's window-state persistence still governs later launches.
        viewport: eframe::egui::ViewportBuilder::default().with_maximized(true),
        hardware_acceleration,
        ..Default::default()
    };

    eframe::run_native(
        "AHP Group Decision System",
        options,
        Box::new(move |_cc| Ok(Box::new(RsahpApp::new(config)))),
    )
}
