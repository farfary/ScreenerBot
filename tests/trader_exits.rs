// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Pure: the five built-in exit rules — stop loss, trailing stop, take-profit ladder,
//! time override, and the emergency risk limit.
//!
//! These are the rules that decide when to sell. Getting one wrong is expensive in both
//! directions: too eager and it dumps a healthy position, too reluctant and it rides a
//! loss to zero. What is tested here is exactly the arithmetic and the gating, with no
//! network and no database, through the seam the exit evaluator itself uses: the live
//! book's inputs for a `Position` and a price, the ordered rules, and the sell decision
//! the rule that fired asks for. Each test enables only the rule it exercises.
//!
//! Every test holds [`common::config_guard`], which serialises writes to the global
//! config and resets it to defaults on acquire, so these run correctly under both
//! `cargo nextest run` (process per test) and plain `cargo test` (threads in one
//! process).

// The config guard is a plain `std::sync::Mutex` held for the body of an async test.
// That is deliberate: the guard must also work for synchronous `#[test]` functions, so
// it cannot be a `tokio::sync::Mutex`. It cannot deadlock here — each test's future is
// driven to completion by its own `block_on`, and no task inside that runtime ever
// contends for the guard.
#![allow(clippy::await_holding_lock)]

use crate::common;

use chrono::{Duration, Utc};
use common::{aged, config_guard, set_config, test_position};
use screenerbot::positions::{Position, PositionManagement};
use screenerbot::trader::evaluators::exit::{live_exit_inputs, rule_decision};
use screenerbot::trader::evaluators::exit_rules::{self, Decision, ExitRule};
use screenerbot::trader::policy::ExitPolicy;
use screenerbot::trader::safety::check_risk_limits;
use screenerbot::trader::{TradeAction, TradeDecision, TradePriority, TradeReason};

/// The configured exit policy with every built-in rule but `rule` switched off.
fn only(rule: ExitRule) -> ExitPolicy {
    let mut policy = ExitPolicy::from_config();
    policy.stop_loss.enabled &= rule == ExitRule::StopLoss;
    policy.trailing.enabled &= rule == ExitRule::Trailing;
    policy.take_profit.enabled &= matches!(rule, ExitRule::TakeProfit { .. });
    policy.time.enabled &= rule == ExitRule::TimeLoss;
    policy
}

/// What the ordered rules find for `position` at `price` now, with only `rule` enabled.
fn evaluate(position: &Position, price: f64, rule: ExitRule) -> Decision {
    exit_rules::decide(&live_exit_inputs(position, price, Utc::now()), &only(rule))
}

/// The sell the exit evaluator asks for `position` at `price` when only `rule` may fire;
/// the rule that fires must be `rule` itself, take-profit step included.
fn sell(position: &Position, price: f64, rule: ExitRule) -> Option<TradeDecision> {
    evaluate(position, price, rule).outcome.map(|outcome| {
        assert_eq!(outcome.rule, rule, "another rule fired");
        rule_decision(position, price, &outcome)
    })
}

/// Whether the evaluation of `rule` neither fires nor reports a configuration error.
fn fires_nothing(position: &Position, price: f64, rule: ExitRule) -> bool {
    let decision = evaluate(position, price, rule);
    decision.outcome.is_none() && decision.rule_errors.is_empty()
}

/// The configuration error `rule` reported for `position` at `price`.
fn rule_error(position: &Position, price: f64, rule: ExitRule) -> String {
    let decision = evaluate(position, price, rule);
    assert!(decision.outcome.is_none(), "an impossible rule fired");
    decision
        .rule_errors
        .iter()
        .find(|(failed, _)| *failed == rule)
        .map(|(_, error)| error.to_string())
        .expect("the rule reports its configuration error")
}

