//! Pure: what the trading system does when the DATA behind a decision is missing, stale
//! or degenerate. No network, no DB, no clock.
//!
//! Every automated buy and sell is the end of a data path:
//!
//! ```text
//! pool price ─┐
//! OHLCV bundle┼─> EvaluationContext ─> condition tree ─> StrategyEngine ─> TradeDecision
//! market data ┘                                            ^
//! connectivity (rpc/dexscreener/rugcheck) ─> entry admission
//! ```
//!
//! The condition math itself is covered by `strategies_conditions.rs` and the tree/cache
//! by `strategies_engine.rs`. This file covers the SEAMS instead: the exact context the
//! trader hands to a strategy, what each rule does when a piece of that context never
//! arrived, and what the entry gate does when the security-data endpoint is not there.
//!
//! The failure mode being hunted is silence. A missing input that produces `false` is
//! indistinguishable from a working rule that did not fire, and a degenerate number
//! (`0.0`, `NaN`) that survives a percentage calculation produces `inf` — which passes
//! every "did the price move enough?" test there is. Both spend SOL without a bug report.
//!
//! Each rule here is a guard that was added after this file first pinned the behaviour
//! without it, so the comments name both the answer and the wrong answer it replaced.

mod common;

use common::{
    anchor_ts, bundle_with, candle, candle_series, condition, context_bare, context_with_candles,
    TEST_MINT,
};
use screenerbot::ohlcvs::{Candle, TimeframeBundle};
use screenerbot::strategies::conditions::{
    ConditionEvaluator, ConsecutiveCandlesCondition, LiquidityLevelCondition,
    PriceBreakoutCondition, PriceChangePercentCondition, PriceToMaCondition, VolumeSpikeCondition,
};
use screenerbot::strategies::engine::{EngineConfig, StrategyEngine};
use screenerbot::strategies::types::{
    Condition, EvaluationContext, LogicalOperator, MarketData, PositionData, RuleTree, Strategy,
    StrategyType,
};
use serde_json::json;

const MINUTE: i64 = 60;
const HOUR: i64 = 3_600;

// ==================== HARNESS ====================

async fn eval(
    evaluator: &dyn ConditionEvaluator,
    cond: &Condition,
    ctx: &EvaluationContext,
) -> bool {
    evaluator
        .evaluate(cond, ctx)
        .await
        .unwrap_or_else(|e| panic!("evaluation failed: {e}"))
}

async fn eval_err(
    evaluator: &dyn ConditionEvaluator,
    cond: &Condition,
    ctx: &EvaluationContext,
) -> String {
    evaluator
        .evaluate(cond, ctx)
        .await
        .expect_err("expected an error, got a boolean verdict")
}

fn uncached_engine() -> StrategyEngine {
    StrategyEngine::new(EngineConfig {
        evaluation_timeout_ms: 5_000,
        cache_ttl_seconds: 0,
        max_concurrent_evaluations: 10,
    })
}

fn strategy(strategy_type: StrategyType, timeframe: &str, rules: RuleTree) -> Strategy {
    Strategy {
        id: "data-readiness".to_owned(),
        name: "Data readiness".to_owned(),
        description: None,
        strategy_type,
        enabled: true,
        priority: 1,
        timeframe: timeframe.to_owned(),
        rules,
        parameters: std::collections::HashMap::new(),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        author: None,
        version: 1,
    }
}

// ---- the conditions used as probes ----

fn liquidity_below(threshold: f64) -> Condition {
    condition(
        "LiquidityLevel",
        &[
            ("threshold", json!(threshold)),
            ("comparison", json!("LESS_THAN")),
        ],
    )
}

fn liquidity_at_least(threshold: f64) -> Condition {
    condition(
        "LiquidityLevel",
        &[
            ("threshold", json!(threshold)),
            ("comparison", json!("GREATER_EQUAL")),
        ],
    )
}

fn price_change(percentage: f64, direction: &str, minutes: f64) -> Condition {
    condition(
        "PriceChangePercent",
        &[
            ("percentage", json!(percentage)),
            ("direction", json!(direction)),
            ("time_value", json!(minutes)),
            ("time_unit", json!("MINUTES")),
        ],
    )
}

fn price_to_ma(period: i64, position: &str, distance: f64) -> Condition {
    condition(
        "PriceToMA",
        &[
            ("period", json!(period)),
            ("position", json!(position)),
            ("distance", json!(distance)),
        ],
    )
}

