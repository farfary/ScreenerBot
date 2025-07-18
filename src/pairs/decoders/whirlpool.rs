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

/// Orca Whirlpool decoder
pub struct WhirlpoolDecoder;

impl WhirlpoolDecoder {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl PoolDecoder for WhirlpoolDecoder {
    fn decode(&self, data: &[u8]) -> Result<PoolInfo> {
        // Whirlpool structure parsing
        if data.len() < 653 {
            return Err(
                (DecoderError::InvalidDataLength {
                    expected: 653,
                    actual: data.len(),
                }).into()
            );
        }

        let mut offset = 0;

        // Parse whirlpools_config (32 bytes)
        let _whirlpools_config = Pubkey::try_from(&data[offset..offset + 32])?;
        offset += 32;

        // Parse whirlpool_bump (1 byte)
        let _whirlpool_bump = data[offset];
        offset += 1;

        // Parse tick_spacing (2 bytes)
        let tick_spacing_bytes: [u8; 2] = data[offset..offset + 2].try_into()?;
        let tick_spacing = u16::from_le_bytes(tick_spacing_bytes);
        offset += 2;

        // Parse fee_tier_index_seed (2 bytes)
        offset += 2;

        // Parse fee_rate (2 bytes)
        let fee_rate_bytes: [u8; 2] = data[offset..offset + 2].try_into()?;
        let fee_rate = u16::from_le_bytes(fee_rate_bytes);
        offset += 2;

        // Parse protocol_fee_rate (2 bytes)
        let _protocol_fee_rate_bytes: [u8; 2] = data[offset..offset + 2].try_into()?;
        let _protocol_fee_rate = u16::from_le_bytes(_protocol_fee_rate_bytes);
        offset += 2;

        // Parse liquidity (16 bytes)
        let liquidity_bytes: [u8; 16] = data[offset..offset + 16].try_into()?;
        let liquidity = u128::from_le_bytes(liquidity_bytes);
        offset += 16;

        // Parse sqrt_price (16 bytes)
        let sqrt_price_bytes: [u8; 16] = data[offset..offset + 16].try_into()?;
        let sqrt_price = u128::from_le_bytes(sqrt_price_bytes);
        offset += 16;

        // Parse tick_current_index (4 bytes)
        let tick_current_bytes: [u8; 4] = data[offset..offset + 4].try_into()?;
        let tick_current_index = i32::from_le_bytes(tick_current_bytes);
        offset += 4;

        // Parse protocol_fee_owed_a (8 bytes)
        let protocol_fee_owed_a_bytes: [u8; 8] = data[offset..offset + 8].try_into()?;
        let protocol_fee_owed_a = u64::from_le_bytes(protocol_fee_owed_a_bytes);
        offset += 8;

        // Parse protocol_fee_owed_b (8 bytes)
        let protocol_fee_owed_b_bytes: [u8; 8] = data[offset..offset + 8].try_into()?;
        let protocol_fee_owed_b = u64::from_le_bytes(protocol_fee_owed_b_bytes);
        offset += 8;

        // Parse token_mint_a (32 bytes)
        let token_mint_a = Pubkey::try_from(&data[offset..offset + 32])?;
        offset += 32;

        // Parse token_vault_a (32 bytes)
        let token_vault_a = Pubkey::try_from(&data[offset..offset + 32])?;
        offset += 32;

        // Parse fee_growth_global_a (16 bytes)
        let fee_growth_global_a_bytes: [u8; 16] = data[offset..offset + 16].try_into()?;
        let fee_growth_global_a = u128::from_le_bytes(fee_growth_global_a_bytes);
        offset += 16;

        // Parse token_mint_b (32 bytes)
        let token_mint_b = Pubkey::try_from(&data[offset..offset + 32])?;
        offset += 32;

        // Parse token_vault_b (32 bytes)
        let token_vault_b = Pubkey::try_from(&data[offset..offset + 32])?;
        offset += 32;

        // Parse fee_growth_global_b (16 bytes)
        let fee_growth_global_b_bytes: [u8; 16] = data[offset..offset + 16].try_into()?;
        let fee_growth_global_b = u128::from_le_bytes(fee_growth_global_b_bytes);
        offset += 16;

        // Parse reward_last_updated_timestamp (8 bytes)
        let reward_last_updated_timestamp_bytes: [u8; 8] = data[offset..offset + 8].try_into()?;
        let reward_last_updated_timestamp = u64::from_le_bytes(reward_last_updated_timestamp_bytes);

        // Pool is active if liquidity > 0
        let status = if liquidity > 0 { PoolStatus::Active } else { PoolStatus::Inactive };

        let metadata = PoolMetadata {
            tick_spacing: Some(tick_spacing),
            fee_growth_global_0: Some(fee_growth_global_a),
            fee_growth_global_1: Some(fee_growth_global_b),
            protocol_fees_0: Some(protocol_fee_owed_a),
            protocol_fees_1: Some(protocol_fee_owed_b),
            last_update_time: Some(reward_last_updated_timestamp),
            ..Default::default()
        };

        let pool_info = PoolInfo {
            pool_address: Pubkey::default(), // This should be set by the caller
            program_id: self.program_id(),
            pool_type: PoolType::Whirlpool,
            token_mint_0: token_mint_a,
            token_mint_1: token_mint_b,
            token_vault_0: token_vault_a,
            token_vault_1: token_vault_b,
            reserve_0: 0, // To be fetched from vault
            reserve_1: 0, // To be fetched from vault
            decimals_0: 9, // Default, should be fetched from token mint
            decimals_1: 6, // Default, should be fetched from token mint
            liquidity: Some(liquidity),
            sqrt_price: Some(sqrt_price),
            current_tick: Some(tick_current_index),
            fee_rate: Some(fee_rate.into()),
            status,
            metadata,
        };

        Ok(pool_info)
    }

    fn calculate_price(&self, pool_info: &PoolInfo) -> Result<f64> {
        if let Some(sqrt_price) = pool_info.sqrt_price {
            // Whirlpool uses Q64.64 sqrt price format
            let price = price_math::sqrt_price_to_price(sqrt_price);

            // Adjust for token decimals
            let decimal_adjustment = (10_f64).powi(
                (pool_info.decimals_1 as i32) - (pool_info.decimals_0 as i32)
            );

            Ok(price * decimal_adjustment)
        } else if let Some(tick) = pool_info.current_tick {
            // Alternative calculation using tick
            let price = price_math::tick_to_price(tick);

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
                    reason: "No sqrt_price, tick, or reserves available".to_string(),
                }).into()
            )
        }
    }

    fn program_id(&self) -> Pubkey {
        program_ids::whirlpool()
    }

    fn name(&self) -> &'static str {
        "Whirlpool"
    }
}

impl Default for WhirlpoolDecoder {
    fn default() -> Self {
        Self::new()
    }
}
