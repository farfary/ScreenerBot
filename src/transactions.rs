/// Transactions Manager - Real-time background transaction monitoring and analysis
/// Tracks wallet transactions, caches data, detects transaction types, and integrates with positions
///
/// **All transaction analysis functionality is integrated directly into this module.**
/// This includes DEX detection, swap analysis, balance calculations, and type classification.
///
/// Debug Tool: Use `cargo run --bin main_debug` for comprehensive debugging,
/// monitoring, analysis, and performance testing of the transaction management system.

use std::collections::{ HashMap, HashSet };
use std::fs;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Notify;
use tokio::time::{ Duration, interval };
use chrono::{ DateTime, Utc };
use serde::{ Deserialize, Serialize };
use solana_sdk::{ pubkey::Pubkey, signature::Signature, commitment_config::CommitmentConfig };
use solana_transaction_status::EncodedConfirmedTransactionWithStatusMeta;
use std::str::FromStr;
use tabled::{ Table, Tabled, settings::{ Style, Modify, object::Rows, Alignment } };
use once_cell::sync::Lazy;
use rand;

use crate::logger::{ log, LogTag };
use crate::global::{ is_debug_transactions_enabled, read_configs, load_wallet_from_config };
use crate::rpc::get_rpc_client;
use crate::utils::{ get_wallet_address, safe_truncate };
use crate::errors::blockchain::{
    BlockchainError,
    parse_structured_solana_error,
    is_permanent_failure,
};
use crate::tokens::{
    get_token_decimals,
    get_token_decimals_safe,
    initialize_price_service,
    get_price,
    PriceOptions,
    TokenDatabase,
    types::PriceSourceType,
};
use crate::transactions_db::TransactionDatabase;
use crate::transactions_types::{
    Transaction,
    TransactionStatus,
    TransactionType,
    TransactionDirection,
    DeferredRetry,
    SwapAnalysis,
    FeeBreakdown,
    CachedAnalysis,
    SwapPnLInfo,
    TransactionStats,
    ANALYSIS_CACHE_VERSION,
    TokenTransfer,
    SolBalanceChange,
    TokenBalanceChange,
    InstructionInfo,
    TokenSwapInfo,
    AtaOperation,
    AtaOperationType,
    AtaAnalysis,
    ATA_RENT_COST_SOL,
    ATA_RENT_TOLERANCE_LAMPORTS,
    DEFAULT_COMPUTE_UNIT_PRICE,
    WSOL_MINT,
    RPC_BATCH_SIZE,
    PROCESS_BATCH_SIZE,
    TRANSACTION_DATA_BATCH_SIZE,
};
use crate::tokens::decimals::{ raw_to_ui_amount, lamports_to_sol, sol_to_lamports };

// Import the implementation methods
use crate::transactions_lib;

// =============================================================================
// CONFIGURATION CONSTANTS
// =============================================================================

const NORMAL_CHECK_INTERVAL_SECS: u64 = 3; // Normal transaction checking every 3 seconds (faster for position verification)

// =============================================================================
// TRANSACTIONS MANAGER
// =============================================================================

/// TransactionsManager - Main service for real-time transaction monitoring
pub struct TransactionsManager {
    pub wallet_pubkey: Pubkey,
    pub debug_enabled: bool,
    pub known_signatures: HashSet<String>,
    pub last_signature_check: Option<String>,
    pub total_transactions: u64,
    pub new_transactions_count: u64,

    // Token database integration
    pub token_database: Option<TokenDatabase>,

    // Transaction database for high-performance caching (replaces JSON files)
    pub transaction_database: Option<TransactionDatabase>,

    // Deferred retries for transactions that failed to process
    // This helps avoid losing verifications due to temporary RPC lag or network issues
    pub deferred_retries: HashMap<String, DeferredRetry>,
}

impl TransactionsManager {
    /// Create new TransactionsManager instance with token database integration
    pub async fn new(wallet_pubkey: Pubkey) -> Result<Self, String> {
        // Initialize token database
        let token_database = match TokenDatabase::new() {
            Ok(db) => Some(db),
            Err(e) => {
                log(
                    LogTag::Transactions,
                    "WARN",
                    &format!("Failed to initialize token database: {}", e)
                );
                None
            }
        };

        // Initialize price service
        if let Err(e) = initialize_price_service().await {
            log(
                LogTag::Transactions,
                "WARN",
                &format!("Failed to initialize price service: {}", e)
            );
        }

        // Initialize transaction database
        let transaction_database = match TransactionDatabase::new().await {
            Ok(db) => Some(db),
            Err(e) => {
                log(
                    LogTag::Transactions,
                    "WARN",
                    &format!("Failed to initialize transaction database: {}", e)
                );
                None
            }
        };

        Ok(Self {
            wallet_pubkey,
            debug_enabled: is_debug_transactions_enabled(),
            known_signatures: HashSet::new(),
            last_signature_check: None,
            total_transactions: 0,
            new_transactions_count: 0,
            token_database,
            transaction_database,
            deferred_retries: HashMap::new(),
        })
    }

    /// Load existing cached signatures to avoid re-processing
    /// When database is available, this loads from database; otherwise falls back to JSON files
    pub async fn initialize_known_signatures(&mut self) -> Result<(), String> {
        if let Some(ref db) = self.transaction_database {
            // Load signatures from known_signatures table into memory for fast lookup
            let signatures = db.get_all_known_signatures().await?;
            let count = signatures.len();

            // Clear and populate the known_signatures HashSet
            self.known_signatures.clear();
            for signature in signatures {
                self.known_signatures.insert(signature);
            }

            self.total_transactions = count as u64;

            if self.debug_enabled {
                log(
                    LogTag::Transactions,
                    "DB_INIT",
                    &format!("Loaded {} existing known signatures from database into memory", count)
                );
            }

            return Ok(());
        }

        // No database available - start with empty known signatures
        if self.debug_enabled {
            log(
                LogTag::Transactions,
                "INIT",
                "No database available - starting with empty transaction cache"
            );
        }

        Ok(())
    }

    /// Perform initial discovery and backfill of recent transactions on startup
    /// This ensures we have a complete picture of recent wallet activity
    pub async fn startup_transaction_discovery(&mut self) -> Result<(), String> {
        log(
            LogTag::Transactions,
            "STARTUP_DISCOVERY",
            "🔍 Starting comprehensive transaction discovery and backfill"
        );

        let rpc_client = get_rpc_client();
        let mut total_processed = 0;
        let mut total_cached = 0;
        let mut batch_number = 0;
        let mut before_signature: Option<String> = None;

        // Step 1: Check last 1000 transactions in batches of 1000
        loop {
            batch_number += 1;

            if self.debug_enabled {
                log(
                    LogTag::Transactions,
                    "STARTUP_DISCOVERY",
                    &format!("📦 Fetching batch {} (1000 signatures)", batch_number)
                );
            }

            // Fetch batch of signatures using rate-limited RPC
            let signatures = rpc_client
                .get_wallet_signatures_main_rpc(
                    &self.wallet_pubkey,
                    1000, // Batch size as requested
                    before_signature.as_deref()
                ).await
                .map_err(|e| format!("Failed to fetch signature batch {}: {}", batch_number, e))?;

            if signatures.is_empty() {
                if self.debug_enabled {
                    log(
                        LogTag::Transactions,
                        "STARTUP_DISCOVERY",
                        "📭 No more signatures found - discovery complete"
                    );
                }
                break;
            }

            let mut new_in_batch = 0;
            let mut known_found = false;

            // Process each signature in the batch
            for sig_info in &signatures {
                let signature = &sig_info.signature;
                total_processed += 1;

                // If we find a known signature, we can potentially stop
                if self.is_signature_known(signature).await {
                    known_found = true;

                    // If this is the first batch and we found known signatures early,
                    // we might have recent gaps to fill
                    if batch_number == 1 && new_in_batch > 0 {
                        if self.debug_enabled {
                            log(
                                LogTag::Transactions,
                                "STARTUP_DISCOVERY",
                                &format!("🔗 Found known signature after {} new ones - continuing to fill gaps", new_in_batch)
                            );
                        }
                        continue; // Continue processing this batch to fill gaps
                    }
                } else {
                    // New signature - add it to known signatures and cache it
                    self.add_known_signature(signature).await?;
                    new_in_batch += 1;
                    total_cached += 1;

                    // Process the transaction to cache its data
                    if let Err(e) = self.process_transaction(signature).await {
                        let error_msg = format!(
                            "Failed to process startup transaction {}: {}",
                            &signature[..8],
                            e
                        );
                        log(LogTag::Transactions, "WARN", &error_msg);

                        // Save failed state to database for startup processing
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
                                    "Failed to save startup transaction failure state for {}: {}",
                                    &signature[..8],
                                    db_err
                                )
                            );
                        }
                    }
                }

