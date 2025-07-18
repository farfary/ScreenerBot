use crate::pairs::{ PairsTrait, TokenPair, PairsError, PairsDatabase };
use anyhow::{ Context, Result };
use async_trait::async_trait;
use reqwest::Client;
use std::time::Duration;
use log::{ debug, error, info, warn };

const BASE_URL: &str = "https://api.dexscreener.com";
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);
const DEFAULT_CACHE_HOURS: i64 = 1; // Cache for 1 hour by default

pub struct PairsClient {
    client: Client,
    base_url: String,
    database: PairsDatabase,
    cache_duration_hours: i64,
}

impl PairsClient {
    /// Create a new PairsClient with default settings
    pub fn new() -> Result<Self> {
        let client = Client::builder()
            .timeout(DEFAULT_TIMEOUT)
            .user_agent("ScreenerBot/1.0")
            .build()
            .context("Failed to create HTTP client")?;

        let database = PairsDatabase::new().context("Failed to initialize pairs database")?;

        Ok(Self {
            client,
            base_url: BASE_URL.to_string(),
            database,
            cache_duration_hours: DEFAULT_CACHE_HOURS,
        })
    }

    /// Create a new PairsClient with custom timeout
    pub fn with_timeout(timeout: Duration) -> Result<Self> {
        let client = Client::builder()
            .timeout(timeout)
            .user_agent("ScreenerBot/1.0")
            .build()
            .context("Failed to create HTTP client")?;

        let database = PairsDatabase::new().context("Failed to initialize pairs database")?;

        Ok(Self {
            client,
            base_url: BASE_URL.to_string(),
            database,
            cache_duration_hours: DEFAULT_CACHE_HOURS,
        })
    }

    /// Create a new PairsClient with custom base URL (useful for testing)
    pub fn with_base_url(base_url: String) -> Result<Self> {
        let client = Client::builder()
            .timeout(DEFAULT_TIMEOUT)
            .user_agent("ScreenerBot/1.0")
            .build()
            .context("Failed to create HTTP client")?;

        let database = PairsDatabase::new().context("Failed to initialize pairs database")?;

        Ok(Self {
            client,
            base_url,
            database,
            cache_duration_hours: DEFAULT_CACHE_HOURS,
        })
    }

    /// Get token pairs for Solana by default
    pub async fn get_solana_token_pairs(&self, token_address: &str) -> Result<Vec<TokenPair>> {
        self.get_token_pairs_by_chain("solana", token_address).await
    }

    /// Get all pairs for a specific token across all supported chains
    async fn fetch_token_pairs_internal(
        &self,
        chain_id: &str,
        token_address: &str
    ) -> Result<Vec<TokenPair>> {
        let url = format!("{}/token-pairs/v1/{}/{}", self.base_url, chain_id, token_address);

        debug!("Fetching token pairs from: {}", url);

        let response = self.client
            .get(&url)
            .send().await
            .context("Failed to send request to DexScreener API")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());

            error!("API request failed with status {}: {}", status, error_text);

