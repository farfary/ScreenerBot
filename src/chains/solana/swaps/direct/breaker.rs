// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Failure memory of the direct engine: a pool, or a venue in one direction,
//! that keeps failing before its swap settles is not offered for a while.
//!
//! A pool whose transaction the engine cannot execute fails before anything is
//! sent (a refused build or simulation) or reverts on chain, and the routing
//! layer then falls back to an aggregator, so no trade is lost. Without this
//! memory the same pool quotes again on the next attempt, wins the comparison
//! again and fails again, so every exit rung pays a wasted load, build and
//! simulation before the fallback runs.
//!
//! Two scopes:
//!
//! * **Pool.** [`POOL_STRIKES_TO_SUSPEND`] failures in a row, each within
//!   [`STRIKE_WINDOW`] of the first, suspend that pool for [`SUSPENSION`]. A
//!   settled swap through the pool clears its strikes.
//! * **Venue and direction.** Failures through [`VENUE_POOLS_TO_SUSPEND`]
//!   distinct pools of one venue in one direction within [`STRIKE_WINDOW`]
//!   suspend that venue's buys or sells for [`SUSPENSION`]. This is what catches
//!   a programme upgrade that breaks every pool of a venue at once. A settled
//!   swap through the venue in that direction clears its strikes.
//!
//! Only a failure that says something about the pool or the engine strikes
//! ([`strikes`]). A lost race (`MarketMoved`), our own node failing, a wallet
//! that cannot fund the trade and a transaction that may still land never do.
//!
//! The memory is process-local and bounded: suspensions expire on their own and
//! a restart forgets them, which is the right failure mode for a cache whose
//! only job is to save time.

use super::error::DirectSwapError;
use crate::chains::solana::pools::types::ProgramKind;
use crate::chains::solana::solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

/// Failures in a row that suspend one pool.
pub const POOL_STRIKES_TO_SUSPEND: usize = 2;

/// Distinct failing pools that suspend a venue in one direction.
pub const VENUE_POOLS_TO_SUSPEND: usize = 3;

/// How far apart the strikes that suspend may lie.
pub const STRIKE_WINDOW: Duration = Duration::from_secs(30 * 60);

/// How long a suspension lasts.
pub const SUSPENSION: Duration = Duration::from_secs(30 * 60);

/// Pools remembered at most; the oldest unsuspended records go first.
const MAX_TRACKED_POOLS: usize = 4_096;

/// Which way a swap trades the pair's token.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Direction {
    /// The token is the output.
    Buy,
    /// The token is the input.
    Sell,
}

impl Direction {
    /// Lower-case label for logs.
    pub fn label(self) -> &'static str {
        match self {
            Direction::Buy => "buys",
            Direction::Sell => "sells",
        }
    }
}

/// What a recorded failure newly suspended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tripped {
    /// The pool itself.
    Pool,
    /// Every pool of the venue, in this direction.
    Venue,
}

/// Whether `error` is evidence against the pool or the engine's handling of it.
///
/// A failure that strikes is one the same pool would most likely produce again:
/// a transaction the engine could not build, size or get past simulation, or one
/// the chain reverted. Everything else is a race, a node or wallet condition, or
/// an outcome that may still settle, and says nothing about the pool.
pub fn strikes(error: &DirectSwapError) -> bool {
    matches!(
        error,
        DirectSwapError::UnsupportedVenue { .. }
            | DirectSwapError::PoolUndecodable { .. }
            | DirectSwapError::QuoteMath { .. }
            | DirectSwapError::Build { .. }
            | DirectSwapError::SimulationRejected { .. }
            | DirectSwapError::TransactionTooLarge { .. }
            | DirectSwapError::TransactionFailed { .. }
    )
}

#[derive(Debug, Default)]
struct PoolRecord {
    program: Option<ProgramKind>,
    strikes: Vec<Instant>,
    suspended_until: Option<Instant>,
}

#[derive(Debug, Default)]
struct VenueRecord {
    strikes: Vec<(Pubkey, Instant)>,
    suspended_until: Option<Instant>,
}

#[derive(Debug, Default)]
struct State {
    pools: HashMap<Pubkey, PoolRecord>,
    venues: HashMap<(ProgramKind, Direction), VenueRecord>,
}

/// The failure memory. [`global`] is the one the router uses; tests build
/// their own.
#[derive(Debug, Default)]
pub struct Breaker {
    state: Mutex<State>,
}

impl Breaker {
    /// An empty memory.
    pub fn new() -> Self {
        Self::default()
    }

    /// When the pool's suspension ends, if it is suspended at `now`.
    pub fn pool_suspended_until(&self, pool: &Pubkey, now: Instant) -> Option<Instant> {
        let state = self.lock();
        state
            .pools
            .get(pool)
            .and_then(|record| record.suspended_until)
            .filter(|until| *until > now)
    }

