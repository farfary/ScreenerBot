//! RPC Manager - Main orchestrator for RPC operations
//!
//! Provides:
//! - Multi-provider management with automatic failover
//! - Rate limiting per provider
//! - Circuit breaker pattern
//! - Statistics collection
//! - Connection pooling

use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use chrono::Utc;
use tokio::sync::{Notify, OnceCell, RwLock};

use crate::rpc::{
    circuit_breaker::{CircuitBreakerConfig, CircuitBreakerManager},
    errors::RpcError,
    provider::config::ProviderConfig,
    rate_limiter::RateLimiterManager,
    stats::StatsManager,
    types::*,
};

// ============================================================
// RpcManager
// ============================================================

/// Main RPC manager orchestrating multi-provider operations
pub struct RpcManager {
    /// Provider configurations
    providers: RwLock<Vec<ProviderConfig>>,

    /// Provider states (runtime)
    provider_states: RwLock<HashMap<String, ProviderState>>,

    /// Rate limiter manager
    rate_limiters: Arc<RateLimiterManager>,

    /// Circuit breaker manager
    circuit_breakers: Arc<CircuitBreakerManager>,

    /// Statistics manager
    stats: Arc<RwLock<StatsManager>>,

    /// Shared HTTP client
    http_client: reqwest::Client,

    /// Round-robin index for provider selection
    round_robin_index: AtomicUsize,

    /// Selection strategy
    selection_strategy: RwLock<SelectionStrategy>,

    /// Default timeout
    default_timeout: Duration,

    /// Max retries
    max_retries: u32,

    /// Retry delay base (exponential backoff)
    retry_delay_base: Duration,

    /// Retry delay max
    retry_delay_max: Duration,

    /// Shutdown signal
    shutdown: Arc<Notify>,

    /// Force single provider mode
    force_single_provider: bool,
}

impl RpcManager {
    /// Create new RpcManager from configuration
    pub async fn new() -> Result<Self, String> {
        // Read RPC URLs from config
        let urls = crate::config::with_config(|cfg| cfg.rpc.urls.clone());

        if urls.is_empty() {
            return Err("No RPC URLs configured".to_string());
        }

        Self::from_urls(&urls).await
    }

    /// Create from URL list
    pub async fn from_urls(urls: &[String]) -> Result<Self, String> {
        if urls.is_empty() {
            return Err("No RPC URLs provided".to_string());
        }

        // Read config values
        let (
            request_timeout_secs,
            connection_timeout_secs,
            pool_connections_per_host,
            pool_idle_timeout_secs,
            max_retries,
            retry_base_delay_ms,
            retry_max_delay_ms,
            selection_strategy_str,
            circuit_breaker_enabled,
            circuit_breaker_failure_threshold,
            circuit_breaker_success_threshold,
            circuit_breaker_open_duration_secs,
            circuit_breaker_half_open_requests,
        ) = crate::config::with_config(|cfg| {
            (
                cfg.rpc.request_timeout_secs,
                cfg.rpc.connection_timeout_secs,
                cfg.rpc.pool_connections_per_host,
                cfg.rpc.pool_idle_timeout_secs,
                cfg.rpc.max_retries,
                cfg.rpc.retry_base_delay_ms,
                cfg.rpc.retry_max_delay_ms,
                cfg.rpc.selection_strategy.clone(),
                cfg.rpc.circuit_breaker_enabled,
                cfg.rpc.circuit_breaker_failure_threshold,
                cfg.rpc.circuit_breaker_success_threshold,
                cfg.rpc.circuit_breaker_open_duration_secs,
                cfg.rpc.circuit_breaker_half_open_requests,
            )
        });

        // Create shared HTTP client with connection pooling
        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(request_timeout_secs))
            .connect_timeout(Duration::from_secs(connection_timeout_secs))
            .pool_max_idle_per_host(pool_connections_per_host as usize)
            .pool_idle_timeout(Duration::from_secs(pool_idle_timeout_secs))
            .tcp_keepalive(Duration::from_secs(60))
            .tcp_nodelay(true)
            .build()
            .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

