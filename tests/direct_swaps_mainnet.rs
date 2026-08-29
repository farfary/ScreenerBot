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

/// Orca Whirlpool, SOL/USDC. The deepest Orca pool on mainnet.
const ORCA_SOL_USDC: &str = "Czfq3xZZDmsdGdUyrNLtRhGc47cXcZtLG4crryfu44zE";

/// Raydium CPMM (CP-Swap), SOL paired against a Token-2022 mint.
const CPMM_POOL: &str = "Q2sPHPdUWFMg7M7wwrQKLrn619cAucfRsmhVJffodSp";

/// Pump.fun AMM, a deep graduated pool whose quote side is SOL. Its market cap
/// is far above the last fee tier, so it pays the cheapest 30 bps.
const PUMP_AMM_POOL: &str = "4w2cysotX6czaUGmmWg13hDpY4QEMG2CzeKYEQyK9Ama";

/// Pump.fun AMM with USDC as its BASE and SOL as its quote — the orientation
/// that catches any code still assuming the SOL side is the token side.
const PUMP_AMM_SOL_QUOTE_POOL: &str = "Gf7sXMoP8iRw4iiXmJ1nq4vxcRycbGXy5RL8a8LnTd3v";

/// Pump.fun legacy bonding curve WITH a creator set, still trading (not
/// migrated, not mayhem, not cashback). Exercises the creator-fee path this
/// venue's whole blocker was about -- verified on chain against a live `buy`
/// paying `creator_vault` exactly `ceil(net * creator_bps / 10_000)`.
const PUMP_LEGACY_CREATOR_POOL: &str = "AUKZMypBMmVi3gPMNmY46eb841La2mZufkdkPSh3PWEj";

/// Pump.fun legacy bonding curve WITH the DEFAULT (unset) creator, the other
/// side of the same property: verified on chain to pay zero to `creator_vault`
/// on a real `buy_exact_sol_in` while still paying the protocol fee in full.
const PUMP_LEGACY_NO_CREATOR_POOL: &str = "cYyAicKQgqecnPNjgaGSee68n6DfhLMUYxfD4zBYKRT";

/// Moonit (formerly Moonshot), a `ConstantProductV1` bonding curve settling in
/// native SOL, the same shape as pump.fun legacy. Chosen because it was the
/// most recently traded live curve found while building this venue, so it is
/// well below its migration threshold. A bonding-curve pool can migrate at any
/// time -- a sudden failure here is more likely a migration than a code
/// regression; repoint the constant rather than weaken the assertion.
const MOONIT_POOL: &str = "Fiw2hDFe4YW4acj1pxpEwXVFf2aBHBBnog6qzoA5pCdW";

/// Meteora DAMM v2 with `collect_fee_mode = OnlyB` and SOL as token B, so a buy
/// pays the pool fee on the INPUT and a sell pays it on the output.
const DAMM_V2_ONLY_B_POOL: &str = "3CVNnECvuyPtUys2QpaLSNRrQMvbqArsNJqKvbp3zmt1";

/// Meteora DAMM v2 with `collect_fee_mode = BothToken`, which always charges the
/// output leg. Together the two pools cover both branches of the fee side.
const DAMM_V2_BOTH_TOKEN_POOL: &str = "6nA26rxJxWZicm5bFnTpjUcN6jCrybqLuRrApKVERSz3";

/// Meteora DLMM, SOL/USDC, bin step 4 (0.04% base fee). The deepest DLMM pool
/// on mainnet -- also, per `venues.md`/module docs, one that trades often
/// enough that the offline vault-delta replay could not be completed against
/// it; this live tier is the exactness proof that IS available.
const DLMM_SOL_USDC: &str = "5rCf1DM8LjKTw4YqhnoLcngyZYeNnQqztScTogYHAS6";

/// A Meteora DLMM pool with NO `bin_array_bitmap_extension` account, which is
/// the COMMON case -- most pools never create one, since it only exists where
/// liquidity reaches past the +/-512 array indices `LbPair`'s own bitmap
/// covers. `DLMM_SOL_USDC` happens to HAVE one, so it cannot catch an
/// instruction that names the un-created PDA instead of spelling the optional
/// account as absent.
///
/// It also has SOL as token Y rather than token X, so buying with SOL walks
/// bins UPWARD -- the direction `DLMM_SOL_USDC` (SOL is token X) never
/// exercises.
const DLMM_NO_BITMAP_EXTENSION_POOL: &str = "46CgAPEz8V2e9UL5PDa3JNWFW6sk7uFCj7TjdB3XbKD3";

/// Meteora DBC (bonding curve), `collect_fee_mode = 0` (`QuoteToken`).
/// Verified not-migrated with a real (~2.5 SOL of 85 SOL) buffer below its
/// own `migration_quote_threshold` when this venue was built -- a DBC pool
/// can cross that threshold and migrate within minutes of real trading, so
/// re-verify on chain before trusting this constant again (see
/// `meteora_dbc.rs`'s module docs for the pool that migrated out from under
/// this venue's first fixture attempt).
const DBC_QUOTE_FEE_POOL: &str = "J1YvC19EHXGjmthszo7sM5FwL3mn3qjS8Bf3zTMjeX2T";

/// Meteora DBC, `collect_fee_mode = 1` (`OutputToken`) -- the OTHER branch of
/// `DbcMarket::fee_on_input`. Essentially untraded when chosen (near-zero
/// `quote_reserve`), so it exercises the branch and the layout rather than
/// depth; re-verify on chain before trusting this constant too.
const DBC_OUTPUT_FEE_POOL: &str = "F71peWVSaCjEL5bsMJMrhguuEN7K8vhiKvFN6tf91Ymk";

/// FluxBeam AMM, a fork of the vanilla `spl-token-swap` reference programme --
/// SOL paired against a Token-2022 mint, SOL as token A. Fees are read live
/// from the pool, never hardcoded, and this one's are extreme: `20/10_000`
/// trade plus `99/100` OWNER, so only 0.8% of any input ever reaches the
/// curve. That is not a decoding error -- it is replayed to the raw unit
/// against a real buy (see `venues/fluxbeam.rs`), and roughly 60% of live
/// FluxBeam pools carry a 90% or 99% owner fee. Kept deliberately as the
/// fixture and live-simulation pool because it exercises the owner-fee term
/// hardest; the SPENDING round trip deliberately uses a sane pool instead.
const FLUXBEAM_POOL: &str = "7uajENggf2MaiZ5XGff91uoVsch1y5QN3bqjisv7eP6V";

