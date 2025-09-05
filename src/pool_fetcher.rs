use crate::logger::{ log, LogTag };
use crate::rpc::get_rpc_client;
use chrono::{ DateTime, Utc };
use solana_sdk::{ account::Account, pubkey::Pubkey };
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use std::time::{ Duration, Instant };
use tokio::sync::RwLock;

// =============================================================================
// CONSTANTS
// =============================================================================

/// Maximum number of accounts to fetch in one get_multiple_accounts call
const RPC_MULTIPLE_ACCOUNTS_BATCH_SIZE: usize = 100;

/// Token account cache TTL in seconds
const TOKEN_ACCOUNT_CACHE_TTL_SECS: i64 = 300; // 5 minutes

// =============================================================================
// DATA STRUCTURES
// =============================================================================

/// Token account information
#[derive(Debug, Clone)]
pub struct TokenAccountInfo {
    pub token_mint: String,
    pub account_address: String,
    pub owner: String,
    pub amount: u64,
    pub decimals: u8,
    pub is_frozen: bool,
    pub is_native: bool,
    pub last_updated: DateTime<Utc>,
}

/// Token mint information
#[derive(Debug, Clone)]
pub struct TokenMintInfo {
    pub mint_address: String,
    pub supply: u64,
    pub decimals: u8,
    pub mint_authority: Option<String>,
    pub freeze_authority: Option<String>,
    pub is_initialized: bool,
    pub last_updated: DateTime<Utc>,
}

/// Token metadata information
#[derive(Debug, Clone)]
pub struct TokenMetadataInfo {
    pub mint_address: String,
    pub name: Option<String>,
    pub symbol: Option<String>,
    pub uri: Option<String>,
    pub last_updated: DateTime<Utc>,
}

/// Pool fetcher service
pub struct PoolFetcher {
    rpc_client: &'static crate::rpc::RpcClient,
    /// Token account cache: token_mint -> TokenAccountInfo
    token_account_cache: Arc<RwLock<HashMap<String, TokenAccountInfo>>>,
    /// Token mint cache: mint_address -> TokenMintInfo
    token_mint_cache: Arc<RwLock<HashMap<String, TokenMintInfo>>>,
    /// Token metadata cache: mint_address -> TokenMetadataInfo
    token_metadata_cache: Arc<RwLock<HashMap<String, TokenMetadataInfo>>>,
    /// Statistics
    stats: Arc<RwLock<PoolFetcherStats>>,
}

/// Pool fetcher statistics
#[derive(Debug, Clone)]
pub struct PoolFetcherStats {
    pub total_requests: u64,
    pub successful_fetches: u64,
    pub failed_fetches: u64,
    pub cache_hits: u64,
    pub average_fetch_time_ms: f64,
    pub last_update: Option<DateTime<Utc>>,
}

impl Default for PoolFetcherStats {
    fn default() -> Self {
        Self {
            total_requests: 0,
            successful_fetches: 0,
            failed_fetches: 0,
            cache_hits: 0,
            average_fetch_time_ms: 0.0,
            last_update: None,
        }
    }
}

// =============================================================================
// IMPLEMENTATIONS
// =============================================================================

