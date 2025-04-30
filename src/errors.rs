use std::fmt;
use std::fmt::Formatter;

/// Error representing an unrecoverable error that will halt the application
/// 
#[derive(Debug)]
pub struct UnrecoverableError(pub String);
impl fmt::Display for UnrecoverableError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "UnrecoverableError: {}", self.0)
    }
}
impl From<ConfigError> for UnrecoverableError {
    fn from(e: ConfigError) -> Self {
        UnrecoverableError(e.to_string())
    }
}
impl From<std::io::Error> for UnrecoverableError {
    fn from(e: std::io::Error) -> Self {
        UnrecoverableError(e.to_string())
    }
}


/// Errors while managing configuration
/// 
pub struct ConfigError(pub String);

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "ConfigError: {}", self.0)
    }
}
impl From<std::io::Error> for ConfigError {
    fn from(e: std::io::Error) -> Self {
        ConfigError(e.to_string())
    }
}
impl From<toml::de::Error> for ConfigError {
    fn from(e: toml::de::Error) -> Self {
        ConfigError(e.to_string())
    }
}


/// Errors while managing tokens
/// 
#[derive(Debug)]
pub enum TokenError {
    RefreshTokenExpired,
    FileIO(String),
    Request(String),
}
impl fmt::Display for TokenError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            TokenError::RefreshTokenExpired => write!(f, "TokenError::RefreshTokenExpired"),
            TokenError::FileIO(e)   => write!(f, "TokenError::File: {}", e),
            TokenError::Request(e)  => write!(f, "TokenError::Request: {}", e),
        }
    }
}
impl From<std::io::Error> for TokenError {
    fn from(e: std::io::Error) -> Self {
        TokenError::FileIO(e.to_string())
    }
}
impl From<serde_json::Error> for TokenError {
    fn from(e: serde_json::Error) -> Self {
        TokenError::FileIO(e.to_string())
    }
}
impl From<reqwest::Error> for TokenError {
    fn from(e: reqwest::Error) -> Self {
        TokenError::Request(e.to_string())
    }
}


/// Errors from main sync loop
///
#[derive(Debug)]
pub enum CloudSyncError {
    OneDrive(String),
    AWS(String),
}
impl fmt::Display for CloudSyncError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            CloudSyncError::OneDrive(e) => write!(f, "SyncError::OneDrive: {}", e),
            CloudSyncError::AWS(e)      => write!(f, "SyncError::AWS: {}", e),
        }
    }
}
impl From<TokenError> for CloudSyncError {
    fn from(e: TokenError) -> Self { CloudSyncError::OneDrive(e.to_string()) }
}

/// Errors while managing OneDrive
///
pub struct OneDriveError(pub String);
impl fmt::Display for OneDriveError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "OneDriveError: {}", self.0)
    }
}
impl From<reqwest::Error> for OneDriveError {
    fn from(e: reqwest::Error) -> Self {
        OneDriveError(e.to_string())
    }
}
impl From<serde_json::Error> for OneDriveError {
    fn from(e: serde_json::Error) -> Self {
        OneDriveError(e.to_string())
    }
}
