// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Pure: the strategy engine's rule-tree evaluation, validation and result cache.
//!
//! The engine is what turns a user's saved condition tree into a buy or a sell. Two
//! classes of failure matter here and neither is visible at runtime: a boolean operator
//! that combines its children wrongly (the strategy fires on the opposite market), and a
//! cached result reused for a context it was not computed for (the bot trades on a stale
//! answer). Both are covered below, with the order enabled entry strategies are evaluated
//! in: each book on its own, lowest priority first, then by name, and the book's first
//! signal ends its evaluation.

use crate::common;

use common::{condition, context_bare, TEST_MINT};
use screenerbot::strategies::engine::{EngineConfig, StrategyEngine};
use screenerbot::strategies::types::{
    Condition, EvaluationContext, LogicalOperator, MarketData, PositionData, RuleOutcome, RuleTree,
    RuleVerdict, Strategy, StrategyType, TransactionCounts, WindowCounts,
};
use screenerbot::strategies::Error as StrategyError;
use screenerbot::trader::paper::TradingMode;
use serde_json::json;

/// A liquidity condition — deterministic, needs no OHLCV, and its truth is controlled
/// entirely by the context, which is what makes it a good probe for tree logic.
fn liquidity_at_least(threshold: f64) -> Condition {
    condition(
        "LiquidityLevel",
        &[
            ("threshold", json!(threshold)),
            ("comparison", json!("GREATER_EQUAL")),
        ],
    )
}

fn liquidity_below(threshold: f64) -> Condition {
    condition(
        "LiquidityLevel",
        &[
            ("threshold", json!(threshold)),
            ("comparison", json!("LESS_THAN")),
        ],
    )
}

fn leaf(cond: Condition) -> RuleTree {
    RuleTree::leaf(cond)
}

fn strategy(rules: RuleTree) -> Strategy {
    strategy_of(StrategyType::Entry, rules)
}

fn strategy_of(strategy_type: StrategyType, rules: RuleTree) -> Strategy {
    Strategy {
        id: "test-strategy".to_owned(),
        name: "Test".to_owned(),
        description: None,
        strategy_type,
        enabled: true,
        priority: 1,
        timeframe: "5m".to_owned(),
        rules,
        parameters: std::collections::HashMap::new(),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        author: None,
        version: 1,
        entry_strategy_ids: Vec::new(),
        mode: TradingMode::Live,
    }
}

fn context(liquidity_sol: f64) -> EvaluationContext {
    let mut ctx = context_bare(Some(1.0));
    ctx.token_mint = TEST_MINT.to_owned();
    ctx.market_data = Some(MarketData {
        liquidity_sol: Some(liquidity_sol),
        liquidity_pool: None,
        transactions: None,
        snapshot_at: None,
        token_created_at: None,
        reference_size_native: None,
    });
    ctx
}

/// An engine with caching OFF, so a test observes the tree, not the cache.
fn uncached_engine() -> StrategyEngine {
    StrategyEngine::new(EngineConfig {
        evaluation_timeout_ms: 5_000,
        cache_ttl_seconds: 0,
        max_concurrent_evaluations: 10,
    })
}

async fn evaluate(engine: &StrategyEngine, rules: RuleTree, liquidity: f64) -> bool {
    engine
        .evaluate_strategy(&strategy(rules), &context(liquidity))
        .await
        .expect("evaluation succeeded")
        .result
}

// ==================== LEAF ====================

#[tokio::test]
async fn a_single_leaf_is_its_condition() {
    let engine = uncached_engine();
    assert!(evaluate(&engine, leaf(liquidity_at_least(100.0)), 150.0).await);
    assert!(!evaluate(&engine, leaf(liquidity_at_least(100.0)), 50.0).await);
}

#[tokio::test]
async fn an_unknown_condition_type_errors_instead_of_evaluating_false() {
    // A strategy referencing a condition the build no longer ships must fail loudly.
    // Returning `false` would silently disable the user's strategy forever.
    let engine = uncached_engine();
    let rules = leaf(condition("NoSuchCondition", &[]));
    let err = engine
        .evaluate_strategy(&strategy(rules), &context(100.0))
        .await
        .expect_err("unknown condition must error");
    assert!(
        matches!(
            err,
            StrategyError::InvalidConditionValue {
                field: "condition type",
                ref value
            } if value == "NoSuchCondition"
        ),
        "got: {err}"
    );
}

#[tokio::test]
async fn a_leaf_validates_its_parameters_before_evaluating() {
    // Validation runs inside evaluation, so a malformed saved strategy cannot slip
    // through and evaluate on garbage parameters.
    let engine = uncached_engine();
    let rules = leaf(condition(
        "LiquidityLevel",
        &[
            ("threshold", json!(-5.0)),
            ("comparison", json!("GREATER_EQUAL")),
        ],
    ));
    let err = engine
        .evaluate_strategy(&strategy(rules), &context(100.0))
        .await
        .expect_err("invalid parameters must error");
    assert!(
        matches!(
            err,
            StrategyError::InvalidConditionValue {
                field: "threshold",
                ..
            }
        ),
        "got: {err}"
    );
}

// ==================== AND / OR / NOT ====================

#[tokio::test]
async fn and_requires_every_child() {
    let engine = uncached_engine();
    let both_true = RuleTree::branch(
        LogicalOperator::And,
        vec![
            leaf(liquidity_at_least(100.0)),
            leaf(liquidity_below(500.0)),
        ],
    );
    assert!(evaluate(&engine, both_true.clone(), 200.0).await);
    // Liquidity 600 satisfies the first child but not the second.
    assert!(!evaluate(&engine, both_true, 600.0).await);
}

#[tokio::test]
async fn or_needs_only_one_child() {
    let engine = uncached_engine();
    let rules = RuleTree::branch(
        LogicalOperator::Or,
        vec![
            leaf(liquidity_at_least(1000.0)),
            leaf(liquidity_below(50.0)),
        ],
    );
    assert!(evaluate(&engine, rules.clone(), 10.0).await); // second child
    assert!(evaluate(&engine, rules.clone(), 2000.0).await); // first child
    assert!(!evaluate(&engine, rules, 500.0).await); // neither
}

#[tokio::test]
async fn not_inverts_its_single_child() {
    let engine = uncached_engine();
    let rules = RuleTree::branch(LogicalOperator::Not, vec![leaf(liquidity_at_least(100.0))]);
    assert!(!evaluate(&engine, rules.clone(), 150.0).await);
    assert!(evaluate(&engine, rules, 50.0).await);
}

