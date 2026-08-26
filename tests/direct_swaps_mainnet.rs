//! Direct pool swaps against real mainnet state.
//!
//! Two tiers live here, and they are deliberately different in what they risk:
//!
//! * **live** (`#[ignore]`, `./test.sh live`) — loads a real pool, quotes it,
//!   builds the real transaction and SIMULATES it. The node executes the exact
//!   account list, instruction data and `min_out` a real swap would carry,
//!   against real balances, and nothing is signed or submitted. A wrong account
//!   order, a bad discriminator, an under-sized compute budget or an
//!   unsatisfiable `min_out` all fail here, for free. This is the tier that must
//!   be green before any real swap is attempted.
//! * **mainnet** (`#[ignore]` + [`common::require_mainnet`], `./test.sh mainnet`)
//!   — submits a real swap and a real sell-back, capped at `SB_TEST_MAX_LAMPORTS`.
//!   Self-skips unless `SB_TEST_MAINNET_SWAP=1` and `SB_TEST_WALLET` are both set,
//!   so the live command above never spends anything.
//!
//! The pools are hardcoded because a live test needs a real one. They are the
//! deepest pool of each venue, chosen so they stay alive; if one is ever drained
//! or migrated, the fix is to repoint the constant, not to weaken the assertion.

mod common;

use screenerbot::chains::solana::solana_sdk::pubkey::Pubkey;
use screenerbot::chains::solana::swaps::direct::{
    self, DirectSwapIntent, FeeSide, PlatformFee, SwapAccounts,
};
use std::str::FromStr;

/// Raydium AMM v4, SOL/USDC. The deepest v4 pool on mainnet.
const AMM_V4_SOL_USDC: &str = "58oQChx4yWmvKdwLLZzBi4ChoCc2fqCUWBkwMihLYQo2";

/// Raydium CLMM, SOL/USDC. The deepest concentrated-liquidity pool on mainnet.
const CLMM_SOL_USDC: &str = "3ucNos4NbumPLZNWztqGHNFFgkHeRMBQAVemeeomsUxv";

/// Raydium CPMM (CP-Swap), SOL paired against a Token-2022 mint.
const CPMM_POOL: &str = "Q2sPHPdUWFMg7M7wwrQKLrn619cAucfRsmhVJffodSp";

const WSOL: &str = "So11111111111111111111111111111111111111112";
const USDC: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";

/// The smallest swap the bot ever makes: 0.005 SOL.
const MINIMUM_SWAP_LAMPORTS: u64 = 5_000_000;

/// A wallet that only needs to EXIST for a simulation — nothing is signed, so
/// this is the app's own trading wallet address and no key is involved.
fn simulation_owner() -> Pubkey {
    let address = std::env::var("SB_TEST_OWNER")
        .unwrap_or_else(|_| "6uodGCMLfDLfeXkyW71WpUDK1BEG19bU2EV51x2dcGMv".to_owned());
    Pubkey::from_str(&address).expect("owner address must be a pubkey")
}

fn mint(address: &str) -> Pubkey {
    Pubkey::from_str(address).expect("mint constant must be a pubkey")
}

/// Quote, plan and simulate one direction, asserting the node accepts it.
async fn simulate_direction(pool: &str, input_mint: &str, output_mint: &str, amount_in: u64) {
    let owner = simulation_owner();
    let intent = DirectSwapIntent {
        pool: Pubkey::from_str(pool).expect("pool constant must be a pubkey"),
        owner,
        input_mint: mint(input_mint),
        output_mint: mint(output_mint),
        amount_in,
        slippage_bps: 300,
    };

    let (quote, market) = direct::quote(&intent)
        .await
        .unwrap_or_else(|e| panic!("live quote for {pool} failed: {e}"));

    assert!(quote.expected_out > 0, "a live pool must return something");
    assert!(
        quote.min_out <= quote.expected_out,
        "the floor can never exceed the estimate"
    );
    assert!(
        quote.min_net_out <= quote.min_out,
        "what the wallet keeps can never exceed what the pool guarantees"
    );

    let plan = direct::build_plan(&intent, market.as_ref(), &quote)
        .unwrap_or_else(|e| panic!("plan for {pool} failed: {e}"));

    assert_fee_is_collected(&quote.fee, &plan);

    let outcome = direct::simulate_plan(&plan, &owner)
        .await
        .unwrap_or_else(|e| panic!("simulation for {pool} could not run: {e}"));

    assert!(
        outcome.succeeded(),
        "the node rejected a {input_mint} -> {output_mint} swap in {pool}: {}\nlogs:\n{}",
        outcome.failure_detail(),
        outcome.logs.join("\n")
    );

    if let Some(units) = outcome.units_consumed {
        assert!(
            units < plan.venue_compute_units as u64,
            "the venue's {} CU estimate must cover the {units} CU the swap actually used",
            plan.venue_compute_units
        );
    }
}

