// Database maintenance, statistics, and bootstrap operations

use chrono::{DateTime, Utc};
use r2d2::PooledConnection;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::logger::{self, LogTag};
use crate::transactions::{types::*, utils::*};

use super::operations::TransactionDatabase;
use super::schema::DATABASE_SCHEMA_VERSION;
use super::types::{
    DatabaseStats, IntegrityReport, TransactionCursor, TransactionListFilters,
    TransactionListResult, TransactionListRow, WalletFlowExportRow,
};

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
            .map_err(|e| format!("Failed to vacuum database: {}", e))?;

        // Analyze for query optimization
        conn.execute("ANALYZE", [])
            .map_err(|e| format!("Failed to analyze database: {}", e))?;

        // Cleanup old pending transactions (older than 1 day)
        let cleaned_pending = conn
            .execute(
                "DELETE FROM pending_transactions WHERE added_at < datetime('now', '-1 day')",
                [],
            )
            .map_err(|e| format!("Failed to cleanup old pending transactions: {}", e))?;

        // Cleanup old deferred retries (older than 1 day with 0 attempts)
        let cleaned_retries = conn
            .execute(
                "DELETE FROM deferred_retries WHERE remaining_attempts = 0 AND created_at < datetime('now', '-1 day')",
                []
            )
            .map_err(|e| format!("Failed to cleanup old deferred retries: {}", e))?;

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

// =============================================================================
// IMPLEMENTATION - BOOTSTRAP STATE AND RECONCILIATION
// =============================================================================

/// Bootstrap state structure for resuming backfill across restarts
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BootstrapState {
    pub backfill_before_cursor: Option<String>,
    pub full_history_completed: bool,
}

impl TransactionDatabase {
    /// Get the current bootstrap state
    pub async fn get_bootstrap_state(&self) -> Result<BootstrapState, String> {
        let conn = self.get_connection()?;
        let mut state = BootstrapState::default();

        let result = conn
            .query_row(
                "SELECT backfill_before_cursor, full_history_completed FROM bootstrap_state WHERE id = 1",
                [],
                |row| {
                    let cursor: Option<String> = row.get(0)?;
                    let completed_i: i64 = row.get(1)?;
                    Ok((cursor, completed_i))
                }
            )
            .optional()
            .map_err(|e| format!("Failed to load bootstrap_state: {}", e))?;

        if let Some((cursor, completed_i)) = result {
            state.backfill_before_cursor = cursor;
            state.full_history_completed = completed_i != 0;
        }

        Ok(state)
    }

    /// Update the backfill cursor (the `before` parameter for next page)
    pub async fn set_backfill_cursor(&self, cursor: Option<&str>) -> Result<(), String> {
        let conn = self.get_connection()?;
        conn.execute(
            "INSERT OR IGNORE INTO bootstrap_state (id, full_history_completed) VALUES (1, 0)",
            [],
        )
        .map_err(|e| format!("Failed to ensure bootstrap_state row: {}", e))?;

        conn
            .execute(
                "UPDATE bootstrap_state SET backfill_before_cursor = ?1, updated_at = datetime('now') WHERE id = 1",
                params![cursor]
            )
            .map_err(|e| format!("Failed to update backfill cursor: {}", e))?;
        Ok(())
    }

    /// Clear the backfill cursor
    pub async fn clear_backfill_cursor(&self) -> Result<(), String> {
        self.set_backfill_cursor(None).await
    }

    /// Mark the full history as completed
    pub async fn mark_full_history_completed(&self) -> Result<(), String> {
        let conn = self.get_connection()?;
        conn
            .execute(
                "UPDATE bootstrap_state SET full_history_completed = 1, updated_at = datetime('now') WHERE id = 1",
                []
            )
            .map_err(|e| format!("Failed to mark full history completed: {}", e))?;
        Ok(())
    }

    /// Reconcile known_signatures with already processed transactions
    /// Ensures no processed transaction is missing from known_signatures
    pub async fn reconcile_known_with_processed(&self) -> Result<usize, String> {
        let conn = self.get_connection()?;
        let affected = conn
            .execute(
                "INSERT OR IGNORE INTO known_signatures(signature) SELECT signature FROM processed_transactions",
                []
            )
            .map_err(|e| format!("Failed to reconcile known signatures: {}", e))?;
        Ok(affected as usize)
    }

    // =============================================================================
    // LIST AND FILTER OPERATIONS FOR UI
    // =============================================================================

