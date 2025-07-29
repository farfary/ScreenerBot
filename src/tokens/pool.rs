/// Pool Price Calculation System
///
/// This module provides direct pool-based price calculations from Solana blockchain data.
/// It supports multiple pool program IDs with dedicated decoders for each pool type.
///
/// Supported pool types:
/// - Raydium CPMM (CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C)
/// - Future pool types can be added with their own decoders

use crate::logger::{ log, LogTag };
use crate::global::is_debug_price_service_enabled;
use crate::tokens::decimals::{ get_token_decimals_from_chain, get_cached_decimals };
use solana_client::rpc_client::RpcClient;
use solana_sdk::{ account::Account, pubkey::Pubkey, commitment_config::CommitmentConfig };
use std::collections::HashMap;
use std::str::FromStr;
use std::time::Instant;
use tokio::sync::RwLock;
use std::sync::Arc;
use serde::{ Deserialize, Serialize };

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/// Get token decimals with cache fallback to chain lookup
async fn get_token_decimals_with_cache(mint: &str) -> u8 {
    // First try cache
    if let Some(decimals) = get_cached_decimals(mint) {
        return decimals;
    }

    // Fallback to chain lookup
    match get_token_decimals_from_chain(mint).await {
        Ok(decimals) => decimals,
        Err(_) => {
            // Final fallback to defaults
            if mint == SOL_MINT {
                9
            } else {
                6 // Most SPL tokens use 6 decimals
            }
        }
    }
}

// =============================================================================
// POOL PRICE CALCULATION ENTRY POINT
// =============================================================================

/// Raydium CPMM Program ID
pub const RAYDIUM_CPMM_PROGRAM_ID: &str = "CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C";

/// SOL mint address
pub const SOL_MINT: &str = "So11111111111111111111111111111111111111112";

// =============================================================================
// POOL DATA STRUCTURES
// =============================================================================

/// Pool information extracted from on-chain data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolInfo {
    pub pool_address: String,
    pub pool_program_id: String,
    pub pool_type: String,
    pub token_0_mint: String,
    pub token_1_mint: String,
    pub token_0_vault: Option<String>,
    pub token_1_vault: Option<String>,
    pub token_0_reserve: u64,
    pub token_1_reserve: u64,
    pub token_0_decimals: u8,
    pub token_1_decimals: u8,
    pub lp_mint: Option<String>,
    pub lp_supply: Option<u64>,
    pub creator: Option<String>,
    pub status: Option<u8>,
}

/// Pool price calculation result
#[derive(Debug, Clone)]
pub struct PoolPriceInfo {
    pub pool_address: String,
    pub pool_program_id: String,
    pub pool_type: String,
    pub token_mint: String,
    pub price_sol: f64,
    pub token_reserve: u64,
    pub sol_reserve: u64,
    pub token_decimals: u8,
    pub sol_decimals: u8,
    pub liquidity_usd: Option<f64>,
}

/// Raydium CPMM Pool decoded data structure
#[derive(Debug, Clone)]
pub struct RaydiumCpmmPoolData {
    pub amm_config: String,
    pub pool_creator: String,
    pub token_0_vault: String,
    pub token_1_vault: String,
    pub lp_mint: String,
    pub token_0_mint: String,
    pub token_1_mint: String,
    pub token_0_program: String,
    pub token_1_program: String,
    pub observation_key: String,
    pub auth_bump: u8,
    pub status: u8,
    pub lp_mint_decimals: u8,
    pub mint_0_decimals: u8,
    pub mint_1_decimals: u8,
    pub lp_supply: u64,
    pub protocol_fees_token_0: u64,
    pub protocol_fees_token_1: u64,
    pub fund_fees_token_0: u64,
    pub fund_fees_token_1: u64,
    pub open_time: u64,
    pub recent_epoch: u64,
}

