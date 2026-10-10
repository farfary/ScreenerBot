// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Transaction database operations — CRUD methods for transaction records.
//
// Core database operations and implementation

use chrono::{DateTime, Utc};
use r2d2::{Pool, PooledConnection};
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
#[cfg(test)]
use std::path::Path;
use std::path::PathBuf;

use crate::chains::ChainId;
use crate::database::{self, catalog};
use crate::transactions::error::Error;
use crate::transactions::types::*;

use super::schema::*;

// =============================================================================
// TRANSACTION DATABASE MANAGER
// =============================================================================

/// High-performance, thread-safe database manager for transactions
///
/// Features:
/// - Connection pooling for concurrent access
/// - Separation of raw blockchain data from analysis results
/// - ACID transactions for data integrity
/// - High-performance batch operations
/// - Comprehensive indexing for fast queries
/// - Built-in health checks and integrity validation
pub struct TransactionDatabase {
    pub(super) pool: Pool<SqliteConnectionManager>,
    pub(super) database_path: String,
    pub(super) schema_version: u32,
    pub(super) chain: ChainId,
}

/// Minimal row for wallet flow cache export
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletFlowExportRow {
    pub signature: String,
    pub timestamp: DateTime<Utc>,
    pub native_delta: f64,
}

impl TransactionDatabase {
    /// The transaction database of `chain`, opened by the storage phase, with the
    /// active wallet recorded as the wallet its rows describe.
    pub async fn new(chain: ChainId) -> Result<Self, Error> {
        let pool = database::open::store(&catalog::TRANSACTIONS)?;
        let db = Self::with_pool(pool, catalog::TRANSACTIONS.path(chain), chain);
        db.record_current_wallet()?;
        Ok(db)
    }

    #[cfg(test)]
    pub(crate) async fn new_with_path<P: AsRef<Path>>(
        path: P,
        chain: ChainId,
    ) -> Result<Self, Error> {
        let path = path.as_ref().to_path_buf();
        let pool = database::open::pool_at(&catalog::TRANSACTIONS, &path, vec![chain])?;
        Ok(Self::with_pool(pool, path, chain))
    }

    fn with_pool(pool: Pool<SqliteConnectionManager>, path: PathBuf, chain: ChainId) -> Self {
        Self {
            pool,
            database_path: path.to_string_lossy().to_string(),
            schema_version: DATABASE_SCHEMA_VERSION,
            chain,
        }
    }

    /// Records the active wallet as the wallet the stored rows describe.
    fn record_current_wallet(&self) -> Result<(), Error> {
        let wallet_address =
            crate::utils::get_wallet_address().map_err(|e| Error::WalletUnavailable {
                detail: e.to_string(),
            })?;
        self.get_connection()?
            .execute(
                "INSERT OR REPLACE INTO db_metadata (key, value) VALUES (?1, ?2)",
                params!["current_wallet", wallet_address],
            )
            .map_err(|e| {
                crate::errors::DatabaseError::classify_sqlite_failure("record current wallet", e)
            })?;
        Ok(())
    }

    /// Get database connection from pool
    pub(super) fn get_connection(
        &self,
    ) -> Result<PooledConnection<SqliteConnectionManager>, Error> {
        Ok(self
            .pool
            .get()
            .map_err(crate::errors::DatabaseError::from)?)
    }

    pub(super) fn require_subject_chain(&self, subject: &Subject) -> Result<&'static str, Error> {
        if subject.chain() != self.chain {
            return Err(Error::ChainMismatch {
                expected: self.chain,
                actual: subject.chain(),
            });
        }
        Ok(self.chain.as_str())
    }

    /// Health check - verify database connectivity and basic operations
    pub async fn health_check(&self) -> Result<(), Error> {
        let conn = self.get_connection()?;

        // Test basic query
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table'",
                [],
                |row| row.get(0),
            )
            .map_err(crate::errors::DatabaseError::from)?;

        if count < 5 {
            return Err(Error::SchemaInspect {
                detail: "database schema incomplete".to_owned(),
            });
        }

        Ok(())
    }
}

