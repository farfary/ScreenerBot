//! Global pool database singleton — provides shared access to the pool data store.

use super::super::types::{PriceResult, PRICE_HISTORY_MAX_ENTRIES};
/// Global database instance and convenience functions
use super::operations::PoolsDatabase;
use super::types::{BlacklistedAccountRecord, BlacklistedPoolRecord};
use crate::chains::ChainId;
use crate::errors::InternalError;
use crate::pools::Error;

use std::sync::LazyLock;
use std::sync::RwLock;

// =============================================================================
// GLOBAL DATABASE INSTANCE
// =============================================================================

/// Global database instance - thread-safe using Lazy + RwLock pattern
static GLOBAL_POOLS_DB: LazyLock<RwLock<Option<PoolsDatabase>>> =
    LazyLock::new(|| RwLock::new(None));

/// Initialize the global pools database
pub async fn initialize_database(chain_id: ChainId) -> Result<(), Error> {
    let mut db = PoolsDatabase::new(chain_id);
    db.initialize().await?;

    match GLOBAL_POOLS_DB.write() {
        Ok(mut guard) => {
            *guard = Some(db);
            Ok(())
        }
        Err(e) => Err(InternalError::InvariantViolation {
            message: format!("pools database write lock poisoned: {e}"),
        }
        .into()),
    }
}

/// Queue a price for storage in the global database
pub async fn queue_price_for_storage(chain_id: ChainId, price: PriceResult) -> Result<(), Error> {
    let db_ref = match GLOBAL_POOLS_DB.read() {
        Ok(guard) => match guard.as_ref() {
            Some(db) => db.clone_for_async(),
            None => return Err(Error::NotInitialized),
        },
        Err(e) => {
            return Err(InternalError::InvariantViolation {
                message: format!("pools database read lock poisoned: {e}"),
            }
            .into())
        }
    };
    if db_ref.chain_id != chain_id {
        return Err(Error::ChainMismatch {
            bound: db_ref.chain_id,
            requested: chain_id,
        });
    }
    db_ref.queue_price_for_storage(price).await
}

/// Load recent price history for cache initialization
pub async fn load_historical_data_for_token(
    chain_id: ChainId,
    mint: &str,
) -> Result<Vec<PriceResult>, Error> {
    let db_ref = match GLOBAL_POOLS_DB.read() {
        Ok(guard) => match guard.as_ref() {
            Some(db) => db.clone_for_async(),
            None => return Ok(Vec::new()),
        },
        Err(_) => return Ok(Vec::new()),
    };
    if db_ref.chain_id != chain_id {
        return Ok(Vec::new());
    }
    db_ref
        .load_recent_price_history(mint, PRICE_HISTORY_MAX_ENTRIES)
        .await
}

/// Get extended price history from database
pub async fn get_extended_price_history(
    chain_id: ChainId,
    mint: &str,
    limit: Option<usize>,
    since_timestamp: Option<i64>,
) -> Result<Vec<PriceResult>, Error> {
    let db_ref = match GLOBAL_POOLS_DB.read() {
        Ok(guard) => match guard.as_ref() {
            Some(db) => db.clone_for_async(),
            None => return Err(Error::NotInitialized),
        },
        Err(e) => {
            return Err(InternalError::InvariantViolation {
                message: format!("pools database read lock poisoned: {e}"),
            }
            .into())
        }
    };
    if db_ref.chain_id != chain_id {
        return Ok(Vec::new());
    }
    db_ref.get_price_history(mint, limit, since_timestamp).await
}

/// Cleanup old database entries
pub async fn cleanup_old_entries(chain_id: ChainId) -> Result<usize, Error> {
    let db_ref = match GLOBAL_POOLS_DB.read() {
        Ok(guard) => match guard.as_ref() {
            Some(db) => db.clone_for_async(),
            None => return Ok(0),
        },
        Err(_) => return Ok(0),
    };
    if db_ref.chain_id != chain_id {
        return Ok(0);
    }
    db_ref.cleanup_old_entries().await
}

/// Cleanup gapped data for a specific token
pub async fn cleanup_gapped_data_for_token(chain_id: ChainId, mint: &str) -> Result<usize, Error> {
    let db_ref = match GLOBAL_POOLS_DB.read() {
        Ok(guard) => match guard.as_ref() {
            Some(db) => db.clone_for_async(),
            None => return Ok(0),
        },
        Err(_) => return Ok(0),
    };
    if db_ref.chain_id != chain_id {
        return Ok(0);
    }
    db_ref.cleanup_gapped_data_for_token(mint).await
}

/// Cleanup gapped data for all tokens
pub async fn cleanup_all_gapped_data(chain_id: ChainId) -> Result<usize, Error> {
    let db_ref = match GLOBAL_POOLS_DB.read() {
        Ok(guard) => match guard.as_ref() {
            Some(db) => db.clone_for_async(),
            None => return Ok(0),
        },
        Err(_) => return Ok(0),
    };
    if db_ref.chain_id != chain_id {
        return Ok(0);
    }
    db_ref.cleanup_all_gapped_data().await
}