                // Update before_signature for next batch
                before_signature = Some(signature.clone());
            }

            if self.debug_enabled {
                log(
                    LogTag::Transactions,
                    "STARTUP_DISCOVERY",
                    &format!(
                        "📈 Batch {} complete: {} new, {} total processed",
                        batch_number,
                        new_in_batch,
                        total_processed
                    )
                );
            }

            // Stopping conditions
            if batch_number == 1 && new_in_batch == 0 {
                // First batch had no new transactions - we're caught up
                log(
                    LogTag::Transactions,
                    "STARTUP_DISCOVERY",
                    "✅ All recent transactions already known - no backfill needed"
                );
                break;
            } else if batch_number > 1 && new_in_batch == 0 && known_found {
                // Later batch with no new transactions and known signatures found - we're done
                log(
                    LogTag::Transactions,
                    "STARTUP_DISCOVERY",
                    "✅ Reached known transaction boundary - backfill complete"
                );
                break;
            } else if total_processed >= 10000 {
                // Safety limit to prevent excessive API calls
                log(
                    LogTag::Transactions,
                    "STARTUP_DISCOVERY",
                    "⚠️ Reached safety limit of 10,000 transactions - stopping discovery"
                );
                break;
            }

            // Rate limiting between batches
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        log(
            LogTag::Transactions,
            "STARTUP_DISCOVERY",
            &format!(
                "🎯 Discovery complete: processed {} signatures, cached {} new transactions across {} batches",
                total_processed,
                total_cached,
                batch_number
            )
        );

        // Update statistics
        self.new_transactions_count += total_cached as u64;

        Ok(())
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
    async fn cache_processed_transaction(&self, transaction: &Transaction) -> Result<(), String> {
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

    /// Update transaction status in database when status changes
    async fn update_transaction_status_in_db(
        &self,
        signature: &str,
        status: &TransactionStatus,
        success: bool,
        error_message: Option<&str>
    ) -> Result<(), String> {
        if let Some(ref db) = self.transaction_database {
            let status_str = match status {
                TransactionStatus::Pending => "Pending",
                TransactionStatus::Confirmed => "Confirmed",
                TransactionStatus::Finalized => "Finalized",
                TransactionStatus::Failed(ref msg) => "Failed",
            };

            db.update_transaction_status(signature, status_str, success, error_message).await?;

            if self.debug_enabled {
                log(
                    LogTag::Transactions,
                    "STATUS_UPDATE",
                    &format!(
                        "Updated transaction {} status to {} in database",
                        &signature[..8],
                        status_str
                    )
                );
            }
        }
        Ok(())
    }

    /// Save failed transaction state to database when processing fails
    async fn save_failed_transaction_state(
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
                .map_err(|e|
                    format!("Failed to fetch gap detection batch {}: {}", batch_number, e)
                )?;

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

    // Removed obsolete build_transaction_from_processed (old schema fields)

    /// Process a single transaction (database-first approach)
    pub async fn process_transaction(&mut self, signature: &str) -> Result<Transaction, String> {
        if self.debug_enabled {
            log(
                LogTag::Transactions,
                "PROCESS",
                &format!("Processing transaction: {}", &signature[..8])
            );
        }

        // Try to load from database first
        // (Processed transaction reconstruction removed - legacy schema)

        // Not in database, fetch fresh data from RPC
        if self.debug_enabled {
            log(
                LogTag::Transactions,
                "RPC_FETCH",
                &format!("Fetching new transaction: {}", &signature[..8])
            );
        }

        let rpc_client = get_rpc_client();
        let tx_data = match rpc_client.get_transaction_details_premium_rpc(signature).await {
            Ok(data) => {
                log(
                    LogTag::Rpc,
                    "SUCCESS",
                    &format!(
                        "Retrieved transaction details for {} from premium RPC",
                        &signature[..8]
                    )
                );
                data
            }
            Err(e) => {
                let error_msg = e.to_string();
                if error_msg.contains("not found") || error_msg.contains("no longer available") {
                    log(
                        LogTag::Rpc,
                        "NOT_FOUND",
                        &format!(
                            "Transaction {} not found on-chain (likely failed swap)",
                            &signature[..8]
                        )
                    );
                    return Err(format!("Transaction not found: {}", signature));
                } else {
                    log(
                        LogTag::Rpc,
                        "ERROR",
                        &format!("RPC error fetching {}: {}", &signature[..8], error_msg)
                    );
                    return Err(format!("Failed to fetch transaction details: {}", e));
                }
            }
        };

        // Create Transaction structure
        let timestamp = if let Some(block_time) = tx_data.block_time {
            DateTime::<Utc>::from_timestamp(block_time, 0).unwrap_or_else(|| Utc::now())
        } else {
            Utc::now()
        };

        let mut transaction = Transaction {
            signature: signature.to_string(),
            slot: Some(tx_data.slot),
            block_time: tx_data.block_time,
            timestamp,
            status: TransactionStatus::Finalized,
            transaction_type: TransactionType::Unknown,
            direction: TransactionDirection::Internal,
            success: tx_data.transaction.meta.as_ref().map_or(false, |meta| meta.err.is_none()),
            error_message: tx_data.transaction.meta
                .as_ref()
                .and_then(|meta| meta.err.as_ref())
                .map(|err| {
                    // Use structured error parsing for comprehensive error handling
                    let structured_error = parse_structured_solana_error(
                        &serde_json::to_value(err).unwrap_or_default(),
                        Some(&signature)
                    );
                    format!(
                        "[{}] {}: {} (code: {})",
                        structured_error.error_type_name(),
                        structured_error.error_name,
                        structured_error.description,
                        structured_error.error_code.map_or("N/A".to_string(), |c| c.to_string())
                    )
                }),
            fee_sol: tx_data.transaction.meta
                .as_ref()
                .map_or(0.0, |meta| lamports_to_sol(meta.fee)),
            sol_balance_change: 0.0,
            token_transfers: Vec::new(),
            raw_transaction_data: Some(serde_json::to_value(&tx_data).unwrap_or_default()),
            log_messages: tx_data.transaction.meta
                .as_ref()
                .map(|meta| {
                    match &meta.log_messages {
                        solana_transaction_status::option_serializer::OptionSerializer::Some(
                            logs,
                        ) => logs.clone(),
                        _ => Vec::new(),
                    }
                })
                .unwrap_or_default(),
            instructions: Vec::new(),
            sol_balance_changes: Vec::new(),
            token_balance_changes: Vec::new(),
            swap_analysis: None,
            position_impact: None,
            profit_calculation: None,
            fee_breakdown: None,
            ata_analysis: None,
            token_info: None,
            calculated_token_price_sol: None,
            price_source: None,
            token_symbol: None,
            token_decimals: None,
            last_updated: Utc::now(),
            cached_analysis: None,
        };

        // Analyze transaction type and extract details
        self.analyze_transaction(&mut transaction).await?;

        // Persist a snapshot for finalized transactions to avoid future re-analysis
        if
            matches!(transaction.status, TransactionStatus::Finalized) &&
            transaction.raw_transaction_data.is_some()
        {
            transaction.cached_analysis = Some(CachedAnalysis::from_transaction(&transaction));
        }

        // Store raw transaction in database first (required for foreign key)
        if let Err(e) = self.cache_transaction(&transaction).await {
            if self.debug_enabled {
                log(
                    LogTag::Transactions,
                    "WARN",
                    &format!("Failed to cache raw transaction: {}", e)
                );
            }
        }

        // Store processed transaction in database
        if let Err(e) = self.cache_processed_transaction(&transaction).await {
            if self.debug_enabled {
                log(
                    LogTag::Transactions,
                    "WARN",
                    &format!("Failed to cache processed transaction: {}", e)
                );
            }
        }

        Ok(transaction)
    }

    /// Analyze transaction to determine type and extract data
    async fn analyze_transaction(&mut self, transaction: &mut Transaction) -> Result<(), String> {
        if self.debug_enabled {
            log(
                LogTag::Transactions,
                "ANALYZE",
                &format!(
                    "Transaction {} - Type: {:?}, SOL change: {:.6}",
                    &transaction.signature[..8],
                    transaction.transaction_type,
                    transaction.sol_balance_change
                )
            );
        }

        // CRITICAL: Extract basic transaction info from raw data FIRST
        // This populates slot, block_time, log_messages, success, fee, etc.
        self.extract_basic_transaction_info(transaction).await?;

        // Analyze transaction type from raw data (now has log messages)
        self.analyze_transaction_type(transaction).await?;

        // Additional analysis based on transaction type
        match &transaction.transaction_type {
            TransactionType::SwapSolToToken { .. } | TransactionType::SwapTokenToSol { .. } => {
                // For swaps, build precise ATA analysis so PnL can exclude ATA rent correctly
                if let Err(e) = self.compute_and_set_ata_analysis(transaction).await {
                    if self.debug_enabled {
                        log(
                            LogTag::Transactions,
                            "WARN",
                            &format!(
                                "ATA analysis failed for swap {}: {}",
                                &transaction.signature[..8],
                                e
                            )
                        );
                    }
                }
            }
            _ => {
                // For other transaction types (transfers, unknown, etc.), no additional analysis needed
            }
        }

        Ok(())
    }

    /// If transaction has a valid cached analysis snapshot, hydrate derived fields from it
    pub fn try_hydrate_from_cached_analysis(&self, transaction: &mut Transaction) -> bool {
        if let Some(snapshot) = &transaction.cached_analysis {
            if snapshot.version == ANALYSIS_CACHE_VERSION && snapshot.hydrated {
                transaction.transaction_type = snapshot.transaction_type.clone();
                transaction.direction = snapshot.direction.clone();
                transaction.success = snapshot.success;
                transaction.fee_sol = snapshot.fee_sol;
                transaction.sol_balance_change = snapshot.sol_balance_change;
                transaction.token_transfers = snapshot.token_transfers.clone();
                transaction.sol_balance_changes = snapshot.sol_balance_changes.clone();
                transaction.token_balance_changes = snapshot.token_balance_changes.clone();
                transaction.ata_analysis = snapshot.ata_analysis.clone();
                transaction.token_info = snapshot.token_info.clone();
                transaction.calculated_token_price_sol = snapshot.calculated_token_price_sol;
                transaction.price_source = snapshot.price_source.clone();
                transaction.token_symbol = snapshot.token_symbol.clone();
                transaction.token_decimals = snapshot.token_decimals;
                // Restore the missing critical fields
                transaction.swap_analysis = snapshot.swap_analysis.clone();
                transaction.fee_breakdown = snapshot.fee_breakdown.clone();
                transaction.log_messages = snapshot.log_messages.clone();
                transaction.instructions = snapshot.instructions.clone();
                return true;
            }
        }
        false
    }

    /// Cache transaction to database only
    async fn cache_transaction(&self, transaction: &Transaction) -> Result<(), String> {
        if let Some(ref db) = self.transaction_database {
            // Store raw transaction data
            let status_string = match &transaction.status {
                TransactionStatus::Pending => "Pending",
                TransactionStatus::Confirmed => "Confirmed",
                TransactionStatus::Finalized => "Finalized",
                TransactionStatus::Failed(_) => "Failed",
            };

            let raw_data_string = match &transaction.raw_transaction_data {
                Some(value) =>
                    Some(
                        serde_json
                            ::to_string(value)
                            .map_err(|e|
                                format!("Failed to serialize raw transaction data: {}", e)
                            )?
                    ),
                None => None,
            };

            db.store_raw_transaction(
                &transaction.signature,
                transaction.slot,
                transaction.block_time,
                &transaction.timestamp,
                status_string,
                transaction.success,
                transaction.error_message.as_deref(),
                raw_data_string.as_deref()
            ).await?;

            if self.debug_enabled {
                log(
                    LogTag::Transactions,
                    "DB_CACHE",
                    &format!("Cached transaction {} to database", &transaction.signature[..8])
                );
            }

            Ok(())
        } else {
            Err("Database not available for caching".to_string())
        }
    }

    /// Fetch and analyze ALL wallet transactions from blockchain (unlimited)
    /// This method fetches comprehensive transaction history directly from the blockchain
    /// and processes each transaction with full analysis, bypassing the cache
    pub async fn fetch_all_wallet_transactions(&mut self) -> Result<Vec<Transaction>, String> {
        log(
            LogTag::Transactions,
            "INFO",
            &format!(
                "Starting comprehensive blockchain fetch for wallet {} (no limit)",
                self.wallet_pubkey
            )
        );

        // Initialize known signatures from cache so we can skip existing ones
        if let Err(e) = self.initialize_known_signatures().await {
            log(
                LogTag::Transactions,
                "ERROR",
                &format!("Failed to initialize known signatures: {}", e)
            );
        } else if self.debug_enabled {
            log(
                LogTag::Transactions,
                "INIT",
                &format!(
                    "Cache has {} transactions; will skip these during fetch",
                    self.known_signatures.len()
                )
            );
        }

        let rpc_client = get_rpc_client();
        let mut all_transactions = Vec::new();
        let mut before_signature = None;
        let batch_size = RPC_BATCH_SIZE; // Fetch in batches to avoid rate limits
        let mut total_fetched = 0;
        let mut total_skipped_cached = 0usize;

        log(
            LogTag::Transactions,
            "FETCH",
            "Fetching ALL transaction signatures from blockchain..."
        );

        // Fetch transaction signatures in batches until exhausted
        loop {
            let signatures = match
                rpc_client.get_wallet_signatures_main_rpc(
                    &self.wallet_pubkey,
                    batch_size,
                    before_signature.as_deref()
                ).await
            {
                Ok(sigs) => sigs,
                Err(e) => {
                    log(
                        LogTag::Transactions,
                        "ERROR",
                        &format!("Failed to fetch signatures batch: {}", e)
                    );
                    break;
                }
            };

            if signatures.is_empty() {
                log(
                    LogTag::Transactions,
                    "INFO",
                    "No more signatures available - completed full fetch"
                );
                break;
            }

            let batch_count = signatures.len();
            total_fetched += batch_count;

            // Build list of signatures we don't already have cached
            let mut signatures_to_process: Vec<String> = Vec::new();
            for s in &signatures {
                if self.known_signatures.contains(&s.signature) {
                    total_skipped_cached += 1;
                } else {
                    signatures_to_process.push(s.signature.clone());
                }
            }

            log(
                LogTag::Transactions,
                "FETCH",
                &format!(
                    "Fetched batch of {} signatures (total seen: {}), to process (not cached): {} | skipped cached: {}",
                    batch_count,
                    total_fetched,
                    signatures_to_process.len(),
                    total_skipped_cached
                )
            );

            for chunk in signatures_to_process.chunks(TRANSACTION_DATA_BATCH_SIZE) {
                let chunk_size = chunk.len();
                log(
                    LogTag::Transactions,
                    "BATCH",
                    &format!("Processing batch of {} transactions using batch RPC call", chunk_size)
                );

                // Use batch RPC call to fetch all transactions in this chunk at once
                match rpc_client.batch_get_transaction_details_premium_rpc(chunk).await {
                    Ok(batch_results) => {
                        log(
                            LogTag::Transactions,
                            "BATCH",
                            &format!(
                                "✅ Batch fetched {}/{} transactions successfully",
                                batch_results.len(),
                                chunk_size
                            )
                        );

                        // Process each transaction from the batch results
                        for (signature, encoded_tx) in batch_results {
                            if self.debug_enabled {
                                log(
                                    LogTag::Transactions,
                                    "BATCH",
                                    &format!(
                                        "Processing transaction from batch: {}",
                                        &signature[..8]
                                    )
                                );
                            }

                            match
                                self.process_transaction_from_encoded_data(
                                    &signature,
                                    encoded_tx
                                ).await
                            {
                                Ok(transaction) => {
                                    all_transactions.push(transaction);
                                    if self.debug_enabled {
                                        log(
                                            LogTag::Transactions,
                                            "BATCH",
                                            &format!(
                                                "✅ Processed transaction: {}",
                                                &signature[..8]
                                            )
                                        );
                                    }
                                }
                                Err(e) => {
                                    log(
                                        LogTag::Transactions,
                                        "WARN",
                                        &format!(
                                            "Failed to process transaction {}: {}",
                                            &signature[..8],
                                            e
                                        )
                                    );
                                }
                            }
                        }
                    }
                    Err(e) => {
                        log(
                            LogTag::Transactions,
                            "ERROR",
                            &format!("Failed to batch fetch {} transactions: {}", chunk_size, e)
                        );

                        // Fallback to individual processing if batch fails
                        log(
                            LogTag::Transactions,
                            "FALLBACK",
                            "Falling back to individual transaction processing"
                        );
                        for signature in chunk {
                            match self.process_transaction_direct(&signature).await {
                                Ok(transaction) => {
                                    all_transactions.push(transaction);
                                }
                                Err(e) => {
                                    log(
                                        LogTag::Transactions,
                                        "WARN",
                                        &format!(
                                            "Failed to process transaction {}: {}",
                                            &signature[..8],
                                            e
                                        )
                                    );
                                }
                            }
                        }
                    }
                }

                // Shorter delay between transaction batches
                if chunk_size == TRANSACTION_DATA_BATCH_SIZE {
                    tokio::time::sleep(Duration::from_millis(200)).await;
                }
            }

            // Set the before signature for the next batch
            if let Some(last_sig) = signatures.last() {
                before_signature = Some(last_sig.signature.clone());
            } else {
                // Empty signatures list - should not happen but handle safely
                log(
                    LogTag::Transactions,
                    "WARN",
                    "Empty signatures list in startup discovery batch"
                );
                break;
            }

            // Batch processing delay
            tokio::time::sleep(Duration::from_millis(500)).await; // Batch processing delay
        }

        log(
            LogTag::Transactions,
            "SUCCESS",
            &format!(
                "Completed comprehensive fetch: {} new transactions processed | {} cached skipped",
                all_transactions.len(),
                total_skipped_cached
            )
        );

        Ok(all_transactions)
    }

    /// Fetch and analyze limited number of wallet transactions from blockchain (for testing)
    /// This method fetches a specific number of transactions for testing purposes
    pub async fn fetch_limited_wallet_transactions(
        &mut self,
        max_count: usize
    ) -> Result<Vec<Transaction>, String> {
        log(
            LogTag::Transactions,
            "INFO",
            &format!(
                "Starting limited blockchain fetch for wallet {} (max {} transactions)",
                self.wallet_pubkey,
                max_count
            )
        );

        // Initialize known signatures from cache so we can skip existing ones
        if let Err(e) = self.initialize_known_signatures().await {
            log(
                LogTag::Transactions,
                "ERROR",
                &format!("Failed to initialize known signatures: {}", e)
            );
        } else if self.debug_enabled {
            log(
                LogTag::Transactions,
                "INIT",
                &format!(
                    "Cache has {} transactions; will skip these during limited fetch",
                    self.known_signatures.len()
                )
            );
        }

        let rpc_client = get_rpc_client();
        let mut all_transactions = Vec::new();
        let mut before_signature = None;
        let batch_size = RPC_BATCH_SIZE;
        let mut total_fetched = 0; // total signatures seen
        let mut total_skipped_cached = 0usize;
        let mut total_to_process = 0usize; // count of new (not cached) we attempted to process

        log(LogTag::Transactions, "FETCH", "Fetching transaction signatures from blockchain...");

        // Fetch transaction signatures in batches
        while total_to_process < max_count {
            let signatures = match
                rpc_client.get_wallet_signatures_main_rpc(
                    &self.wallet_pubkey,
                    batch_size,
                    before_signature.as_deref()
                ).await
            {
                Ok(sigs) => sigs,
                Err(e) => {
                    log(
                        LogTag::Transactions,
                        "ERROR",
                        &format!("Failed to fetch signatures batch: {}", e)
                    );
                    break;
                }
            };

            if signatures.is_empty() {
                log(LogTag::Transactions, "INFO", "No more signatures available");
                break;
            }

            let batch_count = signatures.len();
            total_fetched += batch_count;

            // Filter out cached signatures; only process unknown ones, but cap by remaining_needed
            let mut signatures_to_process: Vec<String> = Vec::new();
            for s in &signatures {
                if self.known_signatures.contains(&s.signature) {
                    total_skipped_cached += 1;
                } else if signatures_to_process.len() + total_to_process < max_count {
                    signatures_to_process.push(s.signature.clone());
                }
            }

            total_to_process += signatures_to_process.len();

            log(
                LogTag::Transactions,
                "FETCH",
                &format!(
                    "Fetched batch of {} signatures (seen total: {}), to process (not cached): {} (goal {}), skipped cached so far: {}",
                    batch_count,
                    total_fetched,
                    signatures_to_process.len(),
                    max_count,
                    total_skipped_cached
                )
            );

            for chunk in signatures_to_process.chunks(TRANSACTION_DATA_BATCH_SIZE) {
                let chunk_size = chunk.len();
                log(
                    LogTag::Transactions,
                    "BATCH",
                    &format!("Processing batch of {} transactions using batch RPC call", chunk_size)
                );

                // Use batch RPC call to fetch all transactions in this chunk at once
                match rpc_client.batch_get_transaction_details_premium_rpc(chunk).await {
                    Ok(batch_results) => {
                        log(
                            LogTag::Transactions,
                            "BATCH",
                            &format!(
                                "✅ Batch fetched {}/{} transactions successfully",
                                batch_results.len(),
                                chunk_size
                            )
                        );

                        // Process each transaction from the batch results
                        for (signature, encoded_tx) in batch_results {
                            if self.debug_enabled {
                                log(
                                    LogTag::Transactions,
                                    "BATCH",
                                    &format!(
                                        "Processing transaction from batch: {}",
                                        &signature[..8]
                                    )
                                );
                            }

                            match
                                self.process_transaction_from_encoded_data(
                                    &signature,
                                    encoded_tx
                                ).await
                            {
                                Ok(transaction) => {
                                    all_transactions.push(transaction);
                                    if self.debug_enabled {
                                        log(
                                            LogTag::Transactions,
                                            "BATCH",
                                            &format!(
                                                "✅ Processed transaction: {}",
                                                &signature[..8]
                                            )
                                        );
                                    }
                                }
                                Err(e) => {
                                    log(
                                        LogTag::Transactions,
                                        "WARN",
                                        &format!(
                                            "Failed to process transaction {}: {}",
                                            &signature[..8],
                                            e
                                        )
                                    );
                                }
                            }
                        }
                    }
                    Err(e) => {
                        log(
                            LogTag::Transactions,
                            "ERROR",
                            &format!("Failed to batch fetch {} transactions: {}", chunk_size, e)
                        );

                        // Fallback to individual processing if batch fails
                        log(
                            LogTag::Transactions,
                            "FALLBACK",
                            "Falling back to individual transaction processing"
                        );
                        for signature in chunk {
                            match self.process_transaction_direct(&signature).await {
                                Ok(transaction) => {
                                    all_transactions.push(transaction);
                                }
                                Err(e) => {
                                    log(
                                        LogTag::Transactions,
                                        "WARN",
                                        &format!(
                                            "Failed to process transaction {}: {}",
                                            &signature[..8],
                                            e
                                        )
                                    );
                                }
                            }
                        }
                    }
                }

                // Shorter delay between transaction batches
                if chunk_size == TRANSACTION_DATA_BATCH_SIZE {
                    tokio::time::sleep(Duration::from_millis(200)).await;
                }
            }

            // Set the before signature for the next batch
            if let Some(last_sig) = signatures.last() {
                before_signature = Some(last_sig.signature.clone());
            } else {
                // Empty signatures list - should not happen but handle safely
                log(LogTag::Transactions, "WARN", "Empty signatures list in gap backfill batch");
                break;
            }

            // Batch processing delay
            tokio::time::sleep(Duration::from_millis(500)).await;
        }

        log(
            LogTag::Transactions,
            "SUCCESS",
            &format!(
                "Completed limited fetch: {} new transactions processed | {} cached skipped",
                all_transactions.len(),
                total_skipped_cached
            )
        );

        Ok(all_transactions)
    }

    /// Process transaction directly from blockchain (bypassing cache)
    /// This is similar to process_transaction but forces fresh fetch from RPC
    pub async fn process_transaction_direct(
        &mut self,
        signature: &str
    ) -> Result<Transaction, String> {
        if self.debug_enabled {
            log(
                LogTag::Transactions,
                "DIRECT",
                &format!("Processing transaction directly from blockchain: {}", &signature[..8])
            );
        }

        // Create new transaction struct
        let mut transaction = Transaction {
            signature: signature.to_string(),
            slot: None,
            block_time: None,
            timestamp: Utc::now(),
            status: TransactionStatus::Confirmed,
            transaction_type: TransactionType::Unknown,
            direction: TransactionDirection::Internal,
            success: false,
            error_message: None,
            fee_sol: 0.0,
            sol_balance_change: 0.0,
            token_transfers: Vec::new(),
            raw_transaction_data: None,
            log_messages: Vec::new(),
            instructions: Vec::new(),
            sol_balance_changes: Vec::new(),
            token_balance_changes: Vec::new(),
            swap_analysis: None,
            position_impact: None,
            profit_calculation: None,
            fee_breakdown: None,
            ata_analysis: None,
            token_info: None,
            calculated_token_price_sol: None,
            price_source: None,
            token_symbol: None,
            token_decimals: None,
            last_updated: Utc::now(),
            cached_analysis: None,
        };

        // Fetch fresh transaction data from blockchain
        self.fetch_transaction_data(&mut transaction).await?;

        // Perform comprehensive analysis
        self.analyze_transaction(&mut transaction).await?;
        // Defensive: if raw data has block_time and no error, treat as finalized
        if transaction.block_time.is_some() && transaction.success {
            transaction.status = TransactionStatus::Finalized;

            // Update status in database
            if
                let Err(e) = self.update_transaction_status_in_db(
                    &transaction.signature,
                    &transaction.status,
                    transaction.success,
                    transaction.error_message.as_deref()
                ).await
            {
                log(
                    LogTag::Transactions,
                    "WARN",
                    &format!("Failed to update transaction status in DB: {}", e)
                );
            }
        }

        // Persist a snapshot for finalized transactions to avoid future re-analysis
        if
            matches!(transaction.status, TransactionStatus::Finalized) &&
            transaction.raw_transaction_data.is_some()
        {
            transaction.cached_analysis = Some(CachedAnalysis::from_transaction(&transaction));
        }

        // Cache the processed transaction
        self.cache_transaction(&transaction).await?;

        // Update known signatures
        self.known_signatures.insert(signature.to_string());

        Ok(transaction)
    }

    /// Process transaction from encoded data (used for batch processing)
    /// This is optimized for batch processing where we already have the transaction data
    async fn process_transaction_from_encoded_data(
        &mut self,
        signature: &str,
        encoded_tx: solana_transaction_status::EncodedConfirmedTransactionWithStatusMeta
    ) -> Result<Transaction, String> {
        if self.debug_enabled {
            log(
                LogTag::Transactions,
                "BATCH_PROCESS",
                &format!("Processing transaction from batch data: {}", &signature[..8])
            );
        }

        // Create new transaction struct
        let mut transaction = Transaction {
            signature: signature.to_string(),
            slot: None,
            block_time: None,
            timestamp: Utc::now(),
            status: TransactionStatus::Confirmed,
            transaction_type: TransactionType::Unknown,
            direction: TransactionDirection::Internal,
            success: false,
            error_message: None,
            fee_sol: 0.0,
            sol_balance_change: 0.0,
            token_transfers: Vec::new(),
            raw_transaction_data: None,
            log_messages: Vec::new(),
            instructions: Vec::new(),
            sol_balance_changes: Vec::new(),
            token_balance_changes: Vec::new(),
            swap_analysis: None,
            position_impact: None,
            profit_calculation: None,
            fee_breakdown: None,
            ata_analysis: None,
            token_info: None,
            calculated_token_price_sol: None,
            price_source: None,
            token_symbol: None,
            token_decimals: None,
            last_updated: Utc::now(),
            cached_analysis: None,
        };

        // Convert encoded transaction to raw data format
        let raw_data = serde_json
            ::to_value(&encoded_tx)
            .map_err(|e| format!("Failed to serialize encoded transaction data: {}", e))?;

        transaction.raw_transaction_data = Some(raw_data);

        // Perform comprehensive analysis
        self.analyze_transaction(&mut transaction).await?;
        // Defensive: if raw data has block_time and no error, treat as finalized
        if transaction.block_time.is_some() && transaction.success {
            transaction.status = TransactionStatus::Finalized;

            // Update status in database
            if
                let Err(e) = self.update_transaction_status_in_db(
                    &transaction.signature,
                    &transaction.status,
                    transaction.success,
                    transaction.error_message.as_deref()
                ).await
            {
                log(
                    LogTag::Transactions,
                    "WARN",
                    &format!("Failed to update transaction status in DB: {}", e)
                );
            }
        }

        // Persist a snapshot for finalized transactions to avoid future re-analysis
        if
            matches!(transaction.status, TransactionStatus::Finalized) &&
            transaction.raw_transaction_data.is_some()
        {
            transaction.cached_analysis = Some(CachedAnalysis::from_transaction(&transaction));
        }

        // Cache the processed transaction
        self.cache_transaction(&transaction).await?;

        // Update known signatures
        self.known_signatures.insert(signature.to_string());

        Ok(transaction)
    }

    /// Recalculate analysis for existing transaction without re-fetching raw data
    /// This preserves all raw blockchain data and only updates calculated fields
    pub async fn recalculate_transaction_analysis(
        &mut self,
        transaction: &mut Transaction
    ) -> Result<(), String> {
        if self.debug_enabled {
            log(
                LogTag::Transactions,
                "RECALC",
                &format!("Recalculating analysis for transaction: {}", &transaction.signature[..8])
            );
        }

        // Update timestamp
        transaction.last_updated = Utc::now();

        // Reset all calculated fields to default values (preserve raw data)
        // Reset basic extracted fields
        transaction.slot = None;
        transaction.block_time = None;
        transaction.success = false;
        transaction.error_message = None;
        transaction.fee_sol = 0.0;
        transaction.log_messages = Vec::new();
        transaction.instructions = Vec::new();

        // Reset analysis fields
        transaction.transaction_type = TransactionType::Unknown;
        transaction.direction = TransactionDirection::Internal;
        transaction.sol_balance_change = 0.0;
        transaction.token_transfers = Vec::new();
        transaction.swap_analysis = None;
        transaction.position_impact = None;
        transaction.profit_calculation = None;
        transaction.fee_breakdown = None;
        transaction.ata_analysis = None; // CRITICAL: Reset ATA analysis for recalculation
        transaction.token_info = None;
        transaction.calculated_token_price_sol = None;
        transaction.price_source = None;
        transaction.token_symbol = None;
        transaction.token_decimals = None;

        // Recalculate all analysis using existing raw data
        if transaction.raw_transaction_data.is_some() {
            // Re-run the comprehensive analysis using cached raw data
            self.analyze_transaction(&mut *transaction).await?;

            if self.debug_enabled {
                log(
                    LogTag::Transactions,
                    "RECALC",
                    &format!(
                        "✅ Analysis recalculated: {} -> {:?}",
                        &transaction.signature[..8],
                        transaction.transaction_type
                    )
                );
            }
        } else {
            log(
                LogTag::Transactions,
                "WARNING",
                &format!(
                    "No raw transaction data available for {}, skipping recalculation",
                    &transaction.signature[..8]
                )
            );
        }

        Ok(())
    }

    /// Get transaction statistics
    pub fn get_stats(&self) -> TransactionStats {
        TransactionStats {
            total_transactions: self.total_transactions,
            new_transactions_count: self.new_transactions_count,
            known_signatures_count: self.known_signatures.len() as u64,
        }
    }

    /// Get known signatures count (for testing)
    pub fn known_signatures(&self) -> &HashSet<String> {
        &self.known_signatures
    }

    /// Get recent transactions from cache (for orphaned position recovery)
    pub async fn get_recent_transactions(
        &mut self,
        limit: usize
    ) -> Result<Vec<Transaction>, String> {
        // Database-only implementation using optimized batch retrieval
        if let Some(db) = &self.transaction_database {
            // Use the new optimized batch function to avoid N+1 queries
            let mut transactions = db.get_recent_transactions_batch(limit).await?;

            // Hydrate transactions that need analysis but only if they have raw data
            let mut hydrated_count = 0;
            for tx in &mut transactions {
                // Only try to hydrate if we have raw data and the transaction type is unknown
                if
                    matches!(tx.transaction_type, TransactionType::Unknown) &&
                    tx.raw_transaction_data.is_some()
                {
                    // Try to hydrate from cached analysis first
                    if !self.try_hydrate_from_cached_analysis(tx) {
                        // If no cached analysis, recalculate (but don't fetch from RPC)
                        if let Err(e) = self.recalculate_transaction_analysis(tx).await {
                            if self.debug_enabled {
                                log(
                                    LogTag::Transactions,
                                    "WARN",
                                    &format!(
                                        "Failed to recalculate analysis for {}: {}",
                                        &tx.signature[..8],
                                        e
                                    )
                                );
                            }
                        } else {
                            hydrated_count += 1;
                        }
                    } else {
                        hydrated_count += 1;
                    }
                }
            }

            if self.debug_enabled && hydrated_count > 0 {
                log(
                    LogTag::Transactions,
                    "HYDRATE",
                    &format!("Hydrated {} transactions from {} requested", hydrated_count, limit)
                );
            }

            Ok(transactions)
        } else {
            Err("Transaction database unavailable".into())
        }
    }

    /// Get recent swap transactions from the last 100 transactions
    /// Returns up to N transactions that are swaps (full Transaction objects)
    pub async fn get_recent_swaps(&mut self, limit: usize) -> Result<Vec<Transaction>, String> {
        // Get last 100 transactions and filter for swaps
        let recent_transactions = self.get_recent_transactions(100).await?;

        let swap_transactions: Vec<Transaction> = recent_transactions
            .into_iter()
            .filter(|tx| self.is_swap_transaction(tx))
            .take(limit)
            .collect();

        if self.debug_enabled {
            log(
                LogTag::Transactions,
                "RECENT_SWAPS",
                &format!(
                    "Found {} swap transactions from last 100 transactions",
                    swap_transactions.len()
                )
            );
        }

        Ok(swap_transactions)
    }

    /// Get transaction data from cache first, fetch from blockchain only if needed
    async fn get_or_fetch_transaction_data(
        &self,
        signature: &str
    ) -> Result<serde_json::Value, String> {
        // Try database first
        if let Some(db) = &self.transaction_database {
            if let Some(raw) = db.get_raw_transaction(signature).await? {
                if let Some(json_str) = raw.raw_transaction_data {
                    if self.debug_enabled {
                        log(LogTag::Transactions, "DB_HIT", &format!("Raw {}", &signature[..8]));
                    }
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&json_str) {
                        return Ok(val);
                    }
                }
            }
        }
        if self.debug_enabled {
            log(LogTag::Transactions, "DB_MISS", &format!("RPC fetch {}", &signature[..8]));
        }

        let rpc_client = get_rpc_client();
        let tx_details = rpc_client
            .get_transaction_details(signature).await
            .map_err(|e| format!("RPC error: {}", e))?;

        // Convert TransactionDetails to JSON for storage
        let raw_data = serde_json
            ::to_value(tx_details)
            .map_err(|e| format!("Failed to serialize transaction data: {}", e))?;

        Ok(raw_data)
    }

    /// Fetch full transaction data from RPC (now uses cache-first strategy)
    async fn fetch_transaction_data(&self, transaction: &mut Transaction) -> Result<(), String> {
        transaction.raw_transaction_data = Some(
            self.get_or_fetch_transaction_data(&transaction.signature).await?
        );
        Ok(())
    }
}