    /// List transactions with filtering and cursor-based pagination
    /// Returns lightweight rows suitable for UI list views
    pub async fn list_transactions(
        &self,
        filters: &TransactionListFilters,
        cursor: Option<&TransactionCursor>,
        limit: usize,
    ) -> Result<TransactionListResult, String> {
        let conn = self.get_connection()?;
        let wallet_address = crate::utils::get_wallet_address().map_err(|e| e.to_string())?;

        // Limit page size to max 200 for performance
        let effective_limit = limit.min(200);

        // Build SQL query with filters
        let mut query = String::from(
            "SELECT
                r.signature, r.timestamp, r.slot, r.status, r.success,
                r.fee_lamports, r.instructions_count,
                p.transaction_type, p.direction, p.token_swap_info,
                p.token_transfers, p.ata_operations,
                p.fee_sol, p.sol_delta
            FROM raw_transactions r
            LEFT JOIN processed_transactions p ON r.signature = p.signature AND p.wallet_address = ?1
            WHERE r.wallet_address = ?1",
        );

        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        params_vec.push(Box::new(wallet_address));

        // Apply cursor for pagination (timestamp desc, signature desc)
        if let Some(cursor) = cursor {
            query.push_str(&format!(
                " AND (r.timestamp < ?{} OR (r.timestamp = ?{} AND r.signature < ?{}))",
                params_vec.len() + 1,
                params_vec.len() + 1,
                params_vec.len() + 2
            ));
            params_vec.push(Box::new(cursor.timestamp.clone()));
            params_vec.push(Box::new(cursor.signature.clone()));
        }

        // Apply time range filters
        if let Some(ref from) = filters.time_from {
            query.push_str(&format!(" AND r.timestamp >= ?{}", params_vec.len() + 1));
            params_vec.push(Box::new(from.to_rfc3339()));
        }

        if let Some(ref to) = filters.time_to {
            query.push_str(&format!(" AND r.timestamp <= ?{}", params_vec.len() + 1));
            params_vec.push(Box::new(to.to_rfc3339()));
        }

        // Apply status filter
        if let Some(ref status) = filters.status {
            if let Some(normalized) = canonical_status(status) {
                query.push_str(&format!(" AND r.status = ?{}", params_vec.len() + 1));
                params_vec.push(Box::new(normalized));
            }
        }

        // Apply success filter
        if filters.only_confirmed.unwrap_or_default() {
            query.push_str(" AND r.status IN ('Confirmed', 'Finalized')");
        }

        // Apply signature filter
        if let Some(ref signature) = filters.signature {
            let trimmed = signature.trim();
            if !trimmed.is_empty() {
                query.push_str(&format!(" AND r.signature LIKE ?{}", params_vec.len() + 1));
                params_vec.push(Box::new(format!("%{}%", trimmed)));
            }
        }

        // Apply mint filter (JSON text search for efficiency)
        if let Some(ref mint) = filters.mint {
            let trimmed = mint.trim();
            if !trimmed.is_empty() {
                // Search in both swap info and transfers
                // We reuse the same param index since we push the same value twice?
                // No, rusqlite params are positional passed as slice.
                // Wait, params_vec is linear. I need to push it once?
                // query string: ... ?5 OR ... ?5 ...
                // Rusqlite supports ?NNN syntax.

                let param_idx = params_vec.len() + 1;
                query.push_str(&format!(
                    " AND (p.token_swap_info LIKE ?{} OR p.token_transfers LIKE ?{})",
                    param_idx, param_idx
                ));
                params_vec.push(Box::new(format!("%{}%", trimmed)));
            }
        }

        // Fetch 3x limit to allow Rust-side filtering
        let fetch_limit = effective_limit * 3;
        query.push_str(&format!(
            " ORDER BY r.timestamp DESC, r.signature DESC LIMIT {}",
            fetch_limit
        ));

        // Execute query
        let params_refs: Vec<&dyn rusqlite::ToSql> =
            params_vec.iter().map(|p| p.as_ref()).collect();
        let mut stmt = conn
            .prepare(&query)
            .map_err(|e| format!("Failed to prepare list query: {}", e))?;

        let rows = stmt
            .query_map(params_refs.as_slice(), |row| {
                let signature: String = row.get(0)?;
                let timestamp = {
                    let timestamp_str: String = row.get(1)?;
                    DateTime::parse_from_rfc3339(&timestamp_str)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now())
                };

                let slot = row.get::<_, Option<i64>>(2)?.and_then(|raw| {
                    if raw >= 0 {
                        Some(raw as u64)
                    } else {
                        None
                    }
                });
                let status: String = row.get(3)?;
                let success: bool = row.get(4)?;

                let fee_lamports = row.get::<_, Option<i64>>(5)?.and_then(|raw| {
                    if raw >= 0 {
                        Some(raw as u64)
                    } else {
                        None
                    }
                });
                let instructions_count = row.get::<_, Option<i64>>(6)?.unwrap_or_default().max(0) as usize;

                let transaction_type: Option<String> = row.get(7)?;
                let direction: Option<String> = row.get(8)?;
                let token_swap_info_json: Option<String> = row.get(9)?;
                let token_transfers_json: Option<String> = row.get(10)?;
                let ata_operations_json: Option<String> = row.get(11)?;
                let fee_sol = row.get::<_, Option<f64>>(12)?.unwrap_or_default();
                let sol_delta = row.get::<_, Option<f64>>(13)?.unwrap_or_default();

                let swap_info: Option<TokenSwapInfo> = token_swap_info_json
                    .as_ref()
                    .and_then(|json| serde_json::from_str(json).ok());
                let token_transfers: Option<Vec<TokenTransfer>> = token_transfers_json
                    .as_ref()
                    .and_then(|json| serde_json::from_str(json).ok());
                let ata_operations: Option<Vec<AtaOperation>> = ata_operations_json
                    .as_ref()
                    .and_then(|json| serde_json::from_str(json).ok());

                let ata_rents = ata_operations
                    .as_ref()
                    .map(|ops| ops.iter().map(|op| op.rent_amount).sum())
                    .unwrap_or_default();

                let mut token_mint = swap_info
                    .as_ref()
                    .map(|info| info.mint.clone())
                    .filter(|mint| !mint.is_empty());

                if token_mint.is_none() {
                    token_mint = swap_info
                        .as_ref()
                        .map(|info| info.output_mint.clone())
                        .filter(|mint| !mint.is_empty());
                }

                if token_mint.is_none() {
                    if let Some(transfers) = token_transfers.as_ref() {
                        token_mint = transfers.iter().find_map(|transfer| {
                            if transfer.mint.is_empty() {
                                None
                            } else {
                                Some(transfer.mint.clone())
                            }
                        });
                    }
                }

                let token_symbol = swap_info
                    .as_ref()
                    .map(|info| info.symbol.clone())
                    .filter(|symbol| !symbol.is_empty());

                let router = swap_info
                    .as_ref()
                    .map(|info| info.router.clone())
                    .filter(|router| !router.is_empty());

                let mut token_amount = swap_info.as_ref().map(|info| {
                    if info.swap_type == "sol_to_token" {
                        info.output_ui_amount
                    } else {
                        info.input_ui_amount
                    }
                });

                if token_amount.is_none() {
                    if let Some(transfers) = token_transfers.as_ref() {
                        token_amount = transfers.iter().find(|t| t.amount > 0.0).map(|t| t.amount);
                    }
                }

                Ok(TransactionListRow {
                    signature,
                    timestamp,
                    slot,
                    status,
                    success,
                    direction,
                    transaction_type,
                    token_mint,
                    token_symbol,
                    router,
                    sol_delta,
                    token_amount,
                    fee_sol,
                    fee_lamports,
                    ata_rents,
                    instructions_count,
                })
            })
            .map_err(|e| format!("Failed to execute list query: {}", e))?;

