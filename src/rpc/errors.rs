// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! RPC error types.

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
                // The generic server-error range includes request-specific transaction failures.
                (*code >= -32099 && *code <= -32000) && !matches!(*code, -32002 | -32015)
            }
            Self::Other(_) => false,
        }
    }

    /// Whether this failure is evidence that the selected provider is unhealthy.
    /// Deterministic JSON-RPC request errors (including unsupported transaction
    /// versions) describe the request, not provider availability.
    pub fn is_provider_health_failure(&self) -> bool {
        match self {
            Self::RateLimited { .. }
            | Self::Network { .. }
            | Self::Timeout { .. }
            | Self::InvalidResponse { .. } => true,
            Self::ProviderError { .. } => self.is_retryable(),
            Self::CircuitOpen { .. }
            | Self::NoProvidersAvailable { .. }
            | Self::AccountNotFound { .. }
            | Self::Configuration { .. }
            | Self::Other(_) => false,
        }
    }

    /// Whether a node decoded this request and refused the request itself.
    ///
    /// Only the JSON-RPC request-shape codes qualify: a parse error (-32700), an
    /// invalid request (-32600) and invalid params (-32602, which is how a node
    /// refuses an oversized or undecodable transaction). They are deterministic
    /// in the request bytes, so no node and no retry would accept them either,
    /// and they prove the request was never acted on. Transport failures, rate
    /// limits, timeouts, open circuits, an empty provider pool and the
    /// retryable server band are not an answer about the request.
    pub fn is_request_rejection(&self) -> bool {
        matches!(
            self,
            Self::ProviderError {
                code: -32700 | -32600 | -32602,
                ..
            }
        )
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
                message: format!("Service unavailable ({status}): {body}"),
                is_timeout: false,
            },
            _ => Self::Other(format!("HTTP {status}: {body}")),
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
                write!(f, "Rate limited by {provider_id}")?;
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
                    write!(f, "Network timeout: {message}")
                } else {
                    write!(f, "Network error: {message}")
                }
            }
            Self::ProviderError { code, message, .. } => {
                write!(f, "Provider error {code}: {message}")
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
                    write!(f, ": {err}")?;
                }
                Ok(())
            }
            Self::AccountNotFound { pubkey } => {
                write!(f, "Account not found: {pubkey}")
            }
            Self::InvalidResponse { message } => {
                write!(f, "Invalid response: {message}")
            }
            Self::Configuration { message } => {
                write!(f, "Configuration error: {message}")
            }
            Self::Other(msg) => write!(f, "{msg}"),
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

