/// Pool decoders for different DEX protocols

use chrono::{ DateTime, Utc };

// Raydium decoders

pub mod raydium_cpmm;
pub mod raydium_legacy_amm;
pub mod raydium_clmm;

// Meteora decoders
pub mod meteora_damm_v2;
pub mod meteora_dlmm;

// Orca decoders
pub mod orca_whirlpool;

// Pump.fun decoders
pub mod pump_fun_amm;

/// Pool decoded result structure
/// Contains comprehensive information about a decoded pool
#[derive(Debug, Clone)]
pub struct PoolDecodedResult {
    /// Pool address
    pub pool_address: String,
    /// Program ID that owns this pool
    pub program_id: String,
    /// Pool type identifier (e.g., "CPMM", "Legacy AMM", "CLMM", etc.)
    pub pool_type: String,
    /// Token A mint address
    pub token_a_mint: String,
    /// Token B mint address
    pub token_b_mint: String,
    /// Token A vault/account address
    pub token_a_vault: Option<String>,
    /// Token B vault/account address
    pub token_b_vault: Option<String>,
    /// Token A reserves (raw amount)
    pub token_a_reserve: u64,
    /// Token B reserves (raw amount)
    pub token_b_reserve: u64,
    /// Token A decimals
    pub token_a_decimals: u8,
    /// Token B decimals
    pub token_b_decimals: u8,
    /// LP token mint address (if applicable)
    pub lp_mint: Option<String>,
    /// LP token supply (if applicable)
    pub lp_supply: Option<u64>,
    /// Current price of token A in terms of token B
    pub price_a_to_b: f64,
    /// Current price of token B in terms of token A
    pub price_b_to_a: f64,
    /// Pool liquidity in USD (if calculable)
    pub liquidity_usd: Option<f64>,
    /// Pool status/state (if applicable)
    pub status: Option<u32>,
    /// Additional pool-specific data (for concentrated liquidity pools, etc.)
    pub additional_data: Option<serde_json::Value>,
    /// When this data was decoded
    pub decoded_at: DateTime<Utc>,
}

impl PoolDecodedResult {
    /// Create a new pool decoded result
    pub fn new(
        pool_address: String,
        program_id: String,
        pool_type: String,
        token_a_mint: String,
        token_b_mint: String,
        token_a_reserve: u64,
        token_b_reserve: u64,
        token_a_decimals: u8,
        token_b_decimals: u8
    ) -> Self {
        // Calculate basic price ratios
        let price_a_to_b = if token_b_reserve > 0 && token_a_reserve > 0 {
            (token_b_reserve as f64) /
                (10f64).powi(token_b_decimals as i32) /
                ((token_a_reserve as f64) / (10f64).powi(token_a_decimals as i32))
        } else {
            0.0
        };

        let price_b_to_a = if price_a_to_b > 0.0 { 1.0 / price_a_to_b } else { 0.0 };

        Self {
            pool_address,
            program_id,
            pool_type,
            token_a_mint,
            token_b_mint,
            token_a_vault: None,
            token_b_vault: None,
            token_a_reserve,
            token_b_reserve,
            token_a_decimals,
            token_b_decimals,
            lp_mint: None,
            lp_supply: None,
            price_a_to_b,
            price_b_to_a,
            liquidity_usd: None,
            status: None,
            additional_data: None,
            decoded_at: Utc::now(),
        }
    }

    /// Get price of a specific token in SOL
    pub fn get_token_price_in_sol(&self, token_mint: &str, sol_mint: &str) -> Option<f64> {
        if self.token_a_mint == token_mint && self.token_b_mint == sol_mint {
            Some(self.price_a_to_b)
        } else if self.token_b_mint == token_mint && self.token_a_mint == sol_mint {
            Some(self.price_b_to_a)
        } else {
            None
        }
    }

    /// Check if this pool contains a specific token
    pub fn contains_token(&self, token_mint: &str) -> bool {
        self.token_a_mint == token_mint || self.token_b_mint == token_mint
    }

    /// Get the other token in the pair
    pub fn get_other_token(&self, token_mint: &str) -> Option<&str> {
        if self.token_a_mint == token_mint {
            Some(&self.token_b_mint)
        } else if self.token_b_mint == token_mint {
            Some(&self.token_a_mint)
        } else {
            None
        }
    }
}

