//! The behavioural consumers of structured errors: how loudly to report a
//! failure and whether/when a caller should retry it.

use std::time::Duration;

use super::{
    AccountError, ConfigurationError, DataError, DatabaseError, Error, InternalError, IoError,
    NetworkError, ServiceError,
};

/// How loudly a failure should be reported.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// Expected, self-healing; log at debug.
    Info,
    /// Degraded but the operation continued.
    Warning,
    /// The operation failed and a human may need to know.
    Error,
    /// Money, keys, or persistence integrity is at risk.
    Critical,
}

/// The behavioural questions callers ask an error. Implementing this is what
/// makes a structured error worth more than a `String`.
pub trait ErrorClass {
    /// True when the exact same call could succeed if repeated unchanged.
    fn is_retryable(&self) -> bool;

    /// Minimum wait before a retry is worth attempting.
    fn retry_after(&self) -> Option<Duration> {
        None
    }

    /// How loudly this should be reported.
    fn severity(&self) -> Severity;

    /// HTTP status when this error surfaces through the webserver.
    fn http_status(&self) -> u16;
}

impl ErrorClass for AccountError {
    fn is_retryable(&self) -> bool {
        // A garbled/unexpected response from screenerbot.io is often a
        // one-off transport hiccup; everything else about an account error
        // is a final verdict that will not change by repeating the call.
        matches!(self, AccountError::UnexpectedResponse { .. })
    }

    fn retry_after(&self) -> Option<Duration> {
        match self {
            AccountError::UnexpectedResponse { .. } => Some(Duration::from_secs(2)),
            _ => None,
        }
    }

    fn severity(&self) -> Severity {
        match self {
            AccountError::NotSignedIn => Severity::Info,
            AccountError::Refused { .. } | AccountError::SessionEnded => Severity::Warning,
            AccountError::UnexpectedResponse { .. }
            | AccountError::Storage { .. }
            | AccountError::Generic { .. } => Severity::Error,
        }
    }

    fn http_status(&self) -> u16 {
        match self {
            AccountError::Refused { .. }
            | AccountError::NotSignedIn
            | AccountError::SessionEnded => 401,
            AccountError::UnexpectedResponse { .. } => 502,
            AccountError::Storage { .. } | AccountError::Generic { .. } => 500,
        }
    }
}

impl ErrorClass for NetworkError {
    fn is_retryable(&self) -> bool {
        match self {
            NetworkError::RequestFailed { .. } | NetworkError::Timeout { .. } => true,
            // A non-success status is only worth retrying unchanged when the
            // server signalled a transient condition (5xx) or explicit
            // throttling (429); any other status is a deterministic verdict.
            NetworkError::HttpStatus { status, .. } => {
                *status == 429 || (500..600).contains(status)
            }
            NetworkError::RateLimited { .. } => true,
        }
    }

    fn retry_after(&self) -> Option<Duration> {
        match self {
            NetworkError::RequestFailed { .. } => Some(Duration::from_millis(500)),
            NetworkError::Timeout { .. } => Some(Duration::from_secs(1)),
            NetworkError::HttpStatus { .. } => Some(Duration::from_secs(2)),
            NetworkError::RateLimited { retry_after_ms, .. } => Some(
                retry_after_ms
                    .map(Duration::from_millis)
                    .unwrap_or(Duration::from_secs(5)),
            ),
        }
    }

    fn severity(&self) -> Severity {
        Severity::Warning
    }

    fn http_status(&self) -> u16 {
        match self {
            NetworkError::RequestFailed { .. } => 502,
            NetworkError::Timeout { .. } => 504,
            NetworkError::HttpStatus { status, .. } => *status,
            NetworkError::RateLimited { .. } => 429,
        }
    }
}

impl ErrorClass for DatabaseError {
    fn is_retryable(&self) -> bool {
        // A connection/pool error is a transient contention problem; a
        // query or a raw SQLite error is either a bad statement or a
        // corruption/constraint issue, neither of which changes on retry.
        matches!(self, DatabaseError::Connection { .. })
    }

    fn retry_after(&self) -> Option<Duration> {
        match self {
            DatabaseError::Connection { .. } => Some(Duration::from_millis(250)),
            _ => None,
        }
    }

    fn severity(&self) -> Severity {
        match self {
            DatabaseError::Connection { .. } => Severity::Error,
            DatabaseError::Sqlite { .. } => Severity::Critical,
            DatabaseError::Query { .. } => Severity::Error,
        }
    }