/// Pool calculation statistics
#[derive(Debug, Clone)]
pub struct PoolStats {
    pub calculations_attempted: u64,
    pub calculations_successful: u64,
    pub calculations_failed: u64,
    pub cache_hits: u64,
    pub average_calculation_time_ms: f64,
    pub pools_by_program: HashMap<String, u64>,
}

impl PoolStats {
    pub fn new() -> Self {
        Self {
            calculations_attempted: 0,
            calculations_successful: 0,
            calculations_failed: 0,
            cache_hits: 0,
            average_calculation_time_ms: 0.0,
            pools_by_program: HashMap::new(),
        }
    }

    pub fn record_calculation(&mut self, success: bool, time_ms: f64, program_id: &str) {
        self.calculations_attempted += 1;
        if success {
            self.calculations_successful += 1;
        } else {
            self.calculations_failed += 1;
        }

        // Track by program ID
        *self.pools_by_program.entry(program_id.to_string()).or_insert(0) += 1;

        // Update average time
        let total_time =
            self.average_calculation_time_ms * ((self.calculations_attempted - 1) as f64);
        self.average_calculation_time_ms =
            (total_time + time_ms) / (self.calculations_attempted as f64);
    }

    pub fn get_success_rate(&self) -> f64 {
        if self.calculations_attempted == 0 {
            0.0
        } else {
            ((self.calculations_successful as f64) / (self.calculations_attempted as f64)) * 100.0
        }
    }
}

impl std::fmt::Display for PoolStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Pool Stats - Attempted: {}, Success Rate: {:.1}%, Avg Time: {:.1}ms, Programs: {}",
            self.calculations_attempted,
            self.get_success_rate(),
            self.average_calculation_time_ms,
            self.pools_by_program.len()
        )
    }
}

// =============================================================================
// POOL PRICE CALCULATOR
// =============================================================================

/// Advanced pool price calculator with multi-program support
pub struct PoolPriceCalculator {
    rpc_client: Arc<RpcClient>,
    pool_cache: Arc<RwLock<HashMap<String, PoolInfo>>>,
    price_cache: Arc<RwLock<HashMap<String, (f64, Instant)>>>,
    stats: Arc<RwLock<PoolStats>>,
    debug_enabled: bool,
}

