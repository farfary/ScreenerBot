// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

#![cfg(test)]

//! Paper-ledger recording, idempotency, reset, and historical schema regressions.

use super::*;

#[tokio::test]
async fn malformed_paper_epoch_blocks_read_reset_and_settlement_without_writes() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("copy_trading.db");
    let db = CopyDatabase::open(&path, crate::chains::active_chain()).unwrap();
    let mut configured = task();
    configured.max_native_per_trade = 1.1;
    configured.max_native_per_token = 3.0;
    let task = db.insert_task(configured).await.unwrap();
    assert_eq!(db.paper_epoch(task.id).await.unwrap(), 0);
    db.record_outcome(paper_buy(&task, "before-malformed-epoch", 100.0, 1.0))
        .await
        .unwrap();
    let activity_before = db.list_task_activity(task.id, 20).await.unwrap();
    let spent_before = db.task_total_spent(task.id, CopyMode::Paper).await.unwrap();
    let position_before = db
        .paper_position(task.id, "book-mint")
        .await
        .unwrap()
        .unwrap();
    let connection = rusqlite::Connection::open(&path).unwrap();
    connection
        .execute(
            "INSERT INTO copy_metadata (key, value) VALUES (?1, ?2)",
            rusqlite::params![format!("paper_epoch:{}", task.id), "invalid"],
        )
        .unwrap();
    drop(connection);

    assert!(db.paper_epoch(task.id).await.is_err());
    assert!(db.reset_paper_book(task.id).await.is_err());
    assert!(db
        .record_outcome(paper_buy(&task, "after-malformed-epoch", 50.0, 0.5))
        .await
        .is_err());

    assert_eq!(
        db.list_task_activity(task.id, 20).await.unwrap(),
        activity_before
    );
    assert_eq!(
        db.task_total_spent(task.id, CopyMode::Paper).await.unwrap(),
        spent_before
    );
    assert_eq!(
        db.paper_position(task.id, "book-mint").await.unwrap(),
        Some(position_before)
    );
    let connection = rusqlite::Connection::open(path).unwrap();
    let epoch: String = connection
        .query_row(
            "SELECT value FROM copy_metadata WHERE key = ?1",
            [format!("paper_epoch:{}", task.id)],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(epoch, "invalid");
}

#[tokio::test]
async fn task_and_outcome_round_trip_with_idempotent_spend() {
    let dir = tempfile::tempdir().unwrap();
    let db = CopyDatabase::open(
        dir.path().join("copy_trading.db"),
        crate::chains::active_chain(),
    )
    .unwrap();
    let task = db.insert_task(task()).await.unwrap();
    assert_eq!(
        db.tasks_for_subject("target").await.unwrap(),
        [task.clone()]
    );

    let outcome = paper_buy(&task, "signature", 9.85, 0.100005);
    db.record_outcome(outcome.clone()).await.unwrap();
    db.record_outcome(outcome).await.unwrap();

    assert_eq!(
        db.spend_state(task.id, CopyMode::Paper, "book-mint")
            .await
            .unwrap(),
        SpendState {
            total_spent_native: 0.100005,
            token_spent_native: 0.100005,
            token_buy_count: 1,
        }
    );
    assert_eq!(
        db.task_total_spent(task.id, CopyMode::Paper).await.unwrap(),
        0.100005
    );
    assert_eq!(
        db.task_total_spent(task.id, CopyMode::Live).await.unwrap(),
        0.0
    );
}

#[tokio::test]
async fn sell_activity_is_idempotent_and_never_increments_entry_spend() {
    let dir = tempfile::tempdir().unwrap();
    let db = CopyDatabase::open(
        dir.path().join("copy_trading.db"),
        crate::chains::active_chain(),
    )
    .unwrap();
    let configured = db.insert_task(task()).await.unwrap();
    db.record_outcome(paper_buy(&configured, "target-buy", 10.0, 0.1))
        .await
        .unwrap();
    let spend_before = db
        .spend_state(configured.id, CopyMode::Paper, "book-mint")
        .await
        .unwrap();
    let mut outcome = paper_sell(&configured, "target-sell", 10.0, 0.2);
    let CopyOutcome::PaperSellObserved(decision) = &mut outcome else {
        unreachable!();
    };
    decision.exit_rule = Some(crate::trader::copy::types::PaperExitRule::Manual);
    db.record_outcome(outcome.clone()).await.unwrap();
    db.record_outcome(outcome).await.unwrap();
    assert_eq!(
        db.spend_state(configured.id, CopyMode::Paper, "book-mint")
            .await
            .unwrap(),
        spend_before
    );
    assert_eq!(db.list_activity(10).await.unwrap().len(), 2);
}

