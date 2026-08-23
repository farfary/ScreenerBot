//! Top-level Error enum — unifies all domain-specific error types.

use super::{
    AccountError, BlockchainError, ConfigurationError, DataError, DatabaseError, InternalError,
    IoError, NetworkError, RpcProviderError, ServiceError,
};
use crate::rpc::errors::RpcError;

/// Top-level error type for ScreenerBot.
///
/// This is re-exported as `crate::Error` for ergonomic usage across the codebase.
#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {
    /// Position lifecycle errors (open/close/DCA/verification).
    #[error(transparent)]
    Positions(#[from] crate::positions::Error),

    /// ScreenerBot account / sign-in errors.
    #[error(transparent)]
    Account(#[from] AccountError),

    /// Blockchain & Solana specific errors.
    #[error(transparent)]
    Blockchain(#[from] BlockchainError),

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
        Error::Network(NetworkError::Generic {
            message: format!("HTTP request failed: {err}"),
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
        Error::Network(NetworkError::Generic {
            message: message.into(),
        })
    }

    /// Create a signing error.
    pub fn signing_error(message: impl Into<String>) -> Self {
        Error::Blockchain(BlockchainError::TransactionDropped {
            signature: "unknown".to_owned(),
            reason: format!("Signing error: {}", message.into()),
            fee_paid: None,
            attempts: 1,
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
        Error::Network(NetworkError::Generic {
            message: format!("Connectivity issue: {}", message.into()),
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

    /// Create an insufficient balance error.
    pub fn insufficient_balance(message: impl Into<String>) -> Self {
        Error::Blockchain(BlockchainError::InsufficientBalance {
            pubkey: "unknown".to_owned(),
            required_lamports: 0,
            available_lamports: 0,
            operation: message.into(),
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
