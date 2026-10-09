// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Account fetcher type definitions.
//!
//! Contains data structures used by the account fetcher module.

use crate::chains::solana::constants::SOL_MINT;
use crate::chains::solana::constants::SYSTEM_PROGRAM_ID;

use crate::chains::solana::solana_sdk::{account::Account, pubkey::Pubkey};
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::LazyLock;
use std::time::{Duration, Instant};

/// Parsed once and reused across the ~500ms fetch/completeness hot loop —
/// re-parsing these fixed strings per account/bundle check showed up in
/// profiling as avoidable work on the pool fetcher's hottest path.
pub(crate) static SOL_MINT_PUBKEY: LazyLock<Pubkey> =
    LazyLock::new(|| Pubkey::from_str(SOL_MINT).expect("SOL_MINT is a valid pubkey"));
pub(crate) static SYSTEM_PROGRAM_PUBKEY: LazyLock<Pubkey> = LazyLock::new(|| {
    Pubkey::from_str(SYSTEM_PROGRAM_ID).expect("SYSTEM_PROGRAM_ID is a valid pubkey")
});

#[derive(Debug, Clone)]
pub(crate) struct MissingAccountState {
    pub(crate) failures: u32,
    pub(crate) last_failure: Instant,
    pub(crate) blacklisted: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct MissingPoolState {
    pub(crate) failures: u32,
    pub(crate) last_failure: Instant,
    pub(crate) blacklisted: bool,
}

/// Message types for fetcher communication
#[derive(Debug, Clone)]
pub enum FetcherMessage {
    /// Request to fetch accounts for a pool
    FetchPool {
        pool_id: Pubkey,
        accounts: Vec<Pubkey>,
    },
    /// Request to fetch specific accounts
    FetchAccounts { accounts: Vec<Pubkey> },
    /// Signal shutdown
    Shutdown,
}

/// Account data with metadata
#[derive(Debug, Clone)]
pub struct AccountData {
    pub pubkey: Pubkey,
    pub data: Vec<u8>,
    pub slot: u64,
    pub fetched_at: Instant,
    pub lamports: u64,
    pub owner: Pubkey,
}

impl AccountData {
    /// Create from Solana Account
    pub fn from_account(pubkey: Pubkey, account: Account, slot: u64) -> Self {
        Self {
            pubkey,
            data: account.data,
            slot,
            fetched_at: Instant::now(),
            lamports: account.lamports,
            owner: account.owner,
        }
    }

    /// Check if account data is stale
    pub fn is_stale(&self, max_age_seconds: u64) -> bool {
        self.fetched_at.elapsed().as_secs() > max_age_seconds
    }

    /// Whether the on-chain content differs from `previous`: data bytes, lamports
    /// (native-SOL reserves) or owner. The fetch slot and instant are excluded
    /// because they change on every fetch even when the account did not.
    pub fn content_differs(&self, previous: &AccountData) -> bool {
        self.data != previous.data
            || self.lamports != previous.lamports
            || self.owner != previous.owner
    }
}

/// Why a complete pool bundle is (re)priced after a fetch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CalculationTrigger {
    /// The bundle has never been submitted for calculation.
    Initial,
    /// At least one re-fetched reserve account changed content.
    ReservesChanged,
    /// Reserves are unchanged, but the token's cached price is missing or at
    /// least `heartbeat` old; repricing keeps it inside the cache TTL.
    Heartbeat,
}

/// Decide whether a complete bundle that just received re-fetched accounts must
/// be repriced. `cached_price_age` is the age of the token's cached pool price
/// regardless of TTL (`None` when absent); `heartbeat` is half the cache TTL, so
/// a token whose pool is quiet is repriced before its price can expire.
pub(crate) fn calculation_trigger(
    calculated_before: bool,
    reserves_changed: bool,
    cached_price_age: Option<Duration>,
    heartbeat: Duration,
) -> Option<CalculationTrigger> {
    if !calculated_before {
        Some(CalculationTrigger::Initial)
    } else if reserves_changed {
        Some(CalculationTrigger::ReservesChanged)
    } else if cached_price_age.is_none_or(|age| age >= heartbeat) {
        Some(CalculationTrigger::Heartbeat)
    } else {
        None
    }
}

/// Longest an open position's pool accounts go without a re-fetch.
const OPEN_POSITION_ACCOUNT_REFRESH: Duration = Duration::from_secs(5);

/// Longest a pool's reserve accounts go without a re-fetch. `heartbeat` is the
/// repricing heartbeat (half the price cache TTL).
///
/// A pool whose reserves do not change is repriced only when its accounts are
/// re-fetched (see [`calculation_trigger`]), so this cadence bounds the age of its
/// published price. Any cadence at or above the TTL lets the price expire before
/// the next re-fetch, and every reader falls back to another price source until
/// the pool is repriced.
pub(crate) fn account_refresh_interval(has_open_position: bool, heartbeat: Duration) -> Duration {
    if has_open_position {
        OPEN_POSITION_ACCOUNT_REFRESH.min(heartbeat)
    } else {
        heartbeat
    }
}

/// Pool account bundle - all accounts for a specific pool
#[derive(Debug, Clone)]
pub struct PoolAccountBundle {
    pub pool_id: Pubkey,
    pub accounts: HashMap<Pubkey, AccountData>,
    pub last_updated: Instant,
    pub slot: u64,
    pub calculation_requested: bool,
}

impl PoolAccountBundle {
    /// Create new bundle
    pub fn new(pool_id: Pubkey) -> Self {
        Self {
            pool_id,
            accounts: HashMap::new(),
            last_updated: Instant::now(),
            slot: 0,
            calculation_requested: false,
        }
    }

