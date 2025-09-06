/// Pool decoders for different DEX protocols

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

use crate::pools::types::PoolDecodedResult;

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
    pub fn decode_pool_data(
        &self,
        pool_address: &str,
        account_data: &[u8]
    ) -> Result<PoolDecodedResult, String> {
        match self {
            PoolDecoder::RaydiumCpmm(decoder) =>
                decoder.decode_pool_data(pool_address, account_data),
            PoolDecoder::RaydiumLegacyAmm(decoder) =>
                decoder.decode_pool_data(pool_address, account_data),
            PoolDecoder::RaydiumClmm(decoder) =>
                decoder.decode_pool_data(pool_address, account_data),
            PoolDecoder::MeteoraDammV2(decoder) =>
                decoder.decode_pool_data(pool_address, account_data),
            PoolDecoder::MeteoraDlmm(decoder) =>
                decoder.decode_pool_data(pool_address, account_data),
            PoolDecoder::OrcaWhirlpool(decoder) =>
                decoder.decode_pool_data(pool_address, account_data),
            PoolDecoder::PumpFunAmm(decoder) =>
                decoder.decode_pool_data(pool_address, account_data),
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
