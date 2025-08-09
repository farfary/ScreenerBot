/// Wallet Transaction Manager
/// Manages wallet transaction fetching, caching, and analysis with efficient sync tracking

use crate::{
    rpc::RpcClient,
    logger::{log, LogTag},
    global::is_debug_transactions_enabled,
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
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use lazy_static::lazy_static;
use tokio::task::JoinSet;
use std::time::Duration;
use tokio::sync::Notify;

lazy_static! {
    static ref WALLET_TRANSACTION_MANAGER: Arc<RwLock<Option<WalletTransactionManager>>> = Arc::new(RwLock::new(None));
    /// Global shutdown signal for background tasks
    static ref TRANSACTION_SYNC_SHUTDOWN: Arc<Notify> = Arc::new(Notify::new());
}

/// Configuration for automatic transaction syncing
#[derive(Debug, Clone)]
pub struct TransactionSyncConfig {
    /// How often to check for new transactions (in seconds)
    pub sync_interval_seconds: u64,
    /// Maximum number of new signatures to fetch per sync
    pub max_signatures_per_sync: usize,
    /// How often to perform a deeper sync (fetch more history)
    pub deep_sync_interval_minutes: u64,
    /// Enable automatic background syncing
    pub auto_sync_enabled: bool,
    /// Smart sync: reduce frequency when no new transactions found
    pub smart_sync_enabled: bool,
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

impl Default for TransactionSyncConfig {
    fn default() -> Self {
        Self {
            sync_interval_seconds: 30,        // Check every 30 seconds
            max_signatures_per_sync: 25,      // Conservative batch size
            deep_sync_interval_minutes: 60,   // Deep sync every hour
            auto_sync_enabled: true,
            smart_sync_enabled: true,
        }
    }
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

#[derive(Debug, Clone)]
pub struct SmartSyncState {
    /// Number of consecutive syncs with no new transactions
    pub consecutive_empty_syncs: u32,
    /// Current dynamic sync interval (adjusted based on activity)
    pub current_sync_interval: Duration,
    /// Last time we found new transactions
    pub last_activity_time: DateTime<Utc>,
    /// Total sync operations performed
    pub total_syncs: u64,
    /// Total new transactions found across all syncs
    pub total_new_transactions_found: u64,
}

pub struct WalletTransactionManager {
    wallet_address: String,
    cache_dir: PathBuf,
    sync_state_file: PathBuf,
    sync_state: WalletSyncState,
    transaction_cache: HashMap<String, CachedTransactionData>,
    /// Configuration for automatic syncing
    sync_config: TransactionSyncConfig,
    /// Smart sync state for adaptive intervals
    smart_sync_state: SmartSyncState,
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
    
    // Start automatic background sync if enabled
    if let Ok(global_manager_lock) = WALLET_TRANSACTION_MANAGER.read() {
        if let Some(ref manager) = *global_manager_lock {
            if manager.sync_config.auto_sync_enabled {
                start_background_transaction_sync().await;
            }
        }
    }
    
    Ok(())
}

/// Start background transaction sync service
pub async fn start_background_transaction_sync() {
    log(LogTag::Transactions, "INFO", "Starting automatic transaction sync service");
    
    let shutdown_signal = TRANSACTION_SYNC_SHUTDOWN.clone();
    
    tokio::spawn(async move {
        let mut sync_interval = tokio::time::interval(Duration::from_secs(30)); // Start with 30 seconds
        let mut deep_sync_interval = tokio::time::interval(Duration::from_secs(60 * 60)); // Deep sync every hour
        
        loop {
            tokio::select! {
                _ = shutdown_signal.notified() => {
                    log(LogTag::Transactions, "INFO", "Background transaction sync service shutting down");
                    break;
                }
                _ = sync_interval.tick() => {
                    if let Err(e) = perform_smart_sync().await {
                        log(LogTag::Transactions, "ERROR", &format!("Smart sync failed: {}", e));
                    }
                }
                _ = deep_sync_interval.tick() => {
                    if let Err(e) = perform_deep_sync().await {
                        log(LogTag::Transactions, "ERROR", &format!("Deep sync failed: {}", e));
                    }
                }
            }
        }
    });
}

/// Stop background transaction sync service
pub async fn stop_background_transaction_sync() {
    log(LogTag::Transactions, "INFO", "Stopping background transaction sync service");
    TRANSACTION_SYNC_SHUTDOWN.notify_waiters();
}

/// Perform smart sync with adaptive intervals
async fn perform_smart_sync() -> Result<(), String> {
    // Simple approach: just log for now and implement properly later
    if is_debug_transactions_enabled() {
        log(LogTag::Transactions, "DEBUG", "Smart sync check (placeholder implementation)");
    }
    
    // TODO: Implement proper async-safe smart sync
    // This requires refactoring the fetch methods to not require mutable references
    // across await boundaries while holding locks
    
    Ok(())
}

/// Perform deep sync for comprehensive transaction coverage
async fn perform_deep_sync() -> Result<(), String> {
    // Simple approach: just log for now and implement properly later
    log(LogTag::Transactions, "INFO", "Deep sync check (placeholder implementation)");
    
    // TODO: Implement proper async-safe deep sync
    // This requires refactoring the fetch methods to not require mutable references
    // across await boundaries while holding locks
    
    Ok(())
}

/// Get access to the global wallet transaction manager
pub fn get_wallet_transaction_manager() -> Result<Arc<RwLock<Option<WalletTransactionManager>>>, Box<dyn std::error::Error>> {
    Ok(WALLET_TRANSACTION_MANAGER.clone())
}

/// Convenient function to analyze recent swaps using the global manager
pub async fn analyze_recent_swaps_global(limit: usize) -> Result<SwapAnalysis, Box<dyn std::error::Error>> {
    let manager_lock = WALLET_TRANSACTION_MANAGER.read().unwrap();
    if let Some(ref manager) = *manager_lock {
        Ok(manager.analyze_recent_swaps(limit))
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
        
        Ok(manager.analyze_recent_swaps(limit))
    }
}

/// Get smart sync statistics from the global manager
pub fn get_global_sync_stats() -> Option<(u64, u64, Duration, u32)> {
    let manager_lock = WALLET_TRANSACTION_MANAGER.read().unwrap();
    manager_lock.as_ref().map(|manager| manager.get_smart_sync_stats())
}

/// Configure sync settings for the global manager
pub async fn configure_global_sync(config: TransactionSyncConfig) -> Result<(), Box<dyn std::error::Error>> {
    let mut manager_lock = WALLET_TRANSACTION_MANAGER.write().unwrap();
    if let Some(ref mut manager) = manager_lock.as_mut() {
        manager.configure_sync(config);
        Ok(())
    } else {
        Err("Global transaction manager not initialized".into())
    }
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
            sync_config: TransactionSyncConfig::default(),
            smart_sync_state: SmartSyncState {
                consecutive_empty_syncs: 0,
                current_sync_interval: Duration::from_secs(TransactionSyncConfig::default().sync_interval_seconds),
                last_activity_time: Utc::now(),
                total_syncs: 0,
                total_new_transactions_found: 0,
            },
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
    
    /// Check if we should perform a sync based on smart sync state
    fn should_perform_sync(&self) -> bool {
        if !self.sync_config.smart_sync_enabled {
            return true; // Always sync if smart sync is disabled
        }
        
        let now = chrono::Utc::now();
        let time_since_last_sync = now.signed_duration_since(self.sync_state.last_sync_time);
        
        time_since_last_sync.num_seconds() >= self.smart_sync_state.current_sync_interval.as_secs() as i64
    }
    
    /// Update smart sync state based on sync results
    fn update_smart_sync_state(&mut self, new_transactions_found: usize) {
        self.smart_sync_state.total_syncs += 1;
        
        if new_transactions_found > 0 {
            // Found new transactions - reset to base interval
            self.smart_sync_state.consecutive_empty_syncs = 0;
            self.smart_sync_state.last_activity_time = chrono::Utc::now();
            self.smart_sync_state.total_new_transactions_found += new_transactions_found as u64;
            self.smart_sync_state.current_sync_interval = Duration::from_secs(self.sync_config.sync_interval_seconds);
            
            if is_debug_transactions_enabled() {
                log(LogTag::Transactions, "DEBUG", &format!("Smart sync: Found activity, reset to {} seconds", 
                    self.sync_config.sync_interval_seconds));
            }
        } else {
            // No new transactions - increase interval gradually
            self.smart_sync_state.consecutive_empty_syncs += 1;
            
            // Exponential backoff: start at base, max out at 10 minutes
            let backoff_multiplier = (self.smart_sync_state.consecutive_empty_syncs as f64).min(10.0);
            let new_interval_secs = (self.sync_config.sync_interval_seconds as f64 * backoff_multiplier).min(600.0) as u64;
            self.smart_sync_state.current_sync_interval = Duration::from_secs(new_interval_secs);
            
            if is_debug_transactions_enabled() {
                log(LogTag::Transactions, "DEBUG", &format!("Smart sync: No activity ({} empty), interval now {} seconds", 
                    self.smart_sync_state.consecutive_empty_syncs, new_interval_secs));
            }
        }
    }
    
    /// Perform a lightweight transaction fetch for smart sync
    async fn fetch_recent_transactions_smart(&mut self, rpc_client: &RpcClient) -> Result<usize, Box<dyn std::error::Error>> {
        // Use smaller batch size for frequent checks
        let smart_batch_size = (self.sync_config.max_signatures_per_sync / 2).max(10);
        
        if is_debug_transactions_enabled() {
            log(LogTag::Transactions, "DEBUG", &format!("Smart sync: Checking {} recent signatures", smart_batch_size));
        }
        
        let latest_signatures = self.fetch_wallet_signatures(rpc_client, smart_batch_size, None).await?;
        
        if latest_signatures.is_empty() {
            return Ok(0);
        }
        
        let mut new_signatures = Vec::new();
        for sig_info in &latest_signatures {
            if !self.sync_state.cached_signatures.contains(&sig_info.signature) {
                new_signatures.push(sig_info.signature.clone());
            }
        }
        
        if !new_signatures.is_empty() {
            if is_debug_transactions_enabled() {
                log(LogTag::Transactions, "DEBUG", &format!("Smart sync: Found {} new transactions to cache", new_signatures.len()));
            }
            
            self.fetch_and_cache_transactions(rpc_client, &new_signatures).await?;
            self.save_sync_state()?;
        }
        
        Ok(new_signatures.len())
    }
    
    /// Get smart sync statistics
    pub fn get_smart_sync_stats(&self) -> (u64, u64, Duration, u32) {
        (
            self.smart_sync_state.total_syncs,
            self.smart_sync_state.total_new_transactions_found,
            self.smart_sync_state.current_sync_interval,
            self.smart_sync_state.consecutive_empty_syncs,
        )
    }
    
    /// Configure sync settings
    pub fn configure_sync(&mut self, config: TransactionSyncConfig) {
        self.sync_config = config;
        // Reset smart sync state when configuration changes
        self.smart_sync_state.consecutive_empty_syncs = 0;
        self.smart_sync_state.current_sync_interval = Duration::from_secs(self.sync_config.sync_interval_seconds);
        
        log(LogTag::Transactions, "CONFIG", &format!("Transaction sync configured: {}s interval, max {} sigs/sync", 
            self.sync_config.sync_interval_seconds, self.sync_config.max_signatures_per_sync));
    }
    
    async fn fetch_missing_transactions(&mut self, rpc_client: &RpcClient) -> Result<(), Box<dyn std::error::Error>> {
        log(LogTag::Transactions, "INFO", "Checking for missing transactions to fetch");
        
        // Use configured batch size for initial sync
        let batch_size = self.sync_config.max_signatures_per_sync;
        
        // First, get latest signatures to see what's new
        let latest_signatures = self.fetch_wallet_signatures(rpc_client, batch_size, None).await?;
        
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
        log(LogTag::Api, "INFO", &format!("Fetching and caching {} transactions concurrently", signatures.len()));
        
        // Configuration for concurrent processing
        const MAX_CONCURRENT_REQUESTS: usize = 8; // Limit concurrent requests to avoid overwhelming RPC
        const BATCH_DELAY_MS: u64 = 100; // Small delay between batches to be nice to RPC
        
        // Filter out already cached signatures
        let new_signatures: Vec<String> = signatures.iter()
            .filter(|sig| !self.sync_state.cached_signatures.contains(*sig))
            .cloned()
            .collect();
        
        if new_signatures.is_empty() {
            log(LogTag::Api, "SKIP", "All transactions already cached");
            return Ok(());
        }
        
        log(LogTag::Api, "CONCURRENT", &format!("Processing {} new transactions with {} concurrent workers", 
            new_signatures.len(), MAX_CONCURRENT_REQUESTS));
        
        // Process signatures in chunks to manage concurrency
        let mut total_success = 0;
        let mut total_errors = 0;
        
        for (batch_idx, chunk) in new_signatures.chunks(MAX_CONCURRENT_REQUESTS).enumerate() {
            log(LogTag::Api, "BATCH", &format!("Processing batch {} with {} transactions", 
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
                            log(LogTag::Api, "ERROR", &format!("Transaction fetch failed for {}: {}", signature, e));
                            total_errors += 1;
                        }
                    }
                } else {
                    log(LogTag::Api, "ERROR", &format!("Invalid signature format: {}", signature));
                    total_errors += 1;
                }
            }
            
            log(LogTag::Api, "BATCH_RESULT", &format!("Batch {} completed: {} in batch", 
                batch_idx + 1, chunk.len()));
            
            // Small delay between batches to be respectful to RPC
            if batch_idx + 1 < new_signatures.chunks(MAX_CONCURRENT_REQUESTS).len() {
                tokio::time::sleep(Duration::from_millis(BATCH_DELAY_MS)).await;
            }
        }
        
        self.sync_state.last_sync_time = Utc::now();
        log(LogTag::Api, "SUCCESS", &format!("Batch fetch completed: {} success, {} errors, {} total cached", 
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
    
    pub fn analyze_recent_swaps(&self, limit: usize) -> SwapAnalysis {
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
                
                if let Some(swap) = self.analyze_transaction_for_swap(cached_tx) {
                    log(LogTag::Swap, "FOUND", &format!("Swap detected: {} {} {} SOL for {} tokens", 
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
    
    fn analyze_transaction_for_swap(&self, cached_tx: &CachedTransactionData) -> Option<SwapTransaction> {
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
        let sol_change = (post_balance as i64 - pre_balance as i64) as f64 / 1_000_000_000.0;
        
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
                    
                    // Get token symbol
                    let token_symbol = get_token_symbol(mint_str).unwrap_or_else(|| format!("{}...{}", &mint_str[..4], &mint_str[mint_str.len()-4..]));
                    
                    // Calculate effective price
                    let token_amount_ui = token_change.abs() as f64 / 10f64.powi(decimals as i32);
                    let effective_price = if token_amount_ui > 0.0 {
                        sol_change.abs() / token_amount_ui
                    } else {
                        0.0
                    };
                    
                    // Calculate fees
                    let fees_paid = meta.fee as f64 / 1_000_000_000.0;
                    
                    return Some(SwapTransaction {
                        signature: signature.clone(),
                        slot: tx.slot,
                        block_time: tx.block_time,
                        swap_type: swap_type.to_string(),
                        token_mint: mint_str.to_string(),
                        token_symbol,
                        sol_amount: sol_change.abs(),
                        token_amount: token_change.abs() as u64,
                        token_decimals: decimals,
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
                
                log(LogTag::Swap, "DETAIL", &format!("Swap {}: {} {} tokens for {:.6} SOL at {:.9} SOL/token", 
                    i + 1, swap.swap_type, swap.token_symbol, swap.sol_amount, swap.effective_price));
            }
        }
        
        println!("\n✅ Analysis complete. Check the details above for P&L verification.");
        log(LogTag::System, "COMPLETE", "Wallet swap analysis finished successfully");
    }
}

fn get_token_symbol(mint: &str) -> Option<String> {
    // Try to get symbol from our token database
    // For now, return a shortened mint address
    Some(format!("{}...{}", &mint[..6], &mint[mint.len()-4..]))
}