/// FluxBeam with SOL as token **B**, so a SOL buy's source is the pool's SECOND
/// side. This is the pool that settles what a same-orientation pool cannot:
/// account slots 9-13 (the two mints and their token programmes) are built
/// POOL-ordered, and on a pool where SOL is token A that is indistinguishable
/// from swap-ordered because the two coincide. Here they do NOT coincide -- a
/// swap-ordered programme would need `(mint_b, mint_a)` and
/// `(program_b, ..., program_a)` -- so this simulation alone discriminates
/// between the two hypotheses, for free and without spending anything.
///
/// Its base mint is Token-2022 with an `interestBearingConfig` extension, which
/// changes only the displayed UI amount and never a raw transfer amount, so it
/// is accepted; a transfer-FEE mint would be refused (see the module docs).
const FLUXBEAM_SOL_QUOTE_POOL: &str = "82Gxnc1ubRPWKn8nQRRb45KhBKJ15LoxtQ9rRnWPPUSq";

// ----------------------------------------------------------------------------
// Pools used ONLY by the sell tier
// ----------------------------------------------------------------------------
//
// A sell simulation borrows a real mainnet holder's address as the owner, so it
// needs a pool whose traders still hold the token. Several constants above were
// picked for a BRANCH (a `collect_fee_mode`, an orientation, an absent optional
// account) rather than for depth, and on those the last hundred transactions
// leave nobody holding more than dust -- `DAMM_V2_ONLY_B_POOL` had exactly one
// holder, of two raw units, and `FLUXBEAM_SOL_QUOTE_POOL` two holders whose
// combined balance the pool will not price. Each pool below is the same venue,
// same orientation, with real holders; verified on chain by loading it, quoting
// a buy and finding its holders before being written here.

/// FluxBeam with SOL as token **A** and real holders of token B.
///
/// A SELL here spends token B for token A, so swap order and pool order are
/// REVERSED -- the same discrimination `FLUXBEAM_SOL_QUOTE_POOL` provides on its
/// buy leg, which is the ordering this venue's first draft got wrong. That pool
/// cannot serve here: its own traders hold only dust the pool will not price.
const FLUXBEAM_SELL_POOL: &str = "3iBMru1J9LzTUPacicZVz3GojvZtuoevT8syreApE8UH";

/// A pump.fun legacy bonding curve with real recent volume and holders. A curve
/// GRADUATES: read a sudden failure here as a probable migration and repoint the
/// constant, exactly as `meteora_dbc.rs` documents for its own test pool.
const PUMP_LEGACY_SELL_POOL: &str = "6ZSN9gAqMphoAoUSsryxcASZv5dUxecT7yfy2HHjaDL5";

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

/// The other side of the pair a SOL pool trades.
async fn paired_token(pool: &str) -> String {
    let market = direct::load_market(&Pubkey::from_str(pool).expect("pool constant"))
        .await
        .unwrap_or_else(|e| panic!("{pool} must decode: {e}"));
    let (mint_a, mint_b) = market.mints();
    if mint_a.to_string() == WSOL {
        mint_b.to_string()
    } else {
        mint_a.to_string()
    }
}

/// Simulate a swap whose `min_out` is the quote EXACTLY, with no slippage room.
///
/// This is the sharpest free check of a venue's curve there is. `min_out` is
/// enforced by the pool programme itself, so a node accepting a zero-slippage
/// swap proves the quote does not over-state the output by even one raw unit —
/// which is the only direction that costs money, because an over-stated quote
/// becomes an unsatisfiable floor and reverts the whole transaction.
async fn simulate_with_no_slippage_room(
    pool: &str,
    input_mint: &str,
    output_mint: &str,
    amount_in: u64,
) {
    simulate_with_no_slippage_room_as(simulation_owner(), pool, input_mint, output_mint, amount_in)
        .await;
}

/// [`simulate_with_no_slippage_room`] with an explicit owner, for the rare test
/// whose size needs more real SOL than the default simulation wallet holds --
/// nothing is signed or submitted, so any real wallet's public address is a
/// valid stand-in for "an account with enough lamports to make the numbers
/// realistic".
async fn simulate_with_no_slippage_room_as(
    owner: Pubkey,
    pool: &str,
    input_mint: &str,
    output_mint: &str,
    amount_in: u64,
) {
    let intent = DirectSwapIntent {
        pool: Pubkey::from_str(pool).expect("pool constant must be a pubkey"),
        owner,
        input_mint: mint(input_mint),
        output_mint: mint(output_mint),
        amount_in,
        slippage_bps: 0,
    };

    let failure = attempt_zero_slippage(&intent, &owner).await;
    assert!(
        failure.is_none(),
        "the venue over-stated its output: a {amount_in} unit {input_mint} -> {output_mint} swap \
         in {pool} was refused on every attempt against fresh state:\n{}",
        failure.unwrap_or_default()
    );
}

/// How many times a zero-slippage proof is re-measured against FRESH state.
///
/// This does not loosen the assertion by one raw unit — `min_out` is still the
/// quote exactly, and the pool programme still enforces it. It removes a race.
/// The deepest pools trade several times per BLOCK, so a floor computed from a
/// `load()` a few hundred milliseconds before `simulate()` can legitimately miss
/// a real intervening swap; that failure is market drift, not arithmetic. An
/// over-stating curve is deterministic in the state it is given, so it fails
/// every attempt, while drift clears on the next one.
const ZERO_SLIPPAGE_ATTEMPTS: usize = 3;

