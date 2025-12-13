/// Comprehensive error handling system for ScreenerBot
/// Replaces old ScreenerBotError with structured blockchain-aware error types
use chrono::{DateTime, Utc};

pub mod blockchain;
pub use blockchain::*;

// =============================================================================
// MAIN ERROR TYPE - Replaces ScreenerBotError completely
// =============================================================================

#[derive(Debug, Clone)]
pub enum ScreenerBotError {
    // Blockchain & Solana specific errors
    Blockchain(BlockchainError),

    // Network connectivity errors
    Network(NetworkError),

    // RPC provider issues
    RpcProvider(RpcProviderError),

    // Configuration errors
    Configuration(ConfigurationError),

    // Data parsing & validation errors
    Data(DataError),

    // Position management errors
    Position(PositionError),

    // Rate limiting errors
    RateLimit(RateLimitError),
}

impl std::fmt::Display for ScreenerBotError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScreenerBotError::Blockchain(e) => write!(f, "Blockchain Error: {}", e),
            ScreenerBotError::Network(e) => write!(f, "Network Error: {}", e),
            ScreenerBotError::RpcProvider(e) => write!(f, "RPC Provider Error: {}", e),
            ScreenerBotError::Configuration(e) => write!(f, "Configuration Error: {}", e),
            ScreenerBotError::Data(e) => write!(f, "Data Error: {}", e),
            ScreenerBotError::Position(e) => write!(f, "Position Error: {}", e),
            ScreenerBotError::RateLimit(e) => write!(f, "Rate Limit Error: {}", e),
        }
    }
}

impl std::error::Error for ScreenerBotError {}

// =============================================================================
// NETWORK ERROR TYPES
// =============================================================================

#[derive(Debug, Clone)]
pub enum NetworkError {
    ConnectionTimeout {
        endpoint: String,
        timeout_ms: u64,
    },
    ConnectionRefused {
        endpoint: String,
        reason: String,
    },
    HttpStatusError {
        endpoint: String,
        status: u16,
        body: Option<String>,
    },
    DnsResolutionFailed {
        hostname: String,
        error: String,
    },
    TlsHandshakeFailed {
        endpoint: String,
        error: String,
    },
    Generic {
        message: String,
    },
}

impl std::fmt::Display for NetworkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NetworkError::ConnectionTimeout {
                endpoint,
                timeout_ms,
            } => {
                write!(
                    f,
                    "Connection timeout to {} after {}ms",
                    endpoint, timeout_ms
                )
            }
            NetworkError::HttpStatusError {
                endpoint,
                status,
                body,
            } => {
                write!(
                    f,
                    "HTTP {} from {}: {}",
                    status,
                    endpoint,
                    body.as_deref().unwrap_or("No body")
                )
            }
            NetworkError::Generic { message } => write!(f, "{}", message),
            _ => write!(f, "{:?}", self),
        }
    }
}

// =============================================================================
// RPC PROVIDER ERROR TYPES
// =============================================================================

#[derive(Debug, Clone)]
pub enum RpcProviderError {
    ProviderDown {
        provider_name: String,
        since: DateTime<Utc>,
    },
    RateLimitExceeded {
        provider_name: String,
        limit_type: String,
        reset_at: DateTime<Utc>,
    },
    MalformedResponse {
        provider_name: String,
        endpoint: String,
        response_body: String,
    },
    ApiKeyInvalid {
        provider_name: String,
    },
    Generic {
        provider_name: String,
        message: String,
    },
}

impl std::fmt::Display for RpcProviderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RpcProviderError::ProviderDown {
                provider_name,
                since,
            } => {
                write!(f, "Provider {} down since {}", provider_name, since)
            }
            RpcProviderError::Generic {
                provider_name,
                message,
            } => {
                write!(f, "Provider {} error: {}", provider_name, message)
            }
            _ => write!(f, "{:?}", self),
        }
    }
}