fn consecutive(count: i64, direction: &str, minimum_change: f64) -> Condition {
    condition(
        "ConsecutiveCandles",
        &[
            ("count", json!(count)),
            ("direction", json!(direction)),
            ("minimum_change", json!(minimum_change)),
        ],
    )
}

fn breakout(lookback: i64, direction: &str, confirmation: f64) -> Condition {
    condition(
        "PriceBreakout",
        &[
            ("lookback", json!(lookback)),
            ("direction", json!(direction)),
            ("confirmation", json!(confirmation)),
        ],
    )
}

fn volume_spike(lookback: i64, multiplier: f64) -> Condition {
    condition(
        "VolumeSpike",
        &[
            ("lookback", json!(lookback)),
            ("multiplier", json!(multiplier)),
        ],
    )
}

/// Every rule that reads candles, paired with a parameter set that is satisfiable on a
/// healthy series — so a verdict of `false` here really would mean "the market did not
/// do that", and anything else is a data-availability answer.
fn every_candle_rule() -> Vec<(&'static str, Box<dyn ConditionEvaluator>, Condition)> {
    vec![
        (
            "PriceChangePercent",
            Box::new(PriceChangePercentCondition),
            price_change(1.0, "ABOVE", 5.0),
        ),
        (
            "PriceToMA",
            Box::new(PriceToMaCondition),
            price_to_ma(5, "ABOVE", 1.0),
        ),
        (
            "ConsecutiveCandles",
            Box::new(ConsecutiveCandlesCondition),
            consecutive(3, "GREEN", 0.5),
        ),
        (
            "PriceBreakout",
            Box::new(PriceBreakoutCondition),
            breakout(5, "UPWARD", 1.0),
        ),
        (
            "VolumeSpike",
            Box::new(VolumeSpikeCondition),
            volume_spike(5, 2.0),
        ),
    ]
}

// ==================== THE CONTEXT THE TRADER ACTUALLY BUILDS ====================
//
// `src/trader/evaluators/strategies.rs` is the only place that turns live bot state into
// an `EvaluationContext`. The two helpers below mirror what it builds, field for field,
// so a change there that quietly starves a rule of its input fails here.

/// `check_entry_strategies`: liquidity comes from the pool price result's `sol_reserves`;
/// every other market field is `None`.
fn entry_market_data(liquidity_sol: f64) -> MarketData {
    MarketData {
        liquidity_sol: Some(liquidity_sol),
        volume_24h: None,
        market_cap: None,
        holder_count: None,
        token_age_hours: None,
    }
}

/// `check_exit_strategies`: liquidity is read from the live pool price result — the same
/// single source the entry path uses — and stays `None` only when no pool price exists
/// for the mint (an API-derived price carries no reserves). Every other field is `None`.
fn exit_market_data(liquidity_sol: Option<f64>) -> MarketData {
    MarketData {
        liquidity_sol,
        volume_24h: None,
        market_cap: None,
        holder_count: None,
        token_age_hours: None,
    }
}

fn exit_position_data(entry_price: f64, current_price: f64) -> PositionData {
    let unrealized_profit_pct = if entry_price > 0.0 {
        Some(((current_price - entry_price) / entry_price) * 100.0)
    } else {
        None
    };
    PositionData {
        entry_price,
        entry_time: chrono::Utc::now(),
        current_size_sol: 1.0,
        unrealized_profit_pct,
        position_age_hours: 1.0,
    }
}

fn entry_context(liquidity_sol: f64, price: f64) -> EvaluationContext {
    let mut ctx = context_bare(Some(price));
    ctx.market_data = Some(entry_market_data(liquidity_sol));
    ctx
}

fn exit_context(price: f64, liquidity_sol: Option<f64>) -> EvaluationContext {
    let mut ctx = context_bare(Some(price));
    ctx.market_data = Some(exit_market_data(liquidity_sol));
    ctx.position_data = Some(exit_position_data(1.0, price));
    ctx
}

#[tokio::test]
async fn an_entry_strategy_can_read_the_pool_liquidity_the_trader_passes_it() {
    // The one market field the entry path fills. A "only buy into a deep pool" rule is
    // answerable, which is what makes the exit-side result below a bug and not a design.
    let ctx = entry_context(120.0, 1.0);
    assert!(eval(&LiquidityLevelCondition, &liquidity_at_least(100.0), &ctx).await);
    assert!(!eval(&LiquidityLevelCondition, &liquidity_at_least(500.0), &ctx).await);
}

