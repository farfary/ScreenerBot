//! Token update rate limiter — prevents API overload during batch token updates.

use crate::apis::dexscreener::{
    RATE_LIMIT_LATEST_BOOSTS_PER_MINUTE as DEX_BOOSTS_PER_MINUTE,
    RATE_LIMIT_LATEST_PROFILES_PER_MINUTE as DEX_PROFILES_PER_MINUTE,
    RATE_LIMIT_TOKEN_BATCH_PER_MINUTE as DEX_BATCH_PER_MINUTE,
    RATE_LIMIT_TOKEN_POOLS_PER_MINUTE as DEX_POOLS_PER_MINUTE,
};
use crate::apis::geckoterminal::RATE_LIMIT_PER_MINUTE as GECKO_DEFAULT_PER_MINUTE;
use crate::apis::rugcheck::RATE_LIMIT_PER_MINUTE as RUG_DEFAULT_PER_MINUTE;
use crate::config::with_config;
use crate::tokens::types::TokenError;
use std::sync::Arc;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

// ============================================================================
// RATE LIMIT COORDINATOR
// ============================================================================

/// Global rate limit coordinator for all API sources
///
/// Uses separate semaphores per endpoint to prevent different operations from blocking each other:
/// - DexScreener token batch (market data): 300/min
/// - DexScreener profiles (discovery): 60/min
/// - DexScreener boosts (discovery): 60/min
/// - DexScreener token pools (full pool fetch): 300/min
/// - GeckoTerminal: 30/min
/// - Rugcheck: 60/min
pub struct RateLimitCoordinator {
    // DexScreener endpoints (separate limits per endpoint)
    dexscreener_batch_sem: Arc<Semaphore>,
    dexscreener_profiles_sem: Arc<Semaphore>,
    dexscreener_boosts_sem: Arc<Semaphore>,
    dexscreener_pools_sem: Arc<Semaphore>,
    dexscreener_batch_budget: usize,
    dexscreener_profiles_budget: usize,
    dexscreener_boosts_budget: usize,
    dexscreener_pools_budget: usize,
    // Other API endpoints
    geckoterminal_sem: Arc<Semaphore>,
    rugcheck_sem: Arc<Semaphore>,
    geckoterminal_budget: usize,
    rugcheck_budget: usize,
}

impl RateLimitCoordinator {
    pub fn new() -> Self {
        // Read limits from config; fall back to API defaults if unset (0)
        let (gecko_limit, rug_limit) = with_config(|cfg| {
            let s = &cfg.tokens.sources;
            let gecko = if s.geckoterminal.rate_limit_per_minute == 0 {
                GECKO_DEFAULT_PER_MINUTE
            } else {
                s.geckoterminal.rate_limit_per_minute as usize
            };
            let rug = if s.rugcheck.rate_limit_per_minute == 0 {
                RUG_DEFAULT_PER_MINUTE
            } else {
                s.rugcheck.rate_limit_per_minute as usize
            };
            (gecko, rug)
        });

        Self {
            // DexScreener endpoints with separate limits
            dexscreener_batch_sem: Arc::new(Semaphore::new(DEX_BATCH_PER_MINUTE)),
            dexscreener_profiles_sem: Arc::new(Semaphore::new(DEX_PROFILES_PER_MINUTE)),
            dexscreener_boosts_sem: Arc::new(Semaphore::new(DEX_BOOSTS_PER_MINUTE)),
            dexscreener_pools_sem: Arc::new(Semaphore::new(DEX_POOLS_PER_MINUTE)),
            dexscreener_batch_budget: DEX_BATCH_PER_MINUTE,
            dexscreener_profiles_budget: DEX_PROFILES_PER_MINUTE,
            dexscreener_boosts_budget: DEX_BOOSTS_PER_MINUTE,
            dexscreener_pools_budget: DEX_POOLS_PER_MINUTE,
            // Other API endpoints
            geckoterminal_sem: Arc::new(Semaphore::new(gecko_limit)),
            rugcheck_sem: Arc::new(Semaphore::new(rug_limit)),
            geckoterminal_budget: gecko_limit,
            rugcheck_budget: rug_limit,
        }
    }

