// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Pure: `ExitPolicy::from_config` — the single place that resolves the config values every
//! exit rule used to read for itself — and `ExitPolicy::validate`, the one owner of the
//! rules a trader config and a copy task's merged overrides must both satisfy.
//!
//! Each sub-policy struct claims to carry an exact, fixed set of config fields. The
//! failure mode this file exists to catch is a field wired to the WRONG getter — for
//! example a stop loss threshold silently sourced from a take-profit level — which would
//! compile fine, read a real number, and quietly disable or misconfigure a real risk
//! rule. So every test here mutates every underlying field of one sub-policy to a
//! distinctive, non-default value and asserts the resolved policy carries exactly those
//! values, nothing swapped and nothing left at its default.
//!
//! Every test holds [`common::config_guard`], which serialises writes to the global
//! config and resets it to defaults on acquire, so these run correctly under both
//! `cargo nextest run` (process per test) and plain `cargo test` (threads in one
//! process).

use crate::common;

use common::config_guard;
use screenerbot::config::{
    DcaLevel, DcaMeasureFrom, TakeProfitLevel, TimeUnit, TraderConfig, TrailingStopBand,
};
use screenerbot::positions::PositionManagement;
use screenerbot::trader::copy::{management_for_exit_mode, ExitMode};
use screenerbot::trader::evaluators::DcaConfigSnapshot;
use screenerbot::trader::policy::{
    BreakEvenPolicy, ExitPolicy, ExitPolicyFault, ExitPolicyOverrides, StopLossPolicy,
    TakeProfitPolicy, TimePolicy, TimeStagnationPolicy, TrailingPolicy,
};

#[test]
fn stop_loss_policy_carries_exactly_its_five_config_fields() {
    let _cfg = config_guard();
    common::set_config(|cfg| {
        cfg.trader.stop_loss_enabled = false; // default: true
        cfg.trader.stop_loss_threshold_pct = 33.0; // default: 50.0
        cfg.trader.stop_loss_min_hold_seconds = 120; // default: 0
        cfg.trader.stop_loss_allow_partial = true; // default: false
        cfg.trader.stop_loss_partial_pct = 17.5; // default: 50.0
    });

    let policy = ExitPolicy::from_config();

    assert_eq!(
        policy.stop_loss,
        StopLossPolicy {
            enabled: false,
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
        cfg.trader.trailing_stop_enabled = true; // default: false
        cfg.trader.trailing_stop_activation_pct = 22.0; // default: 10.0
        cfg.trader.trailing_stop_distance_pct = 8.0; // default: 5.0
    });

    let policy = ExitPolicy::from_config();

    assert_eq!(
        policy.trailing,
        TrailingPolicy {
            enabled: true,
            activation_pct: 22.0,
            distance_pct: 8.0,
            bands: Vec::new(),
        }
    );
}

#[test]
fn take_profit_policy_carries_exactly_its_three_config_fields() {
    let _cfg = config_guard();
    common::set_config(|cfg| {
        cfg.trader.take_profit_enabled = false; // default: true
        cfg.trader.take_profit_levels_pct = vec![15.0, 77.0]; // default: [20.0]
        cfg.trader.take_profit_sell_pct = vec![40.0, 35.0]; // default: [100.0]
    });

    let policy = ExitPolicy::from_config();

    assert_eq!(
        policy.take_profit,
        TakeProfitPolicy {
            enabled: false,
            levels: vec![
                TakeProfitLevel {
                    profit_pct: 15.0,
                    sell_pct: 40.0,
                },
                TakeProfitLevel {
                    profit_pct: 77.0,
                    sell_pct: 35.0,
                },
            ],
        }
    );
}

