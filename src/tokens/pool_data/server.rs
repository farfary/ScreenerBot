//! Token pools via the self-hosted ScreenerBot data server.
//!
//! The server is the central pool registry: it resolves and caches every token's
//! pools (preferring the wSOL pool) centrally. Consumed here as the PRIMARY pool
//! source in the token pool_data fetch, so its pools flow to EVERY consumer
//! (OHLCV monitor, pool service, dashboard) through the single shared
//! `TokenPoolsSnapshot` — not via per-subsystem hooks. DexScreener/GeckoTerminal
//! still run and enrich per-pool price/volume; when they are rate-limited or
//! fail, the server-provided pools alone keep the snapshot non-empty (fixing the
//! "no local pool → OHLCV/chart stuck" case). On any disabled/miss/timeout/error
//! it returns `None` so the direct providers remain the fallback.

use crate::config::with_config;
use crate::tokens::types::TokenPoolInfo;
use chrono::Utc;
use std::time::Duration;

/// Fetch a token's pools from the data server's `/v1/pools`. Returns `None` when
/// the source is disabled/unconfigured or the request misses/times out/errors.
pub async fn fetch_pools_from_server(mint: &str) -> Option<Vec<TokenPoolInfo>> {
    let (enabled, endpoint, timeout_secs) = with_config(|c| {
        let s = &c.tokens.sources.screenerbot_server;
        (s.enabled, s.endpoint.clone(), s.timeout_seconds)
    });
    if !enabled || endpoint.trim().is_empty() {
        return None;
    }

    let url = format!("{}/v1/pools?mint={}", endpoint.trim_end_matches('/'), mint);
    let resp = crate::net::client()
        .get(&url)
        .timeout(Duration::from_secs(timeout_secs))
        .send()
        .await
        .ok()?;
    if !resp.status().is_success() {
        return None;
    }

    let body: serde_json::Value = resp.json().await.ok()?;
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
        let is_sol_pair = p.get("is_sol_pair").and_then(|v| v.as_bool()).unwrap_or(false);
        let liquidity_usd = p.get("liquidity_usd").and_then(|v| v.as_f64());
        out.push(TokenPoolInfo {
            pool_address: pool_address.to_string(),
            dex,
            base_mint: mint.to_string(),
            quote_mint,
            is_sol_pair,
            liquidity_usd,
            pool_data_last_fetched_at: now,
            pool_data_first_seen_at: now,
            ..Default::default()
        });
    }

    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}