impl PoolPriceCalculator {
    /// Create new pool price calculator with default RPC
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        // Use primary RPC from configs
        let rpc_url = Self::get_rpc_url()?;
        Self::new_with_url(&rpc_url)
    }

    /// Create new pool price calculator with custom RPC URL
    pub fn new_with_url(rpc_url: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let rpc_client = Arc::new(
            RpcClient::new_with_commitment(rpc_url.to_string(), CommitmentConfig::confirmed())
        );

        Ok(Self {
            rpc_client,
            pool_cache: Arc::new(RwLock::new(HashMap::new())),
            price_cache: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(PoolStats::new())),
            debug_enabled: false,
        })
    }

    /// Create with optional custom RPC URL (for tool usage)
    pub async fn new_with_rpc(
        rpc_url: Option<&String>
    ) -> Result<Self, Box<dyn std::error::Error>> {
        match rpc_url {
            Some(url) => Self::new_with_url(url),
            None => Self::new(),
        }
    }

    /// Get RPC URL from configs
    fn get_rpc_url() -> Result<String, Box<dyn std::error::Error>> {
        // Try to read from configs.json
        if let Ok(config_content) = std::fs::read_to_string("configs.json") {
            if let Ok(config) = serde_json::from_str::<serde_json::Value>(&config_content) {
                if let Some(rpc_url) = config.get("solana_rpc_url").and_then(|v| v.as_str()) {
                    return Ok(rpc_url.to_string());
                }
            }
        }

        // Fallback to environment variable or default
        Ok(
            std::env
                ::var("SOLANA_RPC_URL")
                .unwrap_or_else(|_| "https://api.mainnet-beta.solana.com".to_string())
        )
    }

    /// Enable debug mode
    pub fn enable_debug(&mut self) {
        self.debug_enabled = true;
        log(LogTag::Pool, "DEBUG", "Pool calculator debug mode enabled");
    }

    /// Get pool information from on-chain data
    pub async fn get_pool_info(&self, pool_address: &str) -> Result<Option<PoolInfo>, String> {
        // Check cache first
        {
            let cache = self.pool_cache.read().await;
            if let Some(cached_pool) = cache.get(pool_address) {
                if self.debug_enabled {
                    log(
                        LogTag::Pool,
                        "CACHE",
                        &format!("Found cached pool info for {}", pool_address)
                    );
                }
                return Ok(Some(cached_pool.clone()));
            }
        }

        let start_time = Instant::now();

        // Parse pool address
        let pool_pubkey = Pubkey::from_str(pool_address).map_err(|e|
            format!("Invalid pool address {}: {}", pool_address, e)
        )?;

        // Get account data
        let account = self.rpc_client
            .get_account(&pool_pubkey)
            .map_err(|e| format!("Failed to get pool account {}: {}", pool_address, e))?;

        // Determine pool type by owner (program ID)
        let program_id = account.owner.to_string();

        if self.debug_enabled {
            log(
                LogTag::Pool,
                "INFO",
                &format!("Pool {} owned by program {}", pool_address, program_id)
            );
        }

        // Decode based on program ID
        let pool_info = match program_id.as_str() {
            RAYDIUM_CPMM_PROGRAM_ID => {
                self.decode_raydium_cpmm_pool(pool_address, &account).await?
            }
            _ => {
                return Err(format!("Unsupported pool program ID: {}", program_id));
            }
        };

        // Cache the result
        {
            let mut cache = self.pool_cache.write().await;
            cache.insert(pool_address.to_string(), pool_info.clone());
        }

        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.record_calculation(true, start_time.elapsed().as_millis() as f64, &program_id);
        }

        if self.debug_enabled {
            log(
                LogTag::Pool,
                "SUCCESS",
                &format!("Pool info decoded in {:.2}ms", start_time.elapsed().as_millis())
            );
        }

        Ok(Some(pool_info))
    }

    /// Calculate token price from pool reserves
    pub async fn calculate_token_price(
        &self,
        pool_address: &str,
        token_mint: &str
    ) -> Result<Option<PoolPriceInfo>, String> {
        let cache_key = format!("{}_{}", pool_address, token_mint);

        // Check price cache (valid for 30 seconds)
        {
            let cache = self.price_cache.read().await;
            if let Some((price, timestamp)) = cache.get(&cache_key) {
                if timestamp.elapsed().as_secs() < 30 {
                    if self.debug_enabled {
                        log(
                            LogTag::Pool,
                            "CACHE",
                            &format!("Using cached price {:.12} SOL for {}", price, token_mint)
                        );
                    }

                    // Update cache hit stats
                    {
                        let mut stats = self.stats.write().await;
                        stats.cache_hits += 1;
                    }

                    // Return cached price with minimal pool info
                    return Ok(
                        Some(PoolPriceInfo {
                            pool_address: pool_address.to_string(),
                            pool_program_id: "cached".to_string(),
                            pool_type: "cached".to_string(),
                            token_mint: token_mint.to_string(),
                            price_sol: *price,
                            token_reserve: 0,
                            sol_reserve: 0,
                            token_decimals: 6, // Default assumption
                            sol_decimals: 9,
                            liquidity_usd: None,
                        })
                    );
                }
            }
        }

        let start_time = Instant::now();

        // Get pool information
        let pool_info = match self.get_pool_info(pool_address).await? {
            Some(info) => info,
            None => {
                return Ok(None);
            }
        };

        // Calculate price based on pool type
        let price_info = match pool_info.pool_program_id.as_str() {
            RAYDIUM_CPMM_PROGRAM_ID => {
                self.calculate_raydium_cpmm_price(&pool_info, token_mint).await?
            }
            _ => {
                return Err(
                    format!(
                        "Price calculation not supported for program: {}",
                        pool_info.pool_program_id
                    )
                );
            }
        };

        // Cache the price
        if let Some(ref price_info) = price_info {
            let mut cache = self.price_cache.write().await;
            cache.insert(cache_key, (price_info.price_sol, Instant::now()));
        }

        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.record_calculation(
                price_info.is_some(),
                start_time.elapsed().as_millis() as f64,
                &pool_info.pool_program_id
            );
        }

        if self.debug_enabled && price_info.is_some() {
            log(
                LogTag::Pool,
                "SUCCESS",
                &format!(
                    "Price calculated: {:.12} SOL for {} in {:.2}ms",
                    price_info.as_ref().unwrap().price_sol,
                    token_mint,
                    start_time.elapsed().as_millis()
                )
            );
        }

        Ok(price_info)
    }

    /// Get multiple account data in a single RPC call (for future optimization)
    pub async fn get_multiple_pool_accounts(
        &self,
        pool_addresses: &[String]
    ) -> Result<HashMap<String, Account>, String> {
        let pubkeys: Result<Vec<Pubkey>, _> = pool_addresses
            .iter()
            .map(|addr| Pubkey::from_str(addr))
            .collect();

        let pubkeys = pubkeys.map_err(|e| format!("Invalid pool address: {}", e))?;

        let accounts = self.rpc_client
            .get_multiple_accounts(&pubkeys)
            .map_err(|e| format!("Failed to get multiple accounts: {}", e))?;

        let mut result = HashMap::new();
        for (i, account_opt) in accounts.into_iter().enumerate() {
            if let Some(account) = account_opt {
                result.insert(pool_addresses[i].clone(), account);
            }
        }

        if self.debug_enabled {
            log(
                LogTag::Pool,
                "RPC",
                &format!("Retrieved {} pool accounts in single call", result.len())
            );
        }

        Ok(result)
    }

    /// Get statistics
    pub async fn get_stats(&self) -> PoolStats {
        self.stats.read().await.clone()
    }

    /// Clear caches
    pub async fn clear_caches(&self) {
        {
            let mut pool_cache = self.pool_cache.write().await;
            pool_cache.clear();
        }
        {
            let mut price_cache = self.price_cache.write().await;
            price_cache.clear();
        }
        log(LogTag::Pool, "CACHE", "Pool and price caches cleared");
    }

    /// Get raw pool account data for debugging
    pub async fn get_raw_pool_data(&self, pool_address: &str) -> Result<Option<Vec<u8>>, String> {
        let pool_pubkey = Pubkey::from_str(pool_address).map_err(|e|
            format!("Invalid pool address: {}", e)
        )?;

        match self.rpc_client.get_account(&pool_pubkey) {
            Ok(account) => Ok(Some(account.data)),
            Err(e) => {
                if e.to_string().contains("not found") {
                    Ok(None)
                } else {
                    Err(format!("Failed to fetch account data: {}", e))
                }
            }
        }
    }
}

