// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Token update rate limiter — prevents API overload during batch token updates.

use crate::apis::dexscreener::{
    RATE_LIMIT_LATEST_BOOSTS_PER_MINUTE as DEX_BOOSTS_PER_MINUTE,
    RATE_LIMIT_LATEST_PROFILES_PER_MINUTE as DEX_PROFILES_PER_MINUTE,
    RATE_LIMIT_TOKEN_BATCH_PER_MINUTE as DEX_BATCH_PER_MINUTE,
    RATE_LIMIT_TOKEN_POOLS_PER_MINUTE as DEX_POOLS_PER_MINUTE,
};
use crate::apis::geckoterminal::RATE_LIMIT_PER_MINUTE as GECKO_DEFAULT_PER_MINUTE;
use crate::apis::rugcheck::RATE_LIMIT_PER_MINUTE as RUG_DEFAULT_PER_MINUTE;
use crate::chains::{adapter_for, enabled_chains, ChainId};
use crate::config::with_config;
use crate::tokens::Error;
use std::sync::Arc;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

// ============================================================================
// RATE LIMIT COORDINATOR
// ============================================================================

/// One chain's share of the per-minute provider budgets
///
/// Every enabled chain gets its own coordinator; each provider's per-minute total is
/// split across the chains that use that provider (see [`share`]), so the shares of
/// all chains add up to one budget. Separate semaphores per endpoint keep different
/// operations from blocking each other. Totals before the split:
/// - DexScreener token batch (market data): 300/min
/// - DexScreener profiles (discovery): 60/min
/// - DexScreener boosts (discovery): 60/min
/// - DexScreener token pools (full pool fetch): 300/min
/// - GeckoTerminal: 30/min
/// - Rugcheck: 60/min
pub struct RateLimitCoordinator {
    // DexScreener endpoints (separate limits per endpoint)
    dexscreener_batch: ProviderBudget,
    dexscreener_profiles: ProviderBudget,
    dexscreener_boosts: ProviderBudget,
    dexscreener_pools: ProviderBudget,
    // Other API endpoints
    geckoterminal: ProviderBudget,
    rugcheck: ProviderBudget,
}

/// Per-minute permit counts for every endpoint of one coordinator.
struct PerMinuteBudgets {
    dexscreener_batch: usize,
    dexscreener_profiles: usize,
    dexscreener_boosts: usize,
    dexscreener_pools: usize,
    geckoterminal: usize,
    rugcheck: usize,
}

/// A participant's share of `total`: an equal split, with the remainder going to the
/// earliest participants, so the shares always sum to `total`. A chain that is not a
/// participant (`position` is `None`) gets nothing.
fn share(total: usize, participants: usize, position: Option<usize>) -> usize {
    match position {
        Some(position) if participants > 0 => {
            total / participants + usize::from(position < total % participants)
        }
        _ => 0,
    }
}

impl RateLimitCoordinator {
    /// The coordinator for `chain`, holding its fair share of every provider budget
    ///
    /// DexScreener and GeckoTerminal are shared by every enabled chain; Rugcheck only by
    /// the enabled chains it has reports for. With a single enabled chain every share is
    /// the full budget.
    pub fn for_chain(chain: ChainId) -> Self {
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

        let chains = enabled_chains();
        let chain_position = chains.iter().position(|&enabled| enabled == chain);
        let shared = |total| share(total, chains.len(), chain_position);

        let rugcheck_chains: Vec<ChainId> = chains
            .iter()
            .copied()
            .filter(|&enabled| adapter_for(enabled).has_rugcheck_reports())
            .collect();
        let rugcheck_position = rugcheck_chains.iter().position(|&enabled| enabled == chain);

        Self::with_budgets(PerMinuteBudgets {
            dexscreener_batch: shared(DEX_BATCH_PER_MINUTE),
            dexscreener_profiles: shared(DEX_PROFILES_PER_MINUTE),
            dexscreener_boosts: shared(DEX_BOOSTS_PER_MINUTE),
            dexscreener_pools: shared(DEX_POOLS_PER_MINUTE),
            geckoterminal: shared(gecko_limit),
            rugcheck: share(rug_limit, rugcheck_chains.len(), rugcheck_position),
        })
    }

    fn with_budgets(budgets: PerMinuteBudgets) -> Self {
        Self {
            dexscreener_batch: ProviderBudget::new("DexScreener-Batch", budgets.dexscreener_batch),
            dexscreener_profiles: ProviderBudget::new(
                "DexScreener-Profiles",
                budgets.dexscreener_profiles,
            ),
            dexscreener_boosts: ProviderBudget::new(
                "DexScreener-Boosts",
                budgets.dexscreener_boosts,
            ),
            dexscreener_pools: ProviderBudget::new("DexScreener-Pools", budgets.dexscreener_pools),
            geckoterminal: ProviderBudget::new("GeckoTerminal", budgets.geckoterminal),
            rugcheck: ProviderBudget::new("Rugcheck", budgets.rugcheck),
        }
    }

    /// Acquire permit for DexScreener token batch API call (market data updates)
    /// Rate limit: 300/min
    pub async fn acquire_dexscreener_batch(&self) -> Result<OwnedSemaphorePermit, Error> {
        self.dexscreener_batch.acquire().await
    }

    /// Acquire permit for DexScreener profiles API call (discovery)
    /// Rate limit: 60/min
    pub async fn acquire_dexscreener_profiles(&self) -> Result<OwnedSemaphorePermit, Error> {
        self.dexscreener_profiles.acquire().await
    }

