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

/// Raydium Stable Swap Pool decoder
/// Program ID: 5quBtoiQqxF9Jv6KYKctB59NT3gtJD2Y65kdnB1Uev3h
pub struct RaydiumStableSwapDecoder {
    rpc_manager: Arc<RpcManager>,
}

impl RaydiumStableSwapDecoder {
    pub fn new(rpc_manager: Arc<RpcManager>) -> Self {
        Self { rpc_manager }
    }

    /// Calculate current price from pool reserves
    async fn calculate_price(&self, pool_address: &str, account_data: &[u8]) -> Result<f64> {
        println!("🔍 DEBUG: Calculating price for Raydium Stable Swap pool {}", pool_address);

        // For stable swaps, the price should be close to 1:1
        // We'll get the actual balances from token accounts
        let token_a_account = utils::bytes_to_pubkey(&account_data[256..288]);
        let token_b_account = utils::bytes_to_pubkey(&account_data[288..320]);

        println!("🔍 DEBUG: token_a_account: {}", token_a_account);
        println!("🔍 DEBUG: token_b_account: {}", token_b_account);

        // Get token account balances
        let balance_a = match self.rpc_manager.get_token_account_balance(&token_a_account).await {
            Ok(balance) => balance,
            Err(e) => {
                println!("⚠️  Failed to get token A balance: {}", e);
                return Ok(0.0);
            }
        };

        let balance_b = match self.rpc_manager.get_token_account_balance(&token_b_account).await {
            Ok(balance) => balance,
            Err(e) => {
                println!("⚠️  Failed to get token B balance: {}", e);
                return Ok(0.0);
            }
        };

        println!("🔍 DEBUG: balance_a: {}, balance_b: {}", balance_a, balance_b);

        // Calculate price: Token B / Token A
        if balance_a > 0 {
            let price = (balance_b as f64) / (balance_a as f64);
            println!("🔍 DEBUG: Calculated Stable Swap price: {}", price);
            Ok(price)
        } else {
            println!("⚠️  Cannot calculate price - no token A balance");
            Ok(0.0)
        }
    }
}

#[async_trait]
impl PoolDecoder for RaydiumStableSwapDecoder {
    fn program_id(&self) -> Pubkey {
        Pubkey::from_str("5quBtoiQqxF9Jv6KYKctB59NT3gtJD2Y65kdnB1Uev3h").unwrap()
    }

    fn can_decode(&self, account_data: &[u8]) -> bool {
        // Raydium Stable Swap pools have specific structure
        // The pool account is typically around 1544 bytes
        if account_data.len() < 1544 {
            return false;
        }

        // Check for Raydium Stable Swap discriminator
        let discriminator = &account_data[0..8];
        // Stable swap pool discriminator
        let expected_discriminator = [0x93, 0x68, 0x8d, 0x26, 0x1e, 0x0a, 0x4c, 0x1d];

        discriminator == expected_discriminator
    }

    async fn decode_pool_info(&self, pool_address: &str, account_data: &[u8]) -> Result<PoolInfo> {
        println!("🔍 DEBUG: Decoding Raydium Stable Swap pool {}", pool_address);
        println!("🔍 DEBUG: Account data length: {} bytes", account_data.len());

        if account_data.len() < 1544 {
            return Err(anyhow::anyhow!("Invalid Raydium Stable Swap pool data length"));
        }

        // Parse Raydium Stable Swap pool structure
        let is_initialized = account_data[8] == 1;
        let bump_seed = account_data[9];
        let token_program_id = utils::bytes_to_pubkey(&account_data[10..42]);
        let token_a_mint = utils::bytes_to_pubkey(&account_data[42..74]);
        let token_b_mint = utils::bytes_to_pubkey(&account_data[74..106]);
        let token_a_account = utils::bytes_to_pubkey(&account_data[106..138]);
        let token_b_account = utils::bytes_to_pubkey(&account_data[138..170]);

        let pool_mint = utils::bytes_to_pubkey(&account_data[170..202]);
        let token_a_mint_decimals = account_data[202];
        let token_b_mint_decimals = account_data[203];

        let trade_fee_numerator = utils::bytes_to_u64(&account_data[204..212]);
        let trade_fee_denominator = utils::bytes_to_u64(&account_data[212..220]);
        let owner_trade_fee_numerator = utils::bytes_to_u64(&account_data[220..228]);
        let owner_trade_fee_denominator = utils::bytes_to_u64(&account_data[228..236]);

        let owner_withdraw_fee_numerator = utils::bytes_to_u64(&account_data[236..244]);
        let owner_withdraw_fee_denominator = utils::bytes_to_u64(&account_data[244..252]);

        let host_fee_numerator = utils::bytes_to_u64(&account_data[252..260]);
        let host_fee_denominator = utils::bytes_to_u64(&account_data[260..268]);

        let curve_type = account_data[268];
        let curve_calculator = &account_data[269..301]; // 32 bytes

        println!("🔍 DEBUG: token_a_mint: {}", token_a_mint);
        println!("🔍 DEBUG: token_b_mint: {}", token_b_mint);
        println!("🔍 DEBUG: is_initialized: {}", is_initialized);
        println!("🔍 DEBUG: curve_type: {}", curve_type);

        let price = self.calculate_price(pool_address, account_data).await.unwrap_or(0.0);

        // Calculate fee rate
        let fee_rate = if trade_fee_denominator > 0 {
            (trade_fee_numerator as f64) / (trade_fee_denominator as f64)
        } else {
            0.0
        };

        Ok(PoolInfo {
            pool_address: pool_address.to_string(),
            pool_type: PoolType::RaydiumStableSwap,
            base_token_mint: token_a_mint.to_string(),
            quote_token_mint: token_b_mint.to_string(),
            base_token_decimals: token_a_mint_decimals,
            quote_token_decimals: token_b_mint_decimals,
            liquidity_usd: 0.0,
            fee_rate,
            created_at: Utc::now(),
            last_updated: Utc::now(),
            is_active: is_initialized,
        })
    }

    async fn decode_pool_reserves(
        &self,
        pool_address: &str,
        account_data: &[u8],
        slot: u64
    ) -> Result<PoolReserve> {
        println!("🔍 DEBUG: Decoding Raydium Stable Swap pool reserves for {}", pool_address);

        if account_data.len() < 1544 {
            return Err(anyhow::anyhow!("Invalid Raydium Stable Swap pool data length"));
        }

        // Get token account addresses
        let token_a_account = utils::bytes_to_pubkey(&account_data[106..138]);
        let token_b_account = utils::bytes_to_pubkey(&account_data[138..170]);

        // Get actual token balances from accounts
        let balance_a = self.rpc_manager
            .get_token_account_balance(&token_a_account).await
            .unwrap_or(0);
        let balance_b = self.rpc_manager
            .get_token_account_balance(&token_b_account).await
            .unwrap_or(0);

        Ok(PoolReserve {
            pool_address: pool_address.to_string(),
            base_token_amount: balance_a,
            quote_token_amount: balance_b,
            timestamp: Utc::now(),
            slot,
        })
    }
}