/// Quote, plan and simulate `intent` at zero slippage, re-measuring against
/// fresh state. Returns `None` once any attempt succeeds, or the collected
/// refusals when none does.
async fn attempt_zero_slippage(intent: &DirectSwapIntent, owner: &Pubkey) -> Option<String> {
    let mut refusals = Vec::new();
    for attempt in 1..=ZERO_SLIPPAGE_ATTEMPTS {
        let (quote, market) = match direct::quote(intent).await {
            Ok(pair) => pair,
            Err(e) => {
                refusals.push(format!("  attempt {attempt}: the quote failed: {e}"));
                continue;
            }
        };
        assert_eq!(
            quote.min_out, quote.expected_out,
            "zero slippage must leave the floor at the estimate"
        );
        let plan = direct::build_plan(intent, market.as_ref(), &quote)
            .unwrap_or_else(|e| panic!("tight plan for {} failed: {e}", intent.pool));
        let outcome = direct::simulate_plan(&plan, owner)
            .await
            .unwrap_or_else(|e| panic!("tight simulation for {} could not run: {e}", intent.pool));
        if outcome.succeeded() {
            return None;
        }
        refusals.push(format!(
            "  attempt {attempt}: promised {} but the pool would not pay it: {}\n    logs: {}",
            quote.expected_out,
            outcome.failure_detail(),
            outcome.logs.join("\n           ")
        ));
    }
    Some(refusals.join("\n"))
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

/// A real Solana validator identity: a plain, System-Program-owned wallet with
/// a large public SOL balance. Nothing here is signed or submitted -- this is
/// only a fee-payer stand-in for a simulation, and only the wallet's real
/// on-chain balance is used, so any well-funded public address works. Chosen
/// deliberately over the default `simulation_owner()`, whose balance is far
/// too small to move the SOL/USDC CLMM pool's price across a whole
/// initialised tick.
fn deep_pockets_owner() -> Pubkey {
    Pubkey::from_str("JUPiTERrZqgf1jUyR7dSkhMx4Kn2qJyekWsg3LT1h4b")
        .expect("validator identity constant must be a pubkey")
}

/// The sharpest proof the tick walk exists at all: a size picked, against the
/// live pool, to cross at least one real initialised tick -- confirmed by
/// hand against this pool's own on-chain tick arrays before this constant was
/// fixed here. A single constant-liquidity step (what the venue did before
/// this task) would over-state the output past a crossing exactly like this
/// one, and `simulate_with_no_slippage_room` sets `min_out == expected_out`,
/// so the node accepting it proves the walk crosses the tick correctly to the
/// raw unit rather than merely refusing to.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "live network"]
async fn a_clmm_quote_that_crosses_a_tick_is_exact_to_the_raw_unit() {
    let _guard = common::isolated_env();
    // 10 SOL against the deepest SOL/USDC CLMM pool: small enough that the
    // deep-pockets wallet above easily covers it, large enough to walk past
    // the first initialised tick below the pool's current price.
    const CROSSES_A_TICK_LAMPORTS: u64 = 10_000_000_000;
    simulate_with_no_slippage_room_as(
        deep_pockets_owner(),
        CLMM_SOL_USDC,
        WSOL,
        USDC,
        CROSSES_A_TICK_LAMPORTS,
    )
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "live network"]
async fn orca_whirlpool_accepts_a_minimum_sol_to_usdc_swap() {
    let _guard = common::isolated_env();
    simulate_direction(ORCA_SOL_USDC, WSOL, USDC, MINIMUM_SWAP_LAMPORTS).await;
}

/// The zero-slippage exactness proof for Orca Whirlpool: with `min_out` set to
/// exactly the quote, a node accepting the swap proves the tick walk (shared
/// with Raydium CLMM via `clmm_ticks::walk_ticks`) is not over-stating the
/// output by even one raw unit on Orca's own 88-tick, decimal-string-seeded
/// tick arrays -- a layout `walk_ticks` itself never sees, only the ticks
/// `load()` decoded from it.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "live network"]
async fn an_orca_whirlpool_quote_is_exact_to_the_raw_unit() {
    let _guard = common::isolated_env();
    simulate_with_no_slippage_room(ORCA_SOL_USDC, WSOL, USDC, MINIMUM_SWAP_LAMPORTS).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "live network"]
async fn meteora_dlmm_accepts_a_minimum_sol_to_usdc_swap() {
    let _guard = common::isolated_env();
    simulate_direction(DLMM_SOL_USDC, WSOL, USDC, MINIMUM_SWAP_LAMPORTS).await;
}

/// The zero-slippage exactness proof for Meteora DLMM: with `min_out` set to
/// exactly the quote, a node accepting the swap proves the bin walk is not
/// over-stating the output by even one raw unit -- the strongest check this
/// engine can run for free, and the one this venue leans on most heavily
/// since its fee formula could not be cross-checked against a replayed
/// vault-delta transaction (see module docs on `meteora_dlmm.rs`).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "live network"]
async fn a_meteora_dlmm_quote_is_exact_to_the_raw_unit() {
    let _guard = common::isolated_env();
    simulate_with_no_slippage_room(DLMM_SOL_USDC, WSOL, USDC, MINIMUM_SWAP_LAMPORTS).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "live network"]
async fn meteora_dbc_accepts_a_minimum_sol_buy_on_the_quote_token_fee_pool() {
    let _guard = common::isolated_env();
    let token = paired_token(DBC_QUOTE_FEE_POOL).await;
    simulate_direction(DBC_QUOTE_FEE_POOL, WSOL, &token, MINIMUM_SWAP_LAMPORTS).await;
}

/// The zero-slippage exactness proof for Meteora DBC: with `min_out` set to
/// exactly the quote, a node accepting the swap proves the double-Q64.64
/// curve walk (see `meteora_dbc.rs`'s module docs for why the scale is
/// double, established by replaying a real transaction) is not over-stating
/// the output by even one raw unit.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "live network"]
async fn a_meteora_dbc_quote_is_exact_to_the_raw_unit() {
    let _guard = common::isolated_env();
    let token = paired_token(DBC_QUOTE_FEE_POOL).await;
    simulate_with_no_slippage_room(DBC_QUOTE_FEE_POOL, WSOL, &token, MINIMUM_SWAP_LAMPORTS).await;
}

/// `collect_fee_mode = 1` (`OutputToken`) is the OTHER branch of
/// `DbcMarket::fee_on_input` -- the quote-fee pool above always charges
/// mode 0. This pool was essentially untraded when chosen, so it proves the
/// branch and the layout, not depth.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "live network"]
async fn meteora_dbc_accepts_a_minimum_sol_buy_on_the_output_token_fee_pool() {
    let _guard = common::isolated_env();
    let token = paired_token(DBC_OUTPUT_FEE_POOL).await;
    simulate_direction(DBC_OUTPUT_FEE_POOL, WSOL, &token, MINIMUM_SWAP_LAMPORTS).await;
}

/// `bin_array_bitmap_extension` is an Anchor OPTIONAL account that most DLMM
/// pools never create. Naming its un-created PDA fails the programme's own
/// deserialisation; absent must be spelled as the programme's own id. The
/// deepest pool HAS an extension, so only a pool without one proves the
/// venue handles the majority case.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "live network"]
async fn meteora_dlmm_swaps_a_pool_that_has_no_bin_array_bitmap_extension() {
    let _guard = common::isolated_env();
    let token = paired_token(DLMM_NO_BITMAP_EXTENSION_POOL).await;
    simulate_direction(
        DLMM_NO_BITMAP_EXTENSION_POOL,
        WSOL,
        &token,
        MINIMUM_SWAP_LAMPORTS,
    )
    .await;
}

/// The reverse leg reaches different tick arrays than the forward one, the
/// same structural property Raydium CLMM's own test asserts -- see that
/// test's doc comment for why this cannot be simulated directly instead.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "live network"]
async fn an_orca_whirlpool_swap_reaches_different_tick_arrays_in_each_direction() {
    let _guard = common::isolated_env();
    let pool = Pubkey::from_str(ORCA_SOL_USDC).unwrap();
    let market = direct::load_market(&pool)
        .await
        .expect("the Orca Whirlpool pool decodes");
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

    // Legacy `swap` (11 accounts): token_program, token_authority, whirlpool,
    // owner_a, vault_a, owner_b, vault_b, then up to 3 tick arrays, then oracle.
    assert!(
        buying.len() > 7,
        "an Orca swap must carry tick arrays; passing none fails on deserialisation"
    );
    let buy_arrays_end = buying.len() - 1;
    let sell_arrays_end = selling.len() - 1;
    let buy_arrays: Vec<_> = buying[7..buy_arrays_end].iter().map(|a| a.pubkey).collect();
    let sell_arrays: Vec<_> = selling[7..sell_arrays_end]
        .iter()
        .map(|a| a.pubkey)
        .collect();
    assert_ne!(
        buy_arrays, sell_arrays,
        "the two directions walk the tick arrays opposite ways"
    );
    assert_eq!(
        buy_arrays[0], sell_arrays[0],
        "both start from the array holding the current tick"
    );
    // Unlike Raydium CLMM's swap_v2, Orca's `swap` never reorders the vaults
    // by direction -- vault_a and vault_b sit at fixed positions 4 and 6 in
    // every transaction, and only the `a_to_b` argument (plus which owner
    // account plays source vs destination) says which way the trade runs.
    assert_eq!(
        buying[4].pubkey, selling[4].pubkey,
        "vault_a is a property of the pool, not of the direction"
    );
    assert_eq!(
        buying[6].pubkey, selling[6].pubkey,
        "vault_b is a property of the pool, not of the direction"
    );
    assert_eq!(
        buying[buy_arrays_end].pubkey, selling[sell_arrays_end].pubkey,
        "the oracle account is a property of the pool, not of the direction"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "live network"]
async fn damm_v2_accepts_a_minimum_sol_to_token_swap_when_the_fee_rides_the_input() {
    let _guard = common::isolated_env();
    let token = paired_token(DAMM_V2_ONLY_B_POOL).await;
    simulate_direction(DAMM_V2_ONLY_B_POOL, WSOL, &token, MINIMUM_SWAP_LAMPORTS).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "live network"]
async fn damm_v2_accepts_a_minimum_sol_to_token_swap_when_the_fee_rides_the_output() {
    let _guard = common::isolated_env();
    let token = paired_token(DAMM_V2_BOTH_TOKEN_POOL).await;
    simulate_direction(DAMM_V2_BOTH_TOKEN_POOL, WSOL, &token, MINIMUM_SWAP_LAMPORTS).await;
}

/// A DAMM v2 pool holds ONE position spanning its whole price band, so the
/// active liquidity does not change with the price and a single
/// constant-liquidity step is exact. That claim is only worth making if the
/// chain agrees with it to the raw unit.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "live network"]
async fn a_damm_v2_quote_is_exact_to_the_raw_unit() {
    let _guard = common::isolated_env();
    for pool in [DAMM_V2_ONLY_B_POOL, DAMM_V2_BOTH_TOKEN_POOL] {
        let token = paired_token(pool).await;
        simulate_with_no_slippage_room(pool, WSOL, &token, MINIMUM_SWAP_LAMPORTS).await;
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "live network"]
async fn pump_amm_accepts_a_minimum_sol_to_token_swap() {
    let _guard = common::isolated_env();
    let token = paired_token(PUMP_AMM_POOL).await;
    simulate_direction(PUMP_AMM_POOL, WSOL, &token, MINIMUM_SWAP_LAMPORTS).await;
}

/// Pump's fee is a market-cap tier, not a constant. A quote that reads the flat
/// rate out of `GlobalConfig` and stops there is right for a big pool and wrong
/// by almost a percent for a small one, so the tier lookup is checked against
/// the pool programme itself with no slippage room to hide in.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "live network"]
async fn a_pump_amm_quote_is_exact_to_the_raw_unit() {
    let _guard = common::isolated_env();
    let token = paired_token(PUMP_AMM_POOL).await;
    simulate_with_no_slippage_room(PUMP_AMM_POOL, WSOL, &token, MINIMUM_SWAP_LAMPORTS).await;
}

/// The deepest pump-swap pool on mainnet holds USDC as its BASE and SOL as its
/// quote. Anything that assumes the SOL leg is the quote leg mis-orients here.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "live network"]
async fn pump_amm_swaps_a_pool_whose_sol_side_is_the_quote() {
    let _guard = common::isolated_env();
    let pool = Pubkey::from_str(PUMP_AMM_SOL_QUOTE_POOL).unwrap();
    let market = direct::load_market(&pool).await.expect("pool decodes");
    let (base, quote) = market.mints();
    assert_eq!(
        quote.to_string(),
        WSOL,
        "this fixture is chosen precisely because SOL is its quote"
    );
    simulate_direction(
        PUMP_AMM_SOL_QUOTE_POOL,
        WSOL,
        &base.to_string(),
        MINIMUM_SWAP_LAMPORTS,
    )
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "live network"]
async fn pump_legacy_accepts_a_minimum_sol_to_token_swap() {
    let _guard = common::isolated_env();
    let token = paired_token(PUMP_LEGACY_CREATOR_POOL).await;
    simulate_direction(
        PUMP_LEGACY_CREATOR_POOL,
        WSOL,
        &token,
        MINIMUM_SWAP_LAMPORTS,
    )
    .await;
}

/// The other side of the creator-vault property: a curve with NO creator must
/// still build and simulate a real swap, naming `bonding_curve_v2` as ABSENT
/// (the account is only appended when a creator is set) rather than the
/// un-created PDA a curve WITH a creator would name.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "live network"]
async fn pump_legacy_accepts_a_minimum_sol_to_token_swap_with_no_creator() {
    let _guard = common::isolated_env();
    let token = paired_token(PUMP_LEGACY_NO_CREATOR_POOL).await;
    simulate_direction(
        PUMP_LEGACY_NO_CREATOR_POOL,
        WSOL,
        &token,
        MINIMUM_SWAP_LAMPORTS,
    )
    .await;
}

/// The zero-slippage exactness proof for the buy leg, on a curve WITH a
/// creator so the creator-fee path is inside the number being proved.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "live network"]
async fn a_pump_legacy_quote_is_exact_to_the_raw_unit() {
    let _guard = common::isolated_env();
    let token = paired_token(PUMP_LEGACY_CREATOR_POOL).await;
    simulate_with_no_slippage_room(
        PUMP_LEGACY_CREATOR_POOL,
        WSOL,
        &token,
        MINIMUM_SWAP_LAMPORTS,
    )
    .await;
}

/// The same proof on a curve with NO creator, where the spend/net split has
/// one fee term instead of two -- the two curves round differently, and the
/// search has to be exact on both.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "live network"]
async fn a_pump_legacy_quote_is_exact_to_the_raw_unit_with_no_creator() {
    let _guard = common::isolated_env();
    let token = paired_token(PUMP_LEGACY_NO_CREATOR_POOL).await;
    simulate_with_no_slippage_room(
        PUMP_LEGACY_NO_CREATOR_POOL,
        WSOL,
        &token,
        MINIMUM_SWAP_LAMPORTS,
    )
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "live network"]
async fn moonit_accepts_a_minimum_sol_to_token_swap() {
    let _guard = common::isolated_env();
    let token = paired_token(MOONIT_POOL).await;
    simulate_direction(MOONIT_POOL, WSOL, &token, MINIMUM_SWAP_LAMPORTS).await;
}

/// The zero-slippage exactness proof: `min_out` IS the quote, and the pool
/// programme's own `SlippageOverflow` check is the enforcement -- confirmed
/// directly on chain while building this venue (a `simulateTransaction` with
/// the exact formula-predicted threshold succeeded; the identical swap with
/// the threshold one raw unit higher failed with `SlippageOverflow`, Anchor
/// error `6003`). This test is that same proof, through the engine's own
/// quote/plan/simulate path rather than a hand-built instruction.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "live network"]
async fn a_moonit_quote_is_exact_to_the_raw_unit() {
    let _guard = common::isolated_env();
    let token = paired_token(MOONIT_POOL).await;
    simulate_with_no_slippage_room(MOONIT_POOL, WSOL, &token, MINIMUM_SWAP_LAMPORTS).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "live network"]
async fn fluxbeam_accepts_a_minimum_sol_to_token_swap() {
    let _guard = common::isolated_env();
    let token = paired_token(FLUXBEAM_POOL).await;
    simulate_direction(FLUXBEAM_POOL, WSOL, &token, MINIMUM_SWAP_LAMPORTS).await;
}

/// The zero-slippage exactness proof for the vanilla `spl-token-swap` curve
/// replayed in `venues/fluxbeam.rs`'s module docs: `min_out` IS the quote, and
/// the pool programme's own on-chain check is the enforcement.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "live network"]
async fn a_fluxbeam_quote_is_exact_to_the_raw_unit() {
    let _guard = common::isolated_env();
    let token = paired_token(FLUXBEAM_POOL).await;
    simulate_with_no_slippage_room(FLUXBEAM_POOL, WSOL, &token, MINIMUM_SWAP_LAMPORTS).await;
}

/// The account-order discriminator described on `FLUXBEAM_SOL_QUOTE_POOL`. If
/// slots 9-13 were swap-ordered rather than pool-ordered, this simulation would
/// fail on the mint/programme check while `fluxbeam_accepts_a_minimum_sol_to_token_swap`
/// (SOL as token A, where the two orderings coincide) still passed.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "live network"]
async fn fluxbeam_accepts_a_swap_whose_sol_side_is_token_b() {
    let _guard = common::isolated_env();
    let token = paired_token(FLUXBEAM_SOL_QUOTE_POOL).await;
    simulate_direction(FLUXBEAM_SOL_QUOTE_POOL, WSOL, &token, MINIMUM_SWAP_LAMPORTS).await;
}

// The SELL direction (token -> SOL) is not simulated stand-alone here, for the
// same reason as every other venue in this file. It is also this venue's one
// remaining unverified fact -- see `venues/fluxbeam.rs`'s module docs on
// account slots 9-13 -- so a real round trip is the definitive proof, not
// just the usual convenience.

// The SELL direction (token -> SOL) is not simulated stand-alone here, the
// same as every other venue in this file: `simulation_owner` is a real
// trading wallet used ONLY for its address, not its holdings, and a fresh
// simulation cannot be relied on to hold the pump.fun token. The sell side's
// distinct account list and the native-SOL fee-wrap step it exercises are
// proven by `a_real_round_trip_through_pump_legacy_...` below instead, which
// sells exactly what a real buy just delivered.

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
async fn a_real_round_trip_through_orca_whirlpool_settles_and_pays_the_platform_fee() {
    let _guard = common::isolated_env();
    let Some(ctx) = common::require_mainnet() else {
        return;
    };
    round_trip(&ctx, ORCA_SOL_USDC, USDC).await;
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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "spends real SOL"]
async fn a_real_round_trip_through_damm_v2_settles_and_pays_the_platform_fee() {
    let _guard = common::isolated_env();
    let Some(ctx) = common::require_mainnet() else {
        return;
    };
    let token = paired_token(DAMM_V2_ONLY_B_POOL).await;
    round_trip(&ctx, DAMM_V2_ONLY_B_POOL, &token).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "spends real SOL"]
async fn a_real_round_trip_through_pump_amm_settles_and_pays_the_platform_fee() {
    let _guard = common::isolated_env();
    let Some(ctx) = common::require_mainnet() else {
        return;
    };
    let token = paired_token(PUMP_AMM_POOL).await;
    round_trip(&ctx, PUMP_AMM_POOL, &token).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "spends real SOL"]
async fn a_real_round_trip_through_meteora_dlmm_settles_and_pays_the_platform_fee() {
    let _guard = common::isolated_env();
    let Some(ctx) = common::require_mainnet() else {
        return;
    };
    round_trip(&ctx, DLMM_SOL_USDC, USDC).await;
}

/// Mirrors the pattern above for Meteora DBC. Never run automatically --
/// `SB_TEST_MAINNET_SWAP` is never set in this repo's own test runs.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "spends real SOL"]
async fn a_real_round_trip_through_meteora_dbc_settles_and_pays_the_platform_fee() {
    let _guard = common::isolated_env();
    let Some(ctx) = common::require_mainnet() else {
        return;
    };
    let token = paired_token(DBC_QUOTE_FEE_POOL).await;
    round_trip(&ctx, DBC_QUOTE_FEE_POOL, &token).await;
}

/// The exactness proof for the native-SOL plan change: the buy leg wraps only
/// the platform fee into the WSOL ATA rather than the whole spend, and the
/// sell leg's proceeds land as lamports in the wallet and have to be wrapped
/// AFTER the swap so the fee transfer has something to read. Never run
/// automatically -- see the module docs' two-tier explanation.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "spends real SOL"]
async fn a_real_round_trip_through_pump_legacy_settles_natively_and_pays_the_platform_fee() {
    let _guard = common::isolated_env();
    let Some(ctx) = common::require_mainnet() else {
        return;
    };
    let token = paired_token(PUMP_LEGACY_CREATOR_POOL).await;
    round_trip(&ctx, PUMP_LEGACY_CREATOR_POOL, &token).await;
}

/// The second native-SOL venue through the same plan path, and the only one
/// whose SELL threshold is measured against the output NET of the programme's
/// own fee rather than the gross curve output -- so a wrong fee orientation
/// here would show up as a real revert, not a rounding difference.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "spends real SOL"]
async fn a_real_round_trip_through_moonit_settles_natively_and_pays_the_platform_fee() {
    let _guard = common::isolated_env();
    let Some(ctx) = common::require_mainnet() else {
        return;
    };
    let token = paired_token(MOONIT_POOL).await;
    round_trip(&ctx, MOONIT_POOL, &token).await;
}

/// Runs against `FLUXBEAM_SOL_QUOTE_POOL`, not `FLUXBEAM_POOL`, for two
/// reasons. It is the REVERSE orientation, so its sell leg spends a
/// Token-2022 mint through account slots 9-13 in the order a same-orientation
/// pool can never exercise -- the ordering this venue got wrong on its first
/// draft. And `FLUXBEAM_POOL` charges a 99.2% owner fee, so a round trip there
/// would return almost nothing and measure the pool's rapacity rather than
/// this engine's correctness.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "spends real SOL"]
async fn a_real_round_trip_through_fluxbeam_settles_and_pays_the_platform_fee() {
    let _guard = common::isolated_env();
    let Some(ctx) = common::require_mainnet() else {
        return;
    };
    let token = paired_token(FLUXBEAM_SOL_QUOTE_POOL).await;
    round_trip(&ctx, FLUXBEAM_SOL_QUOTE_POOL, &token).await;
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

// ============================================================================
// THE SELL LEG — proved on chain, for free, against a real holder
// ============================================================================
//
// Every zero-slippage exactness test above is a BUY: WSOL in, token out. The
// sell leg was only ever proved by actually spending, which is why two venues
// (DAMM v2, pump-swap) have never had a sell executed at all and the rest were
// proved one real round trip at a time.
//
// A simulation runs with `sigVerify: false`, so the owner never signs and
// nothing is spent — which means ANY mainnet address can stand in as the owner,
// including one that already holds the token. Picking a real holder whose
// ASSOCIATED token account carries the balance turns the sell leg into exactly
// the same free, exact proof the buy leg already has: the pool programme
// enforces `min_out`, so a node accepting `min_out == expected_out` proves the
// sell direction does not over-state its output by one raw unit, and it proves
// the sell's account list, orientation and fee side at the same time.

/// Real mainnet wallets whose ASSOCIATED token account for `mint` currently
/// holds something, richest first, with what each holds.
///
/// The candidates come from the POOL'S OWN RECENT TRADES rather than from
/// `getTokenLargestAccounts` — two reasons, both discovered the hard way. Public
/// RPC endpoints refuse `getTokenLargestAccounts` outright (it is one of the
/// expensive scans they disable), and even where it answers, the twenty largest
/// accounts of a real token are protocol vaults and PDAs, not the ordinary
/// wallets this engine is built to spend from. Whoever traded the pool an hour
/// ago is exactly the right population: a wallet, holding the token, in its
/// associated account.
///
/// Only an ATA qualifies, because that is the account the swap plan derives and
/// spends from, and only a system-owned owner qualifies, because a PDA holding
/// the balance is usually the pool itself.
async fn holder_with_associated_account(
    pool: &Pubkey,
    mint: &Pubkey,
    token_program: &Pubkey,
) -> Vec<(Pubkey, u64)> {
    use screenerbot::chains::solana::rpc::{get_rpc_client, RpcClientMethods};
    use screenerbot::chains::solana::solana_sdk::commitment_config::CommitmentLevel;
    use screenerbot::chains::solana::spl_associated_token_account::get_associated_token_address_with_program_id;

    const SIGNATURES_PER_PAGE: usize = 40;
    const PAGES_TO_SCAN: usize = 3;
    const CANDIDATES_WANTED: usize = 40;

    let rpc = get_rpc_client();
    let mint_str = mint.to_string();

    // A quiet pool's last forty transactions can be all adds and removes with
    // no trader left holding anything, so the scan pages backwards rather than
    // giving up on one window.
    let mut signatures = Vec::new();
    let mut before = None;
    for _ in 0..PAGES_TO_SCAN {
        let Ok(page) = rpc
            .get_signatures_for_address(pool, Some(SIGNATURES_PER_PAGE), before.as_ref(), None)
            .await
        else {
            break;
        };
        let Some(last) = page.last() else { break };
        before = Some(last.signature);
        signatures.extend(page);
    }

    let mut candidates: Vec<Pubkey> = Vec::new();
    for info in signatures {
        if info.err.is_some() {
            continue;
        }
        let Ok(details) = rpc
            .get_transaction_details_with_commitment(
                &info.signature.to_string(),
                CommitmentLevel::Confirmed,
            )
            .await
        else {
            continue;
        };
        let Some(meta) = details.meta.as_ref() else {
            continue;
        };
        for balances in [&meta.post_token_balances, &meta.pre_token_balances] {
            let Some(balances) = balances.as_ref() else {
                continue;
            };
            for balance in balances {
                if balance.mint != mint_str {
                    continue;
                }
                let Some(owner) = balance
                    .owner
                    .as_deref()
                    .and_then(|o| Pubkey::from_str(o).ok())
                else {
                    continue;
                };
                if owner == *pool || candidates.contains(&owner) {
                    continue;
                }
                candidates.push(owner);
            }
        }
        if candidates.len() >= CANDIDATES_WANTED {
            break;
        }
    }
    if candidates.is_empty() {
        return Vec::new();
    }

    // What a past transaction says a wallet held is not what it holds now, so
    // the balance is re-read; and the read is against the ATA specifically,
    // because an owner holding the token somewhere else is of no use to a plan
    // that spends from the associated account.
    let atas: Vec<Pubkey> = candidates
        .iter()
        .map(|owner| get_associated_token_address_with_program_id(owner, mint, token_program))
        .collect();
    let Ok(token_accounts) = rpc.get_multiple_accounts(&atas).await else {
        return Vec::new();
    };
    let mut funded: Vec<(Pubkey, u64)> = Vec::new();
    for (owner, account) in candidates.iter().zip(token_accounts.into_iter()) {
        let Some(account) = account else { continue };
        let amount = account
            .data
            .get(64..72)
            .and_then(|b| <[u8; 8]>::try_from(b).ok())
            .map(u64::from_le_bytes)
            .unwrap_or(0);
        if amount > 0 {
            funded.push((*owner, amount));
        }
    }
    if funded.is_empty() {
        return Vec::new();
    }
    // Richest first: a bigger balance is more likely to cover the size the buy
    // leg would have returned, and the caller caps the sell at what it finds.
    funded.sort_by(|a, b| b.1.cmp(&a.1));

    // The owner must be an ordinary wallet with enough SOL to be a plausible
    // fee payer: a program-owned owner is a PDA, and a PDA holding the balance
    // is almost always the pool or a vault.
    let owners: Vec<Pubkey> = funded.iter().map(|(owner, _)| *owner).collect();
    let Ok(owner_accounts) = rpc.get_multiple_accounts(&owners).await else {
        return Vec::new();
    };
    funded
        .into_iter()
        .zip(owner_accounts.into_iter())
        .filter(|(_, account)| {
            account.as_ref().is_some_and(|a| {
                a.owner == screenerbot::chains::solana::solana_sdk::system_program::id()
                    && a.lamports > 10_000_000
            })
        })
        .map(|(holder, _)| holder)
        .collect()
}

/// Every holder of `mint`, straight off the token programme, richest first.
///
/// The fallback for a quiet pool whose last hundred transactions leave nobody
/// still holding a usable balance. `getProgramAccounts` with a mint memcmp is
/// exact where it is allowed at all; a public endpoint refuses it for a mint
/// with hundreds of thousands of holders, which is precisely the case where the
/// recent-trader scan already works.
async fn holders_from_the_token_programme(
    mint: &Pubkey,
    token_program: &Pubkey,
) -> Vec<(Pubkey, u64)> {
    use screenerbot::chains::solana::rpc::{get_rpc_client, RpcClientMethods, RpcFilterType};
    use screenerbot::chains::solana::spl_associated_token_account::get_associated_token_address_with_program_id;

    const TOKEN_ACCOUNT_SIZE: u64 = 165;

    let rpc = get_rpc_client();
    let Ok(accounts) = rpc
        .get_program_accounts(
            token_program,
            Some(vec![
                RpcFilterType::DataSize(TOKEN_ACCOUNT_SIZE),
                RpcFilterType::Memcmp {
                    offset: 0,
                    bytes: mint.to_string(),
                },
            ]),
        )
        .await
    else {
        return Vec::new();
    };

    let mut holders: Vec<(Pubkey, u64)> = accounts
        .into_iter()
        .filter_map(|(address, account)| {
            let owner = account
                .data
                .get(32..64)
                .and_then(|b| <[u8; 32]>::try_from(b).ok())
                .map(Pubkey::new_from_array)?;
            let amount = account
                .data
                .get(64..72)
                .and_then(|b| <[u8; 8]>::try_from(b).ok())
                .map(u64::from_le_bytes)?;
            if amount == 0
                || get_associated_token_address_with_program_id(&owner, mint, token_program)
                    != address
            {
                return None;
            }
            Some((owner, amount))
        })
        .collect();
    holders.sort_by(|a, b| b.1.cmp(&a.1));
    holders.truncate(24);
    if holders.is_empty() {
        return Vec::new();
    }

    let owners: Vec<Pubkey> = holders.iter().map(|(owner, _)| *owner).collect();
    let Ok(owner_accounts) = rpc.get_multiple_accounts(&owners).await else {
        return Vec::new();
    };
    holders
        .into_iter()
        .zip(owner_accounts.into_iter())
        .filter(|(_, account)| {
            account.as_ref().is_some_and(|a| {
                a.owner == screenerbot::chains::solana::solana_sdk::system_program::id()
                    && a.lamports > 10_000_000
            })
        })
        .map(|(holder, _)| holder)
        .collect()
}

/// Simulate a TOKEN -> SOL sell with `min_out` set to the quote exactly.
///
/// The size is the amount a `MINIMUM_SWAP_LAMPORTS` buy in the same pool would
/// have returned, so the sell is the mirror of the buy the other tests already
/// prove, and it stays small enough that price impact is not the thing under
/// test.
async fn simulate_sell_with_no_slippage_room(pool: &str, label: &str) {
    let pool_key = Pubkey::from_str(pool).expect("pool constant must be a pubkey");
    let market = direct::load_market(&pool_key)
        .await
        .unwrap_or_else(|e| panic!("{label}: pool must decode: {e}"));

    let (mint_a, mint_b) = market.mints();
    let (sol, token) = if mint_a.to_string() == WSOL {
        (mint_a, mint_b)
    } else if mint_b.to_string() == WSOL {
        (mint_b, mint_a)
    } else {
        panic!(
            "{label}: {pool} trades {mint_a}/{mint_b}, neither of which is WSOL -- this helper \
             sells a token FOR SOL and needs a pool with a SOL leg"
        );
    };

    let sizing = DirectSwapIntent {
        pool: pool_key,
        owner: simulation_owner(),
        input_mint: sol,
        output_mint: token,
        amount_in: MINIMUM_SWAP_LAMPORTS,
        slippage_bps: 0,
    };
    let sized = direct::quote_with_market(&sizing, market.as_ref())
        .unwrap_or_else(|e| panic!("{label}: the sizing buy would not quote: {e}"));

    let token_program = market
        .token_program(&token)
        .unwrap_or_else(|| panic!("{label}: the pool must report its own token programme"));
    let mut holders = holder_with_associated_account(&pool_key, &token, &token_program).await;
    // A quiet pool can leave every recent trader holding dust. Fall back to the
    // token programme's own account list before giving up on the venue.
    if holders
        .first()
        .is_none_or(|(_, held)| *held * 100 < sized.expected_net_out)
    {
        let from_programme = holders_from_the_token_programme(&token, &token_program).await;
        if from_programme
            .first()
            .is_some_and(|(_, held)| Some(*held) > holders.first().map(|(_, h)| *h))
        {
            holders = from_programme;
        }
    }
    assert!(
        !holders.is_empty(),
        "{label}: no holder of {token} could be found for {pool}, so the sell leg cannot be \
         simulated against a real balance"
    );

    // Mirror the buy where a holder can cover it; otherwise sell what they
    // actually have. Either size is a valid exactness proof -- `min_out` is
    // enforced on whatever amount goes in -- and capping at the balance is what
    // keeps the test alive as a pool's traders come and go. A holder whose dust
    // is too small for the pool to price at all is skipped rather than treated
    // as a venue failure.
    let mut refusals: Vec<String> = Vec::new();
    let mut chosen = None;
    for (owner, held) in &holders {
        let amount_in = sized.expected_net_out.min(*held);
        if amount_in == 0 {
            continue;
        }
        let sell = DirectSwapIntent {
            pool: pool_key,
            owner: *owner,
            input_mint: token,
            output_mint: sol,
            amount_in,
            slippage_bps: 0,
        };
        match direct::quote(&sell).await {
            Ok((quote, market)) => {
                eprintln!(
                    "{label}: selling {amount_in} of {token} as {owner} (holds {held}, a \
                     mirrored buy would have returned {})",
                    sized.expected_net_out
                );
                chosen = Some((*owner, sell, quote, market));
                break;
            }
            Err(e) => refusals.push(format!("{owner} holding {held}: {e}")),
        }
    }
    let Some((owner, sell, quote, market)) = chosen else {
        panic!(
            "{label}: no holder's balance could be quoted as a sell:\n  {}",
            refusals.join("\n  ")
        );
    };
    assert_eq!(
        quote.min_out, quote.expected_out,
        "{label}: zero slippage must leave the floor at the estimate"
    );
    assert_eq!(
        quote.fee.side,
        FeeSide::Output,
        "{label}: a sell for SOL pays the platform fee on the SOL it receives"
    );

    let plan = direct::build_plan(&sell, market.as_ref(), &quote)
        .unwrap_or_else(|e| panic!("{label}: the sell plan would not build: {e}"));
    assert_fee_is_collected(&quote.fee, &plan);
    if let Some(units) = direct::simulate_plan(&plan, &owner)
        .await
        .ok()
        .and_then(|outcome| outcome.units_consumed)
    {
        assert!(
            units < plan.venue_compute_units as u64,
            "{label}: the venue's {} CU estimate must cover the {units} CU the SELL used",
            plan.venue_compute_units
        );
    }

    let failure = attempt_zero_slippage(&sell, &owner).await;
    assert!(
        failure.is_none(),
        "{label}: the venue over-stated its SELL output -- {} raw units in, refused on every \
         attempt against fresh state:\n{}",
        sell.amount_in,
        failure.unwrap_or_default()
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "live network"]
async fn an_amm_v4_sell_is_exact_to_the_raw_unit() {
    let _guard = common::isolated_env();
    simulate_sell_with_no_slippage_room(AMM_V4_SOL_USDC, "amm v4").await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "live network"]
async fn a_cpmm_sell_is_exact_to_the_raw_unit() {
    let _guard = common::isolated_env();
    simulate_sell_with_no_slippage_room(CPMM_POOL, "cpmm").await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "live network"]
async fn a_clmm_sell_is_exact_to_the_raw_unit() {
    let _guard = common::isolated_env();
    simulate_sell_with_no_slippage_room(CLMM_SOL_USDC, "clmm").await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "live network"]
async fn an_orca_whirlpool_sell_is_exact_to_the_raw_unit() {
    let _guard = common::isolated_env();
    simulate_sell_with_no_slippage_room(ORCA_SOL_USDC, "orca whirlpool").await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "live network"]
async fn a_meteora_dlmm_sell_is_exact_to_the_raw_unit() {
    let _guard = common::isolated_env();
    simulate_sell_with_no_slippage_room(DLMM_SOL_USDC, "meteora dlmm").await;
}

/// DAMM v2 has never had a real sell execute against it -- the venue is green on
/// the buy leg only. This is the first proof its sell direction is right.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "live network"]
async fn a_damm_v2_sell_is_exact_to_the_raw_unit() {
    let _guard = common::isolated_env();
    // Both DAMM v2 fee modes charge a SELL on the output leg -- `OnlyB` because
    // the token being spent is token A, `BothToken` always -- so the deeper of
    // the two pools covers the branch, and it is the one with real holders.
    simulate_sell_with_no_slippage_room(DAMM_V2_BOTH_TOKEN_POOL, "damm v2 (BothToken)").await;
}

/// The other venue that has never executed a sell.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "live network"]
async fn a_pump_amm_sell_is_exact_to_the_raw_unit() {
    let _guard = common::isolated_env();
    simulate_sell_with_no_slippage_room(PUMP_AMM_POOL, "pump amm").await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "live network"]
async fn a_pump_legacy_sell_is_exact_to_the_raw_unit() {
    let _guard = common::isolated_env();
    simulate_sell_with_no_slippage_room(PUMP_LEGACY_SELL_POOL, "pump legacy").await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "live network"]
async fn a_moonit_sell_is_exact_to_the_raw_unit() {
    let _guard = common::isolated_env();
    simulate_sell_with_no_slippage_room(MOONIT_POOL, "moonit").await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "live network"]
async fn a_meteora_dbc_sell_is_exact_to_the_raw_unit() {
    let _guard = common::isolated_env();
    simulate_sell_with_no_slippage_room(DBC_QUOTE_FEE_POOL, "meteora dbc").await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "live network"]
async fn a_fluxbeam_sell_is_exact_to_the_raw_unit() {
    let _guard = common::isolated_env();
    simulate_sell_with_no_slippage_room(FLUXBEAM_SELL_POOL, "fluxbeam (reverse order)").await;
}

// ============================================================================
// THE TWO VENUES THAT NEVER GOT A ZERO-SLIPPAGE BUY
// ============================================================================
//
// AMM v4 and CPMM were built before `simulate_with_no_slippage_room` existed, so
// nine of eleven venues carry the exactness proof and the two oldest do not.
// AMM v4 in particular quotes off `vault - need_take_pnl` and ignores whatever
// its OpenBook `open_orders` account holds, which the programme itself adds in;
// only a zero-slippage simulation can say whether that difference is real today.

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "live network"]
async fn an_amm_v4_quote_is_exact_to_the_raw_unit() {
    let _guard = common::isolated_env();
    simulate_with_no_slippage_room(AMM_V4_SOL_USDC, WSOL, USDC, MINIMUM_SWAP_LAMPORTS).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "live network"]
async fn a_cpmm_quote_is_exact_to_the_raw_unit() {
    let _guard = common::isolated_env();
    let token = paired_token(CPMM_POOL).await;
    simulate_with_no_slippage_room(CPMM_POOL, WSOL, &token, MINIMUM_SWAP_LAMPORTS).await;
}
