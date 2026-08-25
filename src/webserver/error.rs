//! Errors produced by the webserver module.

use std::time::Duration;

use crate::errors::{ErrorClass, IoError, Severity};

/// Everything that can go wrong while serving the dashboard.
#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {
    /// A filesystem operation required by the dashboard failed.
    #[error(transparent)]
    Io(#[from] IoError),

    /// Loading, validating, or saving configuration failed.
    #[error(transparent)]
    Config(#[from] crate::config::Error),

    /// The dashboard server could not bind its listening address.
    #[error("could not bind the dashboard server to {address}: {detail}")]
    Bind { address: String, detail: String },

    /// The configured port is already bound by another process. Split from
    /// `Bind` because it is the one bind failure with a specific user remedy,
    /// and it must be selectable without reading the message text.
    #[error("the dashboard port {address} is already in use")]
    PortInUse { address: String },

    /// A TOTP code did not verify.
    #[error("the submitted authentication code is not valid")]
    InvalidTotpCode,

    /// A TOTP secret could not be decoded, created, or verified.
    #[error("could not {operation} the TOTP secret: {detail}")]
    TotpSecret {
        operation: &'static str,
        detail: String,
    },

    /// A TOTP QR code could not be rendered.
    #[error("could not generate the TOTP QR code: {detail}")]
    TotpQrCode { detail: String },

    /// An imported configuration payload could not be accepted.
    #[error("the imported configuration is not valid: {detail}")]
    InvalidImport { detail: String },

    /// A configuration section name is not recognized.
    #[error("'{key}' is not a known configuration key")]
    UnknownConfigKey { key: String },

    /// A UI-state store could not be serialized or persisted.
    #[error("the UI-state store is not valid: {detail}")]
    InvalidUiState { detail: String },

    /// A lockscreen password does not match its selected format.
    #[error("the lockscreen password is not valid: {detail}")]
    InvalidLockscreenPassword { detail: String },

    /// Initialization credentials no longer match a successful validation receipt.
    #[error("the initialization setup is not valid: {detail}")]
    InvalidInitialization { detail: String },

    /// The service manager cannot start the dashboard's remaining services.
    #[error("could not start remaining services: {detail}")]
    ServiceStartup { detail: String },

    /// A third-party dashboard feed could not be fetched or decoded.
    #[error("{detail}")]
    ExternalFeed { detail: String },

    /// A dashboard discovery board could not be loaded from an API client.
    #[error("{operation} failed: {source}")]
    Api {
        operation: &'static str,
        #[source]
        source: crate::apis::Error,
    },

    /// A position-detail key was not a valid position identifier.
    #[error("{detail}")]
    InvalidPositionKey { detail: String },

    /// A stored strategy template could not be read or decoded.
    #[error("{detail}")]
    TemplateDecode { detail: String },
}

/// Result alias for the webserver module.
pub type Result<T> = std::result::Result<T, Error>;

impl ErrorClass for Error {
    fn is_retryable(&self) -> bool {
        match self {
            Error::Io(e) => e.is_retryable(),
            Error::Config(e) => e.is_retryable(),
            Error::Bind { .. }
            | Error::PortInUse { .. }
            | Error::InvalidTotpCode
            | Error::InvalidImport { .. }
            | Error::UnknownConfigKey { .. }
            | Error::InvalidUiState { .. }
            | Error::InvalidLockscreenPassword { .. } => false,
            Error::InvalidInitialization { .. } => false,
            Error::TotpSecret { .. }
            | Error::TotpQrCode { .. }
            | Error::ServiceStartup { .. }
            | Error::ExternalFeed { .. }
            | Error::Api { .. } => true,
            Error::InvalidPositionKey { .. } | Error::TemplateDecode { .. } => false,
        }
    }

    fn retry_after(&self) -> Option<Duration> {
        match self {
            Error::Io(e) => e.retry_after(),
            Error::Config(e) => e.retry_after(),
            Error::TotpSecret { .. } | Error::TotpQrCode { .. } => Some(Duration::from_secs(1)),
            Error::ServiceStartup { .. } => Some(Duration::from_secs(5)),
            Error::ExternalFeed { .. } | Error::Api { .. } => Some(Duration::from_secs(1)),
            Error::Bind { .. }
            | Error::PortInUse { .. }
            | Error::InvalidTotpCode
            | Error::InvalidImport { .. }
            | Error::UnknownConfigKey { .. }
            | Error::InvalidUiState { .. }
            | Error::InvalidLockscreenPassword { .. }
            | Error::InvalidPositionKey { .. }
            | Error::TemplateDecode { .. } => None,
            Error::InvalidInitialization { .. } => None,
        }
    }

    fn severity(&self) -> Severity {
        match self {
            Error::Io(e) => e.severity(),
            Error::Config(e) => e.severity(),
            Error::Bind { .. }
            | Error::PortInUse { .. }
            | Error::TotpSecret { .. }
            | Error::TotpQrCode { .. }
            | Error::ServiceStartup { .. }
            | Error::ExternalFeed { .. }
            | Error::Api { .. }
            | Error::TemplateDecode { .. } => Severity::Error,
            Error::InvalidTotpCode
            | Error::InvalidImport { .. }
            | Error::UnknownConfigKey { .. }
            | Error::InvalidUiState { .. }
            | Error::InvalidLockscreenPassword { .. }
            | Error::InvalidPositionKey { .. } => Severity::Warning,
            Error::InvalidInitialization { .. } => Severity::Warning,
        }
    }

    fn http_status(&self) -> u16 {
        match self {
            Error::Io(e) => e.http_status(),
            Error::Config(e) => e.http_status(),
            Error::InvalidTotpCode => 401,
            Error::InvalidImport { .. }
            | Error::UnknownConfigKey { .. }
            | Error::InvalidUiState { .. }
            | Error::InvalidLockscreenPassword { .. }
            | Error::InvalidPositionKey { .. } => 400,
            Error::InvalidInitialization { .. } => 400,
            Error::PortInUse { .. } => 503,
            Error::Bind { .. }
            | Error::TotpSecret { .. }
            | Error::TotpQrCode { .. }
            | Error::ServiceStartup { .. }
            | Error::ExternalFeed { .. }
            | Error::Api { .. }
            | Error::TemplateDecode { .. } => 500,
        }
    }
}
