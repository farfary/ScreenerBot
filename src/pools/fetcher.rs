use super::types::PoolDescriptor;
use super::utils::is_sol_mint;
use crate::events::{record_safe, Event, EventCategory, Severity};
/// Account fetcher module
///
/// This module handles efficient batched fetching of pool account data from RPC.
/// It optimizes RPC usage by batching requests and managing rate limits.
use crate::logger::{self, LogTag};
use crate::pools::service; // access global calculator
use crate::rpc::{get_rpc_client, RpcClient};
use solana_sdk::{account::Account, pubkey::Pubkey};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};
use std::time::Instant;
use tokio::sync::{mpsc, Notify};

/// Constants for batch processing
const ACCOUNT_BATCH_SIZE: usize = 50; // Optimal batch size for RPC calls
const FETCH_INTERVAL_MS: u64 = 500; // Fetch every 1 second
const ACCOUNT_STALE_THRESHOLD_SECONDS: u64 = 30; // Default stale threshold for inactive tokens
                                                 // Faster refresh threshold for pools backing currently open positions (tighter P&L responsiveness)
const OPEN_POSITION_ACCOUNT_STALE_THRESHOLD_SECONDS: u64 = 5;

/// Message types for fetcher communication
#[derive(Debug, Clone)]
pub enum FetcherMessage {
    /// Request to fetch accounts for a pool
    FetchPool {
        pool_id: Pubkey,
        accounts: Vec<Pubkey>,
    },
    /// Request to fetch specific accounts
    FetchAccounts { accounts: Vec<Pubkey> },
    /// Signal shutdown
    Shutdown,
}

/// Account data with metadata
#[derive(Debug, Clone)]
pub struct AccountData {
    pub pubkey: Pubkey,
    pub data: Vec<u8>,
    pub slot: u64,
    pub fetched_at: Instant,
    pub lamports: u64,
    pub owner: Pubkey,
}

impl AccountData {
    /// Create from Solana Account
    pub fn from_account(pubkey: Pubkey, account: Account, slot: u64) -> Self {
        Self {
            pubkey,
            data: account.data,
            slot,
            fetched_at: Instant::now(),
            lamports: account.lamports,
            owner: account.owner,
        }
    }

    /// Check if account data is stale
    pub fn is_stale(&self, max_age_seconds: u64) -> bool {
        self.fetched_at.elapsed().as_secs() > max_age_seconds
    }
}

/// Pool account bundle - all accounts for a specific pool
#[derive(Debug, Clone)]
pub struct PoolAccountBundle {
    pub pool_id: Pubkey,
    pub accounts: HashMap<Pubkey, AccountData>,
    pub last_updated: Instant,
    pub slot: u64,
    pub calculation_requested: bool,
}

impl PoolAccountBundle {
    /// Create new bundle
    pub fn new(pool_id: Pubkey) -> Self {
        Self {
            pool_id,
            accounts: HashMap::new(),
            last_updated: Instant::now(),
            slot: 0,
            calculation_requested: false,
        }
    }

    /// Add account to bundle
    pub fn add_account(&mut self, account_data: AccountData) {
        self.slot = self.slot.max(account_data.slot);
        self.last_updated = Instant::now();
        self.accounts.insert(account_data.pubkey, account_data);

        // Reset calculation requested flag when fresh account data is added
        // This allows for new calculations when accounts are refreshed
        self.calculation_requested = false;
    }

    /// Check if bundle is complete (has all required accounts)
    pub fn is_complete(&self, required_accounts: &[Pubkey]) -> bool {
        required_accounts
            .iter()
            .all(|key| self.accounts.contains_key(key))
    }

    /// Check if bundle is complete and calculation not yet requested
    pub fn is_complete_and_needs_calculation(&self, required_accounts: &[Pubkey]) -> bool {
        self.is_complete(required_accounts) && !self.calculation_requested
    }

    /// Mark that calculation has been requested for this bundle
    pub fn mark_calculation_requested(&mut self) {
        self.calculation_requested = true;
    }

    /// Check if bundle is stale
    pub fn is_stale(&self, max_age_seconds: u64) -> bool {
        self.last_updated.elapsed().as_secs() > max_age_seconds
    }
}

