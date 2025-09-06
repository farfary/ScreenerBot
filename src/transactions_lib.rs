// Implementation file for TransactionsManager - Transaction analysis methods
use crate::errors::blockchain::{ is_permanent_failure, parse_structured_solana_error };
use crate::global::is_debug_transactions_enabled;
use crate::logger::{ log, LogTag };
use crate::pools::get_pool_service;
use crate::rpc::get_rpc_client;
use crate::tokens::decimals::{ lamports_to_sol, raw_to_ui_amount, sol_to_lamports };
use crate::tokens::{
    get_token_decimals,
    get_token_decimals_safe,
    PriceOptions,
    PriceSourceType,
    TokenDatabase,
};
use crate::transactions::TransactionsManager;
use crate::transactions_types::*;
use crate::utils::{ get_wallet_address, safe_truncate };
use chrono::{ DateTime, Utc };
use serde::{ Deserialize, Serialize };
use solana_sdk::pubkey::Pubkey;
use std::collections::{ HashMap, HashSet };
use std::str::FromStr;
use tokio::time::Duration;

impl TransactionsManager {
    /// Get transaction statistics
    pub fn get_stats(&self) -> TransactionStats {
        TransactionStats {
            total_transactions: self.total_transactions,
            new_transactions_count: self.new_transactions_count,
            known_signatures_count: self.known_signatures.len() as u64,
        }
    }

    /// Get global transaction statistics
    pub async fn get_transaction_stats() -> TransactionStats {
        // Default stats - would integrate with global manager
        TransactionStats {
            total_transactions: 0,
            new_transactions_count: 0,
            known_signatures_count: 0,
        }
    }

    /// Check if signature is known using database (if available) or fallback to HashSet
    pub async fn is_signature_known(&self, signature: &str) -> bool {
        // Use database if available, otherwise fallback to in-memory HashSet
        if let Some(ref db) = self.transaction_database {
            db.is_signature_known(signature).await.unwrap_or(false)
        } else {
            self.known_signatures.contains(signature)
        }
    }

    /// Add signature to known cache using database (if available) or fallback to HashSet
    pub async fn add_known_signature(&mut self, signature: &str) -> Result<(), String> {
        // Use database if available
        if let Some(ref db) = self.transaction_database {
            db.add_known_signature(signature).await?;
        } else {
            // Fallback to in-memory HashSet
            self.known_signatures.insert(signature.to_string());
        }
        Ok(())
    }

    /// Get count of known signatures
    pub async fn get_known_signatures_count(&self) -> u64 {
        if let Some(ref db) = self.transaction_database {
            db.get_known_signatures_count().await.unwrap_or(0)
        } else {
            self.known_signatures.len() as u64
        }
    }

    /// Get known signatures count (for testing)
    pub fn known_signatures(&self) -> &HashSet<String> {
        &self.known_signatures
    }

    /// Store deferred retry using database (if available) or fallback to HashMap
    pub async fn store_deferred_retry(&mut self, retry: &DeferredRetry) -> Result<(), String> {
        if let Some(ref db) = self.transaction_database {
            db.store_deferred_retry(
                &retry.signature,
                &retry.next_retry_at,
                retry.remaining_attempts,
                retry.current_delay_secs,
                retry.last_error.as_deref()
            ).await?;
        } else {
            // Fallback to in-memory HashMap
            self.deferred_retries.insert(retry.signature.clone(), retry.clone());
        }
        Ok(())
    }

    /// Get pending deferred retries using database (if available) or fallback to HashMap
    pub async fn get_pending_deferred_retries(&self) -> Result<Vec<DeferredRetry>, String> {
        if let Some(ref db) = self.transaction_database {
            let db_retries = db.get_pending_deferred_retries().await?;

            // Convert database format to our struct format
            let mut retries = Vec::new();
            for db_retry in db_retries {
                let next_retry_at = DateTime::parse_from_rfc3339(&db_retry.next_retry_at)
                    .map_err(|e| format!("Invalid date format: {}", e))?
                    .with_timezone(&Utc);

                retries.push(DeferredRetry {
                    signature: db_retry.signature,
                    next_retry_at,
                    remaining_attempts: db_retry.remaining_attempts,
                    current_delay_secs: db_retry.current_delay_secs,
                    last_error: db_retry.last_error,
                });
            }
            Ok(retries)
        } else {
            // Fallback to in-memory HashMap - filter for ready retries
            let now = Utc::now();
            let ready_retries: Vec<DeferredRetry> = self.deferred_retries
                .values()
                .filter(|retry| retry.next_retry_at <= now && retry.remaining_attempts > 0)
                .cloned()
                .collect();
            Ok(ready_retries)
        }
    }

    /// Remove deferred retry using database (if available) or fallback to HashMap
    pub async fn remove_deferred_retry(&mut self, signature: &str) -> Result<(), String> {
        if let Some(ref db) = self.transaction_database {
            db.remove_deferred_retry(signature).await?;
        } else {
            // Fallback to in-memory HashMap
            self.deferred_retries.remove(signature);
        }
        Ok(())
    }

    /// Cleanup expired deferred retries to prevent memory leaks
    pub async fn cleanup_expired_deferred_retries(&mut self) -> Result<usize, String> {
        let now = Utc::now();
        let mut cleaned_count = 0;

        if let Some(ref db) = self.transaction_database {
            // Database handles cleanup automatically, but we can still remove very old entries by count
            let pending_retries = db.get_pending_deferred_retries().await?;

            // Simple cleanup: if we have more than 1000 deferred retries, remove the oldest ones with no attempts
            if pending_retries.len() > 1000 {
                let expired_signatures: Vec<String> = pending_retries
                    .iter()
                    .filter(|retry| retry.remaining_attempts <= 0)
                    .take(100) // Remove up to 100 at a time
                    .map(|retry| retry.signature.clone())
                    .collect();

                for signature in expired_signatures {
                    db.remove_deferred_retry(&signature).await?;
                    cleaned_count += 1;
                }
            }
        } else {
            // Cleanup in-memory HashMap - simple size-based cleanup
            if self.deferred_retries.len() > 1000 {
                let expired_signatures: Vec<String> = self.deferred_retries
                    .iter()
                    .filter_map(|(signature, retry)| {
                        if retry.remaining_attempts <= 0 { Some(signature.clone()) } else { None }
                    })
                    .take(100) // Remove up to 100 at a time
                    .collect();

                for signature in expired_signatures {
                    self.deferred_retries.remove(&signature);
                    cleaned_count += 1;
                }
            }
        }

        if cleaned_count > 0 && self.debug_enabled {
            log(
                LogTag::Transactions,
                "CLEANUP",
                &format!("Cleaned up {} expired deferred retries", cleaned_count)
            );
        }

        Ok(cleaned_count)
    }

    /// Store processed transaction analysis in database (if available)
    pub async fn cache_processed_transaction(
        &self,
        transaction: &Transaction
    ) -> Result<(), String> {
        if let Some(ref db) = self.transaction_database {
            // Use the new store_full_transaction_analysis method for complete data
            db.store_full_transaction_analysis(transaction).await?;

            if self.debug_enabled {
                log(
                    LogTag::Transactions,
                    "DB_PROCESSED",
                    &format!(
                        "Cached processed transaction analysis {} to database",
                        &transaction.signature[..8]
                    )
                );
            }
        }
        // No fallback needed - processed data was never cached in JSON files before
        Ok(())
    }

    /// Save failed transaction state to database when processing fails
    pub async fn save_failed_transaction_state(
        &self,
        signature: &str,
        error: &str
    ) -> Result<(), String> {
        if let Some(ref db) = self.transaction_database {
            // Store minimal raw transaction record with failed status
            let now = Utc::now();

            // Try to store raw transaction record if it doesn't exist
            let raw_result = db.store_raw_transaction(
                signature,
                None, // no slot
                None, // no block_time
                &now,
                "Failed",
                false, // not successful
                Some(error),
                None // no raw data
            ).await;

            if raw_result.is_err() {
                // Raw transaction might already exist, try to update status only
                db.update_transaction_status(signature, "Failed", false, Some(error)).await?;
            }

            if self.debug_enabled {
                log(
                    LogTag::Transactions,
                    "FAILED_STATE_SAVED",
                    &format!(
                        "Saved failed transaction state for {} to database: {}",
                        signature,
                        error
                    )
                );
            }
        }
        Ok(())
    }

    /// Check for new transactions from wallet
    pub async fn check_new_transactions(&mut self) -> Result<Vec<String>, String> {
        let rpc_client = get_rpc_client();

        if self.debug_enabled {
            log(
                LogTag::Transactions,
                "RPC_CALL",
                &format!(
                    "Checking for new transactions (known: {}, using latest 50)",
                    self.get_known_signatures_count().await
                )
            );
        }

        // Get recent signatures from wallet
        // IMPORTANT: Always fetch most recent page (no 'before' cursor) to avoid missing new txs
        let signatures = rpc_client
            .get_wallet_signatures_main_rpc(&self.wallet_pubkey, 50, None).await
            .map_err(|e| format!("Failed to fetch wallet signatures: {}", e))?;

        let mut new_signatures = Vec::new();

        for sig_info in signatures {
            let signature = sig_info.signature;

            // Skip if we already know this signature
            if self.is_signature_known(&signature).await {
                continue;
            }

            // Add to known signatures (database or HashSet fallback)
            self.add_known_signature(&signature).await?;
            new_signatures.push(signature.clone());

            // Do not advance pagination cursor here; we always fetch the latest page
        }

        if !new_signatures.is_empty() {
            self.new_transactions_count += new_signatures.len() as u64;

            if self.debug_enabled {
                log(
                    LogTag::Transactions,
                    "NEW",
                    &format!("Found {} new transactions to process", new_signatures.len())
                );
            }
        }

        Ok(new_signatures)
    }

    /// Periodic deep gap detection and backfill (called less frequently than regular monitoring)
    /// This function checks for gaps in transaction history and fills them
    pub async fn check_and_backfill_gaps(&mut self) -> Result<usize, String> {
        log(
            LogTag::Transactions,
            "GAP_DETECTION",
            "🕵️ Starting periodic gap detection and backfill"
        );

        let rpc_client = get_rpc_client();
        let mut total_backfilled = 0;
        let mut batch_number = 0;
        let mut before_signature: Option<String> = None;

        // Check deeper history in batches of 1000 to find gaps
        loop {
            batch_number += 1;

            if self.debug_enabled {
                log(
                    LogTag::Transactions,
                    "GAP_DETECTION",
                    &format!("📦 Checking gap detection batch {} (1000 signatures)", batch_number)
                );
            }

            // Fetch batch of signatures
            let signatures = rpc_client
                .get_wallet_signatures_main_rpc(
                    &self.wallet_pubkey,
                    1000,
                    before_signature.as_deref()
                ).await
                .map_err(|e| {
                    format!("Failed to fetch gap detection batch {}: {}", batch_number, e)
                })?;

            if signatures.is_empty() {
                if self.debug_enabled {
                    log(
                        LogTag::Transactions,
                        "GAP_DETECTION",
                        "📭 No more signatures found - gap detection complete"
                    );
                }
                break;
            }

            let mut new_in_batch = 0;
            let mut all_known = true;

            // Check each signature for gaps
            for sig_info in &signatures {
                let signature = &sig_info.signature;

                if !self.is_signature_known(signature).await {
                    all_known = false;

                    // Found a gap - fill it
                    self.add_known_signature(signature).await?;
                    new_in_batch += 1;
                    total_backfilled += 1;

                    // Process the transaction
                    if let Err(e) = self.process_transaction(signature).await {
                        let error_msg = format!(
                            "Failed to process gap-fill transaction {}: {}",
                            &signature[..8],
                            e
                        );
                        log(LogTag::Transactions, "WARN", &error_msg);

                        // Save failed state to database for gap-fill processing
                        if
                            let Err(db_err) = self.save_failed_transaction_state(
                                &signature,
                                &e
                            ).await
                        {
                            log(
                                LogTag::Transactions,
                                "ERROR",
                                &format!(
                                    "Failed to save gap-fill transaction failure state for {}: {}",
                                    &signature[..8],
                                    db_err
                                )
                            );
                        }
                    }
                }

                // Update pagination cursor
                before_signature = Some(signature.clone());
            }

            if self.debug_enabled {
                log(
                    LogTag::Transactions,
                    "GAP_DETECTION",
                    &format!("📊 Batch {} complete: {} gaps filled", batch_number, new_in_batch)
                );
            }

            // Stopping conditions
            if new_in_batch == 0 && all_known {
                // No gaps found in this batch - we're done
                log(
                    LogTag::Transactions,
                    "GAP_DETECTION",
                    "✅ No more gaps found - backfill complete"
                );
                break;
            } else if batch_number >= 5 {
                // Safety limit - don't check more than 5000 transactions at once
                log(
                    LogTag::Transactions,
                    "GAP_DETECTION",
                    "⚠️ Reached safety limit of 5 batches - stopping gap detection"
                );
                break;
            }

            // Rate limiting between batches
            tokio::time::sleep(Duration::from_millis(200)).await;
        }

        if total_backfilled > 0 {
            log(
                LogTag::Transactions,
                "GAP_DETECTION",
                &format!("🔧 Gap detection complete: backfilled {} missing transactions", total_backfilled)
            );

            // Update statistics
            self.new_transactions_count += total_backfilled as u64;
        } else {
            log(
                LogTag::Transactions,
                "GAP_DETECTION",
                "✨ No gaps found - transaction history is complete"
            );
        }

        Ok(total_backfilled)
    }

