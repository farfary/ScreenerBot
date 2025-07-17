use async_trait::async_trait;
use std::collections::HashMap;
use crate::market_data::PoolType;
use super::types::{ DecodedPoolData, PoolDecoder, PoolDecoderError };

/// PumpFun bonding curve decoder
pub struct PumpFunDecoder;

impl PumpFunDecoder {
    pub fn new() -> Self {
        Self
    }

    /// PumpFun fee rate (1%)
    const FEE_RATE: f64 = 0.01;

    /// Minimum expected data length for PumpFun pools
    const MIN_DATA_LENGTH: usize = 200;

    /// Wrapped SOL mint address
    const WRAPPED_SOL_MINT: &'static str = "So11111111111111111111111111111111111111112";

    /// Data offsets for PumpFun bonding curve structure
    const TOKEN_MINT_OFFSET: usize = 8;
    const VIRTUAL_TOKEN_RESERVES_OFFSET: usize = 32;
    const VIRTUAL_SOL_RESERVES_OFFSET: usize = 40;
    const REAL_TOKEN_RESERVES_OFFSET: usize = 48;
    const REAL_SOL_RESERVES_OFFSET: usize = 56;

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

    fn parse_token_mint(data: &[u8]) -> Result<String, PoolDecoderError> {
        if Self::TOKEN_MINT_OFFSET + 32 > data.len() {
            return Err(PoolDecoderError::ParseError {
                field: "token_mint".to_string(),
                source: "Insufficient data length".into(),
            });
        }

        Ok(bs58::encode(&data[Self::TOKEN_MINT_OFFSET..Self::TOKEN_MINT_OFFSET + 32]).into_string())
    }
}

impl Default for PumpFunDecoder {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PoolDecoder for PumpFunDecoder {
    async fn decode(&self, data: &[u8]) -> Result<DecodedPoolData, PoolDecoderError> {
        if data.len() < Self::MIN_DATA_LENGTH {
            return Err(PoolDecoderError::InvalidDataLength {
                expected: Self::MIN_DATA_LENGTH,
                actual: data.len(),
            });
        }

        // Parse reserve amounts
        let virtual_token_reserves = Self::parse_u64_at_offset(
            data,
            Self::VIRTUAL_TOKEN_RESERVES_OFFSET
        )?;
        let virtual_sol_reserves = Self::parse_u64_at_offset(
            data,
            Self::VIRTUAL_SOL_RESERVES_OFFSET
        )?;
        let real_token_reserves = Self::parse_u64_at_offset(
            data,
            Self::REAL_TOKEN_RESERVES_OFFSET
        )?;
        let real_sol_reserves = Self::parse_u64_at_offset(data, Self::REAL_SOL_RESERVES_OFFSET)?;

        // Parse token mint
        let token_mint = Self::parse_token_mint(data)?;

        // Build additional data with PumpFun-specific information
        let mut additional_data = HashMap::new();
        additional_data.insert(
            "virtual_token_reserves".to_string(),
            serde_json::Value::Number(serde_json::Number::from(virtual_token_reserves))
        );
        additional_data.insert(
            "virtual_sol_reserves".to_string(),
            serde_json::Value::Number(serde_json::Number::from(virtual_sol_reserves))
        );
        additional_data.insert(
            "real_token_reserves".to_string(),
            serde_json::Value::Number(serde_json::Number::from(real_token_reserves))
        );
        additional_data.insert(
            "real_sol_reserves".to_string(),
            serde_json::Value::Number(serde_json::Number::from(real_sol_reserves))
        );

        Ok(DecodedPoolData {
            pool_type: PoolType::PumpFun,
            token_a_mint: token_mint,
            token_b_mint: Self::WRAPPED_SOL_MINT.to_string(),
            token_a_vault: String::new(), // PumpFun doesn't use traditional vaults
            token_b_vault: String::new(),
            token_a_amount: real_token_reserves,
            token_b_amount: real_sol_reserves,
            fee_rate: Self::FEE_RATE,
            sqrt_price: None,
            tick_current: None,
            liquidity: None,
            additional_data,
        })
    }

    fn supported_pool_type(&self) -> PoolType {
        PoolType::PumpFun
    }
}
