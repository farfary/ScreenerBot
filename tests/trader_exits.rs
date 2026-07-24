//! Pure: the five built-in exit rules — stop loss, trailing stop, ROI target, time
//! override, and the emergency risk limit.
//!
//! These are the rules that decide when to sell. Getting one wrong is expensive in both
//! directions: too eager and it dumps a healthy position, too reluctant and it rides a
//! loss to zero. What is tested here is exactly the arithmetic and the gating, with no
//! network and no database — each evaluator takes a `Position` and a price and answers.
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

mod common;

use chrono::Duration;
use common::{aged, config_guard, set_config, test_position};
use screenerbot::positions::Position;
use screenerbot::trader::evaluators::{exit_roi, exit_stop_loss, exit_time, exit_trailing};
use screenerbot::trader::safety::check_risk_limits;
use screenerbot::trader::{TradeAction, TradePriority, TradeReason};

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

#[tokio::test]
async fn stop_loss_fires_once_the_loss_reaches_the_threshold() {
    let _cfg = config_guard();
    enable_stop_loss(25.0);

    let position = test_position(1.0, 1.0);
    // Exactly 25% down: `loss_pct >= threshold` includes the boundary. (0.75 is chosen
    // because it is exactly representable in binary floating point — 0.80 against a 20%
    // threshold computes to 19.999999999999996 and would test rounding, not the rule.)
    let decision = exit_stop_loss::check_stop_loss(&position, 0.75)
        .await
        .expect("evaluation succeeded")
        .expect("stop loss must fire at the threshold");

    assert_eq!(decision.action, TradeAction::Sell);
    assert_eq!(decision.reason, TradeReason::StopLoss);
    assert_eq!(decision.priority, TradePriority::High);
    assert_eq!(decision.mint, position.mint);
    assert_eq!(decision.price_sol, Some(0.75));
    // A sell's size is carried by `exit_percentage`; `size_sol` is a BUY concept and
    // must stay None or it would be read as SOL to spend.
    assert!(decision.size_sol.is_none());
}

#[tokio::test]
async fn stop_loss_holds_above_the_threshold() {
    let _cfg = config_guard();
    enable_stop_loss(25.0);

    let position = test_position(1.0, 1.0);
    assert!(exit_stop_loss::check_stop_loss(&position, 0.76)
        .await
        .unwrap()
        .is_none());
}

#[tokio::test]
async fn stop_loss_measures_from_the_average_entry_not_the_first_buy() {
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
        exit_stop_loss::check_stop_loss(&position, 0.7)
            .await
            .unwrap()
            .is_none(),
        "stop loss must use average_entry_price"
    );
    // 0.6 is exactly 25% below the average.
    assert!(exit_stop_loss::check_stop_loss(&position, 0.6)
        .await
        .unwrap()
        .is_some());
}

#[tokio::test]
async fn stop_loss_respects_the_minimum_hold_time() {
    // The min-hold window exists so a token's first volatile minutes do not stop out a
    // position instantly. It must gate on position AGE, not on the loss.
    let _cfg = config_guard();
    enable_stop_loss(20.0);
    set_config(|cfg| cfg.trader.stop_loss_min_hold_seconds = 300);

    let fresh = test_position(1.0, 1.0);
    assert!(
        exit_stop_loss::check_stop_loss(&fresh, 0.1)
            .await
            .unwrap()
            .is_none(),
        "a brand new position is inside the hold window"
    );

    let held = aged(test_position(1.0, 1.0), Duration::seconds(301));
    assert!(exit_stop_loss::check_stop_loss(&held, 0.1)
        .await
        .unwrap()
        .is_some());
}

#[tokio::test]
async fn stop_loss_is_off_when_disabled() {
    let _cfg = config_guard();
    set_config(|cfg| cfg.trader.stop_loss_enabled = false);

    let position = test_position(1.0, 1.0);
    assert!(exit_stop_loss::check_stop_loss(&position, 0.01)
        .await
        .unwrap()
        .is_none());
}

