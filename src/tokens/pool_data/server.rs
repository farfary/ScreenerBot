// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Token pools via the self-hosted ScreenerBot data server.
//!
//! The server is the central pool registry: it resolves and caches every token's
//! pools centrally and names the canonical pool its candle series is stored
//! under (the OHLCV chart's series pool). Consumed here as the PRIMARY pool
//! source in the token pool_data fetch, so its pools flow to EVERY consumer
//! (OHLCV monitor, pool service, dashboard) through the single shared
//! `TokenPoolsSnapshot` — not via per-subsystem hooks. DexScreener/GeckoTerminal
//! still run and enrich per-pool price/volume; when they are rate-limited or
//! fail, the server-provided pools alone keep the snapshot non-empty (fixing the
//! "no local pool → OHLCV/chart stuck" case). On any disabled/miss/timeout/error
//! it returns `None` so the direct providers remain the fallback.

use crate::chains::ChainId;
use crate::tokens::types::TokenPoolInfo;
use chrono::Utc;

/// The data server's pool registry answer for one mint.
#[derive(Debug, Clone, Default)]
pub struct ServerPools {
    /// Every pool the registry lists for the mint.
    pub pools: Vec<TokenPoolInfo>,
    /// The registry's canonical pool (the top-level `pool` of `/v1/pools`): the
    /// pool the server stores the mint's SOL-denominated candle series under,
    /// whatever its quote token.
    pub series_pool: Option<String>,
}

/// Fetch a token's pools from the data server's `/v1/pools`. Returns `None` when
/// the source is unavailable for any reason, so the direct providers remain the
/// fallback; `data_server::access` carries the reason.
pub async fn fetch_pools_from_server(chain: ChainId, mint: &str) -> Option<ServerPools> {
    let body: serde_json::Value = crate::data_server::get_json(
        crate::data_server::Surface::Tokens,
        chain,
        "/v1/pools",
        &[("mint", mint.to_string())],
    )
    .await?;
    parse_pools_body(mint, &body)
}

/// Parse a `/v1/pools` body. `None` when it lists no usable pool.
fn parse_pools_body(mint: &str, body: &serde_json::Value) -> Option<ServerPools> {
    let arr = body.get("pools")?.as_array()?;
    let now = Utc::now();
    let mut out = Vec::new();
    for p in arr {
        let Some(pool_address) = p.get("pool").and_then(|v| v.as_str()) else {
            continue;
        };
        let dex = p
            .get("dex")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(str::to_string);
        let quote_mint = p
            .get("quote_mint")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let is_native_pair = p
            .get("is_sol_pair")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let liquidity_usd = p.get("liquidity_usd").and_then(|v| v.as_f64());
        out.push(TokenPoolInfo {
            pool_address: pool_address.to_string(),
            dex,
            base_mint: mint.to_string(),
            quote_mint,
            is_native_pair,
            liquidity_usd,
            pool_data_last_fetched_at: now,
            pool_data_first_seen_at: now,
            ..Default::default()
        });
    }

    if out.is_empty() {
        return None;
    }
    let series_pool = body
        .get("pool")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    Some(ServerPools {
        pools: out,
        series_pool,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn the_top_level_pool_is_the_series_pool_whatever_its_quote() {
        let body = json!({
            "pool": "UsdcPool",
            "pools": [
                {"pool": "SolPool", "dex": "raydium", "quote_mint": "So11111111111111111111111111111111111111112", "is_sol_pair": true, "liquidity_usd": 500000.0},
                {"pool": "UsdcPool", "dex": "orca", "quote_mint": "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v", "is_sol_pair": false, "liquidity_usd": 800000.0},
            ],
        });
        let parsed = parse_pools_body("mint", &body).expect("two pools");
        assert_eq!(parsed.series_pool.as_deref(), Some("UsdcPool"));
        assert_eq!(parsed.pools.len(), 2);
        assert!(!parsed.pools[1].is_native_pair);
    }

    #[test]
    fn a_missing_quote_flag_is_not_native_and_a_missing_series_pool_is_none() {
        let body = json!({"pool": null, "pools": [{"pool": "NoQuote", "quote_mint": null}]});
        let parsed = parse_pools_body("mint", &body).expect("one pool");
        assert_eq!(parsed.series_pool, None);
        assert!(!parsed.pools[0].is_native_pair);
        assert!(parse_pools_body("mint", &json!({"pool": "X", "pools": []})).is_none());
    }
}
