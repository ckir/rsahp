use clap::Parser;
use serde::Deserialize;
use std::fs;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct CliArgs {
    #[arg(short, long)]
    pub config: Option<String>,

    #[arg(long)]
    pub database_url: Option<String>,

    #[arg(long)]
    pub port: Option<u16>,
}

#[derive(Deserialize, Debug, Default)]
pub struct AppConfig {
    pub database_url: Option<String>,
    pub port: Option<u16>,
}

impl AppConfig {
    pub fn load() -> Self {
        let cli = CliArgs::parse();

        let mut config = AppConfig::default();

        let config_path = cli.config.unwrap_or_else(|| "config.json".to_string());
        if let Ok(content) = fs::read_to_string(&config_path)
            && let Ok(parsed) = serde_json::from_str::<AppConfig>(&content)
        {
            config = parsed;
        }

        if let Some(db) = cli.database_url {
            config.database_url = Some(db);
        }
        if let Some(p) = cli.port {
            config.port = Some(p);
        }

        config
    }
}