#[tokio::test]
async fn a_partial_stop_loss_may_be_taken_exactly_once() {
    // The trigger is entry-vs-price and selling moves neither, so a repeating partial
    // stop loss would sell a share of the remainder on every monitor cycle, bleeding
    // the position out in a geometric series and paying fees on each leg. If the price
    // is STILL under the stop after exposure was cut once, exit fully.
    let _cfg = config_guard();
    enable_stop_loss(20.0);
    set_config(|cfg| {
        cfg.trader.stop_loss_allow_partial = true;
        cfg.positions.partial_exit_default_pct = 25.0;
    });

    let first = test_position(1.0, 1.0);
    let decision = exit_stop_loss::check_stop_loss(&first, 0.5)
        .await
        .unwrap()
        .expect("first stop loss fires");
    assert_eq!(
        decision.exit_percentage,
        Some(25.0),
        "the first stop loss takes the configured partial"
    );

    let mut after_one = test_position(1.0, 1.0);
    after_one.partial_exit_count = 1;
    let decision = exit_stop_loss::check_stop_loss(&after_one, 0.5)
        .await
        .unwrap()
        .expect("second stop loss fires");
    assert_eq!(
        decision.exit_percentage, None,
        "after one partial the stop loss must exit fully"
    );
}

#[tokio::test]
async fn a_full_stop_loss_carries_no_exit_percentage() {
    let _cfg = config_guard();
    enable_stop_loss(20.0);

    let decision = exit_stop_loss::check_stop_loss(&test_position(1.0, 1.0), 0.5)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(decision.exit_percentage, None);
}

#[tokio::test]
async fn stop_loss_rejects_an_unusable_price() {
    let _cfg = config_guard();
    enable_stop_loss(20.0);
    let position = test_position(1.0, 1.0);

    for bad in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert!(
            exit_stop_loss::check_stop_loss(&position, bad)
                .await
                .is_err(),
            "price {bad} must be an error, never a silent hold"
        );
    }
}

#[tokio::test]
async fn stop_loss_rejects_an_unusable_entry_price() {
    let _cfg = config_guard();
    enable_stop_loss(20.0);

    for bad in [0.0, -1.0, f64::NAN] {
        let mut position = test_position(1.0, 1.0);
        position.average_entry_price = bad;
        assert!(
            exit_stop_loss::check_stop_loss(&position, 0.5)
                .await
                .is_err(),
            "entry price {bad} must be an error"
        );
    }
}

// ==================== TRAILING STOP ====================

fn enable_trailing(activation_pct: f64, distance_pct: f64) {
    set_config(|cfg| {
        cfg.positions.trailing_stop_enabled = true;
        cfg.positions.trailing_stop_activation_pct = activation_pct;
        cfg.positions.trailing_stop_distance_pct = distance_pct;
    });
}

/// A position entered at 1.0 whose price peaked at `peak`.
fn peaked_at(peak: f64) -> Position {
    let mut position = test_position(1.0, 1.0);
    position.price_highest = peak;
    position
}

#[tokio::test]
async fn trailing_stop_does_nothing_before_activation() {
    // The PEAK never reached the activation profit, so the trail was never armed.
    let _cfg = config_guard();
    enable_trailing(50.0, 10.0);

    let position = peaked_at(1.2); // best it ever did was +20%, activation needs +50%
    assert!(exit_trailing::check_trailing_stop(&position, 1.05)
        .await
        .unwrap()
        .is_none());
}

#[tokio::test]
async fn trailing_stop_arms_off_the_peak_not_the_current_price() {
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
    let decision = exit_trailing::check_trailing_stop(&position, 1.12)
        .await
        .unwrap()
        .expect("an armed trail must fire on a retrace through its stop");
    assert_eq!(decision.reason, TradeReason::TrailingStop);
}

#[tokio::test]
async fn trailing_stop_fires_once_armed_and_the_peak_is_given_back() {
    let _cfg = config_guard();
    enable_trailing(20.0, 10.0);

    // Peak 2.0, distance 10% -> stop at 1.80. Price 1.79 is armed (+79% profit) and
    // below the stop.
    let position = peaked_at(2.0);
    let decision = exit_trailing::check_trailing_stop(&position, 1.79)
        .await
        .unwrap()
        .expect("trailing stop must fire");
    assert_eq!(decision.reason, TradeReason::TrailingStop);
    assert_eq!(decision.priority, TradePriority::High);
    assert!(decision.exit_percentage.is_none(), "trailing sells it all");
}

