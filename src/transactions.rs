use chrono::{ DateTime, Utc };
use serde::{ Deserialize, Serialize };
use solana_sdk::{ pubkey::Pubkey, signature::Signature };
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
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::Notify;
use tokio::time::Duration;

use crate::configs::read_configs;
use crate::errors::blockchain::parse_structured_solana_error;
use crate::global::is_debug_transactions_enabled;
use crate::logger::{ log, LogTag };
use crate::rpc::get_rpc_client;
use crate::tokens::decimals::lamports_to_sol;
use crate::tokens::{ TokenDatabase };
use crate::transactions_db::TransactionDatabase;
use crate::transactions_types::{
    AtaAnalysis,
    AtaOperation,
    AtaOperationType,
    CachedAnalysis,
    DeferredRetry,
    InstructionInfo,
    SolBalanceChange,
    SwapPnLInfo,
    TokenBalanceChange,
    TokenSwapInfo,
    TokenTransfer,
    Transaction,
    TransactionDirection,
    TransactionStats,
    TransactionStatus,
    TransactionType,
};
use crate::utils::get_wallet_address;
use crate::websocket;

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

    // WebSocket receiver for real-time transaction monitoring
    pub websocket_receiver: Option<tokio::sync::mpsc::UnboundedReceiver<String>>,

    // Pending transactions that need to be rechecked for status updates
    pub pending_transactions: HashMap<String, chrono::DateTime<chrono::Utc>>,
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
        // Price service initialization moved to pool_service module
        if false {
            log(
                LogTag::Transactions,
                "WARN",
                "Price service initialization moved to pool_service module"
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
            websocket_receiver: None, // Will be set up later
            pending_transactions: HashMap::new(), // Track pending transactions for reprocessing
        })
    }

    /// Initialize WebSocket monitoring for real-time transaction detection
    pub async fn initialize_websocket_monitoring(&mut self) -> Result<(), String> {
        let wallet_address = self.wallet_pubkey.to_string();

        log(
            LogTag::Transactions,
            "WEBSOCKET_INIT",
            &format!("🔌 Initializing WebSocket monitoring for wallet: {}", &wallet_address)
        );

        // Load WebSocket URL from config, use first RPC URL and convert to websocket
        let ws_url = match read_configs() {
            Ok(config) => {
                if let Some(first_rpc_url) = config.rpc_urls.first() {
                    // Convert HTTP RPC URL to WebSocket URL
                    let ws_url = first_rpc_url
                        .replace("https://", "wss://")
                        .replace("http://", "ws://");

                    log(
                        LogTag::Transactions,
                        "WEBSOCKET_CONFIG",
                        &format!("📡 Using WebSocket URL derived from RPC config: {}", &ws_url)
                    );
                    ws_url
                } else {
                    log(
                        LogTag::Transactions,
                        "WEBSOCKET_FALLBACK",
                        "⚠️ No RPC URLs in config, using default WebSocket URL"
                    );
                    websocket::SolanaWebSocketClient::get_default_ws_url()
                }
            }
            Err(e) => {
                log(
                    LogTag::Transactions,
                    "WEBSOCKET_FALLBACK",
                    &format!("⚠️ Failed to load config ({}), using default WebSocket URL", e)
                );
                websocket::SolanaWebSocketClient::get_default_ws_url()
            }
        };

        // Start WebSocket monitoring and get receiver
        let receiver = websocket::start_websocket_monitoring(wallet_address, Some(ws_url)).await?;

        self.websocket_receiver = Some(receiver);

        log(
            LogTag::Transactions,
            "WEBSOCKET_READY",
            "✅ WebSocket monitoring initialized successfully"
        );

        Ok(())
    }

    /// Process pending transactions to check if they've been confirmed/finalized
    pub async fn process_pending_transactions(&mut self) -> Result<usize, String> {
        if self.pending_transactions.is_empty() {
            return Ok(0);
        }

        let mut confirmed_count = 0;
        let mut signatures_to_remove = Vec::new();
        let now = chrono::Utc::now();

        // Collect signatures that need to be rechecked (older than 30 seconds)
        let mut signatures_to_recheck = Vec::new();
        for (signature, first_seen) in &self.pending_transactions {
            if now.signed_duration_since(*first_seen).num_seconds() > 30 {
                signatures_to_recheck.push(signature.clone());
            }
        }

        // Now process the collected signatures without borrowing conflicts
        for signature in signatures_to_recheck {
            if self.debug_enabled {
                log(
                    LogTag::Transactions,
                    "PENDING_RECHECK",
                    &format!("🔄 Rechecking pending transaction: {}", &signature)
                );
            }

            // Reprocess the transaction to check current status
            match self.process_transaction(&signature).await {
                Ok(tx) => {
                    // Check if transaction is now confirmed/finalized
                    if
                        matches!(
                            tx.status,
                            TransactionStatus::Confirmed | TransactionStatus::Finalized
                        )
                    {
                        confirmed_count += 1;
                        signatures_to_remove.push(signature.clone());

                        if self.debug_enabled {
                            log(
                                LogTag::Transactions,
                                "PENDING_CONFIRMED",
                                &format!("✅ Pending transaction {} now confirmed", &signature)
                            );
                        }
                    } else if matches!(tx.status, TransactionStatus::Failed(_)) {
                        // Transaction failed, remove from pending
                        signatures_to_remove.push(signature.clone());

                        log(
                            LogTag::Transactions,
                            "PENDING_FAILED",
                            &format!("❌ Pending transaction {} failed", &signature)
                        );
                    }
                    // If still pending, keep it in the list
                }
                Err(e) => {
                    // If we can't fetch the transaction anymore, remove from pending
                    if e.contains("not found") {
                        signatures_to_remove.push(signature.clone());
                        log(
                            LogTag::Transactions,
                            "PENDING_NOT_FOUND",
                            &format!("🗑️ Pending transaction {} not found, removing", &signature)
                        );
                    }
                    // For other errors, keep trying later
                }
            }
        }

        // Remove confirmed/failed/not-found transactions from pending list
        for signature in signatures_to_remove {
            self.pending_transactions.remove(&signature);
        }

        if self.debug_enabled && confirmed_count > 0 {
            log(
                LogTag::Transactions,
                "PENDING_SUMMARY",
                &format!(
                    "✅ Processed {} pending transactions, {} confirmed",
                    confirmed_count,
                    confirmed_count
                )
            );
        }

        Ok(confirmed_count)
    }

    /// Fallback check - get last 100 transactions when WebSocket is not available
    pub async fn do_websocket_fallback_check(&mut self) -> Result<usize, String> {
        log(
            LogTag::Transactions,
            "FALLBACK",
            "🔄 Performing fallback check of last 100 transactions"
        );

        // Get RPC client
        let rpc_client = get_rpc_client();

        // Get the last 100 transactions from RPC
        let signatures = rpc_client
            .get_wallet_signatures_main_rpc(
                &self.wallet_pubkey,
                100, // Last 100 transactions for fallback
                None // Start from most recent
            ).await
            .map_err(|e| format!("Failed to fetch signatures in fallback: {}", e))?;

        let mut new_transaction_count = 0;
        for sig_info in &signatures {
            let signature = &sig_info.signature;

            // Check if we already know about this transaction
            if !self.is_signature_known(signature).await {
                log(
                    LogTag::Transactions,
                    "FALLBACK_NEW",
                    &format!("🆕 Found new transaction in fallback: {}", &signature)
                );

                // Add to known signatures first
                if let Err(e) = self.add_known_signature(signature).await {
                    log(
                        LogTag::Transactions,
                        "ERROR",
                        &format!("Failed to add fallback signature to known: {}", e)
                    );
                }

                // Process the transaction
                match self.process_transaction(signature).await {
                    Ok(_) => {
                        new_transaction_count += 1;
                    }
                    Err(e) => {
                        log(
                            LogTag::Transactions,
                            "ERROR",
                            &format!("Failed to process fallback transaction {}: {}", &signature, e)
                        );
                    }
                }
            }
        }

        if new_transaction_count > 0 {
            log(
                LogTag::Transactions,
                "FALLBACK_SUCCESS",
                &format!("✅ Fallback check complete - found {} new transactions", new_transaction_count)
            );
        }

        Ok(new_transaction_count)
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
    /// UPDATED: Fetch exactly 1000 transactions at startup as requested
    pub async fn startup_transaction_discovery(&mut self) -> Result<(), String> {
        log(
            LogTag::Transactions,
            "STARTUP_DISCOVERY",
            "🔍 Starting initial fetch of 1000 transactions"
        );

        let rpc_client = get_rpc_client();
        let mut total_processed = 0;
        let mut total_cached = 0;

        // Fetch exactly 1000 transactions in a single batch
        log(LogTag::Transactions, "STARTUP_DISCOVERY", "📦 Fetching 1000 most recent transactions");

        // Fetch batch of signatures using rate-limited RPC
        let signatures = rpc_client
            .get_wallet_signatures_main_rpc(
                &self.wallet_pubkey,
                1000, // Exactly 1000 transactions as requested
                None // Start from most recent
            ).await
            .map_err(|e| format!("Failed to fetch 1000 transactions: {}", e))?;

        if signatures.is_empty() {
            log(LogTag::Transactions, "STARTUP_DISCOVERY", "📭 No transactions found for wallet");
            return Ok(());
        }

        let mut new_in_batch = 0;

        // Process each signature in the batch
        for sig_info in &signatures {
            let signature = &sig_info.signature;
            total_processed += 1;

            // Always add to known signatures and process new transactions
            if !self.is_signature_known(signature).await {
                // New signature - add it to known signatures and cache it
                self.add_known_signature(signature).await?;
                new_in_batch += 1;
                total_cached += 1;

                // Process the transaction to cache its data
                if let Err(e) = self.process_transaction(signature).await {
                    let error_msg = format!(
                        "Failed to process startup transaction {}: {}",
                        &signature,
                        e
                    );
                    log(LogTag::Transactions, "WARN", &error_msg);

                    // Save failed state to database for startup processing
                    if let Err(db_err) = self.save_failed_transaction_state(&signature, &e).await {
                        log(
                            LogTag::Transactions,
                            "ERROR",
                            &format!(
                                "Failed to save startup transaction failure state for {}: {}",
                                &signature,
                                db_err
                            )
                        );
                    }
                }
            }
        }

        log(
            LogTag::Transactions,
            "STARTUP_DISCOVERY",
            &format!(
                "🎯 Discovery complete: processed {} signatures, cached {} new transactions",
                total_processed,
                total_cached
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
                &format!("Processing transaction: {}", &signature)
            );
        }

        // Not in database, fetch fresh data from RPC
        if self.debug_enabled {
            log(
                LogTag::Transactions,
                "RPC_FETCH",
                &format!("Fetching new transaction: {}", &signature)
            );
        }

        let rpc_client = get_rpc_client();
        let tx_data = match rpc_client.get_transaction_details(signature).await {
            Ok(data) => {
                log(
                    LogTag::Rpc,
                    "SUCCESS",
                    &format!("Retrieved transaction details for {} from premium RPC", &signature)
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
                            &signature
                        )
                    );
                    return Err(format!("Transaction not found: {}", signature));
                } else {
                    log(
                        LogTag::Rpc,
                        "ERROR",
                        &format!("RPC error fetching {}: {}", &signature, error_msg)
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
            success: tx_data.meta.as_ref().map_or(false, |meta| meta.err.is_none()),
            error_message: tx_data.meta
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
            fee_sol: tx_data.meta.as_ref().map_or(0.0, |meta| lamports_to_sol(meta.fee)),
            sol_balance_change: 0.0,
            token_transfers: Vec::new(),
            raw_transaction_data: Some(serde_json::to_value(&tx_data).unwrap_or_default()),
            log_messages: tx_data.meta
                .as_ref()
                .and_then(|meta| meta.log_messages.clone())
                .unwrap_or_default(),
            instructions: Vec::new(),
            sol_balance_changes: Vec::new(),
            token_balance_changes: Vec::new(),
            position_impact: None,
            profit_calculation: None,
            ata_analysis: None,
            token_info: None,
            calculated_token_price_sol: None,
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
                    &transaction.signature,
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
                                &transaction.signature,
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
                    &format!("Cached transaction {} to database", &transaction.signature)
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
                &format!("Recalculating analysis for transaction: {}", &transaction.signature)
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
        transaction.position_impact = None;
        transaction.profit_calculation = None;
        transaction.ata_analysis = None; // CRITICAL: Reset ATA analysis for recalculation
        transaction.token_info = None;
        transaction.calculated_token_price_sol = None;
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
                        &transaction.signature,
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
                    &transaction.signature
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
                                    &tx.signature,
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
        let recent_count = recent_transactions.len();

        let swap_transactions: Vec<Transaction> = recent_transactions
            .into_iter()
            .filter(|tx| {
                let is_swap = self.is_swap_transaction(tx);
                if is_debug_transactions_enabled() || self.debug_enabled {
                    log(
                        LogTag::Transactions,
                        "RECENT_SWAPS_FILTER",
                        &format!(
                            "Transaction {}: type = {:?}, is_swap = {}",
                            &tx.signature,
                            tx.transaction_type,
                            is_swap
                        )
                    );
                }
                is_swap
            })
            .take(limit)
            .collect();

        if self.debug_enabled || is_debug_transactions_enabled() {
            log(
                LogTag::Transactions,
                "RECENT_SWAPS",
                &format!(
                    "Found {} swap transactions from last {} transactions (examined {} total)",
                    swap_transactions.len(),
                    examine_count,
                    recent_count
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

    // Initialize WebSocket monitoring after startup discovery
    if let Err(e) = manager.initialize_websocket_monitoring().await {
        log(
            LogTag::Transactions,
            "ERROR",
            &format!("Failed to initialize WebSocket monitoring: {}", e)
        );
        // Fall back to polling if WebSocket fails
        log(LogTag::Transactions, "INFO", "Falling back to polling-based monitoring");
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

    // NEW: WebSocket-based monitoring with periodic checks
    let mut next_gap_check = tokio::time::Instant::now() + Duration::from_secs(300); // Gap detection every 5 minutes
    let mut next_fallback_check = tokio::time::Instant::now() + Duration::from_secs(30); // Fallback check if WebSocket fails
    let mut next_pending_check = tokio::time::Instant::now() + Duration::from_secs(30); // Check pending transactions every 30 seconds

    loop {
        tokio::select! {
            _ = shutdown.notified() => {
                log(LogTag::Transactions, "INFO", "TransactionsManager service shutting down");
                break;
            }
            // NEW: WebSocket real-time transaction monitoring
            result = async {
                if let Some(ref mut receiver) = manager.websocket_receiver {
                    receiver.recv().await
                } else {
                    // No WebSocket receiver available, wait indefinitely
                    std::future::pending().await
                }
            } => {
                match result {
                    Some(signature) => {
                        // NEW: Real-time transaction detected via WebSocket
                        if !manager.is_signature_known(&signature).await {
                            log(
                                LogTag::Transactions,
                                "WEBSOCKET_NEW",
                                &format!("🆕 Processing WebSocket transaction: {}", &signature)
                            );

                            // Add to known signatures first
                            if let Err(e) = manager.add_known_signature(&signature).await {
                                log(
                                    LogTag::Transactions,
                                    "ERROR",
                                    &format!("Failed to add WebSocket signature to known: {}", e)
                                );
                            }

                            // Process the transaction
                            match manager.process_transaction(&signature).await {
                                Ok(tx) => {
                                    // Check transaction status
                                    match tx.status {
                                        TransactionStatus::Pending => {
                                            // Transaction is pending, add to pending list for later reprocessing
                                            manager.pending_transactions.insert(signature.clone(), chrono::Utc::now());
                                            log(
                                                LogTag::Transactions,
                                                "WEBSOCKET_PENDING",
                                                &format!("⏳ WebSocket transaction {} is pending, will recheck later", &signature)
                                            );
                                        }
                                        TransactionStatus::Confirmed | TransactionStatus::Finalized => {
                                            // Transaction is confirmed/finalized, fully processed
                                            manager.new_transactions_count += 1;
                                            if manager.debug_enabled {
                                                log(
                                                    LogTag::Transactions,
                                                    "WEBSOCKET_SUCCESS",
                                                    &format!("✅ WebSocket transaction {} processed successfully", &signature)
                                                );
                                            }
                                            
                                            // CRITICAL: Trigger position verification for confirmed/finalized transactions
                                            // This ensures positions are properly updated when WebSocket detects sell/buy transactions
                                            let sig_clone = signature.clone();
                                            tokio::spawn(async move {
                                                if let Err(e) = crate::positions::verify_position_transaction(&sig_clone).await {
                                                    // Only log verification attempts if debug is enabled - normal "no matching position" is expected
                                                    if crate::arguments::is_debug_positions_enabled() && !e.contains("No matching position found") {
                                                        log(
                                                            LogTag::Transactions,
                                                            "WEBSOCKET_POSITION_VERIFY",
                                                            &format!("Position verification for WebSocket transaction {} result: {}", &sig_clone, e)
                                                        );
                                                    }
                                                } else {
                                                    log(
                                                        LogTag::Transactions,
                                                        "WEBSOCKET_POSITION_SUCCESS",
                                                        &format!("✅ Position verification successful for WebSocket transaction {}", &sig_clone)
                                                    );
                                                }
                                            });
                                        }
                                        TransactionStatus::Failed(_) => {
                                            // Transaction failed, but we still count it as processed
                                            manager.new_transactions_count += 1;
                                            log(
                                                LogTag::Transactions,
                                                "WEBSOCKET_FAILED",
                                                &format!("❌ WebSocket transaction {} failed but processed", &signature)
                                            );
                                        }
                                    }
                                }
                                Err(e) => {
                                    log(
                                        LogTag::Transactions,
                                        "WARN",
                                        &format!("Failed to process WebSocket transaction {}: {}", &signature, e)
                                    );

                                    // Save failed state to database
                                    if let Err(db_err) = manager.save_failed_transaction_state(&signature, &e).await {
                                        log(
                                            LogTag::Transactions,
                                            "ERROR",
                                            &format!("Failed to save WebSocket transaction failure state for {}: {}", &signature, db_err)
                                        );
                                    }

                                    // Create deferred retry
                                    let retry = DeferredRetry {
                                        signature: signature.clone(),
                                        next_retry_at: chrono::Utc::now() + chrono::Duration::minutes(5),
                                        remaining_attempts: 3,
                                        current_delay_secs: 300,
                                        last_error: Some(e),
                                    };

                                    if let Err(retry_err) = manager.store_deferred_retry(&retry).await {
                                        log(
                                            LogTag::Transactions,
                                            "ERROR",
                                            &format!("Failed to store WebSocket deferred retry for {}: {}", &signature, retry_err)
                                        );
                                    }
                                }
                            }
                        } else if manager.debug_enabled {
                            log(
                                LogTag::Transactions,
                                "WEBSOCKET_DUPLICATE",
                                &format!("🔄 WebSocket transaction {} already known, skipping", &signature)
                            );
                        }
                    }
                    None => {
                        // WebSocket channel closed, try to reinitialize
                        log(
                            LogTag::Transactions,
                            "WEBSOCKET_RECONNECT",
                            "WebSocket channel closed, attempting to reinitialize"
                        );
                        
                        if let Err(e) = manager.initialize_websocket_monitoring().await {
                            log(
                                LogTag::Transactions,
                                "ERROR",
                                &format!("Failed to reinitialize WebSocket: {}", e)
                            );
                        }
                    }
                }
            }
            // Fallback check (less frequent, only if WebSocket is not working)
            _ = tokio::time::sleep_until(next_fallback_check) => {
                if manager.websocket_receiver.is_none() {
                    // WebSocket not available, do fallback check
                    match manager.do_websocket_fallback_check().await {
                        Ok(new_transaction_count) => {
                            if new_transaction_count > 0 {
                                log(LogTag::Transactions, "FALLBACK_SUCCESS", &format!(
                                    "Found {} new transactions via fallback check",
                                    new_transaction_count
                                ));
                            }
                        }
                        Err(e) => {
                            log(LogTag::Transactions, "ERROR", &format!("Fallback check error: {}", e));
                        }
                    }
                }
                next_fallback_check = tokio::time::Instant::now() + Duration::from_secs(30);
            }
            _ = tokio::time::sleep_until(next_pending_check) => {
                // Process pending transactions every 30 seconds
                match manager.process_pending_transactions().await {
                    Ok(processed_count) => {
                        if processed_count > 0 {
                            log(LogTag::Transactions, "PENDING_CHECK", &format!(
                                "⏱️  Processed {} pending transactions",
                                processed_count
                            ));
                        } else if manager.debug_enabled {
                            log(LogTag::Transactions, "PENDING_CHECK", "No pending transactions to process");
                        }
                    }
                    Err(e) => {
                        log(LogTag::Transactions, "ERROR", &format!("Pending transaction processing error: {}", e));
                    }
                }
                next_pending_check = tokio::time::Instant::now() + Duration::from_secs(30);
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

                // Periodic cleanup of expired deferred retries every 5 minutes
                if let Err(e) = manager.cleanup_expired_deferred_retries().await {
                    log(LogTag::Transactions, "ERROR", &format!("Deferred retries cleanup error: {}", e));
                }
            }
        }
    }

    log(LogTag::Transactions, "INFO", "TransactionsManager service stopped");
}

/// Check last 100 transactions when WebSocket fails (fallback mechanism)
async fn do_websocket_fallback_check(manager: &mut TransactionsManager) -> Result<usize, String> {
    if manager.debug_enabled {
        log(
            LogTag::Transactions,
            "FALLBACK_CHECK",
            "🔄 WebSocket fallback: checking last 100 transactions"
        );
    }

    let rpc_client = get_rpc_client();

    // Get last 100 transactions
    let signatures = rpc_client
        .get_wallet_signatures_main_rpc(&manager.wallet_pubkey, 100, None).await
        .map_err(|e| format!("Failed to fetch last 100 transactions for fallback: {}", e))?;

    let mut new_transaction_count = 0;

    // Process any new signatures we haven't seen yet
    for sig_info in signatures {
        let signature = sig_info.signature;

        if !manager.is_signature_known(&signature).await {
            // New signature found - add to known signatures
            manager.add_known_signature(&signature).await?;
            new_transaction_count += 1;

            if manager.debug_enabled {
                log(
                    LogTag::Transactions,
                    "FALLBACK_NEW",
                    &format!("🆕 Fallback found new transaction: {}", &signature)
                );
            }

            // Process the transaction
            match manager.process_transaction(&signature).await {
                Ok(tx) => {
                    // Handle transaction status like WebSocket processing
                    match tx.status {
                        TransactionStatus::Pending => {
                            manager.pending_transactions.insert(
                                signature.clone(),
                                chrono::Utc::now()
                            );
                            log(
                                LogTag::Transactions,
                                "FALLBACK_PENDING",
                                &format!("⏳ Fallback transaction {} is pending", &signature)
                            );
                        }
                        TransactionStatus::Confirmed | TransactionStatus::Finalized => {
                            manager.new_transactions_count += 1;

                            // CRITICAL: Trigger position verification for confirmed/finalized fallback transactions too
                            let sig_clone = signature.clone();
                            tokio::spawn(async move {
                                if
                                    let Err(e) = crate::positions::verify_position_transaction(
                                        &sig_clone
                                    ).await
                                {
                                    // Only log verification attempts if debug is enabled - normal "no matching position" is expected
                                    if
                                        crate::arguments::is_debug_positions_enabled() &&
                                        !e.contains("No matching position found")
                                    {
                                        log(
                                            LogTag::Transactions,
                                            "FALLBACK_POSITION_VERIFY",
                                            &format!(
                                                "Position verification for fallback transaction {} result: {}",
                                                &sig_clone,
                                                e
                                            )
                                        );
                                    }
                                } else {
                                    log(
                                        LogTag::Transactions,
                                        "FALLBACK_POSITION_SUCCESS",
                                        &format!(
                                            "✅ Position verification successful for fallback transaction {}",
                                            &sig_clone
                                        )
                                    );
                                }
                            });
                        }
                        TransactionStatus::Failed(_) => {
                            manager.new_transactions_count += 1;
                        }
                    }
                }
                Err(e) => {
                    log(
                        LogTag::Transactions,
                        "WARN",
                        &format!("Failed to process fallback transaction {}: {}", &signature, e)
                    );

                    // Save failed transaction state
                    if
                        let Err(db_err) = manager.save_failed_transaction_state(
                            &signature,
                            &e
                        ).await
                    {
                        log(
                            LogTag::Transactions,
                            "ERROR",
                            &format!(
                                "Failed to save fallback transaction failure state for {}: {}",
                                &signature,
                                db_err
                            )
                        );
                    }
                }
            }
        }
    }

    if new_transaction_count > 0 && manager.debug_enabled {
        log(
            LogTag::Transactions,
            "FALLBACK_SUMMARY",
            &format!("✅ Fallback check complete: found {} new transactions", new_transaction_count)
        );
    }

    Ok(new_transaction_count)
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
        log(LogTag::Transactions, "GET_TX", &format!("{}", &signature));
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
                                position_impact: None,
                                profit_calculation: None,
                                ata_analysis: None,
                                token_info: None,
                                calculated_token_price_sol: None,
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
                                            &signature,
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
                                    &signature
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
                        &format!("Manager timeout for {} - caller should retry", &signature)
                    );
                }
                return Ok(None);
            }
        }
    } else {
        log(
            LogTag::Transactions,
            "NO_GLOBAL_MANAGER",
            &format!("No global transaction manager available for {}", &signature)
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
