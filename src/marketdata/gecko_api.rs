use anyhow::{ Context, Result };
use reqwest::Client;
use serde::{ Deserialize, Serialize };
use crate::marketdata::database::{ TokenData, PoolData };
use chrono::{ DateTime, Utc };

/// GeckoTerminal API response structures
#[derive(Debug, Deserialize)]
pub struct GeckoResponse {
    pub data: Vec<GeckoTokenData>,
    pub included: Option<Vec<GeckoPool>>,
}

#[derive(Debug, Deserialize)]
pub struct GeckoTokenData {
    pub id: String,
    pub attributes: GeckoTokenAttributes,
    pub relationships: Option<GeckoTokenRelationships>,
}

#[derive(Debug, Deserialize)]
pub struct GeckoTokenAttributes {
    pub address: String,
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub total_supply: Option<String>,
    pub price_usd: Option<String>,
    pub fdv_usd: Option<String>,
    pub market_cap_usd: Option<String>,
    pub total_reserve_in_usd: Option<String>,
    pub volume_usd: Option<VolumeData>,
    pub price_change_percentage: Option<PriceChangeData>,
}

#[derive(Debug, Deserialize)]
pub struct VolumeData {
    pub h24: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PriceChangeData {
    pub h24: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct GeckoTokenRelationships {
    pub top_pools: Option<TopPoolsData>,
}

#[derive(Debug, Deserialize)]
pub struct TopPoolsData {
    pub data: Vec<PoolReference>,
}

#[derive(Debug, Deserialize)]
pub struct PoolReference {
    pub id: String,
    #[serde(rename = "type")]
    pub pool_type: String,
}

#[derive(Debug, Deserialize)]
pub struct GeckoPool {
    pub id: String,
    #[serde(rename = "type")]
    pub pool_type: String,
    pub attributes: GeckoPoolAttributes,
}

#[derive(Debug, Deserialize)]
pub struct GeckoPoolAttributes {
    pub address: String,
    pub name: String,
    pub base_token_price_usd: Option<String>,
    pub quote_token_price_usd: Option<String>,
    pub base_token_price_native_currency: Option<String>,
    pub quote_token_price_native_currency: Option<String>,
    pub pool_created_at: Option<String>,
    pub reserve_in_usd: Option<String>,
    pub volume_usd: Option<VolumeData>,
    pub relationships: Option<GeckoPoolRelationships>,
}

#[derive(Debug, Deserialize)]
pub struct GeckoPoolRelationships {
    pub base_token: Option<TokenReference>,
    pub quote_token: Option<TokenReference>,
}

#[derive(Debug, Deserialize)]
pub struct TokenReference {
    pub data: TokenReferenceData,
}

#[derive(Debug, Deserialize)]
pub struct TokenReferenceData {
    pub id: String,
    #[serde(rename = "type")]
    pub token_type: String,
}

/// GeckoTerminal API client
pub struct GeckoTerminalClient {
    client: Client,
    base_url: String,
}

impl GeckoTerminalClient {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            base_url: "https://api.geckoterminal.com/api/v2".to_string(),
        }
    }

    /// Fetch token data for a single token (kept for compatibility, uses batch internally)
    pub async fn fetch_token_data(&self, mint: &str) -> Result<Option<(TokenData, Vec<PoolData>)>> {
        let results = self.fetch_token_data_batch(&[mint.to_string()]).await?;
        Ok(results.into_iter().next())
    }

    /// Fetch token data for a batch of up to 30 tokens
    pub async fn fetch_token_data_batch(
        &self,
        mints: &[String]
    ) -> Result<Vec<(TokenData, Vec<PoolData>)>> {
        if mints.is_empty() {
            return Ok(vec![]);
        }
        if mints.len() > 30 {
            anyhow::bail!("fetch_token_data_batch: cannot fetch more than 30 tokens per call");
        }
        let mints_joined = mints.join(",");
        let url = format!(
            "{}/networks/solana/tokens/multi/{}?include=top_pools",
            self.base_url,
            mints_joined
        );

        let response = self.client
            .get(&url)
            .header("Accept", "application/json")
            .send().await
            .context("Failed to send request to GeckoTerminal")?;

        if !response.status().is_success() {
            return Ok(vec![]);
        }

        let gecko_response: GeckoResponse = response
            .json().await
            .context("Failed to parse GeckoTerminal response")?;

        if gecko_response.data.is_empty() {
            return Ok(vec![]);
        }

        let pools = gecko_response.included
            .as_ref()
            .map(|v| v.as_slice())
            .unwrap_or(&[]);
        let mut results = Vec::new();
        for token_data in &gecko_response.data {
            let token = self.convert_to_token_data(token_data, pools)?;
            let pool_data = self.convert_to_pool_data(pools, &token_data.attributes.address)?;
            results.push((token, pool_data));
        }
        Ok(results)
    }

