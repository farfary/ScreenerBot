//! Transaction database queries — pending transactions and transaction data CRUD.
//
// Pending transaction management and transaction storage/retrieval operations.
// Split from operations.rs for maintainability; extends TransactionDatabase via
// a second `impl` block.

use chrono::{DateTime, Utc};
use rusqlite::{params, OptionalExtension};
use std::collections::HashMap;

use crate::transactions::types::*;

use super::operations::TransactionDatabase;
use super::schema::*;

// =============================================================================
// IMPLEMENTATION - PENDING TRANSACTIONS MANAGEMENT
// =============================================================================

impl TransactionDatabase {
    /// Save pending transactions to database, for the given subject
    pub async fn save_pending_transactions(
        &self,
        subject: Subject,
        pending: &HashMap<String, DateTime<Utc>>,
    ) -> Result<(), String> {
        let conn = self.get_connection()?;
        let wallet_address = subject.address();

        let tx = conn
            .unchecked_transaction()
            .map_err(|e| format!("Failed to start transaction: {e}"))?;

        for (signature, timestamp) in pending {
            tx.execute(
                "INSERT OR REPLACE INTO pending_transactions (signature, wallet_address, added_at) VALUES (?1, ?2, ?3)",
                params![signature, wallet_address, timestamp.to_rfc3339()],
            )
            .map_err(|e| format!("Failed to save pending transaction: {e}"))?;
        }

        tx.commit()
            .map_err(|e| format!("Failed to commit pending transactions: {e}"))?;

        Ok(())
    }

    /// Load pending transactions from database, for the given subject
    pub async fn get_pending_transactions(
        &self,
        subject: Subject,
    ) -> Result<HashMap<String, DateTime<Utc>>, String> {
        let conn = self.get_connection()?;
        let wallet_address = subject.address();

        let mut stmt = conn
            .prepare(
                "SELECT signature, added_at FROM pending_transactions WHERE wallet_address = ?1",
            )
            .map_err(|e| format!("Failed to prepare pending transactions query: {e}"))?;

        let rows = stmt
            .query_map(params![wallet_address], |row| {
                let signature: String = row.get(0)?;
                let timestamp_str: String = row.get(1)?;
                let timestamp = DateTime::parse_from_rfc3339(&timestamp_str)
                    .map_err(|e| {
                        rusqlite::Error::InvalidColumnType(
                            0,
                            "timestamp".to_owned(),
                            rusqlite::types::Type::Text,
                        )
                    })?
                    .with_timezone(&Utc);
                Ok((signature, timestamp))
            })
            .map_err(|e| format!("Failed to query pending transactions: {e}"))?;

        let mut result = HashMap::new();
        for row in rows {
            let (signature, timestamp) =
                row.map_err(|e| format!("Failed to parse pending transaction row: {e}"))?;
            result.insert(signature, timestamp);
        }

        Ok(result)
    }

    /// Remove pending transaction, for the given subject
    pub async fn remove_pending_transaction(
        &self,
        subject: Subject,
        signature: &str,
    ) -> Result<bool, String> {
        let conn = self.get_connection()?;
        let wallet_address = subject.address();

        let affected = conn
            .execute(
                "DELETE FROM pending_transactions WHERE signature = ?1 AND wallet_address = ?2",
                params![signature, wallet_address],
            )
            .map_err(|e| format!("Failed to remove pending transaction: {e}"))?;

        Ok(affected > 0)
    }

    /// Get count of pending transactions, for the given subject
    pub async fn get_pending_transactions_count(&self, subject: Subject) -> Result<u64, String> {
        let conn = self.get_connection()?;
        let wallet_address = subject.address();

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM pending_transactions WHERE wallet_address = ?1",
                params![wallet_address],
                |row| row.get(0),
            )
            .map_err(|e| format!("Failed to get pending transactions count: {e}"))?;

        Ok(count as u64)
    }
}

// =============================================================================
// IMPLEMENTATION - TRANSACTION DATA MANAGEMENT
// =============================================================================

