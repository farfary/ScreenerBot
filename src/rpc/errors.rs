//! RPC error types

use std::fmt;
use std::time::Duration;

/// RPC operation error
#[derive(Debug, Clone)]
pub enum RpcError {
    /// Rate limited by provider
    RateLimited {
        provider_id: String,
        retry_after: Option<Duration>,
    },

    /// Network/connection error
    Network { message: String, is_timeout: bool },

    /// Provider returned an error response
    ProviderError {
        code: i64,
        message: String,
        data: Option<String>,
    },

    /// Request timed out
    Timeout {
        provider_id: String,
        after: Duration,
    },

    /// Circuit breaker is open
    CircuitOpen {
        provider_id: String,
        retry_after: Duration,
    },

    /// No healthy providers available
    NoProvidersAvailable { last_error: Option<String> },

    /// Account not found (not retryable)
    AccountNotFound { pubkey: String },

    /// Invalid response format
    InvalidResponse { message: String },

    /// Configuration error
    Configuration { message: String },

    /// Generic error
    Other(String),
}

impl RpcError {
    /// Whether this error should trigger a retry
    pub fn is_retryable(&self) -> bool {
        match self {
            Self::RateLimited { .. } => true,
            Self::Network { .. } => true,
            Self::Timeout { .. } => true,
            Self::CircuitOpen { .. } => false, // Wait for circuit to close
            Self::NoProvidersAvailable { .. } => false,
            Self::AccountNotFound { .. } => false,
            Self::InvalidResponse { .. } => false,
            Self::Configuration { .. } => false,
            Self::ProviderError { code, .. } => {
                // Server errors are retryable, client errors are not
                *code >= -32099 && *code <= -32000
            }
            Self::Other(_) => false,
        }
    }

    /// Whether this is a rate limit error
    pub fn is_rate_limited(&self) -> bool {
        matches!(self, Self::RateLimited { .. })
    }

    /// Whether this is a timeout
    pub fn is_timeout(&self) -> bool {
        matches!(
            self,
            Self::Timeout { .. }
                | Self::Network {
                    is_timeout: true,
                    ..
                }
        )
    }

    /// Get retry-after duration if available
    pub fn retry_after(&self) -> Option<Duration> {
        match self {
            Self::RateLimited { retry_after, .. } => *retry_after,
            Self::CircuitOpen { retry_after, .. } => Some(*retry_after),
            _ => None,
        }
    }

    /// Create from HTTP status and response
    pub fn from_http_response(status: u16, body: &str, provider_id: &str) -> Self {
        match status {
            429 => {
                // Try to parse Retry-After from body or use default
                let retry_after = parse_retry_after(body);
                Self::RateLimited {
                    provider_id: provider_id.to_string(),
                    retry_after,
                }
            }
            408 | 504 => Self::Timeout {
                provider_id: provider_id.to_string(),
                after: Duration::from_secs(30),
            },
            502 | 503 => Self::Network {
                message: format!("Service unavailable ({}): {}", status, body),
                is_timeout: false,
            },
            _ => Self::Other(format!("HTTP {}: {}", status, body)),
        }
    }

    /// Create from JSON-RPC error
    pub fn from_jsonrpc_error(code: i64, message: &str, data: Option<&str>) -> Self {
        // Check for specific error patterns
        let msg_lower = message.to_lowercase();

        if msg_lower.contains("rate limit") || msg_lower.contains("too many requests") {
            return Self::RateLimited {
                provider_id: String::new(),
                retry_after: None,
            };
        }

        Self::ProviderError {
            code,
            message: message.to_string(),
            data: data.map(String::from),
        }
    }
}

impl fmt::Display for RpcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RateLimited {
                provider_id,
                retry_after,
            } => {
                write!(f, "Rate limited by {}", provider_id)?;
                if let Some(after) = retry_after {
                    write!(f, " (retry after {:?})", after)?;
                }
                Ok(())
            }
            Self::Network {
                message,
                is_timeout,
            } => {
                if *is_timeout {
                    write!(f, "Network timeout: {}", message)
                } else {
                    write!(f, "Network error: {}", message)
                }
            }
            Self::ProviderError { code, message, .. } => {
                write!(f, "Provider error {}: {}", code, message)
            }
            Self::Timeout { provider_id, after } => {
                write!(f, "Timeout after {:?} from {}", after, provider_id)
            }
            Self::CircuitOpen {
                provider_id,
                retry_after,
            } => {
                write!(
                    f,
                    "Circuit open for {} (retry after {:?})",
                    provider_id, retry_after
                )
            }
            Self::NoProvidersAvailable { last_error } => {
                write!(f, "No providers available")?;
                if let Some(err) = last_error {
                    write!(f, ": {}", err)?;
                }
                Ok(())
            }
            Self::AccountNotFound { pubkey } => {
                write!(f, "Account not found: {}", pubkey)
            }
            Self::InvalidResponse { message } => {
                write!(f, "Invalid response: {}", message)
            }
            Self::Configuration { message } => {
                write!(f, "Configuration error: {}", message)
            }
            Self::Other(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for RpcError {}

/// Parse Retry-After from response body or headers
fn parse_retry_after(body: &str) -> Option<Duration> {
    // Try to find retry-after in JSON response
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(body) {
        if let Some(secs) = json.get("retryAfter").and_then(|v| v.as_u64()) {
            return Some(Duration::from_secs(secs));
        }
        if let Some(secs) = json.get("retry_after").and_then(|v| v.as_u64()) {
            return Some(Duration::from_secs(secs));
        }
    }

    // Default retry after for rate limits
    Some(Duration::from_secs(1))
}

/// Convert from standard IO error
impl From<std::io::Error> for RpcError {
    fn from(err: std::io::Error) -> Self {
        Self::Network {
            message: err.to_string(),
            is_timeout: err.kind() == std::io::ErrorKind::TimedOut,
        }
    }
}

/// Convert from reqwest error
impl From<reqwest::Error> for RpcError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            Self::Network {
                message: err.to_string(),
                is_timeout: true,
            }
        } else if err.is_connect() {
            Self::Network {
                message: format!("Connection failed: {}", err),
                is_timeout: false,
            }
        } else {
            Self::Network {
                message: err.to_string(),
                is_timeout: false,
            }
        }
    }
}

/// Convert RpcError to String
impl From<RpcError> for String {
    fn from(err: RpcError) -> Self {
        err.to_string()
    }
}
