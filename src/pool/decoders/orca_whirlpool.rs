use anyhow::{ anyhow, Result };
use async_trait::async_trait;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use std::sync::Arc;
use crate::rpc::RpcManager;
use crate::pool::decoders::utils;
use crate::pool::types::{ PoolType, PoolInfo, PoolReserve };
use crate::pool::decoders::PoolDecoder;

/// Orca Whirlpool decoder
pub struct OrcaWhirlpoolDecoder {
    rpc_manager: Arc<RpcManager>,
    program_id: Pubkey,
}

impl OrcaWhirlpoolDecoder {
    pub fn new(rpc_manager: Arc<RpcManager>) -> Self {
        Self {
            rpc_manager,
            program_id: Pubkey::from_str("whirLb9FtDwZ2Bi4FXe65aaPaJqmCj7QSfUeCrpuHgx").unwrap(),
        }
    }

    pub fn decode_pool_from_account(
        &self,
        _pool_pk: &Pubkey,
        account_data: &[u8]
    ) -> Result<(u64, u64, Pubkey, Pubkey)> {
        if account_data.len() < 653 {
            return Err(anyhow!("Orca Whirlpool account too short"));
        }

        // Extract mint addresses from Whirlpool
        let mint_a = utils::bytes_to_pubkey(&account_data[8..40]);
        let mint_b = utils::bytes_to_pubkey(&account_data[40..72]);

        // Extract reserves from account data directly
        let balance_a = utils::bytes_to_u64(&account_data[136..144]);
        let balance_b = utils::bytes_to_u64(&account_data[144..152]);

        Ok((balance_a, balance_b, mint_a, mint_b))
    }
}

#[async_trait]
impl PoolDecoder for OrcaWhirlpoolDecoder {
    fn program_id(&self) -> Pubkey {
        self.program_id
    }

    fn can_decode(&self, account_data: &[u8]) -> bool {
        account_data.len() >= 653
    }

    async fn decode_pool_info(&self, pool_address: &str, account_data: &[u8]) -> Result<PoolInfo> {
        let pool_pubkey = Pubkey::from_str(pool_address)?;
        let (base_reserves, quote_reserves, base_mint, quote_mint) = self.decode_pool_from_account(
            &pool_pubkey,
            account_data
        )?;

        Ok(PoolInfo {
            pool_address: pool_address.to_string(),
            pool_type: PoolType::OrcaWhirlpool,
            base_token_mint: base_mint.to_string(),
            quote_token_mint: quote_mint.to_string(),
            base_token_decimals: 0,
            quote_token_decimals: 0,
            liquidity_usd: (base_reserves + quote_reserves) as f64,
            fee_rate: 0.003,
            created_at: chrono::Utc::now(),
            last_updated: chrono::Utc::now(),
            is_active: true,
        })
    }

    async fn decode_pool_reserves(
        &self,
        pool_address: &str,
        account_data: &[u8],
        slot: u64
    ) -> Result<PoolReserve> {
        let pool_pubkey = Pubkey::from_str(pool_address)?;
        let (base_reserves, quote_reserves, _base_mint, _quote_mint) =
            self.decode_pool_from_account(&pool_pubkey, account_data)?;

        Ok(PoolReserve {
            pool_address: pool_address.to_string(),
            base_token_amount: base_reserves,
            quote_token_amount: quote_reserves,
            slot,
            timestamp: chrono::Utc::now(),
        })
    }
}