impl PoolFetcher {
    /// Create new pool fetcher
    pub fn new() -> Self {
        Self {
            rpc_client: get_rpc_client(),
            token_account_cache: Arc::new(RwLock::new(HashMap::new())),
            token_mint_cache: Arc::new(RwLock::new(HashMap::new())),
            token_metadata_cache: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(PoolFetcherStats::default())),
        }
    }

    /// Fetch token account data for multiple tokens
    pub async fn fetch_token_accounts(
        &self,
        token_mints: &[String]
    ) -> Result<HashMap<String, TokenAccountInfo>, String> {
        if token_mints.is_empty() {
            return Ok(HashMap::new());
        }

        let start_time = Instant::now();
        let mut result = HashMap::new();
        let mut cache_hits = 0;

        // Check cache first
        {
            let cache = self.token_account_cache.read().await;
            let now = Utc::now();

            for token_mint in token_mints {
                if let Some(cached_info) = cache.get(token_mint) {
                    let age = now.signed_duration_since(cached_info.last_updated);
                    if age.num_seconds() < TOKEN_ACCOUNT_CACHE_TTL_SECS {
                        result.insert(token_mint.clone(), cached_info.clone());
                        cache_hits += 1;
                    }
                }
            }
        }

        // Fetch missing tokens
        let tokens_to_fetch: Vec<String> = token_mints
            .iter()
            .filter(|mint| !result.contains_key(*mint))
            .cloned()
            .collect();

        if !tokens_to_fetch.is_empty() {
            let fetched_accounts = self.fetch_token_accounts_batch(&tokens_to_fetch).await?;
            
            // Update cache
            {
                let mut cache = self.token_account_cache.write().await;
                for (token_mint, account_info) in &fetched_accounts {
                    cache.insert(token_mint.clone(), account_info.clone());
                }
            }

            result.extend(fetched_accounts);
        }

        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.total_requests += token_mints.len() as u64;
            stats.successful_fetches += result.len() as u64;
            stats.cache_hits += cache_hits;
            stats.average_fetch_time_ms = (stats.average_fetch_time_ms + start_time.elapsed().as_millis() as f64) / 2.0;
            stats.last_update = Some(Utc::now());
        }

        Ok(result)
    }

    /// Fetch token mint information for multiple tokens
    pub async fn fetch_token_mints(
        &self,
        token_mints: &[String]
    ) -> Result<HashMap<String, TokenMintInfo>, String> {
        if token_mints.is_empty() {
            return Ok(HashMap::new());
        }

        let start_time = Instant::now();
        let mut result = HashMap::new();
        let mut cache_hits = 0;

        // Check cache first
        {
            let cache = self.token_mint_cache.read().await;
            let now = Utc::now();

            for token_mint in token_mints {
                if let Some(cached_info) = cache.get(token_mint) {
                    let age = now.signed_duration_since(cached_info.last_updated);
                    if age.num_seconds() < TOKEN_ACCOUNT_CACHE_TTL_SECS {
                        result.insert(token_mint.clone(), cached_info.clone());
                        cache_hits += 1;
                    }
                }
            }
        }

        // Fetch missing tokens
        let tokens_to_fetch: Vec<String> = token_mints
            .iter()
            .filter(|mint| !result.contains_key(*mint))
            .cloned()
            .collect();

        if !tokens_to_fetch.is_empty() {
            let fetched_mints = self.fetch_token_mints_batch(&tokens_to_fetch).await?;
            
            // Update cache
            {
                let mut cache = self.token_mint_cache.write().await;
                for (token_mint, mint_info) in &fetched_mints {
                    cache.insert(token_mint.clone(), mint_info.clone());
                }
            }

            result.extend(fetched_mints);
        }

        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.total_requests += token_mints.len() as u64;
            stats.successful_fetches += result.len() as u64;
            stats.cache_hits += cache_hits;
            stats.average_fetch_time_ms = (stats.average_fetch_time_ms + start_time.elapsed().as_millis() as f64) / 2.0;
            stats.last_update = Some(Utc::now());
        }

        Ok(result)
    }

    /// Fetch token metadata for multiple tokens
    pub async fn fetch_token_metadata(
        &self,
        token_mints: &[String]
    ) -> Result<HashMap<String, TokenMetadataInfo>, String> {
        if token_mints.is_empty() {
            return Ok(HashMap::new());
        }

        let start_time = Instant::now();
        let mut result = HashMap::new();
        let mut cache_hits = 0;

        // Check cache first
        {
            let cache = self.token_metadata_cache.read().await;
            let now = Utc::now();

            for token_mint in token_mints {
                if let Some(cached_info) = cache.get(token_mint) {
                    let age = now.signed_duration_since(cached_info.last_updated);
                    if age.num_seconds() < TOKEN_ACCOUNT_CACHE_TTL_SECS {
                        result.insert(token_mint.clone(), cached_info.clone());
                        cache_hits += 1;
                    }
                }
            }
        }

        // Fetch missing tokens
        let tokens_to_fetch: Vec<String> = token_mints
            .iter()
            .filter(|mint| !result.contains_key(*mint))
            .cloned()
            .collect();

        if !tokens_to_fetch.is_empty() {
            let fetched_metadata = self.fetch_token_metadata_batch(&tokens_to_fetch).await?;
            
            // Update cache
            {
                let mut cache = self.token_metadata_cache.write().await;
                for (token_mint, metadata_info) in &fetched_metadata {
                    cache.insert(token_mint.clone(), metadata_info.clone());
                }
            }

            result.extend(fetched_metadata);
        }

        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.total_requests += token_mints.len() as u64;
            stats.successful_fetches += result.len() as u64;
            stats.cache_hits += cache_hits;
            stats.average_fetch_time_ms = (stats.average_fetch_time_ms + start_time.elapsed().as_millis() as f64) / 2.0;
            stats.last_update = Some(Utc::now());
        }

        Ok(result)
    }

    /// Fetch account data for multiple account addresses
    pub async fn fetch_account_data(
        &self,
        account_addresses: &[String]
    ) -> Result<HashMap<String, Vec<u8>>, String> {
        if account_addresses.is_empty() {
            return Ok(HashMap::new());
        }

        let start_time = Instant::now();
        let mut result = HashMap::new();

        // Parse account addresses to Pubkeys
        let pubkeys: Result<Vec<Pubkey>, String> = account_addresses
            .iter()
            .map(|addr| Pubkey::from_str(addr).map_err(|e| format!("Invalid address {}: {}", addr, e)))
            .collect();

        let pubkeys = pubkeys?;

        // Fetch account data in batches
        for chunk in pubkeys.chunks(RPC_MULTIPLE_ACCOUNTS_BATCH_SIZE) {
            match self.rpc_client.get_multiple_accounts(chunk).await {
                Ok(accounts) => {
                    for (i, account) in accounts.iter().enumerate() {
                        if let Some(account) = account {
                            result.insert(chunk[i].to_string(), account.data.clone());
                        }
                    }
                }
                Err(e) => {
                    return Err(format!("Failed to fetch account data: {}", e));
                }
            }
        }

        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.total_requests += account_addresses.len() as u64;
            stats.successful_fetches += result.len() as u64;
            stats.average_fetch_time_ms = (stats.average_fetch_time_ms + start_time.elapsed().as_millis() as f64) / 2.0;
            stats.last_update = Some(Utc::now());
        }

        Ok(result)
    }

    /// Get statistics
    pub async fn get_stats(&self) -> PoolFetcherStats {
        self.stats.read().await.clone()
    }

    /// Clear all caches
    pub async fn clear_caches(&self) {
        {
            let mut cache = self.token_account_cache.write().await;
            cache.clear();
        }
        {
            let mut cache = self.token_mint_cache.write().await;
            cache.clear();
        }
        {
            let mut cache = self.token_metadata_cache.write().await;
            cache.clear();
        }

        log(LogTag::Pool, "CACHE_CLEAR", "Cleared all pool fetcher caches");
    }

    // =============================================================================
    // PRIVATE METHODS
    // =============================================================================

    /// Fetch token accounts in batches
    async fn fetch_token_accounts_batch(
        &self,
        token_mints: &[String]
    ) -> Result<HashMap<String, TokenAccountInfo>, String> {
        // TODO: Implement actual token account fetching
        // This would involve:
        // 1. Getting associated token accounts for each mint
        // 2. Fetching account data using get_multiple_accounts
        // 3. Parsing token account data
        // 4. Returning structured TokenAccountInfo

        let mut result = HashMap::new();
        let now = Utc::now();

        // Placeholder implementation
        for token_mint in token_mints {
            let account_info = TokenAccountInfo {
                token_mint: token_mint.clone(),
                account_address: format!("account_for_{}", &token_mint[0..8]),
                owner: "placeholder_owner".to_string(),
                amount: 1000000,
                decimals: 9,
                is_frozen: false,
                is_native: false,
                last_updated: now,
            };

            result.insert(token_mint.clone(), account_info);
        }

        Ok(result)
    }

    /// Fetch token mints in batches
    async fn fetch_token_mints_batch(
        &self,
        token_mints: &[String]
    ) -> Result<HashMap<String, TokenMintInfo>, String> {
        // TODO: Implement actual token mint fetching
        // This would involve:
        // 1. Parsing mint addresses
        // 2. Fetching mint account data using get_multiple_accounts
        // 3. Parsing mint account data
        // 4. Returning structured TokenMintInfo

        let mut result = HashMap::new();
        let now = Utc::now();

        // Placeholder implementation
        for token_mint in token_mints {
            let mint_info = TokenMintInfo {
                mint_address: token_mint.clone(),
                supply: 1000000000,
                decimals: 9,
                mint_authority: Some("mint_authority".to_string()),
                freeze_authority: Some("freeze_authority".to_string()),
                is_initialized: true,
                last_updated: now,
            };

            result.insert(token_mint.clone(), mint_info);
        }

        Ok(result)
    }

    /// Fetch token metadata in batches
    async fn fetch_token_metadata_batch(
        &self,
        token_mints: &[String]
    ) -> Result<HashMap<String, TokenMetadataInfo>, String> {
        // TODO: Implement actual token metadata fetching
        // This would involve:
        // 1. Getting metadata program accounts
        // 2. Fetching metadata account data
        // 3. Parsing metadata (name, symbol, URI)
        // 4. Returning structured TokenMetadataInfo

        let mut result = HashMap::new();
        let now = Utc::now();

        // Placeholder implementation
        for token_mint in token_mints {
            let metadata_info = TokenMetadataInfo {
                mint_address: token_mint.clone(),
                name: Some(format!("Token {}", &token_mint[0..8])),
                symbol: Some(format!("TK{}", &token_mint[0..4])),
                uri: Some("https://example.com/metadata.json".to_string()),
                last_updated: now,
            };

            result.insert(token_mint.clone(), metadata_info);
        }

        Ok(result)
    }

    /// Fetch account data for pool service integration
    pub async fn fetch_account_data_for_pool_service(
        &self,
        account_queue: &mut Vec<crate::pool_discovery::AccountInfo>
    ) -> Result<usize, String> {
        if account_queue.is_empty() {
            return Ok(0);
        }

        let batch_size = std::cmp::min(100, account_queue.len());
        let accounts_to_fetch: Vec<crate::pool_discovery::AccountInfo> = 
            account_queue.drain(0..batch_size).collect();

        log(
            LogTag::Pool,
            "ACCOUNT_FETCH_START",
            &format!("Starting account data fetch for {} accounts", accounts_to_fetch.len())
        );

        // Group accounts by type for efficient fetching
        let mut pool_accounts = Vec::new();
        let mut vault_accounts = Vec::new();
        let mut other_accounts = Vec::new();

        for account in &accounts_to_fetch {
            match account.account_type.as_str() {
                "pool" => pool_accounts.push(account.address.clone()),
                "vault" => vault_accounts.push(account.address.clone()),
                _ => other_accounts.push(account.address.clone()),
            }
        }

        let mut fetched_count = 0;

        // Fetch pool account data
        if !pool_accounts.is_empty() {
            match self.fetch_account_data(&pool_accounts).await {
                Ok(_account_data) => {
                    fetched_count += pool_accounts.len();
                }
                Err(e) => {
                    log(
                        LogTag::Pool,
                        "ACCOUNT_FETCH_ERROR",
                        &format!("Failed to fetch pool account data: {}", e)
                    );
                }
            }
        }

        // Fetch vault account data
        if !vault_accounts.is_empty() {
            match self.fetch_account_data(&vault_accounts).await {
                Ok(_account_data) => {
                    fetched_count += vault_accounts.len();
                }
                Err(e) => {
                    log(
                        LogTag::Pool,
                        "ACCOUNT_FETCH_ERROR",
                        &format!("Failed to fetch vault account data: {}", e)
                    );
                }
            }
        }

        // Fetch other account data
        if !other_accounts.is_empty() {
            match self.fetch_account_data(&other_accounts).await {
                Ok(_account_data) => {
                    fetched_count += other_accounts.len();
                }
                Err(e) => {
                    log(
                        LogTag::Pool,
                        "ACCOUNT_FETCH_ERROR",
                        &format!("Failed to fetch other account data: {}", e)
                    );
                }
            }
        }

        log(
            LogTag::Pool,
            "ACCOUNT_FETCH_COMPLETE",
            &format!("Account data fetch completed: {} accounts fetched", fetched_count)
        );

        Ok(fetched_count)
    }

    /// Fetch token account data for pool service integration
    pub async fn fetch_token_accounts_for_pool_service(
        &self,
        tracked_tokens: &[String]
    ) -> Result<usize, String> {
        if tracked_tokens.is_empty() {
            return Ok(0);
        }

        // Fetch token accounts, mints, and metadata
        let token_accounts = self.fetch_token_accounts(tracked_tokens).await?;
        let token_mints = self.fetch_token_mints(tracked_tokens).await?;
        let token_metadata = self.fetch_token_metadata(tracked_tokens).await?;

        let fetched_count = token_accounts.len() + token_mints.len() + token_metadata.len();

        log(
            LogTag::Pool,
            "TOKEN_FETCH_SUCCESS",
            &format!(
                "Fetched {} token accounts, {} mints, {} metadata for {} tokens",
                token_accounts.len(),
                token_mints.len(),
                token_metadata.len(),
                tracked_tokens.len()
            )
        );

        Ok(fetched_count)
    }
}

