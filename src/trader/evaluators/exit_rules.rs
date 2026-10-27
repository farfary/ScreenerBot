// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The one order of the built-in exit rules, shared by the live exit monitor and the paper copy book.

use crate::trader::evaluators::{
    exit_break_even, exit_stop_loss, exit_take_profit, exit_time, exit_time_stagnation,
    exit_trailing,
};
use crate::trader::policy::ExitPolicy;

/// What the built-in exit rules read from a holding, on plain numbers. Each book fills it
/// from its own records: the live exit monitor from a `Position`, the paper copy book from
/// a `PaperPosition` and its pool mark.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ExitInputs {
    /// Average cost of one token: the entry every rule measures from.
    pub average_entry_price: f64,
    /// The price the holding is marked at this evaluation.
    pub current_price: f64,
    /// Highest price since the average entry last changed; arms the trailing stop and the
    /// break-even stop.
    pub basis_peak_price: f64,
    /// Seconds since the holding was opened.
    pub held_seconds: i64,
    /// True once the holding has sold part of itself, which spends the one partial stop loss.
    pub partial_exits_taken: bool,
    /// Take-profit steps the holding has reserved this round; the next step due is the one
    /// at this index of the ladder.
    pub take_profit_levels_taken: u32,
}

/// A built-in exit rule, named by what it protects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitRule {
    /// Loss from the average entry reached the stop-loss threshold.
    StopLoss,
    /// The price fell the trailing distance from an armed basis peak while still in profit.
    Trailing,
    /// The price fell back to the kept margin above the average entry after the basis
    /// peak reached the break-even arming profit; fires below the entry too.
    BreakEven,
    /// Profit from the average entry reached the level of take-profit step `level`
    /// (1 is the first step). A partial sale of it reserves that step.
    TakeProfit { level: u32 },
    /// Held past the time limit at or below the loss threshold.
    TimeLoss,
    /// Held past the stagnation time with profit or loss inside the flat band.
    TimeStagnation,
}

/// One rule on its own: the rule as it would fire for these inputs, and
/// `Some(exit_percentage)` when it fires.
type Check = fn(&ExitInputs, &ExitPolicy) -> (ExitRule, crate::trader::Result<Option<Option<f64>>>);

/// The order the rules are checked in; the first that fires wins.
///
/// Stop loss leads: the loss cap and its partial first sale apply whatever else fires.
/// When the trailing stop and the break-even stop both fire, the trailing stop names the
/// sale; the break-even stop catches a fall to its floor that the trail has not caught.
/// Both run before take profit so a retracement sells everything rather than one ladder
/// step. The time rules come last: they act only
/// on holdings no price rule wanted, the loss rule before the flat one.
const ORDER: [Check; 6] = [
    stop_loss,
    trailing,
    break_even,
    take_profit,
    time_loss,
    time_stagnation,
];

/// The rule that fired and how much of the holding it sells: `Some(pct)` is a partial
/// exit of that share, `None` sells everything.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RuleOutcome {
    pub rule: ExitRule,
    pub exit_percentage: Option<f64>,
}

/// What one evaluation of the built-in rules found.
#[derive(Debug, Clone)]
pub struct Decision {
    /// The first rule in the order that fired and was admitted.
    pub outcome: Option<RuleOutcome>,
    /// Rules checked before the outcome (or all of them when none fired) whose
    /// configuration is impossible, in order.
    pub rule_errors: Vec<(ExitRule, crate::trader::Error)>,
}

/// The first rule that fires for `inputs` under `policy`, checked in the order stop loss,
/// trailing stop, break-even stop, take profit, time loss, stagnation.
///
/// A rule whose configuration is impossible does not fire; the rules after it still
/// run, so one broken setting never disables the others, and its error is reported in
/// [`Decision::rule_errors`]. Rules after the one that fires are not evaluated. A mark
/// that is not a positive finite price fires nothing.
pub fn decide(inputs: &ExitInputs, policy: &ExitPolicy) -> Decision {
    decide_admitting(inputs, policy, |_| true)
}