/// Pool decoder enum for different pool types
#[derive(Debug)]
pub enum PoolDecoder {
    RaydiumCpmm(raydium_cpmm::RaydiumCpmmDecoder),
    RaydiumLegacyAmm(raydium_legacy_amm::RaydiumLegacyAmmDecoder),
    RaydiumClmm(raydium_clmm::RaydiumClmmDecoder),
    MeteoraDammV2(meteora_damm_v2::MeteoraDammV2Decoder),
    MeteoraDlmm(meteora_dlmm::MeteoraDlmmDecoder),
    OrcaWhirlpool(orca_whirlpool::OrcaWhirlpoolDecoder),
    PumpFunAmm(pump_fun_amm::PumpFunAmmDecoder),
}

impl PoolDecoder {
    /// Check if this decoder can handle the given program ID
    pub fn can_decode(&self, program_id: &str) -> bool {
        match self {
            PoolDecoder::RaydiumCpmm(decoder) => decoder.can_decode(program_id),
            PoolDecoder::RaydiumLegacyAmm(decoder) => decoder.can_decode(program_id),
            PoolDecoder::RaydiumClmm(decoder) => decoder.can_decode(program_id),
            PoolDecoder::MeteoraDammV2(decoder) => decoder.can_decode(program_id),
            PoolDecoder::MeteoraDlmm(decoder) => decoder.can_decode(program_id),
            PoolDecoder::OrcaWhirlpool(decoder) => decoder.can_decode(program_id),
            PoolDecoder::PumpFunAmm(decoder) => decoder.can_decode(program_id),
        }
    }

    /// Decode pool data from account data
    pub async fn decode_pool_data(
        &self,
        pool_address: &str,
        fetcher: &crate::pools::fetcher::PoolFetcher
    ) -> Result<PoolDecodedResult, String> {
        match self {
            PoolDecoder::RaydiumCpmm(decoder) =>
                decoder.decode_pool_data(pool_address, fetcher).await,
            PoolDecoder::RaydiumLegacyAmm(decoder) =>
                decoder.decode_pool_data(pool_address, fetcher).await,
            PoolDecoder::RaydiumClmm(decoder) =>
                decoder.decode_pool_data(pool_address, fetcher).await,
            PoolDecoder::MeteoraDammV2(decoder) =>
                decoder.decode_pool_data(pool_address, fetcher).await,
            PoolDecoder::MeteoraDlmm(decoder) =>
                decoder.decode_pool_data(pool_address, fetcher).await,
            PoolDecoder::OrcaWhirlpool(decoder) =>
                decoder.decode_pool_data(pool_address, fetcher).await,
            PoolDecoder::PumpFunAmm(decoder) =>
                decoder.decode_pool_data(pool_address, fetcher).await,
        }
    }
}

/// Pool decoder factory
pub struct DecoderFactory {
    decoders: Vec<PoolDecoder>,
}

impl DecoderFactory {
    pub fn new() -> Self {
        Self {
            decoders: vec![
                PoolDecoder::RaydiumCpmm(raydium_cpmm::RaydiumCpmmDecoder::new()),
                PoolDecoder::RaydiumLegacyAmm(raydium_legacy_amm::RaydiumLegacyAmmDecoder::new()),
                PoolDecoder::RaydiumClmm(raydium_clmm::RaydiumClmmDecoder::new()),
                PoolDecoder::MeteoraDammV2(meteora_damm_v2::MeteoraDammV2Decoder::new()),
                PoolDecoder::MeteoraDlmm(meteora_dlmm::MeteoraDlmmDecoder::new()),
                PoolDecoder::OrcaWhirlpool(orca_whirlpool::OrcaWhirlpoolDecoder::new()),
                PoolDecoder::PumpFunAmm(pump_fun_amm::PumpFunAmmDecoder::new())
            ],
        }
    }

    /// Get decoder for a program ID
    pub fn get_decoder(&self, program_id: &str) -> Option<&PoolDecoder> {
        self.decoders.iter().find(|d| d.can_decode(program_id))
    }
}