/// Account fetcher service
pub struct AccountFetcher {
    /// RPC client for fetching data
    rpc_client: Arc<RpcClient>,
    /// Pool directory for getting account requirements
    pool_directory: Arc<RwLock<HashMap<Pubkey, PoolDescriptor>>>,
    /// Fetched account bundles by pool ID
    account_bundles: Arc<RwLock<HashMap<Pubkey, PoolAccountBundle>>>,
    /// Last fetch time for each account
    account_last_fetch: Arc<RwLock<HashMap<Pubkey, Instant>>>,
    /// Channel for receiving fetch requests
    fetcher_rx: Arc<RwLock<Option<mpsc::UnboundedReceiver<FetcherMessage>>>>,
    /// Channel sender for sending fetch requests
    fetcher_tx: mpsc::UnboundedSender<FetcherMessage>,
}

impl AccountFetcher {
    /// Create new account fetcher
    pub fn new(
        rpc_client: Arc<RpcClient>,
        pool_directory: Arc<RwLock<HashMap<Pubkey, PoolDescriptor>>>,
    ) -> Self {
        let (fetcher_tx, fetcher_rx) = mpsc::unbounded_channel();

        Self {
            rpc_client,
            pool_directory,
            account_bundles: Arc::new(RwLock::new(HashMap::new())),
            account_last_fetch: Arc::new(RwLock::new(HashMap::new())),
            fetcher_rx: Arc::new(RwLock::new(Some(fetcher_rx))),
            fetcher_tx,
        }
    }

    /// Get sender for sending fetch requests
    pub fn get_sender(&self) -> mpsc::UnboundedSender<FetcherMessage> {
        self.fetcher_tx.clone()
    }

    /// Get account bundles (read-only access)
    pub fn get_account_bundles(&self) -> Arc<RwLock<HashMap<Pubkey, PoolAccountBundle>>> {
        self.account_bundles.clone()
    }

    /// Start fetcher background task
    pub async fn start_fetcher_task(&self, shutdown: Arc<Notify>) {
        logger::info(LogTag::PoolFetcher, "Starting account fetcher task");

        let rpc_client = self.rpc_client.clone();
        let pool_directory = self.pool_directory.clone();
        let account_bundles = self.account_bundles.clone();
        let account_last_fetch = self.account_last_fetch.clone();

        // Take the receiver from the Arc<RwLock>
        let mut fetcher_rx = {
            let mut rx_lock = self.fetcher_rx.write().unwrap();
            rx_lock.take().expect("Fetcher receiver already taken")
        };

        tokio::spawn(async move {
            let mut interval =
                tokio::time::interval(tokio::time::Duration::from_millis(FETCH_INTERVAL_MS));
            let mut pending_accounts: HashSet<Pubkey> = HashSet::new();

            logger::info(LogTag::PoolFetcher, "Account fetcher task started");

            loop {
                tokio::select! {
                    _ = shutdown.notified() => {
                        logger::info(LogTag::PoolFetcher, "Account fetcher task shutting down");
                        break;
                    }

                    message = fetcher_rx.recv() => {
                        match message {
                            Some(FetcherMessage::FetchPool { pool_id, accounts }) => {
                                logger::debug(
                                    LogTag::PoolFetcher,
                                    &format!("Received fetch request for pool {} with {} accounts", pool_id, accounts.len())
                                );
                                pending_accounts.extend(accounts);
                            }

                            Some(FetcherMessage::FetchAccounts { accounts }) => {
                                logger::debug(
                                    LogTag::PoolFetcher,
                                    &format!("Received fetch request for {} accounts", accounts.len())
                                );
                                pending_accounts.extend(accounts);
                            }

                            Some(FetcherMessage::Shutdown) => {
                                logger::info(LogTag::PoolFetcher, "Fetcher received shutdown signal");
                                break;
                            }

                            None => {
                                logger::info(LogTag::PoolFetcher, "Fetcher channel closed");
                                break;
                            }
                        }
                    }

                    _ = interval.tick() => {
                        // Add accounts that need refresh from pool directory
                        Self::add_stale_accounts_to_pending(
                            &pool_directory,
                            &account_last_fetch,
                            &mut pending_accounts
                        ).await;

                        // Process pending accounts if any
                        if !pending_accounts.is_empty() {
                            Self::process_pending_accounts(
                                &rpc_client,
                                &pool_directory,
                                &account_bundles,
                                &account_last_fetch,
                                &mut pending_accounts
                            ).await;
                        }
                    }
                }
            }

            logger::info(LogTag::PoolFetcher, "Account fetcher task completed");
        });
    }

