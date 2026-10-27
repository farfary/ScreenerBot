// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! GeckoTerminal API endpoint methods.

use super::types::{
    GeckoTerminalPool, GeckoTerminalRecentlyUpdatedResponse, GeckoTerminalResponse,
    GeckoTerminalTokensMultiResponse,
};
use super::{default_network, MAX_TRENDING_PAGE};
use super::{GeckoTerminalClient, OhlcvResponse, RequestLane, TokenInfo};
use crate::apis::Error;
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
    pub async fn fetch_pools(
        &self,
        mint: &str,
        lane: RequestLane,
    ) -> Result<Vec<GeckoTerminalPool>, Error> {
        self.fetch_pools_on_network(mint, None, lane).await
    }

    /// Fetch pools for a token on a specific network
    pub async fn fetch_pools_on_network(
        &self,
        mint: &str,
        network: Option<&str>,
        lane: RequestLane,
    ) -> Result<Vec<GeckoTerminalPool>, Error> {
        let network_id = network.unwrap_or_else(|| default_network());
        let endpoint = format!("networks/{network_id}/tokens/{mint}/pools");
        let url = format!("{}/{endpoint}", self.base_url);

        logger::debug(
            LogTag::Api,
            &format!(
                "[GECKOTERMINAL] Fetching pools: token={}, network={}",
                mint, network_id
            ),
        );

        let api_response: GeckoTerminalResponse = self
            .get_json(lane, &endpoint, self.client.get(&url))
            .await?;

        Ok(api_response
            .data
            .into_iter()
            .map(|p| p.to_pool(mint))
            .collect())
    }

    /// Get trending pools by network
    pub async fn fetch_trending_pools_by_network(
        &self,
        network: Option<&str>,
        page: Option<u32>,
        duration: Option<&str>,
        include: Option<Vec<&str>>,
        lane: RequestLane,
    ) -> Result<Vec<GeckoTerminalPool>, Error> {
        let network_id = network.unwrap_or_else(|| default_network());
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

        let api_response: GeckoTerminalResponse = self.get_json(lane, &endpoint, builder).await?;

        Ok(api_response
            .data
            .into_iter()
            .map(|p| p.to_pool("trending"))
            .collect())
    }

    /// Fetch OHLCV candlestick data of `token` in one of its pools.
    ///
    /// GeckoTerminal prices the pool's base token unless `token` names the side, and its
    /// base side often differs from other providers' (a SOL/USDC pool is SOL-based there),
    /// so the priced token is always named.
    pub async fn fetch_ohlcv(
        &self,
        network: &str,
        pool_address: &str,
        token: &str,
        timeframe: &str,
        aggregate: Option<u32>,
        limit: Option<u32>,
        currency: Option<&str>,
        before_timestamp: Option<i64>,
        lane: RequestLane,
    ) -> Result<OhlcvResponse, Error> {
        let endpoint = format!(
            "networks/{}/pools/{}/ohlcv/{}",
            network, pool_address, timeframe
        );
        let url = format!("{}/{endpoint}", self.base_url);
        let query_params = ohlcv_query(token, aggregate, limit, currency, before_timestamp);
        let builder = self.client.get(&url).query(&query_params);

        logger::debug(
            LogTag::Api,
            &format!(
                "[GECKOTERMINAL] Fetching OHLCV: network={}, pool={}, token={}, timeframe={}, aggregate={:?}, limit={:?}",
                network, pool_address, token, timeframe, aggregate, limit
            ),
        );

        let ohlcv_response: OhlcvResponseRaw = self.get_json(lane, &endpoint, builder).await?;

        Ok(OhlcvResponse {
            ohlcv_list: ohlcv_response.data.attributes.ohlcv_list,
            base_token: ohlcv_response.meta.base,
            quote_token: ohlcv_response.meta.quote,
        })
    }

    /// Fetch latest newly created pools on a network
    pub async fn fetch_new_pools_by_network(
        &self,
        network: &str,
        include: Option<&str>,
        page: Option<u32>,
        lane: RequestLane,
    ) -> Result<Vec<GeckoTerminalPool>, Error> {
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

        let api_response: GeckoTerminalResponse = self.get_json(lane, &endpoint, builder).await?;

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
        lane: RequestLane,
    ) -> Result<GeckoTerminalTokensMultiResponse, Error> {
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

        self.get_json(lane, &endpoint, builder).await
    }

    /// Fetch recently updated tokens (global endpoint)
    pub async fn fetch_recently_updated_tokens(
        &self,
        include: Option<&str>,
        network: Option<&str>,
        lane: RequestLane,
    ) -> Result<GeckoTerminalRecentlyUpdatedResponse, Error> {
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

        self.get_json(lane, endpoint, builder).await
    }
}

/// Query parameters of a pool OHLCV request. `token` is always sent: without it
/// GeckoTerminal answers the pool's base token, whichever side that is.
fn ohlcv_query(
    token: &str,
    aggregate: Option<u32>,
    limit: Option<u32>,
    currency: Option<&str>,
    before_timestamp: Option<i64>,
) -> Vec<(String, String)> {
    let mut query = vec![("token".to_owned(), token.to_owned())];
    if let Some(agg) = aggregate {
        query.push(("aggregate".to_owned(), agg.to_string()));
    }
    if let Some(lim) = limit {
        query.push(("limit".to_owned(), lim.min(1000).to_string()));
    }
    if let Some(curr) = currency {
        query.push(("currency".to_owned(), curr.to_owned()));
    }
    if let Some(ts) = before_timestamp {
        query.push(("before_timestamp".to_owned(), ts.to_string()));
    }
    query
}

#[cfg(test)]
mod tests {
    use super::ohlcv_query;

    const USDC: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";

    fn param<'a>(query: &'a [(String, String)], key: &str) -> Option<&'a str> {
        query
            .iter()
            .find(|(name, _)| name == key)
            .map(|(_, value)| value.as_str())
    }

    #[test]
    fn an_ohlcv_query_names_the_priced_token_on_a_quote_side_pool() {
        // USDC is the quote side of a SOL/USDC pool on GeckoTerminal; only `token`
        // makes the series price USDC instead of SOL.
        let query = ohlcv_query(USDC, Some(5), Some(100), Some("token"), Some(1_700_000_000));
        assert_eq!(param(&query, "token"), Some(USDC));
        assert_eq!(param(&query, "currency"), Some("token"));
        assert_eq!(param(&query, "aggregate"), Some("5"));
        assert_eq!(param(&query, "limit"), Some("100"));
        assert_eq!(param(&query, "before_timestamp"), Some("1700000000"));
    }

    #[test]
    fn an_ohlcv_query_without_options_still_names_the_token() {
        let query = ohlcv_query(USDC, None, Some(5_000), None, None);
        assert_eq!(param(&query, "token"), Some(USDC));
        assert_eq!(param(&query, "limit"), Some("1000"));
        assert_eq!(query.len(), 2);
    }
}
