//! Transaction database reporting — list, filter, and aggregate queries for UI.
//
// Reporting, querying, and export operations

use crate::transactions::error::Error;
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
    ) -> Result<TransactionListResult, Error> {
        let subject = Subject::own().map_err(|e| Error::WalletUnavailable {
            detail: e.to_string(),
        })?;
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
    ) -> Result<TransactionListResult, Error> {
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
                p.type_kind, p.direction, p.token_swap_info,
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
            .map_err(crate::errors::DatabaseError::from)?;

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

                // `type_kind` -- the stable discriminant -- not the serialized
                // payload, which no consumer of a list row can read.
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
            .map_err(crate::errors::DatabaseError::from)?;

        // Collect and apply Rust-side filters
        let mut results: Vec<TransactionListRow> = Vec::new();

        for row_result in rows {
            let row = row_result.map_err(crate::errors::DatabaseError::from)?;

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
    pub async fn count_transactions(&self, filters: &TransactionListFilters) -> Result<u64, Error> {
        let subject = Subject::own().map_err(|e| Error::WalletUnavailable {
            detail: e.to_string(),
        })?;
        self.count_transactions_for_subject(subject, filters).await
    }

    pub async fn count_transactions_for_subject(
        &self,
        subject: Subject,
        filters: &TransactionListFilters,
    ) -> Result<u64, Error> {
        let conn = self.get_connection()?;
        let wallet_address = subject.address();
        let chain_id = self.require_subject_chain(&subject)?;

        // Joined to the processed row so the count answers the same question the
        // list does. Counting `raw_transactions` alone reported the whole wallet
        // total no matter which type or direction was selected.
        let mut query = "SELECT COUNT(*) FROM raw_transactions r \
             LEFT JOIN processed_transactions p \
               ON r.chain_id = p.chain_id AND r.signature = p.signature AND p.wallet_address = ?2 \
             WHERE r.chain_id = ?1 AND r.wallet_address = ?2"
            .to_owned();

        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        params_vec.push(Box::new(chain_id));
        params_vec.push(Box::new(wallet_address));

        // Type filter: `failed` is a status, every other filter is a set of kinds.
        if !filters.types.is_empty() {
            let mut clauses: Vec<String> = Vec::new();
            for filter in &filters.types {
                if filter.trim().eq_ignore_ascii_case("failed") {
                    clauses.push("r.success = 0".to_owned());
                    continue;
                }
                if let Some(kinds) = kinds_for_filter(filter) {
                    let mut placeholders = Vec::new();
                    for kind in kinds {
                        placeholders.push(format!("?{}", params_vec.len() + 1));
                        params_vec.push(Box::new((*kind).to_owned()));
                    }
                    clauses.push(format!("p.type_kind IN ({})", placeholders.join(", ")));
                }
            }
            if !clauses.is_empty() {
                query.push_str(&format!(" AND ({})", clauses.join(" OR ")));
            }
        }

        if let Some(ref direction) = filters.direction {
            if let Some(expected) = canonical_direction(direction) {
                query.push_str(&format!(" AND p.direction = ?{}", params_vec.len() + 1));
                params_vec.push(Box::new(expected));
            }
        }

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
            .map_err(crate::errors::DatabaseError::from)?;

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

/// The `TransactionType::kind()` values one UI filter selects.
///
/// Most filters are a single kind; a few deliberately span kinds (`swap` covers
/// both trade directions, `transfer` covers SOL and token movements, `ata` covers
/// both ends of an account's lifecycle). `failed` returns `None` because it is a
/// property of the transaction's status, not of its type.
fn kinds_for_filter(filter: &str) -> Option<&'static [&'static str]> {
    match filter.trim().to_ascii_lowercase().as_str() {
        "buy" => Some(&["buy"]),
        "sell" => Some(&["sell"]),
        "swap" => Some(&["buy", "sell", "swap"]),
        "transfer" => Some(&["transfer", "sol_transfer", "token_transfer"]),
        "ata" => Some(&["ata", "ata_create", "ata_close"]),
        "dust" => Some(&["dust"]),
        "spam" => Some(&["spam"]),
        "liquidity" => Some(&["liquidity_add", "liquidity_remove"]),
        "nft" => Some(&["nft"]),
        "program" => Some(&["program", "compute"]),
        "unknown" => Some(&["unknown"]),
        _ => None,
    }
}

/// Matches a UI filter against a row's `TransactionType::kind()`.
fn matches_transaction_type(filter: &str, row_type: &str, success: bool) -> bool {
    let filter_norm = filter.trim().to_ascii_lowercase();
    if filter_norm.is_empty() {
        return false;
    }

    let kind = row_type.to_ascii_lowercase();

    if filter_norm == "failed" {
        return !success || kind == "failed";
    }

    match kinds_for_filter(&filter_norm) {
        Some(kinds) => {
            if kinds.contains(&kind.as_str()) {
                return true;
            }
            // Rows written before the reclassification sweep still hold a `Debug`
            // rendering; keep the trade filters working on those until it runs.
            match filter_norm.as_str() {
                "buy" => kind.contains("swapsoltotoken"),
                "sell" => kind.contains("swaptokentosol"),
                "swap" => kind.contains("swap"),
                "ata" => kind.contains("ata"),
                _ => false,
            }
        }
        None => kind == filter_norm,
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

    #[tokio::test]
    async fn a_rich_transaction_type_survives_a_storage_round_trip() {
        // Rich variants were persisted with `Debug`, which is not valid JSON, so
        // every payload-carrying type read back as `Unknown` and its kind was lost.
        let dir = tempdir().expect("create temp dir");
        let db_path = dir.path().join("transactions.db");
        let db = TransactionDatabase::new_with_path(&db_path, crate::chains::ChainId::Solana)
            .await
            .expect("create database");

        let mut transaction = Transaction::new("rich_signature".to_owned());
        transaction.timestamp = Utc::now();
        transaction.status = TransactionStatus::Finalized;
        transaction.success = true;
        transaction.transaction_type = TransactionType::AtaClose {
            recovered_sol: 0.00203928,
            token_mint: "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263".to_owned(),
        };
        transaction.direction = TransactionDirection::Incoming;
        transaction.sol_balance_change = 0.00203428;
        transaction.raw_transaction_data = Some(json!({ "signature": transaction.signature }));

        let subject = Subject::from_account(
            crate::chains::AccountId::new(crate::chains::active_chain(), "ReportingTestWallet111")
                .unwrap(),
        );
        db.upsert_full_transaction(subject.clone(), &transaction)
            .await
            .expect("upsert transaction");

        let fetched = db
            .get_transaction_for_subject(subject.clone(), &transaction.signature)
            .await
            .expect("fetch transaction")
            .expect("transaction exists");
        match fetched.transaction_type {
            TransactionType::AtaClose { token_mint, .. } => {
                assert_eq!(token_mint, "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263");
            }
            other => panic!("expected AtaClose, got {other:?}"),
        }

        // The list path reads the stable discriminant column, and the delta column
        // holds the wallet's own change rather than the transaction's fee.
        let conn = Connection::open(&db_path).expect("open sqlite connection");
        let (kind, direction, sol_delta): (String, String, f64) = conn
            .query_row(
                "SELECT type_kind, direction, sol_delta FROM processed_transactions WHERE signature = ?1",
                [transaction.signature.as_str()],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("query processed row");
        assert_eq!(kind, "ata_close");
        assert_eq!(direction, "Incoming");
        assert!((sol_delta - 0.00203428).abs() < 1e-12);
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

        let subject = Subject::from_account(
            crate::chains::AccountId::new(crate::chains::active_chain(), "ReportingTestWallet111")
                .unwrap(),
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
