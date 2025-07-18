use anyhow::{ anyhow, Result };
use async_trait::async_trait;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use std::sync::Arc;
use crate::rpc::RpcManager;
use crate::pool::decoders::utils;
use crate::pool::types::{ PoolType, PoolInfo, PoolReserve };
use crate::pool::decoders::PoolDecoder;

// Legacy PumpFun v1 decoder
pub struct LegacyPumpFunDecoder {
    rpc_manager: Arc<RpcManager>,
    program_id: Pubkey,
}

impl LegacyPumpFunDecoder {
    pub fn new(rpc_manager: Arc<RpcManager>) -> Self {
        Self {
            rpc_manager,
            program_id: Pubkey::from_str("pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA").unwrap(),
        }
    }

    fn decode_pumpfun_pool_from_account(
        &self,
        _pool_pk: &Pubkey,
        account_data: &[u8]
    ) -> Result<(u64, u64, Pubkey, Pubkey)> {
        if account_data.len() < 211 {
            return Err(anyhow!("Pump.fun account only {} B (<211)", account_data.len()));
        }

        // Skip discriminator (8 bytes) and decode the pool structure
        let pool_data = &account_data[8..211];

        // Extract the pool structure fields
        let _pool_bump = pool_data[0];
        let _index = u16::from_le_bytes([pool_data[1], pool_data[2]]);
        let _creator = utils::bytes_to_pubkey(&pool_data[3..35]);
        let base_mint = utils::bytes_to_pubkey(&pool_data[35..67]);
        let quote_mint = utils::bytes_to_pubkey(&pool_data[67..99]);
        let _lp_mint = utils::bytes_to_pubkey(&pool_data[99..131]);
        let _pool_base_token_account = utils::bytes_to_pubkey(&pool_data[131..163]);
        let _pool_quote_token_account = utils::bytes_to_pubkey(&pool_data[163..195]);
        let _lp_supply = utils::bytes_to_u64(&pool_data[195..203]);

        // For legacy pools, we'll extract reserves directly from account data
        // since we can't make async calls in synchronous methods
        let base_balance = utils::bytes_to_u64(&pool_data[203..211]);
        let quote_balance = utils::bytes_to_u64(&pool_data[195..203]);

        Ok((base_balance, quote_balance, base_mint, quote_mint))
    }
}

#[async_trait]
impl PoolDecoder for LegacyPumpFunDecoder {
    fn program_id(&self) -> Pubkey {
        self.program_id
    }

    fn can_decode(&self, account_data: &[u8]) -> bool {
        account_data.len() >= 211
    }

