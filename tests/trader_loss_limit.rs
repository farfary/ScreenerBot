//! Pure: the period loss limit — the circuit breaker that pauses new entries once
//! realized losses over a rolling window exceed a SOL budget.
//!
//! Two properties matter and neither is visible from the outside until money is gone:
//! it must trip at the configured budget (not one trade later), and tripping must pause
//! ENTRIES ONLY. Exits keep running by design — a bot that stops managing the positions
//! it already holds at the exact moment it is losing money is worse than one with no
//! limit at all.
//!
//! The state lives in a process-global `RwLock`, so every test holds
//! [`common::config_guard`] to serialise and resets the state explicitly on entry.

mod common;

use common::{config_guard, set_config};
use screenerbot::trader::safety::loss_limit::{
    get_loss_limit_status, is_entry_blocked_by_loss_limit, record_realized_loss,
    reset_loss_limit_state, resume_from_loss_limit,
};

/// Enable the limit at `limit_sol` over a long period, and start from a clean slate.
fn enable_limit(limit_sol: f64) {
    set_config(|cfg| {
        cfg.trader.loss_limit_enabled = true;
        cfg.trader.loss_limit_sol = limit_sol;
        cfg.trader.loss_limit_period_hours = 24;
        cfg.trader.loss_limit_auto_resume = true;
    });
    reset_loss_limit_state();
}

#[test]
fn a_fresh_period_blocks_nothing() {
    let _cfg = config_guard();
    enable_limit(1.0);

    assert!(!is_entry_blocked_by_loss_limit());
    assert_eq!(get_loss_limit_status().cumulative_loss_sol, 0.0);
    assert!(!get_loss_limit_status().is_limited);
}

#[test]
fn losses_accumulate_across_positions() {
    // The budget is spent by the PERIOD, not by any single trade, so several small
    // losses must add up to the same limit one large one would hit.
    let _cfg = config_guard();
    enable_limit(1.0);

    record_realized_loss(0.2);
    record_realized_loss(0.3);
    assert!((get_loss_limit_status().cumulative_loss_sol - 0.5).abs() < 1e-12);
    assert!(!is_entry_blocked_by_loss_limit());
}

#[test]
fn the_limit_trips_exactly_at_the_budget() {
    // `cumulative >= limit`, so spending the budget precisely is enough. Requiring one
    // more trade to trip would let the configured ceiling be exceeded every time.
    let _cfg = config_guard();
    enable_limit(1.0);

    record_realized_loss(0.999);
    assert!(!is_entry_blocked_by_loss_limit());

    record_realized_loss(0.001);
    assert!(
        is_entry_blocked_by_loss_limit(),
        "reaching the budget must pause entries"
    );
    assert!(get_loss_limit_status().limited_at.is_some());
}

#[test]
fn a_single_loss_past_the_budget_trips_it_immediately() {
    let _cfg = config_guard();
    enable_limit(1.0);

    record_realized_loss(5.0);
    assert!(is_entry_blocked_by_loss_limit());
    assert!((get_loss_limit_status().cumulative_loss_sol - 5.0).abs() < 1e-12);
}

#[test]
fn the_recorded_amount_is_a_magnitude_not_a_signed_pnl() {
    // The module takes an absolute loss: it does `.abs()` on whatever it is handed. So
    // a caller MUST only call it for a losing position — handing it a PROFIT would
    // count that profit against the loss budget and pause the bot for winning. Every
    // call site in the codebase is guarded by `pnl < 0.0` for exactly this reason;
    // this test documents the contract so a new call site does not get it wrong.
    let _cfg = config_guard();
    enable_limit(10.0);

    record_realized_loss(-2.0);
    assert!(
        (get_loss_limit_status().cumulative_loss_sol - 2.0).abs() < 1e-12,
        "a negative P&L is recorded as its magnitude"
    );

    record_realized_loss(3.0);
    assert!(
        (get_loss_limit_status().cumulative_loss_sol - 5.0).abs() < 1e-12,
        "a positive value is ALSO taken as a loss — callers must guard on pnl < 0"
    );
}

#[test]
fn a_disabled_limit_records_nothing_and_blocks_nothing() {
    let _cfg = config_guard();
    enable_limit(1.0);
    set_config(|cfg| cfg.trader.loss_limit_enabled = false);

    record_realized_loss(100.0);
    assert!(!is_entry_blocked_by_loss_limit());
    assert_eq!(
        get_loss_limit_status().cumulative_loss_sol,
        0.0,
        "a disabled limit must not accumulate a hidden balance"
    );
}