#[tokio::test]
async fn a_paper_buy_restarts_the_peak_and_a_stale_raise_cannot_restore_it() {
    let dir = tempfile::tempdir().unwrap();
    let db = CopyDatabase::open(
        dir.path().join("copy_trading.db"),
        crate::chains::active_chain(),
    )
    .unwrap();
    let mut configured_task = task();
    configured_task.max_native_per_trade = 1.1;
    configured_task.max_native_per_token = 3.0;
    let configured = db.insert_task(configured_task).await.unwrap();
    let holding = |position: Option<crate::trader::copy::types::PaperPosition>| {
        position.expect("paper position")
    };
    let peak = |position: Option<crate::trader::copy::types::PaperPosition>| {
        holding(position).peak_price_native.expect("a peak")
    };

    db.record_outcome(paper_buy(&configured, "buy-1", 100.0, 1.0))
        .await
        .unwrap();
    let opened_at = holding(db.paper_position(configured.id, "book-mint").await.unwrap()).opened_at;
    db.raise_paper_peak(configured.id, "book-mint", opened_at, 1, 0.02)
        .await
        .unwrap();
    assert_eq!(
        peak(db.paper_position(configured.id, "book-mint").await.unwrap()),
        0.02
    );

    // The second buy moves the holding's average: the peak restarts at its market price.
    db.record_outcome(paper_buy(&configured, "buy-2", 100.0, 1.0))
        .await
        .unwrap();
    assert_eq!(
        peak(db.paper_position(configured.id, "book-mint").await.unwrap()),
        0.01
    );

    // A sweep that read the holding before the second buy cannot raise it back.
    db.raise_paper_peak(configured.id, "book-mint", opened_at, 1, 0.02)
        .await
        .unwrap();
    assert_eq!(
        peak(db.paper_position(configured.id, "book-mint").await.unwrap()),
        0.01
    );

    db.raise_paper_peak(configured.id, "book-mint", opened_at, 2, 0.015)
        .await
        .unwrap();
    assert_eq!(
        peak(db.paper_position(configured.id, "book-mint").await.unwrap()),
        0.015
    );
}

#[tokio::test]
async fn a_stale_paper_peak_raise_cannot_land_on_a_holding_opened_after_a_reset() {
    let dir = tempfile::tempdir().unwrap();
    let db = CopyDatabase::open(
        dir.path().join("copy_trading.db"),
        crate::chains::active_chain(),
    )
    .unwrap();
    let mut configured_task = task();
    configured_task.max_native_per_trade = 1.1;
    configured_task.max_native_per_token = 3.0;
    let configured = db.insert_task(configured_task).await.unwrap();
    let holding = |position: Option<crate::trader::copy::types::PaperPosition>| {
        position.expect("paper position")
    };

    db.record_outcome(paper_buy(&configured, "buy-before-reset", 100.0, 1.0))
        .await
        .unwrap();
    // A sweep reads this holding, then the paper book is reset before it writes.
    let stale = holding(db.paper_position(configured.id, "book-mint").await.unwrap());
    assert_eq!(stale.buys, 1);
    db.reset_paper_book(configured.id).await.unwrap();

    // The next holding starts again at one buy, opened later, in the new epoch.
    let mut outcome = paper_buy(&configured, "buy-after-reset", 100.0, 1.0);
    let CopyOutcome::PaperFilled(decision) = &mut outcome else {
        unreachable!();
    };
    decision.telemetry.decided_at = stale.opened_at + chrono::Duration::seconds(1);
    decision.fill.quote.as_mut().expect("a quoted fill").epoch =
        db.paper_epoch(configured.id).await.unwrap();
    db.record_outcome(outcome).await.unwrap();
    let fresh = holding(db.paper_position(configured.id, "book-mint").await.unwrap());
    assert_eq!(fresh.buys, stale.buys);
    assert_ne!(fresh.opened_at, stale.opened_at);
    assert_eq!(fresh.peak_price_native, Some(0.01));

    db.raise_paper_peak(
        configured.id,
        "book-mint",
        stale.opened_at,
        stale.buys,
        0.02,
    )
    .await
    .unwrap();
    assert_eq!(
        holding(db.paper_position(configured.id, "book-mint").await.unwrap()).peak_price_native,
        Some(0.01),
        "a peak read from the reset holding landed on the new one"
    );

    db.raise_paper_peak(
        configured.id,
        "book-mint",
        fresh.opened_at,
        fresh.buys,
        0.015,
    )
    .await
    .unwrap();
    assert_eq!(
        holding(db.paper_position(configured.id, "book-mint").await.unwrap()).peak_price_native,
        Some(0.015)
    );
}