    /// Analyze Jupiter swap transactions
    async fn analyze_jupiter_swap(
        &self,
        transaction: &Transaction
    ) -> Result<TransactionType, String> {
        let log_text = transaction.log_messages.join(" ");

        // Jupiter swaps are identified by:
        // 1. Jupiter program ID presence (already checked in caller)
        // 2. ATA creation for tokens (indicating swap setup)
        // 3. Token transfer instructions
        // 4. Router instruction patterns

        if self.debug_enabled {
            log(
                LogTag::Transactions,
                "PUMP_ANALYSIS",
                &format!("{} - Analyzing Jupiter swap", &transaction.signature[..8])
            );
        }

        // Extract token mint from the transaction data
        let target_token_mint = self.extract_target_token_mint_from_jupiter(transaction).await;

        let has_wsol_operations = log_text.contains("So11111111111111111111111111111111111111112");
        let has_token_operations =
            log_text.contains("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL") ||
            log_text.contains("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
        let has_jupiter_route =
            log_text.contains("Instruction: Route") ||
            log_text.contains("JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4");

        // Extract actual SOL amount from transfer instructions or balance changes
        let sol_amount = self.extract_sol_amount_from_jupiter(transaction).await;
        let token_amount = self.extract_token_amount_from_jupiter(transaction).await;

        // Jupiter swaps can be detected even if they fail, based on intent and instruction patterns
        if has_jupiter_route && has_token_operations {
            // Determine swap direction based on both SOL and token balance changes
            // Priority: 1) Token balance direction, 2) SOL balance direction

            // Check if we have significant token amounts to determine direction
            if token_amount > 1.0 {
                // We have token amounts, determine direction from balance changes
                if transaction.sol_balance_change > 0.000001 {
                    // User gained SOL and we detected token amounts = Token to SOL swap (SELL)
                    return Ok(TransactionType::SwapTokenToSol {
                        router: "Jupiter".to_string(),
                        token_mint: target_token_mint.unwrap_or_else(|| "Unknown".to_string()),
                        token_amount: token_amount,
                        sol_amount: transaction.sol_balance_change.abs(),
                    });
                } else if transaction.sol_balance_change < -0.000001 {
                    // User lost SOL and we detected token amounts = SOL to Token swap (BUY)
                    return Ok(TransactionType::SwapSolToToken {
                        router: "Jupiter".to_string(),
                        token_mint: target_token_mint.unwrap_or_else(|| "Unknown".to_string()),
                        sol_amount: transaction.sol_balance_change.abs(),
                        token_amount: token_amount,
                    });
                }
            }

            // Fallback to original SOL-based logic if token direction is unclear
            if transaction.sol_balance_change < -0.000001 {
                // SOL to Token swap (BUY) - user spent SOL
                return Ok(TransactionType::SwapSolToToken {
                    router: "Jupiter".to_string(),
                    token_mint: target_token_mint.unwrap_or_else(|| "Unknown".to_string()),
                    sol_amount: transaction.sol_balance_change.abs(),
                    token_amount: token_amount,
                });
            } else if transaction.sol_balance_change > 0.000001 {
                // Token to SOL swap (SELL) - user received SOL
                return Ok(TransactionType::SwapTokenToSol {
                    router: "Jupiter".to_string(),
                    token_mint: target_token_mint.unwrap_or_else(|| "Unknown".to_string()),
                    token_amount: token_amount,
                    sol_amount: transaction.sol_balance_change.abs(),
                });
            } else if has_token_operations && !transaction.token_transfers.is_empty() {
                // Token to Token swap
                return Ok(TransactionType::SwapTokenToToken {
                    router: "Jupiter".to_string(),
                    from_mint: "Unknown".to_string(),
                    to_mint: target_token_mint.unwrap_or_else(|| "Unknown".to_string()),
                    from_amount: 0.0,
                    to_amount: token_amount,
                });
            } else {
                // Generic Jupiter swap when we can't determine exact type
                return Ok(TransactionType::SwapSolToToken {
                    router: "Jupiter".to_string(),
                    token_mint: target_token_mint.unwrap_or_else(|| "Unknown".to_string()),
                    sol_amount: sol_amount.max(0.000001),
                    token_amount: token_amount,
                });
            }
        }

        Err("Not a Jupiter swap".to_string())
    }

    /// Extract target token mint from Jupiter transaction
    /// Strategy:
    /// - Use the newly populated token_balance_changes for reliable detection
    /// - For SELL (SOL increase), choose the non-WSOL mint with the most negative delta (tokens decreased)
    /// - For BUY (SOL decrease), choose the non-WSOL mint with the most positive delta (tokens increased)
    /// - Fallback to raw data parsing if token_balance_changes is empty
    async fn extract_target_token_mint_from_jupiter(
        &self,
        transaction: &Transaction
    ) -> Option<String> {
        let epsilon = 1e-12f64;

        // 1) Use newly populated token_balance_changes (most reliable)
        if !transaction.token_balance_changes.is_empty() {
            let is_sell = transaction.sol_balance_change > 0.000001; // gained SOL
            let is_buy = transaction.sol_balance_change < -0.000001; // spent SOL

            if is_sell {
                // Find token with largest decrease (most negative change)
                if
                    let Some(token_change) = transaction.token_balance_changes
                        .iter()
                        .filter(|tc| tc.mint != WSOL_MINT && tc.change < -epsilon)
                        .min_by(|a, b|
                            a.change.partial_cmp(&b.change).unwrap_or(std::cmp::Ordering::Equal)
                        )
                {
                    if self.debug_enabled {
                        log(
                            LogTag::Transactions,
                            "JUPITER_MINT",
                            &format!(
                                "🎯 Found sell token mint from token_balance_changes: {} (change: {})",
                                token_change.mint,
                                token_change.change
                            )
                        );
                    }
                    return Some(token_change.mint.clone());
                }
            } else if is_buy {
                // Find token with largest increase (most positive change)
                if
                    let Some(token_change) = transaction.token_balance_changes
                        .iter()
                        .filter(|tc| tc.mint != WSOL_MINT && tc.change > epsilon)
                        .max_by(|a, b|
                            a.change.partial_cmp(&b.change).unwrap_or(std::cmp::Ordering::Equal)
                        )
                {
                    if self.debug_enabled {
                        log(
                            LogTag::Transactions,
                            "JUPITER_MINT",
                            &format!(
                                "🎯 Found buy token mint from token_balance_changes: {} (change: {})",
                                token_change.mint,
                                token_change.change
                            )
                        );
                    }
                    return Some(token_change.mint.clone());
                }
            }

            // If direction is unclear, pick the largest absolute change
            if
                let Some(token_change) = transaction.token_balance_changes
                    .iter()
                    .filter(|tc| tc.mint != WSOL_MINT)
                    .max_by(|a, b|
                        a.change
                            .abs()
                            .partial_cmp(&b.change.abs())
                            .unwrap_or(std::cmp::Ordering::Equal)
                    )
            {
                if self.debug_enabled {
                    log(
                        LogTag::Transactions,
                        "JUPITER_MINT",
                        &format!(
                            "🎯 Found token mint by largest change: {} (change: {})",
                            token_change.mint,
                            token_change.change
                        )
                    );
                }
                return Some(token_change.mint.clone());
            }
        }

        // 2) Fallback: Parse raw data if token_balance_changes is empty (legacy)
        let wallet_str = self.wallet_pubkey.to_string();

        if let Some(raw_data) = &transaction.raw_transaction_data {
            if let Some(meta) = raw_data.get("meta") {
                if
                    let (Some(pre_balances), Some(post_balances)) = (
                        meta.get("preTokenBalances").and_then(|v| v.as_array()),
                        meta.get("postTokenBalances").and_then(|v| v.as_array()),
                    )
                {
                    // Gather deltas for wallet-owned token accounts (exclude WSOL)
                    let mut candidates: Vec<(String, f64)> = Vec::new();
                    for post_balance in post_balances {
                        let owner = post_balance.get("owner").and_then(|v| v.as_str());
                        let mint = post_balance.get("mint").and_then(|v| v.as_str());
                        if owner == Some(wallet_str.as_str()) {
                            if let Some(mint_str) = mint {
                                if mint_str == WSOL_MINT {
                                    continue;
                                }
                                let account_index = post_balance
                                    .get("accountIndex")
                                    .and_then(|v| v.as_u64());
                                let pre_amount = pre_balances
                                    .iter()
                                    .find(|pre| {
                                        pre.get("accountIndex").and_then(|v| v.as_u64()) ==
                                            account_index
                                    })
                                    .and_then(|pre| pre.get("uiTokenAmount"))
                                    .and_then(|ui| ui.get("uiAmount"))
                                    .and_then(|v| v.as_f64())
                                    .unwrap_or(0.0);
                                let post_amount = post_balance
                                    .get("uiTokenAmount")
                                    .and_then(|ui| ui.get("uiAmount"))
                                    .and_then(|v| v.as_f64())
                                    .unwrap_or(0.0);
                                let delta = post_amount - pre_amount; // positive = increased, negative = decreased
                                if delta.abs() > epsilon {
                                    candidates.push((mint_str.to_string(), delta));
                                }
                            }
                        }
                    }

                    if !candidates.is_empty() {
                        // Decide on expected direction from SOL balance change
                        let is_sell = transaction.sol_balance_change > 0.000001; // gained SOL
                        let is_buy = transaction.sol_balance_change < -0.000001; // spent SOL

                        if is_sell {
                            // Pick most negative delta (largest token decrease)
                            if
                                let Some((mint, _)) = candidates
                                    .iter()
                                    .filter(|(_, d)| *d < -epsilon)
                                    .min_by(|a, b| {
                                        a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal)
                                    })
                            {
                                return Some(mint.clone());
                            }
                        } else if is_buy {
                            // Pick most positive delta (largest token increase)
                            if
                                let Some((mint, _)) = candidates
                                    .iter()
                                    .filter(|(_, d)| *d > epsilon)
                                    .max_by(|a, b| {
                                        a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal)
                                    })
                            {
                                return Some(mint.clone());
                            }
                        }

                        // Fallback: pick largest absolute delta if direction unclear
                        if
                            let Some((mint, _)) = candidates
                                .iter()
                                .max_by(|a, b| {
                                    a.1
                                        .abs()
                                        .partial_cmp(&b.1.abs())
                                        .unwrap_or(std::cmp::Ordering::Equal)
                                })
                        {
                            return Some(mint.clone());
                        }
                    }
                }
            }
        }