    /// Acquire permit for DexScreener boosts API call (discovery)
    /// Rate limit: 60/min
    pub async fn acquire_dexscreener_boosts(&self) -> Result<OwnedSemaphorePermit, Error> {
        self.dexscreener_boosts.acquire().await
    }

    /// Acquire permit for DexScreener full pool fetch API call
    /// Rate limit: 300/min
    pub async fn acquire_dexscreener_pools(&self) -> Result<OwnedSemaphorePermit, Error> {
        self.dexscreener_pools.acquire().await
    }

    /// Acquire permit for GeckoTerminal API call
    pub async fn acquire_geckoterminal(&self) -> Result<OwnedSemaphorePermit, Error> {
        self.geckoterminal.acquire().await
    }

    /// Acquire permit for Rugcheck API call
    pub async fn acquire_rugcheck(&self) -> Result<OwnedSemaphorePermit, Error> {
        self.rugcheck.acquire().await
    }

    /// Top every semaphore back up to its per-minute budget (called every minute)
    ///
    /// Unused permits do not accumulate across minutes: a top-up only restores what was
    /// consumed. Permits handed back by failed in-flight calls can lift availability briefly
    /// above one minute's budget, never above what a full refill would grant.
    pub fn top_up_all(&self) {
        // DexScreener endpoints
        self.dexscreener_batch.top_up();
        self.dexscreener_profiles.top_up();
        self.dexscreener_boosts.top_up();
        self.dexscreener_pools.top_up();
        // Other API endpoints
        self.geckoterminal.top_up();
        self.rugcheck.top_up();
    }
}

/// One endpoint's per-minute permit pool.
struct ProviderBudget {
    provider: &'static str,
    per_minute: usize,
    permits: Arc<Semaphore>,
}

impl ProviderBudget {
    fn new(provider: &'static str, per_minute: usize) -> Self {
        Self {
            provider,
            per_minute,
            permits: Arc::new(Semaphore::new(per_minute)),
        }
    }

    /// Wait for a permit. A zero share is refused at once: a semaphore that is never
    /// topped up would otherwise wait forever.
    async fn acquire(&self) -> Result<OwnedSemaphorePermit, Error> {
        if self.per_minute == 0 {
            return Err(Error::RateLimit {
                provider: self.provider.to_owned(),
                message: "No rate budget share on this chain".to_owned(),
            });
        }
        self.permits
            .clone()
            .acquire_owned()
            .await
            .map_err(|e| Error::RateLimit {
                provider: self.provider.to_owned(),
                message: format!("Failed to acquire permit: {e}"),
            })
    }

    fn top_up(&self) {
        let missing = self
            .per_minute
            .saturating_sub(self.permits.available_permits());
        if missing > 0 {
            self.permits.add_permits(missing);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{share, PerMinuteBudgets, RateLimitCoordinator};
    use crate::tokens::Error;
    use std::time::Duration;

    fn budgets(per_minute: usize) -> PerMinuteBudgets {
        PerMinuteBudgets {
            dexscreener_batch: per_minute,
            dexscreener_profiles: per_minute,
            dexscreener_boosts: per_minute,
            dexscreener_pools: per_minute,
            geckoterminal: per_minute,
            rugcheck: per_minute,
        }
    }

    #[test]
    fn a_single_participant_gets_the_whole_budget() {
        assert_eq!(share(30, 1, Some(0)), 30);
    }

    #[test]
    fn an_even_budget_splits_equally() {
        assert_eq!(share(30, 2, Some(0)), 15);
        assert_eq!(share(30, 2, Some(1)), 15);
    }

    #[test]
    fn the_remainder_goes_to_the_earliest_participants() {
        assert_eq!(share(10, 3, Some(0)), 4);
        assert_eq!(share(10, 3, Some(1)), 3);
        assert_eq!(share(10, 3, Some(2)), 3);
        assert_eq!(share(1, 2, Some(0)), 1);
        assert_eq!(share(1, 2, Some(1)), 0);
    }

    #[test]
    fn a_non_participant_gets_nothing() {
        assert_eq!(share(30, 2, None), 0);
        assert_eq!(share(30, 0, None), 0);
    }

    #[test]
    fn shares_always_sum_to_the_total() {
        for total in 0..=301 {
            for participants in 1..=7 {
                let sum: usize = (0..participants)
                    .map(|position| share(total, participants, Some(position)))
                    .sum();
                assert_eq!(sum, total, "total={total} participants={participants}");
            }
        }
    }

    #[tokio::test]
    async fn top_up_never_exceeds_the_budget() {
        let coordinator = RateLimitCoordinator::with_budgets(budgets(5));
        let budget = &coordinator.dexscreener_batch;

        coordinator.top_up_all();
        assert_eq!(budget.permits.available_permits(), 5);

        for _ in 0..2 {
            coordinator
                .acquire_dexscreener_batch()
                .await
                .expect("permit within budget")
                .forget();
        }
        assert_eq!(budget.permits.available_permits(), 3);

        coordinator.top_up_all();
        assert_eq!(budget.permits.available_permits(), 5);
        coordinator.top_up_all();
        assert_eq!(budget.permits.available_permits(), 5);
    }

    #[tokio::test]
    async fn a_zero_share_acquire_returns_at_once() {
        let coordinator = RateLimitCoordinator::with_budgets(budgets(0));
        let outcome =
            tokio::time::timeout(Duration::from_secs(1), coordinator.acquire_geckoterminal())
                .await
                .expect("a zero share must not wait");
        assert!(matches!(
            outcome,
            Err(Error::RateLimit { ref provider, .. }) if provider == "GeckoTerminal"
        ));

        coordinator.top_up_all();
        assert!(coordinator.acquire_rugcheck().await.is_err());
    }
}