#[tokio::test]
async fn trailing_stop_holds_while_the_price_is_above_the_stop() {
    let _cfg = config_guard();
    enable_trailing(20.0, 10.0);

    // Peak 2.0 -> stop 1.80. At exactly the stop it fires (`<=`); just above it holds.
    let position = peaked_at(2.0);
    assert!(exit_trailing::check_trailing_stop(&position, 1.81)
        .await
        .unwrap()
        .is_none());
    assert!(exit_trailing::check_trailing_stop(&position, 1.80)
        .await
        .unwrap()
        .is_some());
}

#[tokio::test]
async fn trailing_stop_never_exits_at_a_loss() {
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
        exit_trailing::check_trailing_stop(&position, 0.98)
            .await
            .unwrap()
            .is_none(),
        "trailing stop must not fire below the entry price"
    );
}

#[tokio::test]
async fn trailing_stop_does_nothing_without_a_recorded_peak() {
    let _cfg = config_guard();
    enable_trailing(20.0, 10.0);

    let position = peaked_at(0.0);
    assert!(exit_trailing::check_trailing_stop(&position, 5.0)
        .await
        .unwrap()
        .is_none());
}

#[tokio::test]
async fn trailing_stop_rejects_a_config_whose_distance_swallows_the_activation() {
    // distance >= activation is unsatisfiable: the stop sits at or below the entry, so
    // the "still profitable" guard can never pass and the trail would silently never
    // fire. Surfacing it as an error is what makes the misconfiguration visible.
    let _cfg = config_guard();
    enable_trailing(10.0, 10.0);

    let position = peaked_at(2.0);
    let err = exit_trailing::check_trailing_stop(&position, 1.5)
        .await
        .expect_err("equal distance and activation must error");
    assert!(err.contains("must be less than"), "got: {err}");
}

#[tokio::test]
async fn trailing_stop_is_off_when_disabled() {
    let _cfg = config_guard();
    set_config(|cfg| cfg.positions.trailing_stop_enabled = false);

    let position = peaked_at(2.0);
    assert!(exit_trailing::check_trailing_stop(&position, 1.0)
        .await
        .unwrap()
        .is_none());
}

#[tokio::test]
async fn trailing_stop_rejects_an_unusable_price() {
    let _cfg = config_guard();
    enable_trailing(20.0, 10.0);
    let position = peaked_at(2.0);

    for bad in [0.0, -1.0, f64::NAN] {
        assert!(exit_trailing::check_trailing_stop(&position, bad)
            .await
            .is_err());
    }
}

// ==================== ROI TARGET ====================

fn enable_roi(target_pct: f64) {
    set_config(|cfg| {
        cfg.trader.roi_exit_enabled = true;
        cfg.trader.roi_target_percent = target_pct;
    });
}

#[tokio::test]
async fn roi_exit_fires_at_the_target() {
    let _cfg = config_guard();
    enable_roi(50.0);

    let position = test_position(1.0, 1.0);
    let decision = exit_roi::check_roi_exit(&position, 1.50)
        .await
        .unwrap()
        .expect("ROI target reached");
    assert_eq!(decision.reason, TradeReason::TakeProfit);
    assert_eq!(decision.priority, TradePriority::Normal);
    assert!(decision.exit_percentage.is_none());
}

#[tokio::test]
async fn roi_exit_holds_below_the_target() {
    let _cfg = config_guard();
    enable_roi(50.0);

    let position = test_position(1.0, 1.0);
    assert!(exit_roi::check_roi_exit(&position, 1.49)
        .await
        .unwrap()
        .is_none());
}

#[tokio::test]
async fn roi_exit_measures_against_the_average_entry() {
    // Averaging down lowers the bar for the profit target, because the target is on
    // the position's real cost, not on the first buy.
    let _cfg = config_guard();
    enable_roi(50.0);

    let mut position = test_position(1.0, 1.0);
    position.average_entry_price = 0.5;
    assert!(exit_roi::check_roi_exit(&position, 0.75)
        .await
        .unwrap()
        .is_some());
}