#[tokio::test]
async fn and_short_circuits_before_a_broken_later_child() {
    // The first child is false, so the unknown-condition child is never reached. This
    // is what makes short-circuiting observable without timing anything.
    let engine = uncached_engine();
    let rules = RuleTree::branch(
        LogicalOperator::And,
        vec![
            leaf(liquidity_at_least(1000.0)),
            leaf(condition("NoSuchCondition", &[])),
        ],
    );
    assert!(!evaluate(&engine, rules, 10.0).await);
}

#[tokio::test]
async fn or_short_circuits_before_a_broken_later_child() {
    let engine = uncached_engine();
    let rules = RuleTree::branch(
        LogicalOperator::Or,
        vec![
            leaf(liquidity_at_least(100.0)),
            leaf(condition("NoSuchCondition", &[])),
        ],
    );
    assert!(evaluate(&engine, rules, 500.0).await);
}

#[tokio::test]
async fn a_child_error_propagates_when_it_is_actually_reached() {
    // The mirror of short-circuiting: if the broken child IS evaluated, the whole
    // strategy must error rather than treat the failure as a `false`.
    let engine = uncached_engine();
    let rules = RuleTree::branch(
        LogicalOperator::And,
        vec![
            leaf(liquidity_at_least(100.0)),
            leaf(condition("NoSuchCondition", &[])),
        ],
    );
    assert!(engine
        .evaluate_strategy(&strategy(rules), &context(500.0))
        .await
        .is_err());
}

// ==================== THREE-VALUED LOGIC ====================
//
// A condition that cannot be answered for lack of data is Unknown: never true, never
// false. A group is still decided by a decisive child (False for And, True for Or) and
// stops there; an Unknown child never stops it. Any other error is a fault in the
// strategy and propagates the moment it is reached, so a fault placed after a decisive
// child is not reached (strategy validation at save time is what catches it there).

/// The four kinds of child a group can hold, in the order the probes below build them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Kind {
    True,
    False,
    Unknown,
    Fault,
}

const KINDS: [Kind; 4] = [Kind::True, Kind::False, Kind::Unknown, Kind::Fault];

/// What a strategy evaluation produced, by class.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Outcome {
    Signal(bool),
    DataUnavailable,
    Fault,
}

/// The liquidity of [`kleene_context`], which decides the two liquidity probes.
const PROBE_LIQUIDITY: f64 = 500.0;

/// A context with liquidity and nothing else: no transaction counts, no position.
fn kleene_context() -> EvaluationContext {
    context(PROBE_LIQUIDITY)
}

/// Buy pressure reads transaction counts, which [`kleene_context`] does not carry.
fn unknown_condition() -> Condition {
    condition(
        "BuyPressure",
        &[
            ("window", json!("1h")),
            ("min_buy_share_pct", json!(60.0)),
            ("min_transactions", json!(10.0)),
        ],
    )
}

fn probe(kind: Kind) -> RuleTree {
    leaf(match kind {
        Kind::True => liquidity_at_least(100.0),
        Kind::False => liquidity_at_least(1_000.0),
        Kind::Unknown => unknown_condition(),
        Kind::Fault => condition("NoSuchCondition", &[]),
    })
}

async fn outcome_of(engine: &StrategyEngine, rules: RuleTree) -> Outcome {
    match engine
        .evaluate_strategy(&strategy(rules), &kleene_context())
        .await
    {
        Ok(result) => Outcome::Signal(result.result),
        Err(e) if e.is_data_unavailable() => Outcome::DataUnavailable,
        Err(_) => Outcome::Fault,
    }
}

/// The contract, read left to right: a fault ends the group the moment it is reached, a
/// decisive child ends it with its value, an Unknown child is remembered and passed over.
fn contract(operator: LogicalOperator, children: &[Kind]) -> Outcome {
    let decisive = operator == LogicalOperator::Or;
    let mut unknown = false;
    for child in children {
        match child {
            Kind::Fault => return Outcome::Fault,
            Kind::True if decisive => return Outcome::Signal(true),
            Kind::False if !decisive => return Outcome::Signal(false),
            Kind::Unknown => unknown = true,
            Kind::True | Kind::False => {}
        }
    }
    if unknown {
        Outcome::DataUnavailable
    } else {
        Outcome::Signal(!decisive)
    }
}

fn negated(outcome: Outcome) -> Outcome {
    match outcome {
        Outcome::Signal(value) => Outcome::Signal(!value),
        other => other,
    }
}

/// Every sequence of up to four children drawn from the four kinds.
fn sequences() -> Vec<Vec<Kind>> {
    let mut all: Vec<Vec<Kind>> = vec![Vec::new()];
    let mut out = Vec::new();
    for _ in 0..4 {
        all = all
            .iter()
            .flat_map(|prefix| {
                KINDS.iter().map(move |kind| {
                    let mut next = prefix.clone();
                    next.push(*kind);
                    next
                })
            })
            .collect();
        out.extend(all.iter().cloned());
    }
    out
}

#[tokio::test]
async fn every_group_follows_the_three_valued_contract() {
    let engine = uncached_engine();
    let sequences = sequences();
    assert_eq!(sequences.len(), 4 + 16 + 64 + 256);
    for operator in [LogicalOperator::And, LogicalOperator::Or] {
        for children in &sequences {
            let group = RuleTree::branch(operator, children.iter().copied().map(probe).collect());
            let expected = contract(operator, children);
            assert_eq!(
                outcome_of(&engine, group.clone()).await,
                expected,
                "{operator:?} {children:?}"
            );
            let not = RuleTree::branch(LogicalOperator::Not, vec![group]);
            assert_eq!(
                outcome_of(&engine, not).await,
                negated(expected),
                "NOT {operator:?} {children:?}"
            );
        }
    }
}

#[tokio::test]
async fn child_order_never_changes_the_answer_except_where_a_fault_meets_a_decisive_child() {
    // The permutation property. Over true, false and unknown children the result is the
    // same in every order. A fault is reached in every order unless a decisive child
    // stands before it, so a group holding a fault and no decisive child always fails;
    // a group holding both is decided by which comes first, and is then either that
    // fault or the decisive value, never the opposite value and never unknown.
    let engine = uncached_engine();
    for operator in [LogicalOperator::And, LogicalOperator::Or] {
        let decisive = if operator == LogicalOperator::Or {
            Kind::True
        } else {
            Kind::False
        };
        for children in sequences() {
            let mut canonical = children.clone();
            canonical.sort();
            let got = outcome_of(
                &engine,
                RuleTree::branch(operator, children.iter().copied().map(probe).collect()),
            )
            .await;
            let has_fault = children.contains(&Kind::Fault);
            let has_decisive = children.contains(&decisive);
            if has_fault && has_decisive {
                let fault_first = children.iter().position(|kind| *kind == Kind::Fault)
                    < children.iter().position(|kind| *kind == decisive);
                let expected = if fault_first {
                    Outcome::Fault
                } else {
                    Outcome::Signal(decisive == Kind::True)
                };
                assert_eq!(got, expected, "{operator:?} {children:?}");
                continue;
            }
            let reordered = outcome_of(
                &engine,
                RuleTree::branch(operator, canonical.iter().copied().map(probe).collect()),
            )
            .await;
            assert_eq!(got, reordered, "{operator:?} {children:?} vs {canonical:?}");
            if has_fault {
                assert_eq!(got, Outcome::Fault, "{operator:?} {children:?}");
            }
        }
    }
}

