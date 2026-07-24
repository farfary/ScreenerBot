//! Live (read-only): the app's real API clients against live upstreams. No money.
//!
//! `#[ignore]` — run with `cargo nextest run --run-ignored ignored-only -E 'kind(test)'`
//! (or `./test.sh live`). A failure here means an upstream endpoint/schema drifted or a
//! client regressed, not that trading is unsafe.

mod common;

use screenerbot::apis::{DexScreenerClient, RugcheckClient};
use screenerbot::sol_price::fetch_and_cache_sol_price;

const WSOL: &str = "So11111111111111111111111111111111111111112";
const USDC: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";

#[tokio::test]
#[ignore = "live network"]
async fn sol_price_fetches_a_plausible_value() {
    let _guard = common::isolated_env();
    let price = fetch_and_cache_sol_price()
        .await
        .expect("live SOL price fetch should succeed");
    assert!(
        (1.0..=100_000.0).contains(&price),
        "SOL price out of plausible range: ${price}"
    );
}

#[tokio::test]
#[ignore = "live network"]
async fn dexscreener_returns_pools_for_wsol() {
    let _guard = common::isolated_env();
    let client = DexScreenerClient::new(true, 10).expect("construct DexScreener client");
    let pools = client
        .fetch_token_pools(WSOL, Some("solana"))
        .await
        .expect("live DexScreener fetch should succeed");
    assert!(!pools.is_empty(), "wSOL must have at least one live pool");
}

#[tokio::test]
#[ignore = "live network"]
async fn rugcheck_returns_a_report_for_usdc() {
    let _guard = common::isolated_env();
    let client = RugcheckClient::new(true, 30, 10).expect("construct Rugcheck client");
    let report = client
        .fetch_report(USDC)
        .await
        .expect("live Rugcheck fetch should succeed");
    assert_eq!(
        report.mint.as_str(),
        USDC,
        "report is for the requested mint"
    );
    assert!(!report.rugged, "USDC must not be flagged as rugged");
}

// Add next, same pattern (common::isolated_env + a real app client, `#[ignore]`):
//   - GeckoTerminal OHLCV fetch (SOL-denominated, volume > 0, ts snapped) -> ohlcv.rs
//   - screenerbot-data server: /v1/health, /v1/ohlcv, /v1/pools, /v1/rugcheck
//   - Jupiter quote only (no execution)
