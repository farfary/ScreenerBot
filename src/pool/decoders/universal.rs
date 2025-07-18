use anyhow::{ anyhow, Result };
use async_trait::async_trait;
use solana_sdk::pubkey::Pubkey;
use std::sync::Arc;
use crate::rpc::RpcManager;
use crate::pool::decoders::utils;
use crate::pool::types::{ PoolInfo, PoolReserve };
use crate::pool::decoders::PoolDecoder;
use crate::pool::decoders::pumpfun::PumpFunDecoder;
use crate::pool::decoders::raydium_amm::RaydiumAmmDecoder;
use crate::pool::decoders::raydium_clmm::RaydiumClmmDecoder;
use crate::pool::decoders::orca_whirlpool::OrcaWhirlpoolDecoder;

/// Universal pool decoder that can handle any pool type based on program ID
pub struct UniversalPoolDecoder {
    rpc_manager: Arc<RpcManager>,
}

impl UniversalPoolDecoder {
    pub fn new(rpc_manager: Arc<RpcManager>) -> Self {
        Self { rpc_manager }
    }

    pub fn decode_any_pool(
        &self,
        account_data: &[u8],
        owner: &Pubkey,
        pool_pk: &Pubkey
    ) -> Result<(u64, u64, Pubkey, Pubkey)> {
        let owner_str = owner.to_string();

        match owner_str.as_str() {
            // PumpFun v1
            "pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA" => {
                let decoder = PumpFunDecoder::new(self.rpc_manager.clone());
                decoder.decode_pool_from_account(pool_pk, account_data)
            }
            // PumpFun v2 CPMM (current)
            "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P" => {
                // Use the current PumpFun decoder logic
                if account_data.len() < 99 {
                    return Err(anyhow!("PumpFun2 account too short"));
                }

                let base_mint = utils::bytes_to_pubkey(&account_data[35..67]);
                let quote_mint = utils::bytes_to_pubkey(&account_data[67..99]);

                // For PumpFun v2, we need to extract virtual reserves
                let virtual_token_reserves = utils::bytes_to_u64(&account_data[8..16]);
                let virtual_sol_reserves = utils::bytes_to_u64(&account_data[16..24]);

                Ok((virtual_token_reserves, virtual_sol_reserves, base_mint, quote_mint))
            }
            // Raydium CLMM v2
            "CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK" => {
                let decoder = RaydiumClmmDecoder::new(self.rpc_manager.clone());
                decoder.decode_pool_from_account(pool_pk, account_data)
            }
            // Raydium AMM v4
            "RVKd61ztZW9g2VZgPZrFYuXJcZ1t7xvaUo1NkL6MZ5w" => {
                let decoder = RaydiumAmmDecoder::new(self.rpc_manager.clone());
                decoder.decode_pool_from_account(pool_pk, account_data)
            }
            // Raydium AMM v4 (current)
            "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8" => {
                // Use current Raydium AMM v4 decoder
                if account_data.len() < 752 {
                    return Err(anyhow!("Raydium AMM v4 account too short"));
                }

                let base_mint = utils::bytes_to_pubkey(&account_data[8..40]);
                let quote_mint = utils::bytes_to_pubkey(&account_data[40..72]);
                let base_vault = utils::bytes_to_pubkey(&account_data[72..104]);
                let quote_vault = utils::bytes_to_pubkey(&account_data[104..136]);

                let base_balance = self.rpc_manager.get_token_account_balance_sync(&base_vault)?;
                let quote_balance = self.rpc_manager.get_token_account_balance_sync(&quote_vault)?;

                Ok((base_balance, quote_balance, base_mint, quote_mint))
            }
            // Raydium CPMM
            "CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C" => {
                if account_data.len() < 544 {
                    return Err(anyhow!("Raydium CPMM account too short"));
                }

                let mint_a = utils::bytes_to_pubkey(&account_data[8..40]);
                let mint_b = utils::bytes_to_pubkey(&account_data[40..72]);
                let vault_a = utils::bytes_to_pubkey(&account_data[72..104]);
                let vault_b = utils::bytes_to_pubkey(&account_data[104..136]);

                let balance_a = self.rpc_manager.get_token_account_balance_sync(&vault_a)?;
                let balance_b = self.rpc_manager.get_token_account_balance_sync(&vault_b)?;

                Ok((balance_a, balance_b, mint_a, mint_b))
            }
            // Orca Whirlpool
            "whirLb9FtDwZ2Bi4FXe65aaPaJqmCj7QSfUeCrpuHgx" => {
                let decoder = OrcaWhirlpoolDecoder::new(self.rpc_manager.clone());
                decoder.decode_pool_from_account(pool_pk, account_data)
            }
            // Raydium CLMM (current)
            "CAMMCzo5YL8w4VFF8KVHrK22GGUQzXMVCaRz9qfUAEA" => {
                // Use current Raydium CLMM decoder
                if account_data.len() < 1544 {
                    return Err(anyhow!("Raydium CLMM account too short"));
                }

                let mint_a = utils::bytes_to_pubkey(&account_data[8..40]);
                let mint_b = utils::bytes_to_pubkey(&account_data[40..72]);
                let vault_a = utils::bytes_to_pubkey(&account_data[72..104]);
                let vault_b = utils::bytes_to_pubkey(&account_data[104..136]);

                let balance_a = self.rpc_manager.get_token_account_balance_sync(&vault_a)?;
                let balance_b = self.rpc_manager.get_token_account_balance_sync(&vault_b)?;

                Ok((balance_a, balance_b, mint_a, mint_b))
            }
            _ => Err(anyhow!("Unsupported program id {} for pool {}", owner_str, pool_pk)),
        }
    }

    pub fn decode_any_pool_price(
        &self,
        account_data: &[u8],
        owner: &Pubkey,
        pool_pk: &Pubkey
    ) -> Result<(u64, u64, f64)> {
        let (base_amt, quote_amt, base_mint, quote_mint) = self.decode_any_pool(
            account_data,
            owner,
            pool_pk
        )?;

        if base_amt == 0 {
            return Err(anyhow!("base reserve is zero – cannot calculate price"));
        }

        // Get token decimals
        let base_dec = self.rpc_manager.get_token_decimals_sync(&base_mint)? as i32;
        let quote_dec = self.rpc_manager.get_token_decimals_sync(&quote_mint)? as i32;

        // Calculate price of one whole base token expressed in quote tokens
        let price = ((quote_amt as f64) / (base_amt as f64)) * (10f64).powi(base_dec - quote_dec);

        Ok((base_amt, quote_amt, price))
    }
}

#[async_trait]
impl PoolDecoder for UniversalPoolDecoder {
    fn program_id(&self) -> Pubkey {
        // Universal decoder doesn't have a fixed program ID
        Pubkey::default()
    }

    fn can_decode(&self, _account_data: &[u8]) -> bool {
        // Universal decoder can try to decode any pool
        true
    }

    async fn decode_pool_info(
        &self,
        _pool_address: &str,
        _account_data: &[u8]
    ) -> Result<PoolInfo> {
        // This would require additional logic to determine the program ID
        // For now, return an error
        Err(anyhow!("Universal decoder requires program ID context"))
    }

    async fn decode_pool_reserves(
        &self,
        _pool_address: &str,
        _account_data: &[u8],
        _slot: u64
    ) -> Result<PoolReserve> {
        // This would require additional logic to determine the program ID
        // For now, return an error
        Err(anyhow!("Universal decoder requires program ID context"))
    }
}
