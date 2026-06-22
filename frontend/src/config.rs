use clap::Parser;
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct CliArgs {
    #[arg(short, long)]
    pub config: Option<String>,

    #[arg(long)]
    pub api_url: Option<String>,

    #[arg(long)]
    pub use_gpu: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AppConfig {
    pub api_url: Option<String>,
    pub use_gpu: Option<bool>,
    pub zoom_scale: Option<f32>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            api_url: None,
            use_gpu: None,
            zoom_scale: Some(1.25),
        }
    }
}

impl AppConfig {
    pub fn load() -> Self {
        let cli = CliArgs::parse();
        
        let mut config = AppConfig::default();
        
        let config_path = cli.config.unwrap_or_else(|| "config.json".to_string());
        if let Ok(content) = fs::read_to_string(&config_path) {
            if let Ok(parsed) = serde_json::from_str::<AppConfig>(&content) {
                config = parsed;
            }
        }

        // CLI overrides
        if let Some(url) = cli.api_url {
            config.api_url = Some(url);
        }
        if let Some(gpu) = cli.use_gpu {
            config.use_gpu = Some(gpu);
        }

        config
    }

    pub fn save(&self) {
        let cli = CliArgs::parse();
        let config_path = cli.config.unwrap_or_else(|| "config.json".to_string());
        if let Ok(content) = serde_json::to_string_pretty(self) {
            let _ = fs::write(config_path, content);
        }
    }
}
