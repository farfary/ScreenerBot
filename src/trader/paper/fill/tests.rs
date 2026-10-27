// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

#![cfg(test)]

//! Offline quote settlement and paper sale sizing regression tests.

use std::collections::VecDeque;

use super::*;

fn buy_request() -> QuoteRequest {
    QuoteRequest {
        chain: crate::chains::active_chain(),
        input_mint: crate::chains::adapter().native_asset_address().to_owned(),
        output_mint: "token".to_owned(),
        input_amount: RawAmount::from(100_000_000_u64),
        wallet_address: "test-wallet".to_owned(),
        slippage_pct: 1.0,
        swap_mode: SwapMode::ExactIn,
        exclude_dexes: None,
    }
}

fn sell_request() -> QuoteRequest {
    QuoteRequest {
        chain: crate::chains::active_chain(),
        input_mint: "token".to_owned(),
        output_mint: crate::chains::adapter().native_asset_address().to_owned(),
        input_amount: RawAmount::from(100_000_000_u64),
        wallet_address: "test-wallet".to_owned(),
        slippage_pct: 1.0,
        swap_mode: SwapMode::ExactIn,
        exclude_dexes: None,
    }
}

fn quote(
    request: &QuoteRequest,
    router: &str,
    output: u64,
    minimum: u64,
    fee: Option<u64>,
) -> Quote {
    Quote {
        chain: request.chain,
        router_id: router.to_owned(),
        router_name: router.to_owned(),
        input_mint: request.input_mint.clone(),
        output_mint: request.output_mint.clone(),
        input_amount: request.input_amount,
        output_amount: RawAmount::from(output),
        minimum_output_amount: RawAmount::from(minimum),
        price_impact_pct: 0.0,
        platform_fee_lamports: Some(500_000),
        estimated_network_fee_lamports: fee,
        slippage_bps: 100,
        route_plan: "fixture".to_owned(),
        swap_mode: request.swap_mode,
        wallet_address: request.wallet_address.clone(),
        exclude_dexes: None,
        execution_data: Vec::new(),
    }
}

async fn run_buy(
    initial: Quote,
    landing: Quote,
    expected_input: RawAmount,
) -> Result<PaperFill, PaperFillError> {
    let request = buy_request();
    let mut replies = VecDeque::from([Ok(initial), Ok(landing)]);
    simulate_buy(
        request,
        6,
        1000,
        Utc::now() - chrono::Duration::seconds(3),
        7,
        move |received| {
            assert_eq!(received.input_amount, expected_input);
            std::future::ready(replies.pop_front().expect("two quote samples"))
        },
        |duration| async move {
            assert_eq!(duration, Duration::from_millis(1000));
            Ok(())
        },
    )
    .await
}

async fn run_sell(
    initial: Quote,
    landing: Quote,
    expected_input: RawAmount,
) -> Result<PaperSellFill, PaperFillError> {
    let request = sell_request();
    let mut replies = VecDeque::from([Ok(initial), Ok(landing)]);
    simulate_sell(
        request,
        6,
        1000,
        Utc::now() - chrono::Duration::seconds(3),
        7,
        move |received| {
            assert_eq!(received.input_amount, expected_input);
            std::future::ready(replies.pop_front().expect("two quote samples"))
        },
        |duration| async move {
            assert_eq!(duration, Duration::from_millis(1000));
            Ok(())
        },
    )
    .await
}

