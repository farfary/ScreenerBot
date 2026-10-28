// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Tests of the strategy readiness composition: which checks a strategy type gets, the
//! figures they read from the paper rounds, the evidence window the rounds are counted in,
//! and the wording of each live block.

use chrono::{Duration, TimeZone, Utc};

use std::collections::HashMap;

use super::*;
use crate::i18n::LanguageIdentifier;
use crate::positions::db::StrategyRound;
use crate::positions::{Book, Opening, PositionManagement, PositionOrigin};
use crate::strategies::types::{Condition, RuleTree};
use crate::trader::paper::TradingMode;

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
    let readiness = strategy_readiness(StrategyType::Entry, &rounds, None, 2, 0, None);
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
    let readiness = strategy_readiness(StrategyType::Exit, &rounds, None, 1, 0, None);
    assert_eq!(
        ids_of(&readiness),
        ["history", "priced", "exits", "runtime"]
    );
    assert!(readiness.ready);
}

#[test]
fn an_open_round_with_only_its_entry_price_is_not_priced() {
    let rounds = vec![round("A", None, false)];
    let readiness = strategy_readiness(StrategyType::Exit, &rounds, None, 0, 0, None);
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
    assert!(strategy_readiness(StrategyType::Exit, &marked, None, 0, 0, None).ready);
}

