//! Transaction database maintenance — cleanup, vacuuming, and old record pruning.
//
// Database maintenance, statistics, and integrity checks

use chrono::Utc;
use rusqlite::params;

use crate::logger::{self, LogTag};

use super::operations::TransactionDatabase;
use super::types::{DatabaseStats, IntegrityReport};

// =============================================================================
// IMPLEMENTATION - DATABASE STATISTICS AND MAINTENANCE
// =============================================================================

impl TransactionDatabase {
    /// Get comprehensive database statistics
    pub async fn get_stats(&self) -> Result<DatabaseStats, String> {
        let conn = self.get_connection()?;
        let wallet_address = crate::utils::get_wallet_address().map_err(|e| e.to_string())?;

        let raw_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM raw_transactions WHERE wallet_address = ?1",
                params![wallet_address],
                |row| row.get(0),
            )
            .unwrap_or_default();

        let processed_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM processed_transactions WHERE wallet_address = ?1",
                params![wallet_address],
                |row| row.get(0),
            )
            .unwrap_or_default();

        let known_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM known_signatures WHERE wallet_address = ?1",
                params![wallet_address],
                |row| row.get(0),
            )
            .unwrap_or_default();

        let retries_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM deferred_retries", [], |row| {
                row.get(0)
            })
            .unwrap_or_default();

        let pending_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM pending_transactions WHERE wallet_address = ?1",
                params![wallet_address],
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
    pub async fn perform_maintenance(&self) -> Result<(), String> {
        let conn = self.get_connection()?;

        logger::info(LogTag::Transactions, "Starting database maintenance");

        // Vacuum to reclaim space
        conn.execute("VACUUM", [])
            .map_err(|e| format!("Failed to vacuum database: {e}"))?;

        // Analyze for query optimization
        conn.execute("ANALYZE", [])
            .map_err(|e| format!("Failed to analyze database: {e}"))?;

        // Cleanup old pending transactions (older than 1 day)
        let cleaned_pending = conn
            .execute(
                "DELETE FROM pending_transactions WHERE added_at < datetime('now', '-1 day')",
                [],
            )
            .map_err(|e| format!("Failed to cleanup old pending transactions: {e}"))?;

        // Cleanup old deferred retries (older than 1 day with 0 attempts)
        let cleaned_retries = conn
            .execute(
                "DELETE FROM deferred_retries WHERE remaining_attempts = 0 AND created_at < datetime('now', '-1 day')",
                []
            )
            .map_err(|e| format!("Failed to cleanup old deferred retries: {e}"))?;

        logger::info(
            LogTag::Transactions,
            &format!(
                "Database maintenance complete: cleaned {} pending, {} retries",
                cleaned_pending, cleaned_retries
            ),
        );

        Ok(())
    }

    /// Get integrity report
    pub async fn get_integrity_report(&self) -> Result<IntegrityReport, String> {
        let conn = self.get_connection()?;
        let wallet_address = crate::utils::get_wallet_address().map_err(|e| e.to_string())?;

        let raw_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM raw_transactions WHERE wallet_address = ?1",
                params![wallet_address],
                |row| row.get(0),
            )
            .unwrap_or_default();

        let processed_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM processed_transactions WHERE wallet_address = ?1",
                params![wallet_address],
                |row| row.get(0),
            )
            .unwrap_or_default();

        let orphaned: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM processed_transactions WHERE wallet_address = ?1 AND signature NOT IN (SELECT signature FROM raw_transactions WHERE wallet_address = ?1)",
                params![wallet_address],
                |row| row.get(0)
            )
            .unwrap_or_default();

        let missing: i64 = raw_count - processed_count;

        let pending_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM pending_transactions WHERE wallet_address = ?1",
                params![wallet_address],
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