    fn http_status(&self) -> u16 {
        match self {
            DatabaseError::Connection { .. } => 503,
            DatabaseError::Sqlite { .. } | DatabaseError::Query { .. } => 500,
        }
    }
}

impl ErrorClass for ConfigurationError {
    fn is_retryable(&self) -> bool {
        // Nothing in configuration fixes itself; a human has to edit
        // settings and restart.
        false
    }

    fn severity(&self) -> Severity {
        match self {
            ConfigurationError::InvalidPrivateKey { .. } => Severity::Critical,
            ConfigurationError::Generic { .. } => Severity::Error,
        }
    }

    fn http_status(&self) -> u16 {
        500
    }
}

impl ErrorClass for InternalError {
    fn is_retryable(&self) -> bool {
        // A timeout may clear on its own; an invariant violation, a joined
        // task panic, or a missing capability will not.
        matches!(self, InternalError::Timeout { .. })
    }

    fn retry_after(&self) -> Option<Duration> {
        match self {
            InternalError::Timeout { .. } => Some(Duration::from_secs(1)),
            _ => None,
        }
    }

    fn severity(&self) -> Severity {
        match self {
            InternalError::InvariantViolation { .. } | InternalError::TaskJoin { .. } => {
                Severity::Critical
            }
            InternalError::Timeout { .. } => Severity::Warning,
            InternalError::UnsupportedCapability { .. } => Severity::Error,
        }
    }

    fn http_status(&self) -> u16 {
        match self {
            InternalError::InvariantViolation { .. } | InternalError::TaskJoin { .. } => 500,
            InternalError::Timeout { .. } => 504,
            InternalError::UnsupportedCapability { .. } => 501,
        }
    }
}

impl ErrorClass for IoError {
    fn is_retryable(&self) -> bool {
        // Filesystem state (missing file, wrong permissions, already
        // exists, bad input) does not change by repeating the same call.
        false
    }

    fn severity(&self) -> Severity {
        match self {
            IoError::PermissionDenied { .. } => Severity::Critical,
            IoError::NotFound { .. }
            | IoError::AlreadyExists { .. }
            | IoError::InvalidInput { .. } => Severity::Warning,
            IoError::Generic { .. } => Severity::Error,
        }
    }

    fn http_status(&self) -> u16 {
        match self {
            IoError::NotFound { .. } => 404,
            IoError::PermissionDenied { .. } => 403,
            IoError::AlreadyExists { .. } => 409,
            IoError::InvalidInput { .. } => 400,
            IoError::Generic { .. } => 500,
        }
    }
}

impl ErrorClass for DataError {
    fn is_retryable(&self) -> bool {
        // Parsing/validation failures are a property of the data itself,
        // not the attempt.
        false
    }

    fn severity(&self) -> Severity {
        Severity::Warning
    }

    fn http_status(&self) -> u16 {
        400
    }
}

impl ErrorClass for ServiceError {
    fn is_retryable(&self) -> bool {
        // A service that failed to *start* often just lost a race with a
        // dependency that is still coming up; init/stop/dependency failures
        // are structural and repeat identically.
        matches!(self, ServiceError::Start { .. })
    }

    fn retry_after(&self) -> Option<Duration> {
        match self {
            ServiceError::Start { .. } => Some(Duration::from_secs(5)),
            _ => None,
        }
    }

    fn severity(&self) -> Severity {
        match self {
            ServiceError::Initialize { .. } => Severity::Critical,
            ServiceError::Start { .. } | ServiceError::Dependency { .. } => Severity::Error,
            ServiceError::Stop { .. } => Severity::Warning,
            ServiceError::Generic { .. } => Severity::Error,
        }
    }

    fn http_status(&self) -> u16 {
        match self {
            ServiceError::Dependency { .. } => 424,
            _ => 500,
        }
    }
}

