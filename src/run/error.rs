// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Errors produced by the process run loop.

use std::time::Duration;

use crate::errors::{
    source_chain, ErrorClass, ServiceError, Severity, StartupError, StartupErrorCode,
};
use crate::i18n::{ids, UiArg, UiText};

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
                | StartupErrorCode::StorageUpgrade
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
                StartupErrorCode::StorageUpgrade | StartupErrorCode::Generic => 500,
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
            Error::Core { source } => match *source {
                crate::Error::Webserver(crate::webserver::Error::PortInUse { address }) => {
                    StartupError::new(
                        StartupErrorCode::PortInUse,
                        UiText::new(ids::STARTUP_PORT_IN_USE_TITLE),
                        UiText::new(ids::STARTUP_PORT_IN_USE_DETAIL)
                            .arg("address", UiArg::Text(address)),
                        UiText::new(ids::STARTUP_PORT_IN_USE_REMEDY),
                    )
                }
                crate::Error::Config(crate::config::Error::ParseFailed { detail }) => {
                    StartupError::new(
                        StartupErrorCode::ConfigInvalid,
                        UiText::new(ids::STARTUP_CONFIG_INVALID_TITLE),
                        UiText::new(ids::STARTUP_CONFIG_PARSE_DETAIL)
                            .arg("detail", UiArg::Text(detail)),
                        UiText::new(ids::STARTUP_CONFIG_PARSE_REMEDY),
                    )
                }
                source => match source.storage_upgrade_database() {
                    Some(database) => StartupError::storage_upgrade(
                        &database,
                        &source_chain(&Error::Core {
                            source: Box::new(source),
                        }),
                    ),
                    None => StartupError::generic_error(
                        source_chain(&Error::Core {
                            source: Box::new(source),
                        })
                        .join("\n"),
                    ),
                },
            },
            error => StartupError::generic_error(source_chain(&error).join("\n")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn core(source: crate::Error) -> Error {
        Error::Core {
            source: Box::new(source),
        }
    }

    /// Every typed upgrade refusal reaches the storage-upgrade screen with the store
    /// it belongs to and the refusal itself, not only the lifecycle wrapper.
    #[test]
    fn upgrade_refusals_name_their_store_and_cause() {
        let cases = [
            (
                crate::Error::Positions(crate::positions::Error::SchemaMigration {
                    detail: "unrecognized column positions.flag".to_owned(),
                }),
                "positions.db",
            ),
            (
                crate::Error::Positions(crate::positions::Error::SchemaTooNew {
                    stored: 7,
                    supported: 6,
                }),
                "positions.db",
            ),
            (
                crate::Error::Positions(crate::positions::Error::Database(
                    crate::errors::DatabaseError::Backup {
                        store: "positions.db".to_owned(),
                        message: "disk full".to_owned(),
                    },
                )),
                "positions.db",
            ),
            (
                crate::Error::Transactions(crate::transactions::Error::Migration {
                    step: "rename".to_owned(),
                    detail: "refused".to_owned(),
                }),
                "transactions.db",
            ),
            (
                crate::Error::Wallets(crate::wallets::Error::Migration {
                    step: "rebuild tables".to_owned(),
                    detail: "refused".to_owned(),
                }),
                "wallets.db, wallet.db",
            ),
            (
                crate::Error::Tools(crate::tools::Error::Migration {
                    step: "create table".to_owned(),
                    detail: "refused".to_owned(),
                }),
                "tools.db",
            ),
        ];
        for (source, database) in cases {
            let cause = source.to_string();
            let startup = StartupError::from(core(source));
            assert_eq!(startup.code, StartupErrorCode::StorageUpgrade, "{cause}");
            let detail = startup.detail.render_source_plain();
            assert!(detail.contains(database), "{detail}");
            assert!(detail.contains(&cause), "{detail}");
        }
    }

    #[test]
    fn a_generic_failure_shows_its_whole_cause_chain() {
        let source = crate::Error::Service(ServiceError::Start {
            service: "tokens".to_owned(),
            message: "orchestrator missing".to_owned(),
        });
        let cause = source.to_string();
        let startup = StartupError::from(core(source));
        assert_eq!(startup.code, StartupErrorCode::Generic);
        let detail = startup.detail.render_source_plain();
        assert!(detail.contains("service lifecycle failed"), "{detail}");
        assert!(detail.contains(&cause), "{detail}");
    }
}