    /// Add stale accounts from pools to pending fetch list
    async fn add_stale_accounts_to_pending(
        pool_directory: &Arc<RwLock<HashMap<Pubkey, PoolDescriptor>>>,
        account_last_fetch: &Arc<RwLock<HashMap<Pubkey, Instant>>>,
        pending_accounts: &mut HashSet<Pubkey>,
    ) {
        // Snapshot pools & last fetch times under locks (minimize lock duration)
        let (pools, last_fetch_map) = {
            let directory = pool_directory.read().unwrap();
            let pools_vec = directory.values().cloned().collect::<Vec<_>>();
            let last = account_last_fetch.read().unwrap();
            (pools_vec, last.clone())
        };

        // Collect open position mints once (async call) to avoid per-pool await cost
        let open_mints: std::collections::HashSet<String> =
            crate::positions::state::get_open_mints()
                .await
                .into_iter()
                .collect();

        for pool in pools {
            // Determine the tracked (non-SOL) token mint for this pool
            let target_mint = if super::utils::is_sol_mint(&pool.base_mint.to_string()) {
                pool.quote_mint.to_string()
            } else {
                pool.base_mint.to_string()
            };

            // Choose threshold – accelerate if this token has an open position
            let threshold = if open_mints.contains(&target_mint) {
                OPEN_POSITION_ACCOUNT_STALE_THRESHOLD_SECONDS
            } else {
                ACCOUNT_STALE_THRESHOLD_SECONDS
            };

            for account in &pool.reserve_accounts {
                let needs_fetch = match last_fetch_map.get(account) {
                    Some(last_time) => last_time.elapsed().as_secs() > threshold,
                    None => true, // Never fetched
                };
                if needs_fetch {
                    pending_accounts.insert(*account);
                }
            }
        }
    }

