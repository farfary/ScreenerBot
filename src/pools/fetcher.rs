/// Pool account data fetcher
/// Fetches account data and stores directly in PoolCache using AccountData

use crate::pools::cache::{ PoolCache, AccountData };
use crate::pools::constants::RPC_MULTIPLE_ACCOUNTS_BATCH_SIZE;
use crate::rpc::{ get_rpc_client };
use solana_sdk::{ account::Account, pubkey::Pubkey };
use tokio::sync::RwLock;
use std::sync::Arc;
use std::str::FromStr;
use chrono::{ DateTime, Utc };
use crate::logger::{ log, LogTag };

/// Account data fetcher service
pub struct PoolFetcher {
    cache: Arc<PoolCache>,
    /// Whether fetching is currently running
    is_fetching: Arc<RwLock<bool>>,
}

impl PoolFetcher {
    pub fn new(cache: Arc<PoolCache>) -> Self {
        Self {
            cache,
            is_fetching: Arc::new(RwLock::new(false)),
        }
    }

    /// Fetch all account data for addresses prepared by analyzer
    pub async fn fetch_all_required_accounts(&self, addresses: &[String]) -> Result<(), String> {
        if addresses.is_empty() {
            log(LogTag::Pool, "FETCHER_SKIP", "No addresses to fetch");
            return Ok(());
        }

        // Check if already fetching
        {
            let mut is_fetching = self.is_fetching.write().await;
            if *is_fetching {
                log(LogTag::Pool, "FETCHER_BUSY", "Fetcher already running");
                return Ok(());
            }
            *is_fetching = true;
        }

        log(
            LogTag::Pool,
            "FETCHER_START",
            &format!("🔄 Starting to fetch {} account addresses", addresses.len())
        );

        let result = self.fetch_accounts_batch(addresses).await;

        // Reset fetching flag
        {
            let mut is_fetching = self.is_fetching.write().await;
            *is_fetching = false;
        }

        result
    }

    /// Fetch accounts in batches to respect RPC limits
    async fn fetch_accounts_batch(&self, addresses: &[String]) -> Result<(), String> {
        let mut fetched_count = 0;
        let mut cached_count = 0;
        let mut error_count = 0;

        // Filter addresses that need fetching (not cached or expired)
        let mut addresses_to_fetch = Vec::new();

        for address in addresses {
            if self.needs_fetching(address).await {
                addresses_to_fetch.push(address.clone());
            } else {
                cached_count += 1;
            }
        }

        if addresses_to_fetch.is_empty() {
            log(
                LogTag::Pool,
                "FETCHER_CACHED",
                &format!("✅ All {} addresses already cached", addresses.len())
            );
            return Ok(());
        }

        log(
            LogTag::Pool,
            "FETCHER_NEED",
            &format!(
                "📋 Need to fetch {} addresses ({} cached)",
                addresses_to_fetch.len(),
                cached_count
            )
        );

        // Process in batches
        let batch_size = RPC_MULTIPLE_ACCOUNTS_BATCH_SIZE;
        for chunk in addresses_to_fetch.chunks(batch_size) {
            match self.fetch_address_chunk(chunk).await {
                Ok(count) => {
                    fetched_count += count;
                    log(
                        LogTag::Pool,
                        "FETCHER_BATCH",
                        &format!("✅ Fetched batch of {} accounts", count)
                    );
                }
                Err(e) => {
                    error_count += chunk.len();
                    log(
                        LogTag::Pool,
                        "FETCHER_BATCH_ERROR",
                        &format!("❌ Failed to fetch batch: {}", e)
                    );
                }
            }

            // Small delay between batches to avoid overwhelming RPC
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }

        log(
            LogTag::Pool,
            "FETCHER_COMPLETE",
            &format!(
                "✅ Fetch complete: {} fetched, {} cached, {} errors",
                fetched_count,
                cached_count,
                error_count
            )
        );

        Ok(())
    }

    /// Check if an address needs fetching (not cached or expired)
    async fn needs_fetching(&self, address: &str) -> bool {
        match self.cache.get_account(address).await {
            Some(account_data) => account_data.is_expired(),
            None => true,
        }
    }

    /// Fetch a chunk of addresses using RPC
    async fn fetch_address_chunk(&self, addresses: &[String]) -> Result<usize, String> {
        // Convert string addresses to Pubkeys
        let mut pubkeys = Vec::new();
        for address in addresses {
            match Pubkey::from_str(address) {
                Ok(pubkey) => pubkeys.push(pubkey),
                Err(e) => {
                    log(
                        LogTag::Pool,
                        "FETCHER_INVALID_ADDRESS",
                        &format!("❌ Invalid address {}: {}", &address[..8], e)
                    );
                    continue;
                }
            }
        }

        if pubkeys.is_empty() {
            return Ok(0);
        }

        // Get RPC client and fetch accounts
        let rpc_client = get_rpc_client();
        let accounts = rpc_client
            .get_multiple_accounts(&pubkeys).await
            .map_err(|e| format!("RPC error: {}", e))?;

        // Store the results directly in cache
        let mut cached_count = 0;

        for (i, account_option) in accounts.iter().enumerate() {
            let address = &addresses[i];

            match account_option {
                Some(account) => {
                    cached_count += 1;
                    self.cache.store_account(
                        address.clone(),
                        account.data.clone(),
                        account.lamports,
                        account.owner.to_string(),
                    ).await;
                }
                None => {
                    self.cache.store_non_existent_account(address.clone()).await;
                }
            }
        }

        log(
            LogTag::Pool,
            "FETCHER_CHUNK_SUCCESS",
            &format!(
                "📦 Cached {} accounts ({} existing, {} non-existent)",
                addresses.len(),
                cached_count,
                addresses.len() - cached_count
            )
        );

        Ok(cached_count)
    }

    /// Get fetcher statistics
    pub async fn get_fetcher_stats(&self) -> FetcherStats {
        let cache_stats = self.cache.get_stats().await;
        let is_fetching = *self.is_fetching.read().await;

        FetcherStats {
            total_cached: cache_stats.accounts_count,
            existing_accounts: cache_stats.accounts_count, // All cached accounts are valid
            non_existent_accounts: 0, // We don't track this separately anymore
            expired_accounts: 0, // Cache handles expiration automatically
            is_fetching,
            updated_at: Utc::now(),
        }
    }

    /// Force refresh specific addresses (bypass cache)
    pub async fn force_refresh_addresses(&self, addresses: &[String]) -> Result<(), String> {
        // Remove from cache to force refresh
        for address in addresses {
            self.cache.remove_account(address).await;
        }

        // Fetch fresh data
        self.fetch_accounts_batch(addresses).await
    }

    /// Check if fetcher is currently running
    pub async fn is_fetching(&self) -> bool {
        *self.is_fetching.read().await
    }
}

/// Fetcher statistics
#[derive(Debug, Clone)]
pub struct FetcherStats {
    pub total_cached: usize,
    pub existing_accounts: usize,
    pub non_existent_accounts: usize,
    pub expired_accounts: usize,
    pub is_fetching: bool,
    pub updated_at: DateTime<Utc>,
}
