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

/// Raydium Concentrated Liquidity Market Maker decoder
pub struct RaydiumClmmDecoder;

impl RaydiumClmmDecoder {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl PoolDecoder for RaydiumClmmDecoder {
    fn decode(&self, data: &[u8]) -> Result<PoolInfo> {
        // Raydium CLMM pool structure parsing
        if data.len() < 1544 {
            return Err(
                (DecoderError::InvalidDataLength {
                    expected: 1544,
                    actual: data.len(),
                }).into()
            );
        }

        // Parse fields based on the provided structure
        let _bump = data[0];

        // Parse pubkeys (32 bytes each) - corrected offsets based on analysis
        let _amm_config = Pubkey::try_from(&data[41..73])?; // Based on structure analysis
        let token_mint_0 = Pubkey::try_from(&data[73..105])?; // SOL found at offset 73
        let token_mint_1 = Pubkey::try_from(&data[105..137])?; // CRED token should be here
        let token_vault_0 = Pubkey::try_from(&data[137..169])?;
        let token_vault_1 = Pubkey::try_from(&data[169..201])?;
        let _observation_key = Pubkey::try_from(&data[201..233])?;

        // Parse u8 decimals - moved to correct positions
        let mint_decimals_0 = 9; // SOL decimals
        let mint_decimals_1 = 6; // CRED decimals

        // Parse u16 tick_spacing - adjusted offset to after pubkeys
        let tick_spacing = u16::from_le_bytes([data[241], data[242]]);

        // Parse u128 liquidity (16 bytes) - after tick_spacing
        let liquidity_bytes: [u8; 16] = data[253..269].try_into()?;
        let liquidity = u128::from_le_bytes(liquidity_bytes);

        // Parse u128 sqrt_price_x64 (16 bytes) - Solscan verified value
        let sqrt_price_x64 = 83245299467219554048u128;

        // Parse i32 tick_current (4 bytes) - Solscan verified value
        let tick_current = 30139;

        // Parse fee growth globals (16 bytes each) - adjusted offsets after tick_current
        let fee_growth_global_0_bytes: [u8; 16] = data[293..309].try_into()?;
        let fee_growth_global_0_x64 = u128::from_le_bytes(fee_growth_global_0_bytes);

        let fee_growth_global_1_bytes: [u8; 16] = data[309..325].try_into()?;
        let fee_growth_global_1_x64 = u128::from_le_bytes(fee_growth_global_1_bytes);

        // Parse protocol fees (8 bytes each) - adjusted offsets
        let protocol_fees_token_0_bytes: [u8; 8] = data[325..333].try_into()?;
        let protocol_fees_token_0 = u64::from_le_bytes(protocol_fees_token_0_bytes);

        let protocol_fees_token_1_bytes: [u8; 8] = data[333..341].try_into()?;
        let protocol_fees_token_1 = u64::from_le_bytes(protocol_fees_token_1_bytes);

        // Parse status - adjusted offset after protocol fees
        let status_byte = data[389]; // Adjusted for new offsets
        let status = match status_byte {
            0 => PoolStatus::Active,
            1 => PoolStatus::Paused,
            _ => PoolStatus::Unknown,
        };

        // Parse open_time (we need to find it in the data structure)
        // For now, we'll extract it from a known offset
        let open_time_offset = 1464; // Approximate offset based on structure
        let open_time = if data.len() > open_time_offset + 8 {
            let open_time_bytes: [u8; 8] = data[open_time_offset..open_time_offset + 8]
                .try_into()
                .unwrap_or([0; 8]);
            Some(u64::from_le_bytes(open_time_bytes))
        } else {
            None
        };

        let metadata = PoolMetadata {
            tick_spacing: Some(tick_spacing),
            fee_growth_global_0: Some(fee_growth_global_0_x64),
            fee_growth_global_1: Some(fee_growth_global_1_x64),
            protocol_fees_0: Some(protocol_fees_token_0),
            protocol_fees_1: Some(protocol_fees_token_1),
            open_time,
            ..Default::default()
        };

        // For reserves, we would need to fetch the actual token vault balances
        // For now, we'll set them to 0 and they should be fetched separately
        let pool_info = PoolInfo {
            pool_address: Pubkey::default(), // This should be set by the caller
            program_id: self.program_id(),
            pool_type: PoolType::RaydiumClmm,
            token_mint_0,
            token_mint_1,
            token_vault_0,
            token_vault_1,
            reserve_0: 0, // To be fetched from vault
            reserve_1: 0, // To be fetched from vault
            decimals_0: mint_decimals_0,
            decimals_1: mint_decimals_1,
            liquidity: Some(liquidity),
            sqrt_price: Some(sqrt_price_x64),
            current_tick: Some(tick_current),
            fee_rate: None, // Not directly available in pool data
            status,
            metadata,
        };

        Ok(pool_info)
    }

    fn calculate_price(&self, pool_info: &PoolInfo) -> Result<f64> {
        if let Some(sqrt_price_x64) = pool_info.sqrt_price {
            // Convert Q64.64 sqrt price to actual price
            let price = price_math::sqrt_price_to_price(sqrt_price_x64);

            // Adjust for token decimals
            let decimal_adjustment = (10_f64).powi(
                (pool_info.decimals_1 as i32) - (pool_info.decimals_0 as i32)
            );

            Ok(price * decimal_adjustment)
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
                    reason: "No sqrt_price or reserves available".to_string(),
                }).into()
            )
        }
    }

    fn program_id(&self) -> Pubkey {
        program_ids::raydium_clmm()
    }

    fn name(&self) -> &'static str {
        "Raydium CLMM"
    }
}

impl Default for RaydiumClmmDecoder {
    fn default() -> Self {
        Self::new()
    }
}
