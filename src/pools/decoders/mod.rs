/// Pool decoders for different DEX protocols

pub mod raydium;
pub mod meteora;
pub mod orca;
pub mod pumpfun;

use crate::pools::discovery::PoolInfo;

/// Pool decoder enum for different pool types
#[derive(Debug)]
pub enum PoolDecoder {
    Raydium(raydium::RaydiumDecoder),
    Meteora(meteora::MeteoraDecoder),
    Orca(orca::OrcaDecoder),
    Pumpfun(pumpfun::PumpfunDecoder),
}

impl PoolDecoder {
    /// Check if this decoder can handle the given program ID
    pub fn can_decode(&self, program_id: &str) -> bool {
        match self {
            PoolDecoder::Raydium(decoder) => decoder.can_decode(program_id),
            PoolDecoder::Meteora(decoder) => decoder.can_decode(program_id),
            PoolDecoder::Orca(decoder) => decoder.can_decode(program_id),
            PoolDecoder::Pumpfun(decoder) => decoder.can_decode(program_id),
        }
    }

    /// Decode pool data and calculate price
    pub async fn decode_and_calculate(
        &self,
        pool_address: &str,
        token_mint: &str
    ) -> Result<Option<f64>, String> {
        match self {
            PoolDecoder::Raydium(decoder) =>
                decoder.decode_and_calculate(pool_address, token_mint).await,
            PoolDecoder::Meteora(decoder) =>
                decoder.decode_and_calculate(pool_address, token_mint).await,
            PoolDecoder::Orca(decoder) =>
                decoder.decode_and_calculate(pool_address, token_mint).await,
            PoolDecoder::Pumpfun(decoder) =>
                decoder.decode_and_calculate(pool_address, token_mint).await,
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
                PoolDecoder::Raydium(raydium::RaydiumDecoder::new()),
                PoolDecoder::Meteora(meteora::MeteoraDecoder::new()),
                PoolDecoder::Orca(orca::OrcaDecoder::new()),
                PoolDecoder::Pumpfun(pumpfun::PumpfunDecoder::new())
            ],
        }
    }

    /// Get decoder for a program ID
    pub fn get_decoder(&self, program_id: &str) -> Option<&PoolDecoder> {
        self.decoders.iter().find(|d| d.can_decode(program_id))
    }
}