#[test]
fn a_task_profit_target_overrides_the_ladder_with_one_full_step() {
    // A copy task's single profit target is a ladder of one step that sells everything;
    // the configured steps are replaced, never mixed with it.
    let _cfg = config_guard();
    common::set_config(|cfg| {
        cfg.trader.take_profit_levels_pct = vec![15.0, 77.0];
        cfg.trader.take_profit_sell_pct = vec![40.0, 35.0];
    });
    let inherited = ExitPolicy::from_config();
    let mut resolved = inherited.clone();
    let mut overrides = ExitPolicyOverrides::default();
    overrides.roi.target_profit_pct = Some(42.0);
    resolved.apply_overrides(&overrides);

    assert_eq!(
        resolved.take_profit,
        TakeProfitPolicy {
            enabled: inherited.take_profit.enabled,
            levels: vec![TakeProfitLevel::sell_all_at(42.0)],
        }
    );

    let mut disabled = inherited.clone();
    overrides = ExitPolicyOverrides::default();
    overrides.roi.enabled = Some(false);
    disabled.apply_overrides(&overrides);
    assert!(!disabled.take_profit.enabled);
    assert_eq!(disabled.take_profit.levels, inherited.take_profit.levels);
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
        cfg.trader.time_override_unit = TimeUnit::Minutes;
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
fn break_even_policy_carries_exactly_its_three_config_fields() {
    let _cfg = config_guard();
    common::set_config(|cfg| {
        cfg.trader.break_even_enabled = true; // default: false
        cfg.trader.break_even_trigger_pct = 22.5; // default: 15.0
        cfg.trader.break_even_offset_pct = 2.5; // default: 1.0
    });

    let policy = ExitPolicy::from_config();

    assert_eq!(
        policy.break_even,
        BreakEvenPolicy {
            enabled: true,
            trigger_pct: 22.5,
            offset_pct: 2.5,
        }
    );
}

#[test]
fn time_stagnation_policy_carries_exactly_its_three_config_fields() {
    let _cfg = config_guard();
    common::set_config(|cfg| {
        cfg.trader.time_exit_stagnation_enabled = true; // default: false
        cfg.trader.time_exit_stagnation_minutes = 95.0; // default: 240.0
        cfg.trader.time_exit_stagnation_band_pct = 3.5; // default: 5.0
    });

    let policy = ExitPolicy::from_config();

    assert_eq!(
        policy.time_stagnation,
        TimeStagnationPolicy {
            enabled: true,
            duration_seconds: 5700.0, // 95 minutes
            band_pct: 3.5,
        }
    );
}

#[test]
fn task_overrides_leave_the_break_even_stop_and_the_stagnation_exit_inherited() {
    let _cfg = config_guard();
    common::set_config(|cfg| {
        cfg.trader.break_even_enabled = true;
        cfg.trader.time_exit_stagnation_enabled = true;
    });
    let inherited = ExitPolicy::from_config();
    let mut overridden = inherited.clone();
    let mut overrides = ExitPolicyOverrides::default();
    overrides.trailing.enabled = Some(false);
    overrides.time.enabled = Some(false);
    overridden.apply_overrides(&overrides);

    assert_eq!(overridden.break_even, inherited.break_even);
    assert_eq!(overridden.time_stagnation, inherited.time_stagnation);
}

#[test]
fn dca_policy_equals_the_snapshot_the_dca_evaluator_expects() {
    let _cfg = config_guard();
    common::set_config(|cfg| {
        cfg.trader.dca_enabled = true; // default: false
        cfg.trader.dca_levels_pct = vec![-15.0, -30.0, -60.0]; // default: [-10, -10]
        cfg.trader.dca_size_multipliers = vec![0.5, 1.0, 2.0]; // default: [0.5, 0.5]
        cfg.trader.dca_measure_from = DcaMeasureFrom::FirstEntry; // default: average
        cfg.trader.dca_cooldown_minutes = 90; // default: 30
        cfg.trader.dca_min_hold_minutes = 20; // default: 0
    });

    let expected = DcaConfigSnapshot {
        enabled: true,
        levels: vec![
            DcaLevel {
                drop_pct: -15.0,
                size_multiplier: 0.5,
            },
            DcaLevel {
                drop_pct: -30.0,
                size_multiplier: 1.0,
            },
            DcaLevel {
                drop_pct: -60.0,
                size_multiplier: 2.0,
            },
        ],
        measure_from: DcaMeasureFrom::FirstEntry,
        cooldown_minutes: 90,
        min_hold_minutes: 20,
    };

    assert_eq!(ExitPolicy::from_config().dca, expected);
}

#[test]
fn task_override_changes_only_set_fields_and_unset_fields_inherit() {
    let _cfg = config_guard();
    common::set_config(|cfg| {
        cfg.trader.stop_loss_enabled = true;
        cfg.trader.stop_loss_threshold_pct = 40.0;
        cfg.trader.trailing_stop_distance_pct = 5.0;
        cfg.trader.take_profit_levels_pct = vec![20.0];
        cfg.trader.take_profit_sell_pct = vec![100.0];
    });
    let inherited = ExitPolicy::from_config();
    let mut resolved = inherited.clone();
    let mut overrides = ExitPolicyOverrides::default();
    overrides.stop_loss.threshold_pct = Some(17.0);
    overrides.trailing.enabled = Some(false);
    resolved.apply_overrides(&overrides);

    assert_eq!(resolved.stop_loss.threshold_pct, 17.0);
    assert!(!resolved.trailing.enabled);
    assert_eq!(resolved.stop_loss.enabled, inherited.stop_loss.enabled);
    assert_eq!(
        resolved.trailing.distance_pct,
        inherited.trailing.distance_pct
    );
    assert_eq!(resolved.take_profit, inherited.take_profit);
    assert_eq!(resolved.time, inherited.time);
    assert_eq!(resolved.dca.enabled, inherited.dca.enabled);
}

/// A copy round is evaluated by the shared exit monitor in either book: the management
/// its task's exit mode gives the round decides which exits run, and the policy those
/// exits use is the global policy with the task's overrides on top.
#[test]
fn a_copy_rounds_exit_mode_decides_its_exits_and_its_task_overrides_its_policy() {
    let _cfg = config_guard();
    for (exit_mode, policy_exits, copy_sells) in [
        (ExitMode::BuyOnly, true, false),
        (ExitMode::Hybrid, true, true),
        (ExitMode::Mirror, false, true),
    ] {
        let management = management_for_exit_mode(exit_mode);
        assert!(
            management.allows_safety_exits(),
            "{exit_mode:?}: safety exits always run"
        );
        assert_eq!(
            management.allows_policy_exits(),
            policy_exits,
            "{exit_mode:?}: the task's exit rules run only when the bot manages the exits"
        );
        assert_eq!(management.allows_copy_sell(), copy_sells, "{exit_mode:?}");
    }
    assert_eq!(
        management_for_exit_mode(ExitMode::Mirror),
        PositionManagement::CopyTask
    );

    common::set_config(|cfg| {
        cfg.trader.stop_loss_enabled = true;
        cfg.trader.stop_loss_threshold_pct = 40.0;
    });
    let mut task_policy = ExitPolicy::from_config();
    let mut overrides = ExitPolicyOverrides::default();
    overrides.stop_loss.threshold_pct = Some(12.0);
    task_policy.apply_overrides(&overrides);
    assert_eq!(task_policy.stop_loss.threshold_pct, 12.0);
    assert_eq!(ExitPolicy::from_config().stop_loss.threshold_pct, 40.0);
}

fn band(profit_pct: f64, distance_pct: f64) -> TrailingStopBand {
    TrailingStopBand {
        profit_pct,
        distance_pct,
    }
}

/// A trailing policy enabled at activation 20% and distance 10%, with `bands`.
fn trailing_with(bands: Vec<TrailingStopBand>) -> TrailingPolicy {
    TrailingPolicy {
        enabled: true,
        activation_pct: 20.0,
        distance_pct: 10.0,
        bands,
    }
}

#[test]
fn trailing_policy_carries_the_configured_bands_in_order() {
    let _cfg = config_guard();
    common::set_config(|cfg| {
        cfg.trader.trailing_stop_band_profit_pct = vec![50.0, 100.0]; // default: []
        cfg.trader.trailing_stop_band_distance_pct = vec![6.0, 4.0]; // default: []
    });

    assert_eq!(
        ExitPolicy::from_config().trailing.bands,
        vec![band(50.0, 6.0), band(100.0, 4.0)]
    );
}

#[test]
fn the_distance_in_force_is_the_highest_band_the_peak_reached() {
    let trailing = trailing_with(vec![band(50.0, 6.0), band(100.0, 4.0), band(200.0, 2.0)]);

    for (peak_profit_pct, expected) in [
        (25.0, 10.0),
        (49.9, 10.0),
        (50.0, 6.0),
        (99.0, 6.0),
        (100.0, 4.0),
        (199.0, 4.0),
        (500.0, 2.0),
    ] {
        assert_eq!(
            trailing.distance_in_force(peak_profit_pct),
            expected,
            "peak +{peak_profit_pct}%"
        );
    }
    assert_eq!(trailing_with(Vec::new()).distance_in_force(500.0), 10.0);
}

#[test]
fn trailing_bands_must_climb_above_the_activation_and_trail_inside_their_threshold() {
    assert!(trailing_with(vec![band(50.0, 6.0), band(100.0, 4.0)])
        .validate()
        .is_ok());

    for (case, bands) in [
        ("at the activation", vec![band(20.0, 5.0)]),
        ("below the activation", vec![band(15.0, 5.0)]),
        ("not ascending", vec![band(100.0, 4.0), band(50.0, 6.0)]),
        ("repeated threshold", vec![band(50.0, 6.0), band(50.0, 4.0)]),
        ("distance at its threshold", vec![band(50.0, 50.0)]),
        ("no distance", vec![band(50.0, 0.0)]),
        ("not a number", vec![band(f64::NAN, 5.0)]),
        (
            "too many",
            vec![
                band(30.0, 5.0),
                band(40.0, 5.0),
                band(50.0, 5.0),
                band(60.0, 5.0),
            ],
        ),
    ] {
        assert!(trailing_with(bands).validate().is_err(), "{case}");
    }

    // A switched-off trail is not checked, as the trader config never checked it.
    let mut off = trailing_with(vec![band(5.0, 50.0)]);
    off.enabled = false;
    assert!(off.validate().is_ok());
}

#[test]
fn the_trader_config_refuses_bands_the_policy_refuses() {
    let mut trader = TraderConfig {
        trailing_stop_enabled: true,
        trailing_stop_activation_pct: 20.0,
        trailing_stop_distance_pct: 10.0,
        trailing_stop_band_profit_pct: vec![50.0, 100.0],
        trailing_stop_band_distance_pct: vec![6.0, 4.0],
        ..TraderConfig::default()
    };
    assert!(trader.validate().is_ok());

    trader.trailing_stop_band_profit_pct = vec![100.0, 50.0];
    assert!(trader.validate().is_err(), "descending thresholds");

    trader.trailing_stop_band_profit_pct = vec![50.0];
    assert!(
        trader.validate().is_err(),
        "a threshold without its distance"
    );

    // A copy task can switch the trail on and inherit the bands, so their order is
    // checked while the global trail is off too.
    trader.trailing_stop_enabled = false;
    trader.trailing_stop_band_profit_pct = vec![100.0, 50.0];
    trader.trailing_stop_band_distance_pct = vec![6.0, 4.0];
    assert!(
        trader.validate().is_err(),
        "descending thresholds while the trail is off"
    );
    trader.trailing_stop_band_profit_pct = vec![20.0];
    trader.trailing_stop_band_distance_pct = vec![5.0];
    assert!(
        trader.validate().is_err(),
        "a band at the activation while the trail is off"
    );
    trader.trailing_stop_band_profit_pct = vec![50.0, 100.0];
    trader.trailing_stop_band_distance_pct = vec![6.0, 4.0];
    assert!(trader.validate().is_ok());
}

#[test]
fn a_task_that_sets_its_own_trail_drops_the_global_bands() {
    let mut base = ExitPolicy::from_trader_config(&TraderConfig {
        trailing_stop_band_profit_pct: vec![50.0],
        trailing_stop_band_distance_pct: vec![6.0],
        ..TraderConfig::default()
    });
    let mut switched_on = base.clone();
    let mut overrides = ExitPolicyOverrides::default();
    overrides.trailing.enabled = Some(true);
    switched_on.apply_overrides(&overrides);
    assert_eq!(switched_on.trailing.bands, vec![band(50.0, 6.0)]);

    overrides.trailing.distance_pct = Some(4.0);
    base.apply_overrides(&overrides);
    assert!(base.trailing.bands.is_empty());
}

#[test]
fn task_overrides_are_validated_on_the_policy_they_make() {
    let base = || ExitPolicy::from_trader_config(&TraderConfig::default());

    let mut overrides = ExitPolicyOverrides::default();
    assert!(overrides.validate_over(base()).is_ok());

    // Global activation 10%: a 12% distance could never sell at a profit.
    overrides.trailing.distance_pct = Some(12.0);
    assert!(overrides.validate_over(base()).is_err());
    overrides.trailing.activation_pct = Some(30.0);
    let merged = overrides
        .validate_over(base())
        .expect("a distance below the task's own activation");
    assert_eq!(merged.trailing.activation_pct, 30.0);
    assert_eq!(merged.trailing.distance_pct, 12.0);

    let mut positive_loss = ExitPolicyOverrides::default();
    positive_loss.time.loss_threshold_pct = Some(5.0);
    assert!(positive_loss.validate_over(base()).is_err());

    let mut no_target = ExitPolicyOverrides::default();
    no_target.roi.target_profit_pct = Some(0.0);
    assert!(no_target.validate_over(base()).is_err());

    let mut whole_position = ExitPolicyOverrides::default();
    whole_position.stop_loss.partial_exit_default_pct = Some(100.0);
    assert!(whole_position.validate_over(base()).is_err());
}

#[test]
fn a_rule_a_tasks_stored_overrides_break_runs_as_the_global_rule() {
    let global = ExitPolicy::from_trader_config(&TraderConfig {
        trailing_stop_enabled: true,
        trailing_stop_activation_pct: 10.0,
        trailing_stop_distance_pct: 5.0,
        time_override_enabled: true,
        ..TraderConfig::default()
    });
    let mut overrides = ExitPolicyOverrides::default();
    // Valid when stored over a 20% activation; the global activation was lowered since.
    overrides.trailing.distance_pct = Some(12.0);
    // Stored before the time threshold was bounded at -100.
    overrides.time.loss_threshold_pct = Some(-150.0);
    overrides.stop_loss.threshold_pct = Some(25.0);

    let mut resolved = global.clone();
    resolved.apply_overrides(&overrides);

    assert_eq!(resolved.trailing, global.trailing);
    assert_eq!(resolved.time, global.time);
    assert_eq!(resolved.stop_loss.threshold_pct, 25.0);
    assert!(resolved.validate().is_ok());
}

#[test]
fn every_exit_policy_fault_has_catalog_text() {
    let listed = |fault: ExitPolicyFault| match fault {
        ExitPolicyFault::StopLossThreshold { .. }
        | ExitPolicyFault::PartialShare { .. }
        | ExitPolicyFault::TakeProfitTarget { .. }
        | ExitPolicyFault::TrailingActivation { .. }
        | ExitPolicyFault::TrailingDistance { .. }
        | ExitPolicyFault::TrailingDistanceNotBelowActivation { .. }
        | ExitPolicyFault::TrailingBandCount { .. }
        | ExitPolicyFault::TrailingBandOrder { .. }
        | ExitPolicyFault::TrailingBandDistance { .. }
        | ExitPolicyFault::TimeDuration { .. }
        | ExitPolicyFault::TimeLossThreshold { .. } => fault,
    };
    for fault in [
        listed(ExitPolicyFault::StopLossThreshold { value: 120.0 }),
        listed(ExitPolicyFault::PartialShare { value: 100.0 }),
        listed(ExitPolicyFault::TakeProfitTarget { value: 0.0 }),
        listed(ExitPolicyFault::TrailingActivation { value: 0.0 }),
        listed(ExitPolicyFault::TrailingDistance { value: 0.0 }),
        listed(ExitPolicyFault::TrailingDistanceNotBelowActivation {
            distance: 12.0,
            activation: 10.0,
        }),
        listed(ExitPolicyFault::TrailingBandCount { count: 4 }),
        listed(ExitPolicyFault::TrailingBandOrder {
            profit: 15.0,
            floor: 20.0,
        }),
        listed(ExitPolicyFault::TrailingBandDistance {
            profit: 50.0,
            distance: 50.0,
        }),
        listed(ExitPolicyFault::TimeDuration { seconds: 0.0 }),
        listed(ExitPolicyFault::TimeLossThreshold { value: -150.0 }),
    ] {
        let text = fault.ui_text();
        assert_ne!(
            screenerbot::i18n::format_en(&text.id, None),
            text.id,
            "missing {}",
            text.id
        );
    }
}
