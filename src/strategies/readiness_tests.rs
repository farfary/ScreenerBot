// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Tests of the strategy readiness composition: which checks a strategy type gets, the
//! figures they read from the paper rounds, and the wording of each live block.

use chrono::Utc;

use super::*;
use crate::i18n::LanguageIdentifier;
use crate::positions::{Book, Opening, PositionManagement, PositionOrigin};

fn round(mint: &str, pnl: Option<f64>, marked: bool) -> Position {
    let mut position = Position::opening(Opening {
        mint: mint.to_owned(),
        symbol: "PAPER".to_owned(),
        name: "Paper".to_owned(),
        entry_price: 0.001,
        entry_time: Utc::now(),
        size_native: 0.1,
        entry_signature: None,
        origin: PositionOrigin::Auto {
            strategy_id: Some("strategy".to_owned()),
        },
        management: PositionManagement::AutoTrader,
        entry_liquidity: None,
        book: Book::Paper,
    });
    if let Some(pnl) = pnl {
        position.exit_time = Some(Utc::now());
        position.pnl = Some(pnl);
    }
    if marked {
        position.current_price_updated = Some(position.entry_time + chrono::Duration::seconds(5));
    }
    position
}

fn ids_of(readiness: &Readiness) -> Vec<&'static str> {
    readiness.checks.iter().map(|check| check.id).collect()
}

#[test]
fn an_entry_strategy_is_judged_on_history_profit_marks_and_runtime() {
    let en: LanguageIdentifier = "en".parse().unwrap();
    let rounds = vec![
        round("A", Some(0.02), true),
        round("B", Some(-0.005), true),
        round("C", None, false),
    ];
    let readiness = strategy_readiness(StrategyType::Entry, &rounds, 2, 0, None);
    assert_eq!(
        ids_of(&readiness),
        ["history", "profit", "priced", "exits", "runtime"]
    );
    let check = |id| {
        readiness
            .checks
            .iter()
            .find(|check| check.id == id)
            .unwrap()
    };
    assert!(check("history").passed, "two closed rounds meet two needed");
    assert!(check("profit").passed);
    assert_eq!(
        check("profit").detail.render_plain(&en),
        "+0.0150 SOL realized over 2 rounds, 1 won"
    );
    assert!(!check("priced").passed);
    assert_eq!(
        check("priced").detail.render_plain(&en),
        "1 open paper position without a pool price"
    );
    assert!(check("runtime").passed);
    assert!(!readiness.ready);
}

#[test]
fn an_exit_strategy_is_not_judged_on_the_profit_of_rounds_it_did_not_open() {
    let rounds = vec![round("A", Some(-0.5), true)];
    let readiness = strategy_readiness(StrategyType::Exit, &rounds, 1, 0, None);
    assert_eq!(
        ids_of(&readiness),
        ["history", "priced", "exits", "runtime"]
    );
    assert!(readiness.ready);
}

#[test]
fn an_open_round_with_only_its_entry_price_is_not_priced() {
    let rounds = vec![round("A", None, false)];
    let readiness = strategy_readiness(StrategyType::Exit, &rounds, 0, 0, None);
    let priced = readiness
        .checks
        .iter()
        .find(|check| check.id == "priced")
        .unwrap();
    assert!(
        !priced.passed,
        "the entry price an open round starts with is not a mark"
    );
    let marked = vec![round("A", None, true)];
    assert!(strategy_readiness(StrategyType::Exit, &marked, 0, 0, None).ready);
}

#[test]
fn a_run_of_unfilled_exits_fails_readiness() {
    let readiness = strategy_readiness(
        StrategyType::Exit,
        &[],
        0,
        crate::trader::paper::readiness::FAILED_EXIT_LIMIT,
        None,
    );
    let exits = readiness
        .checks
        .iter()
        .find(|check| check.id == "exits")
        .unwrap();
    assert!(!exits.passed);
    assert!(!readiness.ready);
}

#[test]
fn a_live_block_fails_the_runtime_check_with_its_wording() {
    let en: LanguageIdentifier = "en".parse().unwrap();
    for (block, english) in [
        (
            LiveBlock::SetupIncomplete,
            "Finish wallet and RPC setup first",
        ),
        (LiveBlock::ForceStop, "The emergency stop is engaged"),
    ] {
        assert_eq!(live_block_text(block).render_plain(&en), english);
        let readiness = strategy_readiness(StrategyType::Exit, &[], 0, 0, Some(block));
        let runtime = readiness
            .checks
            .iter()
            .find(|check| check.id == "runtime")
            .unwrap();
        assert!(!runtime.passed);
        assert_eq!(runtime.detail.render_plain(&en), english);
        assert!(!readiness.ready);
    }
}