#[tokio::test]
async fn and_or_not_follow_the_kleene_tables() {
    use Kind::{False as F, True as T, Unknown as U};
    use Outcome::{DataUnavailable as Unknown, Signal};
    let engine = uncached_engine();
    let table = [
        (T, T, Signal(true), Signal(true)),
        (T, F, Signal(false), Signal(true)),
        (T, U, Unknown, Signal(true)),
        (F, T, Signal(false), Signal(true)),
        (F, F, Signal(false), Signal(false)),
        (F, U, Signal(false), Unknown),
        (U, T, Unknown, Signal(true)),
        (U, F, Signal(false), Unknown),
        (U, U, Unknown, Unknown),
    ];
    for (left, right, and, or) in table {
        let pair = || vec![probe(left), probe(right)];
        assert_eq!(
            outcome_of(&engine, RuleTree::branch(LogicalOperator::And, pair())).await,
            and,
            "{left:?} AND {right:?}"
        );
        assert_eq!(
            outcome_of(&engine, RuleTree::branch(LogicalOperator::Or, pair())).await,
            or,
            "{left:?} OR {right:?}"
        );
    }
    for (child, not) in [(T, Signal(false)), (F, Signal(true)), (U, Unknown)] {
        assert_eq!(
            outcome_of(
                &engine,
                RuleTree::branch(LogicalOperator::Not, vec![probe(child)])
            )
            .await,
            not,
            "NOT {child:?}"
        );
    }
}

#[tokio::test]
async fn an_undecided_strategy_reports_the_first_data_error_it_reached() {
    // Two different missing inputs: the error names the one evaluated first, which is
    // the same "no signal" the trader logged before unknown children were passed over.
    let engine = uncached_engine();
    let missing_counts = leaf(unknown_condition());
    let missing_position = leaf(profit_condition());
    for operator in [LogicalOperator::And, LogicalOperator::Or] {
        for (children, data) in [
            (
                vec![missing_counts.clone(), missing_position.clone()],
                "transaction counts",
            ),
            (
                vec![missing_position.clone(), missing_counts.clone()],
                "position data",
            ),
        ] {
            let err = engine
                .evaluate_strategy(
                    &strategy(RuleTree::branch(operator, children)),
                    &kleene_context(),
                )
                .await
                .expect_err("an undecided root is no signal");
            assert!(
                matches!(err, StrategyError::MissingContextData { data: reported } if reported == data),
                "{operator:?}: got {err}"
            );
        }
    }
}

#[tokio::test]
async fn an_unknown_child_never_hides_a_fault_after_it() {
    // Unknown passes over to the next child, so a fault there is still reached and
    // reported as a fault, not as missing data.
    let engine = uncached_engine();
    for operator in [LogicalOperator::And, LogicalOperator::Or] {
        let err = engine
            .evaluate_strategy(
                &strategy(RuleTree::branch(
                    operator,
                    vec![probe(Kind::Unknown), probe(Kind::Fault)],
                )),
                &kleene_context(),
            )
            .await
            .expect_err("the fault is reached");
        assert!(
            matches!(
                err,
                StrategyError::InvalidConditionValue {
                    field: "condition type",
                    ..
                }
            ),
            "{operator:?}: got {err}"
        );
    }
}

#[tokio::test]
async fn nested_trees_evaluate_bottom_up() {
    // (liq >= 100 AND NOT (liq >= 1000)) OR liq < 10
    let engine = uncached_engine();
    let rules = RuleTree::branch(
        LogicalOperator::Or,
        vec![
            RuleTree::branch(
                LogicalOperator::And,
                vec![
                    leaf(liquidity_at_least(100.0)),
                    RuleTree::branch(LogicalOperator::Not, vec![leaf(liquidity_at_least(1000.0))]),
                ],
            ),
            leaf(liquidity_below(10.0)),
        ],
    );
    assert!(evaluate(&engine, rules.clone(), 500.0).await); // mid band
    assert!(!evaluate(&engine, rules.clone(), 5000.0).await); // too high
    assert!(!evaluate(&engine, rules.clone(), 50.0).await); // between the bands
    assert!(evaluate(&engine, rules, 5.0).await); // low escape hatch
}

// ==================== MALFORMED TREES ====================

#[tokio::test]
async fn not_with_multiple_children_is_rejected() {
    let engine = uncached_engine();
    let rules = RuleTree::branch(
        LogicalOperator::Not,
        vec![leaf(liquidity_at_least(100.0)), leaf(liquidity_below(10.0))],
    );
    let err = engine
        .evaluate_strategy(&strategy(rules.clone()), &context(100.0))
        .await
        .expect_err("NOT must take exactly one child");
    assert!(
        matches!(
            err,
            StrategyError::InvalidRuleTree {
                reason: "NOT operator must have exactly one child"
            }
        ),
        "got: {err}"
    );
    assert!(engine.validate_strategy(&strategy(rules)).is_err());
}

#[tokio::test]
async fn an_empty_node_is_neither_leaf_nor_branch_and_is_rejected() {
    let engine = uncached_engine();
    let empty = RuleTree {
        operator: None,
        conditions: None,
        condition: None,
        disabled: false,
    };
    let err = engine
        .evaluate_strategy(&strategy(empty.clone()), &context(100.0))
        .await
        .expect_err("empty node must error");
    assert!(
        matches!(err, StrategyError::InvalidRuleTree { .. }),
        "got: {err}"
    );
    assert!(engine.validate_strategy(&strategy(empty)).is_err());
}

#[tokio::test]
async fn a_branch_without_children_fails_validation() {
    let engine = uncached_engine();
    let rules = RuleTree::branch(LogicalOperator::And, vec![]);
    assert!(engine.validate_strategy(&strategy(rules)).is_err());
}

