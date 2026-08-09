use chrono::Utc;

use super::*;
use crate::trader::copy::{CopyMode, ExitMode, SizingMode};

fn task() -> CopyTask {
    CopyTask {
        id: 0,
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
    let db = CopyDatabase::open(dir.path().join("copy_trading.db")).unwrap();
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
        },
    });
    db.record_outcome(outcome.clone()).await.unwrap();
    db.record_outcome(outcome).await.unwrap();

    assert_eq!(
        db.spend_state(task.id, "mint").await.unwrap(),
        SpendState {
            total_spent_sol: 0.1,
            token_spent_sol: 0.1,
            token_buy_count: 1,
        }
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
    let db = CopyDatabase::open(dir.path().join("copy_trading.db")).unwrap();
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
        db.spend_state(configured.id, "live-mint").await.unwrap(),
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
    let db = CopyDatabase::open(dir.path().join("copy_trading.db")).unwrap();
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
    let db = CopyDatabase::open(dir.path().join("copy_trading.db")).unwrap();
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
    let db = CopyDatabase::open(dir.path().join("copy_trading.db")).unwrap();
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
    let db = CopyDatabase::open(dir.path().join("copy_trading.db")).unwrap();
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
    let db = CopyDatabase::open(&path).unwrap();
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
async fn sell_activity_is_idempotent_and_never_increments_entry_spend() {
    let dir = tempfile::tempdir().unwrap();
    let db = CopyDatabase::open(dir.path().join("copy_trading.db")).unwrap();
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
        telemetry: super::super::types::CopyTelemetry {
            target_block_time: Some(1),
            detected_at: now,
            decoded_at: now,
            decided_at: now,
            submitted_at: None,
            confirmed_at: None,
            target_price_sol: Some(0.02),
            fill_price_sol: None,
        },
    });
    db.record_outcome(outcome.clone()).await.unwrap();
    db.record_outcome(outcome).await.unwrap();
    assert_eq!(
        db.spend_state(configured.id, "mint").await.unwrap(),
        SpendState::default()
    );
    assert_eq!(db.list_activity(10).await.unwrap().len(), 1);
}
