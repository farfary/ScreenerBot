// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Current spend resizing, network fees, and quote-only paper accounting.

use super::support::{execute, raw_sol, setup, task, FakeIo, QuoteShape, MINT};
use crate::chains::RawAmount;
use crate::trader::copy::{CopyMode, CopyOutcome, CopySkip};

#[tokio::test]
async fn queued_entries_resize_from_recorded_spend_and_reject_landing_network_cost() {
    let mut configured = task();
    configured.total_budget_native = 0.15;
    configured.max_native_per_token = 0.15;
    let (_dir, database, configured, epoch) = setup(configured).await;

    let first_io = FakeIo::new(database.clone(), configured.id, raw_sol(0.1));
    let first = execute(&database, configured.clone(), "queued-a", epoch, &first_io).await;
    let CopyOutcome::PaperFilled(first_fill) = &first else {
        panic!("first planned entry should fill: {first:?}");
    };
    assert_eq!(first_fill.fill.input_sol, 0.1);
    assert!((first_fill.fill.total_cost_sol - 0.100005).abs() < 1e-12);
    assert_eq!(first_fill.fill.referral_fee_sol, 0.0);
    assert!(
        first_fill
            .fill
            .quote
            .as_ref()
            .unwrap()
            .platform_fee_included_in_output
    );
    assert_eq!(first_io.quote_calls(), 2);
    assert!(super::support::record(&database, first).await);

    let resized_input = RawAmount::from(49_994_999_u64);
    let second_io = FakeIo::new(database.clone(), configured.id, resized_input)
        .with_shapes(QuoteShape::ordinary(10_000), QuoteShape::ordinary(10_000));
    let second = execute(&database, configured.clone(), "queued-b", epoch, &second_io).await;
    assert_eq!(second_io.quote_calls(), 2);
    assert_eq!(
        super::support::skip(&second),
        &CopySkip::BudgetExhausted,
        "the exact-input amount fits, but the landing network fee exceeds the remaining budget"
    );
    assert!(super::support::record(&database, second).await);

    let spend = database
        .spend_state(configured.id, CopyMode::Paper, MINT)
        .await
        .unwrap();
    assert!((spend.total_spent_native - 0.100005).abs() < 1e-12);
    assert!((spend.token_spent_native - 0.100005).abs() < 1e-12);
    assert!(spend.total_spent_native + 0.049995 + 0.00001 > 0.15);
}

#[tokio::test]
async fn successful_executor_entry_records_exact_input_and_one_network_fee() {
    let (_dir, database, configured, epoch) = setup(task()).await;
    let io = FakeIo::new(database.clone(), configured.id, raw_sol(0.1));
    let outcome = execute(
        &database,
        configured.clone(),
        "successful-entry",
        epoch,
        &io,
    )
    .await;
    let CopyOutcome::PaperFilled(decision) = &outcome else {
        panic!("offline fixture should fill: {outcome:?}");
    };
    assert_eq!(io.quote_calls(), 2);
    assert_eq!(decision.fill.input_sol, 0.1);
    assert_eq!(decision.fill.network_fee_sol, 0.000005);
    assert!((decision.fill.total_cost_sol - 0.100005).abs() < 1e-12);
    assert_eq!(decision.fill.referral_fee_sol, 0.0);
    let quote = decision.fill.quote.as_ref().unwrap();
    assert!(quote.platform_fee_included_in_output);
    assert_eq!(quote.minimum_output_amount, RawAmount::from(900_000_u64));
    assert_eq!(quote.final_output_amount, RawAmount::from(1_000_000_u64));
    assert!(decision.fill.has_quote_provenance());

    assert!(super::support::record(&database, outcome).await);
    let spend = database
        .spend_state(configured.id, CopyMode::Paper, MINT)
        .await
        .unwrap();
    assert!((spend.total_spent_native - 0.100005).abs() < 1e-12);
    assert_eq!(spend.token_buy_count, 1);
}
