// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Refreshed admission and task-risk decisions at quote settlement.

use super::support::{execute, raw_sol, setup, task, FakeIo, Mutation, MINT};
use crate::trader::admission::EntryBlock;
use crate::trader::copy::{CopySkip, SpendState};
use crate::trader::paper::TradingMode;

#[tokio::test]
async fn admission_is_rechecked_after_wait_before_the_landing_quote() {
    let (_dir, database, configured, epoch) = setup(task()).await;
    let io = FakeIo::new(database.clone(), configured.id, raw_sol(0.1))
        .at_wait(Mutation::AdmissionBlocked);
    let outcome = execute(&database, configured.clone(), "late-admission", epoch, &io).await;
    assert_eq!(io.admission_calls(), 2);
    assert_eq!(io.quote_calls(), 1);
    assert_eq!(
        super::support::skip(&outcome),
        &CopySkip::EntryBlocked {
            block: EntryBlock::Blacklisted
        }
    );
    assert!(super::support::record(&database, outcome).await);
    assert_eq!(
        database
            .spend_state(configured.id, TradingMode::Paper, MINT)
            .await
            .unwrap(),
        SpendState::default()
    );
}

#[tokio::test]
async fn refreshed_self_wallet_blacklist_and_required_filter_risk_reject_landing() {
    for (mutation, expected, require_filter) in [
        (Mutation::SelfWallet, CopySkip::SelfCopy, false),
        (Mutation::Blacklisted, CopySkip::Blacklisted, false),
        (Mutation::FilterFailed, CopySkip::FilterRequired, true),
    ] {
        let (_dir, database, configured, epoch) = setup(task()).await;
        let mut io = FakeIo::new(database.clone(), configured.id, raw_sol(0.1)).at_wait(mutation);
        if require_filter {
            io = io.require_filter_pass();
        }
        let outcome = execute(&database, configured.clone(), "late-risk", epoch, &io).await;
        assert_eq!(io.quote_calls(), 2);
        assert_eq!(io.admission_calls(), 3);
        assert_eq!(io.risk_calls(), 2);
        assert_eq!(super::support::skip(&outcome), &expected);
        assert!(super::support::record(&database, outcome).await);
        assert_eq!(
            database
                .spend_state(configured.id, TradingMode::Paper, MINT)
                .await
                .unwrap(),
            SpendState::default()
        );
    }
}
