//! Transaction database maintenance — cleanup, vacuuming, and old record pruning.
//
// Database maintenance, statistics, and integrity checks

use chrono::Utc;
use rusqlite::params;

use crate::logger::{self, LogTag};

use crate::transactions::error::Error;

use super::operations::TransactionDatabase;
use super::types::{DatabaseStats, IntegrityReport};

// =============================================================================
// IMPLEMENTATION - DATABASE STATISTICS AND MAINTENANCE
// =============================================================================

impl TransactionDatabase {
    /// Get comprehensive database statistics
    pub async fn get_stats(&self) -> Result<DatabaseStats, Error> {
        let conn = self.get_connection()?;
        let wallet_address =
            crate::utils::get_wallet_address().map_err(|e| Error::WalletUnavailable {
                detail: e.to_string(),
            })?;

        let raw_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM raw_transactions WHERE chain_id = ?1 AND wallet_address = ?2",
                params![self.chain.as_str(), wallet_address],
                |row| row.get(0),
            )
            .unwrap_or_default();

        let processed_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM processed_transactions WHERE chain_id = ?1 AND wallet_address = ?2",
                params![self.chain.as_str(), wallet_address],
                |row| row.get(0),
            )
            .unwrap_or_default();

        let known_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM known_signatures WHERE chain_id = ?1 AND wallet_address = ?2",
                params![self.chain.as_str(), wallet_address],
                |row| row.get(0),
            )
            .unwrap_or_default();

        let retries_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM deferred_retries WHERE chain_id = ?1",
                params![self.chain.as_str()],
                |row| row.get(0),
            )
            .unwrap_or_default();

        let pending_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM pending_transactions WHERE chain_id = ?1 AND wallet_address = ?2",
                params![self.chain.as_str(), wallet_address],
                |row| row.get(0),
            )
            .unwrap_or_default();

        // Get database file size
        let database_size = std::fs::metadata(&self.database_path)
            .map(|metadata| metadata.len())
            .unwrap_or_default();

        Ok(DatabaseStats {
            total_raw_transactions: raw_count as u64,
            total_processed_transactions: processed_count as u64,
            total_known_signatures: known_count as u64,
            total_deferred_retries: retries_count as u64,
            total_pending_transactions: pending_count as u64,
            database_size_bytes: database_size,
            schema_version: self.schema_version,
            last_updated: Utc::now(),
        })
    }

    /// Perform database maintenance (vacuum, analyze, cleanup)
    pub async fn perform_maintenance(&self) -> Result<(), Error> {
        let conn = self.get_connection()?;

        logger::info(LogTag::Transactions, "Starting database maintenance");

        // Vacuum to reclaim space
        conn.execute("VACUUM", [])
            .map_err(crate::errors::DatabaseError::from)?;

        // Analyze for query optimization
        conn.execute("ANALYZE", [])
            .map_err(crate::errors::DatabaseError::from)?;

        // Cleanup old pending transactions (older than 1 day)
        let cleaned_pending = conn
            .execute(
                "DELETE FROM pending_transactions WHERE chain_id = ?1 AND added_at < datetime('now', '-1 day')",
                params![self.chain.as_str()],
            )
            .map_err(crate::errors::DatabaseError::from)?;

        // Cleanup old deferred retries (older than 1 day with 0 attempts)
        let cleaned_retries = conn
            .execute(
                "DELETE FROM deferred_retries WHERE chain_id = ?1 AND remaining_attempts = 0 AND created_at < datetime('now', '-1 day')",
                params![self.chain.as_str()]
            )
            .map_err(crate::errors::DatabaseError::from)?;

        logger::info(
            LogTag::Transactions,
            &format!(
                "Database maintenance complete: cleaned {} pending, {} retries",
                cleaned_pending, cleaned_retries
            ),
        );

        Ok(())
    }

    /// Delete `raw_transactions` / `processed_transactions` rows older than
    /// `retention_days` for every subject EXCEPT `own_wallet_address`.
    ///
    /// The own wallet keeps unlimited retention; a watched target's rolling window
    /// (`wallet.watch_retention_days`) is what keeps `transactions.db` from being
    /// eaten by one busy KOL (§5.4). `known_signatures` is deliberately untouched --
    /// dedupe must keep working for a purged signature so it is never re-decoded, it
    /// just stops carrying analytics data forever.
    pub async fn cleanup_stale_target_transactions(
        &self,
        own_wallet_address: &str,
        retention_days: u32,
    ) -> Result<usize, Error> {
        let conn = self.get_connection()?;
        let cutoff_expr = format!("datetime('now', '-{retention_days} days')");

        let deleted_processed = conn
            .execute(
                &format!(
                    "DELETE FROM processed_transactions \
                     WHERE chain_id = ?1 AND wallet_address != ?2 AND processed_at < {cutoff_expr}"
                ),
                params![self.chain.as_str(), own_wallet_address],
            )
            .map_err(crate::errors::DatabaseError::from)?;

        let deleted_raw = conn
            .execute(
                &format!(
                    "DELETE FROM raw_transactions \
                     WHERE chain_id = ?1 AND wallet_address != ?2 AND created_at < {cutoff_expr}"
                ),
                params![self.chain.as_str(), own_wallet_address],
            )
            .map_err(crate::errors::DatabaseError::from)?;

        let total = deleted_processed + deleted_raw;
        if total > 0 {
            logger::info(
                LogTag::Transactions,
                &format!(
                    "Watch retention cleanup: removed {deleted_processed} processed + \
                     {deleted_raw} raw rows older than {retention_days}d for non-own subjects"
                ),
            );
        }

        Ok(total)
    }

    /// Get integrity report
    pub async fn get_integrity_report(&self) -> Result<IntegrityReport, Error> {
        let conn = self.get_connection()?;
        let wallet_address =
            crate::utils::get_wallet_address().map_err(|e| Error::WalletUnavailable {
                detail: e.to_string(),
            })?;

        let raw_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM raw_transactions WHERE chain_id = ?1 AND wallet_address = ?2",
                params![self.chain.as_str(), wallet_address],
                |row| row.get(0),
            )
            .unwrap_or_default();

        let processed_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM processed_transactions WHERE chain_id = ?1 AND wallet_address = ?2",
                params![self.chain.as_str(), wallet_address],
                |row| row.get(0),
            )
            .unwrap_or_default();

        let orphaned: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM processed_transactions WHERE chain_id = ?1 AND wallet_address = ?2 AND signature NOT IN (SELECT signature FROM raw_transactions WHERE chain_id = ?1 AND wallet_address = ?2)",
                params![self.chain.as_str(), wallet_address],
                |row| row.get(0)
            )
            .unwrap_or_default();

        let missing: i64 = raw_count - processed_count;

        let pending_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM pending_transactions WHERE chain_id = ?1 AND wallet_address = ?2",
                params![self.chain.as_str(), wallet_address],
                |row| row.get(0),
            )
            .unwrap_or_default();

        // Check schema version
        let schema_version_correct = conn
            .query_row(
                "SELECT value FROM db_metadata WHERE key = 'schema_version'",
                [],
                |row| {
                    let version_str: String = row.get(0)?;
                    Ok(version_str == self.schema_version.to_string())
                },
            )
            .unwrap_or_default();

        Ok(IntegrityReport {
            raw_transactions_count: raw_count as u64,
            processed_transactions_count: processed_count as u64,
            orphaned_processed_transactions: orphaned as u64,
            missing_processed_transactions: missing.max(0) as u64,
            schema_version_correct,
            foreign_key_violations: 0, // Would require FK check
            index_integrity_ok: true,  // Would require index check
            pending_transactions_count: pending_count as u64,
        })
    }
}

