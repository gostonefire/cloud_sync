use std::fmt;
use std::fmt::Formatter;
use aws_sdk_s3::operation::put_object::PutObjectError;
use aws_sdk_s3::config::http::HttpResponse;
use aws_sdk_s3::operation::complete_multipart_upload::CompleteMultipartUploadError;
use aws_sdk_s3::operation::create_multipart_upload::CreateMultipartUploadError;
use aws_sdk_s3::operation::head_object::HeadObjectError;
use aws_sdk_s3::operation::list_objects_v2::ListObjectsV2Error;
use aws_sdk_s3::operation::upload_part::UploadPartError;
use aws_smithy_runtime_api::client::result::SdkError;
use log4rs::config::runtime::ConfigErrors;
use log::SetLoggerError;
use reqwest::header::ToStrError;

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
impl From<rustls_pki_types::pem::Error> for UnrecoverableError {
    fn from(e: rustls_pki_types::pem::Error) -> Self { UnrecoverableError(e.to_string()) }
}
impl From<rustls::Error> for UnrecoverableError {
    fn from(e: rustls::Error) -> Self { UnrecoverableError(e.to_string()) }
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
impl From<SetLoggerError> for ConfigError {
    fn from(e: SetLoggerError) -> Self {
        ConfigError(e.to_string())
    }
}
impl From<ConfigErrors> for ConfigError {
    fn from(e: ConfigErrors) -> Self {
        ConfigError(e.to_string())
    }
}


/// Errors while managing tokens
/// 
#[derive(Debug)]
pub enum TokenError {
    NoTokensFile,
    RefreshTokenExpired,
    FileIO(String),
    Request(String),
}
impl fmt::Display for TokenError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            TokenError::NoTokensFile        => write!(f, "TokenError::NoTokensFile"),
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
    TokenExpiredWarning,
    TokenError(String),
    OneDrive(String),
    AWS(String),
}
impl fmt::Display for CloudSyncError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            CloudSyncError::TokenExpiredWarning   => write!(f, "CloudSyncError::TokenExpiredWarning"),
            CloudSyncError::TokenError(e) => write!(f, "CloudSyncError::TokenError: {}", e),
            CloudSyncError::OneDrive(e)   => write!(f, "CloudSyncError::OneDrive: {}", e),
            CloudSyncError::AWS(e)        => write!(f, "CloudSyncError::AWS: {}", e),
        }
    }
}
impl From<TokenError> for CloudSyncError {
    fn from(e: TokenError) -> Self {
        match e {
            TokenError::NoTokensFile => { CloudSyncError::TokenExpiredWarning }
            TokenError::RefreshTokenExpired => { CloudSyncError::TokenExpiredWarning }
            TokenError::FileIO(err) => { CloudSyncError::TokenError(err) }
            TokenError::Request(err) => { CloudSyncError::TokenError(err) }
        }
    }
}
impl From<OneDriveError> for CloudSyncError {
    fn from(e: OneDriveError) -> Self { CloudSyncError::OneDrive(e.to_string()) }
}
impl From<AWSError> for CloudSyncError {
    fn from(e: AWSError) -> Self { CloudSyncError::AWS(e.to_string()) }
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
impl From<ToStrError> for OneDriveError {
    fn from(e: ToStrError) -> Self {
        OneDriveError(e.to_string())
    }
}
impl From<serde_json::Error> for OneDriveError {
    fn from(e: serde_json::Error) -> Self {
        OneDriveError(e.to_string())
    }
}
impl From<std::io::Error> for OneDriveError {
    fn from(e: std::io::Error) -> Self {
        OneDriveError(e.to_string())
    }
}


/// Errors while managing AWS
///
pub struct AWSError(pub String);
impl fmt::Display for AWSError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "AWSError: {}", self.0)
    }
}
impl From<&str> for AWSError {
    fn from(e: &str) -> Self { AWSError(e.to_string()) }
}
impl From<aws_sdk_s3::Error> for AWSError {
    fn from(e: aws_sdk_s3::Error) -> Self { AWSError(e.to_string()) }
}
impl From<SdkError<PutObjectError, HttpResponse>> for AWSError {
    fn from(e: SdkError<PutObjectError, HttpResponse>) -> Self { AWSError(e.to_string()) }
}
impl From<SdkError<HeadObjectError, HttpResponse>> for AWSError {
    fn from(e: SdkError<HeadObjectError, HttpResponse>) -> Self { AWSError(e.to_string()) }
}
impl From<SdkError<ListObjectsV2Error, HttpResponse>> for AWSError {
    fn from(e: SdkError<ListObjectsV2Error, HttpResponse>) -> Self { AWSError(e.to_string()) }
}
impl From<SdkError<CreateMultipartUploadError, HttpResponse>> for AWSError {
    fn from(e: SdkError<CreateMultipartUploadError, HttpResponse>) -> Self { AWSError(e.to_string()) }
}
impl From<SdkError<UploadPartError, HttpResponse>> for AWSError {
    fn from(e: SdkError<UploadPartError, HttpResponse>) -> Self { AWSError(e.to_string()) }
}
impl From<SdkError<CompleteMultipartUploadError, HttpResponse>> for AWSError {
    fn from(e: SdkError<CompleteMultipartUploadError, HttpResponse>) -> Self { AWSError(e.to_string()) }
}

/// Errors while managing mail
/// 
pub enum MailError {
    InvalidEmailAddress(String),
    Document(String),
    SendgridError(String),
}

impl fmt::Display for MailError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            MailError::InvalidEmailAddress(e) => write!(f, "MailError::InvalidEmailAddress: {}", e),
            MailError::Document(e)            => write!(f, "MailError::Document: {}", e),
            MailError::SendgridError(e)       => write!(f, "MailError::SendgridError: {}", e),
        }
    }
}
impl From<serde_json::Error> for MailError {
    fn from(e: serde_json::Error) -> Self { MailError::Document(e.to_string()) }
}
impl From<reqwest::Error> for MailError {
    fn from(e: reqwest::Error) -> Self { MailError::SendgridError(e.to_string()) }
}