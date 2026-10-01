//! Network errors — HTTP request failures, timeouts, and connection issues.
//!
//! `endpoint` is always rendered through `redact_url`: provider endpoints carry
//! API keys in their URL, and these messages reach logs and API responses.

use crate::logger::redact_url;

#[derive(Debug, Clone, thiserror::Error)]
pub enum NetworkError {
    /// The request could not be sent or the connection failed outright
    /// (DNS, TCP, TLS, or the transport dropping mid-request).
    #[error("request to {} failed: {detail}", redact_url(endpoint))]
    RequestFailed { endpoint: String, detail: String },
    /// The endpoint responded, but with a non-success HTTP status.
    #[error("{} returned HTTP {status}{}", redact_url(endpoint), body.as_deref().map(|b| format!(": {b}")).unwrap_or_default())]
    HttpStatus {
        endpoint: String,
        status: u16,
        body: Option<String>,
    },
    /// The request did not complete within its deadline.
    #[error("request to {} timed out after {timeout_ms}ms", redact_url(endpoint))]
    Timeout { endpoint: String, timeout_ms: u64 },
    /// The endpoint rejected the request for exceeding its rate limit.
    #[error("rate limited by {}", redact_url(endpoint))]
    RateLimited {
        endpoint: String,
        retry_after_ms: Option<u64>,
    },
}
