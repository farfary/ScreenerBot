// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Shared copy-database fixtures and task, live-claim, inventory, and schema tests.

use chrono::Utc;

use super::*;
use crate::database::WriteTransaction;
use crate::trader::copy::{ExitMode, SizingMode, SpendState};
use crate::trader::paper::TradingMode;

#[path = "tests/paper.rs"]
mod paper;

fn task() -> CopyTask {
    CopyTask {
        id: 0,
        chain: crate::chains::ChainId::Solana,
        target_address: "target".to_owned(),
        label: Some("Paper".to_owned()),
        enabled: true,
        mode: TradingMode::Paper,
        sizing: SizingMode::Fixed { sol: 0.1 },
        exit_mode: ExitMode::BuyOnly,
        exit_policy_overrides: Default::default(),
        max_native_per_trade: 0.2,
        max_native_per_token: 1.0,
        total_budget_native: 5.0,
        min_target_trade_native: None,
        max_target_trade_native: None,
        buy_once_per_token: false,
        slippage_pct: 1.0,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        require_filter_pass: None,
        copy_stable_quoted: None,
        pause_reason: None,
        paused_at: None,
        paper_started_at: None,
    }
}

fn live_outcome(task: &CopyTask, pending: bool) -> CopyOutcome {
    let now = Utc::now();
    let decision = super::super::types::LiveDecision {
        task_id: task.id,
        target_address: task.target_address.clone(),
        target_signature: "live-target-signature".to_owned(),
        mint: "live-mint".to_owned(),
        target_size_sol: 0.2,
        target_token_amount: 20.0,
        sized_sol: 0.1,
        transaction_signature: Some("our-signature".to_owned()),
        error: None,
        telemetry: super::super::types::CopyTelemetry {
            target_block_time: Some(1),
            detected_at: now,
            decoded_at: now,
            decided_at: now,
            submitted_at: Some(now),
            confirmed_at: (!pending).then_some(now),
            target_price_sol: Some(0.01),
            fill_price_sol: Some(0.011),
            backfill: false,
        },
    };
    if pending {
        CopyOutcome::LiveSubmitted(decision)
    } else {
        CopyOutcome::LiveConfirmed(decision)
    }
}

#[tokio::test]
async fn live_submission_consumes_spend_once_and_confirmation_only_upgrades_status() {
    let dir = tempfile::tempdir().unwrap();
    let db = CopyDatabase::open(
        dir.path().join("copy_trading.db"),
        crate::chains::ChainId::Solana,
    )
    .unwrap();
    let configured = db.insert_task(task()).await.unwrap();
    let configured = db
        .set_task_mode(
            configured.id,
            TradingMode::Live,
            Some(crate::trader::paper::LIVE_ARM_CONFIRMATION.to_owned()),
        )
        .await
        .unwrap();

    let submitted = live_outcome(&configured, true);
    db.record_outcome(submitted.clone()).await.unwrap();
    db.record_outcome(submitted).await.unwrap();
    db.record_outcome(live_outcome(&configured, false))
        .await
        .unwrap();
    let CopyOutcome::LiveSubmitted(mut failed) = live_outcome(&configured, true) else {
        unreachable!()
    };
    failed.target_signature = "definite-pre-submit-failure".to_owned();
    failed.transaction_signature = None;
    failed.error = Some("quote rejected".to_owned());
    failed.telemetry.submitted_at = None;
    db.record_outcome(CopyOutcome::LiveFailed(failed))
        .await
        .unwrap();

    assert_eq!(
        db.live_spend_state(configured.id, "live-mint")
            .await
            .unwrap(),
        SpendState {
            total_spent_native: 0.1,
            token_spent_native: 0.1,
            token_buy_count: 1,
        }
    );
    let activity = db.list_activity(10).await.unwrap();
    assert_eq!(
        activity.len(),
        3,
        "submitted, confirmation, definite failure"
    );
    assert!(activity
        .iter()
        .any(|row| matches!(&row.outcome, CopyOutcome::LiveConfirmed(_))));
}