// =============================================================================
// RAYDIUM CPMM POOL DECODER
// =============================================================================

impl PoolPriceCalculator {
    /// Decode Raydium CPMM pool data from account bytes
    async fn decode_raydium_cpmm_pool(
        &self,
        pool_address: &str,
        account: &Account
    ) -> Result<PoolInfo, String> {
        if account.data.len() < 8 + 32 * 10 + 8 * 10 {
            return Err("Invalid Raydium CPMM pool account data length".to_string());
        }

        let data = &account.data;
        let mut offset = 8; // Skip discriminator

        // Decode pool data according to Raydium CPMM layout
        let amm_config = Self::read_pubkey_at_offset(data, &mut offset)?;
        let pool_creator = Self::read_pubkey_at_offset(data, &mut offset)?;
        let token_0_vault = Self::read_pubkey_at_offset(data, &mut offset)?;
        let token_1_vault = Self::read_pubkey_at_offset(data, &mut offset)?;
        let lp_mint = Self::read_pubkey_at_offset(data, &mut offset)?;
        let token_0_mint = Self::read_pubkey_at_offset(data, &mut offset)?;
        let token_1_mint = Self::read_pubkey_at_offset(data, &mut offset)?;
        let token_0_program = Self::read_pubkey_at_offset(data, &mut offset)?;
        let token_1_program = Self::read_pubkey_at_offset(data, &mut offset)?;
        let observation_key = Self::read_pubkey_at_offset(data, &mut offset)?;

        let auth_bump = Self::read_u8_at_offset(data, &mut offset)?;
        let status = Self::read_u8_at_offset(data, &mut offset)?;
        let lp_mint_decimals = Self::read_u8_at_offset(data, &mut offset)?;
        let pool_mint_0_decimals = Self::read_u8_at_offset(data, &mut offset)?;
        let pool_mint_1_decimals = Self::read_u8_at_offset(data, &mut offset)?;

        // Use decimal cache system with pool data as fallback
        let mint_0_decimals = get_cached_decimals(&token_0_mint.to_string()).unwrap_or(
            pool_mint_0_decimals
        );
        let mint_1_decimals = get_cached_decimals(&token_1_mint.to_string()).unwrap_or(
            pool_mint_1_decimals
        );

        if is_debug_price_service_enabled() {
            log(
                LogTag::Pool,
                "DECIMALS",
                &format!(
                    "Token0 {} decimals: {} (pool: {}), Token1 {} decimals: {} (pool: {})",
                    token_0_mint.to_string().chars().take(8).collect::<String>(),
                    mint_0_decimals,
                    pool_mint_0_decimals,
                    token_1_mint.to_string().chars().take(8).collect::<String>(),
                    mint_1_decimals,
                    pool_mint_1_decimals
                )
            );
        }

        // Skip padding
        offset += 3;

        let lp_supply = Self::read_u64_at_offset(data, &mut offset)?;
        let _protocol_fees_token_0 = Self::read_u64_at_offset(data, &mut offset)?;
        let _protocol_fees_token_1 = Self::read_u64_at_offset(data, &mut offset)?;
        let _fund_fees_token_0 = Self::read_u64_at_offset(data, &mut offset)?;
        let _fund_fees_token_1 = Self::read_u64_at_offset(data, &mut offset)?;
        let _open_time = Self::read_u64_at_offset(data, &mut offset)?;

        // Get vault balances to calculate reserves
        let (token_0_reserve, token_1_reserve) = self.get_vault_balances(
            &token_0_vault,
            &token_1_vault
        ).await?;

        if self.debug_enabled {
            log(
                LogTag::Pool,
                "DECODE",
                &format!(
                    "Raydium CPMM Pool {} - Token0: {} ({} decimals, {} reserve), Token1: {} ({} decimals, {} reserve)",
                    pool_address,
                    token_0_mint,
                    mint_0_decimals,
                    token_0_reserve,
                    token_1_mint,
                    mint_1_decimals,
                    token_1_reserve
                )
            );
        }

        Ok(PoolInfo {
            pool_address: pool_address.to_string(),
            pool_program_id: RAYDIUM_CPMM_PROGRAM_ID.to_string(),
            pool_type: "Raydium CPMM".to_string(),
            token_0_mint,
            token_1_mint,
            token_0_vault: Some(token_0_vault),
            token_1_vault: Some(token_1_vault),
            token_0_reserve,
            token_1_reserve,
            token_0_decimals: mint_0_decimals,
            token_1_decimals: mint_1_decimals,
            lp_mint: Some(lp_mint),
            lp_supply: Some(lp_supply),
            creator: Some(pool_creator),
            status: Some(status),
        })
    }

