use std::collections::HashMap;
use std::time::{ SystemTime, UNIX_EPOCH };
use reqwest::Client;
use serde::Deserialize;
use crate::pricing::{ TokenInfo, TokenPrice, PoolInfo, PoolType, PriceSource };

const GECKOTERMINAL_BASE_URL: &str = "https://api.geckoterminal.com/api/v2";
const SOLANA_NETWORK: &str = "solana";

#[derive(Debug, Clone)]
pub struct GeckoTerminalClient {
    client: Client,
    base_url: String,
}

#[derive(Debug, Deserialize)]
struct GeckoResponse {
    data: Vec<GeckoTokenData>,
    included: Option<Vec<GeckoIncluded>>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct GeckoTokenData {
    id: String,
    r#type: String,
    attributes: GeckoTokenAttributes,
    relationships: Option<GeckoRelationships>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct GeckoTokenAttributes {
    address: String,
    name: String,
    symbol: String,
    decimals: u8,
    total_supply: Option<String>,
    coingecko_coin_id: Option<String>,
    price_usd: Option<String>,
    market_cap_usd: Option<String>,
    fdv_usd: Option<String>,
    total_reserve_in_usd: Option<String>,
    volume_usd: GeckoVolumeData,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
struct GeckoVolumeData {
    h24: Option<String>,
    h6: Option<String>,
    h1: Option<String>,
    m5: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GeckoRelationships {
    top_pools: Option<GeckoTopPools>,
}

#[derive(Debug, Deserialize)]
struct GeckoTopPools {
    data: Vec<GeckoPoolRef>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct GeckoPoolRef {
    id: String,
    r#type: String,
}

#[derive(Debug, Deserialize)]
struct GeckoIncluded {
    id: String,
    r#type: String,
    attributes: GeckoPoolAttributes,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
struct GeckoPoolAttributes {
    address: String,
    name: String,
    pool_created_at: Option<String>,
    token_price_usd: Option<String>,
    base_token_price_usd: Option<String>,
    quote_token_price_usd: Option<String>,
    base_token_price_native_currency: Option<String>,
    quote_token_price_native_currency: Option<String>,
    reserve_in_usd: Option<String>,
    volume_usd: GeckoVolumeData,
    market_cap_usd: Option<String>,
    fdv_usd: Option<String>,
}

impl GeckoTerminalClient {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            base_url: GECKOTERMINAL_BASE_URL.to_string(),
        }
    }

    pub async fn get_multiple_tokens(
        &self,
        addresses: &[String]
    ) -> Result<Vec<TokenInfo>, Box<dyn std::error::Error + Send + Sync>> {
        if addresses.is_empty() {
            return Ok(Vec::new());
        }

        let addresses_str = addresses.join(",");
        let url = format!(
            "{}/networks/{}/tokens/multi/{}?include=top_pools",
            self.base_url,
            SOLANA_NETWORK,
            addresses_str
        );

        let response = self.client
            .get(&url)
            .header("Accept", "application/json")
            .header("User-Agent", "ScreenerBot/1.0")
            .send().await?;

        if !response.status().is_success() {
            return Err(format!("GeckoTerminal API error: {}", response.status()).into());
        }

        let gecko_response: GeckoResponse = response.json().await?;

        // Create a map of pool data for easy lookup
        let mut pools_map: HashMap<String, GeckoPoolAttributes> = HashMap::new();
        if let Some(included) = &gecko_response.included {
            for item in included {
                if item.r#type == "pool" {
                    pools_map.insert(item.id.clone(), item.attributes.clone());
                }
            }
        }

        let mut token_infos = Vec::new();

        for token_data in gecko_response.data {
            let token_info = self.parse_token_data(token_data, &pools_map)?;
            token_infos.push(token_info);
        }

        Ok(token_infos)
    }

    pub async fn get_token_info(
        &self,
        address: &str
    ) -> Result<TokenInfo, Box<dyn std::error::Error + Send + Sync>> {
        let mut tokens = self.get_multiple_tokens(&[address.to_string()]).await?;

        if tokens.is_empty() {
            return Err(format!("Token {} not found", address).into());
        }

        Ok(tokens.remove(0))
    }

    fn parse_token_data(
        &self,
        token_data: GeckoTokenData,
        pools_map: &HashMap<String, GeckoPoolAttributes>
    ) -> Result<TokenInfo, Box<dyn std::error::Error + Send + Sync>> {
        let attrs = token_data.attributes;

        // Parse token price
        let price = if let Some(price_str) = attrs.price_usd {
            Some(TokenPrice {
                address: attrs.address.clone(),
                price_usd: price_str.parse().unwrap_or(0.0),
                price_sol: None,
                market_cap: attrs.market_cap_usd.and_then(|s| s.parse().ok()),
                volume_24h: attrs.volume_usd.h24.and_then(|s| s.parse().ok()).unwrap_or(0.0),
                liquidity_usd: attrs.total_reserve_in_usd
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0.0),
                timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
                source: PriceSource::GeckoTerminal,
                is_cache: false,
            })
        } else {
            None
        };

        // Parse pools
        let mut pools = Vec::new();
        if let Some(relationships) = token_data.relationships {
            if let Some(top_pools) = relationships.top_pools {
                for pool_ref in top_pools.data {
                    if let Some(pool_attrs) = pools_map.get(&pool_ref.id) {
                        let pool_info = self.parse_pool_data(&pool_ref.id, pool_attrs)?;
                        pools.push(pool_info);
                    }
                }
            }
        }

        // Parse total supply
        let total_supply = attrs.total_supply.and_then(|s| s.parse().ok());

        Ok(TokenInfo {
            address: attrs.address,
            name: attrs.name,
            symbol: attrs.symbol,
            decimals: attrs.decimals,
            total_supply,
            pools,
            price,
            last_updated: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
        })
    }

    fn parse_pool_data(
        &self,
        _pool_id: &str,
        pool_attrs: &GeckoPoolAttributes
    ) -> Result<PoolInfo, Box<dyn std::error::Error + Send + Sync>> {
        // Determine pool type from address or name
        let pool_type = self.determine_pool_type(&pool_attrs.address, &pool_attrs.name);

        // Parse liquidity and volume
        let liquidity_usd = pool_attrs.reserve_in_usd
            .as_ref()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0.0);

        let volume_24h = pool_attrs.volume_usd.h24
            .as_ref()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0.0);

        Ok(PoolInfo {
            address: pool_attrs.address.clone(),
            pool_type,
            reserve_0: 0, // TODO: Get actual reserves from pool data
            reserve_1: 0, // TODO: Get actual reserves from pool data
            token_0: String::new(), // TODO: Extract from pool data
            token_1: String::new(), // TODO: Extract from pool data
            liquidity_usd,
            volume_24h,
            fee_tier: None, // TODO: Extract fee tier if available
            last_updated: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
        })
    }

    fn determine_pool_type(&self, address: &str, name: &str) -> PoolType {
        let name_lower = name.to_lowercase();
        let address_lower = address.to_lowercase();

        if name_lower.contains("raydium") || address_lower.contains("ray") {
            PoolType::Raydium
        } else if name_lower.contains("pump") || name_lower.contains("pumpfun") {
            PoolType::PumpFun
        } else if name_lower.contains("meteora") {
            PoolType::Meteora
        } else if name_lower.contains("orca") {
            PoolType::Orca
        } else if name_lower.contains("serum") {
            PoolType::Serum
        } else {
            PoolType::Unknown(name.to_string())
        }
    }
}