#[test]
fn position_management_matches_the_exit_ownership_matrix() {
    let cases = [
        (PositionManagement::AutoTrader, true, true, true, false),
        (PositionManagement::CopyTask, true, false, false, true),
        (PositionManagement::Hybrid, true, true, false, true),
        (PositionManagement::UserOnly, false, false, false, false),
    ];
    for (management, safety, policy, dca, copy_sell) in cases {
        assert_eq!(
            management.allows_safety_exits(),
            safety,
            "{management:?} safety"
        );
        assert_eq!(
            management.allows_policy_exits(),
            policy,
            "{management:?} policy"
        );
        assert_eq!(management.allows_auto_dca(), dca, "{management:?} DCA");
        assert_eq!(
            management.allows_copy_sell(),
            copy_sell,
            "{management:?} copy sell"
        );
    }
}

// ==================== STOP LOSS ====================

/// Enable the stop loss at `threshold_pct`, with no minimum hold and full exits.
fn enable_stop_loss(threshold_pct: f64) {
    set_config(|cfg| {
        cfg.trader.stop_loss_enabled = true;
        cfg.trader.stop_loss_threshold_pct = threshold_pct;
        cfg.trader.stop_loss_min_hold_seconds = 0;
        cfg.trader.stop_loss_allow_partial = false;
    });
}

#[test]
fn stop_loss_fires_once_the_loss_reaches_the_threshold() {
    let _cfg = config_guard();
    enable_stop_loss(25.0);

    let position = test_position(1.0, 1.0);
    // Exactly 25% down: `loss_pct >= threshold` includes the boundary. (0.75 is chosen
    // because it is exactly representable in binary floating point — 0.80 against a 20%
    // threshold computes to 19.999999999999996 and would test rounding, not the rule.)
    let decision =
        sell(&position, 0.75, ExitRule::StopLoss).expect("stop loss must fire at the threshold");

    assert_eq!(decision.action, TradeAction::Sell);
    assert_eq!(decision.reason, TradeReason::StopLoss);
    assert_eq!(decision.priority, TradePriority::High);
    assert_eq!(decision.mint, position.mint);
    assert_eq!(decision.price_native, Some(0.75));
    // A sell's size is carried by `exit_percentage`; `size_sol` is a BUY concept and
    // must stay None or it would be read as SOL to spend.
    assert!(decision.size_native.is_none());
}

#[test]
fn stop_loss_holds_above_the_threshold() {
    let _cfg = config_guard();
    enable_stop_loss(25.0);

    let position = test_position(1.0, 1.0);
    assert!(sell(&position, 0.76, ExitRule::StopLoss).is_none());
}

#[test]
fn stop_loss_measures_from_the_average_entry_not_the_first_buy() {
    // After averaging down, the stop must be measured against what the position
    // actually cost. Using the original entry would keep a DCA'd position under water
    // long past its stop.
    let _cfg = config_guard();
    enable_stop_loss(25.0);

    let mut position = test_position(1.0, 1.0);
    position.average_entry_price = 0.8; // averaged in lower
    position.dca_count = 1;

    // 0.7 is 12.5% below the AVERAGE (no stop) but 30% below the first entry, which
    // WOULD have stopped out if the rule read `entry_price`.
    assert!(
        sell(&position, 0.7, ExitRule::StopLoss).is_none(),
        "stop loss must use average_entry_price"
    );
    // 0.6 is exactly 25% below the average.
    assert!(sell(&position, 0.6, ExitRule::StopLoss).is_some());
}

#[test]
fn stop_loss_respects_the_minimum_hold_time() {
    // The min-hold window exists so a token's first volatile minutes do not stop out a
    // position instantly. It must gate on position AGE, not on the loss.
    let _cfg = config_guard();
    enable_stop_loss(20.0);
    set_config(|cfg| cfg.trader.stop_loss_min_hold_seconds = 300);

    let fresh = test_position(1.0, 1.0);
    assert!(
        sell(&fresh, 0.1, ExitRule::StopLoss).is_none(),
        "a brand new position is inside the hold window"
    );

    let held = aged(test_position(1.0, 1.0), Duration::seconds(301));
    assert!(sell(&held, 0.1, ExitRule::StopLoss).is_some());
}