    /// Acquire permit for DexScreener token batch API call (market data updates)
    /// Rate limit: 300/min
    pub async fn acquire_dexscreener_batch(&self) -> Result<OwnedSemaphorePermit, TokenError> {
        self.dexscreener_batch_sem
            .clone()
            .acquire_owned()
            .await
            .map_err(|e| TokenError::RateLimit {
                source: "DexScreener-Batch".to_owned(),
                message: format!("Failed to acquire permit: {e}"),
            })
    }

    /// Acquire permit for DexScreener profiles API call (discovery)
    /// Rate limit: 60/min
    pub async fn acquire_dexscreener_profiles(&self) -> Result<OwnedSemaphorePermit, TokenError> {
        self.dexscreener_profiles_sem
            .clone()
            .acquire_owned()
            .await
            .map_err(|e| TokenError::RateLimit {
                source: "DexScreener-Profiles".to_owned(),
                message: format!("Failed to acquire permit: {e}"),
            })
    }

    /// Acquire permit for DexScreener boosts API call (discovery)
    /// Rate limit: 60/min
    pub async fn acquire_dexscreener_boosts(&self) -> Result<OwnedSemaphorePermit, TokenError> {
        self.dexscreener_boosts_sem
            .clone()
            .acquire_owned()
            .await
            .map_err(|e| TokenError::RateLimit {
                source: "DexScreener-Boosts".to_owned(),
                message: format!("Failed to acquire permit: {e}"),
            })
    }

    /// Acquire permit for DexScreener full pool fetch API call
    /// Rate limit: 300/min
    pub async fn acquire_dexscreener_pools(&self) -> Result<OwnedSemaphorePermit, TokenError> {
        self.dexscreener_pools_sem
            .clone()
            .acquire_owned()
            .await
            .map_err(|e| TokenError::RateLimit {
                source: "DexScreener-Pools".to_owned(),
                message: format!("Failed to acquire permit: {e}"),
            })
    }

    /// Acquire permit for GeckoTerminal API call
    pub async fn acquire_geckoterminal(&self) -> Result<OwnedSemaphorePermit, TokenError> {
        self.geckoterminal_sem
            .clone()
            .acquire_owned()
            .await
            .map_err(|e| TokenError::RateLimit {
                source: "GeckoTerminal".to_owned(),
                message: format!("Failed to acquire permit: {e}"),
            })
    }

    /// Acquire permit for Rugcheck API call
    pub async fn acquire_rugcheck(&self) -> Result<OwnedSemaphorePermit, TokenError> {
        self.rugcheck_sem
            .clone()
            .acquire_owned()
            .await
            .map_err(|e| TokenError::RateLimit {
                source: "Rugcheck".to_owned(),
                message: format!("Failed to acquire permit: {e}"),
            })
    }

    /// Refill all semaphores (called every minute)
    pub fn refill_all(&self) {
        // DexScreener endpoints
        if self.dexscreener_batch_budget > 0 {
            self.dexscreener_batch_sem
                .add_permits(self.dexscreener_batch_budget);
        }
        if self.dexscreener_profiles_budget > 0 {
            self.dexscreener_profiles_sem
                .add_permits(self.dexscreener_profiles_budget);
        }
        if self.dexscreener_boosts_budget > 0 {
            self.dexscreener_boosts_sem
                .add_permits(self.dexscreener_boosts_budget);
        }
        if self.dexscreener_pools_budget > 0 {
            self.dexscreener_pools_sem
                .add_permits(self.dexscreener_pools_budget);
        }
        // Other API endpoints
        if self.geckoterminal_budget > 0 {
            self.geckoterminal_sem
                .add_permits(self.geckoterminal_budget);
        }
        if self.rugcheck_budget > 0 {
            self.rugcheck_sem.add_permits(self.rugcheck_budget);
        }
    }
}

impl Default for RateLimitCoordinator {
    fn default() -> Self {
        Self::new()
    }
}
