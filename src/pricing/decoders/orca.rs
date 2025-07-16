use async_trait::async_trait;
use std::collections::HashMap;
use crate::pricing::PoolType;
use super::types::{ DecodedPoolData, PoolDecoder, PoolDecoderError };

/// Orca Whirlpool decoder for concentrated liquidity pools
pub struct OrcaDecoder;

impl OrcaDecoder {
    pub fn new() -> Self {
        Self
    }

    /// Minimum expected data length for Orca Whirlpools
    const MIN_DATA_LENGTH: usize = 600;

    /// Fee rate conversion factor (hundredths of a bip to decimal)
    const FEE_RATE_DIVISOR: f64 = 1_000_000.0;

    /// Data offsets for Orca Whirlpool structure
    const TOKEN_MINT_A_OFFSET: usize = 8;
    const TOKEN_MINT_B_OFFSET: usize = 40;
    const TOKEN_VAULT_A_OFFSET: usize = 72;
    const TOKEN_VAULT_B_OFFSET: usize = 104;
    const SQRT_PRICE_OFFSET: usize = 100;
    const TICK_CURRENT_INDEX_OFFSET: usize = 116;
    const LIQUIDITY_OFFSET: usize = 120;
    const FEE_RATE_OFFSET: usize = 200;
    const TICK_SPACING_OFFSET: usize = 202;
    const PROTOCOL_FEE_RATE_OFFSET: usize = 204;

    fn parse_u128_at_offset(data: &[u8], offset: usize) -> Result<u128, PoolDecoderError> {
        if offset + 16 > data.len() {
            return Err(PoolDecoderError::ParseError {
                field: format!("u128 at offset {}", offset),
                source: "Insufficient data length".into(),
            });
        }

        data[offset..offset + 16]
            .try_into()
            .map(u128::from_le_bytes)
            .map_err(|e| PoolDecoderError::ParseError {
                field: format!("u128 at offset {}", offset),
                source: e.to_string().into(),
            })
    }

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

    fn parse_pubkey_at_offset(data: &[u8], offset: usize) -> Result<String, PoolDecoderError> {
        if offset + 32 > data.len() {
            return Err(PoolDecoderError::ParseError {
                field: format!("pubkey at offset {}", offset),
                source: "Insufficient data length".into(),
            });
        }

        Ok(bs58::encode(&data[offset..offset + 32]).into_string())
    }

    fn parse_u8_at_offset(data: &[u8], offset: usize) -> Result<u8, PoolDecoderError> {
        data.get(offset)
            .copied()
            .ok_or_else(|| PoolDecoderError::ParseError {
                field: format!("u8 at offset {}", offset),
                source: "Offset out of bounds".into(),
            })
    }

    fn calculate_fee_rate(fee_rate_raw: u16) -> f64 {
        (fee_rate_raw as f64) / Self::FEE_RATE_DIVISOR
    }
}

impl Default for OrcaDecoder {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PoolDecoder for OrcaDecoder {
    async fn decode(&self, data: &[u8]) -> Result<DecodedPoolData, PoolDecoderError> {
        if data.len() < Self::MIN_DATA_LENGTH {
            return Err(PoolDecoderError::InvalidDataLength {
                expected: Self::MIN_DATA_LENGTH,
                actual: data.len(),
            });
        }

        // Parse concentrated liquidity parameters
        let sqrt_price = Self::parse_u128_at_offset(data, Self::SQRT_PRICE_OFFSET)?;
        let tick_current_index = Self::parse_i32_at_offset(data, Self::TICK_CURRENT_INDEX_OFFSET)?;
        let liquidity = Self::parse_u128_at_offset(data, Self::LIQUIDITY_OFFSET)?;

        // Parse token mints and vaults
        let token_mint_a = Self::parse_pubkey_at_offset(data, Self::TOKEN_MINT_A_OFFSET)?;
        let token_mint_b = Self::parse_pubkey_at_offset(data, Self::TOKEN_MINT_B_OFFSET)?;
        let token_vault_a = Self::parse_pubkey_at_offset(data, Self::TOKEN_VAULT_A_OFFSET)?;
        let token_vault_b = Self::parse_pubkey_at_offset(data, Self::TOKEN_VAULT_B_OFFSET)?;

        // Parse fee configuration
        let fee_rate_raw = Self::parse_u16_at_offset(data, Self::FEE_RATE_OFFSET)?;
        let fee_rate = Self::calculate_fee_rate(fee_rate_raw);

        // Parse additional pool parameters
        let tick_spacing = Self::parse_u8_at_offset(data, Self::TICK_SPACING_OFFSET)?;
        let protocol_fee_rate = Self::parse_u8_at_offset(data, Self::PROTOCOL_FEE_RATE_OFFSET)?;

        // Build additional data with Orca-specific information
        let mut additional_data = HashMap::new();
        additional_data.insert(
            "tick_spacing".to_string(),
            serde_json::Value::Number(serde_json::Number::from(tick_spacing))
        );
        additional_data.insert(
            "protocol_fee_rate".to_string(),
            serde_json::Value::Number(serde_json::Number::from(protocol_fee_rate))
        );
        additional_data.insert(
            "fee_rate_raw".to_string(),
            serde_json::Value::Number(serde_json::Number::from(fee_rate_raw))
        );

        Ok(DecodedPoolData {
            pool_type: PoolType::Orca,
            token_a_mint: token_mint_a,
            token_b_mint: token_mint_b,
            token_a_vault: token_vault_a,
            token_b_vault: token_vault_b,
            token_a_amount: 0, // Requires separate vault account queries
            token_b_amount: 0, // Requires separate vault account queries
            fee_rate,
            sqrt_price: Some(sqrt_price),
            tick_current: Some(tick_current_index),
            liquidity: Some(liquidity),
            additional_data,
        })
    }

    fn supported_pool_type(&self) -> PoolType {
        PoolType::Orca
    }
}
