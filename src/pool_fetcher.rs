use crate::logger::{ log, LogTag };
use crate::rpc::get_rpc_client;
use chrono::{ DateTime, Utc };
use solana_sdk::{ account::Account, pubkey::Pubkey };
use std::collections::HashMap;
use std::collections::HashSet;
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

/// Pool data with associated vault account data
#[derive(Debug, Clone)]
pub struct PoolDataWithVaults {
    pub pool_address: String,
    pub pool_type: crate::pool_decoders::PoolType,
    pub pool_data: Vec<u8>,
    pub vault_data: Vec<Option<Vec<u8>>>, // Vault account data in order
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
            stats.average_fetch_time_ms =
                (stats.average_fetch_time_ms + (start_time.elapsed().as_millis() as f64)) / 2.0;
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
            stats.average_fetch_time_ms =
                (stats.average_fetch_time_ms + (start_time.elapsed().as_millis() as f64)) / 2.0;
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
            stats.average_fetch_time_ms =
                (stats.average_fetch_time_ms + (start_time.elapsed().as_millis() as f64)) / 2.0;
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
            .map(|addr|
                Pubkey::from_str(addr).map_err(|e| format!("Invalid address {}: {}", addr, e))
            )
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
            stats.average_fetch_time_ms =
                (stats.average_fetch_time_ms + (start_time.elapsed().as_millis() as f64)) / 2.0;
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

    /// Fetch pool data with vault accounts in batches (OPTIMIZED for reduced RPC calls)
    /// This method fetches pool accounts first, extracts vault addresses, then batches vault fetches
    pub async fn fetch_pools_with_vaults(
        &self,
        pool_addresses: &[String]
    ) -> Result<HashMap<String, PoolDataWithVaults>, String> {
        if pool_addresses.is_empty() {
            return Ok(HashMap::new());
        }

        let start_time = Instant::now();
        let mut result = HashMap::new();

        log(
            LogTag::Pool,
            "BATCH_POOL_VAULT_FETCH",
            &format!("🔍 Batch fetching {} pools with vault accounts", pool_addresses.len())
        );

        // Phase 1: Fetch all pool account data
        let pool_account_data = self.fetch_account_data(pool_addresses).await?;

        // Phase 2: Extract vault addresses for pools that need them (Raydium CPMM-fast path)
        let mut vault_addresses_to_fetch: Vec<String> = Vec::new();
        let mut pool_vault_mapping: HashMap<String, (usize, usize)> = HashMap::new(); // pool_address -> (start,end)

        for (pool_address, pool_data) in &pool_account_data {
            // Try Raydium CPMM layout to extract two vaults; fallback to none on error
            match extract_raydium_cpmm_vaults(pool_data) {
                Ok(vaults) if !vaults.is_empty() => {
                    let start_idx = vault_addresses_to_fetch.len();
                    vault_addresses_to_fetch.extend(vaults);
                    let end_idx = vault_addresses_to_fetch.len();
                    pool_vault_mapping.insert(pool_address.clone(), (start_idx, end_idx));
                }
                Ok(_) => {
                    // no-op
                }
                Err(e) => {
                    log(
                        LogTag::Pool,
                        "VAULT_EXTRACT_WARN",
                        &format!(
                            "{} vault extract failed: {}",
                            crate::utils::safe_truncate(pool_address, 8),
                            e
                        )
                    );
                }
            }
        }

        // Phase 3: Batch fetch all vault account data
        let vault_account_data = if !vault_addresses_to_fetch.is_empty() {
            log(
                LogTag::Pool,
                "VAULT_BATCH_FETCH",
                &format!("Fetching {} vault accounts in batch", vault_addresses_to_fetch.len())
            );
            self.fetch_account_data(&vault_addresses_to_fetch).await?
        } else {
            HashMap::new()
        };

        // Phase 4: Combine pool and vault data
        for (pool_address, pool_data) in pool_account_data {
            // Default to RaydiumCpmm type for now; expand when decoders are wired
            let pool_type = self.detect_pool_type_from_data(&pool_data)?;

            // Get vault data for this pool if available
            let vault_data = if
                let Some((start_idx, end_idx)) = pool_vault_mapping.get(&pool_address)
            {
                let mut vaults = Vec::new();
                for i in *start_idx..*end_idx {
                    if let Some(vault_addr) = vault_addresses_to_fetch.get(i) {
                        let vault_account = vault_account_data.get(vault_addr).cloned();
                        vaults.push(vault_account);
                    }
                }
                vaults
            } else {
                Vec::new()
            };

            result.insert(pool_address.clone(), PoolDataWithVaults {
                pool_address: pool_address.clone(),
                pool_type,
                pool_data,
                vault_data,
            });
        }

        log(
            LogTag::Pool,
            "BATCH_POOL_VAULT_SUCCESS",
            &format!(
                "Successfully fetched {} pools with vaults in {:.2}ms",
                result.len(),
                start_time.elapsed().as_millis()
            )
        );

        Ok(result)
    }

