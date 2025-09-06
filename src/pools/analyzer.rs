/// Pool analyzer module
/// Analyzes tokens and pools to determine calculability, extract reserve accounts, and maintain availability lists

use crate::pools::types::{ PoolInfo, PriceResult };
use crate::pools::tokens::PoolToken;
use crate::pools::calculator::PoolCalculator;
use crate::pools::cache::PoolCache;
use crate::pools::constants::{ MIN_POOL_LIQUIDITY_USD, SOL_MINT };
use tokio::sync::RwLock;
use std::sync::Arc;
use std::collections::HashMap;
use chrono::{ DateTime, Utc };
use crate::logger::{ log, LogTag };
use solana_sdk::pubkey::Pubkey;

/// Token availability status
#[derive(Debug, Clone)]
pub struct TokenAvailability {
    /// Token mint address
    pub token_mint: String,
    /// Whether the token has calculable pools
    pub calculable: bool,
    /// Best pool for price calculation
    pub best_pool: Option<PoolInfo>,
    /// All discovered pools for this token
    pub pools: Vec<PoolInfo>,
    /// Reserve account addresses for RPC fetching
    pub reserve_accounts: Vec<String>,
    /// Liquidity in USD from best pool
    pub liquidity_usd: f64,
    /// SOL reserves from best pool
    pub sol_reserves: f64,
    /// Last analysis time
    pub analyzed_at: DateTime<Utc>,
    /// Analysis errors (if any)
    pub errors: Vec<String>,
}

impl TokenAvailability {
    /// Create new token availability
    pub fn new(token_mint: String) -> Self {
        Self {
            token_mint,
            calculable: false,
            best_pool: None,
            pools: Vec::new(),
            reserve_accounts: Vec::new(),
            liquidity_usd: 0.0,
            sol_reserves: 0.0,
            analyzed_at: Utc::now(),
            errors: Vec::new(),
        }
    }

    /// Mark as calculable with best pool
    pub fn with_best_pool(mut self, pool: PoolInfo) -> Self {
        self.liquidity_usd = pool.liquidity_usd.unwrap_or(0.0);
        self.sol_reserves = pool.sol_reserves;
        self.best_pool = Some(pool);
        self.calculable = true;
        self.analyzed_at = Utc::now();
        self
    }

    /// Add error
    pub fn with_error(mut self, error: String) -> Self {
        self.errors.push(error);
        self.analyzed_at = Utc::now();
        self
    }

    /// Check if token is ready for trading
    pub fn is_ready_for_trading(&self) -> bool {
        self.calculable &&
            self.best_pool.is_some() &&
            self.sol_reserves > 0.0 &&
            self.liquidity_usd >= MIN_POOL_LIQUIDITY_USD
    }
}

/// Pool analyzer service
pub struct PoolAnalyzer {
    calculator: PoolCalculator,
    cache: Arc<PoolCache>,
    /// Token availability map: token_mint -> TokenAvailability
    availability: Arc<RwLock<HashMap<String, TokenAvailability>>>,
    /// Account addresses that need RPC data
    required_accounts: Arc<RwLock<Vec<String>>>,
}