    /// Add account to bundle
    pub fn add_account(&mut self, account_data: AccountData) {
        self.slot = self.slot.max(account_data.slot);
        self.last_updated = Instant::now();
        self.accounts.insert(account_data.pubkey, account_data);

        // Don't reset calculation_requested flag here - let the calculator manage this flag
        // Resetting it here can cause premature recalculation before all accounts are updated
    }

    /// Check if bundle is complete (has all required accounts)
    /// Skips the native SOL mint since it's not a real on-chain account and
    /// RPC returns null for it, which would prevent bundles from ever completing.
    pub fn is_complete(&self, required_accounts: &[Pubkey]) -> bool {
        required_accounts
            .iter()
            .filter(|key| **key != *SOL_MINT_PUBKEY && **key != *SYSTEM_PROGRAM_PUBKEY)
            .all(|key| self.accounts.contains_key(key))
    }

    /// Mark that calculation has been requested for this bundle
    pub fn mark_calculation_requested(&mut self) {
        self.calculation_requested = true;
    }

    /// Check if bundle is stale
    pub fn is_stale(&self, max_age_seconds: u64) -> bool {
        self.last_updated.elapsed().as_secs() > max_age_seconds
    }
}

/// Fetch statistics
#[derive(Debug, Clone)]
pub struct FetchStats {
    pub total_bundles: usize,
    pub total_accounts_tracked: usize,
    pub bundles_with_data: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    const HEARTBEAT: Duration = Duration::from_secs(15);

    fn account(data: &[u8], lamports: u64) -> AccountData {
        AccountData {
            pubkey: *SOL_MINT_PUBKEY,
            data: data.to_vec(),
            slot: 1,
            fetched_at: Instant::now(),
            lamports,
            owner: *SYSTEM_PROGRAM_PUBKEY,
        }
    }

    #[test]
    fn never_calculated_bundle_is_calculated() {
        assert_eq!(
            calculation_trigger(false, false, Some(Duration::from_secs(1)), HEARTBEAT),
            Some(CalculationTrigger::Initial)
        );
    }

    #[test]
    fn changed_reserves_are_calculated_even_with_a_young_price() {
        assert_eq!(
            calculation_trigger(true, true, Some(Duration::from_secs(1)), HEARTBEAT),
            Some(CalculationTrigger::ReservesChanged)
        );
    }

    #[test]
    fn unchanged_reserves_with_a_young_price_are_not_calculated() {
        assert_eq!(
            calculation_trigger(true, false, Some(Duration::from_secs(14)), HEARTBEAT),
            None
        );
    }

    #[test]
    fn unchanged_reserves_older_than_the_heartbeat_are_calculated() {
        assert_eq!(
            calculation_trigger(true, false, Some(HEARTBEAT), HEARTBEAT),
            Some(CalculationTrigger::Heartbeat)
        );
        assert_eq!(
            calculation_trigger(true, false, None, HEARTBEAT),
            Some(CalculationTrigger::Heartbeat)
        );
    }

    /// Oldest price age a quiet pool reaches over ten minutes of fetch ticks, when
    /// every re-fetch takes `fetch_latency` before its bundle is repriced.
    fn oldest_quiet_pool_price_age(
        has_open_position: bool,
        ttl: Duration,
        fetch_latency: Duration,
    ) -> Duration {
        let tick = Duration::from_millis(500);
        let heartbeat = ttl / 2;
        let refresh = account_refresh_interval(has_open_position, heartbeat);
        let mut now = Duration::ZERO;
        let mut last_fetch = Duration::ZERO;
        let mut priced_at = Duration::ZERO;
        let mut oldest = Duration::ZERO;
        while now < Duration::from_secs(600) {
            now += tick;
            oldest = oldest.max(now - priced_at);
            if now - last_fetch >= refresh {
                let fetched = now + fetch_latency;
                oldest = oldest.max(fetched - priced_at);
                if calculation_trigger(true, false, Some(fetched - priced_at), heartbeat).is_some()
                {
                    priced_at = fetched;
                }
                last_fetch = fetched;
                now = fetched;
            }
        }
        oldest
    }

    #[test]
    fn a_quiet_pool_is_repriced_before_its_price_expires() {
        // The configured TTL range is 10..=120 s; a re-fetch may take a few seconds.
        for ttl_secs in (10..=120).step_by(5) {
            let ttl = Duration::from_secs(ttl_secs);
            for has_open_position in [false, true] {
                let oldest =
                    oldest_quiet_pool_price_age(has_open_position, ttl, Duration::from_secs(3));
                assert!(
                    oldest < ttl,
                    "ttl {ttl_secs}s open_position={has_open_position}: price reached {oldest:?}"
                );
            }
        }
    }

    #[test]
    fn an_open_position_refreshes_at_least_every_five_seconds() {
        let heartbeat = Duration::from_secs(15);
        assert_eq!(
            account_refresh_interval(true, heartbeat),
            Duration::from_secs(5)
        );
        assert_eq!(account_refresh_interval(false, heartbeat), heartbeat);
        assert_eq!(
            account_refresh_interval(true, Duration::from_secs(4)),
            Duration::from_secs(4)
        );
    }

    #[test]
    fn content_change_ignores_slot_and_fetch_instant() {
        let previous = account(&[1, 2, 3], 10);
        let mut refetched = account(&[1, 2, 3], 10);
        refetched.slot = 99;
        assert!(!refetched.content_differs(&previous));
        assert!(account(&[1, 2, 4], 10).content_differs(&previous));
        assert!(account(&[1, 2, 3], 11).content_differs(&previous));
    }
}
