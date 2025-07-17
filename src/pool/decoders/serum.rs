use crate::pool::decoders::{ PoolDecoder, utils };
use crate::pool::types::*;
use anyhow::{ Context, Result };
use async_trait::async_trait;
use chrono::Utc;

/// Serum DEX decoder
pub struct SerumDecoder;

impl SerumDecoder {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl PoolDecoder for SerumDecoder {
    fn pool_type(&self) -> PoolType {
        PoolType::Serum
    }

    fn can_decode(&self, account_data: &[u8]) -> bool {
        // Serum market discriminator
        account_data.len() >= 388 && // Serum market account size
            utils::check_discriminator(
                account_data,
                &[0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]
            )
    }

    async fn decode_pool_info(&self, pool_address: &str, account_data: &[u8]) -> Result<PoolInfo> {
        if account_data.len() < 388 {
            return Err(anyhow::anyhow!("Invalid Serum market account data length"));
        }

        // Serum market structure
        let base_mint = utils
            ::bytes_to_pubkey(&account_data[53..85])
            .context("Failed to decode base mint")?;
        let quote_mint = utils
            ::bytes_to_pubkey(&account_data[85..117])
            .context("Failed to decode quote mint")?;

        // Get vault addresses
        let base_vault = utils
            ::bytes_to_pubkey(&account_data[117..149])
            .context("Failed to decode base vault")?;
        let quote_vault = utils
            ::bytes_to_pubkey(&account_data[149..181])
            .context("Failed to decode quote vault")?;

        // Get lot sizes for price calculation
        let base_lot_size = utils::bytes_to_u64(&account_data[181..189]);
        let quote_lot_size = utils::bytes_to_u64(&account_data[189..197]);

        // Simplified liquidity calculation
        let liquidity_usd = 10000.0; // Placeholder

        // Serum fee is typically 0.22% for taker
        let fee_rate = 0.0022;

        Ok(PoolInfo {
            pool_address: pool_address.to_string(),
            pool_type: PoolType::Serum,
            base_token_mint: base_mint.to_string(),
            quote_token_mint: quote_mint.to_string(),
            base_token_decimals: 9,
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
        if account_data.len() < 388 {
            return Err(anyhow::anyhow!("Invalid Serum market account data length"));
        }

        // For Serum, you'd need to read the vault accounts to get reserves
        // This is a simplified implementation

        // Placeholder reserves - would need to fetch from vaults
        let base_reserve = 1000000000; // 1 token with 9 decimals
        let quote_reserve = 1000000000; // 1 token with 9 decimals

        Ok(PoolReserve {
            pool_address: pool_address.to_string(),
            base_token_amount: base_reserve,
            quote_token_amount: quote_reserve,
            timestamp: Utc::now(),
            slot,
        })
    }

    fn program_id(&self) -> &str {
        // Serum DEX program ID
        "9xQeWvG816bUx9EPjHmaT23yvVM2ZWbrrpZb9PusVFin"
    }
}

impl Default for SerumDecoder {
    fn default() -> Self {
        Self::new()
    }
}
