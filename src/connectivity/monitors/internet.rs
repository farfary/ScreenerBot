//! Internet connectivity monitor — basic reachability check via DNS/HTTP.

use crate::config::get_config_clone;
use crate::connectivity::monitor::EndpointMonitor;
use crate::connectivity::types::{EndpointCriticality, FallbackStrategy, HealthCheckResult};
use crate::errors::NetworkError;
use async_trait::async_trait;
use futures::future::select_ok;
use std::time::Instant;
use tokio::net::TcpStream;
use tokio::time::{timeout, Duration};

/// Upper bound for a single connectivity probe. A TCP connect to a DNS server
/// (or a HEAD to a CDN) completes in well under a second when online, so capping
/// the probe here keeps offline detection fast regardless of the larger general
/// `health_check_timeout_secs` used for heavier API health checks.
const MAX_PROBE_SECS: u64 = 2;

/// Internet connectivity monitor - checks DNS and HTTP connectivity
pub struct InternetMonitor;

impl InternetMonitor {
    /// Create a new monitor instance
    pub fn new() -> Self {
        Self
    }

    /// Check DNS connectivity by racing TCP connections to all configured DNS
    /// servers concurrently. Resolves as soon as ANY connects (fast online
    /// detection) and fails only once ALL fail or time out (bounded by
    /// `MAX_PROBE_SECS`, so offline detection is ~one short timeout, not the sum
    /// across servers).
    async fn check_dns(&self, timeout_secs: u64) -> Result<u64, NetworkError> {
        let cfg = get_config_clone();
        let dns_servers = cfg.connectivity.internet.dns_servers.clone();

        if dns_servers.is_empty() {
            return Err(NetworkError::RequestFailed {
                endpoint: "DNS reachability probe".to_owned(),
                detail: "no DNS servers configured".to_owned(),
            });
        }

        let timeout_duration = Duration::from_secs(timeout_secs.min(MAX_PROBE_SECS).max(1));
        let start = Instant::now();

        let connects: Vec<_> = dns_servers
            .iter()
            .map(|server| {
                let addr = format!("{server}:53");
                Box::pin(async move {
                    match timeout(timeout_duration, TcpStream::connect(&addr)).await {
                        Ok(Ok(_)) => Ok(()),
                        Ok(Err(e)) => Err(NetworkError::RequestFailed {
                            endpoint: addr,
                            detail: e.to_string(),
                        }),
                        Err(_) => Err(NetworkError::Timeout {
                            endpoint: addr,
                            timeout_ms: timeout_duration.as_millis() as u64,
                        }),
                    }
                })
            })
            .collect();

        match select_ok(connects).await {
            Ok(_) => Ok(start.elapsed().as_millis() as u64),
            Err(_) => Err(NetworkError::RequestFailed {
                endpoint: "DNS reachability probe".to_owned(),
                detail: format!("all DNS servers unreachable: {dns_servers:?}"),
            }),
        }
    }

    /// Check HTTP connectivity by racing HEAD requests to all configured check
    /// endpoints concurrently (same fast-resolve / bounded-fail semantics as
    /// `check_dns`).
    async fn check_http(&self, timeout_secs: u64) -> Result<u64, NetworkError> {
        let cfg = get_config_clone();
        let http_checks = cfg.connectivity.internet.http_checks.clone();

        if http_checks.is_empty() {
            return Err(NetworkError::RequestFailed {
                endpoint: "HTTP reachability probe".to_owned(),
                detail: "no HTTP check endpoints configured".to_owned(),
            });
        }

        let probe_secs = timeout_secs.min(MAX_PROBE_SECS).max(1);
        let client = crate::net::client_builder()
            .timeout(Duration::from_secs(probe_secs))
            .build()
            .map_err(|e| NetworkError::RequestFailed {
                endpoint: "HTTP reachability probe".to_owned(),
                detail: e.to_string(),
            })?;

        let start = Instant::now();
        let requests: Vec<_> = http_checks
            .iter()
            .map(|url| {
                let client = client.clone();
                let url = url.clone();
                Box::pin(async move {
                    match client.head(&url).send().await {
                        Ok(response) if response.status().is_success() => Ok(()),
                        Ok(response) => Err(NetworkError::HttpStatus {
                            endpoint: url,
                            status: response.status().as_u16(),
                            body: None,
                        }),
                        Err(e) => Err(NetworkError::RequestFailed {
                            endpoint: url,
                            detail: e.to_string(),
                        }),
                    }
                })
            })
            .collect();

        match select_ok(requests).await {
            Ok(_) => Ok(start.elapsed().as_millis() as u64),
            Err(_) => Err(NetworkError::RequestFailed {
                endpoint: "HTTP reachability probe".to_owned(),
                detail: format!("all HTTP check endpoints unreachable: {http_checks:?}"),
            }),
        }
    }
}

#[async_trait]
impl EndpointMonitor for InternetMonitor {
    fn name(&self) -> &'static str {
        "internet"
    }

    fn criticality(&self) -> EndpointCriticality {
        EndpointCriticality::Critical
    }

    fn fallback_strategy(&self) -> Option<FallbackStrategy> {
        Some(FallbackStrategy::Fail)
    }

    fn is_enabled(&self) -> bool {
        let cfg = get_config_clone();
        cfg.connectivity.enabled && cfg.connectivity.internet.enabled
    }

    async fn check_health(&self) -> HealthCheckResult {
        let cfg = get_config_clone();
        let timeout_secs = cfg.connectivity.health_check_timeout_secs;

        // Try DNS first (faster)
        match self.check_dns(timeout_secs).await {
            Ok(latency) => HealthCheckResult::success(latency),
            Err(dns_error) => {
                // DNS failed, try HTTP as backup
                match self.check_http(timeout_secs).await {
                    Ok(latency) => HealthCheckResult::degraded(
                        latency,
                        format!("DNS check failed but HTTP works: {dns_error}"),
                    ),
                    Err(http_error) => HealthCheckResult::failure(format!(
                        "DNS and HTTP checks failed. DNS: {}. HTTP: {}",
                        dns_error, http_error
                    )),
                }
            }
        }
    }

    fn description(&self) -> &'static str {
        "Internet connectivity (DNS and HTTP)"
    }
}