#[test]
fn stop_loss_is_off_when_disabled() {
    let _cfg = config_guard();
    set_config(|cfg| cfg.trader.stop_loss_enabled = false);

    let position = test_position(1.0, 1.0);
    assert!(sell(&position, 0.01, ExitRule::StopLoss).is_none());
}

#[test]
fn a_partial_stop_loss_may_be_taken_exactly_once() {
    // The trigger is entry-vs-price and selling moves neither, so a repeating partial
    // stop loss would sell a share of the remainder on every monitor cycle, bleeding
    // the position out in a geometric series and paying fees on each leg. If the price
    // is STILL under the stop after exposure was cut once, exit fully.
    let _cfg = config_guard();
    enable_stop_loss(20.0);
    set_config(|cfg| {
        cfg.trader.stop_loss_allow_partial = true;
        cfg.trader.stop_loss_partial_pct = 25.0;
    });

    let first = test_position(1.0, 1.0);
    let decision = sell(&first, 0.5, ExitRule::StopLoss).expect("first stop loss fires");
    assert_eq!(
        decision.exit_percentage,
        Some(25.0),
        "the first stop loss takes the configured partial"
    );

    let mut after_one = test_position(1.0, 1.0);
    after_one.partial_exit_count = 1;
    let decision = sell(&after_one, 0.5, ExitRule::StopLoss).expect("second stop loss fires");
    assert_eq!(
        decision.exit_percentage, None,
        "after one partial the stop loss must exit fully"
    );
}

#[test]
fn a_full_stop_loss_carries_no_exit_percentage() {
    let _cfg = config_guard();
    enable_stop_loss(20.0);

    let decision =
        sell(&test_position(1.0, 1.0), 0.5, ExitRule::StopLoss).expect("the stop loss fires");
    assert_eq!(decision.exit_percentage, None);
}

#[test]
fn stop_loss_never_fires_on_an_unusable_price() {
    let _cfg = config_guard();
    enable_stop_loss(20.0);
    let position = test_position(1.0, 1.0);

    for bad in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert!(
            fires_nothing(&position, bad, ExitRule::StopLoss),
            "price {bad} must never be read as a loss"
        );
    }
}

#[test]
fn stop_loss_never_fires_on_an_unusable_entry_price() {
    let _cfg = config_guard();
    enable_stop_loss(20.0);

    for bad in [0.0, -1.0, f64::NAN] {
        let mut position = test_position(1.0, 1.0);
        position.average_entry_price = bad;
        assert!(
            fires_nothing(&position, 0.5, ExitRule::StopLoss),
            "entry price {bad} must never be read as a loss"
        );
    }
}

// ==================== TRAILING STOP ====================

fn enable_trailing(activation_pct: f64, distance_pct: f64) {
    set_config(|cfg| {
        cfg.trader.trailing_stop_enabled = true;
        cfg.trader.trailing_stop_activation_pct = activation_pct;
        cfg.trader.trailing_stop_distance_pct = distance_pct;
    });
}

/// A position entered at 1.0 whose price peaked at `peak`, with no add since the entry.
fn peaked_at(peak: f64) -> Position {
    let mut position = test_position(1.0, 1.0);
    position.price_highest = peak;
    position.basis_peak_price = peak;
    position
}

/// Entered at 1.00, peaked at 1.09, then averaged down by an add at 0.85 to an average
/// entry of 0.944, and recovered to `basis_peak` since that add.
fn averaged_down_after_a_peak(basis_peak: f64) -> Position {
    let mut position = test_position(1.0, 1.0);
    position.average_entry_price = 0.944;
    position.price_highest = 1.09;
    position.basis_peak_price = basis_peak;
    position.dca_count = 1;
    position
}