    /// Calculate price for Raydium CPMM pool
    async fn calculate_raydium_cpmm_price(
        &self,
        pool_info: &PoolInfo,
        token_mint: &str
    ) -> Result<Option<PoolPriceInfo>, String> {
        // Determine which token is SOL and which is the target token
        let (sol_reserve, token_reserve, sol_decimals, token_decimals, is_token_0) = if
            pool_info.token_0_mint == SOL_MINT &&
            pool_info.token_1_mint == token_mint
        {
            (
                pool_info.token_0_reserve,
                pool_info.token_1_reserve,
                pool_info.token_0_decimals,
                pool_info.token_1_decimals,
                false,
            )
        } else if pool_info.token_1_mint == SOL_MINT && pool_info.token_0_mint == token_mint {
            (
                pool_info.token_1_reserve,
                pool_info.token_0_reserve,
                pool_info.token_1_decimals,
                pool_info.token_0_decimals,
                true,
            )
        } else {
            return Err(format!("Pool does not contain SOL or target token {}", token_mint));
        };

        // Validate reserves
        if sol_reserve == 0 || token_reserve == 0 {
            return Err("Pool has zero reserves".to_string());
        }

        // Calculate price: price = sol_reserve / token_reserve (adjusted for decimals)
        let sol_adjusted = (sol_reserve as f64) / (10_f64).powi(sol_decimals as i32);
        let token_adjusted = (token_reserve as f64) / (10_f64).powi(token_decimals as i32);

        let price_sol = sol_adjusted / token_adjusted;

        // Calculate liquidity in USD (assuming SOL price for USD conversion)
        let sol_price_usd = self.get_sol_price_usd().await.unwrap_or(150.0); // Fallback to $150
        let liquidity_usd = sol_adjusted * 2.0 * sol_price_usd; // Total liquidity is 2x SOL side

        if self.debug_enabled {
            log(
                LogTag::Pool,
                "CALC",
                &format!(
                    "Raydium CPMM Price Calculation:\n  SOL Reserve: {} ({} adjusted)\n  Token Reserve: {} ({} adjusted)\n  Price: {:.12} SOL\n  Liquidity: ${:.2}",
                    sol_reserve,
                    sol_adjusted,
                    token_reserve,
                    token_adjusted,
                    price_sol,
                    liquidity_usd
                )
            );
        }

        Ok(
            Some(PoolPriceInfo {
                pool_address: pool_info.pool_address.clone(),
                pool_program_id: pool_info.pool_program_id.clone(),
                pool_type: pool_info.pool_type.clone(),
                token_mint: token_mint.to_string(),
                price_sol,
                token_reserve,
                sol_reserve,
                token_decimals,
                sol_decimals,
                liquidity_usd: Some(liquidity_usd),
            })
        )
    }

