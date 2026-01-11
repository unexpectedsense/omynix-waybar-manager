use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub display: Display,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Display {
    pub preferred_monitor: String,
    pub available_monitors: Vec<String>,
    #[serde(default = "default_mode")]
    pub mode: String,  // "single" o "multiple"
}

fn default_mode() -> String {
    "multiple".to_string()
}

impl Default for Config {
    fn default() -> Self {
        Config {
            display: Display {
                preferred_monitor: "".to_string(),
                available_monitors: vec![],
                mode: "multiple".to_string(),
            },
        }
    }
}

pub fn get_config_path() -> Result<PathBuf> {
    let home = dirs::home_dir().context("The home directory could not be retrieved.")?;
    Ok(home.join(".local/share/omynix/waybar-manager/config.toml"))
}

pub fn init_config() -> Result<()> {
    let config_path = get_config_path()?;
    
    if config_path.exists() {
        println!("The configuration file already exists in: {}", config_path.display());
        return Ok(());
    }

    // Create directory if it does not exist
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)
            .context("The configuration directory could not be created")?;
    }

    let default_config = Config::default();
    save_config(&default_config)?;

    println!("Configuration file created in: {}", config_path.display());
    Ok(())
}

pub fn load_config() -> Result<Config> {
    let config_path = get_config_path()?;
    
    if !config_path.exists() {
        println!("No configuration file was found, creating a new one...");
        init_config()?;
    }

    let contents = fs::read_to_string(&config_path)
        .context("The configuration file could not be read")?;
    
    let config: Config = toml::from_str(&contents)
        .context("Error parsing configuration file")?;
    
    Ok(config)
}

pub fn save_config(config: &Config) -> Result<()> {
    let config_path = get_config_path()?;
    
    let toml_string = toml::to_string_pretty(config)
        .context("Error serializing configuration")?;
    
    fs::write(&config_path, toml_string)
        .context("Error writing configuration file")?;
    
    Ok(())
}