/// [`decide`] for a caller that cannot act on every outcome right now: an outcome
/// `admit` refuses is passed over and the next rule in the order is checked, exactly as
/// if the refused rule had not fired.
pub fn decide_admitting(
    inputs: &ExitInputs,
    policy: &ExitPolicy,
    admit: impl Fn(&RuleOutcome) -> bool,
) -> Decision {
    let mut decision = Decision {
        outcome: None,
        rule_errors: Vec::new(),
    };
    if !inputs.current_price.is_finite() || inputs.current_price <= 0.0 {
        return decision;
    }
    for check in ORDER {
        let (rule, fired) = check(inputs, policy);
        match fired {
            Ok(Some(exit_percentage)) => {
                let outcome = RuleOutcome {
                    rule,
                    exit_percentage,
                };
                if admit(&outcome) {
                    decision.outcome = Some(outcome);
                    return decision;
                }
            }
            Ok(None) => {}
            Err(e) => decision.rule_errors.push((rule, e)),
        }
    }
    decision
}

fn stop_loss(
    inputs: &ExitInputs,
    policy: &ExitPolicy,
) -> (ExitRule, crate::trader::Result<Option<Option<f64>>>) {
    let fired = exit_stop_loss::stop_loss_exit(
        inputs.average_entry_price,
        inputs.current_price,
        inputs.held_seconds,
        inputs.partial_exits_taken,
        &policy.stop_loss,
    );
    (ExitRule::StopLoss, Ok(fired))
}

fn trailing(
    inputs: &ExitInputs,
    policy: &ExitPolicy,
) -> (ExitRule, crate::trader::Result<Option<Option<f64>>>) {
    let fired = exit_trailing::trailing_stop_triggered(
        inputs.average_entry_price,
        inputs.basis_peak_price,
        inputs.current_price,
        &policy.trailing,
    )
    .map(|triggered| triggered.then_some(None));
    (ExitRule::Trailing, fired)
}

fn break_even(
    inputs: &ExitInputs,
    policy: &ExitPolicy,
) -> (ExitRule, crate::trader::Result<Option<Option<f64>>>) {
    let fired = exit_break_even::break_even_triggered(
        inputs.average_entry_price,
        inputs.basis_peak_price,
        inputs.current_price,
        &policy.break_even,
    )
    .map(|triggered| triggered.then_some(None));
    (ExitRule::BreakEven, fired)
}

fn take_profit(
    inputs: &ExitInputs,
    policy: &ExitPolicy,
) -> (ExitRule, crate::trader::Result<Option<Option<f64>>>) {
    let fired = exit_take_profit::take_profit_step(
        inputs.average_entry_price,
        inputs.current_price,
        inputs.take_profit_levels_taken,
        &policy.take_profit,
    );
    let level = inputs.take_profit_levels_taken.saturating_add(1);
    (ExitRule::TakeProfit { level }, fired)
}

fn time_loss(
    inputs: &ExitInputs,
    policy: &ExitPolicy,
) -> (ExitRule, crate::trader::Result<Option<Option<f64>>>) {
    let fired = exit_time::time_override_triggered(
        inputs.average_entry_price,
        inputs.current_price,
        inputs.held_seconds,
        &policy.time,
    )
    .map(|triggered| triggered.then_some(None));
    (ExitRule::TimeLoss, fired)
}

