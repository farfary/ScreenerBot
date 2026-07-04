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
use std::collections::HashMap;
use std::time::Duration;

/// Max mints per batch call to the server's `/v1/rugcheck?mints=` endpoint. Matches
/// the server-side cap.
pub const SERVER_RUGCHECK_BATCH: usize = 30;

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

/// Batch-fetch Rugcheck reports for many mints in ONE server call
/// (`/v1/rugcheck?mints=a,b,c`, up to `SERVER_RUGCHECK_BATCH`). Returns a map of
/// only the mints the server had cached; the caller falls back to the direct
/// Rugcheck API (one request each) for the misses. This collapses N per-token
/// round-trips into one for the shared-cache hits, and — like the single path —
/// never touches the direct Rugcheck client's rate limiter (the server enforces
/// its own per-IP limit, spread across its proxy egresses).
///
/// Input larger than the batch cap is chunked into multiple calls. Returns an
/// empty map when the source is disabled/unconfigured or every call misses.
pub async fn fetch_reports_from_server(mints: &[String]) -> HashMap<String, RugcheckInfo> {
    let mut out = HashMap::new();
    let (enabled, endpoint, timeout_secs) = with_config(|c| {
        let s = &c.tokens.sources.screenerbot_server;
        (s.enabled, s.endpoint.clone(), s.timeout_seconds)
    });
    if !enabled || endpoint.trim().is_empty() || mints.is_empty() {
        return out;
    }
    let base = endpoint.trim_end_matches('/');

    for chunk in mints.chunks(SERVER_RUGCHECK_BATCH) {
        let joined = chunk.join(",");
        let url = format!("{base}/v1/rugcheck?mints={joined}");
        let Ok(resp) = crate::net::client()
            .get(&url)
            .timeout(Duration::from_secs(timeout_secs))
            .send()
            .await
        else {
            continue;
        };
        if !resp.status().is_success() {
            continue;
        }
        let Ok(body): Result<serde_json::Value, _> = resp.json().await else {
            continue;
        };
        let Some(reports) = body.get("reports").and_then(|v| v.as_object()) else {
            continue;
        };
        for (mint, entry) in reports {
            let Some(report) = entry.get("report") else {
                continue;
            };
            if let Ok(api_response) = serde_json::from_value::<RugcheckResponse>(report.clone()) {
                out.insert(mint.clone(), RugcheckInfo::from_response(api_response));
            }
        }
    }
    out
}
