// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Stable-quoted consent and uncopyable-trade skips recorded by the copy consumer.

use chrono::Utc;

use super::{admit_stable_quoted, record_uncopyable};
use crate::chains::ChainId;
use crate::trader::copy::{CopyDatabase, CopyOutcome, CopySkip, CopyTask, ExitMode, SizingMode};
use crate::trader::paper::TradingMode;
use crate::wallets::watch::{ActivityKind, SwapSide, WalletActivity, WatchSource};

const TARGET: &str = "target-wallet";
const MINT: &str = "traded-mint";

fn task(enabled: bool, mode: TradingMode, copy_stable_quoted: Option<bool>) -> CopyTask {
    CopyTask {
        id: 0,
        chain: ChainId::Solana,
        target_address: TARGET.to_owned(),
        label: None,
        enabled,
        mode,
        sizing: SizingMode::Fixed { sol: 0.1 },
        exit_mode: ExitMode::Mirror,
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
        copy_stable_quoted,
        pause_reason: None,
        paused_at: None,
        paper_started_at: None,
    }
}

fn activity(signature: &str, kind: ActivityKind, task_ids: &[i64]) -> WalletActivity {
    WalletActivity {
        subject: TARGET.to_owned(),
        signature: signature.to_owned(),
        slot: 1,
        block_time: Some(Utc::now().timestamp()),
        detected_at: Utc::now(),
        decoded_at: Utc::now(),
        success: true,
        kind,
        sources: task_ids
            .iter()
            .map(|task_id| WatchSource::Copy { task_id: *task_id })
            .collect(),
        backfill: false,
    }
}

fn stable_buy() -> ActivityKind {
    ActivityKind::StableSwap {
        mint: MINT.to_owned(),
        side: SwapSide::Buy,
        stable_mint: crate::chains::adapter_for(ChainId::Solana).stable_assets()[0].to_owned(),
        stable_amount: 250.0,
        token_amount: 1_000.0,
        venue: None,
    }
}

async fn database() -> (tempfile::TempDir, CopyDatabase) {
    let dir = tempfile::tempdir().unwrap();
    let database = CopyDatabase::open(dir.path().join("copy_trading.db"), ChainId::Solana).unwrap();
    (dir, database)
}

/// Every skip recorded for `task_id`, as (signature, mint, reason, paper-bound).
async fn skips(
    database: &CopyDatabase,
    task_id: i64,
) -> Vec<(String, Option<String>, CopySkip, bool)> {
    database
        .list_task_activity(task_id, 100)
        .await
        .unwrap()
        .into_iter()
        .filter_map(|row| match row.outcome {
            CopyOutcome::Skipped {
                signature,
                mint,
                reason,
                paper,
                ..
            } => Some((signature, mint, reason, paper.is_some())),
            _ => None,
        })
        .collect()
}

#[tokio::test]
async fn only_consenting_tasks_copy_a_stable_quoted_buy() {
    let (_dir, database) = database().await;
    let inherits = database
        .insert_task(task(true, TradingMode::Paper, None))
        .await
        .unwrap();
    let opted_in = database
        .insert_task(task(true, TradingMode::Paper, Some(true)))
        .await
        .unwrap();
    // A live task only arrives through a confirmed mode change; its skip carries
    // no paper book.
    let opted_out = CopyTask {
        mode: TradingMode::Live,
        ..database
            .insert_task(task(true, TradingMode::Paper, Some(false)))
            .await
            .unwrap()
    };
    let observed = activity(
        "stable-buy",
        stable_buy(),
        &[inherits.id, opted_in.id, opted_out.id],
    );
    let tasks = vec![inherits.clone(), opted_in.clone(), opted_out.clone()];

    let admitted = admit_stable_quoted(&database, tasks.clone(), &observed, MINT, false)
        .await
        .unwrap();
    assert_eq!(
        admitted.iter().map(|task| task.id).collect::<Vec<_>>(),
        [opted_in.id]
    );
    let expected = |paper| {
        vec![(
            "stable-buy".to_owned(),
            Some(MINT.to_owned()),
            CopySkip::StableQuoteDisabled,
            paper,
        )]
    };
    assert_eq!(skips(&database, inherits.id).await, expected(true));
    assert_eq!(skips(&database, opted_out.id).await, expected(false));
    assert!(skips(&database, opted_in.id).await.is_empty());

    let admitted = admit_stable_quoted(&database, tasks, &observed, MINT, true)
        .await
        .unwrap();
    assert_eq!(
        admitted.iter().map(|task| task.id).collect::<Vec<_>>(),
        [inherits.id, opted_in.id],
        "the global setting admits a task without an override"
    );
}

#[tokio::test]
async fn an_uncopyable_trade_is_shown_in_every_enabled_watching_task() {
    let (_dir, database) = database().await;
    let enabled = database
        .insert_task(task(true, TradingMode::Paper, None))
        .await
        .unwrap();
    let paused = database
        .insert_task(task(false, TradingMode::Paper, None))
        .await
        .unwrap();
    let observed = activity(
        "mixed-quote",
        ActivityKind::UnresolvedSwap {
            mint: None,
            reason: crate::wallets::watch::UnresolvedSwapReason::MultipleTokens,
            venue: None,
        },
        &[enabled.id, paused.id],
    );

    record_uncopyable(
        &database,
        &observed,
        None,
        CopySkip::SwapMultipleTokens,
        false,
    )
    .await
    .unwrap();
    assert!(
        skips(&database, enabled.id).await.is_empty(),
        "copy processing switched off records nothing"
    );

    record_uncopyable(
        &database,
        &observed,
        None,
        CopySkip::SwapMultipleTokens,
        true,
    )
    .await
    .unwrap();
    assert_eq!(
        skips(&database, enabled.id).await,
        vec![(
            "mixed-quote".to_owned(),
            None,
            CopySkip::SwapMultipleTokens,
            true
        )]
    );
    assert!(skips(&database, paused.id).await.is_empty());
}
