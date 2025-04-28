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

#[derive(Debug)]
pub enum TokenError {
    File(String),
    Request(String),
}
impl fmt::Display for TokenError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            TokenError::File(e) => write!(f, "TokenError::File: {}", e),
            TokenError::Request(e) => write!(f, "TokenError::Request: {}", e),
        }
    }
}
impl From<std::io::Error> for TokenError {
    fn from(e: std::io::Error) -> Self {
        TokenError::File(e.to_string())
    }
}
impl From<serde_json::Error> for TokenError {
    fn from(e: serde_json::Error) -> Self {
        TokenError::File(e.to_string())
    }
}
impl From<reqwest::Error> for TokenError {
    fn from(e: reqwest::Error) -> Self {
        TokenError::Request(e.to_string())
    }
}