#[test]
fn a_manual_resume_reopens_entries_without_clearing_the_tally() {
    // Resuming is the user overriding the pause. The period's loss total is history and
    // stays on the books — otherwise a resume would silently hand out a fresh budget.
    let _cfg = config_guard();
    enable_limit(1.0);

    record_realized_loss(2.0);
    assert!(is_entry_blocked_by_loss_limit());

    resume_from_loss_limit();
    assert!(!is_entry_blocked_by_loss_limit());
    assert!(
        (get_loss_limit_status().cumulative_loss_sol - 2.0).abs() < 1e-12,
        "the period's realized loss is not erased by a resume"
    );
    assert!(get_loss_limit_status().limited_at.is_none());
}

#[test]
fn a_loss_after_a_manual_resume_can_trip_the_limit_again() {
    // The trip is guarded by `!is_limited`, so after a resume the next loss that keeps
    // the tally at or above the budget must pause entries once more rather than
    // leaving the breaker permanently disarmed.
    let _cfg = config_guard();
    enable_limit(1.0);

    record_realized_loss(2.0);
    resume_from_loss_limit();
    assert!(!is_entry_blocked_by_loss_limit());

    record_realized_loss(0.1);
    assert!(
        is_entry_blocked_by_loss_limit(),
        "the breaker must re-arm after a manual resume"
    );
}

#[test]
fn a_reset_starts_a_brand_new_period() {
    let _cfg = config_guard();
    enable_limit(1.0);

    record_realized_loss(5.0);
    assert!(is_entry_blocked_by_loss_limit());

    reset_loss_limit_state();
    let status = get_loss_limit_status();
    assert_eq!(status.cumulative_loss_sol, 0.0);
    assert!(!status.is_limited);
    assert!(status.limited_at.is_none());
    assert!(!is_entry_blocked_by_loss_limit());
}

#[test]
fn an_elapsed_period_auto_resumes_when_configured() {
    // A one-hour period whose start is pushed two hours back has elapsed. With
    // auto-resume on, the next status read rolls the period and reopens entries.
    let _cfg = config_guard();
    enable_limit(1.0);
    set_config(|cfg| {
        cfg.trader.loss_limit_period_hours = 1;
        cfg.trader.loss_limit_auto_resume = true;
    });

    record_realized_loss(5.0);
    assert!(is_entry_blocked_by_loss_limit());

    // Elapse the period by shortening it to zero hours — `period_start + 0h` is always
    // in the past, which is the same condition a real rollover reaches.
    set_config(|cfg| cfg.trader.loss_limit_period_hours = 0);

    assert!(
        !is_entry_blocked_by_loss_limit(),
        "an elapsed period must auto-resume"
    );
    assert_eq!(get_loss_limit_status().cumulative_loss_sol, 0.0);
}

#[test]
fn an_elapsed_period_keeps_the_pause_when_auto_resume_is_off() {
    // With auto-resume off the tally resets but the pause stands: the user asked to be
    // told before the bot starts buying again.
    let _cfg = config_guard();
    enable_limit(1.0);
    set_config(|cfg| cfg.trader.loss_limit_auto_resume = false);

    record_realized_loss(5.0);
    assert!(is_entry_blocked_by_loss_limit());

    set_config(|cfg| cfg.trader.loss_limit_period_hours = 0);

    assert!(
        is_entry_blocked_by_loss_limit(),
        "without auto-resume the pause survives the period rollover"
    );
    assert_eq!(
        get_loss_limit_status().cumulative_loss_sol,
        0.0,
        "the tally still rolls over"
    );
}

#[test]
fn the_status_reports_the_time_left_in_the_period() {
    let _cfg = config_guard();
    enable_limit(1.0);
    set_config(|cfg| cfg.trader.loss_limit_period_hours = 24);
    reset_loss_limit_state();

    let status = get_loss_limit_status();
    assert!(
        status.period_remaining_secs > 23 * 3600 && status.period_remaining_secs <= 24 * 3600,
        "remaining was {}",
        status.period_remaining_secs
    );
}

#[test]
fn the_remaining_time_never_goes_negative() {
    // It is rendered as a countdown; a negative value would print as a nonsense
    // duration on the dashboard.
    let _cfg = config_guard();
    enable_limit(1.0);
    set_config(|cfg| cfg.trader.loss_limit_period_hours = 0);

    assert!(get_loss_limit_status().period_remaining_secs >= 0);
}
