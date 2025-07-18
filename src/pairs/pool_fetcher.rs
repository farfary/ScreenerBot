use super::decoders::{ DecoderRegistry, PoolInfo, PriceInfo };
use super::types::TokenPair;
use crate::rpc::RpcManager;
use anyhow::{ Context, Result };
use reqwest::Client;
use serde::Deserialize;
use solana_sdk::{ account::Account, pubkey::Pubkey };
use std::sync::Arc;
use std::time::Duration;
use log::{ warn, debug, error };
use tokio::time;

/// Pool data fetcher that uses RPC to get account data and decode pools
pub struct PoolDataFetcher {
    rpc_manager: Arc<RpcManager>,
    decoder_registry: DecoderRegistry,
    http_client: Client,
    rate_limit_delay: Duration,
}

#[derive(Debug, Deserialize)]
struct DexScreenerResponse {
    pairs: Option<Vec<TokenPair>>,
}

impl PoolDataFetcher {
    /// Create a new pool data fetcher
    pub fn new(rpc_manager: Arc<RpcManager>) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("ScreenerBot/1.0")
            .build()
            .expect("Failed to create HTTP client");

        Self {
            rpc_manager,
            decoder_registry: DecoderRegistry::new(),
            http_client: client,
            rate_limit_delay: Duration::from_millis(100), // Conservative rate limiting
        }
    }

    /// Create a new pool data fetcher with custom HTTP client and rate limiting
    pub fn new_with_client(
        rpc_manager: Arc<RpcManager>,
        client: Client,
        rate_limit_delay: Duration
    ) -> Self {
        Self {
            rpc_manager,
            decoder_registry: DecoderRegistry::new(),
            http_client: client,
            rate_limit_delay,
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

    /// Fetch multiple pools in batch using get_multiple_accounts for efficiency
    pub async fn fetch_multiple_pools(&self, pool_addresses: &[Pubkey]) -> Result<Vec<PoolInfo>> {
        if pool_addresses.is_empty() {
            return Ok(Vec::new());
        }

        let batch_size = 100; // Solana RPC supports up to 100 accounts per call
        let mut all_pool_infos = Vec::new();

        for chunk in pool_addresses.chunks(batch_size) {
            debug!("Fetching batch of {} pool accounts", chunk.len());

            // 1. Fetch all pool account data in one RPC call
            let pool_accounts = match self.rpc_manager.get_multiple_accounts(chunk).await {
                Ok(accounts) => accounts,
                Err(e) => {
                    warn!("Failed to fetch batch of pool accounts: {}", e);
                    // Fallback to individual fetching for this batch
                    let mut fallback_accounts = Vec::new();
                    for pool_address in chunk {
                        match self.rpc_manager.get_account(pool_address).await {
                            Ok(account) => fallback_accounts.push(Some(account)),
                            Err(e) => {
                                warn!("Failed to fetch pool data for {}: {}", pool_address, e);
                                fallback_accounts.push(None);
                            }
                        }
                    }
                    fallback_accounts
                }
            };

            // 2. Decode all pool accounts and collect all vault pubkeys for batch fetch
            let mut decoded_pools = Vec::new();
            let mut all_vault_pubkeys = Vec::new();
            for (pool_address, account_opt) in chunk.iter().zip(pool_accounts.iter()) {
                if let Some(account) = account_opt {
                    match self.decoder_registry.decode_pool(&account.owner, &account.data) {
                        Ok(mut pool_info) => {
                            pool_info.pool_address = *pool_address;
                            all_vault_pubkeys.push(pool_info.token_vault_0);
                            all_vault_pubkeys.push(pool_info.token_vault_1);
                            all_vault_pubkeys.push(pool_info.token_mint_0);
                            all_vault_pubkeys.push(pool_info.token_mint_1);
                            decoded_pools.push((pool_info, *pool_address));
                        }
                        Err(e) => {
                            warn!("Failed to decode pool data for {}: {}", pool_address, e);
                        }
                    }
                } else {
                    warn!("No account data found for pool {}", pool_address);
                }
            }

            // 3. Batch fetch all vault and mint accounts for this chunk
            let mut vault_accounts_map = std::collections::HashMap::new();
            for vault_chunk in all_vault_pubkeys.chunks(100) {
                match self.rpc_manager.get_multiple_accounts(vault_chunk).await {
                    Ok(accounts) => {
                        for (vault_pubkey, account_opt) in vault_chunk.iter().zip(accounts.iter()) {
                            if let Some(account) = account_opt {
                                vault_accounts_map.insert(*vault_pubkey, account.clone());
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Failed to fetch vault/mint accounts batch: {}", e);
                    }
                }
            }

            // 4. Fill reserves and decimals for all decoded pools using the batch-fetched accounts
            for (mut pool_info, pool_address) in decoded_pools {
                // Reserves
                let vault_0 = vault_accounts_map.get(&pool_info.token_vault_0);
                let vault_1 = vault_accounts_map.get(&pool_info.token_vault_1);
                if let (Some(v0), Some(v1)) = (vault_0, vault_1) {
                    match
                        (
                            self.parse_token_account_balance(&v0.data),
                            self.parse_token_account_balance(&v1.data),
                        )
                    {
                        (Ok(r0), Ok(r1)) => {
                            pool_info.reserve_0 = r0;
                            pool_info.reserve_1 = r1;
                        }
                        (Err(e0), Err(e1)) =>
                            warn!("Failed to parse reserves for {}: {}, {}", pool_address, e0, e1),
                        (Err(e), _) | (_, Err(e)) =>
                            warn!("Failed to parse reserve for {}: {}", pool_address, e),
                    }
                } else {
                    warn!("Missing vault account(s) for pool {}", pool_address);
                }
                // Decimals
                let mint_0 = vault_accounts_map.get(&pool_info.token_mint_0);
                let mint_1 = vault_accounts_map.get(&pool_info.token_mint_1);
                if let (Some(m0), Some(m1)) = (mint_0, mint_1) {
                    match (self.parse_mint_decimals(&m0.data), self.parse_mint_decimals(&m1.data)) {
                        (Ok(d0), Ok(d1)) => {
                            pool_info.decimals_0 = d0;
                            pool_info.decimals_1 = d1;
                        }
                        (Err(e0), Err(e1)) =>
                            warn!("Failed to parse decimals for {}: {}, {}", pool_address, e0, e1),
                        (Err(e), _) | (_, Err(e)) =>
                            warn!("Failed to parse decimals for {}: {}", pool_address, e),
                    }
                } else {
                    warn!("Missing mint account(s) for pool {}", pool_address);
                }
                all_pool_infos.push(pool_info);
            }

            // Rate limiting between batches
            time::sleep(self.rate_limit_delay).await;
        }

        debug!(
            "Successfully fetched {} pools out of {} requested",
            all_pool_infos.len(),
            pool_addresses.len()
        );

        Ok(all_pool_infos)
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

    /// Fetch pools for a token from DEX Screener API
    pub async fn fetch_pools_from_dexscreener(
        &self,
        token_mint: &Pubkey
    ) -> Result<Vec<TokenPair>> {
        let url = format!("https://api.dexscreener.com/tokens/v1/solana/{}", token_mint);

        debug!("Fetching pools from DEX Screener: {}", url);

        // Rate limiting
        time::sleep(self.rate_limit_delay).await;

        let response = self.http_client
            .get(&url)
            .timeout(Duration::from_secs(30))
            .send().await
            .context("Failed to send request to DEX Screener API")?;

        if !response.status().is_success() {
            return Err(
                anyhow::anyhow!(
                    "DEX Screener API returned error status: {} - {}",
                    response.status(),
                    response.text().await.unwrap_or_default()
                )
            );
        }

        let response_text = response
            .text().await
            .context("Failed to read response body from DEX Screener API")?;

        debug!("DEX Screener response length: {} bytes", response_text.len());

        // DEX Screener returns either a direct array of pairs or an object with pairs array
        let pairs: Vec<TokenPair> = if response_text.trim_start().starts_with('[') {
            // Direct array format
            serde_json
                ::from_str(&response_text)
                .context("Failed to parse DEX Screener response as array")?
        } else {
            // Object format with pairs array
            let wrapper: DexScreenerResponse = serde_json
                ::from_str(&response_text)
                .context("Failed to parse DEX Screener response as object")?;
            wrapper.pairs.unwrap_or_default()
        };

        debug!("Found {} pairs for token {} from DEX Screener", pairs.len(), token_mint);

        // Filter to only include Solana pairs and supported DEXes
        let filtered_pairs: Vec<TokenPair> = pairs
            .into_iter()
            .filter(|pair| {
                pair.chain_id == "solana" &&
                    matches!(pair.dex_id.as_str(), "raydium" | "orca" | "meteora" | "pump")
            })
            .collect();

        debug!("Filtered to {} supported DEX pairs", filtered_pairs.len());
        Ok(filtered_pairs)
    }

    /// Find pools by multiple token mints in batch
    pub async fn find_pools_by_tokens(&self, token_mints: &[Pubkey]) -> Result<Vec<PoolInfo>> {
        let mut all_pools = Vec::new();

        for token_mint in token_mints {
            match pool_utils::find_pools_by_token(self, token_mint, &[]).await {
                Ok(mut pools) => {
                    all_pools.append(&mut pools);
                }
                Err(e) => {
                    warn!("Failed to find pools for token {}: {}", token_mint, e);
                }
            }

            // Rate limiting between token requests
            time::sleep(self.rate_limit_delay).await;
        }

        Ok(all_pools)
    }

    /// Get pool data with DEX Screener metadata
    pub async fn get_pool_with_metadata(
        &self,
        pool_address: &Pubkey
    ) -> Result<(PoolInfo, Option<TokenPair>)> {
        // First fetch the pool data
        let pool_info = self.fetch_pool_data(pool_address).await?;

        // Try to find corresponding DEX Screener data for additional metadata
        let token_pairs_0 = self
            .fetch_pools_from_dexscreener(&pool_info.token_mint_0).await
            .unwrap_or_default();
        let token_pairs_1 = self
            .fetch_pools_from_dexscreener(&pool_info.token_mint_1).await
            .unwrap_or_default();

        // Find matching pair by address
        let pool_address_str = pool_address.to_string();
        let metadata = token_pairs_0
            .into_iter()
            .chain(token_pairs_1.into_iter())
            .find(|pair| pair.pair_address == pool_address_str);

        Ok((pool_info, metadata))
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

    /// Find pools by token mint using DEX Screener API
    pub async fn find_pools_by_token(
        fetcher: &PoolDataFetcher,
        token_mint: &Pubkey,
        _program_ids: &[Pubkey]
    ) -> Result<Vec<PoolInfo>> {
        debug!("Finding pools for token: {}", token_mint);

        // Fetch pools from DEX Screener API
        let token_pairs = fetcher
            .fetch_pools_from_dexscreener(token_mint).await
            .context("Failed to fetch pools from DEX Screener")?;

        let mut pool_infos = Vec::new();

        // Convert TokenPair data to PoolInfo by fetching actual pool data
        for pair in token_pairs {
            // Parse the pair address
            match pair.pair_address.parse::<Pubkey>() {
                Ok(pool_address) => {
                    match fetcher.fetch_pool_data(&pool_address).await {
                        Ok(pool_info) => {
                            debug!(
                                "Successfully fetched pool data for {}: {} ({})",
                                pool_address,
                                pair.dex_id,
                                pair.labels
                                    .as_ref()
                                    .map(|l| l.join(","))
                                    .unwrap_or_default()
                            );
                            pool_infos.push(pool_info);
                        }
                        Err(e) => {
                            warn!(
                                "Failed to fetch pool data for {} ({}): {}",
                                pool_address,
                                pair.dex_id,
                                e
                            );
                            // Continue processing other pools
                        }
                    }
                }
                Err(e) => {
                    warn!("Invalid pool address format '{}': {}", pair.pair_address, e);
                }
            }
        }

        debug!("Found {} valid pools for token {}", pool_infos.len(), token_mint);
        Ok(pool_infos)
    }

    /// Find pools by token with filtering options
    pub async fn find_pools_by_token_filtered(
        fetcher: &PoolDataFetcher,
        token_mint: &Pubkey,
        min_liquidity_usd: Option<f64>,
        min_volume_24h: Option<f64>,
        allowed_dexes: Option<&[&str]>
    ) -> Result<Vec<PoolInfo>> {
        debug!("Finding filtered pools for token: {}", token_mint);

        // Fetch pools from DEX Screener API
        let token_pairs = fetcher
            .fetch_pools_from_dexscreener(token_mint).await
            .context("Failed to fetch pools from DEX Screener")?;

        let mut filtered_pairs = token_pairs;

        // Apply filters
        if let Some(min_liq) = min_liquidity_usd {
            filtered_pairs.retain(|pair|
                pair.liquidity.as_ref().map_or(false, |l| l.usd >= min_liq)
            );
            debug!("After liquidity filter (>= ${:.2}): {} pairs", min_liq, filtered_pairs.len());
        }

        if let Some(min_vol) = min_volume_24h {
            filtered_pairs.retain(|pair| pair.volume.h24 >= min_vol);
            debug!("After volume filter (>= ${:.2}): {} pairs", min_vol, filtered_pairs.len());
        }

        if let Some(dexes) = allowed_dexes {
            filtered_pairs.retain(|pair| dexes.contains(&pair.dex_id.as_str()));
            debug!("After DEX filter ({:?}): {} pairs", dexes, filtered_pairs.len());
        }

        let mut pool_infos = Vec::new();

        // Convert filtered TokenPair data to PoolInfo
        for pair in filtered_pairs {
            match pair.pair_address.parse::<Pubkey>() {
                Ok(pool_address) => {
                    match fetcher.fetch_pool_data(&pool_address).await {
                        Ok(pool_info) => {
                            debug!(
                                "Successfully fetched filtered pool data for {}: {} ({})",
                                pool_address,
                                pair.dex_id,
                                pair.labels
                                    .as_ref()
                                    .map(|l| l.join(","))
                                    .unwrap_or_default()
                            );
                            pool_infos.push(pool_info);
                        }
                        Err(e) => {
                            warn!(
                                "Failed to fetch pool data for {} ({}): {}",
                                pool_address,
                                pair.dex_id,
                                e
                            );
                        }
                    }
                }
                Err(e) => {
                    warn!("Invalid pool address format '{}': {}", pair.pair_address, e);
                }
            }
        }

        debug!("Found {} valid filtered pools for token {}", pool_infos.len(), token_mint);
        Ok(pool_infos)
    }

    /// Get the most liquid pools for a token
    pub async fn get_top_liquid_pools(
        fetcher: &PoolDataFetcher,
        token_mint: &Pubkey,
        limit: usize
    ) -> Result<Vec<PoolInfo>> {
        let token_pairs = fetcher
            .fetch_pools_from_dexscreener(token_mint).await
            .context("Failed to fetch pools from DEX Screener")?;

        // Sort by liquidity descending
        let mut sorted_pairs = token_pairs;
        sorted_pairs.sort_by(|a, b| {
            let a_liq = a.liquidity.as_ref().map_or(0.0, |l| l.usd);
            let b_liq = b.liquidity.as_ref().map_or(0.0, |l| l.usd);
            b_liq.partial_cmp(&a_liq).unwrap_or(std::cmp::Ordering::Equal)
        });

        // Take top N pairs
        sorted_pairs.truncate(limit);

        let mut pool_infos = Vec::new();
        for pair in sorted_pairs {
            match pair.pair_address.parse::<Pubkey>() {
                Ok(pool_address) => {
                    match fetcher.fetch_pool_data(&pool_address).await {
                        Ok(pool_info) => {
                            pool_infos.push(pool_info);
                        }
                        Err(e) => {
                            warn!("Failed to fetch pool data for {}: {}", pool_address, e);
                        }
                    }
                }
                Err(e) => {
                    warn!("Invalid pool address format '{}': {}", pair.pair_address, e);
                }
            }
        }

        Ok(pool_infos)
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
