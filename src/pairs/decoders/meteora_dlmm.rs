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

/// Meteora Dynamic Liquidity Market Maker decoder
pub struct MeteoraDlmmDecoder;

impl MeteoraDlmmDecoder {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl PoolDecoder for MeteoraDlmmDecoder {
    fn decode(&self, data: &[u8]) -> Result<PoolInfo> {
        // Meteora DLMM pool structure parsing - based on debug analysis
        if data.len() < 904 {
            return Err(
                (DecoderError::InvalidDataLength {
                    expected: 904,
                    actual: data.len(),
                }).into()
            );
        }

        // Based on debug analysis, we know the exact offsets:
        // activeId (-378) at offset 48
        // binStep (80) at offset 73
        // tokenXMint at offset 88
        // tokenYMint at offset 120
        // reserveX at offset 152
        // reserveY at offset 184

        // Parse activeId (i32) at offset 48
        let active_id_bytes: [u8; 4] = data[48..52].try_into()?;
        let active_id = i32::from_le_bytes(active_id_bytes);

        // Parse binStep (u16) at offset 73
        let bin_step_bytes: [u8; 2] = data[73..75].try_into()?;
        let bin_step = u16::from_le_bytes(bin_step_bytes);

        // Parse status byte (around activeId area, let's use offset 75)
        let status_byte = data[75];
        let status = match status_byte {
            0 => PoolStatus::Active,
            1 => PoolStatus::Paused,
            _ => PoolStatus::Active, // Default to Active for unknown status
        };

        // Parse token mints at known offsets
        let token_mint_x = Pubkey::try_from(&data[88..120])?;
        let token_mint_y = Pubkey::try_from(&data[120..152])?;

        // Parse reserves at known offsets
        let reserve_x = Pubkey::try_from(&data[152..184])?;
        let reserve_y = Pubkey::try_from(&data[184..216])?;

        // Try to parse volatility accumulator from earlier in the structure
        // Based on JSON, this should be around offset 24-28
        let volatility_accumulator = if data.len() > 28 {
            let bytes: [u8; 4] = data[24..28].try_into().unwrap_or([0, 0, 0, 0]);
            u32::from_le_bytes(bytes)
        } else {
            0
        };

        let metadata = PoolMetadata {
            bin_step: Some(bin_step),
            volatility_accumulator: Some(volatility_accumulator),
            active_bin_id: Some(active_id),
            ..Default::default()
        };

        let pool_info = PoolInfo {
            pool_address: Pubkey::default(), // This should be set by the caller
            program_id: self.program_id(),
            pool_type: PoolType::MeteoraDlmm,
            token_mint_0: token_mint_x,
            token_mint_1: token_mint_y,
            token_vault_0: reserve_x,
            token_vault_1: reserve_y,
            reserve_0: 0, // To be fetched from vault
            reserve_1: 0, // To be fetched from vault
            decimals_0: 9, // Default, should be fetched from token mint
            decimals_1: 6, // Default, should be fetched from token mint
            liquidity: None,
            sqrt_price: None,
            current_tick: Some(active_id),
            fee_rate: None,
            status,
            metadata,
        };

        Ok(pool_info)
    }

    fn calculate_price(&self, pool_info: &PoolInfo) -> Result<f64> {
        if let Some(active_bin_id) = pool_info.current_tick {
            if let Some(bin_step) = pool_info.metadata.bin_step {
                // Calculate price from bin ID and bin step
                // Price = (1 + bin_step / 10000) ^ active_bin_id
                let bin_step_decimal = (bin_step as f64) / 10000.0;
                let price = (1.0 + bin_step_decimal).powi(active_bin_id);

                // Adjust for token decimals
                let decimal_adjustment = (10_f64).powi(
                    (pool_info.decimals_1 as i32) - (pool_info.decimals_0 as i32)
                );

                Ok(price * decimal_adjustment)
            } else {
                Err(
                    (DecoderError::MissingField {
                        field: "bin_step".to_string(),
                    }).into()
                )
            }
        } else if pool_info.reserve_0 > 0 && pool_info.reserve_1 > 0 {
            // Fallback to reserve-based calculation
            Ok(
                price_math::reserves_to_price(
                    pool_info.reserve_0,
                    pool_info.reserve_1,
                    pool_info.decimals_0,
                    pool_info.decimals_1
                )
            )
        } else {
            Err(
                (DecoderError::InvalidPrice {
                    reason: "No active_bin_id or reserves available".to_string(),
                }).into()
            )
        }
    }

    fn program_id(&self) -> Pubkey {
        program_ids::meteora_dlmm()
    }

    fn name(&self) -> &'static str {
        "Meteora DLMM"
    }
}

impl Default for MeteoraDlmmDecoder {
    fn default() -> Self {
        Self::new()
    }
}
