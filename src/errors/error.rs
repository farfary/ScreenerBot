use super::{
    BlockchainError, ConfigurationError, DataError, DatabaseError, InternalError, IoError,
    NetworkError, PositionError, RateLimitError, RpcProviderError, ServiceError,
};
use crate::rpc::errors::RpcError;

/// Top-level error type for ScreenerBot.
///
/// This is re-exported as `crate::Error` for ergonomic usage across the codebase.
#[derive(Debug, Clone)]
pub enum Error {
    // Blockchain & Solana specific errors
    Blockchain(BlockchainError),

    // Network connectivity errors
    Network(NetworkError),

    // RPC client operation errors
    Rpc(RpcError),

    // RPC provider issues
    RpcProvider(RpcProviderError),

    // Database errors (rusqlite/r2d2, schema, migrations)
    Database(DatabaseError),

    // Service lifecycle errors (startup/shutdown/deps)
    Service(ServiceError),

    // Filesystem / OS I/O errors
    Io(IoError),

    // Invariants, task join failures, cancellation/timeouts, etc.
    Internal(InternalError),

    // Configuration errors
    Configuration(ConfigurationError),

    // Data parsing & validation errors
    Data(DataError),

    // Position management errors
    Position(PositionError),

    // Rate limiting errors
    RateLimit(RateLimitError),
}

/// Convenient result type for ScreenerBot core code.
pub type Result<T> = std::result::Result<T, Error>;

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Blockchain(e) => write!(f, "Blockchain Error: {}", e),
            Error::Network(e) => write!(f, "Network Error: {}", e),
            Error::Rpc(e) => write!(f, "RPC Error: {}", e),
            Error::RpcProvider(e) => write!(f, "RPC Provider Error: {}", e),
            Error::Database(e) => write!(f, "Database Error: {}", e),
            Error::Service(e) => write!(f, "Service Error: {}", e),
            Error::Io(e) => write!(f, "IO Error: {}", e),
            Error::Internal(e) => write!(f, "Internal Error: {}", e),
            Error::Configuration(e) => write!(f, "Configuration Error: {}", e),
            Error::Data(e) => write!(f, "Data Error: {}", e),
            Error::Position(e) => write!(f, "Position Error: {}", e),
            Error::RateLimit(e) => write!(f, "Rate Limit Error: {}", e),
        }
    }
}

impl std::error::Error for Error {}

// =============================================================================
// Conversions from standard library and external types
// =============================================================================

impl From<reqwest::Error> for Error {
    fn from(err: reqwest::Error) -> Self {
        Error::Network(NetworkError::Generic {
            message: format!("HTTP request failed: {}", err),
        })
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Error::Data(DataError::ParseError {
            data_type: "JSON".to_string(),
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

impl From<RpcError> for Error {
    fn from(err: RpcError) -> Self {
        Error::Rpc(err)
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
            signature: "unknown".to_string(),
            reason: format!("Signing error: {}", message.into()),
            fee_paid: None,
            attempts: 1,
        })
    }

    /// Create an API/provider error.
    pub fn api_error(message: impl Into<String>) -> Self {
        Error::RpcProvider(RpcProviderError::Generic {
            provider_name: "unknown".to_string(),
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
            expected: "valid response".to_string(),
            received: message.into(),
        })
    }

    /// Create a parse error.
    pub fn parse_error(message: impl Into<String>) -> Self {
        Error::Data(DataError::ParseError {
            data_type: "unknown".to_string(),
            error: message.into(),
        })
    }

    /// Create a slippage exceeded error.
    pub fn slippage_exceeded(message: impl Into<String>) -> Self {
        Error::Data(DataError::ValidationError {
            field: "slippage".to_string(),
            value: "exceeded".to_string(),
            reason: message.into(),
        })
    }

    /// Create an insufficient balance error.
    pub fn insufficient_balance(message: impl Into<String>) -> Self {
        Error::Blockchain(BlockchainError::InsufficientBalance {
            pubkey: "unknown".to_string(),
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
}
