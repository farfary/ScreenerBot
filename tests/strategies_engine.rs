//! Pure: the strategy engine's rule-tree evaluation, validation and result cache.
//!
//! The engine is what turns a user's saved condition tree into a buy or a sell. Two
//! classes of failure matter here and neither is visible at runtime: a boolean operator
//! that combines its children wrongly (the strategy fires on the opposite market), and a
//! cached result reused for a context it was not computed for (the bot trades on a stale
//! answer). Both are covered below.

mod common;

use common::{condition, context_bare, TEST_MINT};
use screenerbot::strategies::engine::{EngineConfig, StrategyEngine};
use screenerbot::strategies::types::{
    Condition, EvaluationContext, LogicalOperator, MarketData, RuleTree, Strategy, StrategyType,
};
use screenerbot::strategies::Error as StrategyError;
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
    Strategy {
        id: "test-strategy".to_owned(),
        name: "Test".to_owned(),
        description: None,
        strategy_type: StrategyType::Entry,
        enabled: true,
        priority: 1,
        timeframe: "5m".to_owned(),
        rules,
        parameters: std::collections::HashMap::new(),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        author: None,
        version: 1,
    }
}

fn context(liquidity_sol: f64) -> EvaluationContext {
    let mut ctx = context_bare(Some(1.0));
    ctx.token_mint = TEST_MINT.to_owned();
    ctx.market_data = Some(MarketData {
        liquidity_sol: Some(liquidity_sol),
        volume_24h: None,
        market_cap: None,
        holder_count: None,
        token_age_hours: None,
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
            "CandleSize",
            "ConsecutiveCandles",
            "LiquidityLevel",
            "PositionHoldingTime",
            "PriceBreakout",
            "PriceChangePercent",
            "PriceToMA",
            "VolumeSpike",
        ]
    );
}

#[test]
fn every_registered_condition_publishes_a_parameter_schema() {
    let engine = uncached_engine();
    let schemas = engine.get_condition_registry().get_all_schemas();
    let object = schemas.as_object().expect("schemas are an object");
    assert_eq!(object.len(), 8);
    for (name, schema) in object {
        assert!(
            schema.get("parameters").is_some(),
            "{name} publishes no parameters block"
        );
    }
}
