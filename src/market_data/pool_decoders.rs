//! Pool decoder manager for handling multiple DEX protocols
//!
//! This module provides a unified interface for decoding pool data from various
//! DEX protocols on Solana, including auto-detection capabilities.

use std::collections::HashMap;
use crate::market_data::{ PoolInfo, PoolType };

// Import decoder modules
use super::decoders::{
    PoolDecoder as DecoderTrait,
    PoolDecoderError,
    RaydiumDecoder,
    PumpFunDecoder,
    MeteoraDecoder,
    OrcaDecoder,
};

/// Main pool decoder manager that orchestrates different protocol decoders
pub struct PoolDecoderManager {
    raydium_decoder: RaydiumDecoder,
    pumpfun_decoder: PumpFunDecoder,
    meteora_decoder: MeteoraDecoder,
    orca_decoder: OrcaDecoder,
    // Decoder registry for extensibility
    decoder_registry: HashMap<String, Box<dyn DecoderTrait + Send + Sync>>,
}

impl PoolDecoderManager {
    /// Create a new pool decoder manager with all supported decoders
    pub fn new() -> Self {
        Self {
            raydium_decoder: RaydiumDecoder::new(),
            pumpfun_decoder: PumpFunDecoder::new(),
            meteora_decoder: MeteoraDecoder::new(),
            orca_decoder: OrcaDecoder::new(),
            decoder_registry: HashMap::new(),
        }
    }

    /// Register a custom decoder for a specific pool type
    pub fn register_decoder<T>(&mut self, pool_type: String, decoder: T)
        where T: DecoderTrait + Send + Sync + 'static
    {
        self.decoder_registry.insert(pool_type, Box::new(decoder));
    }

    /// Decode pool data based on the pool type information
    pub async fn decode_pool_data(
        &self,
        pool_info: &PoolInfo,
        raw_data: &[u8]
    ) -> Result<DecodedPoolData, PoolDecoderError> {
        match &pool_info.pool_type {
            PoolType::Raydium => self.raydium_decoder.decode(raw_data).await,
            PoolType::PumpFun => self.pumpfun_decoder.decode(raw_data).await,
            PoolType::Meteora => self.meteora_decoder.decode(raw_data).await,
            PoolType::Orca => self.orca_decoder.decode(raw_data).await,
            PoolType::Serum => {
                // Raydium uses Serum markets as the underlying orderbook
                self.raydium_decoder.decode(raw_data).await
            }
            PoolType::Unknown(pool_type_str) => {
                // First try custom registered decoders
                if let Some(decoder) = self.decoder_registry.get(pool_type_str) {
                    return decoder.decode(raw_data).await;
                }

                // Fall back to auto-detection
                self.auto_detect_and_decode(raw_data).await
            }
        }
    }

    /// Attempt to auto-detect pool type and decode accordingly
    async fn auto_detect_and_decode(
        &self,
        raw_data: &[u8]
    ) -> Result<DecodedPoolData, PoolDecoderError> {
        // Define decoder priority order for auto-detection
        let decoders: Vec<&dyn DecoderTrait> = vec![
            &self.raydium_decoder,
            &self.pumpfun_decoder,
            &self.meteora_decoder,
            &self.orca_decoder
        ];

        // Try each decoder in order until one succeeds
        for decoder in decoders {
            if let Ok(data) = decoder.decode(raw_data).await {
                return Ok(data);
            }
        }

        // Try custom decoders if built-in ones fail
        for decoder in self.decoder_registry.values() {
            if let Ok(data) = decoder.decode(raw_data).await {
                return Ok(data);
            }
        }

        Err(PoolDecoderError::AutoDetectionFailed)
    }

    /// Get supported pool types
    pub fn supported_pool_types(&self) -> Vec<PoolType> {
        vec![
            PoolType::Raydium,
            PoolType::PumpFun,
            PoolType::Meteora,
            PoolType::Orca,
            PoolType::Serum
        ]
    }

    /// Check if a pool type is supported
    pub fn supports_pool_type(&self, pool_type: &PoolType) -> bool {
        match pool_type {
            | PoolType::Raydium
            | PoolType::PumpFun
            | PoolType::Meteora
            | PoolType::Orca
            | PoolType::Serum => true,
            PoolType::Unknown(pool_type_str) => self.decoder_registry.contains_key(pool_type_str),
        }
    }
}

impl Default for PoolDecoderManager {
    fn default() -> Self {
        Self::new()
    }
}

// Re-export commonly used types for backward compatibility
pub use super::decoders::DecodedPoolData;

// Legacy type alias for backward compatibility
pub type PoolDecoder = PoolDecoderManager;
