use crate::pool::decoders::{ PoolDecoder, utils };
use crate::pool::types::*;
use anyhow::{ Context, Result };
use async_trait::async_trait;
use chrono::Utc;

/// Pump.fun bonding curve decoder
pub struct PumpfunDecoder;

impl PumpfunDecoder {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl PoolDecoder for PumpfunDecoder {
    fn pool_type(&self) -> PoolType {
        PoolType::PumpFun
    }

    fn can_decode(&self, account_data: &[u8]) -> bool {
        // Pump.fun bonding curve discriminator
        account_data.len() >= 300 && // Pump.fun bonding curve account size
            utils::check_discriminator(
                account_data,
                &[0x17, 0x97, 0x18, 0x9e, 0x65, 0x6e, 0x95, 0x6c]
            )
    }

    async fn decode_pool_info(&self, pool_address: &str, account_data: &[u8]) -> Result<PoolInfo> {
        if account_data.len() < 300 {
            return Err(anyhow::anyhow!("Invalid Pump.fun pool account data length"));
        }

        // Pump.fun bonding curve structure
        let token_mint = utils
            ::bytes_to_pubkey(&account_data[8..40])
            .context("Failed to decode token mint")?;

        // SOL is always the quote token in Pump.fun
        let sol_mint = "So11111111111111111111111111111111111111112";

        // Get bonding curve parameters
        let virtual_token_reserves = utils::bytes_to_u64(&account_data[40..48]);
        let virtual_sol_reserves = utils::bytes_to_u64(&account_data[48..56]);
        let real_token_reserves = utils::bytes_to_u64(&account_data[56..64]);
        let real_sol_reserves = utils::bytes_to_u64(&account_data[64..72]);

        // Calculate liquidity in USD (assuming SOL price)
        let sol_price_usd = 100.0; // Placeholder - would get from oracle
        let liquidity_usd = ((real_sol_reserves as f64) / 1e9) * sol_price_usd;

        // Pump.fun has no trading fees
        let fee_rate = 0.0;

        Ok(PoolInfo {
            pool_address: pool_address.to_string(),
            pool_type: PoolType::PumpFun,
            base_token_mint: token_mint.to_string(),
            quote_token_mint: sol_mint.to_string(),
            base_token_decimals: 6, // Pump.fun tokens typically use 6 decimals
            quote_token_decimals: 9, // SOL uses 9 decimals
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
        if account_data.len() < 300 {
            return Err(anyhow::anyhow!("Invalid Pump.fun pool account data length"));
        }

        // Get actual reserves from bonding curve
        let real_token_reserves = utils::bytes_to_u64(&account_data[56..64]);
        let real_sol_reserves = utils::bytes_to_u64(&account_data[64..72]);

        Ok(PoolReserve {
            pool_address: pool_address.to_string(),
            base_token_amount: real_token_reserves,
            quote_token_amount: real_sol_reserves,
            timestamp: Utc::now(),
            slot,
        })
    }

    fn program_id(&self) -> &str {
        // Pump.fun program ID
        "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P"
    }
}

impl Default for PumpfunDecoder {
    fn default() -> Self {
        Self::new()
    }
}