#[tokio::test]
async fn a_liquidity_drain_exit_fires_on_the_liquidity_the_exit_path_passes() {
    // `LiquidityLevel`'s schema advertises "Exit: detect liquidity drain". The exit path
    // used to build its `MarketData` with every field `None`, so the rule could not
    // answer: it errored, the error failed the whole strategy, and the user's
    // "liquidity dropped below 5 SOL -> sell" was silently dead for the life of every
    // position while the identical rule worked on entry.
    assert!(
        eval(
            &LiquidityLevelCondition,
            &liquidity_below(5.0),
            &exit_context(1.0, Some(2.0))
        )
        .await
    );
    assert!(
        !eval(
            &LiquidityLevelCondition,
            &liquidity_below(5.0),
            &exit_context(1.0, Some(120.0))
        )
        .await
    );
}

#[tokio::test]
async fn a_pool_with_no_price_reports_unknown_liquidity_rather_than_zero() {
    // The one case that still has no reading: no live pool price for the mint. Absence
    // must stay absence — passing the `0.0` an API-derived price carries would satisfy
    // every "liquidity below X" rule on a token nobody measured, and sell the position
    // on a drain that was never observed.
    let err = eval_err(
        &LiquidityLevelCondition,
        &liquidity_below(5.0),
        &exit_context(1.0, None),
    )
    .await;
    assert!(
        err.contains("Liquidity data not available"),
        "unexpected error: {err}"
    );
}

#[tokio::test]
async fn an_exit_context_still_carries_the_position_the_rules_need() {
    // The position half of the exit context IS populated, so the contrast above is a
    // gap in one field rather than a context that was never built.
    let ctx = exit_context(2.0, Some(50.0));
    let position = ctx
        .position_data
        .as_ref()
        .expect("exit context has position");
    assert_eq!(position.entry_price, 1.0);
    assert_eq!(position.unrealized_profit_pct, Some(100.0));

    // A broken (zero) average entry price is reported as "no percentage", never as 0%.
    assert_eq!(exit_position_data(0.0, 2.0).unrealized_profit_pct, None);
}

// ==================== OHLCV READINESS ====================

#[tokio::test]
async fn an_ohlcv_outage_is_an_error_in_every_candle_rule_not_a_missed_signal() {
    // What the trader does when the bundle could not be built at all: it evaluates
    // "without OHLCV" — i.e. with `timeframe_bundle: None`. Every candle rule must then
    // refuse to answer. If any of them returned `false` instead, an exit strategy would
    // report "the market did not move" during a data outage, which is a claim it cannot
    // make.
    let ctx = context_bare(Some(1.0));
    for (name, evaluator, cond) in every_candle_rule() {
        let err = eval_err(evaluator.as_ref(), &cond, &ctx).await;
        assert!(
            err.contains("OHLCV data not available"),
            "{name} answered `{err}` instead of reporting the missing bundle"
        );
    }
}

#[tokio::test]
async fn a_bundle_that_is_missing_the_strategy_timeframe_names_the_timeframe_it_lacks() {
    // The realistic partial-readiness case: the backfill has finished the coarse frames
    // and not the fine ones (that is the documented backfill order — 1d first, 1m last),
    // so a bundle exists but the 5m slot the strategy runs on is empty.
    let bundle = bundle_with("1d", candle_series(anchor_ts(), 86_400, &[100.0; 30]));
    let ctx = context_with_candles(100.0, "5m", bundle);

    for (name, evaluator, cond) in every_candle_rule() {
        let err = eval_err(evaluator.as_ref(), &cond, &ctx).await;
        assert!(
            err.contains("5m") && err.contains("no candle data"),
            "{name} answered `{err}` instead of naming the empty timeframe"
        );
    }
}

#[tokio::test]
async fn a_bundle_is_judged_on_its_candles_not_on_its_cache_age_or_completeness() {
    // `cache_age_seconds` and `is_complete()` describe the BUNDLE, not the market: a
    // bundle rebuilt this second can hold candles hours old, and a healthy new token has
    // no 12h or 1d series at all. Neither may gate a decision, so a week-old, one-of-
    // seven bundle whose candles are current still answers normally.
    let mut bundle = bundle_with("1m", candle_series(anchor_ts(), MINUTE, &[100.0; 20]));
    bundle.timestamp = chrono::Utc::now() - chrono::Duration::days(7);
    bundle.cache_age_seconds = 604_800;

    assert!(!bundle.is_complete(), "fixture must be a partial bundle");
    assert!(!bundle.is_fresh(60), "fixture must be stale by cache age");

    let ctx = context_with_candles(110.0, "1m", bundle);
    assert!(
        eval(
            &PriceChangePercentCondition,
            &price_change(10.0, "ABOVE", 5.0),
            &ctx
        )
        .await,
        "current candles answer regardless of how the bundle was cached"
    );
}

