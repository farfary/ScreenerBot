/// Comprehensive pool cache system
/// Central data storage for all pool-related data: accounts, pools, prices, tokens, analysis

use crate::pools::types::{ PriceResult, PoolInfo };
use crate::pools::tokens::PoolToken;
use crate::pools::analyzer::{ TokenAvailability, AnalysisStats };
use tokio::sync::RwLock;
use std::sync::Arc;
use std::collections::HashMap;
use chrono::{ DateTime, Utc };
use crate::logger::{ log, LogTag };

/// Unified account data structure
/// Used everywhere in pools module instead of SharedAccountData/CachedAccountData
#[derive(Debug, Clone)]
pub struct AccountData {
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

impl AccountData {
    /// Create new account data
    pub fn new(address: String, data: Vec<u8>, lamports: u64, owner: String) -> Self {
        Self {
            address,
            data,
            lamports,
            owner,
            fetched_at: Utc::now(),
            exists: true,
        }
    }

    /// Create account data for non-existent account
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

    /// Check if account data is expired (10 minutes TTL)
    pub fn is_expired(&self) -> bool {
        let now = Utc::now();
        let age = now.signed_duration_since(self.fetched_at);
        age.num_seconds() > 600 // 10 minutes
    }

    /// Check if account data is fresh (not expired)
    pub fn is_fresh(&self) -> bool {
        !self.is_expired()
    }
}

/// Price history entry
#[derive(Debug, Clone)]
pub struct PriceHistoryEntry {
    pub timestamp: DateTime<Utc>,
    pub price_sol: f64,
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub accounts_count: usize,
    pub pools_count: usize,
    pub prices_count: usize,
    pub tokens_count: usize,
    pub availability_count: usize,
    pub price_history_entries: usize,
    pub updated_at: DateTime<Utc>,
}

/// Comprehensive pool cache
/// Central storage for all pool-related data
pub struct PoolCache {
    /// Account data storage (address -> AccountData)
    accounts: Arc<RwLock<HashMap<String, AccountData>>>,

    /// Pool data storage (token_mint -> Vec<PoolInfo>)
    pools: Arc<RwLock<HashMap<String, Vec<PoolInfo>>>>,

    /// Price data storage (token_mint -> PriceResult)
    prices: Arc<RwLock<HashMap<String, PriceResult>>>,

    /// Price history storage (token_mint -> Vec<PriceHistoryEntry>)
    price_history: Arc<RwLock<HashMap<String, Vec<PriceHistoryEntry>>>>,

    /// Token storage (tokens list)
    tokens: Arc<RwLock<Vec<PoolToken>>>,

    /// Token availability storage (token_mint -> TokenAvailability)
    token_availability: Arc<RwLock<HashMap<String, TokenAvailability>>>,

    /// Required accounts for RPC fetching
    required_accounts: Arc<RwLock<Vec<String>>>,
}

impl PoolCache {
    pub fn new() -> Self {
        Self {
            accounts: Arc::new(RwLock::new(HashMap::new())),
            pools: Arc::new(RwLock::new(HashMap::new())),
            prices: Arc::new(RwLock::new(HashMap::new())),
            price_history: Arc::new(RwLock::new(HashMap::new())),
            tokens: Arc::new(RwLock::new(Vec::new())),
            token_availability: Arc::new(RwLock::new(HashMap::new())),
            required_accounts: Arc::new(RwLock::new(Vec::new())),
        }
    }

    // =============================================================================
    // ACCOUNT DATA METHODS
    // =============================================================================

    /// Store account data
    pub async fn store_account(
        &self,
        address: String,
        data: Vec<u8>,
        lamports: u64,
        owner: String
    ) {
        let account_data = AccountData::new(address.clone(), data, lamports, owner);
        let mut accounts = self.accounts.write().await;
        accounts.insert(address, account_data);
    }

    /// Store non-existent account
    pub async fn store_non_existent_account(&self, address: String) {
        let account_data = AccountData::non_existent(address.clone());
        let mut accounts = self.accounts.write().await;
        accounts.insert(address, account_data);
    }

