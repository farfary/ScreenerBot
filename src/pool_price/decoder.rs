/// Pool Data Decoder Module
///
/// This module handles fetching pool account data using get_multiple_accounts
/// and decoding pool reserves based on program ID classification.

use super::types::*;
use crate::logger::{ log, LogTag };
use crate::global::read_configs;
use crate::decimal_cache::{ DecimalCache, fetch_or_cache_decimals };

use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::Semaphore;
use std::time::Duration;
use tokio::time::timeout;

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/// Debug logging function (conditional based on debug flag)
fn debug_log(log_type: &str, message: &str) {
    // For pool price debugging, we use regular log with Pool tag
    log(LogTag::Pool, log_type, message);
}

// =============================================================================
// POOL DATA DECODER
// =============================================================================

pub struct PoolDecoder {
    rpc_client: RpcClient,
    rate_limiter: Arc<Semaphore>,
}

impl PoolDecoder {
    /// Create new pool decoder instance
    pub fn new() -> Result<Self, PoolPriceError> {
        let configs = read_configs("configs.json").map_err(|e|
            PoolPriceError::SolanaRpc(format!("Failed to read configs: {}", e))
        )?;

        let rpc_client = RpcClient::new(configs.rpc_url);

        Ok(Self {
            rpc_client,
            rate_limiter: Arc::new(Semaphore::new(SOLANA_RPC_RATE_LIMIT_PER_MINUTE as usize)),
        })
    }

