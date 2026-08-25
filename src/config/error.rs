//! Errors produced by the configuration module.

use std::time::Duration;

use crate::errors::{ConfigurationError, ErrorClass, IoError, Severity};

/// Everything that can go wrong while loading, validating, saving, or updating configuration.
#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {
    /// A cross-cutting configuration validation error occurred.
    #[error(transparent)]
    Configuration(#[from] ConfigurationError),

    /// A filesystem operation required by configuration failed.
    #[error(transparent)]
    Io(#[from] IoError),

    /// Configuration access was attempted before initial loading.
    #[error("the configuration is not loaded")]
    NotLoaded,

    /// A configuration file could not be parsed.
    #[error("config.toml could not be parsed: {detail}")]
    ParseFailed { detail: String },

    /// A configuration file could not be written.
    #[error("could not write config.toml: {detail}")]
    WriteFailed { detail: String },

    /// The configured wallet address could not be resolved.
    #[error("could not resolve the configured wallet address")]
    WalletAddress {
        #[source]
        source: crate::chains::solana::Error,
    },
}

/// Result alias for the configuration module.
pub type Result<T> = std::result::Result<T, Error>;

impl ErrorClass for Error {
    fn is_retryable(&self) -> bool {
        match self {
            Error::Configuration(e) => e.is_retryable(),
            Error::Io(e) => e.is_retryable(),
            Error::NotLoaded | Error::ParseFailed { .. } => false,
            Error::WriteFailed { .. } => true,
            Error::WalletAddress { source } => source.is_retryable(),
        }
    }

    fn retry_after(&self) -> Option<Duration> {
        match self {
            Error::Configuration(e) => e.retry_after(),
            Error::Io(e) => e.retry_after(),
            Error::NotLoaded | Error::ParseFailed { .. } => None,
            Error::WriteFailed { .. } => Some(Duration::from_secs(1)),
            Error::WalletAddress { source } => source.retry_after(),
        }
    }

    fn severity(&self) -> Severity {
        match self {
            Error::Configuration(e) => e.severity(),
            Error::Io(e) => e.severity(),
            Error::NotLoaded => Severity::Warning,
            Error::ParseFailed { .. } => Severity::Error,
            Error::WriteFailed { .. } => Severity::Critical,
            Error::WalletAddress { source } => source.severity(),
        }
    }

    fn http_status(&self) -> u16 {
        match self {
            Error::Configuration(e) => e.http_status(),
            Error::Io(e) => e.http_status(),
            Error::NotLoaded => 503,
            Error::ParseFailed { .. } => 400,
            Error::WriteFailed { .. } => 500,
            Error::WalletAddress { source } => source.http_status(),
        }
    }
}
