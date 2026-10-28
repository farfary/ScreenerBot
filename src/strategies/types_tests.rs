// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Tests of the strategy rules fingerprint: what it covers, what it ignores, the projection
//! it hashes, and the rules clock a decision is judged against.

use chrono::{Duration, TimeZone};
use serde_json::json;

use super::*;

fn parameter(value: serde_json::Value) -> Parameter {
    Parameter {
        default: value.clone(),
        value,
        constraints: None,
    }
}

fn condition(condition_type: &str, parameters: &[(&str, Parameter)]) -> Condition {
    Condition {
        condition_type: condition_type.to_owned(),
        parameters: parameters
            .iter()
            .map(|(name, parameter)| ((*name).to_owned(), parameter.clone()))
            .collect(),
    }
}

fn liquidity(threshold: serde_json::Value) -> RuleTree {
    RuleTree::leaf(condition(
        "LiquidityLevel",
        &[
            ("threshold", parameter(threshold)),
            ("comparison", parameter(json!("GREATER_EQUAL"))),
        ],
    ))
}

fn price_change() -> RuleTree {
    RuleTree {
        disabled: true,
        ..RuleTree::leaf(condition(
            "PriceChangePercent",
            &[
                ("percent", parameter(json!(2.5))),
                ("window", parameter(json!([1.0, 0.5]))),
            ],
        ))
    }
}

/// The fixed strategy the golden fingerprint is taken of.
fn golden() -> Strategy {
    let created_at = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
    Strategy {
        id: "golden".to_owned(),
        name: "Golden".to_owned(),
        description: Some("Fixed".to_owned()),
        strategy_type: StrategyType::Entry,
        enabled: true,
        priority: 10,
        timeframe: "5m".to_owned(),
        rules: RuleTree::branch(
            LogicalOperator::And,
            vec![liquidity(json!(10.0)), price_change()],
        ),
        parameters: HashMap::new(),
        created_at,
        updated_at: created_at,
        author: None,
        version: 1,
        entry_strategy_ids: Vec::new(),
        mode: TradingMode::Paper,
        rules_changed_at: created_at,
    }
}

fn exit_bound_to(ids: &[&str]) -> Strategy {
    Strategy {
        strategy_type: StrategyType::Exit,
        entry_strategy_ids: ids.iter().map(|id| (*id).to_owned()).collect(),
        ..golden()
    }
}

/// The projection is part of the stored data: a stored fingerprint computed by an earlier
/// build must keep matching, or every strategy's rules clock moves at the next open. A
/// change to the projection takes a new format prefix.
#[test]
fn the_fingerprint_of_a_fixed_strategy_is_stable() {
    assert_eq!(
        golden().rules_fingerprint(),
        "1:8b20535924221a4e4f364eeb531954739576d5d8c6eb1406b65a81aa2c49cecb"
    );
}