    /// Get vault token balances
    async fn get_vault_balances(&self, vault_0: &str, vault_1: &str) -> Result<(u64, u64), String> {
        let vault_0_pubkey = Pubkey::from_str(vault_0).map_err(|e|
            format!("Invalid vault 0 address {}: {}", vault_0, e)
        )?;
        let vault_1_pubkey = Pubkey::from_str(vault_1).map_err(|e|
            format!("Invalid vault 1 address {}: {}", vault_1, e)
        )?;

        let accounts = self.rpc_client
            .get_multiple_accounts(&[vault_0_pubkey, vault_1_pubkey])
            .map_err(|e| format!("Failed to get vault accounts: {}", e))?;

        let vault_0_account = accounts[0]
            .as_ref()
            .ok_or_else(|| "Vault 0 account not found".to_string())?;
        let vault_1_account = accounts[1]
            .as_ref()
            .ok_or_else(|| "Vault 1 account not found".to_string())?;

        let balance_0 = Self::decode_token_account_amount(&vault_0_account.data)?;
        let balance_1 = Self::decode_token_account_amount(&vault_1_account.data)?;

        if self.debug_enabled {
            log(
                LogTag::Pool,
                "VAULT",
                &format!(
                    "Vault balances - Vault0 ({}): {}, Vault1 ({}): {}",
                    vault_0,
                    balance_0,
                    vault_1,
                    balance_1
                )
            );
        }

        Ok((balance_0, balance_1))
    }