// =============================================================================
// BACKGROUND SERVICE
// =============================================================================

/// Start the transactions manager background service
/// Simple pattern following other bot services
pub async fn start_transactions_service(shutdown: Arc<Notify>) {
    log(LogTag::Transactions, "INFO", "TransactionsManager service starting...");

    // Load wallet address fresh each time
    let wallet_address = match load_wallet_address_from_config().await {
        Ok(address) => address,
        Err(e) => {
            log(LogTag::Transactions, "ERROR", &format!("Failed to load wallet address: {}", e));
            return;
        }
    };

    // Create TransactionsManager instance
    let mut manager = match TransactionsManager::new(wallet_address).await {
        Ok(manager) => manager,
        Err(e) => {
            log(
                LogTag::Transactions,
                "ERROR",
                &format!("Failed to create TransactionsManager: {}", e)
            );
            return;
        }
    };

    // Initialize known signatures
    if let Err(e) = manager.initialize_known_signatures().await {
        log(LogTag::Transactions, "ERROR", &format!("Failed to initialize: {}", e));
        return;
    }

    log(
        LogTag::Transactions,
        "INFO",
        &format!(
            "TransactionsManager initialized for wallet: {} (known transactions: {})",
            wallet_address,
            manager.known_signatures.len()
        )
    );

    // Perform startup transaction discovery and backfill
    if let Err(e) = manager.startup_transaction_discovery().await {
        log(LogTag::Transactions, "ERROR", &format!("Failed to complete startup discovery: {}", e));
        // Don't return here - continue with normal operation even if discovery fails
    }

    // CRITICAL: Initialize global transaction manager for positions manager integration
    if let Err(e) = initialize_global_transaction_manager(wallet_address).await {
        log(
            LogTag::Transactions,
            "ERROR",
            &format!("Failed to initialize global transaction manager: {}", e)
        );
        return;
    }

    // Position verification and management is now handled by the positions manager service
    log(
        LogTag::Transactions,
        "STARTUP",
        "✅ Transaction service started - positions managed separately"
    );

    // Signal that position recalculation is complete - traders can now start
    crate::global::POSITION_RECALCULATION_COMPLETE.store(true, std::sync::atomic::Ordering::SeqCst);
    log(
        LogTag::Transactions,
        "STARTUP",
        "🟢 Position recalculation complete - traders can now operate"
    );

    // Enhanced dual-loop monitoring system with gap detection
    let mut next_normal_check =
        tokio::time::Instant::now() + Duration::from_secs(NORMAL_CHECK_INTERVAL_SECS);
    let mut next_gap_check = tokio::time::Instant::now() + Duration::from_secs(300); // Gap detection every 5 minutes

    loop {
        tokio::select! {
            _ = shutdown.notified() => {
                log(LogTag::Transactions, "INFO", "TransactionsManager service shutting down");
                break;
            }
            _ = tokio::time::sleep_until(next_normal_check) => {
                // Normal transaction monitoring every 10 seconds
                match do_monitoring_cycle(&mut manager).await {
                    Ok((new_transaction_count, _)) => {
                        if manager.debug_enabled {
                            log(LogTag::Transactions, "SUCCESS", &format!(
                                "Found {} swap transactions",
                                new_transaction_count
                            ));
                        }
                    }
                    Err(e) => {
                        log(LogTag::Transactions, "ERROR", &format!("Normal monitoring error: {}", e));
                    }
                }
                next_normal_check = tokio::time::Instant::now() + Duration::from_secs(NORMAL_CHECK_INTERVAL_SECS);
            }
            _ = tokio::time::sleep_until(next_gap_check) => {
                // Periodic gap detection and backfill every 5 minutes
                match manager.check_and_backfill_gaps().await {
                    Ok(backfilled_count) => {
                        if backfilled_count > 0 {
                            log(LogTag::Transactions, "GAP_DETECTION", &format!(
                                "✅ Gap detection complete - backfilled {} transactions",
                                backfilled_count
                            ));
                        } else if manager.debug_enabled {
                            log(LogTag::Transactions, "GAP_DETECTION", "✅ No gaps found");
                        }
                    }
                    Err(e) => {
                        log(LogTag::Transactions, "ERROR", &format!("Gap detection error: {}", e));
                    }
                }
                next_gap_check = tokio::time::Instant::now() + Duration::from_secs(300);

                // Periodic cleanup of expired deferred retries every 5 minutes (with gap detection)
                if let Err(e) = manager.cleanup_expired_deferred_retries().await {
                    log(LogTag::Transactions, "ERROR", &format!("Deferred retries cleanup error: {}", e));
                }
            }
        }
    }

    log(LogTag::Transactions, "INFO", "TransactionsManager service stopped");
}