fn time_stagnation(
    inputs: &ExitInputs,
    policy: &ExitPolicy,
) -> (ExitRule, crate::trader::Result<Option<Option<f64>>>) {
    let fired = exit_time_stagnation::stagnation_triggered(
        inputs.average_entry_price,
        inputs.current_price,
        inputs.held_seconds,
        &policy.time_stagnation,
    )
    .map(|triggered| triggered.then_some(None));
    (ExitRule::TimeStagnation, fired)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{DcaMeasureFrom, TakeProfitLevel};
    use crate::trader::evaluators::dca::DcaConfigSnapshot;
    use crate::trader::policy::{
        BreakEvenPolicy, StopLossPolicy, TakeProfitPolicy, TimePolicy, TimeStagnationPolicy,
        TrailingPolicy,
    };

    /// Every original rule enabled: stop at -30% (partial 50%), trail 10% once +20%, take
    /// profit at +100%, time loss after an hour at -10% or worse. The break-even stop
    /// (+15% / +1%) and the stagnation exit (two hours inside ±5%) are configured but off;
    /// [`with_break_even_and_stagnation`] turns them on.
    fn policy() -> ExitPolicy {
        ExitPolicy {
            stop_loss: StopLossPolicy {
                enabled: true,
                threshold_pct: 30.0,
                min_hold_seconds: 0,
                allow_partial: true,
                partial_exit_default_pct: 50.0,
            },
            trailing: TrailingPolicy {
                enabled: true,
                activation_pct: 20.0,
                distance_pct: 10.0,
            },
            break_even: BreakEvenPolicy {
                enabled: false,
                trigger_pct: 15.0,
                offset_pct: 1.0,
            },
            take_profit: TakeProfitPolicy {
                enabled: true,
                levels: vec![TakeProfitLevel::sell_all_at(100.0)],
            },
            time: TimePolicy {
                enabled: true,
                loss_threshold_pct: -10.0,
                duration_seconds: 3600.0,
            },
            time_stagnation: TimeStagnationPolicy {
                enabled: false,
                duration_seconds: 7200.0,
                band_pct: 5.0,
            },
            dca: DcaConfigSnapshot {
                enabled: false,
                levels: Vec::new(),
                measure_from: DcaMeasureFrom::Average,
                cooldown_minutes: 0,
                min_hold_minutes: 0,
            },
        }
    }

    /// Entered at 1.0, peaked at `peak`, marked at `current`, held `held_seconds`.
    fn inputs(current: f64, peak: f64, held_seconds: i64, partial_taken: bool) -> ExitInputs {
        ExitInputs {
            average_entry_price: 1.0,
            current_price: current,
            basis_peak_price: peak,
            held_seconds,
            partial_exits_taken: partial_taken,
            take_profit_levels_taken: 0,
        }
    }

    const FIRST_STEP: ExitRule = ExitRule::TakeProfit { level: 1 };

    fn outcome(rule: ExitRule, exit_percentage: Option<f64>) -> Option<RuleOutcome> {
        Some(RuleOutcome {
            rule,
            exit_percentage,
        })
    }

    #[test]
    fn each_rule_fires_on_its_own() {
        let policy = policy();
        let cases = [
            (
                inputs(0.6, 1.0, 60, false),
                outcome(ExitRule::StopLoss, Some(50.0)),
            ),
            (
                inputs(1.12, 1.25, 60, false),
                outcome(ExitRule::Trailing, None),
            ),
            (inputs(2.0, 1.0, 60, false), outcome(FIRST_STEP, None)),
            (
                inputs(0.85, 1.0, 7200, false),
                outcome(ExitRule::TimeLoss, None),
            ),
            (inputs(0.95, 1.0, 60, false), None),
        ];
        for (inputs, expected) in cases {
            assert_eq!(decide(&inputs, &policy).outcome, expected, "{inputs:?}");
        }
    }

    #[test]
    fn the_first_rule_in_the_order_wins_when_several_fire() {
        let policy = policy();
        // Old and 40% down: stop loss and time loss both fire; stop loss is first.
        assert_eq!(
            decide(&inputs(0.6, 1.0, 7200, false), &policy).outcome,
            outcome(ExitRule::StopLoss, Some(50.0))
        );
        // Peaked at +150% and retraced to +120%: trailing and take profit both fire.
        assert_eq!(
            decide(&inputs(2.2, 2.5, 60, false), &policy).outcome,
            outcome(ExitRule::Trailing, None)
        );
        // With the trail off, take profit is next.
        let mut no_trail = policy.clone();
        no_trail.trailing.enabled = false;
        assert_eq!(
            decide(&inputs(2.2, 2.5, 60, false), &no_trail).outcome,
            outcome(FIRST_STEP, None)
        );
        // With the stop off, an old losing holding falls to the time loss.
        let mut no_stop = policy.clone();
        no_stop.stop_loss.enabled = false;
        assert_eq!(
            decide(&inputs(0.6, 1.0, 7200, false), &no_stop).outcome,
            outcome(ExitRule::TimeLoss, None)
        );
    }

    #[test]
    fn a_partial_is_taken_only_under_the_stop_loss_allowance_and_only_once() {
        let policy = policy();
        assert_eq!(
            decide(&inputs(0.6, 1.0, 60, false), &policy).outcome,
            outcome(ExitRule::StopLoss, Some(50.0))
        );
        assert_eq!(
            decide(&inputs(0.6, 1.0, 60, true), &policy).outcome,
            outcome(ExitRule::StopLoss, None),
            "a holding that already sold part exits fully"
        );
        let mut full_only = policy.clone();
        full_only.stop_loss.allow_partial = false;
        assert_eq!(
            decide(&inputs(0.6, 1.0, 60, false), &full_only).outcome,
            outcome(ExitRule::StopLoss, None)
        );
        for full in [
            inputs(1.12, 1.25, 60, false),
            inputs(2.0, 1.0, 60, false),
            inputs(0.85, 1.0, 7200, false),
        ] {
            let decided = decide(&full, &policy).outcome.expect("a rule fires");
            assert_eq!(
                decided.exit_percentage, None,
                "{decided:?} sells everything"
            );
        }
    }

    #[test]
    fn a_refused_outcome_passes_to_the_next_rule() {
        let policy = policy();
        let refuse_partials = |outcome: &RuleOutcome| outcome.exit_percentage.is_none();
        // A partial stop loss refused on an old losing holding: the time loss is next.
        assert_eq!(
            decide_admitting(&inputs(0.6, 1.0, 7200, false), &policy, refuse_partials).outcome,
            outcome(ExitRule::TimeLoss, None)
        );
        // Nothing after it fires on a young one.
        assert_eq!(
            decide_admitting(&inputs(0.6, 1.0, 60, false), &policy, refuse_partials).outcome,
            None
        );
        // A full stop loss is admitted.
        assert_eq!(
            decide_admitting(&inputs(0.6, 1.0, 60, true), &policy, refuse_partials).outcome,
            outcome(ExitRule::StopLoss, None)
        );
    }

    #[test]
    fn an_impossible_rule_never_disables_the_rules_around_it() {
        let mut broken = policy();
        broken.trailing.distance_pct = 25.0;
        // Nothing else fires: the configuration error is reported alone.
        let quiet = decide(&inputs(1.05, 1.25, 60, false), &broken);
        assert!(quiet.outcome.is_none());
        assert!(matches!(
            quiet.rule_errors.as_slice(),
            [(ExitRule::Trailing, _)]
        ));
        // A later rule still fires past the broken one, and the error is still reported.
        let later = decide(&inputs(2.0, 1.0, 60, false), &broken);
        assert_eq!(later.outcome, outcome(FIRST_STEP, None));
        assert!(matches!(
            later.rule_errors.as_slice(),
            [(ExitRule::Trailing, _)]
        ));
        // An earlier rule wins before the broken one is reached, so it is not evaluated.
        let earlier = decide(&inputs(0.6, 1.0, 60, false), &broken);
        assert_eq!(earlier.outcome, outcome(ExitRule::StopLoss, Some(50.0)));
        assert!(earlier.rule_errors.is_empty());
    }

    #[test]
    fn every_impossible_rule_is_reported_in_order_when_nothing_fires() {
        let mut broken = policy();
        broken.trailing.distance_pct = 25.0;
        broken.time.duration_seconds = 0.0;
        let decision = decide(&inputs(1.05, 1.25, 60, false), &broken);
        assert!(decision.outcome.is_none());
        assert!(matches!(
            decision.rule_errors.as_slice(),
            [(ExitRule::Trailing, _), (ExitRule::TimeLoss, _)]
        ));
    }

    #[test]
    fn an_unusable_mark_fires_nothing() {
        let policy = policy();
        for bad in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            assert_eq!(
                decide(&inputs(bad, 1.0, 7200, false), &policy).outcome,
                None
            );
        }
    }
    /// A three-step ladder, 30% at +20%, 30% at +50% and the last 40% at +100%, with
    /// every other rule off.
    fn ladder_only() -> ExitPolicy {
        let mut policy = policy();
        policy.stop_loss.enabled = false;
        policy.trailing.enabled = false;
        policy.time.enabled = false;
        policy.take_profit.levels = vec![
            TakeProfitLevel {
                profit_pct: 20.0,
                sell_pct: 30.0,
            },
            TakeProfitLevel {
                profit_pct: 50.0,
                sell_pct: 30.0,
            },
            TakeProfitLevel {
                profit_pct: 100.0,
                sell_pct: 40.0,
            },
        ];
        policy
    }

    fn after_steps(current: f64, taken: u32) -> ExitInputs {
        ExitInputs {
            take_profit_levels_taken: taken,
            ..inputs(current, current, 60, taken > 0)
        }
    }

    #[test]
    fn the_ladder_fires_the_step_due_with_its_share_of_what_is_left() {
        let policy = ladder_only();
        assert_eq!(
            decide(&after_steps(1.2, 0), &policy).outcome,
            outcome(ExitRule::TakeProfit { level: 1 }, Some(30.0))
        );
        let second = decide(&after_steps(1.5, 1), &policy)
            .outcome
            .expect("the second step fires");
        assert_eq!(second.rule, ExitRule::TakeProfit { level: 2 });
        let share = second.exit_percentage.expect("a partial");
        assert!((share - 30.0 / 70.0 * 100.0).abs() < 1e-9, "{share}");
        assert_eq!(
            decide(&after_steps(2.0, 2), &policy).outcome,
            outcome(ExitRule::TakeProfit { level: 3 }, None)
        );
        assert_eq!(decide(&after_steps(9.0, 3), &policy).outcome, None);
    }

    #[test]
    fn one_step_fires_per_evaluation_however_many_levels_the_price_passed() {
        let policy = ladder_only();
        assert_eq!(
            decide(&after_steps(2.5, 0), &policy).outcome,
            outcome(ExitRule::TakeProfit { level: 1 }, Some(30.0)),
            "a gap through every level sells the first step alone"
        );
        assert_eq!(
            decide(&after_steps(1.3, 1), &policy).outcome,
            None,
            "the next step waits for its own level"
        );
    }

    #[test]
    fn a_one_step_ladder_decides_like_the_single_profit_target() {
        let mut policy = ladder_only();
        policy.take_profit.levels = vec![TakeProfitLevel::sell_all_at(20.0)];
        assert_eq!(
            decide(&after_steps(1.2, 0), &policy).outcome,
            outcome(FIRST_STEP, None)
        );
        assert_eq!(decide(&after_steps(1.19, 0), &policy).outcome, None);
        policy.take_profit.enabled = false;
        assert_eq!(decide(&after_steps(5.0, 0), &policy).outcome, None);
    }

    #[test]
    fn a_refused_ladder_step_is_passed_over() {
        let policy = ladder_only();
        let refuse_ladder =
            |outcome: &RuleOutcome| !matches!(outcome.rule, ExitRule::TakeProfit { .. });
        assert_eq!(
            decide_admitting(&after_steps(1.2, 0), &policy, refuse_ladder).outcome,
            None
        );
    }

    fn with_break_even_and_stagnation() -> ExitPolicy {
        let mut policy = policy();
        policy.break_even.enabled = true;
        policy.time_stagnation.enabled = true;
        policy
    }

    #[test]
    fn the_break_even_stop_fires_on_its_own_and_below_the_entry() {
        let mut policy = with_break_even_and_stagnation();
        policy.trailing.enabled = false;
        policy.stop_loss.enabled = false;
        // Peaked at +18%, back at +1%: the kept margin.
        assert_eq!(
            decide(&inputs(1.01, 1.18, 60, false), &policy).outcome,
            outcome(ExitRule::BreakEven, None)
        );
        // Gapped through the floor into a loss: still sells.
        assert_eq!(
            decide(&inputs(0.9, 1.18, 60, false), &policy).outcome,
            outcome(ExitRule::BreakEven, None)
        );
        // Peaked at +14%: never armed.
        assert_eq!(decide(&inputs(1.0, 1.14, 60, false), &policy).outcome, None);
    }

    #[test]
    fn the_trail_names_a_sale_in_profit_and_the_break_even_stop_the_one_below() {
        let mut policy = with_break_even_and_stagnation();
        policy.trailing.activation_pct = 10.0;
        policy.trailing.distance_pct = 5.0;
        // Peak +20%: trail stop at 1.14. A mark at +0.5% is under both stops while still
        // in profit; the trail is first.
        assert_eq!(
            decide(&inputs(1.005, 1.2, 60, false), &policy).outcome,
            outcome(ExitRule::Trailing, None)
        );
        // At -2% the trail is silent (it never sells at a loss) and the break-even stop
        // sells everything.
        assert_eq!(
            decide(&inputs(0.98, 1.2, 60, false), &policy).outcome,
            outcome(ExitRule::BreakEven, None)
        );
        // Deep enough for the stop loss: it wins, partial first.
        assert_eq!(
            decide(&inputs(0.6, 1.2, 60, false), &policy).outcome,
            outcome(ExitRule::StopLoss, Some(50.0))
        );
    }

    #[test]
    fn a_wide_trail_leaves_the_fall_to_the_floor_to_the_break_even_stop() {
        let mut policy = with_break_even_and_stagnation();
        policy.trailing.activation_pct = 15.0;
        policy.trailing.distance_pct = 14.0;
        // Peak +16%: the trail stop sits at 0.9976, under the +1% floor. At +0.5%, still
        // in profit, the trail has not fired and the break-even stop has.
        let decision = decide(&inputs(1.005, 1.16, 60, false), &policy);
        assert_eq!(decision.outcome, outcome(ExitRule::BreakEven, None));
        assert!(
            decision.rule_errors.is_empty(),
            "{:?}",
            decision.rule_errors
        );
    }

    #[test]
    fn the_break_even_stop_sells_before_a_ladder_step() {
        let mut policy = with_break_even_and_stagnation();
        policy.trailing.enabled = false;
        policy.take_profit.levels = vec![TakeProfitLevel::sell_all_at(0.5)];
        // +1% is past a +0.5% ladder step and on the break-even margin: the floor sells.
        assert_eq!(
            decide(&inputs(1.01, 1.2, 60, false), &policy).outcome,
            outcome(ExitRule::BreakEven, None)
        );
    }

    #[test]
    fn the_stagnation_exit_fires_last_and_only_inside_its_band() {
        let policy = with_break_even_and_stagnation();
        // Two hours flat at +2%.
        assert_eq!(
            decide(&inputs(1.02, 1.02, 7200, false), &policy).outcome,
            outcome(ExitRule::TimeStagnation, None)
        );
        // On both band edges.
        for edge in [1.05, 0.95] {
            assert_eq!(
                decide(&inputs(edge, edge, 7200, false), &policy).outcome,
                outcome(ExitRule::TimeStagnation, None),
                "{edge}"
            );
        }
        // Too young.
        assert_eq!(
            decide(&inputs(1.02, 1.02, 7199, false), &policy).outcome,
            None
        );
        // At -15% after two hours inside a ±20% band both time rules fire: the loss rule
        // is first.
        let mut wide = policy.clone();
        wide.time_stagnation.band_pct = 20.0;
        assert_eq!(
            decide(&inputs(0.85, 1.0, 7200, false), &wide).outcome,
            outcome(ExitRule::TimeLoss, None)
        );
    }

    #[test]
    fn a_broken_break_even_stop_never_disables_the_rules_after_it() {
        let mut broken = with_break_even_and_stagnation();
        broken.break_even.offset_pct = 15.0;
        let decision = decide(&inputs(1.02, 1.02, 7200, false), &broken);
        assert_eq!(decision.outcome, outcome(ExitRule::TimeStagnation, None));
        assert!(matches!(
            decision.rule_errors.as_slice(),
            [(ExitRule::BreakEven, _)]
        ));
    }
}
