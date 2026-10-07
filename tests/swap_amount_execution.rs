// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Caller amount ranges on shared swap execution: an out-of-range quote never executes, an out-of-range fallback quote is skipped, and a completed out-of-range result keeps its signature for verification.

mod common;

use async_trait::async_trait;
use screenerbot::chains::{ChainId, RawAmount};
use screenerbot::errors::{ErrorClass, Severity};
use screenerbot::swaps::{
    execute_swap_with_fallback, unconfirmed_swap_signature, NotSubmittedReason, Quote,
    QuoteRequest, RouterRegistry, SwapAmountLimit, SwapExecutionError, SwapMode, SwapResult,
    SwapRouter,
};
use screenerbot::{Error, Result};
use std::sync::atomic::{AtomicU8, AtomicUsize, Ordering};
use std::sync::Arc;

static MODE: AtomicU8 = AtomicU8::new(0);
static PRIMARY_EXECS: AtomicUsize = AtomicUsize::new(0);
static WIDE_FALLBACK_EXECS: AtomicUsize = AtomicUsize::new(0);
static VALID_FALLBACK_EXECS: AtomicUsize = AtomicUsize::new(0);
const WIDE: RawAmount = RawAmount::new(u64::MAX as u128 + 1);
const SIGNATURE: &str =
    "5VERv8NMvzbJMEkV8xnrLkEaWRtSz9CosKDYjCJjBRnbJLgp8uirBgmQpjKhoR4tjF3ZpRzrFmBV6UjKdiSZkQUW";

struct Router {
    id: &'static str,
    priority: u8,
}

fn quote(id: &str, output: RawAmount) -> Quote {
    Quote {
        chain: ChainId::Solana,
        router_id: id.to_owned(),
        router_name: id.to_owned(),
        input_mint: "So11111111111111111111111111111111111111112".to_owned(),
        output_mint: "TokenMint111111111111111111111111111111111".to_owned(),
        input_amount: 1_000_000u64.into(),
        output_amount: output,
        minimum_output_amount: output,
        price_impact_pct: 0.0,
        platform_fee_lamports: None,
        estimated_network_fee_lamports: None,
        slippage_bps: 100,
        route_plan: id.to_owned(),
        swap_mode: SwapMode::ExactIn,
        wallet_address: "Wallet1111111111111111111111111111111111111".to_owned(),
        exclude_dexes: None,
        execution_data: Vec::new(),
    }
}

fn result(id: &str, output: RawAmount) -> SwapResult {
    SwapResult {
        success: true,
        router_id: id.to_owned(),
        router_name: id.to_owned(),
        transaction_signature: SIGNATURE.to_owned(),
        input_amount: 1_000_000u64.into(),
        output_amount: output,
        price_impact_pct: 0.0,
        fee_lamports: 0,
        execution_time_ms: 0,
        effective_price_sol: None,
    }
}

#[async_trait]
impl SwapRouter for Router {
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
    async fn get_quote(&self, request: &QuoteRequest) -> screenerbot::swaps::QuoteResult<Quote> {
        let output = if self.id == "wide" {
            WIDE
        } else {
            25u64.into()
        };
        let mut answer = quote(self.id, output);
        answer.input_amount = request.input_amount;
        answer.wallet_address = request.wallet_address.clone();
        answer.exclude_dexes = request.exclude_dexes.clone();
        Ok(answer)
    }
    async fn execute_swap(
        &self,
        _token: &screenerbot::tokens::Token,
        _quote: &Quote,
    ) -> Result<SwapResult> {
        match self.id {
            "primary" => {
                PRIMARY_EXECS.fetch_add(1, Ordering::SeqCst);
                match MODE.load(Ordering::SeqCst) {
                    1 => Err(Error::Swaps(SwapExecutionError::NotSubmitted {
                        router: self.id.to_owned(),
                        reason: NotSubmittedReason::RequestRejected {
                            detail: "stub request rejection".to_owned(),
                        },
                    })),
                    4 => Err(Error::network_error("stub transport failure")),
                    2 => Ok(result(self.id, WIDE)),
                    3 => Ok(result(self.id, WIDE)),
                    _ => Ok(result(self.id, 25u64.into())),
                }
            }
            "wide" => {
                WIDE_FALLBACK_EXECS.fetch_add(1, Ordering::SeqCst);
                Ok(result(self.id, WIDE))
            }
            _ => {
                VALID_FALLBACK_EXECS.fetch_add(1, Ordering::SeqCst);
                Ok(result(self.id, 25u64.into()))
            }
        }
    }
}

