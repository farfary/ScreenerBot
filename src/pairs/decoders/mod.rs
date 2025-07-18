pub mod raydium_clmm;
pub mod raydium_cpmm;
pub mod raydium_v4;
pub mod meteora_dlmm;
pub mod whirlpool;
pub mod pump_fun_amm;
pub mod types;

use anyhow::Result;
use async_trait::async_trait;
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;

pub use types::{
    PoolInfo,
    PoolType,
    PoolStatus,
    PoolMetadata,
    DecoderError,
    PriceInfo,
    program_ids,
    price_math,
};
pub use raydium_clmm::RaydiumClmmDecoder;
pub use raydium_cpmm::RaydiumCpmmDecoder;
pub use raydium_v4::RaydiumV4Decoder;
pub use meteora_dlmm::MeteoraDlmmDecoder;
pub use whirlpool::WhirlpoolDecoder;
pub use pump_fun_amm::PumpFunAmmDecoder;

/// Common trait for all pool decoders
#[async_trait]
pub trait PoolDecoder: Send + Sync {
    /// Decode pool data from raw account data
    fn decode(&self, data: &[u8]) -> Result<PoolInfo>;

    /// Calculate price from pool reserves
    fn calculate_price(&self, pool_info: &PoolInfo) -> Result<f64>;

    /// Get the program ID this decoder handles
    fn program_id(&self) -> Pubkey;

    /// Get human-readable name for this decoder
    fn name(&self) -> &'static str;
}

/// Pool decoder registry that manages different decoder types
pub struct DecoderRegistry {
    decoders: HashMap<Pubkey, Box<dyn PoolDecoder>>,
}

impl DecoderRegistry {
    /// Create a new decoder registry with all supported decoders
    pub fn new() -> Self {
        let mut decoders: HashMap<Pubkey, Box<dyn PoolDecoder>> = HashMap::new();

        // Register Raydium CLMM decoder
        let raydium_decoder = Box::new(RaydiumClmmDecoder::new());
        decoders.insert(raydium_decoder.program_id(), raydium_decoder);

        // Register Raydium CPMM decoder
        let raydium_cpmm_decoder = Box::new(RaydiumCpmmDecoder::new());
        decoders.insert(raydium_cpmm_decoder.program_id(), raydium_cpmm_decoder);

        // Register Raydium V4 decoder
        let raydium_v4_decoder = Box::new(RaydiumV4Decoder::new());
        decoders.insert(raydium_v4_decoder.program_id(), raydium_v4_decoder);

        // Register Meteora DLMM decoder
        let meteora_decoder = Box::new(MeteoraDlmmDecoder::new());
        decoders.insert(meteora_decoder.program_id(), meteora_decoder);

        // Register Whirlpool decoder
        let whirlpool_decoder = Box::new(WhirlpoolDecoder::new());
        decoders.insert(whirlpool_decoder.program_id(), whirlpool_decoder);

        // Register Pump.fun AMM decoder
        let pump_fun_decoder = Box::new(PumpFunAmmDecoder::new());
        decoders.insert(pump_fun_decoder.program_id(), pump_fun_decoder);

        Self { decoders }
    }

    /// Get decoder for a specific program ID
    pub fn get_decoder(&self, program_id: &Pubkey) -> Option<&dyn PoolDecoder> {
        self.decoders.get(program_id).map(|d| d.as_ref())
    }

    /// Get all registered program IDs
    pub fn get_supported_programs(&self) -> Vec<Pubkey> {
        self.decoders.keys().cloned().collect()
    }

    /// Decode pool data using the appropriate decoder
    pub fn decode_pool(&self, program_id: &Pubkey, data: &[u8]) -> Result<PoolInfo> {
        let decoder = self
            .get_decoder(program_id)
            .ok_or_else(|| anyhow::anyhow!("No decoder found for program ID: {}", program_id))?;

        decoder.decode(data)
    }

    /// Calculate price from pool data
    pub fn calculate_price(&self, program_id: &Pubkey, pool_info: &PoolInfo) -> Result<f64> {
        let decoder = self
            .get_decoder(program_id)
            .ok_or_else(|| anyhow::anyhow!("No decoder found for program ID: {}", program_id))?;

        decoder.calculate_price(pool_info)
    }
}

impl Default for DecoderRegistry {
    fn default() -> Self {
        Self::new()
    }
}