    /// Detect pool type from account data (simplified - would need actual owner checking)
    fn detect_pool_type_from_data(
        &self,
        _data: &[u8]
    ) -> Result<crate::pool_decoders::PoolType, String> {
        // This is a placeholder - in reality you'd check the account owner program ID
        // For now, default to Raydium CPMM
        Ok(crate::pool_decoders::PoolType::RaydiumCpmm)
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

        // 1) Drain a bounded batch from the queue and dedupe by address
        let batch_size = std::cmp::min(300, account_queue.len());
        let mut accounts_to_fetch: Vec<crate::pool_discovery::AccountInfo> = account_queue
            .drain(0..batch_size)
            .collect();
        let mut seen: HashSet<String> = HashSet::new();
        accounts_to_fetch.retain(|a| seen.insert(a.address.clone()));

        log(
            LogTag::Pool,
            "ACCOUNT_FETCH_START",
            &format!("Starting account data fetch for {} accounts", accounts_to_fetch.len())
        );

        // 2) Separate pool accounts for first-phase fetch
        let mut pool_accounts: Vec<String> = Vec::new();
        let mut vault_accounts: Vec<String> = Vec::new();
        let mut other_accounts: Vec<String> = Vec::new();
        for account in &accounts_to_fetch {
            match account.account_type.as_str() {
                "pool" => pool_accounts.push(account.address.clone()),
                "vault" => vault_accounts.push(account.address.clone()),
                _ => other_accounts.push(account.address.clone()),
            }
        }

        let mut fetched_count = 0usize;

        // 3) Phase-1: fetch pool accounts to learn vaults
        let mut learned_vaults: Vec<String> = Vec::new();
        let mut pool_data_map: HashMap<String, Vec<u8>> = HashMap::new();
        if !pool_accounts.is_empty() {
            match self.fetch_account_data(&pool_accounts).await {
                Ok(account_data) => {
                    fetched_count += account_data.len();
                    for (addr, data) in &account_data {
                        // Try extracting Raydium CPMM vaults
                        if let Ok(vs) = extract_raydium_cpmm_vaults(data) {
                            for v in vs {
                                learned_vaults.push(v);
                            }
                        }
                        pool_data_map.insert(addr.clone(), data.clone());
                    }
                }
                Err(e) => {
                    log(LogTag::Pool, "ACCOUNT_FETCH_ERROR", &format!("Pools fetch failed: {}", e));
                }
            }
        }

        // 4) Add newly learned vaults to the queue for future cycles and also fetch them now
        if !learned_vaults.is_empty() {
            // De-dupe against already-planned vaults
            let current: HashSet<String> = vault_accounts.iter().cloned().collect();
            for v in learned_vaults.iter() {
                if !current.contains(v) {
                    account_queue.push(crate::pool_discovery::AccountInfo {
                        address: v.clone(),
                        account_type: "vault".to_string(),
                        token_mint: "".to_string(),
                        last_fetched: None,
                    });
                    vault_accounts.push(v.clone());
                }
            }
        }

        // 5) Phase-2: fetch vault + other accounts in one combined batch
        let mut phase2_batch: Vec<String> = Vec::new();
        phase2_batch.extend(vault_accounts);
        phase2_batch.extend(other_accounts);
        if !phase2_batch.is_empty() {
            match self.fetch_account_data(&phase2_batch).await {
                Ok(account_data) => {
                    fetched_count += account_data.len();
                }
                Err(e) => {
                    log(
                        LogTag::Pool,
                        "ACCOUNT_FETCH_ERROR",
                        &format!("Phase2 fetch failed: {}", e)
                    );
                }
            }
        }

        log(
            LogTag::Pool,
            "ACCOUNT_FETCH_COMPLETE",
            &format!("Fetched {} accounts (phase1 pools + phase2 vaults/others)", fetched_count)
        );

        Ok(fetched_count)
    }

