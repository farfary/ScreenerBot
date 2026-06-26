//! GeckoTerminal API endpoint methods

use super::types::{
    GeckoTerminalDexesResponse, GeckoTerminalPool, GeckoTerminalRecentlyUpdatedResponse,
    GeckoTerminalResponse, GeckoTerminalTokenInfoResponse, GeckoTerminalTokensMultiResponse,
    GeckoTerminalTradesResponse,
};
use super::{GeckoTerminalClient, OhlcvResponse, TokenInfo};
use super::{DEFAULT_NETWORK, MAX_TRENDING_PAGE};
use crate::logger::{self, LogTag};
use serde::Deserialize;

// ============================================================================
// OHLCV Deserialization Types (private to this module)
// ============================================================================

#[derive(Debug, Deserialize)]
struct OhlcvResponseRaw {
    data: OhlcvData,
    meta: OhlcvMeta,
}

#[derive(Debug, Deserialize)]
struct OhlcvData {
    attributes: OhlcvAttributes,
}

#[derive(Debug, Deserialize)]
struct OhlcvAttributes {
    ohlcv_list: Vec<[f64; 6]>,
}

#[derive(Debug, Deserialize)]
struct OhlcvMeta {
    base: TokenInfo,
    quote: TokenInfo,
}

impl GeckoTerminalClient {
    /// Fetch all pools for a single token address
    pub async fn fetch_pools(&self, mint: &str) -> Result<Vec<GeckoTerminalPool>, String> {
        self.fetch_pools_on_network(mint, None).await
    }

    /// Fetch pools for a token on a specific network
    pub async fn fetch_pools_on_network(
        &self,
        mint: &str,
        network: Option<&str>,
    ) -> Result<Vec<GeckoTerminalPool>, String> {
        let network_id = network.unwrap_or(DEFAULT_NETWORK);
        let endpoint = format!("networks/{network_id}/tokens/{mint}/pools");
        let url = format!("{}/{endpoint}", self.base_url);

        logger::debug(
            LogTag::Api,
            &format!(
                "[GECKOTERMINAL] Fetching pools: token={}, network={}",
                mint, network_id
            ),
        );

        let api_response: GeckoTerminalResponse =
            self.get_json(&endpoint, self.client.get(&url)).await?;

        Ok(api_response
            .data
            .into_iter()
            .map(|p| p.to_pool(mint))
            .collect())
    }

    /// Get top pools by token address with optional sorting/filtering
    pub async fn fetch_top_pools_by_token(
        &self,
        token_address: &str,
        network: &str,
        include: Option<&str>,
        page: Option<u32>,
        sort: Option<&str>,
    ) -> Result<Vec<GeckoTerminalPool>, String> {
        let endpoint = format!("networks/{network}/tokens/{token_address}/pools");
        let url = format!("{}/{endpoint}", self.base_url);

        let mut query_params: Vec<(String, String)> = Vec::new();
        if let Some(inc) = include {
            query_params.push(("include".to_owned(), inc.to_string()));
        }
        if let Some(p) = page {
            query_params.push(("page".to_owned(), p.to_string()));
        }
        if let Some(s) = sort {
            query_params.push(("sort".to_owned(), s.to_string()));
        }

        let builder = if query_params.is_empty() {
            self.client.get(&url)
        } else {
            self.client.get(&url).query(&query_params)
        };

        logger::debug(
            LogTag::Api,
            &format!(
                "[GECKOTERMINAL] Fetching top pools by token: token={}, network={}, page={:?}, sort={:?}",
                token_address, network, page, sort
            ),
        );

        let api_response: GeckoTerminalResponse = self.get_json(&endpoint, builder).await?;

        Ok(api_response
            .data
            .into_iter()
            .map(|p| p.to_pool(token_address))
            .collect())
    }

