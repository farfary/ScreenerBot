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

/// Raydium AMM V5 Pool decoder
/// Program ID: 5quBtoiQqxF9Jv6KYKctB59NT3gtJD2Y65kdnB1Uev3h
pub struct RaydiumAmmV5Decoder {
    rpc_manager: Arc<RpcManager>,
}

impl RaydiumAmmV5Decoder {
    pub fn new(rpc_manager: Arc<RpcManager>) -> Self {
        Self { rpc_manager }
    }

    /// Calculate current price from pool reserves
    async fn calculate_price(&self, pool_address: &str, account_data: &[u8]) -> Result<f64> {
        println!("🔍 DEBUG: Calculating price for Raydium AMM V5 pool {}", pool_address);

        // Get token account addresses from pool data
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
            println!("🔍 DEBUG: Calculated AMM V5 price: {}", price);
            Ok(price)
        } else {
            println!("⚠️  Cannot calculate price - no token A balance");
            Ok(0.0)
        }
    }
}

#[async_trait]
impl PoolDecoder for RaydiumAmmV5Decoder {
    fn program_id(&self) -> Pubkey {
        Pubkey::from_str("5quBtoiQqxF9Jv6KYKctB59NT3gtJD2Y65kdnB1Uev3h").unwrap()
    }

    fn can_decode(&self, account_data: &[u8]) -> bool {
        // Raydium AMM V5 pools have specific structure
        // The pool account is typically around 1024 bytes
        if account_data.len() < 1024 {
            return false;
        }

        // Check for Raydium AMM V5 discriminator
        let discriminator = &account_data[0..8];
        // AMM V5 pool discriminator
        let expected_discriminator = [0xa3, 0x7c, 0x8b, 0x9f, 0x2e, 0x1c, 0x9d, 0x4a];

        discriminator == expected_discriminator
    }