#[tokio::test]
async fn validation_recurses_into_every_child() {
    // A broken condition buried three levels deep must still be caught before the
    // strategy is ever saved or run.
    let engine = uncached_engine();
    let rules = RuleTree::branch(
        LogicalOperator::Or,
        vec![
            leaf(liquidity_at_least(100.0)),
            RuleTree::branch(
                LogicalOperator::And,
                vec![
                    leaf(liquidity_below(10.0)),
                    leaf(condition("NoSuchCondition", &[])),
                ],
            ),
        ],
    );
    assert!(engine.validate_strategy(&strategy(rules)).is_err());
}

#[tokio::test]
async fn a_well_formed_tree_passes_validation() {
    let engine = uncached_engine();
    let rules = RuleTree::branch(
        LogicalOperator::And,
        vec![
            leaf(liquidity_at_least(100.0)),
            RuleTree::branch(LogicalOperator::Not, vec![leaf(liquidity_below(10.0))]),
        ],
    );
    assert!(engine.validate_strategy(&strategy(rules)).is_ok());
}

// ==================== SWITCHED-OFF NODES AND NESTING ====================

/// The same node, switched off.
fn off(mut node: RuleTree) -> RuleTree {
    node.disabled = true;
    node
}

fn and(children: Vec<RuleTree>) -> RuleTree {
    RuleTree::branch(LogicalOperator::And, children)
}

/// A chain of `levels` groups ending in a leaf, so the leaf sits at level `levels + 1`.
fn nested_groups(levels: usize) -> RuleTree {
    (0..levels).fold(leaf(liquidity_at_least(1.0)), |inner, _| and(vec![inner]))
}

fn invalid_reason(result: Result<(), StrategyError>) -> &'static str {
    match result {
        Err(StrategyError::InvalidRuleTree { reason }) => reason,
        other => panic!("expected an invalid rule tree, got {other:?}"),
    }
}

#[tokio::test]
async fn a_switched_off_child_is_skipped_by_and() {
    // Alone, the off child (liquidity below 10) would be false and sink the AND.
    let engine = uncached_engine();
    let rules = and(vec![
        leaf(liquidity_at_least(100.0)),
        off(leaf(liquidity_below(10.0))),
    ]);
    assert!(evaluate(&engine, rules, 150.0).await);
}

#[tokio::test]
async fn a_switched_off_child_is_skipped_by_or() {
    // Alone, the off child (liquidity at least 10) would be true and fire the OR.
    let engine = uncached_engine();
    let rules = RuleTree::branch(
        LogicalOperator::Or,
        vec![
            leaf(liquidity_at_least(1000.0)),
            off(leaf(liquidity_at_least(10.0))),
        ],
    );
    assert!(!evaluate(&engine, rules, 150.0).await);
}

#[tokio::test]
async fn a_group_with_every_child_off_is_refused_not_vacuously_true() {
    let engine = uncached_engine();
    let rules = and(vec![off(leaf(liquidity_at_least(1.0)))]);
    assert_eq!(
        invalid_reason(engine.validate_strategy(&strategy(rules.clone()))),
        "a group needs at least one condition switched on"
    );
    let err = engine
        .evaluate_strategy(&strategy(rules), &context(150.0))
        .await
        .expect_err("a group with nothing switched on must not evaluate");
    assert!(
        matches!(err, StrategyError::InvalidRuleTree { .. }),
        "{err}"
    );
}

#[tokio::test]
async fn a_switched_off_root_is_refused() {
    let engine = uncached_engine();
    let rules = off(and(vec![leaf(liquidity_at_least(1.0))]));
    assert_eq!(
        invalid_reason(engine.validate_strategy(&strategy(rules.clone()))),
        "the root group cannot be switched off"
    );
    let err = engine
        .evaluate_strategy(&strategy(rules), &context(150.0))
        .await
        .expect_err("a switched-off root must not evaluate");
    assert!(
        matches!(err, StrategyError::InvalidRuleTree { .. }),
        "{err}"
    );
}

#[tokio::test]
async fn not_cannot_invert_a_switched_off_child() {
    let engine = uncached_engine();
    let rules = and(vec![
        leaf(liquidity_at_least(1.0)),
        RuleTree::branch(LogicalOperator::Not, vec![off(leaf(liquidity_below(10.0)))]),
    ]);
    assert_eq!(
        invalid_reason(engine.validate_strategy(&strategy(rules.clone()))),
        "NOT operator cannot have a switched-off child"
    );
    assert!(engine
        .evaluate_strategy(&strategy(rules), &context(150.0))
        .await
        .is_err());
}

#[test]
fn rules_nest_four_levels_deep_and_no_deeper() {
    let engine = uncached_engine();
    // Root group (1) -> group (2) -> group (3) -> leaf (4).
    assert!(engine
        .validate_strategy(&strategy(nested_groups(3)))
        .is_ok());
    // One more group puts the leaf at level 5.
    assert_eq!(
        invalid_reason(engine.validate_strategy(&strategy(nested_groups(4)))),
        "rules nest deeper than four levels"
    );
}

#[test]
fn a_switched_off_condition_is_still_validated() {
    let engine = uncached_engine();
    let malformed = condition(
        "LiquidityLevel",
        &[
            ("threshold", json!(-5.0)),
            ("comparison", json!("GREATER_EQUAL")),
        ],
    );
    let rules = and(vec![leaf(liquidity_at_least(1.0)), off(leaf(malformed))]);
    assert!(matches!(
        engine.validate_strategy(&strategy(rules)),
        Err(StrategyError::InvalidConditionValue {
            field: "threshold",
            ..
        })
    ));

    let unknown = and(vec![
        leaf(liquidity_at_least(1.0)),
        off(leaf(condition("NoSuchCondition", &[]))),
    ]);
    assert!(engine.validate_strategy(&strategy(unknown)).is_err());
}

fn liquidity_leaf_json() -> serde_json::Value {
    json!({
        "condition": {
            "type": "LiquidityLevel",
            "parameters": {
                "threshold": { "value": 100.0, "default": 50.0 },
                "comparison": { "value": "GREATER_EQUAL", "default": "GREATER_THAN" }
            }
        }
    })
}

#[test]
fn a_tree_saved_without_the_disabled_flag_loads_enabled() {
    let tree: RuleTree = serde_json::from_value(liquidity_leaf_json()).expect("loads");
    assert!(!tree.disabled);
}