fn routers() -> Vec<Arc<dyn SwapRouter>> {
    vec![
        Arc::new(Router {
            id: "primary",
            priority: 0,
        }),
        Arc::new(Router {
            id: "wide",
            priority: 1,
        }),
        Arc::new(Router {
            id: "valid",
            priority: 2,
        }),
    ]
}

#[tokio::test]
async fn amount_policy_covers_primary_fallback_completion_and_unrestricted_execution() {
    screenerbot::swaps::registry::set_router_factory(routers);
    let _registry: &RouterRegistry = screenerbot::swaps::get_registry().unwrap();
    let token = common::filter_token("mint");

    MODE.store(0, Ordering::SeqCst);
    let error = execute_swap_with_fallback(&token, quote("primary", WIDE), SwapAmountLimit::U64)
        .await
        .expect_err("initial quote must be refused before execution");
    assert!(error.to_string().contains("u64 amount range"));
    assert_eq!(PRIMARY_EXECS.load(Ordering::SeqCst), 0);
    assert_eq!(WIDE_FALLBACK_EXECS.load(Ordering::SeqCst), 0);
    assert_eq!(VALID_FALLBACK_EXECS.load(Ordering::SeqCst), 0);

    MODE.store(1, Ordering::SeqCst);
    let filled =
        execute_swap_with_fallback(&token, quote("primary", 25u64.into()), SwapAmountLimit::U64)
            .await
            .expect("a provably unsent primary falls back to the valid router");
    assert_eq!(filled.router_id, "valid");
    assert_eq!(WIDE_FALLBACK_EXECS.load(Ordering::SeqCst), 0);
    assert_eq!(VALID_FALLBACK_EXECS.load(Ordering::SeqCst), 1);

    MODE.store(2, Ordering::SeqCst);
    let error =
        execute_swap_with_fallback(&token, quote("primary", 25u64.into()), SwapAmountLimit::U64)
            .await
            .expect_err("completed wide result must retain completion");
    match &error {
        Error::Swaps(SwapExecutionError::CompletedAmountOutOfRange {
            signature,
            input_amount,
            output_amount,
        }) => {
            assert_eq!(signature, SIGNATURE);
            assert_eq!(*input_amount, 1_000_000u64.into());
            assert_eq!(*output_amount, WIDE);
        }
        other => panic!("unexpected error: {other}"),
    }
    assert_eq!(
        unconfirmed_swap_signature(&error).as_deref(),
        Some(SIGNATURE)
    );
    assert_eq!(PRIMARY_EXECS.load(Ordering::SeqCst), 2);
    assert_eq!(WIDE_FALLBACK_EXECS.load(Ordering::SeqCst), 0);
    assert!(!error.is_retryable());
    assert_eq!(error.severity(), Severity::Critical);
    assert_eq!(error.http_status(), 500);
    assert_eq!(VALID_FALLBACK_EXECS.load(Ordering::SeqCst), 1);

    MODE.store(3, Ordering::SeqCst);
    let filled = execute_swap_with_fallback(
        &token,
        quote("primary", WIDE),
        SwapAmountLimit::Unrestricted,
    )
    .await
    .expect("unrestricted wide result must survive");
    assert_eq!(filled.output_amount, WIDE);

    // A failure that cannot prove nothing was sent may still land, so no
    // fallback router is asked to buy the same trade a second time.
    MODE.store(4, Ordering::SeqCst);
    execute_swap_with_fallback(&token, quote("primary", 25u64.into()), SwapAmountLimit::U64)
        .await
        .expect_err("an unproven send failure must not fall back");
    assert_eq!(WIDE_FALLBACK_EXECS.load(Ordering::SeqCst), 0);
    assert_eq!(VALID_FALLBACK_EXECS.load(Ordering::SeqCst), 1);
}
