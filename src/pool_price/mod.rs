use anyhow::Result;
use std::collections::HashMap;
use std::sync::{ Arc, Mutex };
use std::time::Instant;
use std::str::FromStr;
use std::path::Path;
use reqwest;
use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use spl_token::state::Account as TokenAccount;
use spl_token::ID as TOKEN_PROGRAM_ID;
use solana_sdk::program_pack::Pack;

use crate::logger::{ log, LogTag };
use crate::decimal_cache::{ DecimalCache, fetch_or_cache_decimals };

// Pool logging configuration and helper functions
const ENABLE_POOL_DEBUG_LOGS: bool = false; // Set to true for detailed debugging

/// Helper function for conditional debug logging
fn debug_log(log_type: &str, message: &str) {
    if ENABLE_POOL_DEBUG_LOGS {
        log(LogTag::Pool, log_type, message);
    }
}

/// Helper function for regular pool logging
fn pool_log(log_type: &str, message: &str) {
    log(LogTag::Pool, log_type, message);
}

pub mod types;
pub mod decoder;

// Re-export main types for backwards compatibility
pub use types::*;
pub use decoder::*;

// =============================================================================
// MAIN POOL DISCOVERY AND PRICE CALCULATOR
// =============================================================================

pub struct PoolDiscoveryAndPricing {
    rpc_client: RpcClient,
    http_client: reqwest::Client,
    // Cache for biggest pool per token (token_mint -> PoolCacheEntry)
    pool_cache: Arc<Mutex<HashMap<String, PoolCacheEntry>>>,
    // Cache for program IDs per token (token_mint -> ProgramIdCacheEntry)
    program_id_cache: Arc<Mutex<HashMap<String, ProgramIdCacheEntry>>>,
}

