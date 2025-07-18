use crate::pool::decoders::{ PoolDecoder, utils };
use crate::pool::types::*;
use anyhow::{ Context, Result };
use async_trait::async_trait;
use chrono::Utc;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

pub struct MeteoraDynamicDecoder;

impl MeteoraDynamicDecoder {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl PoolDecoder for MeteoraDynamicDecoder {
    fn program_id(&self) -> Pubkey {
        Pubkey::from_str("dbcij3LWUppWqq96dh6gJWwBifmcGfLSB5D4DuSMaqN").unwrap()
    }

    fn can_decode(&self, account_data: &[u8]) -> bool {
        // Check if the account has enough data and matches expected structure
        account_data.len() >= 200 // Minimum size for meteora dynamic pool
    }

    async fn decode_pool_info(&self, pool_address: &str, account_data: &[u8]) -> Result<PoolInfo> {
        if account_data.len() < 200 {
            return Err(anyhow::anyhow!("Account data too small for Meteora Dynamic pool"));
        }

        // Parse the account data based on the provided structure
        // Skip discriminator (8 bytes)
        let data = &account_data[8..];

        // Skip volatility_tracker (48 bytes)
        let data = &data[48..];

        // Config (32 bytes)
        let config = utils::bytes_to_pubkey(&data[0..32]);

        // Creator (32 bytes)
        let creator = utils::bytes_to_pubkey(&data[32..64]);

        // Base mint (32 bytes)
        let base_mint = utils::bytes_to_pubkey(&data[64..96]);

        // Base vault (32 bytes)
        let base_vault = utils::bytes_to_pubkey(&data[96..128]);

        // Quote vault (32 bytes)
        let quote_vault = utils::bytes_to_pubkey(&data[128..160]);

        // Base reserve (8 bytes)
        let base_reserve = utils::bytes_to_u64(&data[160..168]);

        // Quote reserve (8 bytes)
        let quote_reserve = utils::bytes_to_u64(&data[168..176]);

        // Protocol base fee (8 bytes)
        let _protocol_base_fee = utils::bytes_to_u64(&data[176..184]);

        // Protocol quote fee (8 bytes)
        let _protocol_quote_fee = utils::bytes_to_u64(&data[184..192]);

        // Partner base fee (8 bytes)
        let _partner_base_fee = utils::bytes_to_u64(&data[192..200]);

        // Partner quote fee (8 bytes)
        let _partner_quote_fee = utils::bytes_to_u64(&data[200..208]);

        // sqrt_price (16 bytes)
        let sqrt_price = utils::bytes_to_u128(&data[208..224]);

        // activation_point (8 bytes)
        let _activation_point = utils::bytes_to_u64(&data[224..232]);

        Ok(PoolInfo {
            pool_address: pool_address.to_string(),
            pool_type: PoolType::MeteoraDynamic,
            base_token_mint: base_mint.to_string(),
            quote_token_mint: "So1111111111111111111111111111111111111111112".to_string(), // SOL
            base_token_decimals: 9,
            quote_token_decimals: 9,
            liquidity_usd: 0.0,
            fee_rate: 0.0, // Will be calculated from fees if needed
            created_at: Utc::now(),
            last_updated: Utc::now(),
            is_active: true,
        })
    }

    async fn decode_pool_reserves(
        &self,
        pool_address: &str,
        account_data: &[u8],
        slot: u64
    ) -> Result<PoolReserve> {
        if account_data.len() < 200 {
            return Err(anyhow::anyhow!("Account data too small for Meteora Dynamic pool"));
        }

        // Parse the account data to get reserves
        let data = &account_data[8..]; // Skip discriminator

        // Skip volatility_tracker (48 bytes) + config (32) + creator (32) + base_mint (32) + base_vault (32) + quote_vault (32)
        let data = &data[176..];

        // Base reserve (8 bytes)
        let token_a_reserve = utils::bytes_to_u64(&data[0..8]) as f64;

        // Quote reserve (8 bytes)
        let token_b_reserve = utils::bytes_to_u64(&data[8..16]) as f64;

        Ok(PoolReserve {
            pool_address: pool_address.to_string(),
            base_token_amount: token_a_reserve as u64,
            quote_token_amount: token_b_reserve as u64,
            slot,
            timestamp: Utc::now(),
        })
    }
}