#[tokio::test]
async fn a_series_that_stopped_tracking_the_price_is_refused_not_measured() {
    // `PriceChangePercent` anchors "now" on the newest candle, but the price it compares
    // against is the LIVE pool price. On a token that has not traded for hours those two
    // describe different moments, and the rule used to answer anyway: "price gained 10%
    // in the last 5 minutes" was reported for a move that finished three hours ago.
    //
    // Candles are only written when a trade happens, so a quiet token keeps serving its
    // last series indefinitely — the only way to tell a live series from a dead one is
    // `evaluated_at`, which every context now carries.
    let three_hours_ago = anchor_ts() - 3 * HOUR;
    let stale = candle_series(three_hours_ago, MINUTE, &[100.0; 20]);
    let ctx = context_with_candles(150.0, "1m", bundle_with("1m", stale));

    for (name, evaluator, cond) in every_candle_rule() {
        let err = eval_err(evaluator.as_ref(), &cond, &ctx).await;
        assert!(
            err.contains("is stale") && err.contains("10800s old"),
            "{name} answered `{err}` instead of refusing a three-hour-old series"
        );
    }

    // The bound is generous enough for ordinary quiet: three buckets of silence still
    // answers, four does not.
    let quiet = candle_series(anchor_ts() - 3 * MINUTE, MINUTE, &[100.0; 20]);
    assert!(
        eval(
            &PriceChangePercentCondition,
            &price_change(10.0, "ABOVE", 5.0),
            &context_with_candles(150.0, "1m", bundle_with("1m", quiet)),
        )
        .await
    );

    let too_quiet = candle_series(anchor_ts() - 4 * MINUTE, MINUTE, &[100.0; 20]);
    assert!(eval_err(
        &PriceChangePercentCondition,
        &price_change(10.0, "ABOVE", 5.0),
        &context_with_candles(150.0, "1m", bundle_with("1m", too_quiet)),
    )
    .await
    .contains("is stale"));
}

#[tokio::test]
async fn history_that_covers_the_lookback_is_used_even_when_no_candle_lands_on_it() {
    // The lookback candle is the NEWEST one at or before the lookback instant. Choosing
    // the CLOSEST candle instead used to pick one INSIDE the window and then reject it
    // for being too recent — an "insufficient data" error that reported more seconds
    // available than the lookback had asked for.
    //
    // Five-minute candles, ten minutes of history, a seven-minute lookback: the instant
    // seven minutes back falls between the candle at -10m and the one at -5m.
    let series = candle_series(anchor_ts(), 5 * MINUTE, &[100.0, 100.0, 100.0]);
    let ctx = context_with_candles(150.0, "5m", bundle_with("5m", series));

    assert!(
        eval(
            &PriceChangePercentCondition,
            &price_change(10.0, "ABOVE", 7.0),
            &ctx
        )
        .await,
        "the -10m candle covers a seven-minute lookback"
    );
    // And the lookback that lands exactly on a candle still answers the same way.
    assert!(
        eval(
            &PriceChangePercentCondition,
            &price_change(10.0, "ABOVE", 5.0),
            &ctx
        )
        .await
    );
}

#[tokio::test]
async fn a_series_shorter_than_the_lookback_says_how_much_it_actually_holds() {
    // The genuine short-history case, and the number in the message has to be the truth:
    // it is what tells an operator whether to wait for backfill or shorten the rule.
    let series = candle_series(anchor_ts(), 5 * MINUTE, &[100.0, 100.0, 100.0]);
    let ctx = context_with_candles(150.0, "5m", bundle_with("5m", series));

    let err = eval_err(
        &PriceChangePercentCondition,
        &price_change(10.0, "ABOVE", 20.0),
        &ctx,
    )
    .await;
    assert!(
        err.contains("Insufficient historical data") && err.contains("600 seconds available"),
        "unexpected error: {err}"
    );
}