impl PoolDiscoveryAndPricing {
    pub fn new(rpc_url: &str) -> Self {
        Self {
            rpc_client: RpcClient::new(rpc_url.to_string()),
            http_client: reqwest::Client::new(),
            pool_cache: Arc::new(Mutex::new(HashMap::new())),
            program_id_cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Discover all pools for a given token mint address
    pub async fn discover_pools(&self, token_mint: &str) -> Result<Vec<DiscoveredPool>> {
        let url = format!("{}/{}", DEXSCREENER_API_BASE, token_mint);

        pool_log("INFO", &format!("Discovering pools for token: {}", token_mint));

        let response = self.http_client.get(&url).send().await?;

        if !response.status().is_success() {
            pool_log(
                "ERROR",
                &format!("DexScreener API failed: {} for token {}", response.status(), token_mint)
            );
            return Err(
                anyhow::anyhow!("DexScreener API request failed with status: {}", response.status())
            );
        }

        let pairs: Vec<serde_json::Value> = response.json().await?;
        let mut discovered_pools = Vec::new();

        debug_log("DEBUG", &format!("Received {} pairs from API", pairs.len()));

        if pairs.is_empty() {
            pool_log("WARN", &format!("No pools found for token: {}", token_mint));
            return Ok(discovered_pools);
        }

        let pairs_count = pairs.len();

        for pair in pairs {
            if let Ok(pool) = self.parse_pool_from_api_response(&pair) {
                log(
                    LogTag::Pool,
                    "DEBUG",
                    &format!(
                        "Parsed pool: {} ({}) with ${:.2} liquidity",
                        pool.pair_address,
                        pool.dex_id,
                        pool.liquidity_usd
                    )
                );
                discovered_pools.push(pool);
            } else {
                log(
                    LogTag::Pool,
                    "WARN",
                    &format!(
                        "Failed to parse pool: {}",
                        pair
                            .get("pairAddress")
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown")
                    )
                );
            }
        }

        // Log results - detailed in debug mode, summary otherwise
        if is_debug_pool_price_enabled() {
            pool_log(
                "SUCCESS",
                &format!(
                    "Found {} valid pools out of {} for token {}",
                    discovered_pools.len(),
                    pairs_count,
                    token_mint
                )
            );
        } else if discovered_pools.len() > 0 {
            log_pool_summary("Discovery", discovered_pools.len(), pairs_count);
        }

        if discovered_pools.is_empty() {
            pool_log(
                "WARN",
                &format!(
                    "No valid pools found despite {} API pairs for token: {}",
                    pairs_count,
                    token_mint
                )
            );
        }

        Ok(discovered_pools)
    }

    /// Parse a single pool from DexScreener API response
    fn parse_pool_from_api_response(&self, pair: &serde_json::Value) -> Result<DiscoveredPool> {
        let pair_address = pair["pairAddress"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing pairAddress"))?
            .to_string();

        let dex_id = pair["dexId"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing dexId"))?
            .to_string();

        let base_token = PoolToken {
            address: pair["baseToken"]["address"].as_str().unwrap_or("").to_string(),
            name: pair["baseToken"]["name"].as_str().unwrap_or("").to_string(),
            symbol: pair["baseToken"]["symbol"].as_str().unwrap_or("").to_string(),
        };

        let quote_token = PoolToken {
            address: pair["quoteToken"]["address"].as_str().unwrap_or("").to_string(),
            name: pair["quoteToken"]["name"].as_str().unwrap_or("").to_string(),
            symbol: pair["quoteToken"]["symbol"].as_str().unwrap_or("").to_string(),
        };

        let price_native = pair["priceNative"].as_str().unwrap_or("0").to_string();
        let price_usd = pair["priceUsd"].as_str().unwrap_or("0").to_string();

        let liquidity_usd = pair["liquidity"]["usd"].as_f64().unwrap_or(0.0);
        let volume_24h = pair["volume"]["h24"].as_f64().unwrap_or(0.0);

        let labels = pair["labels"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect::<Vec<String>>()
            })
            .unwrap_or_else(Vec::new);

        Ok(DiscoveredPool {
            pair_address,
            dex_id,
            base_token,
            quote_token,
            price_native,
            price_usd,
            liquidity_usd,
            volume_24h,
            labels,
        })
    }

    /// Get pool prices for all discovered pools of a token
    pub async fn get_token_pool_prices(&self, token_mint: &str) -> Result<Vec<PoolPriceResult>> {
        let discovered_pools = self.discover_pools(token_mint).await?;
        let mut results = Vec::new();

        log(
            LogTag::Pool,
            "INFO",
            &format!("Calculating prices for {} pools", discovered_pools.len())
        );

        for pool in discovered_pools {
            let result = self.calculate_pool_price_with_discovery(&pool).await;

            if !result.calculation_successful {
                log(
                    LogTag::Pool,
                    "ERROR",
                    &format!(
                        "Price calculation failed for pool {} ({}): {}",
                        result.pool_address,
                        result.dex_id,
                        result.error_message.as_ref().unwrap_or(&"Unknown error".to_string())
                    )
                );
            } else {
                log(
                    LogTag::Pool,
                    "DEBUG",
                    &format!(
                        "Calculated price for pool {} ({}): ${:.6}",
                        result.pool_address,
                        result.dex_id,
                        result.calculated_price
                    )
                );
            }

            results.push(result);
        }

        // Sort by liquidity (highest first) for better results
        results.sort_by(|a, b|
            b.liquidity_usd.partial_cmp(&a.liquidity_usd).unwrap_or(std::cmp::Ordering::Equal)
        );

        // Log completion - detailed in debug mode, summary otherwise
        let successful_pools = results
            .iter()
            .filter(|r| r.calculation_successful)
            .count();

        if is_debug_pool_price_enabled() {
            pool_log(
                "SUCCESS",
                &format!(
                    "Completed price calculation for {} pools ({} successful)",
                    results.len(),
                    successful_pools
                )
            );
        } else {
            log_pool_summary("Token Analysis", successful_pools, results.len());
        }

        Ok(results)
    }

    /// Get program ID string for a given pool type
    fn get_program_id_for_pool_type(&self, pool_type: PoolType) -> String {
        match pool_type {
            PoolType::RaydiumCpmm => RAYDIUM_CPMM_PROGRAM_ID.to_string(),
            PoolType::RaydiumAmm => RAYDIUM_AMM_PROGRAM_ID.to_string(),
            PoolType::MeteoraDlmm => METEORA_DLMM_PROGRAM_ID.to_string(),
            PoolType::MeteoraDammV2 => METEORA_DAMM_V2_PROGRAM_ID.to_string(),
            PoolType::RaydiumLaunchLab => RAYDIUM_LAUNCHLAB_PROGRAM_ID.to_string(),
            PoolType::OrcaWhirlpool => ORCA_WHIRLPOOL_PROGRAM_ID.to_string(),
            PoolType::PumpfunAmm => PUMPFUN_AMM_PROGRAM_ID.to_string(),
            _ => "Unknown".to_string(),
        }
    }

    /// Get biggest pool for token with caching (2-minute expiration)
    pub async fn get_biggest_pool_cached(
        &self,
        token_mint: &str
    ) -> Result<Option<PoolPriceResult>> {
        // Check cache first
        {
            let cache = self.pool_cache.lock().unwrap();
            if let Some(entry) = cache.get(token_mint) {
                if !entry.is_expired() {
                    debug_log("DEBUG", &format!("Using cached pool for token: {}", token_mint));
                    return Ok(Some(entry.pool_result.clone()));
                }
            }
        }

        pool_log("INFO", &format!("Fetching biggest pool for token: {}", token_mint));

        // Fetch all pool prices
        let pool_results = self.get_token_pool_prices(token_mint).await?;
        let pool_results_count = pool_results.len(); // Store count before moving

        // Find the biggest successful pool (by liquidity)
        let biggest_pool = pool_results
            .into_iter()
            .filter(|p| p.calculation_successful && p.is_sol_pair)
            .max_by(|a, b|
                a.liquidity_usd.partial_cmp(&b.liquidity_usd).unwrap_or(std::cmp::Ordering::Equal)
            );

        if biggest_pool.is_none() {
            log(
                LogTag::Pool,
                "WARN",
                &format!(
                    "No valid SOL-paired pools found for token {} (discovered {})",
                    token_mint,
                    pool_results_count
                )
            );
        } else {
            log(
                LogTag::Pool,
                "SUCCESS",
                &format!(
                    "Found biggest pool for token {} with ${:.2} liquidity",
                    token_mint,
                    biggest_pool.as_ref().unwrap().liquidity_usd
                )
            );
        }

        // Cache the result if found
        if let Some(pool) = &biggest_pool {
            let mut cache = self.pool_cache.lock().unwrap();
            cache.insert(token_mint.to_string(), PoolCacheEntry {
                pool_result: pool.clone(),
                cached_at: Instant::now(),
            });

            log(
                LogTag::System,
                "INFO",
                &format!(
                    "Cached biggest pool for token {} with liquidity ${:.2}",
                    token_mint,
                    pool.liquidity_usd
                )
            );
        }

        Ok(biggest_pool)
    }

    /// Get program IDs for token with caching (2-minute expiration)
    pub async fn get_program_ids_cached(&self, token_mint: &str) -> Result<Vec<String>> {
        // Check cache first
        {
            let cache = self.program_id_cache.lock().unwrap();
            if let Some(entry) = cache.get(token_mint) {
                if !entry.is_expired() {
                    debug_log(
                        "DEBUG",
                        &format!(
                            "Cache HIT: Program IDs for token {} (count: {}, age: {}s)",
                            token_mint,
                            entry.program_ids.len(),
                            entry.cached_at.elapsed().as_secs()
                        )
                    );
                    return Ok(entry.program_ids.clone());
                } else {
                    debug_log("DEBUG", &format!("Cache EXPIRED for token {}", token_mint));
                }
            } else {
                debug_log("DEBUG", &format!("Cache MISS for token {}", token_mint));
            }
        }

        pool_log("INFO", &format!("Fetching program IDs for token: {}", token_mint));

        // Discover pools to get their program IDs
        let discovered_pools = self.discover_pools(token_mint).await?;
        let mut program_ids = Vec::new();

        for pool in &discovered_pools {
            // Detect program ID for each pool
            if let Ok(pool_type) = self.detect_pool_type(&pool.pair_address).await {
                let program_id = match pool_type {
                    PoolType::RaydiumCpmm => RAYDIUM_CPMM_PROGRAM_ID.to_string(),
                    PoolType::RaydiumAmm => RAYDIUM_AMM_PROGRAM_ID.to_string(),
                    PoolType::MeteoraDlmm => METEORA_DLMM_PROGRAM_ID.to_string(),
                    PoolType::MeteoraDammV2 => METEORA_DAMM_V2_PROGRAM_ID.to_string(),
                    PoolType::RaydiumLaunchLab => RAYDIUM_LAUNCHLAB_PROGRAM_ID.to_string(),
                    PoolType::PumpfunAmm => PUMPFUN_AMM_PROGRAM_ID.to_string(),
                    _ => {
                        continue;
                    } // Skip unknown types
                };

                if !program_ids.contains(&program_id) {
                    program_ids.push(program_id);
                }
            }
        }

        // Cache the result
        {
            let mut cache = self.program_id_cache.lock().unwrap();
            cache.insert(token_mint.to_string(), ProgramIdCacheEntry {
                program_ids: program_ids.clone(),
                cached_at: Instant::now(),
            });
        }

        log(
            LogTag::System,
            "INFO",
            &format!("Cached {} program IDs for token {}", program_ids.len(), token_mint)
        );

        Ok(program_ids)
    }

    /// Clear expired cache entries (maintenance function)
    pub fn cleanup_expired_cache(&self) {
        let mut pool_cache = self.pool_cache.lock().unwrap();
        let mut program_id_cache = self.program_id_cache.lock().unwrap();

        // Remove expired pool cache entries
        let initial_pool_count = pool_cache.len();
        pool_cache.retain(|_, entry| !entry.is_expired());
        let final_pool_count = pool_cache.len();

        // Remove expired program ID cache entries
        let initial_program_count = program_id_cache.len();
        program_id_cache.retain(|_, entry| !entry.is_expired());
        let final_program_count = program_id_cache.len();

        if initial_pool_count > final_pool_count || initial_program_count > final_program_count {
            log(
                LogTag::System,
                "INFO",
                &format!(
                    "Cleaned up expired cache entries: {} pool entries removed, {} program ID entries removed",
                    initial_pool_count - final_pool_count,
                    initial_program_count - final_program_count
                )
            );
        }
    }

    /// Calculate on-chain pool price using discovered pool info
    async fn calculate_pool_price_with_discovery(
        &self,
        discovered_pool: &DiscoveredPool
    ) -> PoolPriceResult {
        let mut pool_type = PoolType::from_dex_id_and_labels(
            &discovered_pool.dex_id,
            &discovered_pool.labels
        );

        // Override with actual program ID detection if possible (more accurate than DexScreener labels)
        if pool_type == PoolType::Orca {
            log(
                LogTag::Pool,
                "DEBUG",
                "DexScreener classified as generic Orca, checking actual program ID..."
            );
            if let Ok(detected_type) = self.detect_pool_type(&discovered_pool.pair_address).await {
                if detected_type == PoolType::OrcaWhirlpool {
                    log(
                        LogTag::Pool,
                        "DEBUG",
                        "Program ID confirms this is a Whirlpool, overriding classification"
                    );
                    pool_type = PoolType::OrcaWhirlpool;
                }
            }
        }

        // Similar override for Meteora pools - check actual program ID for accurate classification
        if pool_type == PoolType::MeteoraDammV2 || pool_type == PoolType::MeteoraDlmm {
            log(
                LogTag::Pool,
                "DEBUG",
                &format!("DexScreener classified as {:?}, checking actual program ID...", pool_type)
            );
            if let Ok(detected_type) = self.detect_pool_type(&discovered_pool.pair_address).await {
                if
                    detected_type == PoolType::MeteoraDammV2 ||
                    detected_type == PoolType::MeteoraDlmm
                {
                    log(
                        LogTag::Pool,
                        "DEBUG",
                        &format!(
                            "Program ID confirms this is {:?}, overriding classification",
                            detected_type
                        )
                    );
                    pool_type = detected_type;
                }
            }
        }

        // Override for Pump.fun pools - detect actual program ID for accurate classification
        if pool_type == PoolType::Unknown || pool_type == PoolType::PumpfunAmm {
            log(
                LogTag::Pool,
                "DEBUG",
                &format!("DexScreener classified as {:?}, checking actual program ID...", pool_type)
            );
            if let Ok(detected_type) = self.detect_pool_type(&discovered_pool.pair_address).await {
                if detected_type == PoolType::PumpfunAmm {
                    log(
                        LogTag::Pool,
                        "DEBUG",
                        "Program ID confirms this is a Pump.fun AMM, overriding classification"
                    );
                    pool_type = PoolType::PumpfunAmm;
                }
            }
        }

        let dexscreener_price = discovered_pool.price_native.parse::<f64>().unwrap_or(0.0);

        let is_sol_pair =
            discovered_pool.base_token.address == "So11111111111111111111111111111111111111112" ||
            discovered_pool.quote_token.address == "So11111111111111111111111111111111111111112";

        // Try to calculate on-chain price
        let (calculated_price, calculation_successful, error_message) = match
            self.calculate_pool_price_with_type(&discovered_pool.pair_address, pool_type).await
        {
            Ok((price, _, _, _)) => {
                if price <= 0.0 {
                    let error_msg =
                        format!("Invalid price calculated: {} (price must be > 0)", price);
                    pool_log("WARN", &error_msg);
                    (price, false, Some(error_msg))
                } else {
                    (price, true, None)
                }
            }
            Err(e) => {
                let error_msg = format!("Failed to calculate on-chain price: {}", e);
                pool_log(
                    "ERROR",
                    &format!(
                        "Pool calculation failed - Address: {}, Type: {:?}, Error: {}",
                        discovered_pool.pair_address,
                        pool_type,
                        e
                    )
                );
                (0.0, false, Some(error_msg))
            }
        };

        let price_difference_percent = if dexscreener_price > 0.0 && calculated_price > 0.0 {
            ((calculated_price - dexscreener_price).abs() / dexscreener_price) * 100.0
        } else {
            0.0
        };

        PoolPriceResult {
            pool_address: discovered_pool.pair_address.clone(),
            pool_type,
            dex_id: discovered_pool.dex_id.clone(),
            token_a_mint: discovered_pool.base_token.address.clone(),
            token_b_mint: discovered_pool.quote_token.address.clone(),
            token_a_symbol: discovered_pool.base_token.symbol.clone(),
            token_b_symbol: discovered_pool.quote_token.symbol.clone(),
            calculated_price,
            dexscreener_price,
            price_difference_percent,
            liquidity_usd: discovered_pool.liquidity_usd,
            volume_24h: discovered_pool.volume_24h,
            is_sol_pair,
            calculation_successful,
            error_message,
        }
    }

    /// Universal pool price calculation method
    pub async fn calculate_pool_price(
        &self,
        pool_address: &str
    ) -> Result<(f64, String, String, PoolType)> {
        pool_log("INFO", &format!("Starting price calculation for pool: {}", pool_address));

        // First detect the pool type
        let pool_type = self.detect_pool_type(pool_address).await?;
        debug_log("DEBUG", &format!("Pool type detected: {:?}", pool_type));

        // Parse the pool data based on type
        let pool_data = self.parse_pool_data(pool_address, pool_type).await?;
        debug_log("DEBUG", "Pool data parsed successfully");

        // Calculate price using the universal method
        let price = self.calculate_price_from_pool_data(&pool_data).await?;
        pool_log(
            "SUCCESS",
            &format!("Price calculation completed: {} (pool type: {:?})", price, pool_type)
        );

        Ok((price, pool_data.token_a.mint.clone(), pool_data.token_b.mint.clone(), pool_type))
    }

    /// Calculate price with explicit pool type (for manual override)
    pub async fn calculate_pool_price_with_type(
        &self,
        pool_address: &str,
        pool_type: PoolType
    ) -> Result<(f64, String, String, PoolType)> {
        log(
            LogTag::Pool,
            "INFO",
            &format!(
                "Calculating price with explicit type {:?} for pool: {}",
                pool_type,
                pool_address
            )
        );
        let pool_data = self.parse_pool_data(pool_address, pool_type).await?;
        let price = self.calculate_price_from_pool_data(&pool_data).await?;

        Ok((price, pool_data.token_a.mint.clone(), pool_data.token_b.mint.clone(), pool_type))
    }

    /// Legacy method for backward compatibility
    pub async fn calculate_raydium_cpmm_price(
        &self,
        pool_address: &str
    ) -> Result<(f64, String, String)> {
        let (price, token_a, token_b, _) = self.calculate_pool_price_with_type(
            pool_address,
            PoolType::RaydiumCpmm
        ).await?;
        Ok((price, token_a, token_b))
    }

    /// Legacy method for backward compatibility
    pub async fn calculate_meteora_dlmm_price(
        &self,
        pool_address: &str
    ) -> Result<(f64, String, String)> {
        let (price, token_a, token_b, _) = self.calculate_pool_price_with_type(
            pool_address,
            PoolType::MeteoraDlmm
        ).await?;
        Ok((price, token_a, token_b))
    }

    /// Auto-detect pool type based on pool address and program ID owner
    pub async fn detect_pool_type(&self, pool_address: &str) -> Result<PoolType> {
        let pool_pubkey = match Pubkey::from_str(pool_address) {
            Ok(pubkey) => pubkey,
            Err(e) => {
                log(
                    LogTag::Pool,
                    "ERROR",
                    &format!(
                        "❌ INVALID POOL ADDRESS\n\
                    Pool Address: {}\n\
                    Error: Failed to parse as Pubkey - {}",
                        pool_address,
                        e
                    )
                );
                return Err(anyhow::anyhow!("Invalid pool address: {}", e));
            }
        };

        let account_info = match self.rpc_client.get_account(&pool_pubkey) {
            Ok(info) => info,
            Err(e) => {
                log(
                    LogTag::Pool,
                    "ERROR",
                    &format!(
                        "❌ FAILED TO FETCH POOL ACCOUNT\n\
                    Pool Address: {}\n\
                    Error: RPC call failed - {}",
                        pool_address,
                        e
                    )
                );
                return Err(anyhow::anyhow!("Failed to fetch pool account: {}", e));
            }
        };

        // Get the program ID that owns this account
        let program_id = account_info.owner.to_string();

        debug_log("DEBUG", &format!("Pool account data size: {} bytes", account_info.data.len()));

        pool_log(
            "INFO",
            &format!("Detecting pool type for {} (program: {})", pool_address, program_id)
        );

        // Determine pool type based on program ID
        match program_id.as_str() {
            // Raydium CPMM Program ID
            id if id == RAYDIUM_CPMM_PROGRAM_ID => {
                pool_log("SUCCESS", "Detected: Raydium CPMM pool");
                Ok(PoolType::RaydiumCpmm)
            }
            // Meteora DLMM Program ID
            id if id == METEORA_DLMM_PROGRAM_ID => {
                pool_log("SUCCESS", "Detected: Meteora DLMM pool");
                Ok(PoolType::MeteoraDlmm)
            }
            // Meteora DAMM v2 Program ID
            id if id == METEORA_DAMM_V2_PROGRAM_ID => {
                pool_log("SUCCESS", "Detected: Meteora DAMM v2 pool");
                Ok(PoolType::MeteoraDammV2)
            }
            // Raydium LaunchLab Program ID
            id if id == RAYDIUM_LAUNCHLAB_PROGRAM_ID => {
                pool_log("SUCCESS", "Detected: Raydium LaunchLab pool");
                Ok(PoolType::RaydiumLaunchLab)
            }
            // Orca Whirlpool Program ID
            id if id == ORCA_WHIRLPOOL_PROGRAM_ID => {
                pool_log("SUCCESS", "Detected: Orca Whirlpool pool");
                Ok(PoolType::OrcaWhirlpool)
            }
            // Pump.fun AMM Program ID
            id if id == PUMPFUN_AMM_PROGRAM_ID => {
                pool_log("SUCCESS", "Detected: Pump.fun AMM pool");
                Ok(PoolType::PumpfunAmm)
            }
            // Add other DEX program IDs as needed
            // Phoenix, Orca, etc.

            // Unknown program ID
            _ => {
                pool_log(
                    "WARN",
                    &format!(
                        "⚠️ UNKNOWN PROGRAM ID\n\
                    Pool Address: {}\n\
                    Program ID: {}\n\
                    Data Size: {} bytes\n\
                    Falling back to size-based detection",
                        pool_address,
                        program_id,
                        account_info.data.len()
                    )
                );

                // Fallback to size-based detection as a last resort
                let account_data = account_info.data.clone();
                log(
                    LogTag::Pool,
                    "DEBUG",
                    &format!(
                        "Using fallback detection with data size: {} bytes",
                        account_data.len()
                    )
                );

                if account_data.len() >= 800 {
                    log(
                        LogTag::Pool,
                        "WARN",
                        "Guessing: Meteora DLMM (based on data size >= 800 bytes)"
                    );
                    Ok(PoolType::MeteoraDlmm)
                } else if account_data.len() >= 600 {
                    log(
                        LogTag::Pool,
                        "WARN",
                        "Guessing: Raydium CPMM (based on data size >= 600 bytes)"
                    );
                    Ok(PoolType::RaydiumCpmm)
                } else {
                    log(
                        LogTag::Pool,
                        "ERROR",
                        &format!(
                            "❌ POOL TYPE DETECTION FAILED\n\
                        Pool Address: {}\n\
                        Program ID: {}\n\
                        Data Size: {} bytes\n\
                        Defaulting to: Raydium CPMM",
                            pool_address,
                            program_id,
                            account_data.len()
                        )
                    );
                    Ok(PoolType::RaydiumCpmm)
                }
            }
        }
    }

    /// Universal pool data parser
    pub async fn parse_pool_data(
        &self,
        pool_address: &str,
        pool_type: PoolType
    ) -> Result<PoolData> {
        let pool_pubkey = Pubkey::from_str(pool_address)?;
        let account_data = self.rpc_client.get_account_data(&pool_pubkey)?;

        match pool_type {
            PoolType::RaydiumCpmm => {
                let raw_data = parse_raydium_cpmm_data(&account_data)?;

                // Get token vault balances
                let token_0_vault_pubkey = Pubkey::from_str(&raw_data.token_0_vault)?;
                let token_1_vault_pubkey = Pubkey::from_str(&raw_data.token_1_vault)?;

                let token_0_balance = self.get_token_balance(&token_0_vault_pubkey).await?;
                let token_1_balance = self.get_token_balance(&token_1_vault_pubkey).await?;

                Ok(PoolData {
                    pool_type,
                    token_a: TokenInfo {
                        mint: raw_data.token_0_mint,
                        decimals: raw_data.mint_0_decimals,
                    },
                    token_b: TokenInfo {
                        mint: raw_data.token_1_mint,
                        decimals: raw_data.mint_1_decimals,
                    },
                    reserve_a: ReserveInfo {
                        vault_address: raw_data.token_0_vault,
                        balance: token_0_balance,
                    },
                    reserve_b: ReserveInfo {
                        vault_address: raw_data.token_1_vault,
                        balance: token_1_balance,
                    },
                    specific_data: PoolSpecificData::RaydiumCpmm {
                        lp_mint: "".to_string(),
                        observation_key: "".to_string(),
                    },
                })
            }
            PoolType::RaydiumAmm => {
                let raw_data = parse_raydium_amm_data(&account_data)?;

                // Get token vault balances
                let base_vault_pubkey = Pubkey::from_str(&raw_data.base_vault)?;
                let quote_vault_pubkey = Pubkey::from_str(&raw_data.quote_vault)?;

                let base_balance = self.get_token_balance(&base_vault_pubkey).await?;
                let quote_balance = self.get_token_balance(&quote_vault_pubkey).await?;

                // Get decimals for both tokens
                let base_decimals = self.get_token_decimals(&raw_data.base_mint).await?;
                let quote_decimals = self.get_token_decimals(&raw_data.quote_mint).await?;

                Ok(PoolData {
                    pool_type,
                    token_a: TokenInfo {
                        mint: raw_data.base_mint,
                        decimals: base_decimals,
                    },
                    token_b: TokenInfo {
                        mint: raw_data.quote_mint,
                        decimals: quote_decimals,
                    },
                    reserve_a: ReserveInfo {
                        vault_address: raw_data.base_vault.clone(),
                        balance: base_balance,
                    },
                    reserve_b: ReserveInfo {
                        vault_address: raw_data.quote_vault.clone(),
                        balance: quote_balance,
                    },
                    specific_data: PoolSpecificData::RaydiumAmm {
                        base_vault: raw_data.base_vault,
                        quote_vault: raw_data.quote_vault,
                    },
                })
            }
            PoolType::MeteoraDlmm => {
                let raw_data = parse_meteora_dlmm_data(&account_data)?;

                // Get token reserve balances
                let reserve_x_pubkey = Pubkey::from_str(&raw_data.reserve_x)?;
                let reserve_y_pubkey = Pubkey::from_str(&raw_data.reserve_y)?;

                let reserve_x_balance = self.get_token_balance(&reserve_x_pubkey).await?;
                let reserve_y_balance = self.get_token_balance(&reserve_y_pubkey).await?;

                // Get decimals for both tokens
                let token_x_decimals = self.get_token_decimals(&raw_data.token_x_mint).await?;
                let token_y_decimals = self.get_token_decimals(&raw_data.token_y_mint).await?;

                Ok(PoolData {
                    pool_type,
                    token_a: TokenInfo {
                        mint: raw_data.token_x_mint,
                        decimals: token_x_decimals,
                    },
                    token_b: TokenInfo {
                        mint: raw_data.token_y_mint,
                        decimals: token_y_decimals,
                    },
                    reserve_a: ReserveInfo {
                        vault_address: raw_data.reserve_x,
                        balance: reserve_x_balance,
                    },
                    reserve_b: ReserveInfo {
                        vault_address: raw_data.reserve_y,
                        balance: reserve_y_balance,
                    },
                    specific_data: PoolSpecificData::MeteoraDlmm {
                        active_id: raw_data.active_id,
                        bin_step: raw_data.bin_step,
                        oracle: "".to_string(),
                    },
                })
            }
            PoolType::MeteoraDammV2 => {
                let raw_data = parse_meteora_damm_v2_data(&account_data)?;

                // Get token vault balances
                let token_a_vault_pubkey = Pubkey::from_str(&raw_data.token_a_vault)?;
                let token_b_vault_pubkey = Pubkey::from_str(&raw_data.token_b_vault)?;

                let token_a_balance = self.get_token_balance(&token_a_vault_pubkey).await?;
                let token_b_balance = self.get_token_balance(&token_b_vault_pubkey).await?;

                // Get decimals for both tokens
                let token_a_decimals = self.get_token_decimals(&raw_data.token_a_mint).await?;
                let token_b_decimals = self.get_token_decimals(&raw_data.token_b_mint).await?;

                Ok(PoolData {
                    pool_type,
                    token_a: TokenInfo {
                        mint: raw_data.token_a_mint,
                        decimals: token_a_decimals,
                    },
                    token_b: TokenInfo {
                        mint: raw_data.token_b_mint,
                        decimals: token_b_decimals,
                    },
                    reserve_a: ReserveInfo {
                        vault_address: raw_data.token_a_vault,
                        balance: token_a_balance,
                    },
                    reserve_b: ReserveInfo {
                        vault_address: raw_data.token_b_vault,
                        balance: token_b_balance,
                    },
                    specific_data: PoolSpecificData::MeteoraDammV2 {
                        sqrt_price: raw_data.sqrt_price,
                        liquidity: raw_data.liquidity,
                    },
                })
            }
            PoolType::RaydiumLaunchLab => {
                let raw_data = parse_raydium_launchlab_data(&account_data)?;

                // Get token vault balances - handle cases where vaults might not exist
                let base_vault_pubkey = Pubkey::from_str(&raw_data.base_vault)?;
                let quote_vault_pubkey = Pubkey::from_str(&raw_data.quote_vault)?;

                let base_balance = match self.get_token_balance(&base_vault_pubkey).await {
                    Ok(balance) => balance,
                    Err(e) => {
                        log(
                            LogTag::Pool,
                            "WARN",
                            &format!("Failed to get base vault balance, using real_base: {}", e)
                        );
                        raw_data.real_base
                    }
                };

                let quote_balance = match self.get_token_balance(&quote_vault_pubkey).await {
                    Ok(balance) => balance,
                    Err(e) => {
                        log(
                            LogTag::Pool,
                            "WARN",
                            &format!("Failed to get quote vault balance, using real_quote: {}", e)
                        );
                        raw_data.real_quote
                    }
                };

                Ok(PoolData {
                    pool_type,
                    token_a: TokenInfo {
                        mint: raw_data.base_mint,
                        decimals: raw_data.base_decimals,
                    },
                    token_b: TokenInfo {
                        mint: raw_data.quote_mint,
                        decimals: raw_data.quote_decimals,
                    },
                    reserve_a: ReserveInfo {
                        vault_address: raw_data.base_vault,
                        balance: base_balance,
                    },
                    reserve_b: ReserveInfo {
                        vault_address: raw_data.quote_vault,
                        balance: quote_balance,
                    },
                    specific_data: PoolSpecificData::RaydiumLaunchLab {
                        total_base_sell: raw_data.total_base_sell,
                        real_base: raw_data.real_base,
                        real_quote: raw_data.real_quote,
                    },
                })
            }
            PoolType::OrcaWhirlpool => {
                let raw_data = parse_orca_whirlpool_data(&account_data)?;

                // Get decimals for both tokens (required)
                let token_a_decimals = self.get_token_decimals(&raw_data.token_mint_a).await?;
                let token_b_decimals = self.get_token_decimals(&raw_data.token_mint_b).await?;

                // Try to get token vault balances (optional - for fallback calculation)
                let (token_a_balance, token_b_balance) = match
                    (
                        Pubkey::from_str(&raw_data.token_vault_a),
                        Pubkey::from_str(&raw_data.token_vault_b),
                    )
                {
                    (Ok(vault_a), Ok(vault_b)) => {
                        let balance_a = self.get_token_balance(&vault_a).await.unwrap_or(0);
                        let balance_b = self.get_token_balance(&vault_b).await.unwrap_or(0);
                        log(
                            LogTag::Pool,
                            "DEBUG",
                            &format!("Got Orca vault balances: A={}, B={}", balance_a, balance_b)
                        );
                        (balance_a, balance_b)
                    }
                    _ => {
                        log(
                            LogTag::Pool,
                            "WARN",
                            "Failed to get Orca vault balances, using sqrt_price calculation"
                        );
                        (0, 0)
                    }
                };

                Ok(PoolData {
                    pool_type,
                    token_a: TokenInfo {
                        mint: raw_data.token_mint_a,
                        decimals: token_a_decimals,
                    },
                    token_b: TokenInfo {
                        mint: raw_data.token_mint_b,
                        decimals: token_b_decimals,
                    },
                    reserve_a: ReserveInfo {
                        vault_address: raw_data.token_vault_a,
                        balance: token_a_balance,
                    },
                    reserve_b: ReserveInfo {
                        vault_address: raw_data.token_vault_b,
                        balance: token_b_balance,
                    },
                    specific_data: PoolSpecificData::OrcaWhirlpool {
                        sqrt_price: raw_data.sqrt_price,
                        liquidity: raw_data.liquidity,
                        tick_current_index: raw_data.tick_current_index,
                        tick_spacing: raw_data.tick_spacing,
                        fee_rate: raw_data.fee_rate,
                        protocol_fee_rate: raw_data.protocol_fee_rate,
                    },
                })
            }
            PoolType::PumpfunAmm => {
                let raw_data = parse_pumpfun_amm_data(&account_data)?;

                // Get token vault balances
                let base_vault_pubkey = Pubkey::from_str(&raw_data.pool_base_token_account)?;
                let quote_vault_pubkey = Pubkey::from_str(&raw_data.pool_quote_token_account)?;

                let base_balance = self.get_token_balance(&base_vault_pubkey).await?;
                let quote_balance = self.get_token_balance(&quote_vault_pubkey).await?;

                // Get decimals for both tokens
                let base_decimals = self.get_token_decimals(&raw_data.base_mint).await?;
                let quote_decimals = self.get_token_decimals(&raw_data.quote_mint).await?;

                Ok(PoolData {
                    pool_type,
                    token_a: TokenInfo {
                        mint: raw_data.base_mint,
                        decimals: base_decimals,
                    },
                    token_b: TokenInfo {
                        mint: raw_data.quote_mint,
                        decimals: quote_decimals,
                    },
                    reserve_a: ReserveInfo {
                        vault_address: raw_data.pool_base_token_account,
                        balance: base_balance,
                    },
                    reserve_b: ReserveInfo {
                        vault_address: raw_data.pool_quote_token_account,
                        balance: quote_balance,
                    },
                    specific_data: PoolSpecificData::PumpfunAmm {
                        pool_bump: raw_data.pool_bump,
                        index: raw_data.index,
                        creator: raw_data.creator,
                        lp_mint: raw_data.lp_mint,
                        lp_supply: raw_data.lp_supply,
                        coin_creator: raw_data.coin_creator,
                    },
                })
            }
            _ => {
                return Err(anyhow::anyhow!("Unsupported pool type: {:?}", pool_type));
            }
        }
    }

    /// Universal price calculation method with smart SOL/Token orientation
    pub async fn calculate_price_from_pool_data(&self, pool_data: &PoolData) -> Result<f64> {
        // Load decimal cache
        let cache_path = Path::new("decimal_cache.json");
        let mut decimal_cache = match DecimalCache::load_from_file(cache_path) {
            Ok(cache) => {
                debug_log("DEBUG", "Decimal cache loaded successfully");
                cache
            }
            Err(e) => {
                pool_log("WARN", &format!("Failed to load decimal cache: {}", e));
                DecimalCache::new()
            }
        };

        // Get actual token decimals from cache or fetch from chain
        let mints_to_check = vec![pool_data.token_a.mint.clone(), pool_data.token_b.mint.clone()];
        debug_log("DEBUG", &format!("Checking decimals for {} tokens", mints_to_check.len()));

        let decimal_map = match
            fetch_or_cache_decimals(
                &self.rpc_client,
                &mints_to_check,
                &mut decimal_cache,
                cache_path
            ).await
        {
            Ok(map) => {
                debug_log("DEBUG", "Successfully fetched/cached token decimals");
                map
            }
            Err(e) => {
                pool_log("WARN", &format!("Failed to fetch decimals from cache: {}", e));
                debug_log("DEBUG", "Using fallback decimals from pool data");
                // Create fallback map using pool data decimals
                let mut fallback_map = HashMap::new();
                fallback_map.insert(pool_data.token_a.mint.clone(), pool_data.token_a.decimals);
                fallback_map.insert(pool_data.token_b.mint.clone(), pool_data.token_b.decimals);
                fallback_map
            }
        };

        let token_a_decimals = decimal_map
            .get(&pool_data.token_a.mint)
            .copied()
            .unwrap_or(pool_data.token_a.decimals);
        let token_b_decimals = decimal_map
            .get(&pool_data.token_b.mint)
            .copied()
            .unwrap_or(pool_data.token_b.decimals);

        // Calculate UI amounts (considering actual decimals from cache)
        let token_a_ui_amount =
            (pool_data.reserve_a.balance as f64) / (10_f64).powi(token_a_decimals as i32);
        let token_b_ui_amount =
            (pool_data.reserve_b.balance as f64) / (10_f64).powi(token_b_decimals as i32);

        log(
            LogTag::Pool,
            "DEBUG",
            &format!(
                "Token A UI amount: {} (cached decimals: {} vs pool decimals: {}) - {}",
                token_a_ui_amount,
                token_a_decimals,
                pool_data.token_a.decimals,
                if self.is_sol_mint(&pool_data.token_a.mint) {
                    "SOL"
                } else {
                    "TOKEN"
                }
            )
        );
        log(
            LogTag::Pool,
            "DEBUG",
            &format!(
                "Token B UI amount: {} (cached decimals: {} vs pool decimals: {}) - {}",
                token_b_ui_amount,
                token_b_decimals,
                pool_data.token_b.decimals,
                if self.is_sol_mint(&pool_data.token_b.mint) {
                    "SOL"
                } else {
                    "TOKEN"
                }
            )
        );

        // Smart price calculation: Always return SOL per Token regardless of internal ordering
        let (sol_amount, token_amount, sol_symbol, token_symbol) = if
            self.is_sol_mint(&pool_data.token_a.mint)
        {
            // Token A is SOL, Token B is the token
            (token_a_ui_amount, token_b_ui_amount, "SOL", &pool_data.token_b.mint[0..8])
        } else if self.is_sol_mint(&pool_data.token_b.mint) {
            // Token B is SOL, Token A is the token
            (token_b_ui_amount, token_a_ui_amount, "SOL", &pool_data.token_a.mint[0..8])
        } else {
            // Neither is SOL, use original order (Token A per Token B)
            (
                token_a_ui_amount,
                token_b_ui_amount,
                &pool_data.token_a.mint[0..8],
                &pool_data.token_b.mint[0..8],
            )
        };

        // For LaunchLab pools, we can use real_base and real_quote for more accurate pricing
        let price = if pool_data.pool_type == PoolType::RaydiumLaunchLab {
            if
                let PoolSpecificData::RaydiumLaunchLab { real_base, real_quote, .. } =
                    &pool_data.specific_data
            {
                // Use cached decimals for more accurate calculation
                let ui_real_base = (*real_base as f64) / (10_f64).powi(token_a_decimals as i32);
                let ui_real_quote = (*real_quote as f64) / (10_f64).powi(token_b_decimals as i32);

                log(
                    LogTag::Pool,
                    "DEBUG",
                    &format!(
                        "LaunchLab Real Values - Base: {} (raw: {}, decimals: {}), Quote: {} (raw: {}, decimals: {})",
                        ui_real_base,
                        *real_base,
                        token_a_decimals,
                        ui_real_quote,
                        *real_quote,
                        token_b_decimals
                    )
                );

                if ui_real_quote > 0.0 {
                    let adjusted_price = ui_real_base / ui_real_quote;
                    log(
                        LogTag::Pool,
                        "DEBUG",
                        &format!("LaunchLab price calculated using real values: {}", adjusted_price)
                    );
                    adjusted_price
                } else {
                    log(
                        LogTag::Pool,
                        "WARN",
                        "LaunchLab real_quote is zero, falling back to standard calculation"
                    );
                    if token_amount > 0.0 {
                        sol_amount / token_amount
                    } else {
                        0.0
                    }
                }
            } else {
                log(
                    LogTag::Pool,
                    "WARN",
                    "LaunchLab pool missing specific data, using standard calculation"
                );
                if token_amount > 0.0 {
                    sol_amount / token_amount
                } else {
                    0.0
                }
            }
        } else if pool_data.pool_type == PoolType::OrcaWhirlpool {
            if let PoolSpecificData::OrcaWhirlpool { sqrt_price, .. } = &pool_data.specific_data {
                let adjusted_price = self.calculate_price_from_sqrt_price(
                    *sqrt_price,
                    token_a_decimals,
                    token_b_decimals
                );
                log(
                    LogTag::Pool,
                    "DEBUG",
                    &format!("Orca Whirlpool price calculated from sqrt_price: {}", adjusted_price)
                );
                adjusted_price
            } else {
                log(
                    LogTag::Pool,
                    "WARN",
                    "Orca Whirlpool pool missing sqrt_price, using standard calculation"
                );
                if token_amount > 0.0 {
                    sol_amount / token_amount
                } else {
                    0.0
                }
            }
        } else if pool_data.pool_type == PoolType::MeteoraDammV2 {
            if let PoolSpecificData::MeteoraDammV2 { sqrt_price, .. } = &pool_data.specific_data {
                let adjusted_price = self.calculate_price_from_sqrt_price(
                    *sqrt_price,
                    token_a_decimals,
                    token_b_decimals
                );
                log(
                    LogTag::Pool,
                    "DEBUG",
                    &format!("Meteora DAMM v2 price calculated from sqrt_price: {}", adjusted_price)
                );
                adjusted_price
            } else {
                log(
                    LogTag::Pool,
                    "WARN",
                    "Meteora DAMM v2 pool missing sqrt_price, using standard calculation"
                );
                if token_amount > 0.0 {
                    sol_amount / token_amount
                } else {
                    0.0
                }
            }
        } else {
            // Standard AMM calculation for all other pool types
            if token_amount > 0.0 {
                sol_amount / token_amount
            } else {
                0.0
            }
        };

        log(
            LogTag::Pool,
            "DEBUG",
            &format!(
                "Price calculation result: {} {} per {} (using {} calculation)",
                price,
                sol_symbol,
                token_symbol,
                match pool_data.pool_type {
                    PoolType::RaydiumLaunchLab => "LaunchLab real values",
                    PoolType::OrcaWhirlpool => "Whirlpool sqrt_price",
                    PoolType::MeteoraDammV2 => "DAMM v2 sqrt_price",
                    _ => "Standard AMM",
                }
            )
        );

        Ok(price)
    }

    /// Calculate price from sqrt_price for concentrated liquidity pools
    fn calculate_price_from_sqrt_price(
        &self,
        sqrt_price: u128,
        token_a_decimals: u8,
        token_b_decimals: u8
    ) -> f64 {
        // sqrt_price is Q64.96 format (96 fractional bits)
        let q96 = (2_u128).pow(96);
        let price_ratio = (sqrt_price as f64) / (q96 as f64);
        let price_squared = price_ratio * price_ratio;

        // Adjust for token decimals
        let decimal_adjustment = (10_f64).powi(
            (token_a_decimals as i32) - (token_b_decimals as i32)
        );
        price_squared * decimal_adjustment
    }

    /// Check if a mint address is SOL
    fn is_sol_mint(&self, mint: &str) -> bool {
        mint == "So11111111111111111111111111111111111111112"
    }

    /// Get token account balance using RPC
    pub async fn get_token_balance(&self, token_account: &Pubkey) -> Result<u64> {
        let account_info = self.rpc_client.get_account(token_account)?;

        // Parse token account data to get balance
        // Token account balance is stored at offset 64 (8 bytes, little-endian)
        if account_info.data.len() < 72 {
            return Err(anyhow::anyhow!("Token account data too short"));
        }

        let balance_bytes: [u8; 8] = account_info.data[64..72].try_into()?;
        let balance = u64::from_le_bytes(balance_bytes);

        Ok(balance)
    }

    /// Get token decimals from mint account
    pub async fn get_token_decimals(&self, mint_address: &str) -> Result<u8> {
        let mint_pubkey = Pubkey::from_str(mint_address)?;
        let account_info = self.rpc_client.get_account(&mint_pubkey)?;

        // For SPL Token mints, decimals is stored at offset 44 (1 byte)
        if account_info.data.len() < 45 {
            return Err(anyhow::anyhow!("Mint account data too short"));
        }

        // Decimals is at offset 44 (1 byte)
        let decimals = account_info.data[44];

        Ok(decimals)
    }

    /// Get pool metadata (token symbols, names, etc.)
    pub async fn get_pool_metadata(
        &self,
        token_0_mint: &str,
        token_1_mint: &str
    ) -> Result<(String, String)> {
        // This would integrate with your existing token discovery system
        // For now, return mint addresses as symbols
        Ok((
            if token_0_mint == "So11111111111111111111111111111111111111112" {
                "SOL".to_string()
            } else {
                format!("{}..{}", &token_0_mint[..4], &token_0_mint[token_0_mint.len() - 4..])
            },
            format!("{}..{}", &token_1_mint[..4], &token_1_mint[token_1_mint.len() - 4..]),
        ))
    }

    /// Generate a comprehensive pool price report for a token
    pub async fn generate_pool_price_report(&self, token_mint: &str) -> Result<String> {
        let pool_results = self.get_token_pool_prices(token_mint).await?;

        if pool_results.is_empty() {
            return Ok(format!("❌ No pools found for token: {}", token_mint));
        }

        let mut report = String::new();
        report.push_str(&format!("\n🎯 Pool Price Analysis for Token: {}\n", &token_mint[0..8]));
        report.push_str("═══════════════════════════════════════════════════════════════\n");

        let mut sol_pairs = Vec::new();
        let mut other_pairs = Vec::new();

        for result in &pool_results {
            if result.is_sol_pair {
                sol_pairs.push(result);
            } else {
                other_pairs.push(result);
            }
        }

        if !sol_pairs.is_empty() {
            report.push_str("\n💰 SOL-Paired Pools:\n");
            report.push_str("─────────────────────\n");
            for (i, result) in sol_pairs.iter().enumerate() {
                let status = if result.calculation_successful { "✅" } else { "❌" };
                report.push_str(
                    &format!(
                        "{}. {} | {} | ${:.8} | ${:.2}K liq | {:.1}% diff\n",
                        i + 1,
                        status,
                        result.dex_id,
                        result.calculated_price,
                        result.liquidity_usd / 1000.0,
                        result.price_difference_percent
                    )
                );
            }
        }

        if !other_pairs.is_empty() {
            report.push_str("\n🔄 Other Pairs:\n");
            report.push_str("──────────────\n");
            for (i, result) in other_pairs.iter().enumerate() {
                let status = if result.calculation_successful { "✅" } else { "❌" };
                report.push_str(
                    &format!(
                        "{}. {} | {} | ${:.8} | ${:.2}K liq\n",
                        i + 1,
                        status,
                        result.dex_id,
                        result.calculated_price,
                        result.liquidity_usd / 1000.0
                    )
                );
            }
        }

        let successful_count = pool_results
            .iter()
            .filter(|r| r.calculation_successful)
            .count();
        report.push_str(
            &format!(
                "\n📊 Summary: {}/{} pools calculated successfully\n",
                successful_count,
                pool_results.len()
            )
        );

        Ok(report)
    }
}

// =============================================================================
// HELPER FUNCTIONS - Legacy compatibility functions
// =============================================================================

/// Decode Raydium AMM pool data from account - legacy compatibility function
pub fn decode_raydium_amm(
    rpc: &RpcClient,
    pool_pk: &Pubkey,
    acct: &Account
) -> Result<(u64, u64, Pubkey, Pubkey)> {
    if acct.data.len() < 264 {
        return Err(anyhow::anyhow!("AMM account too short"));
    }

    // Extract mint addresses from pool account
    let base_mint = Pubkey::new_from_array(acct.data[168..200].try_into()?);
    let quote_mint = Pubkey::new_from_array(acct.data[216..248].try_into()?);

    let base_vault = Pubkey::new_from_array(acct.data[200..232].try_into()?);
    let quote_vault = Pubkey::new_from_array(acct.data[232..264].try_into()?);

    let base = rpc.get_token_account_balance(&base_vault)?.amount.parse::<u64>().unwrap_or(0);
    let quote = rpc.get_token_account_balance(&quote_vault)?.amount.parse::<u64>().unwrap_or(0);

    use num_format::{ Locale, ToFormattedString };
    println!(
        "✅ Raydium AMM   → Base: {} | Quote: {}",
        base.to_formatted_string(&Locale::en),
        quote.to_formatted_string(&Locale::en)
    );
    Ok((base, quote, base_mint, quote_mint))
}

/// Decode Raydium AMM from account - wrapper function for legacy compatibility
pub fn decode_raydium_amm_from_account(
    rpc: &RpcClient,
    pool_pk: &Pubkey,
    acct: &Account
) -> Result<(u64, u64, Pubkey, Pubkey)> {
    decode_raydium_amm(rpc, pool_pk, acct)
}

/// Legacy compatibility struct and implementation
impl PoolPriceResult {
    fn pool_type_display(&self) -> String {
        match self.pool_type {
            PoolType::RaydiumCpmm => "CPMM".to_string(),
            PoolType::RaydiumAmm => "AMM".to_string(),
            PoolType::MeteoraDlmm => "DLMM".to_string(),
            PoolType::MeteoraDammV2 => "DAMM v2".to_string(),
            PoolType::Orca => "Orca".to_string(),
            PoolType::OrcaWhirlpool => "Whirlpool".to_string(),
            PoolType::Phoenix => "Phoenix".to_string(),
            PoolType::RaydiumLaunchLab => "LaunchLab".to_string(),
            PoolType::PumpfunAmm => "Pump.fun AMM".to_string(),
            PoolType::Unknown => "Unknown".to_string(),
        }
    }
}

// Import required types for legacy functions
use solana_sdk::account::Account;
