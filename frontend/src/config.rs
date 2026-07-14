//! This module defines the configuration structures and parsing logic for the application.

use clap::Parser;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Command-line arguments for the application.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct CliArgs {
    /// Optional path to a configuration file.
    #[arg(short, long)]
    pub config: Option<String>,

    /// Optional API URL override.
    #[arg(long)]
    pub api_url: Option<String>,

    /// Enable GPU hardware acceleration (OFF by default; equivalent to config `use_gpu: true`).
    #[arg(long)]
    pub enable_gpu: bool,
}

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
