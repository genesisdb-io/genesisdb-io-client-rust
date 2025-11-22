//! Error types for the Genesis DB client

use thiserror::Error;

/// Result type for Genesis DB client operations
pub type Result<T> = std::result::Result<T, Error>;

/// Errors that can occur when using the Genesis DB client
#[derive(Error, Debug)]
pub enum Error {
    /// Missing required configuration
    #[error("Missing required configuration: {0}")]
    MissingConfig(String),

    /// API error from Genesis DB server
    #[error("API Error: {status} {status_text}")]
    ApiError {
        status: u16,
        status_text: String,
    },

    /// HTTP request error
    #[error("HTTP request error: {0}")]
    RequestError(#[from] reqwest::Error),

    /// JSON serialization/deserialization error
    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),

    /// Invalid response from server
    #[error("Invalid response: {0}")]
    InvalidResponse(String),

    /// Environment variable error
    #[error("Environment variable error: {0}")]
    EnvError(String),
}