    /// When the venue's suspension in `direction` ends, if it is suspended at
    /// `now`.
    pub fn venue_suspended_until(
        &self,
        program: ProgramKind,
        direction: Direction,
        now: Instant,
    ) -> Option<Instant> {
        let state = self.lock();
        state
            .venues
            .get(&(program, direction))
            .and_then(|record| record.suspended_until)
            .filter(|until| *until > now)
    }

    /// Record a failure through `pool` that [`strikes`]; returns what it newly
    /// suspended.
    pub fn record_failure(
        &self,
        pool: Pubkey,
        program: ProgramKind,
        direction: Direction,
        now: Instant,
    ) -> Option<Tripped> {
        let mut state = self.lock();
        if !state.pools.contains_key(&pool) {
            prune(&mut state, now);
        }

        let mut tripped = None;
        let record = state.pools.entry(pool).or_default();
        record.program = Some(program);
        record
            .strikes
            .retain(|at| now.saturating_duration_since(*at) <= STRIKE_WINDOW);
        record.strikes.push(now);
        let pool_active = record.suspended_until.is_some_and(|until| until > now);
        if record.strikes.len() >= POOL_STRIKES_TO_SUSPEND && !pool_active {
            record.suspended_until = Some(now + SUSPENSION);
            record.strikes.clear();
            tripped = Some(Tripped::Pool);
        }

        let venue = state.venues.entry((program, direction)).or_default();
        venue
            .strikes
            .retain(|(_, at)| now.saturating_duration_since(*at) <= STRIKE_WINDOW);
        venue.strikes.retain(|(struck, _)| *struck != pool);
        venue.strikes.push((pool, now));
        let venue_active = venue.suspended_until.is_some_and(|until| until > now);
        if venue.strikes.len() >= VENUE_POOLS_TO_SUSPEND && !venue_active {
            venue.suspended_until = Some(now + SUSPENSION);
            venue.strikes.clear();
            tripped = Some(Tripped::Venue);
        }
        tripped
    }

