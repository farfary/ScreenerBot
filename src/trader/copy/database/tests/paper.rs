// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

#![cfg(test)]

//! Paper decisions in the copy store: recorded once as Activity, booking nothing in the
//! copy store; the archived copy paper book read-only; the per-task paper run start; and
//! the historical schema steps that built the archive.

use super::*;

fn open(dir: &tempfile::TempDir) -> CopyDatabase {
    CopyDatabase::open(
        dir.path().join("copy_trading.db"),
        crate::chains::active_chain(),
    )
    .unwrap()
}

fn archive_rows(dir: &tempfile::TempDir) -> i64 {
    rusqlite::Connection::open(dir.path().join("copy_trading.db"))
        .unwrap()
        .query_row("SELECT COUNT(*) FROM copy_paper_positions", [], |row| {
            row.get(0)
        })
        .unwrap()
}

#[tokio::test]
async fn a_paper_decision_is_recorded_once_and_books_nothing_in_the_copy_store() {
    let dir = tempfile::tempdir().unwrap();
    let db = open(&dir);
    let task = db.insert_task(task()).await.unwrap();
    assert_eq!(
        db.tasks_for_subject("target").await.unwrap(),
        [task.clone()]
    );

    let outcome = paper_buy(&task, "signature", 9.85, 0.100005);
    assert!(db.record_outcome(outcome.clone()).await.unwrap());
    assert!(!db.record_outcome(outcome).await.unwrap());

    assert_eq!(db.list_task_activity(task.id, 10).await.unwrap().len(), 1);
    assert_eq!(
        db.live_spend_state(task.id, "book-mint").await.unwrap(),
        SpendState::default()
    );
    assert_eq!(db.live_total_spent(task.id).await.unwrap(), 0.0);
    assert_eq!(db.archived_paper_spent(task.id).await.unwrap(), 0.0);
    assert!(db
        .archived_paper_positions(task.id)
        .await
        .unwrap()
        .is_empty());
    assert_eq!(
        archive_rows(&dir),
        0,
        "a paper fill never books the archive"
    );
}

#[tokio::test]
async fn a_paper_sell_is_recorded_once_and_never_spends() {
    let dir = tempfile::tempdir().unwrap();
    let db = open(&dir);
    let configured = db.insert_task(task()).await.unwrap();
    db.record_outcome(paper_buy(&configured, "target-buy", 10.0, 0.1))
        .await
        .unwrap();
    let mut outcome = paper_sell(&configured, "manual-close:1", 10.0, 0.2);
    let CopyOutcome::PaperSellObserved(decision) = &mut outcome else {
        unreachable!();
    };
    decision.exit_rule = Some(crate::trader::copy::types::PaperExitRule::Manual);
    assert!(db.record_outcome(outcome.clone()).await.unwrap());
    assert!(!db.record_outcome(outcome).await.unwrap());
    assert_eq!(db.list_activity(10).await.unwrap().len(), 2);
    assert_eq!(db.live_total_spent(configured.id).await.unwrap(), 0.0);
    assert_eq!(archive_rows(&dir), 0);
}

#[tokio::test]
async fn raw_amounts_above_f64_integer_precision_round_trip_through_the_stored_decision() {
    let dir = tempfile::tempdir().unwrap();
    let db = open(&dir);
    let configured = db.insert_task(task()).await.unwrap();
    let raw = crate::chains::RawAmount::new((1_u128 << 53) + 17);
    let mut outcome = paper_buy(&configured, "precision-buy", 10.0, 0.1);
    let CopyOutcome::PaperFilled(decision) = &mut outcome else {
        unreachable!();
    };
    let quote = decision.fill.quote.as_mut().unwrap();
    quote.minimum_output_amount = raw;
    quote.final_output_amount = raw;
    let json = serde_json::to_string(&outcome).unwrap();
    assert!(json.contains(&format!("\"final_output_amount\":\"{raw}\"")));
    db.record_outcome(outcome).await.unwrap();
    let stored = db.list_task_activity(configured.id, 1).await.unwrap();
    let CopyOutcome::PaperFilled(decision) = &stored[0].outcome else {
        panic!("expected the paper fill, got {:?}", stored[0].outcome);
    };
    assert_eq!(
        decision.fill.quote.as_ref().unwrap().final_output_amount,
        raw
    );
}

#[tokio::test]
async fn starting_a_paper_run_moves_the_start_and_deletes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let db = open(&dir);
    let configured = db.insert_task(task()).await.unwrap();
    assert_eq!(
        configured.paper_started_at, None,
        "a new task has no own start"
    );
    db.record_outcome(paper_buy(&configured, "before-the-start", 10.0, 0.1))
        .await
        .unwrap();
    let at = Utc::now();
    assert!(db.start_paper_run(configured.id, at).await.unwrap());
    let reread = db.get_task(configured.id).await.unwrap().unwrap();
    assert_eq!(
        reread
            .paper_started_at
            .map(|start| start.timestamp_millis()),
        Some(at.timestamp_millis())
    );
    assert_eq!(
        db.list_task_activity(configured.id, 10)
            .await
            .unwrap()
            .len(),
        1
    );
    assert!(!db.start_paper_run(configured.id + 1, at).await.unwrap());
}

#[tokio::test]
async fn schema_v5_splits_shared_spend_by_mode_and_rebuilds_the_legacy_book() {
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
    assert_eq!(db.archived_paper_spent(configured.id).await.unwrap(), 0.0);
    assert_eq!(
        db.legacy_task_total_spent(configured.id).await.unwrap(),
        0.499995
    );
    assert_eq!(db.live_total_spent(configured.id).await.unwrap(), 0.1);
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
    assert!(db
        .archived_paper_positions(configured.id)
        .await
        .unwrap()
        .is_empty());
    drop(db);
    // Re-running the initializer is a no-op.
    let db = CopyDatabase::open(&path, crate::chains::active_chain()).unwrap();
    assert!(db
        .archived_paper_positions(configured.id)
        .await
        .unwrap()
        .is_empty());
    assert_eq!(
        db.legacy_paper_positions(configured.id)
            .await
            .unwrap()
            .len(),
        1
    );
}
