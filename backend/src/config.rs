// SPDX-License-Identifier: LicenseRef-PolyForm-Noncommercial-1.0.0
//! Module for application configuration and command line parsing.
//! This module handles loading configurations from a JSON file and
//! overriding them with command-line arguments.

use clap::Parser;
use serde::Deserialize;
use std::fs;

/// Command line arguments structure.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct CliArgs {
    /// Optional path to the configuration file.
    #[arg(short, long)]
    pub config: Option<String>,

    /// Optional database URL to connect to.
    #[arg(long)]
    pub database_url: Option<String>,

    /// Optional port number to listen on.
    #[arg(long)]
    pub port: Option<u16>,
}

/// Application configuration structure.
#[derive(Deserialize, Debug, Default)]
pub struct AppConfig {
    /// Database connection URL.
    pub database_url: Option<String>,
    /// Server listening port.
    pub port: Option<u16>,
}

/// Implementation block for `AppConfig`.
impl AppConfig {
    /// Loads the configuration by parsing command line arguments and reading the config file.
    pub fn load() -> Self {
        // Parse the command line arguments
        let cli = CliArgs::parse();

        // Initialize with default configuration
        let mut config = AppConfig::default();

        // Determine the configuration file path (defaulting to "config.json")
        let config_path = cli.config.unwrap_or_else(|| "config.json".to_string());
        // Try reading and parsing the configuration file
        if let Ok(content) = fs::read_to_string(&config_path)
            && let Ok(parsed) = serde_json::from_str::<AppConfig>(&content)
        {
            // If successful, update the config with the parsed values
            config = parsed;
        }

        // Override config with command-line database URL if provided
        if let Some(db) = cli.database_url {
            config.database_url = Some(db);
        }
        // Override config with command-line port if provided
        if let Some(p) = cli.port {
            config.port = Some(p);
        }

        // Return the final configuration
        config
    }
}
