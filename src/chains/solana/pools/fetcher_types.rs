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
