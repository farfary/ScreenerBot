// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Account fetcher type definitions.
//!
//! Contains data structures used by the account fetcher module.

use super::fetcher::ACCOUNT_BATCH_SIZE;
use crate::chains::solana::constants::SOL_MINT;
use crate::chains::solana::constants::SYSTEM_PROGRAM_ID;

use crate::chains::solana::solana_sdk::{account::Account, pubkey::Pubkey};
use std::collections::{HashMap, HashSet};
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

/// Whether `key` names an account `getMultipleAccounts` returns data for. The
/// native SOL mint and the system program appear among pool reserve keys, but the
/// RPC answers null for them, so they are never fetched, never complete a bundle
/// and never take part in a stale-account scan.
pub(crate) fn is_fetchable_account(key: &Pubkey) -> bool {
    *key != *SOL_MINT_PUBKEY && *key != *SYSTEM_PROGRAM_PUBKEY
}

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
/// regardless of TTL (`None` when absent); `reprice_after` is the
/// [`quiet_batch_floor`] of the pool's [`account_refresh_interval`], the age from
/// which [`accounts_to_refresh`] fetches an account early to share a call, so an
/// early fetch is repriced too and a price never outlives its refresh interval.
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

/// Age from which an account refreshed every `heartbeat` (a quiet account; for a
/// held pool, its own refresh interval) joins a call that is going out anyway,
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
    /// The scan's index of the pool the account belongs to.
    pub pool: usize,
}

impl RefreshCandidate {
    fn is_due(&self) -> bool {
        self.elapsed.is_none_or(|elapsed| elapsed >= self.interval)
    }

    /// Fraction of its refresh interval the account has aged; `None` when never fetched.
    fn overdue_ratio(&self) -> Option<f64> {
        self.elapsed
            .map(|elapsed| elapsed.as_secs_f64() / self.interval.as_secs_f64().max(f64::EPSILON))
    }
}

/// The keys and refresh candidates of a stale-account scan over `accounts`
/// (key, [`account_refresh_interval`], quiet, pool index), aged from `last_fetch`
/// at `now`.
///
/// Only keys the batch fetch fetches are candidates: [`is_fetchable_account`] keys
/// outside `blacklisted`. A key the fetch skips never records a fetch, so it would
/// stay due at every scan, keep the quiet batch open and pull every quiet account
/// each [`quiet_batch_floor`] instead of each heartbeat.
pub(crate) fn refresh_candidates(
    accounts: &[(Pubkey, Duration, bool, usize)],
    last_fetch: &HashMap<Pubkey, Instant>,
    blacklisted: &HashSet<Pubkey>,
    now: Instant,
) -> (Vec<Pubkey>, Vec<RefreshCandidate>) {
    accounts
        .iter()
        .filter(|(key, _, _, _)| is_fetchable_account(key) && !blacklisted.contains(key))
        .map(|(key, interval, quiet, pool)| {
            let candidate = RefreshCandidate {
                elapsed: last_fetch
                    .get(key)
                    .map(|fetched| now.saturating_duration_since(*fetched)),
                interval: *interval,
                quiet: *quiet,
                pool: *pool,
            };
            (*key, candidate)
        })
        .unzip()
}

/// Indices of the candidates a stale-account scan fetches now. `keys[i]` is the
/// account of `candidates[i]`; `queued` holds the accounts the same fetch cycle
/// already sends (a newly analyzed pool's first fetch), without the keys the
/// drain drops.
///
/// Every due or never-fetched account is fetched. When a quiet account is due or
/// new, every quiet account at least [`quiet_batch_floor`] old joins the same
/// `getMultipleAccounts` batch, so quiet pools added at different moments merge
/// into one phase within a cycle and quiet RPC follows the account count
/// (`ceil(accounts / 50)` calls per heartbeat), not the number of moments pools
/// were added. The calls the cycle sends anyway are then topped up by
/// [`fill_spare_capacity`]. An account fetched early is repriced (see
/// [`calculation_trigger`]), so its price age stays below the cache TTL.
pub(crate) fn accounts_to_refresh(
    keys: &[Pubkey],
    candidates: &[RefreshCandidate],
    heartbeat: Duration,
    queued: &HashSet<Pubkey>,
) -> Vec<usize> {
    let mut selected = due_and_quiet_batch(candidates, heartbeat);
    fill_spare_capacity(keys, candidates, &mut selected, queued);
    selected
        .iter()
        .enumerate()
        .filter(|(_, chosen)| **chosen)
        .map(|(idx, _)| idx)
        .collect()
}