#[test]
fn the_disabled_flag_is_written_only_when_set() {
    let enabled = serde_json::to_value(leaf(liquidity_at_least(1.0))).unwrap();
    assert!(enabled.get("disabled").is_none(), "{enabled}");

    let switched_off = serde_json::to_value(off(leaf(liquidity_at_least(1.0)))).unwrap();
    assert_eq!(switched_off["disabled"], json!(true));
    let reloaded: RuleTree = serde_json::from_value(switched_off).unwrap();
    assert!(reloaded.disabled);
}

// ==================== EXPLAINED EVALUATION ====================

fn verdict_of(kind: Kind) -> RuleVerdict {
    match kind {
        Kind::True => RuleVerdict::Pass,
        Kind::False => RuleVerdict::Fail,
        Kind::Unknown => RuleVerdict::Unknown,
        Kind::Fault => unreachable!("a fault has no verdict"),
    }
}

async fn explained(engine: &StrategyEngine, rules: RuleTree) -> RuleOutcome {
    engine
        .explain_strategy(&strategy(rules), &kleene_context())
        .await
        .expect("explanation succeeded")
}

#[tokio::test]
async fn an_explained_group_reaches_the_evaluated_verdict_and_reports_every_child() {
    // The explanation folds with the evaluation's rule, so its root says what the trader
    // would do; it never stops at a decisive child, so every condition reports itself.
    let engine = uncached_engine();
    let fault_free = sequences()
        .into_iter()
        .filter(|children| !children.contains(&Kind::Fault));
    for children in fault_free {
        for operator in [LogicalOperator::And, LogicalOperator::Or] {
            let group = RuleTree::branch(operator, children.iter().copied().map(probe).collect());
            let expected = match outcome_of(&engine, group.clone()).await {
                Outcome::Signal(true) => RuleVerdict::Pass,
                Outcome::Signal(false) => RuleVerdict::Fail,
                Outcome::DataUnavailable => RuleVerdict::Unknown,
                Outcome::Fault => panic!("{operator:?} {children:?} has no fault"),
            };
            let outcome = explained(&engine, group.clone()).await;
            assert_eq!(outcome.verdict, expected, "{operator:?} {children:?}");
            assert_eq!(outcome.operator, Some(operator));
            assert_eq!(
                outcome.reason.is_some(),
                expected == RuleVerdict::Unknown,
                "{operator:?} {children:?}"
            );
            assert_eq!(
                outcome
                    .children
                    .iter()
                    .map(|child| child.verdict)
                    .collect::<Vec<_>>(),
                children.iter().copied().map(verdict_of).collect::<Vec<_>>(),
                "{operator:?} {children:?}"
            );

            let not = explained(&engine, RuleTree::branch(LogicalOperator::Not, vec![group])).await;
            let negated = match expected {
                RuleVerdict::Pass => RuleVerdict::Fail,
                RuleVerdict::Fail => RuleVerdict::Pass,
                other => other,
            };
            assert_eq!(not.verdict, negated, "NOT {operator:?} {children:?}");
            assert_eq!(not.children.len(), 1);
        }
    }
}

#[tokio::test]
async fn an_explained_tree_names_each_node_and_reports_switched_off_ones_as_off() {
    let engine = uncached_engine();
    let rules = and(vec![
        probe(Kind::True),
        off(probe(Kind::False)),
        RuleTree::branch(
            LogicalOperator::Not,
            vec![RuleTree::branch(
                LogicalOperator::Or,
                vec![probe(Kind::Unknown), probe(Kind::False)],
            )],
        ),
    ]);
    let outcome = serde_json::to_value(explained(&engine, rules).await).expect("serializes");
    assert_eq!(
        outcome,
        json!({
            "verdict": "unknown",
            "operator": "AND",
            "reason": outcome["children"][2]["reason"],
            "children": [
                { "verdict": "pass", "condition_type": "LiquidityLevel" },
                { "verdict": "off", "condition_type": "LiquidityLevel" },
                {
                    "verdict": "unknown",
                    "operator": "NOT",
                    "reason": outcome["children"][2]["children"][0]["children"][0]["reason"],
                    "children": [{
                        "verdict": "unknown",
                        "operator": "OR",
                        "reason": outcome["children"][2]["children"][0]["children"][0]["reason"],
                        "children": [
                            {
                                "verdict": "unknown",
                                "condition_type": "BuyPressure",
                                "reason": outcome["children"][2]["children"][0]["children"][0]["reason"],
                            },
                            { "verdict": "fail", "condition_type": "LiquidityLevel" },
                        ],
                    }],
                },
            ],
        })
    );
    assert_eq!(
        outcome["reason"]["id"],
        json!("strategies-error-missing-data"),
        "an undecided node carries the data error it reached: {outcome}"
    );
}

#[tokio::test]
async fn an_explanation_refuses_what_the_evaluation_refuses() {
    let engine = uncached_engine();
    for rules in [
        and(vec![probe(Kind::True), probe(Kind::Fault)]),
        and(vec![off(probe(Kind::True))]),
        off(and(vec![probe(Kind::True)])),
    ] {
        assert!(engine
            .explain_strategy(&strategy(rules), &kleene_context())
            .await
            .is_err());
    }
}

// ==================== APPLICABILITY ====================

fn profit_condition() -> Condition {
    condition(
        "PositionProfit",
        &[
            ("comparison", json!("ABOVE")),
            ("threshold_pct", json!(20.0)),
        ],
    )
}

#[test]
fn an_exit_only_condition_is_refused_in_an_entry_strategy() {
    let engine = uncached_engine();
    let rules = and(vec![leaf(profit_condition())]);
    assert!(matches!(
        engine.validate_strategy(&strategy_of(StrategyType::Entry, rules.clone())),
        Err(StrategyError::ConditionNotApplicable {
            ref condition_type,
            strategy_type: StrategyType::Entry,
        }) if condition_type == "PositionProfit"
    ));
    assert!(engine
        .validate_strategy(&strategy_of(StrategyType::Exit, rules))
        .is_ok());
}

#[test]
fn a_switched_off_exit_only_condition_is_still_refused_in_an_entry_strategy() {
    let engine = uncached_engine();
    let rules = and(vec![
        leaf(liquidity_at_least(1.0)),
        off(leaf(profit_condition())),
    ]);
    assert!(matches!(
        engine.validate_strategy(&strategy_of(StrategyType::Entry, rules)),
        Err(StrategyError::ConditionNotApplicable { .. })
    ));
}

