use super::decoders::{ DecoderRegistry, PoolInfo, PriceInfo };
use crate::rpc::RpcManager;
use anyhow::{ Context, Result };
use solana_sdk::{ account::Account, pubkey::Pubkey };
use std::sync::Arc;
use log::warn;

/// Pool data fetcher that uses RPC to get account data and decode pools
pub struct PoolDataFetcher {
    rpc_manager: Arc<RpcManager>,
    decoder_registry: DecoderRegistry,
}

impl PoolDataFetcher {
    /// Create a new pool data fetcher
    pub fn new(rpc_manager: Arc<RpcManager>) -> Self {
        Self {
            rpc_manager,
            decoder_registry: DecoderRegistry::new(),
        }
    }

    /// Fetch and decode pool data from a pool address
    pub async fn fetch_pool_data(&self, pool_address: &Pubkey) -> Result<PoolInfo> {
        // Get account data from RPC
        let account_data = self
            .fetch_account_data(pool_address).await
            .context("Failed to fetch pool account data")?;

        // Find the appropriate decoder based on account owner (program ID)
        let program_id = account_data.owner;
        let mut pool_info = self.decoder_registry
            .decode_pool(&program_id, &account_data.data)
            .context("Failed to decode pool data")?;

        // Set the pool address
        pool_info.pool_address = *pool_address;

        // Fetch token vault balances to get actual reserves
        self.fetch_and_update_reserves(&mut pool_info).await?;

        // Fetch token decimals if not already set correctly
        self.fetch_and_update_token_decimals(&mut pool_info).await?;

        Ok(pool_info)
    }

    /// Fetch multiple pools in batch
    pub async fn fetch_multiple_pools(&self, pool_addresses: &[Pubkey]) -> Result<Vec<PoolInfo>> {
        let mut pool_infos = Vec::new();

        // For now, fetch sequentially. In the future, we could implement batch fetching
        for pool_address in pool_addresses {
            match self.fetch_pool_data(pool_address).await {
                Ok(pool_info) => pool_infos.push(pool_info),
                Err(e) => {
                    warn!("Failed to fetch pool data for {}: {}", pool_address, e);
                }
            }
        }

        Ok(pool_infos)
    }

    /// Calculate price for a pool
    pub fn calculate_price(&self, pool_info: &PoolInfo) -> Result<f64> {
        self.decoder_registry.calculate_price(&pool_info.program_id, pool_info)
    }

    /// Get price information with additional metadata
    pub fn get_price_info(&self, pool_info: &PoolInfo) -> Result<PriceInfo> {
        let price = self.calculate_price(pool_info)?;

        Ok(PriceInfo {
            price,
            token_0_symbol: "UNKNOWN".to_string(), // Would need to fetch from token metadata
            token_1_symbol: "UNKNOWN".to_string(), // Would need to fetch from token metadata
            pool_type: pool_info.pool_type.clone(),
            program_id: pool_info.program_id,
            pool_address: pool_info.pool_address,
            last_update: chrono::Utc::now(),
        })
    }

    /// Get supported program IDs
    pub fn get_supported_programs(&self) -> Vec<Pubkey> {
        self.decoder_registry.get_supported_programs()
    }

    /// Check if a program ID is supported
    pub fn is_program_supported(&self, program_id: &Pubkey) -> bool {
        self.decoder_registry.get_decoder(program_id).is_some()
    }

    /// Fetch account data using the RPC manager
    async fn fetch_account_data(&self, address: &Pubkey) -> Result<Account> {
        // Use the RPC manager to get account data
        let account = self.rpc_manager
            .get_account(address).await
            .context("Failed to get account from RPC")?;

        Ok(account)
    }

    /// Fetch and update token vault balances
    async fn fetch_and_update_reserves(&self, pool_info: &mut PoolInfo) -> Result<()> {
        // Fetch vault balances
        let vault_0_account = self.fetch_account_data(&pool_info.token_vault_0).await?;
        let vault_1_account = self.fetch_account_data(&pool_info.token_vault_1).await?;

        // Parse token account data to get balances
        pool_info.reserve_0 = self.parse_token_account_balance(&vault_0_account.data)?;
        pool_info.reserve_1 = self.parse_token_account_balance(&vault_1_account.data)?;

        Ok(())
    }

    /// Fetch and update token decimals from mint accounts
    async fn fetch_and_update_token_decimals(&self, pool_info: &mut PoolInfo) -> Result<()> {
        // Fetch mint account data
        let mint_0_account = self.fetch_account_data(&pool_info.token_mint_0).await?;
        let mint_1_account = self.fetch_account_data(&pool_info.token_mint_1).await?;

        // Parse mint data to get decimals
        pool_info.decimals_0 = self.parse_mint_decimals(&mint_0_account.data)?;
        pool_info.decimals_1 = self.parse_mint_decimals(&mint_1_account.data)?;

        Ok(())
    }

    /// Parse token account data to extract balance
    fn parse_token_account_balance(&self, data: &[u8]) -> Result<u64> {
        if data.len() < 72 {
            return Err(anyhow::anyhow!("Invalid token account data length"));
        }

        // Token account balance is at offset 64 (8 bytes)
        let balance_bytes: [u8; 8] = data[64..72].try_into()?;
        Ok(u64::from_le_bytes(balance_bytes))
    }

    /// Parse mint account data to extract decimals
    fn parse_mint_decimals(&self, data: &[u8]) -> Result<u8> {
        if data.len() < 45 {
            return Err(anyhow::anyhow!("Invalid mint account data length"));
        }

        // Mint decimals is at offset 44 (1 byte)
        Ok(data[44])
    }
}

/// Helper functions for working with pool data
pub mod pool_utils {
    use super::*;

    /// Find pools by token mint
    pub async fn find_pools_by_token(
        _fetcher: &PoolDataFetcher,
        _token_mint: &Pubkey,
        _program_ids: &[Pubkey]
    ) -> Result<Vec<PoolInfo>> {
        // This would require scanning for pools containing the token
        // For now, this is a placeholder that would need to be implemented
        // with program-specific logic or indexing
        todo!("Implement pool scanning by token mint")
    }

    /// Calculate pool TVL in base token units
    pub fn calculate_tvl(pool_info: &PoolInfo) -> f64 {
        let reserve_0_adjusted =
            (pool_info.reserve_0 as f64) / (10_f64).powi(pool_info.decimals_0 as i32);
        let reserve_1_adjusted =
            (pool_info.reserve_1 as f64) / (10_f64).powi(pool_info.decimals_1 as i32);

        // Return TVL in terms of token 1 (quote token)
        reserve_1_adjusted + reserve_0_adjusted * pool_info.calculate_price().unwrap_or(0.0)
    }

    /// Check if pool has sufficient liquidity
    pub fn has_sufficient_liquidity(pool_info: &PoolInfo, min_tvl: f64) -> bool {
        calculate_tvl(pool_info) >= min_tvl
    }
}

// Add a method to PoolInfo for convenience
impl PoolInfo {
    /// Calculate price using the appropriate decoder
    pub fn calculate_price(&self) -> Result<f64> {
        let registry = DecoderRegistry::new();
        registry.calculate_price(&self.program_id, self)
    }
}
