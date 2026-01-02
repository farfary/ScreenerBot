//! RPC Client wrapper with Solana SDK integration
//!
//! Provides high-level methods that wrap the RpcManager's raw JSON-RPC calls
//! and translate to/from Solana SDK types.

pub mod methods;

pub use methods::{
    ProviderHealthInfo,
    RpcClientMethods,
    // Program account types
    RpcFilterType,
    // Token supply types
    RpcTokenAccountBalance,
    // Transaction history types
    SignatureInfo,
    TokenSupply,
};

use crate::rpc::manager::RpcManager;
use crate::rpc::stats::{RpcStatsResponse, StatsManager};
use crate::rpc::types::mask_url;
use std::sync::Arc;

/// RPC Client wrapper providing Solana-typed methods
pub struct RpcClient {
    /// The underlying RPC manager
    manager: Arc<RpcManager>,
}

impl RpcClient {
    /// Create new RPC client wrapping a manager
    pub fn new(manager: Arc<RpcManager>) -> Self {
        Self { manager }
    }

    /// Get the underlying manager
    pub fn manager(&self) -> &RpcManager {
        &self.manager
    }

    /// Get manager Arc
    pub fn manager_arc(&self) -> Arc<RpcManager> {
        self.manager.clone()
    }

    /// Check if the client is initialized (always true once constructed)
    pub fn is_initialized(&self) -> bool {
        true
    }

    /// Get provider count
    pub async fn provider_count(&self) -> usize {
        self.manager.provider_count().await
    }

    /// Get primary provider URL (masked for security)
    pub async fn primary_url_masked(&self) -> String {
        match self.manager.primary_url().await {
            Some(url) => mask_url(&url),
            None => String::from("(no providers)"),
        }
    }

    /// Get stats manager reference (for advanced usage)
    ///
    /// Note: Returns None as stats manager is internal to RpcManager.
    /// Use `get_stats()` method instead for statistics.
    pub fn stats_manager(&self) -> Option<Arc<StatsManager>> {
        // StatsManager is internal to RpcManager and wrapped in RwLock,
        // so we don't expose it directly. Use get_stats() instead.
        None
    }

    /// Get RPC statistics
    pub async fn get_stats(&self) -> RpcStatsResponse {
        self.manager.get_stats().await
    }

    /// Get health information for all providers
    pub async fn get_provider_health(&self) -> Vec<ProviderHealthInfo> {
        let states = self.manager.get_provider_states().await;
        let configs = self.manager.get_provider_configs().await;
        let cb_statuses = self.manager.circuit_breakers().get_all_status().await;

        // Build a map of circuit breaker statuses by provider ID
        let cb_map: std::collections::HashMap<String, _> = cb_statuses
            .into_iter()
            .map(|s| (s.provider_id.clone(), s))
            .collect();

        states
            .into_iter()
            .map(|state| {
                // Find matching config for rate limit info
                let config = configs.iter().find(|c| c.id == state.id);
                let base_rate_limit = config.map(|c| c.effective_rate_limit()).unwrap_or(100);

                // Get circuit breaker status
                let cb_status = cb_map.get(&state.id);
                let circuit_state = cb_status
                    .map(|s| s.state)
                    .unwrap_or(crate::rpc::types::CircuitState::Closed);

                ProviderHealthInfo {
                    provider_id: state.id.clone(),
                    url_masked: state.url_masked.clone(),
                    kind: state.kind,
                    is_healthy: state.is_healthy(),
                    is_enabled: state.enabled,
                    circuit_state,
                    total_calls: state.total_calls,
                    total_errors: state.total_errors,
                    success_rate: state.success_rate(),
                    avg_latency_ms: state.avg_latency_ms,
                    consecutive_failures: state.consecutive_failures,
                    consecutive_successes: state.consecutive_successes,
                    base_rate_limit,
                    last_success: state.last_success,
                    last_failure: state.last_failure,
                    last_error: state.last_error.clone(),
                }
            })
            .collect()
    }

    /// Force circuit breaker reset for a specific provider
    pub async fn reset_circuit_breaker(&self, provider_id: &str) -> Result<(), String> {
        let breaker = self
            .manager
            .circuit_breakers()
            .get_breaker(provider_id)
            .await;
        breaker.force_close().await;
        Ok(())
    }

    /// Force reset all circuit breakers
    pub async fn reset_all_circuit_breakers(&self) {
        self.manager.reset_circuit_breakers().await;
    }
}

impl std::fmt::Debug for RpcClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RpcClient")
            .field("manager", &"RpcManager")
            .finish()
    }
}
