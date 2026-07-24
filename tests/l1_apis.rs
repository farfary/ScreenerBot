//! L1 (read-only live network): exercises the app's real API clients against live
//! upstreams. No money is spent.
//!
//! Ignored by default — run with `./test.sh live`, which lifts `#[ignore]`. A failure
//! here means an upstream endpoint/schema drifted or a client regressed, not that
//! trading is unsafe.

mod common;

use screenerbot::apis::DexScreenerClient;
use screenerbot::sol_price::fetch_and_cache_sol_price;

const WSOL: &str = "So11111111111111111111111111111111111111112";

#[tokio::test]
#[ignore = "L1 live network: run via `./test.sh live`"]
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
#[ignore = "L1 live network: run via `./test.sh live`"]
async fn dexscreener_returns_pools_for_wsol() {
    let _guard = common::isolated_env();
    let client = DexScreenerClient::new(true, 10).expect("construct DexScreener client");
    let pools = client
        .fetch_token_pools(WSOL, Some("solana"))
        .await
        .expect("live DexScreener fetch should succeed");
    assert!(!pools.is_empty(), "wSOL must have at least one live pool");
}

// Extension points — add next, each following the same pattern (isolated_env + a real
// app client, `#[ignore]` + `#[tokio::test]`):
//   - GeckoTerminal OHLCV fetch (candles are SOL-denominated, volume > 0, ts snapped)
//   - Rugcheck report fetch (tokens::security::rugcheck)
//   - screenerbot-data server: /v1/health, /v1/ohlcv, /v1/pools, /v1/rugcheck
//   - RPC read: get_multiple_accounts via get_rpc_client() (<= 50 accounts, jsonParsed)
//   - Jupiter quote only (no execution)
//   - Pool decoders/calculators against real on-chain account bytes (fetch then decode)