    /// Decode token account amount from account data
    fn decode_token_account_amount(data: &[u8]) -> Result<u64, String> {
        if data.len() < 72 {
            return Err("Invalid token account data length".to_string());
        }

        // Token account amount is at offset 64 (8 bytes)
        let amount_bytes = &data[64..72];
        let amount = u64::from_le_bytes(
            amount_bytes.try_into().map_err(|_| "Failed to parse token account amount".to_string())?
        );

        Ok(amount)
    }

    /// Get current SOL price in USD (simplified)
    async fn get_sol_price_usd(&self) -> Option<f64> {
        // In a real implementation, this would fetch SOL price from a price oracle
        // For now, return a reasonable default
        Some(150.0)
    }

    // Helper functions for reading pool data
    fn read_pubkey_at_offset(data: &[u8], offset: &mut usize) -> Result<String, String> {
        if *offset + 32 > data.len() {
            return Err("Insufficient data for pubkey".to_string());
        }

        let pubkey_bytes = &data[*offset..*offset + 32];
        *offset += 32;

        let pubkey = Pubkey::new_from_array(
            pubkey_bytes.try_into().map_err(|_| "Failed to parse pubkey".to_string())?
        );

        Ok(pubkey.to_string())
    }

    fn read_u8_at_offset(data: &[u8], offset: &mut usize) -> Result<u8, String> {
        if *offset >= data.len() {
            return Err("Insufficient data for u8".to_string());
        }

        let value = data[*offset];
        *offset += 1;
        Ok(value)
    }

    fn read_u64_at_offset(data: &[u8], offset: &mut usize) -> Result<u64, String> {
        if *offset + 8 > data.len() {
            return Err("Insufficient data for u64".to_string());
        }

        let bytes = &data[*offset..*offset + 8];
        *offset += 8;

        let value = u64::from_le_bytes(
            bytes.try_into().map_err(|_| "Failed to parse u64".to_string())?
        );

        Ok(value)
    }
}

// =============================================================================
// PUBLIC API FUNCTIONS
// =============================================================================

/// Get pool price from specific pool address (main API function)
pub async fn get_pool_price_from_address(
    pool_address: &str,
    token_mint: &str
) -> Result<Option<PoolPriceInfo>, String> {
    let calculator = PoolPriceCalculator::new().map_err(|e|
        format!("Failed to create pool calculator: {}", e)
    )?;

    calculator.calculate_token_price(pool_address, token_mint).await
}

/// Get pool price with custom RPC
pub async fn get_pool_price_with_rpc(
    pool_address: &str,
    token_mint: &str,
    rpc_url: &str
) -> Result<Option<PoolPriceInfo>, String> {
    let calculator = PoolPriceCalculator::new_with_url(rpc_url).map_err(|e|
        format!("Failed to create pool calculator: {}", e)
    )?;

    calculator.calculate_token_price(pool_address, token_mint).await
}

/// Batch calculate prices from multiple pools (future optimization)
pub async fn get_multiple_pool_prices(
    pool_token_pairs: &[(String, String)]
) -> Result<HashMap<String, PoolPriceInfo>, String> {
    let calculator = PoolPriceCalculator::new().map_err(|e|
        format!("Failed to create pool calculator: {}", e)
    )?;

    let mut results = HashMap::new();

    // Get all pool addresses for batch RPC call
    let pool_addresses: Vec<String> = pool_token_pairs
        .iter()
        .map(|(pool, _)| pool.clone())
        .collect();

    // Batch fetch pool accounts (optimization for multiple calls)
    let _pool_accounts = calculator.get_multiple_pool_accounts(&pool_addresses).await?;

    // Calculate prices for each pair
    for (pool_address, token_mint) in pool_token_pairs {
        match calculator.calculate_token_price(pool_address, token_mint).await {
            Ok(Some(price_info)) => {
                results.insert(format!("{}_{}", pool_address, token_mint), price_info);
            }
            Ok(None) => {
                log(
                    LogTag::Pool,
                    "WARN",
                    &format!("No price data for pool {} token {}", pool_address, token_mint)
                );
            }
            Err(e) => {
                log(
                    LogTag::Pool,
                    "ERROR",
                    &format!(
                        "Failed to calculate price for pool {} token {}: {}",
                        pool_address,
                        token_mint,
                        e
                    )
                );
            }
        }
    }

    Ok(results)
}

