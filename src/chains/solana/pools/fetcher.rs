//! Account fetcher module.
//!
//! Handles efficient batched fetching of pool account data from RPC.
//! Optimizes RPC usage by batching requests and managing rate limits.
//!
//! Split across three files:
//! - `fetcher.rs` — struct definition, constructor, main loop, public API
//! - `fetcher_types.rs` — data types (AccountData, PoolAccountBundle, etc.)
//! - `fetcher_ops.rs` — helper methods (batch processing, missing-account handling, etc.)

// Re-export types from fetcher_types for backward compatibility
pub use super::fetcher_types::{AccountData, FetchStats, FetcherMessage, PoolAccountBundle};

use super::fetcher_types::{MissingAccountState, MissingPoolState};

use crate::logger::{self, LogTag};
use crate::pools::types::PoolDescriptor;

use crate::chains::solana::solana_sdk::pubkey::Pubkey;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, Notify};

/// Constants for batch processing
pub(crate) const ACCOUNT_BATCH_SIZE: usize = 50;
const FETCH_INTERVAL_MS: u64 = 500;
pub(crate) const ACCOUNT_STALE_THRESHOLD_SECONDS: u64 = 30;
pub(crate) const OPEN_POSITION_ACCOUNT_STALE_THRESHOLD_SECONDS: u64 = 5;

/// Account fetcher service
pub struct AccountFetcher {
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
    /// Metrics
    operations: Arc<std::sync::atomic::AtomicU64>,
    errors: Arc<std::sync::atomic::AtomicU64>,
    accounts_fetched: Arc<std::sync::atomic::AtomicU64>,
    rpc_batches: Arc<std::sync::atomic::AtomicU64>,
}

impl AccountFetcher {
    /// Create new account fetcher
    pub fn new(pool_directory: Arc<RwLock<HashMap<Pubkey, PoolDescriptor>>>) -> Self {
        let (fetcher_tx, fetcher_rx) = mpsc::unbounded_channel();

        Self {
            pool_directory,
            account_bundles: Arc::new(RwLock::new(HashMap::new())),
            account_last_fetch: Arc::new(RwLock::new(HashMap::new())),
            fetcher_rx: Arc::new(RwLock::new(Some(fetcher_rx))),
            fetcher_tx,
            operations: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            errors: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            accounts_fetched: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            rpc_batches: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        }
    }

    /// Get metrics for this fetcher instance
    pub fn get_metrics(&self) -> (u64, u64, u64, u64) {
        (
            self.operations.load(std::sync::atomic::Ordering::Relaxed),
            self.errors.load(std::sync::atomic::Ordering::Relaxed),
            self.accounts_fetched
                .load(std::sync::atomic::Ordering::Relaxed),
            self.rpc_batches.load(std::sync::atomic::Ordering::Relaxed),
        )
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

        let pool_directory = self.pool_directory.clone();
        let account_bundles = self.account_bundles.clone();
        let account_last_fetch = self.account_last_fetch.clone();

        // Clone metrics for tracking in background task
        let operations = Arc::clone(&self.operations);
        let errors = Arc::clone(&self.errors);
        let accounts_fetched = Arc::clone(&self.accounts_fetched);
        let rpc_batches = Arc::clone(&self.rpc_batches);

        // Take the receiver from the Arc<RwLock>
        let mut fetcher_rx = {
            let mut rx_lock = self.fetcher_rx.write().unwrap();
            rx_lock.take().expect("Fetcher receiver already taken")
        };

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(FETCH_INTERVAL_MS));
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
                            // Add timeout to prevent hanging on slow RPC responses
                            match tokio::time::timeout(
                                Duration::from_secs(30),
                                Self::process_pending_accounts(
                                    &pool_directory,
                                    &account_bundles,
                                    &account_last_fetch,
                                    &mut pending_accounts,
                                    &mut account_failure_tracker,
                                    &mut pool_failure_tracker,
                                    &operations,
                                    &errors,
                                    &accounts_fetched,
                                    &rpc_batches,
                                )
                            ).await {
                                Ok(_) => {},
                                Err(_) => {
                                    logger::warning(
                                        LogTag::PoolFetcher,
                                        "Account processing timed out after 30 seconds",
                                    );
                                }
                            }
                        }
                    }
                }
            }

            logger::info(LogTag::PoolFetcher, "Account fetcher task completed");
        });
    }

    /// Public interface: Request fetching of accounts for a pool
    pub fn request_pool_fetch(
        &self,
        pool_id: Pubkey,
        accounts: Vec<Pubkey>,
    ) -> crate::chains::solana::Result<()> {
        let message = FetcherMessage::FetchPool { pool_id, accounts };
        self.fetcher_tx
            .send(message)
            .map_err(|e| crate::chains::solana::Error::Rpc {
                operation: "request_pool_fetch",
                detail: e.to_string(),
            })?;
        Ok(())
    }

    /// Public interface: Request fetching of specific accounts
    pub fn request_accounts_fetch(
        &self,
        accounts: Vec<Pubkey>,
    ) -> crate::chains::solana::Result<()> {
        let message = FetcherMessage::FetchAccounts { accounts };
        self.fetcher_tx
            .send(message)
            .map_err(|e| crate::chains::solana::Error::Rpc {
                operation: "request_accounts_fetch",
                detail: e.to_string(),
            })?;
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
                    &format!("Removing stale bundle for pool: {pool_id}"),
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
