//! Rate limiting for RPC providers
//!
//! Uses Governor (GCRA algorithm) for efficient, fair rate limiting.
//! Features:
//! - Per-provider rate limits
//! - Adaptive backoff on 429 errors
//! - Method cost weighting
//! - Gradual recovery after rate limiting

pub mod adaptive;
pub mod provider;

pub use adaptive::{ExponentialBackoff, SlidingWindowTracker};
pub use provider::ProviderRateLimiter;

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::rpc::types::ProviderKind;

/// Manager for all provider rate limiters
pub struct RateLimiterManager {
    /// Per-provider rate limiters
    limiters: RwLock<HashMap<String, Arc<ProviderRateLimiter>>>,

    /// Default rate limits per provider kind
    default_rates: HashMap<ProviderKind, u32>,

    /// Global backoff multiplier
    backoff_multiplier: f64,

    /// Minimum rate after backoff
    min_rate: u32,
}

impl RateLimiterManager {
    /// Create new manager with default settings
    pub fn new() -> Self {
        let mut default_rates = HashMap::new();
        for kind in [
            ProviderKind::Helius,
            ProviderKind::QuickNode,
            ProviderKind::Triton,
            ProviderKind::Alchemy,
            ProviderKind::GetBlock,
            ProviderKind::Shyft,
            ProviderKind::Public,
            ProviderKind::Unknown,
        ] {
            default_rates.insert(kind, kind.default_rate_limit());
        }

        Self {
            limiters: RwLock::new(HashMap::new()),
            default_rates,
            backoff_multiplier: 0.5,
            min_rate: 1,
        }
    }

    /// Create from application config
    pub fn from_config() -> Self {
        let (
            default_rate_limit,
            helius_rate_limit,
            quicknode_rate_limit,
            triton_rate_limit,
            public_rate_limit,
        ) = crate::config::with_config(|cfg| {
            (
                cfg.rpc.default_rate_limit,
                cfg.rpc.helius_rate_limit,
                cfg.rpc.quicknode_rate_limit,
                cfg.rpc.triton_rate_limit,
                cfg.rpc.public_rate_limit,
            )
        });

        let mut default_rates = HashMap::new();
        default_rates.insert(ProviderKind::Helius, helius_rate_limit);
        default_rates.insert(ProviderKind::QuickNode, quicknode_rate_limit);
        default_rates.insert(ProviderKind::Triton, triton_rate_limit);
        default_rates.insert(ProviderKind::Alchemy, quicknode_rate_limit); // Similar to QuickNode
        default_rates.insert(ProviderKind::GetBlock, quicknode_rate_limit);
        default_rates.insert(ProviderKind::Shyft, quicknode_rate_limit);
        default_rates.insert(ProviderKind::Public, public_rate_limit);
        default_rates.insert(ProviderKind::Unknown, default_rate_limit);

        Self {
            limiters: RwLock::new(HashMap::new()),
            default_rates,
            backoff_multiplier: 0.5,
            min_rate: 1,
        }
    }

    /// Create with custom settings
    pub fn with_settings(backoff_multiplier: f64, min_rate: u32) -> Self {
        let mut manager = Self::new();
        manager.backoff_multiplier = backoff_multiplier.clamp(0.1, 0.9);
        manager.min_rate = min_rate.max(1);
        manager
    }

    /// Set default rate for a provider kind
    pub fn set_default_rate(&mut self, kind: ProviderKind, rate: u32) {
        self.default_rates.insert(kind, rate);
    }

    /// Get or create rate limiter for a provider
    pub async fn get_limiter(
        &self,
        provider_id: &str,
        rate_override: Option<u32>,
        kind: ProviderKind,
    ) -> Arc<ProviderRateLimiter> {
        // Fast path: check if limiter exists
        {
            let limiters = self.limiters.read().await;
            if let Some(limiter) = limiters.get(provider_id) {
                return limiter.clone();
            }
        }

        // Slow path: create new limiter
        let mut limiters = self.limiters.write().await;

        // Double-check after acquiring write lock
        if let Some(limiter) = limiters.get(provider_id) {
            return limiter.clone();
        }

        // Determine rate limit
        let rate = rate_override.unwrap_or_else(|| {
            self.default_rates.get(&kind).copied().unwrap_or(10)
        });

        let limiter = Arc::new(ProviderRateLimiter::with_backoff(
            provider_id,
            rate,
            self.backoff_multiplier,
            self.min_rate,
        ));

        limiters.insert(provider_id.to_string(), limiter.clone());
        limiter
    }

    /// Remove rate limiter for a provider
    pub async fn remove_limiter(&self, provider_id: &str) {
        let mut limiters = self.limiters.write().await;
        limiters.remove(provider_id);
    }

    /// Reset all rate limiters
    pub async fn reset_all(&self) {
        let limiters = self.limiters.read().await;
        for limiter in limiters.values() {
            limiter.reset();
        }
    }

    /// Get status of all rate limiters
    pub async fn get_status(&self) -> Vec<RateLimiterStatus> {
        let limiters = self.limiters.read().await;

        limiters
            .values()
            .map(|limiter| RateLimiterStatus {
                provider_id: limiter.provider_id().to_string(),
                base_rate: limiter.base_rate(),
                current_rate: limiter.current_rate(),
                is_backing_off: limiter.is_backing_off(),
            })
            .collect()
    }

    /// Get limiter count
    pub async fn limiter_count(&self) -> usize {
        self.limiters.read().await.len()
    }
}

impl Default for RateLimiterManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Status of a rate limiter
#[derive(Debug, Clone)]
pub struct RateLimiterStatus {
    pub provider_id: String,
    pub base_rate: u32,
    pub current_rate: u32,
    pub is_backing_off: bool,
}
