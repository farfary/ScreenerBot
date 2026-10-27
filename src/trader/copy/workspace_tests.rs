// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Tests the copy workspace: task comparison rows, default settings, paper holdings from
//! the task's rounds, and the earlier paper summaries. Paper maintenance is tested in
//! `workspace/book_tests.rs`.

use super::*;
use crate::trader::copy::round_fixtures::{closed, marked, open_round};
use crate::trader::copy::{CopyTaskStats, ExitMode, SizingMode};
use crate::trader::types::TradeReason;

const ADDRESS: &str = "CopyTarget1111111111111111111111111111111111";

pub(super) fn task(label: Option<&str>) -> CopyTask {
    CopyTask {
        id: 7,
        chain: crate::chains::ChainId::Solana,
        target_address: ADDRESS.to_owned(),
        label: label.map(str::to_owned),
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

#[test]
fn an_unnamed_task_compares_by_its_full_address() {
    let row = comparison(&task(None), CopyInsights::default());
    assert_eq!(row.label, None);
    assert_eq!(row.target_address, ADDRESS);

    let json = serde_json::to_value(&row).expect("comparison serializes");
    assert!(json.get("name").is_none(), "no pre-shortened display name");
    assert_eq!(json["target_address"], ADDRESS);
}

#[test]
fn a_named_task_compares_by_its_label() {
    let row = comparison(&task(Some("Momentum desk")), CopyInsights::default());
    assert_eq!(row.label.as_deref(), Some("Momentum desk"));
    assert_eq!(row.target_address, ADDRESS);
}

/// A paper task's summary with empty stats, for the workspace and readiness tests.
pub(super) fn summary_fixture() -> CopyTaskSummary {
    CopyTaskSummary {
        task: task(None),
        stats: CopyTaskStats::default(),
        spent_native: 0.0,
        remaining_budget_native: 5.0,
        effective_state: "running",
        effective_require_filter_pass: false,
        effective_copy_stable_quoted: false,
        pnl_trend: Vec::new(),
        pause_text: None,
        exit_policy_issue: None,
    }
}

fn archived(mint: &str, last_price: Option<f64>) -> ArchivedPaperPosition {
    ArchivedPaperPosition {
        task_id: 7,
        mint: mint.to_owned(),
        token_amount: 10.0,
        cost_basis_native: 2.0,
        realized_proceeds_native: 1.0,
        realized_cost_native: 0.5,
        last_price_native: last_price,
        last_price_at: last_price.map(|_| Utc::now()),
        opened_at: Utc::now(),
        closed_at: None,
    }
}

#[test]
fn legacy_workspace_values_use_only_saved_paper_estimates() {
    let summary = legacy_paper_summary(&[archived("legacy-mint", Some(0.3))], 2.5);
    assert_eq!(summary.spent_native, 2.5);
    assert_eq!(summary.realized_pnl_native, 0.5);
    assert_eq!(summary.holdings[0].last_recorded_price_native, Some(0.3));
    assert_eq!(
        summary.holdings[0].remaining_pnl_at_last_recorded_price_native,
        Some(1.0)
    );
}

#[test]
fn the_workspace_shows_the_archived_book_and_the_estimates_apart_and_read_only() {
    crate::config::utils::install_default_config();
    let earlier = EarlierPaper {
        archived: vec![
            archived("archived-mint", Some(0.25)),
            archived("archived-two", None),
        ],
        archived_spent_native: 4.0,
        legacy: vec![archived("legacy-mint", Some(0.3))],
        legacy_spent_native: 2.5,
    };
    let workspace = build_workspace(
        summary_fixture(),
        0.0,
        &[],
        &TaskRounds::default(),
        earlier,
        0,
        None,
        Utc::now(),
    );
    assert_eq!(workspace.archived_paper.holdings.len(), 2);
    assert_eq!(workspace.archived_paper.spent_native, 4.0);
    assert_eq!(
        workspace.archived_paper.holdings[1].remaining_pnl_at_last_recorded_price_native,
        None
    );
    assert_eq!(workspace.legacy_paper.holdings.len(), 1);
    assert_eq!(workspace.legacy_paper.spent_native, 2.5);
    assert!(workspace.paper_holdings.is_empty());
}

#[test]
fn paper_holdings_are_the_task_rounds_of_the_paper_book_marked_at_their_latest_price() {
    crate::config::utils::install_default_config();
    let now = Utc::now();
    let mut priced = marked(
        open_round(7, Book::Paper, "priced", 0.01, 0.5, 30),
        Some(0.012),
    );
    priced.basis_peak_price = 0.013;
    priced.dca_count = 1;
    priced.partial_exit_count = 1;
    priced.unrealized_pnl_percent = Some(20.0);
    let unpriced = marked(open_round(7, Book::Paper, "unpriced", 0.01, 0.4, 20), None);
    let done = closed(
        open_round(7, Book::Paper, "closed", 0.01, 0.3, 90),
        0.1,
        TradeReason::StopLoss,
        now,
    );
    let live = marked(open_round(7, Book::Live, "live", 0.01, 0.5, 10), Some(0.02));
    let rounds = TaskRounds {
        positions: vec![priced.clone(), unpriced, done, live],
        run_start: Some(now - Duration::hours(2)),
    };
    let workspace = build_workspace(
        summary_fixture(),
        0.0,
        &[],
        &rounds,
        EarlierPaper::default(),
        0,
        None,
        now,
    );
    let holdings = &workspace.paper_holdings;
    assert_eq!(holdings.len(), 3, "the live round is not a paper holding");
    let first = &holdings[0];
    assert_eq!(first.position_id, priced.id.unwrap());
    assert!(first.open);
    assert_eq!(first.entry_price_native, Some(0.01));
    assert_eq!(first.mark_price_native, Some(0.012));
    assert_eq!(first.peak_price_native, Some(0.013));
    assert_eq!(first.cost_basis_native, 0.5);
    assert!(first
        .unrealized_pnl_native
        .is_some_and(|pnl| (pnl - 0.1).abs() < 1e-9));
    assert_eq!(first.unrealized_pnl_pct, Some(20.0));
    assert_eq!((first.buys, first.sells), (2, 1));
    assert_eq!(first.opened_at, priced.entry_time);
    let unpriced = &holdings[1];
    assert_eq!(unpriced.mark_price_native, None);
    assert_eq!(unpriced.unrealized_pnl_native, None);
    let closed = &holdings[2];
    assert!(!closed.open);
    assert_eq!(closed.entry_price_native, None);
    assert_eq!(closed.mark_price_native, None);
    assert!((closed.realized_pnl_native - 0.1).abs() < 1e-9);
    assert!(closed.exit_watch.is_none());
    let priced_check = workspace
        .readiness
        .checks
        .iter()
        .find(|check| check.id == "priced")
        .expect("a priced check");
    assert!(!priced_check.passed, "one open round has no price");
}

#[test]
fn a_live_task_shows_no_paper_holdings() {
    crate::config::utils::install_default_config();
    let mut summary = summary_fixture();
    summary.task.mode = TradingMode::Live;
    let rounds = TaskRounds {
        positions: vec![marked(
            open_round(7, Book::Paper, "paper", 0.01, 0.5, 10),
            Some(0.02),
        )],
        run_start: None,
    };
    let workspace = build_workspace(
        summary,
        0.0,
        &[],
        &rounds,
        EarlierPaper::default(),
        0,
        None,
        Utc::now(),
    );
    assert!(workspace.paper_holdings.is_empty());
}

#[test]
fn copy_defaults_fee_rate_uses_the_chain_adapter_policy() {
    crate::config::utils::install_default_config();
    let defaults = super::defaults();
    assert_eq!(
        defaults.swap_fee_pct,
        f64::from(crate::chains::platform_swap_fee_bps()) / 100.0
    );
}

/// Every exit rule a copy task can run shows on its holdings: with only that rule on, the
/// holding's exit watch names at least one price or moment. A new `ExitRule` variant fails
/// to compile here until it says what its watch shows.
#[test]
fn every_exit_rule_alone_shows_on_the_holding_watch() {
    use crate::trader::evaluators::exit_rules::ExitRule;

    crate::config::utils::install_default_config();
    let now = Utc::now();
    let inputs = watch_inputs(0.1, 0.1);
    let mut none = ExitPolicy::from_config();
    none.stop_loss.enabled = false;
    none.trailing.enabled = false;
    none.break_even.enabled = false;
    none.take_profit.enabled = false;
    none.time.enabled = false;
    none.time_stagnation.enabled = false;
    let shows_something = |policy: &ExitPolicy| {
        let watch = serde_json::to_value(exit_watch(&inputs, now, policy)).expect("serializes");
        watch
            .as_object()
            .expect("an object")
            .values()
            .any(|value| !value.is_null() && *value != serde_json::Value::Bool(false))
    };
    assert!(!shows_something(&none), "an all-off policy shows nothing");

    for rule in [
        ExitRule::StopLoss,
        ExitRule::Trailing,
        ExitRule::BreakEven,
        ExitRule::TakeProfit { level: 1 },
        ExitRule::TimeLoss,
        ExitRule::TimeStagnation,
    ] {
        let mut policy = none.clone();
        match rule {
            ExitRule::StopLoss => {
                policy.stop_loss.enabled = true;
                policy.stop_loss.threshold_pct = 20.0;
            }
            ExitRule::Trailing => {
                policy.trailing.enabled = true;
                policy.trailing.activation_pct = 15.0;
                policy.trailing.distance_pct = 5.0;
            }
            ExitRule::BreakEven => {
                policy.break_even.enabled = true;
                policy.break_even.trigger_pct = 15.0;
                policy.break_even.offset_pct = 1.0;
            }
            ExitRule::TakeProfit { .. } => {
                policy.take_profit.enabled = true;
                policy.take_profit.levels = vec![TakeProfitLevel::sell_all_at(30.0)];
            }
            ExitRule::TimeLoss => {
                policy.time.enabled = true;
                policy.time.duration_seconds = 3600.0;
                policy.time.loss_threshold_pct = -10.0;
            }
            ExitRule::TimeStagnation => {
                policy.time_stagnation.enabled = true;
                policy.time_stagnation.duration_seconds = 3600.0;
                policy.time_stagnation.band_pct = 5.0;
            }
        }
        assert!(shows_something(&policy), "{rule:?} alone shows nothing");
    }
}

/// The exit inputs of a round entered at `entry` whose basis peak is `peak`.
fn watch_inputs(entry: f64, peak: f64) -> ExitInputs {
    ExitInputs {
        average_entry_price: entry,
        current_price: entry,
        basis_peak_price: peak,
        held_seconds: 0,
        partial_exits_taken: false,
        take_profit_levels_taken: 0,
    }
}

#[test]
fn the_holding_watch_places_the_break_even_floor_and_the_flat_band() {
    crate::config::utils::install_default_config();
    let now = Utc::now();
    let inputs = watch_inputs(1.0, 1.2);
    let mut policy = ExitPolicy::from_config();
    policy.break_even.enabled = true;
    policy.break_even.trigger_pct = 15.0;
    policy.break_even.offset_pct = 1.0;
    policy.time_stagnation.enabled = true;
    policy.time_stagnation.duration_seconds = 4.0 * 3600.0;
    policy.time_stagnation.band_pct = 5.0;

    let watch = exit_watch(&inputs, now, &policy);
    let close = |value: Option<f64>, expected: f64| {
        value.is_some_and(|value| (value - expected).abs() < 1e-9)
    };
    assert!(close(watch.break_even_activation_price_native, 1.15));
    assert!(watch.break_even_armed, "a peak of +20% arms a +15% trigger");
    assert!(close(watch.break_even_floor_price_native, 1.01));
    assert_eq!(watch.stagnation_from, Some(now + Duration::hours(4)));
    assert!(close(watch.stagnation_band_low_price_native, 0.95));
    assert!(close(watch.stagnation_band_high_price_native, 1.05));

    assert!(!exit_watch(&watch_inputs(1.0, 1.1), now, &policy).break_even_armed);
}

#[test]
fn the_holding_watch_names_the_next_take_profit_step_of_the_round() {
    crate::config::utils::install_default_config();
    let mut policy = ExitPolicy::from_config();
    policy.take_profit.enabled = true;
    policy.take_profit.levels = vec![
        TakeProfitLevel::sell_all_at(30.0),
        TakeProfitLevel::sell_all_at(60.0),
    ];
    let mut inputs = watch_inputs(1.0, 1.0);
    let first = exit_watch(&inputs, Utc::now(), &policy).take_profit_price_native;
    assert!(first.is_some_and(|price| (price - 1.3).abs() < 1e-9));
    inputs.take_profit_levels_taken = 1;
    let second = exit_watch(&inputs, Utc::now(), &policy).take_profit_price_native;
    assert!(second.is_some_and(|price| (price - 1.6).abs() < 1e-9));
    inputs.take_profit_levels_taken = 2;
    assert_eq!(
        exit_watch(&inputs, Utc::now(), &policy).take_profit_price_native,
        None
    );
}
