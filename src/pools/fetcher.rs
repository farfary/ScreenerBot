/// Account fetcher module
///
/// This module handles efficient batched fetching of pool account data from RPC.
/// It optimizes RPC usage by batching requests and managing rate limits.
use super::types::PoolDescriptor;
use super::utils::is_sol_mint;

use crate::events::{record_safe, Event, EventCategory, Severity};
use crate::logger::{self, LogTag};
use crate::pools::service;
use crate::rpc::{get_rpc_client, RpcClient};

use solana_sdk::{account::Account, pubkey::Pubkey};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, Notify};

/// Constants for batch processing
const ACCOUNT_BATCH_SIZE: usize = 50;
const FETCH_INTERVAL_MS: u64 = 500;
const ACCOUNT_STALE_THRESHOLD_SECONDS: u64 = 30;
const OPEN_POSITION_ACCOUNT_STALE_THRESHOLD_SECONDS: u64 = 5;
const ACCOUNT_BLACKLIST_THRESHOLD: u32 = 3;
const POOL_BLACKLIST_THRESHOLD: u32 = 2;
const FAILURE_WINDOW_SECS: u64 = 300;

#[derive(Debug, Clone)]
struct MissingAccountState {
    failures: u32,
    last_failure: Instant,
    blacklisted: bool,
}

