use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use anyhow::{Context, Result};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    #[serde(default = "default_refresh_interval")]
    pub refresh_interval_secs: u64,

    #[serde(default = "default_shell")]
    pub shell: String,

    #[serde(default)]
    pub repos: Vec<RepoConfig>,

    #[serde(default)]
    pub keybindings: std::collections::HashMap<String, String>,
}

fn default_refresh_interval() -> u64 {
    30
}

fn default_shell() -> String {
    std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string())
}

impl Default for Config {
    fn default() -> Self {
        Self {
            refresh_interval_secs: default_refresh_interval(),
            shell: default_shell(),
            repos: vec![],
            keybindings: std::collections::HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RepoConfig {
    pub path: PathBuf,
    pub name: Option<String>,
}

pub fn load_config() -> Result<Config> {
    let config_path = config_path();

    if !config_path.exists() {
        tracing::info!("No config file found at {:?}, using defaults", config_path);
        return Ok(Config::default());
    }

    let content = std::fs::read_to_string(&config_path)
        .with_context(|| format!("Failed to read config from {:?}", config_path))?;

    let config: Config = toml::from_str(&content)
        .with_context(|| format!("Failed to parse config from {:?}", config_path))?;

    Ok(config)
}

pub fn config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("~/.config"))
        .join("gitopiary")
        .join("config.toml")
}

pub fn save_config(config: &Config) -> Result<()> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create config directory {:?}", parent))?;
    }
    let content = toml::to_string_pretty(config)
        .with_context(|| "Failed to serialize config")?;
    std::fs::write(&path, content)
        .with_context(|| format!("Failed to write config to {:?}", path))?;
    Ok(())
}