impl PoolAnalyzer {
    pub fn new(cache: Arc<PoolCache>) -> Self {
        Self {
            calculator: PoolCalculator::new(),
            cache,
            availability: Arc::new(RwLock::new(HashMap::new())),
            required_accounts: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Analyze all tokens and their pools
    pub async fn analyze_all_tokens(&self, tokens: &[PoolToken]) -> Result<(), String> {
        log(
            LogTag::Pool,
            "ANALYZER_START",
            &format!("🔬 Starting analysis of {} tokens", tokens.len())
        );

        let mut analyzed_count = 0;
        let mut calculable_count = 0;
        let mut error_count = 0;

        for token in tokens {
            match self.analyze_token(&token.mint).await {
                Ok(availability) => {
                    if availability.calculable {
                        calculable_count += 1;
                    }

                    // Store in availability map
                    {
                        let mut avail_map = self.availability.write().await;
                        avail_map.insert(token.mint.clone(), availability);
                    }

                    analyzed_count += 1;
                }
                Err(e) => {
                    error_count += 1;
                    log(
                        LogTag::Pool,
                        "ANALYZER_TOKEN_ERROR",
                        &format!("❌ Failed to analyze {}: {}", &token.mint[..8], e)
                    );

                    // Store error result
                    let availability = TokenAvailability::new(token.mint.clone()).with_error(e);

                    let mut avail_map = self.availability.write().await;
                    avail_map.insert(token.mint.clone(), availability);
                }
            }
        }

        // Update required accounts list
        self.update_required_accounts().await;

        log(
            LogTag::Pool,
            "ANALYZER_COMPLETE",
            &format!(
                "✅ Analysis complete: {}/{} tokens analyzed, {} calculable, {} errors",
                analyzed_count,
                tokens.len(),
                calculable_count,
                error_count
            )
        );

        Ok(())
    }

    /// Analyze a single token
    pub async fn analyze_token(&self, token_mint: &str) -> Result<TokenAvailability, String> {
        let mut availability = TokenAvailability::new(token_mint.to_string());

        // Get all pools for this token
        let pools = match self.cache.get_cached_pools(token_mint).await {
            Some(pools) => pools,
            None => {
                return Ok(availability.with_error("No pools found in cache".to_string()));
            }
        };

        availability.pools = pools.clone();

        // Filter pools that can be calculated
        let mut calculable_pools = Vec::new();
        let mut pool_errors = Vec::new();

        for pool in &pools {
            match self.validate_pool_calculability(pool).await {
                Ok(true) => calculable_pools.push(pool.clone()),
                Ok(false) => {
                    pool_errors.push(format!("Pool {} not calculable", &pool.pool_address[..8]));
                }
                Err(e) => {
                    pool_errors.push(
                        format!("Pool {} validation error: {}", &pool.pool_address[..8], e)
                    );
                }
            }
        }

        if calculable_pools.is_empty() {
            let error_msg = if pool_errors.is_empty() {
                "No calculable pools found".to_string()
            } else {
                format!("No calculable pools: {}", pool_errors.join(", "))
            };
            return Ok(availability.with_error(error_msg));
        }

        // Find best pool (highest liquidity)
        let best_pool = self.select_best_pool(&calculable_pools)?;

        // Extract reserve accounts
        let reserve_accounts = self.extract_reserve_accounts(&best_pool).await?;
        availability.reserve_accounts = reserve_accounts;

        // Test price calculation
        match self.calculator.calculate_price(&best_pool, token_mint).await {
            Ok(Some(_)) => {
                log(
                    LogTag::Pool,
                    "ANALYZER_SUCCESS",
                    &format!(
                        "✅ Token {} analyzable via pool {}",
                        &token_mint[..8],
                        &best_pool.pool_address[..8]
                    )
                );
                Ok(availability.with_best_pool(best_pool))
            }
            Ok(None) => {
                Ok(availability.with_error("Price calculation returned None".to_string()))
            }
            Err(e) => { Ok(availability.with_error(format!("Price calculation failed: {}", e))) }
        }
    }

    /// Validate if a pool can be used for price calculation
    async fn validate_pool_calculability(&self, pool: &PoolInfo) -> Result<bool, String> {
        // Check if pool has SOL pair
        let has_sol = pool.base_token_mint == SOL_MINT || pool.quote_token_mint == SOL_MINT;
        if !has_sol {
            return Ok(false);
        }

        // Check if reserves are available
        if pool.sol_reserves <= 0.0 || pool.token_reserves <= 0.0 {
            return Ok(false);
        }

        // Check minimum liquidity
        if let Some(liquidity) = pool.liquidity_usd {
            if liquidity < MIN_POOL_LIQUIDITY_USD {
                return Ok(false);
            }
        }

        // Check if we have required account data
        // TODO: Add RPC account data validation when needed

        Ok(true)
    }

    /// Select the best pool from calculable pools (highest liquidity)
    fn select_best_pool(&self, pools: &[PoolInfo]) -> Result<PoolInfo, String> {
        if pools.is_empty() {
            return Err("No pools provided".to_string());
        }

        let best_pool = pools
            .iter()
            .max_by(|a, b| {
                let a_liquidity = a.liquidity_usd.unwrap_or(0.0);
                let b_liquidity = b.liquidity_usd.unwrap_or(0.0);
                a_liquidity.partial_cmp(&b_liquidity).unwrap_or(std::cmp::Ordering::Equal)
            })
            .unwrap();

        Ok(best_pool.clone())
    }

    /// Extract reserve account addresses from pool
    async fn extract_reserve_accounts(&self, pool: &PoolInfo) -> Result<Vec<String>, String> {
        let mut accounts = Vec::new();

        // Add pool address itself
        accounts.push(pool.pool_address.clone());

        // Extract vault addresses based on pool type/program
        match pool.pool_program_id.as_str() {
            crate::pools::constants::METEORA_DAMM_V2_PROGRAM_ID => {
                // For Meteora DAMM v2, we need to decode the pool to get vault addresses
                if
                    let Some(vault_addresses) = self.extract_meteora_damm_v2_vaults(
                        &pool.pool_address
                    ).await?
                {
                    accounts.extend(vault_addresses);
                }
            }
            crate::pools::constants::RAYDIUM_CPMM_PROGRAM_ID => {
                // TODO: Add Raydium CPMM vault extraction
                log(LogTag::Pool, "ANALYZER_TODO", "TODO: Raydium CPMM vault extraction");
            }
            crate::pools::constants::RAYDIUM_LEGACY_AMM_PROGRAM_ID => {
                // TODO: Add Raydium Legacy AMM vault extraction
                log(LogTag::Pool, "ANALYZER_TODO", "TODO: Raydium Legacy AMM vault extraction");
            }
            _ => {
                log(
                    LogTag::Pool,
                    "ANALYZER_UNSUPPORTED",
                    &format!("Unsupported pool program: {}", pool.pool_program_id)
                );
            }
        }

        Ok(accounts)
    }

    /// Extract Meteora DAMM v2 vault addresses from pool data
    async fn extract_meteora_damm_v2_vaults(
        &self,
        pool_address: &str
    ) -> Result<Option<Vec<String>>, String> {
        // Get pool account data from cache if available
        if let Some(account_data) = self.cache.get_cached_account_data(pool_address).await {
            if account_data.len() >= 200 {
                // Light decode to extract vault addresses
                let mut offset = 136;

                // Skip token mints (64 bytes)
                offset += 64;

                // Read vault addresses
                if
                    let (Ok(vault_a), Ok(vault_b)) = (
                        Self::read_pubkey_at_offset(&account_data, &mut offset),
                        Self::read_pubkey_at_offset(&account_data, &mut offset),
                    )
                {
                    return Ok(Some(vec![vault_a, vault_b]));
                }
            }
        }

        // If we can't extract vault addresses, that's ok - the fetcher will fetch just the pool address
        log(
            LogTag::Pool,
            "ANALYZER_VAULT_SKIP",
            &format!("Could not extract vault addresses for pool {}", &pool_address[..8])
        );
        Ok(None)
    }

    /// Helper function to read pubkey at offset (lightweight version)
    fn read_pubkey_at_offset(data: &[u8], offset: &mut usize) -> Result<String, String> {
        if *offset + 32 > data.len() {
            return Err("Insufficient data for pubkey".to_string());
        }

        let pubkey_bytes = &data[*offset..*offset + 32];
        *offset += 32;

        let pubkey = solana_sdk::pubkey::Pubkey::new_from_array(
            pubkey_bytes.try_into().map_err(|_| "Failed to parse pubkey".to_string())?
        );

        Ok(pubkey.to_string())
    }

    /// Update the required accounts list for RPC fetching
    async fn update_required_accounts(&self) -> Result<(), String> {
        let availability_map = self.availability.read().await;
        let mut all_accounts = Vec::new();

        for availability in availability_map.values() {
            if availability.calculable {
                all_accounts.extend(availability.reserve_accounts.iter().cloned());
            }
        }

        // Remove duplicates
        all_accounts.sort();
        all_accounts.dedup();

        // Update required accounts
        {
            let mut required = self.required_accounts.write().await;
            *required = all_accounts;
        }

        log(
            LogTag::Pool,
            "ANALYZER_ACCOUNTS",
            &format!(
                "📋 Updated required accounts list: {} accounts",
                self.required_accounts.read().await.len()
            )
        );

        Ok(())
    }

    /// Get token availability
    pub async fn get_token_availability(&self, token_mint: &str) -> Option<TokenAvailability> {
        let availability_map = self.availability.read().await;
        availability_map.get(token_mint).cloned()
    }

    /// Get all calculable tokens
    pub async fn get_calculable_tokens(&self) -> Vec<String> {
        let availability_map = self.availability.read().await;
        availability_map
            .values()
            .filter(|a| a.calculable)
            .map(|a| a.token_mint.clone())
            .collect()
    }

    /// Get tokens ready for trading
    pub async fn get_trading_ready_tokens(&self) -> Vec<String> {
        let availability_map = self.availability.read().await;
        availability_map
            .values()
            .filter(|a| a.is_ready_for_trading())
            .map(|a| a.token_mint.clone())
            .collect()
    }

    /// Get required account addresses for RPC fetching
    pub async fn get_required_accounts(&self) -> Vec<String> {
        let required = self.required_accounts.read().await;
        required.clone()
    }

    /// Get analysis statistics
    pub async fn get_analysis_stats(&self) -> AnalysisStats {
        let availability_map = self.availability.read().await;
        let total_tokens = availability_map.len();
        let calculable_tokens = availability_map
            .values()
            .filter(|a| a.calculable)
            .count();
        let trading_ready_tokens = availability_map
            .values()
            .filter(|a| a.is_ready_for_trading())
            .count();
        let error_tokens = availability_map
            .values()
            .filter(|a| !a.errors.is_empty())
            .count();
        let required_accounts = self.required_accounts.read().await.len();

        AnalysisStats {
            total_tokens,
            calculable_tokens,
            trading_ready_tokens,
            error_tokens,
            required_accounts,
            updated_at: Utc::now(),
        }
    }

    /// Re-analyze a specific token (when pools are updated)
    pub async fn re_analyze_token(&self, token_mint: &str) -> Result<(), String> {
        match self.analyze_token(token_mint).await {
            Ok(availability) => {
                let mut avail_map = self.availability.write().await;
                avail_map.insert(token_mint.to_string(), availability);

                // Update required accounts
                drop(avail_map);
                self.update_required_accounts().await?;

                Ok(())
            }
            Err(e) => {
                log(
                    LogTag::Pool,
                    "ANALYZER_REANALYZE_ERROR",
                    &format!("❌ Failed to re-analyze {}: {}", &token_mint[..8], e)
                );
                Err(e)
            }
        }
    }
}

/// Analysis statistics
#[derive(Debug, Clone)]
pub struct AnalysisStats {
    pub total_tokens: usize,
    pub calculable_tokens: usize,
    pub trading_ready_tokens: usize,
    pub error_tokens: usize,
    pub required_accounts: usize,
    pub updated_at: DateTime<Utc>,
}
