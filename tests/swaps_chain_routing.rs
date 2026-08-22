//! Chain-aware swap routing regression suite.
//!
//! Own test binary (own process) because `crate::swaps::registry::ROUTER_FACTORY`
//! is a process-wide `OnceLock<fn() -> Vec<Arc<dyn SwapRouter>>>` that can only be
//! set once per process — see `src/swaps/registry.rs::set_router_factory`. This
//! file installs its stub routers into that global exactly once, the same way
//! `tests/wallet_readiness.rs` documents owning its own process-global statics.
//!
//! NOTE ON CHAIN COVERAGE: `crate::chains::ChainId` currently has a single
//! variant (`Solana`) — multi-chain support is prepared but no second chain has
//! landed yet. That means a genuine "router for a different chain" or "request
//! for a different chain" cannot be constructed with real, distinct `ChainId`
//! values today: any two `ChainId` values compare equal. The tests below cover
//! everything that today's single-variant enum allows to be exercised for real
//! (id-based refusal, priority-based selection, disabled-router exclusion,
//! fallback ordering, and the no-chain delegates matching the `active_chain()`
//! forms). The chain-equality guards themselves (`accept_own_chain`, the
//! `r.chain() == chain` filters in the registry) are exercised on their MATCH
//! path only; their MISMATCH path is straight-line code that cannot be driven
//! to the false branch until a second `ChainId` variant exists.

use async_trait::async_trait;
use screenerbot::chains::{active_chain, ChainId};
use screenerbot::swaps::registry::{set_router_factory, try_get_registry, RouterRegistry};
use screenerbot::swaps::router::SwapRouter;
use screenerbot::swaps::types::{Quote, QuoteRequest, SwapMode, SwapResult};
use screenerbot::tokens::Token;
use screenerbot::Result;
use std::sync::Arc;

// ============================================================================
// STUB ROUTER
// ============================================================================

struct StubRouter {
    id: &'static str,
    name: &'static str,
    enabled: bool,
    priority: u8,
    chain: ChainId,
}

impl StubRouter {
    fn new(id: &'static str, priority: u8) -> Self {
        Self {
            id,
            name: id,
            enabled: true,
            priority,
            chain: ChainId::Solana,
        }
    }

    fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }
}

#[async_trait]
impl SwapRouter for StubRouter {
    fn id(&self) -> &'static str {
        self.id
    }
    fn name(&self) -> &'static str {
        self.name
    }
    fn is_enabled(&self) -> bool {
        self.enabled
    }
    fn priority(&self) -> u8 {
        self.priority
    }
    fn chain(&self) -> ChainId {
        self.chain
    }
    async fn get_quote(&self, request: &QuoteRequest) -> Result<Quote> {
        self.accept_own_chain(request)?;
        Ok(quote_from(self, request))
    }
    async fn execute_swap(&self, _token: &Token, _quote: &Quote) -> Result<SwapResult> {
        Err(screenerbot::Error::api_error("stub router never executes"))
    }
}

fn quote_from(router: &StubRouter, request: &QuoteRequest) -> Quote {
    Quote {
        chain: request.chain,
        router_id: router.id.to_owned(),
        router_name: router.name.to_owned(),
        input_mint: request.input_mint.clone(),
        output_mint: request.output_mint.clone(),
        input_amount: request.input_amount,
        output_amount: 1,
        price_impact_pct: 0.0,
        fee_lamports: 0,
        slippage_bps: 100,
        route_plan: router.id.to_owned(),
        swap_mode: request.swap_mode,
        wallet_address: request.wallet_address.clone(),
        execution_data: router.id.as_bytes().to_vec(),
    }
}

fn request(chain: ChainId) -> QuoteRequest {
    QuoteRequest {
        chain,
        input_mint: "So11111111111111111111111111111111111111112".to_owned(),
        output_mint: "TokenMint111111111111111111111111111111111".to_owned(),
        input_amount: 1_000_000,
        wallet_address: "Wallet1111111111111111111111111111111111111".to_owned(),
        slippage_pct: 1.0,
        swap_mode: SwapMode::ExactIn,
        exclude_dexes: None,
    }
}

fn arc(router: StubRouter) -> Arc<dyn SwapRouter> {
    Arc::new(router)
}

// ============================================================================
// 1. A quote produced by router A is refused when handed to router B.
// ============================================================================

