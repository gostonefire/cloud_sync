use std::{env, fs};
use std::path::PathBuf;
use serde::Deserialize;
use tokio::sync::mpsc::{UnboundedSender};
use crate::errors::ConfigError;
use crate::logging::setup_logger;

#[derive(Deserialize, Clone)]
pub struct OneDrive {
    pub redirect_uri: String,
    #[serde(default)]
    pub client_id: String,
    #[serde(default)]
    pub client_secret: String,
    pub scope: String,
    pub tokens_path: String,
    pub delta_link_path: String,
}

#[derive(Deserialize, Clone)]
pub struct AWS {
    #[serde(default)]
    access_key_id: String,
    #[serde(default)]
    secret_access_key: String,
    region: String,
    pub bucket: String,
}

#[derive(Deserialize)]
pub struct MailParameters {
    #[serde(default)]
    pub smtp_user: String,
    #[serde(default)]
    pub smtp_password: String,
    pub smtp_endpoint: String,
    pub from: String,
    pub to: String,
}

#[derive(Deserialize)]
pub struct WebServerParameters {
    pub bind_address: String,
    pub bind_port: u16,
}

    
#[derive(Deserialize, Clone)]
pub struct General {
    pub sync_time: String,
    pub log_path: String,
}

#[derive(Deserialize)]
pub struct Config {
    pub onedrive: OneDrive,
    pub aws: AWS,
    pub mail: MailParameters,
    pub web_server: WebServerParameters,
    pub general: General,
}

/// Returns a configuration struct for the application and starts logging
/// 
pub fn config(tx: UnboundedSender<String>) -> Result<Config, ConfigError> {
    let args: Vec<String> = env::args().collect();
    let config_path = args.iter()
        .find(|p| p.starts_with("--config="))
        .expect("config file argument should be present");
    let config_path = config_path
        .split_once('=')
        .expect("config file argument should be correct")
        .1;

    let mut config = load_config(&config_path)?;
    config.onedrive.client_id = read_credential("onedrive_client_id")?;
    config.onedrive.client_secret = read_credential("onedrive_client_secret")?;
    config.aws.access_key_id = read_credential("aws_access_key_id")?;
    config.aws.secret_access_key = read_credential("aws_secret_access_key")?;
    config.mail.smtp_user = read_credential("mail_smtp_user")?;
    config.mail.smtp_password = read_credential("mail_smtp_password")?;
    
    env::set_var("AWS_ACCESS_KEY_ID", &config.aws.access_key_id);
    env::set_var("AWS_SECRET_ACCESS_KEY", &config.aws.secret_access_key);
    env::set_var("AWS_REGION", &config.aws.region);
    
    setup_logger(&config.general.log_path, tx)?;
    
    Ok(config)
}

/// Loads the configuration file and returns a struct with all configuration items
///
/// # Arguments
///
/// * 'config_path' - path to the configuration file
pub fn load_config(config_path: &str) -> Result<Config, ConfigError> {
    let toml = fs::read_to_string(config_path)?;
    let config: Config = toml::from_str(&toml)?;

    Ok(config)
}

/// Reads a credential from the file system supported by the credstore and
/// given from systemd
///
/// # Arguments
///
/// * 'name' - name of the credential to read
fn read_credential(name: &str) -> Result<String, ConfigError> {
    let dir = env::var("CREDENTIALS_DIRECTORY")?;
    let mut p = PathBuf::from(dir);
    p.push(name);
    let bytes = fs::read(p)?;
    Ok(String::from_utf8(bytes)?.trim_end().to_string())
}