impl ErrorClass for Error {
    fn is_retryable(&self) -> bool {
        match self {
            Error::Ai(e) => e.is_retryable(),
            Error::Positions(e) => e.is_retryable(),
            Error::Transactions(e) => e.is_retryable(),
            Error::Trader(e) => e.is_retryable(),
            Error::Wallets(e) => e.is_retryable(),
            Error::Tools(e) => e.is_retryable(),
            Error::Chains(e) => e.is_retryable(),
            Error::Solana(e) => e.is_retryable(),
            Error::Apis(e) => e.is_retryable(),
            Error::Pools(e) => e.is_retryable(),
            Error::Telegram(e) => e.is_retryable(),
            Error::Filtering(e) => e.is_retryable(),
            Error::Actions(e) => e.is_retryable(),
            Error::Account(e) => e.is_retryable(),
            Error::Network(e) => e.is_retryable(),
            Error::Database(e) => e.is_retryable(),
            Error::Service(e) => e.is_retryable(),
            Error::Io(e) => e.is_retryable(),
            Error::Internal(e) => e.is_retryable(),
            Error::Configuration(e) => e.is_retryable(),
            Error::Data(e) => e.is_retryable(),
            // `RpcError` and `RpcProviderError` do not implement
            // `ErrorClass` yet — owned by the RPC provider retirement.
            // Treat them as not-automatically-retryable until that
            // classification exists; see the T0 report for this
            // limitation.
            Error::Rpc(_) | Error::RpcProvider(_) => false,
        }
    }

    fn retry_after(&self) -> Option<Duration> {
        match self {
            Error::Ai(e) => e.retry_after(),
            Error::Positions(e) => e.retry_after(),
            Error::Transactions(e) => e.retry_after(),
            Error::Trader(e) => e.retry_after(),
            Error::Wallets(e) => e.retry_after(),
            Error::Tools(e) => e.retry_after(),
            Error::Chains(e) => e.retry_after(),
            Error::Solana(e) => e.retry_after(),
            Error::Apis(e) => e.retry_after(),
            Error::Pools(e) => e.retry_after(),
            Error::Telegram(e) => e.retry_after(),
            Error::Filtering(e) => e.retry_after(),
            Error::Actions(e) => e.retry_after(),
            Error::Account(e) => e.retry_after(),
            Error::Network(e) => e.retry_after(),
            Error::Database(e) => e.retry_after(),
            Error::Service(e) => e.retry_after(),
            Error::Io(e) => e.retry_after(),
            Error::Internal(e) => e.retry_after(),
            Error::Configuration(e) => e.retry_after(),
            Error::Data(e) => e.retry_after(),
            Error::Rpc(_) | Error::RpcProvider(_) => None,
        }
    }

    fn severity(&self) -> Severity {
        match self {
            Error::Ai(e) => e.severity(),
            Error::Positions(e) => e.severity(),
            Error::Transactions(e) => e.severity(),
            Error::Trader(e) => e.severity(),
            Error::Wallets(e) => e.severity(),
            Error::Tools(e) => e.severity(),
            Error::Chains(e) => e.severity(),
            Error::Solana(e) => e.severity(),
            Error::Apis(e) => e.severity(),
            Error::Pools(e) => e.severity(),
            Error::Telegram(e) => e.severity(),
            Error::Filtering(e) => e.severity(),
            Error::Actions(e) => e.severity(),
            Error::Account(e) => e.severity(),
            Error::Network(e) => e.severity(),
            Error::Database(e) => e.severity(),
            Error::Service(e) => e.severity(),
            Error::Io(e) => e.severity(),
            Error::Internal(e) => e.severity(),
            Error::Configuration(e) => e.severity(),
            Error::Data(e) => e.severity(),
            // Coarse pending the same later-task classification noted above.
            Error::Rpc(_) | Error::RpcProvider(_) => Severity::Error,
        }
    }

    fn http_status(&self) -> u16 {
        match self {
            Error::Ai(e) => e.http_status(),
            Error::Positions(e) => e.http_status(),
            Error::Transactions(e) => e.http_status(),
            Error::Trader(e) => e.http_status(),
            Error::Wallets(e) => e.http_status(),
            Error::Tools(e) => e.http_status(),
            Error::Chains(e) => e.http_status(),
            Error::Solana(e) => e.http_status(),
            Error::Apis(e) => e.http_status(),
            Error::Pools(e) => e.http_status(),
            Error::Telegram(e) => e.http_status(),
            Error::Filtering(e) => e.http_status(),
            Error::Actions(e) => e.http_status(),
            Error::Account(e) => e.http_status(),
            Error::Network(e) => e.http_status(),
            Error::Database(e) => e.http_status(),
            Error::Service(e) => e.http_status(),
            Error::Io(e) => e.http_status(),
            Error::Internal(e) => e.http_status(),
            Error::Configuration(e) => e.http_status(),
            Error::Data(e) => e.http_status(),
            Error::Rpc(_) | Error::RpcProvider(_) => 502,
        }
    }
}