#[test]
fn every_schema_declares_the_strategy_types_it_applies_to() {
    let schemas = uncached_engine().get_condition_registry().get_all_schemas();
    let exit_only = [
        "PositionHoldingTime",
        "PositionProfit",
        "DrawdownFromPeak",
        "ProfitGiveback",
        "PositionAdds",
        "LiquidityChangeSinceEntry",
    ];
    let entry_only = ["TokenAge"];
    for (name, schema) in schemas.as_object().expect("schemas are an object") {
        let expected = if exit_only.contains(&name.as_str()) {
            json!(["EXIT"])
        } else if entry_only.contains(&name.as_str()) {
            json!(["ENTRY"])
        } else {
            json!(["ENTRY", "EXIT"])
        };
        assert_eq!(schema["applies_to"], expected, "{name}");
    }
}

// ==================== CACHE ====================
//
// Every assertion below is on the RESULT, never on `execution_time_ms`: a trivial tree
// evaluates in well under a millisecond, so that counter is legitimately 0 on both the
// cached and the fresh path and proves nothing.
//
// To observe whether a cached answer was reused, these tests exploit the one input the
// fingerprint deliberately summarises rather than hashes in full: a `TimeframeBundle`
// is represented by its build TIMESTAMP, not by its candles. Two bundles stamped with
// the same instant are therefore the same cache key even if their candles differ, so
// "cached" and "re-evaluated" produce visibly different booleans. That summary is sound
// in production — a rebuilt bundle always carries a new timestamp, and the TTL is
// seconds — but it is exactly the lever a test needs.

fn cached_engine() -> StrategyEngine {
    StrategyEngine::new(EngineConfig {
        evaluation_timeout_ms: 5_000,
        cache_ttl_seconds: 60,
        max_concurrent_evaluations: 10,
    })
}

/// A context whose 1m series closes flat at 100, stamped at a FIXED instant so two
/// contexts can share a cache key while differing in their candles.
fn candle_context(current_price: f64, close: f64) -> EvaluationContext {
    let mut bundle = common::bundle_with(
        "1m",
        common::candle_series(common::anchor_ts(), 60, &[close; 20]),
    );
    bundle.timestamp = chrono::DateTime::parse_from_rfc3339("2026-01-01T00:00:00Z")
        .expect("fixed bundle stamp")
        .with_timezone(&chrono::Utc);
    let mut ctx = common::context_with_candles(current_price, "1m", bundle);
    ctx.token_mint = TEST_MINT.to_owned();
    ctx
}

/// "Price gained at least 10% over the last 5 minutes" — true or false purely from the
/// current price against the candle closes.
fn gained_ten_percent() -> Condition {
    condition(
        "PriceChangePercent",
        &[
            ("percentage", json!(10.0)),
            ("direction", json!("ABOVE")),
            ("time_value", json!(5.0)),
            ("time_unit", json!("MINUTES")),
        ],
    )
}

#[tokio::test]
async fn a_repeated_evaluation_is_served_from_cache() {
    let engine = cached_engine();
    let s = strategy(leaf(gained_ten_percent()));

    // Price 110 against closes of 100 -> +10% -> true, and gets cached.
    assert!(
        engine
            .evaluate_strategy(&s, &candle_context(110.0, 100.0))
            .await
            .unwrap()
            .result
    );

    // Same price and same bundle stamp, but the closes moved to 110 — a fresh
    // evaluation would answer `false` (0% change). The cached `true` proves the second
    // call never walked the tree.
    assert!(
        engine
            .evaluate_strategy(&s, &candle_context(110.0, 110.0))
            .await
            .unwrap()
            .result,
        "an identical cache key must return the cached answer"
    );
}

#[tokio::test]
async fn a_changed_price_busts_the_cache() {
    // The whole risk of caching a trading decision. The price is part of the
    // fingerprint, so a moved price is a new question and must be re-answered.
    let engine = cached_engine();
    let s = strategy(leaf(gained_ten_percent()));

    assert!(
        engine
            .evaluate_strategy(&s, &candle_context(110.0, 100.0))
            .await
            .unwrap()
            .result
    );
    assert!(
        !engine
            .evaluate_strategy(&s, &candle_context(100.0, 100.0))
            .await
            .unwrap()
            .result,
        "a price that no longer qualifies must not inherit the cached buy signal"
    );
}

#[tokio::test]
async fn a_changed_market_input_busts_the_cache() {
    // Liquidity collapsing from 500 to 5 must produce a fresh `false`, never the
    // cached `true` — market data is fingerprinted for exactly this reason.
    let engine = cached_engine();
    let s = strategy(leaf(liquidity_at_least(100.0)));

    assert!(
        engine
            .evaluate_strategy(&s, &context(500.0))
            .await
            .unwrap()
            .result
    );
    assert!(
        !engine
            .evaluate_strategy(&s, &context(5.0))
            .await
            .unwrap()
            .result,
        "a collapsed-liquidity context must be re-evaluated"
    );
}

/// An exit context: a position entered at 4 whose highest price was `peak`, priced at 6.
fn exit_context(peak: f64) -> EvaluationContext {
    let mut ctx = common::context_bare(Some(6.0));
    ctx.token_mint = TEST_MINT.to_owned();
    ctx.position_data = Some(PositionData {
        entry_price: 4.0,
        entry_time: chrono::DateTime::parse_from_rfc3339("2026-01-01T00:00:00Z")
            .expect("fixed entry time")
            .with_timezone(&chrono::Utc),
        current_size_sol: 1.0,
        unrealized_profit_pct: Some(50.0),
        position_age_hours: 1.0,
        peak_price: Some(peak),
        dca_count: 0,
        partial_exit_count: 0,
        remaining_value_native: None,
        entry_liquidity_native: None,
        entry_liquidity_pool: None,
    });
    ctx
}

#[tokio::test]
async fn a_changed_peak_price_busts_the_cache() {
    // Peak 8 against a price of 6 is a 25% drawdown; peak 6 is none.
    let engine = cached_engine();
    let s = strategy_of(
        StrategyType::Exit,
        leaf(condition(
            "DrawdownFromPeak",
            &[
                ("drop_pct", json!(20.0)),
                ("arm_after_profit_pct", json!(0.0)),
            ],
        )),
    );

    assert!(
        engine
            .evaluate_strategy(&s, &exit_context(8.0))
            .await
            .unwrap()
            .result
    );
    assert!(
        !engine
            .evaluate_strategy(&s, &exit_context(6.0))
            .await
            .unwrap()
            .result,
        "a position whose peak moved must be re-evaluated"
    );
}

fn counts_context(buys: u64, sells: u64) -> EvaluationContext {
    let mut ctx = context(100.0);
    ctx.market_data = Some(MarketData {
        liquidity_sol: Some(100.0),
        liquidity_pool: None,
        transactions: Some(TransactionCounts {
            m5: None,
            h1: Some(WindowCounts { buys, sells }),
            h6: None,
            h24: None,
        }),
        snapshot_at: Some(common::anchor_time()),
        token_created_at: None,
        reference_size_native: None,
    });
    ctx
}

