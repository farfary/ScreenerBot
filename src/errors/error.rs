//! Top-level Error enum — unifies all domain-specific error types.

use super::{
    AccountError, ConfigurationError, DataError, DatabaseError, InternalError, IoError,
    NetworkError, RpcProviderError, ServiceError,
};
use crate::rpc::errors::RpcError;

/// Top-level error type for ScreenerBot.
///
/// This is re-exported as `crate::Error` for ergonomic usage across the codebase.
#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {
    /// AI evaluation, chat, tool execution and scheduled automation errors.
    #[error(transparent)]
    Ai(#[from] crate::ai::Error),

    /// Position lifecycle errors (open/close/DCA/verification).
    #[error(transparent)]
    Positions(#[from] crate::positions::Error),

    /// Transaction processing, persistence and verification errors.
    #[error(transparent)]
    Transactions(#[from] crate::transactions::Error),

    /// Trader control, execution and copy-trading errors.
    #[error(transparent)]
    Trader(#[from] crate::trader::Error),

    /// Wallet storage, watch-target and balance-monitor errors.
    #[error(transparent)]
    Wallets(#[from] crate::wallets::Error),

    /// Tool session errors: ATA cleanup, multi-wallet sessions, favorites, trade watching.
    #[error(transparent)]
    Tools(#[from] crate::tools::Error),

    /// Chain-neutral identity/execution errors.
    #[error(transparent)]
    Chains(#[from] crate::chains::Error),

    /// Solana chain adapter errors.
    #[error(transparent)]
    Solana(#[from] crate::chains::solana::Error),

    /// External API client errors (HTTP data sources, LLM providers).
    #[error(transparent)]
    Apis(#[from] crate::apis::Error),

    /// Pool pricing, persistence and blacklist errors.
    #[error(transparent)]
    Pools(#[from] crate::pools::Error),

    /// Telegram bot lifecycle, notification, session and command errors.
    #[error(transparent)]
    Telegram(#[from] crate::telegram::Error),

    /// Filtering snapshot and token-query errors.
    #[error(transparent)]
    Filtering(#[from] crate::filtering::Error),

    /// Action progress persistence and lifecycle errors.
    #[error(transparent)]
    Actions(#[from] crate::actions::Error),

    /// Strategy condition validation and evaluation errors.
    #[error(transparent)]
    Strategies(#[from] crate::strategies::Error),

    /// Persistent event recording, querying and maintenance errors.
    #[error(transparent)]
    Events(#[from] crate::events::Error),

    /// Application update checking, download, and installer staging errors.
    #[error(transparent)]
    Version(#[from] crate::version::Error),

    /// Configuration loading, validation, persistence, and hot-update errors.
    #[error(transparent)]
    Config(#[from] crate::config::Error),

    /// Dashboard server lifecycle, authentication, and route-input errors.
    #[error(transparent)]
    Webserver(#[from] crate::webserver::Error),

    /// Runtime reset operations.
    #[error(transparent)]
    Reset(#[from] crate::reset::Error),

    /// Local encryption and password-hashing operations.
    #[error(transparent)]
    SecureStorage(#[from] crate::secure_storage::Error),

    /// Process lifecycle and service-manager orchestration.
    #[error(transparent)]
    Run(#[from] crate::run::Error),

    /// Process-wide lock and lifecycle errors.
    #[error(transparent)]
    Process(#[from] crate::process::Error),

    /// ScreenerBot account / sign-in errors.
    #[error(transparent)]
    Account(#[from] AccountError),

    /// Network connectivity errors.
    #[error(transparent)]
    Network(#[from] NetworkError),

    /// RPC client operation errors.
    #[error(transparent)]
    Rpc(#[from] RpcError),

    /// RPC provider issues.
    #[error(transparent)]
    RpcProvider(#[from] RpcProviderError),

    /// Database errors (rusqlite/r2d2, schema, migrations).
    #[error(transparent)]
    Database(#[from] DatabaseError),

    /// Service lifecycle errors (startup/shutdown/deps).
    #[error(transparent)]
    Service(#[from] ServiceError),

    /// Filesystem / OS I/O errors.
    #[error(transparent)]
    Io(#[from] IoError),

    /// Invariants, task join failures, cancellation/timeouts, etc.
    #[error(transparent)]
    Internal(#[from] InternalError),

    /// Configuration errors.
    #[error(transparent)]
    Configuration(#[from] ConfigurationError),

    /// Data parsing & validation errors.
    #[error(transparent)]
    Data(#[from] DataError),
}

/// Convenient result type for ScreenerBot core code.
pub type Result<T> = std::result::Result<T, Error>;

// =============================================================================
// Conversions from standard library and external types
// =============================================================================

impl From<reqwest::Error> for Error {
    fn from(err: reqwest::Error) -> Self {
        let endpoint = err
            .url()
            .map(|u| u.as_str().to_owned())
            .unwrap_or_else(|| "unknown".to_owned());
        Error::Network(NetworkError::RequestFailed {
            endpoint,
            detail: err.to_string(),
        })
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Error::Data(DataError::ParseError {
            data_type: "JSON".to_owned(),
            error: err.to_string(),
        })
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::Io(IoError::from(err))
    }
}

impl From<rusqlite::Error> for Error {
    fn from(err: rusqlite::Error) -> Self {
        Error::Database(DatabaseError::from(err))
    }
}

impl From<r2d2::Error> for Error {
    fn from(err: r2d2::Error) -> Self {
        Error::Database(DatabaseError::from(err))
    }
}

impl From<tokio::task::JoinError> for Error {
    fn from(err: tokio::task::JoinError) -> Self {
        Error::Internal(InternalError::from(err))
    }
}

impl From<tokio::time::error::Elapsed> for Error {
    fn from(err: tokio::time::error::Elapsed) -> Self {
        Error::Internal(InternalError::from(err))
    }
}

// =============================================================================
// Structured error builders (migration helpers)
// =============================================================================

impl Error {
    /// Create an invalid amount error (replaces older string-based errors).
    pub fn invalid_amount(amount: impl Into<String>, reason: impl Into<String>) -> Self {
        Error::Data(DataError::InvalidAmount {
            amount: amount.into(),
            reason: reason.into(),
        })
    }

    /// Create a network error.
    pub fn network_error(message: impl Into<String>) -> Self {
        Error::Network(NetworkError::RequestFailed {
            endpoint: "unknown".to_owned(),
            detail: message.into(),
        })
    }

    /// Create an API/provider error.
    pub fn api_error(message: impl Into<String>) -> Self {
        Error::RpcProvider(RpcProviderError::Generic {
            provider_name: "unknown".to_owned(),
            message: message.into(),
        })
    }

    /// Create a connectivity error for endpoint health issues.
    pub fn connectivity_error(message: impl Into<String>) -> Self {
        Error::Network(NetworkError::RequestFailed {
            endpoint: "connectivity".to_owned(),
            detail: message.into(),
        })
    }

    /// Create an invalid response error.
    pub fn invalid_response(message: impl Into<String>) -> Self {
        Error::Data(DataError::InvalidFormat {
            expected: "valid response".to_owned(),
            received: message.into(),
        })
    }

    /// Create a parse error.
    pub fn parse_error(message: impl Into<String>) -> Self {
        Error::Data(DataError::ParseError {
            data_type: "unknown".to_owned(),
            error: message.into(),
        })
    }

    /// Create a slippage exceeded error.
    pub fn slippage_exceeded(message: impl Into<String>) -> Self {
        Error::Data(DataError::ValidationError {
            field: "slippage".to_owned(),
            value: "exceeded".to_owned(),
            reason: message.into(),
        })
    }

    /// Create a configuration error.
    pub fn configuration_error(message: impl Into<String>) -> Self {
        Error::Configuration(ConfigurationError::Generic {
            message: message.into(),
        })
    }

    /// Create an internal error.
    pub fn internal_error(message: impl Into<String>) -> Self {
        Error::Internal(InternalError::InvariantViolation {
            message: message.into(),
        })
    }

    /// Create an unsupported-capability error (well-formed request, missing
    /// implementation on the named owner).
    pub fn unsupported_capability(capability: impl Into<String>, owner: impl Into<String>) -> Self {
        Error::Internal(InternalError::UnsupportedCapability {
            capability: capability.into(),
            owner: owner.into(),
        })
    }
}