    /// Convert GeckoTerminal token data to our TokenData structure
    fn convert_to_token_data(
        &self,
        gecko_token: &GeckoTokenData,
        pools: &[GeckoPool]
    ) -> Result<TokenData> {
        let attrs = &gecko_token.attributes;

        // Parse numeric values with fallbacks
        let price_usd = attrs.price_usd
            .as_ref()
            .and_then(|p| p.parse::<f64>().ok())
            .unwrap_or(0.0);

        let price_change_24h = attrs.price_change_percentage
            .as_ref()
            .and_then(|pc| pc.h24.as_ref())
            .and_then(|h24| h24.parse::<f64>().ok())
            .unwrap_or(0.0);

        let volume_24h = attrs.volume_usd
            .as_ref()
            .and_then(|v| v.h24.as_ref())
            .and_then(|h24| h24.parse::<f64>().ok())
            .unwrap_or(0.0);

        let market_cap = attrs.market_cap_usd
            .as_ref()
            .and_then(|mc| mc.parse::<f64>().ok())
            .unwrap_or(0.0);

        let fdv = attrs.fdv_usd
            .as_ref()
            .and_then(|fdv| fdv.parse::<f64>().ok())
            .unwrap_or(0.0);

        let total_supply = attrs.total_supply
            .as_ref()
            .and_then(|ts| ts.parse::<f64>().ok())
            .unwrap_or(0.0);

        let liquidity_usd = attrs.total_reserve_in_usd
            .as_ref()
            .and_then(|tr| tr.parse::<f64>().ok())
            .unwrap_or(0.0);

        // Find the top pool
        let mut top_pool_address = None;
        let mut top_pool_base_reserve = None;
        let mut top_pool_quote_reserve = None;

        if let Some(relationships) = &gecko_token.relationships {
            if let Some(top_pools) = &relationships.top_pools {
                if let Some(pool_ref) = top_pools.data.first() {
                    if let Some(pool) = pools.iter().find(|p| p.id == pool_ref.id) {
                        top_pool_address = Some(pool.attributes.address.clone());
                        // Note: GeckoTerminal doesn't provide raw reserve amounts, only USD values
                        top_pool_base_reserve = pool.attributes.base_token_price_usd
                            .as_ref()
                            .and_then(|p| p.parse::<f64>().ok());
                        top_pool_quote_reserve = pool.attributes.quote_token_price_usd
                            .as_ref()
                            .and_then(|p| p.parse::<f64>().ok());
                    }
                }
            }
        }

        Ok(TokenData {
            mint: attrs.address.clone(),
            symbol: attrs.symbol.clone(),
            name: attrs.name.clone(),
            decimals: attrs.decimals,
            price_usd,
            price_change_24h,
            volume_24h,
            market_cap,
            fdv,
            total_supply,
            circulating_supply: total_supply, // GeckoTerminal doesn't distinguish
            liquidity_usd,
            top_pool_address,
            top_pool_base_reserve,
            top_pool_quote_reserve,
            last_updated: Utc::now(),
        })
    }

    /// Convert GeckoTerminal pool data to our PoolData structure
    fn convert_to_pool_data(&self, pools: &[GeckoPool], token_mint: &str) -> Result<Vec<PoolData>> {
        let mut pool_data = Vec::new();

        for pool in pools {
            let attrs = &pool.attributes;

            let liquidity_usd = attrs.reserve_in_usd
                .as_ref()
                .and_then(|r| r.parse::<f64>().ok())
                .unwrap_or(0.0);

            let volume_24h = attrs.volume_usd
                .as_ref()
                .and_then(|v| v.h24.as_ref())
                .and_then(|h24| h24.parse::<f64>().ok())
                .unwrap_or(0.0);

            let base_token_reserve = attrs.base_token_price_usd
                .as_ref()
                .and_then(|p| p.parse::<f64>().ok())
                .unwrap_or(0.0);

            let quote_token_reserve = attrs.quote_token_price_usd
                .as_ref()
                .and_then(|p| p.parse::<f64>().ok())
                .unwrap_or(0.0);

            let created_at = attrs.pool_created_at
                .as_ref()
                .and_then(|date_str| DateTime::parse_from_rfc3339(date_str).ok())
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|| Utc::now());

            // Extract base and quote token addresses from relationships
            let mut base_token_address = String::new();
            let mut quote_token_address = String::new();

            if let Some(relationships) = &attrs.relationships {
                if let Some(base_token) = &relationships.base_token {
                    base_token_address = base_token.data.id.clone();
                }
                if let Some(quote_token) = &relationships.quote_token {
                    quote_token_address = quote_token.data.id.clone();
                }
            }

            pool_data.push(PoolData {
                pool_address: attrs.address.clone(),
                token_mint: token_mint.to_string(),
                base_token_address,
                quote_token_address,
                base_token_reserve,
                quote_token_reserve,
                liquidity_usd,
                volume_24h,
                created_at,
                last_updated: Utc::now(),
            });
        }

        Ok(pool_data)
    }
}
