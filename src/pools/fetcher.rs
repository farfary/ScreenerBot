/// Pool account data fetcher
/// Fetches account data for all addresses prepared by the analyzer using RPC client

use crate::pools::cache::PoolCache;
use crate::pools::constants::RPC_MULTIPLE_ACCOUNTS_BATCH_SIZE;
use crate::rpc::{ get_rpc_client };
use solana_sdk::{ account::Account, pubkey::Pubkey };
use tokio::sync::RwLock;
use std::sync::Arc;
use std::collections::HashMap;
use std::str::FromStr;
use chrono::{ DateTime, Utc };
use crate::logger::{ log, LogTag };

/// Cached account data with expiration
#[derive(Debug, Clone)]
pub struct CachedAccountData {
    /// Account address
    pub address: String,
    /// Raw account data
    pub data: Vec<u8>,
    /// Account lamports
    pub lamports: u64,
    /// Account owner program
    pub owner: String,
    /// When this data was fetched
    pub fetched_at: DateTime<Utc>,
    /// Whether the account exists
    pub exists: bool,
}

impl CachedAccountData {
    /// Create new cached account data
    pub fn new(address: String, account: Account) -> Self {
        Self {
            address,
            data: account.data,
            lamports: account.lamports,
            owner: account.owner.to_string(),
            fetched_at: Utc::now(),
            exists: true,
        }
    }

    /// Create cached data for non-existent account
    pub fn non_existent(address: String) -> Self {
        Self {
            address,
            data: Vec::new(),
            lamports: 0,
            owner: String::new(),
            fetched_at: Utc::now(),
            exists: false,
        }
    }

    /// Check if cached data is expired (10 minutes TTL)
    pub fn is_expired(&self) -> bool {
        let now = Utc::now();
        let age = now.signed_duration_since(self.fetched_at);
        age.num_seconds() > 600 // 10 minutes
    }
}

/// Account data fetcher service
pub struct PoolFetcher {
    cache: Arc<PoolCache>,
    /// Local cache for fetched account data
    account_cache: Arc<RwLock<HashMap<String, CachedAccountData>>>,
    /// Whether fetching is currently running
    is_fetching: Arc<RwLock<bool>>,
}

impl PoolFetcher {
    pub fn new(cache: Arc<PoolCache>) -> Self {
        Self {
            cache,
            account_cache: Arc::new(RwLock::new(HashMap::new())),
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
        let cache = self.account_cache.read().await;
        match cache.get(address) {
            Some(cached) => cached.is_expired(),
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

        // Cache the results
        let mut cache = self.account_cache.write().await;
        let mut cached_count = 0;

        for (i, account_option) in accounts.iter().enumerate() {
            let address: &String = &addresses[i];

            let cached_data = match account_option {
                Some(account) => {
                    cached_count += 1;
                    CachedAccountData::new(address.clone(), account.clone())
                }
                None => { CachedAccountData::non_existent(address.clone()) }
            };

            // Store in local cache
            cache.insert(address.clone(), cached_data.clone());

            // Also store in pool cache for broader access
            if cached_data.exists {
                self.cache.cache_account_data(address, cached_data.data.clone()).await;
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

    /// Get cached account data for an address
    pub async fn get_cached_account_data(&self, address: &str) -> Option<CachedAccountData> {
        let cache = self.account_cache.read().await;
        cache
            .get(address)
            .filter(|data| !data.is_expired())
            .cloned()
    }

    /// Get all cached account data
    pub async fn get_all_cached_accounts(&self) -> HashMap<String, CachedAccountData> {
        let cache = self.account_cache.read().await;
        cache
            .iter()
            .filter(|(_, data)| !data.is_expired())
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }

    /// Clean expired cache entries
    pub async fn clean_expired_cache(&self) -> usize {
        let mut cache = self.account_cache.write().await;
        let initial_count = cache.len();

        cache.retain(|_, data| !data.is_expired());

        let cleaned_count = initial_count - cache.len();

        if cleaned_count > 0 {
            log(
                LogTag::Pool,
                "FETCHER_CLEANUP",
                &format!("🧹 Cleaned {} expired account cache entries", cleaned_count)
            );
        }

        cleaned_count
    }

    /// Get fetcher statistics
    pub async fn get_fetcher_stats(&self) -> FetcherStats {
        let cache = self.account_cache.read().await;
        let total_cached = cache.len();
        let existing_accounts = cache
            .values()
            .filter(|data| data.exists && !data.is_expired())
            .count();
        let non_existent_accounts = cache
            .values()
            .filter(|data| !data.exists && !data.is_expired())
            .count();
        let expired_accounts = cache
            .values()
            .filter(|data| data.is_expired())
            .count();
        let is_fetching = *self.is_fetching.read().await;

        FetcherStats {
            total_cached,
            existing_accounts,
            non_existent_accounts,
            expired_accounts,
            is_fetching,
            updated_at: Utc::now(),
        }
    }

    /// Force refresh specific addresses (bypass cache)
    pub async fn force_refresh_addresses(&self, addresses: &[String]) -> Result<(), String> {
        // Remove from cache to force refresh
        {
            let mut cache = self.account_cache.write().await;
            for address in addresses {
                cache.remove(address);
            }
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
