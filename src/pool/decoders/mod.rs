pub mod raydium;
pub mod orca;
pub mod meteora;
pub mod pumpfun;
pub mod jupiter;
pub mod serum;

use crate::pool::types::*;
use anyhow::Result;
use async_trait::async_trait;

pub use raydium::RaydiumDecoder;
pub use orca::OrcaDecoder;
pub use meteora::MeteoraDecoder;
pub use pumpfun::PumpfunDecoder;
pub use jupiter::JupiterDecoder;
pub use serum::SerumDecoder;

/// Trait for decoding pool data from different DEX protocols
#[async_trait]
pub trait PoolDecoder: Send + Sync {
    /// Get the pool type this decoder handles
    fn pool_type(&self) -> PoolType;

    /// Check if the given account data matches this pool type
    fn can_decode(&self, account_data: &[u8]) -> bool;

    /// Decode pool information from account data
    async fn decode_pool_info(&self, pool_address: &str, account_data: &[u8]) -> Result<PoolInfo>;

    /// Decode current reserves from account data
    async fn decode_reserves(
        &self,
        pool_address: &str,
        account_data: &[u8],
        slot: u64
    ) -> Result<PoolReserve>;

    /// Get the program ID for this pool type
    fn program_id(&self) -> &str;
}

/// Factory for creating pool decoders
pub struct DecoderFactory;

impl DecoderFactory {
    /// Create all available decoders
    pub fn create_all() -> Vec<Box<dyn PoolDecoder>> {
        vec![
            Box::new(RaydiumDecoder::new()),
            Box::new(OrcaDecoder::new()),
            Box::new(MeteoraDecoder::new()),
            Box::new(PumpfunDecoder::new()),
            Box::new(JupiterDecoder::new()),
            Box::new(SerumDecoder::new())
        ]
    }

    /// Create decoder for specific pool type
    pub fn create_for_type(pool_type: PoolType) -> Option<Box<dyn PoolDecoder>> {
        match pool_type {
            PoolType::Raydium => Some(Box::new(RaydiumDecoder::new())),
            PoolType::Orca => Some(Box::new(OrcaDecoder::new())),
            PoolType::Meteora => Some(Box::new(MeteoraDecoder::new())),
            PoolType::PumpFun => Some(Box::new(PumpfunDecoder::new())),
            PoolType::Jupiter => Some(Box::new(JupiterDecoder::new())),
            PoolType::Serum => Some(Box::new(SerumDecoder::new())),
            PoolType::Unknown => None,
        }
    }

    /// Find decoder that can handle the given account data
    pub fn find_decoder(account_data: &[u8]) -> Option<Box<dyn PoolDecoder>> {
        let decoders = Self::create_all();

        for decoder in decoders {
            if decoder.can_decode(account_data) {
                return Some(decoder);
            }
        }

        None
    }
}

/// Common utility functions for decoders
pub mod utils {
    use solana_sdk::pubkey::Pubkey;
    use std::str::FromStr;

    /// Convert bytes to u64 (little endian)
    pub fn bytes_to_u64(bytes: &[u8]) -> u64 {
        let mut array = [0u8; 8];
        array.copy_from_slice(&bytes[..8]);
        u64::from_le_bytes(array)
    }

    /// Convert bytes to u32 (little endian)
    pub fn bytes_to_u32(bytes: &[u8]) -> u32 {
        let mut array = [0u8; 4];
        array.copy_from_slice(&bytes[..4]);
        u32::from_le_bytes(array)
    }

    /// Convert bytes to f64 (little endian)
    pub fn bytes_to_f64(bytes: &[u8]) -> f64 {
        let mut array = [0u8; 8];
        array.copy_from_slice(&bytes[..8]);
        f64::from_le_bytes(array)
    }

    /// Convert bytes to pubkey
    pub fn bytes_to_pubkey(bytes: &[u8]) -> Result<Pubkey, anyhow::Error> {
        if bytes.len() != 32 {
            return Err(anyhow::anyhow!("Invalid pubkey length"));
        }

        let mut array = [0u8; 32];
        array.copy_from_slice(bytes);
        Ok(Pubkey::from(array))
    }

    /// Check if account data starts with expected discriminator
    pub fn check_discriminator(data: &[u8], expected: &[u8]) -> bool {
        if data.len() < expected.len() {
            return false;
        }

        data[..expected.len()] == *expected
    }

    /// Extract field from account data with bounds checking
    pub fn extract_field<T>(data: &[u8], offset: usize, size: usize) -> Result<T, anyhow::Error>
        where T: From<Vec<u8>>
    {
        if offset + size > data.len() {
            return Err(anyhow::anyhow!("Field extraction out of bounds"));
        }

        let field_data = data[offset..offset + size].to_vec();
        Ok(T::from(field_data))
    }
}