#[tokio::test]
async fn the_paper_book_books_buys_and_partial_then_full_sells_exactly_once() {
    let dir = tempfile::tempdir().unwrap();
    let db = CopyDatabase::open(
        dir.path().join("copy_trading.db"),
        crate::chains::active_chain(),
    )
    .unwrap();
    let mut configured_task = task();
    configured_task.max_native_per_trade = 1.1;
    configured_task.max_native_per_token = 3.0;
    let configured = db.insert_task(configured_task).await.unwrap();

    db.record_outcome(paper_buy(&configured, "buy-1", 100.0, 1.0))
        .await
        .unwrap();
    db.record_outcome(paper_buy(&configured, "buy-2", 100.0, 1.0))
        .await
        .unwrap();
    let half = paper_sell(&configured, "sell-1", 100.0, 1.5);
    db.record_outcome(half.clone()).await.unwrap();
    db.record_outcome(half).await.unwrap();

    let position = db
        .paper_position(configured.id, "book-mint")
        .await
        .unwrap()
        .expect("paper position");
    assert!(position.is_open());
    assert_eq!((position.buys, position.sells), (2, 1));
    assert!((position.token_amount - 100.0).abs() < 1e-9);
    assert!((position.cost_basis_native - 1.0).abs() < 1e-9);
    assert!((position.realized_proceeds_native - 1.5).abs() < 1e-9);
    assert!((position.realized_cost_native - 1.0).abs() < 1e-9);

    db.record_outcome(paper_sell(&configured, "sell-2", 100.0, 0.5))
        .await
        .unwrap();
    let closed = db
        .paper_position(configured.id, "book-mint")
        .await
        .unwrap()
        .unwrap();
    assert!(!closed.is_open());
    assert_eq!(closed.token_amount, 0.0);
    assert!((closed.realized_proceeds_native - closed.realized_cost_native - 0.0).abs() < 1e-9);

    assert_eq!(
        db.spend_state(configured.id, CopyMode::Paper, "book-mint")
            .await
            .unwrap()
            .token_buy_count,
        2
    );
    assert_eq!(
        db.spend_state(configured.id, CopyMode::Live, "book-mint")
            .await
            .unwrap(),
        SpendState::default(),
        "paper fills never consume the live budget"
    );
}

/// Each paper round counts its own partial sales and take-profit steps: a sale that
/// leaves tokens is a partial of the round, a policy sale records the step it takes, and
/// a holding that reopens starts both again at zero while `sells` keeps every round's
/// sales.
#[tokio::test]
async fn a_reopened_paper_round_starts_its_partials_and_take_profit_steps_again() {
    let dir = tempfile::tempdir().unwrap();
    let db = CopyDatabase::open(
        dir.path().join("copy_trading.db"),
        crate::chains::active_chain(),
    )
    .unwrap();
    let mut configured_task = task();
    configured_task.max_native_per_trade = 1.1;
    configured_task.max_native_per_token = 3.0;
    let configured = db.insert_task(configured_task).await.unwrap();
    let round = |position: &crate::trader::copy::PaperPosition| {
        (
            position.sells,
            position.round_partial_exits,
            position.take_profit_level,
        )
    };

    db.record_outcome(paper_buy(&configured, "buy-1", 100.0, 1.0))
        .await
        .unwrap();
    let mut first_step = paper_sell(&configured, "sell-1", 40.0, 0.6);
    if let CopyOutcome::PaperSellObserved(decision) = &mut first_step {
        decision.exit_rule = Some(crate::trader::copy::PaperExitRule::TakeProfit);
        decision.take_profit_level = Some(1);
    }
    db.record_outcome(first_step.clone()).await.unwrap();
    db.record_outcome(first_step).await.unwrap();
    let stepped = db
        .paper_position(configured.id, "book-mint")
        .await
        .unwrap()
        .expect("paper position");
    assert!(stepped.is_open());
    assert_eq!(round(&stepped), (1, 1, 1), "one partial sale took step 1");

    db.record_outcome(paper_sell(&configured, "sell-2", 60.0, 0.9))
        .await
        .unwrap();
    let closed = db
        .paper_position(configured.id, "book-mint")
        .await
        .unwrap()
        .unwrap();
    assert!(!closed.is_open());
    assert_eq!(
        round(&closed),
        (2, 1, 1),
        "the closing sale is not a partial and takes no step"
    );

    db.record_outcome(paper_buy(&configured, "buy-2", 100.0, 1.0))
        .await
        .unwrap();
    let reopened = db
        .paper_position(configured.id, "book-mint")
        .await
        .unwrap()
        .unwrap();
    assert!(reopened.is_open());
    assert_eq!(
        round(&reopened),
        (2, 0, 0),
        "a new round has sold nothing and taken no step"
    );
}