/// The platform fee must be a real instruction in the plan, not a number in a
/// struct. This is the assertion that catches a fee silently going uncollected.
fn assert_fee_is_collected(fee: &PlatformFee, plan: &direct::SwapPlan) {
    if fee.side == FeeSide::None {
        return;
    }
    assert!(
        fee.amount > 0,
        "a 0.005 SOL swap is far above the rounding floor -- a zero fee here means it is not being charged"
    );
    let destination = fee
        .destination
        .expect("a collectible fee has a destination");

    let carried = plan.instructions.iter().any(|ix| {
        ix.program_id == screenerbot::chains::solana::spl_token::id()
            && ix.data.first() == Some(&12)
            && ix.accounts.iter().any(|a| a.pubkey == destination)
    });
    assert!(
        carried,
        "the plan must contain a TransferChecked into {destination} -- the fee is only real if it is in the transaction"
    );
}

// ============================================================================
// LIVE TIER — real pools, real balances, nothing submitted
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "live network"]
async fn amm_v4_accepts_a_minimum_sol_to_usdc_swap() {
    let _guard = common::isolated_env();
    simulate_direction(AMM_V4_SOL_USDC, WSOL, USDC, MINIMUM_SWAP_LAMPORTS).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "live network"]
async fn cpmm_accepts_a_minimum_sol_to_token_swap() {
    let _guard = common::isolated_env();
    let market = direct::load_market(&Pubkey::from_str(CPMM_POOL).unwrap())
        .await
        .expect("the CPMM pool must decode");
    let (mint_a, mint_b) = market.mints();
    let token = if mint_a.to_string() == WSOL {
        mint_b
    } else {
        mint_a
    };

    simulate_direction(CPMM_POOL, WSOL, &token.to_string(), MINIMUM_SWAP_LAMPORTS).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "live network"]
async fn clmm_accepts_a_minimum_sol_to_usdc_swap() {
    let _guard = common::isolated_env();
    simulate_direction(CLMM_SOL_USDC, WSOL, USDC, MINIMUM_SWAP_LAMPORTS).await;
}