/// Convert from reqwest error.
///
/// The URL is stripped first: reqwest's `Display` appends the full request URL,
/// and provider endpoints carry their API key in it.
impl From<reqwest::Error> for RpcError {
    fn from(err: reqwest::Error) -> Self {
        let err = err.without_url();
        if err.is_timeout() {
            Self::Network {
                message: err.to_string(),
                is_timeout: true,
            }
        } else if err.is_connect() {
            Self::Network {
                message: format!("Connection failed: {err}"),
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

#[cfg(test)]
mod tests {
    use super::RpcError;
    use crate::rpc::types::CircuitState;
    use crate::rpc::{CircuitBreakerConfig, ProviderCircuitBreaker};
    use std::time::Duration;

    #[tokio::test]
    async fn unsupported_transaction_version_does_not_retry_or_open_provider_circuit() {
        let error = RpcError::ProviderError {
            code: -32015,
            message: "Transaction version (1) is not supported by the requesting client".to_owned(),
            data: None,
        };
        assert!(!error.is_retryable());
        assert!(!error.is_provider_health_failure());

        let breaker = ProviderCircuitBreaker::new(
            "test-provider",
            CircuitBreakerConfig {
                min_state_duration: Duration::from_millis(1),
                ..Default::default()
            },
        );
        tokio::time::sleep(Duration::from_millis(2)).await;
        for _ in 0..5 {
            if error.is_provider_health_failure() {
                breaker.record_failure(&error.to_string(), false).await;
            }
        }
        assert_eq!(breaker.current_state().await, CircuitState::Closed);
        assert!(breaker.can_execute().await.is_ok());
    }

    #[tokio::test]
    async fn transport_and_retryable_server_errors_still_open_provider_circuit() {
        let errors = [
            RpcError::Network {
                message: "connection reset".to_owned(),
                is_timeout: false,
            },
            RpcError::ProviderError {
                code: -32005,
                message: "node is unhealthy".to_owned(),
                data: None,
            },
        ];
        let breaker = ProviderCircuitBreaker::new(
            "test-provider",
            CircuitBreakerConfig {
                min_state_duration: Duration::from_millis(1),
                ..Default::default()
            },
        );
        tokio::time::sleep(Duration::from_millis(2)).await;
        for error in errors.iter().cycle().take(5) {
            assert!(error.is_retryable());
            assert!(error.is_provider_health_failure());
            breaker.record_failure(&error.to_string(), false).await;
        }
        assert_eq!(breaker.current_state().await, CircuitState::Open);
        assert!(breaker.can_execute().await.is_err());
    }

    #[test]
    fn json_rpc_request_errors_are_not_retryable_or_provider_health_failures() {
        for code in [-32700, -32600, -32601, -32602] {
            let error = RpcError::ProviderError {
                code,
                message: "request rejected".to_owned(),
                data: None,
            };
            assert!(!error.is_retryable(), "code {code}");
            assert!(!error.is_provider_health_failure(), "code {code}");
        }

        let simulation_failed = RpcError::ProviderError {
            code: -32002,
            message: "Transaction simulation failed".to_owned(),
            data: None,
        };
        assert!(!simulation_failed.is_retryable());
        assert!(!simulation_failed.is_provider_health_failure());
    }

    /// The variant name a table row stands for. Exhaustive, so a new variant
    /// cannot compile without being given a row below.
    fn variant(error: &RpcError) -> &'static str {
        match error {
            RpcError::RateLimited { .. } => "RateLimited",
            RpcError::Network { .. } => "Network",
            RpcError::ProviderError { .. } => "ProviderError",
            RpcError::Timeout { .. } => "Timeout",
            RpcError::CircuitOpen { .. } => "CircuitOpen",
            RpcError::NoProvidersAvailable { .. } => "NoProvidersAvailable",
            RpcError::AccountNotFound { .. } => "AccountNotFound",
            RpcError::InvalidResponse { .. } => "InvalidResponse",
            RpcError::Configuration { .. } => "Configuration",
            RpcError::Other(_) => "Other",
        }
    }

    /// Every variant, and every provider-code class, against the three
    /// questions callers ask: retry it, blame the provider, or read it as the
    /// node's answer about the request. Columns: retryable, provider health
    /// failure, request rejection.
    #[test]
    fn every_rpc_error_is_classified_on_all_three_questions() {
        let provider = |code: i64| RpcError::ProviderError {
            code,
            message: "stub".to_owned(),
            data: None,
        };
        let rows: Vec<(RpcError, bool, bool, bool)> = vec![
            (
                RpcError::RateLimited {
                    provider_id: "p".to_owned(),
                    retry_after: None,
                },
                true,
                true,
                false,
            ),
            (
                RpcError::Network {
                    message: "reset".to_owned(),
                    is_timeout: false,
                },
                true,
                true,
                false,
            ),
            (
                RpcError::Network {
                    message: "timed out".to_owned(),
                    is_timeout: true,
                },
                true,
                true,
                false,
            ),
            (provider(-32700), false, false, true),
            (provider(-32600), false, false, true),
            (provider(-32601), false, false, false),
            (provider(-32602), false, false, true),
            (provider(-32002), false, false, false),
            (provider(-32005), true, true, false),
            (provider(-32015), false, false, false),
            (
                RpcError::Timeout {
                    provider_id: "p".to_owned(),
                    after: Duration::from_secs(1),
                },
                true,
                true,
                false,
            ),
            (
                RpcError::CircuitOpen {
                    provider_id: "p".to_owned(),
                    retry_after: Duration::from_secs(1),
                },
                false,
                false,
                false,
            ),
            (
                RpcError::NoProvidersAvailable { last_error: None },
                false,
                false,
                false,
            ),
            (
                RpcError::AccountNotFound {
                    pubkey: "k".to_owned(),
                },
                false,
                false,
                false,
            ),
            (
                RpcError::InvalidResponse {
                    message: "garbled".to_owned(),
                },
                false,
                true,
                false,
            ),
            (
                RpcError::Configuration {
                    message: "bad".to_owned(),
                },
                false,
                false,
                false,
            ),
            (RpcError::Other("other".to_owned()), false, false, false),
        ];

        let mut covered: Vec<&str> = rows.iter().map(|(error, ..)| variant(error)).collect();
        covered.sort_unstable();
        covered.dedup();
        assert_eq!(
            covered,
            vec![
                "AccountNotFound",
                "CircuitOpen",
                "Configuration",
                "InvalidResponse",
                "Network",
                "NoProvidersAvailable",
                "Other",
                "ProviderError",
                "RateLimited",
                "Timeout",
            ],
            "every RpcError variant needs a row"
        );

        for (error, retryable, health, rejection) in rows {
            assert_eq!(error.is_retryable(), retryable, "retryable: {error}");
            assert_eq!(
                error.is_provider_health_failure(),
                health,
                "provider health: {error}"
            );
            assert_eq!(
                error.is_request_rejection(),
                rejection,
                "rejection: {error}"
            );
        }
    }

    /// A transport failure against a credential-bearing endpoint, produced
    /// locally: the port is bound, released, then dialled, so the connect is
    /// refused without leaving the machine.
    async fn refused_request_error(url_path: &str) -> reqwest::Error {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);
        let client = reqwest::Client::builder().no_proxy().build().unwrap();
        client
            .post(format!("http://127.0.0.1:{port}{url_path}"))
            .send()
            .await
            .expect_err("request to a released port must fail")
    }

    #[tokio::test]
    async fn reqwest_errors_never_carry_the_endpoint_api_key() {
        let raw = refused_request_error("/?api-key=secret").await;
        assert!(raw.to_string().contains("secret"));

        let rpc_error = RpcError::from(refused_request_error("/?api-key=secret").await);
        assert!(!rpc_error.to_string().contains("secret"), "{rpc_error}");
        assert!(!format!("{rpc_error:?}").contains("secret"));

        let error = crate::Error::from(refused_request_error("/?api-key=secret").await);
        let rendered = error.to_string();
        assert!(!rendered.contains("secret"), "{rendered}");
        assert!(rendered.contains("api-key=***"), "{rendered}");
        assert!(!format!("{error:?}").contains("secret"));
    }
}