#[tokio::test]
async fn both_lookback_rules_require_the_same_history_for_the_same_lookback() {
    // Both rules mean the same thing by "lookback": N candles BEFORE the current one, so
    // both need `N + 1`. `PriceBreakout` used to accept exactly N and then compute its
    // period high from `N - 1` candles — a narrower window than the strategy configured,
    // which makes the breakout easier to clear than the user asked for.
    let five = candle_series(anchor_ts(), MINUTE, &[100.0; 5]);

    for (name, err) in [
        (
            "VolumeSpike",
            eval_err(
                &VolumeSpikeCondition,
                &volume_spike(5, 2.0),
                &context_with_candles(100.0, "1m", bundle_with("1m", five.clone())),
            )
            .await,
        ),
        (
            "PriceBreakout",
            eval_err(
                &PriceBreakoutCondition,
                &breakout(5, "UPWARD", 1.0),
                &context_with_candles(1_000.0, "1m", bundle_with("1m", five)),
            )
            .await,
        ),
    ] {
        assert!(
            err.contains("Not enough candles: 5 < 6"),
            "{name} answered `{err}` instead of asking for six candles"
        );
    }

    // With six candles the window is the full five, oldest candle included: a high of
    // 1000 there must still cap the breakout level. A window short by one would drop it
    // and let a price of 500 clear a period high of 100.
    let mut six = candle_series(anchor_ts(), MINUTE, &[100.0; 6]);
    six[0].high = 1_000.0;
    let ctx = context_with_candles(500.0, "1m", bundle_with("1m", six));
    assert!(
        !eval(&PriceBreakoutCondition, &breakout(5, "UPWARD", 0.0), &ctx).await,
        "the oldest candle in the lookback window still sets the level"
    );
}

// ==================== DEGENERATE NUMBERS ====================
//
// Every percentage rule divides by a price that came from somewhere else. These pin what
// survives that division when the divisor is not a real price.

/// A candle whose body sits entirely at `price` (a zero-range candle) with real volume.
fn flat_candle(timestamp: i64, price: f64) -> Candle {
    candle(timestamp, price, price, price, price, 1.0)
}

#[tokio::test]
async fn a_zero_reference_price_is_refused_instead_of_reading_as_an_infinite_gain() {
    // `(current - past) / past` with `past == 0.0` is `+inf`, and `inf` clears EVERY
    // "ABOVE" threshold — including the 1000% maximum the validator allows. A single
    // zero-close candle (a denomination slip, a pool that priced at zero for a minute)
    // was therefore a guaranteed buy signal for every gain rule on that token.
    let series: Vec<Candle> = (0..10)
        .map(|i| flat_candle(anchor_ts() - (9 - i) * MINUTE, 0.0))
        .collect();
    let ctx = context_with_candles(0.000_001, "1m", bundle_with("1m", series));

    for direction in ["ABOVE", "BELOW", "WITHIN"] {
        let err = eval_err(
            &PriceChangePercentCondition,
            &price_change(1000.0, direction, 5.0),
            &ctx,
        )
        .await;
        assert!(
            err.contains("not a usable basis"),
            "{direction} answered `{err}` instead of refusing the zero reference"
        );
    }
}

#[tokio::test]
async fn a_dead_series_cannot_place_the_price_above_its_moving_average() {
    // Same division, different rule: an all-zero moving average made `distance_pct`
    // infinite, so "price is at least 100% above its MA" fired on a token whose recorded
    // history is entirely zeros.
    let series: Vec<Candle> = (0..10)
        .map(|i| flat_candle(anchor_ts() - (9 - i) * MINUTE, 0.0))
        .collect();
    let ctx = context_with_candles(0.000_001, "1m", bundle_with("1m", series));

    let err = eval_err(&PriceToMaCondition, &price_to_ma(5, "ABOVE", 100.0), &ctx).await;
    assert!(
        err.contains("not a usable basis"),
        "unexpected error: {err}"
    );
}

#[tokio::test]
async fn a_zero_open_candle_cannot_be_coloured_so_it_breaks_the_streak() {
    // `ConsecutiveCandles` measures each body as `(close - open) / open`. With `open` at
    // zero that is `+inf`, which passed any `minimum_change` filter — so the "filters
    // noise" parameter was inert exactly on the candles most likely to be junk. An
    // unclassifiable candle now breaks the streak instead of extending it.
    let series: Vec<Candle> = (0..5)
        .map(|i| candle(anchor_ts() - (4 - i) * MINUTE, 0.0, 1.0, 0.0, 1.0, 1.0))
        .collect();
    let ctx = context_with_candles(1.0, "1m", bundle_with("1m", series));

    assert!(
        !eval(
            &ConsecutiveCandlesCondition,
            &consecutive(3, "GREEN", 50.0),
            &ctx
        )
        .await
    );

    // The control: real candles of the same shape still make a streak.
    let real: Vec<Candle> = (0..5)
        .map(|i| candle(anchor_ts() - (4 - i) * MINUTE, 1.0, 2.0, 1.0, 2.0, 1.0))
        .collect();
    assert!(
        eval(
            &ConsecutiveCandlesCondition,
            &consecutive(3, "GREEN", 50.0),
            &context_with_candles(2.0, "1m", bundle_with("1m", real))
        )
        .await
    );
}

