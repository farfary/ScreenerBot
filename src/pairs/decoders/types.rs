use serde::{ Deserialize, Serialize };
use solana_sdk::pubkey::Pubkey;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolMetadata {
    pub tick_spacing: Option<u16>,
    pub bin_step: Option<u16>,
    pub fee_growth_global_0: Option<u128>,
    pub fee_growth_global_1: Option<u128>,
    pub protocol_fees_0: Option<u64>,
    pub protocol_fees_1: Option<u64>,
    pub volatility_accumulator: Option<u32>,
    pub active_bin_id: Option<i32>,
    pub open_time: Option<u64>,
    pub last_update_time: Option<u64>,
    pub lp_mint: Option<Pubkey>,
    pub lp_supply: Option<u64>,
    pub creator: Option<Pubkey>,
    pub coin_creator: Option<Pubkey>,
    // Raydium CPMM specific fields
    pub amm_config: Option<Pubkey>,
    pub auth_bump: Option<u8>,
}

impl PoolMetadata {
    pub fn new() -> Self {
        Self {
            tick_spacing: None,
            bin_step: None,
            fee_growth_global_0: None,
            fee_growth_global_1: None,
            protocol_fees_0: None,
            protocol_fees_1: None,
            volatility_accumulator: None,
            active_bin_id: None,
            open_time: None,
            last_update_time: None,
            lp_mint: None,
            lp_supply: None,
            creator: None,
            coin_creator: None,
            amm_config: None,
            auth_bump: None,
        }
    }
}

impl Default for PoolMetadata {
    fn default() -> Self {
        Self {
            tick_spacing: None,
            bin_step: None,
            fee_growth_global_0: None,
            fee_growth_global_1: None,
            protocol_fees_0: None,
            protocol_fees_1: None,
            volatility_accumulator: None,
            active_bin_id: None,
            open_time: None,
            last_update_time: None,
            lp_mint: None,
            lp_supply: None,
            creator: None,
            coin_creator: None,
            amm_config: None,
            auth_bump: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolInfo {
    pub pool_address: Pubkey,
    pub program_id: Pubkey,
    pub pool_type: PoolType,
    pub token_mint_0: Pubkey,
    pub token_mint_1: Pubkey,
    pub token_vault_0: Pubkey,
    pub token_vault_1: Pubkey,
    pub reserve_0: u64,
    pub reserve_1: u64,
    pub decimals_0: u8,
    pub decimals_1: u8,
    pub liquidity: Option<u128>,
    pub sqrt_price: Option<u128>,
    pub current_tick: Option<i32>,
    pub fee_rate: Option<u64>,
    pub status: PoolStatus,
    pub metadata: PoolMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PoolType {
    RaydiumClmm,
    RaydiumCpmm,
    RaydiumV4,
    MeteoraDlmm,
    Whirlpool,
    PumpFunAmm,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PoolStatus {
    Active,
    Paused,
    Inactive,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceInfo {
    pub price: f64,
    pub token_0_symbol: String,
    pub token_1_symbol: String,
    pub pool_type: PoolType,
    pub program_id: Pubkey,
    pub pool_address: Pubkey,
    pub last_update: chrono::DateTime<chrono::Utc>,
}

/// Error types for pool decoders
#[derive(Debug, thiserror::Error)]
pub enum DecoderError {
    #[error("Invalid data length: expected {expected}, got {actual}")] InvalidDataLength {
        expected: usize,
        actual: usize,
    },

    #[error("Invalid price: {reason}")] InvalidPrice {
        reason: String,
    },

    #[error("Invalid pool state: {reason}")] InvalidPoolState {
        reason: String,
    },

    #[error("Unsupported pool type: {pool_type:?}")] UnsupportedPoolType {
        pool_type: String,
    },

    #[error("Missing field: {field}")] MissingField {
        field: String,
    },
}

/// Constants for different program IDs
pub mod program_ids {
    use solana_sdk::pubkey::Pubkey;
    use std::str::FromStr;

    /// Raydium Concentrated Liquidity Market Maker
    pub const RAYDIUM_CLMM: &str = "CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK";

    /// Raydium Constant Product Market Maker
    pub const RAYDIUM_CPMM: &str = "CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C";

    /// Raydium V4 AMM Program
    pub const RAYDIUM_V4: &str = "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8";

    /// Meteora Dynamic Liquidity Market Maker
    pub const METEORA_DLMM: &str = "LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo";

    /// Orca Whirlpools
    pub const WHIRLPOOL: &str = "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc";

    /// Pump.fun AMM
    pub const PUMP_FUN_AMM: &str = "pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA";

    pub fn raydium_clmm() -> Pubkey {
        Pubkey::from_str(RAYDIUM_CLMM).unwrap()
    }

    pub fn raydium_cpmm() -> Pubkey {
        Pubkey::from_str(RAYDIUM_CPMM).unwrap()
    }

    pub fn raydium_v4() -> Pubkey {
        Pubkey::from_str(RAYDIUM_V4).unwrap()
    }

    pub fn meteora_dlmm() -> Pubkey {
        Pubkey::from_str(METEORA_DLMM).unwrap()
    }

    pub fn whirlpool() -> Pubkey {
        Pubkey::from_str(WHIRLPOOL).unwrap()
    }

    pub fn pump_fun_amm() -> Pubkey {
        Pubkey::from_str(PUMP_FUN_AMM).unwrap()
    }
}

/// Helper functions for price calculations
pub mod price_math {
    /// Calculate price from sqrt price (Q64.64)
    pub fn sqrt_price_to_price(sqrt_price: u128) -> f64 {
        let price_q64 = (sqrt_price as f64) / ((1u128 << 64) as f64);
        price_q64 * price_q64
    }

    /// Calculate price from reserves (for constant product AMMs)
    pub fn reserves_to_price(
        reserve_0: u64,
        reserve_1: u64,
        decimals_0: u8,
        decimals_1: u8
    ) -> f64 {
        if reserve_0 == 0 {
            return 0.0;
        }

        let adjusted_reserve_0 = (reserve_0 as f64) / (10_f64).powi(decimals_0 as i32);
        let adjusted_reserve_1 = (reserve_1 as f64) / (10_f64).powi(decimals_1 as i32);

        adjusted_reserve_1 / adjusted_reserve_0
    }

    /// Calculate price from tick (for concentrated liquidity AMMs)
    pub fn tick_to_price(tick: i32) -> f64 {
        (1.0001_f64).powi(tick)
    }
}
