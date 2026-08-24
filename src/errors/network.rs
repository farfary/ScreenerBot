//! Network errors — HTTP request failures, timeouts, and connection issues.

#[derive(Debug, Clone, thiserror::Error)]
pub enum NetworkError {
    /// The request could not be sent or the connection failed outright
    /// (DNS, TCP, TLS, or the transport dropping mid-request).
    #[error("request to {endpoint} failed: {detail}")]
    RequestFailed { endpoint: String, detail: String },
    /// The endpoint responded, but with a non-success HTTP status.
    #[error("{endpoint} returned HTTP {status}{}", body.as_deref().map(|b| format!(": {b}")).unwrap_or_default())]
    HttpStatus {
        endpoint: String,
        status: u16,
        body: Option<String>,
    },
    /// The request did not complete within its deadline.
    #[error("request to {endpoint} timed out after {timeout_ms}ms")]
    Timeout { endpoint: String, timeout_ms: u64 },
    /// The endpoint rejected the request for exceeding its rate limit.
    #[error("rate limited by {endpoint}")]
    RateLimited {
        endpoint: String,
        retry_after_ms: Option<u64>,
    },
}
