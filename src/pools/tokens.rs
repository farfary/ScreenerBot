/// Token management for pools module
/// Gets top tokens by liquidity from the tokens database

use crate::tokens::{ get_all_tokens_by_liquidity, ApiToken };

/// Token information for pool operations
#[derive(Debug, Clone)]
pub struct PoolToken {
    pub mint: String,
    pub symbol: String,
    pub name: String,
    pub liquidity_usd: f64,
    pub price_sol: Option<f64>,
    pub price_usd: Option<f64>,
}

impl From<ApiToken> for PoolToken {
    fn from(token: ApiToken) -> Self {
        PoolToken {
            mint: token.mint,
            symbol: token.symbol,
            name: token.name,
            liquidity_usd: token.liquidity
                .as_ref()
                .and_then(|l| l.usd)
                .unwrap_or(0.0),
            price_sol: token.price_sol,
            price_usd: Some(token.price_usd),
        }
    }
}

/// Pool token manager
#[derive(Clone)]
pub struct PoolTokenManager {
    // TODO: Add fields as needed
}

impl PoolTokenManager {
    pub fn new() -> Self {
        Self {
            // TODO: Initialize
        }
    }

    /// Get top 100 tokens by liquidity from the tokens database
    pub async fn get_top_liquidity_tokens(&self) -> Result<Vec<PoolToken>, String> {
        match get_all_tokens_by_liquidity().await {
            Ok(tokens) => {
                // Take top 100 and convert to PoolToken
                let pool_tokens: Vec<PoolToken> = tokens
                    .into_iter()
                    .take(100)
                    .map(PoolToken::from)
                    .collect();

                Ok(pool_tokens)
            }
            Err(e) => Err(format!("Failed to get tokens by liquidity: {}", e)),
        }
    }

    /// Get top N tokens by liquidity (configurable limit)
    pub async fn get_top_n_liquidity_tokens(&self, limit: usize) -> Result<Vec<PoolToken>, String> {
        match get_all_tokens_by_liquidity().await {
            Ok(tokens) => {
                let pool_tokens: Vec<PoolToken> = tokens
                    .into_iter()
                    .take(limit)
                    .map(PoolToken::from)
                    .collect();

                Ok(pool_tokens)
            }
            Err(e) => Err(format!("Failed to get tokens by liquidity: {}", e)),
        }
    }

    /// Get tokens above a specific liquidity threshold
    pub async fn get_tokens_above_liquidity(
        &self,
        min_liquidity: f64
    ) -> Result<Vec<PoolToken>, String> {
        match get_all_tokens_by_liquidity().await {
            Ok(tokens) => {
                let pool_tokens: Vec<PoolToken> = tokens
                    .into_iter()
                    .filter(|token| {
                        token.liquidity
                            .as_ref()
                            .and_then(|l| l.usd)
                            .unwrap_or(0.0) >= min_liquidity
                    })
                    .map(PoolToken::from)
                    .collect();

                Ok(pool_tokens)
            }
            Err(e) => Err(format!("Failed to get tokens by liquidity: {}", e)),
        }
    }

    /// Get token mint addresses for the top liquidity tokens (for pool operations)
    pub async fn get_top_token_mints(&self, limit: usize) -> Result<Vec<String>, String> {
        let tokens = self.get_top_n_liquidity_tokens(limit).await?;
        Ok(
            tokens
                .into_iter()
                .map(|t| t.mint)
                .collect()
        )
    }
}