/// Legacy compatibility function
pub async fn get_token_price_from_pools(mint: &str) -> Option<f64> {
    // This function would need a way to discover pools for a given token
    // For now, return None since we need specific pool addresses
    log(
        LogTag::Pool,
        "WARN",
        &format!("get_token_price_from_pools called for {} - pool discovery not implemented yet", mint)
    );
    None
}

/// Decoder function specifically for Raydium CPMM (as requested)
pub async fn decoder_raydium_cpmm(
    pool_address: &str,
    account_data: &[u8]
) -> Result<RaydiumCpmmPoolData, String> {
    if account_data.len() < 8 + 32 * 10 + 8 * 10 {
        return Err("Invalid Raydium CPMM pool account data length".to_string());
    }

    let data = account_data;
    let mut offset = 8; // Skip discriminator

    // Decode all fields according to Raydium CPMM layout
    let amm_config = PoolPriceCalculator::read_pubkey_at_offset(data, &mut offset)?;
    let pool_creator = PoolPriceCalculator::read_pubkey_at_offset(data, &mut offset)?;
    let token_0_vault = PoolPriceCalculator::read_pubkey_at_offset(data, &mut offset)?;
    let token_1_vault = PoolPriceCalculator::read_pubkey_at_offset(data, &mut offset)?;
    let lp_mint = PoolPriceCalculator::read_pubkey_at_offset(data, &mut offset)?;
    let token_0_mint = PoolPriceCalculator::read_pubkey_at_offset(data, &mut offset)?;
    let token_1_mint = PoolPriceCalculator::read_pubkey_at_offset(data, &mut offset)?;
    let token_0_program = PoolPriceCalculator::read_pubkey_at_offset(data, &mut offset)?;
    let token_1_program = PoolPriceCalculator::read_pubkey_at_offset(data, &mut offset)?;
    let observation_key = PoolPriceCalculator::read_pubkey_at_offset(data, &mut offset)?;

    let auth_bump = PoolPriceCalculator::read_u8_at_offset(data, &mut offset)?;
    let status = PoolPriceCalculator::read_u8_at_offset(data, &mut offset)?;
    let lp_mint_decimals = PoolPriceCalculator::read_u8_at_offset(data, &mut offset)?;
    let mint_0_decimals = PoolPriceCalculator::read_u8_at_offset(data, &mut offset)?;
    let mint_1_decimals = PoolPriceCalculator::read_u8_at_offset(data, &mut offset)?;

    // Skip padding
    offset += 3;

    let lp_supply = PoolPriceCalculator::read_u64_at_offset(data, &mut offset)?;
    let protocol_fees_token_0 = PoolPriceCalculator::read_u64_at_offset(data, &mut offset)?;
    let protocol_fees_token_1 = PoolPriceCalculator::read_u64_at_offset(data, &mut offset)?;
    let fund_fees_token_0 = PoolPriceCalculator::read_u64_at_offset(data, &mut offset)?;
    let fund_fees_token_1 = PoolPriceCalculator::read_u64_at_offset(data, &mut offset)?;
    let open_time = PoolPriceCalculator::read_u64_at_offset(data, &mut offset)?;
    let recent_epoch = PoolPriceCalculator::read_u64_at_offset(data, &mut offset)?;

    Ok(RaydiumCpmmPoolData {
        amm_config,
        pool_creator,
        token_0_vault,
        token_1_vault,
        lp_mint,
        token_0_mint,
        token_1_mint,
        token_0_program,
        token_1_program,
        observation_key,
        auth_bump,
        status,
        lp_mint_decimals,
        mint_0_decimals,
        mint_1_decimals,
        lp_supply,
        protocol_fees_token_0,
        protocol_fees_token_1,
        fund_fees_token_0,
        fund_fees_token_1,
        open_time,
        recent_epoch,
    })
}
