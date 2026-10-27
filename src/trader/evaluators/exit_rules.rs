// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The one order of the built-in exit rules, shared by the live exit monitor and the paper copy book.

use crate::trader::evaluators::{exit_roi, exit_stop_loss, exit_time, exit_trailing};
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
    /// Highest price since the average entry last changed; arms the trailing stop.
    pub basis_peak_price: f64,
    /// Seconds since the holding was opened.
    pub held_seconds: i64,
    /// True once the holding has sold part of itself, which spends the one partial stop loss.
    pub partial_exits_taken: bool,
}

/// A built-in exit rule, named by what it protects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitRule {
    /// Loss from the average entry reached the stop-loss threshold.
    StopLoss,
    /// The price fell the trailing distance from an armed basis peak while still in profit.
    Trailing,
    /// Profit from the average entry reached the target.
    TakeProfit,
    /// Held past the time limit at or below the loss threshold.
    TimeLoss,
}

impl ExitRule {
    /// The order the rules are checked in; the first that fires wins.
    pub const ORDER: [ExitRule; 4] = [
        ExitRule::StopLoss,
        ExitRule::Trailing,
        ExitRule::TakeProfit,
        ExitRule::TimeLoss,
    ];
}

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

/// The first rule in [`ExitRule::ORDER`] that fires for `inputs` under `policy`.
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
    for rule in ExitRule::ORDER {
        match fires(rule, inputs, policy) {
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

/// One rule on its own: `Some(exit_percentage)` when it fires.
fn fires(
    rule: ExitRule,
    inputs: &ExitInputs,
    policy: &ExitPolicy,
) -> crate::trader::Result<Option<Option<f64>>> {
    let entry_price = inputs.average_entry_price;
    let current_price = inputs.current_price;
    Ok(match rule {
        ExitRule::StopLoss => exit_stop_loss::stop_loss_exit(
            entry_price,
            current_price,
            inputs.held_seconds,
            inputs.partial_exits_taken,
            &policy.stop_loss,
        ),
        ExitRule::Trailing => exit_trailing::trailing_stop_triggered(
            entry_price,
            inputs.basis_peak_price,
            current_price,
            &policy.trailing,
        )?
        .then_some(None),
        ExitRule::TakeProfit => {
            exit_roi::roi_target_reached(entry_price, current_price, &policy.roi).then_some(None)
        }
        ExitRule::TimeLoss => exit_time::time_override_triggered(
            entry_price,
            current_price,
            inputs.held_seconds as f64,
            &policy.time,
        )?
        .then_some(None),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trader::evaluators::dca::DcaConfigSnapshot;
    use crate::trader::policy::{RoiPolicy, StopLossPolicy, TimePolicy, TrailingPolicy};

    /// Every rule enabled: stop at -30% (partial 50%), trail 10% once +20%, take profit at
    /// +100%, time loss after an hour at -10% or worse.
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
            roi: RoiPolicy {
                enabled: true,
                target_profit_pct: 100.0,
            },
            time: TimePolicy {
                enabled: true,
                loss_threshold_pct: -10.0,
                duration_seconds: 3600.0,
            },
            dca: DcaConfigSnapshot {
                enabled: false,
                max_count: 0,
                cooldown_minutes: 0,
                threshold_pct: 0.0,
                size_percentage: 0.0,
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
        }
    }

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
            (
                inputs(2.0, 1.0, 60, false),
                outcome(ExitRule::TakeProfit, None),
            ),
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
            outcome(ExitRule::TakeProfit, None)
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
        assert_eq!(later.outcome, outcome(ExitRule::TakeProfit, None));
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
}
