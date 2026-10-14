//! Library error types for `clipvault-core`.

use std::path::PathBuf;

use thiserror::Error;

/// Errors returned by core library APIs.
#[derive(Debug, Error)]
pub enum CoreError {
    /// Configuration file is missing, unreadable, or invalid.
    #[error("config error: {0}")]
    Config(String),

    /// Failed to resolve or create application directories.
    #[error("path error for {path}: {message}")]
    Path { path: PathBuf, message: String },

    /// SQLite or migration failure.
    #[error("storage error: {0}")]
    Storage(String),

    /// Requested clipboard item was not found.
    #[error("item not found: {0}")]
    NotFound(i64),

    /// Input failed validation before touching storage.
    #[error("invalid input: {0}")]
    InvalidInput(String),
}

impl From<rusqlite::Error> for CoreError {
    fn from(value: rusqlite::Error) -> Self {
        CoreError::Storage(value.to_string())
    }
}

impl From<toml::de::Error> for CoreError {
    fn from(value: toml::de::Error) -> Self {
        CoreError::Config(value.to_string())
    }
}

impl From<std::io::Error> for CoreError {
    fn from(value: std::io::Error) -> Self {
        CoreError::Config(value.to_string())
    }
}

/// Convenient result alias for core operations.
pub type CoreResult<T> = Result<T, CoreError>;
