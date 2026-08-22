//! Money-path swap tests — offline, deterministic, no network, no keys, no RPC.
//!
//! Scope: the swap build/execute path spends real money, and almost none of it had
//! tests before this file. This suite covers what is reachable from an external
//! integration-test crate (the library's `pub` API only — see `tests/common/mod.rs`'s
//! own header on why private pure logic is tested co-located instead).
//!
//! NOT duplicated here (see file headers): `tests/swaps_chain_routing.rs` (router
//! selection/fallback/chain scoping), `tests/swap_resilience.rs`, `tests/trader_*.rs`,
//! and `src/chains/solana/swaps/routers/gmgn.rs`'s own quote-decode unit tests.
//! Item 4 (instruction structure) is covered co-located in
//! `src/chains/solana/swaps/programs/raydium_clmm.rs` (CPMM already had its own).

use async_trait::async_trait;
use screenerbot::chains::ChainId;
use screenerbot::swaps::operations::get_best_quote;
use screenerbot::swaps::registry::set_router_factory;
use screenerbot::swaps::router::SwapRouter;
use screenerbot::swaps::types::{Quote, QuoteRequest, SwapMode, SwapResult};
use screenerbot::tokens::Token;
use screenerbot::Result;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

// ============================================================================
// ITEM 2 — slippage / minimum-out math
// ============================================================================
//
// There is no standalone, pure, PUBLIC "compute minimum out" function in the
// money path: Jupiter and GMGN both outsource min-out enforcement to their own
// APIs (they just forward `slippageBps`), and the only place that computes a
// local minimum output is the direct Raydium builders
// (`src/chains/solana/swaps/programs/raydium_cpmm.rs:164-166` and
// `raydium_clmm.rs`'s equivalent), which are private `async fn`s already
// exercised co-located for instruction STRUCTURE. This helper mirrors that
// documented formula exactly so its numeric properties — the ones a wrong
// swap would violate — are pinned as a regression spec:
//   minimum_output = expected_output * (1 - slippage_bps / 10000)
// If that formula ever changes in raydium_cpmm.rs without a matching change
// here, this test does NOT catch it (it does not call the private code) —
// see the report for this limitation.
fn minimum_out(expected_output: u64, slippage_bps: u16) -> u64 {
    let out = expected_output as u128;
    let bps = slippage_bps as u128;
    (out * (10_000 - bps) / 10_000) as u64
}

#[test]
fn minimum_out_at_zero_bps_equals_expected_output() {
    assert_eq!(minimum_out(1_000_000, 0), 1_000_000);
}

#[test]
fn minimum_out_at_a_normal_slippage_setting_shaves_the_expected_percentage() {
    // 100 bps = 1%
    assert_eq!(minimum_out(1_000_000, 100), 990_000);
}

#[test]
fn minimum_out_at_a_large_slippage_setting_still_computes_without_overflow_or_panic() {
    // 5000 bps = 50%, the ceiling enforced in SwapBuilder::validate_request.
    assert_eq!(minimum_out(1_000_000, 5_000), 500_000);
    // Even a (deliberately invalid) 100% slippage must not panic or underflow.
    assert_eq!(minimum_out(1_000_000, 10_000), 0);
}

#[test]
fn minimum_out_rounding_is_always_protective_never_rounds_up() {
    // 1 output unit at 1 bps: true minimum is 0.9999 units. Rounding UP to 1
    // would let a trade through at zero effective protection; the integer
    // division here truncates down, so this must be 0, never 1.
    assert_eq!(minimum_out(1, 1), 0);

    // A less trivial case: 333 units at 33 bps must never exceed the exact
    // (non-integer) minimum, i.e. floor(333 * 9967 / 10000) = floor(331.9011).
    let exact = 333.0 * (1.0 - 33.0 / 10_000.0);
    let computed = minimum_out(333, 33);
    assert!(
        (computed as f64) <= exact,
        "computed minimum {computed} must not exceed the exact minimum {exact}"
    );
}

// ============================================================================
// ITEM 2 (continued) — a zero or inverted quote must never reach a trade.
//
// `get_best_quote` (src/swaps/operations.rs:94-97) selects
// `quotes.into_iter().max_by_key(|q| q.output_amount)` with NO check that the
// winning quote's output_amount is nonzero, and NO check that the quote's
// input_mint/output_mint match the request that was sent. Both properties are
// exercised below using stub routers registered through the same process-wide
// `set_router_factory` seam `tests/swaps_chain_routing.rs` documents (own test
// binary, installed exactly once).
// ============================================================================

#[derive(Clone, Copy, PartialEq, Eq)]
enum Scenario {
    /// The only router returns output_amount == 0.
    ZeroOutput,
    /// The only router returns a quote whose mints are swapped relative to
    /// the request (input/output inverted).
    InvertedMint,
    /// Two valid routers; the higher-output one must win (positive control).
    TwoValidQuotes,
}