// =============================================================================
// GLOBAL INSTANCE
// =============================================================================

use std::sync::OnceLock;

static POOL_FETCHER: OnceLock<PoolFetcher> = OnceLock::new();

/// Initialize the global pool fetcher instance
pub fn init_pool_fetcher() -> &'static PoolFetcher {
    POOL_FETCHER.get_or_init(|| {
        log(LogTag::Pool, "INIT", "🏗️ Initializing Pool Fetcher");
        PoolFetcher::new()
    })
}

/// Get the global pool fetcher instance
pub fn get_pool_fetcher() -> &'static PoolFetcher {
    POOL_FETCHER.get().expect("Pool fetcher not initialized")
}

// =============================================================================
// CONVENIENCE FUNCTIONS
// =============================================================================

/// Fetch account data for pool service (convenience function)
pub async fn fetch_account_data_for_pool_service(
    account_queue: &mut Vec<crate::pool_discovery::AccountInfo>
) -> Result<usize, String> {
    get_pool_fetcher().fetch_account_data_for_pool_service(account_queue).await
}

/// Fetch token account data for pool service (convenience function)
pub async fn fetch_token_accounts_for_pool_service(
    tracked_tokens: &[String]
) -> Result<usize, String> {
    get_pool_fetcher().fetch_token_accounts_for_pool_service(tracked_tokens).await
}