    /// Get account data
    pub async fn get_account(&self, address: &str) -> Option<AccountData> {
        let accounts = self.accounts.read().await;
        accounts
            .get(address)
            .filter(|acc| acc.is_fresh())
            .cloned()
    }

    /// Get multiple account data
    pub async fn get_multiple_accounts(
        &self,
        addresses: &[String]
    ) -> HashMap<String, AccountData> {
        let accounts = self.accounts.read().await;
        addresses
            .iter()
            .filter_map(|addr| {
                accounts
                    .get(addr)
                    .filter(|acc| acc.is_fresh())
                    .map(|acc| (addr.clone(), acc.clone()))
            })
            .collect()
    }

    /// Get all fresh account data
    pub async fn get_all_accounts(&self) -> HashMap<String, AccountData> {
        let accounts = self.accounts.read().await;
        accounts
            .iter()
            .filter(|(_, acc)| acc.is_fresh())
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }

    /// Check if account exists and is fresh
    pub async fn has_fresh_account(&self, address: &str) -> bool {
        let accounts = self.accounts.read().await;
        accounts
            .get(address)
            .map(|acc| acc.is_fresh())
            .unwrap_or(false)
    }

    /// Remove expired accounts
    pub async fn clean_expired_accounts(&self) -> usize {
        let mut accounts = self.accounts.write().await;
        let initial_count = accounts.len();
        accounts.retain(|_, acc| acc.is_fresh());
        let cleaned = initial_count - accounts.len();

        if cleaned > 0 {
            log(
                LogTag::Pool,
                "CACHE_CLEANUP_ACCOUNTS",
                &format!("🧹 Cleaned {} expired accounts", cleaned)
            );
        }

        cleaned
    }

    // =============================================================================
    // POOL DATA METHODS
    // =============================================================================

    /// Store pools for a token
    pub async fn store_pools(&self, token_mint: &str, pools: Vec<PoolInfo>) {
        let mut pools_map = self.pools.write().await;
        pools_map.insert(token_mint.to_string(), pools);

        log(
            LogTag::Pool,
            "CACHE_STORE_POOLS",
            &format!("📦 Stored pools for token {}", &token_mint[..8])
        );
    }

    /// Get pools for a token
    pub async fn get_pools(&self, token_mint: &str) -> Option<Vec<PoolInfo>> {
        let pools_map = self.pools.read().await;
        pools_map.get(token_mint).cloned()
    }

    /// Get all tokens with pools
    pub async fn get_tokens_with_pools(&self) -> Vec<String> {
        let pools_map = self.pools.read().await;
        pools_map.keys().cloned().collect()
    }

    /// Remove pools for a token
    pub async fn remove_pools(&self, token_mint: &str) {
        let mut pools_map = self.pools.write().await;
        pools_map.remove(token_mint);
    }

    // =============================================================================
    // PRICE DATA METHODS
    // =============================================================================

    /// Store price result
    pub async fn store_price(&self, token_mint: &str, price_result: PriceResult) {
        // Store current price
        {
            let mut prices = self.prices.write().await;
            prices.insert(token_mint.to_string(), price_result.clone());
        }

        // Add to price history
        self.add_price_to_history(token_mint, price_result.price_sol).await;

        log(
            LogTag::Pool,
            "CACHE_STORE_PRICE",
            &format!(
                "💰 Stored price for token {}: {} SOL",
                &token_mint[..8],
                price_result.price_sol
            )
        );
    }

    /// Get cached price
    pub async fn get_price(&self, token_mint: &str) -> Option<PriceResult> {
        let prices = self.prices.read().await;
        prices.get(token_mint).cloned()
    }

    /// Add price to history
    pub async fn add_price_to_history(&self, token_mint: &str, price_sol: f64) {
        let mut history = self.price_history.write().await;
        let entry = PriceHistoryEntry {
            timestamp: Utc::now(),
            price_sol,
        };

        history.entry(token_mint.to_string()).or_insert_with(Vec::new).push(entry);

        // Keep only last 1000 entries per token
        if let Some(token_history) = history.get_mut(token_mint) {
            if token_history.len() > 1000 {
                token_history.remove(0);
            }
        }
    }