static SCENARIO: AtomicU8 = AtomicU8::new(0);
/// Serializes tests that flip the shared `SCENARIO` — this binary may run the
/// suite's tests on multiple threads (plain `cargo test`), and the scenario is
/// process-wide state read by the stub routers' `get_quote`.
static SCENARIO_MUTEX: OnceLock<Mutex<()>> = OnceLock::new();

fn scenario_guard() -> std::sync::MutexGuard<'static, ()> {
    SCENARIO_MUTEX
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn set_scenario(s: Scenario) {
    SCENARIO.store(s as u8, Ordering::SeqCst);
}

fn current_scenario() -> Scenario {
    match SCENARIO.load(Ordering::SeqCst) {
        0 => Scenario::ZeroOutput,
        1 => Scenario::InvertedMint,
        _ => Scenario::TwoValidQuotes,
    }
}

struct ScenarioRouter {
    id: &'static str,
    priority: u8,
}

#[async_trait]
impl SwapRouter for ScenarioRouter {
    fn id(&self) -> &'static str {
        self.id
    }
    fn name(&self) -> &'static str {
        self.id
    }
    fn is_enabled(&self) -> bool {
        true
    }
    fn priority(&self) -> u8 {
        self.priority
    }
    fn chain(&self) -> ChainId {
        ChainId::Solana
    }
    async fn get_quote(&self, request: &QuoteRequest) -> Result<Quote> {
        self.accept_own_chain(request)?;
        let base = Quote {
            chain: request.chain,
            router_id: self.id.to_owned(),
            router_name: self.id.to_owned(),
            input_mint: request.input_mint.clone(),
            output_mint: request.output_mint.clone(),
            input_amount: request.input_amount,
            output_amount: 1,
            price_impact_pct: 0.0,
            fee_lamports: 0,
            slippage_bps: 100,
            route_plan: self.id.to_owned(),
            swap_mode: request.swap_mode,
            wallet_address: request.wallet_address.clone(),
            execution_data: self.id.as_bytes().to_vec(),
        };
        match (current_scenario(), self.id) {
            (Scenario::ZeroOutput, "only") => Ok(Quote {
                output_amount: 0,
                ..base
            }),
            (Scenario::InvertedMint, "only") => Ok(Quote {
                // Swapped relative to the REQUEST — this is not the pair that
                // was quoted, and must not be accepted as "the" quote.
                input_mint: request.output_mint.clone(),
                output_mint: request.input_mint.clone(),
                output_amount: 999_999,
                ..base
            }),
            (Scenario::TwoValidQuotes, "low") => Ok(Quote {
                output_amount: 100,
                ..base
            }),
            (Scenario::TwoValidQuotes, "high") => Ok(Quote {
                output_amount: 200,
                ..base
            }),
            _ => Err(screenerbot::Error::api_error("router not part of scenario")),
        }
    }
    async fn execute_swap(&self, _token: &Token, _quote: &Quote) -> Result<SwapResult> {
        Err(screenerbot::Error::api_error("stub router never executes"))
    }
}

fn build_factory_only() -> Vec<Arc<dyn SwapRouter>> {
    vec![Arc::new(ScenarioRouter {
        id: "only",
        priority: 0,
    })]
}

fn build_factory_two() -> Vec<Arc<dyn SwapRouter>> {
    vec![
        Arc::new(ScenarioRouter {
            id: "low",
            priority: 0,
        }),
        Arc::new(ScenarioRouter {
            id: "high",
            priority: 1,
        }),
    ]
}

/// This process installs the "only" + scenario-driven two-router set combined:
/// both `build_factory_only` and `build_factory_two` routers are registered
/// together (the factory can only be set ONCE per process — see
/// `tests/swaps_chain_routing.rs`'s module doc), and each scenario's match arm
/// above only answers for the router IDs it cares about; every other router
/// for that scenario returns an `Err` so it never contributes a quote.
fn build_factory_combined() -> Vec<Arc<dyn SwapRouter>> {
    let mut routers = build_factory_only();
    routers.extend(build_factory_two());
    routers
}

fn request() -> QuoteRequest {
    QuoteRequest {
        chain: ChainId::Solana,
        input_mint: "So11111111111111111111111111111111111111112".to_owned(),
        output_mint: "TokenMint111111111111111111111111111111111".to_owned(),
        input_amount: 1_000_000,
        wallet_address: "Wallet1111111111111111111111111111111111111".to_owned(),
        slippage_pct: 1.0,
        swap_mode: SwapMode::ExactIn,
        exclude_dexes: None,
    }
}

