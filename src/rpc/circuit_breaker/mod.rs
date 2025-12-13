//! Circuit breaker pattern for RPC provider failover
//!
//! Provides automatic failover when providers become unhealthy.
//! States:
//! - Closed: Normal operation, requests allowed
//! - Open: Provider failing, requests blocked
//! - Half-Open: Testing recovery, limited requests allowed

pub mod config;
pub mod state;

pub use config::CircuitBreakerConfig;
pub use state::{CircuitBreakerStatus, ProviderCircuitBreaker};

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

use crate::rpc::types::CircuitState;

/// Manager for all provider circuit breakers
pub struct CircuitBreakerManager {
    /// Per-provider circuit breakers
    breakers: RwLock<HashMap<String, Arc<ProviderCircuitBreaker>>>,

    /// Default configuration for new circuit breakers
    default_config: CircuitBreakerConfig,
}

impl CircuitBreakerManager {
    /// Create new manager with default config
    pub fn new() -> Self {
        Self {
            breakers: RwLock::new(HashMap::new()),
            default_config: CircuitBreakerConfig::default(),
        }
    }

    /// Create with custom default config
    pub fn with_config(config: CircuitBreakerConfig) -> Self {
        Self {
            breakers: RwLock::new(HashMap::new()),
            default_config: config,
        }
    }

    /// Get or create circuit breaker for a provider
    pub async fn get_breaker(&self, provider_id: &str) -> Arc<ProviderCircuitBreaker> {
        // Fast path
        {
            let breakers = self.breakers.read().await;
            if let Some(breaker) = breakers.get(provider_id) {
                return breaker.clone();
            }
        }

        // Slow path - create new breaker
        let mut breakers = self.breakers.write().await;

        // Double-check
        if let Some(breaker) = breakers.get(provider_id) {
            return breaker.clone();
        }

        let breaker = Arc::new(ProviderCircuitBreaker::new(
            provider_id,
            self.default_config.clone(),
        ));

        breakers.insert(provider_id.to_string(), breaker.clone());
        breaker
    }

    /// Get circuit breaker with custom config
    pub async fn get_breaker_with_config(
        &self,
        provider_id: &str,
        config: CircuitBreakerConfig,
    ) -> Arc<ProviderCircuitBreaker> {
        let mut breakers = self.breakers.write().await;

        let breaker = Arc::new(ProviderCircuitBreaker::new(provider_id, config));
        breakers.insert(provider_id.to_string(), breaker.clone());
        breaker
    }

    /// Remove circuit breaker
    pub async fn remove_breaker(&self, provider_id: &str) {
        let mut breakers = self.breakers.write().await;
        breakers.remove(provider_id);
    }

    /// Check if provider is available (circuit closed or half-open)
    pub async fn is_available(&self, provider_id: &str) -> bool {
        let breakers = self.breakers.read().await;
        if let Some(breaker) = breakers.get(provider_id) {
            breaker.can_execute().await.is_ok()
        } else {
            true // No breaker = available
        }
    }

    /// Get all healthy provider IDs
    pub async fn get_healthy_providers(&self) -> Vec<String> {
        let breakers = self.breakers.read().await;
        let mut healthy = Vec::new();

        for (id, breaker) in breakers.iter() {
            if breaker.can_execute().await.is_ok() {
                healthy.push(id.clone());
            }
        }

        healthy
    }

    /// Get all unhealthy provider IDs with time until retry
    pub async fn get_unhealthy_providers(&self) -> Vec<(String, Duration)> {
        let breakers = self.breakers.read().await;
        let mut unhealthy = Vec::new();

        for (id, breaker) in breakers.iter() {
            if let Err(wait_time) = breaker.can_execute().await {
                unhealthy.push((id.clone(), wait_time));
            }
        }

        unhealthy
    }

    /// Force all circuits closed (reset)
    pub async fn reset_all(&self) {
        let breakers = self.breakers.read().await;
        for breaker in breakers.values() {
            breaker.force_close().await;
        }
    }

    /// Get status of all circuit breakers
    pub async fn get_all_status(&self) -> Vec<CircuitBreakerStatus> {
        let breakers = self.breakers.read().await;
        let mut statuses = Vec::new();

        for breaker in breakers.values() {
            statuses.push(breaker.status().await);
        }

        statuses
    }

    /// Get status for specific provider
    pub async fn get_status(&self, provider_id: &str) -> Option<CircuitBreakerStatus> {
        let breakers = self.breakers.read().await;
        if let Some(breaker) = breakers.get(provider_id) {
            Some(breaker.status().await)
        } else {
            None
        }
    }

    /// Count of open circuits
    pub async fn open_circuit_count(&self) -> usize {
        let breakers = self.breakers.read().await;
        let mut count = 0;

        for breaker in breakers.values() {
            if breaker.current_state().await == CircuitState::Open {
                count += 1;
            }
        }

        count
    }

    /// Total breaker count
    pub async fn breaker_count(&self) -> usize {
        self.breakers.read().await.len()
    }
}

impl Default for CircuitBreakerManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_manager_creation() {
        let manager = CircuitBreakerManager::new();
        assert_eq!(manager.breaker_count().await, 0);
    }

    #[tokio::test]
    async fn test_get_breaker() {
        let manager = CircuitBreakerManager::new();

        let breaker1 = manager.get_breaker("provider1").await;
        let breaker2 = manager.get_breaker("provider1").await;

        // Should be same breaker
        assert_eq!(breaker1.provider_id(), breaker2.provider_id());
        assert_eq!(manager.breaker_count().await, 1);
    }

    #[tokio::test]
    async fn test_availability() {
        let manager = CircuitBreakerManager::new();

        // Non-existent provider should be available
        assert!(manager.is_available("unknown").await);

        // Create breaker
        let breaker = manager.get_breaker("test").await;
        assert!(manager.is_available("test").await);

        // Force open
        breaker.force_open("test error").await;
        assert!(!manager.is_available("test").await);
    }

    #[tokio::test]
    async fn test_healthy_unhealthy_lists() {
        let config = CircuitBreakerConfig {
            failure_threshold: 1,
            min_state_duration: Duration::from_millis(1),
            ..Default::default()
        };
        let manager = CircuitBreakerManager::with_config(config);

        // Create breakers
        let _healthy = manager.get_breaker("healthy").await;
        let unhealthy = manager.get_breaker("unhealthy").await;

        // Wait for min_state_duration to pass
        tokio::time::sleep(Duration::from_millis(5)).await;

        // Trip one circuit
        unhealthy.record_failure("error", false).await;

        let healthy_list = manager.get_healthy_providers().await;
        let unhealthy_list = manager.get_unhealthy_providers().await;

        assert!(healthy_list.contains(&"healthy".to_string()));
        assert!(unhealthy_list.iter().any(|(id, _)| id == "unhealthy"));
    }
}