#[derive(Debug, Clone)]
struct MissingPoolState {
    failures: u32,
    last_failure: Instant,
    blacklisted: bool,
}

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
            let mut account_failure_tracker: HashMap<Pubkey, MissingAccountState> = HashMap::new();
            let mut pool_failure_tracker: HashMap<Pubkey, MissingPoolState> = HashMap::new();

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
                                &mut pending_accounts,
                                &mut account_failure_tracker,
                                &mut pool_failure_tracker,
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
            // Check if pool is blacklisted
            match super::db::is_pool_blacklisted(&pool.pool_id.to_string()).await {
                Ok(true) => {
                    continue; // Skip blacklisted pool silently
                }
                Ok(false) => {
                    // Not blacklisted, proceed to process pool
                }
                Err(e) => {
                    logger::warning(
                        LogTag::PoolFetcher,
                        &format!(
                            "Failed to check blacklist for pool {}: {} - skipping as precaution",
                            pool.pool_id, e
                        ),
                    );
                    // FAIL-CLOSED: Skip pool if blacklist check fails
                    continue;
                }
            }

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
                // Check if account is blacklisted
                match super::db::is_account_blacklisted(&account.to_string()).await {
                    Ok(true) => {
                        continue; // Skip blacklisted account silently
                    }
                    Ok(false) => {
                        // Not blacklisted, proceed to check if needs fetch
                    }
                    Err(e) => {
                        logger::warning(
                            LogTag::PoolFetcher,
                            &format!(
                                "Failed to check blacklist for account {}: {} - skipping as precaution",
                                account, e
                            ),
                        );
                        // FAIL-CLOSED: Skip account if blacklist check fails to avoid RPC waste
                        continue;
                    }
                }

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
        account_failure_tracker: &mut HashMap<Pubkey, MissingAccountState>,
        pool_failure_tracker: &mut HashMap<Pubkey, MissingPoolState>,
    ) {
        if pending_accounts.is_empty() {
            return;
        }

        // Convert to vector and batch
        let drained_accounts: Vec<Pubkey> = pending_accounts.drain().collect();

        if drained_accounts.is_empty() {
            return;
        }

        let mut accounts_to_fetch = Vec::with_capacity(drained_accounts.len());
        for account in drained_accounts {
            let account_key = account.to_string();
            match super::db::is_account_blacklisted(&account_key).await {
                Ok(true) => {
                    pending_accounts.remove(&account);
                }
                Ok(false) => {
                    accounts_to_fetch.push(account);
                }
                Err(e) => {
                    pending_accounts.remove(&account);
                    logger::warning(
                        LogTag::PoolFetcher,
                        &format!(
                            "Failed to check blacklist for account {}: {} - skipping as precaution",
                            account_key, e
                        ),
                    );
                }
            }
        }

        if accounts_to_fetch.is_empty() {
            return;
        }

        logger::debug(
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
                Ok((account_data_list, missing_accounts)) => {
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

                    // Update last fetch times only for successful accounts
                    {
                        let mut last_fetch = account_last_fetch.write().unwrap();
                        let now = Instant::now();
                        for acc_data in &account_data_list {
                            last_fetch.insert(acc_data.pubkey, now);
                        }
                        for missing in &missing_accounts {
                            last_fetch.insert(*missing, now);
                        }
                    }

                    // Ensure missing accounts are not kept pending within this tick
                    for missing in &missing_accounts {
                        pending_accounts.remove(missing);
                    }

                    Self::handle_missing_accounts(
                        &missing_accounts,
                        pool_directory,
                        account_failure_tracker,
                        pool_failure_tracker,
                    )
                    .await;
                    Self::cleanup_missing_failure_trackers(
                        account_failure_tracker,
                        pool_failure_tracker,
                    );

                    // Organize accounts into pool bundles
                    Self::organize_accounts_into_bundles(
                        &account_data_list,
                        pool_directory,
                        account_bundles,
                    )
                    .await;

                    logger::debug(
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

    async fn handle_missing_accounts(
        missing_accounts: &[Pubkey],
        pool_directory: &Arc<RwLock<HashMap<Pubkey, PoolDescriptor>>>,
        account_failure_tracker: &mut HashMap<Pubkey, MissingAccountState>,
        pool_failure_tracker: &mut HashMap<Pubkey, MissingPoolState>,
    ) {
        if missing_accounts.is_empty() {
            return;
        }

        for account in missing_accounts {
            let directory_snapshot: Vec<(Pubkey, PoolDescriptor)> = {
                let directory_guard = pool_directory.read().unwrap();
                directory_guard
                    .iter()
                    .filter(|(_, descriptor)| descriptor.reserve_accounts.contains(account))
                    .map(|(pool_id, descriptor)| (*pool_id, descriptor.clone()))
                    .collect()
            };

            let account_state =
                account_failure_tracker
                    .entry(*account)
                    .or_insert(MissingAccountState {
                        failures: 0,
                        last_failure: Instant::now(),
                        blacklisted: false,
                    });
            account_state.failures = account_state.failures.saturating_add(1);
            account_state.last_failure = Instant::now();

            if account_state.failures >= ACCOUNT_BLACKLIST_THRESHOLD && !account_state.blacklisted {
                let (pool_id_str, token_mint_str) = directory_snapshot
                    .get(0)
                    .map(|(pool_id, descriptor)| {
                        let token_mint = if is_sol_mint(&descriptor.base_mint.to_string()) {
                            descriptor.quote_mint.to_string()
                        } else {
                            descriptor.base_mint.to_string()
                        };
                        (Some(pool_id.to_string()), Some(token_mint))
                    })
                    .unwrap_or((None, None));

                match super::db::add_account_to_blacklist(
                    &account.to_string(),
                    "account_not_found_threshold",
                    Some("rpc_fetch"),
                    pool_id_str.as_deref(),
                    token_mint_str.as_deref(),
                )
                .await
                {
                    Ok(()) => {
                        account_state.blacklisted = true;
                        logger::warning(
                            LogTag::PoolFetcher,
                            &format!(
                                "Blacklisted account {} after {} consecutive misses",
                                account, account_state.failures
                            ),
                        );
                        record_safe(Event::warn(
                            EventCategory::Pool,
                            Some("account_blacklisted_after_threshold".to_string()),
                            token_mint_str.clone(),
                            pool_id_str.clone(),
                            serde_json::json!({
                                "account": account.to_string(),
                                "failures": account_state.failures,
                                "threshold": ACCOUNT_BLACKLIST_THRESHOLD,
                                "pool_id": pool_id_str,
                                "token_mint": token_mint_str,
                            }),
                        ))
                        .await;
                    }
                    Err(e) => {
                        logger::warning(
                            LogTag::PoolFetcher,
                            &format!("Failed to persist account blacklist for {}: {}", account, e),
                        );
                    }
                }
            }

            for (pool_id, descriptor) in directory_snapshot.iter() {
                let pool_state = pool_failure_tracker
                    .entry(*pool_id)
                    .or_insert(MissingPoolState {
                        failures: 0,
                        last_failure: Instant::now(),
                        blacklisted: false,
                    });
                pool_state.failures = pool_state.failures.saturating_add(1);
                pool_state.last_failure = Instant::now();

                if pool_state.failures >= POOL_BLACKLIST_THRESHOLD && !pool_state.blacklisted {
                    let token_mint = if is_sol_mint(&descriptor.base_mint.to_string()) {
                        descriptor.quote_mint.to_string()
                    } else {
                        descriptor.base_mint.to_string()
                    };
                    let program_id = descriptor.program_kind.program_id();

                    match super::db::add_pool_to_blacklist(
                        &pool_id.to_string(),
                        "missing_accounts",
                        Some(&token_mint),
                        if program_id.is_empty() {
                            None
                        } else {
                            Some(program_id)
                        },
                    )
                    .await
                    {
                        Ok(()) => {
                            pool_state.blacklisted = true;
                            logger::warning(
                                LogTag::PoolFetcher,
                                &format!(
                                    "Blacklisted pool {} (token {}) after {} missing-account hits",
                                    pool_id, token_mint, pool_state.failures
                                ),
                            );
                            record_safe(Event::warn(
                                EventCategory::Pool,
                                Some("pool_blacklisted_missing_accounts".to_string()),
                                Some(token_mint.clone()),
                                Some(pool_id.to_string()),
                                serde_json::json!({
                                    "pool_id": pool_id.to_string(),
                                    "program_kind": descriptor.program_kind.display_name(),
                                    "program_id": program_id,
                                    "failures": pool_state.failures,
                                    "threshold": POOL_BLACKLIST_THRESHOLD,
                                    "missing_account": account.to_string(),
                                }),
                            ))
                            .await;
                        }
                        Err(e) => {
                            logger::warning(
                                LogTag::PoolFetcher,
                                &format!("Failed to persist pool blacklist for {}: {}", pool_id, e),
                            );
                        }
                    }
                }
            }
        }
    }

    fn cleanup_missing_failure_trackers(
        account_failure_tracker: &mut HashMap<Pubkey, MissingAccountState>,
        pool_failure_tracker: &mut HashMap<Pubkey, MissingPoolState>,
    ) {
        let expiry = Duration::from_secs(FAILURE_WINDOW_SECS);
        let now = Instant::now();

        account_failure_tracker.retain(|_, state| {
            state.blacklisted || now.duration_since(state.last_failure) <= expiry
        });

        pool_failure_tracker.retain(|_, state| {
            state.blacklisted || now.duration_since(state.last_failure) <= expiry
        });
    }

    /// Fetch a batch of accounts
    async fn fetch_account_batch(
        rpc_client: &Arc<RpcClient>,
        accounts: &[Pubkey],
    ) -> Result<(Vec<AccountData>, Vec<Pubkey>), String> {
        // Check connectivity before RPC batch fetch - graceful degradation
        if let Some(unhealthy) = crate::connectivity::check_endpoints_healthy(&["rpc"]).await {
            logger::debug(
                LogTag::PoolFetcher,
                &format!(
                    "Skipping account batch fetch ({} accounts) - Unhealthy endpoints: {}",
                    accounts.len(),
                    unhealthy
                ),
            );
            // Return empty list - caller will use cached data
            return Ok((Vec::new(), Vec::new()));
        }

        if accounts.is_empty() {
            return Ok((Vec::new(), Vec::new()));
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

        let mut account_data_list: Vec<AccountData> = Vec::new();
        let mut missing_accounts: Vec<Pubkey> = Vec::new();

        for (i, account_opt) in account_results.iter().enumerate() {
            if let Some(account) = account_opt {
                let account_data = AccountData::from_account(accounts[i], account.clone(), 0);
                account_data_list.push(account_data);
            } else {
                let missing_key = accounts[i];
                missing_accounts.push(missing_key);
                logger::warning(
                    LogTag::PoolFetcher,
                    &format!("Account not found: {}", missing_key),
                );
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
                    "missing_accounts": missing_accounts.iter().map(|p| p.to_string()).collect::<Vec<_>>(),
                    "action": "failure_recorded"
                }),
            ))
            .await;
        }

        Ok((account_data_list, missing_accounts))
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
                                logger::debug(
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
                logger::debug(
                    LogTag::PoolFetcher,
                    &format!("Removing stale bundle for pool: {}", pool_id),
                );
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
