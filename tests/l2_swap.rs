//! L2 (real mainnet swap — spends SOL): the full manual buy -> verify -> sell ->
//! verify lifecycle against mainnet.
//!
//! NEVER runs by default. It is `#[ignore]`'d AND guarded by `require_mainnet()`, which
//! returns `None` (clean skip, no spend, no failure) unless the owner explicitly opts
//! in with `SB_TEST_MAINNET_SWAP=1` and a funded `SB_TEST_WALLET`. Drive it with
//! `./test.sh mainnet`. Spend is capped by `SB_TEST_MAX_LAMPORTS`.

mod common;

#[tokio::test]
#[ignore = "L2 mainnet swap (spends real SOL): run via `./test.sh mainnet`"]
async fn manual_buy_then_sell_lifecycle() {
    let Some(ctx) = common::require_mainnet() else {
        return; // Not opted in — skip without spending or failing.
    };
    let _guard = common::isolated_env();

    // Prove the guard + wallet resolve so `./test.sh mainnet` exercises the harness.
    assert!(ctx.max_lamports > 0, "spend cap must be positive");
    assert!(
        std::path::Path::new(&ctx.wallet_path).exists(),
        "funded test wallet not found at {}",
        ctx.wallet_path
    );

    // TODO(wire): drive the real manual-trade path end to end, capped at ctx.max_lamports:
    //   1. load keypair from ctx.wallet_path; init RPC via get_rpc_client()
    //   2. trader::manual::manual_buy(ctx.mint, size <= cap) -> assert a position opens
    //   3. await entry verification (EntryVerified)          -> assert holdings > 0
    //   4. trader::manual::manual_sell(ctx.mint, close_all)  -> assert exit submitted
    //   5. await ExitVerified                                -> assert closed, PnL recorded,
    //      slot released exactly once, exit record written
    eprintln!(
        "L2 harness ready: mint={} cap={} lamports (swap execution not yet wired)",
        ctx.mint, ctx.max_lamports
    );
}