impl TransactionDatabase {
    /// Load raw transaction JSON blob and deserialize into TransactionDetails (cache-first path)
    pub async fn get_raw_transaction_details(
        &self,
        signature: &str,
    ) -> Result<Option<crate::rpc::TransactionDetails>, String> {
        let conn = self.get_connection()?;
        let wallet_address = crate::utils::get_wallet_address().map_err(|e| e.to_string())?;

        let result: rusqlite::Result<Option<String>> = conn
            .query_row(
                "SELECT raw_transaction_data FROM raw_transactions WHERE signature = ?1 AND wallet_address = ?2",
                params![signature, wallet_address],
                |row| row.get(0),
            )
            .optional();

        match result {
            Ok(Some(json_str)) => {
                if json_str.trim().is_empty() {
                    return Ok(None);
                }
                match serde_json::from_str::<crate::rpc::TransactionDetails>(&json_str) {
                    Ok(details) => Ok(Some(details)),
                    Err(e) => Err(format!(
                        "Failed to deserialize cached raw transaction for {}: {}",
                        signature, e
                    )),
                }
            }
            Ok(None) => Ok(None),
            Err(e) => Err(format!("Failed to read cached raw transaction: {e}")),
        }
    }

    /// Store raw transaction data, for the given subject
    pub async fn store_raw_transaction(
        &self,
        subject: Subject,
        transaction: &Transaction,
    ) -> Result<(), String> {
        let conn = self.get_connection()?;
        let wallet_address = subject.address();

        let status_str = match &transaction.status {
            TransactionStatus::Pending => "Pending",
            TransactionStatus::Confirmed => "Confirmed",
            TransactionStatus::Finalized => "Finalized",
            TransactionStatus::Failed(msg) => "Failed",
        };

        let raw_transaction_json = transaction
            .raw_transaction_data
            .as_ref()
            .and_then(|value| serde_json::to_string(value).ok());

        conn
            .execute(
                r#"INSERT OR REPLACE INTO raw_transactions
               (signature, wallet_address, slot, block_time, timestamp, status, success, error_message,
                fee_lamports, compute_units_consumed, instructions_count, accounts_count, raw_transaction_data, updated_at)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, datetime('now'))"#,
                params![
                    transaction.signature,
                    wallet_address,
                    transaction.slot,
                    transaction.block_time,
                    transaction.timestamp.to_rfc3339(),
                    status_str,
                    transaction.success,
                    transaction.error_message,
                    transaction.fee_lamports,
                    transaction.compute_units_consumed,
                    transaction.instructions_count,
                    transaction.accounts_count,
                    raw_transaction_json
                ]
            )
            .map_err(|e| format!("Failed to store raw transaction: {e}"))?;

