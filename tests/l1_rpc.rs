//! L1 (read-only live network): Solana RPC connectivity through the app's global
//! client. Validates `get_rpc_client()` + RpcManager end to end against mainnet.
//!
//! Ignored by default — run with `./test.sh live`. In its own file so the global RPC
//! client (a `OnceLock`) initialises cleanly in a fresh process. Uses the multi-thread
//! runtime flavour because the lazy RPC init calls `block_in_place`, which panics on a
//! current-thread runtime.
//!
//! No wallet, no keys, no writes — a `getSlot` read only. The default config ships a
//! public mainnet RPC URL (`cfg.rpc.urls`), so `isolated_env()` is enough.

mod common;

use screenerbot::rpc::{get_rpc_client, RpcClientMethods};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "L1 live network: run via `./test.sh live`"]
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
