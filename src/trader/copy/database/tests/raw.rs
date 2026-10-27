// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

#![cfg(test)]

//! Exact raw-amount paper-ledger persistence and rounding regressions.

use super::*;

fn exact_paper_buy(
    task: &CopyTask,
    signature: &str,
    raw: crate::chains::RawAmount,
    decimals: u8,
) -> CopyOutcome {
    let now = Utc::now();
    let metadata = crate::trader::copy::types::PaperQuoteMetadata {
        router_id: "test-router".to_owned(),
        initial_at: now,
        landing_at: now,
        requested_delay_ms: 0,
        queue_duration_ms: 0,
        minimum_output_amount: raw,
        final_output_amount: raw,
        input_amount: Some(crate::chains::RawAmount::from(100_000_000_u64)),
        token_decimals: Some(decimals),
        epoch: 0,
        network_fee_estimated: true,
        platform_fee_included_in_output: true,
    };
    CopyOutcome::PaperFilled(crate::trader::copy::types::PaperDecision {
        task_id: task.id,
        target_address: task.target_address.clone(),
        signature: signature.to_owned(),
        mint: "raw-mint".to_owned(),
        target_size_sol: 0.1,
        target_token_amount: raw.to_whole_units(decimals),
        sized_sol: 0.1,
        fill: crate::trader::copy::types::PaperFill {
            input_sol: 0.1,
            market_price_sol: 0.01,
            priced_from_pool: false,
            fill_price_sol: 0.01,
            token_amount: raw.to_whole_units(decimals),
            referral_fee_sol: 0.0,
            network_fee_sol: 0.000005,
            priority_fee_sol: 0.0,
            total_cost_sol: 0.100005,
            model: crate::trader::copy::types::PaperModel::Quote,
            quote: Some(metadata),
        },
        telemetry: crate::trader::copy::types::CopyTelemetry {
            target_block_time: Some(1),
            detected_at: now,
            decoded_at: now,
            decided_at: now,
            submitted_at: None,
            confirmed_at: Some(now),
            target_price_sol: Some(0.01),
            fill_price_sol: Some(0.01),
            backfill: false,
        },
    })
}

fn exact_paper_sell(
    task: &CopyTask,
    signature: &str,
    raw: crate::chains::RawAmount,
    decimals: u8,
) -> CopyOutcome {
    let now = Utc::now();
    CopyOutcome::PaperSellObserved(crate::trader::copy::types::CopySellDecision {
        task_id: task.id,
        target_address: task.target_address.clone(),
        target_signature: signature.to_owned(),
        mint: "raw-mint".to_owned(),
        target_token_amount: 0.0,
        target_sol_amount: 0.05,
        exit_percentage: None,
        transaction_signature: None,
        error: None,
        exit_rule: Some(crate::trader::copy::types::PaperExitRule::Manual),
        take_profit_level: None,
        telemetry: crate::trader::copy::types::CopyTelemetry {
            target_block_time: None,
            detected_at: now,
            decoded_at: now,
            decided_at: now,
            submitted_at: None,
            confirmed_at: None,
            target_price_sol: None,
            fill_price_sol: Some(0.01),
            backfill: false,
        },
        paper_fill: Some(crate::trader::copy::types::PaperSellFill {
            token_amount: raw.to_whole_units(decimals),
            market_price_sol: 0.01,
            priced_from_pool: false,
            fill_price_sol: 0.01,
            gross_sol: 0.05,
            referral_fee_sol: 0.0,
            network_fee_sol: 0.0,
            priority_fee_sol: 0.0,
            net_proceeds_sol: 0.05,
            model: crate::trader::copy::types::PaperModel::Quote,
            quote: Some(crate::trader::copy::types::PaperQuoteMetadata {
                router_id: "test-router".to_owned(),
                initial_at: now,
                landing_at: now,
                requested_delay_ms: 0,
                queue_duration_ms: 0,
                minimum_output_amount: crate::chains::RawAmount::from(50_000_000_u64),
                final_output_amount: crate::chains::RawAmount::from(50_000_000_u64),
                input_amount: Some(raw),
                token_decimals: Some(decimals),
                epoch: 0,
                network_fee_estimated: true,
                platform_fee_included_in_output: true,
            }),
        }),
        paper_context: None,
    })
}

#[tokio::test]
async fn exact_raw_dca_and_manual_full_close_preserve_all_15_nine_decimal_units() {
    let dir = tempfile::tempdir().unwrap();
    let db = CopyDatabase::open(
        dir.path().join("copy_trading.db"),
        crate::chains::active_chain(),
    )
    .unwrap();
    let configured = db.insert_task(task()).await.unwrap();
    let first = exact_paper_buy(
        &configured,
        "raw-buy-1",
        crate::chains::RawAmount::from(10_u64),
        9,
    );
    let second = exact_paper_buy(
        &configured,
        "raw-buy-2",
        crate::chains::RawAmount::from(5_u64),
        9,
    );
    let encoded = serde_json::to_string(&first).unwrap();
    assert!(encoded.contains("\"final_output_amount\":\"10\""));
    assert!(encoded.contains("\"input_amount\":\"100000000\""));
    db.record_outcome(first).await.unwrap();
    db.record_outcome(second).await.unwrap();
    let open = db
        .paper_position(configured.id, "raw-mint")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        open.raw_token_amount,
        Some(crate::chains::RawAmount::from(15_u64))
    );
    assert_eq!(open.token_decimals, Some(9));

    db.record_outcome(exact_paper_sell(
        &configured,
        "raw-manual-full-close",
        crate::chains::RawAmount::from(15_u64),
        9,
    ))
    .await
    .unwrap();
    let closed = db
        .paper_position(configured.id, "raw-mint")
        .await
        .unwrap()
        .unwrap();
    assert!(!closed.is_open());
    assert_eq!(
        closed.raw_token_amount,
        Some(crate::chains::RawAmount::ZERO)
    );
    assert_eq!(closed.token_amount, 0.0);
    assert_eq!(closed.cost_basis_native, 0.0);
    assert!((closed.realized_cost_native - 0.20001).abs() < 1e-12);
    assert_eq!(closed.realized_proceeds_native, 0.05);
}

