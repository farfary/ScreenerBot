/// Pool decoders for different DEX protocols

pub mod raydium;
pub mod meteora;
pub mod orca;
pub mod pumpfun;

use crate::pools::discovery::PoolInfo;

/// Pool decoder trait
pub trait PoolDecoder {
    /// Check if this decoder can handle the given program ID
    fn can_decode(&self, program_id: &str) -> bool;
    
    /// Decode pool data and calculate price
    async fn decode_and_calculate(&self, pool_address: &str, token_mint: &str) -> Result<Option<f64>, String>;
}

/// Pool decoder factory
pub struct DecoderFactory {
    decoders: Vec<Box<dyn PoolDecoder>>,
}

impl DecoderFactory {
    pub fn new() -> Self {
        Self {
            decoders: vec![
                Box::new(raydium::RaydiumDecoder::new()),
                Box::new(meteora::MeteoraDecoder::new()),
                Box::new(orca::OrcaDecoder::new()),
                Box::new(pumpfun::PumpfunDecoder::new()),
            ],
        }
    }

    /// Get decoder for a program ID
    pub fn get_decoder(&self, program_id: &str) -> Option<&dyn PoolDecoder> {
        self.decoders.iter().find(|d| d.can_decode(program_id)).map(|d| d.as_ref())
    }
}