// =============================================================================
// CONFIGURATION ERROR TYPES
// =============================================================================

#[derive(Debug, Clone)]
pub enum ConfigurationError {
    InvalidConfig { field: String, reason: String },
    MissingConfig { field: String },
    InvalidPrivateKey { error: String },
    InvalidWalletAddress { address: String, error: String },
    InvalidUrl { url: String, error: String },
    FileNotFound { path: String },
    Generic { message: String },
}

impl std::fmt::Display for ConfigurationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigurationError::InvalidConfig { field, reason } => {
                write!(f, "Invalid config field '{}': {}", field, reason)
            }
            ConfigurationError::InvalidPrivateKey { error } => {
                write!(f, "Invalid private key: {}", error)
            }
            ConfigurationError::Generic { message } => write!(f, "{}", message),
            _ => write!(f, "{:?}", self),
        }
    }
}

// =============================================================================
// DATA ERROR TYPES
// =============================================================================

#[derive(Debug, Clone)]
pub enum DataError {
    ParseError {
        data_type: String,
        error: String,
    },
    ValidationError {
        field: String,
        value: String,
        reason: String,
    },
    InvalidFormat {
        expected: String,
        received: String,
    },
    InvalidAmount {
        amount: String,
        reason: String,
    },
    Generic {
        message: String,
    },
}

impl std::fmt::Display for DataError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DataError::ParseError { data_type, error } => {
                write!(f, "Failed to parse {}: {}", data_type, error)
            }
            DataError::InvalidAmount { amount, reason } => {
                write!(f, "Invalid amount '{}': {}", amount, reason)
            }
            DataError::Generic { message } => write!(f, "{}", message),
            _ => write!(f, "{:?}", self),
        }
    }
}

// =============================================================================
// POSITION ERROR TYPES
// =============================================================================

#[derive(Debug, Clone)]
pub enum PositionError {
    PositionNotFound {
        token_mint: String,
        signature: String,
    },
    VerificationTimeout {
        signature: String,
        timeout_seconds: u64,
    },
    VerificationFailed {
        signature: String,
        reason: String,
    },
    PhantomPositionDetected {
        token_mint: String,
        signature: String,
    },
    Generic {
        message: String,
    },
    DatabaseError(String),
}

impl std::fmt::Display for PositionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PositionError::PositionNotFound {
                token_mint,
                signature,
            } => {
                write!(
                    f,
                    "Position not found for token {} with signature {}",
                    token_mint, signature
                )
            }
            PositionError::Generic { message } => write!(f, "{}", message),
            PositionError::DatabaseError(msg) => write!(f, "Database error: {}", msg),
            _ => write!(f, "{:?}", self),
        }
    }
}

// =============================================================================
// RATE LIMIT ERROR TYPES
// =============================================================================

#[derive(Debug, Clone)]
pub enum RateLimitError {
    ExceededLimit {
        limit_type: String,
        current_rate: f64,
        limit: f64,
    },
    TemporaryThrottle {
        duration_seconds: u64,
    },
    Generic {
        message: String,
    },
}

impl std::fmt::Display for RateLimitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RateLimitError::ExceededLimit {
                limit_type,
                current_rate,
                limit,
            } => {
                write!(
                    f,
                    "Rate limit exceeded for {}: {}/s > {}/s",
                    limit_type, current_rate, limit
                )
            }
            RateLimitError::Generic { message } => write!(f, "{}", message),
            _ => write!(f, "{:?}", self),
        }
    }
}

// =============================================================================
// BACKWARD COMPATIBILITY: Error conversions from standard library types
// =============================================================================

impl From<String> for ScreenerBotError {
    fn from(err: String) -> Self {
        ScreenerBotError::Network(NetworkError::Generic { message: err })
    }
}

impl From<&str> for ScreenerBotError {
    fn from(err: &str) -> Self {
        ScreenerBotError::Network(NetworkError::Generic {
            message: err.to_string(),
        })
    }
}

