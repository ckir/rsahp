//! This module defines the configuration structures and parsing logic for the application.

use clap::Parser;
use serde::{Deserialize, Serialize};
use std::fs;

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

    /// Optional flag to force GPU usage.
    #[arg(long)]
    pub use_gpu: Option<bool>,
}

/// The main configuration structure for the application.
#[derive(Serialize, Deserialize, Debug)]
pub struct AppConfig {
    /// The URL of the API server.
    pub api_url: Option<String>,
    /// Whether to use GPU acceleration.
    pub use_gpu: Option<bool>,
    /// The UI zoom scale factor.
    pub zoom_scale: Option<f32>,
}

/// Implementation of the `Default` trait for `AppConfig`.
impl Default for AppConfig {
    /// Returns a default `AppConfig` instance.
    fn default() -> Self {
        Self {
            // Default API URL is None.
            api_url: None,
            // Default GPU usage is None.
            use_gpu: None,
            // Default zoom scale is 1.25.
            zoom_scale: Some(1.25),
        }
    }
}

/// Implementation of methods for `AppConfig`.
impl AppConfig {
    /// Loads the configuration from the file and overrides with CLI arguments.
    pub fn load() -> Self {
        // Parse the command-line arguments.
        let cli = CliArgs::parse();

        // Initialize with default configuration.
        let mut config = AppConfig::default();

        // Determine the configuration path, defaulting to "config.json".
        let config_path = cli.config.unwrap_or_else(|| "config.json".to_string());

        // Attempt to read and parse the configuration file.
        if let Ok(content) = fs::read_to_string(&config_path)
            && let Ok(parsed) = serde_json::from_str::<AppConfig>(&content)
        {
            // If successful, update the config with parsed values.
            config = parsed;
        }

        // Apply CLI overrides for API URL.
        if let Some(url) = cli.api_url {
            config.api_url = Some(url);
        }

        // Apply CLI overrides for GPU usage.
        if let Some(gpu) = cli.use_gpu {
            config.use_gpu = Some(gpu);
        }

        // Return the final configuration.
        config
    }

    /// Saves the current configuration to the configuration file.
    pub fn save(&self) {
        // Parse the command-line arguments to find the config path.
        let cli = CliArgs::parse();

        // Determine the configuration path, defaulting to "config.json".
        let config_path = cli.config.unwrap_or_else(|| "config.json".to_string());

        // Serialize the configuration to a pretty JSON string.
        if let Ok(content) = serde_json::to_string_pretty(self) {
            // Write the JSON string to the file, ignoring any errors.
            let _ = fs::write(config_path, content);
        }
    }
}
