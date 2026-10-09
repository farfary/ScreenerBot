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
    /// least `reprice_after` old; repricing keeps it inside the cache TTL.
    Heartbeat,
}

/// Decide whether a complete bundle that just received re-fetched accounts must
/// be repriced. `cached_price_age` is the age of the token's cached pool price
/// regardless of TTL (`None` when absent); `reprice_after` is
/// [`quiet_batch_floor`], so a quiet account fetched early to share a batch is
/// repriced too and its price never outlives the cache TTL.
pub(crate) fn calculation_trigger(
    calculated_before: bool,
    reserves_changed: bool,
    cached_price_age: Option<Duration>,
    reprice_after: Duration,
) -> Option<CalculationTrigger> {
    if !calculated_before {
        Some(CalculationTrigger::Initial)
    } else if reserves_changed {
        Some(CalculationTrigger::ReservesChanged)
    } else if cached_price_age.is_none_or(|age| age >= reprice_after) {
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

/// Age from which a quiet account joins a quiet batch that is going out anyway,
/// and from which a re-fetched bundle with unchanged reserves is repriced.
///
/// Two phases whose fetches complete `gap` apart merge when either sees the
/// other at least this old: `gap >= floor` or `heartbeat - gap - latency >=
/// floor`. A quarter of the heartbeat leaves no gap that never merges for any
/// fetch latency up to half the heartbeat.
pub(crate) fn quiet_batch_floor(heartbeat: Duration) -> Duration {
    heartbeat / 4
}

/// One reserve account considered by a stale-account scan.
#[derive(Debug, Clone, Copy)]
pub(crate) struct RefreshCandidate {
    /// Time since the account was last fetched; `None` when never fetched.
    pub elapsed: Option<Duration>,
    /// The account's [`account_refresh_interval`].
    pub interval: Duration,
    /// The account belongs to a pool without an open position.
    pub quiet: bool,
}

impl RefreshCandidate {
    fn is_due(&self) -> bool {
        self.elapsed.is_none_or(|elapsed| elapsed >= self.interval)
    }
}

/// Indices of the candidates a stale-account scan fetches now.
///
/// Every due or never-fetched account is fetched. When a quiet account is due or
/// new, every quiet account at least [`quiet_batch_floor`] old joins the same
/// `getMultipleAccounts` batch, so quiet pools added at different moments merge
/// into one phase within a cycle and quiet RPC follows the account count
/// (`ceil(accounts / 50)` calls per heartbeat), not the number of moments pools
/// were added. An account fetched early is repriced (see [`calculation_trigger`]),
/// so its price age stays below the cache TTL.
pub(crate) fn accounts_to_refresh(
    candidates: &[RefreshCandidate],
    heartbeat: Duration,
) -> Vec<usize> {
    let quiet_batch_due = candidates.iter().any(|c| c.quiet && c.is_due());
    let floor = quiet_batch_floor(heartbeat);
    candidates
        .iter()
        .enumerate()
        .filter(|(_, c)| {
            c.is_due()
                || (quiet_batch_due && c.quiet && c.elapsed.is_some_and(|elapsed| elapsed >= floor))
        })
        .map(|(idx, _)| idx)
        .collect()
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
    use super::super::fetcher::ACCOUNT_BATCH_SIZE;
    use super::*;
    use std::collections::HashSet;

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
                if calculation_trigger(
                    true,
                    false,
                    Some(fetched - priced_at),
                    quiet_batch_floor(heartbeat),
                )
                .is_some()
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

    /// Quiet `getMultipleAccounts` calls and price ages over simulated scans.
    struct ScanOutcome {
        calls_per_minute: f64,
        oldest_price_age: Duration,
    }

    /// Scan quiet pools of `accounts_per_pool` reserve accounts, the n-th added
    /// `n * stagger` after start, with every fetch taking `fetch_latency`. Calls
    /// are counted over an hour once every pool has had three heartbeats to
    /// settle; price ages are tracked from each pool's first pricing.
    fn simulate_quiet_scans(
        select: fn(&[RefreshCandidate], Duration) -> Vec<usize>,
        reprice_after: fn(Duration) -> Duration,
        pools: usize,
        accounts_per_pool: usize,
        stagger: Duration,
        ttl: Duration,
    ) -> ScanOutcome {
        let tick = Duration::from_millis(500);
        let fetch_latency = Duration::from_secs(1);
        let heartbeat = ttl / 2;
        let interval = account_refresh_interval(false, heartbeat);
        let window_start = stagger * pools as u32 + heartbeat * 3;
        let window_end = window_start + Duration::from_secs(3_600);
        let added_at = |pool: usize| stagger * pool as u32;
        let mut last_fetch: Vec<Option<Duration>> = vec![None; pools * accounts_per_pool];
        let mut priced_at: Vec<Option<Duration>> = vec![None; pools];
        let mut calls = 0usize;
        let mut oldest = Duration::ZERO;
        let mut now = Duration::ZERO;
        let oldest_at = |priced_at: &[Option<Duration>], at: Duration| {
            priced_at
                .iter()
                .flatten()
                .map(|priced| at - *priced)
                .max()
                .unwrap_or_default()
        };
        while now < window_end {
            now += tick;
            oldest = oldest.max(oldest_at(&priced_at, now));
            let accounts: Vec<usize> = (0..last_fetch.len())
                .filter(|account| added_at(account / accounts_per_pool) <= now)
                .collect();
            let candidates: Vec<RefreshCandidate> = accounts
                .iter()
                .map(|&account| RefreshCandidate {
                    elapsed: last_fetch[account].map(|fetched| now - fetched),
                    interval,
                    quiet: true,
                })
                .collect();
            let selected = select(&candidates, heartbeat);
            if selected.is_empty() {
                continue;
            }
            if now >= window_start {
                calls += selected.len().div_ceil(ACCOUNT_BATCH_SIZE);
            }
            let fetched = now + fetch_latency;
            oldest = oldest.max(oldest_at(&priced_at, fetched));
            let mut touched = HashSet::new();
            for idx in selected {
                last_fetch[accounts[idx]] = Some(fetched);
                touched.insert(accounts[idx] / accounts_per_pool);
            }
            for pool in touched {
                let age = priced_at[pool].map(|priced| fetched - priced);
                if calculation_trigger(
                    priced_at[pool].is_some(),
                    false,
                    age,
                    reprice_after(heartbeat),
                )
                .is_some()
                {
                    priced_at[pool] = Some(fetched);
                }
            }
            now = fetched;
        }
        ScanOutcome {
            calls_per_minute: calls as f64 / 60.0,
            oldest_price_age: oldest,
        }
    }

    /// The selection before quiet batching: each account only when its own
    /// interval has passed, repriced at the heartbeat.
    fn due_accounts_only(candidates: &[RefreshCandidate], _heartbeat: Duration) -> Vec<usize> {
        candidates
            .iter()
            .enumerate()
            .filter(|(_, c)| c.is_due())
            .map(|(idx, _)| idx)
            .collect()
    }

    #[test]
    fn quiet_pools_added_at_different_moments_share_batches() {
        let stagger = Duration::from_millis(1_300);
        for ttl_secs in (10..=120).step_by(10) {
            let ttl = Duration::from_secs(ttl_secs);
            let heartbeat_secs = (ttl / 2).as_secs_f64();
            for (pools, accounts_per_pool) in [(1, 2), (7, 2), (30, 3)] {
                let outcome = simulate_quiet_scans(
                    accounts_to_refresh,
                    quiet_batch_floor,
                    pools,
                    accounts_per_pool,
                    stagger,
                    ttl,
                );
                let batches = (pools * accounts_per_pool).div_ceil(ACCOUNT_BATCH_SIZE) as f64;
                let bound = batches * 60.0 / heartbeat_secs;
                assert!(
                    outcome.calls_per_minute <= bound,
                    "ttl {ttl_secs}s, {pools} pools: {} calls/min above {bound}",
                    outcome.calls_per_minute
                );
                assert!(
                    outcome.oldest_price_age < ttl,
                    "ttl {ttl_secs}s, {pools} pools: price reached {:?}",
                    outcome.oldest_price_age
                );
            }
        }
    }

    #[test]
    fn per_account_due_selection_scales_with_moments_pools_were_added() {
        let outcome = simulate_quiet_scans(
            due_accounts_only,
            |heartbeat| heartbeat,
            7,
            2,
            Duration::from_millis(1_300),
            Duration::from_secs(30),
        );
        assert!(
            outcome.calls_per_minute > 4.0,
            "{} calls/min",
            outcome.calls_per_minute
        );
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