    /// Record a settled swap through `pool`: the pool and the venue in that
    /// direction both work, so their strikes are cleared.
    pub fn record_success(&self, pool: Pubkey, program: ProgramKind, direction: Direction) {
        let mut state = self.lock();
        if let Some(record) = state.pools.get_mut(&pool) {
            record.strikes.clear();
            record.suspended_until = None;
        }
        if let Some(venue) = state.venues.get_mut(&(program, direction)) {
            venue.strikes.clear();
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, State> {
        // The state is a cache: a panic while it was held leaves nothing to
        // protect, so a poisoned lock is recovered rather than propagated.
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

/// Make room for a new pool record: expired entries first, then the oldest
/// unsuspended ones.
fn prune(state: &mut State, now: Instant) {
    if state.pools.len() < MAX_TRACKED_POOLS {
        return;
    }
    state.pools.retain(|_, record| {
        let suspended = record.suspended_until.is_some_and(|until| until > now);
        let recent = record
            .strikes
            .last()
            .is_some_and(|at| now.saturating_duration_since(*at) <= STRIKE_WINDOW);
        suspended || recent
    });
    while state.pools.len() >= MAX_TRACKED_POOLS {
        let oldest = state
            .pools
            .iter()
            .filter(|(_, record)| record.suspended_until.is_none())
            .min_by_key(|(_, record)| record.strikes.last().copied())
            .map(|(pool, _)| *pool);
        match oldest {
            Some(pool) => {
                state.pools.remove(&pool);
            }
            None => break,
        }
    }
}

/// The breaker the direct router consults.
pub fn global() -> &'static Breaker {
    static BREAKER: OnceLock<Breaker> = OnceLock::new();
    BREAKER.get_or_init(Breaker::new)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pool(n: u8) -> Pubkey {
        Pubkey::new_from_array([n; 32])
    }

    const ORCA: ProgramKind = ProgramKind::OrcaWhirlpool;

    #[test]
    fn one_failure_does_not_suspend_a_pool() {
        let breaker = Breaker::new();
        let now = Instant::now();
        assert_eq!(
            breaker.record_failure(pool(1), ORCA, Direction::Sell, now),
            None
        );
        assert_eq!(breaker.pool_suspended_until(&pool(1), now), None);
    }

    #[test]
    fn failures_in_a_row_suspend_the_pool_until_the_suspension_ends() {
        let breaker = Breaker::new();
        let start = Instant::now();
        breaker.record_failure(pool(1), ORCA, Direction::Sell, start);
        let second = start + Duration::from_secs(60);
        assert_eq!(
            breaker.record_failure(pool(1), ORCA, Direction::Sell, second),
            Some(Tripped::Pool)
        );
        assert_eq!(
            breaker.pool_suspended_until(&pool(1), second),
            Some(second + SUSPENSION)
        );
        assert!(breaker
            .pool_suspended_until(&pool(1), second + SUSPENSION - Duration::from_secs(1))
            .is_some());
        assert_eq!(
            breaker.pool_suspended_until(&pool(1), second + SUSPENSION),
            None,
            "a suspension ends on its own"
        );
        assert_eq!(
            breaker.pool_suspended_until(&pool(2), second),
            None,
            "only the failing pool is suspended"
        );
    }

    #[test]
    fn strikes_further_apart_than_the_window_do_not_add_up() {
        let breaker = Breaker::new();
        let start = Instant::now();
        breaker.record_failure(pool(1), ORCA, Direction::Buy, start);
        let late = start + STRIKE_WINDOW + Duration::from_secs(1);
        assert_eq!(
            breaker.record_failure(pool(1), ORCA, Direction::Buy, late),
            None
        );
        assert_eq!(breaker.pool_suspended_until(&pool(1), late), None);
    }

    #[test]
    fn a_settled_swap_clears_the_pools_strikes() {
        let breaker = Breaker::new();
        let now = Instant::now();
        breaker.record_failure(pool(1), ORCA, Direction::Buy, now);
        breaker.record_success(pool(1), ORCA, Direction::Buy);
        assert_eq!(
            breaker.record_failure(pool(1), ORCA, Direction::Buy, now),
            None,
            "the count restarts after a success"
        );
    }

    #[test]
    fn distinct_failing_pools_suspend_the_venue_in_that_direction_only() {
        let breaker = Breaker::new();
        let now = Instant::now();
        assert_eq!(
            breaker.record_failure(pool(1), ORCA, Direction::Sell, now),
            None
        );
        assert_eq!(
            breaker.record_failure(pool(2), ORCA, Direction::Sell, now),
            None
        );
        assert_eq!(
            breaker.record_failure(pool(3), ORCA, Direction::Sell, now),
            Some(Tripped::Venue)
        );
        assert!(breaker
            .venue_suspended_until(ORCA, Direction::Sell, now)
            .is_some());
        assert_eq!(
            breaker.venue_suspended_until(ORCA, Direction::Buy, now),
            None,
            "the other direction keeps trading"
        );
        assert_eq!(
            breaker.venue_suspended_until(ProgramKind::RaydiumClmm, Direction::Sell, now),
            None,
            "other venues keep trading"
        );
    }

    #[test]
    fn one_pool_failing_repeatedly_counts_once_towards_its_venue() {
        let breaker = Breaker::new();
        let now = Instant::now();
        for _ in 0..5 {
            breaker.record_failure(pool(1), ORCA, Direction::Buy, now);
        }
        breaker.record_failure(pool(2), ORCA, Direction::Buy, now);
        assert_eq!(
            breaker.venue_suspended_until(ORCA, Direction::Buy, now),
            None
        );
    }

    #[test]
    fn only_failures_about_the_pool_strike() {
        let pool_faults = [
            DirectSwapError::Build {
                detail: String::new(),
            },
            DirectSwapError::SimulationRejected {
                detail: String::new(),
                logs: Vec::new(),
            },
            DirectSwapError::TransactionTooLarge {
                bytes: 1_300,
                limit: 1_232,
            },
            DirectSwapError::TransactionFailed {
                signature: String::new(),
                detail: String::new(),
            },
            DirectSwapError::PoolUndecodable {
                pool: pool(1),
                detail: String::new(),
            },
            DirectSwapError::QuoteMath {
                detail: String::new(),
            },
        ];
        for error in &pool_faults {
            assert!(strikes(error), "{error:?} must strike");
        }
        let not_pool_faults = [
            DirectSwapError::MarketMoved {
                pool: pool(1),
                accepted_min_net_out: 2,
                fresh_expected_net_out: 1,
            },
            DirectSwapError::NodeUnavailable {
                operation: "getLatestBlockhash",
                detail: String::new(),
            },
            DirectSwapError::SimulationUnavailable {
                detail: String::new(),
            },
            DirectSwapError::SubmitFailed {
                detail: String::new(),
            },
            DirectSwapError::BlockhashExpired {
                signature: String::new(),
                last_valid_block_height: 0,
                current_block_height: 1,
            },
            DirectSwapError::ConfirmationTimeout {
                signature: String::new(),
                waited_ms: 0,
            },
            DirectSwapError::OutputNotReceived {
                signature: String::new(),
                expected_minimum: 1,
                received: 0,
            },
            DirectSwapError::InsufficientBalance {
                mint: pool(1),
                required: 2,
                available: 1,
            },
            DirectSwapError::AccountUnavailable {
                address: pool(1),
                detail: String::new(),
            },
        ];
        for error in &not_pool_faults {
            assert!(!strikes(error), "{error:?} must not strike");
        }
    }

    #[test]
    fn the_memory_stays_bounded() {
        let breaker = Breaker::new();
        let now = Instant::now();
        for n in 0..(MAX_TRACKED_POOLS + 10) {
            let mut bytes = [0u8; 32];
            bytes[..8].copy_from_slice(&(n as u64).to_le_bytes());
            breaker.record_failure(
                Pubkey::new_from_array(bytes),
                ORCA,
                Direction::Buy,
                now + Duration::from_secs(n as u64 * 3_600),
            );
        }
        assert!(breaker.lock().pools.len() <= MAX_TRACKED_POOLS);
    }
}
