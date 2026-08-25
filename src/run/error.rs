//! Errors produced by the process run loop.

use std::time::Duration;

use crate::errors::{ErrorClass, ServiceError, Severity, StartupError, StartupErrorCode};

/// Everything that can go wrong while starting or stopping the process lifecycle.
#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {
    /// A process-level failure already has a user-facing startup diagnosis.
    #[error(transparent)]
    Startup(#[from] StartupError),

    /// Initializing, starting, or stopping the service manager failed.
    #[error(transparent)]
    Service(#[from] ServiceError),

    /// A service-layer operation returned the top-level composed error.
    #[error("service lifecycle failed")]
    Core {
        #[source]
        source: Box<crate::Error>,
    },

    /// The global service-manager handle was unavailable for an operation.
    #[error("service manager was unavailable while attempting to {operation}")]
    ServiceManagerUnavailable { operation: &'static str },

    /// The service manager was unexpectedly unavailable after being taken.
    #[error("service manager was already taken while attempting to {operation}")]
    ServiceManagerTaken { operation: &'static str },

    /// The setup screen remained unresolved past the startup deadline.
    #[error("setup timed out after {minutes} minutes")]
    SetupTimedOut { minutes: u64 },

    /// Shutdown was requested before setup selected an operational mode.
    #[error("shutdown requested during initialization")]
    ShutdownDuringInitialization,

    /// Registering an operating-system shutdown signal failed.
    #[error("could not bind {signal} shutdown signal: {detail}")]
    SignalBinding {
        signal: &'static str,
        detail: String,
    },
}

/// Result alias for the process run loop.
pub type Result<T> = std::result::Result<T, Error>;

impl ErrorClass for Error {
    fn is_retryable(&self) -> bool {
        match self {
            Error::Startup(_) => false,
            Error::Service(error) => error.is_retryable(),
            Error::Core { source } => source.is_retryable(),
            Error::ServiceManagerUnavailable { .. }
            | Error::ServiceManagerTaken { .. }
            | Error::SetupTimedOut { .. }
            | Error::ShutdownDuringInitialization
            | Error::SignalBinding { .. } => false,
        }
    }

    fn retry_after(&self) -> Option<Duration> {
        match self {
            Error::Startup(_) => None,
            Error::Service(error) => error.retry_after(),
            Error::Core { source } => source.retry_after(),
            Error::ServiceManagerUnavailable { .. }
            | Error::ServiceManagerTaken { .. }
            | Error::SetupTimedOut { .. }
            | Error::ShutdownDuringInitialization
            | Error::SignalBinding { .. } => None,
        }
    }

    fn severity(&self) -> Severity {
        match self {
            Error::Startup(startup) => match startup.code {
                StartupErrorCode::WalletMismatch => Severity::Warning,
                StartupErrorCode::PortInUse
                | StartupErrorCode::LockHeld
                | StartupErrorCode::ConfigInvalid
                | StartupErrorCode::Generic => Severity::Error,
            },
            Error::Service(error) => error.severity(),
            Error::Core { source } => source.severity(),
            Error::ServiceManagerUnavailable { .. }
            | Error::ServiceManagerTaken { .. }
            | Error::SignalBinding { .. } => Severity::Critical,
            Error::SetupTimedOut { .. } | Error::ShutdownDuringInitialization => Severity::Warning,
        }
    }

    fn http_status(&self) -> u16 {
        match self {
            Error::Startup(startup) => match startup.code {
                StartupErrorCode::WalletMismatch | StartupErrorCode::ConfigInvalid => 400,
                StartupErrorCode::PortInUse | StartupErrorCode::LockHeld => 409,
                StartupErrorCode::Generic => 500,
            },
            Error::Service(error) => error.http_status(),
            Error::Core { source } => source.http_status(),
            Error::ServiceManagerUnavailable { .. }
            | Error::ServiceManagerTaken { .. }
            | Error::SignalBinding { .. } => 500,
            Error::SetupTimedOut { .. } => 504,
            Error::ShutdownDuringInitialization => 503,
        }
    }
}

impl From<Error> for StartupError {
    fn from(error: Error) -> Self {
        match error {
            Error::Startup(error) => error,
            error => StartupError::generic(error.to_string()),
        }
    }
}