    /// Get trending pools by network
    pub async fn fetch_trending_pools_by_network(
        &self,
        network: Option<&str>,
        page: Option<u32>,
        duration: Option<&str>,
        include: Option<Vec<&str>>,
    ) -> Result<Vec<GeckoTerminalPool>, String> {
        let network_id = network.unwrap_or(DEFAULT_NETWORK);
        let endpoint = format!("networks/{network_id}/trending_pools");
        let url = format!("{}/{endpoint}", self.base_url);

        let mut query_params: Vec<(String, String)> = Vec::new();
        if let Some(p) = page {
            query_params.push(("page".to_owned(), p.min(MAX_TRENDING_PAGE).to_string()));
        }
        if let Some(d) = duration {
            query_params.push(("duration".to_owned(), d.to_string()));
        }
        if let Some(includes) = include {
            if !includes.is_empty() {
                query_params.push(("include".to_owned(), includes.join(",")));
            }
        }

        let builder = if query_params.is_empty() {
            self.client.get(&url)
        } else {
            self.client.get(&url).query(&query_params)
        };

        logger::debug(
            LogTag::Api,
            &format!(
                "[GECKOTERMINAL] Fetching trending pools: network={}, page={:?}, duration={:?}",
                network_id, page, duration
            ),
        );

        let api_response: GeckoTerminalResponse = self.get_json(&endpoint, builder).await?;

        Ok(api_response
            .data
            .into_iter()
            .map(|p| p.to_pool("trending"))
            .collect())
    }

    /// Get top pools by network
    pub async fn fetch_top_pools_by_network(
        &self,
        network: Option<&str>,
        include: Option<Vec<&str>>,
        page: Option<u32>,
        sort: Option<&str>,
    ) -> Result<Vec<GeckoTerminalPool>, String> {
        let network_id = network.unwrap_or(DEFAULT_NETWORK);
        let endpoint = format!("networks/{network_id}/pools");
        let url = format!("{}/{endpoint}", self.base_url);

        let mut query_params: Vec<(String, String)> = Vec::new();
        if let Some(p) = page {
            let page_num = p.clamp(1, 10);
            query_params.push(("page".to_owned(), page_num.to_string()));
        }
        if let Some(s) = sort {
            query_params.push(("sort".to_owned(), s.to_string()));
        }
        if let Some(includes) = include {
            if !includes.is_empty() {
                query_params.push(("include".to_owned(), includes.join(",")));
            }
        }

        let builder = if query_params.is_empty() {
            self.client.get(&url)
        } else {
            self.client.get(&url).query(&query_params)
        };

        logger::debug(
            LogTag::Api,
            &format!(
                "[GECKOTERMINAL] Fetching top pools: network={}, page={:?}, sort={:?}",
                network_id, page, sort
            ),
        );

        let api_response: GeckoTerminalResponse = self.get_json(&endpoint, builder).await?;

        Ok(api_response
            .data
            .into_iter()
            .map(|p| p.to_pool("top_pools"))
            .collect())
    }

    /// Get specific pool data by address
    pub async fn fetch_pool_by_address(
        &self,
        network: Option<&str>,
        pool_address: &str,
        include: Option<Vec<&str>>,
        include_volume_breakdown: bool,
        include_composition: bool,
    ) -> Result<GeckoTerminalPool, String> {
        let network_id = network.unwrap_or(DEFAULT_NETWORK);
        let endpoint = format!("networks/{network_id}/pools/{pool_address}");
        let url = format!("{}/{endpoint}", self.base_url);

        let mut query_params: Vec<(String, String)> = Vec::new();
        if let Some(includes) = include {
            if !includes.is_empty() {
                query_params.push(("include".to_owned(), includes.join(",")));
            }
        }
        if include_volume_breakdown {
            query_params.push(("include_volume_breakdown".to_owned(), "true".to_owned()));
        }
        if include_composition {
            query_params.push(("include_composition".to_owned(), "true".to_owned()));
        }

        let builder = if query_params.is_empty() {
            self.client.get(&url)
        } else {
            self.client.get(&url).query(&query_params)
        };

        logger::debug(
            LogTag::Api,
            &format!(
                "[GECKOTERMINAL] Fetching pool: network={}, address={}",
                network_id, pool_address
            ),
        );

        let api_response: GeckoTerminalResponse = self.get_json(&endpoint, builder).await?;

        api_response
            .data
            .into_iter()
            .next()
            .map(|p| p.to_pool(pool_address))
            .ok_or_else(|| "No pool data returned".to_owned())
    }