/// Add account to blacklist (global helper)
pub async fn add_account_to_blacklist(
    chain_id: ChainId,
    account_pubkey: &str,
    reason: &str,
    source: Option<&str>,
    pool_id: Option<&str>,
    token_mint: Option<&str>,
) -> Result<(), Error> {
    let db_ref = match GLOBAL_POOLS_DB.read() {
        Ok(guard) => match guard.as_ref() {
            Some(db) => db.clone_for_async(),
            None => return Err(Error::NotInitialized),
        },
        Err(e) => {
            return Err(InternalError::InvariantViolation {
                message: format!("pools database read lock poisoned: {e}"),
            }
            .into())
        }
    };
    if db_ref.chain_id != chain_id {
        return Err(Error::ChainMismatch {
            bound: db_ref.chain_id,
            requested: chain_id,
        });
    }
    db_ref
        .add_account_to_blacklist(account_pubkey, reason, source, pool_id, token_mint)
        .await
}

/// Check if account is blacklisted (global helper)
pub async fn is_account_blacklisted(
    chain_id: ChainId,
    account_pubkey: &str,
) -> Result<bool, Error> {
    let db_ref = match GLOBAL_POOLS_DB.read() {
        Ok(guard) => match guard.as_ref() {
            Some(db) => db.clone_for_async(),
            None => return Err(Error::NotInitialized),
        },
        Err(e) => {
            return Err(InternalError::InvariantViolation {
                message: format!("pools database read lock poisoned: {e}"),
            }
            .into())
        }
    };
    if db_ref.chain_id != chain_id {
        return Ok(false);
    }
    db_ref.is_account_blacklisted(account_pubkey).await
}

/// Add pool to blacklist (global helper)
pub async fn add_pool_to_blacklist(
    chain_id: ChainId,
    pool_id: &str,
    reason: &str,
    token_mint: Option<&str>,
    program_id: Option<&str>,
) -> Result<(), Error> {
    let db_ref = match GLOBAL_POOLS_DB.read() {
        Ok(guard) => match guard.as_ref() {
            Some(db) => db.clone_for_async(),
            None => return Err(Error::NotInitialized),
        },
        Err(e) => {
            return Err(InternalError::InvariantViolation {
                message: format!("pools database read lock poisoned: {e}"),
            }
            .into())
        }
    };
    if db_ref.chain_id != chain_id {
        return Err(Error::ChainMismatch {
            bound: db_ref.chain_id,
            requested: chain_id,
        });
    }
    db_ref
        .add_pool_to_blacklist(pool_id, reason, token_mint, program_id)
        .await
}

/// Check if pool is blacklisted (global helper)
pub async fn is_pool_blacklisted(chain_id: ChainId, pool_id: &str) -> Result<bool, Error> {
    let db_ref = match GLOBAL_POOLS_DB.read() {
        Ok(guard) => match guard.as_ref() {
            Some(db) => db.clone_for_async(),
            None => return Err(Error::NotInitialized),
        },
        Err(e) => {
            return Err(InternalError::InvariantViolation {
                message: format!("pools database read lock poisoned: {e}"),
            }
            .into())
        }
    };
    if db_ref.chain_id != chain_id {
        return Ok(false);
    }
    db_ref.is_pool_blacklisted(pool_id).await
}

/// Get blacklist statistics (global helper)
pub async fn get_blacklist_stats(chain_id: ChainId) -> Result<(usize, usize), Error> {
    let db_ref = match GLOBAL_POOLS_DB.read() {
        Ok(guard) => match guard.as_ref() {
            Some(db) => db.clone_for_async(),
            None => return Ok((0, 0)),
        },
        Err(_) => return Ok((0, 0)),
    };
    if db_ref.chain_id != chain_id {
        return Ok((0, 0));
    }
    db_ref.get_blacklist_stats().await
}

/// List blacklisted accounts with an optional limit.
pub async fn list_blacklisted_accounts(
    chain_id: ChainId,
    limit: Option<usize>,
) -> Result<Vec<BlacklistedAccountRecord>, Error> {
    let db_ref = match GLOBAL_POOLS_DB.read() {
        Ok(guard) => match guard.as_ref() {
            Some(db) => db.clone_for_async(),
            None => return Ok(Vec::new()),
        },
        Err(_) => return Ok(Vec::new()),
    };
    if db_ref.chain_id != chain_id {
        return Ok(Vec::new());
    }
    db_ref.list_blacklisted_accounts(limit).await
}

/// List blacklisted pools with an optional limit.
pub async fn list_blacklisted_pools(
    chain_id: ChainId,
    limit: Option<usize>,
) -> Result<Vec<BlacklistedPoolRecord>, Error> {
    let db_ref = match GLOBAL_POOLS_DB.read() {
        Ok(guard) => match guard.as_ref() {
            Some(db) => db.clone_for_async(),
            None => return Ok(Vec::new()),
        },
        Err(_) => return Ok(Vec::new()),
    };
    if db_ref.chain_id != chain_id {
        return Ok(Vec::new());
    }
    db_ref.list_blacklisted_pools(limit).await
}
