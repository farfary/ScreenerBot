//! Pure: `ExitPolicy::from_config` — the single place that resolves the config values every
//! exit rule used to read for itself.
//!
//! Each sub-policy struct claims to carry an exact, fixed set of config fields. The
//! failure mode this file exists to catch is a field wired to the WRONG getter — for
//! example a stop loss threshold silently sourced from the ROI target — which would
//! compile fine, read a real number, and quietly disable or misconfigure a real risk
//! rule. So every test here mutates every underlying field of one sub-policy to a
//! distinctive, non-default value and asserts the resolved policy carries exactly those
//! values, nothing swapped and nothing left at its default.
//!
//! Every test holds [`common::config_guard`], which serialises writes to the global
//! config and resets it to defaults on acquire, so these run correctly under both
//! `cargo nextest run` (process per test) and plain `cargo test` (threads in one
//! process).

mod common;

use common::config_guard;
use screenerbot::trader::evaluators::DcaConfigSnapshot;
use screenerbot::trader::policy::{
    ExitPolicy, RoiPolicy, StopLossPolicy, TimePolicy, TrailingPolicy,
};

#[test]
fn stop_loss_policy_carries_exactly_its_five_config_fields() {
    let _cfg = config_guard();
    common::set_config(|cfg| {
        cfg.trader.stop_loss_enabled = true; // default: false
        cfg.trader.stop_loss_threshold_pct = 33.0; // default: 50.0
        cfg.trader.stop_loss_min_hold_seconds = 120; // default: 0
        cfg.trader.stop_loss_allow_partial = true; // default: false
        cfg.positions.partial_exit_default_pct = 17.5; // default: 50.0
    });

    let policy = ExitPolicy::from_config();

    assert_eq!(
        policy.stop_loss,
        StopLossPolicy {
            enabled: true,
            threshold_pct: 33.0,
            min_hold_seconds: 120,
            allow_partial: true,
            partial_exit_default_pct: 17.5,
        }
    );
}

#[test]
fn trailing_policy_carries_exactly_its_three_config_fields() {
    let _cfg = config_guard();
    common::set_config(|cfg| {
        cfg.positions.trailing_stop_enabled = true; // default: false
        cfg.positions.trailing_stop_activation_pct = 22.0; // default: 10.0
        cfg.positions.trailing_stop_distance_pct = 8.0; // default: 5.0
    });

    let policy = ExitPolicy::from_config();

    assert_eq!(
        policy.trailing,
        TrailingPolicy {
            enabled: true,
            activation_pct: 22.0,
            distance_pct: 8.0,
        }
    );
}

#[test]
fn roi_policy_carries_exactly_its_two_config_fields() {
    let _cfg = config_guard();
    common::set_config(|cfg| {
        cfg.trader.roi_exit_enabled = false; // default: true
        cfg.trader.roi_target_percent = 77.0; // default: 20.0
    });

    let policy = ExitPolicy::from_config();

    assert_eq!(
        policy.roi,
        RoiPolicy {
            enabled: false,
            target_profit_pct: 77.0,
        }
    );
}

#[test]
fn time_policy_carries_exactly_its_three_config_fields() {
    let _cfg = config_guard();
    common::set_config(|cfg| {
        cfg.trader.time_override_enabled = false; // default: true
        cfg.trader.time_override_loss_threshold_percent = -55.0; // default: -40.0
                                                                 // 7 minutes, not the default 168 hours — proves the unit conversion is read too,
                                                                 // not just the raw duration number.
        cfg.trader.time_override_duration = 7.0;
        cfg.trader.time_override_unit = "minutes".to_owned();
    });

    let policy = ExitPolicy::from_config();

    assert_eq!(
        policy.time,
        TimePolicy {
            enabled: false,
            loss_threshold_pct: -55.0,
            duration_seconds: 420.0, // 7 minutes
        }
    );
}

#[test]
fn dca_policy_equals_the_snapshot_the_dca_evaluator_expects() {
    let _cfg = config_guard();
    common::set_config(|cfg| {
        cfg.trader.dca_enabled = true; // default: false
        cfg.trader.dca_max_count = 5; // default: 2
        cfg.trader.dca_cooldown_minutes = 90; // default: 30
        cfg.trader.dca_threshold_pct = -25.0; // default: -10.0
        cfg.trader.dca_size_percentage = 30.0; // default: 50.0
    });

    let expected = DcaConfigSnapshot {
        enabled: true,
        max_count: 5,
        cooldown_minutes: 90,
        threshold_pct: -25.0,
        size_percentage: 30.0,
    };

    let dca = ExitPolicy::from_config().dca;

    assert_eq!(dca.enabled, expected.enabled);
    assert_eq!(dca.max_count, expected.max_count);
    assert_eq!(dca.cooldown_minutes, expected.cooldown_minutes);
    assert_eq!(dca.threshold_pct, expected.threshold_pct);
    assert_eq!(dca.size_percentage, expected.size_percentage);
}
