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
use crate::data_server::{get_json, Surface};
use std::collections::HashMap;

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
    let body: serde_json::Value = get_json(
        Surface::Tokens,
        "/v1/rugcheck",
        &[("mint", mint.to_string())],
    )
    .await?;
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
    // One question before a loop of up to N chunks: an install with no access
    // must not spend a refused round trip per chunk to learn the same thing.
    if mints.is_empty() || !crate::data_server::is_usable(Surface::Tokens) {
        return out;
    }

    for chunk in mints.chunks(SERVER_RUGCHECK_BATCH) {
        let Some(body) = get_json::<serde_json::Value>(
            Surface::Tokens,
            "/v1/rugcheck",
            &[("mints", chunk.join(","))],
        )
        .await
        else {
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