    /// Fetch multiple pools in one call (max 30 pool addresses)
    pub async fn fetch_pools_multi(
        &self,
        network: Option<&str>,
        addresses: Vec<&str>,
        include: Option<Vec<&str>>,
        include_volume_breakdown: bool,
        include_composition: bool,
    ) -> Result<Vec<GeckoTerminalPool>, String> {
        if addresses.is_empty() {
            return Err("At least one address is required".to_owned());
        }
        if addresses.len() > 30 {
            return Err("Maximum 30 addresses allowed".to_owned());
        }

        let network_id = network.unwrap_or(DEFAULT_NETWORK);
        let address_count = addresses.len();
        let addresses_str = addresses.join(",");
        let endpoint = format!("networks/{network_id}/pools/multi/{addresses_str}");
        let url = format!("{}/{endpoint}", self.base_url);

        let mut query_params: Vec<(String, String)> = Vec::new();
        if let Some(includes) = include {
            if !includes.is_empty() {
                query_params.push(("include".to_owned(), includes.join(",")));
            }
        }
        if include_volume_breakdown {
            query_params.push(("include_volume_breakdown".to_owned(), "true".to_owned()));
        }
        if include_composition {
            query_params.push(("include_composition".to_owned(), "true".to_owned()));
        }

        let builder = if query_params.is_empty() {
            self.client.get(&url)
        } else {
            self.client.get(&url).query(&query_params)
        };

        logger::debug(
            LogTag::Api,
            &format!(
                "[GECKOTERMINAL] Fetching multi pools: network={}, count={}",
                network_id, address_count
            ),
        );

        let api_response: GeckoTerminalResponse = self.get_json(&endpoint, builder).await?;

        Ok(api_response
            .data
            .into_iter()
            .map(|p| p.to_pool("multi"))
            .collect())
    }

    /// Fetch OHLCV candlestick data for a pool
    pub async fn fetch_ohlcv(
        &self,
        network: &str,
        pool_address: &str,
        timeframe: &str,
        aggregate: Option<u32>,
        limit: Option<u32>,
        currency: Option<&str>,
        before_timestamp: Option<i64>,
        token: Option<&str>,
    ) -> Result<OhlcvResponse, String> {
        let endpoint = format!(
            "networks/{}/pools/{}/ohlcv/{}",
            network, pool_address, timeframe
        );
        let url = format!("{}/{endpoint}", self.base_url);

        let mut query_params: Vec<(String, String)> = Vec::new();
        if let Some(agg) = aggregate {
            query_params.push(("aggregate".to_owned(), agg.to_string()));
        }
        if let Some(lim) = limit {
            query_params.push(("limit".to_owned(), lim.min(1000).to_string()));
        }
        if let Some(curr) = currency {
            query_params.push(("currency".to_owned(), curr.to_string()));
        }
        if let Some(ts) = before_timestamp {
            query_params.push(("before_timestamp".to_owned(), ts.to_string()));
        }
        if let Some(tok) = token {
            query_params.push(("token".to_owned(), tok.to_string()));
        }

        let builder = if query_params.is_empty() {
            self.client.get(&url)
        } else {
            self.client.get(&url).query(&query_params)
        };

        logger::debug(
            LogTag::Api,
            &format!(
                "[GECKOTERMINAL] Fetching OHLCV: network={}, pool={}, timeframe={}, aggregate={:?}, limit={:?}",
                network, pool_address, timeframe, aggregate, limit
            ),
        );

        let ohlcv_response: OhlcvResponseRaw = self.get_json(&endpoint, builder).await?;

        Ok(OhlcvResponse {
            ohlcv_list: ohlcv_response.data.attributes.ohlcv_list,
            base_token: ohlcv_response.meta.base,
            quote_token: ohlcv_response.meta.quote,
        })
    }

    /// Get supported DEX list for a network
    pub async fn fetch_dexes_by_network(
        &self,
        network: &str,
        page: Option<u32>,
    ) -> Result<Vec<(String, String)>, String> {
        let endpoint = format!("networks/{network}/dexes");
        let url = format!("{}/{endpoint}", self.base_url);

        let builder = if let Some(p) = page {
            self.client
                .get(&url)
                .query(&[("page".to_owned(), p.to_string())])
        } else {
            self.client.get(&url)
        };

        logger::debug(
            LogTag::Api,
            &format!(
                "[GECKOTERMINAL] Fetching DEXes: network={}, page={:?}",
                network, page
            ),
        );

        let dex_response: GeckoTerminalDexesResponse = self.get_json(&endpoint, builder).await?;

        Ok(dex_response
            .data
            .into_iter()
            .map(|d| (d.id, d.attributes.name))
            .collect())
    }

