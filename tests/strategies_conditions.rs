//! Pure: the eight strategy condition evaluators — the leaves every user strategy is
//! built from. No network, no DB.
//!
//! A condition is what actually decides to spend SOL, so the properties that matter are
//! the ones that are silent when wrong: which side of a threshold `>=` falls on, whether
//! a direction is measured with the right sign, and whether missing data errors out
//! instead of quietly evaluating to `false` (a `false` reads as "no signal" and is
//! indistinguishable from a working condition that simply did not fire).

mod common;

use common::{
    anchor_ts, bundle_with, candle, candle_series, condition, context_bare, context_with_candles,
};
use screenerbot::strategies::conditions::{
    CandleSizeCondition, ConditionEvaluator, ConsecutiveCandlesCondition, LiquidityLevelCondition,
    PositionHoldingTimeCondition, PriceBreakoutCondition, PriceChangePercentCondition,
    PriceToMaCondition, VolumeSpikeCondition,
};
use screenerbot::strategies::types::{Condition, EvaluationContext, MarketData, PositionData};
use screenerbot::strategies::Error as StrategyError;
use serde_json::json;

const MINUTE: i64 = 60;

/// Evaluate a condition, asserting it did not error.
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

/// Evaluate a condition, asserting it DID error, and return the typed error.
async fn eval_err(
    evaluator: &dyn ConditionEvaluator,
    cond: &Condition,
    ctx: &EvaluationContext,
) -> StrategyError {
    evaluator
        .evaluate(cond, ctx)
        .await
        .expect_err("expected an error")
}

fn market_context(liquidity_sol: Option<f64>) -> EvaluationContext {
    let mut ctx = context_bare(Some(1.0));
    ctx.market_data = Some(MarketData {
        liquidity_sol,
        volume_24h: None,
        market_cap: None,
        holder_count: None,
        token_age_hours: None,
    });
    ctx
}

fn position_context(age_hours: f64) -> EvaluationContext {
    let mut ctx = context_bare(Some(1.0));
    ctx.position_data = Some(PositionData {
        entry_price: 1.0,
        entry_time: chrono::Utc::now(),
        current_size_sol: 1.0,
        unrealized_profit_pct: None,
        position_age_hours: age_hours,
    });
    ctx
}

// ==================== SHARED CANDLE PLUMBING ====================
//
// Every OHLCV condition reaches its candles through the same two helpers, so the
// data-availability contract is tested once here rather than eight times.

#[tokio::test]
async fn missing_bundle_is_an_error_not_a_false_signal() {
    // No OHLCV at all must be distinguishable from "the condition did not fire".
    let cond = condition(
        "PriceToMA",
        &[
            ("period", json!(5)),
            ("position", json!("ABOVE")),
            ("distance", json!(1.0)),
        ],
    );
    let err = eval_err(&PriceToMaCondition, &cond, &context_bare(Some(1.0))).await;
    assert!(
        matches!(
            err,
            StrategyError::MissingContextData { data: "OHLCV data" }
        ),
        "unexpected error: {err}"
    );
}

#[tokio::test]
async fn empty_timeframe_is_an_error() {
    let cond = condition(
        "PriceToMA",
        &[
            ("period", json!(5)),
            ("position", json!("ABOVE")),
            ("distance", json!(1.0)),
        ],
    );
    let ctx = context_with_candles(1.0, "5m", bundle_with("1m", vec![]));
    let err = eval_err(&PriceToMaCondition, &cond, &ctx).await;
    assert!(
        matches!(err, StrategyError::NoCandleData { .. }),
        "unexpected error: {err}"
    );
}

#[tokio::test]
async fn per_condition_timeframe_overrides_the_strategy_timeframe() {
    // Strategy runs on 5m; the condition asks for 1h. It must read the 1h series — a
    // silent fallback to 5m would evaluate a completely different market.
    let cond = condition(
        "PriceToMA",
        &[
            ("timeframe", json!("1h")),
            ("period", json!(2)),
            ("position", json!("ABOVE")),
            ("distance", json!(0.0)),
        ],
    );
    let mut bundle = bundle_with(
        "5m",
        candle_series(anchor_ts(), MINUTE * 5, &[100.0, 100.0]),
    );
    bundle.h1 = candle_series(anchor_ts(), MINUTE * 60, &[10.0, 10.0]);

    // Price 11 is ABOVE the 1h MA (10) but BELOW the 5m MA (100).
    let ctx = context_with_candles(11.0, "5m", bundle);
    assert!(eval(&PriceToMaCondition, &cond, &ctx).await);
}

