use crate::connectivity::monitor::EndpointMonitor;
use crate::connectivity::types::{EndpointCriticality, FallbackStrategy, HealthCheckResult};
use crate::config::get_config_clone;
use async_trait::async_trait;
use std::time::Instant;
use tokio::net::TcpStream;
use tokio::time::{timeout, Duration};

/// Internet connectivity monitor - checks DNS and HTTP connectivity
pub struct InternetMonitor;

impl InternetMonitor {
    pub fn new() -> Self {
        Self
    }

    /// Check DNS connectivity by attempting TCP connection to DNS servers
    async fn check_dns(&self, timeout_secs: u64) -> Result<u64, String> {
        let cfg = get_config_clone();
        let dns_servers = &cfg.connectivity.internet.dns_servers;

        if dns_servers.is_empty() {
            return Err("No DNS servers configured".to_string());
        }

        let timeout_duration = Duration::from_secs(timeout_secs);

        for dns_server in dns_servers {
            let addr = format!("{}:53", dns_server);
            let start = Instant::now();

            match timeout(timeout_duration, TcpStream::connect(&addr)).await {
                Ok(Ok(_)) => {
                    let latency = start.elapsed().as_millis() as u64;
                    return Ok(latency);
                }
                Ok(Err(e)) => {
                    continue; // Try next DNS server
                }
                Err(_) => {
                    continue; // Timeout, try next
                }
            }
        }

        Err(format!(
            "All DNS servers unreachable: {:?}",
            dns_servers
        ))
    }

    /// Check HTTP connectivity by making request to known endpoints
    async fn check_http(&self, timeout_secs: u64) -> Result<u64, String> {
        let cfg = get_config_clone();
        let http_checks = &cfg.connectivity.internet.http_checks;

        if http_checks.is_empty() {
            return Err("No HTTP check endpoints configured".to_string());
        }

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .build()
            .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

        for url in http_checks {
            let start = Instant::now();

            match client.head(url).send().await {
                Ok(response) if response.status().is_success() => {
                    let latency = start.elapsed().as_millis() as u64;
                    return Ok(latency);
                }
                Ok(_) => continue, // Non-success status, try next
                Err(_) => continue, // Request failed, try next
            }
        }

        Err(format!(
            "All HTTP check endpoints unreachable: {:?}",
            http_checks
        ))
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
                        format!("DNS check failed but HTTP works: {}", dns_error),
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