    /// Process pending accounts by fetching them in batches
    async fn process_pending_accounts(
        rpc_client: &Arc<RpcClient>,
        pool_directory: &Arc<RwLock<HashMap<Pubkey, PoolDescriptor>>>,
        account_bundles: &Arc<RwLock<HashMap<Pubkey, PoolAccountBundle>>>,
        account_last_fetch: &Arc<RwLock<HashMap<Pubkey, Instant>>>,
        pending_accounts: &mut HashSet<Pubkey>,
    ) {
        if pending_accounts.is_empty() {
            return;
        }

        // Convert to vector and batch
        let accounts_to_fetch: Vec<Pubkey> = pending_accounts.drain().collect();

        logger::info(
            LogTag::PoolFetcher,
            &format!("Processing {} pending accounts", accounts_to_fetch.len()),
        );

        // Process in batches
        for batch in accounts_to_fetch.chunks(ACCOUNT_BATCH_SIZE) {
            let batch_start = Instant::now();

            record_safe(Event::info(
                EventCategory::Pool,
                Some("rpc_batch_started".to_string()),
                None,
                None,
                serde_json::json!({
                    "batch_size": batch.len(),
                    "max_batch_size": ACCOUNT_BATCH_SIZE,
                    "accounts": batch.iter().map(|p| p.to_string()).collect::<Vec<_>>()
                }),
            ))
            .await;

            match Self::fetch_account_batch(rpc_client, batch).await {
                Ok(account_data_list) => {
                    let batch_duration = batch_start.elapsed();

                    record_safe(Event::info(
                        EventCategory::Pool,
                        Some("rpc_batch_completed".to_string()),
                        None,
                        None,
                        serde_json::json!({
                            "batch_size": batch.len(),
                            "accounts_fetched": account_data_list.len(),
                            "duration_ms": batch_duration.as_millis(),
                            "success": true
                        }),
                    ))
                    .await;

                    // Update last fetch times
                    {
                        let mut last_fetch = account_last_fetch.write().unwrap();
                        for account in batch {
                            last_fetch.insert(*account, Instant::now());
                        }
                    }

                    // Organize accounts into pool bundles
                    Self::organize_accounts_into_bundles(
                        &account_data_list,
                        pool_directory,
                        account_bundles,
                    )
                    .await;

                    logger::info(
                        LogTag::PoolFetcher,
                        &format!("Successfully fetched {} accounts", account_data_list.len()),
                    );
                }
                Err(e) => {
                    let batch_duration = batch_start.elapsed();

                    logger::error(
                        LogTag::PoolFetcher,
                        &format!("Failed to fetch account batch: {}", e),
                    );

                    record_safe(Event::error(
                        EventCategory::Pool,
                        Some("rpc_batch_failed".to_string()),
                        None,
                        None,
                        serde_json::json!({
                            "batch_size": batch.len(),
                            "error": e,
                            "duration_ms": batch_duration.as_millis(),
                            "accounts": batch.iter().map(|p| p.to_string()).collect::<Vec<_>>()
                        }),
                    ))
                    .await;
                }
            }

            // Small delay between batches to respect rate limits
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
    }

    /// Fetch a batch of accounts
    async fn fetch_account_batch(
        rpc_client: &Arc<RpcClient>,
        accounts: &[Pubkey],
    ) -> Result<Vec<AccountData>, String> {
        if accounts.is_empty() {
            return Ok(Vec::new());
        }

        logger::debug(
            LogTag::PoolFetcher,
            &format!("Fetching batch of {} accounts", accounts.len()),
        );

        // Fetch accounts using RPC client
        let rpc_start = Instant::now();
        let account_results = match rpc_client.get_multiple_accounts(accounts).await {
            Ok(results) => {
                let rpc_duration = rpc_start.elapsed();

                record_safe(Event::info(
                    EventCategory::Rpc,
                    Some("get_multiple_accounts_success".to_string()),
                    None,
                    None,
                    serde_json::json!({
                        "account_count": accounts.len(),
                        "duration_ms": rpc_duration.as_millis(),
                        "success": true
                    }),
                ))
                .await;

                results
            }
            Err(e) => {
                let rpc_duration = rpc_start.elapsed();

                record_safe(Event::error(
                    EventCategory::Rpc,
                    Some("get_multiple_accounts_failed".to_string()),
                    None,
                    None,
                    serde_json::json!({
                        "account_count": accounts.len(),
                        "error": e.to_string(),
                        "duration_ms": rpc_duration.as_millis(),
                        "accounts": accounts.iter().map(|p| p.to_string()).collect::<Vec<_>>()
                    }),
                ))
                .await;

                return Err(e);
            }
        };

        let mut account_data_list = Vec::new();
        let mut missing_accounts = Vec::new();

        for (i, account_opt) in account_results.iter().enumerate() {
            if let Some(account) = account_opt {
                let account_data = AccountData::from_account(accounts[i], account.clone(), 0);
                account_data_list.push(account_data);
            } else {
                missing_accounts.push(accounts[i].to_string());
                logger::warning(LogTag::PoolFetcher, &format!("Account not found: {}", accounts[i]));
            }
        }

        if !missing_accounts.is_empty() {
            record_safe(Event::warn(
                EventCategory::Pool,
                Some("accounts_not_found".to_string()),
                None,
                None,
                serde_json::json!({
                    "missing_count": missing_accounts.len(),
                    "total_requested": accounts.len(),
                    "missing_accounts": missing_accounts
                }),
            ))
            .await;
        }

        Ok(account_data_list)
    }

    /// Organize fetched accounts into pool bundles
    ///
    /// Creates isolated account data instances for each pool to prevent race conditions
    /// when multiple pools share the same vault accounts (common in Raydium Legacy AMM)
    async fn organize_accounts_into_bundles(
        account_data_list: &[AccountData],
        pool_directory: &Arc<RwLock<HashMap<Pubkey, PoolDescriptor>>>,
        account_bundles: &Arc<RwLock<HashMap<Pubkey, PoolAccountBundle>>>,
    ) {
        let pools = {
            let directory = pool_directory.read().unwrap();
            directory.clone()
        };

        let mut bundles = account_bundles.write().unwrap();

        // For each account, find which pools it belongs to
        for account_data in account_data_list {
            for (pool_id, pool_descriptor) in &pools {
                if pool_descriptor
                    .reserve_accounts
                    .contains(&account_data.pubkey)
                {
                    let bundle = bundles
                        .entry(*pool_id)
                        .or_insert_with(|| PoolAccountBundle::new(*pool_id));

                    // Create isolated account data for each pool to prevent race conditions
                    // when multiple pools share the same vault accounts
                    let isolated_account_data = AccountData {
                        pubkey: account_data.pubkey,
                        data: account_data.data.clone(),
                        slot: account_data.slot,
                        fetched_at: Instant::now(), // Fresh timestamp for this pool context
                        lamports: account_data.lamports,
                        owner: account_data.owner,
                    };
                    bundle.add_account(isolated_account_data);

                    {
                        let target_token = if is_sol_mint(&pool_descriptor.base_mint.to_string()) {
                            pool_descriptor.quote_mint
                        } else {
                            pool_descriptor.base_mint
                        };
                        logger::debug(
                            LogTag::PoolFetcher,
                            &format!(
                                "Added account {} to bundle for token {} in pool {}",
                                account_data.pubkey, target_token, pool_id
                            ),
                        );
                    }

                    // If bundle now complete and calculation not yet requested, trigger price calculation
                    if bundle.is_complete_and_needs_calculation(&pool_descriptor.reserve_accounts) {
                        bundle.mark_calculation_requested();

                        if let Some(calculator) = service::get_price_calculator() {
                            if let Err(e) = calculator.request_calculation(
                                *pool_id,
                                pool_descriptor.clone(),
                                bundle.clone(),
                            ) {
                                logger::warning(
                                    LogTag::PoolFetcher,
                                    &format!(
                                        "Failed to request calculation for token {} in pool {}: {}",
                                        if is_sol_mint(&pool_descriptor.base_mint.to_string()) {
                                            pool_descriptor.quote_mint
                                        } else {
                                            pool_descriptor.base_mint
                                        },
                                        pool_id,
                                        e
                                    ),
                                );
                            } else {
                                let target_token =
                                    if is_sol_mint(&pool_descriptor.base_mint.to_string()) {
                                        pool_descriptor.quote_mint
                                    } else {
                                        pool_descriptor.base_mint
                                    };
                                logger::info(
                                    LogTag::PoolFetcher,
                                    &format!(
                                        "Requested calculation for complete bundle - token {} in pool {}",
                                        target_token,
                                        pool_id
                                    ),
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    /// Public interface: Request fetching of accounts for a pool
    pub fn request_pool_fetch(&self, pool_id: Pubkey, accounts: Vec<Pubkey>) -> Result<(), String> {
        let message = FetcherMessage::FetchPool { pool_id, accounts };
        self.fetcher_tx
            .send(message)
            .map_err(|e| format!("Failed to send fetch request: {}", e))?;
        Ok(())
    }

    /// Public interface: Request fetching of specific accounts
    pub fn request_accounts_fetch(&self, accounts: Vec<Pubkey>) -> Result<(), String> {
        let message = FetcherMessage::FetchAccounts { accounts };
        self.fetcher_tx
            .send(message)
            .map_err(|e| format!("Failed to send fetch request: {}", e))?;
        Ok(())
    }

    /// Get account bundle for a specific pool
    pub fn get_pool_bundle(&self, pool_id: &Pubkey) -> Option<PoolAccountBundle> {
        let bundles = self.account_bundles.read().unwrap();
        bundles.get(pool_id).cloned()
    }

    /// Get all account bundles
    pub fn get_all_bundles(&self) -> Vec<PoolAccountBundle> {
        let bundles = self.account_bundles.read().unwrap();
        bundles.values().cloned().collect()
    }

    /// Get specific account data
    pub fn get_account_data(&self, account: &Pubkey) -> Option<AccountData> {
        let bundles = self.account_bundles.read().unwrap();
        for bundle in bundles.values() {
            if let Some(account_data) = bundle.accounts.get(account) {
                return Some(account_data.clone());
            }
        }
        None
    }

    /// Clean up stale bundles
    pub fn cleanup_stale_bundles(&self, max_age_seconds: u64) {
        let mut bundles = self.account_bundles.write().unwrap();
        bundles.retain(|pool_id, bundle| {
            let should_keep = !bundle.is_stale(max_age_seconds);
            if !should_keep {
                logger::debug(LogTag::PoolFetcher, &format!("Removing stale bundle for pool: {}", pool_id));
            }
            should_keep
        });
    }

    /// Get fetch statistics
    pub fn get_fetch_stats(&self) -> FetchStats {
        let bundles = self.account_bundles.read().unwrap();
        let last_fetch = self.account_last_fetch.read().unwrap();

        FetchStats {
            total_bundles: bundles.len(),
            total_accounts_tracked: last_fetch.len(),
            bundles_with_data: bundles.values().filter(|b| !b.accounts.is_empty()).count(),
        }
    }
}

/// Fetch statistics
#[derive(Debug, Clone)]
pub struct FetchStats {
    pub total_bundles: usize,
    pub total_accounts_tracked: usize,
    pub bundles_with_data: usize,
}
