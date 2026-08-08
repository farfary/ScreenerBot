//! Pure: DCA (averaging down) decision logic — `DcaEvaluation::evaluate`.
//!
//! DCA is the only auto-trader path that SPENDS MORE SOL on a position that is already
//! losing, so every gate it has is a spend limiter: the drop threshold, the count cap,
//! the cooldown, and the minimum trade size. A gate that fails open buys again and
//! again into a token that is going to zero.
//!
//! The threshold arithmetic is the subtle part and the reason this file exists.
//! `threshold_pct` is stored NEGATIVE (-10 means "trigger at a 10% loss") and `pnl_pct`
//! is negative while losing, so the comparison is inverted relative to how it reads:
//! `pnl_pct >= threshold_pct` means "the loss is not deep enough yet".

mod common;

use chrono::{Duration, Utc};
use common::test_position;
use screenerbot::positions::Position;
use screenerbot::trader::evaluators::{DcaConfigSnapshot, DcaEvaluation};
use screenerbot::trader::MIN_TRADE_SIZE_SOL;

/// Config that permits a DCA whenever the position is more than 10% down.
fn permissive_config() -> DcaConfigSnapshot {
    DcaConfigSnapshot {
        enabled: true,
        max_count: 3,
        cooldown_minutes: 60,
        threshold_pct: -10.0,
        size_percentage: 50.0,
    }
}

/// A 1 SOL position entered at price 1.0, currently priced at `current_price`.
fn position_at(current_price: f64) -> Position {
    let mut position = test_position(1.0, 1.0);
    position.current_price = Some(current_price);
    position
}

fn evaluate(position: &Position, config: DcaConfigSnapshot) -> DcaEvaluation {
    DcaEvaluation::evaluate(position, config).expect("evaluation must not fail")
}

// ==================== DROP THRESHOLD ====================

#[test]
fn a_deep_enough_loss_triggers_a_dca() {
    // 15% down against a 10% threshold.
    let evaluation = evaluate(&position_at(0.85), permissive_config());
    assert!(
        evaluation.should_trigger,
        "expected a trigger, got: {:?}",
        evaluation.reasons
    );
    assert!((evaluation.calculations.pnl_pct - -15.0).abs() < 1e-9);
}

#[test]
fn a_shallow_loss_does_not_trigger() {
    let evaluation = evaluate(&position_at(0.95), permissive_config());
    assert!(!evaluation.should_trigger);
    assert!(
        evaluation
            .reasons
            .iter()
            .any(|r| r.contains("Price drop insufficient")),
        "reasons: {:?}",
        evaluation.reasons
    );
}

#[test]
fn a_position_in_profit_never_dcas() {
    // Averaging UP is not what this is for. A rising price must never buy more.
    let evaluation = evaluate(&position_at(2.0), permissive_config());
    assert!(!evaluation.should_trigger);
    assert!((evaluation.calculations.pnl_pct - 100.0).abs() < 1e-9);
}

#[test]
fn the_threshold_boundary_requires_a_strictly_deeper_loss() {
    // Exactly -10% against a -10% threshold does NOT trigger: the comparison is
    // `pnl_pct >= threshold_pct` -> blocked. One tick deeper does trigger. Locking the
    // boundary down matters because the sign inversion makes it easy to flip by
    // accident, and a flipped boundary changes when real SOL is spent.
    let at_threshold = evaluate(&position_at(0.90), permissive_config());
    assert!(
        !at_threshold.should_trigger,
        "exactly at the threshold must not trigger"
    );

    let past_threshold = evaluate(&position_at(0.8999), permissive_config());
    assert!(past_threshold.should_trigger);
}

#[test]
fn a_positive_threshold_is_read_as_its_magnitude_for_reporting() {
    // `required_drop_pct` is reported as an absolute value so the UI never shows a
    // double negative, while the trigger comparison keeps using the raw signed value.
    let mut config = permissive_config();
    config.threshold_pct = -10.0;
    let evaluation = evaluate(&position_at(0.85), config);
    assert_eq!(evaluation.calculations.required_drop_pct, 10.0);
}

// ==================== ENABLED / COUNT / COOLDOWN ====================

#[test]
fn a_disabled_config_never_triggers() {
    let mut config = permissive_config();
    config.enabled = false;
    let evaluation = evaluate(&position_at(0.5), config);
    assert!(!evaluation.should_trigger);
    assert!(evaluation
        .reasons
        .iter()
        .any(|r| r.contains("DCA disabled")));
}