        Ok(())
    }

    /// Store processed transaction analysis snapshot, for the given subject
    pub async fn store_processed_transaction(
        &self,
        subject: Subject,
        transaction: &Transaction,
    ) -> Result<(), String> {
        let conn = self.get_connection()?;
        let wallet_address = subject.address();

        // Serialize complex fields as JSON strings
        let sol_balance_change_json = serde_json::to_string(&transaction.sol_balance_changes)
            .unwrap_or_else(|_| "[]".to_owned());
        let token_balance_changes_json = serde_json::to_string(&transaction.token_balance_changes)
            .unwrap_or_else(|_| "[]".to_owned());
        let token_swap_info_json = serde_json::to_string(&transaction.token_swap_info)
            .unwrap_or_else(|_| "null".to_owned());
        let swap_pnl_info_json =
            serde_json::to_string(&transaction.swap_pnl_info).unwrap_or_else(|_| "null".to_owned());
        let ata_operations_json =
            serde_json::to_string(&transaction.ata_operations).unwrap_or_else(|_| "[]".to_owned());
        let token_transfers_json =
            serde_json::to_string(&transaction.token_transfers).unwrap_or_else(|_| "[]".to_owned());
        let instruction_info_json =
            serde_json::to_string(&transaction.instructions).unwrap_or_else(|_| "[]".to_owned());
        let cached_analysis_json = serde_json::to_string(&transaction.cached_analysis)
            .unwrap_or_else(|_| "null".to_owned());

        let tx_type = format!("{:?}", transaction.transaction_type);
        let dir = format!("{:?}", transaction.direction);

        let sol_delta = if !transaction.sol_balance_changes.is_empty() {
            transaction
                .sol_balance_changes
                .iter()
                .map(|change| change.change)
                .sum()
        } else {
            transaction.sol_balance_change
        };

        conn
            .execute(
                r#"INSERT OR REPLACE INTO processed_transactions
                   (signature, wallet_address, transaction_type, direction, sol_balance_change, token_balance_changes,
                    token_swap_info, swap_pnl_info, ata_operations, token_transfers, instruction_info,
                    analysis_duration_ms, cached_analysis, analysis_version, fee_sol, sol_delta, updated_at)
                 VALUES
                   (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, datetime('now'))"#,
                params![
                    transaction.signature,
                    wallet_address,
                    tx_type,
                    dir,
                    sol_balance_change_json,
                    token_balance_changes_json,
                    token_swap_info_json,
                    swap_pnl_info_json,
                    ata_operations_json,
                    token_transfers_json,
                    instruction_info_json,
                    transaction.analysis_duration_ms,
                    cached_analysis_json,
                    ANALYSIS_CACHE_VERSION as i64,
                    transaction.fee_sol,
                    sol_delta
                ]
            )
            .map_err(|e| format!("Failed to store processed transaction: {e}"))?;

        Ok(())
    }

    /// Convenience: upsert both raw and processed snapshots, for the bot's own wallet.
    /// No subject is threaded through here because this convenience method has no
    /// caller that is watching another wallet today; it resolves the own-wallet
    /// subject directly rather than forcing every caller to plumb one through.
    pub async fn upsert_full_transaction(
        &self,
        subject: Subject,
        transaction: &Transaction,
    ) -> Result<(), String> {
        self.store_raw_transaction(subject, transaction).await?;
        self.store_processed_transaction(subject, transaction)
            .await?;
        Ok(())
    }

    /// Update transaction status, for the given subject
    pub async fn update_transaction_status(
        &self,
        subject: Subject,
        signature: &str,
        status: &str,
        success: bool,
        error_message: Option<&str>,
    ) -> Result<(), String> {
        let conn = self.get_connection()?;
        let wallet_address = subject.address();

        conn
            .execute(
                "UPDATE raw_transactions SET status = ?1, success = ?2, error_message = ?3, updated_at = datetime('now') WHERE signature = ?4 AND wallet_address = ?5",
                params![status, success, error_message, signature, wallet_address]
            )
            .map_err(|e| format!("Failed to update transaction status: {e}"))?;

        Ok(())
    }

    /// Get transaction by signature with full analysis data
    pub async fn get_transaction(&self, signature: &str) -> Result<Option<Transaction>, String> {
        let conn = self.get_connection()?;
        let wallet_address = crate::utils::get_wallet_address().map_err(|e| e.to_string())?;

        // Join raw_transactions with processed_transactions to get full data
        let result = conn.query_row(
            r#"SELECT
                r.signature, r.slot, r.block_time, r.timestamp, r.status, r.success, r.error_message,
                r.fee_lamports, r.compute_units_consumed, r.instructions_count, r.accounts_count,
                r.raw_transaction_data,
                p.transaction_type, p.direction, p.sol_balance_change, p.token_balance_changes,
                p.token_swap_info, p.swap_pnl_info, p.ata_operations, p.token_transfers,
                p.instruction_info, p.analysis_duration_ms, p.cached_analysis, p.fee_sol, p.sol_delta
            FROM raw_transactions r
            LEFT JOIN processed_transactions p ON r.signature = p.signature AND p.wallet_address = ?2
            WHERE r.signature = ?1 AND r.wallet_address = ?2"#,
            params![signature, wallet_address],
            |row| {
                let timestamp_str: String = row.get(3)?;
                let timestamp = DateTime::parse_from_rfc3339(&timestamp_str)
                    .map_err(|_| {
                        rusqlite::Error::InvalidColumnType(
                            3,
                            "timestamp".to_owned(),
                            rusqlite::types::Type::Text,
                        )
                    })?
                    .with_timezone(&Utc);

                let status_str: String = row.get(4)?;
                let status = match status_str.as_str() {
                    "Pending" => TransactionStatus::Pending,
                    "Confirmed" => TransactionStatus::Confirmed,
                    "Finalized" => TransactionStatus::Finalized,
                    s if s.starts_with("Failed") => TransactionStatus::Failed(s.to_string()),
                    _ => TransactionStatus::Pending,
                };

                // Parse raw_transaction_data JSON if present
                let raw_transaction_data: Option<serde_json::Value> = row
                    .get::<_, Option<String>>(11)?
                    .and_then(|json| serde_json::from_str(&json).ok());

                // Parse processed fields from joined data
                let transaction_type_str: Option<String> = row.get(12)?;
                let transaction_type = transaction_type_str
                    .as_ref()
                    .and_then(|s| {
                        // First try parsing as JSON object (for rich variants like SwapSolToToken)
                        serde_json::from_str(s)
                            .ok()
                            // Then try as quoted string (for simple variants like "Sell")
                            .or_else(|| serde_json::from_str(&format!("\"{}\"", s)).ok())
                    })
                    .unwrap_or(TransactionType::Unknown);

                let direction_str: Option<String> = row.get(13)?;
                let direction = match direction_str.as_deref() {
                    Some("Incoming") => TransactionDirection::Incoming,
                    Some("Outgoing") => TransactionDirection::Outgoing,
                    Some("Internal") => TransactionDirection::Internal,
                    _ => TransactionDirection::Unknown,
                };

                let sol_balance_change_json: Option<String> = row.get(14)?;
                let sol_balance_changes: Vec<SolBalanceChange> = sol_balance_change_json
                    .as_ref()
                    .and_then(|json| serde_json::from_str(json).ok())
                    .unwrap_or_default();

                // Use sol_delta from the dedicated column (index 24) for the aggregate change
                let sol_delta: f64 = row.get::<_, Option<f64>>(24)?.unwrap_or_default();

                let token_balance_changes_json: Option<String> = row.get(15)?;
                let token_balance_changes: Vec<TokenBalanceChange> = token_balance_changes_json
                    .as_ref()
                    .and_then(|json| serde_json::from_str(json).ok())
                    .unwrap_or_default();

                let token_swap_info_json: Option<String> = row.get(16)?;
                let token_swap_info: Option<TokenSwapInfo> = token_swap_info_json
                    .as_ref()
                    .and_then(|json| serde_json::from_str(json).ok());

                let swap_pnl_info_json: Option<String> = row.get(17)?;
                let swap_pnl_info: Option<SwapPnLInfo> = swap_pnl_info_json
                    .as_ref()
                    .and_then(|json| serde_json::from_str(json).ok());

                let ata_operations_json: Option<String> = row.get(18)?;
                let ata_operations: Vec<AtaOperation> = ata_operations_json
                    .as_ref()
                    .and_then(|json| serde_json::from_str(json).ok())
                    .unwrap_or_default();

                let token_transfers_json: Option<String> = row.get(19)?;
                let token_transfers: Vec<TokenTransfer> = token_transfers_json
                    .as_ref()
                    .and_then(|json| serde_json::from_str(json).ok())
                    .unwrap_or_default();

                let instruction_info_json: Option<String> = row.get(20)?;
                let instruction_info: Vec<InstructionInfo> = instruction_info_json
                    .as_ref()
                    .and_then(|json| serde_json::from_str(json).ok())
                    .unwrap_or_default();

                let analysis_duration_ms: Option<u64> = row.get::<_, Option<i64>>(21)?
                    .map(|v| v as u64);

                let cached_analysis_json: Option<String> = row.get(22)?;
                let cached_analysis: Option<CachedAnalysis> = cached_analysis_json
                    .as_ref()
                    .and_then(|json| serde_json::from_str(json).ok());

                let fee_sol: f64 = row.get::<_, Option<f64>>(23)?.unwrap_or_default();

                let mut tx = Transaction {
                    signature: row.get(0)?,
                    slot: row.get(1)?,
                    block_time: row.get(2)?,
                    timestamp,
                    status,
                    transaction_type,
                    direction,
                    success: row.get(5)?,
                    error_message: row.get(6)?,
                    fee_sol,
                    fee_lamports: row.get(7)?,
                    compute_units_consumed: row.get(8)?,
                    instructions_count: row.get(9).unwrap_or_default(),
                    accounts_count: row.get(10).unwrap_or_default(),
                    sol_balance_change: sol_delta,
                    sol_balance_changes,
                    token_transfers,
                    token_balance_changes,
                    token_swap_info,
                    swap_pnl_info,
                    ata_operations,
                    instruction_info,
                    raw_transaction_data,
                    analysis_duration_ms,
                    cached_analysis,
                    last_updated: Utc::now(),
                    // These are populated from raw_transaction_data below
                    wallet_lamport_change: 0,
                    wallet_signed: false,
                    log_messages: Vec::new(),
                    instructions: Vec::new(),
                    position_impact: None,
                    profit_calculation: None,
                    ata_analysis: None,
                    token_info: None,
                    calculated_token_price_sol: None,
                    token_symbol: None,
                    token_decimals: None,
                };

                // Populate log_messages and instructions from raw_transaction_data
                tx.populate_from_raw_data();

                Ok(tx)
            },
        );

        match result {
            Ok(transaction) => Ok(Some(transaction)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(format!("Failed to get transaction: {e}")),
        }
    }

    /// Get successful transactions count
    pub async fn get_successful_transactions_count(&self) -> Result<u64, String> {
        let conn = self.get_connection()?;
        let wallet_address = crate::utils::get_wallet_address().map_err(|e| e.to_string())?;

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM raw_transactions WHERE wallet_address = ?1 AND success = true AND status != 'Failed'",
                params![wallet_address],
                |row| row.get(0),
            )
            .map_err(|e| format!("Failed to get successful transactions count: {e}"))?;

        Ok(count as u64)
    }

    /// Get failed transactions count
    pub async fn get_failed_transactions_count(&self) -> Result<u64, String> {
        let conn = self.get_connection()?;
        let wallet_address = crate::utils::get_wallet_address().map_err(|e| e.to_string())?;

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM raw_transactions WHERE wallet_address = ?1 AND (success = false OR status = 'Failed')",
                params![wallet_address],
                |row| row.get(0),
            )
            .map_err(|e| format!("Failed to get failed transactions count: {e}"))?;

        Ok(count as u64)
    }

    /// Get the timestamp of the most recent transaction for a wallet address.
    ///
    /// This is the canonical source for a wallet's "last used" time: the last
    /// moment any on-chain transaction involving the wallet was recorded. Prefers
    /// the on-chain `block_time` (unix seconds) and falls back to the stored
    /// RFC3339 `timestamp` text when `block_time` is missing. Returns `None` when
    /// the wallet has no recorded transactions.
    pub async fn get_latest_transaction_time(
        &self,
        wallet_address: &str,
    ) -> Result<Option<DateTime<Utc>>, String> {
        let conn = self.get_connection()?;

        let row: Option<(Option<i64>, String)> = conn
            .query_row(
                "SELECT block_time, timestamp FROM raw_transactions
                 WHERE wallet_address = ?1
                 ORDER BY COALESCE(block_time, 0) DESC, timestamp DESC
                 LIMIT 1",
                params![wallet_address],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(|e| format!("Failed to query latest transaction time: {e}"))?;

        Ok(row.and_then(|(block_time, timestamp)| {
            block_time
                .and_then(|secs| DateTime::<Utc>::from_timestamp(secs, 0))
                .or_else(|| {
                    DateTime::parse_from_rfc3339(&timestamp)
                        .ok()
                        .map(|dt| dt.with_timezone(&Utc))
                })
        }))
    }
}