/// Perform one normal monitoring cycle and return number of new transactions found
async fn do_monitoring_cycle(manager: &mut TransactionsManager) -> Result<(usize, bool), String> {
    // Check for new transactions
    let new_signatures = manager.check_new_transactions().await?;
    let new_transaction_count = new_signatures.len();

    // Process new transactions
    for signature in new_signatures {
        match manager.process_transaction(&signature).await {
            Ok(_) => {
                // Successfully processed
            }
            Err(e) => {
                log(
                    LogTag::Transactions,
                    "WARN",
                    &format!("Failed to process transaction {}: {}", &signature[..8], e)
                );

                // CRITICAL: Save failed transaction state to database
                if let Err(db_err) = manager.save_failed_transaction_state(&signature, &e).await {
                    log(
                        LogTag::Transactions,
                        "ERROR",
                        &format!(
                            "Failed to save failed transaction state for {}: {}",
                            &signature[..8],
                            db_err
                        )
                    );
                }

                // Create deferred retry for failed processing
                let retry = DeferredRetry {
                    signature: signature.clone(),
                    next_retry_at: Utc::now() + chrono::Duration::minutes(5),
                    remaining_attempts: 3,
                    current_delay_secs: 300, // 5 minutes
                    last_error: Some(e),
                };

                if let Err(retry_err) = manager.store_deferred_retry(&retry).await {
                    log(
                        LogTag::Transactions,
                        "ERROR",
                        &format!(
                            "Failed to store deferred retry for {}: {}",
                            &signature[..8],
                            retry_err
                        )
                    );
                }
            }
        }
    }

    // Check and verify position transactions
    // Position verification now handled by PositionsManager
    // PositionsManager automatically processes verified transactions

    // Log stats periodically
    // Update statistics
    if manager.debug_enabled {
        let stats = manager.get_stats();
        log(
            LogTag::Transactions,
            "STATS",
            &format!(
                "Total: {}, New: {}, Cached: {}",
                stats.total_transactions,
                stats.new_transactions_count,
                stats.known_signatures_count
            )
        );
    }

    Ok((new_transaction_count, false)) // Second value no longer used in simplified system
}