#[tokio::test]
async fn generic_task_update_cannot_change_mode() {
    let dir = tempfile::tempdir().unwrap();
    let db = CopyDatabase::open(
        dir.path().join("copy_trading.db"),
        crate::chains::ChainId::Solana,
    )
    .unwrap();
    let mut configured = db.insert_task(task()).await.unwrap();
    assert!(db
        .set_task_mode(configured.id, TradingMode::Live, None)
        .await
        .is_err());
    configured.mode = TradingMode::Live;
    assert!(db.update_task(configured).await.is_err());
}

#[tokio::test]
async fn live_activity_claim_is_atomic_and_idempotent() {
    let dir = tempfile::tempdir().unwrap();
    let db = CopyDatabase::open(
        dir.path().join("copy_trading.db"),
        crate::chains::ChainId::Solana,
    )
    .unwrap();
    let configured = db.insert_task(task()).await.unwrap();
    assert!(db
        .claim_live_activity(configured.id, "target-signature")
        .await
        .unwrap());
    assert!(!db
        .claim_live_activity(configured.id, "target-signature")
        .await
        .unwrap());
}

#[tokio::test]
async fn target_inventory_is_idempotent_and_returns_pre_sell_holding() {
    let dir = tempfile::tempdir().unwrap();
    let db = CopyDatabase::open(
        dir.path().join("copy_trading.db"),
        crate::chains::ChainId::Solana,
    )
    .unwrap();
    let task = db.insert_task(task()).await.unwrap();

    assert_eq!(
        db.observe_target_inventory(task.id, "buy", "mint", 100.0)
            .await
            .unwrap(),
        0.0
    );
    db.observe_target_inventory(task.id, "buy", "mint", 100.0)
        .await
        .unwrap();
    assert_eq!(db.target_holding(task.id, "mint").await.unwrap(), 100.0);
    assert_eq!(
        db.observe_target_inventory(task.id, "sell", "mint", -30.0)
            .await
            .unwrap(),
        100.0
    );
    assert_eq!(db.target_holding(task.id, "mint").await.unwrap(), 70.0);
}

#[tokio::test]
async fn stale_claim_reconciliation_is_fail_closed_and_idempotent() {
    let dir = tempfile::tempdir().unwrap();
    let db = CopyDatabase::open(
        dir.path().join("copy_trading.db"),
        crate::chains::ChainId::Solana,
    )
    .unwrap();
    let task = db.insert_task(task()).await.unwrap();
    assert!(db.claim_live_activity(task.id, "orphan").await.unwrap());
    assert_eq!(db.reconcile_stale_claims(0).await.unwrap(), 1);
    assert_eq!(db.reconcile_stale_claims(0).await.unwrap(), 0);
    assert!(!db.claim_live_activity(task.id, "orphan").await.unwrap());
}

#[tokio::test]
async fn schema_v1_backfills_explicit_empty_exit_policy_overrides() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("copy_trading.db");
    {
        let connection = rusqlite::Connection::open(&path).unwrap();
        connection
            .execute_batch(
                "CREATE TABLE copy_tasks (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    target_address TEXT NOT NULL,
                    label TEXT,
                    enabled INTEGER NOT NULL,
                    mode_json TEXT NOT NULL,
                    sizing_json TEXT NOT NULL,
                    exit_mode_json TEXT NOT NULL,
                    max_sol_per_trade REAL NOT NULL,
                    max_sol_per_token REAL NOT NULL,
                    total_budget_sol REAL NOT NULL,
                    min_target_trade_sol REAL,
                    max_target_trade_sol REAL,
                    buy_once_per_token INTEGER NOT NULL,
                    slippage_pct REAL NOT NULL,
                    created_at TEXT NOT NULL,
                    updated_at TEXT NOT NULL
                );",
            )
            .unwrap();
    }
    let db = CopyDatabase::open(&path, crate::chains::ChainId::Solana).unwrap();
    let inserted = db.insert_task(task()).await.unwrap();
    assert_eq!(
        db.get_task(inserted.id)
            .await
            .unwrap()
            .unwrap()
            .exit_policy_overrides,
        crate::trader::policy::ExitPolicyOverrides::default()
    );
}

