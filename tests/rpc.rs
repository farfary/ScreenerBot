//! Live (read-only): Solana RPC connectivity through the app's global client.
//! Validates `get_rpc_client()` + RpcManager end to end against mainnet.
//!
//! `#[ignore]` — run with the live command (`./test.sh live`). No wallet, no keys, a
//! `getSlot` read only. Multi-thread runtime flavour because the lazy RPC init calls
//! `block_in_place` (panics on a current-thread runtime). The default config ships a
//! public mainnet RPC URL (`cfg.rpc.urls`), so `isolated_env()` is enough.

mod common;

use screenerbot::rpc::{get_rpc_client, RpcClientMethods};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "live network"]
async fn rpc_client_reads_current_slot() {
    let _guard = common::isolated_env();
    let client = get_rpc_client();
    let slot = client
        .get_slot()
        .await
        .expect("live RPC get_slot should succeed");
    assert!(
        slot > 0,
        "current mainnet slot must be positive, got {slot}"
    );
}

// Add next: get_multiple_accounts (<= 50, jsonParsed) of a known pool's accounts, then
// feed the bytes to pools::decoders::decode_pool -> assert a sane SOL price. That live
// decode test belongs in pools.rs and is the clean way to validate decoders/calculators.