#[test]
fn quote_from_one_router_is_refused_by_another_naming_both() {
    let jupiter = StubRouter::new("jupiter", 0);
    let gmgn = StubRouter::new("gmgn", 1);

    let req = request(ChainId::Solana);
    let quote = futures::executor::block_on(jupiter.get_quote(&req)).expect("jupiter quotes");

    let err = gmgn
        .accept_own_quote(&quote)
        .expect_err("gmgn must refuse a jupiter quote");
    let message = err.to_string();
    assert!(
        message.contains("jupiter") && message.contains("gmgn"),
        "error must name both routers: {message}"
    );
}

// ============================================================================
// 2. accept_own_chain: match path (mismatch path needs a second ChainId
//    variant that does not exist yet — see the module doc comment).
// ============================================================================

#[test]
fn accept_own_chain_accepts_a_request_for_its_own_chain() {
    let jupiter = StubRouter::new("jupiter", 0);
    let req = request(ChainId::Solana);
    assert_eq!(jupiter.chain(), ChainId::Solana);
    jupiter
        .accept_own_chain(&req)
        .expect("request chain matches router chain");
}

// ============================================================================
// 3. get_primary_router_for: lowest-priority ENABLED router wins, not the
//    first registered; None when every router for the chain is disabled.
// ============================================================================

#[test]
fn primary_router_for_chain_is_lowest_priority_enabled_router() {
    let registry = RouterRegistry::new(vec![
        arc(StubRouter::new("raydium", 2)),
        arc(StubRouter::new("gmgn", 1)),
        arc(StubRouter::new("jupiter", 0)),
    ]);

    let primary = registry
        .get_primary_router_for(ChainId::Solana)
        .expect("an enabled router exists");
    assert_eq!(primary.id(), "jupiter");
}

#[test]
fn primary_router_for_chain_is_none_when_all_routers_for_that_chain_are_disabled() {
    let registry = RouterRegistry::new(vec![
        arc(StubRouter::new("jupiter", 0).disabled()),
        arc(StubRouter::new("gmgn", 1).disabled()),
    ]);

    assert!(registry.get_primary_router_for(ChainId::Solana).is_none());
    assert!(!registry.has_enabled_routers_for(ChainId::Solana));
}

// ============================================================================
// 4. Foreign-chain exclusion. See module doc comment: cannot be exercised
//    with a real second `ChainId` value today. Documented as a gap, not
//    faked with a fabricated variant.
// ============================================================================

// ============================================================================
// 5. get_fallback_chain_for excludes the failed router, preserves priority
//    order, and stays within the requested chain.
// ============================================================================

#[test]
fn fallback_chain_excludes_failed_router_and_is_priority_ordered() {
    let registry = RouterRegistry::new(vec![
        arc(StubRouter::new("jupiter", 0)),
        arc(StubRouter::new("raydium", 2)),
        arc(StubRouter::new("gmgn", 1)),
        arc(StubRouter::new("disabled_one", 3).disabled()),
    ]);

    let chain: Vec<_> = registry
        .get_fallback_chain_for(ChainId::Solana, "jupiter")
        .iter()
        .map(|r| r.id())
        .collect();

    assert_eq!(
        chain,
        vec!["gmgn", "raydium"],
        "fallback must exclude the failed router, exclude disabled routers, and be priority-ordered"
    );
}

// ============================================================================
// 6. No-chain delegates return exactly what the active_chain() forms return.
// ============================================================================

fn stub_factory() -> Vec<Arc<dyn SwapRouter>> {
    vec![
        arc(StubRouter::new("jupiter", 0)),
        arc(StubRouter::new("gmgn", 1)),
        arc(StubRouter::new("raydium", 2).disabled()),
    ]
}

#[test]
fn no_chain_delegates_match_the_active_chain_forms() {
    set_router_factory(stub_factory);
    let registry = try_get_registry().expect("factory installed above");

    let enabled_ids: Vec<_> = registry.enabled_routers().iter().map(|r| r.id()).collect();
    let enabled_for_ids: Vec<_> = registry
        .enabled_routers_for(active_chain())
        .iter()
        .map(|r| r.id())
        .collect();
    assert_eq!(enabled_ids, enabled_for_ids);

    assert_eq!(
        registry.get_primary_router().map(|r| r.id()),
        registry
            .get_primary_router_for(active_chain())
            .map(|r| r.id())
    );

    let fallback_ids: Vec<_> = registry
        .get_fallback_chain("jupiter")
        .iter()
        .map(|r| r.id())
        .collect();
    let fallback_for_ids: Vec<_> = registry
        .get_fallback_chain_for(active_chain(), "jupiter")
        .iter()
        .map(|r| r.id())
        .collect();
    assert_eq!(fallback_ids, fallback_for_ids);

    assert_eq!(
        registry.has_enabled_routers(),
        registry.has_enabled_routers_for(active_chain())
    );
}
