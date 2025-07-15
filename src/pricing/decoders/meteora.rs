use async_trait::async_trait;
use std::collections::HashMap;
use crate::pricing::PoolType;
use super::types::{ DecodedPoolData, PoolDecoder, PoolDecoderError };

/// Meteora DLMM (Dynamic Liquidity Market Maker) decoder
pub struct MeteoraDecoder;

impl MeteoraDecoder {
    pub fn new() -> Self {
        Self
    }

    /// Minimum expected data length for Meteora pools
    const MIN_DATA_LENGTH: usize = 1000;

    /// Data offsets for Meteora DLMM structure
    const TOKEN_X_MINT_OFFSET: usize = 32;
    const TOKEN_Y_MINT_OFFSET: usize = 64;
    const ACTIVE_ID_OFFSET: usize = 200;
    const BIN_STEP_OFFSET: usize = 204;
    const RESERVE_X_OFFSET: usize = 300;
    const RESERVE_Y_OFFSET: usize = 308;

    /// Conversion factor for bin step to fee rate
    const BIN_STEP_TO_FEE_DIVISOR: f64 = 10000.0;

    fn parse_i32_at_offset(data: &[u8], offset: usize) -> Result<i32, PoolDecoderError> {
        if offset + 4 > data.len() {
            return Err(PoolDecoderError::ParseError {
                field: format!("i32 at offset {}", offset),
                source: "Insufficient data length".into(),
            });
        }

        data[offset..offset + 4]
            .try_into()
            .map(i32::from_le_bytes)
            .map_err(|e| PoolDecoderError::ParseError {
                field: format!("i32 at offset {}", offset),
                source: e.to_string().into(),
            })
    }

    fn parse_u16_at_offset(data: &[u8], offset: usize) -> Result<u16, PoolDecoderError> {
        if offset + 2 > data.len() {
            return Err(PoolDecoderError::ParseError {
                field: format!("u16 at offset {}", offset),
                source: "Insufficient data length".into(),
            });
        }

        data[offset..offset + 2]
            .try_into()
            .map(u16::from_le_bytes)
            .map_err(|e| PoolDecoderError::ParseError {
                field: format!("u16 at offset {}", offset),
                source: e.to_string().into(),
            })
    }

    fn parse_u64_at_offset(data: &[u8], offset: usize) -> Result<u64, PoolDecoderError> {
        if offset + 8 > data.len() {
            return Err(PoolDecoderError::ParseError {
                field: format!("u64 at offset {}", offset),
                source: "Insufficient data length".into(),
            });
        }

        data[offset..offset + 8]
            .try_into()
            .map(u64::from_le_bytes)
            .map_err(|e| PoolDecoderError::ParseError {
                field: format!("u64 at offset {}", offset),
                source: e.to_string().into(),
            })
    }

    fn parse_pubkey_at_offset(data: &[u8], offset: usize) -> Result<String, PoolDecoderError> {
        if offset + 32 > data.len() {
            return Err(PoolDecoderError::ParseError {
                field: format!("pubkey at offset {}", offset),
                source: "Insufficient data length".into(),
            });
        }

        Ok(bs58::encode(&data[offset..offset + 32]).into_string())
    }

    fn calculate_fee_rate(bin_step: u16) -> f64 {
        (bin_step as f64) / Self::BIN_STEP_TO_FEE_DIVISOR
    }
}

impl Default for MeteoraDecoder {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PoolDecoder for MeteoraDecoder {
    async fn decode(&self, data: &[u8]) -> Result<DecodedPoolData, PoolDecoderError> {
        if data.len() < Self::MIN_DATA_LENGTH {
            return Err(PoolDecoderError::InvalidDataLength {
                expected: Self::MIN_DATA_LENGTH,
                actual: data.len(),
            });
        }

        // Parse DLMM parameters
        let active_id = Self::parse_i32_at_offset(data, Self::ACTIVE_ID_OFFSET)?;
        let bin_step = Self::parse_u16_at_offset(data, Self::BIN_STEP_OFFSET)?;

        // Parse token mints
        let token_x_mint = Self::parse_pubkey_at_offset(data, Self::TOKEN_X_MINT_OFFSET)?;
        let token_y_mint = Self::parse_pubkey_at_offset(data, Self::TOKEN_Y_MINT_OFFSET)?;

        // Parse reserves (simplified - actual Meteora has complex bin structure)
        let reserve_x = Self::parse_u64_at_offset(data, Self::RESERVE_X_OFFSET)?;
        let reserve_y = Self::parse_u64_at_offset(data, Self::RESERVE_Y_OFFSET)?;

        // Calculate fee rate from bin step
        let fee_rate = Self::calculate_fee_rate(bin_step);

        // Build additional data with Meteora-specific information
        let mut additional_data = HashMap::new();
        additional_data.insert(
            "active_id".to_string(),
            serde_json::Value::Number(serde_json::Number::from(active_id))
        );
        additional_data.insert(
            "bin_step".to_string(),
            serde_json::Value::Number(serde_json::Number::from(bin_step))
        );

        Ok(DecodedPoolData {
            pool_type: PoolType::Meteora,
            token_a_mint: token_x_mint,
            token_b_mint: token_y_mint,
            token_a_vault: String::new(), // Meteora uses complex bin structure
            token_b_vault: String::new(),
            token_a_amount: reserve_x,
            token_b_amount: reserve_y,
            fee_rate,
            sqrt_price: None,
            tick_current: Some(active_id),
            liquidity: None,
            additional_data,
        })
    }

    fn supported_pool_type(&self) -> PoolType {
        PoolType::Meteora
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_meteora_decoder_creation() {
        let decoder = MeteoraDecoder::new();
        assert_eq!(decoder.supported_pool_type(), PoolType::Meteora);
    }

    #[test]
    fn test_meteora_decoder_invalid_length() {
        let decoder = MeteoraDecoder::new();
        let short_data = vec![0u8; 500];

        tokio_test::block_on(async {
            let result = decoder.decode(&short_data).await;
            assert!(matches!(result, Err(PoolDecoderError::InvalidDataLength { .. })));
        });
    }

    #[test]
    fn test_fee_rate_calculation() {
        assert_eq!(MeteoraDecoder::calculate_fee_rate(100), 0.01); // 1%
        assert_eq!(MeteoraDecoder::calculate_fee_rate(25), 0.0025); // 0.25%
        assert_eq!(MeteoraDecoder::calculate_fee_rate(0), 0.0); // 0%
    }
}
