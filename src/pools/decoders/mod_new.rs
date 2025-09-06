/// Pool decoders for different DEX protocols

use chrono::{ DateTime, Utc };
use std::collections::HashMap;

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
    /// Token A reserve amount (raw amount)
    pub token_a_reserve: u64,
    /// Token B reserve amount (raw amount)
    pub token_b_reserve: u64,
    /// Token A decimals
    pub token_a_decimals: u8,
    /// Token B decimals
    pub token_b_decimals: u8,
    /// Fee rate (percentage, e.g., 0.3 for 0.3%)
    pub fee_rate: f64,
    /// Total liquidity in the pool
    pub liquidity: f64,
    /// 24h volume (if available)
    pub volume_24h: Option<f64>,
    /// 24h fees (if available)
    pub fees_24h: Option<f64>,
    /// Annual Percentage Yield (if available)
    pub apy: Option<f64>,
    /// When this data was decoded
    pub last_updated: DateTime<Utc>,
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
        token_b_decimals: u8,
        fee_rate: f64
    ) -> Self {
        Self {
            pool_address,
            program_id,
            pool_type,
            token_a_mint,
            token_b_mint,
            token_a_reserve,
            token_b_reserve,
            token_a_decimals,
            token_b_decimals,
            fee_rate,
            liquidity: 0.0,
            volume_24h: None,
            fees_24h: None,
            apy: None,
            last_updated: Utc::now(),
        }
    }

    /// Calculate price of token A in terms of token B
    pub fn price_a_to_b(&self) -> f64 {
        if self.token_a_reserve == 0 {
            return 0.0;
        }

        let a_decimal = (self.token_a_reserve as f64) / (10_f64).powi(self.token_a_decimals as i32);
        let b_decimal = (self.token_b_reserve as f64) / (10_f64).powi(self.token_b_decimals as i32);

        if a_decimal > 0.0 {
            b_decimal / a_decimal
        } else {
            0.0
        }
    }

    /// Calculate price of token B in terms of token A
    pub fn price_b_to_a(&self) -> f64 {
        if self.token_b_reserve == 0 {
            return 0.0;
        }

        let a_decimal = (self.token_a_reserve as f64) / (10_f64).powi(self.token_a_decimals as i32);
        let b_decimal = (self.token_b_reserve as f64) / (10_f64).powi(self.token_b_decimals as i32);

        if b_decimal > 0.0 {
            a_decimal / b_decimal
        } else {
            0.0
        }
    }
}

/// Pool decoder enum for different pool types
#[derive(Debug, Clone)]
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

    /// Extract vault addresses from pool account data
    pub fn extract_vault_addresses(&self, pool_data: &[u8]) -> Result<Vec<String>, String> {
        match self {
            PoolDecoder::RaydiumCpmm(decoder) => decoder.extract_vault_addresses(pool_data),
            PoolDecoder::RaydiumLegacyAmm(decoder) => decoder.extract_vault_addresses(pool_data),
            PoolDecoder::RaydiumClmm(decoder) => decoder.extract_vault_addresses(pool_data),
            PoolDecoder::MeteoraDammV2(decoder) => decoder.extract_vault_addresses(pool_data),
            PoolDecoder::MeteoraDlmm(decoder) => decoder.extract_vault_addresses(pool_data),
            PoolDecoder::OrcaWhirlpool(decoder) => decoder.extract_vault_addresses(pool_data),
            PoolDecoder::PumpFunAmm(decoder) => decoder.extract_vault_addresses(pool_data),
        }
    }

    /// Decode pool data from account data
    pub fn decode_pool_data(
        &self,
        pool_data: &[u8],
        reserve_accounts_data: &HashMap<String, Vec<u8>>
    ) -> Result<PoolDecodedResult, String> {
        match self {
            PoolDecoder::RaydiumCpmm(decoder) =>
                decoder.decode_pool_data(pool_data, reserve_accounts_data),
            PoolDecoder::RaydiumLegacyAmm(decoder) =>
                decoder.decode_pool_data(pool_data, reserve_accounts_data),
            PoolDecoder::RaydiumClmm(decoder) =>
                decoder.decode_pool_data(pool_data, reserve_accounts_data),
            PoolDecoder::MeteoraDammV2(decoder) =>
                decoder.decode_pool_data(pool_data, reserve_accounts_data),
            PoolDecoder::MeteoraDlmm(decoder) =>
                decoder.decode_pool_data(pool_data, reserve_accounts_data),
            PoolDecoder::OrcaWhirlpool(decoder) =>
                decoder.decode_pool_data(pool_data, reserve_accounts_data),
            PoolDecoder::PumpFunAmm(decoder) =>
                decoder.decode_pool_data(pool_data, reserve_accounts_data),
        }
    }
}

/// Pool decoder factory
#[derive(Clone)]
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

    /// Get decoder for specific program ID
    pub fn get_decoder(&self, program_id: &str) -> Option<&PoolDecoder> {
        self.decoders.iter().find(|decoder| decoder.can_decode(program_id))
    }

    /// Get all available decoders
    pub fn get_all_decoders(&self) -> &[PoolDecoder] {
        &self.decoders
    }
}
