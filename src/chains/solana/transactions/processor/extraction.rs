//! Transaction processor extraction — extracts structured data from raw transaction bytes.
//
// Transaction processing pipeline - Data extraction methods
//
// This module contains methods for extracting data from RPC responses,
// including fetching transaction data and creating Transaction structures.

use crate::chains::solana::constants::lamports_to_sol;
use crate::logger::{self, LogTag};
use crate::transactions::types::*;
use chrono::{DateTime, Utc};

use super::core::TransactionProcessor;

// =============================================================================
// DATA EXTRACTION
// =============================================================================

impl TransactionProcessor {
    /// Load a cached raw transaction blob and decode it as Solana wire data. The
    /// shared database returns a generic JSON `Value` -- this is the one place that
    /// turns it into `TransactionDetails`, so the persistence layer stays free of
    /// Solana vendor types.
    async fn cached_transaction_details(
        &self,
        database: &crate::transactions::database::TransactionDatabase,
        signature: &str,
    ) -> Result<Option<crate::chains::solana::rpc::TransactionDetails>, String> {
        let Some(value) = database.get_raw_transaction_json(signature).await? else {
            return Ok(None);
        };
        serde_json::from_value(value).map(Some).map_err(|e| {
            format!("Failed to deserialize cached raw transaction for {signature}: {e}")
        })
    }

    /// Fetch transaction data with cache-first strategy
    pub(super) async fn fetch_transaction_data(
        &self,
        signature: &str,
    ) -> Result<crate::chains::solana::rpc::TransactionDetails, String> {
        // Import the global database (avoiding multiple instances for now)
        let database = crate::transactions::database::get_transaction_database()
            .await
            .ok_or_else(|| "Transaction database not initialized".to_owned())?;

        // Step 1: Handle cache-only mode - only try cache, never fetch from RPC
        if self.cache_only {
            if let Some(cached_details) = self
                .cached_transaction_details(&database, signature)
                .await?
            {
                if self.debug_enabled {
                    logger::info(
                        LogTag::Transactions,
                        &format!(
                            "Using cached raw transaction data (cache-only mode): {}",
                            signature
                        ),
                    );
                }
                return Ok(cached_details);
            } else {
                return Err(format!(
                    "Transaction {} not found in cache (cache-only mode)",
                    signature
                ));
            }
        }

        // Step 2: Handle force-refresh mode - skip cache and fetch fresh
        if self.force_refresh {
            if self.debug_enabled {
                logger::info(
                    LogTag::Transactions,
                    &format!("Force fetching fresh transaction data: {signature}"),
                );
            }
        } else {
            // Step 3: Normal mode - try cache first
            if let Some(cached_details) = self
                .cached_transaction_details(&database, signature)
                .await?
            {
                if self.debug_enabled {
                    logger::info(
                        LogTag::Transactions,
                        &format!("Using cached raw transaction data for: {signature}"),
                    );
                }
                return Ok(cached_details);
            }

            if self.debug_enabled {
                logger::info(
                    LogTag::Transactions,
                    &format!("Fetching fresh transaction data for: {signature}"),
                );
            }
        }

        // Step 4: Fetch from blockchain
        let tx_details = self.fetcher.fetch_transaction_details(signature).await?;

        // Step 5: Store in cache for future use (unless cache-only mode)
        if !self.cache_only {
            // Create a minimal transaction for caching raw data. A watched target's
            // processor sets `retain_raw_json = false`: the JSON blob is what makes a
            // busy wallet's rows far larger than the own wallet's, and dedupe already
            // means this signature is decoded once, so there is no repeat fetch to
            // save by caching it.
            let mut temp_transaction = Transaction::new(signature.to_string());
            temp_transaction.raw_transaction_data = if self.retain_raw_json {
                Some(
                    serde_json::to_value(&tx_details)
                        .map_err(|e| format!("Failed to serialize transaction details: {e}"))?,
                )
            } else {
                None
            };
            temp_transaction.slot = Some(tx_details.slot);
            temp_transaction.block_time = tx_details.block_time;
            if let Some(block_time) = tx_details.block_time {
                temp_transaction.timestamp =
                    DateTime::from_timestamp(block_time, 0).unwrap_or_else(|| Utc::now());
            }
            if let Some(meta) = &tx_details.meta {
                temp_transaction.success = match &meta.err {
                    None => true,
                    Some(v) => v.is_null(),
                };
                if !temp_transaction.success {
                    temp_transaction.error_message = meta.err.as_ref().map(|v| v.to_string());
                }
                temp_transaction.fee_lamports = Some(meta.fee);
            }
            temp_transaction.status = TransactionStatus::Confirmed;

            // Store raw transaction data in cache
            if let Err(e) = database
                .store_raw_transaction(self.subject(), &temp_transaction)
                .await
            {
                if self.debug_enabled {
                    logger::info(
                        LogTag::Transactions,
                        &format!(
                            "Failed to cache raw transaction data for {}: {}",
                            signature, e
                        ),
                    );
                }
            } else if self.debug_enabled {
                logger::info(
                    LogTag::Transactions,
                    &format!("Cached raw transaction data for: {signature}"),
                );
            }
        }

        Ok(tx_details)
    }