        // Create provider configs
        let mut providers = Vec::new();
        let mut provider_states = HashMap::new();

        for (i, url) in urls.iter().enumerate() {
            let config = ProviderConfig::from_url_with_priority(url, (i * 10) as u8);
            let state = ProviderState::new(config.id.clone(), url, config.kind, config.priority);
            provider_states.insert(config.id.clone(), state);
            providers.push(config);
        }

        // Initialize stats manager
        let stats = StatsManager::new().await?;

        // Register providers in stats
        for config in &providers {
            stats.register_provider(
                &config.id,
                &mask_url(&config.url),
                config.kind,
                config.priority,
            );
        }

        // Create circuit breaker config from application config
        let cb_config = if circuit_breaker_enabled {
            CircuitBreakerConfig {
                failure_threshold: circuit_breaker_failure_threshold,
                success_threshold: circuit_breaker_success_threshold,
                open_duration: Duration::from_secs(circuit_breaker_open_duration_secs),
                half_open_max_requests: circuit_breaker_half_open_requests,
                ..Default::default()
            }
        } else {
            // Disabled: very high thresholds
            CircuitBreakerConfig {
                failure_threshold: u32::MAX,
                ..Default::default()
            }
        };

        // Parse selection strategy
        let selection_strategy = SelectionStrategy::from_str(&selection_strategy_str);

        let manager = Self {
            providers: RwLock::new(providers),
            provider_states: RwLock::new(provider_states),
            rate_limiters: Arc::new(RateLimiterManager::from_config()),
            circuit_breakers: Arc::new(CircuitBreakerManager::with_config(cb_config)),
            stats: Arc::new(RwLock::new(stats)),
            http_client,
            round_robin_index: AtomicUsize::new(0),
            selection_strategy: RwLock::new(selection_strategy),
            default_timeout: Duration::from_secs(request_timeout_secs),
            max_retries,
            retry_delay_base: Duration::from_millis(retry_base_delay_ms),
            retry_delay_max: Duration::from_millis(retry_max_delay_ms),
            shutdown: Arc::new(Notify::new()),
            force_single_provider: false,
        };