/// Per candidate, whether it is due or joins a due quiet batch.
fn due_and_quiet_batch(candidates: &[RefreshCandidate], heartbeat: Duration) -> Vec<bool> {
    let quiet_batch_due = candidates.iter().any(|c| c.quiet && c.is_due());
    let floor = quiet_batch_floor(heartbeat);
    candidates
        .iter()
        .map(|c| {
            c.is_due()
                || (quiet_batch_due && c.quiet && c.elapsed.is_some_and(|elapsed| elapsed >= floor))
        })
        .collect()
}

/// Top up the `getMultipleAccounts` calls a fetch cycle sends anyway with the
/// pools closest to due, so a call never leaves slots empty while an account it
/// could carry becomes due a moment later and costs a call of its own.
///
/// The cycle sends each distinct key among the queued and selected accounts
/// once, in `ceil(distinct / 50)` calls; the slots that count leaves free are
/// filled with whole pools, most overdue first, whose accounts not already sent
/// are each at least a quarter of their own refresh interval old (the
/// [`quiet_batch_floor`] of that interval). A pool joins only when all of those
/// accounts fit, so its reserves are always read together. Keys are counted
/// once however many pools or queues name them, as the drain sends them, so the
/// fill never adds a call.
fn fill_spare_capacity(
    keys: &[Pubkey],
    candidates: &[RefreshCandidate],
    selected: &mut [bool],
    queued: &HashSet<Pubkey>,
) {
    let mut sending = queued.clone();
    sending.extend(
        keys.iter()
            .zip(selected.iter())
            .filter(|(_, chosen)| **chosen)
            .map(|(key, _)| *key),
    );
    if sending.is_empty() {
        return;
    }
    let mut spare = sending.len().div_ceil(ACCOUNT_BATCH_SIZE) * ACCOUNT_BATCH_SIZE - sending.len();
    if spare == 0 {
        return;
    }

    struct PoolFill {
        accounts: Vec<usize>,
        most_overdue: f64,
        eligible: bool,
    }
    let mut pools: HashMap<usize, PoolFill> = HashMap::new();
    for (idx, candidate) in candidates.iter().enumerate() {
        if selected[idx] || sending.contains(&keys[idx]) {
            continue;
        }
        let entry = pools.entry(candidate.pool).or_insert(PoolFill {
            accounts: Vec::new(),
            most_overdue: 0.0,
            eligible: true,
        });
        entry.accounts.push(idx);
        let floor = quiet_batch_floor(candidate.interval);
        match (candidate.elapsed, candidate.overdue_ratio()) {
            (Some(elapsed), Some(ratio)) if elapsed >= floor => {
                entry.most_overdue = entry.most_overdue.max(ratio);
            }
            _ => entry.eligible = false,
        }
    }

    let mut ranked: Vec<(usize, PoolFill)> = pools
        .into_iter()
        .filter(|(_, pool)| pool.eligible)
        .collect();
    ranked.sort_by(|(a_pool, a), (b_pool, b)| {
        b.most_overdue
            .total_cmp(&a.most_overdue)
            .then(a_pool.cmp(b_pool))
    });
    for (_, pool) in ranked {
        let added: HashSet<Pubkey> = pool
            .accounts
            .iter()
            .map(|idx| keys[*idx])
            .filter(|key| !sending.contains(key))
            .collect();
        if added.len() > spare {
            continue;
        }
        spare -= added.len();
        sending.extend(added);
        for idx in pool.accounts {
            selected[idx] = true;
        }
        if spare == 0 {
            break;
        }
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

    /// Whether the bundle holds every required account that can be fetched
    /// (see [`is_fetchable_account`]).
    pub fn is_complete(&self, required_accounts: &[Pubkey]) -> bool {
        required_accounts
            .iter()
            .filter(|key| is_fetchable_account(key))
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
                    quiet_batch_floor(refresh),
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

    /// One simulated pool: its reserve account count, whether a position holds
    /// it, and when it was added to the scan.
    #[derive(Clone, Copy)]
    struct SimPool {
        accounts: usize,
        held: bool,
        added: Duration,
    }

    /// `pools` quiet pools of `accounts` reserve accounts, the n-th added
    /// `n * stagger` after start.
    fn staggered_quiet_pools(pools: usize, accounts: usize, stagger: Duration) -> Vec<SimPool> {
        (0..pools)
            .map(|n| SimPool {
                accounts,
                held: false,
                added: stagger * n as u32,
            })
            .collect()
    }

    /// The main profile measured live on 2026-10-30: four held pools (14
    /// accounts) analyzed together at start, then 36 quiet pools (106 accounts)
    /// added 1.3 s apart.
    fn measured_profile(held_stagger: Duration) -> Vec<SimPool> {
        let held = [3, 4, 3, 4]
            .into_iter()
            .enumerate()
            .map(|(n, accounts)| SimPool {
                accounts,
                held: true,
                added: held_stagger * n as u32,
            });
        let quiet = [3; 22]
            .into_iter()
            .chain([2; 10])
            .chain([5; 4])
            .enumerate()
            .map(|(n, accounts)| SimPool {
                accounts,
                held: false,
                added: Duration::from_millis(1_300) * (n as u32 + 4),
            });
        held.chain(quiet).collect()
    }

    /// `getMultipleAccounts` calls and staleness over simulated scans.
    struct ScanOutcome {
        calls_per_minute: f64,
        /// Oldest price a quiet pool showed, from its first pricing on.
        oldest_quiet_price_age: Duration,
        /// Oldest price a held pool showed, from its first pricing on.
        oldest_held_price_age: Duration,
        /// Fetch cycles that read only part of a pool's accounts.
        split_pool_reads: usize,
    }

    type Select = fn(&[RefreshCandidate], Duration) -> Vec<usize>;

    /// Scan `pools` every 500 ms tick, with every fetch taking 1 s and no
    /// reserve ever changing, so a pool is repriced only once its price is
    /// `reprice_after(its refresh interval)` old. Calls are counted over an hour
    /// once every pool has had three heartbeats to settle; staleness is tracked
    /// over the same hour.
    fn simulate_scans(
        select: Select,
        reprice_after: fn(Duration) -> Duration,
        pools: &[SimPool],
        ttl: Duration,
    ) -> ScanOutcome {
        let tick = Duration::from_millis(500);
        let fetch_latency = Duration::from_secs(1);
        let heartbeat = ttl / 2;
        let last_added = pools.iter().map(|p| p.added).max().unwrap_or_default();
        let window_start = last_added + heartbeat * 3;
        let window_end = window_start + Duration::from_secs(3_600);
        let owner: Vec<usize> = pools
            .iter()
            .enumerate()
            .flat_map(|(pool, spec)| std::iter::repeat_n(pool, spec.accounts))
            .collect();
        let mut last_fetch: Vec<Option<Duration>> = vec![None; owner.len()];
        let mut priced_at: Vec<Option<Duration>> = vec![None; pools.len()];
        let mut outcome = ScanOutcome {
            calls_per_minute: 0.0,
            oldest_quiet_price_age: Duration::ZERO,
            oldest_held_price_age: Duration::ZERO,
            split_pool_reads: 0,
        };
        let mut calls = 0usize;
        let mut now = Duration::ZERO;
        let observe = |outcome: &mut ScanOutcome, priced_at: &[Option<Duration>], at: Duration| {
            if at < window_start {
                return;
            }
            for (pool, spec) in pools.iter().enumerate() {
                let Some(priced) = priced_at[pool] else {
                    continue;
                };
                let oldest = if spec.held {
                    &mut outcome.oldest_held_price_age
                } else {
                    &mut outcome.oldest_quiet_price_age
                };
                *oldest = (*oldest).max(at - priced);
            }
        };
        while now < window_end {
            now += tick;
            observe(&mut outcome, &priced_at, now);
            let accounts: Vec<usize> = (0..owner.len())
                .filter(|account| pools[owner[*account]].added <= now)
                .collect();
            let candidates: Vec<RefreshCandidate> = accounts
                .iter()
                .map(|&account| {
                    let spec = pools[owner[account]];
                    RefreshCandidate {
                        elapsed: last_fetch[account].map(|fetched| now - fetched),
                        interval: account_refresh_interval(spec.held, heartbeat),
                        quiet: !spec.held,
                        pool: owner[account],
                    }
                })
                .collect();
            let selected = select(&candidates, heartbeat);
            if selected.is_empty() {
                continue;
            }
            let fetched = now + fetch_latency;
            if now >= window_start {
                calls += selected.len().div_ceil(ACCOUNT_BATCH_SIZE);
                let mut read_per_pool: HashMap<usize, usize> = HashMap::new();
                for idx in &selected {
                    *read_per_pool.entry(owner[accounts[*idx]]).or_default() += 1;
                }
                outcome.split_pool_reads += read_per_pool
                    .iter()
                    .filter(|(pool, read)| **read < pools[**pool].accounts)
                    .count();
            }
            observe(&mut outcome, &priced_at, fetched);
            let mut touched = HashSet::new();
            for idx in selected {
                last_fetch[accounts[idx]] = Some(fetched);
                touched.insert(owner[accounts[idx]]);
            }
            for pool in touched {
                let age = priced_at[pool].map(|priced| fetched - priced);
                if calculation_trigger(
                    priced_at[pool].is_some(),
                    false,
                    age,
                    reprice_after(account_refresh_interval(pools[pool].held, heartbeat)),
                )
                .is_some()
                {
                    priced_at[pool] = Some(fetched);
                }
            }
            now = fetched;
        }
        outcome.calls_per_minute = calls as f64 / 60.0;
        outcome
    }

    fn indices(selected: Vec<bool>) -> Vec<usize> {
        selected
            .iter()
            .enumerate()
            .filter(|(_, chosen)| **chosen)
            .map(|(idx, _)| idx)
            .collect()
    }

    /// A distinct key per index, as accounts no two pools share.
    fn distinct_keys(count: usize) -> Vec<Pubkey> {
        (0..count)
            .map(|n| {
                let mut bytes = [0u8; 32];
                bytes[..8].copy_from_slice(&(n as u64 + 1).to_le_bytes());
                Pubkey::new_from_array(bytes)
            })
            .collect()
    }

    /// The production selection over unshared accounts with nothing else queued.
    fn with_fill(candidates: &[RefreshCandidate], heartbeat: Duration) -> Vec<usize> {
        accounts_to_refresh(
            &distinct_keys(candidates.len()),
            candidates,
            heartbeat,
            &HashSet::new(),
        )
    }

    /// The selection before spare slots were filled: due accounts and the quiet batch.
    fn without_fill(candidates: &[RefreshCandidate], heartbeat: Duration) -> Vec<usize> {
        indices(due_and_quiet_batch(candidates, heartbeat))
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
                let outcome = simulate_scans(
                    with_fill,
                    quiet_batch_floor,
                    &staggered_quiet_pools(pools, accounts_per_pool, stagger),
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
                    outcome.oldest_quiet_price_age < ttl,
                    "ttl {ttl_secs}s, {pools} pools: price reached {:?}",
                    outcome.oldest_quiet_price_age
                );
            }
        }
    }

    #[test]
    fn per_account_due_selection_scales_with_moments_pools_were_added() {
        let outcome = simulate_scans(
            due_accounts_only,
            |heartbeat| heartbeat,
            &staggered_quiet_pools(7, 2, Duration::from_millis(1_300)),
            Duration::from_secs(30),
        );
        assert!(
            outcome.calls_per_minute > 4.0,
            "{} calls/min",
            outcome.calls_per_minute
        );
    }

    /// Held pools refresh every 5 s; the quiet pools ride in the slots those
    /// calls leave free instead of costing calls of their own, and nothing
    /// becomes staler than without the fill.
    #[test]
    fn spare_slots_carry_pools_that_would_cost_their_own_call() {
        for ttl_secs in [10, 30, 60, 120] {
            let ttl = Duration::from_secs(ttl_secs);
            for held_stagger in [Duration::ZERO, Duration::from_millis(1_300)] {
                let profile = measured_profile(held_stagger);
                let before = simulate_scans(without_fill, quiet_batch_floor, &profile, ttl);
                let after = simulate_scans(with_fill, quiet_batch_floor, &profile, ttl);
                let case = format!("ttl {ttl_secs}s, held stagger {held_stagger:?}");
                assert!(
                    after.calls_per_minute <= before.calls_per_minute * 0.8,
                    "{case}: {} calls/min with the fill, {} without",
                    after.calls_per_minute,
                    before.calls_per_minute
                );
                // Due at its interval, seen within a 500 ms tick, priced 1 s later.
                let held_bound =
                    account_refresh_interval(true, ttl / 2) + Duration::from_millis(1_500);
                assert!(
                    after.oldest_held_price_age <= held_bound,
                    "{case}: a held price reached {:?}",
                    after.oldest_held_price_age
                );
                assert!(
                    after.oldest_quiet_price_age < ttl
                        && after.oldest_quiet_price_age <= before.oldest_quiet_price_age,
                    "{case}: quiet price reached {:?} ({:?} without the fill)",
                    after.oldest_quiet_price_age,
                    before.oldest_quiet_price_age
                );
                assert_eq!(
                    after.split_pool_reads, 0,
                    "{case}: a pool was read in parts"
                );
            }
        }
    }

    /// Positions opened at different moments: their pools merge into one call
    /// per refresh interval instead of a call each, and an early fetch reprices,
    /// so no held price outlives its interval.
    #[test]
    fn held_pools_opened_at_different_moments_share_calls() {
        let ttl = Duration::from_secs(30);
        let interval = account_refresh_interval(true, ttl / 2);
        let pools: Vec<SimPool> = (0..10)
            .map(|n| SimPool {
                accounts: 3,
                held: true,
                added: Duration::from_millis(500) * n,
            })
            .collect();
        let before = simulate_scans(without_fill, quiet_batch_floor, &pools, ttl);
        let after = simulate_scans(with_fill, quiet_batch_floor, &pools, ttl);
        let one_call_per_interval = 60.0 / interval.as_secs_f64();
        assert!(
            after.calls_per_minute <= one_call_per_interval,
            "{} calls/min with the fill, {} without",
            after.calls_per_minute,
            before.calls_per_minute
        );
        assert!(
            after.calls_per_minute < before.calls_per_minute,
            "{} calls/min with the fill, {} without",
            after.calls_per_minute,
            before.calls_per_minute
        );
        assert!(
            after.oldest_held_price_age <= interval + Duration::from_millis(1_500),
            "a held price reached {:?}",
            after.oldest_held_price_age
        );
    }

    /// Over random scans with keys shared between pools and first fetches
    /// already queued (some of them scan candidates too): the fill keeps
    /// everything already selected, never adds a `getMultipleAccounts` call
    /// for the distinct keys the drain sends, only takes accounts at least a
    /// quarter of their interval old, and sends a pool's accounts together.
    #[test]
    fn the_fill_never_adds_a_call_and_reads_whole_pools() {
        use rand::{Rng, SeedableRng};
        let mut rng = rand::rngs::StdRng::seed_from_u64(0x5eed);
        let heartbeat = Duration::from_secs(15);
        let shared = distinct_keys(4);
        for _ in 0..2_000 {
            let mut candidates = Vec::new();
            let mut keys = Vec::new();
            for pool in 0..rng.gen_range(1..80) {
                let held = rng.gen_bool(0.2);
                let interval = account_refresh_interval(held, heartbeat);
                let elapsed =
                    (!rng.gen_bool(0.05)).then(|| Duration::from_millis(rng.gen_range(0..20_000)));
                for _ in 0..rng.gen_range(1..6) {
                    candidates.push(RefreshCandidate {
                        elapsed,
                        interval,
                        quiet: !held,
                        pool,
                    });
                    keys.push(if rng.gen_bool(0.05) {
                        shared[rng.gen_range(0..shared.len())]
                    } else {
                        Pubkey::new_unique()
                    });
                }
            }
            let mut queued = HashSet::new();
            if rng.gen_bool(0.3) {
                for _ in 0..rng.gen_range(1..12) {
                    queued.insert(if rng.gen_bool(0.5) {
                        keys[rng.gen_range(0..keys.len())]
                    } else {
                        Pubkey::new_unique()
                    });
                }
            }
            let base = due_and_quiet_batch(&candidates, heartbeat);
            let filled = accounts_to_refresh(&keys, &candidates, heartbeat, &queued);
            let filled_set: HashSet<usize> = filled.iter().copied().collect();

            let sent = |chosen: &mut dyn Iterator<Item = usize>| {
                let mut sent = queued.clone();
                sent.extend(chosen.map(|idx| keys[idx]));
                sent
            };
            let base_sent = sent(&mut (0..base.len()).filter(|idx| base[*idx]));
            let filled_sent = sent(&mut filled.iter().copied());
            assert_eq!(
                filled_sent.len().div_ceil(ACCOUNT_BATCH_SIZE),
                base_sent.len().div_ceil(ACCOUNT_BATCH_SIZE)
            );
            for (idx, chosen) in base.iter().enumerate() {
                assert!(!chosen || filled_set.contains(&idx));
            }
            for &idx in &filled {
                if base[idx] {
                    continue;
                }
                let added = candidates[idx];
                assert!(!base_sent.contains(&keys[idx]));
                assert!(added
                    .elapsed
                    .is_some_and(|elapsed| elapsed >= quiet_batch_floor(added.interval)));
                for (other, candidate) in candidates.iter().enumerate() {
                    if candidate.pool == added.pool {
                        assert!(
                            filled_sent.contains(&keys[other]),
                            "pool {} read in parts",
                            added.pool
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn keys_the_batch_fetch_skips_do_not_hold_the_quiet_batch_open() {
        let start = Instant::now();
        let ttl = Duration::from_secs(30);
        let heartbeat = ttl / 2;
        let interval = account_refresh_interval(false, heartbeat);
        let blacklisted_key = Pubkey::new_unique();
        let blacklisted = HashSet::from([blacklisted_key]);
        let mut accounts = vec![
            (*SOL_MINT_PUBKEY, interval, true, 0),
            (blacklisted_key, interval, true, 0),
        ];
        for pool in 1..5 {
            accounts.push((Pubkey::new_unique(), interval, true, pool));
            accounts.push((Pubkey::new_unique(), interval, true, pool));
            accounts.push((*SYSTEM_PROGRAM_PUBKEY, interval, true, pool));
        }
        // A blacklisted key last fetched while it was still answering.
        let mut last_fetch: HashMap<Pubkey, Instant> = HashMap::from([(blacklisted_key, start)]);
        let fetched_by_batch =
            |key: &Pubkey| is_fetchable_account(key) && !blacklisted.contains(key);
        let window = Duration::from_secs(600);
        let mut calls = 0usize;
        let mut now = Duration::ZERO;
        while now < heartbeat * 3 + window {
            now += Duration::from_millis(500);
            let (keys, candidates) =
                refresh_candidates(&accounts, &last_fetch, &blacklisted, start + now);
            assert!(keys.iter().all(fetched_by_batch));
            let fetched: Vec<Pubkey> =
                accounts_to_refresh(&keys, &candidates, heartbeat, &HashSet::new())
                    .into_iter()
                    .map(|idx| keys[idx])
                    .filter(fetched_by_batch)
                    .collect();
            if fetched.is_empty() {
                continue;
            }
            if now >= heartbeat * 3 {
                calls += 1;
            }
            now += Duration::from_secs(1);
            for key in fetched {
                last_fetch.insert(key, start + now);
            }
        }
        let calls_per_minute = calls as f64 * 60.0 / window.as_secs_f64();
        let bound = 60.0 / heartbeat.as_secs_f64();
        assert!(
            calls_per_minute <= bound,
            "{calls_per_minute} calls/min above {bound}"
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
