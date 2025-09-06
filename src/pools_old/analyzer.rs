/// Pool analyzer task
/// Runs as background task analyzing pools for calculability and extracting basic pool info

use crate::pools::types::PoolInfo;
use crate::pools::tokens::PoolToken;
use crate::pools::decoders::DecoderFactory;
use crate::pools::cache::{ PoolCache, AccountData };
use crate::pools::constants::{ MIN_POOL_LIQUIDITY_USD, SOL_MINT };
use tokio::sync::RwLock;
use tokio::time::{ sleep, Duration };
use std::sync::Arc;
use std::collections::HashMap;
use chrono::{ DateTime, Utc };
use crate::logger::{ log, LogTag };

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

    /// Check if token is ready for trading
    pub fn is_ready_for_trading(&self) -> bool {
        self.calculable &&
            self.best_pool.is_some() &&
            self.sol_reserves > 0.0 &&
            self.liquidity_usd >= MIN_POOL_LIQUIDITY_USD
    }
}

/// Pool analyzer task service
pub struct PoolAnalyzerTask {
    decoder_factory: DecoderFactory,
    cache: Arc<PoolCache>,
    /// Task running status
    is_running: Arc<RwLock<bool>>,
    /// Last analysis times per token
    last_analyzed: Arc<RwLock<HashMap<String, DateTime<Utc>>>>,
}