/// FINDING: `get_best_quote` has no guard against a router handing back
/// output_amount == 0. A zero-output quote is not a real trade — accepting it
/// as "the best quote" would carry a bogus quote into the trade path. This
/// test asserts the SAFE behavior (reject) and is left FAILING against
/// today's code, which currently returns `Ok` — see
/// `src/swaps/operations.rs:94-97` (`max_by_key` with no output_amount check).
#[tokio::test]
async fn get_best_quote_rejects_a_zero_output_amount_quote() {
    let _guard = scenario_guard();
    set_router_factory(build_factory_combined);
    set_scenario(Scenario::ZeroOutput);

    let result = get_best_quote(request()).await;
    assert!(
        result.is_err(),
        "a zero-output quote must never be selected as the best quote — \
         FINDING: get_best_quote (src/swaps/operations.rs:94-97) has no such guard, got: {:?}",
        result.map(|q| q.output_amount)
    );
}

/// FINDING: `get_best_quote` never checks that the winning quote's
/// input_mint/output_mint match the REQUEST's mints. A router that returns a
/// quote for the wrong pair (accidentally or maliciously) is accepted exactly
/// like a correct one as long as its output_amount looks attractive. Left
/// FAILING against today's code — see `src/swaps/operations.rs:94-97`.
#[tokio::test]
async fn get_best_quote_rejects_a_quote_with_inverted_mints() {
    let _guard = scenario_guard();
    set_router_factory(build_factory_combined);
    set_scenario(Scenario::InvertedMint);

    let req = request();
    let result = get_best_quote(req.clone()).await;
    match result {
        Err(_) => {} // safe behavior
        Ok(quote) => {
            assert!(
                quote.input_mint == req.input_mint && quote.output_mint == req.output_mint,
                "FINDING: get_best_quote (src/swaps/operations.rs:94-97) accepted a quote \
                 for the WRONG mint pair (input={}, output={}) as the best quote for a \
                 request of input={}, output={}",
                quote.input_mint,
                quote.output_mint,
                req.input_mint,
                req.output_mint
            );
        }
    }
}

/// Positive control: among two structurally valid quotes, the higher output
/// wins — the ordinary, correct case `max_by_key` is meant to serve.
#[tokio::test]
async fn get_best_quote_picks_the_higher_output_among_valid_quotes() {
    let _guard = scenario_guard();
    set_router_factory(build_factory_combined);
    set_scenario(Scenario::TwoValidQuotes);

    let quote = get_best_quote(request())
        .await
        .expect("two valid quotes must produce a winner");
    assert_eq!(quote.router_id, "high");
    assert_eq!(quote.output_amount, 200);
}

// ============================================================================
// ITEM 1 — quote decoding (Jupiter, GMGN)
// ============================================================================
//
// SKIPPED: Jupiter's `JupiterQuoteResponse` and GMGN's `GMGNApiResponse` /
// `SwapQuote` are declared in modules that are NOT `pub` from the crate root
// (`mod jupiter;` / `mod gmgn;` in
// `src/chains/solana/swaps/routers/mod.rs`), so they are unreachable from this
// external integration-test crate — only the library's `pub` surface is
// visible here. Widening that visibility (or the router's hardcoded API base
// URL, which also blocks driving `get_quote()` against a local mock server)
// would be a production edit outside this stretch's scope. GMGN's own decode
// edge cases (well-formed / error / malformed) are already covered by
// `src/chains/solana/swaps/routers/gmgn.rs`'s co-located unit tests
// (`success_response_decodes_quote_and_raw_transaction`,
// `error_response_decodes_with_no_data`, `malformed_response_is_rejected`).
// Jupiter's own co-located tests cover well-formed decode, a bad
// `priceImpactPct` falling back to 0.0, and malformed-JSON rejection, but do
// NOT cover an EMPTY body or the real "no route" provider error shape — both
// gaps are noted here rather than chased with a production visibility change.

// ============================================================================
// ITEM 3 — referral fee account selection
// ============================================================================
//
// SKIPPED: `referral_fee_account` (src/chains/solana/swaps/routers/jupiter.rs:289)
// is `pub(crate)`, which restricts it to the library crate itself — an
// external integration-test crate cannot see it regardless of module path.
// Widening it to `pub` would be a production edit outside this stretch's
// scope. The underlying logic is already fully covered in-crate by
// `jupiter.rs`'s own `referral_account_prefers_the_output_mint_then_falls_back_to_input`
// test, which checks wSOL and USDC on BOTH the input and output side. The
// brief's "Token-2022 mint takes the V2 instruction path" case does not exist
// as a conditional in current code: `instruction_version` is unconditionally
// set to `Some("V2")` for every quote request regardless of mint type
// (jupiter.rs:420-429, comment at jupiter.rs:91-95), so there is no branch to
// exercise.
