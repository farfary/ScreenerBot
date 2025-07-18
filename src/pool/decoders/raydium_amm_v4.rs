use anyhow::Result;
use async_trait::async_trait;
use chrono::Utc;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use std::sync::Arc;
use crate::rpc::RpcManager;
use crate::pool::decoders::utils;
use crate::pool::types::{ PoolType, PoolInfo, PoolReserve };
use crate::pool::decoders::PoolDecoder;

/// Raydium AMM V4 Pool decoder
/// Program ID: 675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8
pub struct RaydiumAmmV4Decoder {
    rpc_manager: Arc<RpcManager>,
}

impl RaydiumAmmV4Decoder {
    pub fn new(rpc_manager: Arc<RpcManager>) -> Self {
        Self { rpc_manager }
    }

    /// Calculate current price from pool reserves
    async fn calculate_price(&self, pool_address: &str, account_data: &[u8]) -> Result<f64> {
        println!("🔍 DEBUG: Calculating price for Raydium AMM V4 pool {}", pool_address);

        // Raydium AMM V4 pool structure (simplified)
        // The actual reserves are stored in separate token accounts
        let pool_coin_token_account = utils::bytes_to_pubkey(&account_data[64..96]);
        let pool_pc_token_account = utils::bytes_to_pubkey(&account_data[96..128]);

        println!("🔍 DEBUG: pool_coin_token_account: {}", pool_coin_token_account);
        println!("🔍 DEBUG: pool_pc_token_account: {}", pool_pc_token_account);

        // Get token account balances
        let coin_balance = match
            self.rpc_manager.get_token_account_balance(&pool_coin_token_account).await
        {
            Ok(balance) => balance,
            Err(e) => {
                println!("⚠️  Failed to get coin token account balance: {}", e);
                return Ok(0.0);
            }
        };

        let pc_balance = match
            self.rpc_manager.get_token_account_balance(&pool_pc_token_account).await
        {
            Ok(balance) => balance,
            Err(e) => {
                println!("⚠️  Failed to get PC token account balance: {}", e);
                return Ok(0.0);
            }
        };

        println!("🔍 DEBUG: coin_balance: {}, pc_balance: {}", coin_balance, pc_balance);

        // Calculate price: PC tokens / Coin tokens
        if coin_balance > 0 {
            let price = (pc_balance as f64) / (coin_balance as f64);
            println!("🔍 DEBUG: Calculated price: {}", price);
            Ok(price)
        } else {
            println!("⚠️  Cannot calculate price - no coin balance");
            Ok(0.0)
        }
    }
}

#[async_trait]
impl PoolDecoder for RaydiumAmmV4Decoder {
    fn program_id(&self) -> Pubkey {
        Pubkey::from_str("675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8").unwrap()
    }

    fn can_decode(&self, account_data: &[u8]) -> bool {
        // Raydium AMM V4 pools have specific discriminator and structure
        // The pool account is typically 752 bytes
        if account_data.len() < 752 {
            return false;
        }

        // Check for Raydium AMM V4 discriminator at the beginning
        let discriminator = &account_data[0..8];
        // Raydium AMM V4 uses a specific discriminator
        let expected_discriminator = [0x98, 0x86, 0x6d, 0x6f, 0xba, 0x5d, 0x1e, 0x5c];

        discriminator == expected_discriminator
    }

    async fn decode_pool_info(&self, pool_address: &str, account_data: &[u8]) -> Result<PoolInfo> {
        println!("🔍 DEBUG: Decoding Raydium AMM V4 pool {}", pool_address);
        println!("🔍 DEBUG: Account data length: {} bytes", account_data.len());

        if account_data.len() < 752 {
            return Err(anyhow::anyhow!("Invalid Raydium AMM V4 pool data length"));
        }

        // Parse Raydium AMM V4 pool structure
        let status = utils::bytes_to_u64(&account_data[8..16]);
        let _nonce = account_data[16];
        let _order_num = utils::bytes_to_u64(&account_data[17..25]);
        let _depth = utils::bytes_to_u64(&account_data[25..33]);
        let coin_decimals = utils::bytes_to_u64(&account_data[33..41]);
        let pc_decimals = utils::bytes_to_u64(&account_data[41..49]);
        let state = utils::bytes_to_u64(&account_data[49..57]);
        let _reset_flag = utils::bytes_to_u64(&account_data[57..65]);

        let _pool_coin_token_account = utils::bytes_to_pubkey(&account_data[64..96]);
        let _pool_pc_token_account = utils::bytes_to_pubkey(&account_data[96..128]);
        let coin_mint = utils::bytes_to_pubkey(&account_data[128..160]);
        let pc_mint = utils::bytes_to_pubkey(&account_data[160..192]);

        println!("🔍 DEBUG: coin_mint: {}", coin_mint);
        println!("🔍 DEBUG: pc_mint: {}", pc_mint);
        println!("🔍 DEBUG: status: {}, state: {}", status, state);

        let _price = self.calculate_price(pool_address, account_data).await.unwrap_or(0.0);

        Ok(PoolInfo {
            pool_address: pool_address.to_string(),
            pool_type: PoolType::RaydiumAmmV4,
            base_token_mint: coin_mint.to_string(),
            quote_token_mint: pc_mint.to_string(),
            base_token_decimals: coin_decimals as u8,
            quote_token_decimals: pc_decimals as u8,
            liquidity_usd: 0.0,
            fee_rate: 0.0025, // Raydium typically uses 0.25% fee
            created_at: Utc::now(),
            last_updated: Utc::now(),
            is_active: status == 1 && state == 1,
        })
    }

    async fn decode_pool_reserves(
        &self,
        pool_address: &str,
        account_data: &[u8],
        slot: u64
    ) -> Result<PoolReserve> {
        println!("🔍 DEBUG: Decoding Raydium AMM V4 pool reserves for {}", pool_address);

        if account_data.len() < 752 {
            return Err(anyhow::anyhow!("Invalid Raydium AMM V4 pool data length"));
        }

        // Get token account addresses
        let pool_coin_token_account = utils::bytes_to_pubkey(&account_data[64..96]);
        let pool_pc_token_account = utils::bytes_to_pubkey(&account_data[96..128]);

        // Get actual token balances
        let coin_balance = self.rpc_manager
            .get_token_account_balance(&pool_coin_token_account).await
            .unwrap_or(0);
        let pc_balance = self.rpc_manager
            .get_token_account_balance(&pool_pc_token_account).await
            .unwrap_or(0);

        Ok(PoolReserve {
            pool_address: pool_address.to_string(),
            base_token_amount: coin_balance,
            quote_token_amount: pc_balance,
            timestamp: Utc::now(),
            slot,
        })
    }
}
