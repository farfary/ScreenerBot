// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The period loss limit — the circuit breaker that pauses new entries once realized losses
//! over a rolling window reach a SOL budget — following the books of a real positions store.
//!
//! Two properties matter and neither is visible from the outside until money is gone:
//! it must trip at the configured budget (not one trade later), and tripping must pause
//! ENTRIES ONLY. Exits keep running by design — a bot that stops managing the positions
//! it already holds at the exact moment it is losing money is worse than one with no
//! limit at all.
//!
//! The limiter's figure is what the closed positions in the store realized since the period
//! started, so a loss reaches it by being booked. The state lives in a process-global
//! `RwLock`: every test holds [`common::config_guard`] and resets the state explicitly, and
//! a test that books runs in a child process of its own.

mod common;

use chrono::Utc;
use common::{config_guard, set_config};
use screenerbot::positions::{db, PositionOrigin};
use screenerbot::trader::safety::loss_limit::{
    get_loss_limit_status, is_entry_blocked_by_loss_limit, reset_loss_limit_state,
    resume_from_loss_limit, sync_from_books,
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

/// Opens an isolated positions store for the configured wallet.
async fn open_books() {
    screenerbot::paths::ensure_all_directories().expect("create data directories");
    common::configure_own_wallet();
    screenerbot::positions::initialize_positions_database()
        .await
        .expect("initialise positions database");
}

/// Books a bot position closed now with a realized P&L of `pnl`, then lets the limiter
/// follow the books.
async fn book_close(pnl: f64) {
    book_close_with(pnl, |_| {}).await;
}

async fn book_close_with(pnl: f64, configure: impl FnOnce(&mut screenerbot::positions::Position)) {
    let mut position = common::test_position(1.0, 1.0);
    position.id = None;
    position.exit_time = Some(Utc::now());
    position.transaction_exit_verified = true;
    position.pnl = Some(pnl);
    configure(&mut position);
    db::save_position(&position)
        .await
        .expect("persist a closed position");
    sync_from_books().await;
}

fn recorded() -> f64 {
    get_loss_limit_status().cumulative_loss_native
}

#[test]
fn a_fresh_period_blocks_nothing() {
    let _cfg = config_guard();
    enable_limit(1.0);

    assert!(!is_entry_blocked_by_loss_limit());
    assert_eq!(recorded(), 0.0);
    assert!(!get_loss_limit_status().is_limited);
}

#[test]
fn losses_accumulate_across_positions() {
    // The budget is spent by the PERIOD, not by any single trade, so several small
    // losses must add up to the same limit one large one would hit.
    common::run_isolated("losses_accumulate_across_positions", || async {
        let _dir = common::isolated_env();
        let _cfg = config_guard();
        open_books().await;
        enable_limit(1.0);

        book_close(-0.2).await;
        book_close(-0.3).await;
        assert!((recorded() - 0.5).abs() < 1e-12);
        assert!(!is_entry_blocked_by_loss_limit());
    });
}

#[test]
fn the_limit_trips_exactly_at_the_budget() {
    // `loss >= limit`, so spending the budget precisely is enough. Requiring one more
    // trade to trip would let the configured ceiling be exceeded every time.
    common::run_isolated("the_limit_trips_exactly_at_the_budget", || async {
        let _dir = common::isolated_env();
        let _cfg = config_guard();
        open_books().await;
        enable_limit(1.0);

        book_close(-0.75).await;
        assert!(!is_entry_blocked_by_loss_limit());

        book_close(-0.25).await;
        assert!(
            is_entry_blocked_by_loss_limit(),
            "reaching the budget must pause entries"
        );
        assert!(get_loss_limit_status().limited_at.is_some());
    });
}

#[test]
fn a_single_loss_past_the_budget_trips_it_immediately() {
    common::run_isolated(
        "a_single_loss_past_the_budget_trips_it_immediately",
        || async {
            let _dir = common::isolated_env();
            let _cfg = config_guard();
            open_books().await;
            enable_limit(1.0);

            book_close(-5.0).await;
            assert!(is_entry_blocked_by_loss_limit());
            assert!((recorded() - 5.0).abs() < 1e-12);
        },
    );
}

#[test]
fn a_profit_never_counts_against_the_budget() {
    // The figure is the period's realized LOSS: a winning close neither adds to it nor
    // offsets a losing one.
    common::run_isolated("a_profit_never_counts_against_the_budget", || async {
        let _dir = common::isolated_env();
        let _cfg = config_guard();
        open_books().await;
        enable_limit(10.0);

        book_close(3.0).await;
        assert_eq!(recorded(), 0.0, "a profit is not a loss");

        book_close(-2.0).await;
        assert!(
            (recorded() - 2.0).abs() < 1e-12,
            "a profit does not offset a loss"
        );
    });
}

#[test]
fn a_loss_on_a_wallet_derived_round_is_not_counted() {
    // A wallet-derived round is the user's own holding, not risk the bot took.
    common::run_isolated(
        "a_loss_on_a_wallet_derived_round_is_not_counted",
        || async {
            let _dir = common::isolated_env();
            let _cfg = config_guard();
            open_books().await;
            enable_limit(1.0);

            book_close_with(-5.0, |position| position.origin = PositionOrigin::External).await;
            assert_eq!(recorded(), 0.0);
            assert!(!is_entry_blocked_by_loss_limit());
        },
    );
}

#[test]
fn a_disabled_limit_records_nothing_and_blocks_nothing() {
    common::run_isolated(
        "a_disabled_limit_records_nothing_and_blocks_nothing",
        || async {
            let _dir = common::isolated_env();
            let _cfg = config_guard();
            open_books().await;
            enable_limit(1.0);
            set_config(|cfg| cfg.trader.loss_limit_enabled = false);

            book_close(-100.0).await;
            assert!(!is_entry_blocked_by_loss_limit());
            assert_eq!(
                recorded(),
                0.0,
                "a disabled limit must not accumulate a hidden balance"
            );
        },
    );
}

#[test]
fn a_manual_resume_reopens_entries_without_clearing_the_tally() {
    // Resuming is the user overriding the pause. The period's loss total is history and
    // stays on the books — otherwise a resume would silently hand out a fresh budget.
    common::run_isolated(
        "a_manual_resume_reopens_entries_without_clearing_the_tally",
        || async {
            let _dir = common::isolated_env();
            let _cfg = config_guard();
            open_books().await;
            enable_limit(1.0);

            book_close(-2.0).await;
            assert!(is_entry_blocked_by_loss_limit());

            resume_from_loss_limit();
            assert!(!is_entry_blocked_by_loss_limit());
            assert!(
                (recorded() - 2.0).abs() < 1e-12,
                "the period's realized loss is not erased by a resume"
            );
            assert!(get_loss_limit_status().limited_at.is_none());

            sync_from_books().await;
            assert!(
                !is_entry_blocked_by_loss_limit(),
                "following unchanged books never undoes a resume"
            );
        },
    );
}

#[test]
fn a_loss_after_a_manual_resume_can_trip_the_limit_again() {
    // The trip is guarded by `!is_limited`, so after a resume the next loss that keeps
    // the tally at or above the budget must pause entries once more rather than
    // leaving the breaker permanently disarmed.
    common::run_isolated(
        "a_loss_after_a_manual_resume_can_trip_the_limit_again",
        || async {
            let _dir = common::isolated_env();
            let _cfg = config_guard();
            open_books().await;
            enable_limit(1.0);

            book_close(-2.0).await;
            resume_from_loss_limit();
            assert!(!is_entry_blocked_by_loss_limit());

            book_close(-0.1).await;
            assert!(
                is_entry_blocked_by_loss_limit(),
                "the breaker must re-arm after a manual resume"
            );
        },
    );
}

#[test]
fn a_reset_starts_a_brand_new_period() {
    common::run_isolated("a_reset_starts_a_brand_new_period", || async {
        let _dir = common::isolated_env();
        let _cfg = config_guard();
        open_books().await;
        enable_limit(1.0);

        book_close(-5.0).await;
        assert!(is_entry_blocked_by_loss_limit());

        reset_loss_limit_state();
        let status = get_loss_limit_status();
        assert_eq!(status.cumulative_loss_native, 0.0);
        assert!(!status.is_limited);
        assert!(status.limited_at.is_none());
        assert!(!is_entry_blocked_by_loss_limit());
    });
}

#[test]
fn an_elapsed_period_auto_resumes_when_configured() {
    // A period that has elapsed rolls over on the next status read. With auto-resume on,
    // that reopens entries.
    common::run_isolated("an_elapsed_period_auto_resumes_when_configured", || async {
        let _dir = common::isolated_env();
        let _cfg = config_guard();
        open_books().await;
        enable_limit(1.0);
        set_config(|cfg| {
            cfg.trader.loss_limit_period_hours = 1;
            cfg.trader.loss_limit_auto_resume = true;
        });

        book_close(-5.0).await;
        assert!(is_entry_blocked_by_loss_limit());

        // Elapse the period by shortening it to zero hours — `period_start + 0h` is
        // always in the past, which is the same condition a real rollover reaches.
        set_config(|cfg| cfg.trader.loss_limit_period_hours = 0);

        assert!(
            !is_entry_blocked_by_loss_limit(),
            "an elapsed period must auto-resume"
        );
        assert_eq!(recorded(), 0.0);
    });
}

#[test]
fn an_elapsed_period_keeps_the_pause_when_auto_resume_is_off() {
    // With auto-resume off the tally resets but the pause stands: the user asked to be
    // told before the bot starts buying again.
    common::run_isolated(
        "an_elapsed_period_keeps_the_pause_when_auto_resume_is_off",
        || async {
            let _dir = common::isolated_env();
            let _cfg = config_guard();
            open_books().await;
            enable_limit(1.0);
            set_config(|cfg| cfg.trader.loss_limit_auto_resume = false);

            book_close(-5.0).await;
            assert!(is_entry_blocked_by_loss_limit());

            set_config(|cfg| cfg.trader.loss_limit_period_hours = 0);

            assert!(
                is_entry_blocked_by_loss_limit(),
                "without auto-resume the pause survives the period rollover"
            );
            assert_eq!(recorded(), 0.0, "the tally still rolls over");
        },
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