#[test]
fn trailing_stop_measures_from_the_peak_since_the_last_add() {
    // The round peaked at 1.09 before the add: +15.5% over the new 0.944 average, which
    // would arm a 10% activation and set a 5% trail at 1.0355. A recovery to 0.99 would
    // then sell near break-even on a profit the averaged position never made. Since the
    // add the price only reached 0.99 (+4.9%), so the trail is not armed.
    let _cfg = config_guard();
    enable_trailing(10.0, 5.0);

    let position = averaged_down_after_a_peak(0.99);
    assert!(
        sell(&position, 0.98, ExitRule::Trailing).is_none(),
        "a peak from before the add must not arm the trail"
    );

    // The same figures with the pre-add peak as the basis do arm and fire: the peak the
    // trail reads is what decides it.
    let stale = averaged_down_after_a_peak(1.09);
    assert!(sell(&stale, 0.98, ExitRule::Trailing).is_some());
}

#[test]
fn trailing_stop_arms_on_a_new_peak_after_the_add() {
    // Since the add the price ran to 1.10, +16.5% over the 0.944 average: armed, stop at
    // 1.045, and 1.04 is under it while still in profit.
    let _cfg = config_guard();
    enable_trailing(10.0, 5.0);

    let position = averaged_down_after_a_peak(1.10);
    let decision = sell(&position, 1.04, ExitRule::Trailing)
        .expect("a peak reached after the add arms the trail");
    assert_eq!(decision.reason, TradeReason::TrailingStop);
}

#[test]
fn trailing_stop_does_nothing_before_activation() {
    // The PEAK never reached the activation profit, so the trail was never armed.
    let _cfg = config_guard();
    enable_trailing(50.0, 10.0);

    let position = peaked_at(1.2); // best it ever did was +20%, activation needs +50%
    assert!(sell(&position, 1.05, ExitRule::Trailing).is_none());
}

#[test]
fn trailing_stop_arms_off_the_peak_not_the_current_price() {
    // The bug this pins down: activation used to be measured against the CURRENT
    // price, so a trail that had armed at the peak silently DIS-armed as the price
    // fell — precisely when it is supposed to act.
    //
    // Peak 1.25 is +25%, past the 20% activation. Distance 10% puts the stop at
    // 1.125. A current price of 1.12 is below that stop and still +12% in profit, so
    // the trail must take the remaining gain. Reading activation off the current price
    // sees +12% < 20%, stays silent, and hands the whole run back.
    let _cfg = config_guard();
    enable_trailing(20.0, 10.0);

    let position = peaked_at(1.25);
    let decision = sell(&position, 1.12, ExitRule::Trailing)
        .expect("an armed trail must fire on a retrace through its stop");
    assert_eq!(decision.reason, TradeReason::TrailingStop);
}

#[test]
fn trailing_stop_fires_once_armed_and_the_peak_is_given_back() {
    let _cfg = config_guard();
    enable_trailing(20.0, 10.0);

    // Peak 2.0, distance 10% -> stop at 1.80. Price 1.79 is armed (+79% profit) and
    // below the stop.
    let position = peaked_at(2.0);
    let decision = sell(&position, 1.79, ExitRule::Trailing).expect("trailing stop must fire");
    assert_eq!(decision.reason, TradeReason::TrailingStop);
    assert_eq!(decision.priority, TradePriority::High);
    assert!(decision.exit_percentage.is_none(), "trailing sells it all");
}

#[test]
fn trailing_stop_holds_while_the_price_is_above_the_stop() {
    let _cfg = config_guard();
    enable_trailing(20.0, 10.0);

    // Peak 2.0 -> stop 1.80. At exactly the stop it fires (`<=`); just above it holds.
    let position = peaked_at(2.0);
    assert!(sell(&position, 1.81, ExitRule::Trailing).is_none());
    assert!(sell(&position, 1.80, ExitRule::Trailing).is_some());
}

