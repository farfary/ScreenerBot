/// Pool analyzer module
/// Analyzes pools to determine calculability and extracts basic pool info using decoders

use crate::pools::types::PoolInfo;
use crate::pools::tokens::PoolToken;
use crate::pools::decoders::DecoderFactory;
use crate::pools::cache::{ PoolC    pub async fn get_token_availability(&self, token_mint: &str) -> Option<TokenAvailability> {
        self.cache.get_token_availability(token_mint).await
    }
}ta };
use crate::pools::constants::{ MIN_POOL_LIQUIDITY_USD, SOL_MINT };
use std::sync::Arc;
use crate::logger::{ log, LogTag };

/// Pool analyzer service
pub struct PoolAnalyzer {
    decoder_factory: DecoderFactory,
    cache: Arc<PoolCache>,
}

impl PoolAnalyzer {
    pub fn new(cache: Arc<PoolCache>) -> Self {
        Self {
            decoder_factory: DecoderFactory::new(),
            cache,
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
                Ok(is_calculable) => {
                    analyzed_count += 1;
                    if is_calculable {
                        calculable_count += 1;
                    }
                }
                Err(e) => {
                    error_count += 1;
                    log(
                        LogTag::Pool,
                        "ANALYZER_TOKEN_ERROR",
                        &format!("❌ Error analyzing {}: {}", &token.mint[..8], e)
                    );
                }
            }
        }

        log(
            LogTag::Pool,
            "ANALYZER_COMPLETE",
            &format!(
                "✅ Analysis complete: {}/{} analyzed, {} calculable, {} errors",
                analyzed_count,
                tokens.len(),
                calculable_count,
                error_count
            )
        );