#[tokio::test]
async fn a_not_a_number_price_is_refused_by_every_price_rule() {
    // The other half of the degenerate-input story, and the one that is dangerous on the
    // SELL side: `NaN` loses every comparison, so a rule fed a `NaN` price answered
    // `false` — "no signal" — for gains, losses AND for a "within range" band that a real
    // number could not miss. An exit strategy simply stopped working, silently.
    //
    // `resolve_evaluation_input` (src/trader/evaluators/exit.rs) keeps `NaN` out of the
    // exit path by rejecting any price that is not finite and positive; this is the same
    // guard one layer in, where a rule can no longer answer a question it cannot compute.
    let ctx = context_with_candles(
        f64::NAN,
        "1m",
        bundle_with("1m", candle_series(anchor_ts(), MINUTE, &[100.0; 20])),
    );

    for direction in ["ABOVE", "BELOW", "WITHIN"] {
        let err = eval_err(
            &PriceChangePercentCondition,
            &price_change(1.0, direction, 5.0),
            &ctx,
        )
        .await;
        assert!(
            err.contains("Current price is not usable"),
            "a NaN price answered `{err}` for {direction}"
        );
    }
    assert!(
        eval_err(&PriceToMaCondition, &price_to_ma(5, "WITHIN", 100.0), &ctx)
            .await
            .contains("Current price is not usable")
    );
    assert!(
        eval_err(&PriceBreakoutCondition, &breakout(5, "DOWNWARD", 0.0), &ctx)
            .await
            .contains("Current price is not usable")
    );
}

// ==================== WHOLE-STRATEGY BEHAVIOUR ====================

fn candles_1m(closes: &[f64]) -> TimeframeBundle {
    bundle_with("1m", candle_series(anchor_ts(), MINUTE, closes))
}

#[tokio::test]
async fn a_strategy_with_no_candle_rule_still_trades_through_a_total_ohlcv_outage() {
    // Concretely: what the bot keeps doing when OHLCV is down. A strategy built only
    // from market-context rules is unaffected, because nothing in the entry path makes
    // the bundle a precondition for evaluating.
    let engine = uncached_engine();
    let s = strategy(
        StrategyType::Entry,
        "5m",
        RuleTree::leaf(liquidity_at_least(100.0)),
    );

    let result = engine
        .evaluate_strategy(&s, &entry_context(150.0, 1.0))
        .await
        .expect("a market-only strategy needs no candles");
    assert!(result.result, "the bot would buy here with no OHLCV at all");
}

#[tokio::test]
async fn one_unavailable_input_fails_the_whole_strategy_not_just_its_own_leaf() {
    // A missing bundle does not evaluate to `false` inside an AND — it aborts the tree.
    // The trader turns that `Err` into `Ok(None)`, so the strategy neither fires nor
    // reports a market opinion: it is inert until the data comes back.
    let engine = uncached_engine();
    let s = strategy(
        StrategyType::Exit,
        "1m",
        RuleTree::branch(
            LogicalOperator::And,
            vec![
                RuleTree::leaf(price_to_ma(5, "BELOW", 1.0)),
                RuleTree::leaf(liquidity_at_least(1.0)),
            ],
        ),
    );

    let err = engine
        .evaluate_strategy(&s, &exit_context(1.0, Some(50.0)))
        .await
        .expect_err("a tree containing an unanswerable rule must not return a verdict");
    assert!(
        err.contains("OHLCV data not available"),
        "unexpected error: {err}"
    );
}