    async fn decode_pool_info(&self, pool_address: &str, account_data: &[u8]) -> Result<PoolInfo> {
        println!("🔍 DEBUG: Decoding Raydium AMM V5 pool {}", pool_address);
        println!("🔍 DEBUG: Account data length: {} bytes", account_data.len());

        if account_data.len() < 1024 {
            return Err(anyhow::anyhow!("Invalid Raydium AMM V5 pool data length"));
        }

        // Parse Raydium AMM V5 pool structure
        let is_initialized = account_data[8] == 1;
        let _nonce = account_data[9];
        let _order_num = utils::bytes_to_u64(&account_data[10..18]);
        let _depth = utils::bytes_to_u64(&account_data[18..26]);
        let base_decimal = utils::bytes_to_u64(&account_data[26..34]);
        let quote_decimal = utils::bytes_to_u64(&account_data[34..42]);
        let _state = utils::bytes_to_u64(&account_data[42..50]);
        let _reset_flag = utils::bytes_to_u64(&account_data[50..58]);
        let _min_size = utils::bytes_to_u64(&account_data[58..66]);
        let _vol_max_cut_ratio = utils::bytes_to_u64(&account_data[66..74]);
        let _amount_wave_ratio = utils::bytes_to_u64(&account_data[74..82]);
        let _base_lot_size = utils::bytes_to_u64(&account_data[82..90]);
        let _quote_lot_size = utils::bytes_to_u64(&account_data[90..98]);
        let _min_price_multiplier = utils::bytes_to_u64(&account_data[98..106]);
        let _max_price_multiplier = utils::bytes_to_u64(&account_data[106..114]);
        let _system_decimal_value = utils::bytes_to_u64(&account_data[114..122]);
        let _min_separate_numerator = utils::bytes_to_u64(&account_data[122..130]);
        let _min_separate_denominator = utils::bytes_to_u64(&account_data[130..138]);
        let trade_fee_numerator = utils::bytes_to_u64(&account_data[138..146]);
        let trade_fee_denominator = utils::bytes_to_u64(&account_data[146..154]);
        let _pnl_numerator = utils::bytes_to_u64(&account_data[154..162]);
        let _pnl_denominator = utils::bytes_to_u64(&account_data[162..170]);
        let swap_fee_numerator = utils::bytes_to_u64(&account_data[170..178]);
        let swap_fee_denominator = utils::bytes_to_u64(&account_data[178..186]);
        let _base_need_take_pnl = utils::bytes_to_u64(&account_data[186..194]);
        let _quote_need_take_pnl = utils::bytes_to_u64(&account_data[194..202]);
        let _quote_total_pnl = utils::bytes_to_u64(&account_data[202..210]);
        let _base_total_pnl = utils::bytes_to_u64(&account_data[210..218]);
        let _pool_open_time = utils::bytes_to_u64(&account_data[218..226]);
        let _punish_pc_amount = utils::bytes_to_u64(&account_data[226..234]);
        let _punish_coin_amount = utils::bytes_to_u64(&account_data[234..242]);
        let _orderbook_to_init_time = utils::bytes_to_u64(&account_data[242..250]);
        let _swap_base_in_amount = utils::bytes_to_u128(&account_data[250..266]);
        let _swap_quote_out_amount = utils::bytes_to_u128(&account_data[266..282]);
        let _swap_base2_quote_fee = utils::bytes_to_u64(&account_data[282..290]);
        let _swap_quote_in_amount = utils::bytes_to_u128(&account_data[290..306]);
        let _swap_base_out_amount = utils::bytes_to_u128(&account_data[306..322]);
        let _swap_quote2_base_fee = utils::bytes_to_u64(&account_data[322..330]);
        let base_mint = utils::bytes_to_pubkey(&account_data[400..432]);
        let quote_mint = utils::bytes_to_pubkey(&account_data[432..464]);
        let _base_vault = utils::bytes_to_pubkey(&account_data[464..496]);
        let _quote_vault = utils::bytes_to_pubkey(&account_data[496..528]);
        let base_decimal_raw = base_decimal as u8;
        let quote_decimal_raw = quote_decimal as u8;

        println!("🔍 DEBUG: base_mint: {}", base_mint);
        println!("🔍 DEBUG: quote_mint: {}", quote_mint);
        println!("🔍 DEBUG: is_initialized: {}", is_initialized);
        println!("🔍 DEBUG: base_decimal: {}", base_decimal_raw);
        println!("🔍 DEBUG: quote_decimal: {}", quote_decimal_raw);

        let _price = self.calculate_price(pool_address, account_data).await.unwrap_or(0.0);

        // Calculate fee rate
        let fee_rate = if swap_fee_denominator > 0 {
            (swap_fee_numerator as f64) / (swap_fee_denominator as f64)
        } else if trade_fee_denominator > 0 {
            (trade_fee_numerator as f64) / (trade_fee_denominator as f64)
        } else {
            0.0
        };

        Ok(PoolInfo {
            pool_address: pool_address.to_string(),
            pool_type: PoolType::RaydiumAmmV5,
            base_token_mint: base_mint.to_string(),
            quote_token_mint: quote_mint.to_string(),
            base_token_decimals: base_decimal_raw,
            quote_token_decimals: quote_decimal_raw,
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
        println!("🔍 DEBUG: Decoding Raydium AMM V5 pool reserves for {}", pool_address);

        if account_data.len() < 1024 {
            return Err(anyhow::anyhow!("Invalid Raydium AMM V5 pool data length"));
        }

        // Get token vault addresses
        let base_vault = utils::bytes_to_pubkey(&account_data[464..496]);
        let quote_vault = utils::bytes_to_pubkey(&account_data[496..528]);

        // Get actual token balances from vaults
        let base_balance = self.rpc_manager
            .get_token_account_balance(&base_vault).await
            .unwrap_or(0);
        let quote_balance = self.rpc_manager
            .get_token_account_balance(&quote_vault).await
            .unwrap_or(0);

        Ok(PoolReserve {
            pool_address: pool_address.to_string(),
            base_token_amount: base_balance,
            quote_token_amount: quote_balance,
            timestamp: Utc::now(),
            slot,
        })
    }
}