#[tokio::test]
async fn invalid_timeframe_string_is_rejected() {
    let cond = condition(
        "PriceToMA",
        &[
            ("timeframe", json!("3m")),
            ("period", json!(2)),
            ("position", json!("ABOVE")),
            ("distance", json!(0.0)),
        ],
    );
    assert!(PriceToMaCondition.validate(&cond).is_err());
}

// ==================== PriceChangePercent ====================

fn pcp(percentage: f64, direction: &str, minutes: f64) -> Condition {
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

/// 20 one-minute candles all closing at 100, ending at the anchor.
fn flat_1m_series() -> Vec<screenerbot::ohlcvs::Candle> {
    candle_series(anchor_ts(), MINUTE, &[100.0; 20])
}

#[tokio::test]
async fn price_change_above_fires_exactly_at_the_threshold() {
    let ctx = context_with_candles(110.0, "1m", bundle_with("1m", flat_1m_series()));
    // +10.0% against a 10% threshold: `>=` must include the boundary.
    assert!(eval(&PriceChangePercentCondition, &pcp(10.0, "ABOVE", 5.0), &ctx).await);
    // A hair above the threshold must not fire.
    assert!(
        !eval(
            &PriceChangePercentCondition,
            &pcp(10.001, "ABOVE", 5.0),
            &ctx
        )
        .await
    );
}

#[tokio::test]
async fn price_change_below_measures_a_drop_not_a_signed_comparison() {
    // BELOW compares against -percentage: a 10% DROP satisfies a 10% BELOW threshold,
    // and a RISE never can, whatever its size.
    let dropped = context_with_candles(90.0, "1m", bundle_with("1m", flat_1m_series()));
    assert!(
        eval(
            &PriceChangePercentCondition,
            &pcp(10.0, "BELOW", 5.0),
            &dropped
        )
        .await
    );

    let risen = context_with_candles(300.0, "1m", bundle_with("1m", flat_1m_series()));
    assert!(
        !eval(
            &PriceChangePercentCondition,
            &pcp(10.0, "BELOW", 5.0),
            &risen
        )
        .await
    );
}

#[tokio::test]
async fn price_change_within_is_symmetric() {
    for price in [95.0, 105.0] {
        let ctx = context_with_candles(price, "1m", bundle_with("1m", flat_1m_series()));
        assert!(
            eval(&PriceChangePercentCondition, &pcp(5.0, "WITHIN", 5.0), &ctx).await,
            "±5% must be inside a 5% WITHIN band (price {price})"
        );
    }
    let outside = context_with_candles(120.0, "1m", bundle_with("1m", flat_1m_series()));
    assert!(
        !eval(
            &PriceChangePercentCondition,
            &pcp(5.0, "WITHIN", 5.0),
            &outside
        )
        .await
    );
}

#[tokio::test]
async fn price_change_errors_when_history_is_shorter_than_the_lookback() {
    // Three minutes of history cannot answer a 60-minute question. Answering `false`
    // would read as "price did not move", which is a different claim entirely.
    let ctx = context_with_candles(
        110.0,
        "1m",
        bundle_with("1m", candle_series(anchor_ts(), MINUTE, &[100.0; 3])),
    );
    let err = eval_err(
        &PriceChangePercentCondition,
        &pcp(10.0, "ABOVE", 60.0),
        &ctx,
    )
    .await;
    assert!(
        matches!(
            err,
            StrategyError::InsufficientHistory {
                indicator: "price change lookback",
                ..
            }
        ),
        "unexpected error: {err}"
    );
}

#[tokio::test]
async fn price_change_reads_the_candle_at_the_lookback_distance() {
    // Closes step up by 1 each minute, ending at 120. A 5-minute lookback must price
    // against the candle 5 steps back (115), not the first or last one.
    let closes: Vec<f64> = (101..=120).map(f64::from).collect();
    let ctx = context_with_candles(
        115.0,
        "1m",
        bundle_with("1m", candle_series(anchor_ts(), MINUTE, &closes)),
    );
    // 115 vs 115 = 0% change: a 0.1% ABOVE threshold must NOT fire.
    assert!(!eval(&PriceChangePercentCondition, &pcp(0.1, "ABOVE", 5.0), &ctx).await);
    // Against the 20-minute-old candle (101) the same price is a ~13.9% gain.
    assert!(
        eval(
            &PriceChangePercentCondition,
            &pcp(13.0, "ABOVE", 19.0),
            &ctx
        )
        .await
    );
}

#[tokio::test]
async fn price_change_validation_bounds() {
    assert!(PriceChangePercentCondition
        .validate(&pcp(0.05, "ABOVE", 5.0))
        .is_err()); // below the 0.1 floor
    assert!(PriceChangePercentCondition
        .validate(&pcp(1001.0, "ABOVE", 5.0))
        .is_err()); // above the 1000 ceiling
    assert!(PriceChangePercentCondition
        .validate(&pcp(10.0, "SIDEWAYS", 5.0))
        .is_err()); // unknown direction
    assert!(PriceChangePercentCondition
        .validate(&pcp(10.0, "ABOVE", 1441.0))
        .is_err()); // minutes cap is 1440
    assert!(PriceChangePercentCondition
        .validate(&pcp(10.0, "ABOVE", 5.0))
        .is_ok());
}

#[tokio::test]
async fn price_change_requires_a_current_price() {
    let mut ctx = context_with_candles(1.0, "1m", bundle_with("1m", flat_1m_series()));
    ctx.current_price = None;
    let err = eval_err(&PriceChangePercentCondition, &pcp(10.0, "ABOVE", 5.0), &ctx).await;
    assert!(
        matches!(
            err,
            StrategyError::MissingContextData {
                data: "current price"
            }
        ),
        "unexpected error: {err}"
    );
}

// ==================== PriceToMA ====================

fn ma(period: i64, position: &str, distance: f64) -> Condition {
    condition(
        "PriceToMA",
        &[
            ("period", json!(period)),
            ("position", json!(position)),
            ("distance", json!(distance)),
        ],
    )
}

#[tokio::test]
async fn ma_uses_only_the_last_period_closes() {
    // Ancient 1.0 closes must not drag the average: a 4-period MA over
    // [1,1,1,1,10,10,10,10] is 10, not 5.5.
    let closes = [1.0, 1.0, 1.0, 1.0, 10.0, 10.0, 10.0, 10.0];
    let ctx = context_with_candles(
        10.0,
        "5m",
        bundle_with("5m", candle_series(anchor_ts(), MINUTE * 5, &closes)),
    );
    // Price 10 is exactly ON the MA -> 0% distance. WITHIN 0 must hold...
    assert!(eval(&PriceToMaCondition, &ma(4, "WITHIN", 0.0), &ctx).await);
    // ...and it is NOT 2% above it.
    assert!(!eval(&PriceToMaCondition, &ma(4, "ABOVE", 2.0), &ctx).await);
    // Against the full 8-candle MA (5.5) the same price IS far above.
    assert!(eval(&PriceToMaCondition, &ma(8, "ABOVE", 50.0), &ctx).await);
}

#[tokio::test]
async fn ma_distance_boundary_is_inclusive() {
    let ctx = context_with_candles(
        102.0,
        "5m",
        bundle_with("5m", candle_series(anchor_ts(), MINUTE * 5, &[100.0; 5])),
    );
    // Exactly 2% above a flat 100 MA.
    assert!(eval(&PriceToMaCondition, &ma(5, "ABOVE", 2.0), &ctx).await);
    assert!(!eval(&PriceToMaCondition, &ma(5, "ABOVE", 2.001), &ctx).await);
}

#[tokio::test]
async fn ma_below_needs_the_price_under_the_average() {
    let ctx = context_with_candles(
        95.0,
        "5m",
        bundle_with("5m", candle_series(anchor_ts(), MINUTE * 5, &[100.0; 5])),
    );
    assert!(eval(&PriceToMaCondition, &ma(5, "BELOW", 5.0), &ctx).await);
    assert!(!eval(&PriceToMaCondition, &ma(5, "ABOVE", 0.0), &ctx).await);
}

#[tokio::test]
async fn ma_errors_when_there_are_fewer_candles_than_the_period() {
    let ctx = context_with_candles(
        100.0,
        "5m",
        bundle_with("5m", candle_series(anchor_ts(), MINUTE * 5, &[100.0; 3])),
    );
    let err = eval_err(&PriceToMaCondition, &ma(20, "ABOVE", 1.0), &ctx).await;
    assert!(
        matches!(
            err,
            StrategyError::InsufficientCandles {
                indicator: "moving average",
                ..
            }
        ),
        "unexpected error: {err}"
    );
}

#[tokio::test]
async fn ma_validation_bounds() {
    assert!(PriceToMaCondition.validate(&ma(1, "ABOVE", 1.0)).is_err()); // period floor is 2
    assert!(PriceToMaCondition.validate(&ma(201, "ABOVE", 1.0)).is_err()); // ceiling is 200
    assert!(PriceToMaCondition.validate(&ma(20, "ABOVE", -1.0)).is_err()); // negative distance
    assert!(PriceToMaCondition.validate(&ma(20, "SIDE", 1.0)).is_err());
    assert!(PriceToMaCondition.validate(&ma(20, "ABOVE", 2.0)).is_ok());
}

// ==================== ConsecutiveCandles ====================

fn streak(count: i64, direction: &str, minimum_change: f64) -> Condition {
    condition(
        "ConsecutiveCandles",
        &[
            ("count", json!(count)),
            ("direction", json!(direction)),
            ("minimum_change", json!(minimum_change)),
        ],
    )
}

/// Candles with explicit open/close pairs so the body sign is unambiguous.
fn bodies(pairs: &[(f64, f64)]) -> Vec<screenerbot::ohlcvs::Candle> {
    pairs
        .iter()
        .enumerate()
        .map(|(i, &(open, close))| {
            let ts = anchor_ts() - ((pairs.len() - 1 - i) as i64) * MINUTE;
            candle(ts, open, open.max(close), open.min(close), close, 1.0)
        })
        .collect()
}

#[tokio::test]
async fn consecutive_green_requires_every_candle_in_the_window() {
    let all_green = bodies(&[(100.0, 102.0), (102.0, 104.0), (104.0, 106.0)]);
    let ctx = context_with_candles(106.0, "1m", bundle_with("1m", all_green));
    assert!(eval(&ConsecutiveCandlesCondition, &streak(3, "GREEN", 0.5), &ctx).await);

    // One red candle in the middle breaks the streak — the run must reset, not merely
    // fall one short.
    let broken = bodies(&[(100.0, 102.0), (102.0, 99.0), (99.0, 101.0)]);
    let ctx = context_with_candles(101.0, "1m", bundle_with("1m", broken));
    assert!(!eval(&ConsecutiveCandlesCondition, &streak(3, "GREEN", 0.5), &ctx).await);
}

#[tokio::test]
async fn consecutive_red_uses_the_negative_side_of_minimum_change() {
    let all_red = bodies(&[(100.0, 98.0), (98.0, 96.0), (96.0, 94.0)]);
    let ctx = context_with_candles(94.0, "1m", bundle_with("1m", all_red));
    assert!(eval(&ConsecutiveCandlesCondition, &streak(3, "RED", 1.0), &ctx).await);
    // Green candles can never satisfy RED.
    let all_green = bodies(&[(100.0, 102.0), (102.0, 104.0), (104.0, 106.0)]);
    let ctx = context_with_candles(106.0, "1m", bundle_with("1m", all_green));
    assert!(!eval(&ConsecutiveCandlesCondition, &streak(3, "RED", 1.0), &ctx).await);
}

#[tokio::test]
async fn consecutive_minimum_change_filters_out_flat_candles() {
    // Bodies of +0.5% each: a 2% minimum must reject them even though all are green.
    let tiny = bodies(&[(100.0, 100.5), (100.5, 101.0), (101.0, 101.5)]);
    let ctx = context_with_candles(101.5, "1m", bundle_with("1m", tiny));
    assert!(!eval(&ConsecutiveCandlesCondition, &streak(3, "GREEN", 2.0), &ctx).await);
    assert!(eval(&ConsecutiveCandlesCondition, &streak(3, "GREEN", 0.4), &ctx).await);
}

#[tokio::test]
async fn consecutive_only_inspects_the_most_recent_window() {
    // Older red candles are irrelevant when the last 2 are green.
    let mixed = bodies(&[(100.0, 90.0), (90.0, 80.0), (80.0, 85.0), (85.0, 90.0)]);
    let ctx = context_with_candles(90.0, "1m", bundle_with("1m", mixed));
    assert!(eval(&ConsecutiveCandlesCondition, &streak(2, "GREEN", 1.0), &ctx).await);
}

#[tokio::test]
async fn consecutive_validation_bounds() {
    assert!(ConsecutiveCandlesCondition
        .validate(&streak(1, "GREEN", 1.0))
        .is_err());
    assert!(ConsecutiveCandlesCondition
        .validate(&streak(21, "GREEN", 1.0))
        .is_err());
    assert!(ConsecutiveCandlesCondition
        .validate(&streak(3, "BLUE", 1.0))
        .is_err());
    assert!(ConsecutiveCandlesCondition
        .validate(&streak(3, "GREEN", 1.0))
        .is_ok());
}

// ==================== CandleSize ====================

fn shape(pattern: &str, threshold: f64) -> Condition {
    condition(
        "CandleSize",
        &[("pattern", json!(pattern)), ("threshold", json!(threshold))],
    )
}

#[tokio::test]
async fn candle_size_large_body_needs_both_body_share_and_price_move() {
    // Body is 100% of the range AND a 10% move.
    let big = vec![candle(anchor_ts(), 100.0, 110.0, 100.0, 110.0, 1.0)];
    let ctx = context_with_candles(110.0, "1m", bundle_with("1m", big));
    assert!(eval(&CandleSizeCondition, &shape("LARGE_BODY", 10.0), &ctx).await);

    // Body dominates the range (100%) but the move is only 1% — LARGE_BODY requires
    // BOTH, so a 10% threshold must reject it.
    let dominant_but_small = vec![candle(anchor_ts(), 100.0, 101.0, 100.0, 101.0, 1.0)];
    let ctx = context_with_candles(101.0, "1m", bundle_with("1m", dominant_but_small));
    assert!(!eval(&CandleSizeCondition, &shape("LARGE_BODY", 10.0), &ctx).await);
}

#[tokio::test]
async fn candle_size_small_body_detects_a_doji() {
    // Open == close with long wicks: body share is 0%.
    let doji = vec![candle(anchor_ts(), 100.0, 110.0, 90.0, 100.0, 1.0)];
    let ctx = context_with_candles(100.0, "1m", bundle_with("1m", doji));
    assert!(eval(&CandleSizeCondition, &shape("SMALL_BODY", 10.0), &ctx).await);
}

#[tokio::test]
async fn candle_size_wicks_are_measured_on_the_correct_side() {
    // Range 100..120, body 100..105: upper wick 15/20 = 75%, lower wick 0%.
    let upper = vec![candle(anchor_ts(), 100.0, 120.0, 100.0, 105.0, 1.0)];
    let ctx = context_with_candles(105.0, "1m", bundle_with("1m", upper));
    assert!(eval(&CandleSizeCondition, &shape("LONG_UPPER_WICK", 70.0), &ctx).await);
    assert!(!eval(&CandleSizeCondition, &shape("LONG_LOWER_WICK", 70.0), &ctx).await);

    // Mirror image: range 80..100, body 95..100 -> lower wick 15/20 = 75%.
    let lower = vec![candle(anchor_ts(), 100.0, 100.0, 80.0, 95.0, 1.0)];
    let ctx = context_with_candles(95.0, "1m", bundle_with("1m", lower));
    assert!(eval(&CandleSizeCondition, &shape("LONG_LOWER_WICK", 70.0), &ctx).await);
    assert!(!eval(&CandleSizeCondition, &shape("LONG_UPPER_WICK", 70.0), &ctx).await);
}

#[tokio::test]
async fn candle_size_zero_range_candle_does_not_produce_nan() {
    // A candle with no range at all (open == high == low == close) divides by zero if
    // unguarded. It must answer a plain bool, never NaN-propagate into the rule tree.
    let flat = vec![candle(anchor_ts(), 100.0, 100.0, 100.0, 100.0, 1.0)];
    let ctx = context_with_candles(100.0, "1m", bundle_with("1m", flat));
    assert!(!eval(&CandleSizeCondition, &shape("LARGE_BODY", 10.0), &ctx).await);
    assert!(!eval(&CandleSizeCondition, &shape("LONG_UPPER_WICK", 10.0), &ctx).await);
    assert!(!eval(&CandleSizeCondition, &shape("LONG_LOWER_WICK", 10.0), &ctx).await);
    // Zero body share still satisfies SMALL_BODY, which is the honest answer.
    assert!(eval(&CandleSizeCondition, &shape("SMALL_BODY", 10.0), &ctx).await);
}

#[tokio::test]
async fn candle_size_unknown_pattern_errors() {
    let big = vec![candle(anchor_ts(), 100.0, 110.0, 100.0, 110.0, 1.0)];
    let ctx = context_with_candles(110.0, "1m", bundle_with("1m", big));
    let err = eval_err(&CandleSizeCondition, &shape("HAMMER", 10.0), &ctx).await;
    assert!(
        matches!(
            err,
            StrategyError::InvalidConditionValue {
                field: "pattern",
                ref value
            } if value == "HAMMER"
        ),
        "unexpected error: {err}"
    );
}

// ==================== PriceBreakout ====================

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

/// Five candles ranging 90..110, plus a final candle the evaluator excludes.
fn breakout_series() -> Vec<screenerbot::ohlcvs::Candle> {
    (0..6)
        .map(|i| {
            let ts = anchor_ts() - (5 - i) * MINUTE;
            candle(ts, 100.0, 110.0, 90.0, 100.0, 1.0)
        })
        .collect()
}

#[tokio::test]
async fn breakout_upward_requires_clearing_the_period_high_plus_confirmation() {
    let series = breakout_series();
    // Period high is 110; a 2% confirmation puts the level at 112.2.
    let below = context_with_candles(112.0, "1m", bundle_with("1m", series.clone()));
    assert!(!eval(&PriceBreakoutCondition, &breakout(5, "UPWARD", 2.0), &below).await);

    let above = context_with_candles(112.2, "1m", bundle_with("1m", series));
    assert!(eval(&PriceBreakoutCondition, &breakout(5, "UPWARD", 2.0), &above).await);
}

#[tokio::test]
async fn breakout_downward_requires_breaking_the_period_low_minus_confirmation() {
    let series = breakout_series();
    // Period low is 90; a 2% confirmation puts the level at 88.2.
    let above = context_with_candles(88.3, "1m", bundle_with("1m", series.clone()));
    assert!(
        !eval(
            &PriceBreakoutCondition,
            &breakout(5, "DOWNWARD", 2.0),
            &above
        )
        .await
    );

    let below = context_with_candles(88.2, "1m", bundle_with("1m", series));
    assert!(
        eval(
            &PriceBreakoutCondition,
            &breakout(5, "DOWNWARD", 2.0),
            &below
        )
        .await
    );
}

#[tokio::test]
async fn breakout_zero_confirmation_uses_the_raw_period_extreme() {
    let ctx = context_with_candles(110.0, "1m", bundle_with("1m", breakout_series()));
    assert!(eval(&PriceBreakoutCondition, &breakout(5, "UPWARD", 0.0), &ctx).await);
}

#[tokio::test]
async fn breakout_errors_without_enough_history() {
    let short = vec![candle(anchor_ts(), 100.0, 110.0, 90.0, 100.0, 1.0)];
    let ctx = context_with_candles(200.0, "1m", bundle_with("1m", short));
    let err = eval_err(&PriceBreakoutCondition, &breakout(20, "UPWARD", 1.0), &ctx).await;
    assert!(
        matches!(
            err,
            StrategyError::InsufficientCandles {
                indicator: "price breakout",
                ..
            }
        ),
        "unexpected error: {err}"
    );
}

// ==================== VolumeSpike ====================

fn spike(lookback: i64, multiplier: f64) -> Condition {
    condition(
        "VolumeSpike",
        &[
            ("lookback", json!(lookback)),
            ("multiplier", json!(multiplier)),
        ],
    )
}

/// `lookback` candles of volume 100 followed by one candle of `current_volume`.
fn volume_series(lookback: usize, current_volume: f64) -> Vec<screenerbot::ohlcvs::Candle> {
    let mut out: Vec<_> = (0..lookback)
        .map(|i| {
            let ts = anchor_ts() - ((lookback - i) as i64) * MINUTE;
            candle(ts, 100.0, 100.0, 100.0, 100.0, 100.0)
        })
        .collect();
    out.push(candle(
        anchor_ts(),
        100.0,
        100.0,
        100.0,
        100.0,
        current_volume,
    ));
    out
}

#[tokio::test]
async fn volume_spike_boundary_is_inclusive() {
    // Average of the lookback window is 100; the current candle is exactly 2x.
    let ctx = context_with_candles(100.0, "1m", bundle_with("1m", volume_series(5, 200.0)));
    assert!(eval(&VolumeSpikeCondition, &spike(5, 2.0), &ctx).await);
    assert!(!eval(&VolumeSpikeCondition, &spike(5, 2.001), &ctx).await);
}

#[tokio::test]
async fn volume_spike_excludes_the_current_candle_from_its_own_average() {
    // A single enormous candle must not inflate the baseline it is measured against.
    // Baseline stays 100, so 1000 is a 10x spike, not ~5.7x.
    let ctx = context_with_candles(100.0, "1m", bundle_with("1m", volume_series(5, 1000.0)));
    assert!(eval(&VolumeSpikeCondition, &spike(5, 9.9), &ctx).await);
}

#[tokio::test]
async fn volume_spike_errors_when_the_baseline_is_zero() {
    // A zero baseline would divide to infinity and fire on every token forever.
    let mut candles: Vec<_> = (0..5)
        .map(|i| {
            let ts = anchor_ts() - (5 - i) * MINUTE;
            candle(ts, 100.0, 100.0, 100.0, 100.0, 0.0)
        })
        .collect();
    candles.push(candle(anchor_ts(), 100.0, 100.0, 100.0, 100.0, 500.0));
    let ctx = context_with_candles(100.0, "1m", bundle_with("1m", candles));
    let err = eval_err(&VolumeSpikeCondition, &spike(5, 2.0), &ctx).await;
    assert!(
        matches!(
            err,
            StrategyError::InvalidConditionValue {
                field: "average volume",
                ..
            }
        ),
        "unexpected error: {err}"
    );
}

#[tokio::test]
async fn volume_spike_needs_more_candles_than_the_lookback() {
    let ctx = context_with_candles(100.0, "1m", bundle_with("1m", volume_series(2, 500.0)));
    let err = eval_err(&VolumeSpikeCondition, &spike(20, 2.0), &ctx).await;
    assert!(
        matches!(
            err,
            StrategyError::InsufficientCandles {
                indicator: "volume spike",
                ..
            }
        ),
        "unexpected error: {err}"
    );
}

#[tokio::test]
async fn volume_spike_validation_bounds() {
    assert!(VolumeSpikeCondition.validate(&spike(1, 2.0)).is_err());
    assert!(VolumeSpikeCondition.validate(&spike(101, 2.0)).is_err());
    assert!(VolumeSpikeCondition.validate(&spike(20, 0.9)).is_err());
    assert!(VolumeSpikeCondition.validate(&spike(20, 51.0)).is_err());
    assert!(VolumeSpikeCondition.validate(&spike(20, 2.0)).is_ok());
}

// ==================== LiquidityLevel ====================

fn liquidity(threshold: f64, comparison: &str) -> Condition {
    condition(
        "LiquidityLevel",
        &[
            ("threshold", json!(threshold)),
            ("comparison", json!(comparison)),
        ],
    )
}

#[tokio::test]
async fn liquidity_comparisons_split_strict_from_inclusive() {
    let ctx = market_context(Some(100.0));
    assert!(
        !eval(
            &LiquidityLevelCondition,
            &liquidity(100.0, "GREATER_THAN"),
            &ctx
        )
        .await
    );
    assert!(
        eval(
            &LiquidityLevelCondition,
            &liquidity(100.0, "GREATER_EQUAL"),
            &ctx
        )
        .await
    );
    assert!(
        !eval(
            &LiquidityLevelCondition,
            &liquidity(100.0, "LESS_THAN"),
            &ctx
        )
        .await
    );
    assert!(
        eval(
            &LiquidityLevelCondition,
            &liquidity(100.0, "LESS_EQUAL"),
            &ctx
        )
        .await
    );
}

#[tokio::test]
async fn liquidity_missing_data_errors_rather_than_passing_the_filter() {
    // A liquidity floor exists to keep the bot out of untradeable pools. If unknown
    // liquidity evaluated to `true`, the guard would invert exactly when it matters.
    let err = eval_err(
        &LiquidityLevelCondition,
        &liquidity(100.0, "GREATER_THAN"),
        &market_context(None),
    )
    .await;
    assert!(
        matches!(
            err,
            StrategyError::MissingContextData {
                data: "liquidity data"
            }
        ),
        "unexpected error: {err}"
    );

    let err = eval_err(
        &LiquidityLevelCondition,
        &liquidity(100.0, "GREATER_THAN"),
        &context_bare(Some(1.0)),
    )
    .await;
    assert!(
        matches!(
            err,
            StrategyError::MissingContextData {
                data: "market data"
            }
        ),
        "unexpected error: {err}"
    );
}

#[tokio::test]
async fn liquidity_validation_rejects_negative_thresholds() {
    assert!(LiquidityLevelCondition
        .validate(&liquidity(-1.0, "GREATER_THAN"))
        .is_err());
    assert!(LiquidityLevelCondition
        .validate(&liquidity(10.0, "EQUALS"))
        .is_err());
    assert!(LiquidityLevelCondition
        .validate(&liquidity(10.0, "GREATER_THAN"))
        .is_ok());
}

// ==================== PositionHoldingTime ====================

fn holding(hours: f64, comparison: &str) -> Condition {
    condition(
        "PositionHoldingTime",
        &[("hours", json!(hours)), ("comparison", json!(comparison))],
    )
}

#[tokio::test]
async fn holding_time_comparisons_split_strict_from_inclusive() {
    let ctx = position_context(2.0);
    assert!(
        !eval(
            &PositionHoldingTimeCondition,
            &holding(2.0, "GREATER_THAN"),
            &ctx
        )
        .await
    );
    assert!(
        eval(
            &PositionHoldingTimeCondition,
            &holding(2.0, "GREATER_EQUAL"),
            &ctx
        )
        .await
    );
    assert!(
        eval(
            &PositionHoldingTimeCondition,
            &holding(3.0, "LESS_THAN"),
            &ctx
        )
        .await
    );
    assert!(
        !eval(
            &PositionHoldingTimeCondition,
            &holding(1.0, "LESS_EQUAL"),
            &ctx
        )
        .await
    );
}

#[tokio::test]
async fn holding_time_without_a_position_errors() {
    // Entry strategies carry no position; a holding-time condition there is a mistake
    // the author must see, not a silent `false`.
    let err = eval_err(
        &PositionHoldingTimeCondition,
        &holding(2.0, "GREATER_THAN"),
        &context_bare(Some(1.0)),
    )
    .await;
    assert!(
        matches!(
            err,
            StrategyError::MissingContextData {
                data: "position data"
            }
        ),
        "unexpected error: {err}"
    );
}

#[tokio::test]
async fn holding_time_validation_bounds() {
    assert!(PositionHoldingTimeCondition
        .validate(&holding(-1.0, "GREATER_THAN"))
        .is_err());
    assert!(PositionHoldingTimeCondition
        .validate(&holding(1.0, "AROUND"))
        .is_err());
    assert!(PositionHoldingTimeCondition
        .validate(&holding(1.0, "GREATER_THAN"))
        .is_ok());
}

// ==================== PARAMETER PLUMBING ====================

#[tokio::test]
async fn missing_parameter_is_reported_by_name() {
    let cond = condition("PriceToMA", &[("period", json!(5))]);
    let err = PriceToMaCondition.validate(&cond).unwrap_err();
    assert!(
        matches!(
            err,
            StrategyError::MissingConditionParameter { field: "distance" }
        ),
        "unexpected error: {err}"
    );
}

#[tokio::test]
async fn wrongly_typed_parameter_is_rejected_not_coerced() {
    // A numeric parameter arriving as a JSON string must fail loudly: silently reading
    // it as 0 would turn a 20-period MA into a division by zero.
    let cond = condition(
        "PriceToMA",
        &[
            ("period", json!("20")),
            ("position", json!("ABOVE")),
            ("distance", json!(2.0)),
        ],
    );
    let err = PriceToMaCondition.validate(&cond).unwrap_err();
    assert!(
        matches!(
            err,
            StrategyError::ConditionParameterType {
                field: "period",
                expected: "a number"
            }
        ),
        "unexpected error: {err}"
    );
}

#[test]
fn every_condition_reports_the_type_name_it_is_registered_under() {
    // The registry is keyed on `condition_type()`, and a strategy stored in the DB
    // references that exact string. A rename here orphans every saved strategy.
    assert_eq!(
        PriceChangePercentCondition.condition_type(),
        "PriceChangePercent"
    );
    assert_eq!(PriceToMaCondition.condition_type(), "PriceToMA");
    assert_eq!(
        ConsecutiveCandlesCondition.condition_type(),
        "ConsecutiveCandles"
    );
    assert_eq!(CandleSizeCondition.condition_type(), "CandleSize");
    assert_eq!(PriceBreakoutCondition.condition_type(), "PriceBreakout");
    assert_eq!(VolumeSpikeCondition.condition_type(), "VolumeSpike");
    assert_eq!(LiquidityLevelCondition.condition_type(), "LiquidityLevel");
    assert_eq!(
        PositionHoldingTimeCondition.condition_type(),
        "PositionHoldingTime"
    );
}
