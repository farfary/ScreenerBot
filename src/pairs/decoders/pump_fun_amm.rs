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

/// Pump.fun AMM decoder
pub struct PumpFunAmmDecoder;

impl PumpFunAmmDecoder {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl PoolDecoder for PumpFunAmmDecoder {
    fn decode(&self, data: &[u8]) -> Result<PoolInfo> {
        // Pump.fun AMM pool structure parsing
        // Based on debug analysis: 8-byte discriminator + 235 bytes of data = 243 bytes minimum
        if data.len() < 243 {
            return Err(
                (DecoderError::InvalidDataLength {
                    expected: 243,
                    actual: data.len(),
                }).into()
            );
        }

        // Skip the 8-byte discriminator at the beginning
        let mut offset = 8;

        // Parse pool_bump (1 byte)
        let _pool_bump = data[offset];
        offset += 1;

        // Parse index (2 bytes)
        let index_bytes: [u8; 2] = data[offset..offset + 2].try_into()?;
        let _index = u16::from_le_bytes(index_bytes);
        offset += 2;

        // Parse creator (32 bytes)
        let creator = Pubkey::try_from(&data[offset..offset + 32])?;
        offset += 32;

        // Parse base_mint (32 bytes)
        let base_mint = Pubkey::try_from(&data[offset..offset + 32])?;
        offset += 32;

        // Parse quote_mint (32 bytes)
        let quote_mint = Pubkey::try_from(&data[offset..offset + 32])?;
        offset += 32;

        // Parse lp_mint (32 bytes)
        let lp_mint = Pubkey::try_from(&data[offset..offset + 32])?;
        offset += 32;

        // Parse pool_base_token_account (32 bytes)
        let pool_base_token_account = Pubkey::try_from(&data[offset..offset + 32])?;
        offset += 32;

        // Parse pool_quote_token_account (32 bytes)
        let pool_quote_token_account = Pubkey::try_from(&data[offset..offset + 32])?;
        offset += 32;

        // Parse lp_supply (8 bytes)
        let lp_supply_bytes: [u8; 8] = data[offset..offset + 8].try_into()?;
        let lp_supply = u64::from_le_bytes(lp_supply_bytes);
        offset += 8;

        // Parse coin_creator (32 bytes)
        let coin_creator = Pubkey::try_from(&data[offset..offset + 32])?;

        // Pump.fun AMM pools are typically active if they have LP supply
        let status = if lp_supply > 0 { PoolStatus::Active } else { PoolStatus::Inactive };

        let metadata = PoolMetadata {
            lp_mint: Some(lp_mint),
            lp_supply: Some(lp_supply),
            creator: Some(creator),
            coin_creator: Some(coin_creator),
            ..Default::default()
        };

        let pool_info = PoolInfo {
            pool_address: Pubkey::default(), // This should be set by the caller
            program_id: self.program_id(),
            pool_type: PoolType::PumpFunAmm,
            token_mint_0: base_mint,
            token_mint_1: quote_mint,
            token_vault_0: pool_base_token_account,
            token_vault_1: pool_quote_token_account,
            reserve_0: 0, // To be fetched from vault
            reserve_1: 0, // To be fetched from vault
            decimals_0: 6, // Typical for pump.fun tokens, should be fetched from token mint
            decimals_1: 9, // SOL has 9 decimals
            liquidity: None, // Could derive from LP supply if needed
            sqrt_price: None,
            current_tick: None,
            fee_rate: None, // Pump.fun typically has fixed fees
            status,
            metadata,
        };

        Ok(pool_info)
    }

    fn calculate_price(&self, pool_info: &PoolInfo) -> Result<f64> {
        if pool_info.reserve_0 > 0 && pool_info.reserve_1 > 0 {
            // Pump.fun AMM uses constant product formula (x * y = k)
            // Price = reserve_quote / reserve_base
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
                    reason: "No reserves available for price calculation".to_string(),
                }).into()
            )
        }
    }

    fn program_id(&self) -> Pubkey {
        program_ids::pump_fun_amm()
    }

    fn name(&self) -> &'static str {
        "Pump.fun AMM"
    }
}

impl Default for PumpFunAmmDecoder {
    fn default() -> Self {
        Self::new()
    }
}