/// A task whose stop loss carries `allow_partial`, and nothing else overridden.
fn task_with_partial_override(allow_partial: Option<bool>) -> CopyTask {
    let mut task = task();
    task.exit_policy_overrides.stop_loss.allow_partial = allow_partial;
    task
}

/// Every task's stored partial stop-loss override and `updated_at`, read from the
/// rows as written, by task id.
fn stored_partial_overrides(path: &std::path::Path) -> Vec<(i64, Option<bool>, String)> {
    let connection = rusqlite::Connection::open(path).unwrap();
    let mut statement = connection
        .prepare("SELECT id, exit_policy_json, updated_at FROM copy_tasks ORDER BY id")
        .unwrap();
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    rows.into_iter()
        .map(|(id, json, updated_at)| {
            let overrides: crate::trader::policy::ExitPolicyOverrides =
                serde_json::from_str(&json).unwrap();
            (id, overrides.stop_loss.allow_partial, updated_at)
        })
        .collect()
}

#[tokio::test]
async fn a_v10_store_retires_partial_stop_loss_overrides_unless_the_switch_was_on() {
    for switch in [None, Some(false), Some(true)] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("copy_trading.db");
        let db = CopyDatabase::open(&path, crate::chains::ChainId::Solana).unwrap();
        let live = db
            .insert_task(task_with_partial_override(Some(true)))
            .await
            .unwrap();
        let other_chain = db
            .insert_task(task_with_partial_override(Some(true)))
            .await
            .unwrap();
        let off = db
            .insert_task(task_with_partial_override(Some(false)))
            .await
            .unwrap();
        let inherited = db
            .insert_task(task_with_partial_override(None))
            .await
            .unwrap();
        drop(db);
        {
            let connection = rusqlite::Connection::open(&path).unwrap();
            connection
                .execute(
                    "UPDATE copy_tasks SET mode_json = '\"live\"' WHERE id = ?1",
                    [live.id],
                )
                .unwrap();
            connection
                .execute(
                    "UPDATE copy_tasks SET chain_id = 'future-chain' WHERE id = ?1",
                    [other_chain.id],
                )
                .unwrap();
            connection
                .execute(
                    "UPDATE copy_metadata SET value = '10' WHERE key = 'schema_version'",
                    [],
                )
                .unwrap();
        }
        let before = stored_partial_overrides(&path);
        assert_eq!(
            before.iter().map(|row| row.1).collect::<Vec<_>>(),
            vec![Some(true), Some(true), Some(false), None]
        );

        let mut connection = rusqlite::Connection::open(&path).unwrap();
        let transaction = connection.write_tx().unwrap();
        schema::retire_stop_loss_partial_overrides(&transaction, switch).unwrap();
        transaction.commit().unwrap();
        drop(connection);

        let partial = if switch == Some(true) {
            Some(true)
        } else {
            Some(false)
        };
        let updated_at = |index: usize| before[index].2.clone();
        assert_eq!(
            stored_partial_overrides(&path),
            vec![
                (live.id, partial, updated_at(0)),
                (other_chain.id, partial, updated_at(1)),
                (off.id, Some(false), updated_at(2)),
                (inherited.id, None, updated_at(3)),
            ],
            "switch {switch:?}"
        );
    }
}