#[tokio::test]
async fn a_changed_transaction_count_busts_the_cache() {
    let engine = cached_engine();
    let s = strategy(leaf(condition(
        "BuyPressure",
        &[
            ("window", json!("1h")),
            ("min_buy_share_pct", json!(60.0)),
            ("min_transactions", json!(50)),
        ],
    )));

    assert!(
        engine
            .evaluate_strategy(&s, &counts_context(60, 40))
            .await
            .unwrap()
            .result
    );
    assert!(
        !engine
            .evaluate_strategy(&s, &counts_context(50, 50))
            .await
            .unwrap()
            .result,
        "a window whose counts moved must be re-evaluated"
    );
}

#[tokio::test]
async fn two_tokens_never_share_a_cache_entry() {
    // Sharing an entry across mints would trade one token on another token's signal.
    // Both contexts are identical apart from the mint, and the candles differ, so a
    // shared key would surface as the wrong boolean.
    let engine = cached_engine();
    let s = strategy(leaf(gained_ten_percent()));

    let mut first = candle_context(110.0, 100.0);
    first.token_mint = "MintAAA111111111111111111111111111111111111".to_owned();
    assert!(engine.evaluate_strategy(&s, &first).await.unwrap().result);

    let mut second = candle_context(110.0, 110.0);
    second.token_mint = "MintBBB222222222222222222222222222222222222".to_owned();
    assert!(
        !engine.evaluate_strategy(&s, &second).await.unwrap().result,
        "a different mint must be evaluated on its own data"
    );
}

#[tokio::test]
async fn two_strategies_never_share_a_cache_entry() {
    let engine = cached_engine();
    let ctx = context(500.0);

    let mut a = strategy(leaf(liquidity_at_least(100.0)));
    a.id = "strategy-a".to_owned();
    assert!(engine.evaluate_strategy(&a, &ctx).await.unwrap().result);

    // Same context, opposite condition: it must answer for itself.
    let mut b = strategy(leaf(liquidity_below(100.0)));
    b.id = "strategy-b".to_owned();
    assert!(
        !engine.evaluate_strategy(&b, &ctx).await.unwrap().result,
        "a second strategy must not inherit the first one's cached answer"
    );
}

#[tokio::test]
async fn clearing_the_cache_forces_a_fresh_evaluation() {
    let engine = cached_engine();
    let s = strategy(leaf(gained_ten_percent()));

    assert!(
        engine
            .evaluate_strategy(&s, &candle_context(110.0, 100.0))
            .await
            .unwrap()
            .result
    );

    engine.clear_cache().await;

    assert!(
        !engine
            .evaluate_strategy(&s, &candle_context(110.0, 110.0))
            .await
            .unwrap()
            .result,
        "after a clear the tree must be walked against the current data"
    );
}

#[tokio::test]
async fn a_zero_ttl_disables_caching_entirely() {
    let engine = uncached_engine();
    let s = strategy(leaf(gained_ten_percent()));

    assert!(
        engine
            .evaluate_strategy(&s, &candle_context(110.0, 100.0))
            .await
            .unwrap()
            .result
    );
    assert!(
        !engine
            .evaluate_strategy(&s, &candle_context(110.0, 110.0))
            .await
            .unwrap()
            .result,
        "cache_ttl_seconds = 0 must mean no cache"
    );
}

// ==================== REGISTRY ====================

#[test]
fn the_registry_exposes_every_built_in_condition() {
    // The dashboard's condition picker is built from this list; a condition missing
    // here is a condition users cannot select, even though strategies may use it.
    let engine = uncached_engine();
    let mut types = engine.get_condition_registry().list_types();
    types.sort();
    assert_eq!(
        types,
        vec![
            "BollingerBands",
            "BuyPressure",
            "CandleSize",
            "ConsecutiveCandles",
            "DrawdownFromPeak",
            "LiquidityChangeSinceEntry",
            "LiquidityLevel",
            "LiquidityToSize",
            "MaCross",
            "PositionAdds",
            "PositionHoldingTime",
            "PositionProfit",
            "PriceBreakout",
            "PriceChangePercent",
            "PriceToMA",
            "ProfitGiveback",
            "PullbackFromHigh",
            "Rsi",
            "TimeOfDay",
            "TokenAge",
            "TransactionVelocity",
            "VolatilityRange",
            "VolumeSpike",
            "Vwap",
        ]
    );
}

#[test]
fn every_registered_condition_publishes_a_parameter_schema() {
    let engine = uncached_engine();
    let schemas = engine.get_condition_registry().get_all_schemas();
    let object = schemas.as_object().expect("schemas are an object");
    assert_eq!(object.len(), 24);
    for (name, schema) in object {
        assert!(
            schema.get("parameters").is_some(),
            "{name} publishes no parameters block"
        );
    }
}

/// The candle timeframes a strategy reads are those of its switched-on candle rules: a
/// rule's own timeframe, else the strategy's, each once. A rule that reads no candles, a
/// switched-off rule and an unknown condition type add none, so a live test fetches only
/// the series the evaluation will read.
#[test]
fn a_strategy_reads_the_candle_timeframes_of_its_switched_on_candle_rules() {
    use screenerbot::ohlcvs::Timeframe;

    let candles = |timeframe: Option<&str>| {
        let mut params = vec![
            ("period", json!(14)),
            ("comparison", json!("BELOW")),
            ("threshold", json!(30)),
        ];
        if let Some(timeframe) = timeframe {
            params.push(("timeframe", json!(timeframe)));
        }
        RuleTree::leaf(condition("Rsi", &params))
    };
    let mut switched_off = candles(Some("1d"));
    switched_off.disabled = true;
    let rules = RuleTree::branch(
        LogicalOperator::And,
        vec![
            candles(Some("1h")),
            RuleTree::leaf(liquidity_at_least(10.0)),
            RuleTree::branch(
                LogicalOperator::Or,
                vec![candles(None), candles(Some("1h")), switched_off],
            ),
            RuleTree::leaf(condition("NoSuchCondition", &[("timeframe", json!("4h"))])),
        ],
    );

    let registry = uncached_engine();
    let registry = registry.get_condition_registry();
    assert_eq!(
        registry.candle_timeframes(&rules, "5m"),
        [Timeframe::Hour1, Timeframe::Minute5]
    );
    assert!(
        registry
            .candle_timeframes(&RuleTree::leaf(liquidity_at_least(10.0)), "5m")
            .is_empty(),
        "a strategy without a candle rule reads no candles"
    );
}