#[tokio::test]
async fn whether_a_data_outage_is_even_visible_depends_on_the_order_of_the_conditions() {
    // AND short-circuits on the first `false`, so an outage in a LATER condition is
    // never reached. The same two rules, same missing data, produce either "no signal"
    // or "unanswerable" depending purely on the order the user happened to add them in.
    //
    // This is why an outage cannot be detected by watching for strategy errors alone —
    // `check_entry_strategies`/`check_exit_strategies` log the missing bundle at warning
    // level at the boundary instead, where rule order cannot hide it.
    let engine = uncached_engine();
    let ctx = entry_context(10.0, 1.0); // liquidity 10 SOL, no bundle

    let liquidity_first = strategy(
        StrategyType::Entry,
        "1m",
        RuleTree::branch(
            LogicalOperator::And,
            vec![
                RuleTree::leaf(liquidity_at_least(1_000.0)), // false: 10 < 1000
                RuleTree::leaf(price_to_ma(5, "ABOVE", 1.0)), // would error
            ],
        ),
    );
    let verdict = engine
        .evaluate_strategy(&liquidity_first, &ctx)
        .await
        .expect("short-circuited before the missing data");
    assert!(!verdict.result, "reported as a plain `no signal`");

    let candles_first = strategy(
        StrategyType::Entry,
        "1m",
        RuleTree::branch(
            LogicalOperator::And,
            vec![
                RuleTree::leaf(price_to_ma(5, "ABOVE", 1.0)),
                RuleTree::leaf(liquidity_at_least(1_000.0)),
            ],
        ),
    );
    assert!(
        engine
            .evaluate_strategy(&candles_first, &ctx)
            .await
            .is_err(),
        "the identical strategy, reordered, reports the outage"
    );
}

#[tokio::test]
async fn a_strategy_reads_the_timeframe_it_was_saved_with_not_whatever_is_populated() {
    // The bundle carries all seven timeframes; the strategy's own `timeframe` field
    // selects one. A strategy saved on 5m must not silently fall back to the 1m data
    // that happens to be there — the two answer different questions.
    let engine = uncached_engine();
    let rules = RuleTree::leaf(price_change(10.0, "ABOVE", 5.0));
    let mut ctx = context_with_candles(110.0, "1m", candles_1m(&[100.0; 20]));

    ctx.strategy_timeframe = "1m".to_owned();
    assert!(
        engine
            .evaluate_strategy(&strategy(StrategyType::Entry, "1m", rules.clone()), &ctx)
            .await
            .expect("1m is populated")
            .result
    );

    ctx.strategy_timeframe = "5m".to_owned();
    let err = engine
        .evaluate_strategy(&strategy(StrategyType::Entry, "5m", rules), &ctx)
        .await
        .expect_err("5m is empty in this bundle");
    assert!(err.contains("5m"), "unexpected error: {err}");
}

// ==================== THE SECURITY-DATA GATE ====================
//
// Before any strategy runs, `check_entry_admission` requires rpc + dexscreener +
// rugcheck to be healthy — rugcheck being the security/rug analysis endpoint. These
// tests drive the real global connectivity state, so they hold `config_guard()` both to
// initialise config (the loss-limit check reads it) and to serialise with each other.

use screenerbot::connectivity::state as connectivity_state;
use screenerbot::connectivity::types::EndpointCriticality;
use screenerbot::trader::admission::{check_entry_admission, EntryBlock};

const ENTRY_ENDPOINTS: [&str; 3] = ["rpc", "dexscreener", "rugcheck"];

async fn mark_healthy(name: &'static str) {
    connectivity_state::ensure_endpoint_registered(name, EndpointCriticality::Important, None)
        .await;
    // recovery_threshold = 1: one good check is enough to report healthy.
    connectivity_state::update_health(name, true, 1, None, 3, 1).await;
}

async fn mark_unhealthy(name: &'static str) {
    connectivity_state::ensure_endpoint_registered(name, EndpointCriticality::Important, None)
        .await;
    // failure_threshold = 1: one bad check is enough to report unhealthy.
    connectivity_state::update_health(name, false, 1, Some("test".to_owned()), 1, 1).await;
}

#[tokio::test]
async fn an_unknown_endpoint_is_not_healthy_but_is_also_not_confirmed_down() {
    // Two different questions, and the entry gate must ask the second one. `is_healthy`
    // is `unwrap_or_default()` over the health map, so a name no monitor ever registered
    // is false forever — never probed is not the same as measured and failing.
    assert!(!connectivity_state::is_endpoint_healthy("no_such_endpoint").await);
    assert_eq!(
        screenerbot::connectivity::check_endpoints_healthy(&["no_such_endpoint"]).await,
        Some("no_such_endpoint".to_owned())
    );
    assert_eq!(
        screenerbot::connectivity::check_endpoints_usable(&["no_such_endpoint"]).await,
        None,
        "an endpoint with no reading is not evidence of an outage"
    );
}