#[tokio::test]
async fn a_partial_stop_loss_override_saved_after_the_upgrade_is_kept_on_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("copy_trading.db");
    let db = CopyDatabase::open(&path, crate::chains::ChainId::Solana).unwrap();
    let saved = db
        .insert_task(task_with_partial_override(Some(true)))
        .await
        .unwrap();
    drop(db);

    let mut connection = rusqlite::Connection::open(&path).unwrap();
    let transaction = connection.write_tx().unwrap();
    schema::retire_stop_loss_partial_overrides(&transaction, None).unwrap();
    transaction.commit().unwrap();
    drop(connection);
    let db = CopyDatabase::open(&path, crate::chains::ChainId::Solana).unwrap();
    assert_eq!(
        db.get_task(saved.id)
            .await
            .unwrap()
            .unwrap()
            .exit_policy_overrides
            .stop_loss
            .allow_partial,
        Some(true)
    );
}

#[tokio::test]
async fn chain_migration_preserves_legacy_task_children_and_allows_chain_qualified_identity() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("copy_trading.db");
    {
        let connection = rusqlite::Connection::open(&path).unwrap();
        connection.execute_batch(
            "PRAGMA foreign_keys = ON;
             CREATE TABLE copy_tasks (
                id INTEGER PRIMARY KEY, target_address TEXT NOT NULL, label TEXT, enabled INTEGER NOT NULL,
                mode_json TEXT NOT NULL, sizing_json TEXT NOT NULL, exit_mode_json TEXT NOT NULL,
                max_sol_per_trade REAL NOT NULL, max_sol_per_token REAL NOT NULL, total_budget_sol REAL NOT NULL,
                min_target_trade_sol REAL, max_target_trade_sol REAL, buy_once_per_token INTEGER NOT NULL,
                slippage_pct REAL NOT NULL, created_at TEXT NOT NULL, updated_at TEXT NOT NULL
             );
             CREATE TABLE copy_spend (task_id INTEGER NOT NULL, mint TEXT NOT NULL, spent_sol REAL NOT NULL DEFAULT 0,
                buy_count INTEGER NOT NULL DEFAULT 0, updated_at TEXT NOT NULL, PRIMARY KEY(task_id, mint),
                FOREIGN KEY(task_id) REFERENCES copy_tasks(id) ON DELETE CASCADE);",
        ).unwrap();
        connection.execute(
            "INSERT INTO copy_tasks VALUES (9, 'target', NULL, 1, '\"paper\"', '{\"kind\":\"fixed\",\"sol\":1.0}', '\"buy_only\"', 1, 1, 1, NULL, NULL, 0, 1, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
            [],
        ).unwrap();
        connection
            .execute(
                "INSERT INTO copy_spend VALUES (9, 'mint', 0.5, 1, '2026-01-01T00:00:00Z')",
                [],
            )
            .unwrap();
    }
    let db = CopyDatabase::open(&path, crate::chains::ChainId::Solana).unwrap();
    assert_eq!(
        db.get_task(9).await.unwrap().unwrap().chain,
        crate::chains::ChainId::Solana
    );
    let connection = rusqlite::Connection::open(&path).unwrap();
    connection.execute(
        "INSERT INTO copy_tasks (id, chain_id, target_address, label, enabled, mode_json, sizing_json, exit_mode_json, exit_policy_json, max_native_per_trade, max_native_per_token, total_budget_native, buy_once_per_token, slippage_pct, created_at, updated_at) VALUES (10, 'future-chain', 'target', NULL, 1, '\"paper\"', '{\"kind\":\"fixed\",\"sol\":1.0}', '\"buy_only\"', '{}', 1, 1, 1, 0, 1, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
        [],
    ).unwrap();
    assert_eq!(
        connection
            .query_row(
                "SELECT task_id, mode FROM copy_spend WHERE mint='mint'",
                [],
                |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
            )
            .unwrap(),
        (9, "paper_legacy".to_owned()),
        "unexplained historical spend is retained in the legacy paper ledger"
    );
}