    /// Get price history for a token
    pub async fn get_price_history(&self, token_mint: &str) -> Vec<(DateTime<Utc>, f64)> {
        let history = self.price_history.read().await;
        history
            .get(token_mint)
            .map(|entries|
                entries
                    .iter()
                    .map(|e| (e.timestamp, e.price_sol))
                    .collect()
            )
            .unwrap_or_default()
    }

    /// Get price history since a specific time
    pub async fn get_price_history_since(
        &self,
        token_mint: &str,
        since: DateTime<Utc>
    ) -> Vec<(DateTime<Utc>, f64)> {
        let history = self.price_history.read().await;
        history
            .get(token_mint)
            .map(|entries| {
                entries
                    .iter()
                    .filter(|e| e.timestamp >= since)
                    .map(|e| (e.timestamp, e.price_sol))
                    .collect()
            })
            .unwrap_or_default()
    }

    // =============================================================================
    // TOKEN DATA METHODS
    // =============================================================================

    /// Store tokens list
    pub async fn store_tokens(&self, tokens: Vec<PoolToken>) {
        let mut tokens_storage = self.tokens.write().await;
        *tokens_storage = tokens.clone();

        log(LogTag::Pool, "CACHE_STORE_TOKENS", &format!("🪙 Stored {} tokens", tokens.len()));
    }

    /// Get all tokens
    pub async fn get_tokens(&self) -> Vec<PoolToken> {
        let tokens = self.tokens.read().await;
        tokens.clone()
    }

    /// Get token mints
    pub async fn get_token_mints(&self) -> Vec<String> {
        let tokens = self.tokens.read().await;
        tokens
            .iter()
            .map(|t| t.mint.clone())
            .collect()
    }

    // =============================================================================
    // TOKEN AVAILABILITY METHODS
    // =============================================================================

    /// Store token availability
    pub async fn store_token_availability(
        &self,
        token_mint: &str,
        availability: TokenAvailability
    ) {
        let mut avail_map = self.token_availability.write().await;
        avail_map.insert(token_mint.to_string(), availability);
    }

    /// Get token availability
    pub async fn get_token_availability(&self, token_mint: &str) -> Option<TokenAvailability> {
        let avail_map = self.token_availability.read().await;
        avail_map.get(token_mint).cloned()
    }

    /// Get all calculable tokens
    pub async fn get_calculable_tokens(&self) -> Vec<String> {
        let avail_map = self.token_availability.read().await;
        avail_map
            .values()
            .filter(|a| a.calculable)
            .map(|a| a.token_mint.clone())
            .collect()
    }

    /// Get calculable token availabilities
    pub async fn get_calculable_token_availabilities(&self) -> HashMap<String, TokenAvailability> {
        let avail_map = self.token_availability.read().await;
        avail_map
            .iter()
            .filter(|(_, a)| a.calculable)
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }

    /// Get trading ready tokens
    pub async fn get_trading_ready_tokens(&self) -> Vec<String> {
        let avail_map = self.token_availability.read().await;
        avail_map
            .values()
            .filter(|a| a.is_ready_for_trading())
            .map(|a| a.token_mint.clone())
            .collect()
    }

    // =============================================================================
    // REQUIRED ACCOUNTS METHODS
    // =============================================================================

    /// Store required accounts for RPC fetching
    pub async fn store_required_accounts(&self, accounts: Vec<String>) {
        let mut req_accounts = self.required_accounts.write().await;
        *req_accounts = accounts.clone();

        log(
            LogTag::Pool,
            "CACHE_STORE_REQUIRED",
            &format!("📋 Stored {} required accounts", accounts.len())
        );
    }

    /// Get required accounts
    pub async fn get_required_accounts(&self) -> Vec<String> {
        let req_accounts = self.required_accounts.read().await;
        req_accounts.clone()
    }

