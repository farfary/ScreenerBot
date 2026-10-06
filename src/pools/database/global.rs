// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Per-chain pools database instances and the facade every caller goes
//! through. Each function takes the chain whose database it reads or writes.

use super::super::types::{PoolBlacklistPolicy, PriceResult, PRICE_HISTORY_MAX_ENTRIES};
use super::operations::PoolsDatabase;
use super::types::{BlacklistedAccountRecord, BlacklistedPoolRecord, PoolFailureOutcome};
use crate::chains::{ChainId, PerChain};
use crate::pools::Error;

use arc_swap::ArcSwapOption;
use std::sync::Arc;

// =============================================================================
// PER-CHAIN DATABASE INSTANCES
// =============================================================================

// One installed pools database per chain. A slot is written when the pools
// service opens that chain's file; reads are lock-free. Reinstalling replaces
// the previous handle, whose writer flushes and exits once its sender drops.
static DATABASES: PerChain<ArcSwapOption<PoolsDatabase>> = PerChain::new(empty_database_slot);

fn empty_database_slot(_chain: ChainId) -> ArcSwapOption<PoolsDatabase> {
    ArcSwapOption::empty()
}

/// Install an opened database into its own chain's slot (`db.chain()`).
pub(crate) fn install_database(db: Arc<PoolsDatabase>) {
    DATABASES.get(db.chain()).store(Some(db));
}

/// The installed pools database for `chain`; `None` before the pools service
/// has opened it, and for a chain that is not enabled.
pub(crate) fn database(chain: ChainId) -> Option<Arc<PoolsDatabase>> {
    DATABASES.get(chain).load_full()
}

/// Open `chain`'s pools database and install it into that chain's slot.
pub async fn initialize_database(chain: ChainId) -> Result<(), Error> {
    let mut db = PoolsDatabase::new(chain);
    db.initialize().await?;
    install_database(Arc::new(db));
    Ok(())
}

/// Queue a price for storage in `chain`'s database (a non-blocking send).
pub fn queue_price_for_storage(chain: ChainId, price: PriceResult) -> Result<(), Error> {
    database(chain)
        .ok_or(Error::NotInitialized)?
        .queue_price_for_storage(price)
}

/// Load recent price history for cache initialization
pub async fn load_historical_data_for_token(
    chain: ChainId,
    mint: &str,
) -> Result<Vec<PriceResult>, Error> {
    let Some(db) = database(chain) else {
        return Ok(Vec::new());
    };
    db.load_recent_price_history(mint, PRICE_HISTORY_MAX_ENTRIES)
        .await
}

/// Cleanup old database entries
pub async fn cleanup_old_entries(chain: ChainId) -> Result<usize, Error> {
    let Some(db) = database(chain) else {
        return Ok(0);
    };
    db.cleanup_old_entries().await
}

/// Cleanup gapped data for a specific token
pub async fn cleanup_gapped_data_for_token(chain: ChainId, mint: &str) -> Result<usize, Error> {
    let Some(db) = database(chain) else {
        return Ok(0);
    };
    db.cleanup_gapped_data_for_token(mint).await
}

/// Cleanup gapped data for all tokens
pub async fn cleanup_all_gapped_data(chain: ChainId) -> Result<usize, Error> {
    let Some(db) = database(chain) else {
        return Ok(0);
    };
    db.cleanup_all_gapped_data().await
}

/// Add account to blacklist (global helper)
pub async fn add_account_to_blacklist(
    chain: ChainId,
    account_pubkey: &str,
    reason: &str,
    source: Option<&str>,
    pool_id: Option<&str>,
    token_mint: Option<&str>,
) -> Result<(), Error> {
    database(chain)
        .ok_or(Error::NotInitialized)?
        .add_account_to_blacklist(account_pubkey, reason, source, pool_id, token_mint)
        .await
}

/// Check if account is blacklisted (global helper)
pub async fn is_account_blacklisted(chain: ChainId, account_pubkey: &str) -> Result<bool, Error> {
    database(chain)
        .ok_or(Error::NotInitialized)?
        .is_account_blacklisted(account_pubkey)
        .await
}

/// Record `observed_failures` failures of a pool under the configured pool
/// blacklist policy (global helper). See `PoolsDatabase::add_pool_to_blacklist`.
pub async fn add_pool_to_blacklist(
    chain: ChainId,
    pool_id: &str,
    reason: &str,
    token_mint: Option<&str>,
    program_id: Option<&str>,
    observed_failures: u32,
) -> Result<PoolFailureOutcome, Error> {
    database(chain)
        .ok_or(Error::NotInitialized)?
        .add_pool_to_blacklist(
            pool_id,
            reason,
            token_mint,
            program_id,
            observed_failures,
            PoolBlacklistPolicy::from_config(),
        )
        .await
}

/// Check if pool is blacklisted (global helper)
pub async fn is_pool_blacklisted(chain: ChainId, pool_id: &str) -> Result<bool, Error> {
    database(chain)
        .ok_or(Error::NotInitialized)?
        .is_pool_blacklisted(pool_id)
        .await
}

/// List blacklisted accounts with an optional limit.
pub async fn list_blacklisted_accounts(
    chain: ChainId,
    limit: Option<usize>,
) -> Result<Vec<BlacklistedAccountRecord>, Error> {
    let Some(db) = database(chain) else {
        return Ok(Vec::new());
    };
    db.list_blacklisted_accounts(limit).await
}

/// List pools currently blacklisted under the configured policy, with an optional limit.
pub async fn list_blacklisted_pools(
    chain: ChainId,
    limit: Option<usize>,
) -> Result<Vec<BlacklistedPoolRecord>, Error> {
    let Some(db) = database(chain) else {
        return Ok(Vec::new());
    };
    db.list_blacklisted_pools(limit, PoolBlacklistPolicy::from_config())
        .await
}

#[cfg(test)]
mod tests {
    use super::*;

    const CHAIN: ChainId = ChainId::Solana;

    #[tokio::test]
    async fn an_uninstalled_database_keeps_its_uninitialized_results() {
        assert!(database(CHAIN).is_none());
        assert!(matches!(
            queue_price_for_storage(CHAIN, PriceResult::default()),
            Err(Error::NotInitialized)
        ));
        assert!(load_historical_data_for_token(CHAIN, "mint")
            .await
            .expect("empty history")
            .is_empty());
        assert_eq!(cleanup_old_entries(CHAIN).await.expect("no rows"), 0);
        assert_eq!(
            cleanup_gapped_data_for_token(CHAIN, "mint")
                .await
                .expect("no rows"),
            0
        );
        assert_eq!(cleanup_all_gapped_data(CHAIN).await.expect("no rows"), 0);
        assert!(list_blacklisted_accounts(CHAIN, None)
            .await
            .expect("empty list")
            .is_empty());
        assert!(list_blacklisted_pools(CHAIN, None)
            .await
            .expect("empty list")
            .is_empty());
        assert!(matches!(
            is_account_blacklisted(CHAIN, "account").await,
            Err(Error::NotInitialized)
        ));
        assert!(matches!(
            is_pool_blacklisted(CHAIN, "pool").await,
            Err(Error::NotInitialized)
        ));
        assert!(matches!(
            add_account_to_blacklist(CHAIN, "account", "reason", None, None, None).await,
            Err(Error::NotInitialized)
        ));
    }
}
