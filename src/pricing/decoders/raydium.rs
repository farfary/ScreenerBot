use async_trait::async_trait;
use std::collections::HashMap;
use crate::pricing::PoolType;
use super::types::{ DecodedPoolData, PoolDecoder, PoolDecoderError };

/// Raydium AMM Pool decoder
pub struct RaydiumDecoder;

impl RaydiumDecoder {
    pub fn new() -> Self {
        Self
    }

    /// Default Raydium AMM fee rate (0.25%)
    const DEFAULT_FEE_RATE: f64 = 0.0025;

    /// Minimum expected data length for Raydium pools
    const MIN_DATA_LENGTH: usize = 752;

    /// Data offsets for Raydium AMM pool structure
    const TOKEN_A_AMOUNT_OFFSET: usize = 200;
    const TOKEN_B_AMOUNT_OFFSET: usize = 208;
    const TOKEN_A_MINT_OFFSET: usize = 40;
    const TOKEN_B_MINT_OFFSET: usize = 72;
    const TOKEN_A_VAULT_OFFSET: usize = 104;
    const TOKEN_B_VAULT_OFFSET: usize = 136;
    const NONCE_OFFSET: usize = 8;

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
}

impl Default for RaydiumDecoder {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PoolDecoder for RaydiumDecoder {
    async fn decode(&self, data: &[u8]) -> Result<DecodedPoolData, PoolDecoderError> {
        if data.len() < Self::MIN_DATA_LENGTH {
            return Err(PoolDecoderError::InvalidDataLength {
                expected: Self::MIN_DATA_LENGTH,
                actual: data.len(),
            });
        }

        // Parse token amounts
        let token_a_amount = Self::parse_u64_at_offset(data, Self::TOKEN_A_AMOUNT_OFFSET)?;
        let token_b_amount = Self::parse_u64_at_offset(data, Self::TOKEN_B_AMOUNT_OFFSET)?;

        // Parse mint addresses
        let token_a_mint = Self::parse_pubkey_at_offset(data, Self::TOKEN_A_MINT_OFFSET)?;
        let token_b_mint = Self::parse_pubkey_at_offset(data, Self::TOKEN_B_MINT_OFFSET)?;

        // Parse vault addresses
        let token_a_vault = Self::parse_pubkey_at_offset(data, Self::TOKEN_A_VAULT_OFFSET)?;
        let token_b_vault = Self::parse_pubkey_at_offset(data, Self::TOKEN_B_VAULT_OFFSET)?;

        // Parse additional data
        let mut additional_data = HashMap::new();
        additional_data.insert(
            "status".to_string(),
            serde_json::Value::Number(serde_json::Number::from(1))
        );

        if let Some(nonce) = data.get(Self::NONCE_OFFSET) {
            additional_data.insert(
                "nonce".to_string(),
                serde_json::Value::Number(serde_json::Number::from(*nonce))
            );
        }

        Ok(DecodedPoolData {
            pool_type: PoolType::Raydium,
            token_a_mint,
            token_b_mint,
            token_a_vault,
            token_b_vault,
            token_a_amount,
            token_b_amount,
            fee_rate: Self::DEFAULT_FEE_RATE,
            sqrt_price: None,
            tick_current: None,
            liquidity: None,
            additional_data,
        })
    }

    fn supported_pool_type(&self) -> PoolType {
        PoolType::Raydium
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_raydium_decoder_creation() {
        let decoder = RaydiumDecoder::new();
        assert_eq!(decoder.supported_pool_type(), PoolType::Raydium);
    }

    #[test]
    fn test_raydium_decoder_invalid_length() {
        let decoder = RaydiumDecoder::new();
        let short_data = vec![0u8; 100];

        tokio_test::block_on(async {
            let result = decoder.decode(&short_data).await;
            assert!(matches!(result, Err(PoolDecoderError::InvalidDataLength { .. })));
        });
    }
}
