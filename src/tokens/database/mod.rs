// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! SQLite persistence layer for token metadata, market data, and blacklists.
mod assembly;
mod async_api;
mod authority;
mod blacklist;
mod helpers;
mod market;
mod media;
/// Unified database operations for tokens system
///
/// Split into focused submodules:
/// - metadata: Token CRUD operations
/// - market: DexScreener and GeckoTerminal market data
/// - security: Rugcheck security data
/// - pool_data: Token pool snapshots
/// - rejections: Rejection tracking, history, and stats
/// - blacklist: Token blacklist operations
/// - priority: Priority management
/// - tracking: Update tracking and diagnostics
/// - assembly: Complex token assembly queries
/// - async_api: Async convenience wrappers
mod metadata;
mod pool_data;
mod priority;
mod rejections;
pub mod retention;
mod security;
mod tracking;

#[cfg(test)]
mod authority_tests;
#[cfg(test)]
mod rejections_tests;
#[cfg(test)]
mod retention_tests;

use crate::errors::DatabaseError;
use arc_swap::ArcSwapOption;
use r2d2::{Pool, PooledConnection};
use r2d2_sqlite::SqliteConnectionManager;
use std::sync::Arc;

use crate::chains::{ChainId, PerChain};
use crate::tokens::types::TokenResult;
use crate::tokens::Error;

// One installed database handle per chain. A slot is written once per boot,
// after its chain's file opens and before that chain's loops start; reads are
// lock-free.
static DATABASES: PerChain<ArcSwapOption<TokenDatabase>> = PerChain::new(empty_database_slot);

fn empty_database_slot(_chain: ChainId) -> ArcSwapOption<TokenDatabase> {
    ArcSwapOption::empty()
}

/// Install an opened database into its own chain's slot (`db.chain()`).
pub fn install_database(db: Arc<TokenDatabase>) {
    DATABASES.get(db.chain()).store(Some(db));
}

/// The installed database for `chain`; `None` before the tokens service has
/// opened it, and for a chain that is not enabled.
pub fn database(chain: ChainId) -> Option<Arc<TokenDatabase>> {
    DATABASES.get(chain).load_full()
}

/// The installed database for `chain`, or `NotInitialized` naming the chain.
pub(crate) fn require_database(chain: ChainId) -> TokenResult<Arc<TokenDatabase>> {
    database(chain).ok_or_else(|| Error::NotInitialized {
        resource: format!("token database ({chain})"),
    })
}

/// Remove every installed database handle (service stop, tests).
pub fn uninstall_databases() {
    for (_, slot) in DATABASES.built() {
        slot.store(None);
    }
}

/// Token database.
///
/// A real r2d2 pool, matching every other database in the bot. It used to be a single
/// `Mutex<Connection>` shared by the whole process, which made the token database a global
/// serialization point: the filtering snapshot's full-corpus batch load held that one
/// connection for seconds, and EVERY unrelated token read queued behind it — the header's
/// counts, the wallet's balances, the positions panel. Measured against the owner's
/// database, a bare `SELECT COUNT(*)` issued during a snapshot build took 8.9s, tracking
/// the build's duration exactly rather than the ~20ms the query itself costs.
///
/// The database is in WAL mode (see `database::configure_connection`), so readers run
/// concurrently with each other and with a writer; `busy_timeout` covers writer overlap.
pub struct TokenDatabase {
    pool: Pool<SqliteConnectionManager>,
    chain: ChainId,
}

/// Token-level blacklist entry with metadata for diagnostics and UI
#[derive(Debug, Clone)]
pub struct TokenBlacklistRecord {
    pub mint: String,
    pub reason: String,
    pub source: String,
    pub added_at: i64,
}

impl TokenDatabase {
    /// Open `chain`'s token store at `path`.
    pub fn new(path: &str, chain: ChainId) -> TokenResult<Self> {
        let pool = crate::database::open::pool_at(
            &crate::database::catalog::TOKENS,
            std::path::Path::new(path),
            vec![chain],
        )?;
        Ok(Self { pool, chain })
    }

    /// Check out a connection from the pool.
    pub(super) fn conn(&self) -> TokenResult<PooledConnection<SqliteConnectionManager>> {
        self.pool.get().map_err(|e| {
            Error::Database(DatabaseError::Query {
                operation: "Failed to get connection".to_owned(),
                message: e.to_string(),
            })
        })
    }

    /// The explicit canonical chain scope for every operation issued by this repository.
    pub const fn chain(&self) -> ChainId {
        self.chain
    }

    pub(crate) const fn chain_id(&self) -> &'static str {
        self.chain.as_str()
    }
}

// Re-export all public items from submodules
pub use async_api::*;
pub use authority::CreatorReputationRow;
pub use rejections::{FilterBuildWrite, FilterOutcome, FilterRejectionRecord, RecentRejectionRow};