#[test]
fn a_run_of_unfilled_exits_fails_readiness() {
    let readiness = strategy_readiness(
        StrategyType::Exit,
        &[],
        None,
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
        let readiness = strategy_readiness(StrategyType::Exit, &[], None, 0, 0, Some(block));
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

fn paper_strategy(id: &str, strategy_type: StrategyType) -> Strategy {
    Strategy {
        id: id.to_owned(),
        name: id.to_owned(),
        description: None,
        strategy_type,
        enabled: true,
        priority: 10,
        timeframe: "5m".to_owned(),
        rules: RuleTree::leaf(Condition {
            condition_type: "PriceChangePercent".to_owned(),
            parameters: HashMap::new(),
        }),
        parameters: HashMap::new(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
        author: None,
        version: 1,
        entry_strategy_ids: Vec::new(),
        mode: TradingMode::Paper,
        rules_changed_at: Utc::now(),
    }
}

/// A paper round `entry` opened and, once closed with `pnl`, the exit leg of `exit`
/// closed.
fn strategy_round(entry: Option<&str>, exit: Option<&str>, pnl: Option<f64>) -> StrategyRound {
    let mut position = round("MINT", pnl, true);
    position.origin = PositionOrigin::Auto {
        strategy_id: entry.map(str::to_owned),
    };
    StrategyRound {
        position,
        exit_strategy_id: exit.map(str::to_owned),
    }
}

/// The overview reads every strategy's readiness from one batch of evidence; the arm
/// dialog reads one strategy's from the same evidence. Both agree for every strategy, and
/// each strategy's rounds are those it opened or whose exit leg it decided.
#[test]
fn the_batch_and_the_single_strategy_readiness_agree() {
    crate::config::utils::install_default_config();
    let evidence = PaperStrategyEvidence::new(
        None,
        vec![
            strategy_round(Some("entry-a"), Some("exit-x"), Some(0.02)),
            strategy_round(Some("entry-a"), None, Some(-0.01)),
            strategy_round(Some("entry-b"), Some("exit-x"), Some(0.01)),
            strategy_round(Some("entry-b"), None, None),
            strategy_round(None, Some("exit-y"), Some(-0.02)),
        ],
        HashMap::from([("exit-y".to_owned(), 2)]),
    );
    let needed = with_config(|config| config.trader.paper.readiness_min_closed_rounds);
    for (strategy, rounds, closed) in [
        (paper_strategy("entry-a", StrategyType::Entry), 2, 2),
        (paper_strategy("entry-b", StrategyType::Entry), 2, 1),
        (paper_strategy("exit-x", StrategyType::Exit), 2, 2),
        (paper_strategy("exit-y", StrategyType::Exit), 1, 1),
        (paper_strategy("unknown", StrategyType::Entry), 0, 0),
    ] {
        assert_eq!(
            evidence.rounds_of(&strategy.id).len(),
            rounds,
            "{}",
            strategy.id
        );
        let single = strategy_readiness(
            strategy.strategy_type,
            &evidence.rounds_of(&strategy.id),
            None,
            needed,
            evidence.failed_exit_streak_of(&strategy.id),
            crate::trader::live_block(),
        );
        let shared = readiness_of(&strategy, &evidence);
        assert_eq!(
            ids_of(&shared),
            ids_of(&single),
            "{}: the shared composition is the single-strategy one",
            strategy.id
        );
        assert_eq!(shared.ready, single.ready, "{}", strategy.id);
        let progress = readiness_progress(&strategy, &evidence);
        assert_eq!(
            progress,
            ReadinessProgress {
                closed,
                needed,
                ready: single.ready,
                failing: single
                    .checks
                    .iter()
                    .find(|check| !check.passed)
                    .map(|check| check.id),
                since_rules_changed: None,
            },
            "{}",
            strategy.id
        );
    }
    assert_eq!(evidence.failed_exit_streak_of("exit-y"), 2);
}

fn at(seconds: i64) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 3, 1, 12, 0, 0).unwrap() + Duration::seconds(seconds)
}

/// A paper round of `entry` entered at `entered`, closed with `pnl` when given.
fn entered_round(entry: &str, entered: DateTime<Utc>, pnl: Option<f64>) -> StrategyRound {
    let mut round = strategy_round(Some(entry), None, pnl);
    round.position.entry_time = entered;
    if let Some(exit_time) = round.position.exit_time.as_mut() {
        *exit_time = entered + Duration::seconds(30);
    }
    if let Some(marked_at) = round.position.current_price_updated.as_mut() {
        *marked_at = entered + Duration::seconds(5);
    }
    round
}

fn changed_at(id: &str, rules_changed_at: DateTime<Utc>) -> Strategy {
    Strategy {
        rules_changed_at,
        ..paper_strategy(id, StrategyType::Entry)
    }
}

#[test]
fn the_window_starts_at_the_later_of_the_run_start_and_the_rules_change() {
    let strategy = changed_at("entry", at(100));
    assert_eq!(
        EvidenceWindow::of(&strategy, None),
        EvidenceWindow {
            since: None,
            rules_changed_at: None,
        },
        "no paper run, no window"
    );
    assert_eq!(
        EvidenceWindow::of(&strategy, Some(at(0))),
        EvidenceWindow {
            since: Some(at(100)),
            rules_changed_at: Some(at(100)),
        },
        "a change inside the run starts the window"
    );
    for run_started_at in [at(100), at(200)] {
        assert_eq!(
            EvidenceWindow::of(&strategy, Some(run_started_at)),
            EvidenceWindow {
                since: Some(run_started_at),
                rules_changed_at: None,
            },
            "a change at or before the run start leaves the whole run"
        );
    }
}

/// A round entered before the rules change is evidence of other rules and is not counted,
/// by the readiness, its progress and the history; one entered exactly at the change is.
/// The failed-exit streak is read over the whole run whatever the window.
#[test]
fn a_round_entered_before_the_rules_change_is_not_counted() {
    crate::config::utils::install_default_config();
    let strategy = changed_at("entry", at(100));
    let evidence = PaperStrategyEvidence::new(
        Some(at(0)),
        vec![
            entered_round("entry", at(10), Some(0.5)),
            entered_round("entry", at(99), Some(0.5)),
            entered_round("entry", at(100), Some(0.01)),
            entered_round("entry", at(150), Some(-0.02)),
            entered_round("entry", at(160), None),
        ],
        HashMap::from([(
            "entry".to_owned(),
            crate::trader::paper::readiness::FAILED_EXIT_LIMIT,
        )]),
    );
    let (rounds, window) = counted_rounds(&strategy, &evidence);
    assert_eq!(window.rules_changed_at, Some(at(100)));
    assert_eq!(
        rounds
            .iter()
            .map(|round| round.entry_time)
            .collect::<Vec<_>>(),
        [at(100), at(150), at(160)]
    );

    let readiness = readiness_of(&strategy, &evidence);
    let check = |id| {
        readiness
            .checks
            .iter()
            .find(|check| check.id == id)
            .unwrap()
    };
    let en: LanguageIdentifier = "en".parse().unwrap();
    assert_eq!(
        check("profit").detail.render_plain(&en),
        "\u{2212}0.0100 SOL realized over 2 rounds, 1 won",
        "the profit of rounds under the old rules is not counted"
    );
    assert!(
        !check("exits").passed,
        "the failed-exit streak is not windowed"
    );

    let progress = readiness_progress(&strategy, &evidence);
    assert_eq!(progress.closed, 2);
    assert_eq!(progress.since_rules_changed, Some(at(100)));
    assert!(!progress.ready);
}

/// The history check names the rules change, with its time as an argument, only when the
/// change falls inside the run; otherwise it keeps the run wording.
#[test]
fn the_history_names_the_rules_change_only_when_it_falls_inside_the_run() {
    crate::config::utils::install_default_config();
    let en: LanguageIdentifier = "en".parse().unwrap();
    let rounds = vec![
        entered_round("entry", at(100), Some(0.01)),
        entered_round("entry", at(150), Some(0.01)),
    ];
    let history = |strategy: &Strategy, run_started_at| {
        let evidence = PaperStrategyEvidence::new(run_started_at, rounds.clone(), HashMap::new());
        readiness_of(strategy, &evidence)
            .checks
            .into_iter()
            .find(|check| check.id == "history")
            .unwrap()
    };

    let since_change = history(&changed_at("entry", at(100)), Some(at(0)));
    let rendered = since_change.detail.render_plain(&en);
    assert_eq!(
        rendered,
        format!(
            "2 of {} closed paper rounds since the rules changed {}",
            min_closed_rounds(),
            at(100).to_rfc3339()
        )
    );
    let met = readiness::history(
        1,
        1,
        readiness::HistoryText {
            met: ids::STRATEGIES_READINESS_HISTORY_SINCE_MET,
            short: ids::STRATEGIES_READINESS_HISTORY_SINCE_SHORT,
            since: Some(at(100)),
        },
    );
    assert!(met.passed);
    assert_eq!(
        met.detail.render_plain(&en),
        format!(
            "1 closed paper round since the rules changed {}, 1 needed",
            at(100).to_rfc3339()
        )
    );

    for (rules_changed_at, run_started_at) in [(at(100), Some(at(100))), (at(0), Some(at(50)))] {
        let whole_run = history(&changed_at("entry", rules_changed_at), run_started_at);
        let rendered = whole_run.detail.render_plain(&en);
        assert!(!rendered.contains("since"), "{rendered}");
        assert_eq!(
            whole_run.detail.render_plain(&en),
            readiness::history(2, min_closed_rounds(), readiness::HistoryText::RUN)
                .detail
                .render_plain(&en)
        );
    }
}