impl From<reqwest::Error> for ScreenerBotError {
    fn from(err: reqwest::Error) -> Self {
        ScreenerBotError::Network(NetworkError::Generic {
            message: format!("HTTP request failed: {}", err),
        })
    }
}

impl From<serde_json::Error> for ScreenerBotError {
    fn from(err: serde_json::Error) -> Self {
        ScreenerBotError::Data(DataError::ParseError {
            data_type: "JSON".to_string(),
            error: err.to_string(),
        })
    }
}

// =============================================================================
// STRUCTURED ERROR BUILDERS: Migration helpers for backward compatibility
// =============================================================================

impl ScreenerBotError {
    /// Create an invalid amount error (replaces ScreenerBotError::InvalidAmount)
    pub fn invalid_amount(amount: impl Into<String>, reason: impl Into<String>) -> Self {
        ScreenerBotError::Data(DataError::InvalidAmount {
            amount: amount.into(),
            reason: reason.into(),
        })
    }

    /// Create a network error (replaces ScreenerBotError::NetworkError)
    pub fn network_error(message: impl Into<String>) -> Self {
        ScreenerBotError::Network(NetworkError::Generic {
            message: message.into(),
        })
    }

    /// Create a signing error (replaces ScreenerBotError::SigningError)
    pub fn signing_error(message: impl Into<String>) -> Self {
        ScreenerBotError::Blockchain(BlockchainError::TransactionDropped {
            signature: "unknown".to_string(),
            reason: format!("Signing error: {}", message.into()),
            fee_paid: None,
            attempts: 1,
        })
    }

    /// Create an API error (replaces ScreenerBotError::ApiError)
    pub fn api_error(message: impl Into<String>) -> Self {
        ScreenerBotError::RpcProvider(RpcProviderError::Generic {
            provider_name: "unknown".to_string(),
            message: message.into(),
        })
    }

    /// Create a connectivity error for endpoint health issues
    pub fn connectivity_error(message: impl Into<String>) -> Self {
        ScreenerBotError::Network(NetworkError::Generic {
            message: format!("Connectivity issue: {}", message.into()),
        })
    }

    /// Create an invalid response error (replaces ScreenerBotError::InvalidResponse)
    pub fn invalid_response(message: impl Into<String>) -> Self {
        ScreenerBotError::Data(DataError::InvalidFormat {
            expected: "valid response".to_string(),
            received: message.into(),
        })
    }

    /// Create a parse error (replaces ScreenerBotError::ParseError)
    pub fn parse_error(message: impl Into<String>) -> Self {
        ScreenerBotError::Data(DataError::ParseError {
            data_type: "unknown".to_string(),
            error: message.into(),
        })
    }

    /// Create a slippage exceeded error (replaces ScreenerBotError::SlippageExceeded)
    pub fn slippage_exceeded(message: impl Into<String>) -> Self {
        ScreenerBotError::Data(DataError::ValidationError {
            field: "slippage".to_string(),
            value: "exceeded".to_string(),
            reason: message.into(),
        })
    }

    /// Create an insufficient balance error (replaces ScreenerBotError::InsufficientBalance)
    pub fn insufficient_balance(message: impl Into<String>) -> Self {
        ScreenerBotError::Blockchain(BlockchainError::InsufficientBalance {
            pubkey: "unknown".to_string(),
            required_lamports: 0,
            available_lamports: 0,
            operation: message.into(),
        })
    }

    /// Create a configuration error
    pub fn configuration_error(message: impl Into<String>) -> Self {
        ScreenerBotError::Configuration(ConfigurationError::Generic {
            message: message.into(),
        })
    }

    /// Create an internal error
    pub fn internal_error(message: impl Into<String>) -> Self {
        ScreenerBotError::Data(DataError::ValidationError {
            field: "internal".to_string(),
            value: "error".to_string(),
            reason: message.into(),
        })
    }
}