    /// Add required account
    pub async fn add_required_account(&self, address: String) {
        let mut req_accounts = self.required_accounts.write().await;
        if !req_accounts.contains(&address) {
            req_accounts.push(address);
        }
    }

    // =============================================================================
    // STATISTICS AND MAINTENANCE
    // =============================================================================

    /// Get cache statistics
    pub async fn get_stats(&self) -> CacheStats {
        let accounts = self.accounts.read().await;
        let pools = self.pools.read().await;
        let prices = self.prices.read().await;
        let tokens = self.tokens.read().await;
        let availability = self.token_availability.read().await;
        let history = self.price_history.read().await;

        let price_history_entries = history
            .values()
            .map(|h| h.len())
            .sum();

        CacheStats {
            accounts_count: accounts.len(),
            pools_count: pools.len(),
            prices_count: prices.len(),
            tokens_count: tokens.len(),
            availability_count: availability.len(),
            price_history_entries,
            updated_at: Utc::now(),
        }
    }

    /// Clean all expired data
    pub async fn cleanup_expired(&self) -> (usize, usize) {
        let cleaned_accounts = self.clean_expired_accounts().await;

        // Clean old price history (keep only last 24 hours per token)
        let mut history = self.price_history.write().await;
        let cutoff = Utc::now() - chrono::Duration::hours(24);
        let mut cleaned_history = 0;

        for token_history in history.values_mut() {
            let initial_len = token_history.len();
            token_history.retain(|entry| entry.timestamp >= cutoff);
            cleaned_history += initial_len - token_history.len();
        }

        if cleaned_history > 0 {
            log(
                LogTag::Pool,
                "CACHE_CLEANUP_HISTORY",
                &format!("🧹 Cleaned {} old price history entries", cleaned_history)
            );
        }

        (cleaned_accounts, cleaned_history)
    }

    /// Clear all data (for testing/reset)
    pub async fn clear_all(&self) {
        let mut accounts = self.accounts.write().await;
        let mut pools = self.pools.write().await;
        let mut prices = self.prices.write().await;
        let mut history = self.price_history.write().await;
        let mut tokens = self.tokens.write().await;
        let mut availability = self.token_availability.write().await;
        let mut required = self.required_accounts.write().await;

        accounts.clear();
        pools.clear();
        prices.clear();
        history.clear();
        tokens.clear();
        availability.clear();
        required.clear();

        log(LogTag::Pool, "CACHE_CLEARED", "🗑️ Cleared all cache data");
    }

    /// Get memory usage estimate (rough)
    pub async fn get_memory_usage_mb(&self) -> f64 {
        let stats = self.get_stats().await;

        // Rough estimates
        let accounts_mb = (stats.accounts_count as f64) * 0.001; // ~1KB per account
        let pools_mb = (stats.pools_count as f64) * 0.002; // ~2KB per pool
        let prices_mb = (stats.prices_count as f64) * 0.0005; // ~0.5KB per price
        let history_mb = (stats.price_history_entries as f64) * 0.0001; // ~0.1KB per entry

        accounts_mb + pools_mb + prices_mb + history_mb
    }

    /// Set token as calculable (mark token as ready for price calculation)
    pub async fn set_token_calculable(
        &self,
        token_mint: &str,
        calculable: bool,
        best_pool: Option<PoolInfo>
    ) {
        let availability = if calculable {
            if let Some(pool) = best_pool {
                TokenAvailability {
                    token_mint: token_mint.to_string(),
                    calculable: true,
                    best_pool: Some(pool),
                    pools: Vec::new(),
                    reserve_accounts: Vec::new(),
                    liquidity_usd: 0.0,
                    sol_reserves: 0.0,
                    analyzed_at: Utc::now(),
                    errors: Vec::new(),
                }
            } else {
                TokenAvailability {
                    token_mint: token_mint.to_string(),
                    calculable: false,
                    best_pool: None,
                    pools: Vec::new(),
                    reserve_accounts: Vec::new(),
                    liquidity_usd: 0.0,
                    sol_reserves: 0.0,
                    analyzed_at: Utc::now(),
                    errors: vec!["No best pool available".to_string()],
                }
            }
        } else {
            TokenAvailability {
                token_mint: token_mint.to_string(),
                calculable: false,
                best_pool: None,
                pools: Vec::new(),
                reserve_accounts: Vec::new(),
                liquidity_usd: 0.0,
                sol_reserves: 0.0,
                analyzed_at: Utc::now(),
                errors: Vec::new(),
            }
        };

        self.store_token_availability(token_mint, availability).await;
    }

