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

/// Raydium CPMM (Constant Product Market Maker) Pool decoder
/// Program ID: CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C
pub struct RaydiumCpmmDecoder {
    rpc_manager: Arc<RpcManager>,
}

impl RaydiumCpmmDecoder {
    pub fn new(rpc_manager: Arc<RpcManager>) -> Self {
        Self { rpc_manager }
    }

    /// Calculate current price from pool reserves
    async fn calculate_price(&self, pool_address: &str, account_data: &[u8]) -> Result<f64> {
        println!("🔍 DEBUG: Calculating price for Raydium CPMM pool {}", pool_address);

        // Get token vault addresses
        let token_0_vault = utils::bytes_to_pubkey(&account_data[72..104]);
        let token_1_vault = utils::bytes_to_pubkey(&account_data[104..136]);

        println!("🔍 DEBUG: token_0_vault: {}", token_0_vault);
        println!("🔍 DEBUG: token_1_vault: {}", token_1_vault);

        // Get token account balances
        let balance_0 = match self.rpc_manager.get_token_account_balance(&token_0_vault).await {
            Ok(balance) => balance,
            Err(e) => {
                println!("⚠️  Failed to get token 0 balance: {}", e);
                return Ok(0.0);
            }
        };

        let balance_1 = match self.rpc_manager.get_token_account_balance(&token_1_vault).await {
            Ok(balance) => balance,
            Err(e) => {
                println!("⚠️  Failed to get token 1 balance: {}", e);
                return Ok(0.0);
            }
        };

        println!("🔍 DEBUG: balance_0: {}, balance_1: {}", balance_0, balance_1);

        // Calculate price: Token1 / Token0
        if balance_0 > 0 {
            let price = (balance_1 as f64) / (balance_0 as f64);
            println!("🔍 DEBUG: Calculated CPMM price: {}", price);
            Ok(price)
        } else {
            println!("⚠️  Cannot calculate price - no token 0 balance");
            Ok(0.0)
        }
    }
}

#[async_trait]
impl PoolDecoder for RaydiumCpmmDecoder {
    fn program_id(&self) -> Pubkey {
        Pubkey::from_str("CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C").unwrap()
    }

    fn can_decode(&self, account_data: &[u8]) -> bool {
        // Raydium CPMM pools have specific structure
        // The pool state account is typically around 544 bytes
        if account_data.len() < 544 {
            return false;
        }

        // Check for Raydium CPMM discriminator
        let discriminator = &account_data[0..8];
        // CPMM pool state discriminator
        let expected_discriminator = [0x7c, 0x0a, 0x5e, 0x68, 0x9a, 0x4b, 0x0b, 0x28];

        discriminator == expected_discriminator
    }

    async fn decode_pool_info(&self, pool_address: &str, account_data: &[u8]) -> Result<PoolInfo> {
        println!("🔍 DEBUG: Decoding Raydium CPMM pool {}", pool_address);
        println!("🔍 DEBUG: Account data length: {} bytes", account_data.len());

        if account_data.len() < 544 {
            return Err(anyhow::anyhow!("Invalid Raydium CPMM pool data length"));
        }

        // Parse Raydium CPMM pool structure
        let _bump = account_data[8];
        let _config_index = utils::bytes_to_u16(&account_data[9..11]);
        let _pool_creator = utils::bytes_to_pubkey(&account_data[11..43]);

        let token_0_mint = utils::bytes_to_pubkey(&account_data[43..75]);
        let token_1_mint = utils::bytes_to_pubkey(&account_data[75..107]);
        let _lp_mint = utils::bytes_to_pubkey(&account_data[107..139]);

        let _token_0_vault = utils::bytes_to_pubkey(&account_data[139..171]);
        let _token_1_vault = utils::bytes_to_pubkey(&account_data[171..203]);

        let _observation_key = utils::bytes_to_pubkey(&account_data[203..235]);

        let _auth_bump = account_data[235];
        let status = account_data[236];
        let _lp_mint_decimals = account_data[237];
        let mint_0_decimals = account_data[238];
        let mint_1_decimals = account_data[239];

        let lp_supply = utils::bytes_to_u64(&account_data[240..248]);
        let _protocol_fees_token_0 = utils::bytes_to_u64(&account_data[248..256]);
        let _protocol_fees_token_1 = utils::bytes_to_u64(&account_data[256..264]);
        let _fund_fees_token_0 = utils::bytes_to_u64(&account_data[264..272]);
        let _fund_fees_token_1 = utils::bytes_to_u64(&account_data[272..280]);

        let _open_time = utils::bytes_to_u64(&account_data[280..288]);

        println!("🔍 DEBUG: token_0_mint: {}", token_0_mint);
        println!("🔍 DEBUG: token_1_mint: {}", token_1_mint);
        println!("🔍 DEBUG: lp_supply: {}", lp_supply);
        println!("🔍 DEBUG: status: {}", status);

        let _price = self.calculate_price(pool_address, account_data).await.unwrap_or(0.0);

        Ok(PoolInfo {
            pool_address: pool_address.to_string(),
            pool_type: PoolType::RaydiumCpmm,
            base_token_mint: token_0_mint.to_string(),
            quote_token_mint: token_1_mint.to_string(),
            base_token_decimals: mint_0_decimals,
            quote_token_decimals: mint_1_decimals,
            liquidity_usd: 0.0,
            fee_rate: 0.0025, // Default fee rate
            created_at: Utc::now(),
            last_updated: Utc::now(),
            is_active: status == 1,
        })
    }

    async fn decode_pool_reserves(
        &self,
        pool_address: &str,
        account_data: &[u8],
        slot: u64
    ) -> Result<PoolReserve> {
        println!("🔍 DEBUG: Decoding Raydium CPMM pool reserves for {}", pool_address);

        if account_data.len() < 544 {
            return Err(anyhow::anyhow!("Invalid Raydium CPMM pool data length"));
        }

        // Get token vault addresses
        let token_0_vault = utils::bytes_to_pubkey(&account_data[139..171]);
        let token_1_vault = utils::bytes_to_pubkey(&account_data[171..203]);

        // Get actual token balances from vaults
        let balance_0 = self.rpc_manager
            .get_token_account_balance(&token_0_vault).await
            .unwrap_or(0);
        let balance_1 = self.rpc_manager
            .get_token_account_balance(&token_1_vault).await
            .unwrap_or(0);

        Ok(PoolReserve {
            pool_address: pool_address.to_string(),
            base_token_amount: balance_0,
            quote_token_amount: balance_1,
            timestamp: Utc::now(),
            slot,
        })
    }
}