#[test]
fn trailing_stop_never_exits_at_a_loss() {
    // A position that ran up and then fell all the way back THROUGH the entry must be
    // left to the STOP LOSS, which is the rule that owns losses. Trailing exists to
    // protect a profit; taking it at a loss would book one the user never accepted.
    //
    // Peak 1.21 (+21%) arms the 20% activation. A 19% distance puts the stop at
    // 0.9801, BELOW the 1.0 entry — so a price of 0.98 is under the stop and yet at a
    // 2% loss. Only the profitability guard stops that sale.
    let _cfg = config_guard();
    enable_trailing(20.0, 19.0);

    let position = peaked_at(1.21);
    assert!(
        sell(&position, 0.98, ExitRule::Trailing).is_none(),
        "trailing stop must not fire below the entry price"
    );
}

#[test]
fn trailing_stop_does_nothing_without_a_recorded_peak() {
    let _cfg = config_guard();
    enable_trailing(20.0, 10.0);

    let position = peaked_at(0.0);
    assert!(sell(&position, 5.0, ExitRule::Trailing).is_none());
}

#[test]
fn trailing_stop_rejects_a_config_whose_distance_swallows_the_activation() {
    // distance >= activation is unsatisfiable: the stop sits at or below the entry, so
    // the "still profitable" guard can never pass and the trail would silently never
    // fire. Surfacing it as an error is what makes the misconfiguration visible.
    let _cfg = config_guard();
    enable_trailing(10.0, 10.0);

    let position = peaked_at(2.0);
    let err = rule_error(&position, 1.5, ExitRule::Trailing);
    assert!(err.to_string().contains("must be less than"), "got: {err}");
}

#[test]
fn trailing_stop_is_off_when_disabled() {
    let _cfg = config_guard();
    set_config(|cfg| cfg.trader.trailing_stop_enabled = false);

    let position = peaked_at(2.0);
    assert!(sell(&position, 1.0, ExitRule::Trailing).is_none());
}

#[test]
fn trailing_stop_never_fires_on_an_unusable_price() {
    let _cfg = config_guard();
    enable_trailing(20.0, 10.0);
    let position = peaked_at(2.0);

    for bad in [0.0, -1.0, f64::NAN] {
        assert!(fires_nothing(&position, bad, ExitRule::Trailing));
    }
}

// ==================== TAKE PROFIT ====================

/// The first step of the take-profit ladder, the one a position with no step taken sells.
const FIRST_STEP: ExitRule = ExitRule::TakeProfit { level: 1 };

/// Enable a one-step take-profit ladder that sells the whole position at `profit_pct`.
fn enable_take_profit(profit_pct: f64) {
    enable_take_profit_ladder(&[profit_pct], &[100.0]);
}

/// Enable a take-profit ladder of `levels_pct` paired with `sell_pct`.
fn enable_take_profit_ladder(levels_pct: &[f64], sell_pct: &[f64]) {
    set_config(|cfg| {
        cfg.trader.take_profit_enabled = true;
        cfg.trader.take_profit_levels_pct = levels_pct.to_vec();
        cfg.trader.take_profit_sell_pct = sell_pct.to_vec();
    });
}

#[test]
fn take_profit_fires_at_the_target() {
    let _cfg = config_guard();
    enable_take_profit(50.0);

    let position = test_position(1.0, 1.0);
    let decision = sell(&position, 1.50, FIRST_STEP).expect("the profit target is reached");
    assert_eq!(decision.reason, TradeReason::TakeProfit);
    assert_eq!(decision.priority, TradePriority::Normal);
    assert!(decision.exit_percentage.is_none());
    assert_eq!(decision.take_profit_level, Some(1));
}