        // Collect and apply Rust-side filters
        let mut results: Vec<TransactionListRow> = Vec::new();

        for row_result in rows {
            let row = row_result.map_err(|e| format!("Failed to parse row: {}", e))?;

            if !Self::row_matches_filters(&row, filters) {
                continue;
            }

            results.push(row);

            // Stop when we have enough results
            if results.len() >= effective_limit {
                break;
            }
        }

        // Determine next cursor
        let next_cursor = if results.len() == effective_limit {
            results.last().map(|row| TransactionCursor {
                timestamp: row.timestamp.to_rfc3339(),
                signature: row.signature.clone(),
            })
        } else {
            None
        };

        Ok(TransactionListResult {
            items: results,
            next_cursor,
            total_estimate: None, // Optional, can be computed with COUNT query
        })
    }

    /// Helper to check if a row matches all filters
    fn row_matches_filters(row: &TransactionListRow, filters: &TransactionListFilters) -> bool {
        // Type filter
        if !filters.types.is_empty() {
            let row_type = row.transaction_type.as_deref().unwrap_or("Unknown");
            let matches_type = filters
                .types
                .iter()
                .any(|t| matches_transaction_type(t, row_type, row.success));
            if !matches_type {
                return false;
            }
        }

        // Mint filter
        if let Some(ref mint) = filters.mint {
            let mint_trimmed = mint.trim();
            if !mint_trimmed.is_empty() {
                if let Some(ref row_mint) = row.token_mint {
                    if !row_mint.contains(mint_trimmed) {
                        return false;
                    }
                } else {
                    return false;
                }
            }
        }

        // Direction filter
        if let Some(ref dir) = filters.direction {
            if let Some(expected) = canonical_direction(dir) {
                let row_dir = row.direction.as_deref().unwrap_or("Unknown");
                if !row_dir.eq_ignore_ascii_case(&expected) {
                    return false;
                }
            }
        }

        // Status filter (safety check, SQL already applies exact match)
        if let Some(ref status) = filters.status {
            if let Some(expected) = canonical_status(status) {
                if !row.status.eq_ignore_ascii_case(&expected) {
                    return false;
                }
            }
        }

        // Router filter (case-insensitive contains)
        if let Some(ref router) = filters.router {
            let router_trimmed = router.trim();
            if !router_trimmed.is_empty() {
                let needle = router_trimmed.to_ascii_lowercase();
                if let Some(ref row_router) = row.router {
                    if !row_router.to_ascii_lowercase().contains(&needle) {
                        return false;
                    }
                } else {
                    return false;
                }
            }
        }

        // SOL delta range filter
        if let Some(min_sol) = filters.min_sol {
            if row.sol_delta < min_sol {
                return false;
            }
        }

        if let Some(max_sol) = filters.max_sol {
            if row.sol_delta > max_sol {
                return false;
            }
        }

        true
    }

    /// Aggregate SOL inflow/outflow metrics within a time window for wallet dashboard usage
    pub async fn aggregate_sol_flows_since(
        &self,
        from: DateTime<Utc>,
        to: Option<DateTime<Utc>>,
    ) -> Result<(f64, f64, usize), String> {
        let conn = self.get_connection()?;
        let wallet_address = crate::utils::get_wallet_address().map_err(|e| e.to_string())?;

        // Check if this is "all time" query (from epoch = no time filter)
        let epoch = DateTime::<Utc>::from(std::time::UNIX_EPOCH);
        let is_all_time = from == epoch;

        let mut query = String::from(
            "SELECT \
                COALESCE(SUM(CASE WHEN COALESCE(p.sol_delta, 0) > 0 THEN p.sol_delta ELSE 0 END), 0), \
                COALESCE(SUM(CASE WHEN COALESCE(p.sol_delta, 0) < 0 THEN -p.sol_delta ELSE 0 END), 0), \
                COUNT(r.signature) \
             FROM raw_transactions r \
             LEFT JOIN processed_transactions p ON r.signature = p.signature AND p.wallet_address = ?1 \
             WHERE r.wallet_address = ?1 AND r.status IN ('Confirmed', 'Finalized')",
        );

        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = vec![];
        params_vec.push(Box::new(wallet_address.clone()));

        // Only add timestamp filter if NOT all-time query
        if !is_all_time {
            query.push_str(&format!(" AND r.timestamp >= ?{}", params_vec.len() + 1));
            params_vec.push(Box::new(from.to_rfc3339()));
        }

        if let Some(to_ts) = to {
            query.push_str(&format!(" AND r.timestamp <= ?{}", params_vec.len() + 1));
            params_vec.push(Box::new(to_ts.to_rfc3339()));
        }

        let params_refs: Vec<&dyn rusqlite::ToSql> =
            params_vec.iter().map(|value| value.as_ref()).collect();

        logger::debug(
            LogTag::Transactions,
            &format!(
                "Aggregating SOL flows for wallet {} from {}",
                wallet_address,
                from.to_rfc3339()
            ),
        );

        // Change query to get all rows so we can parse JSON
        let row_query = query.replace(
            "SELECT \
                COALESCE(SUM(CASE WHEN COALESCE(p.sol_delta, 0) > 0 THEN p.sol_delta ELSE 0 END), 0), \
                COALESCE(SUM(CASE WHEN COALESCE(p.sol_delta, 0) < 0 THEN -p.sol_delta ELSE 0 END), 0), \
                COUNT(r.signature)",
            "SELECT r.signature, r.timestamp, p.sol_balance_change",
        );

        let mut stmt = conn
            .prepare(&row_query)
            .map_err(|e| format!("Failed to prepare flow aggregation query: {}", e))?;

        let mut rows = stmt
            .query(params_refs.as_slice())
            .map_err(|e| format!("Failed to execute flow aggregation query: {}", e))?;

        let mut inflow = 0.0;
        let mut outflow = 0.0;
        let mut count = 0;
        let mut parsed_count = 0;
        let mut no_json_count = 0;
        let mut parse_error_count = 0;
        let mut no_wallet_account_count = 0;

        while let Some(row) = rows
            .next()
            .map_err(|e| format!("Failed to read flow row: {}", e))?
        {
            count += 1;
            let signature: String = row.get(0).unwrap_or_default();
            let sol_balance_change_json: Option<String> = row.get(2).ok();

            if let Some(json_str) = sol_balance_change_json {
                // Parse JSON array of balance changes
                match serde_json::from_str::<Vec<serde_json::Value>>(&json_str) {
                    Ok(changes) => {
                        let mut found_wallet = false;
                        let changes_len = changes.len();
                        for change_obj in &changes {
                            if let Some(account) =
                                change_obj.get("account").and_then(|v| v.as_str())
                            {
                                if account == wallet_address {
                                    found_wallet = true;
                                    if let Some(change) =
                                        change_obj.get("change").and_then(|v| v.as_f64())
                                    {
                                        parsed_count += 1;
                                        if count <= 5 {
                                            logger::debug(
                                                LogTag::Transactions,
                                                &format!(
                                                    "TX {}: wallet change={:.6} SOL",
                                                    &signature[..8],
                                                    change
                                                ),
                                            );
                                        }
                                        if change > 0.0 {
                                            inflow += change;
                                        } else if change < 0.0 {
                                            outflow += change.abs();
                                        }
                                    }
                                    break; // Found wallet, no need to check other accounts
                                }
                            }
                        }
                        if !found_wallet {
                            no_wallet_account_count += 1;
                            if no_wallet_account_count <= 3 {
                                logger::debug(
                                    LogTag::Transactions,
                                    &format!(
                                        "TX {}: no wallet account in {} balance changes",
                                        &signature[..8],
                                        changes_len
                                    ),
                                );
                            }
                        }
                    }
                    Err(e) => {
                        parse_error_count += 1;
                        if parse_error_count <= 3 {
                            logger::debug(
                                LogTag::Transactions,
                                &format!("TX {}: JSON parse error: {}", &signature[..8], e),
                            );
                        }
                    }
                }
            } else {
                no_json_count += 1;
            }
        }

        logger::debug(
        LogTag::Transactions,
                &format!(
                    "Aggregated {} txs: parsed={} with wallet, no_json={}, parse_errors={}, no_wallet_account={} | inflow={:.6} SOL, outflow={:.6} SOL, net={:.6} SOL",
                    count,
                    parsed_count,
                    no_json_count,
                    parse_error_count,
                    no_wallet_account_count,
                    inflow,
                    outflow,
                    inflow - outflow
                ),
            );

        Ok((inflow, outflow, count))
    }

    /// Get daily flow aggregation for time-series chart
    pub async fn aggregate_daily_flows(
        &self,
        from: DateTime<Utc>,
        to: Option<DateTime<Utc>>,
    ) -> Result<Vec<(String, f64, f64, usize)>, String> {
        let conn = self.get_connection()?;
        let wallet_address = crate::utils::get_wallet_address().map_err(|e| e.to_string())?;

        // Check if this is "all time" query
        let epoch = DateTime::<Utc>::from(std::time::UNIX_EPOCH);
        let is_all_time = from == epoch;

        // Query to get daily aggregated flows
        let mut query = String::from(
            "SELECT \
                DATE(r.timestamp) as day, \
                r.signature, \
                p.sol_balance_change \
             FROM raw_transactions r \
             LEFT JOIN processed_transactions p ON r.signature = p.signature AND p.wallet_address = ?1 \
             WHERE r.wallet_address = ?1 AND r.status IN ('Confirmed', 'Finalized')",
        );

        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = vec![];
        params_vec.push(Box::new(wallet_address.clone()));

        if !is_all_time {
            query.push_str(&format!(" AND r.timestamp >= ?{}", params_vec.len() + 1));
            params_vec.push(Box::new(from.to_rfc3339()));
        }

        if let Some(to_ts) = to {
            query.push_str(&format!(" AND r.timestamp <= ?{}", params_vec.len() + 1));
            params_vec.push(Box::new(to_ts.to_rfc3339()));
        }

        query.push_str(" ORDER BY day ASC, r.timestamp ASC");

        let params_refs: Vec<&dyn rusqlite::ToSql> =
            params_vec.iter().map(|value| value.as_ref()).collect();

        let mut stmt = conn
            .prepare(&query)
            .map_err(|e| format!("Failed to prepare daily flows query: {}", e))?;

        let mut rows = stmt
            .query(params_refs.as_slice())
            .map_err(|e| format!("Failed to execute daily flows query: {}", e))?;

        // Group by day manually
        use std::collections::HashMap;
        let mut daily_data: HashMap<String, (f64, f64, usize)> = HashMap::new();

        while let Some(row) = rows
            .next()
            .map_err(|e| format!("Failed to read daily flow row: {}", e))?
        {
            let day: String = row.get(0).unwrap_or_default();
            let sol_balance_change_json: Option<String> = row.get(2).ok();

            if let Some(json_str) = sol_balance_change_json {
                if let Ok(changes) = serde_json::from_str::<Vec<serde_json::Value>>(&json_str) {
                    for change_obj in &changes {
                        if let Some(account) = change_obj.get("account").and_then(|v| v.as_str()) {
                            if account == wallet_address {
                                if let Some(change) =
                                    change_obj.get("change").and_then(|v| v.as_f64())
                                {
                                    let entry =
                                        daily_data.entry(day.clone()).or_insert((0.0, 0.0, 0));
                                    if change > 0.0 {
                                        entry.0 += change; // inflow
                                    } else if change < 0.0 {
                                        entry.1 += change.abs(); // outflow
                                    }
                                    entry.2 += 1; // tx count
                                }
                                break;
                            }
                        }
                    }
                }
            }
        }

        // Convert to sorted vec
        let mut result: Vec<(String, f64, f64, usize)> = daily_data
            .into_iter()
            .map(|(day, (inflow, outflow, count))| (day, inflow, outflow, count))
            .collect();
        result.sort_by(|a, b| a.0.cmp(&b.0));

        Ok(result)
    }

    /// Lightweight export of processed transactions for wallet flow cache
    pub async fn export_processed_for_wallet_flow(
        &self,
        from: DateTime<Utc>,
        limit: usize,
    ) -> Result<Vec<WalletFlowExportRow>, String> {
        let conn = self.get_connection()?;
        let wallet_address = crate::utils::get_wallet_address().map_err(|e| e.to_string())?;

        let mut stmt = conn
            .prepare(
                "SELECT r.signature, r.timestamp, COALESCE(p.sol_delta, 0) as sol_delta \
                 FROM raw_transactions r \
                 LEFT JOIN processed_transactions p ON r.signature = p.signature AND p.wallet_address = ?1 \
                 WHERE r.wallet_address = ?1 AND r.timestamp >= ?2 AND r.status IN ('Confirmed', 'Finalized') \
                 ORDER BY r.timestamp ASC, r.signature ASC \
                 LIMIT ?3",
            )
            .map_err(|e| format!("Failed to prepare wallet flow export: {}", e))?;

        let mut rows = stmt
            .query(params![
                wallet_address,
                from.to_rfc3339(),
                (limit as i64).max(1)
            ])
            .map_err(|e| format!("Failed to query wallet flow export: {}", e))?;

        let mut results = Vec::new();
        while let Some(row) = rows
            .next()
            .map_err(|e| format!("Failed to iterate wallet flow export: {}", e))?
        {
            let signature: String = row
                .get(0)
                .map_err(|e| format!("Failed to read signature: {}", e))?;
            let ts_str: String = row
                .get(1)
                .map_err(|e| format!("Failed to read timestamp: {}", e))?;
            let timestamp = DateTime::parse_from_rfc3339(&ts_str)
                .map(|dt| dt.with_timezone(&Utc))
                .map_err(|e| format!("Failed to parse timestamp: {}", e))?;
            let sol_delta: f64 = row
                .get::<_, Option<f64>>(2)
                .unwrap_or(Some(0.0))
                .unwrap_or_default();
            results.push(WalletFlowExportRow {
                signature,
                timestamp,
                sol_delta,
            });
        }

        Ok(results)
    }
    /// Get estimated count of transactions matching filters (optional, for UI)
    pub async fn count_transactions(
        &self,
        filters: &TransactionListFilters,
    ) -> Result<u64, String> {
        let conn = self.get_connection()?;
        let wallet_address = crate::utils::get_wallet_address().map_err(|e| e.to_string())?;

        let mut query =
            String::from("SELECT COUNT(*) FROM raw_transactions r WHERE r.wallet_address = ?1");

        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        params_vec.push(Box::new(wallet_address));

        // Apply coarse filters (can't filter by JSON columns efficiently)
        if let Some(ref from) = filters.time_from {
            query.push_str(&format!(" AND r.timestamp >= ?{}", params_vec.len() + 1));
            params_vec.push(Box::new(from.to_rfc3339()));
        }

        if let Some(ref to) = filters.time_to {
            query.push_str(&format!(" AND r.timestamp <= ?{}", params_vec.len() + 1));
            params_vec.push(Box::new(to.to_rfc3339()));
        }

        if let Some(ref status) = filters.status {
            if let Some(normalized) = canonical_status(status) {
                query.push_str(&format!(" AND r.status = ?{}", params_vec.len() + 1));
                params_vec.push(Box::new(normalized));
            }
        }

        if filters.only_confirmed.unwrap_or_default() {
            query.push_str(" AND r.status IN ('Confirmed', 'Finalized')");
        }

        if let Some(ref signature) = filters.signature {
            let trimmed = signature.trim();
            if !trimmed.is_empty() {
                query.push_str(&format!(" AND r.signature LIKE ?{}", params_vec.len() + 1));
                params_vec.push(Box::new(format!("%{}%", trimmed)));
            }
        }

        let params_refs: Vec<&dyn rusqlite::ToSql> =
            params_vec.iter().map(|p| p.as_ref()).collect();

        let count: i64 = conn
            .query_row(&query, params_refs.as_slice(), |row| row.get(0))
            .map_err(|e| format!("Failed to count transactions: {}", e))?;

        Ok(count as u64)
    }
}

