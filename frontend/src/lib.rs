//! The main entry point for the frontend application.

#![allow(warnings)]
use eframe::egui;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

pub mod config;
pub mod ui;
