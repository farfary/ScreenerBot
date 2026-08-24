//! Errors produced by the external API clients: HTTP data sources and LLM
//! providers.

use std::time::Duration;

use crate::apis::llm::LlmError;
use crate::errors::{DataError, ErrorClass, NetworkError, Severity};

/// Everything that can go wrong talking to an external API: transport,
/// decoding, and the provider's own rejection reasons.
#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {
    /// HTTP transport failure: connection, timeout, non-success status, or
    /// provider-side rate limiting.
    #[error(transparent)]
    Network(#[from] NetworkError),
    /// The response body could not be parsed/decoded, or a caller-supplied
    /// argument failed validation before a request was even sent.
    #[error(transparent)]
    Data(#[from] DataError),
    /// An LLM provider call failed.
    #[error(transparent)]
    Llm(#[from] LlmError),
    /// An invariant inside a client was violated (e.g. a cache lock was
    /// poisoned by a panicked holder).
    #[error(transparent)]
    Internal(#[from] crate::errors::InternalError),
    /// The client is turned off via configuration.
    #[error("{provider} is disabled")]
    Disabled { provider: String },
    /// The client's internal rate limiter could not admit the request (the
    /// limiter's semaphore was closed — an invariant violation, not a normal
    /// "too many requests" response from the provider).
    #[error("rate limiter is unavailable: {detail}")]
    RateLimiter { detail: String },
    /// A provider with a metered/credit-based plan has exhausted its budget.
    #[error("{provider} credits are exhausted")]
    CreditsExhausted { provider: String },
    /// The provider had no data for the requested resource.
    #[error("{provider} returned no {resource}")]
    NotFound { provider: String, resource: String },
    /// Every source in a fallback cascade failed.
    #[error("all {resource} sources failed: {detail}")]
    SourcesExhausted { resource: String, detail: String },
}

/// Result alias for the API client modules.
pub type Result<T> = std::result::Result<T, Error>;

impl ErrorClass for Error {
    fn is_retryable(&self) -> bool {
        match self {
            Error::Network(e) => e.is_retryable(),
            Error::Data(_) => false,
            Error::Llm(e) => e.is_retryable(),
            Error::Internal(e) => e.is_retryable(),
            Error::Disabled { .. } | Error::CreditsExhausted { .. } => false,
            Error::RateLimiter { .. } => true,
            Error::NotFound { .. } => false,
            Error::SourcesExhausted { .. } => true,
        }
    }

    fn retry_after(&self) -> Option<Duration> {
        match self {
            Error::Network(e) => e.retry_after(),
            Error::Llm(e) => e.retry_after(),
            Error::Internal(e) => e.retry_after(),
            Error::RateLimiter { .. } => Some(Duration::from_millis(250)),
            Error::SourcesExhausted { .. } => Some(Duration::from_secs(5)),
            _ => None,
        }
    }

    fn severity(&self) -> Severity {
        match self {
            Error::Network(e) => e.severity(),
            Error::Data(_) => Severity::Warning,
            Error::Llm(e) => e.severity(),
            Error::Internal(e) => e.severity(),
            Error::Disabled { .. } => Severity::Info,
            Error::RateLimiter { .. } => Severity::Error,
            Error::CreditsExhausted { .. } => Severity::Warning,
            Error::NotFound { .. } => Severity::Info,
            Error::SourcesExhausted { .. } => Severity::Error,
        }
    }

    fn http_status(&self) -> u16 {
        match self {
            Error::Network(e) => e.http_status(),
            Error::Data(_) => 502,
            Error::Llm(e) => e.http_status(),
            Error::Internal(e) => e.http_status(),
            Error::Disabled { .. } => 503,
            Error::RateLimiter { .. } => 503,
            Error::CreditsExhausted { .. } => 402,
            Error::NotFound { .. } => 404,
            Error::SourcesExhausted { .. } => 502,
        }
    }
}