#[test]
fn the_count_cap_stops_averaging_down_forever() {
    // The cap is the only thing bounding total exposure to one dying token.
    let mut position = position_at(0.5);
    position.dca_count = 3;
    let evaluation = evaluate(&position, permissive_config());
    assert!(!evaluation.should_trigger);
    assert!(evaluation
        .reasons
        .iter()
        .any(|r| r.contains("DCA count limit reached")));
}

#[test]
fn the_last_permitted_dca_is_the_one_below_the_cap() {
    let mut position = position_at(0.5);
    position.dca_count = 2; // cap is 3
    assert!(evaluate(&position, permissive_config()).should_trigger);
}

#[test]
fn a_zero_cap_disables_dca_entirely() {
    let mut config = permissive_config();
    config.max_count = 0;
    assert!(!evaluate(&position_at(0.5), config).should_trigger);
}

#[test]
fn the_cooldown_blocks_a_rapid_second_buy() {
    let mut position = position_at(0.5);
    position.last_dca_time = Some(Utc::now() - Duration::minutes(10)); // cooldown is 60
    let evaluation = evaluate(&position, permissive_config());
    assert!(!evaluation.should_trigger);
    assert!(evaluation
        .reasons
        .iter()
        .any(|r| r.contains("DCA cooldown active")));
}

#[test]
fn the_cooldown_expires_at_the_configured_minute() {
    // `minutes < cooldown_minutes` blocks, so reaching the cooldown exactly releases it.
    let mut position = position_at(0.5);
    position.last_dca_time = Some(Utc::now() - Duration::minutes(60));
    assert!(evaluate(&position, permissive_config()).should_trigger);
}

#[test]
fn a_position_that_never_dcad_has_no_cooldown_to_serve() {
    let position = position_at(0.5);
    assert!(position.last_dca_time.is_none());
    let evaluation = evaluate(&position, permissive_config());
    assert!(evaluation.should_trigger);
    assert!(evaluation.calculations.minutes_since_last.is_none());
}

// ==================== SIZING ====================

#[test]
fn the_dca_amount_is_a_share_of_the_first_entry() {
    // 50% of a 1 SOL entry. Note the base is `entry_size_sol` (the FIRST buy), not the
    // cumulative `total_size_sol` — so a third DCA is the same size as the first and
    // exposure grows linearly rather than compounding.
    let mut position = position_at(0.5);
    position.entry_size_sol = 1.0;
    position.total_size_sol = 4.0; // already averaged in several times
    position.dca_count = 2;

    let evaluation = evaluate(&position, permissive_config());
    assert!((evaluation.calculations.dca_amount_sol - 0.5).abs() < 1e-12);
}

#[test]
fn a_dust_sized_dca_is_refused() {
    // A swap below the minimum trade size cannot execute, so evaluating it as a
    // trigger would produce a decision that only ever fails at the executor.
    let mut position = position_at(0.5);
    position.entry_size_sol = MIN_TRADE_SIZE_SOL; // 50% of this is half the minimum
    let evaluation = evaluate(&position, permissive_config());
    assert!(!evaluation.should_trigger);
    assert!(
        evaluation
            .reasons
            .iter()
            .any(|r| r.contains("below minimum")),
        "reasons: {:?}",
        evaluation.reasons
    );
}

#[test]
fn a_dca_exactly_at_the_minimum_trade_size_is_allowed() {
    let mut position = position_at(0.5);
    position.entry_size_sol = MIN_TRADE_SIZE_SOL * 2.0; // 50% lands exactly on the floor
    assert!(evaluate(&position, permissive_config()).should_trigger);
}

#[test]
fn a_zero_size_percentage_can_never_trigger() {
    let mut config = permissive_config();
    config.size_percentage = 0.0;
    assert!(!evaluate(&position_at(0.5), config).should_trigger);
}

// ==================== INVALID INPUTS ====================

#[test]
fn a_position_without_a_current_price_cannot_be_evaluated() {
    let mut position = position_at(0.5);
    position.current_price = None;
    let evaluation = evaluate(&position, permissive_config());
    assert!(!evaluation.should_trigger);
    assert_eq!(evaluation.reasons, vec!["No valid current price"]);
}

