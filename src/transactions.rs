use chrono::{ DateTime, Utc };
use once_cell::sync::Lazy;
use rand;
use serde::{ Deserialize, Serialize };
use solana_sdk::{ commitment_config::CommitmentConfig, pubkey::Pubkey, signature::Signature };
use solana_transaction_status::EncodedConfirmedTransactionWithStatusMeta;
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
use std::str::FromStr;
use std::sync::Arc;
use tabled::{ settings::{ object::Rows, Alignment, Modify, Style }, Table, Tabled };
use tokio::sync::Notify;
use tokio::time::{ interval, Duration };

use crate::errors::blockchain::{
    is_permanent_failure,
    parse_structured_solana_error,
    BlockchainError,
};
use crate::global::{ is_debug_transactions_enabled, load_wallet_from_config, read_configs };
use crate::logger::{ log, LogTag };
use crate::rpc::get_rpc_client;
use crate::tokens::decimals::{ lamports_to_sol, raw_to_ui_amount, sol_to_lamports };
use crate::tokens::{
    get_price,
    get_token_decimals,
    get_token_decimals_safe,
    initialize_price_service,
    types::PriceSourceType,
    PriceOptions,
    TokenDatabase,
};
use crate::transactions_db::TransactionDatabase;
use crate::transactions_types::{
    AtaAnalysis,
    AtaOperation,
    AtaOperationType,
    CachedAnalysis,
    DeferredRetry,
    FeeBreakdown,
    InstructionInfo,
    SolBalanceChange,
    SwapAnalysis,
    SwapPnLInfo,
    TokenBalanceChange,
    TokenSwapInfo,
    TokenTransfer,
    Transaction,
    TransactionDirection,
    TransactionStats,
    TransactionStatus,
    TransactionType,
    ANALYSIS_CACHE_VERSION,
    ATA_RENT_COST_SOL,
    ATA_RENT_TOLERANCE_LAMPORTS,
    DEFAULT_COMPUTE_UNIT_PRICE,
    PROCESS_BATCH_SIZE,
    RPC_BATCH_SIZE,
    TRANSACTION_DATA_BATCH_SIZE,
    WSOL_MINT,
};
use crate::utils::{ get_wallet_address, safe_truncate };

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

    /// Process a single transaction (database-first approach)
    pub async fn process_transaction(&mut self, signature: &str) -> Result<Transaction, String> {
        if self.debug_enabled {
            log(
                LogTag::Transactions,
                "PROCESS",
                &format!("Processing transaction: {}", &signature[..8])
            );
        }

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
                        ) => {
                            logs.clone()
                        }
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
    pub async fn analyze_transaction(
        &mut self,
        transaction: &mut Transaction
    ) -> Result<(), String> {
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

    /// Cache transaction to database only
    pub async fn cache_transaction(&self, transaction: &Transaction) -> Result<(), String> {
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

    /// Get recent transactions from cache (for orphaned position recovery)
    pub async fn get_recent_transactions(
        &mut self,
        limit: usize
    ) -> Result<Vec<Transaction>, String> {
        // Database-only implementation using optimized batch retrieval
        if let Some(db) = &self.transaction_database {
            // Use the new optimized batch function to avoid N+1 queries
            let mut transactions = db.get_recent_transactions_batch(limit).await?;

            // Recalculate analysis for transactions that need it
            let mut recalculated_count = 0;
            for tx in &mut transactions {
                // Only recalculate if we have raw data and the transaction type is unknown
                if
                    matches!(tx.transaction_type, TransactionType::Unknown) &&
                    tx.raw_transaction_data.is_some()
                {
                    // Always recalculate analysis (don't use cached analysis)
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
                        recalculated_count += 1;
                    }
                }
            }

            if self.debug_enabled && recalculated_count > 0 {
                log(
                    LogTag::Transactions,
                    "RECALC",
                    &format!(
                        "Recalculated analysis for {} transactions from {} requested",
                        recalculated_count,
                        limit
                    )
                );
            }

            Ok(transactions)
        } else {
            Err("Transaction database unavailable".into())
        }
    }

    /// Get recent swap transactions from the last N transactions
    /// Returns up to limit transactions that are swaps (full Transaction objects)
    pub async fn get_recent_swaps(&mut self, limit: usize) -> Result<Vec<Transaction>, String> {
        // Calculate how many total transactions to examine based on requested swap count
        // Use a multiplier to ensure we examine enough transactions to find the requested swaps
        // Assume roughly 50% of transactions are swaps in an active trading wallet
        let examine_count = if limit <= 50 {
            // For small limits, examine 100 transactions (existing behavior)
            100
        } else {
            // For larger limits, examine 2-3x the requested amount to account for non-swap transactions
            std::cmp::max(limit * 3, 200).min(10000) // Cap at 10k transactions to avoid performance issues
        };

        if self.debug_enabled {
            log(
                LogTag::Transactions,
                "RECENT_SWAPS",
                &format!("Looking for {} swaps in last {} transactions", limit, examine_count)
            );
        }

        let recent_transactions = self.get_recent_transactions(examine_count).await?;

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
                    "Found {} swap transactions from last {} transactions",
                    swap_transactions.len(),
                    examine_count
                )
            );
        }

        Ok(swap_transactions)
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
    let wallet_address_str = match get_wallet_address() {
        Ok(address) => address,
        Err(e) => {
            log(LogTag::Transactions, "ERROR", &format!("Failed to load wallet address: {}", e));
            return;
        }
    };

    let wallet_address = match Pubkey::from_str(&wallet_address_str) {
        Ok(address) => address,
        Err(e) => {
            log(LogTag::Transactions, "ERROR", &format!("Invalid wallet address format: {}", e));
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
/// CRITICAL: Only returns transactions that are in Finalized or Confirmed status with complete analysis
/// This is the single function that handles ALL transaction requests properly
pub async fn get_transaction(signature: &str) -> Result<Option<Transaction>, String> {
    let debug = is_debug_transactions_enabled();
    if debug {
        log(LogTag::Transactions, "GET_TX", &format!("{}", &signature[..8]));
    }

    if let Some(global) = get_global_transaction_manager().await {
        // Wait for manager with reasonable timeout to avoid hanging
        match tokio::time::timeout(Duration::from_secs(15), global.lock()).await {
            Ok(mut manager_guard) => {
                if let Some(manager) = manager_guard.as_mut() {
                    if let Some(db) = &manager.transaction_database {
                        // Step 1: Check if transaction exists in database
                        if let Some(raw) = db.get_raw_transaction(signature).await? {
                            // Build Transaction skeleton from database
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

                            // Step 2: Check if transaction needs fresh analysis or blockchain update
                            let needs_fresh_analysis =
                                // Transaction is not finalized/confirmed
                                !matches!(
                                    tx.status,
                                    TransactionStatus::Finalized | TransactionStatus::Confirmed
                                ) ||
                                // Transaction was successful but has no raw data (incomplete)
                                (tx.success && tx.raw_transaction_data.is_none()) ||
                                // Transaction status is pending (might be finalized now)
                                matches!(tx.status, TransactionStatus::Pending) ||
                                // Transaction type is Unknown (incomplete analysis)
                                matches!(tx.transaction_type, TransactionType::Unknown);

                            if needs_fresh_analysis {
                                if debug {
                                    log(
                                        LogTag::Transactions,
                                        "FRESH_FETCH_NEEDED",
                                        &format!(
                                            "Transaction {} needs fresh analysis - status: {:?}, success: {}, has_raw_data: {}",
                                            &signature[..8],
                                            tx.status,
                                            tx.success,
                                            tx.raw_transaction_data.is_some()
                                        )
                                    );
                                }

                                // Fetch fresh from blockchain and analyze completely
                                match manager.process_transaction_direct(signature).await {
                                    Ok(fresh_tx) => {
                                        if debug {
                                            log(
                                                LogTag::Transactions,
                                                "FRESH_ANALYSIS_COMPLETE",
                                                &format!(
                                                    "Fresh analysis completed for {} - type: {:?}, status: {:?}",
                                                    &signature[..8],
                                                    fresh_tx.transaction_type,
                                                    fresh_tx.status
                                                )
                                            );
                                        }

                                        // Only return if transaction is now finalized/confirmed and successful
                                        if
                                            matches!(
                                                fresh_tx.status,
                                                TransactionStatus::Finalized |
                                                    TransactionStatus::Confirmed
                                            )
                                        {
                                            return Ok(Some(fresh_tx));
                                        } else {
                                            if debug {
                                                log(
                                                    LogTag::Transactions,
                                                    "FRESH_NOT_FINALIZED",
                                                    &format!(
                                                        "Fresh transaction {} still not finalized - status: {:?}",
                                                        &signature[..8],
                                                        fresh_tx.status
                                                    )
                                                );
                                            }
                                            return Ok(None);
                                        }
                                    }
                                    Err(e) => {
                                        if debug {
                                            log(
                                                LogTag::Transactions,
                                                "FRESH_FETCH_ERROR",
                                                &format!(
                                                    "Failed to fetch fresh transaction {}: {}",
                                                    &signature[..8],
                                                    e
                                                )
                                            );
                                        }
                                        return Ok(None);
                                    }
                                }
                            }

                            // Step 3: Transaction exists and is finalized/confirmed, but ensure analysis is complete
                            if manager.recalculate_transaction_analysis(&mut tx).await.is_ok() {
                                if debug {
                                    log(
                                        LogTag::Transactions,
                                        "ANALYSIS_COMPLETE",
                                        &format!(
                                            "Analysis completed for {} - type: {:?}",
                                            &signature[..8],
                                            tx.transaction_type
                                        )
                                    );
                                }

                                // Store the updated analysis back to database
                                if let Err(e) = manager.cache_processed_transaction(&tx).await {
                                    if debug {
                                        log(
                                            LogTag::Transactions,
                                            "WARN",
                                            &format!("Failed to update processed transaction in DB: {}", e)
                                        );
                                    }
                                }

                                return Ok(Some(tx));
                            } else {
                                if debug {
                                    log(
                                        LogTag::Transactions,
                                        "ANALYSIS_FAILED",
                                        &format!(
                                            "Failed to recalculate analysis for {}",
                                            &signature[..8]
                                        )
                                    );
                                }
                                return Ok(None);
                            }
                        }

                        // Step 4: Transaction not in database, fetch fresh from blockchain
                        if debug {
                            log(
                                LogTag::Transactions,
                                "NOT_IN_DB",
                                &format!(
                                    "Transaction {} not found in database, fetching fresh",
                                    &signature[..8]
                                )
                            );
                        }

                        match manager.process_transaction(signature).await {
                            Ok(fresh_tx) => {
                                if debug {
                                    log(
                                        LogTag::Transactions,
                                        "FRESH_PROCESS_COMPLETE",
                                        &format!(
                                            "Fresh process completed for {} - type: {:?}, status: {:?}",
                                            &signature[..8],
                                            fresh_tx.transaction_type,
                                            fresh_tx.status
                                        )
                                    );
                                }

                                // Only return if transaction is finalized/confirmed
                                if
                                    matches!(
                                        fresh_tx.status,
                                        TransactionStatus::Finalized | TransactionStatus::Confirmed
                                    )
                                {
                                    return Ok(Some(fresh_tx));
                                } else {
                                    return Ok(None);
                                }
                            }
                            Err(e) => {
                                if debug {
                                    log(
                                        LogTag::Transactions,
                                        "FRESH_PROCESS_ERROR",
                                        &format!(
                                            "Failed to process fresh transaction {}: {}",
                                            &signature[..8],
                                            e
                                        )
                                    );
                                }
                                return Ok(None);
                            }
                        }
                    }
                }
            }
            Err(_) => {
                // Manager timeout - return None to trigger retry
                if debug {
                    log(
                        LogTag::Transactions,
                        "MANAGER_TIMEOUT",
                        &format!("Manager timeout for {} - caller should retry", &signature[..8])
                    );
                }
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
    let wallet_address_str = get_wallet_address().map_err(|e| e.to_string())?;
    let wallet_address = Pubkey::from_str(&wallet_address_str).map_err(|e| e.to_string())?;
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
