use super::{
    PoolDecoder,
    PoolInfo,
    PoolType,
    PoolStatus,
    PoolMetadata,
    DecoderError,
    program_ids,
    price_math,
};
use anyhow::Result;
use async_trait::async_trait;
use solana_sdk::pubkey::Pubkey;

/// Raydium V4 AMM decoder
pub struct RaydiumV4Decoder;

impl RaydiumV4Decoder {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl PoolDecoder for RaydiumV4Decoder {
    fn decode(&self, data: &[u8]) -> Result<PoolInfo> {
        // Raydium V4 AMM pool structure parsing
        if data.len() < 752 {
            return Err(
                (DecoderError::InvalidDataLength {
                    expected: 752,
                    actual: data.len(),
                }).into()
            );
        }

        // Parse discriminator (8 bytes at start)
        let _discriminator = u64::from_le_bytes([
            data[0],
            data[1],
            data[2],
            data[3],
            data[4],
            data[5],
            data[6],
            data[7],
        ]);

        // Based on pool debug analysis, we found SOL at offset 400 and USDC at offset 432
        let token_mint_0 = Pubkey::try_from(&data[400..432])?; // SOL
        let token_mint_1 = Pubkey::try_from(&data[432..464])?; // USDC

        // For Raydium V4, we need to find the vault addresses
        // These are typically stored after the token mints
        let token_vault_0 = Pubkey::try_from(&data[464..496])?;
        let token_vault_1 = Pubkey::try_from(&data[496..528])?;

        // Parse pool coin decimals and pc decimals (typically at fixed offsets)
        let coin_decimals = data[16]; // Based on common Raydium V4 structure
        let pc_decimals = data[17];

        // Parse status - Raydium V4 uses different status encoding
        let status_byte = data[24];
        let status = match status_byte {
            1 => PoolStatus::Active,
            0 => PoolStatus::Inactive,
            _ => PoolStatus::Unknown,
        };

        // For reserves, we'll need to fetch them separately via token account data
        // Set placeholder values for now - they'll be updated by pool fetcher
        let reserve_0 = 0;
        let reserve_1 = 0;

        // Parse open time if available (usually near the end of the structure)
        let open_time = if data.len() >= 700 {
            let open_time_bytes: [u8; 8] = data[692..700].try_into().unwrap_or([0; 8]);
            Some(u64::from_le_bytes(open_time_bytes))
        } else {
            None
        };

        // Create metadata
        let metadata = PoolMetadata {
            tick_spacing: None,
            bin_step: None,
            fee_growth_global_0: None,
            fee_growth_global_1: None,
            protocol_fees_0: None,
            protocol_fees_1: None,
            volatility_accumulator: None,
            active_bin_id: None,
            open_time,
            last_update_time: None,
            lp_mint: None, // Would need to parse LP mint from data
            lp_supply: None,
            creator: None,
            coin_creator: None,
            amm_config: None,
            auth_bump: None,
        };

        Ok(PoolInfo {
            pool_address: Pubkey::default(), // Will be set by caller
            program_id: program_ids::raydium_v4(),
            pool_type: PoolType::RaydiumV4,
            token_mint_0,
            token_mint_1,
            token_vault_0,
            token_vault_1,
            reserve_0,
            reserve_1,
            decimals_0: coin_decimals,
            decimals_1: pc_decimals,
            liquidity: None, // V4 AMM doesn't use concentrated liquidity
            sqrt_price: None, // V4 AMM uses reserves, not sqrt price
            current_tick: None, // V4 AMM doesn't use ticks
            fee_rate: None, // Would need to parse from pool data
            status,
            metadata,
        })
    }

    fn calculate_price(&self, pool_info: &PoolInfo) -> Result<f64> {
        // Use constant product formula for Raydium V4 AMM
        let price = price_math::reserves_to_price(
            pool_info.reserve_0,
            pool_info.reserve_1,
            pool_info.decimals_0,
            pool_info.decimals_1
        );

        if price.is_finite() && price > 0.0 {
            Ok(price)
        } else {
            Err(
                anyhow::anyhow!(DecoderError::InvalidPrice {
                    reason: format!("Invalid price calculated: {}", price),
                })
            )
        }
    }

    fn program_id(&self) -> Pubkey {
        program_ids::raydium_v4()
    }

    fn name(&self) -> &'static str {
        "Raydium V4 AMM"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_raydium_v4_decoder_creation() {
        let decoder = RaydiumV4Decoder::new();
        assert_eq!(decoder.name(), "Raydium V4 AMM");
        assert_eq!(decoder.program_id(), program_ids::raydium_v4());
    }

    #[test]
    fn test_raydium_v4_price_calculation() {
        let pool_info = PoolInfo {
            pool_address: Pubkey::default(),
            program_id: program_ids::raydium_v4(),
            pool_type: PoolType::RaydiumV4,
            token_mint_0: Pubkey::default(),
            token_mint_1: Pubkey::default(),
            token_vault_0: Pubkey::default(),
            token_vault_1: Pubkey::default(),
            reserve_0: 1_000_000_000, // 1 SOL (9 decimals)
            reserve_1: 150_000_000, // 150 USDC (6 decimals)
            decimals_0: 9,
            decimals_1: 6,
            liquidity: None,
            sqrt_price: None,
            current_tick: None,
            fee_rate: None,
            status: PoolStatus::Active,
            metadata: PoolMetadata::new(),
        };

        let decoder = RaydiumV4Decoder::new();
        let price = decoder.calculate_price(&pool_info).unwrap();

        // Should be approximately 150 USDC per SOL
        assert!((price - 150.0).abs() < 0.1);
    }
}