impl PoolAnalyzerTask {
    pub fn new(cache: Arc<PoolCache>) -> Self {
        Self {
            decoder_factory: DecoderFactory::new(),
            cache,
            is_running: Arc::new(RwLock::new(false)),
            last_analyzed: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Start the analyzer background task
    pub async fn start_task(&self) {
        let mut is_running = self.is_running.write().await;
        if *is_running {
            log(LogTag::Pool, "ANALYZER_TASK_RUNNING", "Analyzer task already running");
            return;
        }
        *is_running = true;
        drop(is_running);

        log(LogTag::Pool, "ANALYZER_TASK_START", "🔬 Starting pool analyzer task");

        // Clone necessary data for the background task
        let decoder_factory = self.decoder_factory.clone();
        let cache = self.cache.clone();
        let is_running = self.is_running.clone();
        let last_analyzed = self.last_analyzed.clone();

        tokio::spawn(async move {
            while *is_running.read().await {
                match Self::analyze_tokens_with_pools(
                    &decoder_factory,
                    &cache,
                    &last_analyzed,
                ).await {
                    Ok(analyzed_count) => {
                        if analyzed_count > 0 {
                            log(
                                LogTag::Pool,
                                "ANALYZER_TASK_CYCLE",
                                &format!("✅ Analyzed {} tokens", analyzed_count)
                            );
                        }
                    }
                    Err(e) => {
                        log(
                            LogTag::Pool,
                            "ANALYZER_TASK_ERROR",
                            &format!("❌ Analyzer task error: {}", e)
                        );
                    }
                }

                // Sleep between analysis cycles
                sleep(Duration::from_secs(10)).await;
            }

            log(LogTag::Pool, "ANALYZER_TASK_STOP", "🛑 Analyzer task stopped");
        });
    }

    /// Stop the analyzer task
    pub async fn stop_task(&self) {
        let mut is_running = self.is_running.write().await;
        *is_running = false;
    }

    /// Analyze tokens that have pools and determine calculability
    async fn analyze_tokens_with_pools(
        decoder_factory: &DecoderFactory,
        cache: &Arc<PoolCache>,
        last_analyzed: &Arc<RwLock<HashMap<String, DateTime<Utc>>>>,
    ) -> Result<usize, String> {
        let tokens_with_pools = cache.get_tokens_with_pools().await;
        let mut analyzed_count = 0;

        for token_mint in tokens_with_pools {
            // Check if we need to analyze (every 60 seconds)
            if !Self::should_analyze_token(&token_mint, last_analyzed).await {
                continue;
            }

            // Get pools for this token
            if let Some(pools) = cache.get_pools(&token_mint).await {
                match Self::analyze_token_pools(
                    &token_mint,
                    &pools,
                    decoder_factory,
                    cache,
                ).await {
                    Ok(_) => {
                        analyzed_count += 1;
                        
                        // Update last analyzed time
                        {
                            let mut last_anal = last_analyzed.write().await;
                            last_anal.insert(token_mint.clone(), Utc::now());
                        }
                    }
                    Err(e) => {
                        log(
                            LogTag::Pool,
                            "ANALYZER_TOKEN_ERROR",
                            &format!("Failed to analyze token {}: {}", &token_mint[..8], e)
                        );
                    }
                }
            }
        }

        Ok(analyzed_count)
    }

    /// Check if we should analyze a token (time-based)
    async fn should_analyze_token(
        token_mint: &str,
        last_analyzed: &Arc<RwLock<HashMap<String, DateTime<Utc>>>>,
    ) -> bool {
        let last_anal_map = last_analyzed.read().await;
        match last_anal_map.get(token_mint) {
            Some(last_time) => {
                let now = Utc::now();
                let duration = now.signed_duration_since(*last_time);
                duration.num_seconds() > 60 // Analyze every 60 seconds
            }
            None => true, // Never analyzed
        }
    }

    /// Analyze pools for a specific token
    async fn analyze_token_pools(
        token_mint: &str,
        pools: &[PoolInfo],
        decoder_factory: &DecoderFactory,
        cache: &Arc<PoolCache>,
    ) -> Result<(), String> {
        let mut calculable_pools = Vec::new();
        let mut all_vault_addresses = Vec::new();

        // Analyze each pool
        for pool in pools {
            // Check if pool has SOL pair
            let has_sol = pool.base_token_mint == SOL_MINT || pool.quote_token_mint == SOL_MINT;
            if !has_sol {
                continue;
            }

            // Check minimum liquidity
            if let Some(liquidity) = pool.liquidity_usd {
                if liquidity < MIN_POOL_LIQUIDITY_USD {
                    continue;
                }
            }

            // Extract vault addresses using decoder
            if let Some(vault_addresses) = Self::extract_vault_addresses(
                &pool.pool_address,
                &pool.pool_program_id,
                decoder_factory,
                cache,
            ).await? {
                all_vault_addresses.extend(vault_addresses);
                calculable_pools.push(pool.clone());
            }
        }

        // Determine if token is calculable
        if !calculable_pools.is_empty() {
            // Find best pool (highest liquidity)
            let best_pool = calculable_pools
                .iter()
                .max_by(|a, b| {
                    let a_liquidity = a.liquidity_usd.unwrap_or(0.0);
                    let b_liquidity = b.liquidity_usd.unwrap_or(0.0);
                    a_liquidity.partial_cmp(&b_liquidity).unwrap_or(std::cmp::Ordering::Equal)
                })
                .unwrap();

            // Create availability data
            let availability = TokenAvailability {
                token_mint: token_mint.to_string(),
                calculable: true,
                best_pool: Some(best_pool.clone()),
                pools: calculable_pools,
                reserve_accounts: all_vault_addresses.clone(),
                liquidity_usd: best_pool.liquidity_usd.unwrap_or(0.0),
                sol_reserves: best_pool.sol_reserves,
                analyzed_at: Utc::now(),
                errors: Vec::new(),
            };

            // Store in cache
            cache.store_token_availability(token_mint, availability).await;
            cache.add_required_accounts(&all_vault_addresses).await;

            log(
                LogTag::Pool,
                "ANALYZER_TOKEN_CALCULABLE",
                &format!("✅ Token {} is calculable via {} pools", &token_mint[..8], calculable_pools.len())
            );
        } else {
            // Not calculable
            let availability = TokenAvailability {
                token_mint: token_mint.to_string(),
                calculable: false,
                best_pool: None,
                pools: pools.to_vec(),
                reserve_accounts: Vec::new(),
                liquidity_usd: 0.0,
                sol_reserves: 0.0,
                analyzed_at: Utc::now(),
                errors: vec!["No calculable pools found".to_string()],
            };

            cache.store_token_availability(token_mint, availability).await;

            log(
                LogTag::Pool,
                "ANALYZER_TOKEN_NOT_CALCULABLE",
                &format!("❌ Token {} has no calculable pools", &token_mint[..8])
            );
        }

        Ok(())
    }

    /// Extract vault addresses using decoders
    async fn extract_vault_addresses(
        pool_address: &str,
        program_id: &str,
        decoder_factory: &DecoderFactory,
        cache: &Arc<PoolCache>,
    ) -> Result<Option<Vec<String>>, String> {
        // Get pool account data
        if let Some(account_data) = cache.get_account(pool_address).await {
            if !account_data.exists || account_data.is_expired() {
                return Ok(None);
            }

            // Get appropriate decoder
            if let Some(decoder) = decoder_factory.get_decoder(program_id) {
                match decoder.extract_vault_addresses(&account_data.data).await {
                    Ok(mut vault_addresses) => {
                        // Always include the pool address itself
                        vault_addresses.insert(0, pool_address.to_string());
                        Ok(Some(vault_addresses))
                    }
                    Err(e) => {
                        log(
                            LogTag::Pool,
                            "ANALYZER_VAULT_ERROR",
                            &format!("Failed to extract vaults for {}: {}", &pool_address[..8], e)
                        );
                        // Even if vault extraction fails, include the pool address
                        Ok(Some(vec![pool_address.to_string()]))
                    }
                }
            } else {
                log(
                    LogTag::Pool,
                    "ANALYZER_NO_DECODER",
                    &format!("No decoder for program: {}", program_id)
                );
                // Include just the pool address
                Ok(Some(vec![pool_address.to_string()]))
            }
        } else {
            Ok(None)
        }
    }

    /// Check if analyzer task is running
    pub async fn is_running(&self) -> bool {
        *self.is_running.read().await
    }

    /// Get analyzer statistics
    pub async fn get_analyzer_stats(&self) -> AnalysisStats {
        let all_availability = self.cache.get_all_token_availability().await;
        let total_tokens = all_availability.len();
        let calculable_tokens = all_availability.values().filter(|a| a.calculable).count();
        let trading_ready_tokens = all_availability.values().filter(|a| a.is_ready_for_trading()).count();
        let error_tokens = all_availability.values().filter(|a| !a.errors.is_empty()).count();
        let required_accounts = self.cache.get_required_accounts().await.len();

        AnalysisStats {
            total_tokens,
            calculable_tokens,
            trading_ready_tokens,
            error_tokens,
            required_accounts,
            updated_at: Utc::now(),
        }
    }

    /// Get token availability information
    pub async fn get_token_availability(&self, token_mint: &str) -> Option<TokenAvailability> {
        self.cache.get_token_availability(token_mint).await
    }

    /// Get all calculable tokens
    pub async fn get_calculable_tokens(&self) -> Vec<String> {
        let all_availability = self.cache.get_all_token_availability().await;
        all_availability
            .values()
            .filter(|a| a.calculable)
            .map(|a| a.token_mint.clone())
            .collect()
    }

    /// Get tokens ready for trading
    pub async fn get_trading_ready_tokens(&self) -> Vec<String> {
        let all_availability = self.cache.get_all_token_availability().await;
        all_availability
            .values()
            .filter(|a| a.is_ready_for_trading())
            .map(|a| a.token_mint.clone())
            .collect()
    }

    /// Get required account addresses for RPC fetching
    pub async fn get_required_accounts(&self) -> Vec<String> {
        self.cache.get_required_accounts().await
    }
}
