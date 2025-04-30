use std::{env, fs};
use serde::Deserialize;
use crate::errors::ConfigError;

#[derive(Deserialize, Clone)]
pub struct OneDrive {
    pub redirect_uri: String,
    pub client_id: String,
    pub client_secret: String,
    pub scope: String,
    pub tokens_path: String,
    pub drive_id: String,
}

#[derive(Deserialize)]
pub struct Config {
    pub onedrive: OneDrive,
}

/// Returns a configuration struct for the application and starts logging
/// 
pub fn config() -> Result<Config, ConfigError> {
    let config_dir = env::var("CONFIG_DIR")
        .expect("Error getting CONFIG_DIR");

    let log_path = format!("{}logging.yaml", config_dir);
    log4rs::init_file(log_path, Default::default())
        .map_err(|e| ConfigError(format!("Unable to start logging: {}", e.to_string())))?;
    
    load_config(&config_dir)
}


/// Loads the configuration file and returns a struct with all configuration items
///
/// # Arguments
///
/// * 'config_dir' - directory where to find configuration file
fn load_config(config_dir: &str) -> Result<Config, ConfigError> {
    let file_path = format!("{}config.toml", config_dir);

    let toml = fs::read_to_string(file_path)?;
    let config: Config = toml::from_str(&toml)?;

    Ok(config)
}