        // 2) Fallback: Look for ATA creation instructions for non-WSOL tokens
        if let Some(raw_data) = &transaction.raw_transaction_data {
            if let Some(meta) = raw_data.get("meta") {
                if
                    let Some(inner_instructions) = meta
                        .get("innerInstructions")
                        .and_then(|v| v.as_array())
                {
                    for inner_group in inner_instructions {
                        if
                            let Some(instructions) = inner_group
                                .get("instructions")
                                .and_then(|v| v.as_array())
                        {
                            for instruction in instructions {
                                if let Some(parsed) = instruction.get("parsed") {
                                    if let Some(info) = parsed.get("info") {
                                        if
                                            let Some(mint) = info
                                                .get("mint")
                                                .and_then(|v| v.as_str())
                                        {
                                            if mint != WSOL_MINT {
                                                return Some(mint.to_string());
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        None
    }

    /// Extract SOL amount from Jupiter transaction
    async fn extract_sol_amount_from_jupiter(&self, transaction: &Transaction) -> f64 {
        // Look for SOL transfer instructions in the transaction
        if let Some(raw_data) = &transaction.raw_transaction_data {
            if let Some(transaction_data) = raw_data.get("transaction") {
                if let Some(message) = transaction_data.get("message") {
                    if
                        let Some(instructions) = message
                            .get("instructions")
                            .and_then(|v| v.as_array())
                    {
                        for instruction in instructions {
                            if let Some(parsed) = instruction.get("parsed") {
                                if let Some(info) = parsed.get("info") {
                                    if
                                        let Some(lamports) = info
                                            .get("lamports")
                                            .and_then(|v| v.as_u64())
                                    {
                                        return (lamports as f64) / 1_000_000_000.0;
                                        // Convert to SOL
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        transaction.sol_balance_change.abs()
    }

    /// Extract token amount from Jupiter transaction
    /// Returns absolute token amount moved to/from the wallet for the non-WSOL mint.
    async fn extract_token_amount_from_jupiter(&self, transaction: &Transaction) -> f64 {
        // Use the newly populated token_balance_changes (most reliable)
        if !transaction.token_balance_changes.is_empty() {
            // Find the largest non-WSOL token balance change
            let mut largest_change = 0.0f64;
            for token_change in &transaction.token_balance_changes {
                if token_change.mint != WSOL_MINT && token_change.change.abs() > largest_change {
                    largest_change = token_change.change.abs();
                }
            }

            if largest_change > 0.0 {
                if self.debug_enabled {
                    log(
                        LogTag::Transactions,
                        "JUPITER_TOKEN",
                        &format!("🔢 Jupiter token amount from token_balance_changes: {}", largest_change)
                    );
                }
                return largest_change;
            }
        }

        // Fallback: Check existing token_transfers
        if !transaction.token_transfers.is_empty() {
            return transaction.token_transfers[0].amount;
        }

        if self.debug_enabled {
            log(
                LogTag::Transactions,
                "JUPITER_TOKEN_WARN",
                &format!(
                    "⚠️ No token amount found for Jupiter transaction {}",
                    &transaction.signature[..8]
                )
            );
        }

        0.0
    }

    /// Analyze GMGN swap transactions
    /// GMGN is an external router that shows token balance changes but doesn't match standard program IDs
    async fn analyze_gmgn_swap(
        &self,
        transaction: &Transaction
    ) -> Result<TransactionType, String> {
        if self.debug_enabled {
            log(
                LogTag::Transactions,
                "GMGN_ANALYSIS",
                &format!("{} - Analyzing GMGN swap", &transaction.signature[..8])
            );
        }

        // For GMGN swaps, we primarily rely on balance changes since program IDs may vary
        let has_token_operations =
            !transaction.token_transfers.is_empty() ||
            transaction.log_messages
                .iter()
                .any(|msg| {
                    msg.contains("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA") ||
                        msg.contains("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL")
                });

        // Extract token mint from transaction
        let target_token_mint = self
            .extract_target_token_mint_from_gmgn(transaction).await
            .unwrap_or_else(|| "Unknown".to_string());

        // Extract amounts
        let token_amount = self.extract_token_amount_from_gmgn(transaction).await;

        if self.debug_enabled {
            log(
                LogTag::Transactions,
                "GMGN_ANALYSIS",
                &format!(
                    "{} - GMGN analysis: sol_change={}, token_amount={}, has_token_ops={}",
                    &transaction.signature[..8],
                    transaction.sol_balance_change,
                    token_amount,
                    has_token_operations
                )
            );
        }

        // Determine swap direction based on SOL balance change
        if has_token_operations && transaction.sol_balance_change.abs() > 0.000001 {
            if transaction.sol_balance_change < -0.000001 {
                // User spent SOL = SOL to Token swap (BUY)
                return Ok(TransactionType::SwapSolToToken {
                    router: "GMGN".to_string(),
                    token_mint: target_token_mint,
                    sol_amount: transaction.sol_balance_change.abs(),
                    token_amount: token_amount,
                });
            } else if transaction.sol_balance_change > 0.000001 {
                // User received SOL = Token to SOL swap (SELL)
                return Ok(TransactionType::SwapTokenToSol {
                    router: "GMGN".to_string(),
                    token_mint: target_token_mint,
                    token_amount: token_amount,
                    sol_amount: transaction.sol_balance_change.abs(),
                });
            }
        }

        Err("Not a GMGN swap".to_string())
    }

    /// Extract target token mint from GMGN transaction
    async fn extract_target_token_mint_from_gmgn(
        &self,
        transaction: &Transaction
    ) -> Option<String> {
        // First check token transfers
        if !transaction.token_transfers.is_empty() {
            return Some(transaction.token_transfers[0].mint.clone());
        }

        // Check pre/post token balance changes similar to Jupiter
        if let Some(raw_data) = &transaction.raw_transaction_data {
            if let Some(meta) = raw_data.get("meta") {
                if
                    let (Some(pre_balances), Some(post_balances)) = (
                        meta.get("preTokenBalances").and_then(|v| v.as_array()),
                        meta.get("postTokenBalances").and_then(|v| v.as_array()),
                    )
                {
                    let wallet_str = self.wallet_pubkey.to_string();

                    // First, look for token balance changes for our wallet in post-balances (excluding WSOL)
                    for post_balance in post_balances {
                        if
                            let Some(post_owner) = post_balance
                                .get("owner")
                                .and_then(|v| v.as_str())
                        {
                            if let Some(mint) = post_balance.get("mint").and_then(|v| v.as_str()) {
                                if
                                    post_owner == wallet_str &&
                                    mint != "So11111111111111111111111111111111111111112"
                                {
                                    return Some(mint.to_string());
                                }
                            }
                        }
                    }

                    // If not found in post-balances, check pre-balances for tokens that were sold (ATA closed)
                    for pre_balance in pre_balances {
                        if let Some(pre_owner) = pre_balance.get("owner").and_then(|v| v.as_str()) {
                            if let Some(mint) = pre_balance.get("mint").and_then(|v| v.as_str()) {
                                if
                                    pre_owner == wallet_str &&
                                    mint != "So11111111111111111111111111111111111111112"
                                {
                                    // Check if this token's ATA was closed (not in post-balances)
                                    let account_index = pre_balance
                                        .get("accountIndex")
                                        .and_then(|v| v.as_u64())
                                        .unwrap_or(999);

                                    let still_exists = post_balances
                                        .iter()
                                        .any(|post| {
                                            post.get("accountIndex").and_then(|v| v.as_u64()) ==
                                                Some(account_index)
                                        });

                                    if !still_exists {
                                        return Some(mint.to_string());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        None
    }

    /// Extract token amount from GMGN transaction
    async fn extract_token_amount_from_gmgn(&self, transaction: &Transaction) -> f64 {
        // First check existing token transfers
        if !transaction.token_transfers.is_empty() {
            return transaction.token_transfers[0].amount;
        }

        // Check pre/post token balance changes similar to Jupiter method
        if let Some(raw_data) = &transaction.raw_transaction_data {
            if let Some(meta) = raw_data.get("meta") {
                if
                    let (Some(pre_balances), Some(post_balances)) = (
                        meta.get("preTokenBalances").and_then(|v| v.as_array()),
                        meta.get("postTokenBalances").and_then(|v| v.as_array()),
                    )
                {
                    let wallet_str = self.wallet_pubkey.to_string();

                    // First check for token balance changes in post-balances
                    for (post_idx, post_balance) in post_balances.iter().enumerate() {
                        if
                            let Some(post_owner) = post_balance
                                .get("owner")
                                .and_then(|v| v.as_str())
                        {
                            let mint_str = post_balance
                                .get("mint")
                                .and_then(|v| v.as_str())
                                .unwrap_or("unknown");

                            // Skip WSOL
                            if mint_str == "So11111111111111111111111111111111111111112" {
                                continue;
                            }

                            if post_owner == wallet_str {
                                let account_index = post_balance
                                    .get("accountIndex")
                                    .and_then(|v| v.as_u64())
                                    .unwrap_or(999);

                                // Get pre-balance for same account
                                let pre_amount = pre_balances
                                    .iter()
                                    .find(|pre| {
                                        pre.get("accountIndex").and_then(|v| v.as_u64()) ==
                                            Some(account_index)
                                    })
                                    .and_then(|pre| pre.get("uiTokenAmount"))
                                    .and_then(|ui| ui.get("uiAmount"))
                                    .and_then(|v| v.as_f64())
                                    .unwrap_or(0.0);

                                // Get post-balance
                                let post_amount = post_balance
                                    .get("uiTokenAmount")
                                    .and_then(|ui| ui.get("uiAmount"))
                                    .and_then(|v| v.as_f64())
                                    .unwrap_or(0.0);

                                let token_change = post_amount - pre_amount;

                                if self.debug_enabled {
                                    log(
                                        LogTag::Transactions,
                                        "GMGN_TOKEN",
                                        &format!(
                                            "💰 GMGN token balance change for account[{}]: {} -> {} = {} (mint: {})",
                                            account_index,
                                            pre_amount,
                                            post_amount,
                                            token_change,
                                            mint_str
                                        )
                                    );
                                }

                                if token_change.abs() > 1e-12 {
                                    return token_change.abs();
                                }
                            }
                        }
                    }

                    // If no change found in post-balances, check for tokens that were sold (ATA closed)
                    for pre_balance in pre_balances {
                        if let Some(pre_owner) = pre_balance.get("owner").and_then(|v| v.as_str()) {
                            let mint_str = pre_balance
                                .get("mint")
                                .and_then(|v| v.as_str())
                                .unwrap_or("unknown");

                            // Skip WSOL
                            if mint_str == "So11111111111111111111111111111111111111112" {
                                continue;
                            }

                            if pre_owner == wallet_str {
                                let account_index = pre_balance
                                    .get("accountIndex")
                                    .and_then(|v| v.as_u64())
                                    .unwrap_or(999);

                                // Check if this token's ATA was closed (not in post-balances)
                                let still_exists = post_balances
                                    .iter()
                                    .any(|post| {
                                        post.get("accountIndex").and_then(|v| v.as_u64()) ==
                                            Some(account_index)
                                    });

                                if !still_exists {
                                    // ATA was closed, return the pre-balance amount
                                    let pre_amount = pre_balance
                                        .get("uiTokenAmount")
                                        .and_then(|ui| ui.get("uiAmount"))
                                        .and_then(|v| v.as_f64())
                                        .unwrap_or(0.0);

                                    if self.debug_enabled {
                                        log(
                                            LogTag::Transactions,
                                            "GMGN_TOKEN",
                                            &format!(
                                                "💰 GMGN token sold (ATA closed) for account[{}]: {} tokens (mint: {})",
                                                account_index,
                                                pre_amount,
                                                mint_str
                                            )
                                        );
                                    }

                                    return pre_amount;
                                }
                            }
                        }
                    }
                }
            }
        }

        0.0
    }

    /// Analyze Raydium swap transactions (both AMM and CPMM)
    async fn analyze_raydium_swap(
        &self,
        transaction: &Transaction
    ) -> Result<TransactionType, String> {
        let log_text = transaction.log_messages.join(" ");

        // Raydium swaps are identified by:
        // 1. Raydium program ID presence (already checked in caller)
        // 2. Token operations (Token program, ATA operations)
        // 3. SOL balance changes indicating SOL involvement
        // 4. CPMM or AMM program instructions

        let has_token_operations =
            log_text.contains("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA") ||
            log_text.contains("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");

        // Extract actual token information from Raydium swap
        let (token_mint, token_symbol, token_amount, mut sol_amount) =
            self.extract_raydium_swap_info(transaction).await;

        // Try to extract SOL (WSOL) amount from inner instructions by summing transferChecked amounts (handles fee/referral splits)
        if sol_amount.is_none() {
            if let Some(raw_data) = &transaction.raw_transaction_data {
                if let Some(meta) = raw_data.get("meta") {
                    if let Some(inner) = meta.get("innerInstructions").and_then(|v| v.as_array()) {
                        let mut wsol_sum = 0.0f64;
                        for group in inner {
                            if
                                let Some(instructions) = group
                                    .get("instructions")
                                    .and_then(|v| v.as_array())
                            {
                                for instr in instructions {
                                    if let Some(parsed) = instr.get("parsed") {
                                        if let Some(info) = parsed.get("info") {
                                            if
                                                let (Some(mint), Some(token_amount)) = (
                                                    info.get("mint").and_then(|v| v.as_str()),
                                                    info.get("tokenAmount"),
                                                )
                                            {
                                                if
                                                    mint ==
                                                    "So11111111111111111111111111111111111111112"
                                                {
                                                    if
                                                        let Some(ui) = token_amount
                                                            .get("uiAmount")
                                                            .and_then(|v| v.as_f64())
                                                    {
                                                        if ui > 0.0 {
                                                            wsol_sum += ui;
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        if wsol_sum > 0.0 {
                            sol_amount = Some(wsol_sum);
                        }
                    }
                }
            }
        }

        // Check for SOL to Token swap (SOL spent) - lower threshold for failed transactions
        if transaction.sol_balance_change < -0.000001 {
            // Spent more than 0.000001 SOL
            return Ok(TransactionType::SwapSolToToken {
                token_mint: token_mint.clone(),
                sol_amount: sol_amount.unwrap_or_else(|| transaction.sol_balance_change.abs()),
                token_amount,
                router: self.determine_raydium_router(transaction),
            });
        } else if
            // Check for Token to SOL swap (SOL received)
            transaction.sol_balance_change > 0.000001
        {
            // Received more than 0.000001 SOL
            return Ok(TransactionType::SwapTokenToSol {
                token_mint: token_mint.clone(),
                token_amount,
                sol_amount: sol_amount.unwrap_or_else(|| transaction.sol_balance_change.abs()),
                router: self.determine_raydium_router(transaction),
            });
        } else if
            // Check for Token to Token swap (minimal SOL change but has token operations)
            has_token_operations &&
            !transaction.token_transfers.is_empty()
        {
            return Ok(TransactionType::SwapTokenToToken {
                from_mint: token_mint.clone(),
                to_mint: "Unknown".to_string(), // For now, handle as single token
                from_amount: token_amount,
                to_amount: 0.0,
                router: self.determine_raydium_router(transaction),
            });
        } else if
            // Detect based on program presence even if no clear balance change
            has_token_operations
        {
            return Ok(TransactionType::SwapSolToToken {
                token_mint: token_mint.clone(),
                sol_amount: sol_amount.unwrap_or_else(|| transaction.sol_balance_change.abs()),
                token_amount,
                router: self.determine_raydium_router(transaction),
            });
        }

        Err("Not a Raydium swap".to_string())
    }

    /// Extract token information from Raydium swap transaction
    async fn extract_raydium_swap_info(
        &self,
        transaction: &Transaction
    ) -> (String, String, f64, Option<f64>) {
        // Method 1: Check pre/post token balance changes (most reliable for Raydium)
        if let Some(raw_data) = &transaction.raw_transaction_data {
            if let Some(meta) = raw_data.get("meta") {
                if
                    let (Some(pre_balances), Some(post_balances)) = (
                        meta.get("preTokenBalances").and_then(|v| v.as_array()),
                        meta.get("postTokenBalances").and_then(|v| v.as_array()),
                    )
                {
                    let wallet_str = self.wallet_pubkey.to_string();
                    log(
                        LogTag::Transactions,
                        "RAYDIUM_TOKEN",
                        &format!("🔍 Analyzing Raydium token balance changes for wallet: {}", wallet_str)
                    );

                    for (post_idx, post_balance) in post_balances.iter().enumerate() {
                        if
                            let Some(post_owner) = post_balance
                                .get("owner")
                                .and_then(|v| v.as_str())
                        {
                            if post_owner == wallet_str {
                                let account_index = post_balance
                                    .get("accountIndex")
                                    .and_then(|v| v.as_u64())
                                    .unwrap_or(999);

                                // Get pre-balance for same account
                                let pre_amount = pre_balances
                                    .iter()
                                    .find(|pre| {
                                        pre.get("accountIndex").and_then(|v| v.as_u64()) ==
                                            Some(account_index)
                                    })
                                    .and_then(|pre| pre.get("uiTokenAmount"))
                                    .and_then(|ui| ui.get("uiAmount"))
                                    .and_then(|v| v.as_f64())
                                    .unwrap_or(0.0);

                                // Get post-balance
                                let post_amount = post_balance
                                    .get("uiTokenAmount")
                                    .and_then(|ui| ui.get("uiAmount"))
                                    .and_then(|v| v.as_f64())
                                    .unwrap_or(0.0);

                                let token_change = post_amount - pre_amount;

                                if
                                    let Some(mint) = post_balance
                                        .get("mint")
                                        .and_then(|v| v.as_str())
                                {
                                    // Skip SOL/WSOL
                                    if mint == "So11111111111111111111111111111111111111112" {
                                        continue;
                                    }

                                    // Check for significant token balance change
                                    if token_change.abs() > 0.1 {
                                        // More than 0.1 token changed
                                        log(
                                            LogTag::Transactions,
                                            "RAYDIUM_TOKEN",
                                            &format!(
                                                "💰 Raydium token balance change: {} -> {} = {} (mint: {})",
                                                pre_amount,
                                                post_amount,
                                                token_change,
                                                mint
                                            )
                                        );

                                        // Get token symbol from database
                                        let token_symbol = if
                                            let Some(ref db) = self.token_database
                                        {
                                            match db.get_token_by_mint(mint) {
                                                Ok(Some(token_info)) => token_info.symbol,
                                                _ => format!("TOKEN_{}", mint),
                                            }
                                        } else {
                                            format!("TOKEN_{}", mint)
                                        };

                                        return (
                                            mint.to_string(),
                                            token_symbol,
                                            token_change.abs(),
                                            None,
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Method 2: Fallback to existing token_transfers if available
        if !transaction.token_transfers.is_empty() {
            let transfer = &transaction.token_transfers[0];
            let token_symbol = if let Some(ref db) = self.token_database {
                match db.get_token_by_mint(&transfer.mint) {
                    Ok(Some(token_info)) => token_info.symbol,
                    _ => format!("TOKEN_{}", &transfer.mint),
                }
            } else {
                format!("TOKEN_{}", &transfer.mint)
            };

            return (transfer.mint.clone(), token_symbol, transfer.amount, None);
        }

        // Method 3: Final fallback
        ("Unknown".to_string(), "TOKEN_Unknown".to_string(), 0.0, None)
    }

    /// Determine the specific Raydium router being used
    fn determine_raydium_router(&self, transaction: &Transaction) -> String {
        let log_text = transaction.log_messages.join(" ");

        // Check for specific Raydium program IDs
        if log_text.contains("CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C") {
            "Raydium".to_string()
        } else if log_text.contains("CPMMoo8L3wrBtphwOYMpCX4LtjRWB3gjCMFdukgp6EEh") {
            "Raydium CPMM".to_string()
        } else if log_text.contains("CPMMoo8L3VgkEru3h4j8mu4baRUeJBmK7nfD5fC2pXg") {
            "Raydium CAMM".to_string()
        } else if log_text.contains("675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8") {
            "Raydium AMM".to_string()
        } else {
            "Raydium".to_string()
        }
    }

    /// Analyze Orca swap transactions
    async fn analyze_orca_swap(
        &self,
        transaction: &Transaction
    ) -> Result<TransactionType, String> {
        let log_text = transaction.log_messages.join(" ");

        if log_text.contains("swap") || log_text.contains("Swap") {
            let has_wsol = log_text.contains("So11111111111111111111111111111111111111112");

            if has_wsol && transaction.sol_balance_change < 0.0 {
                return Ok(TransactionType::SwapSolToToken {
                    token_mint: "Unknown".to_string(),
                    sol_amount: transaction.sol_balance_change.abs(),
                    token_amount: 0.0,
                    router: "Orca".to_string(),
                });
            } else if has_wsol && transaction.sol_balance_change > 0.0 {
                return Ok(TransactionType::SwapTokenToSol {
                    token_mint: "Unknown".to_string(),
                    token_amount: 0.0,
                    sol_amount: transaction.sol_balance_change.abs(),
                    router: "Orca".to_string(),
                });
            } else if !transaction.token_transfers.is_empty() {
                return Ok(TransactionType::SwapTokenToToken {
                    from_mint: "Unknown".to_string(),
                    to_mint: "Unknown".to_string(),
                    from_amount: 0.0,
                    to_amount: 0.0,
                    router: "Orca".to_string(),
                });
            }
        }

        Err("Not an Orca swap".to_string())
    }

    /// Analyze generic DEX swap transactions (Meteora, Aldrin, etc.)
    async fn analyze_generic_dex_swap(
        &self,
        transaction: &Transaction
    ) -> Result<TransactionType, String> {
        let log_text = transaction.log_messages.join(" ");

        // Check for common swap indicators
        if
            log_text.contains("swap") ||
            log_text.contains("Swap") ||
            log_text.contains("exchange") ||
            log_text.contains("trade")
        {
            // Identify DEX by program IDs
            let router = if
                transaction.instructions.iter().any(|i| i.program_id.contains("meteor"))
            {
                "Meteora"
            } else if transaction.instructions.iter().any(|i| i.program_id.contains("aldrin")) {
                "Aldrin"
            } else if transaction.instructions.iter().any(|i| i.program_id.contains("saber")) {
                "Saber"
            } else {
                "Unknown DEX"
            };

            let has_wsol = log_text.contains("So11111111111111111111111111111111111111112");

            if has_wsol && transaction.sol_balance_change < 0.0 {
                return Ok(TransactionType::SwapSolToToken {
                    token_mint: "Unknown".to_string(),
                    sol_amount: transaction.sol_balance_change.abs(),
                    token_amount: 0.0,
                    router: router.to_string(),
                });
            } else if has_wsol && transaction.sol_balance_change > 0.0 {
                return Ok(TransactionType::SwapTokenToSol {
                    token_mint: "Unknown".to_string(),
                    token_amount: 0.0,
                    sol_amount: transaction.sol_balance_change.abs(),
                    router: router.to_string(),
                });
            } else if !transaction.token_transfers.is_empty() {
                return Ok(TransactionType::SwapTokenToToken {
                    from_mint: "Unknown".to_string(),
                    to_mint: "Unknown".to_string(),
                    from_amount: 0.0,
                    to_amount: 0.0,
                    router: router.to_string(),
                });
            }
        }

        Err("Not a generic DEX swap".to_string())
    }

    /// Analyze ATA operations and calculate rent amounts
    async fn analyze_ata_operations(&self, transaction: &Transaction) -> Result<f64, String> {
        let mut total_ata_rent = 0.0;

        // Look for ATA account closures and creations in pre/post balances
        if let Some(raw_data) = &transaction.raw_transaction_data {
            if let Some(meta) = raw_data.get("meta") {
                if let Some(pre_balances) = meta.get("preBalances").and_then(|v| v.as_array()) {
                    if
                        let Some(post_balances) = meta
                            .get("postBalances")
                            .and_then(|v| v.as_array())
                    {
                        // Compare pre and post balances to detect ATA rent flows
                        for (index, (pre, post)) in pre_balances
                            .iter()
                            .zip(post_balances.iter())
                            .enumerate() {
                            if let (Some(pre_val), Some(post_val)) = (pre.as_u64(), post.as_u64()) {
                                let change = (post_val as i64) - (pre_val as i64);

                                // Check if this is an ATA account by looking at the change amount
                                // Standard ATA rent is 2039280 lamports (0.00203928 SOL)
                                // Also check for partial ATA rent amounts
                                if change.abs() >= 1000000 && change.abs() <= 3000000 {
                                    // Check if this involves CloseAccount instructions
                                    let has_close_account = transaction.log_messages
                                        .iter()
                                        .any(|log| log.contains("Instruction: CloseAccount"));

                                    if has_close_account {
                                        // If an account went from having balance to 0, it's likely ATA closure
                                        if pre_val > 1000000 && post_val == 0 {
                                            total_ata_rent += lamports_to_sol(pre_val);
                                            if self.debug_enabled {
                                                log(
                                                    LogTag::Transactions,
                                                    "ATA_RENT",
                                                    &format!(
                                                        "Detected ATA closure rent refund: {} lamports ({:.9} SOL)",
                                                        pre_val,
                                                        lamports_to_sol(pre_val)
                                                    )
                                                );
                                            }
                                        } else if
                                            // If account went from 0 to some amount and then back, it's temporary ATA
                                            pre_val == 0 &&
                                            post_val == 0
                                        {
                                            // Check if this account was created and closed in the same transaction
                                            // by looking for both CreateAccount and CloseAccount patterns
                                            let has_create_account = transaction.log_messages
                                                .iter()
                                                .any(|log| {
                                                    log.contains("createAccount") ||
                                                        log.contains("CreateIdempotent")
                                                });

                                            if has_create_account {
                                                // Estimate typical ATA rent for temporary accounts
                                                total_ata_rent += 0.00203928; // Standard ATA rent
                                                if self.debug_enabled {
                                                    log(
                                                        LogTag::Transactions,
                                                        "ATA_RENT",
                                                        "Detected temporary ATA creation/closure: 0.00203928 SOL"
                                                    );
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(total_ata_rent)
    }

    /// Analyze NFT operations (DISABLED - no longer detected)
    async fn analyze_nft_operations(
        &self,
        _transaction: &Transaction
    ) -> Result<TransactionType, String> {
        Err("NFT operations no longer detected".to_string())
    }

    /// Analyze wrapped SOL operations
    async fn analyze_wsol_operations(
        &self,
        transaction: &Transaction
    ) -> Result<TransactionType, String> {
        let log_text = transaction.log_messages.join(" ");
        let wsol_mint = "So11111111111111111111111111111111111111112";

        // Check for WSOL wrapping (SOL -> WSOL)
        if log_text.contains(wsol_mint) && transaction.sol_balance_change < 0.0 {
            // Look for token account creation and transfer to WSOL account
            if transaction.instructions.iter().any(|i| i.instruction_type == "transfer") {
                return Ok(TransactionType::SwapSolToToken {
                    token_mint: wsol_mint.to_string(),
                    sol_amount: transaction.sol_balance_change.abs(),
                    token_amount: transaction.sol_balance_change.abs(), // 1:1 ratio for WSOL
                    router: "Native WSOL".to_string(),
                });
            }
        }

        // Check for WSOL unwrapping (WSOL -> SOL)
        if log_text.contains(wsol_mint) && transaction.sol_balance_change > 0.0 {
            if transaction.instructions.iter().any(|i| i.instruction_type == "closeAccount") {
                return Ok(TransactionType::SwapTokenToSol {
                    token_mint: wsol_mint.to_string(),
                    token_amount: transaction.sol_balance_change.abs(),
                    sol_amount: transaction.sol_balance_change.abs(), // 1:1 ratio for WSOL
                    router: "Native WSOL".to_string(),
                });
            }
        }

        Err("No WSOL operation detected".to_string())
    }

    /// Analyze Pump.fun swap operations
    async fn analyze_pump_fun_swap(
        &self,
        transaction: &Transaction
    ) -> Result<TransactionType, String> {
        let log_text = transaction.log_messages.join(" ");

        if self.debug_enabled {
            log(
                LogTag::Transactions,
                "PUMP_ANALYSIS",
                &format!("{} - Analyzing Pump.fun swap", &transaction.signature[..8])
            );
        }

        // Extract token mint from Pump.fun transaction
        let target_token_mint = self.extract_target_token_mint_from_pumpfun(transaction).await;

        // Check for Pump.fun specific patterns - both program IDs and logs
        let has_pumpfun_program =
            log_text.contains("6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P") ||
            log_text.contains("pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA") ||
            transaction.instructions
                .iter()
                .any(|i| {
                    i.program_id == "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P" ||
                        i.program_id == "pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA"
                });

        let has_buy_instruction = log_text.contains("Instruction: Buy");
        let has_sell_instruction = log_text.contains("Instruction: Sell");

        if has_pumpfun_program {
            // Extract actual amounts from transaction data
            let sol_amount = self.extract_sol_amount_from_pumpfun(transaction).await;
            let token_amount = self.extract_token_amount_from_pumpfun(transaction).await;

            // Determine direction based on instruction patterns and balance changes
            // Note: sol_amount is always positive (abs value), so we use sol_balance_change for direction
            if has_buy_instruction || transaction.sol_balance_change < -0.000001 {
                // SOL to Token (Buy) - SOL was spent (negative balance change)
                return Ok(TransactionType::SwapSolToToken {
                    token_mint: target_token_mint.unwrap_or_else(|| "Pump.fun_Token".to_string()),
                    sol_amount: sol_amount, // Use extracted amount (excludes ATA rent)
                    token_amount: token_amount,
                    router: "Pump.fun".to_string(),
                });
            } else if has_sell_instruction || transaction.sol_balance_change > 0.000001 {
                // Token to SOL (Sell) - SOL was received (positive balance change)
                return Ok(TransactionType::SwapTokenToSol {
                    token_mint: target_token_mint.unwrap_or_else(|| "Pump.fun_Token".to_string()),
                    token_amount: token_amount,
                    sol_amount: sol_amount, // Use extracted amount (excludes ATA rent)
                    router: "Pump.fun".to_string(),
                });
            } else {
                // Fallback: if we have Pump.fun program but unclear direction, use balance change
                if transaction.sol_balance_change.abs() > 0.000001 {
                    if transaction.sol_balance_change < 0.0 {
                        // SOL spent = Buy
                        return Ok(TransactionType::SwapSolToToken {
                            token_mint: target_token_mint.unwrap_or_else(||
                                "Pump.fun_Token".to_string()
                            ),
                            sol_amount: sol_amount, // Use extracted amount (excludes ATA rent)
                            token_amount: token_amount,
                            router: "Pump.fun".to_string(),
                        });
                    } else {
                        // SOL received = Sell
                        return Ok(TransactionType::SwapTokenToSol {
                            token_mint: target_token_mint.unwrap_or_else(||
                                "Pump.fun_Token".to_string()
                            ),
                            token_amount: token_amount,
                            sol_amount: sol_amount, // Use extracted amount (excludes ATA rent)
                            router: "Pump.fun".to_string(),
                        });
                    }
                }
                // Final fallback - unclear transaction, return error instead of defaulting to buy
                return Err("Cannot determine Pump.fun swap direction".to_string());
            }
        }

        Err("No Pump.fun swap pattern found".to_string())
    }

    /// Extract target token mint from Pump.fun transaction
    async fn extract_target_token_mint_from_pumpfun(
        &self,
        transaction: &Transaction
    ) -> Option<String> {
        // Look for token account creation or transfer instructions for non-WSOL tokens
        if let Some(raw_data) = &transaction.raw_transaction_data {
            if let Some(meta) = raw_data.get("meta") {
                if
                    let Some(inner_instructions) = meta
                        .get("innerInstructions")
                        .and_then(|v| v.as_array())
                {
                    for inner_group in inner_instructions {
                        if
                            let Some(instructions) = inner_group
                                .get("instructions")
                                .and_then(|v| v.as_array())
                        {
                            for instruction in instructions {
                                if let Some(parsed) = instruction.get("parsed") {
                                    if let Some(info) = parsed.get("info") {
                                        if
                                            let Some(mint) = info
                                                .get("mint")
                                                .and_then(|v| v.as_str())
                                        {
                                            if
                                                mint !=
                                                "So11111111111111111111111111111111111111112"
                                            {
                                                return Some(mint.to_string());
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        None
    }

    /// Extract SOL amount from Pump.fun transaction
    async fn extract_sol_amount_from_pumpfun(&self, transaction: &Transaction) -> f64 {
        // Sum all WSOL transferChecked uiAmounts found in inner instructions (covers splits to fees/referrals)
        if let Some(raw_data) = &transaction.raw_transaction_data {
            if let Some(meta) = raw_data.get("meta") {
                if
                    let Some(inner_instructions) = meta
                        .get("innerInstructions")
                        .and_then(|v| v.as_array())
                {
                    let mut wsol_sum = 0.0f64;
                    for inner_group in inner_instructions {
                        if
                            let Some(instructions) = inner_group
                                .get("instructions")
                                .and_then(|v| v.as_array())
                        {
                            for instruction in instructions {
                                if let Some(parsed) = instruction.get("parsed") {
                                    if let Some(info) = parsed.get("info") {
                                        if
                                            let (Some(mint), Some(token_amount)) = (
                                                info.get("mint").and_then(|v| v.as_str()),
                                                info.get("tokenAmount"),
                                            )
                                        {
                                            if
                                                mint ==
                                                "So11111111111111111111111111111111111111112"
                                            {
                                                if
                                                    let Some(ui_amount) = token_amount
                                                        .get("uiAmount")
                                                        .and_then(|v| v.as_f64())
                                                {
                                                    // Include even micro amounts; they'll round correctly in display
                                                    if ui_amount > 0.0 {
                                                        wsol_sum += ui_amount;
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    if wsol_sum > 0.0 {
                        return wsol_sum;
                    }
                }
            }
        }

        // Calculate ATA rent to exclude from balance change as a fallback
        let ata_rent = self.analyze_ata_operations(transaction).await.unwrap_or(0.0);

        // Use balance change minus ATA rent as fallback
        let adjusted_balance_change = transaction.sol_balance_change.abs() - ata_rent;

        if self.debug_enabled && ata_rent > 0.0 {
            log(
                LogTag::Transactions,
                "SOL_EXTRACT",
                &format!(
                    "Excluding ATA rent: {:.9} SOL from balance change {:.9} SOL",
                    ata_rent,
                    transaction.sol_balance_change.abs()
                )
            );
        }

        // Return the adjusted amount, ensuring it's not negative
        adjusted_balance_change.max(0.0)
    }

    /// Extract token amount from Pump.fun transaction
    async fn extract_token_amount_from_pumpfun(&self, transaction: &Transaction) -> f64 {
        // Look for token transfer amounts in inner instructions
        if let Some(raw_data) = &transaction.raw_transaction_data {
            if let Some(meta) = raw_data.get("meta") {
                if
                    let Some(inner_instructions) = meta
                        .get("innerInstructions")
                        .and_then(|v| v.as_array())
                {
                    for inner_group in inner_instructions {
                        if
                            let Some(instructions) = inner_group
                                .get("instructions")
                                .and_then(|v| v.as_array())
                        {
                            for instruction in instructions {
                                if let Some(parsed) = instruction.get("parsed") {
                                    if let Some(info) = parsed.get("info") {
                                        if let Some(token_amount) = info.get("tokenAmount") {
                                            if
                                                let Some(ui_amount) = token_amount
                                                    .get("uiAmount")
                                                    .and_then(|v| v.as_f64())
                                            {
                                                if ui_amount > 0.0 {
                                                    return ui_amount;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Fallback to token_transfers if available
        if !transaction.token_transfers.is_empty() {
            return transaction.token_transfers[0].amount;
        }
        0.0
    }

    /// Analyze Serum/OpenBook swap operations
    async fn analyze_serum_swap(
        &self,
        transaction: &Transaction
    ) -> Result<TransactionType, String> {
        let log_text = transaction.log_messages.join(" ");

        if self.debug_enabled {
            log(
                LogTag::Transactions,
                "SERUM_ANALYSIS",
                &format!("{} - Analyzing Serum/OpenBook swap", &transaction.signature[..8])
            );
        }

        // Check for Serum specific patterns
        if log_text.contains("9xQeWvG816bUx9EPjHmaT23yvVM2ZWbrrpZb9PusVFin") {
            // Determine direction based on SOL balance change
            if transaction.sol_balance_change < -0.001 {
                // SOL to Token (Buy)
                return Ok(TransactionType::SwapSolToToken {
                    token_mint: "Serum_Token".to_string(),
                    sol_amount: transaction.sol_balance_change.abs(),
                    token_amount: 0.0,
                    router: "Serum/OpenBook".to_string(),
                });
            } else if transaction.sol_balance_change > 0.001 {
                // Token to SOL (Sell)
                return Ok(TransactionType::SwapTokenToSol {
                    token_mint: "Serum_Token".to_string(),
                    token_amount: 0.0,
                    sol_amount: transaction.sol_balance_change,
                    router: "Serum/OpenBook".to_string(),
                });
            }
        }

        Err("No Serum/OpenBook swap pattern found".to_string())
    }

    /// Extract SOL transfer data
    async fn extract_sol_transfer_data(
        &self,
        transaction: &Transaction
    ) -> Result<TransactionType, String> {
        // Only detect simple SOL transfers with very specific criteria:
        // 1. Must be 1-3 instructions maximum (simple transfers)
        // 2. Must have meaningful SOL amount change (not just fees)
        // 3. Must be primarily system program transfers

        if transaction.instructions.len() > 3 {
            return Err("Too many instructions for simple SOL transfer".to_string());
        }

        // Check if SOL amount change is meaningful (more than just transaction fees)
        if transaction.sol_balance_change.abs() < 0.0001 {
            return Err("SOL amount too small for meaningful transfer".to_string());
        }

        // Check if it's primarily system program transfers
        let system_transfer_count = transaction.instructions
            .iter()
            .filter(|i| {
                i.program_id == "11111111111111111111111111111111" &&
                    i.instruction_type == "transfer"
            })
            .count();

        // Must have at least one system transfer and it should be the majority of instructions
        if system_transfer_count == 0 || system_transfer_count < transaction.instructions.len() / 2 {
            return Err("Not primarily system program transfers".to_string());
        }

        Ok(TransactionType::SolTransfer {
            amount: transaction.sol_balance_change.abs(),
            from: "wallet".to_string(),
            to: "destination".to_string(),
        })
    }

    /// Extract ATA close operation data (standalone ATA closures)
    async fn extract_ata_close_data(
        &self,
        transaction: &Transaction
    ) -> Result<TransactionType, String> {
        // Check for single closeAccount instruction
        if transaction.instructions.len() != 1 {
            return Err("Not a single instruction transaction".to_string());
        }

        let instruction = &transaction.instructions[0];

        // Check if it's a Token Program (original or Token-2022) closeAccount instruction
        if
            (instruction.program_id == "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA" ||
                instruction.program_id == "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb") &&
            instruction.instruction_type == "closeAccount"
        {
            // Check if SOL balance increased (ATA rent recovery)
            if transaction.sol_balance_change > 0.0 {
                // Try to extract token mint from ATA closure
                let token_mint = self
                    .extract_token_mint_from_ata_close(transaction)
                    .unwrap_or_else(|| "Unknown".to_string());

                return Ok(TransactionType::AtaClose {
                    recovered_sol: transaction.sol_balance_change,
                    token_mint,
                });
            }
        }

        Err("No ATA close pattern found".to_string())
    }

    /// Extract token mint from ATA close operation
    fn extract_token_mint_from_ata_close(&self, transaction: &Transaction) -> Option<String> {
        // Look for token balance changes to identify the mint
        if !transaction.token_balance_changes.is_empty() {
            return Some(transaction.token_balance_changes[0].mint.clone());
        }

        // If no token balance changes, check log messages for mint information
        let log_text = transaction.log_messages.join(" ");
        if let Some(start) = log_text.find("mint: ") {
            let mint_start = start + 6;
            if let Some(end) = log_text[mint_start..].find(' ') {
                return Some(log_text[mint_start..mint_start + end].to_string());
            }
        }

        None
    }

    /// Extract bulk operation data (spam detection) - DISABLED
    async fn extract_bulk_operation_data(
        &self,
        _transaction: &Transaction
    ) -> Result<TransactionType, String> {
        Err(
            "Bulk operation detection disabled - only core transaction types are detected".to_string()
        )
    }

    // =============================================================================
    // MISSING ANALYSIS FUNCTIONS - COMPREHENSIVE SWAP DETECTION
    // =============================================================================

    /// Detect Jupiter swap transactions
    async fn detect_jupiter_swap(
        &self,
        transaction: &Transaction
    ) -> Result<Option<TransactionType>, String> {
        let jupiter_program_id = "JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4";

        // Check if transaction involves Jupiter
        if
            !transaction.instructions.iter().any(|i| i.program_id == jupiter_program_id) &&
            !transaction.log_messages.iter().any(|log| log.contains(jupiter_program_id))
        {
            return Ok(None);
        }

        // Analyze Jupiter swap pattern
        self.analyze_jupiter_swap(transaction).await.map(Some)
    }

    /// Detect Raydium swap transactions
    async fn detect_raydium_swap(
        &self,
        transaction: &Transaction
    ) -> Result<Option<TransactionType>, String> {
        let raydium_program_ids = [
            "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8", // Raydium AMM
            "routeUGWgWzqBWFcrCfv8tritsqukccJPu3q5GPP3xS", // Raydium Router
        ];

        // Check if transaction involves Raydium
        if
            !transaction.instructions
                .iter()
                .any(|i| raydium_program_ids.contains(&i.program_id.as_str())) &&
            !transaction.log_messages
                .iter()
                .any(|log| raydium_program_ids.iter().any(|id| log.contains(id)))
        {
            return Ok(None);
        }

        // Analyze Raydium swap pattern
        self.analyze_raydium_swap(transaction).await.map(Some)
    }

    /// Detect Orca swap transactions
    async fn detect_orca_swap(
        &self,
        transaction: &Transaction
    ) -> Result<Option<TransactionType>, String> {
        let orca_program_ids = [
            "9W959DqEETiGZocYWCQPaJ6sBmUzgfxXfqGeTEdp3aQP", // Orca V1
            "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc", // Orca Whirlpool
        ];

        // Check if transaction involves Orca
        if
            !transaction.instructions
                .iter()
                .any(|i| orca_program_ids.contains(&i.program_id.as_str())) &&
            !transaction.log_messages
                .iter()
                .any(|log| orca_program_ids.iter().any(|id| log.contains(id)))
        {
            return Ok(None);
        }

        // Analyze Orca swap pattern
        self.analyze_orca_swap(transaction).await.map(Some)
    }

    /// Detect Serum/OpenBook swap transactions
    async fn detect_serum_swap(
        &self,
        transaction: &Transaction
    ) -> Result<Option<TransactionType>, String> {
        let serum_program_ids = [
            "9xQeWvG816bUx9EPjHmaT23yvVM2ZWbrrpZb9PusVFin", // Serum DEX
            "srmqPiDkJMShKEGHHJG3w4dWnGr5Hge6F3H5HKpVYuN", // Serum V3
        ];

        // Check if transaction involves Serum
        if
            !transaction.instructions
                .iter()
                .any(|i| serum_program_ids.contains(&i.program_id.as_str())) &&
            !transaction.log_messages
                .iter()
                .any(|log| serum_program_ids.iter().any(|id| log.contains(id)))
        {
            return Ok(None);
        }

        // Analyze Serum swap pattern
        self.analyze_serum_swap(transaction).await.map(Some)
    }

    /// Detect Pump.fun swap transactions
    async fn detect_pump_fun_swap(
        &self,
        transaction: &Transaction
    ) -> Result<Option<TransactionType>, String> {
        let pump_program_id = "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P";

        // Check if transaction involves Pump.fun
        if
            !transaction.instructions.iter().any(|i| i.program_id == pump_program_id) &&
            !transaction.log_messages.iter().any(|log| log.contains(pump_program_id))
        {
            return Ok(None);
        }

        // Analyze Pump.fun swap pattern
        self.analyze_pump_fun_swap(transaction).await.map(Some)
    }

    /// Detect SOL transfer transactions
    async fn detect_sol_transfer(
        &self,
        transaction: &Transaction
    ) -> Result<Option<TransactionType>, String> {
        // Look for system program transfers
        let system_program_id = "11111111111111111111111111111111";

        for instruction in &transaction.instructions {
            if
                instruction.program_id == system_program_id &&
                instruction.instruction_type.contains("transfer")
            {
                return self.extract_sol_transfer_data(transaction).await.map(Some);
            }
        }

        Ok(None)
    }

    /// Detect token transfer transactions
    async fn detect_token_transfer(
        &self,
        transaction: &Transaction
    ) -> Result<Option<TransactionType>, String> {
        let token_program_id = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";

        for instruction in &transaction.instructions {
            if
                instruction.program_id == token_program_id &&
                instruction.instruction_type.contains("transfer")
            {
                return self.extract_token_transfer_data(transaction).await.map(Some);
            }
        }

        Ok(None)
    }

    /// Detect ATA operations (creation/closure) - DISABLED
    async fn detect_ata_operations(
        &self,
        _transaction: &Transaction
    ) -> Result<Option<TransactionType>, String> {
        Ok(None)
    }

    /// Detect staking operations - DISABLED
    async fn detect_staking_operations(
        &self,
        _transaction: &Transaction
    ) -> Result<Option<TransactionType>, String> {
        Ok(None)
    }

    /// Detect spam/bulk transactions - DISABLED
    async fn detect_spam_bulk_transactions(
        &self,
        _transaction: &Transaction
    ) -> Result<Option<TransactionType>, String> {
        Ok(None)
    }

    /// Extract ATA operation data - DISABLED
    async fn extract_ata_operation_data(
        &self,
        _transaction: &Transaction
    ) -> Result<TransactionType, String> {
        Err("ATA operations no longer detected as transaction types".to_string())
    }

    /// Extract staking operation data - DISABLED
    async fn extract_staking_operation_data(
        &self,
        _transaction: &Transaction
    ) -> Result<TransactionType, String> {
        Err("Staking operations no longer detected as transaction types".to_string())
    }

    /// Extract token transfer data
    async fn extract_token_transfer_data(
        &self,
        transaction: &Transaction
    ) -> Result<TransactionType, String> {
        if transaction.token_transfers.is_empty() {
            return Err("No token transfer found".to_string());
        }

        let wallet = self.wallet_pubkey.to_string();
        let wsol_mint = "So11111111111111111111111111111111111111112";

        // 1) Prefer transfers involving the wallet (sender or recipient)
        let mut candidates: Vec<&TokenTransfer> = transaction.token_transfers
            .iter()
            .filter(|t| (t.from == wallet || t.to == wallet))
            .collect();

        // 2) Exclude WSOL mint for generic token transfer detection (it's usually part of swaps)
        let mut non_wsol: Vec<&TokenTransfer> = candidates
            .iter()
            .copied()
            .filter(|t| t.mint != wsol_mint)
            .collect();

        if non_wsol.is_empty() {
            // If all involve WSOL fall back to original candidates
            non_wsol = candidates.clone();
        }

        // 3) If still none (wallet not directly in transfers), fall back to all non-WSOL transfers
        if non_wsol.is_empty() {
            non_wsol = transaction.token_transfers
                .iter()
                .filter(|t| t.mint != wsol_mint)
                .collect();
        }

        // 4) Final fallback: all transfers
        if non_wsol.is_empty() {
            non_wsol = transaction.token_transfers.iter().collect();
        }

        // Choose the transfer with the largest absolute amount (UI amount already normalized)
        if
            let Some(best) = non_wsol
                .into_iter()
                .max_by(|a, b| {
                    a.amount.partial_cmp(&b.amount).unwrap_or(std::cmp::Ordering::Equal)
                })
        {
            if self.debug_enabled && transaction.token_transfers.len() > 1 {
                log(
                    LogTag::Transactions,
                    "TOKEN_TRANSFER_SELECT",
                    &format!(
                        "{} selected mint={} amount={} from={} to={} among {} transfers",
                        &transaction.signature[..8],
                        &best.mint,
                        best.amount,
                        &best.from,
                        &best.to,
                        transaction.token_transfers.len()
                    )
                );
            }
            return Ok(TransactionType::TokenTransfer {
                mint: best.mint.clone(),
                amount: best.amount,
                from: best.from.clone(),
                to: best.to.clone(),
            });
        }

        Err("Failed to select token transfer".to_string())
    }

    /// Extract router from transaction
    fn extract_router_from_transaction(&self, transaction: &Transaction) -> String {
        match &transaction.transaction_type {
            TransactionType::SwapSolToToken { router, .. } => router.clone(),
            TransactionType::SwapTokenToSol { router, .. } => router.clone(),
            TransactionType::SwapTokenToToken { router, .. } => router.clone(),
            _ => "Unknown".to_string(),
        }
    }

    /// Extract input token
    fn extract_input_token(&self, transaction: &Transaction) -> String {
        match &transaction.transaction_type {
            TransactionType::SwapSolToToken { .. } => "SOL".to_string(),
            TransactionType::SwapTokenToSol { token_mint, .. } => token_mint.clone(),
            _ => "Unknown".to_string(),
        }
    }

    /// Extract output token
    fn extract_output_token(&self, transaction: &Transaction) -> String {
        match &transaction.transaction_type {
            TransactionType::SwapSolToToken { token_mint, .. } => token_mint.clone(),
            TransactionType::SwapTokenToSol { .. } => "SOL".to_string(),
            _ => "Unknown".to_string(),
        }
    }

    /// Extract token mint from transaction
    pub fn extract_token_mint_from_transaction(&self, transaction: &Transaction) -> Option<String> {
        match &transaction.transaction_type {
            TransactionType::SwapSolToToken { token_mint, .. } => Some(token_mint.clone()),
            TransactionType::SwapTokenToSol { token_mint, .. } => Some(token_mint.clone()),
            TransactionType::SwapTokenToToken { to_mint, .. } => Some(to_mint.clone()),
            TransactionType::AtaClose { token_mint, .. } => Some(token_mint.clone()),
            _ => None,
        }
    }

    /// Force recalculation of transaction analysis (for priority/fallback requests)
    pub async fn force_recalculate_analysis(
        &mut self,
        transaction: &mut Transaction
    ) -> Result<(), String> {
        // Validate transaction is ready for analysis
        if
            !matches!(
                transaction.status,
                TransactionStatus::Confirmed | TransactionStatus::Finalized
            )
        {
            return Err(
                format!(
                    "Transaction {} not confirmed - status: {:?}",
                    &transaction.signature,
                    transaction.status
                )
            );
        }

        if !transaction.success {
            return Err(format!("Transaction {} failed - cannot analyze", &transaction.signature));
        }

        if transaction.log_messages.is_empty() {
            return Err(
                format!(
                    "Transaction {} has no log messages - cannot analyze",
                    &transaction.signature
                )
            );
        }

        log(
            LogTag::Transactions,
            "FORCE_ANALYSIS",
            &format!(
                "Force recalculating analysis for {} (confirmed, successful, {} logs)",
                &transaction.signature,
                transaction.log_messages.len()
            )
        );

        // Ensure transaction type is properly set
        if matches!(transaction.transaction_type, TransactionType::Unknown) {
            // Need to re-analyze raw transaction data to classify type
            if transaction.raw_transaction_data.is_some() {
                // Re-run classification logic here (simplified)
                self.classify_transaction_from_raw_data(
                    transaction,
                    &serde_json::Value::Null
                ).await?;
            }
        }

        // Force swap analysis recalculation if this is a swap
        if self.is_swap_transaction(transaction) {
            // Use the existing recalculate_transaction_analysis method
            self.recalculate_transaction_analysis(transaction).await?;
        }

        // ATA analysis is included in recalculate_transaction_analysis

        // Update cached analysis
        transaction.cached_analysis = Some(CachedAnalysis::from_transaction(transaction));

        log(
            LogTag::Transactions,
            "FORCE_ANALYSIS_COMPLETE",
            &format!(
                "Completed force analysis for {} - type: {:?}, sol_change: {:.9}",
                &transaction.signature,
                transaction.transaction_type,
                transaction.sol_balance_change
            )
        );

        Ok(())
    }

    /// Classify transaction from raw data (helper for force analysis)
    async fn classify_transaction_from_raw_data(
        &self,
        transaction: &mut Transaction,
        raw_data: &serde_json::Value
    ) -> Result<(), String> {
        if self.debug_enabled {
            log(
                LogTag::Transactions,
                "FORCE_CLASSIFY",
                &format!("Force classifying transaction type for {}", &transaction.signature[..8])
            );
        }

        // Use the existing transaction classification logic
        self.analyze_transaction_type(transaction).await?;

        // If still unknown, try more aggressive classification based on patterns
        if matches!(transaction.transaction_type, TransactionType::Unknown) {
            // Check for GMGN patterns more aggressively in force analysis
            let log_text = transaction.log_messages.join(" ");

            // Force GMGN detection if we have token operations and SOL changes
            if
                transaction.sol_balance_change.abs() > 0.001 &&
                (log_text.contains("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL") ||
                    log_text.contains("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA") ||
                    transaction.instructions
                        .iter()
                        .any(|i| {
                            i.program_id.starts_with("ATokenGP") ||
                                i.program_id.starts_with("Tokenkeg")
                        }))
            {
                log(
                    LogTag::Transactions,
                    "FORCE_GMGN_DETECT",
                    &format!("{} - Force detecting GMGN swap pattern", &transaction.signature[..8])
                );

                if let Ok(swap_type) = self.analyze_gmgn_swap(transaction).await {
                    transaction.transaction_type = swap_type;
                }
            }
        }

        Ok(())
    }

    /// Integrate token information from tokens module
    async fn integrate_token_information(
        &mut self,
        transaction: &mut Transaction
    ) -> Result<(), String> {
        let token_mint = match self.extract_token_mint_from_transaction(transaction) {
            Some(mint) => mint,
            None => {
                return Ok(());
            } // No token involved
        };

        if self.debug_enabled {
            log(
                LogTag::Transactions,
                "TOKEN_INFO",
                &format!("Integrating token info for mint: {}", &token_mint[..8])
            );
        }

        // Get token decimals
        let decimals = get_token_decimals(&token_mint).await.unwrap_or(9);
        transaction.token_decimals = Some(decimals);

        // Get token symbol from database
        let symbol = if let Some(ref db) = self.token_database {
            match db.get_token_by_mint(&token_mint) {
                Ok(Some(token_info)) => token_info.symbol,
                _ => format!("TOKEN_{}", &token_mint[..8]),
            }
        } else {
            format!("TOKEN_{}", &token_mint[..8])
        };
        transaction.token_symbol = Some(symbol.clone());

        // Get current market price from price service
        match get_pool_service().await.get_price(&token_mint).await {
            Some(price_info) => {
                if let Some(price_sol) = price_info.pool_price_sol.or(price_info.api_price_sol) {
                    transaction.calculated_token_price_sol = Some(price_sol);
                    transaction.price_source = Some(PriceSourceType::CachedPrice);

                    if self.debug_enabled {
                        log(
                            LogTag::Transactions,
                            "PRICE",
                            &format!("Market price for {}: {:.12} SOL", symbol, price_sol)
                        );
                    }
                }
            }
            None => {
                log(
                    LogTag::Transactions,
                    "WARN",
                    &format!("Failed to get market price for {}", symbol)
                );
            }
        }

        // Create TokenSwapInfo
        transaction.token_info = Some(TokenSwapInfo {
            mint: token_mint,
            symbol: symbol.clone(),
            decimals,
            current_price_sol: transaction.calculated_token_price_sol,
            price_source: transaction.price_source.clone(),
            is_verified: transaction.success,
        });

        Ok(())
    }

    /// Quick transaction type detection for filtering
    pub fn is_swap_transaction(&self, transaction: &Transaction) -> bool {
        matches!(
            transaction.transaction_type,
            TransactionType::SwapSolToToken { .. } |
                TransactionType::SwapTokenToSol { .. } |
                TransactionType::SwapTokenToToken { .. }
        )
    }

    /// Check if transaction involves specific token
    pub fn involves_token(&self, transaction: &Transaction, token_mint: &str) -> bool {
        match &transaction.transaction_type {
            | TransactionType::SwapSolToToken { token_mint: mint, .. }
            | TransactionType::SwapTokenToSol { token_mint: mint, .. } => mint == token_mint,
            TransactionType::SwapTokenToToken { from_mint, to_mint, .. } =>
                from_mint == token_mint || to_mint == token_mint,
            TransactionType::TokenTransfer { mint, .. } => mint == token_mint,
            TransactionType::AtaClose { token_mint: mint, .. } => mint == token_mint,
            _ => false,
        }
    }

    /// Extract basic transaction information (slot, time, fee, success)
    pub async fn extract_basic_transaction_info(
        &self,
        transaction: &mut Transaction
    ) -> Result<(), String> {
        if let Some(raw_data) = &transaction.raw_transaction_data {
            // Extract slot directly from the transaction details
            if let Some(slot) = raw_data.get("slot").and_then(|v| v.as_u64()) {
                transaction.slot = Some(slot);
            }

            // Extract block time
            if let Some(block_time) = raw_data.get("blockTime").and_then(|v| v.as_i64()) {
                transaction.block_time = Some(block_time);
                // Update timestamp to use blockchain time instead of processing time
                transaction.timestamp = DateTime::<Utc>
                    ::from_timestamp(block_time, 0)
                    .unwrap_or(transaction.timestamp);
            }

            // Extract meta information
            if let Some(meta) = raw_data.get("meta") {
                // Extract fee
                if let Some(fee) = meta.get("fee").and_then(|v| v.as_u64()) {
                    transaction.fee_sol = lamports_to_sol(fee); // Convert lamports to SOL
                }

                // Calculate SOL balance change from pre/post balances (signed!)
                if
                    let (Some(pre_balances), Some(post_balances)) = (
                        meta.get("preBalances").and_then(|v| v.as_array()),
                        meta.get("postBalances").and_then(|v| v.as_array()),
                    )
                {
                    if !pre_balances.is_empty() && !post_balances.is_empty() {
                        // First account is always the main wallet account
                        let pre_balance_lamports = pre_balances[0].as_i64().unwrap_or(0);
                        let post_balance_lamports = post_balances[0].as_i64().unwrap_or(0);

                        // Signed change in lamports and convert to SOL
                        let balance_change_lamports: i64 =
                            post_balance_lamports - pre_balance_lamports;
                        transaction.sol_balance_change =
                            (balance_change_lamports as f64) / 1_000_000_000.0;

                        if self.debug_enabled {
                            log(
                                LogTag::Transactions,
                                "BALANCE",
                                &format!(
                                    "SOL balance change for {}: {} lamports ({:.9} SOL)",
                                    &transaction.signature[..8],
                                    balance_change_lamports,
                                    transaction.sol_balance_change
                                )
                            );
                        }
                    }
                }

                // Extract token balance changes from pre/post token balances
                if
                    let (Some(pre_token_balances), Some(post_token_balances)) = (
                        meta.get("preTokenBalances").and_then(|v| v.as_array()),
                        meta.get("postTokenBalances").and_then(|v| v.as_array()),
                    )
                {
                    let wallet_str = self.wallet_pubkey.to_string();

                    // Process token balance changes for wallet-owned accounts
                    for post_balance in post_token_balances {
                        if let Some(owner) = post_balance.get("owner").and_then(|v| v.as_str()) {
                            if owner == wallet_str {
                                if
                                    let Some(account_index) = post_balance
                                        .get("accountIndex")
                                        .and_then(|v| v.as_u64())
                                {
                                    // Find corresponding pre-balance
                                    let pre_balance = pre_token_balances
                                        .iter()
                                        .find(
                                            |pre|
                                                pre.get("accountIndex").and_then(|v| v.as_u64()) ==
                                                Some(account_index)
                                        );

                                    // Extract token balance data
                                    if
                                        let Some(mint) = post_balance
                                            .get("mint")
                                            .and_then(|v| v.as_str())
                                    {
                                        if
                                            let Some(post_ui_token) =
                                                post_balance.get("uiTokenAmount")
                                        {
                                            let decimals = post_ui_token
                                                .get("decimals")
                                                .and_then(|v| v.as_u64())
                                                .unwrap_or(9) as u8;
                                            let post_amount = post_ui_token
                                                .get("uiAmount")
                                                .and_then(|v| v.as_f64())
                                                .unwrap_or(0.0);

                                            let pre_amount = if let Some(pre) = pre_balance {
                                                pre.get("uiTokenAmount")
                                                    .and_then(|ui| ui.get("uiAmount"))
                                                    .and_then(|v| v.as_f64())
                                                    .unwrap_or(0.0)
                                            } else {
                                                0.0 // Account didn't exist before
                                            };

                                            let change = post_amount - pre_amount;

                                            // Only add if there's a significant change (avoid float noise)
                                            if change.abs() > 1e-12 {
                                                transaction.token_balance_changes.push(
                                                    TokenBalanceChange {
                                                        mint: mint.to_string(),
                                                        decimals,
                                                        pre_balance: if pre_balance.is_some() {
                                                            Some(pre_amount)
                                                        } else {
                                                            None
                                                        },
                                                        post_balance: Some(post_amount),
                                                        change,
                                                        usd_value: None, // Will be calculated later if needed
                                                    }
                                                );

                                                // Also populate token_transfers for compatibility
                                                if change != 0.0 {
                                                    transaction.token_transfers.push(TokenTransfer {
                                                        mint: mint.to_string(),
                                                        amount: change.abs(),
                                                        from: if change < 0.0 {
                                                            wallet_str.clone()
                                                        } else {
                                                            "external".to_string()
                                                        },
                                                        to: if change > 0.0 {
                                                            wallet_str.clone()
                                                        } else {
                                                            "external".to_string()
                                                        },
                                                        program_id: "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA".to_string(),
                                                    });
                                                }

                                                if self.debug_enabled {
                                                    log(
                                                        LogTag::Transactions,
                                                        "TOKEN_BALANCE",
                                                        &format!(
                                                            "Token balance change for {}: {} -> {} = {} (mint: {})",
                                                            &transaction.signature[..8],
                                                            pre_amount,
                                                            post_amount,
                                                            change,
                                                            mint
                                                        )
                                                    );
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // Check if transaction succeeded (err field is None or null)
                transaction.success = meta.get("err").map_or(true, |v| v.is_null());

                if let Some(err) = meta.get("err") {
                    // Parse structured blockchain error for comprehensive error handling

                    let structured_error = parse_structured_solana_error(
                        err,
                        Some(&transaction.signature)
                    );

                    // Store detailed error information
                    transaction.error_message = Some(
                        format!(
                            "[{}] {}: {} (code: {})",
                            structured_error.error_type_name(),
                            structured_error.error_name,
                            structured_error.description,
                            structured_error.error_code.map_or("N/A".to_string(), |c| c.to_string())
                        )
                    );

                    // Log permanent failures for immediate attention
                    if is_permanent_failure(&structured_error) {
                        log(
                            LogTag::Transactions,
                            "PERMANENT_FAILURE",
                            &format!(
                                "Transaction {} failed permanently: {} ({})",
                                transaction.signature,
                                structured_error.error_name,
                                structured_error.description
                            )
                        );
                    }
                }

                // Extract log messages for analysis - THIS IS CRITICAL FOR SWAP DETECTION
                if let Some(logs) = meta.get("logMessages").and_then(|v| v.as_array()) {
                    transaction.log_messages = logs
                        .iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect();

                    if self.debug_enabled && !transaction.log_messages.is_empty() {
                        log(
                            LogTag::Transactions,
                            "LOGS",
                            &format!(
                                "Found {} log messages for {}",
                                transaction.log_messages.len(),
                                &transaction.signature[..8]
                            )
                        );
                    }
                }

                // Extract instruction information for program ID detection
                if let Some(transaction_data) = raw_data.get("transaction") {
                    if let Some(message) = transaction_data.get("message") {
                        if
                            let Some(instructions) = message
                                .get("instructions")
                                .and_then(|v| v.as_array())
                        {
                            for (index, instruction) in instructions.iter().enumerate() {
                                // Handle both parsed and raw instruction formats
                                let (program_id_str, instruction_type, accounts) = if
                                    let Some(program_id) = instruction
                                        .get("programId")
                                        .and_then(|v| v.as_str())
                                {
                                    // Parsed instruction format
                                    let instruction_type = if
                                        let Some(parsed) = instruction.get("parsed")
                                    {
                                        if
                                            let Some(type_name) = parsed
                                                .get("type")
                                                .and_then(|v| v.as_str())
                                        {
                                            type_name.to_string()
                                        } else {
                                            "parsed".to_string()
                                        }
                                    } else {
                                        format!("instruction_{}", index)
                                    };

                                    // Extract account information from parsed instruction
                                    let accounts = if let Some(parsed) = instruction.get("parsed") {
                                        if let Some(info) = parsed.get("info") {
                                            let mut acc_list = Vec::new();
                                            // Extract common account fields
                                            if
                                                let Some(source) = info
                                                    .get("source")
                                                    .and_then(|v| v.as_str())
                                            {
                                                acc_list.push(source.to_string());
                                            }
                                            if
                                                let Some(destination) = info
                                                    .get("destination")
                                                    .and_then(|v| v.as_str())
                                            {
                                                acc_list.push(destination.to_string());
                                            }
                                            if
                                                let Some(owner) = info
                                                    .get("owner")
                                                    .and_then(|v| v.as_str())
                                            {
                                                acc_list.push(owner.to_string());
                                            }
                                            if
                                                let Some(mint) = info
                                                    .get("mint")
                                                    .and_then(|v| v.as_str())
                                            {
                                                acc_list.push(mint.to_string());
                                            }
                                            if
                                                let Some(wallet) = info
                                                    .get("wallet")
                                                    .and_then(|v| v.as_str())
                                            {
                                                acc_list.push(wallet.to_string());
                                            }
                                            if
                                                let Some(account) = info
                                                    .get("account")
                                                    .and_then(|v| v.as_str())
                                            {
                                                acc_list.push(account.to_string());
                                            }
                                            if
                                                let Some(authority) = info
                                                    .get("authority")
                                                    .and_then(|v| v.as_str())
                                            {
                                                acc_list.push(authority.to_string());
                                            }
                                            acc_list
                                        } else {
                                            Vec::new()
                                        }
                                    } else {
                                        Vec::new()
                                    };

                                    (program_id.to_string(), instruction_type, accounts)
                                } else if
                                    let Some(program_id_index) = instruction
                                        .get("programIdIndex")
                                        .and_then(|v| v.as_u64())
                                {
                                    // Raw instruction format - need to resolve program_id from account keys
                                    let program_id_str = if
                                        let Some(account_keys) = message
                                            .get("accountKeys")
                                            .and_then(|v| v.as_array())
                                    {
                                        if
                                            let Some(account_obj) = account_keys.get(
                                                program_id_index as usize
                                            )
                                        {
                                            if
                                                let Some(pubkey) = account_obj
                                                    .get("pubkey")
                                                    .and_then(|v| v.as_str())
                                            {
                                                pubkey.to_string()
                                            } else {
                                                "unknown".to_string()
                                            }
                                        } else {
                                            "unknown".to_string()
                                        }
                                    } else {
                                        "unknown".to_string()
                                    };

                                    // Extract accounts from instruction
                                    let accounts = if
                                        let Some(accounts_array) = instruction
                                            .get("accounts")
                                            .and_then(|v| v.as_array())
                                    {
                                        accounts_array
                                            .iter()
                                            .filter_map(|v| v.as_u64())
                                            .filter_map(|idx| {
                                                if
                                                    let Some(account_keys) = message
                                                        .get("accountKeys")
                                                        .and_then(|v| v.as_array())
                                                {
                                                    if
                                                        let Some(account_obj) = account_keys.get(
                                                            idx as usize
                                                        )
                                                    {
                                                        account_obj
                                                            .get("pubkey")
                                                            .and_then(|v| v.as_str())
                                                            .map(|s| s.to_string())
                                                    } else {
                                                        None
                                                    }
                                                } else {
                                                    None
                                                }
                                            })
                                            .collect()
                                    } else {
                                        Vec::new()
                                    };

                                    (program_id_str, format!("instruction_{}", index), accounts)
                                } else {
                                    (
                                        "unknown".to_string(),
                                        format!("instruction_{}", index),
                                        Vec::new(),
                                    )
                                };

                                transaction.instructions.push(InstructionInfo {
                                    program_id: program_id_str,
                                    instruction_type,
                                    accounts,
                                    data: instruction
                                        .get("data")
                                        .and_then(|v| v.as_str())
                                        .map(|s| s.to_string()),
                                });
                            }
                        }

                        if self.debug_enabled && !transaction.instructions.is_empty() {
                            log(
                                LogTag::Transactions,
                                "INSTRUCTIONS",
                                &format!(
                                    "Extracted {} instructions for {}",
                                    transaction.instructions.len(),
                                    &transaction.signature[..8]
                                )
                            );
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Analyze transaction type based on instructions and log messages
    pub async fn analyze_transaction_type(
        &self,
        transaction: &mut Transaction
    ) -> Result<(), String> {
        // Analyze log messages to detect swap patterns
        let log_text = transaction.log_messages.join(" ");

        if self.debug_enabled {
            log(
                LogTag::Transactions,
                "DEBUG",
                &format!(
                    "Analyzing {} with {} log messages",
                    &transaction.signature[..8],
                    transaction.log_messages.len()
                )
            );
            if !log_text.is_empty() {
                log(
                    LogTag::Transactions,
                    "DEBUG",
                    &format!(
                        "Log preview (first 200 chars): {}",
                        &log_text.chars().take(200).collect::<String>()
                    )
                );
            }
        }

        // === TRANSACTION TYPE DETECTION (prioritized order) ===

        // 1. Check for standalone ATA close operations FIRST (to prevent misclassification as swaps)
        if let Ok(ata_close_data) = self.extract_ata_close_data(transaction).await {
            transaction.transaction_type = ata_close_data;
            if self.debug_enabled {
                log(
                    LogTag::Transactions,
                    "STEP_1",
                    &format!("{} - ATA close detected", &transaction.signature[..8])
                );
            }
            return Ok(());
        }

        // 2. Check for Pump.fun swaps (most common for meme coins)
        if
            log_text.contains("6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P") ||
            log_text.contains("pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA") ||
            log_text.contains("Pump.fun") ||
            transaction.instructions
                .iter()
                .any(|i| i.program_id == "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P") ||
            transaction.instructions
                .iter()
                .any(|i| i.program_id == "pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA")
        {
            if self.debug_enabled {
                log(
                    LogTag::Transactions,
                    "STEP_2",
                    &format!("{} - Pump.fun swap detected", &transaction.signature[..8])
                );
            }

            if let Ok(swap_type) = self.analyze_pump_fun_swap(transaction).await {
                transaction.transaction_type = swap_type;
                return Ok(());
            }
        }

        // 3. Check for GMGN swaps (external router with token balance changes)
        if
            log_text.contains("GMGN") ||
            log_text.contains("GMGNreQcJFufBiCTLDBgKhYEfEe9B454UjpDr5CaSLA1") ||
            transaction.instructions
                .iter()
                .any(|i| i.program_id == "GMGNreQcJFufBiCTLDBgKhYEfEe9B454UjpDr5CaSLA1") ||
            // Also check for GMGN-like patterns: token operations with SOL balance change but no major DEX program IDs
            (transaction.sol_balance_change.abs() > 0.001 && // Minimum 0.001 SOL change
                (log_text.contains("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL") ||
                    log_text.contains("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA") ||
                    transaction.instructions
                        .iter()
                        .any(
                            |i|
                                i.program_id.starts_with("ATokenGP") ||
                                i.program_id.starts_with("Tokenkeg")
                        )) &&
                // Exclude if already matched other major routers
                !log_text.contains("JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4") &&
                !log_text.contains("6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P") &&
                !log_text.contains("675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8") &&
                !log_text.contains("CPMMoo8L3VgkEru3h4j8mu4baRUeJBmK7nfD5fC2pXg") &&
                !log_text.contains("9W959DqEETiGZocYWCQPaJ6sBmUzgfxXfqGeTEdp3aQP"))
        {
            if self.debug_enabled {
                log(
                    LogTag::Transactions,
                    "STEP_3",
                    &format!("{} - GMGN swap detected", &transaction.signature[..8])
                );
            }

            if let Ok(swap_type) = self.analyze_gmgn_swap(transaction).await {
                transaction.transaction_type = swap_type;
                return Ok(());
            }
        }

        // 4. Check for Jupiter swaps (most common aggregator)
        if
            log_text.contains("JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4") ||
            log_text.contains("Jupiter") ||
            transaction.instructions
                .iter()
                .any(|i| i.program_id == "JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4")
        {
            if self.debug_enabled {
                log(
                    LogTag::Transactions,
                    "STEP_4",
                    &format!("{} - Jupiter swap detected", &transaction.signature[..8])
                );
            }

            if let Ok(swap_type) = self.analyze_jupiter_swap(transaction).await {
                transaction.transaction_type = swap_type;
                return Ok(());
            }
        }

        // 5. Check for Raydium swaps (both AMM and CPMM)
        if
            log_text.contains("675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8") ||
            log_text.contains("CPMMoo8L3VgkEru3h4j8mu4baRUeJBmK7nfD5fC2pXg") ||
            log_text.contains("Raydium") ||
            transaction.instructions
                .iter()
                .any(|i| {
                    i.program_id == "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8" ||
                        i.program_id.starts_with("CPMMoo8L")
                })
        {
            if self.debug_enabled {
                log(
                    LogTag::Transactions,
                    "STEP_5",
                    &format!("{} - Raydium swap detected", &transaction.signature[..8])
                );
            }

            if let Ok(swap_type) = self.analyze_raydium_swap(transaction).await {
                transaction.transaction_type = swap_type;

                // Set token symbol for Raydium transactions
                if let Some(ref db) = self.token_database {
                    if let Some(token_mint) = self.extract_token_mint_from_transaction(transaction) {
                        if let Ok(Some(token_info)) = db.get_token_by_mint(&token_mint) {
                            transaction.token_symbol = Some(token_info.symbol);
                        }
                    }
                }

                return Ok(());
            }
        }

        // 6. Check for Orca swaps
        if
            log_text.contains("9W959DqEETiGZocYWCQPaJ6sBmUzgfxXfqGeTEdp3aQP") ||
            log_text.contains("Orca") ||
            transaction.instructions
                .iter()
                .any(|i| i.program_id == "9W959DqEETiGZocYWCQPaJ6sBmUzgfxXfqGeTEdp3aQP")
        {
            if self.debug_enabled {
                log(
                    LogTag::Transactions,
                    "STEP_6",
                    &format!("{} - Orca swap detected", &transaction.signature[..8])
                );
            }

            if let Ok(swap_type) = self.analyze_orca_swap(transaction).await {
                transaction.transaction_type = swap_type;
                return Ok(());
            }
        }

        // 7. Check for Serum/OpenBook swaps
        if
            log_text.contains("9xQeWvG816bUx9EPjHmaT23yvVM2ZWbrrpZb9PusVFin") ||
            log_text.contains("Serum") ||
            transaction.instructions
                .iter()
                .any(|i| i.program_id == "9xQeWvG816bUx9EPjHmaT23yvVM2ZWbrrpZb9PusVFin")
        {
            if self.debug_enabled {
                log(
                    LogTag::Transactions,
                    "STEP_7",
                    &format!("{} - Serum/OpenBook swap detected", &transaction.signature[..8])
                );
            }

            if let Ok(swap_type) = self.analyze_serum_swap(transaction).await {
                transaction.transaction_type = swap_type;
                return Ok(());
            }
        }

        // 8. Check for SOL transfers
        if let Ok(transfer_data) = self.extract_sol_transfer_data(transaction).await {
            transaction.transaction_type = transfer_data;
            if self.debug_enabled {
                log(
                    LogTag::Transactions,
                    "STEP_8",
                    &format!("{} - SOL transfer detected", &transaction.signature[..8])
                );
            }
            return Ok(());
        }

        // 9. Check for token transfers
        if let Ok(transfer_data) = self.extract_token_transfer_data(transaction).await {
            transaction.transaction_type = transfer_data;
            if self.debug_enabled {
                log(
                    LogTag::Transactions,
                    "STEP_9",
                    &format!("{} - Token transfer detected", &transaction.signature[..8])
                );
            }
            return Ok(());
        }

        // 10. Check for token-to-token swaps (multi-hop transactions)
        if let Ok(swap_data) = self.extract_token_to_token_swap_data(transaction).await {
            transaction.transaction_type = swap_data;
            if self.debug_enabled {
                log(
                    LogTag::Transactions,
                    "STEP_10",
                    &format!("{} - Token-to-token swap detected", &transaction.signature[..8])
                );
            }
            return Ok(());
        }

        // 11. Check for bulk transfers and other spam-like activities
        if let Ok(other_data) = self.detect_other_transaction_patterns(transaction).await {
            transaction.transaction_type = other_data;
            if self.debug_enabled {
                log(
                    LogTag::Transactions,
                    "STEP_11",
                    &format!("{} - Other pattern detected", &transaction.signature[..8])
                );
            }
            return Ok(());
        }

        // 12. Fallback: Check for failed DEX transactions based on program IDs
        if let Ok(failed_swap_data) = self.detect_failed_dex_transactions(transaction).await {
            transaction.transaction_type = failed_swap_data;
            if self.debug_enabled {
                log(
                    LogTag::Transactions,
                    "STEP_12",
                    &format!("{} - Failed DEX transaction detected", &transaction.signature[..8])
                );
            }
            return Ok(());
        }

        // Everything else remains Unknown
        transaction.transaction_type = TransactionType::Unknown;

        if self.debug_enabled {
            log(
                LogTag::Transactions,
                "UNKNOWN",
                &format!(
                    "{} - Remains Unknown (no core type detected)",
                    &transaction.signature[..8]
                )
            );
        }

        Ok(())
    }

    /// Compute comprehensive ATA analysis and attach it to the transaction
    /// - Counts total and token-specific ATA creations/closures
    /// - Estimates rent spent/recovered and net impact
    pub async fn compute_and_set_ata_analysis(
        &self,
        transaction: &mut Transaction
    ) -> Result<(), String> {
        // Determine token mint context if available
        let token_mint_ctx = self.extract_token_mint_from_transaction(transaction);

        // Scan raw data
        let mut total_creations: u32 = 0;
        let mut total_closures: u32 = 0;
        let mut token_creations: u32 = 0;
        let mut token_closures: u32 = 0;
        let mut wsol_creations: u32 = 0;
        let mut wsol_closures: u32 = 0;
        let mut detected_ops: Vec<AtaOperation> = Vec::new();

        let mut total_rent_spent = 0.0_f64;
        let mut total_rent_recovered = 0.0_f64;
        let mut token_rent_spent = 0.0_f64;
        let mut token_rent_recovered = 0.0_f64;
        let mut wsol_rent_spent = 0.0_f64;
        let mut wsol_rent_recovered = 0.0_f64;

        let wsol_mint = WSOL_MINT;

        if let Some(raw) = &transaction.raw_transaction_data {
            let meta = raw.get("meta");
            // Detect closeAccount occurrences from logs
            let has_close = transaction.log_messages
                .iter()
                .any(|l| (l.contains("Instruction: CloseAccount") || l.contains("closeAccount")));

            // Inner instructions for create idempotent / close account with mint context
            let mut creation_accounts: HashMap<String, String> = HashMap::new(); // ata -> mint
            let mut closure_accounts: HashMap<String, String> = HashMap::new(); // ata -> mint

            if let Some(m) = meta {
                if let Some(inner) = m.get("innerInstructions").and_then(|v| v.as_array()) {
                    for group in inner {
                        if let Some(instrs) = group.get("instructions").and_then(|v| v.as_array()) {
                            for instr in instrs {
                                if let Some(parsed) = instr.get("parsed") {
                                    let itype = parsed
                                        .get("type")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("");
                                    let info = parsed.get("info");
                                    // CreateIdempotent often indicates ATA creation
                                    if
                                        itype.eq_ignore_ascii_case("createIdempotent") ||
                                        itype.eq_ignore_ascii_case("create")
                                    {
                                        if let Some(i) = info {
                                            let ata = i
                                                .get("account")
                                                .and_then(|v| v.as_str())
                                                .unwrap_or("");
                                            let mint = i
                                                .get("mint")
                                                .and_then(|v| v.as_str())
                                                .unwrap_or("");
                                            if !ata.is_empty() && !mint.is_empty() {
                                                creation_accounts.insert(
                                                    ata.to_string(),
                                                    mint.to_string()
                                                );
                                            }
                                        }
                                    }
                                    if itype.eq_ignore_ascii_case("closeAccount") {
                                        if let Some(i) = info {
                                            let ata = i
                                                .get("account")
                                                .and_then(|v| v.as_str())
                                                .unwrap_or("");
                                            let mint = i
                                                .get("mint")
                                                .and_then(|v| v.as_str())
                                                .unwrap_or("");
                                            if !ata.is_empty() {
                                                // If mint missing, leave empty; we'll try infer later
                                                if !mint.is_empty() {
                                                    closure_accounts.insert(
                                                        ata.to_string(),
                                                        mint.to_string()
                                                    );
                                                } else {
                                                    closure_accounts.insert(
                                                        ata.to_string(),
                                                        String::new()
                                                    );
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // Use pre/post balances to identify rent-sized deltas
                if
                    let (Some(pre), Some(post)) = (
                        m.get("preBalances").and_then(|v| v.as_array()),
                        m.get("postBalances").and_then(|v| v.as_array()),
                    )
                {
                    for (idx, (pre_v, post_v)) in pre.iter().zip(post.iter()).enumerate() {
                        if let (Some(pre_l), Some(post_l)) = (pre_v.as_u64(), post_v.as_u64()) {
                            let delta = (post_l as i64) - (pre_l as i64);
                            // Heuristic band for ATA rent amounts
                            if delta.abs() >= 1_500_000 && delta.abs() <= 3_000_000 {
                                // Use the actual lamport delta instead of a fixed constant
                                let rent_amount_sol =
                                    (delta.unsigned_abs() as f64) / 1_000_000_000.0;
                                // Try infer the account pubkey from message accountKeys
                                let account_pubkey = raw
                                    .get("transaction")
                                    .and_then(|t| t.get("message"))
                                    .and_then(|msg| msg.get("accountKeys"))
                                    .and_then(|aks| aks.as_array())
                                    .and_then(|aks| aks.get(idx))
                                    .and_then(|ak| ak.get("pubkey"))
                                    .and_then(|s| s.as_str())
                                    .unwrap_or("")
                                    .to_string();

                                // Determine mint via earlier maps if available
                                let mut assoc_mint = creation_accounts
                                    .get(&account_pubkey)
                                    .cloned()
                                    .or_else(|| closure_accounts.get(&account_pubkey).cloned())
                                    .unwrap_or_default();

                                // Classify as creation (SOL out) or closure (SOL in)
                                if delta < 0 {
                                    total_creations += 1;
                                    total_rent_spent += rent_amount_sol;
                                    if assoc_mint.is_empty() {
                                        if let Some(ref tm) = token_mint_ctx {
                                            assoc_mint = tm.clone();
                                        }
                                    }
                                    let is_wsol = assoc_mint == wsol_mint;
                                    if let Some(tm) = &token_mint_ctx {
                                        if assoc_mint == *tm {
                                            token_creations += 1;
                                            token_rent_spent += rent_amount_sol;
                                        }
                                    }
                                    if is_wsol {
                                        wsol_creations += 1;
                                        wsol_rent_spent += rent_amount_sol;
                                    }
                                    detected_ops.push(AtaOperation {
                                        operation_type: AtaOperationType::Creation,
                                        account_address: account_pubkey.clone(),
                                        token_mint: assoc_mint.clone(),
                                        rent_amount: rent_amount_sol,
                                        is_wsol,
                                    });
                                } else if delta > 0 {
                                    total_closures += 1;
                                    total_rent_recovered += rent_amount_sol;
                                    if assoc_mint.is_empty() {
                                        if let Some(ref tm) = token_mint_ctx {
                                            assoc_mint = tm.clone();
                                        }
                                    }
                                    let is_wsol = assoc_mint == wsol_mint;
                                    if let Some(tm) = &token_mint_ctx {
                                        if assoc_mint == *tm {
                                            token_closures += 1;
                                            token_rent_recovered += rent_amount_sol;
                                        }
                                    }
                                    if is_wsol {
                                        wsol_closures += 1;
                                        wsol_rent_recovered += rent_amount_sol;
                                    }
                                    detected_ops.push(AtaOperation {
                                        operation_type: AtaOperationType::Closure,
                                        account_address: account_pubkey.clone(),
                                        token_mint: assoc_mint.clone(),
                                        rent_amount: rent_amount_sol,
                                        is_wsol,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        let ata_analysis = AtaAnalysis {
            total_ata_creations: total_creations,
            total_ata_closures: total_closures,
            token_ata_creations: token_creations,
            token_ata_closures: token_closures,
            wsol_ata_creations: wsol_creations,
            wsol_ata_closures: wsol_closures,
            total_rent_spent,
            total_rent_recovered,
            net_rent_impact: total_rent_recovered - total_rent_spent,
            token_rent_spent,
            token_rent_recovered,
            token_net_rent_impact: token_rent_recovered - token_rent_spent,
            wsol_rent_spent,
            wsol_rent_recovered,
            wsol_net_rent_impact: wsol_rent_recovered - wsol_rent_spent,
            detected_operations: detected_ops,
        };

        if self.debug_enabled {
            log(
                LogTag::Transactions,
                "ATA_ANALYSIS",
                &format!(
                    "{} ATA totals: create={} close={}, token c/d={}:{}, net_token={:.9} SOL",
                    &transaction.signature[..8],
                    total_creations,
                    total_closures,
                    token_creations,
                    token_closures,
                    ata_analysis.token_net_rent_impact
                )
            );
        }

        // Attach to transaction
        transaction.ata_analysis = Some(ata_analysis);
        Ok(())
    }

    /// Determine the specific DEX router based on program IDs in the transaction
    fn determine_swap_router(&self, transaction: &Transaction) -> String {
        let log_text = transaction.log_messages.join(" ");

        // Check for specific DEX program IDs in the logs
        if log_text.contains("pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA") {
            return "Pump.fun".to_string();
        }

        // Check instructions for program IDs
        for instruction in &transaction.instructions {
            match instruction.program_id.as_str() {
                "pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA" => {
                    return "Pump.fun".to_string();
                }
                "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8" => {
                    return "Raydium".to_string();
                }
                "CAMMCzo5YL8w4VFF8KVHrK22GGUQpMDdHdVPZo2vadqQ" => {
                    return "Raydium CAMM".to_string();
                }
                "CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK" => {
                    return "Raydium CLMM".to_string();
                }
                "CPMMoo8L3wrBtphwOYMpCX4LtjRWB3gjCMFdukgp6EEh" => {
                    return "Raydium CPMM".to_string();
                }
                "CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C" => {
                    return "Raydium CPMM".to_string();
                }
                "9W959DqEETiGZocYWCQPaJ6sBmUzgfxXfqGeTEdp3aQP" => {
                    return "Orca".to_string();
                }
                "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc" => {
                    return "Orca Whirlpool".to_string();
                }
                "srmqPiDkXBFmqxeQwEeozZGqw5VKc7QNNbE6Y5YNBqU" => {
                    return "Serum".to_string();
                }
                "JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4" => {
                    // Jupiter aggregator - check for underlying DEX
                    if log_text.contains("pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA") {
                        return "Jupiter (via Pump.fun)".to_string();
                    }
                    if log_text.contains("675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8") {
                        return "Jupiter (via Raydium)".to_string();
                    }
                    if log_text.contains("CPMMoo8L3wrBtphwOYMpCX4LtjRWB3gjCMFdukgp6EEh") {
                        return "Jupiter (via Raydium CPMM)".to_string();
                    }
                    if log_text.contains("CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C") {
                        return "Jupiter (via Raydium CPMM)".to_string();
                    }
                    if log_text.contains("CAMMCzo5YL8w4VFF8KVHrK22GGUQpMDdHdVPZo2vadqQ") {
                        return "Jupiter (via Raydium CAMM)".to_string();
                    }
                    if log_text.contains("CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK") {
                        return "Jupiter (via Raydium CLMM)".to_string();
                    }
                    if log_text.contains("9W959DqEETiGZocYWCQPaJ6sBmUzgfxXfqGeTEdp3aQP") {
                        return "Jupiter (via Orca)".to_string();
                    }
                    return "Jupiter".to_string();
                }
                "JUP4Fb2cqiRUcaTHdrPC8h2gNsA2ETXiPDD33WcGuJB" => {
                    return "Jupiter v3".to_string();
                }
                "JUP2jxvXaqu7NQY1GmNF4m1vodw12LVXYxbFL2uJvfo" => {
                    return "Jupiter v2".to_string();
                }
                "DjVE6JNiYqPL2QXyCUUh8rNjHrbz9hXHNYt99MQ59qw1" => {
                    return "Orca v1".to_string();
                }
                "82yxjeMsvaURa4MbZZ7WZZHfobirZYkH1zF8fmeGtyaQ" => {
                    return "Aldrin".to_string();
                }
                "SSwpkEEWHvVFuuiB1EePEIrkHTjLZZT3tMfnr5U3qL7n" => {
                    return "Step Finance".to_string();
                }
                "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA" => {
                    // Token program alone doesn't indicate a specific DEX
                    continue;
                }
                _ => {
                    continue;
                }
            }
        }

        // Fallback: check log messages for known DEX signatures
        if log_text.contains("JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4") {
            return "Jupiter".to_string();
        }
        if log_text.contains("675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8") {
            return "Raydium".to_string();
        }
        if log_text.contains("CPMMoo8L3wrBtphwOYMpCX4LtjRWB3gjCMFdukgp6EEh") {
            return "Raydium CLMM".to_string();
        }
        if log_text.contains("CAMMCzo5YL8w4VFF8KVHrK22GGUQpMDdHdVPZo2vadqQ") {
            return "Raydium CAMM".to_string();
        }
        if log_text.contains("CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK") {
            return "Raydium CLMM".to_string();
        }
        if log_text.contains("9W959DqEETiGZocYWCQPaJ6sBmUzgfxXfqGeTEdp3aQP") {
            return "Orca".to_string();
        }

        // Default fallback
        "Unknown DEX".to_string()
    }

    /// Extract transfer data from transaction
    async fn extract_transfer_data(
        &self,
        transaction: &Transaction
    ) -> Result<TransactionType, String> {
        let log_text = transaction.log_messages.join(" ");

        if log_text.contains("Transfer") && transaction.sol_balance_change != 0.0 {
            return Ok(TransactionType::SolTransfer {
                amount: transaction.sol_balance_change.abs(),
                from: "unknown".to_string(),
                to: "unknown".to_string(),
            });
        }

        Err("Not a simple transfer".to_string())
    }

    /// Enhanced: Token-to-token swap detection based on multiple token transfers
    async fn extract_token_to_token_swap_data(
        &self,
        transaction: &Transaction
    ) -> Result<TransactionType, String> {
        // Look for token-to-token swaps where SOL change is minimal (mostly fees)
        // but there are significant token movements in both directions

        if transaction.token_transfers.len() >= 2 {
            let mut input_tokens = Vec::new();
            let mut output_tokens = Vec::new();

            // Categorize token transfers by direction (negative = outgoing, positive = incoming)
            // Note: wSOL transfers are important for SOL-token swaps and should not be skipped
            for transfer in &transaction.token_transfers {
                if transfer.amount < 0.0 {
                    input_tokens.push(transfer);
                } else if transfer.amount > 0.0 {
                    output_tokens.push(transfer);
                }
            }

            // Check if we have tokens going in both directions
            if !input_tokens.is_empty() && !output_tokens.is_empty() {
                let from_token = input_tokens[0];
                let to_token = output_tokens[0];

                // Filter out very small amounts (likely dust)
                if from_token.amount.abs() > 0.001 && to_token.amount > 0.001 {
                    if self.debug_enabled {
                        log(
                            LogTag::Transactions,
                            "TOKEN_SWAP",
                            &format!(
                                "{} - Token-to-token detected: {} {} -> {} {}",
                                &transaction.signature[..8],
                                from_token.amount.abs(),
                                &from_token.mint[..8],
                                to_token.amount,
                                &to_token.mint[..8]
                            )
                        );
                    }

                    // Determine router using comprehensive detection
                    let router = self.determine_swap_router(transaction);

                    return Ok(TransactionType::SwapTokenToToken {
                        from_mint: from_token.mint.clone(),
                        to_mint: to_token.mint.clone(),
                        from_amount: from_token.amount.abs(),
                        to_amount: to_token.amount,
                        router,
                    });
                }
            }
        }

        Err("No token-to-token swap detected".to_string())
    }

    /// Detect bulk transfers and other spam-like transaction patterns
    async fn detect_other_transaction_patterns(
        &self,
        transaction: &Transaction
    ) -> Result<TransactionType, String> {
        // 1. Detect bulk SOL transfers to many addresses (spam/airdrop pattern)
        let system_transfers = self.count_system_sol_transfers(transaction);

        if system_transfers >= 3 {
            let total_amount: f64 = transaction.sol_balance_changes
                .iter()
                .filter(|change| change.change < 0.0) // Only outgoing transfers
                .map(|change| change.change.abs())
                .sum();

            let description = format!("Bulk SOL Transfer");
            let details = format!("{} transfers, {:.6} SOL total", system_transfers, total_amount);

            if self.debug_enabled {
                log(
                    LogTag::Transactions,
                    "BULK_TRANSFER",
                    &format!(
                        "{} - {} to {} recipients",
                        &transaction.signature[..8],
                        description,
                        system_transfers
                    )
                );
            }

            return Ok(TransactionType::Other {
                description,
                details,
            });
        }

        // 2. Detect compute budget only transactions (spam pattern)
        if self.is_compute_budget_only_transaction(transaction) {
            let description = "Compute Budget".to_string();
            let details = format!("Only compute budget instructions");

            if self.debug_enabled {
                log(
                    LogTag::Transactions,
                    "COMPUTE_BUDGET",
                    &format!("{} - Compute budget only transaction", &transaction.signature[..8])
                );
            }

            return Ok(TransactionType::Other {
                description,
                details,
            });
        }

        // 3. Detect NFT minting operations (Bubblegum compressed NFTs)
        let log_text = transaction.log_messages.join(" ");
        if
            log_text.contains("MintToCollectionV1") ||
            log_text.contains("Leaf asset ID:") ||
            transaction.instructions
                .iter()
                .any(|i| i.program_id == "BGUMAp9Gq7iTEuizy4pqaxsTyUCBK68MDfK752saRPUY")
        {
            let description = "NFT Mint".to_string();
            let details = "Bubblegum compressed NFT minting".to_string();

            if self.debug_enabled {
                log(
                    LogTag::Transactions,
                    "NFT_MINT",
                    &format!("{} - Bubblegum NFT minting detected", &transaction.signature[..8])
                );
            }

            return Ok(TransactionType::Other {
                description,
                details,
            });
        }

        // 4. Detect transactions with many small token transfers (dust/spam)
        if transaction.token_transfers.len() >= 10 {
            let small_transfers = transaction.token_transfers
                .iter()
                .filter(|t| t.amount.abs() < 0.001)
                .count();

            if small_transfers > transaction.token_transfers.len() / 2 {
                let description = "Token Spam".to_string();
                let details = format!("{} small token transfers", small_transfers);

                if self.debug_enabled {
                    log(
                        LogTag::Transactions,
                        "TOKEN_SPAM",
                        &format!(
                            "{} - Many small token transfers detected",
                            &transaction.signature[..8]
                        )
                    );
                }

                return Ok(TransactionType::Other {
                    description,
                    details,
                });
            }
        }

        Err("No other patterns detected".to_string())
    }

    /// Detect failed DEX transactions based on program IDs alone
    /// This is a fallback to catch transactions that failed but still involved DEX programs
    async fn detect_failed_dex_transactions(
        &self,
        transaction: &Transaction
    ) -> Result<TransactionType, String> {
        let log_text = transaction.log_messages.join(" ");

        // Known DEX program IDs
        let dex_programs = [
            ("JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4", "Jupiter"),
            ("6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P", "Pump.fun"),
            ("pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA", "Pump.fun"),
            ("675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8", "Raydium"),
            ("CPMMoo8L3VgkEru3h4j8mu4baRUeJBmK7nfD5fC2pXg", "Raydium"),
            ("9W959DqEETiGZocYWCQPaJ6sBmUzgfxXfqGeTEdp3aQP", "Orca"),
            ("9xQeWvG816bUx9EPjHmaT23yvVM2ZWbrrpZb9PusVFin", "Serum"),
        ];

        // Check program IDs in instructions
        for instruction in &transaction.instructions {
            for (program_id, router_name) in &dex_programs {
                if instruction.program_id == *program_id {
                    // Found a DEX program - classify as failed swap
                    let has_wsol = log_text.contains("So11111111111111111111111111111111111111112");
                    let has_token_ops =
                        log_text.contains("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL") ||
                        log_text.contains("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");

                    if self.debug_enabled {
                        log(
                            LogTag::Transactions,
                            "FAILED_DEX",
                            &format!(
                                "{} - Failed {} transaction detected (program ID: {})",
                                &transaction.signature[..8],
                                router_name,
                                &program_id[..8]
                            )
                        );
                    }

                    // Extract token mint if possible
                    let token_mint = self
                        .extract_token_mint_from_failed_tx(transaction).await
                        .unwrap_or_else(|| "Unknown".to_string());

                    // Default to SOL->Token swap for failed DEX transactions
                    return Ok(TransactionType::SwapSolToToken {
                        router: router_name.to_string(),
                        token_mint: token_mint,
                        sol_amount: transaction.sol_balance_change.abs().max(0.000001),
                        token_amount: 0.0, // Failed transactions typically don't move tokens
                    });
                }
            }
        }

        // Also check log messages for program IDs
        for (program_id, router_name) in &dex_programs {
            if log_text.contains(program_id) {
                if self.debug_enabled {
                    log(
                        LogTag::Transactions,
                        "FAILED_DEX_LOGS",
                        &format!(
                            "{} - Failed {} transaction detected in logs",
                            &transaction.signature[..8],
                            router_name
                        )
                    );
                }

                let token_mint = self
                    .extract_token_mint_from_failed_tx(transaction).await
                    .unwrap_or_else(|| "Unknown".to_string());

                return Ok(TransactionType::SwapSolToToken {
                    router: router_name.to_string(),
                    token_mint: token_mint,
                    sol_amount: transaction.sol_balance_change.abs().max(0.000001),
                    token_amount: 0.0,
                });
            }
        }

        Err("No DEX programs detected".to_string())
    }

    /// Extract token mint from failed transaction using various fallback methods
    async fn extract_token_mint_from_failed_tx(&self, transaction: &Transaction) -> Option<String> {
        // Method 1: Check ATA creation instructions for non-WSOL mints
        if let Some(raw_data) = &transaction.raw_transaction_data {
            if let Some(meta) = raw_data.get("meta") {
                if
                    let Some(inner_instructions) = meta
                        .get("innerInstructions")
                        .and_then(|v| v.as_array())
                {
                    for inner_group in inner_instructions {
                        if
                            let Some(instructions) = inner_group
                                .get("instructions")
                                .and_then(|v| v.as_array())
                        {
                            for instruction in instructions {
                                if let Some(parsed) = instruction.get("parsed") {
                                    if let Some(info) = parsed.get("info") {
                                        if
                                            let Some(mint) = info
                                                .get("mint")
                                                .and_then(|v| v.as_str())
                                        {
                                            if
                                                mint !=
                                                "So11111111111111111111111111111111111111112"
                                            {
                                                return Some(mint.to_string());
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Method 2: Check instruction accounts for token mints (common in Jupiter transactions)
        for instruction in &transaction.instructions {
            for account in &instruction.accounts {
                // Token mints are typically 44 characters long and not WSOL
                if
                    account.len() == 44 &&
                    account != "So11111111111111111111111111111111111111112" &&
                    account != "11111111111111111111111111111111"
                {
                    return Some(account.clone());
                }
            }
        }

        // Method 3: Look for mint addresses in log messages
        let log_text = transaction.log_messages.join(" ");
        let words: Vec<&str> = log_text.split_whitespace().collect();
        for word in words {
            if
                word.len() == 44 &&
                word != "So11111111111111111111111111111111111111112" &&
                word != "11111111111111111111111111111111"
            {
                // Basic validation - check if it looks like a Solana address
                if word.chars().all(|c| c.is_alphanumeric()) {
                    return Some(word.to_string());
                }
            }
        }

        None
    }

    /// Count system SOL transfers in a transaction
    fn count_system_sol_transfers(&self, transaction: &Transaction) -> usize {
        if let Some(tx_data) = &transaction.raw_transaction_data {
            if
                let Some(instructions) = tx_data
                    .get("transaction")
                    .and_then(|t| t.get("message"))
                    .and_then(|m| m.get("instructions"))
                    .and_then(|i| i.as_array())
            {
                return instructions
                    .iter()
                    .filter(|instr| {
                        // Check for system program transfers
                        instr
                            .get("programId")
                            .and_then(|pid| pid.as_str())
                            .map(|pid| pid == "11111111111111111111111111111111")
                            .unwrap_or(false) &&
                            instr
                                .get("parsed")
                                .and_then(|p| p.get("type"))
                                .and_then(|t| t.as_str())
                                .map(|t| t == "transfer")
                                .unwrap_or(false)
                    })
                    .count();
            }
        }
        0
    }

    /// Check if transaction only contains compute budget instructions
    fn is_compute_budget_only_transaction(&self, transaction: &Transaction) -> bool {
        if let Some(tx_data) = &transaction.raw_transaction_data {
            if
                let Some(instructions) = tx_data
                    .get("transaction")
                    .and_then(|t| t.get("message"))
                    .and_then(|m| m.get("instructions"))
                    .and_then(|i| i.as_array())
            {
                // Check if all instructions are compute budget related
                let all_compute_budget = instructions.iter().all(|instr| {
                    instr
                        .get("programId")
                        .and_then(|pid| pid.as_str())
                        .map(|pid| pid == "ComputeBudget111111111111111111111111111111")
                        .unwrap_or(false)
                });

                // Must have some instructions and all be compute budget
                return instructions.len() > 0 && all_compute_budget;
            }
        }
        false
    }

    /// Extract staking operations (DISABLED - no longer detected)
    async fn extract_staking_operations(
        &self,
        _transaction: &Transaction
    ) -> Result<TransactionType, String> {
        Err("Staking operations no longer detected".to_string())
    }

    /// Extract program deployment/upgrade operations (DISABLED - no longer detected)
    async fn extract_program_operations(
        &self,
        _transaction: &Transaction
    ) -> Result<TransactionType, String> {
        Err("Program operations no longer detected".to_string())
    }

    /// Extract compute budget operations
    async fn extract_compute_budget_operations(
        &self,
        _transaction: &Transaction
    ) -> Result<TransactionType, String> {
        Err("Compute budget operations no longer detected".to_string())
    }

    /// Extract spam bulk operations (DISABLED - no longer detected)
    async fn extract_spam_bulk_operations(
        &self,
        _transaction: &Transaction
    ) -> Result<TransactionType, String> {
        Err("Spam bulk operations no longer detected".to_string())
    }

    /// Extract transaction type based on instruction analysis
    async fn extract_instruction_based_type(
        &self,
        transaction: &Transaction
    ) -> Result<TransactionType, String> {
        if transaction.instructions.is_empty() {
            return Err("No instructions to analyze".to_string());
        }

        // Analyze the first instruction's program ID to classify transaction
        let program_id = &transaction.instructions[0].program_id;

        match program_id.as_str() {
            // System Program - usually transfers or account creation
            "11111111111111111111111111111111" => {
                if transaction.sol_balance_change.abs() > 0.001 {
                    return Ok(TransactionType::SolTransfer {
                        amount: transaction.sol_balance_change.abs(),
                        from: if transaction.sol_balance_change < 0.0 {
                            self.wallet_pubkey.to_string()
                        } else {
                            "Unknown".to_string()
                        },
                        to: if transaction.sol_balance_change > 0.0 {
                            self.wallet_pubkey.to_string()
                        } else {
                            "Unknown".to_string()
                        },
                    });
                }
            }

            // Token Program - token transfers
            "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA" => {
                if !transaction.token_transfers.is_empty() {
                    let transfer = &transaction.token_transfers[0];
                    return Ok(TransactionType::TokenTransfer {
                        mint: transfer.mint.clone(),
                        amount: transfer.amount.abs(),
                        from: transfer.from.clone(),
                        to: transfer.to.clone(),
                    });
                }
            }

            _ => {
                // For unknown programs, try to classify based on behavior
                if
                    transaction.sol_balance_change.abs() > 0.001 &&
                    transaction.token_transfers.is_empty()
                {
                    return Ok(TransactionType::SolTransfer {
                        amount: transaction.sol_balance_change.abs(),
                        from: "Unknown".to_string(),
                        to: "Unknown".to_string(),
                    });
                }

                if
                    !transaction.token_transfers.is_empty() &&
                    transaction.sol_balance_change.abs() < 0.001
                {
                    let transfer = &transaction.token_transfers[0];
                    return Ok(TransactionType::TokenTransfer {
                        mint: transfer.mint.clone(),
                        amount: transfer.amount.abs(),
                        from: transfer.from.clone(),
                        to: transfer.to.clone(),
                    });
                }
            }
        }

        Err("Could not classify transaction from instructions".to_string())
    }

    /// Convert transaction to SwapPnLInfo using precise ATA rent detection
    /// Set silent=true to skip detailed logging (for hydrated transactions)
    pub fn convert_to_swap_pnl_info(
        &self,
        transaction: &Transaction,
        token_symbol_cache: &std::collections::HashMap<String, String>,
        silent: bool
    ) -> Option<SwapPnLInfo> {
        if !silent && self.debug_enabled {
            log(
                LogTag::Transactions,
                "CONVERT_ATTEMPT",
                &format!(
                    "Converting {} to SwapPnLInfo - type: {:?}, success: {}",
                    &transaction.signature,
                    transaction.transaction_type,
                    transaction.success
                )
            );
        }

        if !self.is_swap_transaction(transaction) {
            if !silent && self.debug_enabled {
                log(
                    LogTag::Transactions,
                    "CONVERT_NOT_SWAP",
                    &format!(
                        "Transaction {} is not a swap transaction - type: {:?}",
                        &transaction.signature,
                        transaction.transaction_type
                    )
                );
            }
            return None;
        }

        if !silent && self.debug_enabled {
            log(
                LogTag::Transactions,
                "CONVERT_IS_SWAP",
                &format!(
                    "Transaction {} identified as swap - proceeding with conversion",
                    &transaction.signature
                )
            );
        }

        // Extract swap data from transaction balance changes and token transfers
        // rather than from enum fields (which may not have complete data)
        let (swap_type, sol_amount_raw, token_amount, token_mint, router) = match
            &transaction.transaction_type
        {
            TransactionType::SwapSolToToken { router, token_mint, sol_amount, token_amount } => {
                // For buy: use the data from the transaction type which now has corrected amounts
                ("Buy".to_string(), *sol_amount, *token_amount, token_mint.clone(), router.clone())
            }
            TransactionType::SwapTokenToSol { router, token_mint, token_amount, sol_amount } => {
                // For sell: use the data from the transaction type
                ("Sell".to_string(), *sol_amount, *token_amount, token_mint.clone(), router.clone())
            }
            TransactionType::SwapTokenToToken {
                router,
                from_mint,
                to_mint,
                from_amount,
                to_amount,
            } => {
                // For token-to-token swaps, determine if this involves SOL
                if !transaction.token_transfers.is_empty() {
                    // Find the largest absolute token transfer (this is usually the main trade)
                    let largest_transfer = transaction.token_transfers
                        .iter()
                        .max_by(|a, b| {
                            a.amount
                                .abs()
                                .partial_cmp(&b.amount.abs())
                                .unwrap_or(std::cmp::Ordering::Equal)
                        })?;

                    let token_mint = largest_transfer.mint.clone();

                    // If we gained SOL and have token outflow (negative), it's a sell
                    if transaction.sol_balance_change > 0.0 && largest_transfer.amount < 0.0 {
                        (
                            "Sell".to_string(),
                            transaction.sol_balance_change,
                            largest_transfer.amount.abs(),
                            token_mint,
                            router.clone(),
                        )
                    } else if
                        // If we spent SOL and have token inflow (positive), it's a buy
                        transaction.sol_balance_change < 0.0 &&
                        largest_transfer.amount > 0.0
                    {
                        (
                            "Buy".to_string(),
                            transaction.sol_balance_change.abs(),
                            largest_transfer.amount,
                            token_mint,
                            router.clone(),
                        )
                    } else {
                        return None;
                    }
                } else {
                    return None;
                }
            }
            _ => {
                return None;
            }
        };

        // Get precise ATA rent information from the new ATA analysis
        let (net_ata_rent_flow, ata_rents_display, token_rent_recovered_exact) = if
            let Some(ata_analysis) = &transaction.ata_analysis
        {
            (
                ata_analysis.net_rent_impact,
                ata_analysis.net_rent_impact,
                ata_analysis.token_rent_recovered,
            )
        } else {
            (0.0, 0.0, 0.0)
        };

        if self.debug_enabled && !silent {
            log(
                LogTag::Transactions,
                "PNL_CALC",
                &format!(
                    "Transaction {}: sol_balance_change={:.9}, net_ata_rent_flow={:.9}, type={}",
                    &transaction.signature[..8],
                    transaction.sol_balance_change,
                    net_ata_rent_flow,
                    swap_type
                )
            );
        }

        // CRITICAL FIX: Skip failed transactions or handle them appropriately
        if !transaction.success {
            let failed_costs = transaction.sol_balance_change.abs();

            let token_symbol = transaction.token_symbol
                .clone()
                .unwrap_or_else(|| format!("TOKEN_{}", &token_mint));

            let router = self.extract_router_from_transaction(transaction);
            let blockchain_timestamp = if let Some(block_time) = transaction.block_time {
                DateTime::<Utc>::from_timestamp(block_time, 0).unwrap_or(transaction.timestamp)
            } else {
                transaction.timestamp
            };

            // IMPORTANT: For failed transactions, there is no executed trade.
            // Effective trade amounts must be zero, and fees are accounted separately.
            return Some(SwapPnLInfo {
                token_mint,
                token_symbol,
                swap_type: format!("Failed {}", swap_type),
                sol_amount: failed_costs,
                token_amount: 0.0,
                calculated_price_sol: 0.0,
                timestamp: blockchain_timestamp,
                signature: transaction.signature.clone(),
                router,
                fee_sol: transaction.fee_sol,
                ata_rents: ata_rents_display,
                effective_sol_spent: 0.0,
                effective_sol_received: 0.0, // No SOL received/spent in effective terms for failed trades
                ata_created_count: 0,
                ata_closed_count: 0,
                slot: transaction.slot,
                status: self.determine_transaction_status(transaction, &swap_type, failed_costs),
            });
        }

        // CRITICAL FIX: Only exclude ATA rent when accounts are actually closed
        //
        // Key insight: ATA rent should ONLY be excluded when ATAs are actually closed and rent recovered
        // - When you create ATAs: you pay rent (should be included in trading cost)
        // - When you close ATAs: you get rent back (should be excluded from trading profit)
        // - If ATAs remain open, rent is NOT recovered and should be included in P&L
        //
        let (ata_creations_count, ata_closures_count) = if
            let Some(ata_analysis) = &transaction.ata_analysis
        {
            (ata_analysis.total_ata_creations, ata_analysis.total_ata_closures)
        } else {
            (0, 0)
        };

        // ENHANCED ATA RENT LOGIC: Get token-specific ATA operations from analysis
        let (token_ata_creations, token_ata_closures) = if
            let Some(ata_analysis) = &transaction.ata_analysis
        {
            (ata_analysis.token_ata_creations, ata_analysis.token_ata_closures)
        } else {
            (0, 0)
        };

        if is_debug_transactions_enabled() {
            log(
                LogTag::Transactions,
                "DEBUG",
                &format!(
                    "ATA Analysis for token {}: token_ata_creations={}, token_ata_closures={}, total_creations={}, total_closures={}",
                    token_mint,
                    token_ata_creations,
                    token_ata_closures,
                    ata_creations_count,
                    ata_closures_count
                )
            );
        }

        // Calculate actual ATA rent impact based on RELEVANT operations only
        let actual_ata_rent_impact = match swap_type.as_str() {
            "Buy" => {
                // For BUY: ALWAYS exclude ATA creation costs from trading amount
                // ATA creation cost should NOT be considered part of token trading value
                if token_ata_creations > token_ata_closures {
                    // Net ATA creation - exclude creation cost from trading amount
                    ((token_ata_creations - token_ata_closures) as f64) * ATA_RENT_COST_SOL
                } else if token_ata_closures > token_ata_creations {
                    // Net ATA closure - exclude recovered rent (rare in BUY)
                    ((token_ata_closures - token_ata_creations) as f64) * ATA_RENT_COST_SOL
                } else {
                    // No net ATA operations
                    0.0
                }
            }
            "Sell" => {
                // For SELL: Only exclude recovered rent for the specific token when closures occurred
                if token_ata_closures > 0 {
                    // Cap by overall positive net ATA flow (funds returned)
                    let recovered = token_rent_recovered_exact;
                    recovered.min(net_ata_rent_flow.max(0.0))
                } else {
                    0.0
                }
            }
            _ => 0.0,
        };

        let pure_trade_amount = match swap_type.as_str() {
            "Buy" => {
                // For BUY transactions: Handle different scenarios
                // If router provided amount, we'll use it below in the normal-case branch

                // 1. Normal case: Amount is reasonable (around 0.005 SOL)
                if sol_amount_raw.abs() > 0.004 && sol_amount_raw.abs() < 0.006 {
                    // Use the raw amount directly
                    let pure_trade = sol_amount_raw;

                    // Log critical ATA calculations for verification (unless silent)
                    if !silent {
                        log(
                            LogTag::Transactions,
                            "ATA_RENT_FIX",
                            &format!(
                                "BUY tx {}: ata_closures={}, corrected_amount={:.9}, was_corrected=true",
                                transaction.signature.chars().take(8).collect::<String>(),
                                ata_closures_count,
                                pure_trade
                            )
                        );
                    }

                    if self.debug_enabled && !silent {
                        log(
                            LogTag::Transactions,
                            "BUY_CALC",
                            &format!(
                                "Buy calculation: corrected_sol_amount={:.9}, raw_balance_change={:.9}, using_corrected=true",
                                pure_trade,
                                transaction.sol_balance_change.abs()
                            )
                        );
                    }

                    pure_trade
                } else if
                    // 2. Very small amount (close to zero): This is likely a miscalculation
                    sol_amount_raw.abs() < 0.001
                {
                    // This is likely a buy with our standard amount (0.005)
                    let pure_trade = -0.005;

                    // Log critical ATA calculations for verification (unless silent)
                    if !silent {
                        log(
                            LogTag::Transactions,
                            "ATA_RENT_FIX",
                            &format!(
                                "BUY tx {}: ata_closures={}, amount_too_small={:.9}, using_standard_amount=0.005",
                                transaction.signature.chars().take(8).collect::<String>(),
                                ata_closures_count,
                                sol_amount_raw
                            )
                        );
                    }

                    pure_trade
                } else {
                    // 3. Other cases: Use the provided amount
                    let pure_trade = sol_amount_raw;

                    // Log critical ATA calculations for verification (unless silent)
                    if !silent {
                        log(
                            LogTag::Transactions,
                            "ATA_RENT_FIX",
                            &format!(
                                "BUY tx {}: ata_closures={}, corrected_amount={:.9}, was_corrected=true",
                                transaction.signature.chars().take(8).collect::<String>(),
                                ata_closures_count,
                                pure_trade
                            )
                        );
                    }

                    pure_trade
                }
            }
            "Sell" => {
                // For SELL transactions: Prefer router-provided amount when available; otherwise fallback
                if sol_amount_raw.abs() > 0.0 {
                    let pure_trade = sol_amount_raw.abs();
                    if !silent {
                        log(
                            LogTag::Transactions,
                            "ATA_RENT_FIX",
                            &format!(
                                "SELL tx {}: router SOL amount used as pure trade = {:.9}",
                                transaction.signature.chars().take(8).collect::<String>(),
                                pure_trade
                            )
                        );
                    }
                    pure_trade
                } else {
                    // Fallback: derive from balance changes and token-specific ATA rent recovery
                    let total_sol_received = transaction.sol_balance_change;
                    let pure_trade = total_sol_received - actual_ata_rent_impact;

                    // Log critical ATA calculations for verification (unless silent)
                    if !silent {
                        log(
                            LogTag::Transactions,
                            "ATA_RENT_FIX",
                            &format!(
                                "SELL tx {}: ata_closures={}, token_rent_recovered={:.9}, pure_trade_adjusted={}",
                                transaction.signature.chars().take(8).collect::<String>(),
                                ata_closures_count,
                                actual_ata_rent_impact,
                                ata_closures_count > 0
                            )
                        );
                    }

                    if self.debug_enabled {
                        log(
                            LogTag::Transactions,
                            "SELL_CALC",
                            &format!(
                                "Sell calculation: total_received={:.9}, token_rent_recovered={:.9}, pure_trade={:.9}, ata_ops={}c/{}d",
                                total_sol_received,
                                actual_ata_rent_impact,
                                pure_trade,
                                ata_creations_count,
                                ata_closures_count
                            )
                        );
                    }

                    pure_trade.max(0.0)
                }
            }
            _ => {
                // Fallback for unknown swap types
                (transaction.sol_balance_change.abs() - net_ata_rent_flow.abs()).max(0.0)
            }
        };

        // Cross-validation: Check if our calculation makes sense
        let validation_threshold = 0.0001; // 0.1 mSOL tolerance
        if pure_trade_amount < validation_threshold {
            if self.debug_enabled {
                log(
                    LogTag::Transactions,
                    "VALIDATION_WARN",
                    &format!(
                        "Pure trade amount very small ({:.9} SOL) - might be dust or calculation error",
                        pure_trade_amount
                    )
                );
            }

            // For very small amounts, fall back to using balance change directly
            // This handles edge cases where ATA calculations might be imprecise
            let fallback_amount = transaction.sol_balance_change.abs();

            if fallback_amount > validation_threshold {
                if self.debug_enabled {
                    log(
                        LogTag::Transactions,
                        "FALLBACK",
                        &format!("Using fallback calculation: {:.9} SOL", fallback_amount)
                    );
                }
            }
        }

        // Final amount calculation with multiple validation checks
        let final_sol_amount = if pure_trade_amount >= validation_threshold {
            pure_trade_amount
        } else {
            // Last resort: try to find meaningful SOL transfer in token_transfers
            let sol_transfer_amount = transaction.token_transfers
                .iter()
                .find(|transfer| transfer.mint == "So11111111111111111111111111111111111111112")
                .map(|transfer| transfer.amount.abs())
                .unwrap_or(0.0);

            if sol_transfer_amount >= validation_threshold {
                if self.debug_enabled {
                    log(
                        LogTag::Transactions,
                        "SOL_TRANSFER",
                        &format!("Using SOL transfer amount: {:.9} SOL", sol_transfer_amount)
                    );
                }
                sol_transfer_amount
            } else {
                // CRITICAL FIX: Use sol_amount_raw from transaction type instead of raw balance change
                // This prevents ATA rent from being included in position calculations
                if self.debug_enabled {
                    log(
                        LogTag::Transactions,
                        "FALLBACK_FIXED",
                        &format!(
                            "Using transaction type sol_amount: {:.9} SOL instead of raw balance change: {:.9} SOL",
                            sol_amount_raw,
                            transaction.sol_balance_change.abs()
                        )
                    );
                }
                sol_amount_raw.abs()
            }
        };

        // Calculate price using the pure trade amount
        let calculated_price_sol = if token_amount.abs() > 0.0 && final_sol_amount > 0.0 {
            final_sol_amount / token_amount.abs()
        } else {
            0.0
        };

        let token_symbol = if let Some(existing_symbol) = &transaction.token_symbol {
            // Use existing symbol if available
            existing_symbol.clone()
        } else if let Some(cached_symbol) = token_symbol_cache.get(&token_mint) {
            // Use cached symbol from database lookup
            cached_symbol.clone()
        } else {
            // Fallback to mint-based name
            if token_mint.len() >= 8 {
                format!("TOKEN_{}", &token_mint[..8])
            } else {
                format!("TOKEN_{}", token_mint)
            }
        };

        let blockchain_timestamp = if let Some(block_time) = transaction.block_time {
            DateTime::<Utc>::from_timestamp(block_time, 0).unwrap_or(transaction.timestamp)
        } else {
            transaction.timestamp
        };

        if self.debug_enabled && !silent {
            log(
                LogTag::Transactions,
                "FINAL_RESULT",
                &format!(
                    "Final calculation for {}: {:.9} SOL, price={:.12} SOL/token",
                    &transaction.signature[..8],
                    final_sol_amount,
                    calculated_price_sol
                )
            );
        }

        // Calculate effective amounts (excluding ATA rent but including fees)
        let (effective_sol_spent, effective_sol_received) = match swap_type.as_str() {
            "Buy" => {
                // For BUY: effective_sol_spent = pure trading amount (final_sol_amount already excludes ATA rent)
                let effective_spent = final_sol_amount;

                if self.debug_enabled && !silent {
                    log(
                        LogTag::Transactions,
                        "EFFECTIVE_BUY",
                        &format!(
                            "Buy {}: effective_spent={:.9} (pure trade amount)",
                            &transaction.signature[..8],
                            effective_spent
                        )
                    );
                }

                (effective_spent.max(0.0), 0.0)
            }
            "Sell" => {
                // For SELL: effective_sol_received = pure trading amount (final_sol_amount already excludes ATA rent)
                let effective_received = final_sol_amount;

                if self.debug_enabled && !silent {
                    log(
                        LogTag::Transactions,
                        "EFFECTIVE_SELL",
                        &format!(
                            "Sell {}: effective_received={:.9} (pure trade amount)",
                            &transaction.signature[..8],
                            effective_received
                        )
                    );
                }

                (0.0, effective_received.max(0.0))
            }
            _ => (0.0, 0.0),
        };

        Some(SwapPnLInfo {
            token_mint,
            token_symbol,
            swap_type: swap_type.clone(),
            sol_amount: final_sol_amount,
            token_amount,
            calculated_price_sol,
            timestamp: blockchain_timestamp,
            signature: transaction.signature.clone(),
            router, // Use the router we extracted from the transaction type
            fee_sol: transaction.fee_sol,
            ata_rents: ata_rents_display,
            effective_sol_spent,
            effective_sol_received,
            ata_created_count: token_ata_creations as u32,
            ata_closed_count: token_ata_closures as u32,
            slot: transaction.slot,
            status: self.determine_transaction_status(transaction, &swap_type, final_sol_amount),
        })
    }

    /// Determine transaction status based on success, error, and swap characteristics
    fn determine_transaction_status(
        &self,
        transaction: &Transaction,
        swap_type: &str,
        sol_amount: f64
    ) -> String {
        if !transaction.success {
            if let Some(ref error_msg) = transaction.error_message {
                if error_msg.contains("6001") {
                    "❌ Failed (6001)".to_string()
                } else if error_msg.contains("InstructionError") {
                    "❌ Failed (Instr)".to_string()
                } else {
                    "❌ Failed".to_string()
                }
            } else {
                "❌ Failed".to_string()
            }
        } else {
            // Transaction succeeded, check for abnormal characteristics
            if sol_amount < 0.00001 {
                // Very small amount, likely mostly fees
                "⚠️ Minimal".to_string()
            } else if sol_amount > 1.0 {
                // Very large swap
                "✅ Large".to_string()
            } else {
                "✅ Success".to_string()
            }
        }
    }
}