    /// Create Transaction structure from raw blockchain data
    pub(super) async fn create_transaction_from_data(
        &self,
        signature: &str,
        tx_data: &crate::chains::solana::rpc::TransactionDetails,
    ) -> Result<Transaction, String> {
        let mut transaction = Transaction::new(signature.to_string());

        // Store raw transaction data for future reference
        transaction.raw_transaction_data = Some(
            serde_json::to_value(tx_data)
                .map_err(|e| format!("Failed to serialize transaction data: {e}"))?,
        );

        // Add comprehensive debug logging for transaction structure
        if self.debug_enabled {
            // Parse instruction count from the transaction message
            let instructions_info =
                if let Some(instructions) = tx_data.transaction.message.get("instructions") {
                    if let Some(instructions_array) = instructions.as_array() {
                        format!("{} instructions found", instructions_array.len())
                    } else {
                        "instructions field not an array".to_owned()
                    }
                } else {
                    "no instructions field found".to_owned()
                };

            logger::info(
                LogTag::Transactions,
                &format!("Transaction {signature} structure: {instructions_info}"),
            );

            if let Some(meta) = &tx_data.meta {
                if let Some(log_messages) = &meta.log_messages {
                    let log_preview = if log_messages.len() > 5 {
                        format!(
                            "{} logs (showing first 5): {}",
                            log_messages.len(),
                            log_messages
                                .iter()
                                .take(5)
                                .cloned()
                                .collect::<Vec<_>>()
                                .join("| ")
                        )
                    } else {
                        format!("{} logs: {}", log_messages.len(), log_messages.join("| "))
                    };

                    logger::info(
                        LogTag::Transactions,
                        &format!("Transaction {signature} {log_preview}"),
                    );
                }

                logger::info(
                    LogTag::Transactions,
                    &format!(
                        "Transaction {} balance changes: pre_count={}, post_count={}",
                        signature,
                        meta.pre_balances.len(),
                        meta.post_balances.len()
                    ),
                );
            }

            // Parse account keys count
            let account_keys_info =
                if let Some(account_keys) = tx_data.transaction.message.get("accountKeys") {
                    if let Some(keys_array) = account_keys.as_array() {
                        format!("{} account keys", keys_array.len())
                    } else {
                        "accountKeys field not an array".to_owned()
                    }
                } else {
                    "no accountKeys field found".to_owned()
                };

            logger::info(
                LogTag::Transactions,
                &format!("Transaction {signature} accounts: {account_keys_info}"),
            );
        }

        // Extract basic data from tx_data
        if let Some(meta) = &tx_data.meta {
            // Treat JSON null as success (Solana encodes no-error as null)
            let success = match &meta.err {
                None => true,
                Some(v) => v.is_null(),
            };
            transaction.success = success;
            if !success {
                transaction.error_message = meta.err.as_ref().map(|v| v.to_string());
            }
            transaction.fee_lamports = Some(meta.fee);
            transaction.fee_sol = lamports_to_sol(meta.fee);
        }

        if let Some(block_time) = tx_data.block_time {
            transaction.block_time = Some(block_time);
            transaction.timestamp =
                DateTime::from_timestamp(block_time, 0).unwrap_or_else(|| Utc::now());
        }

        transaction.slot = Some(tx_data.slot);
        transaction.status = TransactionStatus::Confirmed;
        transaction.last_updated = Utc::now();

        // Populate log_messages and instructions from raw_transaction_data
        transaction.populate_from_raw_data();

        Ok(transaction)
    }
}

#[cfg(test)]
mod tests {
    use crate::chains::solana::solana_sdk::pubkey::Pubkey;
    use serde_json::json;

    use super::*;

    #[tokio::test]
    async fn watch_target_decode_projection_retains_raw_json_in_returned_value() {
        let details: crate::chains::solana::rpc::TransactionDetails =
            serde_json::from_value(json!({
                "slot": 42,
                "transaction": {
                    "message": {
                        "accountKeys": [Pubkey::default().to_string()],
                        "instructions": []
                    },
                    "signatures": ["signature"]
                },
                "meta": {
                    "err": null,
                    "preBalances": [1_000_000],
                    "postBalances": [995_000],
                    "preTokenBalances": [],
                    "postTokenBalances": [],
                    "fee": 5_000,
                    "computeUnitsConsumed": null,
                    "logMessages": [],
                    "innerInstructions": []
                },
                "blockTime": 1
            }))
            .expect("valid transaction fixture");
        let processor = TransactionProcessor::new_for_watch_target(Pubkey::default());

        let decoded = processor
            .create_transaction_from_data("signature", &details)
            .await
            .expect("project fixture");

        assert!(decoded.raw_transaction_data.is_some());
        assert!(!processor.retain_raw_json);
    }
}