#[cfg(test)]
mod tests {
    use rusqlite::params;

    use super::TransactionDatabase;

    #[tokio::test]
    async fn target_retention_never_deletes_own_wallet_rows() {
        let dir = tempfile::tempdir().unwrap();
        let db = TransactionDatabase::new_with_path(
            dir.path().join("transactions.db"),
            crate::chains::ChainId::Solana,
        )
        .await
        .unwrap();
        let connection = db.get_connection().unwrap();
        for (signature, wallet, created_at) in [
            ("own-old", "own", "2020-01-01 00:00:00"),
            ("target-old", "target", "2020-01-01 00:00:00"),
            ("target-new", "target", "2999-01-01 00:00:00"),
        ] {
            connection.execute(
                "INSERT INTO raw_transactions (signature, wallet_address, timestamp, status, success, created_at) VALUES (?1, ?2, ?3, 'Confirmed', 1, ?3)",
                params![signature, wallet, created_at],
            ).unwrap();
        }
        for (signature, wallet, processed_at) in [
            ("own-old", "own", "2020-01-01 00:00:00"),
            ("target-old", "target", "2020-01-01 00:00:00"),
        ] {
            connection.execute(
                "INSERT INTO processed_transactions (signature, wallet_address, transaction_type, direction, processed_at) VALUES (?1, ?2, 'Unknown', 'Unknown', ?3)",
                params![signature, wallet, processed_at],
            ).unwrap();
        }
        drop(connection);

        assert_eq!(
            db.cleanup_stale_target_transactions("own", 30)
                .await
                .unwrap(),
            2
        );
        let connection = db.get_connection().unwrap();
        let remaining_raw: Vec<String> = connection
            .prepare("SELECT signature FROM raw_transactions ORDER BY signature")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert_eq!(remaining_raw, ["own-old", "target-new"]);
        let own_processed: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM processed_transactions WHERE signature='own-old'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(own_processed, 1);
    }
}
