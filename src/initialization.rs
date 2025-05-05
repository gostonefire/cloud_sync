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
    pub delta_link_path: String,
}

#[derive(Deserialize, Clone)]
pub struct AWS {
    access_key_id: String,
    secret_access_key: String,
    region: String,
    pub bucket: String,
}

#[derive(Deserialize, Clone)]
pub struct General {
    pub sync_time: String,
}

#[derive(Deserialize)]
pub struct Config {
    pub onedrive: OneDrive,
    pub aws: AWS,
    pub general: General,
}

/// Returns a configuration struct for the application and starts logging
/// 
pub fn config() -> Result<Config, ConfigError> {
    let config_dir = env::var("CONFIG_DIR")
        .expect("Error getting CONFIG_DIR");

    let log_path = format!("{}logging.yaml", config_dir);
    log4rs::init_file(log_path, Default::default())
        .map_err(|e| ConfigError(format!("Unable to start logging: {}", e.to_string())))?;
    
    let config = load_config(&config_dir)?;
    
    env::set_var("AWS_ACCESS_KEY_ID", &config.aws.access_key_id);
    env::set_var("AWS_SECRET_ACCESS_KEY", &config.aws.secret_access_key);
    env::set_var("AWS_REGION", &config.aws.region);
    
    Ok(config)
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