    /// Fetch latest newly created pools on a network
    pub async fn fetch_new_pools_by_network(
        &self,
        network: &str,
        include: Option<&str>,
        page: Option<u32>,
    ) -> Result<Vec<GeckoTerminalPool>, String> {
        let endpoint = format!("networks/{network}/new_pools");
        let url = format!("{}/{endpoint}", self.base_url);

        let mut query_params: Vec<(String, String)> = Vec::new();
        if let Some(inc) = include {
            query_params.push(("include".to_owned(), inc.to_string()));
        }
        if let Some(p) = page {
            query_params.push(("page".to_owned(), p.to_string()));
        }

        let builder = if query_params.is_empty() {
            self.client.get(&url)
        } else {
            self.client.get(&url).query(&query_params)
        };

        logger::debug(
            LogTag::Api,
            &format!(
                "[GECKOTERMINAL] Fetching new pools: network={}, page={:?}",
                network, page
            ),
        );

        let api_response: GeckoTerminalResponse = self.get_json(&endpoint, builder).await?;

        Ok(api_response
            .data
            .into_iter()
            .map(|p| p.to_pool("new_pools"))
            .collect())
    }

    /// Fetch multiple token metadata entries
    pub async fn fetch_tokens_multi(
        &self,
        network: &str,
        addresses: &str,
        include: Option<&str>,
        include_composition: Option<bool>,
    ) -> Result<GeckoTerminalTokensMultiResponse, String> {
        let endpoint = format!("networks/{network}/tokens/multi/{addresses}");
        let url = format!("{}/{endpoint}", self.base_url);

        let mut query_params: Vec<(String, String)> = Vec::new();
        if let Some(inc) = include {
            query_params.push(("include".to_owned(), inc.to_string()));
        }
        if let Some(comp) = include_composition {
            query_params.push(("include_composition".to_owned(), comp.to_string()));
        }

        let builder = if query_params.is_empty() {
            self.client.get(&url)
        } else {
            self.client.get(&url).query(&query_params)
        };

        logger::debug(
            LogTag::Api,
            &format!(
                "[GECKOTERMINAL] Fetching tokens multi: network={}, addresses_count={}",
                network,
                addresses.split(',').count()
            ),
        );

        self.get_json(&endpoint, builder).await
    }

    /// Fetch token metadata for a single address
    pub async fn fetch_token_info(
        &self,
        network: &str,
        address: &str,
    ) -> Result<GeckoTerminalTokenInfoResponse, String> {
        let endpoint = format!("networks/{network}/tokens/{address}/info");
        let url = format!("{}/{endpoint}", self.base_url);

        logger::debug(
            LogTag::Api,
            &format!(
                "[GECKOTERMINAL] Fetching token info: network={}, address={}",
                network, address
            ),
        );

        self.get_json(&endpoint, self.client.get(&url)).await
    }

    /// Fetch recently updated tokens (global endpoint)
    pub async fn fetch_recently_updated_tokens(
        &self,
        include: Option<&str>,
        network: Option<&str>,
    ) -> Result<GeckoTerminalRecentlyUpdatedResponse, String> {
        let endpoint = "tokens/info_recently_updated";
        let url = format!("{}/{endpoint}", self.base_url);

        let mut query_params: Vec<(String, String)> = Vec::new();
        if let Some(inc) = include {
            query_params.push(("include".to_owned(), inc.to_string()));
        }
        if let Some(net) = network {
            query_params.push(("network".to_owned(), net.to_string()));
        }

        let builder = if query_params.is_empty() {
            self.client.get(&url)
        } else {
            self.client.get(&url).query(&query_params)
        };

        logger::debug(
            LogTag::Api,
            &format!(
                "[GECKOTERMINAL] Fetching recently updated tokens: network={:?}",
                network
            ),
        );

        self.get_json(endpoint, builder).await
    }

    /// Fetch trades for a pool in the last 24 hours
    pub async fn fetch_pool_trades(
        &self,
        network: &str,
        pool_address: &str,
        trade_volume_in_usd_greater_than: Option<f64>,
        token: Option<&str>,
    ) -> Result<GeckoTerminalTradesResponse, String> {
        let endpoint = format!("networks/{network}/pools/{pool_address}/trades");
        let url = format!("{}/{endpoint}", self.base_url);

        let mut query_params: Vec<(String, String)> = Vec::new();
        if let Some(min_volume) = trade_volume_in_usd_greater_than {
            query_params.push((
                "trade_volume_in_usd_greater_than".to_owned(),
                min_volume.to_string(),
            ));
        }
        if let Some(tok) = token {
            query_params.push(("token".to_owned(), tok.to_string()));
        }

        let builder = if query_params.is_empty() {
            self.client.get(&url)
        } else {
            self.client.get(&url).query(&query_params)
        };

        logger::debug(
            LogTag::Api,
            &format!(
                "[GECKOTERMINAL] Fetching pool trades: network={}, pool={}, min_volume={:?}",
                network, pool_address, trade_volume_in_usd_greater_than
            ),
        );

        self.get_json(&endpoint, builder).await
    }
}
