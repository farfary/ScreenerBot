use chrono::Utc;

use super::*;
use crate::trader::copy::{CopyMode, ExitMode, SizingMode, SpendState};

fn task() -> CopyTask {
    CopyTask {
        id: 0,
        chain: crate::chains::ChainId::Solana,
        target_address: "target".to_owned(),
        label: Some("Paper".to_owned()),
        enabled: true,
        mode: CopyMode::Paper,
        sizing: SizingMode::Fixed { sol: 0.1 },
        exit_mode: ExitMode::BuyOnly,
        exit_policy_overrides: Default::default(),
        max_sol_per_trade: 0.2,
        max_sol_per_token: 1.0,
        total_budget_sol: 5.0,
        min_target_trade_sol: None,
        max_target_trade_sol: None,
        buy_once_per_token: false,
        slippage_pct: 1.0,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

#[tokio::test]
async fn task_and_outcome_round_trip_with_idempotent_spend() {
    let dir = tempfile::tempdir().unwrap();
    let db = CopyDatabase::open(
        dir.path().join("copy_trading.db"),
        crate::chains::ChainId::Solana,
    )
    .unwrap();
    let task = db.insert_task(task()).await.unwrap();
    assert_eq!(
        db.enabled_tasks_for_subject("target").await.unwrap(),
        [task.clone()]
    );

    let now = Utc::now();
    let outcome = CopyOutcome::PaperFilled(super::super::types::PaperDecision {
        task_id: task.id,
        target_address: task.target_address.clone(),
        signature: "signature".to_owned(),
        mint: "mint".to_owned(),
        target_size_sol: 0.2,
        target_token_amount: 20.0,
        sized_sol: 0.1,
        fill: super::super::types::PaperFill {
            input_sol: 0.1,
            market_price_sol: 0.01,
            fill_price_sol: 0.0101,
            token_amount: 9.85,
            referral_fee_sol: 0.0005,
            network_fee_sol: 0.000005,
            priority_fee_sol: 0.0,
            total_cost_sol: 0.100005,
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
    });
    db.record_outcome(outcome.clone()).await.unwrap();
    db.record_outcome(outcome).await.unwrap();

    assert_eq!(
        db.spend_state(task.id, CopyMode::Paper, "mint")
            .await
            .unwrap(),
        SpendState {
            total_spent_sol: 0.1,
            token_spent_sol: 0.1,
            token_buy_count: 1,
        }
    );
    assert_eq!(
        db.task_total_spent(task.id, CopyMode::Paper).await.unwrap(),
        0.1
    );
    assert_eq!(
        db.task_total_spent(task.id, CopyMode::Live).await.unwrap(),
        0.0
    );
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
            CopyMode::Live,
            Some(super::super::types::LIVE_ARM_CONFIRMATION.to_owned()),
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
        db.spend_state(configured.id, CopyMode::Live, "live-mint")
            .await
            .unwrap(),
        SpendState {
            total_spent_sol: 0.1,
            token_spent_sol: 0.1,
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
        .set_task_mode(configured.id, CopyMode::Live, None)
        .await
        .is_err());
    configured.mode = CopyMode::Live;
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
        "INSERT INTO copy_tasks (id, chain_id, target_address, label, enabled, mode_json, sizing_json, exit_mode_json, exit_policy_json, max_sol_per_trade, max_sol_per_token, total_budget_sol, buy_once_per_token, slippage_pct, created_at, updated_at) VALUES (10, 'future-chain', 'target', NULL, 1, '\"paper\"', '{\"kind\":\"fixed\",\"sol\":1.0}', '\"buy_only\"', '{}', 1, 1, 1, 0, 1, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
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
        (9, "paper".to_owned()),
        "legacy spend no decision explains is kept under the task's mode"
    );
}

#[tokio::test]
async fn sell_activity_is_idempotent_and_never_increments_entry_spend() {
    let dir = tempfile::tempdir().unwrap();
    let db = CopyDatabase::open(
        dir.path().join("copy_trading.db"),
        crate::chains::ChainId::Solana,
    )
    .unwrap();
    let configured = db.insert_task(task()).await.unwrap();
    let now = Utc::now();
    let outcome = CopyOutcome::PaperSellObserved(super::super::types::CopySellDecision {
        task_id: configured.id,
        target_address: configured.target_address.clone(),
        target_signature: "target-sell".to_owned(),
        mint: "mint".to_owned(),
        target_token_amount: 10.0,
        target_sol_amount: 0.2,
        exit_percentage: None,
        transaction_signature: None,
        error: None,
        exit_rule: None,
        paper_fill: None,
        telemetry: super::super::types::CopyTelemetry {
            target_block_time: Some(1),
            detected_at: now,
            decoded_at: now,
            decided_at: now,
            submitted_at: None,
            confirmed_at: None,
            target_price_sol: Some(0.02),
            fill_price_sol: None,
            backfill: false,
        },
    });
    db.record_outcome(outcome.clone()).await.unwrap();
    db.record_outcome(outcome).await.unwrap();
    assert_eq!(
        db.spend_state(configured.id, CopyMode::Paper, "mint")
            .await
            .unwrap(),
        SpendState::default()
    );
    assert_eq!(db.list_activity(10).await.unwrap().len(), 1);
}

fn paper_buy(task: &CopyTask, signature: &str, tokens: f64, cost: f64) -> CopyOutcome {
    let now = Utc::now();
    CopyOutcome::PaperFilled(super::super::types::PaperDecision {
        task_id: task.id,
        target_address: task.target_address.clone(),
        signature: signature.to_owned(),
        mint: "book-mint".to_owned(),
        target_size_sol: 0.2,
        target_token_amount: 20.0,
        sized_sol: 0.1,
        fill: super::super::types::PaperFill {
            input_sol: 0.1,
            market_price_sol: 0.01,
            fill_price_sol: 0.0101,
            token_amount: tokens,
            referral_fee_sol: 0.0005,
            network_fee_sol: 0.000005,
            priority_fee_sol: 0.0,
            total_cost_sol: cost,
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
        paper_fill: Some(super::super::types::PaperSellFill {
            token_amount: tokens,
            market_price_sol: 0.012,
            fill_price_sol: 0.0119,
            gross_sol: proceeds,
            referral_fee_sol: 0.0,
            network_fee_sol: 0.0,
            priority_fee_sol: 0.0,
            net_proceeds_sol: proceeds,
        }),
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

#[tokio::test]
async fn the_paper_book_books_buys_and_partial_then_full_sells_exactly_once() {
    let dir = tempfile::tempdir().unwrap();
    let db = CopyDatabase::open(
        dir.path().join("copy_trading.db"),
        crate::chains::ChainId::Solana,
    )
    .unwrap();
    let configured = db.insert_task(task()).await.unwrap();

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
    assert!((position.cost_basis_sol - 1.0).abs() < 1e-9);
    assert!((position.realized_proceeds_sol - 1.5).abs() < 1e-9);
    assert!((position.realized_cost_sol - 1.0).abs() < 1e-9);

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
    assert!((closed.realized_proceeds_sol - closed.realized_cost_sol - 0.0).abs() < 1e-9);

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

#[tokio::test]
async fn schema_v5_splits_shared_spend_by_mode_and_rebuilds_the_paper_book() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("copy_trading.db");
    let db = CopyDatabase::open(&path, crate::chains::ChainId::Solana).unwrap();
    let configured = db.insert_task(task()).await.unwrap();
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
                 DELETE FROM copy_paper_positions;",
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

    let db = CopyDatabase::open(&path, crate::chains::ChainId::Solana).unwrap();
    assert_eq!(
        db.task_total_spent(configured.id, CopyMode::Paper)
            .await
            .unwrap(),
        0.1
    );
    assert_eq!(
        db.task_total_spent(configured.id, CopyMode::Live)
            .await
            .unwrap(),
        0.1
    );
    let position = db
        .paper_position(configured.id, "book-mint")
        .await
        .unwrap()
        .expect("paper fill rebuilt into the book");
    assert_eq!(
        (position.token_amount, position.cost_basis_sol),
        (50.0, 0.5)
    );
    drop(db);
    // Re-running the initializer is a no-op.
    let db = CopyDatabase::open(&path, crate::chains::ChainId::Solana).unwrap();
    assert_eq!(db.paper_positions(configured.id).await.unwrap().len(), 1);
}