#[tokio::test]
async fn buy_accepts_exact_minimum_improved_and_within_minimum_landing_outputs() {
    let request = buy_request();
    for landing_output in [95_000_000, 97_000_000, 110_000_000] {
        let fill = run_buy(
            quote(&request, "initial", 100_000_000, 95_000_000, Some(5_000)),
            quote(&request, "landing", landing_output, 90_000_000, Some(5_000)),
            request.input_amount,
        )
        .await
        .expect("landing output meets initial minimum");
        assert_eq!(fill.token_amount, landing_output as f64 / 1_000_000.0);
        assert_eq!(fill.input_sol, 0.1);
        assert_eq!(fill.network_fee_sol, 0.000_005);
        assert_eq!(fill.priority_fee_sol, 0.0);
        assert!((fill.total_cost_sol - 0.100_005).abs() < 1e-12);
        assert_eq!(fill.referral_fee_sol, 0.0);
        assert!(!fill.priced_from_pool);
        let metadata = fill.quote.as_ref().unwrap();
        assert_eq!(metadata.router_id, "landing");
        assert_eq!(
            metadata.minimum_output_amount,
            RawAmount::from(95_000_000_u64)
        );
        assert_eq!(
            metadata.final_output_amount,
            RawAmount::from(landing_output)
        );
        assert_eq!(metadata.input_amount, Some(request.input_amount));
        assert_eq!(metadata.token_decimals, Some(6));
        assert_eq!(metadata.requested_delay_ms, 1000);
        assert!(metadata.queue_duration_ms >= 3_000);
        assert!(metadata.initial_at <= metadata.landing_at);
        assert!(metadata.network_fee_estimated);
        assert!(metadata.platform_fee_included_in_output);
        assert!(fill.has_quote_provenance());
    }
}

#[tokio::test]
async fn buy_rejects_output_below_initial_minimum_and_quote_identity_changes() {
    let request = buy_request();
    assert_eq!(
        run_buy(
            quote(&request, "initial", 100_000_000, 95_000_000, Some(5_000)),
            quote(&request, "landing", 94_999_999, 90_000_000, Some(5_000)),
            request.input_amount,
        )
        .await
        .unwrap_err(),
        PaperFillError::MinimumOutputNotMet
    );
    let mut wrong_pair = quote(&request, "landing", 100_000_000, 90_000_000, Some(5_000));
    wrong_pair.output_mint = "different-token".to_owned();
    assert_eq!(
        run_buy(
            quote(&request, "initial", 100_000_000, 95_000_000, Some(5_000)),
            wrong_pair,
            request.input_amount,
        )
        .await
        .unwrap_err(),
        PaperFillError::QuoteUnfillable
    );
}

#[tokio::test]
async fn sell_uses_exact_raw_input_and_fee_net_output_with_one_network_estimate() {
    let request = sell_request();
    let fill = run_sell(
        quote(&request, "initial", 20_000_000, 18_000_000, Some(5_000)),
        quote(&request, "landing", 18_000_000, 17_000_000, Some(5_000)),
        request.input_amount,
    )
    .await
    .expect("landing output meets initial minimum");
    assert_eq!(fill.token_amount, 100.0);
    assert_eq!(fill.gross_sol, 0.018);
    assert_eq!(fill.network_fee_sol, 0.000_005);
    assert_eq!(fill.priority_fee_sol, 0.0);
    assert!((fill.net_proceeds_sol - 0.017_995).abs() < 1e-12);
    assert!((fill.fill_price_sol - 0.000_18).abs() < 1e-12);
    assert_eq!(fill.referral_fee_sol, 0.0);
    assert!(!fill.priced_from_pool);
    assert!(fill.has_quote_provenance());
    let metadata = fill.quote.as_ref().unwrap();
    assert!(metadata.queue_duration_ms >= 3_000);
    assert!(metadata.initial_at <= metadata.landing_at);
    assert_eq!(metadata.input_amount, Some(request.input_amount));
    assert_eq!(metadata.token_decimals, Some(6));
    assert!(metadata.platform_fee_included_in_output);
}

#[tokio::test]
async fn sell_rejects_below_minimum_and_changed_quote_identity() {
    let request = sell_request();
    assert_eq!(
        run_sell(
            quote(&request, "initial", 20_000_000, 18_000_000, Some(5_000)),
            quote(&request, "landing", 17_999_999, 17_000_000, Some(5_000)),
            request.input_amount,
        )
        .await
        .unwrap_err(),
        PaperFillError::MinimumOutputNotMet
    );
    let mut wrong_pair = quote(&request, "landing", 20_000_000, 18_000_000, Some(5_000));
    wrong_pair.input_mint = "different-token".to_owned();
    assert_eq!(
        run_sell(
            quote(&request, "initial", 20_000_000, 18_000_000, Some(5_000)),
            wrong_pair,
            request.input_amount,
        )
        .await
        .unwrap_err(),
        PaperFillError::QuoteUnfillable
    );
}