    /// Add multiple required accounts
    pub async fn add_required_accounts(&self, addresses: &[String]) {
        let mut req_accounts = self.required_accounts.write().await;
        for address in addresses {
            if !req_accounts.contains(address) {
                req_accounts.push(address.clone());
            }
        }
    }

    /// Get all token availabilities
    pub async fn get_all_token_availability(&self) -> HashMap<String, TokenAvailability> {
        let availability_map = self.token_availability.read().await;
        availability_map.clone()
    }

    /// Get tokens without pools (tokens that need discovery)
    pub async fn get_tokens_without_pools(&self) -> Vec<String> {
        let tokens = self.get_tokens().await;
        let pools_map = self.pools.read().await;

        tokens
            .into_iter()
            .filter(|token| !pools_map.contains_key(&token.mint))
            .map(|token| token.mint)
            .collect()
    }

    /// Mark token discovery as in progress
    pub async fn mark_in_progress(&self, _token_mint: &str) -> bool {
        // For now, always allow processing (can add actual state tracking later)
        true
    }

    /// Mark token discovery as completed
    pub async fn mark_completed(&self, _token_mint: &str) {
        // For now, no-op (can add actual state tracking later)
    }

    /// Cache pools for a token (alias for store_pools)
    pub async fn cache_pools(&self, token_mint: &str, pools: Vec<PoolInfo>) {
        self.store_pools(token_mint, pools).await;
    }

    /// Get cached pools for a token (alias for get_pools)
    pub async fn get_cached_pools(&self, token_mint: &str) -> Option<Vec<PoolInfo>> {
        self.get_pools(token_mint).await
    }

    /// Cache price for a token (alias for store_price)
    pub async fn cache_price(&self, token_mint: &str, price_result: PriceResult) {
        self.store_price(token_mint, price_result).await;
    }

    /// Get cached price for a token (alias for get_price)
    pub async fn get_cached_price(&self, token_mint: &str) -> Option<PriceResult> {
        self.get_price(token_mint).await
    }

    /// Cache account data (alias for store_account)
    pub async fn cache_account_data(&self, address: &str, data: Vec<u8>) {
        self.store_account(address.to_string(), data, 0, String::new()).await;
    }

    /// Get cached account data (alias for get_account that returns Vec<u8>)
    pub async fn get_cached_account_data(&self, address: &str) -> Option<Vec<u8>> {
        if let Some(account) = self.get_account(address).await { Some(account.data) } else { None }
    }

    /// Remove account from cache
    pub async fn remove_account(&self, address: &str) {
        let mut accounts = self.accounts.write().await;
        accounts.remove(address);
    }

    /// Get analysis statistics
    pub async fn get_analysis_stats(&self) -> AnalysisStats {
        let availability_map = self.token_availability.read().await;
        let total_tokens = availability_map.len();
        let calculable_tokens = availability_map
            .values()
            .filter(|a| a.calculable)
            .count();
        let trading_ready_tokens = calculable_tokens; // Simplified
        let error_tokens = availability_map
            .values()
            .filter(|a| !a.errors.is_empty())
            .count();
        let required_accounts = self.get_required_accounts().await.len();

        AnalysisStats {
            total_tokens,
            calculable_tokens,
            trading_ready_tokens,
            error_tokens,
            required_accounts,
            updated_at: Utc::now(),
        }
    }
}

impl Default for PoolCache {
    fn default() -> Self {
        Self::new()
    }
}