#[test]
fn take_profit_holds_below_the_target() {
    let _cfg = config_guard();
    enable_take_profit(50.0);

    let position = test_position(1.0, 1.0);
    assert!(sell(&position, 1.49, FIRST_STEP).is_none());
}

#[test]
fn take_profit_measures_against_the_average_entry() {
    // Averaging down lowers the bar for the profit target, because the target is on
    // the position's real cost, not on the first buy.
    let _cfg = config_guard();
    enable_take_profit(50.0);

    let mut position = test_position(1.0, 1.0);
    position.average_entry_price = 0.5;
    assert!(sell(&position, 0.75, FIRST_STEP).is_some());
}

#[test]
fn take_profit_is_off_when_disabled() {
    let _cfg = config_guard();
    set_config(|cfg| cfg.trader.take_profit_enabled = false);

    assert!(sell(&test_position(1.0, 1.0), 100.0, FIRST_STEP).is_none());
}

#[test]
fn take_profit_never_fires_on_an_unusable_price() {
    let _cfg = config_guard();
    enable_take_profit(50.0);
    let position = test_position(1.0, 1.0);

    for bad in [0.0, -1.0, f64::NAN] {
        assert!(fires_nothing(&position, bad, FIRST_STEP));
    }
}

#[test]
fn the_default_ladder_sells_everything_past_twenty_percent() {
    // The shipped ladder is one step that sells the whole position at +20%, the
    // single profit target it replaces. (1.25 is exactly representable; 1.20 computes
    // to 19.999999999999996% and would test rounding, not the rule.)
    let _cfg = config_guard();
    set_config(|cfg| cfg.trader.take_profit_enabled = true);

    let position = test_position(1.0, 1.0);
    assert!(sell(&position, 1.19, FIRST_STEP).is_none());
    let decision = sell(&position, 1.25, FIRST_STEP).expect("+25% passes the default step");
    assert!(
        decision.exit_percentage.is_none(),
        "the default step sells it all"
    );
    assert_eq!(decision.take_profit_level, Some(1));
}

#[test]
fn a_one_step_ladder_whose_close_sold_nothing_takes_profit_again() {
    // A full step reserves nothing: a close that ended without a sale, or that sold only
    // part of the holding and left a residual to retry, leaves the ladder where it was,
    // so the next evaluation fires the same step while the price holds above its level.
    let _cfg = config_guard();
    enable_take_profit(20.0);

    let position = test_position(1.0, 1.0);
    let first = sell(&position, 1.25, FIRST_STEP).expect("the step is due");
    assert!(first.exit_percentage.is_none(), "the one step sells it all");
    assert_eq!(first.take_profit_level, Some(1));

    let mut failed_attempt = position.clone();
    failed_attempt.exit_transaction_signature = None;
    let mut residual_left = position.clone();
    residual_left.partial_exit_count = 1;
    for after in [failed_attempt, residual_left] {
        let again = sell(&after, 1.25, FIRST_STEP).expect("the step fires again");
        assert!(again.exit_percentage.is_none());
        assert_eq!(again.take_profit_level, Some(1));
    }
}

#[test]
fn each_ladder_step_sells_its_share_of_what_is_left() {
    // 30% / 30% / 40% of the original position: the first step sells 30% of the
    // holding, the second 30 of the 70 left (42.857%), the last whatever remains.
    let _cfg = config_guard();
    enable_take_profit_ladder(&[20.0, 50.0, 100.0], &[30.0, 30.0, 40.0]);

    let mut position = test_position(1.0, 1.0);
    let first = sell(&position, 1.25, FIRST_STEP).expect("the first step is due");
    assert_eq!(first.take_profit_level, Some(1));
    assert!((first.exit_percentage.expect("a partial step") - 30.0).abs() < 1e-9);

    position.take_profit_level = 1;
    assert!(
        sell(&position, 1.25, ExitRule::TakeProfit { level: 2 }).is_none(),
        "a taken step never fires again"
    );
    let second =
        sell(&position, 1.50, ExitRule::TakeProfit { level: 2 }).expect("the second step is due");
    assert_eq!(second.take_profit_level, Some(2));
    assert!((second.exit_percentage.expect("a partial step") - 300.0 / 7.0).abs() < 1e-9);

    position.take_profit_level = 2;
    let last =
        sell(&position, 2.0, ExitRule::TakeProfit { level: 3 }).expect("the last step is due");
    assert_eq!(last.take_profit_level, Some(3));
    assert!(
        last.exit_percentage.is_none(),
        "the last step sells what is left"
    );

    position.take_profit_level = 3;
    assert!(
        fires_nothing(&position, 10.0, ExitRule::TakeProfit { level: 4 }),
        "a finished ladder sells nothing more"
    );
}