/// Load wallet address from config
pub async fn load_wallet_address_from_config() -> Result<Pubkey, String> {
    let wallet_address_str = get_wallet_address().map_err(|e|
        format!("Failed to get wallet address: {}", e)
    )?;

    Pubkey::from_str(&wallet_address_str).map_err(|e|
        format!("Invalid wallet address format: {}", e)
    )
}

impl Default for FeeBreakdown {
    fn default() -> Self {
        Self {
            transaction_fee: 0.0,
            router_fee: 0.0,
            platform_fee: 0.0,
            compute_units_consumed: 0,
            compute_unit_price: 0,
            priority_fee: 0.0,
            total_fees: 0.0,
            fee_percentage: 0.0,
        }
    }
}

// =============================================================================
// PUBLIC API FOR INTEGRATION
// =============================================================================

/// Global transaction manager instance for monitoring
pub static GLOBAL_TRANSACTION_MANAGER: once_cell::sync::Lazy<std::sync::Arc<tokio::sync::Mutex<Option<TransactionsManager>>>> = once_cell::sync::Lazy::new(
    || std::sync::Arc::new(tokio::sync::Mutex::new(None))
);

/// Initialize global transaction manager for monitoring
pub async fn initialize_global_transaction_manager(wallet_pubkey: Pubkey) -> Result<(), String> {
    // Use try_lock to prevent deadlock with timeout
    match tokio::time::timeout(Duration::from_secs(5), GLOBAL_TRANSACTION_MANAGER.lock()).await {
        Ok(mut manager_guard) => {
            if manager_guard.is_some() {
                return Ok(());
            }

            let manager = TransactionsManager::new(wallet_pubkey).await?;
            *manager_guard = Some(manager);

            log(
                LogTag::Transactions,
                "INIT",
                "Global transaction manager initialized for monitoring"
            );
            Ok(())
        }
        Err(_) => {
            let error_msg = "Failed to acquire global transaction manager lock within timeout";
            log(LogTag::Transactions, "ERROR", error_msg);
            Err(error_msg.to_string())
        }
    }
}

