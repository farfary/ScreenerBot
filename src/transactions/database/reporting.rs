//! Transaction database reporting — list, filter, and aggregate queries for UI.
//
// Reporting, querying, and export operations

use crate::transactions::types::*;
use chrono::{DateTime, Utc};

use super::operations::TransactionDatabase;
use super::types::{
    TransactionCursor, TransactionListFilters, TransactionListResult, TransactionListRow,
};

impl TransactionDatabase {
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
        let subject = Subject::own().map_err(|e| e.to_string())?;
        self.list_transactions_for_subject(subject, filters, cursor, limit)
            .await
    }

    /// List transactions for an explicitly validated subject. Web/API callers must
    /// resolve authorization before reaching this database boundary.
    pub async fn list_transactions_for_subject(
        &self,
        subject: Subject,
        filters: &TransactionListFilters,
        cursor: Option<&TransactionCursor>,
        limit: usize,
    ) -> Result<TransactionListResult, String> {
        let conn = self.get_connection()?;
        let wallet_address = subject.address();
        let chain_id = self.require_subject_chain(&subject)?;

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
            LEFT JOIN processed_transactions p ON r.chain_id = p.chain_id AND r.signature = p.signature AND p.wallet_address = ?2
            WHERE r.chain_id = ?1 AND r.wallet_address = ?2",
        );

        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        params_vec.push(Box::new(chain_id));
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
                params_vec.push(Box::new(format!("%{trimmed}%")));
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
                params_vec.push(Box::new(format!("%{trimmed}%")));
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
            .map_err(|e| format!("Failed to prepare list query: {e}"))?;

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
                let instructions_count =
                    row.get::<_, Option<i64>>(6)?.unwrap_or_default().max(0) as usize;

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
            .map_err(|e| format!("Failed to execute list query: {e}"))?;

        // Collect and apply Rust-side filters
        let mut results: Vec<TransactionListRow> = Vec::new();

        for row_result in rows {
            let row = row_result.map_err(|e| format!("Failed to parse row: {e}"))?;

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

    /// Get estimated count of transactions matching filters (optional, for UI)
    pub async fn count_transactions(
        &self,
        filters: &TransactionListFilters,
    ) -> Result<u64, String> {
        let subject = Subject::own().map_err(|e| e.to_string())?;
        self.count_transactions_for_subject(subject, filters).await
    }

    pub async fn count_transactions_for_subject(
        &self,
        subject: Subject,
        filters: &TransactionListFilters,
    ) -> Result<u64, String> {
        let conn = self.get_connection()?;
        let wallet_address = subject.address();
        let chain_id = self.require_subject_chain(&subject)?;

        let mut query =
            "SELECT COUNT(*) FROM raw_transactions r WHERE r.chain_id = ?1 AND r.wallet_address = ?2".to_owned();

        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        params_vec.push(Box::new(chain_id));
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
                params_vec.push(Box::new(format!("%{trimmed}%")));
            }
        }

        let params_refs: Vec<&dyn rusqlite::ToSql> =
            params_vec.iter().map(|p| p.as_ref()).collect();

        let count: i64 = conn
            .query_row(&query, params_refs.as_slice(), |row| row.get(0))
            .map_err(|e| format!("Failed to count transactions: {e}"))?;

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
            signature: "sig".to_owned(),
            timestamp: Utc::now(),
            slot: None,
            status: "Finalized".to_owned(),
            success,
            direction: direction.map(|s| s.to_string()),
            transaction_type: transaction_type.map(|s| s.to_string()),
            token_mint: None,
            token_symbol: None,
            router: router.map(|s| s.to_string()),
            sol_delta,
            token_amount: None,
            fee_sol: 0.0,
            fee_lamports: None,
            ata_rents: 0.0,
            instructions_count: 0,
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn upsert_and_fetch_transaction_caches_raw_and_processed() {
        let dir = tempdir().expect("create temp dir");
        let db_path = dir.path().join("transactions.db");
        let db = TransactionDatabase::new_with_path(&db_path, crate::chains::ChainId::Solana)
            .await
            .expect("create database");

        let mut transaction = Transaction::new("test_signature".to_owned());
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
            account: "wallet".to_owned(),
            pre_balance: 1.0,
            post_balance: 0.75,
            change: -0.25,
        }];
        let raw_json = json!({ "signature": transaction.signature });
        let raw_json_string = raw_json.to_string();
        transaction.raw_transaction_data = Some(raw_json);

        let subject = crate::chains::solana::transactions::subject::from_pubkey(
            crate::chains::solana::solana_sdk::pubkey::Pubkey::new_unique(),
        );
        db.upsert_full_transaction(subject.clone(), &transaction)
            .await
            .expect("upsert transaction");

        let fetched = db
            .get_transaction_for_subject(subject, &transaction.signature)
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
            types: vec!["buy".to_owned()],
            ..Default::default()
        };

        assert!(TransactionDatabase::row_matches_filters(
            &row_swap, &filters
        ));
        assert!(TransactionDatabase::row_matches_filters(&row_buy, &filters));

        let failed_filters = TransactionListFilters {
            types: vec!["failed".to_owned()],
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
            direction: Some("incoming".to_owned()),
            ..Default::default()
        };

        assert!(TransactionDatabase::row_matches_filters(&row, &filters));
    }

    #[test]
    fn router_filter_handles_case_insensitive_search() {
        let row = sample_row(Some("Swap"), Some("Outgoing"), true, Some("Raydium"), 0.0);

        let filters = TransactionListFilters {
            router: Some("ray".to_owned()),
            ..Default::default()
        };

        assert!(TransactionDatabase::row_matches_filters(&row, &filters));
    }
}