fn paper_quote_metadata(
    epoch: u64,
    output_amount: crate::chains::RawAmount,
) -> crate::trader::paper::PaperQuoteMetadata {
    let now = Utc::now();
    crate::trader::paper::PaperQuoteMetadata {
        router_id: "test-router".to_owned(),
        initial_at: now,
        landing_at: now,
        requested_delay_ms: 1000,
        queue_duration_ms: 0,
        minimum_output_amount: output_amount,
        final_output_amount: output_amount,
        input_amount: Some(crate::chains::RawAmount::from(1_u64)),
        token_decimals: Some(6),
        epoch,
        network_fee_estimated: true,
        platform_fee_included_in_output: true,
    }
}

fn paper_buy(task: &CopyTask, signature: &str, tokens: f64, cost: f64) -> CopyOutcome {
    let now = Utc::now();
    let input_sol = cost - 0.000005;
    let input_raw = crate::chains::RawAmount::from(
        crate::chains::adapter_for(task.chain).native_to_raw(input_sol),
    );
    CopyOutcome::PaperFilled(super::super::types::PaperDecision {
        task_id: task.id,
        target_address: task.target_address.clone(),
        signature: signature.to_owned(),
        mint: "book-mint".to_owned(),
        target_size_sol: 0.2,
        target_token_amount: 20.0,
        sized_sol: input_sol,
        fill: crate::trader::paper::PaperFill {
            input_sol,
            market_price_sol: 0.01,
            priced_from_pool: false,
            fill_price_sol: 0.0101,
            token_amount: tokens,
            referral_fee_sol: 0.0,
            network_fee_sol: 0.000005,
            priority_fee_sol: 0.0,
            total_cost_sol: cost,
            model: crate::trader::paper::PaperModel::Quote,
            quote: Some({
                let mut metadata = paper_quote_metadata(
                    0,
                    crate::chains::RawAmount::from_integral_f64(tokens * 1_000_000.0).unwrap(),
                );
                metadata.input_amount = Some(input_raw);
                metadata
            }),
        },
        telemetry: super::super::types::CopyTelemetry {
            target_block_time: Some(1),
            detected_at: now,
            decoded_at: now,
            decided_at: now,
            submitted_at: None,
            confirmed_at: Some(now),
            target_price_sol: Some(0.01),
            fill_price_sol: Some(0.0101),
            backfill: false,
        },
    })
}

fn paper_sell(task: &CopyTask, signature: &str, tokens: f64, proceeds: f64) -> CopyOutcome {
    let now = Utc::now();
    let raw_tokens = crate::chains::RawAmount::from_integral_f64(tokens * 1_000_000.0).unwrap();
    let mut quote = paper_quote_metadata(
        0,
        crate::chains::RawAmount::from_integral_f64(proceeds * 1_000_000_000.0).unwrap(),
    );
    quote.input_amount = Some(raw_tokens);
    quote.token_decimals = Some(6);
    CopyOutcome::PaperSellObserved(super::super::types::CopySellDecision {
        task_id: task.id,
        target_address: task.target_address.clone(),
        target_signature: signature.to_owned(),
        mint: "book-mint".to_owned(),
        target_token_amount: 10.0,
        target_sol_amount: 0.2,
        exit_percentage: None,
        transaction_signature: None,
        error: None,
        exit_rule: None,
        take_profit_level: None,
        paper_fill: Some(crate::trader::paper::PaperSellFill {
            token_amount: tokens,
            market_price_sol: 0.012,
            priced_from_pool: false,
            fill_price_sol: 0.0119,
            gross_sol: proceeds,
            referral_fee_sol: 0.0,
            network_fee_sol: 0.0,
            priority_fee_sol: 0.0,
            net_proceeds_sol: proceeds,
            model: crate::trader::paper::PaperModel::Quote,
            quote: Some(quote),
        }),
        paper_context: None,
        telemetry: super::super::types::CopyTelemetry {
            target_block_time: Some(1),
            detected_at: now,
            decoded_at: now,
            decided_at: now,
            submitted_at: None,
            confirmed_at: None,
            target_price_sol: Some(0.012),
            fill_price_sol: Some(0.0119),
            backfill: false,
        },
    })
}