#[tokio::test]
async fn a_disabled_security_monitor_does_not_block_automated_entry() {
    // `ConnectivityChecker::register_monitors` registers a monitor only when
    // `is_enabled()`, and `RugcheckMonitor::is_enabled()` is
    // `connectivity.enabled && connectivity.endpoints.rugcheck.enabled`. Turning either
    // off leaves "rugcheck" unregistered and permanently `Unknown` — which used to block
    // EVERY strategy entry through this gate, for an endpoint declared `Optional` with a
    // `Skip` fallback, saying so only at info level, per token. The bot stopped buying
    // and nothing named the cause.
    let _cfg = common::config_guard();
    // Global connectivity state outlives this test and is shared with the other
    // tests in this file that drive the same endpoint names ("rpc",
    // "dexscreener", "rugcheck") — reset it under the same mutex so
    // "rugcheck deliberately left unregistered" below is true regardless of
    // which order the config_guard-serialized tests actually ran in.
    connectivity_state::reset_all_for_tests().await;
    screenerbot::global::set_force_stopped(false, None);

    mark_healthy("rpc").await;
    mark_healthy("dexscreener").await;
    // "rugcheck" deliberately left unregistered, exactly as a disabled monitor leaves it.

    let outcome = check_entry_admission(TEST_MINT, &ENTRY_ENDPOINTS).await;
    assert!(
        !matches!(outcome, Err(EntryBlock::Connectivity(_))),
        "an unprobed optional endpoint must not gate entry: {outcome:?}"
    );
}

#[tokio::test]
async fn a_failing_security_endpoint_blocks_entries_and_names_itself() {
    // The other half: an endpoint that WAS probed and failed is real evidence, and it
    // still stops entries — the relaxation above is about absence of data, not about
    // trading through a known outage.
    let _cfg = common::config_guard();
    connectivity_state::reset_all_for_tests().await;
    screenerbot::global::set_force_stopped(false, None);

    mark_healthy("rpc").await;
    mark_unhealthy("dexscreener").await;
    mark_unhealthy("rugcheck").await;

    let block = check_entry_admission(TEST_MINT, &ENTRY_ENDPOINTS)
        .await
        .expect_err("entry must be blocked");
    assert_eq!(
        block,
        EntryBlock::Connectivity("dexscreener, rugcheck".to_owned()),
        "every unhealthy endpoint is named, in the order asked for"
    );
}

#[tokio::test]
async fn force_stop_is_answered_before_any_data_source_is_consulted() {
    // The emergency halt must not depend on health state, config reads or the database
    // being reachable — it is the one answer that has to work when everything else is
    // broken.
    let _cfg = common::config_guard();
    connectivity_state::reset_all_for_tests().await;
    mark_unhealthy("rpc").await;
    screenerbot::global::set_force_stopped(true, Some("test"));

    let block = check_entry_admission(TEST_MINT, &ENTRY_ENDPOINTS).await;
    screenerbot::global::set_force_stopped(false, None);

    assert_eq!(block, Err(EntryBlock::ForceStopped));
}

#[test]
fn every_entry_block_serialises_with_a_stable_machine_readable_kind() {
    // `EntryBlock` is persisted by copy trading and rendered by the dashboard, so the
    // tag values are a contract, not an internal detail.
    let cases = [
        (EntryBlock::ForceStopped, json!({"kind": "force_stopped"})),
        (EntryBlock::LossLimit, json!({"kind": "loss_limit"})),
        (
            EntryBlock::Connectivity("rugcheck".to_owned()),
            json!({"kind": "connectivity", "detail": "rugcheck"}),
        ),
        (EntryBlock::PositionLimit, json!({"kind": "position_limit"})),
        (EntryBlock::AlreadyOpen, json!({"kind": "already_open"})),
        (
            EntryBlock::ReentryCooldown,
            json!({"kind": "reentry_cooldown"}),
        ),
        (
            EntryBlock::OpenCooldown { wait_secs: 12 },
            json!({"kind": "open_cooldown", "detail": {"wait_secs": 12}}),
        ),
        (EntryBlock::EntryReserved, json!({"kind": "entry_reserved"})),
        (EntryBlock::Blacklisted, json!({"kind": "blacklisted"})),
        (
            EntryBlock::CheckFailed("db down".to_owned()),
            json!({"kind": "check_failed", "detail": "db down"}),
        ),
    ];

    for (block, expected) in cases {
        assert_eq!(
            serde_json::to_value(&block).expect("EntryBlock serialises"),
            expected
        );
    }
}
