use thiserror::Error;

#[derive(Error, Debug)]
pub enum BotError {
    #[error("Configuration error: {0}")] Config(String),

    #[error("Wallet error: {0}")] Wallet(String),

    #[error("RPC error: {0}")] Rpc(String),

    #[error("Trading error: {0}")] Trading(String),

    #[error("Screener error: {0}")] Screener(String),

    #[error("Cache error: {0}")] Cache(String),

    #[error("Portfolio error: {0}")] Portfolio(String),

    #[error("Network error: {0}")] Network(String),

    #[error("Parse error: {0}")] Parse(String),

    #[error("Insufficient funds: need {needed} SOL, have {available} SOL")] InsufficientFunds {
        needed: f64,
        available: f64,
    },

    #[error("Invalid token: {mint}")] InvalidToken {
        mint: String,
    },

    #[error("Slippage too high: {actual}% > {max}%")] SlippageTooHigh {
        actual: f64,
        max: f64,
    },

    #[error("Transaction failed: {reason}")] TransactionFailed {
        reason: String,
    },

    #[error("Rate limit exceeded: {service}")] RateLimit {
        service: String,
    },

    #[error("Service unavailable: {service}")] ServiceUnavailable {
        service: String,
    },

    #[error("Database error: {0}")] Database(#[from] rusqlite::Error),

    #[error("Serialization error: {0}")] Serialization(#[from] serde_json::Error),

    #[error("IO error: {0}")] Io(#[from] std::io::Error),

    #[error("HTTP error: {0}")] Http(String),

    #[error("Timeout error: operation timed out after {seconds} seconds")] Timeout {
        seconds: u64,
    },

    #[error("Unknown error: {0}")] Unknown(String),
}

impl BotError {
    pub fn is_recoverable(&self) -> bool {
        match self {
            BotError::Network(_) => true,
            BotError::Http(_) => true,
            BotError::RateLimit { .. } => true,
            BotError::ServiceUnavailable { .. } => true,
            BotError::Timeout { .. } => true,
            BotError::Rpc(_) => true,
            _ => false,
        }
    }

    pub fn is_critical(&self) -> bool {
        match self {
            BotError::Config(_) => true,
            BotError::Wallet(_) => true,
            BotError::InsufficientFunds { .. } => true,
            BotError::Database(_) => true,
            _ => false,
        }
    }

    pub fn retry_after_seconds(&self) -> Option<u64> {
        match self {
            BotError::RateLimit { .. } => Some(60),
            BotError::ServiceUnavailable { .. } => Some(30),
            BotError::Network(_) => Some(10),
            BotError::Http(_) => Some(5),
            BotError::Timeout { .. } => Some(5),
            _ => None,
        }
    }
}

pub type BotResult<T> = Result<T, BotError>;
