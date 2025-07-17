use crate::pool::decoders::{ PoolDecoder, utils };
use crate::pool::types::*;
use anyhow::{ Context, Result };
use async_trait::async_trait;
use chrono::Utc;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

/// Raydium AMM pool decoder
pub struct RaydiumDecoder;

impl RaydiumDecoder {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl PoolDecoder for RaydiumDecoder {
    fn pool_type(&self) -> PoolType {
        PoolType::Raydium
    }

    fn can_decode(&self, account_data: &[u8]) -> bool {
        // Raydium AMM pools have a specific discriminator
        // This is a simplified check - in reality, you'd check the program ID and account structure
        account_data.len() >= 752 && // Raydium AMM account size
            utils::check_discriminator(
                account_data,
                &[0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]
            )
    }

    async fn decode_pool_info(&self, pool_address: &str, account_data: &[u8]) -> Result<PoolInfo> {
        if account_data.len() < 752 {
            return Err(anyhow::anyhow!("Invalid Raydium pool account data length"));
        }

        // Raydium AMM pool structure offsets (simplified)
        let base_mint = utils
            ::bytes_to_pubkey(&account_data[8..40])
            .context("Failed to decode base mint")?;
        let quote_mint = utils
            ::bytes_to_pubkey(&account_data[40..72])
            .context("Failed to decode quote mint")?;

        // Get token decimals (you'd typically fetch this from token mint accounts)
        let base_decimals = 9; // Most Solana tokens use 9 decimals
        let quote_decimals = 9;

        // Get liquidity info from reserves
        let base_reserve = utils::bytes_to_u64(&account_data[72..80]);
        let quote_reserve = utils::bytes_to_u64(&account_data[80..88]);

        // Calculate approximate USD liquidity (simplified)
        let liquidity_usd =
            (base_reserve as f64) * (10.0_f64).powi(-(base_decimals as i32)) * 100.0; // Assuming $100 SOL

        // Raydium fee is typically 0.25%
        let fee_rate = 0.0025;

        Ok(PoolInfo {
            pool_address: pool_address.to_string(),
            pool_type: PoolType::Raydium,
            base_token_mint: base_mint.to_string(),
            quote_token_mint: quote_mint.to_string(),
            base_token_decimals: base_decimals,
            quote_token_decimals: quote_decimals,
            liquidity_usd,
            fee_rate,
            created_at: Utc::now(), // You'd get this from transaction history
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
        if account_data.len() < 752 {
            return Err(anyhow::anyhow!("Invalid Raydium pool account data length"));
        }

        // Extract reserve amounts from pool account
        let base_reserve = utils::bytes_to_u64(&account_data[72..80]);
        let quote_reserve = utils::bytes_to_u64(&account_data[80..88]);

        Ok(PoolReserve {
            pool_address: pool_address.to_string(),
            base_token_amount: base_reserve,
            quote_token_amount: quote_reserve,
            timestamp: Utc::now(),
            slot,
        })
    }

    fn program_id(&self) -> &str {
        // Raydium AMM program ID
        "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8"
    }
}

impl Default for RaydiumDecoder {
    fn default() -> Self {
        Self::new()
    }
}