        Ok(manager)
    }

    /// Start background services
    pub async fn start(&self) {
        let mut stats = self.stats.write().await;
        stats.start().await;
    }

    /// Stop background services
    pub async fn stop(&self) {
        self.shutdown.notify_waiters();
        let mut stats = self.stats.write().await;
        stats.stop().await;
    }

    /// Get provider count
    pub async fn provider_count(&self) -> usize {
        self.providers.read().await.len()
    }

    /// Get healthy provider count
    pub async fn healthy_provider_count(&self) -> usize {
        let states = self.provider_states.read().await;
        states.values().filter(|s| s.is_healthy()).count()
    }

    /// Select next provider based on strategy
    async fn select_provider(&self, excluded: &[String]) -> Option<ProviderConfig> {
        let providers = self.providers.read().await;
        let states = self.provider_states.read().await;
        let strategy = *self.selection_strategy.read().await;

        // Filter to available providers
        let available: Vec<_> = providers
            .iter()
            .filter(|p| {
                p.enabled
                    && !excluded.contains(&p.id)
                    && states.get(&p.id).map(|s| s.is_healthy()).unwrap_or(true)
            })
            .collect();

        if available.is_empty() {
            // Fallback: try any enabled provider
            return providers
                .iter()
                .find(|p| p.enabled && !excluded.contains(&p.id))
                .cloned();
        }

        match strategy {
            SelectionStrategy::RoundRobin => {
                let idx = self.round_robin_index.fetch_add(1, Ordering::SeqCst) % available.len();
                available.get(idx).cloned().cloned()
            }
            SelectionStrategy::Priority => {
                // Already sorted by priority in config
                available.first().cloned().cloned()
            }
            SelectionStrategy::LatencyBased => {
                // Select lowest latency
                available
                    .iter()
                    .min_by(|a, b| {
                        let lat_a = states.get(&a.id).map(|s| s.avg_latency_ms).unwrap_or(f64::MAX);
                        let lat_b = states.get(&b.id).map(|s| s.avg_latency_ms).unwrap_or(f64::MAX);
                        lat_a
                            .partial_cmp(&lat_b)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .cloned()
                    .cloned()
            }
            SelectionStrategy::Adaptive => {
                // Score based on: success rate (70%), latency (20%), priority (10%)
                available
                    .iter()
                    .max_by(|a, b| {
                        let score_a = self.calculate_provider_score(&states, &a.id, a.priority);
                        let score_b = self.calculate_provider_score(&states, &b.id, b.priority);
                        score_a
                            .partial_cmp(&score_b)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .cloned()
                    .cloned()
            }
        }
    }

    /// Calculate provider score for adaptive selection
    fn calculate_provider_score(
        &self,
        states: &HashMap<String, ProviderState>,
        provider_id: &str,
        priority: u8,
    ) -> f64 {
        let state = match states.get(provider_id) {
            Some(s) => s,
            None => return 50.0, // Default score for unknown
        };

        // Success rate component (0-70 points)
        let success_score = state.success_rate() * 0.7;

        // Latency component (0-20 points, inverse - lower is better)
        let latency_score = if state.avg_latency_ms > 0.0 {
            (1000.0 / state.avg_latency_ms).min(20.0)
        } else {
            20.0
        };

        // Priority component (0-10 points, lower priority number = higher score)
        let priority_score = 10.0 - (priority as f64 / 25.5);

        success_score + latency_score + priority_score
    }

    /// Execute raw JSON-RPC request with automatic retries and failover
    pub async fn execute_raw(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, RpcError> {
        let rpc_method = RpcMethod::from_str(method);
        let mut last_error: Option<RpcError> = None;
        let mut tried_providers: Vec<String> = Vec::new();

        for retry in 0..=self.max_retries {
            // Select provider
            let provider = match self.select_provider(&tried_providers).await {
                Some(p) => p,
                None => {
                    return Err(RpcError::NoProvidersAvailable {
                        last_error: last_error.map(|e| e.to_string()),
                    });
                }
            };

            let provider_id = provider.id.clone();
            tried_providers.push(provider_id.clone());

            // Check circuit breaker
            let breaker = self.circuit_breakers.get_breaker(&provider_id).await;
            if let Err(wait_time) = breaker.can_execute().await {
                last_error = Some(RpcError::CircuitOpen {
                    provider_id: provider_id.clone(),
                    retry_after: wait_time,
                });
                continue;
            }

            // Acquire rate limit
            let limiter = self
                .rate_limiters
                .get_limiter(
                    &provider_id,
                    Some(provider.effective_rate_limit()),
                    provider.kind,
                )
                .await;
            limiter.acquire(&rpc_method).await;

            // Execute request
            let request_start = Instant::now();
            match self.execute_single(&provider, method, &params).await {
                Ok(result) => {
                    let latency_ms = request_start.elapsed().as_millis() as u64;

                    // Record success
                    breaker.record_success().await;
                    limiter.record_success();
                    self.update_provider_state(&provider_id, true, latency_ms, None)
                        .await;

                    // Record stats
                    self.record_call_result(RpcCallResult {
                        provider_id,
                        method: rpc_method,
                        success: true,
                        latency_ms,
                        error: None,
                        timestamp: Utc::now(),
                        retry_count: retry,
                        was_rate_limited: false,
                    })
                    .await;

                    return Ok(result);
                }
                Err(e) => {
                    let latency_ms = request_start.elapsed().as_millis() as u64;
                    let is_rate_limited = e.is_rate_limited();

                    // Handle error
                    if is_rate_limited {
                        limiter.record_429(e.retry_after()).await;
                    } else {
                        breaker.record_failure(&e.to_string(), is_rate_limited).await;
                    }

                    self.update_provider_state(&provider_id, false, latency_ms, Some(&e.to_string()))
                        .await;

                    // Record stats
                    self.record_call_result(RpcCallResult {
                        provider_id: provider_id.clone(),
                        method: rpc_method.clone(),
                        success: false,
                        latency_ms,
                        error: Some(e.to_string()),
                        timestamp: Utc::now(),
                        retry_count: retry,
                        was_rate_limited: is_rate_limited,
                    })
                    .await;

                    last_error = Some(e.clone());

                    // Don't retry non-retryable errors
                    if !e.is_retryable() {
                        return Err(e);
                    }

                    // Exponential backoff
                    if retry < self.max_retries {
                        let delay = std::cmp::min(
                            self.retry_delay_base * 2u32.pow(retry),
                            self.retry_delay_max,
                        );
                        tokio::time::sleep(delay).await;
                    }
                }
            }
        }

        Err(last_error.unwrap_or(RpcError::NoProvidersAvailable { last_error: None }))
    }

    /// Execute single request to specific provider
    async fn execute_single(
        &self,
        provider: &ProviderConfig,
        method: &str,
        params: &serde_json::Value,
    ) -> Result<serde_json::Value, RpcError> {
        let request_body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": method,
            "params": params
        });

        let response = self
            .http_client
            .post(&provider.url)
            .header("Content-Type", "application/json")
            .timeout(Duration::from_secs(provider.timeout_secs))
            .json(&request_body)
            .send()
            .await?;

        let status = response.status();
        let body = response.text().await?;

        // Check HTTP status
        if !status.is_success() {
            return Err(RpcError::from_http_response(
                status.as_u16(),
                &body,
                &provider.id,
            ));
        }

        // Parse JSON-RPC response
        let json: serde_json::Value =
            serde_json::from_str(&body).map_err(|e| RpcError::InvalidResponse {
                message: format!("Invalid JSON: {}", e),
            })?;

        // Check for JSON-RPC error
        if let Some(error) = json.get("error") {
            let code = error.get("code").and_then(|c| c.as_i64()).unwrap_or(-1);
            let message = error
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("Unknown error");
            let data = error.get("data").map(|d| d.to_string());

            return Err(RpcError::from_jsonrpc_error(code, message, data.as_deref()));
        }

        // Extract result
        json.get("result")
            .cloned()
            .ok_or_else(|| RpcError::InvalidResponse {
                message: "Missing result field".to_string(),
            })
    }

    /// Update provider state after request
    async fn update_provider_state(
        &self,
        provider_id: &str,
        success: bool,
        latency_ms: u64,
        error: Option<&str>,
    ) {
        let mut states = self.provider_states.write().await;
        if let Some(state) = states.get_mut(provider_id) {
            state.total_calls += 1;

            if success {
                state.consecutive_failures = 0;
                state.consecutive_successes += 1;
                state.last_success = Some(Utc::now());

                // Update average latency (exponential moving average)
                if state.avg_latency_ms == 0.0 {
                    state.avg_latency_ms = latency_ms as f64;
                } else {
                    state.avg_latency_ms = state.avg_latency_ms * 0.9 + latency_ms as f64 * 0.1;
                }
            } else {
                state.total_errors += 1;
                state.consecutive_failures += 1;
                state.consecutive_successes = 0;
                state.last_failure = Some(Utc::now());
                state.last_error = error.map(String::from);
            }
        }
    }

    /// Record call result to stats
    async fn record_call_result(&self, result: RpcCallResult) {
        let stats = self.stats.read().await;
        stats.record_call(result).await;
    }

    /// Get all provider states
    pub async fn get_provider_states(&self) -> Vec<ProviderState> {
        let states = self.provider_states.read().await;
        states.values().cloned().collect()
    }

    /// Get provider state by ID
    pub async fn get_provider_state(&self, provider_id: &str) -> Option<ProviderState> {
        let states = self.provider_states.read().await;
        states.get(provider_id).cloned()
    }

    /// Get stats snapshot
    pub async fn get_stats(&self) -> crate::rpc::stats::RpcStatsResponse {
        let stats = self.stats.read().await;
        let provider_count = self.provider_count().await;
        let healthy_count = self.healthy_provider_count().await;
        stats.get_stats_response(provider_count, healthy_count)
    }

    /// Get first (primary) provider URL
    pub async fn primary_url(&self) -> Option<String> {
        let providers = self.providers.read().await;
        providers.first().map(|p| p.url.clone())
    }

    /// Get all provider URLs
    pub async fn all_urls(&self) -> Vec<String> {
        let providers = self.providers.read().await;
        providers.iter().map(|p| p.url.clone()).collect()
    }

    /// Set selection strategy
    pub async fn set_selection_strategy(&self, strategy: SelectionStrategy) {
        let mut current = self.selection_strategy.write().await;
        *current = strategy;
    }

    /// Get current selection strategy
    pub async fn get_selection_strategy(&self) -> SelectionStrategy {
        *self.selection_strategy.read().await
    }

    /// Force reset all circuit breakers
    pub async fn reset_circuit_breakers(&self) {
        self.circuit_breakers.reset_all().await;
    }

    /// Force reset all rate limiters
    pub async fn reset_rate_limiters(&self) {
        self.rate_limiters.reset_all().await;
    }

    /// Get HTTP client reference
    pub fn http_client(&self) -> &reqwest::Client {
        &self.http_client
    }

    /// Cleanup old stats
    pub async fn cleanup_stats(&self, retention_hours: u64) {
        let stats = self.stats.read().await;
        stats.cleanup(retention_hours);
    }

    /// Get circuit breaker manager reference
    pub fn circuit_breakers(&self) -> &Arc<CircuitBreakerManager> {
        &self.circuit_breakers
    }

    /// Get rate limiter manager reference
    pub fn rate_limiters(&self) -> &Arc<RateLimiterManager> {
        &self.rate_limiters
    }

    /// Check if any provider is healthy
    pub async fn has_healthy_provider(&self) -> bool {
        self.healthy_provider_count().await > 0
    }

    /// Get provider configs
    pub async fn get_provider_configs(&self) -> Vec<ProviderConfig> {
        self.providers.read().await.clone()
    }

    /// Enable/disable a provider
    pub async fn set_provider_enabled(&self, provider_id: &str, enabled: bool) {
        let mut providers = self.providers.write().await;
        if let Some(provider) = providers.iter_mut().find(|p| p.id == provider_id) {
            provider.enabled = enabled;
        }

        let mut states = self.provider_states.write().await;
        if let Some(state) = states.get_mut(provider_id) {
            state.enabled = enabled;
        }
    }
}

impl std::fmt::Debug for RpcManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RpcManager")
            .field("max_retries", &self.max_retries)
            .field("default_timeout", &self.default_timeout)
            .field("force_single_provider", &self.force_single_provider)
            .finish()
    }
}

// ============================================================
// Global Singleton
// ============================================================

static RPC_MANAGER: OnceCell<Arc<RpcManager>> = OnceCell::const_new();

/// Initialize global RPC manager
pub async fn init_rpc_manager() -> Result<Arc<RpcManager>, String> {
    RPC_MANAGER
        .get_or_try_init(|| async {
            let manager = RpcManager::new().await?;
            manager.start().await;
            Ok(Arc::new(manager))
        })
        .await
        .cloned()
}

/// Get global RPC manager (returns None if not initialized)
pub fn get_rpc_manager() -> Option<Arc<RpcManager>> {
    RPC_MANAGER.get().cloned()
}

/// Get or initialize global RPC manager
pub async fn get_or_init_rpc_manager() -> Result<Arc<RpcManager>, String> {
    init_rpc_manager().await
}
