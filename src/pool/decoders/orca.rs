use crate::pool::decoders::{ PoolDecoder, utils };
use crate::pool::types::*;
use anyhow::{ Context, Result };
use async_trait::async_trait;
use chrono::Utc;
use solana_sdk::pubkey::Pubkey;

/// Orca whirlpool decoder
pub struct OrcaDecoder;

impl OrcaDecoder {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl PoolDecoder for OrcaDecoder {
    fn pool_type(&self) -> PoolType {
        PoolType::Orca
    }

    fn can_decode(&self, account_data: &[u8]) -> bool {
        // Orca whirlpool discriminator check
        account_data.len() >= 653 && // Orca whirlpool account size
            utils::check_discriminator(
                account_data,
                &[0x63, 0x31, 0x1a, 0x2e, 0x2d, 0x1d, 0x7b, 0x29]
            )
    }

    async fn decode_pool_info(&self, pool_address: &str, account_data: &[u8]) -> Result<PoolInfo> {
        if account_data.len() < 653 {
            return Err(anyhow::anyhow!("Invalid Orca pool account data length"));
        }

        // Orca whirlpool structure offsets
        let token_mint_a = utils
            ::bytes_to_pubkey(&account_data[101..133])
            .context("Failed to decode token mint A")?;
        let token_mint_b = utils
            ::bytes_to_pubkey(&account_data[181..213])
            .context("Failed to decode token mint B")?;

        // Get token vaults for reserves
        let token_vault_a = utils
            ::bytes_to_pubkey(&account_data[133..165])
            .context("Failed to decode token vault A")?;
        let token_vault_b = utils
            ::bytes_to_pubkey(&account_data[213..245])
            .context("Failed to decode token vault B")?;

        // Get fee rate (in basis points)
        let fee_rate_bps = utils::bytes_to_u32(&account_data[73..77]);
        let fee_rate = (fee_rate_bps as f64) / 10000.0;

        // Get tick spacing and current tick for price calculation
        let tick_spacing = utils::bytes_to_u32(&account_data[77..81]);
        let current_tick = utils::bytes_to_u32(&account_data[245..249]) as i32;

        // Simplified liquidity calculation
        let liquidity_usd = 1000.0; // Placeholder - would calculate from tick and liquidity

        Ok(PoolInfo {
            pool_address: pool_address.to_string(),
            pool_type: PoolType::Orca,
            base_token_mint: token_mint_a.to_string(),
            quote_token_mint: token_mint_b.to_string(),
            base_token_decimals: 9, // Default - fetch from mint account
            quote_token_decimals: 9,
            liquidity_usd,
            fee_rate,
            created_at: Utc::now(),
            last_updated: Utc::now(),
            is_active: true,
        })
    }

    async fn decode_reserves(
        &self,
        pool_address: &str,
        account_data: &[u8],
        slot: u64
    ) -> Result<PoolReserve> {
        if account_data.len() < 653 {
            return Err(anyhow::anyhow!("Invalid Orca pool account data length"));
        }

        // For Orca, you'd need to read the token vault accounts to get actual reserves
        // This is a simplified implementation

        // Get liquidity from the pool
        let liquidity = utils::bytes_to_u64(&account_data[253..261]);

        // Calculate approximate reserves based on current tick and liquidity
        // This is a simplified calculation - actual implementation would be more complex
        let base_reserve = liquidity / 2;
        let quote_reserve = liquidity / 2;

        Ok(PoolReserve {
            pool_address: pool_address.to_string(),
            base_token_amount: base_reserve,
            quote_token_amount: quote_reserve,
            timestamp: Utc::now(),
            slot,
        })
    }

    fn program_id(&self) -> &str {
        // Orca whirlpool program ID
        "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc"
    }
}

impl Default for OrcaDecoder {
    fn default() -> Self {
        Self::new()
    }
}
