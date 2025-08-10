/// Wallet Transaction Manager
/// Manages wallet transaction fetching, caching, and analysis with efficient sync tracking

use crate::{
    rpc::RpcClient,
    logger::{log, LogTag},
    global::is_debug_transactions_enabled,
    tokens::{get_token_decimals, TokenDatabase},
    tokens::decimals::{SOL_DECIMALS, LAMPORTS_PER_SOL, lamports_to_sol},
};
use solana_sdk::{
    signature::{Signature, Signer},
    commitment_config::CommitmentConfig,
    pubkey::Pubkey,
};
use solana_transaction_status::{UiTransactionEncoding, EncodedConfirmedTransactionWithStatusMeta};
use solana_client::rpc_client::GetConfirmedSignaturesForAddress2Config;
use std::str::FromStr;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::fs;
use std::sync::{Arc, RwLock};
use std::sync::atomic::{AtomicBool, Ordering};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use lazy_static::lazy_static;
use tokio::task::JoinSet;
use std::time::Duration;
use tokio::sync::Notify;

lazy_static! {
    static ref WALLET_TRANSACTION_MANAGER: Arc<RwLock<Option<WalletTransactionManager>>> = Arc::new(RwLock::new(None));
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapTransaction {
    pub signature: String,
    pub slot: u64,
    pub block_time: Option<i64>,
    pub swap_type: String, // "buy" or "sell"
    pub token_mint: String,
    pub token_symbol: String,
    pub sol_amount: f64,
    pub token_amount: u64,
    pub token_decimals: u8,
    pub effective_price: f64,
    pub fees_paid: f64,
    pub success: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapAnalysis {
    pub total_swaps: usize,
    pub buy_swaps: usize,
    pub sell_swaps: usize,
    pub total_sol_in: f64,
    pub total_sol_out: f64,
    pub total_fees: f64,
    pub net_sol_change: f64,
    pub recent_swaps: Vec<SwapTransaction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifiedSwapData {
    pub signature: String,
    pub success: bool,
    pub direction: String, // "buy" or "sell"
    pub token_mint: String,
    pub sol_amount: f64, // SOL amount involved in swap
    pub token_amount: u64, // Raw token amount
    pub token_decimals: u8,
    pub effective_price: f64, // Price per token in SOL
    pub transaction_fee: u64, // Network transaction fee in lamports
    pub priority_fee: Option<u64>, // Priority fee in lamports
    pub ata_created: bool,
    pub ata_closed: bool,
    pub ata_rent_paid: u64, // ATA rent paid in lamports
    pub ata_rent_reclaimed: u64, // ATA rent reclaimed in lamports
    pub slot: u64,
    pub block_time: Option<i64>,
}

#[derive(Serialize, Deserialize)]
#[derive(Debug)]
pub struct CachedTransactionData {
    pub signature: String,
    pub transaction_data: EncodedConfirmedTransactionWithStatusMeta,
    pub cached_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletSyncState {
    pub wallet_address: String,
    pub total_transactions_fetched: usize,
    pub oldest_signature: Option<String>,
    pub newest_signature: Option<String>,
    pub last_sync_time: DateTime<Utc>,
    pub cached_signatures: HashSet<String>,
    pub fetch_ranges: Vec<FetchRange>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FetchRange {
    pub start_signature: Option<String>,
    pub end_signature: Option<String>,
    pub fetched_at: DateTime<Utc>,
    pub transaction_count: usize,
}

impl Default for WalletSyncState {
    fn default() -> Self {
        Self {
            wallet_address: String::new(),
            total_transactions_fetched: 0,
            oldest_signature: None,
            newest_signature: None,
            last_sync_time: Utc::now(),
            cached_signatures: HashSet::new(),
            fetch_ranges: Vec::new(),
        }
    }
}

pub struct WalletTransactionManager {
    wallet_address: String,
    cache_dir: PathBuf,
    sync_state_file: PathBuf,
    sync_state: WalletSyncState,
    transaction_cache: HashMap<String, CachedTransactionData>,
    is_periodic_sync_running: Arc<std::sync::atomic::AtomicBool>,
}

/// Initialize the global wallet transaction manager
pub async fn initialize_wallet_transaction_manager() -> Result<(), Box<dyn std::error::Error>> {
    log(LogTag::Transactions, "INFO", "Initializing global wallet transaction manager");
    
    // Get the main bot wallet address from global configs
    let configs = crate::global::read_configs()
        .map_err(|e| format!("Failed to load configs: {}", e))?;
    
    let main_wallet_keypair = crate::global::load_wallet_from_config(&configs)
        .map_err(|e| format!("Failed to load main wallet: {}", e))?;
    
    let wallet_address = main_wallet_keypair.pubkey().to_string();
    
    log(LogTag::Transactions, "INFO", &format!("Initializing transaction manager for wallet: {}", wallet_address));
    
    // Create and initialize the manager
    let mut manager = WalletTransactionManager::new(wallet_address)?;
    
    // Get RPC client
    let rpc_client = crate::rpc::get_rpc_client();
    
    // Initialize and sync
    manager.initialize_and_sync(rpc_client).await?;
    
    // Store in global state
    {
        let mut global_manager = WALLET_TRANSACTION_MANAGER.write().unwrap();
        *global_manager = Some(manager);
    }
    
    log(LogTag::Transactions, "SUCCESS", "Global wallet transaction manager initialized and ready");
    Ok(())
}

/// Start the periodic sync background task for the global wallet transaction manager
pub async fn start_wallet_transaction_sync_task(shutdown: Arc<Notify>) -> Result<tokio::task::JoinHandle<()>, Box<dyn std::error::Error>> {
    let manager_lock = WALLET_TRANSACTION_MANAGER.read().unwrap();
    if let Some(ref manager) = *manager_lock {
        let sync_handle = manager.start_periodic_sync(shutdown).await;
        drop(manager_lock);
        Ok(sync_handle)
    } else {
        Err("Wallet transaction manager not initialized".into())
    }
}

/// Get access to the global wallet transaction manager
pub fn get_wallet_transaction_manager() -> Result<Arc<RwLock<Option<WalletTransactionManager>>>, Box<dyn std::error::Error>> {
    Ok(WALLET_TRANSACTION_MANAGER.clone())
}

/// Get global wallet transaction statistics for summary display
pub fn get_global_wallet_transaction_stats() -> Option<(usize, usize, String, bool, Option<String>, Option<String>)> {
    let manager_lock = WALLET_TRANSACTION_MANAGER.read().unwrap();
    if let Some(ref manager) = *manager_lock {
        Some(manager.get_detailed_sync_stats())
    } else {
        None
    }
}

/// Convenient function to analyze recent swaps using the global manager
pub async fn analyze_recent_swaps_global(limit: usize) -> Result<SwapAnalysis, Box<dyn std::error::Error>> {
    let manager_lock = WALLET_TRANSACTION_MANAGER.read().unwrap();
    if let Some(ref manager) = *manager_lock {
        Ok(manager.analyze_recent_swaps(limit).await)
    } else {
        // Create a standalone manager if global one isn't available
        log(LogTag::Transactions, "INFO", "Creating standalone wallet transaction manager");
        
        // Get the main bot wallet address from global configs
        let configs = crate::global::read_configs()
            .map_err(|e| format!("Failed to load configs: {}", e))?;
        
        let main_wallet_keypair = crate::global::load_wallet_from_config(&configs)
            .map_err(|e| format!("Failed to load main wallet: {}", e))?;
        
        let wallet_address = main_wallet_keypair.pubkey().to_string();
        
        log(LogTag::Transactions, "INFO", &format!("Creating standalone manager for wallet: {}", wallet_address));
        
        // Create and initialize the manager
        let mut manager = WalletTransactionManager::new(wallet_address)?;
        
        // Get RPC client
        let rpc_client = crate::rpc::get_rpc_client();
        
        // Initialize and sync
        manager.initialize_and_sync(rpc_client).await?;
        
        log(LogTag::Transactions, "SUCCESS", "Standalone wallet transaction manager ready");
        
        Ok(manager.analyze_recent_swaps(limit).await)
    }
}

/// Get transaction details via the global wallet transaction manager with automatic caching
pub async fn get_transaction_details_global(signature: &str) -> Result<crate::rpc::TransactionDetails, Box<dyn std::error::Error + Send + Sync>> {
    let rpc_client = crate::rpc::get_rpc_client();
    
    // Get the manager with proper locking
    let manager = {
        let mut manager_lock = WALLET_TRANSACTION_MANAGER.write().unwrap();
        match manager_lock.take() {
            Some(mgr) => mgr,
            None => return Err("Wallet transaction manager not initialized".into()),
        }
    };
    
    // Check cache first
    if let Some(cached_tx) = manager.get_cached_transaction(signature) {
        // Convert cached transaction to TransactionDetails
        let transaction_details = convert_cached_to_transaction_details(cached_tx)?;
        
        // Return the manager
        {
            let mut manager_lock = WALLET_TRANSACTION_MANAGER.write().unwrap();
            *manager_lock = Some(manager);
        }
        
        return Ok(transaction_details);
    }
    
    // If not cached, we need to fetch it - this requires mutable access
    let mut manager = manager;
    match manager.get_or_fetch_transaction(signature, &rpc_client).await {
        Ok(_) => {
            // Now get the cached transaction
            if let Some(cached_tx) = manager.get_cached_transaction(signature) {
                let transaction_details = convert_cached_to_transaction_details(cached_tx)?;
                
                // Return the manager
                {
                    let mut manager_lock = WALLET_TRANSACTION_MANAGER.write().unwrap();
                    *manager_lock = Some(manager);
                }
                
                Ok(transaction_details)
            } else {
                // Return the manager
                {
                    let mut manager_lock = WALLET_TRANSACTION_MANAGER.write().unwrap();
                    *manager_lock = Some(manager);
                }
                Err("Transaction not found after fetch".into())
            }
        },
        Err(e) => {
            // Return the manager
            {
                let mut manager_lock = WALLET_TRANSACTION_MANAGER.write().unwrap();
                *manager_lock = Some(manager);
            }
            Err(format!("Failed to fetch transaction: {}", e).into())
        }
    }
}

/// Convert cached transaction data to TransactionDetails format
fn convert_cached_to_transaction_details(cached_tx: &CachedTransactionData) -> Result<crate::rpc::TransactionDetails, Box<dyn std::error::Error + Send + Sync>> {
    use crate::rpc::{TransactionDetails, TransactionData, TransactionMeta};
    
    let encoded_tx = &cached_tx.transaction_data;
    
    // Extract transaction data from the encoded structure
    let transaction_data = if let Some(decoded_tx) = encoded_tx.transaction.transaction.decode() {
        TransactionData {
            message: serde_json::to_value(&decoded_tx.message)?,
            signatures: decoded_tx.signatures.iter().map(|s| s.to_string()).collect(),
        }
    } else {
        // Fall back to extracting signatures from the encoded transaction
        TransactionData {
            message: serde_json::to_value(&encoded_tx.transaction.transaction)?,
            signatures: vec![], // Will be filled from signatures field if available
        }
    };
    
    // Extract meta if available  
    let meta = if let Some(ref meta) = encoded_tx.transaction.meta {
        Some(TransactionMeta {
            fee: meta.fee,
            pre_balances: meta.pre_balances.clone(),
            post_balances: meta.post_balances.clone(),
            pre_token_balances: Some(vec![]), // Simplified for now - type conversion needed
            post_token_balances: Some(vec![]), // Simplified for now - type conversion needed  
            log_messages: Some(meta.log_messages.clone().unwrap_or(vec![])),
            err: meta.err.as_ref().map(|e| serde_json::to_value(e).unwrap_or_default()),
        })
    } else {
        None
    };
    
    Ok(TransactionDetails {
        slot: encoded_tx.slot,
        transaction: transaction_data,
        meta,
    })
}

/// Global helper function to get cached transaction JSON data via wallet transaction manager
/// This replaces direct file access to maintain architectural compliance
pub async fn get_cached_transaction_json_global(signature: &str) -> Option<serde_json::Value> {
    let manager_lock = WALLET_TRANSACTION_MANAGER.read().unwrap();
    if let Some(ref manager) = *manager_lock {
        if let Some(cached_data) = manager.get_cached_transaction(signature) {
            // Convert the transaction data to JSON for compatibility with existing code
            if let Ok(json_value) = serde_json::to_value(&cached_data.transaction_data) {
                // Wrap in the expected format (original cached files had "transaction_data" field)
                let wrapped_json = serde_json::json!({
                    "transaction_data": json_value,
                    "signature": cached_data.signature,
                    "cached_at": cached_data.cached_at
                });
                return Some(wrapped_json);
            }
        }
    }
    None
}

/// Verify and analyze a specific swap transaction using the global manager
/// This is the main function positions should use for transaction verification
pub async fn verify_swap_transaction_global(signature: &str, expected_direction: &str) -> Result<VerifiedSwapData, Box<dyn std::error::Error + Send + Sync>> {
    let rpc_client = crate::rpc::get_rpc_client();
    
    // We need to extract the manager temporarily to avoid holding the lock across await
    let manager = {
        let mut manager_lock = WALLET_TRANSACTION_MANAGER.write().unwrap();
        match manager_lock.take() {
            Some(mgr) => mgr,
            None => return Err("Wallet transaction manager not initialized".into()),
        }
    };
    
    // Perform verification outside the lock
    let result = {
        let mut temp_manager = manager;
        let verification_result = temp_manager.verify_swap_transaction(signature, expected_direction, &rpc_client).await;
        
        // Put the manager back
        {
            let mut manager_lock = WALLET_TRANSACTION_MANAGER.write().unwrap();
            *manager_lock = Some(temp_manager);
        }
        
        verification_result
    };
    
    // Convert to Send + Sync error
    match result {
        Ok(data) => Ok(data),
        Err(e) => Err(format!("Verification failed: {}", e).into()),
    }
}

/// Perform periodic sync check (helper function to avoid Send trait issues)
async fn perform_periodic_sync_check() {
    // Get RPC client first
    let rpc_client = crate::rpc::get_rpc_client();
    
    // Read lock first to check if manager exists and get wallet address
    let wallet_address = {
        let manager_lock = WALLET_TRANSACTION_MANAGER.read().unwrap();
        if let Some(ref manager) = *manager_lock {
            manager.wallet_address.clone()
        } else {
            if is_debug_transactions_enabled() {
                log(LogTag::Transactions, "DEBUG", "Manager not available for periodic sync");
            }
            return;
        }
    };
    
    // Get latest signatures to see what's new (outside of any locks)
    let latest_signatures = match fetch_wallet_signatures_standalone(&wallet_address, &rpc_client, 20).await {
        Ok(sigs) => sigs,
        Err(e) => {
            if is_debug_transactions_enabled() {
                log(LogTag::Transactions, "ERROR", &format!("Failed to fetch signatures: {}", e));
            }
            return;
        }
    };
    
    if latest_signatures.is_empty() {
        return;
    }
    
    // Check which signatures are new (briefly lock to read cached signatures)
    let mut new_signatures = Vec::new();
    {
        let manager_lock = WALLET_TRANSACTION_MANAGER.read().unwrap();
        if let Some(ref manager) = *manager_lock {
            for sig_info in &latest_signatures {
                if !manager.sync_state.cached_signatures.contains(&sig_info.signature) {
                    new_signatures.push(sig_info.signature.clone());
                }
            }
        } else {
            return; // Manager disappeared during sync
        }
    }
    
    if !new_signatures.is_empty() {
        log(LogTag::Transactions, "INFO", &format!("Found {} new transactions in periodic sync", new_signatures.len()));
        
        // Fetch new transaction details (outside of any locks)
        if let Ok(new_transactions) = fetch_transaction_details_standalone(&new_signatures, &rpc_client).await {
            if !new_transactions.is_empty() {
                log(LogTag::Transactions, "INFO", &format!("Successfully fetched {} of {} new transactions", new_transactions.len(), new_signatures.len()));
            } else {
                log(LogTag::Transactions, "INFO", "New transactions detected but none could be fetched yet (likely very recent), will retry in next sync");
            }
            
            // Update manager with new data (brief write lock)
            let mut manager_lock = WALLET_TRANSACTION_MANAGER.write().unwrap();
            if let Some(ref mut manager) = *manager_lock {
                // Only add signatures for transactions we successfully fetched
                for (sig, _) in &new_transactions {
                    manager.sync_state.cached_signatures.insert(sig.clone());
                }
                
                // Update cached transactions  
                for (sig, tx) in new_transactions {
                    manager.transaction_cache.insert(sig.clone(), CachedTransactionData {
                        signature: sig,
                        transaction_data: tx,
                        cached_at: Utc::now(),
                    });
                }
                
                // Save sync state (outside the lock to avoid blocking)
                let sync_state_clone = manager.sync_state.clone();
                let sync_state_file = manager.sync_state_file.clone();
                drop(manager_lock);
                
                // Save to disk outside of lock
                if let Err(e) = save_sync_state_to_file(&sync_state_clone, &sync_state_file) {
                    if is_debug_transactions_enabled() {
                        log(LogTag::Transactions, "ERROR", &format!("Failed to save sync state: {}", e));
                    }
                }
            }
        }
    } else if is_debug_transactions_enabled() {
        log(LogTag::Transactions, "DEBUG", "No new transactions found in periodic sync");
    }
}

/// Helper function to fetch wallet signatures without holding manager lock
async fn fetch_wallet_signatures_standalone(
    wallet_address: &str, 
    rpc_client: &RpcClient, 
    limit: usize
) -> Result<Vec<solana_client::rpc_response::RpcConfirmedTransactionStatusWithSignature>, Box<dyn std::error::Error>> {
    let wallet_pubkey = Pubkey::from_str(wallet_address)?;
    
    log(LogTag::Transactions, "MAIN_RPC", &format!("Using main RPC to check for {} new signatures", limit));
    
    // Use main RPC for lightweight signature checking
    let signatures = rpc_client.get_wallet_signatures_main_rpc(&wallet_pubkey, limit, None).await
        .map_err(|e| format!("Failed to get signatures from main RPC: {}", e))?;
    
    log(LogTag::Transactions, "SUCCESS", &format!("Retrieved {} signatures from main RPC", signatures.len()));
    Ok(signatures)
}

/// Helper function to fetch transaction details without holding manager lock
async fn fetch_transaction_details_standalone(
    signatures: &[String],
    rpc_client: &RpcClient
) -> Result<Vec<(String, EncodedConfirmedTransactionWithStatusMeta)>, Box<dyn std::error::Error>> {
    if signatures.is_empty() {
        return Ok(Vec::new());
    }
    
    log(LogTag::Transactions, "PREMIUM_RPC", &format!("Using premium RPC to fetch {} transaction details", signatures.len()));
    
    // Use premium RPC for data-intensive transaction fetching
    let transactions = rpc_client.batch_get_transaction_details_premium_rpc(signatures).await
        .map_err(|e| format!("Failed to fetch transaction details from premium RPC: {}", e))?;
    
    log(LogTag::Transactions, "SUCCESS", &format!("Successfully fetched {}/{} transactions from premium RPC", transactions.len(), signatures.len()));
    Ok(transactions)
}

/// Helper function to save sync state to file without holding manager lock
fn save_sync_state_to_file(
    sync_state: &WalletSyncState,
    sync_state_file: &Path
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent) = sync_state_file.parent() {
        fs::create_dir_all(parent)?;
    }
    
    let json_data = serde_json::to_string_pretty(sync_state)?;
    fs::write(sync_state_file, json_data)?;
    
    Ok(())
}

impl WalletTransactionManager {
    pub fn new(wallet_address: String) -> Result<Self, Box<dyn std::error::Error>> {
        let cache_dir = PathBuf::from("data/transactions");
        let sync_state_file = PathBuf::from("data/wallet_transactions_stats.json");
        
        // Create cache directory if it doesn't exist
        if !cache_dir.exists() {
            fs::create_dir_all(&cache_dir)?;
            log(LogTag::Transactions, "INFO", &format!("Created transaction cache directory: {:?}", cache_dir));
        }
        
        // Create data directory if it doesn't exist
        let data_dir = PathBuf::from("data");
        if !data_dir.exists() {
            fs::create_dir_all(&data_dir)?;
            log(LogTag::Transactions, "INFO", &format!("Created data directory: {:?}", data_dir));
        }
        
        // Load or create sync state
        let sync_state = if sync_state_file.exists() {
            match fs::read_to_string(&sync_state_file) {
                Ok(content) => {
                    match serde_json::from_str::<WalletSyncState>(&content) {
                        Ok(mut state) => {
                            state.wallet_address = wallet_address.clone(); // Ensure consistency
                            log(LogTag::Transactions, "LOADED", &format!("Loaded sync state: {} transactions, {} cached", 
                                state.total_transactions_fetched, state.cached_signatures.len()));
                            state
                        },
                        Err(e) => {
                            log(LogTag::Transactions, "ERROR", &format!("Failed to parse sync state: {}", e));
                            let mut state = WalletSyncState::default();
                            state.wallet_address = wallet_address.clone();
                            state
                        }
                    }
                },
                Err(e) => {
                    log(LogTag::Transactions, "ERROR", &format!("Failed to read sync state: {}", e));
                    let mut state = WalletSyncState::default();
                    state.wallet_address = wallet_address.clone();
                    state
                }
            }
        } else {
            log(LogTag::Transactions, "INFO", "Creating new sync state");
            let mut state = WalletSyncState::default();
            state.wallet_address = wallet_address.clone();
            state
        };
        
        let mut manager = Self {
            wallet_address,
            cache_dir,
            sync_state_file,
            sync_state,
            transaction_cache: HashMap::new(),
            is_periodic_sync_running: Arc::new(AtomicBool::new(false)),
        };
        
        // Load existing transaction cache into memory
        manager.load_cached_transactions()?;
        
        Ok(manager)
    }
    
    fn load_cached_transactions(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        log(LogTag::Transactions, "INFO", "Loading cached transactions into memory");
        let mut loaded = 0;
        
        for signature in &self.sync_state.cached_signatures.clone() {
            let cache_file = self.cache_dir.join(format!("{}.json", signature));
            if cache_file.exists() {
                match fs::read_to_string(&cache_file) {
                    Ok(content) => {
                        match serde_json::from_str::<CachedTransactionData>(&content) {
                            Ok(cached_tx) => {
                                if is_debug_transactions_enabled() {
                                    log(LogTag::Transactions, "DEBUG", &format!("Loaded cached transaction: {}...", &signature[..8]));
                                }
                                self.transaction_cache.insert(signature.clone(), cached_tx);
                                loaded += 1;
                            },
                            Err(e) => {
                                log(LogTag::Transactions, "ERROR", &format!("Failed to parse cached transaction {}: {}", signature, e));
                                // Remove invalid signature from state
                                self.sync_state.cached_signatures.remove(signature);
                            }
                        }
                    },
                    Err(e) => {
                        log(LogTag::Transactions, "ERROR", &format!("Failed to read cached transaction {}: {}", signature, e));
                        // Remove missing signature from state
                        self.sync_state.cached_signatures.remove(signature);
                    }
                }
            } else {
                // Remove missing signature from state
                self.sync_state.cached_signatures.remove(signature);
            }
        }
        
        log(LogTag::Transactions, "SUCCESS", &format!("Loaded {} cached transactions into memory", loaded));
        Ok(())
    }
    
    fn save_sync_state(&self) -> Result<(), Box<dyn std::error::Error>> {
        let content = serde_json::to_string_pretty(&self.sync_state)?;
        fs::write(&self.sync_state_file, content)?;
        log(LogTag::Transactions, "SAVED", &format!("Sync state saved: {} transactions tracked", 
            self.sync_state.total_transactions_fetched));
        Ok(())
    }
    
    pub async fn initialize_and_sync(&mut self, rpc_client: &RpcClient) -> Result<(), Box<dyn std::error::Error>> {
        log(LogTag::Transactions, "INFO", &format!("Initializing wallet transaction manager for {}", self.wallet_address));
        
        // Fetch missing transactions
        self.fetch_missing_transactions(rpc_client).await?;
        
        // Save updated sync state
        self.save_sync_state()?;
        
        log(LogTag::Transactions, "SUCCESS", &format!("Wallet transaction manager initialized: {} total transactions", 
            self.sync_state.total_transactions_fetched));
        
        Ok(())
    }
    
    /// Start periodic sync background task that runs every 5 seconds
    pub async fn start_periodic_sync(&self, shutdown: Arc<Notify>) -> tokio::task::JoinHandle<()> {
        let wallet_address = self.wallet_address.clone();
        let sync_running = self.is_periodic_sync_running.clone();
        
        tokio::spawn(async move {
            // Mark sync as running
            sync_running.store(true, Ordering::SeqCst);
            
            log(LogTag::Transactions, "INFO", &format!("Starting periodic transaction sync every 5 seconds for wallet: {}", &wallet_address[..8]));
            
            let mut interval = tokio::time::interval(Duration::from_secs(5));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            
            loop {
                tokio::select! {
                    _ = shutdown.notified() => {
                        log(LogTag::Transactions, "INFO", "Periodic transaction sync shutting down");
                        break;
                    }
                    _ = interval.tick() => {
                        // Perform lightweight sync check for new transactions
                        perform_periodic_sync_check().await;
                    }
                }
            }
            
            // Mark sync as stopped
            sync_running.store(false, Ordering::SeqCst);
            log(LogTag::Transactions, "INFO", "Periodic transaction sync task ended");
        })
    }
    
    /// Sync only new transactions (lightweight operation for periodic sync)
    pub async fn sync_new_transactions_only(&mut self, rpc_client: &RpcClient) -> Result<(), Box<dyn std::error::Error>> {
        if is_debug_transactions_enabled() {
            log(LogTag::Transactions, "DEBUG", "Checking for new transactions (periodic sync)");
        }
        
        // Get latest signatures to see what's new (smaller batch for frequent checks)
        let latest_signatures = self.fetch_wallet_signatures(rpc_client, 20, None).await?;
        
        if latest_signatures.is_empty() {
            return Ok(());
        }
        
        let mut new_signatures = Vec::new();
        for sig_info in &latest_signatures {
            if !self.sync_state.cached_signatures.contains(&sig_info.signature) {
                new_signatures.push(sig_info.signature.clone());
            }
        }
        
        if !new_signatures.is_empty() {
            log(LogTag::Transactions, "INFO", &format!("Found {} new transactions in periodic sync", new_signatures.len()));
            self.fetch_and_cache_transactions(rpc_client, &new_signatures).await?;
            self.save_sync_state()?;
        } else if is_debug_transactions_enabled() {
            log(LogTag::Transactions, "DEBUG", "No new transactions found in periodic sync");
        }
        
        Ok(())
    }
    
    async fn fetch_missing_transactions(&mut self, rpc_client: &RpcClient) -> Result<(), Box<dyn std::error::Error>> {
        log(LogTag::Transactions, "INFO", "Checking for missing transactions to fetch");
        
        // First, get latest signatures to see what's new
        let latest_signatures = self.fetch_wallet_signatures(rpc_client, 50, None).await?;
        
        if latest_signatures.is_empty() {
            log(LogTag::Transactions, "INFO", "No transactions found for wallet");
            return Ok(());
        }
        
        let mut new_signatures = Vec::new();
        for sig_info in &latest_signatures {
            if !self.sync_state.cached_signatures.contains(&sig_info.signature) {
                new_signatures.push(sig_info.signature.clone());
            }
        }
        
        if !new_signatures.is_empty() {
            log(LogTag::Transactions, "INFO", &format!("Found {} new transactions to fetch", new_signatures.len()));
            self.fetch_and_cache_transactions(rpc_client, &new_signatures).await?;
        } else {
            log(LogTag::Transactions, "INFO", "All recent transactions are already cached");
        }
        
        // If this is the first sync or we need more history, fetch older transactions
        if self.sync_state.total_transactions_fetched < 1000 {
            log(LogTag::Transactions, "INFO", "Fetching transaction history for complete coverage");
            self.fetch_transaction_history(rpc_client).await?;
        }
        
        Ok(())
    }
    
    async fn fetch_transaction_history(&mut self, rpc_client: &RpcClient) -> Result<(), Box<dyn std::error::Error>> {
        let mut before_signature = self.sync_state.oldest_signature.clone();
        let batch_size = 100;
        let max_total = 1000; // Limit to reasonable number
        
        while self.sync_state.total_transactions_fetched < max_total {
            let signatures = self.fetch_wallet_signatures(rpc_client, batch_size, before_signature.as_deref()).await?;
            
            if signatures.is_empty() {
                log(LogTag::Transactions, "INFO", "No more historical transactions to fetch");
                break;
            }
            
            // Set newest_signature from the first batch only (first signature is newest)
            if self.sync_state.newest_signature.is_none() && !signatures.is_empty() {
                self.sync_state.newest_signature = Some(signatures[0].signature.clone());
                log(LogTag::Transactions, "UPDATE", &format!("Set newest_signature to: {}", signatures[0].signature));
            }
            
            let mut new_signatures = Vec::new();
            for sig_info in &signatures {
                if !self.sync_state.cached_signatures.contains(&sig_info.signature) {
                    new_signatures.push(sig_info.signature.clone());
                }
            }
            
            if !new_signatures.is_empty() {
                log(LogTag::Transactions, "INFO", &format!("Fetching {} historical transactions", new_signatures.len()));
                self.fetch_and_cache_transactions(rpc_client, &new_signatures).await?;
            }
            
            // Update before_signature for next batch (last signature in current batch)
            before_signature = signatures.last().map(|s| s.signature.clone());
            
            // Update oldest_signature to the last signature from this batch (oldest so far)
            if let Some(ref oldest_sig) = before_signature {
                self.sync_state.oldest_signature = Some(oldest_sig.clone());
                log(LogTag::Transactions, "UPDATE", &format!("Updated oldest_signature to: {}", oldest_sig));
            }
            
            // Break if we didn't get a full batch (reached the end)
            if signatures.len() < batch_size {
                log(LogTag::Transactions, "INFO", "Reached end of transaction history");
                break;
            }
        }
        
        Ok(())
    }
    
    async fn fetch_and_cache_transactions(&mut self, rpc_client: &RpcClient, signatures: &[String]) -> Result<(), Box<dyn std::error::Error>> {
        log(LogTag::Transactions, "INFO", &format!("Fetching and caching {} transactions concurrently", signatures.len()));
        
        // Configuration for concurrent processing
        const MAX_CONCURRENT_REQUESTS: usize = 5; // Limit concurrent requests to avoid overwhelming RPC
        const BATCH_DELAY_MS: u64 = 100; // Small delay between batches to be nice to RPC
        
        // Filter out already cached signatures
        let new_signatures: Vec<String> = signatures.iter()
            .filter(|sig| !self.sync_state.cached_signatures.contains(*sig))
            .cloned()
            .collect();
        
        if new_signatures.is_empty() {
            log(LogTag::Transactions, "SKIP", "All transactions already cached");
            return Ok(());
        }
        
        log(LogTag::Transactions, "CONCURRENT", &format!("Processing {} new transactions with {} concurrent workers", 
            new_signatures.len(), MAX_CONCURRENT_REQUESTS));
        
        // Process signatures in chunks to manage concurrency
        let mut total_success = 0;
        let mut total_errors = 0;
        
        for (batch_idx, chunk) in new_signatures.chunks(MAX_CONCURRENT_REQUESTS).enumerate() {
            log(LogTag::Transactions, "BATCH", &format!("Processing batch {} with {} transactions", 
                batch_idx + 1, chunk.len()));
            
            // Process each transaction in this batch sequentially for now to avoid lifetime issues
            // In a future optimization, we can move to true concurrency with owned RPC clients
            for signature in chunk {
                if let Ok(sig) = Signature::from_str(signature) {
                    match self.fetch_and_cache_single_transaction(rpc_client, &sig).await {
                        Ok(()) => {
                            // Transaction was successfully fetched and cached
                            // Update our state
                            self.sync_state.cached_signatures.insert(signature.clone());
                            self.sync_state.total_transactions_fetched += 1;
                            
                            // Save sync state after each successful fetch to preserve progress
                            if let Err(e) = self.save_sync_state() {
                                log(LogTag::Transactions, "ERROR", &format!("Failed to save sync state after caching {}: {}", signature, e));
                            } else {
                                log(LogTag::Transactions, "SAVED", &format!("Sync state saved after caching transaction {} (total: {})", 
                                    signature, self.sync_state.total_transactions_fetched));
                            }
                            
                            total_success += 1;
                        },
                        Err(e) => {
                            log(LogTag::Transactions, "ERROR", &format!("Transaction fetch failed for {}: {}", signature, e));
                            total_errors += 1;
                        }
                    }
                } else {
                    log(LogTag::Transactions, "ERROR", &format!("Invalid signature format: {}", signature));
                    total_errors += 1;
                }
            }
            
            log(LogTag::Transactions, "BATCH_RESULT", &format!("Batch {} completed: {} in batch", 
                batch_idx + 1, chunk.len()));
            
            // Small delay between batches to be respectful to RPC
            if batch_idx + 1 < new_signatures.chunks(MAX_CONCURRENT_REQUESTS).len() {
                tokio::time::sleep(Duration::from_millis(BATCH_DELAY_MS)).await;
            }
        }
        
        self.sync_state.last_sync_time = Utc::now();
        log(LogTag::Transactions, "SUCCESS", &format!("Batch fetch completed: {} success, {} errors, {} total cached", 
            total_success, total_errors, self.sync_state.total_transactions_fetched));
        
        Ok(())
    }
    
    async fn fetch_and_cache_single_transaction(
        &mut self,
        rpc_client: &RpcClient,
        signature: &Signature,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let sig_str = signature.to_string();
        
        // Check if already cached
        if self.transaction_cache.contains_key(&sig_str) {
            return Ok(());
        }
        
        log(LogTag::Transactions, "PREMIUM_RPC", &format!("Fetching transaction {} using premium RPC", &sig_str[..8]));
        
        // Use premium RPC for individual transaction fetching
        let tx_result = rpc_client.get_transaction_details_premium_rpc(&sig_str).await
            .map_err(|e| {
                let error_msg = e.to_string();
                if error_msg.contains("Transaction not found") {
                    log(LogTag::Transactions, "SKIP", &format!("Transaction {} not yet available, will retry later", &sig_str[..8]));
                    format!("Transaction not found: {}", error_msg)
                } else {
                    log(LogTag::Transactions, "ERROR", &format!("Failed to fetch transaction {}: {}", &sig_str[..8], error_msg));
                    error_msg
                }
            })?;
        
        // Create cached data
        let cached_tx = CachedTransactionData {
            signature: sig_str.clone(),
            transaction_data: tx_result,
            cached_at: Utc::now(),
        };
        
        // Save to disk
        let cache_file = self.cache_dir.join(format!("{}.json", sig_str));
        let content = serde_json::to_string_pretty(&cached_tx)?;
        fs::write(&cache_file, content)?;
        
        // Store in memory
        self.transaction_cache.insert(sig_str.clone(), cached_tx);
        
        log(LogTag::Transactions, "CACHED", &format!("Transaction {} cached successfully", &sig_str[..8]));
        Ok(())
    }
    
    // Static version for concurrent processing
    async fn fetch_and_cache_single_transaction_static_sync(
        rpc_client: &RpcClient,
        signature: &Signature,
        signature_str: &str,
        cache_dir: &Path,
    ) -> Result<CachedTransactionData, Box<dyn std::error::Error + Send + Sync>> {
        let cache_file = cache_dir.join(format!("{}.json", signature_str));
        
        // Check if already cached on disk
        if cache_file.exists() {
            match fs::read_to_string(&cache_file) {
                Ok(content) => {
                    if let Ok(cached_data) = serde_json::from_str::<CachedTransactionData>(&content) {
                        log(LogTag::Transactions, "CACHE_HIT", &format!("Transaction {} loaded from disk cache", signature_str));
                        return Ok(cached_data);
                    }
                },
                Err(_) => {
                    // Cache file exists but is corrupted, we'll refetch
                    log(LogTag::Transactions, "CACHE_CORRUPT", &format!("Corrupted cache file for {}, refetching", signature_str));
                }
            }
        }
        
        log(LogTag::Transactions, "PREMIUM_RPC", &format!("Fetching transaction {} using premium RPC (concurrent)", &signature_str[..8]));
        
        // Use premium RPC for concurrent transaction fetching
        let tx_result = rpc_client.get_transaction_details_premium_rpc(signature_str).await
            .map_err(|e| Box::new(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())) as Box<dyn std::error::Error + Send + Sync>)?;
        
        // Create cached data
        let cached_tx = CachedTransactionData {
            signature: signature_str.to_string(),
            transaction_data: tx_result,
            cached_at: Utc::now(),
        };
        
        // Save to disk
        let content = serde_json::to_string_pretty(&cached_tx)
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
        fs::write(&cache_file, content)
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
        
        log(LogTag::Transactions, "CACHED", &format!("Transaction {} cached successfully (concurrent)", signature_str));
        Ok(cached_tx)
    }
    
    async fn fetch_wallet_signatures(
        &self,
        rpc_client: &RpcClient,
        limit: usize,
        before: Option<&str>,
    ) -> Result<Vec<solana_client::rpc_response::RpcConfirmedTransactionStatusWithSignature>, Box<dyn std::error::Error>> {
        
        let pubkey = Pubkey::from_str(&self.wallet_address)?;
        
        log(LogTag::Transactions, "MAIN_RPC", &format!("Fetching {} signatures using main RPC", limit));
        
        // Use main RPC for lightweight signature fetching
        let signatures = rpc_client.get_wallet_signatures_main_rpc(&pubkey, limit, before).await
            .map_err(|e| format!("Failed to get signatures from main RPC: {}", e))?;
        
        Ok(signatures)
    }
    
    pub fn get_cached_transaction(&self, signature: &str) -> Option<&CachedTransactionData> {
        self.transaction_cache.get(signature)
    }
    
    /// Get all cached transactions, optionally limited to most recent N transactions
    pub fn get_cached_transactions(&self, limit: Option<usize>) -> Vec<&CachedTransactionData> {
        let mut transactions: Vec<&CachedTransactionData> = self.transaction_cache.values().collect();
        
        // Sort by cached time (most recent first)
        transactions.sort_by(|a, b| b.cached_at.cmp(&a.cached_at));
        
        if let Some(limit) = limit {
            transactions.into_iter().take(limit).collect()
        } else {
            transactions
        }
    }
    
    /// Get or fetch a specific transaction by signature for position verification
    /// This ensures the transaction is cached and available for analysis
    pub async fn get_or_fetch_transaction(&mut self, signature: &str, rpc_client: &RpcClient) -> Result<(), String> {
        // Check if already cached
        if self.transaction_cache.contains_key(signature) {
            log(LogTag::Transactions, "CACHED", &format!("Transaction {} found in cache", signature));
            return Ok(());
        }
        
        // Not cached, fetch it with retry logic
        log(LogTag::Transactions, "FETCH", &format!("Transaction {} not cached, fetching from RPC", signature));
        
        // Convert string to Signature
        let signature_obj = solana_sdk::signature::Signature::from_str(signature)
            .map_err(|e| format!("Invalid signature format: {}", e))?;
        
        // Retry logic for fresh transactions that might not be confirmed yet
        let mut attempts = 0;
        const MAX_ATTEMPTS: u32 = 3;
        const RETRY_DELAY_MS: u64 = 1000; // 1 second between retries
        
        loop {
            attempts += 1;
            
            let result = self.fetch_and_cache_single_transaction(rpc_client, &signature_obj).await;
            
            match result {
                Ok(_) => {
                    log(LogTag::Transactions, "SUCCESS", &format!("Transaction {} fetched and cached on attempt {}", signature, attempts));
                    return Ok(());
                }
                Err(e) => {
                    let error_string = e.to_string();
                    drop(e); // Explicitly drop the error
                    
                    // Now handle the error using only the string
                    let should_continue = self.handle_fetch_error_string(signature, &error_string, attempts).await?;
                    if !should_continue {
                        return Ok(()); // This will never happen since handle_fetch_error returns Err for final failures
                    }
                }
            }
        }
    }
    
    /// Handle fetch error with proper Send + Sync compatibility
    async fn handle_fetch_error_string(&self, signature: &str, error_string: &str, attempts: u32) -> Result<bool, String> {
        const MAX_ATTEMPTS: u32 = 3;
        const RETRY_DELAY_MS: u64 = 1000;
        
        // Check if this is a "transaction not yet available" error
        let is_temp_unavailable = error_string.contains("Transaction not yet available") || 
                                error_string.contains("Transaction not found") || 
                                error_string.contains("null, expected struct");
        
        if attempts >= MAX_ATTEMPTS {
            if is_temp_unavailable {
                let warning_msg = format!("Transaction {} is not yet available after {} attempts (very recent transaction), verification may be delayed", signature, attempts);
                log(LogTag::Transactions, "WARNING", &warning_msg);
                return Err(warning_msg);
            } else {
                let error_msg = format!("Failed to fetch transaction {} after {} attempts: {}", signature, attempts, error_string);
                log(LogTag::Transactions, "ERROR", &error_msg);
                return Err(error_msg);
            }
        } else {
            let retry_msg = if is_temp_unavailable {
                format!("Transaction {} not yet available (attempt {}), retrying in {}ms...", signature, attempts, RETRY_DELAY_MS)
            } else {
                format!("Fetch attempt {} failed for {}: {}. Retrying in {}ms...", attempts, signature, error_string, RETRY_DELAY_MS)
            };
            
            if is_debug_transactions_enabled() {
                log(LogTag::Transactions, "DEBUG", &retry_msg);
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(RETRY_DELAY_MS)).await;
            return Ok(true); // Continue retrying
        }
    }

    /// Verify and analyze a swap transaction for position data
    /// Returns verified transaction data including effective price, token amounts, and fees
    pub async fn verify_swap_transaction(&mut self, signature: &str, expected_direction: &str, rpc_client: &RpcClient) -> Result<VerifiedSwapData, String> {
        // Try to ensure transaction is cached with retry logic
        match self.get_or_fetch_transaction(signature, rpc_client).await {
            Ok(_) => {
                log(LogTag::Transactions, "SUCCESS", &format!("Transaction {} cached successfully", signature));
            }
            Err(e) => {
                let error_msg = e.to_string();
                if error_msg.contains("not yet available") || error_msg.contains("very recent transaction") {
                    // For very recent transactions, we can still try to analyze from the blockchain
                    log(LogTag::Transactions, "WARNING", &format!("Transaction {} not yet cached, attempting direct blockchain analysis", signature));
                } else {
                    return Err(format!("Failed to cache transaction {}: {}", signature, error_msg));
                }
            }
        }
        
        log(LogTag::Transactions, "VERIFY", &format!("Verifying swap transaction {} for {} operation", signature, expected_direction));
        
        // Use the comprehensive swap analysis from transactions_tools
        // First try to analyze using the signature-only method (works even if not cached)
        match crate::transactions_tools::analyze_post_swap_transaction_simple(signature, &self.wallet_address).await {
            Ok(analysis) => {
                log(LogTag::Transactions, "SUCCESS", &format!(
                    "Transaction {} verified using simple analysis - Direction: {}, Effective Price: {:.12} SOL", 
                    signature, expected_direction, analysis.effective_price
                ));
                
                // Convert the analysis to our VerifiedSwapData format
                Ok(VerifiedSwapData {
                    signature: signature.to_string(),
                    success: true,
                    direction: expected_direction.to_string(),
                    token_mint: analysis.token_mint.unwrap_or_default(),
                    sol_amount: analysis.sol_amount,
                    token_amount: analysis.token_amount as u64,
                    token_decimals: analysis.token_decimals.unwrap_or(9) as u8,
                    effective_price: analysis.effective_price,
                    transaction_fee: analysis.transaction_fee.unwrap_or(0),
                    priority_fee: analysis.priority_fee,
                    ata_created: analysis.ata_created,
                    ata_closed: analysis.ata_closed,
                    ata_rent_paid: 0, // Default value
                    ata_rent_reclaimed: (analysis.ata_rent_reclaimed.unwrap_or(0.0) * 1_000_000_000.0) as u64,
                    slot: analysis.slot.unwrap_or(0),
                    block_time: analysis.block_time,
                })
            }
            Err(e) => {
                // If simple analysis fails, try to get cached transaction data and use legacy analysis
                if let Some(cached_tx) = self.transaction_cache.get(signature) {
                    log(LogTag::Transactions, "WARNING", &format!(
                        "Simple analysis failed for {}, falling back to legacy analysis: {}", 
                        signature, e
                    ));
                    
                    // Fallback to the original analysis method
                    self.analyze_transaction_for_verified_swap(&cached_tx.transaction_data, expected_direction).await
                } else {
                    // Transaction not cached and simple analysis failed
                    Err(format!("Transaction verification failed: simple analysis error ({}), and transaction not cached", e))
                }
            }
        }
    }
    
    /// Analyze a cached transaction for swap data extraction
    async fn analyze_transaction_for_verified_swap(&self, transaction: &EncodedConfirmedTransactionWithStatusMeta, expected_direction: &str) -> Result<VerifiedSwapData, String> {
        if is_debug_transactions_enabled() {
            log(LogTag::Transactions, "DEBUG", "Starting transaction analysis for swap verification");
        }
        
        // Get transaction meta
        let meta = transaction.transaction.meta.as_ref()
            .ok_or("No transaction meta found")?;

        if is_debug_transactions_enabled() {
            log(LogTag::Transactions, "DEBUG", &format!("Transaction meta found, error: {:?}", meta.err));
        }

        // Try to get signature from multiple possible sources
        let signature = if let Some(decoded_tx) = transaction.transaction.transaction.decode() {
            // Raw transaction format
            if is_debug_transactions_enabled() {
                log(LogTag::Transactions, "DEBUG", "Successfully decoded raw transaction");
            }
            decoded_tx.signatures.get(0)
                .ok_or("No signature found in decoded transaction")?
                .to_string()
        } else {
            // Fallback: try to extract from the transaction structure
            if is_debug_transactions_enabled() {
                log(LogTag::Transactions, "DEBUG", "Raw transaction decode failed, trying alternative signature extraction");
            }
            
            // For JsonParsed transactions, we might need to extract differently
            match &transaction.transaction.transaction {
                solana_transaction_status::EncodedTransaction::Json(ui_tx) => {
                    if let Some(first_signature) = ui_tx.signatures.get(0) {
                        first_signature.clone()
                    } else {
                        return Err("No signature found in JSON transaction".into());
                    }
                }
                solana_transaction_status::EncodedTransaction::LegacyBinary(_) => {
                    return Err("Cannot extract signature from legacy binary transaction without decoding".into());
                }
                solana_transaction_status::EncodedTransaction::Binary(_, _) => {
                    return Err("Cannot extract signature from binary transaction without decoding".into());
                }
                _ => {
                    return Err("Unknown transaction encoding format".into());
                }
            }
        };
        
        if is_debug_transactions_enabled() {
            log(LogTag::Transactions, "DEBUG", &format!("Extracted signature: {}", signature));
        }
        
        let slot = transaction.slot;
        let block_time = transaction.block_time;
        
        let success = meta.err.is_none();
        
        if !success {
            return Ok(VerifiedSwapData {
                signature,
                success: false,
                direction: expected_direction.to_string(),
                token_mint: String::new(),
                sol_amount: 0.0,
                token_amount: 0,
                token_decimals: 0,
                effective_price: 0.0,
                transaction_fee: meta.fee,
                priority_fee: None,
                ata_created: false,
                ata_closed: false,
                ata_rent_paid: 0,
                ata_rent_reclaimed: 0,
                slot,
                block_time,
            });
        }
        
        // Get wallet address for analysis
        let wallet_pubkey = Pubkey::from_str(&self.wallet_address)
            .map_err(|e| format!("Invalid wallet address: {}", e))?;
        
        // Analyze SOL balance changes
        let sol_change = self.analyze_sol_balance_change(&wallet_pubkey, meta)
            .map_err(|e| format!("Failed to analyze SOL balance change: {}", e))?;
        
        // Analyze token balance changes  
        let token_changes = self.analyze_token_balance_changes(&wallet_pubkey, meta);
        
        // Find the main token involved (the one with the largest balance change)
        let main_token = token_changes.iter()
            .max_by(|a, b| a.1.abs().partial_cmp(&b.1.abs()).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(mint, _)| mint.clone())
            .unwrap_or_default();
        
        let token_amount = token_changes.get(&main_token).map(|change| change.abs() as u64).unwrap_or(0);
        
        // Get token decimals
        let token_decimals = if main_token.is_empty() { 
            0 
        } else { 
            get_token_decimals_safe_local(&main_token).await.unwrap_or(6) 
        };
        
        // Calculate effective price
        let effective_price = if token_amount > 0 && sol_change.abs() > 0.0 {
            sol_change.abs() / (token_amount as f64 / 10_f64.powi(token_decimals as i32))
        } else {
            0.0
        };
        
        // Analyze ATA operations
        let (ata_created, ata_closed, ata_rent_paid, ata_rent_reclaimed) = self.analyze_ata_operations(meta);
        
        // Extract priority fee
        let priority_fee = self.extract_priority_fee_from_meta(meta);
        
        Ok(VerifiedSwapData {
            signature,
            success: true,
            direction: expected_direction.to_string(),
            token_mint: main_token,
            sol_amount: sol_change.abs(),
            token_amount,
            token_decimals,
            effective_price,
            transaction_fee: meta.fee,
            priority_fee,
            ata_created,
            ata_closed,
            ata_rent_paid,
            ata_rent_reclaimed,
            slot,
            block_time,
        })
    }
    
    /// Analyze SOL balance change for a wallet in a transaction
    fn analyze_sol_balance_change(&self, wallet_pubkey: &Pubkey, meta: &solana_transaction_status::UiTransactionStatusMeta) -> Result<f64, String> {
        // Find wallet index in account keys
        let pre_balances = &meta.pre_balances;
        let post_balances = &meta.post_balances;
        
        if pre_balances.len() != post_balances.len() {
            return Err("Balance arrays length mismatch".to_string());
        }
        
        // For now, we'll use account index 0 (typically the fee payer/wallet)
        // In a more complete implementation, we'd parse account keys to find the exact wallet index
        if let (Some(&pre_balance), Some(&post_balance)) = (pre_balances.get(0), post_balances.get(0)) {
            let change_lamports = (post_balance as i64) - (pre_balance as i64);
            Ok(lamports_to_sol(change_lamports.abs() as u64))
        } else {
            Ok(0.0)
        }
    }
    
    /// Analyze token balance changes for a wallet
    fn analyze_token_balance_changes(&self, _wallet_pubkey: &Pubkey, meta: &solana_transaction_status::UiTransactionStatusMeta) -> HashMap<String, f64> {
        let mut token_changes = HashMap::new();
        
        // TODO: Implement token balance analysis
        // For now, return empty map
        
        token_changes
    }
    
    /// Analyze ATA creation/closure operations
    fn analyze_ata_operations(&self, meta: &solana_transaction_status::UiTransactionStatusMeta) -> (bool, bool, u64, u64) {
        // For now, return default values
        // In a complete implementation, we'd analyze the instruction logs for ATA operations
        (false, false, 0, 0)
    }
    
    /// Extract priority fee from transaction meta
    fn extract_priority_fee_from_meta(&self, meta: &solana_transaction_status::UiTransactionStatusMeta) -> Option<u64> {
        // Priority fee extraction would require parsing compute budget instructions
        // For now, return None
        None
    }
    
    pub async fn analyze_recent_swaps(&self, limit: usize) -> SwapAnalysis {
        log(LogTag::Transactions, "INFO", &format!("Analyzing {} recent swaps from cache", limit));
        
        let mut swap_transactions = Vec::new();
        let mut processed = 0;
        
        // Get signatures sorted by newest first (we'll need to implement proper sorting)
        let mut signatures: Vec<_> = self.sync_state.cached_signatures.iter().collect();
        signatures.sort(); // This is basic sorting, ideally we'd sort by slot/timestamp
        
        for signature in signatures.iter().rev().take(1000) { // Check up to 1000 recent transactions
            if let Some(cached_tx) = self.transaction_cache.get(*signature) {
                processed += 1;
                
                if is_debug_transactions_enabled() {
                    log(LogTag::Transactions, "DEBUG", &format!("Processing transaction {} (slot: {:?})", 
                        &signature[..8], cached_tx.transaction_data.slot));
                }
                
                if let Some(swap) = self.analyze_transaction_for_swap(cached_tx).await {
                    log(LogTag::Transactions, "FOUND", &format!("Swap detected: {} {} {} SOL for {} tokens", 
                        swap.swap_type, swap.token_symbol, swap.sol_amount, swap.token_amount));
                    swap_transactions.push(swap);
                    
                    if swap_transactions.len() >= limit {
                        break;
                    }
                }
            }
        }
        
        log(LogTag::Transactions, "SUCCESS", &format!("Analysis complete. Found {} swaps from {} transactions", 
            swap_transactions.len(), processed));
        
        self.generate_swap_analysis(swap_transactions)
    }
    
    async fn analyze_transaction_for_swap(&self, cached_tx: &CachedTransactionData) -> Option<SwapTransaction> {
        let tx = &cached_tx.transaction_data;
        let signature = &cached_tx.signature;
        
        let meta = tx.transaction.meta.as_ref()?;
        let transaction = tx.transaction.transaction.decode()?;
        
        // Check if transaction was successful
        if meta.err.is_some() {
            return None;
        }
        
        // Analyze pre/post balances for SOL changes
        let wallet_pubkey = match Pubkey::from_str(&self.wallet_address) {
            Ok(pk) => pk,
            Err(_) => return None,
        };
        
        // Find wallet account index
        let account_keys = match transaction.message {
            solana_sdk::message::VersionedMessage::Legacy(ref msg) => &msg.account_keys,
            solana_sdk::message::VersionedMessage::V0(ref msg) => &msg.account_keys,
        };
        
        let wallet_index = account_keys.iter()
            .position(|key| *key == wallet_pubkey)?;
        
        let pre_balance = meta.pre_balances.get(wallet_index).copied().unwrap_or(0);
        let post_balance = meta.post_balances.get(wallet_index).copied().unwrap_or(0);
        let sol_change = lamports_to_sol((post_balance as i64 - pre_balance as i64) as u64);
        
        // Analyze token account changes
        let pre_token_balances = &meta.pre_token_balances;
        let post_token_balances = &meta.post_token_balances;
        
        // Look for token balance changes that indicate swaps
        let post_balances_vec = match post_token_balances {
            solana_transaction_status::option_serializer::OptionSerializer::Some(balances) => balances,
            _ => return None,
        };
        
        for post_balance in post_balances_vec {
            // Extract owner and mint values from OptionSerializer
            let owner_str = match &post_balance.owner {
                solana_transaction_status::option_serializer::OptionSerializer::Some(owner) => owner,
                _ => continue,
            };
            
            let mint_str = &post_balance.mint;
            
            if owner_str == &self.wallet_address && mint_str != "So11111111111111111111111111111111111111112" {
                // This is a token account owned by our wallet (not SOL)
                let post_amount = post_balance.ui_token_amount.amount.parse::<u64>().unwrap_or(0);
                let decimals = post_balance.ui_token_amount.decimals;
                
                // Find corresponding pre-balance
                let pre_amount = match pre_token_balances {
                    solana_transaction_status::option_serializer::OptionSerializer::Some(pre_balances) => {
                        pre_balances.iter()
                            .find(|pre| {
                                let pre_mint = &pre.mint;
                                let pre_owner = match &pre.owner {
                                    solana_transaction_status::option_serializer::OptionSerializer::Some(owner) => owner,
                                    _ => return false,
                                };
                                pre_mint == mint_str && pre_owner == owner_str
                            })
                            .and_then(|pre| pre.ui_token_amount.amount.parse::<u64>().ok())
                            .unwrap_or(0)
                    },
                    _ => 0,
                };
                
                let token_change = post_amount as i64 - pre_amount as i64;
                
                // If there's a significant token change and SOL change, it's likely a swap
                if token_change.abs() > 1000 && sol_change.abs() > 0.001 {
                    let swap_type = if token_change > 0 { "buy" } else { "sell" };
                    
                    // Get token symbol from database
                    let token_symbol = get_token_symbol_safe(mint_str).await;
                    
                    // Validate decimals from transaction against our token database
                    let validated_decimals = if let Some(db_decimals) = get_token_decimals_safe_local(mint_str).await {
                        if db_decimals != decimals {
                            log(LogTag::Transactions, "WARNING", &format!("Decimal mismatch for {}: transaction={}, database={}, using database value", 
                                &mint_str[..8], decimals, db_decimals));
                            db_decimals
                        } else {
                            decimals
                        }
                    } else {
                        decimals // Fallback to transaction data if database lookup fails
                    };
                    
                    // Calculate effective price using validated decimals
                    let token_amount_ui = token_change.abs() as f64 / 10f64.powi(validated_decimals as i32);
                    let effective_price = if token_amount_ui > 0.0 {
                        sol_change.abs() / token_amount_ui
                    } else {
                        0.0
                    };
                    
                    // Calculate fees
                    let fees_paid = lamports_to_sol(meta.fee);
                    
                    return Some(SwapTransaction {
                        signature: signature.clone(),
                        slot: tx.slot,
                        block_time: tx.block_time,
                        swap_type: swap_type.to_string(),
                        token_mint: mint_str.to_string(),
                        token_symbol,
                        sol_amount: sol_change.abs(),
                        token_amount: token_change.abs() as u64,
                        token_decimals: validated_decimals,
                        effective_price,
                        fees_paid,
                        success: true,
                    });
                }
            }
        }
        
        None
    }
    
    fn generate_swap_analysis(&self, swaps: Vec<SwapTransaction>) -> SwapAnalysis {
        let total_swaps = swaps.len();
        let buy_swaps = swaps.iter().filter(|s| s.swap_type == "buy").count();
        let sell_swaps = swaps.iter().filter(|s| s.swap_type == "sell").count();
        
        let total_sol_in = swaps.iter()
            .filter(|s| s.swap_type == "buy")
            .map(|s| s.sol_amount)
            .sum::<f64>();
        
        let total_sol_out = swaps.iter()
            .filter(|s| s.swap_type == "sell")
            .map(|s| s.sol_amount)
            .sum::<f64>();
        
        let total_fees = swaps.iter().map(|s| s.fees_paid).sum::<f64>();
        let net_sol_change = total_sol_out - total_sol_in - total_fees;
        
        SwapAnalysis {
            total_swaps,
            buy_swaps,
            sell_swaps,
            total_sol_in,
            total_sol_out,
            total_fees,
            net_sol_change,
            recent_swaps: swaps,
        }
    }
    
    pub fn get_sync_stats(&self) -> (usize, usize, String) {
        let cached_count = self.sync_state.cached_signatures.len();
        let total_fetched = self.sync_state.total_transactions_fetched;
        let last_sync = self.sync_state.last_sync_time.format("%Y-%m-%d %H:%M:%S UTC").to_string();
        (cached_count, total_fetched, last_sync)
    }
    
    /// Get detailed sync statistics including periodic sync status
    pub fn get_detailed_sync_stats(&self) -> (usize, usize, String, bool, Option<String>, Option<String>) {
        let cached_count = self.sync_state.cached_signatures.len();
        let total_fetched = self.sync_state.total_transactions_fetched;
        let last_sync = self.sync_state.last_sync_time.format("%Y-%m-%d %H:%M:%S UTC").to_string();
        let is_periodic_running = self.is_periodic_sync_running.load(Ordering::SeqCst);
        let oldest_sig = self.sync_state.oldest_signature.clone();
        let newest_sig = self.sync_state.newest_signature.clone();
        
        (cached_count, total_fetched, last_sync, is_periodic_running, oldest_sig, newest_sig)
    }
    
    pub fn display_analysis(analysis: &SwapAnalysis) {
        // Only display detailed analysis if debug mode is enabled
        if is_debug_transactions_enabled() {
            log(LogTag::Transactions, "INFO", "=== WALLET SWAP ANALYSIS RESULTS ===");
            
            log(LogTag::Transactions, "RESULTS", &format!("Analysis Summary: {} total swaps ({} buy, {} sell)", 
                analysis.total_swaps, analysis.buy_swaps, analysis.sell_swaps));
            
            log(LogTag::Transactions, "INFO", "📊 Summary:");
            log(LogTag::Transactions, "INFO", &format!("   Total swaps found: {}", analysis.total_swaps));
            log(LogTag::Transactions, "INFO", &format!("   Buy transactions: {}", analysis.buy_swaps));
            log(LogTag::Transactions, "INFO", &format!("   Sell transactions: {}", analysis.sell_swaps));
            log(LogTag::Transactions, "INFO", &format!("   Total SOL spent (buys): {:.6}", analysis.total_sol_in));
            log(LogTag::Transactions, "INFO", &format!("   Total SOL received (sells): {:.6}", analysis.total_sol_out));
            log(LogTag::Transactions, "INFO", &format!("   Total fees paid: {:.6}", analysis.total_fees));
            log(LogTag::Transactions, "INFO", &format!("   Net SOL change: {:.6}", analysis.net_sol_change));
        }
        
        log(LogTag::Transactions, "CALCULATION", &format!("Net P&L: {:.6} SOL (buys: {:.6}, sells: {:.6}, fees: {:.6})", 
            analysis.net_sol_change, analysis.total_sol_in, analysis.total_sol_out, analysis.total_fees));
        
        if analysis.net_sol_change > 0.0 {
            if is_debug_transactions_enabled() {
                log(LogTag::Transactions, "INFO", &format!("   🟢 Net profit: {:.6} SOL", analysis.net_sol_change));
            }
            log(LogTag::Transactions, "PROFIT", &format!("Net profit: {:.6} SOL", analysis.net_sol_change));
        } else if analysis.net_sol_change < 0.0 {
            if is_debug_transactions_enabled() {
                log(LogTag::Transactions, "INFO", &format!("   🔴 Net loss: {:.6} SOL", analysis.net_sol_change.abs()));
            }
            log(LogTag::Transactions, "LOSS", &format!("Net loss: {:.6} SOL", analysis.net_sol_change.abs()));
        } else {
            if is_debug_transactions_enabled() {
                log(LogTag::Transactions, "INFO", "   ⚪ Break even");
            }
            log(LogTag::Transactions, "EVEN", "Portfolio is break even");
        }
        
        if !analysis.recent_swaps.is_empty() && is_debug_transactions_enabled() {
            log(LogTag::Transactions, "INFO", "📋 Recent Swap Details:");
            log(LogTag::Transactions, "INFO", &format!("{:<8} {:<16} {:<12} {:<15} {:<12} {:<10}", 
                     "Type", "Token", "SOL Amount", "Price", "Fees", "Status"));
            log(LogTag::Transactions, "INFO", &format!("{}", "=".repeat(80)));
            
            log(LogTag::Transactions, "DETAILS", &format!("Displaying {} recent swaps", analysis.recent_swaps.len()));
            
            for (i, swap) in analysis.recent_swaps.iter().enumerate() {
                let status = if swap.success { "✅" } else { "❌" };
                log(LogTag::Transactions, "INFO", &format!("{:<8} {:<16} {:<12.6} {:<15.9} {:<12.6} {:<10}",
                         swap.swap_type,
                         swap.token_symbol,
                         swap.sol_amount,
                         swap.effective_price,
                         swap.fees_paid,
                         status));
                
                log(LogTag::Transactions, "DETAIL", &format!("Swap {}: {} {} tokens for {:.6} SOL at {:.9} SOL/token", 
                    i + 1, swap.swap_type, swap.token_symbol, swap.sol_amount, swap.effective_price));
            }
        } else if !analysis.recent_swaps.is_empty() {
            // Log summary without detailed table when debug is disabled
            log(LogTag::Transactions, "DETAILS", &format!("Found {} recent swaps", analysis.recent_swaps.len()));
        }
        
        if is_debug_transactions_enabled() {
            log(LogTag::Transactions, "INFO", "✅ Analysis complete. Check the details above for P&L verification.");
        }
        log(LogTag::Transactions, "COMPLETE", "Wallet swap analysis finished successfully");
    }
}

/// Get token symbol from database, with fallback to shortened mint
async fn get_token_symbol_safe(mint: &str) -> String {
    // Try to get symbol from token database
    if let Ok(db) = TokenDatabase::new() {
        if let Ok(Some(token)) = db.get_token_by_mint(mint) {
            if !token.symbol.is_empty() && token.symbol != "unknown" {
                return token.symbol;
            }
        }
    }
    
    // Fallback to shortened mint address
    if mint.len() >= 8 {
        format!("{}...{}", &mint[..6], &mint[mint.len()-4..])
    } else {
        mint.to_string()
    }
}

/// Get token info (symbol and name) from database
async fn get_token_info_safe(mint: &str) -> (String, String) {
    // Try to get info from token database
    if let Ok(db) = TokenDatabase::new() {
        if let Ok(Some(token)) = db.get_token_by_mint(mint) {
            let symbol = if !token.symbol.is_empty() && token.symbol != "unknown" {
                token.symbol
            } else {
                format!("{}...{}", &mint[..6], &mint[mint.len()-4..])
            };
            
            let name = if !token.name.is_empty() && token.name != "unknown" {
                token.name
            } else {
                symbol.clone()
            };
            
            return (symbol, name);
        }
    }
    
    // Fallback to shortened mint address for both
    let fallback = if mint.len() >= 8 {
        format!("{}...{}", &mint[..6], &mint[mint.len()-4..])
    } else {
        mint.to_string()
    };
    
    (fallback.clone(), fallback)
}

/// Get token decimals with proper error handling and cache
async fn get_token_decimals_safe_local(mint: &str) -> Option<u8> {
    // Use the centralized decimals function from tokens module
    get_token_decimals(mint).await
}

/// Comprehensive Swap Transaction Discovery and Analysis Functions
/// 
/// This module provides functionality to analyze wallet transactions for swap operations:
/// - Scans entire transaction history for swap patterns  
/// - Detects token purchases and sales from instruction analysis
/// - Extracts swap amounts, prices, and fees
/// - Identifies swap routers (Jupiter, Raydium, etc.)
/// - Provides detailed swap analytics and statistics
/// - Exports swap data to JSON for further analysis
/// - Displays transactions in table format with types

use tabled::{Tabled, Table, settings::{Style, Alignment, object::Rows, Modify}};
use clap::Parser;

/// Display structure for wallet transactions table
#[derive(Tabled)]
pub struct TransactionDisplay {
    #[tabled(rename = "📅 Date")]
    date: String,
    #[tabled(rename = "⏰ Time")]
    time: String,
    #[tabled(rename = "🔑 Signature")]
    signature: String,
    #[tabled(rename = "� Value (SOL)")]
    value_sol: String,
    #[tabled(rename = "📋 Instructions")]
    instructions: String,
    #[tabled(rename = "�💸 Fee (SOL)")]
    fee_sol: String,
    #[tabled(rename = "✅ Success")]
    success: String,
    #[tabled(rename = "🏷️ Type")]
    transaction_type: String,
}

/// Run comprehensive swap transaction discovery and analysis
pub async fn run_swap_analysis(args: crate::transactions_tools::Args) -> Result<(), Box<dyn std::error::Error>> {
    use crate::{
        rpc::{init_rpc_client, get_rpc_client},
        logger::{log, LogTag},
        global::read_configs,
        transactions_tools::{
            analyze_wallet_swaps, get_all_configured_wallets,
            display_comprehensive_results, export_results_to_json
        },
    };

    // Initialize RPC client
    init_rpc_client()?;
    let _rpc_client = get_rpc_client();
    
    // Read configurations
    let _configs = read_configs().map_err(|e| format!("Failed to read configs: {}", e))?;
    
    // If table-only mode is requested, handle it separately without any analysis
    if args.table_only {
        log(LogTag::Transactions, "INFO", "� Initializing wallet transaction manager for table-only display");
        initialize_wallet_transaction_manager().await?;
        
        // Determine which wallets to display
        let wallets_to_display = if let Some(ref wallet_addr) = args.wallet {
            vec![wallet_addr.clone()]
        } else {
            // Get all configured wallets
            get_all_configured_wallets().await?
        };
        
        if wallets_to_display.is_empty() {
            return Err("No wallets found to display".into());
        }
        
        log(LogTag::Transactions, "INFO", &format!("📊 Displaying table for {} wallet(s)", wallets_to_display.len()));
        
        // Display only the transaction table(s)
        for wallet_address in &wallets_to_display {
            match display_wallet_transactions_table(wallet_address).await {
                Ok(()) => {
                    log(LogTag::Transactions, "SUCCESS", &format!("📊 Transaction table displayed for wallet {}", &wallet_address[..8]));
                }
                Err(e) => {
                    log(LogTag::Transactions, "ERROR", &format!("❌ Failed to display transaction table for wallet {}: {}", &wallet_address[..8], e));
                }
            }
        }
        
        return Ok(());
    }
    
    // For full analysis mode (including when table is combined with other options)
    log(LogTag::Transactions, "INFO", "🔍 Starting comprehensive swap transaction discovery");
    
    // Initialize wallet transaction manager if table display is requested
    if args.table {
        log(LogTag::Transactions, "INFO", "📊 Initializing wallet transaction manager for table-only display");
        initialize_wallet_transaction_manager().await?;
    }
    
    // Determine which wallets to analyze
    let wallets_to_analyze = if let Some(ref wallet_addr) = args.wallet {
        vec![wallet_addr.clone()]
    } else {
        // Get all configured wallets
        get_all_configured_wallets().await?
    };
    
    if wallets_to_analyze.is_empty() {
        return Err("No wallets found to analyze".into());
    }
    
    log(LogTag::Transactions, "INFO", &format!("📊 Analyzing {} wallet(s)", wallets_to_analyze.len()));
    
    let mut all_reports = Vec::new();
    
    for wallet_address in &wallets_to_analyze {
        log(LogTag::Transactions, "INFO", &format!("🔍 Analyzing wallet: {}", &wallet_address[..8]));
        
        // Display transaction table if requested
        if args.table {
            match display_wallet_transactions_table(wallet_address).await {
                Ok(()) => {
                    log(LogTag::Transactions, "SUCCESS", &format!("📊 Transaction table displayed for wallet {}", &wallet_address[..8]));
                }
                Err(e) => {
                    log(LogTag::Transactions, "ERROR", &format!("❌ Failed to display transaction table for wallet {}: {}", &wallet_address[..8], e));
                }
            }
        }
        
        match analyze_wallet_swaps(wallet_address, &args).await {
            Ok(report) => {
                log(LogTag::Transactions, "SUCCESS", &format!(
                    "✅ Found {} swaps in wallet {}", 
                    report.analytics.total_swaps, 
                    &wallet_address[..8]
                ));
                all_reports.push(report);
            },
            Err(e) => {
                log(LogTag::Transactions, "ERROR", &format!(
                    "❌ Failed to analyze wallet {}: {}", 
                    &wallet_address[..8], 
                    e
                ));
            }
        }
    }
    
    // Display comprehensive results
    display_comprehensive_results(&all_reports, &args)?;
    
    // Export to JSON if requested
    if args.export {
        export_results_to_json(&all_reports, &args)?;
    }
    
    log(LogTag::Transactions, "SUCCESS", "🎉 Swap discovery analysis completed successfully");
    
    Ok(())
}

/// Display the last 40 transactions for a wallet in table format with transaction types
pub async fn display_wallet_transactions_table(wallet_address: &str) -> Result<(), Box<dyn std::error::Error>> {
    log(LogTag::Transactions, "INFO", &format!("📊 Fetching transaction history for wallet {}", &wallet_address[..8]));
    
    // Get wallet transaction manager
    let tx_manager_arc = get_wallet_transaction_manager()?;
    let manager_lock = tx_manager_arc.read().unwrap();
    
    if let Some(ref manager) = *manager_lock {
        // Get cached transactions (up to 40, most recent first)
        let recent_transactions = manager.get_cached_transactions(Some(40));
        
        if recent_transactions.is_empty() {
            log(LogTag::Transactions, "WARNING", "No transactions found for wallet");
            if is_debug_transactions_enabled() {
                log(LogTag::Transactions, "INFO", "⚠️  No transactions found in cache. Run the bot first to populate transaction cache.");
            }
            return Ok(());
        }
        
        log(LogTag::Transactions, "INFO", &format!("Found {} transactions to display", recent_transactions.len()));
        
        // Convert transactions to display format
        let mut transaction_displays = Vec::new();
        
        for cached_tx in &recent_transactions {
            let tx_data = &cached_tx.transaction_data;
            
            // Extract basic transaction info
            let signature = cached_tx.signature.clone();
            let short_signature = format!("{}...{}", &signature[..8], &signature[signature.len()-8..]);
            
            // Get block time for date/time
            let (date, time) = if let Some(block_time) = tx_data.block_time {
                let dt = DateTime::from_timestamp(block_time, 0)
                    .unwrap_or_else(|| Utc::now());
                let date = dt.format("%Y-%m-%d").to_string();
                let time = dt.format("%H:%M:%S").to_string();
                (date, time)
            } else {
                ("Unknown".to_string(), "Unknown".to_string())
            };
            
            // Get transaction fee
            let fee_sol = if let Some(meta) = &tx_data.transaction.meta {
                let fee_lamports = meta.fee;
                format!("{:.6}", fee_lamports as f64 / 1_000_000_000.0)
            } else {
                "Unknown".to_string()
            };
            
            // Check if transaction was successful
            let success = if let Some(meta) = &tx_data.transaction.meta {
                if meta.err.is_none() {
                    "✅ YES"
                } else {
                    "❌ NO"
                }
            } else {
                "❓ UNKNOWN"
            }.to_string();
            
            // Analyze transaction type
            let transaction_type = analyze_transaction_type(&cached_tx.transaction_data, wallet_address).await;
            
            // Calculate SOL value transferred (excluding fees)
            let value_sol = if let Some(meta) = &tx_data.transaction.meta {
                let pre_balances = &meta.pre_balances;
                let post_balances = &meta.post_balances;
                
                if !pre_balances.is_empty() && !post_balances.is_empty() && pre_balances.len() == post_balances.len() {
                    let sol_change = post_balances[0] as i64 - pre_balances[0] as i64;
                    let sol_value = sol_change.abs() as f64 / 1_000_000_000.0;
                    
                    // Show significant SOL transfers, but show 0 for very small amounts (likely just fees)
                    if sol_value >= 0.00001 {
                        format!("{:.8}", sol_value)
                    } else {
                        "0.00000001".to_string()
                    }
                } else {
                    "0.00000001".to_string()
                }
            } else {
                "0.00000001".to_string()
            };
            
            // Count instructions in the transaction
            let instructions = if let Some(decoded_tx) = tx_data.transaction.transaction.decode() {
                match &decoded_tx.message {
                    solana_sdk::message::VersionedMessage::Legacy(legacy_message) => {
                        let instr_count = legacy_message.instructions.len();
                        format!("{}", instr_count)
                    }
                    solana_sdk::message::VersionedMessage::V0(v0_message) => {
                        let instr_count = v0_message.instructions.len();
                        format!("{}", instr_count)
                    }
                }
            } else {
                // Simple fallback: just default to showing 1 instruction
                // This avoids complex type issues with OptionSerializer
                "1".to_string()
            };

            transaction_displays.push(TransactionDisplay {
                date,
                time,
                signature: short_signature,
                value_sol,
                instructions,
                fee_sol,
                success,
                transaction_type,
            });
        }
        
        // Sort by most recent first (reverse chronological)
        transaction_displays.reverse();
        
        // Display the table - this is intentionally kept as println! for clean console output
        // as this is the primary user-facing output for the table feature
        println!("\n📋 Last {} Transactions for Wallet {}", 
                 transaction_displays.len(), 
                 &wallet_address[..8]);
        println!("═══════════════════════════════════════════════════════════════════════════════════════");
        
        let mut table = Table::new(transaction_displays);
        table
            .with(Style::rounded())
            .with(Modify::new(Rows::new(1..)).with(Alignment::center()));
        
        println!("{}", table);
        println!("");
        
        Ok(())
    } else {
        Err("Wallet transaction manager not initialized".into())
    }
}

/// Analyze transaction to determine its type based on instruction patterns
async fn analyze_transaction_type(tx_data: &solana_transaction_status::EncodedConfirmedTransactionWithStatusMeta, wallet_address: &str) -> String {
    use solana_sdk::message::VersionedMessage;
    
    // Try to decode the transaction to analyze instructions
    if let Some(decoded_tx) = tx_data.transaction.transaction.decode() {
        // Check the message type to get instructions properly
        match &decoded_tx.message {
            VersionedMessage::Legacy(legacy_message) => {
                let instructions = &legacy_message.instructions;
                
                if instructions.len() == 1 {
                    return classify_single_legacy_instruction(&legacy_message, wallet_address).await;
                } else if instructions.len() > 1 {
                    return classify_complex_legacy_transaction(&legacy_message, wallet_address).await;
                }
            }
            VersionedMessage::V0(v0_message) => {
                let instructions = &v0_message.instructions;
                
                if instructions.len() == 1 {
                    return classify_single_v0_instruction(&v0_message, wallet_address).await;
                } else if instructions.len() > 1 {
                    return classify_complex_v0_transaction(&v0_message, wallet_address).await;
                }
            }
        }
    }
    
    // Fallback: analyze based on account changes
    if let Some(meta) = &tx_data.transaction.meta {
        let pre_balances = &meta.pre_balances;
        let post_balances = &meta.post_balances;
        
        // Check SOL balance changes
        if pre_balances.len() == post_balances.len() && pre_balances.len() > 0 {
            let sol_change = post_balances[0] as i64 - pre_balances[0] as i64;
            
            if sol_change > 0 {
                return "💰 SOL_RECEIVED".to_string();
            } else if sol_change < 0 {
                return "💸 SOL_SENT".to_string();
            }
        }
        
        // Check token balance changes
        if meta.pre_token_balances.is_some() || meta.post_token_balances.is_some() {
            return "🔄 TOKEN_ACTIVITY".to_string();
        }
    }
    
    "❓ UNKNOWN".to_string()
}

/// Classify a single instruction transaction (Legacy message)
async fn classify_single_legacy_instruction(message: &solana_sdk::message::Message, _wallet_address: &str) -> String {
    use solana_sdk::pubkey::Pubkey;
    
    if message.instructions.is_empty() {
        return "❓ EMPTY".to_string();
    }
    
    let instruction = &message.instructions[0];
    let program_id = message.account_keys[instruction.program_id_index as usize];
    
    classify_program_id(&program_id)
}

/// Classify a complex multi-instruction transaction (Legacy message)
async fn classify_complex_legacy_transaction(message: &solana_sdk::message::Message, _wallet_address: &str) -> String {
    analyze_multiple_instructions(&message.instructions, &message.account_keys)
}

/// Classify a single instruction transaction (V0 message)
async fn classify_single_v0_instruction(message: &solana_sdk::message::v0::Message, _wallet_address: &str) -> String {
    if message.instructions.is_empty() {
        return "❓ EMPTY".to_string();
    }
    
    let instruction = &message.instructions[0];
    if let Some(program_id) = message.account_keys.get(instruction.program_id_index as usize) {
        classify_program_id(program_id)
    } else {
        "❓ INVALID".to_string()
    }
}

/// Classify a complex multi-instruction transaction (V0 message)  
async fn classify_complex_v0_transaction(message: &solana_sdk::message::v0::Message, _wallet_address: &str) -> String {
    analyze_multiple_instructions(&message.instructions, &message.account_keys)
}

/// Common function to classify program ID
fn classify_program_id(program_id: &solana_sdk::pubkey::Pubkey) -> String {
    let program_str = program_id.to_string();
    
    match program_str.as_str() {
        // System program
        "11111111111111111111111111111111" => "🏛️ SYSTEM",
        // SPL Token program
        "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA" => "🪙 TOKEN_OP",
        // SPL Associated Token Account program
        "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL" => "🔗 ATA_OP",
        // Jupiter programs
        program if program.contains("JUP") => "🪐 JUPITER_SWAP",
        // Raydium programs  
        program if program.contains("Ray") => "🌊 RAYDIUM_SWAP",
        // Other DEX programs
        program if program.contains("DEX") || program.contains("dex") => "🔄 DEX_SWAP",
        _ => "🔧 CONTRACT"
    }
    .to_string()
}

/// Common function to analyze multiple instructions
fn analyze_multiple_instructions(instructions: &[solana_sdk::instruction::CompiledInstruction], account_keys: &[solana_sdk::pubkey::Pubkey]) -> String {
    let mut has_token_activity = false;
    let mut has_swap_activity = false;
    let mut has_ata_creation = false;
    let mut has_system_activity = false;
    
    for instruction in instructions {
        if let Some(program_id) = account_keys.get(instruction.program_id_index as usize) {
            let program_str = program_id.to_string();
            
            match program_str.as_str() {
                "11111111111111111111111111111111" => has_system_activity = true,
                "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA" => has_token_activity = true,
                "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL" => has_ata_creation = true,
                program if program.contains("JUP") || program.contains("Ray") || program.contains("DEX") || program.contains("dex") => {
                    has_swap_activity = true;
                }
                _ => {}
            }
        }
    }
    
    // Prioritize classification based on what activities were detected
    if has_swap_activity {
        if has_ata_creation {
            "🔄 SWAP_+_ATA".to_string()
        } else {
            "🔄 SWAP".to_string()
        }
    } else if has_ata_creation && has_token_activity {
        "🔗 ATA_+_TOKEN".to_string()
    } else if has_ata_creation {
        "🔗 ATA_CREATE".to_string()
    } else if has_token_activity {
        "🪙 TOKEN_TX".to_string()
    } else if has_system_activity {
        "🏛️ SYSTEM_TX".to_string()
    } else {
        "🔧 COMPLEX".to_string()
    }
}