#[tokio::test]
async fn raw_amounts_above_f64_integer_precision_round_trip_through_json_and_sqlite() {
    let dir = tempfile::tempdir().unwrap();
    let db = CopyDatabase::open(
        dir.path().join("copy_trading.db"),
        crate::chains::active_chain(),
    )
    .unwrap();
    let configured = db.insert_task(task()).await.unwrap();
    let raw = crate::chains::RawAmount::new((1_u128 << 53) + 17);
    let outcome = exact_paper_buy(&configured, "precision-buy", raw, 0);
    let json = serde_json::to_string(&outcome).unwrap();
    assert!(json.contains(&format!("\"final_output_amount\":\"{}\"", raw)));
    db.record_outcome(outcome).await.unwrap();
    let position = db
        .paper_position(configured.id, "raw-mint")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(position.raw_token_amount, Some(raw));
    let stored: String = rusqlite::Connection::open(dir.path().join("copy_trading.db"))
        .unwrap()
        .query_row(
            "SELECT token_amount_raw FROM copy_paper_positions WHERE task_id=?1 AND mint='raw-mint'",
            [configured.id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(stored, raw.to_string());
}

#[tokio::test]
async fn odd_raw_partial_then_full_close_uses_the_exact_remainder_and_full_basis() {
    let dir = tempfile::tempdir().unwrap();
    let db = CopyDatabase::open(
        dir.path().join("copy_trading.db"),
        crate::chains::active_chain(),
    )
    .unwrap();
    let configured = db.insert_task(task()).await.unwrap();
    db.record_outcome(exact_paper_buy(
        &configured,
        "odd-buy",
        crate::chains::RawAmount::from(17_u64),
        0,
    ))
    .await
    .unwrap();
    db.record_outcome(exact_paper_sell(
        &configured,
        "odd-partial",
        crate::chains::RawAmount::from(8_u64),
        0,
    ))
    .await
    .unwrap();
    let remainder = db
        .paper_position(configured.id, "raw-mint")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        remainder.raw_token_amount,
        Some(crate::chains::RawAmount::from(9_u64))
    );
    assert_eq!(remainder.token_amount, 9.0);
    assert!(remainder.cost_basis_native > 0.0);

    db.record_outcome(exact_paper_sell(
        &configured,
        "odd-final-close",
        crate::chains::RawAmount::from(9_u64),
        0,
    ))
    .await
    .unwrap();
    let closed = db
        .paper_position(configured.id, "raw-mint")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        closed.raw_token_amount,
        Some(crate::chains::RawAmount::ZERO)
    );
    assert_eq!(closed.cost_basis_native, 0.0);
    assert!((closed.realized_cost_native - 0.100005).abs() < 1e-12);
}

#[tokio::test]
async fn raw_balance_overflow_and_sell_mismatches_roll_back_the_whole_outcome() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("copy_trading.db");
    let db = CopyDatabase::open(&path, crate::chains::active_chain()).unwrap();
    let configured = db.insert_task(task()).await.unwrap();
    db.record_outcome(exact_paper_buy(
        &configured,
        "max-buy",
        crate::chains::RawAmount::new(u128::MAX - 1),
        0,
    ))
    .await
    .unwrap();
    let before = db
        .paper_position(configured.id, "raw-mint")
        .await
        .unwrap()
        .unwrap();
    let activity_before = db.list_task_activity(configured.id, 20).await.unwrap();
    assert!(db
        .record_outcome(exact_paper_buy(
            &configured,
            "overflow-buy",
            crate::chains::RawAmount::from(2_u64),
            0,
        ))
        .await
        .is_err());
    assert_eq!(
        db.paper_position(configured.id, "raw-mint").await.unwrap(),
        Some(before.clone())
    );
    assert_eq!(
        db.list_task_activity(configured.id, 20).await.unwrap(),
        activity_before
    );
    assert_eq!(
        db.task_total_spent(configured.id, CopyMode::Paper)
            .await
            .unwrap(),
        0.100005
    );

    assert!(db
        .record_outcome(exact_paper_sell(
            &configured,
            "oversell",
            crate::chains::RawAmount::MAX,
            0,
        ))
        .await
        .is_err());
    assert!(db
        .record_outcome(exact_paper_sell(
            &configured,
            "decimal-mismatch",
            crate::chains::RawAmount::from(1_u64),
            1,
        ))
        .await
        .is_err());
    assert_eq!(
        db.paper_position(configured.id, "raw-mint").await.unwrap(),
        Some(before)
    );
    assert_eq!(
        db.list_task_activity(configured.id, 20).await.unwrap(),
        activity_before
    );
}
