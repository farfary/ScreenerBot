// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Connectivity data types — endpoint health status, criticality levels, and fallback strategies.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize, Serializer};

use crate::i18n::{ids, UiArg, UiText};

/// Criticality level determines system behavior when endpoint is unavailable
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EndpointCriticality {
    /// System pauses completely if endpoint is down (e.g., Internet, RPC)
    Critical,
    /// System continues but with warnings and degraded mode (e.g., DexScreener, Jupiter)
    Important,
    /// System continues silently with fallback (e.g., Rugcheck, CoinGecko)
    Optional,
}

impl EndpointCriticality {
    /// Parse criticality level from a string (defaults to Optional for unknown values)
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "critical" => Self::Critical,
            "important" => Self::Important,
            "optional" => Self::Optional,
            _ => Self::Optional,
        }
    }
}

/// Why a health probe failed or ran degraded. Technical values are carried as data
/// and rendered through the catalog (`connectivity-probe-*`); on the wire the value
/// is its [`UiText`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProbeFailure {
    /// The HTTP client could not be built.
    ClientSetupFailed { detail: String },
    /// The endpoint answered with a non-success status, formatted as `503 Service Unavailable`.
    HttpStatus { status: String },
    /// No response within the configured timeout.
    Timeout { seconds: u64 },
    /// The request failed before a response arrived.
    RequestFailed { detail: String },
    /// No RPC provider is configured.
    NoRpcProviders,
    /// Some, but not all, RPC providers are healthy.
    RpcPartiallyHealthy {
        healthy: usize,
        total: usize,
        unhealthy: String,
    },
    /// Every RPC provider is unhealthy.
    RpcAllUnreachable { total: usize, unhealthy: String },
    /// DNS resolution failed while the HTTP check succeeded.
    InternetDnsFailed { detail: String },
    /// Both the DNS and the HTTP reachability checks failed.
    InternetChecksFailed { dns: String, http: String },
    /// A failure was reported without a reason.
    Unknown,
}

impl ProbeFailure {
    pub fn ui_text(&self) -> UiText {
        let text = |value: &str| UiArg::Text(value.to_owned());
        let count = |value: usize| UiArg::Text(value.to_string());
        match self {
            Self::ClientSetupFailed { detail } => {
                UiText::new(ids::CONNECTIVITY_PROBE_CLIENT_SETUP_FAILED).arg("detail", text(detail))
            }
            Self::HttpStatus { status } => {
                UiText::new(ids::CONNECTIVITY_PROBE_HTTP_STATUS).arg("status", text(status))
            }
            Self::Timeout { seconds } => UiText::new(ids::CONNECTIVITY_PROBE_TIMEOUT)
                .arg("seconds", UiArg::Text(seconds.to_string())),
            Self::RequestFailed { detail } => {
                UiText::new(ids::CONNECTIVITY_PROBE_REQUEST_FAILED).arg("detail", text(detail))
            }
            Self::NoRpcProviders => UiText::new(ids::CONNECTIVITY_PROBE_RPC_NONE),
            Self::RpcPartiallyHealthy {
                healthy,
                total,
                unhealthy,
            } => UiText::new(ids::CONNECTIVITY_PROBE_RPC_PARTIAL)
                .arg("healthy", count(*healthy))
                .arg("total", count(*total))
                .arg("unhealthy", text(unhealthy)),
            Self::RpcAllUnreachable { total, unhealthy } => {
                UiText::new(ids::CONNECTIVITY_PROBE_RPC_ALL_UNREACHABLE)
                    .arg("total", count(*total))
                    .arg("unhealthy", text(unhealthy))
            }
            Self::InternetDnsFailed { detail } => {
                UiText::new(ids::CONNECTIVITY_PROBE_INTERNET_DNS_FAILED).arg("detail", text(detail))
            }
            Self::InternetChecksFailed { dns, http } => {
                UiText::new(ids::CONNECTIVITY_PROBE_INTERNET_ALL_FAILED)
                    .arg("dns", text(dns))
                    .arg("http", text(http))
            }
            Self::Unknown => UiText::new(ids::CONNECTIVITY_PROBE_UNKNOWN),
        }
    }

    /// Source-locale plain text for logs and event payloads.
    pub fn render_source_plain(&self) -> String {
        self.ui_text().render_source_plain()
    }
}

impl Serialize for ProbeFailure {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.ui_text().serialize(serializer)
    }
}

/// Health status of an endpoint with detailed information
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "status", rename_all = "lowercase")]
pub enum EndpointHealth {
    /// Endpoint is functioning normally
    Healthy {
        latency_ms: u64,
        last_check: DateTime<Utc>,
    },
    /// Endpoint is functioning but with degraded performance
    Degraded {
        latency_ms: u64,
        reason: ProbeFailure,
        last_check: DateTime<Utc>,
    },
    /// Endpoint is not functioning
    Unhealthy {
        reason: ProbeFailure,
        last_check: DateTime<Utc>,
        last_success: Option<DateTime<Utc>>,
        consecutive_failures: u32,
    },
    /// Health status unknown (not checked yet)
    Unknown,
}

impl EndpointHealth {
    /// Returns true if the endpoint is fully healthy
    pub fn is_healthy(&self) -> bool {
        matches!(self, EndpointHealth::Healthy { .. })
    }

    /// Returns true if the endpoint is in degraded mode
    pub fn is_degraded(&self) -> bool {
        matches!(self, EndpointHealth::Degraded { .. })
    }