        Ok(())
    }

    /// Analyze a single token and its pools
    pub async fn analyze_token(&self, token_mint: &str) -> Result<bool, String> {
        // Get all pools for this token
        let pools = match self.cache.get_pools(token_mint).await {
            Some(pools) => pools,
            None => {
                log(
                    LogTag::Pool,
                    "ANALYZER_NO_POOLS",
                    &format!("No pools found for token {}", &token_mint[..8])
                );
                return Ok(false);
            }
        };

        let mut best_pool: Option<PoolInfo> = None;
        let mut best_liquidity = 0.0;
        let mut all_vault_addresses = Vec::new();

        // Analyze each pool
        for pool in &pools {
            match self.analyze_pool(pool, token_mint).await {
                Ok(Some((vault_addresses, liquidity))) => {
                    // Add vault addresses to collection
                    all_vault_addresses.extend(vault_addresses);
                    
                    // Track best pool
                    if liquidity > best_liquidity {
                        best_liquidity = liquidity;
                        best_pool = Some(pool.clone());
                    }
                }
                Ok(None) => {
                    log(
                        LogTag::Pool,
                        "ANALYZER_POOL_NOT_CALCULABLE",
                        &format!("Pool {} not calculable for token {}", &pool.pool_address[..8], &token_mint[..8])
                    );
                }
                Err(e) => {
                    log(
                        LogTag::Pool,
                        "ANALYZER_POOL_ERROR",
                        &format!("Error analyzing pool {}: {}", &pool.pool_address[..8], e)
                    );
                }
            }
        }

        if let Some(best_pool) = best_pool {
            // Remove duplicates from vault addresses
            all_vault_addresses.sort();
            all_vault_addresses.dedup();

            // Store token availability in cache
            self.cache.set_token_calculable(
                token_mint,
                true,
                Some(best_pool)
            ).await;

            // Store required account addresses for fetcher
            self.cache.add_required_accounts(&all_vault_addresses).await;

            log(
                LogTag::Pool,
                "ANALYZER_TOKEN_CALCULABLE",
                &format!(
                    "✅ Token {} calculable via {} pools, {} vault addresses",
                    &token_mint[..8],
                    pools.len(),
                    all_vault_addresses.len()
                )
            );

            Ok(true)
        } else {
            // Mark as not calculable
            self.cache.set_token_calculable(
                token_mint,
                false,
                None
            ).await;

            log(
                LogTag::Pool,
                "ANALYZER_TOKEN_NOT_CALCULABLE",
                &format!("❌ Token {} not calculable from {} pools", &token_mint[..8], pools.len())
            );

            Ok(false)
        }
    }

    /// Analyze a single pool for calculability
    async fn analyze_pool(
        &self,
        pool: &PoolInfo,
        token_mint: &str
    ) -> Result<Option<(Vec<String>, f64)>, String> {
        // Check if pool has SOL pair
        let has_sol = pool.base_token_mint == SOL_MINT || pool.quote_token_mint == SOL_MINT;
        if !has_sol {
            return Ok(None);
        }

        // Check if reserves are available
        if pool.sol_reserves <= 0.0 || pool.token_reserves <= 0.0 {
            return Ok(None);
        }

        // Check minimum liquidity
        let liquidity = pool.liquidity_usd.unwrap_or(0.0);
        if liquidity < MIN_POOL_LIQUIDITY_USD {
            return Ok(None);
        }

        // Extract vault addresses using decoder
        let vault_addresses = self.extract_vault_addresses(pool).await?;

        Ok(Some((vault_addresses, liquidity)))
    }

    /// Extract vault addresses from pool using appropriate decoder
    async fn extract_vault_addresses(&self, pool: &PoolInfo) -> Result<Vec<String>, String> {
        let mut vault_addresses = Vec::new();

        // Always add pool address itself
        vault_addresses.push(pool.pool_address.clone());

        // Get decoder for this pool type
        if let Some(decoder) = self.decoder_factory.get_decoder(&pool.pool_program_id) {
            // Get pool account data
            if let Some(pool_account) = self.cache.get_account(&pool.pool_address).await {
                // Use decoder to extract vault addresses
                if let Ok(vaults) = decoder.extract_vault_addresses(&pool_account.data) {
                    vault_addresses.extend(vaults);
                }
            } else {
                log(
                    LogTag::Pool,
                    "ANALYZER_NO_POOL_DATA",
                    &format!("No account data for pool {}", &pool.pool_address[..8])
                );
            }
        } else {
            log(
                LogTag::Pool,
                "ANALYZER_NO_DECODER",
                &format!("No decoder for program {}", pool.pool_program_id)
            );
        }

        Ok(vault_addresses)
    }

    /// Re-analyze a specific token (when pools are updated)
    pub async fn re_analyze_token(&self, token_mint: &str) -> Result<(), String> {
        self.analyze_token(token_mint).await?;
        Ok(())
    }

    /// Get analysis statistics from cache
    pub async fn get_analysis_stats(&self) -> AnalysisStats {
        let calculable_tokens = self.cache.get_calculable_tokens().await;
        let required_accounts = self.cache.get_required_accounts().await;

        AnalysisStats {
            total_tokens: calculable_tokens.len(), // This is approximate
            calculable_tokens: calculable_tokens.len(),
            trading_ready_tokens: calculable_tokens.len(), // This is approximate
            error_tokens: 0, // Not tracked separately
            required_accounts: required_accounts.len(),
            updated_at: chrono::Utc::now(),
        }
    }

    /// Get calculable tokens from cache
    pub async fn get_calculable_tokens(&self) -> Vec<String> {
        self.cache.get_calculable_tokens().await
    }

    /// Get trading ready tokens from cache
    pub async fn get_trading_ready_tokens(&self) -> Vec<String> {
        self.cache.get_trading_ready_tokens().await
    }

    /// Get required accounts from cache
    pub async fn get_required_accounts(&self) -> Vec<String> {
        self.cache.get_required_accounts().await
    }

    /// Get token availability from cache
    pub async fn get_token_availability(&self, token_mint: &str) -> Option<TokenAvailability> {
        self.cache.get_token_availability(token_mint).await
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
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Token availability status (kept for compatibility)
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
    pub analyzed_at: chrono::DateTime<chrono::Utc>,
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
            analyzed_at: chrono::Utc::now(),
            errors: Vec::new(),
        }
    }

    /// Mark as calculable with best pool
    pub fn with_best_pool(mut self, pool: PoolInfo) -> Self {
        self.liquidity_usd = pool.liquidity_usd.unwrap_or(0.0);
        self.sol_reserves = pool.sol_reserves;
        self.best_pool = Some(pool);
        self.calculable = true;
        self.analyzed_at = chrono::Utc::now();
        self
    }

    /// Add error
    pub fn with_error(mut self, error: String) -> Self {
        self.errors.push(error);
        self.analyzed_at = chrono::Utc::now();
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