// ==================== EVALUATION ORDER ====================

/// A paper entry strategy with `priority` whose rule needs `threshold` SOL of liquidity,
/// stored under the id `name`.
fn stored_entry(name: &str, priority: i32, threshold: f64, enabled: bool) -> Strategy {
    stored_entry_of(TradingMode::Paper, name, priority, threshold, enabled)
}

/// An entry strategy of `mode` with `priority` whose rule needs `threshold` SOL of
/// liquidity, stored under the id `name`.
fn stored_entry_of(
    mode: TradingMode,
    name: &str,
    priority: i32,
    threshold: f64,
    enabled: bool,
) -> Strategy {
    let strategy = Strategy {
        id: name.to_owned(),
        name: name.to_owned(),
        priority,
        enabled,
        mode,
        ..strategy(leaf(liquidity_at_least(threshold)))
    };
    screenerbot::strategies::db::insert_strategy(&strategy).expect("insert strategy");
    strategy
}

async fn init_strategy_system() {
    screenerbot::strategies::init_strategy_system(EngineConfig {
        evaluation_timeout_ms: 5_000,
        cache_ttl_seconds: 0,
        max_concurrent_evaluations: 10,
    })
    .await
    .expect("initialise the strategy system");
}

/// Waits until every evaluation recorded before `sentinel` is stored: evaluations are
/// recorded in order through one queue, so once the disabled `sentinel` evaluated after
/// them is stored, every evaluation before it is stored too.
async fn flush_evaluations(sentinel: Strategy) {
    let id = sentinel.id.clone();
    screenerbot::strategies::evaluate_entry_strategies(
        &[sentinel],
        TradingMode::Paper,
        TEST_MINT,
        1.0,
        Some(&liquidity(50.0)),
        None,
    )
    .await
    .expect("evaluate the sentinel");
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while recorded(&id).is_none() {
        assert!(
            std::time::Instant::now() < deadline,
            "the sentinel evaluation was never recorded"
        );
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
}

fn liquidity(liquidity_sol: f64) -> MarketData {
    MarketData {
        liquidity_sol: Some(liquidity_sol),
        liquidity_pool: None,
        transactions: None,
        snapshot_at: None,
        token_created_at: None,
        reference_size_native: None,
    }
}

/// The evaluations recorded for `strategy_id`, `None` when it was never evaluated.
fn recorded(strategy_id: &str) -> Option<(u64, u64)> {
    screenerbot::strategies::db::get_strategy_performance(strategy_id)
        .expect("read evaluations")
        .map(|performance| {
            (
                performance.total_evaluations,
                performance.successful_signals,
            )
        })
}

/// The enabled entry strategies are read lowest priority first with the name breaking a
/// tie, the first one of the book that signals decides that book's entry, and no strategy
/// of the book after it is evaluated for that token.
#[tokio::test]
async fn entry_strategies_run_lowest_priority_first_and_stop_at_the_first_signal() {
    let _dir = common::isolated_env();
    init_strategy_system().await;

    // Stored in the reverse of the expected order, so only the read's ordering can sort them.
    stored_entry("late", 9, 10.0, true);
    stored_entry("beta", 5, 10.0, true);
    stored_entry("alpha", 5, 10.0, true);
    stored_entry("early", 3, 1_000_000.0, true);
    stored_entry("first", 1, 1_000_000.0, true);
    let sentinel = stored_entry("sentinel", 50, 10.0, false);

    let strategies =
        screenerbot::strategies::db::get_enabled_strategies(StrategyType::Entry).expect("read");
    let order: Vec<&str> = strategies.iter().map(|s| s.id.as_str()).collect();
    assert_eq!(order, ["first", "early", "alpha", "beta", "late"]);

    let decided = screenerbot::strategies::evaluate_entry_strategies(
        &strategies,
        TradingMode::Paper,
        TEST_MINT,
        1.0,
        Some(&liquidity(50.0)),
        None,
    )
    .await
    .expect("evaluate the entry strategies");
    assert_eq!(decided, Some("alpha".to_owned()));

    flush_evaluations(sentinel).await;
    assert_eq!(recorded("first"), Some((1, 0)));
    assert_eq!(recorded("early"), Some((1, 0)));
    assert_eq!(recorded("alpha"), Some((1, 1)));
    assert_eq!(
        recorded("beta"),
        None,
        "a strategy after the signal was evaluated"
    );
    assert_eq!(
        recorded("late"),
        None,
        "a strategy after the signal was evaluated"
    );
}

/// Each book runs its own pass and stops at its own first signal: a paper strategy ranked
/// above every live one never takes the token from the live pass, a live strategy never
/// takes it from the paper pass, and a live strategy above the paper signal that does not
/// signal stops nothing.
#[tokio::test]
async fn each_book_stops_at_its_own_first_signal() {
    let _dir = common::isolated_env();
    init_strategy_system().await;

    stored_entry_of(TradingMode::Live, "l0", 0, 1_000_000.0, true);
    stored_entry_of(TradingMode::Paper, "p1", 1, 10.0, true);
    stored_entry_of(TradingMode::Live, "l1", 2, 10.0, true);
    stored_entry_of(TradingMode::Live, "l2", 3, 10.0, true);
    stored_entry_of(TradingMode::Paper, "p2", 4, 10.0, true);
    let sentinel = stored_entry("sentinel", 50, 10.0, false);

    let strategies =
        screenerbot::strategies::db::get_enabled_strategies(StrategyType::Entry).expect("read");
    let market = liquidity(50.0);
    let decide = |mode| {
        screenerbot::strategies::evaluate_entry_strategies(
            &strategies,
            mode,
            TEST_MINT,
            1.0,
            Some(&market),
            None,
        )
    };
    let live = decide(TradingMode::Live).await.expect("live pass");
    let paper = decide(TradingMode::Paper).await.expect("paper pass");
    assert_eq!(
        live,
        Some("l1".to_owned()),
        "the paper signal took the live pass"
    );
    assert_eq!(
        paper,
        Some("p1".to_owned()),
        "a live signal took the paper pass"
    );

    flush_evaluations(sentinel).await;
    assert_eq!(recorded("l0"), Some((1, 0)));
    assert_eq!(recorded("p1"), Some((1, 1)));
    assert_eq!(recorded("l1"), Some((1, 1)));
    assert_eq!(
        recorded("l2"),
        None,
        "a live strategy after the live signal was evaluated"
    );
    assert_eq!(
        recorded("p2"),
        None,
        "a paper strategy after the paper signal was evaluated"
    );
}