    /// Fetch account data for pool service and return the fetched map (address -> data)
    /// Implements two-phase fetching: pools first (to learn vaults), then vaults/others
    pub async fn fetch_and_collect_account_data_for_pool_service(
        &self,
        account_queue: &mut Vec<crate::pool_discovery::AccountInfo>
    ) -> Result<HashMap<String, Vec<u8>>, String> {
        let mut out: HashMap<String, Vec<u8>> = HashMap::new();
        if account_queue.is_empty() {
            return Ok(out);
        }

        // Drain a bounded batch and dedupe by address
        let batch_size = std::cmp::min(300, account_queue.len());
        let mut accounts_to_fetch: Vec<crate::pool_discovery::AccountInfo> = account_queue
            .drain(0..batch_size)
            .collect();
        let mut seen: HashSet<String> = HashSet::new();
        accounts_to_fetch.retain(|a| seen.insert(a.address.clone()));

        // Separate by type
        let mut pool_accounts: Vec<String> = Vec::new();
        let mut vault_accounts: Vec<String> = Vec::new();
        let mut other_accounts: Vec<String> = Vec::new();
        for a in &accounts_to_fetch {
            match a.account_type.as_str() {
                "pool" => pool_accounts.push(a.address.clone()),
                "vault" => vault_accounts.push(a.address.clone()),
                _ => other_accounts.push(a.address.clone()),
            }
        }

        // Phase-1: pools
        let mut learned_vaults: Vec<String> = Vec::new();
        if !pool_accounts.is_empty() {
            if let Ok(account_data) = self.fetch_account_data(&pool_accounts).await {
                for (addr, data) in &account_data {
                    out.insert(addr.clone(), data.clone());
                    if let Ok(vs) = extract_raydium_cpmm_vaults(data) {
                        learned_vaults.extend(vs);
                    }
                }
            }
        }

        // Add learned vaults to queue for future cycles and include in this fetch
        if !learned_vaults.is_empty() {
            let existing: HashSet<String> = vault_accounts.iter().cloned().collect();
            for v in learned_vaults.into_iter() {
                if !existing.contains(&v) {
                    account_queue.push(crate::pool_discovery::AccountInfo {
                        address: v.clone(),
                        account_type: "vault".to_string(),
                        token_mint: "".to_string(),
                        last_fetched: None,
                    });
                    vault_accounts.push(v);
                }
            }
        }

        // Phase-2: vaults + others
        let mut phase2: Vec<String> = Vec::new();
        phase2.extend(vault_accounts);
        phase2.extend(other_accounts);
        if !phase2.is_empty() {
            if let Ok(account_data) = self.fetch_account_data(&phase2).await {
                for (addr, data) in account_data.into_iter() {
                    out.insert(addr, data);
                }
            }
        }

        Ok(out)
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

/// Fetch pools with vaults optimized (convenience function)
pub async fn fetch_pools_with_vaults_optimized(
    pool_addresses: &[String]
) -> Result<HashMap<String, PoolDataWithVaults>, String> {
    get_pool_fetcher().fetch_pools_with_vaults(pool_addresses).await
}

// =============================================================================
// LOCAL HELPERS
// =============================================================================

/// Try to extract Raydium CPMM token vault addresses from pool account data.
/// Layout (approx):
///  - 8 bytes: discriminator
///  - 32 bytes: amm_config
///  - 32 bytes: pool_creator
///  - 32 bytes: token_0_vault
///  - 32 bytes: token_1_vault
///  - ... (rest not needed for vault extraction)
fn extract_raydium_cpmm_vaults(data: &[u8]) -> Result<Vec<String>, String> {
    // Need at least up to token_1_vault end offset
    const DISC: usize = 8;
    const AMM_CFG: usize = 32;
    const CREATOR: usize = 32;
    const VAULT: usize = 32;
    let token0_start = DISC + AMM_CFG + CREATOR; // 72
    let token1_start = token0_start + VAULT; // 104
    let token1_end = token1_start + VAULT; // 136

    if data.len() < token1_end {
        return Err("pool data too short for Raydium CPMM layout".to_string());
    }

    let t0_bytes: [u8; 32] = data[token0_start..token0_start + 32]
        .try_into()
        .map_err(|_| "invalid token_0_vault slice")?;
    let t1_bytes: [u8; 32] = data[token1_start..token1_start + 32]
        .try_into()
        .map_err(|_| "invalid token_1_vault slice")?;

    let t0 = Pubkey::new_from_array(t0_bytes).to_string();
    let t1 = Pubkey::new_from_array(t1_bytes).to_string();

    Ok(vec![t0, t1])
}