#[test]
fn a_non_finite_or_non_positive_price_is_refused() {
    for bad in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        let mut position = position_at(1.0);
        position.current_price = Some(bad);
        let evaluation = evaluate(&position, permissive_config());
        assert!(
            !evaluation.should_trigger,
            "price {bad} must not produce a DCA"
        );
        assert_eq!(evaluation.reasons, vec!["No valid current price"]);
    }
}

#[test]
fn a_broken_entry_price_is_refused_rather_than_dividing_by_zero() {
    // `pnl_pct` divides by the entry price. A zero or NaN entry would produce an
    // infinite or NaN loss, and NaN compares false against every threshold — which
    // fails OPEN in some orderings. It must be rejected before the arithmetic.
    for bad in [0.0, -1.0, f64::NAN] {
        let mut position = position_at(0.5);
        position.average_entry_price = bad;
        let evaluation = evaluate(&position, permissive_config());
        assert!(!evaluation.should_trigger, "entry price {bad} must refuse");
        assert_eq!(evaluation.reasons, vec!["No valid entry price"]);
    }
}

// ==================== REPORTING ====================

#[test]
fn every_failing_gate_is_reported_not_just_the_first() {
    // The dashboard shows these reasons; stopping at the first would hide that a
    // position is blocked by three separate rules.
    let mut position = position_at(0.99); // barely down
    position.dca_count = 5; // over the cap
    position.last_dca_time = Some(Utc::now()); // inside the cooldown

    let evaluation = evaluate(&position, permissive_config());
    assert!(!evaluation.should_trigger);
    assert!(
        evaluation.reasons.len() >= 3,
        "expected every blocker, got: {:?}",
        evaluation.reasons
    );
}

#[test]
fn the_summary_describes_the_next_dca_when_triggering() {
    let mut position = position_at(0.85);
    position.dca_count = 1;
    let evaluation = evaluate(&position, permissive_config());
    assert!(evaluation.should_trigger);
    // Human-facing count is 1-indexed on the NEXT buy: after one DCA this is "#2".
    assert!(
        evaluation.summary().starts_with("DCA #2:"),
        "summary: {}",
        evaluation.summary()
    );
}

#[test]
fn the_summary_lists_the_blockers_when_not_triggering() {
    let evaluation = evaluate(&position_at(0.99), permissive_config());
    assert!(!evaluation.should_trigger);
    assert_eq!(evaluation.summary(), evaluation.reasons.join(", "));
}

#[test]
fn the_calculations_snapshot_reports_what_was_measured() {
    let mut position = position_at(0.8);
    position.dca_count = 1;
    position.last_dca_time = Some(Utc::now() - Duration::minutes(90));

    let evaluation = evaluate(&position, permissive_config());
    let calc = &evaluation.calculations;
    assert_eq!(calc.current_dca_count, 1);
    assert_eq!(calc.minutes_since_last, Some(90));
    assert_eq!(calc.entry_price, 1.0);
    assert_eq!(calc.current_price, 0.8);
    assert!((calc.pnl_pct - -20.0).abs() < 1e-9);
}

// ==================== MANAGEMENT AUTHORITY ====================

/// A manually managed position must never be auto-DCA'd — the auto-trader may only spend
/// SOL on positions whose exits it also owns. If this regresses, the bot averages down
/// into a bag the user asked to manage themselves.
#[test]
fn every_non_auto_trader_management_refuses_auto_dca() {
    for management in [
        screenerbot::positions::PositionManagement::UserOnly,
        screenerbot::positions::PositionManagement::CopyTask,
        screenerbot::positions::PositionManagement::Hybrid,
    ] {
        let mut position = position_at(0.85);
        position.management = management;
        let evaluation = evaluate(&position, permissive_config());
        assert!(!evaluation.should_trigger, "management={management:?}");
        assert!(
            evaluation
                .reasons
                .iter()
                .any(|r| r.contains("does not allow auto-DCA")),
            "reasons: {:?}",
            evaluation.reasons
        );
    }
}

/// Positive control for the test above: the identical position and config, but
/// AutoTrader ownership DOES trigger — proving the management gate is the only
/// difference and this isn't accidentally blocked by some other condition.
#[test]
fn the_same_position_triggers_once_auto_managed() {
    let mut position = position_at(0.85);
    position.management = screenerbot::positions::PositionManagement::AutoTrader;
    assert!(evaluate(&position, permissive_config()).should_trigger);
}