#[tokio::test]
async fn roi_exit_is_off_when_disabled() {
    let _cfg = config_guard();
    set_config(|cfg| cfg.trader.roi_exit_enabled = false);

    assert!(exit_roi::check_roi_exit(&test_position(1.0, 1.0), 100.0)
        .await
        .unwrap()
        .is_none());
}

#[tokio::test]
async fn roi_exit_rejects_an_unusable_price() {
    let _cfg = config_guard();
    enable_roi(50.0);
    let position = test_position(1.0, 1.0);

    for bad in [0.0, -1.0, f64::NAN] {
        assert!(exit_roi::check_roi_exit(&position, bad).await.is_err());
    }
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

#[tokio::test]
async fn time_override_needs_both_the_age_and_the_loss() {
    // It exists to cut positions that have been stuck at a loss, so BOTH conditions
    // must hold — an old winner and a fresh loser must both be left alone.
    let _cfg = config_guard();
    enable_time_override(24.0, -40.0);

    let old_and_down = aged(test_position(1.0, 1.0), Duration::hours(25));
    assert!(exit_time::check_time_override(&old_and_down, 0.5)
        .await
        .unwrap()
        .is_some());

    let old_but_fine = aged(test_position(1.0, 1.0), Duration::hours(25));
    assert!(
        exit_time::check_time_override(&old_but_fine, 0.9)
            .await
            .unwrap()
            .is_none(),
        "a 10% loss is inside the 40% tolerance"
    );

    let fresh_and_down = aged(test_position(1.0, 1.0), Duration::hours(1));
    assert!(
        exit_time::check_time_override(&fresh_and_down, 0.5)
            .await
            .unwrap()
            .is_none(),
        "a young position has not had its time yet"
    );
}

#[tokio::test]
async fn time_override_boundary_is_inclusive_on_both_axes() {
    let _cfg = config_guard();
    enable_time_override(24.0, -40.0);

    // Exactly 24h old and exactly 40% down.
    let position = aged(test_position(1.0, 1.0), Duration::hours(24));
    assert!(exit_time::check_time_override(&position, 0.60)
        .await
        .unwrap()
        .is_some());
    // One tick shallower is inside tolerance.
    assert!(exit_time::check_time_override(&position, 0.601)
        .await
        .unwrap()
        .is_none());
}

#[tokio::test]
async fn time_override_carries_a_high_priority_full_exit() {
    let _cfg = config_guard();
    enable_time_override(24.0, -40.0);

    let position = aged(test_position(1.0, 1.0), Duration::hours(25));
    let decision = exit_time::check_time_override(&position, 0.5)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(decision.reason, TradeReason::TimeOverride);
    assert_eq!(decision.priority, TradePriority::High);
    assert!(decision.exit_percentage.is_none());
}

#[tokio::test]
async fn time_override_rejects_a_positive_loss_threshold() {
    // The threshold is stored negative ("exit at 40% down"). A positive value would
    // mean "exit once you are UP 40%", quietly turning a loss-cutter into a
    // take-profit. It must be refused, not obeyed.
    let _cfg = config_guard();
    enable_time_override(24.0, 40.0);

    let position = aged(test_position(1.0, 1.0), Duration::hours(25));
    let err = exit_time::check_time_override(&position, 0.5)
        .await
        .expect_err("a positive threshold must error");
    assert!(err.contains("must be <= 0"), "got: {err}");
}

#[tokio::test]
async fn time_override_rejects_a_non_positive_duration() {
    let _cfg = config_guard();
    enable_time_override(0.0, -40.0);

    let position = aged(test_position(1.0, 1.0), Duration::hours(25));
    assert!(exit_time::check_time_override(&position, 0.5)
        .await
        .is_err());
}

#[tokio::test]
async fn time_override_is_off_when_disabled() {
    let _cfg = config_guard();
    set_config(|cfg| cfg.trader.time_override_enabled = false);

    let position = aged(test_position(1.0, 1.0), Duration::days(30));
    assert!(exit_time::check_time_override(&position, 0.01)
        .await
        .unwrap()
        .is_none());
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