/// Get global transaction manager instance
pub async fn get_global_transaction_manager() -> Option<std::sync::Arc<tokio::sync::Mutex<Option<TransactionsManager>>>> {
    Some(GLOBAL_TRANSACTION_MANAGER.clone())
}

/// Get transaction by signature (for positions.rs integration) - cache-first approach with status validation
/// CRITICAL: Only returns transactions that are in Finalized or Confirmed status
/// Pending/Failed transactions trigger fresh RPC fetch or return None
pub async fn get_transaction(signature: &str) -> Result<Option<Transaction>, String> {
    // Use global manager and database only
    let debug = is_debug_transactions_enabled();
    if debug {
        log(LogTag::Transactions, "GET_TX", &format!("{}", &signature[..8]));
    }

    if let Some(global) = get_global_transaction_manager().await {
        // Add timeout to prevent hanging on slow lock acquisition (similar to priority transactions)
        match tokio::time::timeout(Duration::from_secs(10), global.lock()).await {
            Ok(manager_guard) => {
                if let Some(manager) = manager_guard.as_ref() {
                    if let Some(db) = &manager.transaction_database {
                        if let Some(raw) = db.get_raw_transaction(signature).await? {
                            // Build Transaction skeleton and recalc analysis
                            let mut tx = Transaction {
                                signature: raw.signature.clone(),
                                slot: raw.slot,
                                block_time: raw.block_time,
                                timestamp: DateTime::parse_from_rfc3339(&raw.timestamp)
                                    .map(|dt| dt.with_timezone(&Utc))
                                    .unwrap_or_else(|_| Utc::now()),
                                status: match raw.status.as_str() {
                                    "Finalized" => TransactionStatus::Finalized,
                                    "Confirmed" => TransactionStatus::Confirmed,
                                    "Pending" => TransactionStatus::Pending,
                                    s if s.starts_with("Failed") =>
                                        TransactionStatus::Failed(
                                            raw.error_message
                                                .clone()
                                                .unwrap_or_else(|| s.to_string())
                                        ),
                                    _ => TransactionStatus::Pending,
                                },
                                transaction_type: TransactionType::Unknown,
                                direction: TransactionDirection::Internal,
                                success: raw.success,
                                error_message: raw.error_message.clone(),
                                fee_sol: 0.0,
                                sol_balance_change: 0.0,
                                token_transfers: Vec::new(),
                                raw_transaction_data: raw.raw_transaction_data
                                    .as_ref()
                                    .and_then(|s| serde_json::from_str(s).ok()),
                                log_messages: Vec::new(),
                                instructions: Vec::new(),
                                sol_balance_changes: Vec::new(),
                                token_balance_changes: Vec::new(),
                                swap_analysis: None,
                                position_impact: None,
                                profit_calculation: None,
                                fee_breakdown: None,
                                ata_analysis: None,
                                token_info: None,
                                calculated_token_price_sol: None,
                                price_source: None,
                                token_symbol: None,
                                token_decimals: None,
                                last_updated: Utc::now(),
                                cached_analysis: None,
                            };

                            // Always try to recalculate analysis for complete transaction data
                            match tokio::time::timeout(Duration::from_secs(2), global.lock()).await {
                                Ok(mut guard) => {
                                    if let Some(manager_mut) = guard.as_mut() {
                                        let _ = manager_mut.recalculate_transaction_analysis(
                                            &mut tx
                                        ).await;
                                        if debug {
                                            log(
                                                LogTag::Transactions,
                                                "ANALYSIS_COMPLETE",
                                                &format!(
                                                    "Completed analysis for {} - type: {:?}",
                                                    &signature[..8],
                                                    tx.transaction_type
                                                )
                                            );
                                        }
                                    }
                                }
                                Err(_) => {
                                    // Manager is busy - return transaction without forced analysis
                                    // Creating new instances violates architecture - never do this!
                                    if debug {
                                        log(
                                            LogTag::Transactions,
                                            "MANAGER_BUSY_SKIP",
                                            &format!(
                                                "Manager busy - returning transaction {} without force analysis to avoid creating unauthorized instance",
                                                &signature[..8]
                                            )
                                        );
                                    }

                                    // Transaction will be analyzed later when manager becomes available
                                    // This preserves architectural integrity
                                }
                            }

                            return Ok(Some(tx));
                        }
                    }
                }
            }
            Err(_) => {
                // Timeout occurred - DO NOT create temporary manager (architecture violation)
                log(
                    LogTag::Transactions,
                    "LOCK_TIMEOUT",
                    &format!(
                        "Global transaction manager busy - returning None for {} to preserve architecture",
                        &signature[..8]
                    )
                );

                // Return None instead of creating unauthorized instance
                // The caller should retry later when the global manager is available
                return Ok(None);
            }
        }
    } else {
        log(
            LogTag::Transactions,
            "NO_GLOBAL_MANAGER",
            &format!("No global transaction manager available for {}", &signature[..8])
        );
    }

    Ok(None)
}

