//! Rugcheck security reports via the self-hosted ScreenerBot data server.
//!
//! This is a SEPARATE source from the direct Rugcheck API client
//! (`crate::apis::rugcheck`). The security fetcher tries this server FIRST: it
//! serves a shared cache fast and warms itself, sparing the direct Rugcheck rate
//! budget. On any disabled/miss/timeout/error it returns `None` so the caller
//! falls straight through to the direct Rugcheck API — purely an accelerator,
//! never a hard dependency.
//!
//! Because this path never touches `RugcheckClient`, it does NOT consume that
//! client's rate limiter; the server enforces its own per-IP limit upstream.

use crate::apis::rugcheck::{RugcheckInfo, RugcheckResponse};
use crate::config::with_config;
use std::time::Duration;

/// Try to fetch a Rugcheck report for `mint` from the self-hosted ScreenerBot
/// data server. Returns `None` (so the caller falls back) when the source is
/// disabled, unconfigured, or the request misses/times out/errors.
///
/// The server responds with `{ mint, fetched_at, report: <raw Rugcheck JSON> }`;
/// the `report` is the byte-identical upstream payload, so it deserializes into
/// the same `RugcheckResponse` and converts via the shared `from_response`.
pub async fn fetch_report_from_server(mint: &str) -> Option<RugcheckInfo> {
    let (enabled, endpoint, timeout_secs) = with_config(|c| {
        let s = &c.tokens.sources.screenerbot_server;
        (s.enabled, s.endpoint.clone(), s.timeout_seconds)
    });
    if !enabled || endpoint.trim().is_empty() {
        return None;
    }

    let url = format!(
        "{}/v1/rugcheck?mint={}",
        endpoint.trim_end_matches('/'),
        mint
    );
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
    let report = body.get("report")?.clone();
    let api_response: RugcheckResponse = serde_json::from_value(report).ok()?;
    Some(RugcheckInfo::from_response(api_response))
}
