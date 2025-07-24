use anyhow::Result;
use serde::{ Deserialize, Serialize };
use std::time::{ Duration, Instant };
use crate::logger::{ log, LogTag };

// =============================================================================
// CONSTANTS
// =============================================================================

pub const RAYDIUM_CPMM_PROGRAM_ID: &str = "CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C";
pub const RAYDIUM_AMM_PROGRAM_ID: &str = "RVKd61ztZW9g2VZgPZrFYuXJcZ1t7xvaUo1NkL6MZ5w";
pub const METEORA_DLMM_PROGRAM_ID: &str = "LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo";
pub const METEORA_DAMM_V2_PROGRAM_ID: &str = "cpamdpZCGKUy5JxQXB4dcpGPiikHawvSWAd6mEn1sGG";
pub const RAYDIUM_LAUNCHLAB_PROGRAM_ID: &str = "LanMV9sAd7wArD4vJFi2qDdfnVhFxYSUg6eADduJ3uj";
pub const ORCA_WHIRLPOOL_PROGRAM_ID: &str = "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc";
pub const PUMPFUN_AMM_PROGRAM_ID: &str = "pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA";
pub const PUMPFUN_BONDING_CURVE_PROGRAM_ID: &str = "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8";
pub const DEXSCREENER_API_BASE: &str = "https://api.dexscreener.com/token-pairs/v1/solana";

// Cache expiration time - 2 minutes
pub const CACHE_EXPIRATION_SECONDS: u64 = 120;

// =============================================================================
// DEBUG CONFIGURATION
// =============================================================================

/// Check if debug pool price mode is enabled via command line args
pub fn is_debug_pool_price_enabled() -> bool {
    if let Ok(args) = crate::global::CMD_ARGS.lock() {
        args.contains(&"--debug-pool-price".to_string())
    } else {
        false
    }
}

/// Helper function for conditional debug logging
pub fn debug_log(log_type: &str, message: &str) {
    if is_debug_pool_price_enabled() {
        log(LogTag::Pool, log_type, message);
    }
}

/// Helper function for regular pool logging (always visible)
pub fn pool_log(log_type: &str, message: &str) {
    log(LogTag::Pool, log_type, message);
}

/// Log pool price system summary when debug mode is disabled
pub fn log_pool_summary(operation: &str, success_count: usize, total_count: usize) {
    if !is_debug_pool_price_enabled() && total_count > 0 {
        let success_rate = ((success_count as f64) / (total_count as f64)) * 100.0;
        pool_log(
            "INFO",
            &format!(
                "Pool Price System - {}: {}/{} pools processed ({:.1}% success)",
                operation,
                success_count,
                total_count,
                success_rate
            )
        );
    }
}

// =============================================================================
// POOL TYPE ENUMERATION
// =============================================================================

/// Pool type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum PoolType {
    RaydiumCpmm,
    RaydiumAmm,
    MeteoraDlmm,
    MeteoraDammV2,
    RaydiumLaunchLab,
    Orca,
    OrcaWhirlpool,
    Phoenix,
    PumpfunAmm,
    PumpfunBondingCurve,
    Unknown,
}

impl PoolType {
    pub fn from_dex_id_and_labels(dex_id: &str, labels: &[String]) -> Self {
        debug_log(
            "DEBUG",
            &format!("Determining pool type: dex_id='{}', labels={:?}", dex_id, labels)
        );

        match dex_id.to_lowercase().as_str() {
            "raydium" => {
                if labels.iter().any(|l| l.eq_ignore_ascii_case("CPMM")) {
                    PoolType::RaydiumCpmm
                } else if labels.iter().any(|l| l.eq_ignore_ascii_case("CLMM")) {
                    PoolType::RaydiumCpmm // Treat CLMM similar to CPMM for now
                } else if labels.iter().any(|l| l.eq_ignore_ascii_case("LaunchLab")) {
                    PoolType::RaydiumLaunchLab
                } else if labels.iter().any(|l| l.eq_ignore_ascii_case("AMM")) {
                    PoolType::RaydiumAmm
                } else {
                    // Default to AMM for standard Raydium pools (legacy support)
                    debug_log("DEBUG", "Standard Raydium pool, defaulting to AMM");
                    PoolType::RaydiumAmm
                }
            }
            "launchlab" => PoolType::RaydiumLaunchLab,
            "meteora" => {
                if labels.iter().any(|l| l.eq_ignore_ascii_case("DLMM")) {
                    PoolType::MeteoraDlmm
                } else {
                    debug_log("DEBUG", "Meteora pool without DLMM label, using DAMM V2");
                    PoolType::MeteoraDammV2
                }
            }
            "orca" => {
                if labels.iter().any(|l| l.eq_ignore_ascii_case("Whirlpool")) {
                    PoolType::OrcaWhirlpool
                } else {
                    PoolType::Orca
                }
            }
            "phoenix" => PoolType::Phoenix,
            "pump" | "pump.fun" | "pumpswap" | "pumpfun" => PoolType::PumpfunAmm,
            _ => {
                pool_log("WARN", &format!("Unknown DEX ID: {}", dex_id));
                PoolType::Unknown
            }
        }
    }
}

// =============================================================================
// DATA STRUCTURES
// =============================================================================

/// Pool discovery information from DexScreener API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredPool {
    pub pair_address: String,
    pub dex_id: String,
    pub base_token: PoolToken,
    pub quote_token: PoolToken,
    pub price_native: String,
    pub price_usd: String,
    pub liquidity_usd: f64,
    pub volume_24h: f64,
    pub labels: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolToken {
    pub address: String,
    pub name: String,
    pub symbol: String,
}