/// The reverse leg matters on its own: a concentrated-liquidity swap walks tick
/// arrays in the direction the price moves, so selling reaches DIFFERENT
/// accounts than buying. A derivation that is right one way and wrong the other
/// passes a one-way test.
///
/// This cannot be simulated the way the forward leg is, because a simulation
/// runs against the wallet's REAL balances and the wallet holds no USDC to sell
/// — the token programme fails the transfer before the swap is reached. So the
/// free tier asserts the structure that differs, and the real reverse execution
/// is covered by the spending tier's round trip, which buys the token first.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "live network"]
async fn a_clmm_swap_reaches_different_tick_arrays_in_each_direction() {
    let _guard = common::isolated_env();
    let pool = Pubkey::from_str(CLMM_SOL_USDC).unwrap();
    let market = direct::load_market(&pool)
        .await
        .expect("the CLMM pool decodes");
    let owner = simulation_owner();

    let accounts_for = |input: &str, output: &str| {
        market
            .swap_instruction(
                &SwapAccounts {
                    owner,
                    input_mint: mint(input),
                    output_mint: mint(output),
                    input_token_account: Pubkey::new_unique(),
                    output_token_account: Pubkey::new_unique(),
                },
                MINIMUM_SWAP_LAMPORTS,
                1,
            )
            .expect("both directions build")
            .accounts
    };

    let buying = accounts_for(WSOL, USDC);
    let selling = accounts_for(USDC, WSOL);

    // 13 named accounts, then the bitmap extension, then the tick arrays.
    assert!(
        buying.len() > 14,
        "a CLMM swap must carry tick arrays; passing none is why the old implementation          could never execute"
    );
    assert_eq!(
        buying[13].pubkey, selling[13].pubkey,
        "the bitmap extension is a property of the pool, not of the direction"
    );
    let buy_arrays: Vec<_> = buying[14..].iter().map(|a| a.pubkey).collect();
    let sell_arrays: Vec<_> = selling[14..].iter().map(|a| a.pubkey).collect();
    assert_ne!(
        buy_arrays, sell_arrays,
        "the two directions walk the tick arrays opposite ways"
    );
    assert_eq!(
        buy_arrays[0], sell_arrays[0],
        "both start from the array holding the current tick"
    );
    assert_eq!(
        buying[5].pubkey, selling[6].pubkey,
        "the input vault of one direction is the output vault of the other"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "live network"]
async fn a_pool_quotes_both_directions_of_its_pair() {
    let _guard = common::isolated_env();
    let pool = Pubkey::from_str(AMM_V4_SOL_USDC).unwrap();
    let owner = simulation_owner();
    let market = direct::load_market(&pool).await.expect("pool decodes");

    let buy = DirectSwapIntent {
        pool,
        owner,
        input_mint: mint(WSOL),
        output_mint: mint(USDC),
        amount_in: MINIMUM_SWAP_LAMPORTS,
        slippage_bps: 300,
    };
    let buy_quote = direct::quote_with_market(&buy, market.as_ref()).expect("SOL -> USDC quotes");

    // Sell back roughly what the buy would return, so both legs are the same size.
    let sell = DirectSwapIntent {
        input_mint: mint(USDC),
        output_mint: mint(WSOL),
        amount_in: buy_quote.expected_net_out,
        ..buy
    };
    let sell_quote = direct::quote_with_market(&sell, market.as_ref()).expect("USDC -> SOL quotes");

    assert_eq!(
        buy_quote.fee.side,
        FeeSide::Output,
        "SOL -> USDC pays on the USDC leg, which is the output"
    );
    assert_eq!(
        sell_quote.fee.side,
        FeeSide::Output,
        "USDC -> SOL pays on the SOL leg, which is again the output"
    );

    // A round trip through a 0.25% pool with a 0.5% platform fee must lose value.
    assert!(
        sell_quote.expected_net_out < MINIMUM_SWAP_LAMPORTS,
        "a round trip that returns MORE than it started with means a fee is missing: \
         {} lamports back from {MINIMUM_SWAP_LAMPORTS}",
        sell_quote.expected_net_out
    );
    assert!(
        sell_quote.expected_net_out > MINIMUM_SWAP_LAMPORTS * 95 / 100,
        "a round trip should lose about 1%, not {}%",
        100 - (sell_quote.expected_net_out * 100 / MINIMUM_SWAP_LAMPORTS)
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "live network"]
async fn a_pool_refuses_a_pair_it_does_not_hold() {
    let _guard = common::isolated_env();
    let intent = DirectSwapIntent {
        pool: Pubkey::from_str(AMM_V4_SOL_USDC).unwrap(),
        owner: simulation_owner(),
        input_mint: mint(WSOL),
        // A real mint, but not one this pool trades.
        output_mint: mint("DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263"),
        amount_in: MINIMUM_SWAP_LAMPORTS,
        slippage_bps: 300,
    };
    let error = direct::quote(&intent)
        .await
        .expect_err("a pool that does not hold the pair must refuse it");
    assert!(
        error.is_token_fault(),
        "a pair mismatch is a verdict about the route, got {error}"
    );
}

// ============================================================================
// MAINNET TIER — real SOL, capped, opt-in
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "spends real SOL"]
async fn a_real_round_trip_through_amm_v4_settles_and_pays_the_platform_fee() {
    let _guard = common::isolated_env();
    let Some(ctx) = common::require_mainnet() else {
        return;
    };
    round_trip(&ctx, AMM_V4_SOL_USDC, USDC).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "spends real SOL"]
async fn a_real_round_trip_through_clmm_settles_and_pays_the_platform_fee() {
    let _guard = common::isolated_env();
    let Some(ctx) = common::require_mainnet() else {
        return;
    };
    round_trip(&ctx, CLMM_SOL_USDC, USDC).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "spends real SOL"]
async fn a_real_round_trip_through_cpmm_settles_and_pays_the_platform_fee() {
    let _guard = common::isolated_env();
    let Some(ctx) = common::require_mainnet() else {
        return;
    };
    let market = direct::load_market(&Pubkey::from_str(CPMM_POOL).unwrap())
        .await
        .expect("the CPMM pool must decode");
    let (mint_a, mint_b) = market.mints();
    let token = if mint_a.to_string() == WSOL {
        mint_b
    } else {
        mint_a
    };
    round_trip(&ctx, CPMM_POOL, &token.to_string()).await;
}

/// Buy `token` with the capped amount of SOL, then sell every unit back.
///
/// Both legs are real. The buy proves SOL -> TOKEN settles and that the wallet
/// received at least the guaranteed minimum; the sell proves the reverse
/// direction and returns the funds, so the test is close to cost-neutral apart
/// from fees. The sell uses exactly what the buy delivered, which is why the buy
/// receipt has to be exact rather than estimated.
async fn round_trip(ctx: &common::MainnetCtx, pool: &str, token: &str) {
    let keypair = ctx.keypair();
    let owner = {
        use screenerbot::chains::solana::solana_sdk::signature::Signer;
        keypair.pubkey()
    };
    let amount_in = ctx.max_lamports.min(MINIMUM_SWAP_LAMPORTS);
    assert!(
        amount_in <= ctx.max_lamports,
        "the spend cap is the hard ceiling"
    );

    let buy = DirectSwapIntent {
        pool: Pubkey::from_str(pool).unwrap(),
        owner,
        input_mint: mint(WSOL),
        output_mint: mint(token),
        amount_in,
        slippage_bps: 500,
    };

    let (buy_quote, _) = direct::quote(&buy).await.expect("buy quotes");
    assert!(
        buy_quote.fee.amount > 0,
        "the platform fee must be charged on a real swap"
    );

    let bought = direct::swap(&buy, &keypair)
        .await
        .unwrap_or_else(|e| panic!("the buy leg failed: {e}"));

    assert!(
        bought.receipt.exact,
        "a token output must be measured exactly, not inferred"
    );
    assert!(
        bought.receipt.received >= buy_quote.min_net_out,
        "received {} but the transaction guaranteed at least {}",
        bought.receipt.received,
        buy_quote.min_net_out
    );
    eprintln!(
        "BUY  {} -> {} sig={} received={} fee={}",
        amount_in, token, bought.signature, bought.receipt.received, bought.platform_fee
    );

    let sell = DirectSwapIntent {
        input_mint: mint(token),
        output_mint: mint(WSOL),
        amount_in: bought.receipt.received,
        ..buy
    };
    let sold = direct::swap(&sell, &keypair)
        .await
        .unwrap_or_else(|e| panic!("the sell leg failed: {e}"));

    eprintln!(
        "SELL {} -> SOL sig={} received={} fee={}",
        bought.receipt.received, sold.signature, sold.receipt.received, sold.platform_fee
    );
    assert!(
        sold.platform_fee > 0,
        "the sell leg must pay the platform fee too"
    );
    assert!(
        sold.receipt.received > 0,
        "the sell must return SOL to the wallet"
    );
}
