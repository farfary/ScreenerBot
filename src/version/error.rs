//! Errors produced by the version module.

use std::time::Duration;

use crate::errors::{DataError, ErrorClass, InternalError, IoError, NetworkError, Severity};

/// Everything that can go wrong while checking, downloading, or staging an update.
#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {
    /// Update transport failed before a response could be trusted.
    #[error(transparent)]
    Network(#[from] NetworkError),

    /// Update staging or state persistence encountered a filesystem failure.
    #[error(transparent)]
    Io(#[from] IoError),

    /// Update metadata could not be parsed or failed validation.
    #[error(transparent)]
    Data(#[from] DataError),

    /// Update orchestration encountered an unexpected task failure.
    #[error(transparent)]
    Internal(#[from] InternalError),

    /// A release-supplied URL did not meet the update allowlist.
    #[error("'{url}' is not a usable update URL: {reason}")]
    InvalidUpdateUrl { url: String, reason: String },

    /// The update endpoint rejected an otherwise valid check request.
    #[error("the update check failed with HTTP {status}")]
    UpdateCheckFailed { status: u16 },

    /// A digest or integrity verification did not agree with the trusted release metadata.
    #[error("the release digest did not verify: {detail}")]
    DigestMismatch { detail: String },

    /// No currently advertised update can be downloaded or installed.
    #[error("no update is currently available")]
    NoUpdateAvailable,

    /// The requested update no longer matches the update currently advertised.
    #[error("the requested update no longer matches the available release")]
    UpdateChanged,

    /// Another update download already owns the staging slot.
    #[error("an update download is already in progress")]
    DownloadInProgress,

    /// A download's actual byte count differed from its authenticated metadata.
    #[error("downloaded size mismatch: expected {expected}, got {actual}")]
    DownloadSizeMismatch { expected: u64, actual: u64 },

    /// The requested installer cannot run in the current application mode or platform.
    #[error("update installation is unsupported: {detail}")]
    UnsupportedInstall { detail: String },
}

/// Result alias for the version module.
pub type Result<T> = std::result::Result<T, Error>;

impl ErrorClass for Error {
    fn is_retryable(&self) -> bool {
        match self {
            Error::Network(error) => error.is_retryable(),
            Error::Io(error) => error.is_retryable(),
            Error::Data(error) => error.is_retryable(),
            Error::Internal(error) => error.is_retryable(),
            Error::UpdateCheckFailed { status } => *status == 429 || (500..600).contains(status),
            Error::InvalidUpdateUrl { .. }
            | Error::DigestMismatch { .. }
            | Error::NoUpdateAvailable
            | Error::UpdateChanged
            | Error::DownloadInProgress
            | Error::DownloadSizeMismatch { .. }
            | Error::UnsupportedInstall { .. } => false,
        }
    }

    fn retry_after(&self) -> Option<Duration> {
        match self {
            Error::Network(error) => error.retry_after(),
            Error::Io(error) => error.retry_after(),
            Error::Data(error) => error.retry_after(),
            Error::Internal(error) => error.retry_after(),
            Error::UpdateCheckFailed { .. } => Some(Duration::from_secs(2)),
            Error::InvalidUpdateUrl { .. }
            | Error::DigestMismatch { .. }
            | Error::NoUpdateAvailable
            | Error::UpdateChanged
            | Error::DownloadInProgress
            | Error::DownloadSizeMismatch { .. }
            | Error::UnsupportedInstall { .. } => None,
        }
    }

    fn severity(&self) -> Severity {
        match self {
            Error::Network(error) => error.severity(),
            Error::Io(error) => error.severity(),
            Error::Data(error) => error.severity(),
            Error::Internal(error) => error.severity(),
            Error::DigestMismatch { .. } => Severity::Critical,
            Error::UpdateCheckFailed { status } if (500..600).contains(status) => Severity::Warning,
            Error::InvalidUpdateUrl { .. }
            | Error::UpdateCheckFailed { .. }
            | Error::UpdateChanged
            | Error::DownloadSizeMismatch { .. }
            | Error::UnsupportedInstall { .. } => Severity::Error,
            Error::NoUpdateAvailable | Error::DownloadInProgress => Severity::Warning,
        }
    }

    fn http_status(&self) -> u16 {
        match self {
            Error::Network(error) => error.http_status(),
            Error::Io(error) => error.http_status(),
            Error::Data(error) => error.http_status(),
            Error::Internal(error) => error.http_status(),
            Error::InvalidUpdateUrl { .. } => 400,
            Error::UpdateCheckFailed { status } => *status,
            Error::DigestMismatch { .. } | Error::DownloadSizeMismatch { .. } => 422,
            Error::NoUpdateAvailable => 404,
            Error::UpdateChanged | Error::DownloadInProgress => 409,
            Error::UnsupportedInstall { .. } => 501,
        }
    }
}