#[test]
fn a_price_past_several_levels_fires_only_the_next_step() {
    // One step per evaluation: a jump through every level sells the first step's share,
    // and the next evaluation the next one.
    let _cfg = config_guard();
    enable_take_profit_ladder(&[20.0, 50.0], &[50.0, 50.0]);

    let position = test_position(1.0, 1.0);
    let decision = sell(&position, 3.0, FIRST_STEP).expect("the first step is due");
    assert_eq!(decision.take_profit_level, Some(1));
    assert!((decision.exit_percentage.expect("a partial step") - 50.0).abs() < 1e-9);
}

#[test]
fn a_ladder_that_keeps_a_runner_never_sells_it() {
    // Two steps of 25% leave half the position to the other rules.
    let _cfg = config_guard();
    enable_take_profit_ladder(&[20.0, 50.0], &[25.0, 25.0]);

    let mut position = test_position(1.0, 1.0);
    position.take_profit_level = 1;
    let second =
        sell(&position, 1.5, ExitRule::TakeProfit { level: 2 }).expect("the second step is due");
    assert!(
        (second.exit_percentage.expect("the runner is kept") - 100.0 / 3.0).abs() < 1e-9,
        "25 of the 75 left"
    );

    position.take_profit_level = 2;
    assert!(fires_nothing(
        &position,
        10.0,
        ExitRule::TakeProfit { level: 3 }
    ));
}

// ==================== TIME OVERRIDE ====================

fn enable_time_override(duration_hours: f64, loss_threshold_pct: f64) {
    set_config(|cfg| {
        cfg.trader.time_override_enabled = true;
        cfg.trader.time_override_duration = duration_hours;
        cfg.trader.time_override_unit = "hours".to_owned();
        cfg.trader.time_override_loss_threshold_percent = loss_threshold_pct;
    });
}

#[test]
fn time_override_needs_both_the_age_and_the_loss() {
    // It exists to cut positions that have been stuck at a loss, so BOTH conditions
    // must hold — an old winner and a fresh loser must both be left alone.
    let _cfg = config_guard();
    enable_time_override(24.0, -40.0);

    let old_and_down = aged(test_position(1.0, 1.0), Duration::hours(25));
    assert!(sell(&old_and_down, 0.5, ExitRule::TimeLoss).is_some());

    let old_but_fine = aged(test_position(1.0, 1.0), Duration::hours(25));
    assert!(
        sell(&old_but_fine, 0.9, ExitRule::TimeLoss).is_none(),
        "a 10% loss is inside the 40% tolerance"
    );

    let fresh_and_down = aged(test_position(1.0, 1.0), Duration::hours(1));
    assert!(
        sell(&fresh_and_down, 0.5, ExitRule::TimeLoss).is_none(),
        "a young position has not had its time yet"
    );
}

#[test]
fn time_override_boundary_is_inclusive_on_both_axes() {
    let _cfg = config_guard();
    enable_time_override(24.0, -40.0);

    // Exactly 24h old and exactly 40% down.
    let position = aged(test_position(1.0, 1.0), Duration::hours(24));
    assert!(sell(&position, 0.60, ExitRule::TimeLoss).is_some());
    // One tick shallower is inside tolerance.
    assert!(sell(&position, 0.601, ExitRule::TimeLoss).is_none());
}

