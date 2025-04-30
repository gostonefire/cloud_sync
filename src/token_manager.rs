use std::fs;
use std::path::Path;
use chrono::{DateTime, Utc};
use log::warn;
use serde::{Deserialize, Serialize};
use crate::initialization::{Config, OneDrive};
use crate::errors::{CloudSyncError, TokenError};

#[derive(Deserialize)]
struct TokensImport {
    token_type: String,
    scope: String,
    expires_in: i64,
    ext_expires_in: i64,
    access_token: String,
    refresh_token: String,
}

#[derive(Serialize, Deserialize)]
pub struct Tokens {
    pub token_type: String,
    pub scope: String,
    pub expires_in: i64,
    pub ext_expires_in: i64,
    pub access_token: String,
    pub refresh_token: String,
    pub granted_at: DateTime<Utc>,
    pub refreshed_at: DateTime<Utc>,
}

impl Tokens {

    /// Creates a new Tokens instance given an OAuth2.0 code to be traded for tokens
    ///
    /// # Arguments
    ///
    /// * 'config' - configuration struct for OneDrive
    /// * 'code' - code from an initiated OAuth2.0 code flow
    pub async fn from_code(config: &OneDrive, code: &str) -> Result<Self, TokenError> {
        let body: [(&str, &str);6] = [
            ("client_id", &config.client_id),
            ("scope", &config.scope),
            ("code", code),
            ("redirect_uri", &config.redirect_uri),
            ("grant_type", "authorization_code"),
            ("client_secret", &config.client_secret),
        ];

        let client = reqwest::Client::new();
        let resp = client
            .post("https://login.microsoftonline.com/consumers/oauth2/v2.0/token")
            .header("Content-Type", "application/x-www-form-urlencoded")
            .form(&body)
            .send()
            .await?;

        let json = resp.text().await?;

        let import: TokensImport = serde_json::from_str(&json)?;
        let granted_at = Utc::now();

        let tokens = Tokens {
            token_type: import.token_type,
            scope: import.scope,
            expires_in: import.expires_in,
            ext_expires_in: import.ext_expires_in,
            access_token: import.access_token,
            refresh_token: import.refresh_token,
            granted_at,
            refreshed_at: granted_at,
        };
        
        tokens.save_tokens(&config.tokens_path).await?;
        
        Ok(tokens)
    }

    /// Creates a new Tokens instance from file. If the file is missing a warning is issued
    /// and the function tries again every 60 seconds.
    ///
    /// # Arguments
    /// 
    /// * 'tokens_path' - path to file holding tokens
    pub async fn from_file(tokens_path: &str) -> Result<Self, TokenError> {
        let path = Path::new(tokens_path);
        let mut file_warning_issued = false;
        loop {
            if path.exists() {
                let json = fs::read_to_string(tokens_path)?;
                let tokens: Tokens = serde_json::from_str(&json)?;

                return Ok(tokens);

            } else {
                if !file_warning_issued {   
                    warn!("token file missing, please authenticate/authorize cloud_sync");
                    file_warning_issued = true;
                }
                tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
                continue;
            }
        }
    }

    /// Returns the access token
    ///
    pub fn get_access_token(&self) -> String {
        self.access_token.clone()
    }

    /// Saves self as json to file
    ///
    /// # Arguments
    ///
    /// * 'tokens_path' - path to file holding tokens
    async fn save_tokens(&self, tokens_path: &str) -> Result<(), TokenError> {
        let json = serde_json::to_string(&self)?;
        fs::write(tokens_path, json)?;
        
        Ok(())
    }

    /// Removes the tokens file
    ///
    /// # Arguments
    ///
    /// * 'tokens_path' - path to file holding tokens
    async fn remove_tokens(&self, tokens_path: &str) -> Result<(), TokenError> {
        fs::remove_file(tokens_path)?;
        Ok(())
    }
    
    /// Checks if the access token needs to be refreshed
    ///
    pub fn is_expired(&self) -> bool {
        let age = (Utc::now() - self.refreshed_at).num_seconds();

        age > self.expires_in || age > 1800
    }

    /// Refreshes tokens using the refresh token
    ///
    /// # Arguments
    ///
    /// * 'config' - configuration struct for OneDrive
    pub async fn refresh_tokens(&mut self, config: &OneDrive) -> Result<(), TokenError> {
        let body: [(&str, &str);5] = [
            ("client_id", &config.client_id),
            ("scope", &config.scope),
            ("refresh_token", &self.refresh_token),
            ("grant_type", "refresh_token"),
            ("client_secret", &config.client_secret),
        ];

        let client = reqwest::Client::new();
        let resp = client
            .post("https://login.microsoftonline.com/consumers/oauth2/v2.0/token")
            .header("Content-Type", "application/x-www-form-urlencoded")
            .form(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            warn!("refresh token process failed, removing tokens file");
            self.remove_tokens(&config.tokens_path).await?;
            return Err(TokenError::RefreshTokenExpired);
        }
        
        let json = resp.text().await?;

        let import: TokensImport = serde_json::from_str(&json)?;

        self.token_type = import.token_type;
        self.scope = import.scope;
        self.expires_in = import.expires_in;
        self.ext_expires_in = import.ext_expires_in;
        self.access_token = import.access_token;
        self.refresh_token = import.refresh_token;
        self.refreshed_at = Utc::now();

        self.save_tokens(&config.tokens_path).await
    }
}

/// Returns a valid access token
///
/// # Arguments
///
/// * 'config' - configuration struct
pub async fn get_token(config: &Config) -> Result<String, CloudSyncError> {
    loop {
        let mut tokens = Tokens::from_file(&config.onedrive.tokens_path).await?;
        if tokens.is_expired() {
            match tokens.refresh_tokens(&config.onedrive).await {
                Ok(_) => (),
                Err(e) => {
                    match e {
                        TokenError::RefreshTokenExpired => continue,
                        _ => return Err(e.into()),
                    }
                }
            }
        }

        return Ok(tokens.get_access_token());
    }
}