    /// Returns true if the endpoint is completely unavailable
    pub fn is_unhealthy(&self) -> bool {
        matches!(self, EndpointHealth::Unhealthy { .. })
    }

    /// Returns true if the endpoint is usable (healthy or degraded)
    pub fn is_available(&self) -> bool {
        matches!(
            self,
            EndpointHealth::Healthy { .. } | EndpointHealth::Degraded { .. }
        )
    }
}

/// Fallback strategy when endpoint is unavailable
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum FallbackStrategy {
    /// Use cached data if available and not older than max_age_secs
    UseCache { max_age_secs: u64 },
    /// Use alternative endpoint
    UseAlternative { endpoint_name: String },
    /// Skip the operation silently
    Skip,
    /// Fail the operation with error
    Fail,
}

impl FallbackStrategy {
    /// Parse a fallback strategy from a config string (defaults to Skip)
    pub fn from_config(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "cache" => Self::UseCache {
                max_age_secs: 86400,
            }, // 24h default
            "skip" => Self::Skip,
            "fail" => Self::Fail,
            _ => Self::Skip,
        }
    }
}

/// Check result for health monitoring
#[derive(Debug)]
pub struct HealthCheckResult {
    pub healthy: bool,
    pub latency_ms: u64,
    pub error: Option<ProbeFailure>,
}

impl HealthCheckResult {
    /// Create a successful health check result with measured latency
    pub fn success(latency_ms: u64) -> Self {
        Self {
            healthy: true,
            latency_ms,
            error: None,
        }
    }

    /// Create a failed health check result with the failure
    pub fn failure(error: ProbeFailure) -> Self {
        Self {
            healthy: false,
            latency_ms: 0,
            error: Some(error),
        }
    }

    /// Create a degraded health check result (healthy but with performance issues)
    pub fn degraded(latency_ms: u64, reason: ProbeFailure) -> Self {
        Self {
            healthy: true,
            latency_ms,
            error: Some(reason),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::format_en;

    /// Every variant, listed through an exhaustive match so a new one fails to
    /// compile until it is added here and to the catalog.
    fn failures() -> Vec<(ProbeFailure, &'static str)> {
        let listed = |failure: ProbeFailure| match failure {
            ProbeFailure::ClientSetupFailed { .. }
            | ProbeFailure::HttpStatus { .. }
            | ProbeFailure::Timeout { .. }
            | ProbeFailure::RequestFailed { .. }
            | ProbeFailure::NoRpcProviders
            | ProbeFailure::RpcPartiallyHealthy { .. }
            | ProbeFailure::RpcAllUnreachable { .. }
            | ProbeFailure::InternetDnsFailed { .. }
            | ProbeFailure::InternetChecksFailed { .. }
            | ProbeFailure::Unknown => failure,
        };
        vec![
            (
                ProbeFailure::ClientSetupFailed {
                    detail: "bad tls".to_owned(),
                },
                "Failed to create client: bad tls",
            ),
            (
                ProbeFailure::HttpStatus {
                    status: "503 Service Unavailable".to_owned(),
                },
                "HTTP 503 Service Unavailable",
            ),
            (ProbeFailure::Timeout { seconds: 5 }, "Timeout after 5s"),
            (
                ProbeFailure::RequestFailed {
                    detail: "connection reset".to_owned(),
                },
                "Request failed: connection reset",
            ),
            (ProbeFailure::NoRpcProviders, "No RPC providers configured"),
            (
                ProbeFailure::RpcPartiallyHealthy {
                    healthy: 1,
                    total: 3,
                    unhealthy: "a (Http): Open; b (Http): Open".to_owned(),
                },
                "1/3 RPC providers healthy. Unhealthy: a (Http): Open; b (Http): Open",
            ),
            (
                ProbeFailure::RpcAllUnreachable {
                    total: 2,
                    unhealthy: "a (Http): Open".to_owned(),
                },
                "All 2 RPC providers unreachable: a (Http): Open",
            ),
            (
                ProbeFailure::InternetDnsFailed {
                    detail: "dns down".to_owned(),
                },
                "DNS check failed but HTTP works: dns down",
            ),
            (
                ProbeFailure::InternetChecksFailed {
                    dns: "dns down".to_owned(),
                    http: "http down".to_owned(),
                },
                "DNS and HTTP checks failed. DNS: dns down. HTTP: http down",
            ),
            (ProbeFailure::Unknown, "Unknown error"),
        ]
        .into_iter()
        .map(|(failure, english)| (listed(failure), english))
        .collect()
    }

    #[test]
    fn every_probe_failure_has_catalog_text_with_its_original_wording() {
        for (failure, english) in failures() {
            let text = failure.ui_text();
            assert_ne!(format_en(&text.id, None), text.id, "missing {}", text.id);
            assert_eq!(failure.render_source_plain(), english);
        }
    }

    #[test]
    fn unhealthy_reason_serializes_as_ui_text() {
        let health = EndpointHealth::Unhealthy {
            reason: ProbeFailure::Timeout { seconds: 5 },
            last_check: Utc::now(),
            last_success: None,
            consecutive_failures: 3,
        };
        let value = serde_json::to_value(&health).unwrap();
        assert_eq!(value["reason"]["id"], "connectivity-probe-timeout");
        assert_eq!(value["reason"]["args"]["seconds"]["value"], "5");
    }
}
