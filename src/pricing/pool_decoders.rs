//! Pool decoder manager for handling multiple DEX protocols
//!
//! This module provides a unified interface for decoding pool data from various
//! DEX protocols on Solana, including auto-detection capabilities.

use std::collections::HashMap;
use crate::pricing::{ PoolInfo, PoolType };

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pricing::PoolType;

    #[test]
    fn test_pool_decoder_manager_creation() {
        let manager = PoolDecoderManager::new();
        assert!(manager.supports_pool_type(&PoolType::Raydium));
        assert!(manager.supports_pool_type(&PoolType::PumpFun));
        assert!(manager.supports_pool_type(&PoolType::Meteora));
        assert!(manager.supports_pool_type(&PoolType::Orca));
        assert!(manager.supports_pool_type(&PoolType::Serum));
    }

    #[test]
    fn test_supported_pool_types() {
        let manager = PoolDecoderManager::new();
        let supported_types = manager.supported_pool_types();

        assert!(supported_types.contains(&PoolType::Raydium));
        assert!(supported_types.contains(&PoolType::PumpFun));
        assert!(supported_types.contains(&PoolType::Meteora));
        assert!(supported_types.contains(&PoolType::Orca));
        assert!(supported_types.contains(&PoolType::Serum));
    }

    #[test]
    fn test_unknown_pool_type_support() {
        let manager = PoolDecoderManager::new();
        let unknown_type = PoolType::Unknown("CustomDEX".to_string());

        // Should not be supported initially
        assert!(!manager.supports_pool_type(&unknown_type));
    }

    #[tokio::test]
    async fn test_invalid_data_handling() {
        let manager = PoolDecoderManager::new();
        let pool_info = PoolInfo {
            pool_type: PoolType::Raydium,
            address: "test".to_string(),
            reserve_0: 0,
            reserve_1: 0,
            token_0: "token0".to_string(),
            token_1: "token1".to_string(),
            liquidity_usd: 0.0,
            volume_24h: 0.0,
            fee_tier: None,
            last_updated: 0,
        };

        // Test with insufficient data
        let short_data = vec![0u8; 10];
        let result = manager.decode_pool_data(&pool_info, &short_data).await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), PoolDecoderError::InvalidDataLength { .. }));
    }

    #[tokio::test]
    async fn test_auto_detection_with_no_matches() {
        let manager = PoolDecoderManager::new();
        let pool_info = PoolInfo {
            pool_type: PoolType::Unknown("NonexistentDEX".to_string()),
            address: "test".to_string(),
            reserve_0: 0,
            reserve_1: 0,
            token_0: "token0".to_string(),
            token_1: "token1".to_string(),
            liquidity_usd: 0.0,
            volume_24h: 0.0,
            fee_tier: None,
            last_updated: 0,
        };

        // Test with data that won't match any decoder
        let invalid_data = vec![0u8; 100];
        let result = manager.decode_pool_data(&pool_info, &invalid_data).await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), PoolDecoderError::AutoDetectionFailed));
    }
}