#[test]
fn the_fingerprint_ignores_everything_but_the_rules() {
    let fingerprint = golden().rules_fingerprint();
    let later = Utc.with_ymd_and_hms(2026, 6, 1, 0, 0, 0).unwrap();
    let mut reordered_rules = golden();
    // The same parameters inserted in the other order.
    reordered_rules.rules = RuleTree::branch(
        LogicalOperator::And,
        vec![
            RuleTree::leaf(Condition {
                condition_type: "LiquidityLevel".to_owned(),
                parameters: [
                    ("comparison", parameter(json!("GREATER_EQUAL"))),
                    ("threshold", parameter(json!(10.0))),
                ]
                .into_iter()
                .map(|(name, parameter)| (name.to_owned(), parameter))
                .collect(),
            }),
            price_change(),
        ],
    );
    let mut editor_metadata = golden();
    if let Some(children) = editor_metadata.rules.conditions.as_mut() {
        let threshold = children[0]
            .condition
            .as_mut()
            .and_then(|condition| condition.parameters.get_mut("threshold"))
            .unwrap();
        threshold.default = json!(99);
        threshold.constraints = Some(ParameterConstraints {
            min: Some(0.0),
            max: Some(1_000.0),
            options: None,
            format: None,
        });
    }
    let unchanged = [
        reordered_rules,
        editor_metadata,
        Strategy {
            name: "Renamed".to_owned(),
            description: None,
            priority: 99,
            enabled: false,
            mode: TradingMode::Live,
            author: Some("someone".to_owned()),
            version: 7,
            created_at: later,
            updated_at: later,
            rules_changed_at: later,
            parameters: HashMap::from([("unused".to_owned(), json!(1))]),
            ..golden()
        },
        Strategy {
            rules: RuleTree::branch(
                LogicalOperator::And,
                vec![liquidity(json!(10)), price_change()],
            ),
            ..golden()
        },
    ];
    for (index, strategy) in unchanged.iter().enumerate() {
        assert_eq!(strategy.rules_fingerprint(), fingerprint, "variant {index}");
    }
    assert_eq!(
        exit_bound_to(&["b", "a", "b"]).rules_fingerprint(),
        exit_bound_to(&["a", "b"]).rules_fingerprint(),
        "the binding is a set: order and duplicates do not count"
    );
}

#[test]
fn the_fingerprint_changes_with_every_rule() {
    let fingerprint = golden().rules_fingerprint();
    let with_rules = |rules: RuleTree| Strategy { rules, ..golden() };
    let enabled_price_change = RuleTree {
        disabled: false,
        ..price_change()
    };
    let changed = [
        (
            "a parameter value",
            with_rules(RuleTree::branch(
                LogicalOperator::And,
                vec![liquidity(json!(10.5)), price_change()],
            )),
        ),
        (
            "a condition type",
            with_rules(RuleTree::branch(
                LogicalOperator::And,
                vec![
                    RuleTree::leaf(condition(
                        "LiquidityTrend",
                        &[
                            ("threshold", parameter(json!(10.0))),
                            ("comparison", parameter(json!("GREATER_EQUAL"))),
                        ],
                    )),
                    price_change(),
                ],
            )),
        ),
        (
            "an operator",
            with_rules(RuleTree::branch(
                LogicalOperator::Or,
                vec![liquidity(json!(10.0)), price_change()],
            )),
        ),
        (
            "the child order",
            with_rules(RuleTree::branch(
                LogicalOperator::And,
                vec![price_change(), liquidity(json!(10.0))],
            )),
        ),
        (
            "a disabled flag",
            with_rules(RuleTree::branch(
                LogicalOperator::And,
                vec![liquidity(json!(10.0)), enabled_price_change],
            )),
        ),
        (
            "the timeframe",
            Strategy {
                timeframe: "1h".to_owned(),
                ..golden()
            },
        ),
        (
            "the type",
            Strategy {
                strategy_type: StrategyType::Exit,
                ..golden()
            },
        ),
    ];
    for (what, strategy) in changed {
        assert_ne!(strategy.rules_fingerprint(), fingerprint, "{what}");
    }
    assert_ne!(
        exit_bound_to(&["a"]).rules_fingerprint(),
        exit_bound_to(&["a", "b"]).rules_fingerprint(),
        "the binding set"
    );
    assert_ne!(
        exit_bound_to(&[]).rules_fingerprint(),
        exit_bound_to(&["a"]).rules_fingerprint(),
        "managing every position is not managing one strategy's"
    );
}

#[test]
fn a_decision_counts_as_under_the_current_rules_from_the_change_on() {
    let strategy = golden();
    let changed_at = strategy.rules_changed_at;
    assert!(strategy.decided_under_current_rules(changed_at));
    assert!(strategy.decided_under_current_rules(changed_at + Duration::milliseconds(1)));
    assert!(!strategy.decided_under_current_rules(changed_at - Duration::milliseconds(1)));
}