// =============================================================================
// IMPLEMENTATION - KNOWN SIGNATURES MANAGEMENT
// =============================================================================

impl TransactionDatabase {
    /// Check if signature is known in database, for the given subject
    pub async fn is_signature_known(
        &self,
        subject: Subject,
        signature: &str,
    ) -> Result<bool, Error> {
        let conn = self.get_connection()?;
        let wallet_address = subject.address();
        let chain_id = self.require_subject_chain(&subject)?;

        let exists: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM known_signatures WHERE chain_id = ?1 AND signature = ?2 AND wallet_address = ?3)",
                params![chain_id, signature, wallet_address],
                |row| row.get(0),
            )
            .map_err(crate::errors::DatabaseError::from)?;

        Ok(exists)
    }

    /// Add signature to known signatures, for the given subject
    pub async fn add_known_signature(
        &self,
        subject: Subject,
        signature: &str,
    ) -> Result<(), Error> {
        let conn = self.get_connection()?;
        let wallet_address = subject.address();
        let chain_id = self.require_subject_chain(&subject)?;

        conn.execute(
            "INSERT OR IGNORE INTO known_signatures (chain_id, signature, wallet_address) VALUES (?1, ?2, ?3)",
            params![chain_id, signature, wallet_address],
        )
        .map_err(crate::errors::DatabaseError::from)?;

        Ok(())
    }

    /// Get count of known signatures, for the given subject
    pub async fn get_known_signatures_count(&self, subject: Subject) -> Result<u64, Error> {
        let conn = self.get_connection()?;
        let wallet_address = subject.address();
        let chain_id = self.require_subject_chain(&subject)?;

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM known_signatures WHERE chain_id = ?1 AND wallet_address = ?2",
                params![chain_id, wallet_address],
                |row| row.get(0),
            )
            .map_err(crate::errors::DatabaseError::from)?;

        Ok(count as u64)
    }

    /// Get the newest known signature (most recently added), for the given subject
    pub async fn get_newest_known_signature(
        &self,
        subject: Subject,
    ) -> Result<Option<String>, Error> {
        let conn = self.get_connection()?;
        let wallet_address = subject.address();
        let chain_id = self.require_subject_chain(&subject)?;

        let result: Option<String> = conn
            .query_row(
                "SELECT signature FROM known_signatures WHERE chain_id = ?1 AND wallet_address = ?2 ORDER BY added_at DESC LIMIT 1",
                params![chain_id, wallet_address],
                |row| row.get(0),
            )
            .optional()
            .map_err(crate::errors::DatabaseError::from)?;

        Ok(result)
    }

    /// Get the oldest known signature for incremental fetching checkpoint, for the given subject
    /// Returns None if no signatures are known yet (first run)
    ///
    /// When fetching backwards from blockchain (newest→oldest), we stop when we hit
    /// the oldest signature we already have, ensuring we only fetch missing history.
    pub async fn get_oldest_known_signature(
        &self,
        subject: Subject,
    ) -> Result<Option<String>, Error> {
        let conn = self.get_connection()?;
        let wallet_address = subject.address();
        let chain_id = self.require_subject_chain(&subject)?;

        let result: Option<String> = conn
            .query_row(
                "SELECT signature FROM known_signatures WHERE chain_id = ?1 AND wallet_address = ?2 ORDER BY added_at ASC LIMIT 1",
                params![chain_id, wallet_address],
                |row| row.get(0),
            )
            .optional()
            .map_err(crate::errors::DatabaseError::from)?;

        Ok(result)
    }

    /// Remove old known signatures (cleanup)
    pub async fn cleanup_old_known_signatures(&self, days: i64) -> Result<usize, Error> {
        let conn = self.get_connection()?;

        let affected = conn
            .execute(
                "DELETE FROM known_signatures WHERE chain_id = ?1 AND added_at < datetime('now', '-' || ?2 || ' days')",
                params![self.chain.as_str(), days]
            )
            .map_err(crate::errors::DatabaseError::from)?;

        Ok(affected)
    }
}