#[tokio::test]
async fn buy_and_sell_reject_unknown_network_fees_and_router_failures() {
    let buy = buy_request();
    assert_eq!(
        run_buy(
            quote(&buy, "initial", 100_000_000, 95_000_000, Some(5_000)),
            quote(&buy, "landing", 100_000_000, 95_000_000, None),
            buy.input_amount,
        )
        .await
        .unwrap_err(),
        PaperFillError::QuoteUnfillable
    );
    let sell = sell_request();
    assert_eq!(
        run_sell(
            quote(&sell, "initial", 20_000_000, 18_000_000, Some(5_000)),
            quote(&sell, "landing", 20_000_000, 18_000_000, None),
            sell.input_amount,
        )
        .await
        .unwrap_err(),
        PaperFillError::QuoteUnfillable
    );

    let mut replies = VecDeque::from([Err(QuoteError::RouterRejected {
        router: "fixture".to_owned(),
        detail: "offline fixture failure".to_owned(),
    })]);
    let result: Result<PaperFill, PaperFillError> = simulate_buy(
        buy,
        6,
        0,
        Utc::now(),
        7,
        move |_| std::future::ready(replies.pop_front().expect("one failed quote")),
        |_| async { Ok(()) },
    )
    .await;
    assert_eq!(result.unwrap_err(), PaperFillError::QuoteUnavailable);

    let sell = sell_request();
    let mut replies = VecDeque::from([Err(QuoteError::RouterRejected {
        router: "fixture".to_owned(),
        detail: "offline fixture failure".to_owned(),
    })]);
    let result: Result<PaperSellFill, PaperFillError> = simulate_sell(
        sell,
        6,
        0,
        Utc::now(),
        7,
        move |_| std::future::ready(replies.pop_front().expect("one failed quote")),
        |_| async { Ok(()) },
    )
    .await;
    assert_eq!(result.unwrap_err(), PaperFillError::QuoteUnavailable);
}

#[tokio::test]
async fn zero_outputs_and_exact_out_requests_never_settle() {
    let buy = buy_request();
    assert_eq!(
        run_buy(
            quote(&buy, "initial", 0, 0, Some(5_000)),
            quote(&buy, "landing", 100_000_000, 95_000_000, Some(5_000)),
            buy.input_amount,
        )
        .await
        .unwrap_err(),
        PaperFillError::QuoteUnfillable
    );
    let mut exact_out = buy_request();
    exact_out.swap_mode = SwapMode::ExactOut;
    let result: Result<PaperFill, PaperFillError> = simulate_buy(
        exact_out,
        6,
        0,
        Utc::now(),
        0,
        |_| async { panic!("exact-out must fail before quoting") },
        |_| async { Ok(()) },
    )
    .await;
    assert_eq!(result.unwrap_err(), PaperFillError::InvalidSize);
}

#[test]
fn partial_sell_sizing_preserves_large_and_full_width_raw_balances() {
    let above_f64_integer = RawAmount::new((1_u128 << 53) + 17);
    let selected = raw_sell_amount(above_f64_integer, Some(50.0)).unwrap();
    assert_eq!(selected, RawAmount::new(((1_u128 << 53) + 17) / 2));
    assert_eq!(
        raw_sell_amount(RawAmount::MAX, Some(50.0)).unwrap(),
        RawAmount::new(u128::MAX / 2)
    );
    assert_eq!(
        raw_sell_amount(RawAmount::MAX, None).unwrap(),
        RawAmount::MAX
    );
}

#[test]
fn sale_sizing_refuses_empty_holdings_and_unrepresentable_shares() {
    let held = RawAmount::from(15_u64);
    assert_eq!(
        raw_sell_amount(RawAmount::ZERO, None),
        Err(PaperFillError::EmptySell)
    );
    for percentage in [f64::NAN, f64::INFINITY, 0.0, -1.0, 1e-9] {
        assert_eq!(
            raw_sell_amount(held, Some(percentage)),
            Err(PaperFillError::InvalidSize),
            "{percentage}"
        );
    }
    assert_eq!(raw_sell_amount(held, Some(100.0)), Ok(held));
    assert_eq!(raw_sell_amount(held, Some(250.0)), Ok(held));
}