#[test]
fn time_override_carries_a_high_priority_full_exit() {
    let _cfg = config_guard();
    enable_time_override(24.0, -40.0);

    let position = aged(test_position(1.0, 1.0), Duration::hours(25));
    let decision = sell(&position, 0.5, ExitRule::TimeLoss).expect("the rule fires");
    assert_eq!(decision.reason, TradeReason::TimeOverride);
    assert_eq!(decision.priority, TradePriority::High);
    assert!(decision.exit_percentage.is_none());
}

#[test]
fn time_override_rejects_a_positive_loss_threshold() {
    // The threshold is stored negative ("exit at 40% down"). A positive value would
    // mean "exit once you are UP 40%", quietly turning a loss-cutter into a
    // take-profit. It must be refused, not obeyed.
    let _cfg = config_guard();
    enable_time_override(24.0, 40.0);

    let position = aged(test_position(1.0, 1.0), Duration::hours(25));
    let err = rule_error(&position, 0.5, ExitRule::TimeLoss);
    assert!(err.to_string().contains("must be <= 0"), "got: {err}");
}

#[test]
fn time_override_rejects_a_non_positive_duration() {
    let _cfg = config_guard();
    enable_time_override(0.0, -40.0);

    let position = aged(test_position(1.0, 1.0), Duration::hours(25));
    assert!(fires_nothing(&position, 0.5, ExitRule::TimeLoss));
}

#[test]
fn time_override_is_off_when_disabled() {
    let _cfg = config_guard();
    set_config(|cfg| cfg.trader.time_override_enabled = false);

    let position = aged(test_position(1.0, 1.0), Duration::days(30));
    assert!(sell(&position, 0.01, ExitRule::TimeLoss).is_none());
}

// ==================== EMERGENCY RISK LIMIT ====================

#[tokio::test]
async fn the_risk_limit_force_exits_a_near_total_loss() {
    // The last-resort guard: at 90% down the position is written off rather than
    // ridden to zero. It has no config toggle by design.
    let _cfg = config_guard();

    let position = test_position(1.0, 1.0);
    let decision = check_risk_limits(&position, 0.10)
        .await
        .unwrap()
        .expect("90% loss must trigger the emergency exit");
    assert_eq!(decision.reason, TradeReason::RiskManagement);
    assert_eq!(decision.priority, TradePriority::Emergency);
    assert!(
        decision.exit_percentage.is_none(),
        "an emergency exit liquidates the position"
    );
}

#[tokio::test]
async fn the_risk_limit_leaves_a_survivable_loss_to_the_other_rules() {
    let _cfg = config_guard();

    let position = test_position(1.0, 1.0);
    assert!(check_risk_limits(&position, 0.11).await.unwrap().is_none());
}

#[tokio::test]
async fn the_risk_limit_measures_against_the_average_entry() {
    let _cfg = config_guard();

    let mut position = test_position(1.0, 1.0);
    position.average_entry_price = 0.1; // averaged down hard
                                        // 0.05 is 50% below the average but 95% below the first buy — not an emergency.
    assert!(check_risk_limits(&position, 0.05).await.unwrap().is_none());
}

#[tokio::test]
async fn the_risk_limit_skips_a_position_with_a_broken_entry_price() {
    // A zero entry price would make the loss percentage infinite and force-liquidate a
    // perfectly healthy position on a data glitch.
    let _cfg = config_guard();

    for bad in [0.0, -1.0, f64::NAN] {
        let mut position = test_position(1.0, 1.0);
        position.average_entry_price = bad;
        assert!(
            check_risk_limits(&position, 0.5).await.unwrap().is_none(),
            "entry price {bad} must not emergency-exit"
        );
    }
}
