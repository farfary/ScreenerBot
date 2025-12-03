use crate::config::get_config_clone;
use crate::connectivity::monitor::EndpointMonitor;
use crate::connectivity::types::{EndpointCriticality, FallbackStrategy, HealthCheckResult};
use crate::constants::{SOL_MINT, USDC_MINT};
use crate::utils::get_wallet_address;
use async_trait::async_trait;
use serde_json::Value;
use std::time::Instant;
use tokio::time::{timeout, Duration};

/// GMGN API endpoint
const GMGN_QUOTE_API: &str = "https://gmgn.ai/defi/router/v1/sol/tx/get_swap_route";

/// GMGN router health monitor
pub struct GmgnMonitor;

impl GmgnMonitor {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl EndpointMonitor for GmgnMonitor {
    fn name(&self) -> &'static str {
        "gmgn"
    }

    fn criticality(&self) -> EndpointCriticality {
        EndpointCriticality::Important
    }

    fn fallback_strategy(&self) -> Option<FallbackStrategy> {
        Some(FallbackStrategy::UseAlternative {
            endpoint_name: "jupiter".to_string(),
        })
    }

    fn is_enabled(&self) -> bool {
        let cfg = get_config_clone();
        // GMGN monitor only enabled if swap router is explicitly enabled
        // (GMGN router is hidden from users, so this will be false by default)
        cfg.connectivity.enabled
            && cfg.connectivity.endpoints.gmgn.enabled
            && cfg.swaps.gmgn.enabled
    }

    async fn check_health(&self) -> HealthCheckResult {
        let cfg = get_config_clone();
        let timeout_secs = cfg.connectivity.endpoints.gmgn.timeout_secs.max(1);

        let wallet_address = match get_wallet_address() {
            Ok(address) => address,
            Err(e) => {
                return HealthCheckResult::failure(format!(
                    "Failed to resolve wallet address for GMGN health check: {}",
                    e
                ))
            }
        };

        let partner = cfg.swaps.gmgn.partner.clone();
        let swap_mode = cfg.swaps.gmgn.default_swap_mode.clone();
        let fee_sol = cfg.swaps.gmgn.fee_sol;
        let anti_mev = cfg.swaps.gmgn.anti_mev;
        let slippage = cfg.swaps.slippage.quote_default_pct;
        let input_amount = 1_000_000u64; // 0.001 SOL keeps call lightweight

        let url = format!(
            "{}?token_in_address={}&token_out_address={}&in_amount={}&from_address={}&slippage={}&swap_mode={}&fee={}&is_anti_mev={}&partner={}",
            GMGN_QUOTE_API,
            SOL_MINT,
            USDC_MINT,
            input_amount,
            wallet_address,
            slippage,
            swap_mode,
            fee_sol,
            anti_mev,
            partner
        );

        let client = match reqwest::Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .build()
        {
            Ok(client) => client,
            Err(e) => {
                return HealthCheckResult::failure(format!(
                    "Failed to build HTTP client for GMGN health check: {}",
                    e
                ))
            }
        };

        let start = Instant::now();
        match timeout(Duration::from_secs(timeout_secs), client.get(&url).send()).await {
            Ok(Ok(response)) => {
                let latency = start.elapsed().as_millis() as u64;

                if response.status().is_success() {
                    match response.json::<Value>().await {
                        Ok(body) => {
                            if body.get("code").and_then(|c| c.as_i64()) == Some(0) {
                                return HealthCheckResult::success(latency);
                            }

                            let code = body.get("code").and_then(|c| c.as_i64()).unwrap_or(-1);
                            let message = body
                                .get("msg")
                                .and_then(|m| m.as_str())
                                .unwrap_or("Unknown error");

                            HealthCheckResult::failure(format!(
                                "GMGN API returned error code {}: {}",
                                code, message
                            ))
                        }
                        Err(e) => HealthCheckResult::failure(format!(
                            "Failed to parse GMGN health response: {}",
                            e
                        )),
                    }
                } else if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
                    HealthCheckResult::degraded(
                        latency,
                        "GMGN API rate limited (HTTP 429)".to_string(),
                    )
                } else {
                    HealthCheckResult::failure(format!(
                        "GMGN health check HTTP error: {}",
                        response.status()
                    ))
                }
            }
            Ok(Err(e)) => {
                HealthCheckResult::failure(format!("GMGN health check request failed: {}", e))
            }
            Err(_) => HealthCheckResult::failure(format!(
                "GMGN health check timed out after {}s",
                timeout_secs
            )),
        }
    }

    fn description(&self) -> &'static str {
        "GMGN Router API"
    }
}