/// Get transaction statistics
pub async fn get_transaction_stats() -> TransactionStats {
    // Default stats - would integrate with global manager
    TransactionStats {
        total_transactions: 0,
        new_transactions_count: 0,
        known_signatures_count: 0,
    }
}

/// Get swap transactions for a specific token mint (OPTIMIZED for phantom cleanup)
/// This uses efficient database filtering instead of scanning all transactions
pub async fn get_swap_transactions_for_token(
    token_mint: &str,
    swap_type: Option<&str>, // "Sell", "Buy", or None for both
    limit: Option<usize>
) -> Result<Vec<SwapPnLInfo>, String> {
    log(
        LogTag::Transactions,
        "FILTER_START",
        &format!(
            "Getting swap transactions for token {} (type: {:?}, limit: {:?})",
            token_mint,
            swap_type,
            limit
        )
    );

    // Create temporary manager for deadlock safety
    let wallet_address = load_wallet_address_from_config().await?;
    let temp_manager = TransactionsManager::new(wallet_address).await?;

    // Get filtered signatures from database efficiently
    let signatures = if let Some(ref db) = temp_manager.transaction_database {
        db.get_swap_signatures_for_token(token_mint, swap_type, limit).await?
    } else {
        log(
            LogTag::Transactions,
            "WARN",
            "No database available for filtering, falling back to empty result"
        );
        return Ok(Vec::new());
    };

    log(
        LogTag::Transactions,
        "FILTER_SIGNATURES",
        &format!("Found {} filtered signatures for token {}", signatures.len(), token_mint)
    );

    // Convert filtered signatures to SwapPnLInfo
    let mut swap_transactions = Vec::new();
    let token_symbol_cache = std::collections::HashMap::new();

    for (index, signature) in signatures.iter().enumerate() {
        if let Ok(Some(tx)) = get_transaction(signature).await {
            if
                let Some(swap_info) = temp_manager.convert_to_swap_pnl_info(
                    &tx,
                    &token_symbol_cache,
                    true
                )
            {
                // Double-check the token mint matches (in case database filtering wasn't exact)
                if swap_info.token_mint == token_mint {
                    swap_transactions.push(swap_info);
                }
            }
        }

        // Log progress for larger sets
        if signatures.len() > 10 && (index + 1) % 5 == 0 {
            log(
                LogTag::Transactions,
                "FILTER_PROGRESS",
                &format!(
                    "Processed {}/{} filtered signatures for {}",
                    index + 1,
                    signatures.len(),
                    token_mint
                )
            );
        }
    }

    log(
        LogTag::Transactions,
        "FILTER_COMPLETE",
        &format!(
            "Converted {} swap transactions for token {} (from {} signatures)",
            swap_transactions.len(),
            token_mint,
            signatures.len()
        )
    );

    Ok(swap_transactions)
}