            return Err(
                anyhow::anyhow!(
                    "DexScreener API request failed with status {}: {}",
                    status,
                    error_text
                )
            );
        }

        let pairs: Vec<TokenPair> = response
            .json().await
            .context("Failed to parse JSON response from DexScreener API")?;

        info!(
            "Successfully fetched {} token pairs for {}/{}",
            pairs.len(),
            chain_id,
            token_address
        );

        Ok(pairs)
    }

    /// Filter pairs by minimum liquidity
    pub fn filter_by_liquidity(
        &self,
        pairs: Vec<TokenPair>,
        min_liquidity_usd: f64
    ) -> Vec<TokenPair> {
        let filtered: Vec<TokenPair> = pairs
            .into_iter()
            .filter(|pair| pair.liquidity.usd >= min_liquidity_usd)
            .collect();

        debug!(
            "Filtered to {} pairs with minimum liquidity ${}",
            filtered.len(),
            min_liquidity_usd
        );
        filtered
    }

    /// Filter pairs by minimum volume
    pub fn filter_by_volume(&self, pairs: Vec<TokenPair>, min_volume_24h: f64) -> Vec<TokenPair> {
        let filtered: Vec<TokenPair> = pairs
            .into_iter()
            .filter(|pair| pair.volume.h24 >= min_volume_24h)
            .collect();

        debug!("Filtered to {} pairs with minimum 24h volume ${}", filtered.len(), min_volume_24h);
        filtered
    }

    /// Filter pairs by DEX
    pub fn filter_by_dex(&self, pairs: Vec<TokenPair>, dex_ids: Vec<&str>) -> Vec<TokenPair> {
        let filtered: Vec<TokenPair> = pairs
            .into_iter()
            .filter(|pair| dex_ids.contains(&pair.dex_id.as_str()))
            .collect();

        debug!("Filtered to {} pairs from specified DEXes", filtered.len());
        filtered
    }

    /// Filter pairs that only have major quote tokens (SOL, USDC, USDT, etc.)
    pub fn filter_major_pairs(&self, pairs: Vec<TokenPair>) -> Vec<TokenPair> {
        let filtered: Vec<TokenPair> = pairs
            .into_iter()
            .filter(|pair| pair.is_major_pair())
            .collect();

        debug!("Filtered to {} major trading pairs", filtered.len());
        filtered
    }

    /// Sort pairs by liquidity (highest first)
    pub fn sort_by_liquidity(&self, mut pairs: Vec<TokenPair>) -> Vec<TokenPair> {
        pairs.sort_by(|a, b|
            b.liquidity.usd.partial_cmp(&a.liquidity.usd).unwrap_or(std::cmp::Ordering::Equal)
        );
        pairs
    }

    /// Sort pairs by volume (highest first)
    pub fn sort_by_volume(&self, mut pairs: Vec<TokenPair>) -> Vec<TokenPair> {
        pairs.sort_by(|a, b|
            b.volume.h24.partial_cmp(&a.volume.h24).unwrap_or(std::cmp::Ordering::Equal)
        );
        pairs
    }

    /// Get the best trading pair based on liquidity and volume
    pub fn get_best_pair(&self, pairs: Vec<TokenPair>) -> Option<TokenPair> {
        if pairs.is_empty() {
            return None;
        }

        // Score pairs based on liquidity and volume
        let mut scored_pairs: Vec<(TokenPair, f64)> = pairs
            .into_iter()
            .map(|pair| {
                let liquidity_score = pair.liquidity.usd;
                let volume_score = pair.volume.h24;
                let total_score = liquidity_score + volume_score;
                (pair, total_score)
            })
            .collect();

        scored_pairs.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        scored_pairs
            .into_iter()
            .next()
            .map(|(pair, _)| pair)
    }

    /// Print summary of pairs for debugging
    pub fn print_pairs_summary(&self, pairs: &[TokenPair]) {
        info!("=== Token Pairs Summary ===");
        info!("Total pairs found: {}", pairs.len());

        for (i, pair) in pairs.iter().enumerate() {
            info!(
                "{}. {} - {}/{} | Liquidity: ${:.2} | Volume 24h: ${:.2} | Price: ${}",
                i + 1,
                pair.dex_id,
                pair.base_token.symbol,
                pair.quote_token.symbol,
                pair.liquidity.usd,
                pair.volume.h24,
                pair.price_usd
            );
        }
    }

    /// Set cache duration in hours
    pub fn set_cache_duration(&mut self, hours: i64) {
        self.cache_duration_hours = hours;
    }

    /// Get cache statistics
    pub fn get_cache_stats(&self) -> Result<crate::pairs::database::CacheStats> {
        self.database.get_cache_stats()
    }

    /// Clean expired cache entries
    pub fn clean_expired_cache(&self) -> Result<usize> {
        self.database.clean_expired_cache()
    }

    /// Get top pairs by liquidity from cache
    pub fn get_top_pairs_by_liquidity(&self, limit: usize) -> Result<Vec<TokenPair>> {
        self.database.get_top_pairs_by_liquidity(limit)
    }

    /// Get pairs by DEX ID from cache
    pub fn get_cached_pairs_by_dex(&self, dex_id: &str) -> Result<Vec<TokenPair>> {
        self.database.get_cached_pairs_by_dex(dex_id)
    }
}

impl Default for PairsClient {
    fn default() -> Self {
        Self::new().expect("Failed to create default PairsClient")
    }
}

#[async_trait]
impl PairsTrait for PairsClient {
    /// Get token pairs for Solana by default
    async fn get_token_pairs(&self, token_address: &str) -> Result<Vec<TokenPair>> {
        self.get_solana_token_pairs(token_address).await
    }

    /// Get token pairs for a specific chain
    async fn get_token_pairs_by_chain(
        &self,
        chain_id: &str,
        token_address: &str
    ) -> Result<Vec<TokenPair>> {
        if token_address.trim().is_empty() {
            return Err(anyhow::anyhow!("Token address cannot be empty"));
        }

        if chain_id.trim().is_empty() {
            return Err(anyhow::anyhow!("Chain ID cannot be empty"));
        }

        self.fetch_token_pairs_internal(chain_id, token_address).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_jupiter_pairs() {
        let client = PairsClient::new().unwrap();
        let pairs = client
            .get_solana_token_pairs("JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN").await
            .unwrap();

        assert!(!pairs.is_empty());
        assert!(pairs.iter().all(|p| p.chain_id == "solana"));
    }

    #[test]
    fn test_filter_by_liquidity() {
        let client = PairsClient::new().unwrap();
        // This would need mock data for a proper test
        let pairs = vec![];
        let filtered = client.filter_by_liquidity(pairs, 100_000.0);
        assert_eq!(filtered.len(), 0);
    }
}