fn canonical_status(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }

    let lowered = trimmed.to_ascii_lowercase();
    let normalized = match lowered.as_str() {
        "pending" => "Pending",
        "confirmed" => "Confirmed",
        "finalized" => "Finalized",
        "failed" => "Failed",
        _ => return Some(trimmed.to_string()),
    };

    Some(normalized.to_string())
}

fn canonical_direction(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }

    let lowered = trimmed.to_ascii_lowercase();
    let normalized = match lowered.as_str() {
        "incoming" => "Incoming",
        "outgoing" => "Outgoing",
        "internal" => "Internal",
        "unknown" => "Unknown",
        _ => return Some(trimmed.to_string()),
    };

    Some(normalized.to_string())
}

fn matches_transaction_type(filter: &str, row_type: &str, success: bool) -> bool {
    let filter_norm = filter.trim().to_ascii_lowercase();
    if filter_norm.is_empty() {
        return false;
    }

    let row_lower = row_type.to_ascii_lowercase();

    match filter_norm.as_str() {
        "buy" => row_lower.contains("swapsoltotoken") || row_lower == "buy",
        "sell" => row_lower.contains("swaptokentosol") || row_lower == "sell",
        "swap" => row_lower.contains("swap") || row_lower == "buy" || row_lower == "sell",
        "transfer" => row_lower.contains("transfer"),
        "ata" => row_lower.contains("ata"),
        "failed" => !success || row_lower.contains("fail"),
        "unknown" => row_lower.contains("unknown"),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use rusqlite::Connection;
    use serde_json::json;
    use tempfile::tempdir;

    use crate::transactions::types::{
        SolBalanceChange, TransactionDirection, TransactionStatus, TransactionType,
    };

    fn sample_row(
        transaction_type: Option<&str>,
        direction: Option<&str>,
        success: bool,
        router: Option<&str>,
        sol_delta: f64,
    ) -> TransactionListRow {
        TransactionListRow {
            signature: "sig".to_string(),
            timestamp: Utc::now(),
            slot: None,
            status: "Finalized".to_string(),
            success,
            direction: direction.map(|s| s.to_string()),
            transaction_type: transaction_type.map(|s| s.to_string()),
            token_mint: None,
            token_symbol: None,
            router: router.map(|s| s.to_string()),
            sol_delta,
            fee_sol: 0.0,
            fee_lamports: None,
            ata_rents: 0.0,
            instructions_count: 0,
        }
    }

    #[tokio::test]
    async fn upsert_and_fetch_transaction_caches_raw_and_processed() {
        let dir = tempdir().expect("create temp dir");
        let db_path = dir.path().join("transactions.db");
        let db = TransactionDatabase::new_with_path(&db_path)
            .await
            .expect("create database");

        let mut transaction = Transaction::new("test_signature".to_string());
        transaction.slot = Some(12345);
        transaction.block_time = Some(1_700_000_000);
        transaction.timestamp = Utc::now();
        transaction.status = TransactionStatus::Finalized;
        transaction.success = true;
        transaction.fee_lamports = Some(5_000);
        transaction.fee_sol = 0.000005;
        transaction.instructions_count = 2;
        transaction.accounts_count = 3;
        transaction.transaction_type = TransactionType::Transfer;
        transaction.direction = TransactionDirection::Outgoing;
        transaction.sol_balance_change = -0.25;
        transaction.sol_balance_changes = vec![SolBalanceChange {
            account: "wallet".to_string(),
            pre_balance: 1.0,
            post_balance: 0.75,
            change: -0.25,
        }];
        let raw_json = json!({ "signature": transaction.signature });
        let raw_json_string = raw_json.to_string();
        transaction.raw_transaction_data = Some(raw_json);

        db.upsert_full_transaction(&transaction)
            .await
            .expect("upsert transaction");

        let fetched = db
            .get_transaction(&transaction.signature)
            .await
            .expect("fetch transaction")
            .expect("transaction exists");

        assert_eq!(fetched.signature, transaction.signature);
        assert!(fetched.success);
        assert_eq!(fetched.fee_lamports, transaction.fee_lamports);
        assert_eq!(fetched.instructions_count, transaction.instructions_count);

        let conn = Connection::open(&db_path).expect("open sqlite connection");
        let stored_raw: Option<String> = conn
            .query_row(
                "SELECT raw_transaction_data FROM raw_transactions WHERE signature = ?1",
                [transaction.signature.as_str()],
                |row| row.get(0),
            )
            .expect("query raw data");
        assert_eq!(stored_raw, Some(raw_json_string));

        let stored_fee: f64 = conn
            .query_row(
                "SELECT fee_sol FROM processed_transactions WHERE signature = ?1",
                [transaction.signature.as_str()],
                |row| row.get(0),
            )
            .expect("query processed fee");
        assert!((stored_fee - transaction.fee_sol).abs() < 1e-12);

        let stored_delta: f64 = conn
            .query_row(
                "SELECT sol_delta FROM processed_transactions WHERE signature = ?1",
                [transaction.signature.as_str()],
                |row| Ok(row.get::<_, Option<f64>>(0)?.unwrap_or_default()),
            )
            .expect("query processed sol_delta");
        assert!((stored_delta - transaction.sol_balance_change).abs() < 1e-9);
    }

    #[test]
    fn type_filters_match_modern_and_legacy_variants() {
        let row_swap = sample_row(
            Some("SwapSolToToken { .. }"),
            Some("Outgoing"),
            true,
            None,
            0.0,
        );
        let row_buy = sample_row(Some("Buy"), Some("Outgoing"), true, None, 0.0);

        let filters = TransactionListFilters {
            types: vec!["buy".to_string()],
            ..Default::default()
        };

        assert!(TransactionDatabase::row_matches_filters(
            &row_swap, &filters
        ));
        assert!(TransactionDatabase::row_matches_filters(&row_buy, &filters));

        let failed_filters = TransactionListFilters {
            types: vec!["failed".to_string()],
            ..Default::default()
        };

        let failed_row = sample_row(
            Some("SwapTokenToSol { .. }"),
            Some("Outgoing"),
            false,
            None,
            0.0,
        );
        assert!(TransactionDatabase::row_matches_filters(
            &failed_row,
            &failed_filters
        ));
    }

    #[test]
    fn direction_filter_is_case_insensitive() {
        let row = sample_row(Some("Transfer"), Some("Incoming"), true, None, 0.0);

        let filters = TransactionListFilters {
            direction: Some("incoming".to_string()),
            ..Default::default()
        };

        assert!(TransactionDatabase::row_matches_filters(&row, &filters));
    }

    #[test]
    fn router_filter_handles_case_insensitive_search() {
        let row = sample_row(Some("Swap"), Some("Outgoing"), true, Some("Raydium"), 0.0);

        let filters = TransactionListFilters {
            router: Some("ray".to_string()),
            ..Default::default()
        };

        assert!(TransactionDatabase::row_matches_filters(&row, &filters));
    }
}