    /// Fetch pool data for multiple pools in batch using get_multiple_accounts
    pub async fn fetch_multiple_pool_data(
        &self,
        pool_addresses: &[Pubkey]
    ) -> Result<HashMap<Pubkey, PoolAccountData>, Box<dyn std::error::Error + Send + Sync>> {
        if pool_addresses.is_empty() {
            return Ok(HashMap::new());
        }

        // Acquire rate limit permit once for the entire batch
        let _permit = timeout(Duration::from_secs(30), self.rate_limiter.acquire()).await
            .map_err(|_| "Timeout waiting for RPC rate limit permit for batch fetch")?
            .map_err(|_| "Failed to acquire RPC rate limit permit for batch fetch")?;

        debug_log(
            "BATCH",
            &format!("Fetching data for {} pools using get_multiple_accounts", pool_addresses.len())
        );

        // Use get_multiple_accounts for efficient batch fetching
        let account_infos = self.rpc_client
            .get_multiple_accounts(pool_addresses)
            .map_err(|e| format!("Failed to fetch multiple pool accounts: {}", e))?;

        let mut results = HashMap::new();

        for (i, account_info_opt) in account_infos.iter().enumerate() {
            let pool_address = pool_addresses[i];

            if let Some(account_info) = account_info_opt {
                let account_data = PoolAccountData {
                    address: pool_address,
                    program_id: account_info.owner.to_string(),
                    dex_name: "Unknown".to_string(), // Will be determined later
                    account_data: account_info.data.clone(),
                    liquidity_usd: 0.0, // Will be calculated later
                };
                results.insert(pool_address, account_data);
                debug_log("BATCH", &format!("Successfully fetched pool data for {}", pool_address));
            } else {
                debug_log("WARN", &format!("No account data for pool {}", pool_address));
            }
        }

        debug_log(
            "SUCCESS",
            &format!("Batch fetched {}/{} pool accounts", results.len(), pool_addresses.len())
        );
        Ok(results)
    }
    pub async fn fetch_and_decode_pools(
        &self,
        pool_addresses: &[PoolAddressInfo]
    ) -> PoolPriceResult<Vec<DecodedPoolData>> {
        if pool_addresses.is_empty() {
            return Ok(Vec::new());
        }

        log(LogTag::Pool, "RPC", &format!("Fetching data for {} pools", pool_addresses.len()));

        // Convert addresses to Pubkeys
        let pubkeys: Result<Vec<Pubkey>, _> = pool_addresses
            .iter()
            .map(|info| Pubkey::from_str(&info.address))
            .collect();

        let pubkeys = pubkeys.map_err(|e|
            PoolPriceError::PoolDecoding(format!("Invalid pubkey: {}", e))
        )?;

        // Fetch account data in batches
        let mut all_decoded_pools = Vec::new();

        for chunk in pubkeys.chunks(MULTI_ACCOUNT_BATCH_SIZE) {
            let chunk_pools = self.fetch_and_decode_batch(chunk, pool_addresses).await?;
            all_decoded_pools.extend(chunk_pools);

            // Small delay between batches to respect rate limits
            if pubkeys.len() > MULTI_ACCOUNT_BATCH_SIZE {
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }

        log(
            LogTag::Pool,
            "SUCCESS",
            &format!(
                "Successfully decoded {}/{} pools",
                all_decoded_pools
                    .iter()
                    .filter(|p| p.is_valid)
                    .count(),
                all_decoded_pools.len()
            )
        );

        Ok(all_decoded_pools)
    }

    /// Fetch and decode a batch of pool accounts
    async fn fetch_and_decode_batch(
        &self,
        pubkeys: &[Pubkey],
        pool_info: &[PoolAddressInfo]
    ) -> PoolPriceResult<Vec<DecodedPoolData>> {
        // Acquire rate limit permit
        let _permit = timeout(Duration::from_secs(30), self.rate_limiter.acquire()).await
            .map_err(|_|
                PoolPriceError::RateLimit("Timeout waiting for RPC rate limit permit".to_string())
            )?
            .map_err(|_|
                PoolPriceError::RateLimit("Failed to acquire RPC rate limit permit".to_string())
            )?;

        log(
            LogTag::Pool,
            "DEBUG",
            &format!("RPC get_multiple_accounts for {} addresses", pubkeys.len())
        );

        // Fetch multiple accounts at once
        // Fetch account data using get_multiple_accounts (blocking call)
        let accounts = self.rpc_client
            .get_multiple_accounts(pubkeys)
            .map_err(|e| PoolPriceError::SolanaRpc(format!("Failed to fetch accounts: {}", e)))?;

        let mut decoded_pools = Vec::new();

        // Process each account
        for (i, account_opt) in accounts.iter().enumerate() {
            let pubkey = pubkeys[i];
            let pool_info = &pool_info[i];

            match account_opt {
                Some(account) => {
                    log(
                        LogTag::Pool,
                        "DEBUG",
                        &format!(
                            "Decoding pool {} ({} bytes, owner: {})",
                            pubkey,
                            account.data.len(),
                            account.owner
                        )
                    );

                    // Decode the pool data based on program ID
                    let decoded = self.decode_pool_data(
                        pubkey.to_string(),
                        &pool_info.program_id,
                        &pool_info.dex_name,
                        &account.data,
                        pool_info.liquidity_usd
                    ).await;

                    decoded_pools.push(decoded);
                }
                None => {
                    log(
                        LogTag::Pool,
                        "WARN",
                        &format!("No account data found for pool {}", pubkey)
                    );

                    // Create invalid entry for missing account
                    decoded_pools.push(DecodedPoolData {
                        address: pubkey.to_string(),
                        program_id: pool_info.program_id.clone(),
                        dex_name: pool_info.dex_name.clone(),
                        token_a_mint: String::new(),
                        token_b_mint: String::new(),
                        token_a_reserve: 0,
                        token_b_reserve: 0,
                        token_a_decimals: 0,
                        token_b_decimals: 0,
                        liquidity_usd: pool_info.liquidity_usd,
                        is_valid: false,
                    });
                }
            }
        }

        Ok(decoded_pools)
    }

    /// Decode pool data based on program ID and pool type
    pub async fn decode_pool_data(
        &self,
        address: String,
        program_id: &str,
        dex_name: &str,
        account_data: &[u8],
        liquidity_usd: f64
    ) -> DecodedPoolData {
        let pool_type = PoolType::from_program_id(program_id);

        // Check if pool type is supported
        if !pool_type.is_supported() {
            log(
                LogTag::Pool,
                "WARN",
                &format!("Unsupported pool type: {} for address {}", pool_type.dex_name(), address)
            );

            return DecodedPoolData {
                address,
                program_id: program_id.to_string(),
                dex_name: dex_name.to_string(),
                token_a_mint: String::new(),
                token_b_mint: String::new(),
                token_a_reserve: 0,
                token_b_reserve: 0,
                token_a_decimals: 0,
                token_b_decimals: 0,
                liquidity_usd,
                is_valid: false,
            };
        }

        // Decode based on pool type
        match pool_type {
            PoolType::RaydiumAmm =>
                self.decode_raydium_pool(
                    address,
                    program_id,
                    dex_name,
                    account_data,
                    liquidity_usd
                ).await,
            PoolType::Orca =>
                self.decode_orca_pool(
                    address,
                    program_id,
                    dex_name,
                    account_data,
                    liquidity_usd
                ).await,
            PoolType::MeteoraDlmm =>
                self.decode_meteora_pool(
                    address,
                    program_id,
                    dex_name,
                    account_data,
                    liquidity_usd
                ).await,
            PoolType::PumpFun =>
                self.decode_pumpfun_pool(
                    address,
                    program_id,
                    dex_name,
                    account_data,
                    liquidity_usd
                ).await,
            _ =>
                DecodedPoolData {
                    address,
                    program_id: program_id.to_string(),
                    dex_name: dex_name.to_string(),
                    token_a_mint: String::new(),
                    token_b_mint: String::new(),
                    token_a_reserve: 0,
                    token_b_reserve: 0,
                    token_a_decimals: 0,
                    token_b_decimals: 0,
                    liquidity_usd,
                    is_valid: false,
                },
        }
    }

    /// Decode Raydium AMM pool data
    async fn decode_raydium_pool(
        &self,
        address: String,
        program_id: &str,
        dex_name: &str,
        account_data: &[u8],
        liquidity_usd: f64
    ) -> DecodedPoolData {
        // Raydium AMM pool structure offsets
        // Based on Raydium AMM program account layout

        if account_data.len() < 752 {
            log(
                LogTag::Pool,
                "WARN",
                &format!(
                    "Raydium pool data too short: {} bytes (expected 752+)",
                    account_data.len()
                )
            );
            return self.create_invalid_pool(address, program_id, dex_name, liquidity_usd);
        }

        // Parse pool data using known offsets
        let token_a_mint = self.parse_pubkey_at_offset(account_data, 400); // coinMint offset
        let token_b_mint = self.parse_pubkey_at_offset(account_data, 432); // pcMint offset
        let token_a_reserve = self.parse_u64_at_offset(account_data, 500); // coinVault reserve
        let token_b_reserve = self.parse_u64_at_offset(account_data, 508); // pcVault reserve

        // Validate parsed data
        if
            token_a_mint.is_empty() ||
            token_b_mint.is_empty() ||
            (token_a_reserve == 0 && token_b_reserve == 0)
        {
            log(
                LogTag::Pool,
                "WARN",
                &format!(
                    "Invalid Raydium pool data: mint_a={}, mint_b={}, reserve_a={}, reserve_b={}",
                    token_a_mint,
                    token_b_mint,
                    token_a_reserve,
                    token_b_reserve
                )
            );
            return self.create_invalid_pool(address, program_id, dex_name, liquidity_usd);
        }

        // Fetch token decimals
        // Fetch decimals for both tokens
        let mut decimal_cache = match DecimalCache::load_from_file("decimal_cache.json") {
            Ok(cache) => cache,
            Err(_) => DecimalCache::new(),
        };
        let mints = vec![token_a_mint.clone(), token_b_mint.clone()];
        let decimals_result = fetch_or_cache_decimals(
            &self.rpc_client,
            &mints,
            &mut decimal_cache,
            std::path::Path::new("decimal_cache.json")
        ).await;

        let token_a_decimals = match &decimals_result {
            Ok(decimals_map) => *decimals_map.get(&token_a_mint).unwrap_or(&9),
            Err(_) => 9,
        };

        let token_b_decimals = match &decimals_result {
            Ok(decimals_map) => *decimals_map.get(&token_b_mint).unwrap_or(&9),
            Err(_) => 9,
        };

        log(
            LogTag::Pool,
            "SUCCESS",
            &format!(
                "Decoded Raydium pool {}: {} ({}) / {} ({})",
                address,
                token_a_mint,
                token_a_reserve,
                token_b_mint,
                token_b_reserve
            )
        );

        DecodedPoolData {
            address,
            program_id: program_id.to_string(),
            dex_name: dex_name.to_string(),
            token_a_mint,
            token_b_mint,
            token_a_reserve,
            token_b_reserve,
            token_a_decimals,
            token_b_decimals,
            liquidity_usd,
            is_valid: true,
        }
    }

    /// Decode Orca pool data
    async fn decode_orca_pool(
        &self,
        address: String,
        program_id: &str,
        dex_name: &str,
        account_data: &[u8],
        liquidity_usd: f64
    ) -> DecodedPoolData {
        // Orca pool structure - simplified for now
        // TODO: Implement actual Orca pool decoding offsets

        if account_data.len() < 300 {
            log(
                LogTag::Pool,
                "WARN",
                &format!("Orca pool data too short: {} bytes", account_data.len())
            );
            return self.create_invalid_pool(address, program_id, dex_name, liquidity_usd);
        }

        // For now, return invalid until we implement proper Orca decoding
        log(
            LogTag::Pool,
            "TODO",
            &format!("Orca pool decoding not yet implemented for {}", address)
        );
        self.create_invalid_pool(address, program_id, dex_name, liquidity_usd)
    }

    /// Decode Meteora DLMM pool data
    async fn decode_meteora_pool(
        &self,
        address: String,
        program_id: &str,
        dex_name: &str,
        account_data: &[u8],
        liquidity_usd: f64
    ) -> DecodedPoolData {
        // Meteora DLMM pool structure - simplified for now
        // TODO: Implement actual Meteora pool decoding offsets

        if account_data.len() < 500 {
            log(
                LogTag::Pool,
                "WARN",
                &format!("Meteora pool data too short: {} bytes", account_data.len())
            );
            return self.create_invalid_pool(address, program_id, dex_name, liquidity_usd);
        }

        // For now, return invalid until we implement proper Meteora decoding
        log(
            LogTag::Pool,
            "TODO",
            &format!("Meteora pool decoding not yet implemented for {}", address)
        );
        self.create_invalid_pool(address, program_id, dex_name, liquidity_usd)
    }

    /// Decode PumpFun pool data
    async fn decode_pumpfun_pool(
        &self,
        address: String,
        program_id: &str,
        dex_name: &str,
        account_data: &[u8],
        liquidity_usd: f64
    ) -> DecodedPoolData {
        // PumpFun pool structure - simplified for now
        // TODO: Implement actual PumpFun pool decoding offsets

        if account_data.len() < 200 {
            log(
                LogTag::Pool,
                "WARN",
                &format!("PumpFun pool data too short: {} bytes", account_data.len())
            );
            return self.create_invalid_pool(address, program_id, dex_name, liquidity_usd);
        }

        // For now, return invalid until we implement proper PumpFun decoding
        log(
            LogTag::Pool,
            "TODO",
            &format!("PumpFun pool decoding not yet implemented for {}", address)
        );
        self.create_invalid_pool(address, program_id, dex_name, liquidity_usd)
    }

    /// Helper to create invalid pool data
    fn create_invalid_pool(
        &self,
        address: String,
        program_id: &str,
        dex_name: &str,
        liquidity_usd: f64
    ) -> DecodedPoolData {
        DecodedPoolData {
            address,
            program_id: program_id.to_string(),
            dex_name: dex_name.to_string(),
            token_a_mint: String::new(),
            token_b_mint: String::new(),
            token_a_reserve: 0,
            token_b_reserve: 0,
            token_a_decimals: 0,
            token_b_decimals: 0,
            liquidity_usd,
            is_valid: false,
        }
    }

    /// Parse a Pubkey from account data at specific offset
    fn parse_pubkey_at_offset(&self, data: &[u8], offset: usize) -> String {
        if offset + 32 > data.len() {
            return String::new();
        }

        let pubkey_bytes = &data[offset..offset + 32];
        match Pubkey::try_from(pubkey_bytes) {
            Ok(pubkey) => pubkey.to_string(),
            Err(_) => String::new(),
        }
    }

    /// Parse a u64 from account data at specific offset (little-endian)
    fn parse_u64_at_offset(&self, data: &[u8], offset: usize) -> u64 {
        if offset + 8 > data.len() {
            return 0;
        }

        let bytes = &data[offset..offset + 8];
        u64::from_le_bytes(bytes.try_into().unwrap_or([0; 8]))
    }

    /// Parse a f64 from account data at specific offset (little-endian)
    fn parse_f64_at_offset(&self, data: &[u8], offset: usize) -> f64 {
        if offset + 8 > data.len() {
            return 0.0;
        }

        let bytes = &data[offset..offset + 8];
        f64::from_le_bytes(bytes.try_into().unwrap_or([0; 8]))
    }
}

// =============================================================================
// GLOBAL POOL DECODER INSTANCE
// =============================================================================

use once_cell::sync::Lazy;

/// Global pool decoder instance
pub static POOL_DECODER: Lazy<Result<PoolDecoder, PoolPriceError>> = Lazy::new(||
    PoolDecoder::new()
);

/// Convenience function to fetch and decode pools
pub async fn fetch_and_decode_pools(
    pool_addresses: &[PoolAddressInfo]
) -> PoolPriceResult<Vec<DecodedPoolData>> {
    match &*POOL_DECODER {
        Ok(decoder) => decoder.fetch_and_decode_pools(pool_addresses).await,
        Err(e) =>
            Err(PoolPriceError::SolanaRpc(format!("Pool decoder initialization failed: {}", e))),
    }
}
