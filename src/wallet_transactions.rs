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

#[derive(Serialize, Deserialize)]
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
            // Update manager with new data (brief write lock)
            let mut manager_lock = WALLET_TRANSACTION_MANAGER.write().unwrap();
            if let Some(ref mut manager) = *manager_lock {
                // Update cached signatures
                for sig in &new_signatures {
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
    
    let config = GetConfirmedSignaturesForAddress2Config {
        before: None,
        until: None,
        limit: Some(limit),
        commitment: Some(CommitmentConfig::confirmed()),
    };
    
    let signatures = rpc_client.client()
        .get_signatures_for_address_with_config(&wallet_pubkey, config)?;
    Ok(signatures)
}

/// Helper function to fetch transaction details without holding manager lock
async fn fetch_transaction_details_standalone(
    signatures: &[String],
    rpc_client: &RpcClient
) -> Result<Vec<(String, EncodedConfirmedTransactionWithStatusMeta)>, Box<dyn std::error::Error>> {
    let mut transactions = Vec::new();
    
    for signature_str in signatures {
        if let Ok(signature) = Signature::from_str(signature_str) {
            match rpc_client.client().get_transaction(&signature, UiTransactionEncoding::Json) {
                Ok(tx) => {
                    transactions.push((signature_str.clone(), tx));
                }
                Err(e) => {
                    if is_debug_transactions_enabled() {
                        log(LogTag::Transactions, "ERROR", &format!("Failed to fetch transaction {}: {}", signature_str, e));
                    }
                }
            }
        }
    }
    
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
            log(LogTag::System, "INFO", &format!("Created transaction cache directory: {:?}", cache_dir));
        }
        
        // Create data directory if it doesn't exist
        let data_dir = PathBuf::from("data");
        if !data_dir.exists() {
            fs::create_dir_all(&data_dir)?;
            log(LogTag::System, "INFO", &format!("Created data directory: {:?}", data_dir));
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
                            log(LogTag::System, "ERROR", &format!("Failed to parse sync state: {}", e));
                            let mut state = WalletSyncState::default();
                            state.wallet_address = wallet_address.clone();
                            state
                        }
                    }
                },
                Err(e) => {
                    log(LogTag::System, "ERROR", &format!("Failed to read sync state: {}", e));
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
                                log(LogTag::System, "ERROR", &format!("Failed to parse cached transaction {}: {}", signature, e));
                                // Remove invalid signature from state
                                self.sync_state.cached_signatures.remove(signature);
                            }
                        }
                    },
                    Err(e) => {
                        log(LogTag::System, "ERROR", &format!("Failed to read cached transaction {}: {}", signature, e));
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
            log(LogTag::System, "INFO", "No transactions found for wallet");
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
            log(LogTag::System, "INFO", "All recent transactions are already cached");
        }
        
        // If this is the first sync or we need more history, fetch older transactions
        if self.sync_state.total_transactions_fetched < 1000 {
            log(LogTag::System, "INFO", "Fetching transaction history for complete coverage");
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
                log(LogTag::System, "INFO", "No more historical transactions to fetch");
                break;
            }
            
            let mut new_signatures = Vec::new();
            for sig_info in &signatures {
                if !self.sync_state.cached_signatures.contains(&sig_info.signature) {
                    new_signatures.push(sig_info.signature.clone());
                }
            }
            
            if !new_signatures.is_empty() {
                log(LogTag::System, "INFO", &format!("Fetching {} historical transactions", new_signatures.len()));
                self.fetch_and_cache_transactions(rpc_client, &new_signatures).await?;
            }
            
            // Update before_signature for next batch
            before_signature = signatures.last().map(|s| s.signature.clone());
            
            // Update oldest signature if this is our first time or we went further back
            if self.sync_state.oldest_signature.is_none() || before_signature.is_some() {
                self.sync_state.oldest_signature = before_signature.clone();
            }
            
            // Break if we didn't get a full batch (reached the end)
            if signatures.len() < batch_size {
                log(LogTag::System, "INFO", "Reached end of transaction history");
                break;
            }
        }
        
        Ok(())
    }
    
    async fn fetch_and_cache_transactions(&mut self, rpc_client: &RpcClient, signatures: &[String]) -> Result<(), Box<dyn std::error::Error>> {
        log(LogTag::Transactions, "INFO", &format!("Fetching and caching {} transactions concurrently", signatures.len()));
        
        // Configuration for concurrent processing
        const MAX_CONCURRENT_REQUESTS: usize = 8; // Limit concurrent requests to avoid overwhelming RPC
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
                            
                            // Update newest signature
                            if self.sync_state.newest_signature.is_none() {
                                self.sync_state.newest_signature = Some(signature.clone());
                            }
                            
                            // Save sync state after each successful fetch to preserve progress
                            if let Err(e) = self.save_sync_state() {
                                log(LogTag::System, "ERROR", &format!("Failed to save sync state after caching {}: {}", signature, e));
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
    ) -> Result<(), Box<dyn std::error::Error>> {
        let sig_str = signature.to_string();
        
        // Check if already cached
        if self.transaction_cache.contains_key(&sig_str) {
            return Ok(());
        }
        
        log(LogTag::Rpc, "FETCH", &format!("Fetching transaction {} from RPC", sig_str));
        
        // Fetch from RPC
        let tx_result = rpc_client.client().get_transaction_with_config(
            signature,
            solana_client::rpc_config::RpcTransactionConfig {
                encoding: Some(UiTransactionEncoding::JsonParsed),
                commitment: Some(CommitmentConfig::confirmed()),
                max_supported_transaction_version: Some(0),
            },
        )?;
        
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
        
        log(LogTag::Transactions, "CACHED", &format!("Transaction {} cached successfully", sig_str));
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
                        log(LogTag::System, "CACHE_HIT", &format!("Transaction {} loaded from disk cache", signature_str));
                        return Ok(cached_data);
                    }
                },
                Err(_) => {
                    // Cache file exists but is corrupted, we'll refetch
                    log(LogTag::System, "CACHE_CORRUPT", &format!("Corrupted cache file for {}, refetching", signature_str));
                }
            }
        }
        
        log(LogTag::Rpc, "FETCH", &format!("Fetching transaction {} from RPC (concurrent)", signature_str));
        
        // Fetch from RPC
        let tx_result = rpc_client.client().get_transaction_with_config(
            signature,
            solana_client::rpc_config::RpcTransactionConfig {
                encoding: Some(UiTransactionEncoding::JsonParsed),
                commitment: Some(CommitmentConfig::confirmed()),
                max_supported_transaction_version: Some(0),
            },
        ).map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
        
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
        
        let mut config = GetConfirmedSignaturesForAddress2Config {
            limit: Some(limit),
            commitment: Some(CommitmentConfig::confirmed()),
            ..Default::default()
        };
        
        if let Some(before_sig) = before {
            if let Ok(sig) = Signature::from_str(before_sig) {
                config.before = Some(sig);
            }
        }
        
        let signatures = rpc_client.client()
            .get_signatures_for_address_with_config(&pubkey, config)?;
        
        Ok(signatures)
    }
    
    pub fn get_cached_transaction(&self, signature: &str) -> Option<&CachedTransactionData> {
        self.transaction_cache.get(signature)
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
        println!("\n=== WALLET SWAP ANALYSIS RESULTS ===\n");
        
        log(LogTag::Summary, "RESULTS", &format!("Analysis Summary: {} total swaps ({} buy, {} sell)", 
            analysis.total_swaps, analysis.buy_swaps, analysis.sell_swaps));
        
        println!("📊 Summary:");
        println!("   Total swaps found: {}", analysis.total_swaps);
        println!("   Buy transactions: {}", analysis.buy_swaps);
        println!("   Sell transactions: {}", analysis.sell_swaps);
        println!("   Total SOL spent (buys): {:.6}", analysis.total_sol_in);
        println!("   Total SOL received (sells): {:.6}", analysis.total_sol_out);
        println!("   Total fees paid: {:.6}", analysis.total_fees);
        println!("   Net SOL change: {:.6}", analysis.net_sol_change);
        
        log(LogTag::Profit, "CALCULATION", &format!("Net P&L: {:.6} SOL (buys: {:.6}, sells: {:.6}, fees: {:.6})", 
            analysis.net_sol_change, analysis.total_sol_in, analysis.total_sol_out, analysis.total_fees));
        
        if analysis.net_sol_change > 0.0 {
            println!("   🟢 Net profit: {:.6} SOL", analysis.net_sol_change);
            log(LogTag::Profit, "PROFIT", &format!("Net profit: {:.6} SOL", analysis.net_sol_change));
        } else if analysis.net_sol_change < 0.0 {
            println!("   🔴 Net loss: {:.6} SOL", analysis.net_sol_change.abs());
            log(LogTag::Profit, "LOSS", &format!("Net loss: {:.6} SOL", analysis.net_sol_change.abs()));
        } else {
            println!("   ⚪ Break even");
            log(LogTag::Profit, "EVEN", "Portfolio is break even");
        }
        
        if !analysis.recent_swaps.is_empty() {
            println!("\n📋 Recent Swap Details:");
            println!("   {:<8} {:<16} {:<12} {:<15} {:<12} {:<10}", 
                     "Type", "Token", "SOL Amount", "Price", "Fees", "Status");
            println!("   {}", "=".repeat(80));
            
            log(LogTag::Summary, "DETAILS", &format!("Displaying {} recent swaps", analysis.recent_swaps.len()));
            
            for (i, swap) in analysis.recent_swaps.iter().enumerate() {
                let status = if swap.success { "✅" } else { "❌" };
                println!("   {:<8} {:<16} {:<12.6} {:<15.9} {:<12.6} {:<10}",
                         swap.swap_type,
                         swap.token_symbol,
                         swap.sol_amount,
                         swap.effective_price,
                         swap.fees_paid,
                         status);
                
                log(LogTag::Transactions, "DETAIL", &format!("Swap {}: {} {} tokens for {:.6} SOL at {:.9} SOL/token", 
                    i + 1, swap.swap_type, swap.token_symbol, swap.sol_amount, swap.effective_price));
            }
        }
        
        println!("\n✅ Analysis complete. Check the details above for P&L verification.");
        log(LogTag::System, "COMPLETE", "Wallet swap analysis finished successfully");
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