#[tokio::test]
async fn paper_reset_preserves_legacy_live_data_and_rejects_stale_quote_fills_and_skips() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("copy_trading.db");
    let db = CopyDatabase::open(&path, crate::chains::active_chain()).unwrap();
    let configured = db.insert_task(task()).await.unwrap();
    db.record_outcome(paper_buy(&configured, "current-buy", 50.0, 0.100005))
        .await
        .unwrap();
    let stale_fill = paper_buy(&configured, "stale-buy", 25.0, 0.050005);
    let stale_skip = CopyOutcome::Skipped {
        task_id: configured.id,
        signature: "stale-skip".to_owned(),
        mint: Some("book-mint".to_owned()),
        reason: crate::trader::copy::types::CopySkip::PaperQueueFull,
        decided_at: Utc::now(),
        telemetry: None,
        paper: Some(crate::trader::copy::types::PaperContext {
            model: crate::trader::copy::types::PaperModel::Quote,
            epoch: 0,
        }),
    };
    db.record_outcome(stale_skip.clone()).await.unwrap();
    let live_skip = CopyOutcome::Skipped {
        task_id: configured.id,
        signature: "live-skip".to_owned(),
        mint: Some("live-mint".to_owned()),
        reason: crate::trader::copy::types::CopySkip::TaskDisabled,
        decided_at: Utc::now(),
        telemetry: None,
        paper: None,
    };
    db.record_outcome(live_skip).await.unwrap();

    let legacy_decision = r#"{"outcome":"paper_filled","fill":{"model":"legacy"},"marker":"keep"}"#;
    let legacy_activity = r#"{"outcome":"skipped","paper":{"model":"legacy"},"marker":"keep"}"#;
    let connection = rusqlite::Connection::open(&path).unwrap();
    connection
        .execute(
            "INSERT INTO copy_paper_legacy_positions (task_id, mint, token_amount, cost_basis_native, \
             invested_native, realized_proceeds_native, realized_cost_native, buys, sells, \
             last_price_native, last_price_at, opened_at, closed_at, updated_at, peak_price_native) \
             VALUES (?1, 'legacy-mint', 12, 0.2, 0.25, 0.1, 0.05, 2, 1, 0.02, NULL, \
             '2026-01-01T00:00:00Z', NULL, '2026-01-02T00:00:00Z', 0.03)",
            [configured.id],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO copy_spend (task_id, mode, mint, spent_native, buy_count, updated_at) \
             VALUES (?1, 'paper_legacy', 'legacy-mint', 0.25, 2, '2026-01-02T00:00:00Z')",
            [configured.id],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO copy_decisions (task_id, signature, mint, outcome_json, decided_at) \
             VALUES (?1, 'legacy-decision', 'legacy-mint', ?2, '2026-01-02T00:00:00Z')",
            rusqlite::params![configured.id, legacy_decision],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO copy_activity (task_id, kind, details_json, created_at) \
             VALUES (?1, 'skipped', ?2, '2026-01-02T00:00:00Z')",
            rusqlite::params![configured.id, legacy_activity],
        )
        .unwrap();
    drop(connection);

    let removed = db.reset_paper_book(configured.id).await.unwrap();
    assert_eq!(
        removed, 2,
        "only the current fill and quote skip are removed"
    );
    assert_eq!(db.paper_epoch(configured.id).await.unwrap(), 1);
    assert!(db.paper_positions(configured.id).await.unwrap().is_empty());
    assert_eq!(
        db.task_total_spent(configured.id, CopyMode::Paper)
            .await
            .unwrap(),
        0.0
    );
    assert_eq!(
        db.legacy_task_total_spent(configured.id).await.unwrap(),
        0.25
    );
    assert_eq!(
        db.legacy_paper_positions(configured.id)
            .await
            .unwrap()
            .len(),
        1
    );

    assert!(!db.record_outcome(stale_fill).await.unwrap());
    assert!(!db.record_outcome(stale_skip).await.unwrap());
    let connection = rusqlite::Connection::open(&path).unwrap();
    let quote_rows: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM copy_decisions WHERE task_id = ?1 \
             AND json_extract(outcome_json, '$.fill.model') = 'quote' \
             OR task_id = ?1 AND json_extract(outcome_json, '$.paper.model') = 'quote'",
            [configured.id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(quote_rows, 0);
    let preserved: (String, String, i64) = connection
        .query_row(
            "SELECT (SELECT outcome_json FROM copy_decisions WHERE signature='legacy-decision'), \
             (SELECT details_json FROM copy_activity WHERE kind='skipped' AND details_json LIKE '%legacy%'), \
             (SELECT COUNT(*) FROM copy_decisions WHERE signature='live-skip')",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();
    assert_eq!(
        preserved,
        (legacy_decision.to_owned(), legacy_activity.to_owned(), 1)
    );
}

#[tokio::test]
async fn schema_v5_splits_shared_spend_by_mode_and_rebuilds_the_paper_book() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("copy_trading.db");
    let db = CopyDatabase::open(&path, crate::chains::active_chain()).unwrap();
    let mut configured_task = task();
    configured_task.max_native_per_trade = 0.6;
    let configured = db.insert_task(configured_task).await.unwrap();
    db.record_outcome(paper_buy(&configured, "buy-1", 50.0, 0.5))
        .await
        .unwrap();
    db.record_outcome(live_outcome(&configured, true))
        .await
        .unwrap();
    drop(db);
    {
        // Put the file back into the v4 shape: one shared spend ledger, no book.
        let connection = rusqlite::Connection::open(&path).unwrap();
        connection
            .execute_batch(
                "DROP TABLE copy_spend;
                 CREATE TABLE copy_spend (task_id INTEGER NOT NULL, mint TEXT NOT NULL,
                    spent_sol REAL NOT NULL DEFAULT 0, buy_count INTEGER NOT NULL DEFAULT 0,
                    updated_at TEXT NOT NULL, PRIMARY KEY (task_id, mint),
                    FOREIGN KEY (task_id) REFERENCES copy_tasks(id) ON DELETE CASCADE);
                 DELETE FROM copy_paper_positions;
                 DROP TABLE copy_paper_legacy_positions;
                 UPDATE copy_metadata SET value = '4' WHERE key = 'schema_version';",
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO copy_spend VALUES (?1, 'book-mint', 0.1, 1, '2026-01-01T00:00:00Z'), \
                 (?1, 'live-mint', 0.1, 1, '2026-01-01T00:00:00Z')",
                [configured.id],
            )
            .unwrap();
    }

    let db = CopyDatabase::open(&path, crate::chains::active_chain()).unwrap();
    assert_eq!(
        db.task_total_spent(configured.id, CopyMode::Paper)
            .await
            .unwrap(),
        0.0
    );
    assert_eq!(
        db.legacy_task_total_spent(configured.id).await.unwrap(),
        0.499995
    );
    assert_eq!(
        db.task_total_spent(configured.id, CopyMode::Live)
            .await
            .unwrap(),
        0.1
    );
    let position = db
        .legacy_paper_positions(configured.id)
        .await
        .unwrap()
        .into_iter()
        .find(|position| position.mint == "book-mint")
        .expect("historical paper fill archived");
    assert_eq!(
        (position.token_amount, position.cost_basis_native),
        (50.0, 0.5)
    );
    assert_eq!(position.raw_token_amount, None);
    assert_eq!(position.token_decimals, None);
    assert!(db.paper_positions(configured.id).await.unwrap().is_empty());
    drop(db);
    // Re-running the initializer is a no-op.
    let db = CopyDatabase::open(&path, crate::chains::active_chain()).unwrap();
    assert!(db.paper_positions(configured.id).await.unwrap().is_empty());
    assert_eq!(
        db.legacy_paper_positions(configured.id)
            .await
            .unwrap()
            .len(),
        1
    );
}