    async fn decode_pool_info(&self, pool_address: &str, account_data: &[u8]) -> Result<PoolInfo> {
        let pool_pubkey = Pubkey::from_str(pool_address)?;
        let (base_reserves, quote_reserves, base_mint, quote_mint) =
            self.decode_pumpfun_pool_from_account(&pool_pubkey, account_data)?;

        Ok(PoolInfo {
            pool_address: pool_address.to_string(),
            pool_type: PoolType::PumpFunAmm,
            base_token_mint: base_mint.to_string(),
            quote_token_mint: quote_mint.to_string(),
            base_token_decimals: 0,
            quote_token_decimals: 0,
            liquidity_usd: (base_reserves + quote_reserves) as f64,
            fee_rate: 0.0,
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
            self.decode_pumpfun_pool_from_account(&pool_pubkey, account_data)?;

        Ok(PoolReserve {
            pool_address: pool_address.to_string(),
            base_token_amount: base_reserves,
            quote_token_amount: quote_reserves,
            slot,
            timestamp: chrono::Utc::now(),
        })
    }
}

// Legacy Raydium AMM v4 decoder with correct program ID
pub struct LegacyRaydiumAmmDecoder {
    rpc_manager: Arc<RpcManager>,
    program_id: Pubkey,
}

impl LegacyRaydiumAmmDecoder {
    pub fn new(rpc_manager: Arc<RpcManager>) -> Self {
        Self {
            rpc_manager,
            program_id: Pubkey::from_str("RVKd61ztZW9g2VZgPZrFYuXJcZ1t7xvaUo1NkL6MZ5w").unwrap(),
        }
    }

    fn decode_raydium_amm_from_account(
        &self,
        _pool_pk: &Pubkey,
        account_data: &[u8]
    ) -> Result<(u64, u64, Pubkey, Pubkey)> {
        if account_data.len() < 264 {
            return Err(anyhow!("AMM account too short"));
        }

        // Extract mint addresses from pool account
        let base_mint = utils::bytes_to_pubkey(&account_data[168..200]);
        let quote_mint = utils::bytes_to_pubkey(&account_data[216..248]);

        // Extract reserves directly from account data
        let base_balance = utils::bytes_to_u64(&account_data[248..256]);
        let quote_balance = utils::bytes_to_u64(&account_data[256..264]);

        Ok((base_balance, quote_balance, base_mint, quote_mint))
    }
}

#[async_trait]
impl PoolDecoder for LegacyRaydiumAmmDecoder {
    fn program_id(&self) -> Pubkey {
        self.program_id
    }

    fn can_decode(&self, account_data: &[u8]) -> bool {
        account_data.len() >= 264
    }

    async fn decode_pool_info(&self, pool_address: &str, account_data: &[u8]) -> Result<PoolInfo> {
        let pool_pubkey = Pubkey::from_str(pool_address)?;
        let (base_reserves, quote_reserves, base_mint, quote_mint) =
            self.decode_raydium_amm_from_account(&pool_pubkey, account_data)?;

        Ok(PoolInfo {
            pool_address: pool_address.to_string(),
            pool_type: PoolType::RaydiumAmmV4,
            base_token_mint: base_mint.to_string(),
            quote_token_mint: quote_mint.to_string(),
            base_token_decimals: 0,
            quote_token_decimals: 0,
            liquidity_usd: (base_reserves + quote_reserves) as f64,
            fee_rate: 0.0025,
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
            self.decode_raydium_amm_from_account(&pool_pubkey, account_data)?;

        Ok(PoolReserve {
            pool_address: pool_address.to_string(),
            base_token_amount: base_reserves,
            quote_token_amount: quote_reserves,
            slot,
            timestamp: chrono::Utc::now(),
        })
    }
}

// Legacy Raydium CLMM v2 decoder with correct program ID
pub struct LegacyRaydiumClmmDecoder {
    rpc_manager: Arc<RpcManager>,
    program_id: Pubkey,
}

impl LegacyRaydiumClmmDecoder {
    pub fn new(rpc_manager: Arc<RpcManager>) -> Self {
        Self {
            rpc_manager,
            program_id: Pubkey::from_str("CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK").unwrap(),
        }
    }

    fn decode_raydium_clmm_from_account(
        &self,
        _pool_pk: &Pubkey,
        account_data: &[u8]
    ) -> Result<(u64, u64, Pubkey, Pubkey)> {
        if account_data.len() < 1544 {
            return Err(anyhow!("CLMM account too short"));
        }

        // Extract mint addresses from CLMM pool
        let mint_a = utils::bytes_to_pubkey(&account_data[8..40]);
        let mint_b = utils::bytes_to_pubkey(&account_data[40..72]);

        // Extract reserves directly from account data
        let balance_a = utils::bytes_to_u64(&account_data[136..144]);
        let balance_b = utils::bytes_to_u64(&account_data[144..152]);

        Ok((balance_a, balance_b, mint_a, mint_b))
    }
}

#[async_trait]
impl PoolDecoder for LegacyRaydiumClmmDecoder {
    fn program_id(&self) -> Pubkey {
        self.program_id
    }

    fn can_decode(&self, account_data: &[u8]) -> bool {
        account_data.len() >= 1544
    }

    async fn decode_pool_info(&self, pool_address: &str, account_data: &[u8]) -> Result<PoolInfo> {
        let pool_pubkey = Pubkey::from_str(pool_address)?;
        let (base_reserves, quote_reserves, base_mint, quote_mint) =
            self.decode_raydium_clmm_from_account(&pool_pubkey, account_data)?;

        Ok(PoolInfo {
            pool_address: pool_address.to_string(),
            pool_type: PoolType::RaydiumClmm,
            base_token_mint: base_mint.to_string(),
            quote_token_mint: quote_mint.to_string(),
            base_token_decimals: 0,
            quote_token_decimals: 0,
            liquidity_usd: (base_reserves + quote_reserves) as f64,
            fee_rate: 0.0025,
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
            self.decode_raydium_clmm_from_account(&pool_pubkey, account_data)?;

        Ok(PoolReserve {
            pool_address: pool_address.to_string(),
            base_token_amount: base_reserves,
            quote_token_amount: quote_reserves,
            slot,
            timestamp: chrono::Utc::now(),
        })
    }
}

// Orca Whirlpool decoder
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

    fn decode_orca_whirlpool_from_account(
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
        let (base_reserves, quote_reserves, base_mint, quote_mint) =
            self.decode_orca_whirlpool_from_account(&pool_pubkey, account_data)?;

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
            self.decode_orca_whirlpool_from_account(&pool_pubkey, account_data)?;

        Ok(PoolReserve {
            pool_address: pool_address.to_string(),
            base_token_amount: base_reserves,
            quote_token_amount: quote_reserves,
            slot,
            timestamp: chrono::Utc::now(),
        })
    }
}

// Universal decoder that can handle any pool type based on program ID
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
            // Pump.fun v1 (legacy)
            "pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA" => {
                let decoder = LegacyPumpFunDecoder::new(self.rpc_manager.clone());
                decoder.decode_pumpfun_pool_from_account(pool_pk, account_data)
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
                let decoder = LegacyRaydiumClmmDecoder::new(self.rpc_manager.clone());
                decoder.decode_raydium_clmm_from_account(pool_pk, account_data)
            }
            // Raydium AMM v4 (legacy)
            "RVKd61ztZW9g2VZgPZrFYuXJcZ1t7xvaUo1NkL6MZ5w" => {
                let decoder = LegacyRaydiumAmmDecoder::new(self.rpc_manager.clone());
                decoder.decode_raydium_amm_from_account(pool_pk, account_data)
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
                decoder.decode_orca_whirlpool_from_account(pool_pk, account_data)
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
