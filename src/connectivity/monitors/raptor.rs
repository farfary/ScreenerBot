// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Raptor (Solana Tracker) swap API health monitor.

use crate::config::get_config_clone;
use crate::connectivity::monitor::EndpointMonitor;
use crate::connectivity::types::{
    EndpointCriticality, FallbackStrategy, HealthCheckResult, ProbeFailure,
};
use async_trait::async_trait;
use std::time::Instant;
use tokio::time::Duration;

/// Raptor swap API monitor
pub struct RaptorMonitor;

impl RaptorMonitor {
    /// Create a new monitor instance
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl EndpointMonitor for RaptorMonitor {
    fn name(&self) -> &'static str {
        "raptor"
    }

    fn criticality(&self) -> EndpointCriticality {
        // Raptor is the optional second aggregator and ships disabled. Losing it
        // costs a price comparison, never the ability to trade, so it must never
        // degrade the app the way the primary router does.
        EndpointCriticality::Important
    }

    fn fallback_strategy(&self) -> Option<FallbackStrategy> {
        // Jupiter is the monitored alternative, but the routing layer already
        // falls back by priority on its own — nothing here needs to redirect.
        None
    }

    fn is_enabled(&self) -> bool {
        let cfg = get_config_clone();
        // Only worth checking when the router that uses it is actually on.
        cfg.connectivity.enabled
            && cfg.connectivity.endpoints.raptor.enabled
            && cfg.swaps.raptor.enabled
    }

    async fn check_health(&self) -> HealthCheckResult {
        let cfg = get_config_clone();
        let timeout_secs = cfg.connectivity.endpoints.raptor.timeout_secs.max(1);

        let client = match crate::net::client_builder()
            .timeout(Duration::from_secs(timeout_secs))
            .build()
        {
            Ok(c) => c,
            Err(e) => {
                return HealthCheckResult::failure(ProbeFailure::ClientSetupFailed {
                    detail: e.to_string(),
                })
            }
        };

        // A tiny SOL->USDC quote: the cheapest call that proves routing works
        // rather than merely that the host answers. No key, no side effect.
        let url = crate::chains::solana::swaps::routers::health_probe_url();
        let start = Instant::now();

        match client.get(&url).send().await {
            Ok(response) => {
                let latency = start.elapsed().as_millis() as u64;

                if response.status().is_success() {
                    HealthCheckResult::success(latency)
                } else {
                    HealthCheckResult::failure(ProbeFailure::HttpStatus {
                        status: response.status().to_string(),
                    })
                }
            }
            Err(e) => {
                if e.is_timeout() {
                    HealthCheckResult::failure(ProbeFailure::Timeout {
                        seconds: timeout_secs,
                    })
                } else {
                    HealthCheckResult::failure(ProbeFailure::RequestFailed {
                        detail: e.to_string(),
                    })
                }
            }
        }
    }

    fn description(&self) -> &'static str {
        "Raptor Aggregator API"
    }
}

impl Default for RaptorMonitor {
    fn default() -> Self {
        Self::new()
    }
}