/// Universal pool data structure that works for all pool types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolData {
    pub pool_type: PoolType,
    pub token_a: TokenInfo,
    pub token_b: TokenInfo,
    pub reserve_a: ReserveInfo,
    pub reserve_b: ReserveInfo,
    pub specific_data: PoolSpecificData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenInfo {
    pub mint: String,
    pub decimals: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReserveInfo {
    pub vault_address: String,
    pub balance: u64,
}

/// Pool-specific data that varies by pool type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PoolSpecificData {
    RaydiumCpmm {
        lp_mint: String,
        observation_key: String,
    },
    RaydiumAmm {
        base_vault: String,
        quote_vault: String,
    },
    MeteoraDlmm {
        active_id: i32,
        bin_step: u16,
        oracle: String,
    },
    MeteoraDammV2 {
        sqrt_price: u128,
        liquidity: u128,
    },
    RaydiumLaunchLab {
        total_base_sell: u64,
        real_base: u64,
        real_quote: u64,
    },
    OrcaWhirlpool {
        sqrt_price: u128,
        liquidity: u128,
        tick_current_index: i32,
        tick_spacing: u16,
        fee_rate: u16,
        protocol_fee_rate: u16,
    },
    PumpfunAmm {
        pool_bump: u8,
        index: u16,
        creator: String,
        lp_mint: String,
        lp_supply: u64,
        coin_creator: String,
    },
    PumpfunBondingCurve {
        pool_bump: u8,
        index: u16,
        creator: String,
        lp_mint: String,
        lp_supply: u64,
        coin_creator: String,
    },
    Orca {},
    Phoenix {},
    Unknown {},
}

/// Pool price result with on-chain calculated price
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolPriceResult {
    pub pool_address: String,
    pub pool_type: PoolType,
    pub dex_id: String,
    pub token_a_mint: String,
    pub token_b_mint: String,
    pub token_a_symbol: String,
    pub token_b_symbol: String,
    pub calculated_price: f64, // Our calculated price from on-chain data
    pub dexscreener_price: f64, // DexScreener reported price for comparison
    pub price_difference_percent: f64, // Difference between our calc and dexscreener
    pub liquidity_usd: f64,
    pub volume_24h: f64,
    pub is_sol_pair: bool,
    pub calculation_successful: bool,
    pub error_message: Option<String>,
}

// =============================================================================
// CACHE STRUCTURES
// =============================================================================

/// Cache entry for biggest pool per token
#[derive(Debug, Clone)]
pub struct PoolCacheEntry {
    pub pool_result: PoolPriceResult,
    pub cached_at: Instant,
}

/// Cache entry for program IDs per token
#[derive(Debug, Clone)]
pub struct ProgramIdCacheEntry {
    pub program_ids: Vec<String>,
    pub cached_at: Instant,
}

impl PoolCacheEntry {
    pub fn is_expired(&self) -> bool {
        self.cached_at.elapsed() > Duration::from_secs(CACHE_EXPIRATION_SECONDS)
    }
}

impl ProgramIdCacheEntry {
    pub fn is_expired(&self) -> bool {
        self.cached_at.elapsed() > Duration::from_secs(CACHE_EXPIRATION_SECONDS)
    }
}

// =============================================================================
// DECODER DATA STRUCTURES
// =============================================================================

/// Legacy Raydium CPMM pool data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaydiumCpmmData {
    pub token_0_mint: String,
    pub token_1_mint: String,
    pub token_0_vault: String,
    pub token_1_vault: String,
    pub mint_0_decimals: u8,
    pub mint_1_decimals: u8,
    pub status: u8,
}

/// Raydium AMM pool data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaydiumAmmData {
    pub base_mint: String,
    pub quote_mint: String,
    pub base_vault: String,
    pub quote_vault: String,
    pub base_decimals: u8,
    pub quote_decimals: u8,
}

/// Legacy Meteora DLMM pool data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeteoraPoolData {
    pub token_x_mint: String,
    pub token_y_mint: String,
    pub reserve_x: String,
    pub reserve_y: String,
    pub active_id: i32,
    pub bin_step: u16,
    pub status: u8,
}

/// Meteora DAMM v2 pool data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeteoraDammV2Data {
    pub token_a_mint: String,
    pub token_b_mint: String,
    pub token_a_vault: String,
    pub token_b_vault: String,
    pub liquidity: u128,
    pub sqrt_price: u128,
    pub pool_status: u8,
}

/// Raydium LaunchLab pool data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaydiumLaunchLabData {
    pub base_mint: String,
    pub quote_mint: String,
    pub base_vault: String,
    pub quote_vault: String,
    pub base_decimals: u8,
    pub quote_decimals: u8,
    pub total_base_sell: u64,
    pub real_base: u64,
    pub real_quote: u64,
    pub status: u8,
}

/// Orca Whirlpool pool data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrcaWhirlpoolData {
    pub whirlpools_config: String,
    pub token_mint_a: String,
    pub token_mint_b: String,
    pub token_vault_a: String,
    pub token_vault_b: String,
    pub fee_rate: u16,
    pub protocol_fee_rate: u16,
    pub liquidity: u128,
    pub sqrt_price: u128,
    pub tick_current_index: i32,
    pub tick_spacing: u16,
    pub protocol_fee_owed_a: u64,
    pub protocol_fee_owed_b: u64,
    pub fee_growth_global_a: u128,
    pub fee_growth_global_b: u128,
    pub whirlpool_bump: u8,
}

#[derive(Debug, Clone)]
pub struct PumpfunAmmData {
    pub pool_bump: u8,
    pub index: u16,
    pub creator: String,
    pub base_mint: String,
    pub quote_mint: String,
    pub lp_mint: String,
    pub pool_base_token_account: String,
    pub pool_quote_token_account: String,
    pub lp_supply: u64,
    pub coin_creator: String,
}
