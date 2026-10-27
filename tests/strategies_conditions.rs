// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Pure: the strategy condition evaluators — the leaves every user strategy is
//! built from. No network, no DB.
//!
//! A condition is what actually decides to spend SOL, so the properties that matter are
//! the ones that are silent when wrong: which side of a threshold `>=` falls on, whether
//! a direction is measured with the right sign, and whether missing data errors out
//! instead of quietly evaluating to `false` (a `false` reads as "no signal" and is
//! indistinguishable from a working condition that simply did not fire).

use crate::common;

use common::{
    anchor_ts, bundle_with, candle, candle_series, condition, context_bare, context_with_candles,
};
use screenerbot::strategies::conditions::{
    get_current_price, usable_basis, BollingerBandsCondition, BuyPressureCondition,
    CandleSizeCondition, ConditionEvaluator, ConsecutiveCandlesCondition,
    DrawdownFromPeakCondition, LiquidityChangeSinceEntryCondition, LiquidityLevelCondition,
    LiquidityToSizeCondition, MaCrossCondition, PositionAddsCondition,
    PositionHoldingTimeCondition, PositionProfitCondition, PriceBreakoutCondition,
    PriceChangePercentCondition, PriceToMaCondition, ProfitGivebackCondition,
    PullbackFromHighCondition, RsiCondition, TimeOfDayCondition, TokenAgeCondition,
    TransactionVelocityCondition, VolatilityRangeCondition, VolumeSpikeCondition, VwapCondition,
};
use screenerbot::strategies::types::{
    Condition, EvaluationContext, LiquidityPool, MarketData, PositionData, StrategyType,
    TransactionCounts, WindowCounts,
};
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
        liquidity_pool: None,
        transactions: None,
        snapshot_at: None,
        token_created_at: None,
        reference_size_native: None,
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
        peak_price: None,
        dca_count: 0,
        partial_exit_count: 0,
        remaining_value_native: None,
        entry_liquidity_native: None,
        entry_liquidity_pool: None,
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
async fn candle_size_refuses_a_zero_open_as_unusable_data() {
    // A zero open divides to infinity and would clear every large-body threshold.
    let broken = vec![candle(anchor_ts(), 0.0, 110.0, 0.0, 110.0, 1.0)];
    let ctx = context_with_candles(110.0, "1m", bundle_with("1m", broken));
    let err = eval_err(&CandleSizeCondition, &shape("LARGE_BODY", 10.0), &ctx).await;
    assert!(
        matches!(
            err,
            StrategyError::UnusableData {
                field: "candle open",
                ..
            }
        ),
        "unexpected error: {err}"
    );
    assert!(err.is_data_unavailable());
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
async fn volume_spike_refuses_a_zero_baseline_as_unusable_data() {
    // A zero baseline would divide to infinity and fire on every token forever. It is a
    // fact about the market, not the configuration, so a rule tree reads it as unknown.
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
            StrategyError::UnusableData {
                field: "average volume",
                ..
            }
        ),
        "unexpected error: {err}"
    );
    assert!(err.is_data_unavailable());
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

// ==================== Rsi ====================

fn rsi(period: i64, comparison: &str, threshold: f64) -> Condition {
    condition(
        "Rsi",
        &[
            ("period", json!(period)),
            ("comparison", json!(comparison)),
            ("threshold", json!(threshold)),
        ],
    )
}

fn closes_context(closes: &[f64]) -> EvaluationContext {
    context_with_candles(
        1.0,
        "5m",
        bundle_with("5m", candle_series(anchor_ts(), MINUTE * 5, closes)),
    )
}

#[tokio::test]
async fn rsi_of_strictly_rising_closes_is_one_hundred() {
    let rising: Vec<f64> = (1..=15).map(f64::from).collect();
    let ctx = closes_context(&rising);
    assert!(eval(&RsiCondition, &rsi(14, "ABOVE", 99.0), &ctx).await);
    assert!(!eval(&RsiCondition, &rsi(14, "BELOW", 99.0), &ctx).await);
}

#[tokio::test]
async fn rsi_of_strictly_falling_closes_is_zero() {
    let falling: Vec<f64> = (1..=15).rev().map(f64::from).collect();
    let ctx = closes_context(&falling);
    assert!(eval(&RsiCondition, &rsi(14, "BELOW", 1.0), &ctx).await);
    assert!(!eval(&RsiCondition, &rsi(14, "ABOVE", 1.0), &ctx).await);
}

#[tokio::test]
async fn rsi_of_equal_gains_and_losses_is_fifty_and_the_comparisons_are_strict() {
    let ctx = closes_context(&[10.0, 11.0, 10.0]);
    assert!(!eval(&RsiCondition, &rsi(2, "ABOVE", 50.0), &ctx).await);
    assert!(!eval(&RsiCondition, &rsi(2, "BELOW", 50.0), &ctx).await);
    assert!(eval(&RsiCondition, &rsi(2, "ABOVE", 49.0), &ctx).await);
    assert!(eval(&RsiCondition, &rsi(2, "BELOW", 51.0), &ctx).await);
}

#[tokio::test]
async fn rsi_of_a_flat_series_is_fifty() {
    let ctx = closes_context(&[5.0; 4]);
    assert!(eval(&RsiCondition, &rsi(3, "ABOVE", 49.0), &ctx).await);
    assert!(eval(&RsiCondition, &rsi(3, "BELOW", 51.0), &ctx).await);
}

#[tokio::test]
async fn rsi_matches_a_hand_computed_wilder_value() {
    // Period 2 over [10, 11, 10, 12]: changes +1, -1, +2. The first two seed the averages
    // (gain 0.5, loss 0.5); the third smooths in: gain (0.5 + 2) / 2 = 1.25, loss
    // 0.5 / 2 = 0.25. RS = 5, RSI = 100 - 100 / 6 = 83.33.
    let ctx = closes_context(&[10.0, 11.0, 10.0, 12.0]);
    assert!(eval(&RsiCondition, &rsi(2, "ABOVE", 83.3), &ctx).await);
    assert!(!eval(&RsiCondition, &rsi(2, "ABOVE", 83.34), &ctx).await);
    assert!(eval(&RsiCondition, &rsi(2, "BELOW", 83.34), &ctx).await);
    assert!(!eval(&RsiCondition, &rsi(2, "BELOW", 83.3), &ctx).await);
}

#[tokio::test]
async fn rsi_reads_candles_and_never_the_live_price() {
    let mut ctx = closes_context(&[10.0, 11.0, 10.0, 12.0]);
    ctx.current_price = None;
    assert!(eval(&RsiCondition, &rsi(2, "ABOVE", 80.0), &ctx).await);
}

#[tokio::test]
async fn rsi_needs_one_more_close_than_its_period() {
    let ctx = closes_context(&[10.0; 14]);
    let err = eval_err(&RsiCondition, &rsi(14, "BELOW", 30.0), &ctx).await;
    assert!(
        matches!(
            err,
            StrategyError::InsufficientCandles {
                indicator: "rsi",
                available: 14,
                required: 15
            }
        ),
        "unexpected error: {err}"
    );
    let missing = eval_err(&RsiCondition, &rsi(14, "BELOW", 30.0), &context_bare(None)).await;
    assert!(matches!(
        missing,
        StrategyError::MissingContextData { data: "OHLCV data" }
    ));
}

#[test]
fn rsi_validation_bounds() {
    assert!(RsiCondition.validate(&rsi(1, "BELOW", 30.0)).is_err());
    assert!(RsiCondition.validate(&rsi(101, "BELOW", 30.0)).is_err());
    assert!(RsiCondition.validate(&rsi(14, "BELOW", 0.0)).is_err());
    assert!(RsiCondition.validate(&rsi(14, "BELOW", 100.0)).is_err());
    assert!(RsiCondition.validate(&rsi(14, "NEAR", 30.0)).is_err());
    assert!(RsiCondition.validate(&rsi(2, "ABOVE", 1.0)).is_ok());
    assert!(RsiCondition.validate(&rsi(100, "BELOW", 99.0)).is_ok());
}

// ==================== MaCross ====================

fn cross(ma_type: &str, fast: i64, slow: i64, direction: &str, within_candles: i64) -> Condition {
    condition(
        "MaCross",
        &[
            ("ma_type", json!(ma_type)),
            ("fast_period", json!(fast)),
            ("slow_period", json!(slow)),
            ("direction", json!(direction)),
            ("within_candles", json!(within_candles)),
        ],
    )
}

#[tokio::test]
async fn sma_bullish_cross_is_found_on_the_last_close() {
    // Fast SMA(2) and slow SMA(3) are both 10 before the jump, then 15 against 13.33.
    let ctx = closes_context(&[10.0, 10.0, 10.0, 10.0, 10.0, 20.0]);
    assert!(eval(&MaCrossCondition, &cross("SMA", 2, 3, "BULLISH", 1), &ctx).await);
    assert!(!eval(&MaCrossCondition, &cross("SMA", 2, 3, "BEARISH", 1), &ctx).await);
}

#[tokio::test]
async fn a_cross_is_found_within_n_candles_and_not_one_further_back() {
    // The cross happens on the second-to-last close; the last close keeps fast above slow.
    let ctx = closes_context(&[10.0, 10.0, 10.0, 10.0, 10.0, 20.0, 20.0]);
    assert!(!eval(&MaCrossCondition, &cross("SMA", 2, 3, "BULLISH", 1), &ctx).await);
    assert!(eval(&MaCrossCondition, &cross("SMA", 2, 3, "BULLISH", 2), &ctx).await);
}

#[tokio::test]
async fn sma_bearish_cross_mirrors_the_bullish_one() {
    let ctx = closes_context(&[20.0, 20.0, 20.0, 20.0, 20.0, 10.0]);
    assert!(eval(&MaCrossCondition, &cross("SMA", 2, 3, "BEARISH", 1), &ctx).await);
    assert!(!eval(&MaCrossCondition, &cross("SMA", 2, 3, "BULLISH", 1), &ctx).await);
}

#[tokio::test]
async fn ema_bullish_cross_is_found() {
    // EMA(2) reaches 16.67 against EMA(3) at 15 after both sat at 10.
    let ctx = closes_context(&[10.0, 10.0, 10.0, 10.0, 10.0, 20.0]);
    assert!(eval(&MaCrossCondition, &cross("EMA", 2, 3, "BULLISH", 1), &ctx).await);
}

#[tokio::test]
async fn ema_and_sma_disagree_where_their_weights_differ() {
    // SMA(2) 12.5 crosses above SMA(3) 11.67. EMA(2) 13.70 stays under EMA(3) 14.17,
    // because the old 30 still weighs on the quicker average through its seed.
    let ctx = closes_context(&[30.0, 10.0, 10.0, 10.0, 15.0]);
    assert!(eval(&MaCrossCondition, &cross("SMA", 2, 3, "BULLISH", 1), &ctx).await);
    assert!(!eval(&MaCrossCondition, &cross("EMA", 2, 3, "BULLISH", 1), &ctx).await);
}

#[tokio::test]
async fn touching_without_crossing_is_not_a_cross() {
    let ctx = closes_context(&[10.0; 6]);
    assert!(!eval(&MaCrossCondition, &cross("SMA", 2, 3, "BULLISH", 3), &ctx).await);
    assert!(!eval(&MaCrossCondition, &cross("EMA", 2, 3, "BEARISH", 3), &ctx).await);
}

#[tokio::test]
async fn ma_cross_needs_the_slow_period_plus_the_window() {
    let ctx = closes_context(&[10.0; 4]);
    let err = eval_err(&MaCrossCondition, &cross("SMA", 2, 3, "BULLISH", 2), &ctx).await;
    assert!(
        matches!(
            err,
            StrategyError::InsufficientCandles {
                indicator: "moving average cross",
                available: 4,
                required: 5
            }
        ),
        "unexpected error: {err}"
    );
    let missing = eval_err(
        &MaCrossCondition,
        &cross("SMA", 2, 3, "BULLISH", 1),
        &context_bare(Some(1.0)),
    )
    .await;
    assert!(matches!(
        missing,
        StrategyError::MissingContextData { data: "OHLCV data" }
    ));
}

#[test]
fn ma_cross_validation_bounds() {
    let validate = |c: Condition| MaCrossCondition.validate(&c);
    assert!(validate(cross("EMA", 9, 21, "BULLISH", 1)).is_ok());
    assert!(validate(cross("SMA", 100, 200, "BEARISH", 20)).is_ok());
    assert!(validate(cross("EMA", 1, 21, "BULLISH", 1)).is_err());
    assert!(validate(cross("EMA", 101, 150, "BULLISH", 1)).is_err());
    assert!(validate(cross("EMA", 9, 201, "BULLISH", 1)).is_err());
    assert!(validate(cross("EMA", 9, 21, "BULLISH", 0)).is_err());
    assert!(validate(cross("EMA", 9, 21, "BULLISH", 21)).is_err());
    assert!(validate(cross("WMA", 9, 21, "BULLISH", 1)).is_err());
    assert!(validate(cross("EMA", 9, 21, "SIDEWAYS", 1)).is_err());
    // The slow average must be strictly slower than the fast one.
    assert!(matches!(
        validate(cross("EMA", 9, 9, "BULLISH", 1)),
        Err(StrategyError::InvalidConditionValue {
            field: "slow period",
            ..
        })
    ));
    assert!(validate(cross("EMA", 9, 5, "BULLISH", 1)).is_err());
}

// ==================== PositionProfit ====================

fn profit(comparison: &str, threshold_pct: f64) -> Condition {
    condition(
        "PositionProfit",
        &[
            ("comparison", json!(comparison)),
            ("threshold_pct", json!(threshold_pct)),
        ],
    )
}

fn profit_context(unrealized_profit_pct: Option<f64>) -> EvaluationContext {
    let mut ctx = context_bare(Some(1.0));
    ctx.position_data = Some(PositionData {
        entry_price: 1.0,
        entry_time: chrono::Utc::now(),
        current_size_sol: 1.0,
        unrealized_profit_pct,
        position_age_hours: 1.0,
        peak_price: None,
        dca_count: 0,
        partial_exit_count: 0,
        remaining_value_native: None,
        entry_liquidity_native: None,
        entry_liquidity_pool: None,
    });
    ctx
}

#[tokio::test]
async fn position_profit_boundaries_are_inclusive_on_both_sides() {
    let ctx = profit_context(Some(20.0));
    assert!(eval(&PositionProfitCondition, &profit("ABOVE", 20.0), &ctx).await);
    assert!(!eval(&PositionProfitCondition, &profit("ABOVE", 20.001), &ctx).await);
    assert!(eval(&PositionProfitCondition, &profit("BELOW", 20.0), &ctx).await);
    assert!(!eval(&PositionProfitCondition, &profit("BELOW", 19.999), &ctx).await);
}

#[tokio::test]
async fn position_profit_reads_negative_thresholds() {
    let ctx = profit_context(Some(-30.0));
    assert!(eval(&PositionProfitCondition, &profit("BELOW", -25.0), &ctx).await);
    assert!(!eval(&PositionProfitCondition, &profit("BELOW", -35.0), &ctx).await);
}

#[tokio::test]
async fn position_profit_without_a_reading_errors() {
    for ctx in [context_bare(Some(1.0)), profit_context(None)] {
        let err = eval_err(&PositionProfitCondition, &profit("ABOVE", 20.0), &ctx).await;
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
}

#[test]
fn position_profit_validation_bounds() {
    assert!(PositionProfitCondition
        .validate(&profit("ABOVE", -99.0))
        .is_ok());
    assert!(PositionProfitCondition
        .validate(&profit("BELOW", 10000.0))
        .is_ok());
    assert!(PositionProfitCondition
        .validate(&profit("ABOVE", -100.0))
        .is_err());
    assert!(PositionProfitCondition
        .validate(&profit("ABOVE", 10001.0))
        .is_err());
    assert!(PositionProfitCondition
        .validate(&profit("AROUND", 20.0))
        .is_err());
}

// ==================== DrawdownFromPeak ====================

fn drawdown(drop_pct: f64, arm_after_profit_pct: f64) -> Condition {
    condition(
        "DrawdownFromPeak",
        &[
            ("drop_pct", json!(drop_pct)),
            ("arm_after_profit_pct", json!(arm_after_profit_pct)),
        ],
    )
}

/// A position entered at `entry`, whose highest pool price was `peak`, priced at `price`.
fn peak_context(entry: f64, peak: Option<f64>, price: Option<f64>) -> EvaluationContext {
    let mut ctx = context_bare(price);
    ctx.position_data = Some(PositionData {
        entry_price: entry,
        entry_time: chrono::Utc::now(),
        current_size_sol: 1.0,
        unrealized_profit_pct: None,
        position_age_hours: 1.0,
        peak_price: peak,
        dca_count: 0,
        partial_exit_count: 0,
        remaining_value_native: None,
        entry_liquidity_native: None,
        entry_liquidity_pool: None,
    });
    ctx
}

#[tokio::test]
async fn drawdown_fires_exactly_at_the_drop_and_not_just_short_of_it() {
    // 8 -> 6 is exactly a 25% drop; entry 4 puts the peak exactly 100% above entry.
    let ctx = peak_context(4.0, Some(8.0), Some(6.0));
    assert!(eval(&DrawdownFromPeakCondition, &drawdown(25.0, 0.0), &ctx).await);
    assert!(!eval(&DrawdownFromPeakCondition, &drawdown(25.01, 0.0), &ctx).await);
}

#[tokio::test]
async fn drawdown_waits_for_the_peak_to_clear_the_arming_profit() {
    let ctx = peak_context(4.0, Some(8.0), Some(6.0));
    assert!(eval(&DrawdownFromPeakCondition, &drawdown(10.0, 100.0), &ctx).await);
    assert!(!eval(&DrawdownFromPeakCondition, &drawdown(10.0, 100.5), &ctx).await);
}

#[tokio::test]
async fn drawdown_does_not_fire_at_a_new_high() {
    let ctx = peak_context(4.0, Some(8.0), Some(8.0));
    assert!(!eval(&DrawdownFromPeakCondition, &drawdown(0.1, 0.0), &ctx).await);
}

#[tokio::test]
async fn drawdown_refuses_missing_or_unusable_data() {
    let cond = drawdown(10.0, 0.0);
    let no_position = eval_err(&DrawdownFromPeakCondition, &cond, &context_bare(Some(6.0))).await;
    assert!(matches!(
        no_position,
        StrategyError::MissingContextData {
            data: "position data"
        }
    ));
    let no_peak = eval_err(
        &DrawdownFromPeakCondition,
        &cond,
        &peak_context(4.0, None, Some(6.0)),
    )
    .await;
    assert!(matches!(
        no_peak,
        StrategyError::MissingContextData { data: "peak price" }
    ));
    let no_price = eval_err(
        &DrawdownFromPeakCondition,
        &cond,
        &peak_context(4.0, Some(8.0), None),
    )
    .await;
    assert!(matches!(
        no_price,
        StrategyError::MissingContextData {
            data: "current price"
        }
    ));
    let zero_entry = eval_err(
        &DrawdownFromPeakCondition,
        &cond,
        &peak_context(0.0, Some(8.0), Some(6.0)),
    )
    .await;
    assert!(matches!(
        zero_entry,
        StrategyError::UnusableData {
            field: "entry price",
            ..
        }
    ));
    let zero_peak = eval_err(
        &DrawdownFromPeakCondition,
        &cond,
        &peak_context(4.0, Some(0.0), Some(6.0)),
    )
    .await;
    assert!(matches!(
        zero_peak,
        StrategyError::UnusableData {
            field: "peak price",
            ..
        }
    ));
}

#[test]
fn drawdown_validation_bounds() {
    assert!(DrawdownFromPeakCondition
        .validate(&drawdown(0.1, 0.0))
        .is_ok());
    assert!(DrawdownFromPeakCondition
        .validate(&drawdown(99.0, 10000.0))
        .is_ok());
    assert!(DrawdownFromPeakCondition
        .validate(&drawdown(0.09, 0.0))
        .is_err());
    assert!(DrawdownFromPeakCondition
        .validate(&drawdown(99.5, 0.0))
        .is_err());
    assert!(DrawdownFromPeakCondition
        .validate(&drawdown(10.0, -1.0))
        .is_err());
    assert!(DrawdownFromPeakCondition
        .validate(&drawdown(10.0, 10001.0))
        .is_err());
}

// ==================== BuyPressure ====================

fn pressure(window: &str, min_buy_share_pct: f64, min_transactions: i64) -> Condition {
    condition(
        "BuyPressure",
        &[
            ("window", json!(window)),
            ("min_buy_share_pct", json!(min_buy_share_pct)),
            ("min_transactions", json!(min_transactions)),
        ],
    )
}

#[tokio::test]
async fn liquidity_read_from_another_pool_than_the_entry_is_unknown() {
    // A graduation from the bonding curve, or a deeper pool found later: the live
    // reserves belong to a pool the entry reading says nothing about. Measured anyway,
    // 64 at entry against 20 now reads as a 69% drain and sells.
    let cond = liquidity_change("DROPPED_BY", 25.0);
    let mut moved = liquidity_since(Some(64.0), Some(20.0));
    live_pool(&mut moved).address = "graduated-pool".to_owned();
    let err = eval_err(&LiquidityChangeSinceEntryCondition, &cond, &moved).await;
    assert!(
        matches!(
            err,
            StrategyError::MissingContextData {
                data: "entry pool liquidity"
            }
        ),
        "unexpected error: {err}"
    );
    assert!(err.is_data_unavailable());

    // The same drain in the pool the entry was read from still fires.
    assert!(
        eval(
            &LiquidityChangeSinceEntryCondition,
            &cond,
            &liquidity_since(Some(64.0), Some(20.0))
        )
        .await
    );
}

#[tokio::test]
async fn liquidity_of_a_pool_without_a_known_reserve_kind_is_unknown() {
    let cond = liquidity_change("DROPPED_BY", 25.0);

    // The pool's decoder reports virtual reserves: the bonding curve's fixed virtual base
    // dilutes every change, so even a full drain of its real coin can read under 50%.
    let mut curve = liquidity_since(Some(64.0), Some(20.0));
    live_pool(&mut curve).virtual_reserves = Some(true);
    let err = eval_err(&LiquidityChangeSinceEntryCondition, &cond, &curve).await;
    assert!(
        matches!(
            err,
            StrategyError::MissingContextData {
                data: "real pool reserves"
            }
        ),
        "unexpected error: {err}"
    );

    // A pool whose program is not known says nothing about its reserves either.
    let mut unknown = liquidity_since(Some(64.0), Some(20.0));
    live_pool(&mut unknown).virtual_reserves = None;
    let err = eval_err(&LiquidityChangeSinceEntryCondition, &cond, &unknown).await;
    assert!(
        matches!(
            err,
            StrategyError::MissingContextData {
                data: "real pool reserves"
            }
        ),
        "unexpected error: {err}"
    );

    // A live reading that names no pool cannot be matched to the entry pool.
    let mut unnamed = liquidity_since(Some(64.0), Some(20.0));
    unnamed.market_data.as_mut().expect("market").liquidity_pool = None;
    let err = eval_err(&LiquidityChangeSinceEntryCondition, &cond, &unnamed).await;
    assert!(
        matches!(
            err,
            StrategyError::MissingContextData {
                data: "entry pool liquidity"
            }
        ),
        "unexpected error: {err}"
    );
}

fn counts(buys: u64, sells: u64) -> Option<WindowCounts> {
    Some(WindowCounts { buys, sells })
}

fn transactions_context(transactions: Option<TransactionCounts>) -> EvaluationContext {
    let mut ctx = context_bare(Some(1.0));
    ctx.market_data = Some(MarketData {
        liquidity_sol: None,
        liquidity_pool: None,
        transactions,
        snapshot_at: Some(common::anchor_time()),
        token_created_at: None,
        reference_size_native: None,
    });
    ctx
}

fn one_hour(buys: u64, sells: u64) -> EvaluationContext {
    transactions_context(Some(TransactionCounts {
        m5: None,
        h1: counts(buys, sells),
        h6: None,
        h24: None,
    }))
}

#[tokio::test]
async fn buy_share_boundary_is_inclusive() {
    let ctx = one_hour(60, 40);
    assert!(eval(&BuyPressureCondition, &pressure("1h", 60.0, 100), &ctx).await);
    assert!(!eval(&BuyPressureCondition, &pressure("1h", 60.5, 100), &ctx).await);
}

#[tokio::test]
async fn buy_pressure_is_false_below_the_minimum_transactions() {
    let ctx = one_hour(60, 40);
    assert!(!eval(&BuyPressureCondition, &pressure("1h", 50.0, 101), &ctx).await);
    // All buys, but too few of them to mean anything.
    let thin = one_hour(5, 0);
    assert!(!eval(&BuyPressureCondition, &pressure("1h", 50.0, 50), &thin).await);
}

#[tokio::test]
async fn buy_pressure_reads_the_selected_window() {
    let ctx = transactions_context(Some(TransactionCounts {
        m5: counts(10, 0),
        h1: counts(0, 10),
        h6: counts(10, 10),
        h24: counts(30, 10),
    }));
    assert!(eval(&BuyPressureCondition, &pressure("5m", 100.0, 10), &ctx).await);
    assert!(!eval(&BuyPressureCondition, &pressure("1h", 50.0, 10), &ctx).await);
    assert!(eval(&BuyPressureCondition, &pressure("6h", 50.0, 20), &ctx).await);
    assert!(eval(&BuyPressureCondition, &pressure("24h", 75.0, 40), &ctx).await);
    assert!(!eval(&BuyPressureCondition, &pressure("24h", 76.0, 40), &ctx).await);
}

#[tokio::test]
async fn buy_pressure_refuses_missing_counts() {
    let cond = pressure("1h", 60.0, 50);
    let missing_window = transactions_context(Some(TransactionCounts {
        m5: counts(100, 0),
        h1: None,
        h6: None,
        h24: None,
    }));
    for ctx in [
        context_bare(Some(1.0)),
        transactions_context(None),
        missing_window,
    ] {
        let err = eval_err(&BuyPressureCondition, &cond, &ctx).await;
        assert!(
            matches!(
                err,
                StrategyError::MissingContextData {
                    data: "transaction counts"
                }
            ),
            "unexpected error: {err}"
        );
    }
}

/// Every window's counts from a snapshot taken `age_seconds` before the evaluation instant.
fn snapshot_aged(age_seconds: i64) -> EvaluationContext {
    let counts = counts(80, 20);
    let mut ctx = transactions_context(Some(TransactionCounts {
        m5: counts,
        h1: counts,
        h6: counts,
        h24: counts,
    }));
    let market = ctx.market_data.as_mut().expect("market data");
    market.snapshot_at = Some(common::anchor_time() - chrono::Duration::seconds(age_seconds));
    ctx
}

#[tokio::test]
async fn buy_pressure_refuses_a_snapshot_older_than_its_window() {
    // Counts of a rolling window describe the market at the moment they were fetched; a
    // snapshot older than the window may count none of the trades still inside it.
    for (window, max_age) in [("5m", 300), ("1h", 3_600), ("6h", 21_600), ("24h", 86_400)] {
        let cond = pressure(window, 60.0, 50);
        let fresh = snapshot_aged(max_age);
        assert!(
            eval(&BuyPressureCondition, &cond, &fresh).await,
            "{window} at its own length is still current"
        );

        let stale = snapshot_aged(max_age + 1);
        let err = eval_err(&BuyPressureCondition, &cond, &stale).await;
        assert!(
            matches!(
                &err,
                StrategyError::StaleMarketData {
                    window: stale_window,
                    age_seconds,
                    max_age_seconds,
                } if stale_window == window
                    && *age_seconds == max_age + 1
                    && *max_age_seconds == max_age
            ),
            "{window}: unexpected error: {err}"
        );
        assert!(
            err.is_data_unavailable(),
            "{window}: stale data is not configuration"
        );
    }
}

#[tokio::test]
async fn buy_pressure_refuses_counts_without_a_snapshot_time() {
    // Counts whose age is unknown are never assumed fresh.
    let mut ctx = one_hour(60, 40);
    ctx.market_data.as_mut().expect("market data").snapshot_at = None;
    let err = eval_err(&BuyPressureCondition, &pressure("1h", 60.0, 100), &ctx).await;
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

#[test]
fn a_zero_or_non_finite_basis_or_price_is_unusable_data() {
    // A degenerate number is a fact about the market, so a rule tree reads the condition
    // as unknown instead of failing the whole strategy as a configuration error.
    for value in [0.0, -1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let err = usable_basis("moving average", value).expect_err("refused basis");
        assert!(
            matches!(
                err,
                StrategyError::UnusableData {
                    field: "moving average",
                    ..
                }
            ),
            "{value}: unexpected error: {err}"
        );
        assert!(err.is_data_unavailable());

        let err = get_current_price(&context_bare(Some(value))).expect_err("refused price");
        assert!(
            matches!(
                err,
                StrategyError::UnusableData {
                    field: "current price",
                    ..
                }
            ),
            "{value}: unexpected error: {err}"
        );
        assert!(err.is_data_unavailable());
    }
    assert_eq!(usable_basis("moving average", 2.5).expect("usable"), 2.5);
}

#[test]
fn buy_pressure_validation_bounds() {
    assert!(BuyPressureCondition
        .validate(&pressure("5m", 50.0, 1))
        .is_ok());
    assert!(BuyPressureCondition
        .validate(&pressure("24h", 100.0, 100000))
        .is_ok());
    assert!(BuyPressureCondition
        .validate(&pressure("2h", 60.0, 50))
        .is_err());
    assert!(BuyPressureCondition
        .validate(&pressure("1h", 49.0, 50))
        .is_err());
    assert!(BuyPressureCondition
        .validate(&pressure("1h", 101.0, 50))
        .is_err());
    assert!(BuyPressureCondition
        .validate(&pressure("1h", 60.0, 0))
        .is_err());
    assert!(BuyPressureCondition
        .validate(&pressure("1h", 60.0, 100001))
        .is_err());
}

#[test]
fn a_window_reported_on_one_side_only_or_negative_is_not_a_window() {
    assert_eq!(WindowCounts::from_reported(Some(3), Some(2)), counts(3, 2));
    assert_eq!(WindowCounts::from_reported(Some(0), Some(0)), counts(0, 0));
    assert_eq!(WindowCounts::from_reported(Some(3), None), None);
    assert_eq!(WindowCounts::from_reported(None, Some(2)), None);
    assert_eq!(WindowCounts::from_reported(Some(-1), Some(2)), None);
    assert_eq!(WindowCounts::from_reported(Some(3), Some(-2)), None);
}

#[test]
fn transaction_counts_follow_the_token_snapshot_windows() {
    let mut token = common::filter_token(common::TEST_MINT);
    token.txns_m5_buys = Some(1);
    token.txns_m5_sells = Some(2);
    token.txns_h1_buys = Some(3);
    token.txns_h1_sells = None;
    token.txns_h6_buys = Some(5);
    token.txns_h6_sells = Some(6);
    token.txns_h24_buys = Some(-7);
    token.txns_h24_sells = Some(8);
    let transactions = TransactionCounts::from_token(&token);
    assert_eq!(transactions.m5, counts(1, 2));
    assert_eq!(transactions.h1, None);
    assert_eq!(transactions.h6, counts(5, 6));
    assert_eq!(transactions.h24, None);
}

// ==================== TransactionVelocity ====================

fn velocity(window: &str, comparison: &str, ratio: f64, min_transactions: i64) -> Condition {
    condition(
        "TransactionVelocity",
        &[
            ("window", json!(window)),
            ("comparison", json!(comparison)),
            ("ratio", json!(ratio)),
            ("min_transactions", json!(min_transactions)),
        ],
    )
}

/// 60 trades in the last 5 minutes against 360 in the hour: twice the hourly pace.
fn twice_the_hourly_pace() -> EvaluationContext {
    transactions_context(Some(TransactionCounts {
        m5: counts(30, 30),
        h1: counts(200, 160),
        h6: None,
        h24: None,
    }))
}

#[tokio::test]
async fn transaction_velocity_boundaries_are_inclusive() {
    let ctx = twice_the_hourly_pace();
    let cond = |comparison, ratio| velocity("5m_vs_1h", comparison, ratio, 30);
    assert!(eval(&TransactionVelocityCondition, &cond("ABOVE", 2.0), &ctx).await);
    assert!(!eval(&TransactionVelocityCondition, &cond("ABOVE", 2.1), &ctx).await);
    assert!(eval(&TransactionVelocityCondition, &cond("BELOW", 2.0), &ctx).await);
    assert!(!eval(&TransactionVelocityCondition, &cond("BELOW", 1.9), &ctx).await);
}

#[tokio::test]
async fn transaction_velocity_is_false_below_the_short_window_floor() {
    let ctx = twice_the_hourly_pace();
    for comparison in ["ABOVE", "BELOW"] {
        let cond = velocity("5m_vs_1h", comparison, 2.0, 61);
        assert!(
            !eval(&TransactionVelocityCondition, &cond, &ctx).await,
            "{comparison}"
        );
    }
}

#[tokio::test]
async fn transaction_velocity_reads_the_selected_pair() {
    // 6h: 600 trades over 21600 s; 24h: 1200 over 86400 s, so the 6h pace is twice the
    // daily one, while the 5m/1h pair is flat.
    let ctx = transactions_context(Some(TransactionCounts {
        m5: counts(5, 0),
        h1: counts(60, 0),
        h6: counts(300, 300),
        h24: counts(600, 600),
    }));
    assert!(
        eval(
            &TransactionVelocityCondition,
            &velocity("6h_vs_24h", "ABOVE", 2.0, 1),
            &ctx
        )
        .await
    );
    assert!(
        !eval(
            &TransactionVelocityCondition,
            &velocity("5m_vs_1h", "ABOVE", 1.1, 1),
            &ctx
        )
        .await
    );
}

#[tokio::test]
async fn transaction_velocity_refuses_missing_zero_or_stale_counts() {
    let cond = velocity("5m_vs_1h", "ABOVE", 2.0, 1);

    let missing = transactions_context(Some(TransactionCounts {
        m5: counts(30, 30),
        h1: None,
        h6: None,
        h24: None,
    }));
    let err = eval_err(&TransactionVelocityCondition, &cond, &missing).await;
    assert!(
        matches!(
            err,
            StrategyError::MissingContextData {
                data: "transaction counts"
            }
        ),
        "unexpected error: {err}"
    );

    let empty_long = transactions_context(Some(TransactionCounts {
        m5: counts(30, 30),
        h1: counts(0, 0),
        h6: None,
        h24: None,
    }));
    let err = eval_err(&TransactionVelocityCondition, &cond, &empty_long).await;
    assert!(
        matches!(
            err,
            StrategyError::UnusableData {
                field: "long window transactions",
                ..
            }
        ),
        "unexpected error: {err}"
    );
    assert!(err.is_data_unavailable());

    // The short window decides staleness: a snapshot older than five minutes no longer
    // counts the trades inside the last five minutes.
    let err = eval_err(&TransactionVelocityCondition, &cond, &snapshot_aged(301)).await;
    assert!(
        matches!(&err, StrategyError::StaleMarketData { window, .. } if window == "5m"),
        "unexpected error: {err}"
    );
}

#[test]
fn transaction_velocity_validation_bounds() {
    let validate = |cond: Condition| TransactionVelocityCondition.validate(&cond);
    assert!(validate(velocity("5m_vs_1h", "ABOVE", 2.0, 30)).is_ok());
    assert!(validate(velocity("5m_vs_1h", "ABOVE", 0.05, 30)).is_err());
    assert!(validate(velocity("5m_vs_1h", "ABOVE", 51.0, 30)).is_err());
    assert!(validate(velocity("5m_vs_1h", "ABOVE", 2.0, 0)).is_err());
    assert!(validate(velocity("1h", "ABOVE", 2.0, 30)).is_err());
    assert!(validate(velocity("5m_vs_1h", "EQUAL", 2.0, 30)).is_err());
}

// ==================== PullbackFromHigh ====================

fn pullback(lookback: i64, min_drop_pct: f64, max_drop_pct: f64) -> Condition {
    condition(
        "PullbackFromHigh",
        &[
            ("lookback", json!(lookback)),
            ("min_drop_pct", json!(min_drop_pct)),
            ("max_drop_pct", json!(max_drop_pct)),
        ],
    )
}

/// One-minute candles ending at the anchor, all closing at 10, with the given highs.
fn highs_series(highs: &[f64]) -> Vec<screenerbot::ohlcvs::Candle> {
    let n = highs.len() as i64;
    highs
        .iter()
        .enumerate()
        .map(|(i, &high)| {
            let ts = anchor_ts() - (n - 1 - i as i64) * MINUTE;
            candle(ts, 10.0, high, 9.0, 10.0, 1.0)
        })
        .collect()
}

fn pullback_context(price: f64) -> EvaluationContext {
    context_with_candles(
        price,
        "1m",
        bundle_with("1m", highs_series(&[11.0, 16.0, 13.0, 12.0, 11.0])),
    )
}

#[tokio::test]
async fn pullback_measures_the_highest_high_not_the_closes() {
    // Every close is 10, so a pullback measured from the closes could never see 12 as a
    // 25% dip; the highest high is 16.
    assert!(
        eval(
            &PullbackFromHighCondition,
            &pullback(5, 25.0, 50.0),
            &pullback_context(12.0)
        )
        .await
    );
}

#[tokio::test]
async fn pullback_band_boundaries_are_inclusive() {
    let cond = pullback(5, 25.0, 50.0);
    assert!(eval(&PullbackFromHighCondition, &cond, &pullback_context(12.0)).await);
    assert!(eval(&PullbackFromHighCondition, &cond, &pullback_context(8.0)).await);
    assert!(
        !eval(
            &PullbackFromHighCondition,
            &pullback(5, 26.0, 50.0),
            &pullback_context(12.0)
        )
        .await
    );
    assert!(!eval(&PullbackFromHighCondition, &cond, &pullback_context(7.9)).await);
}

#[tokio::test]
async fn pullback_is_false_above_the_high_and_reads_only_the_lookback() {
    let cond = pullback(5, 1.0, 50.0);
    assert!(!eval(&PullbackFromHighCondition, &cond, &pullback_context(17.0)).await);
    // With a lookback of 3 the 16 high is outside the window: the high is 13.
    assert!(
        !eval(
            &PullbackFromHighCondition,
            &pullback(3, 25.0, 50.0),
            &pullback_context(12.0)
        )
        .await
    );
}

#[tokio::test]
async fn pullback_refuses_short_or_stale_series() {
    let err = eval_err(
        &PullbackFromHighCondition,
        &pullback(20, 10.0, 30.0),
        &pullback_context(12.0),
    )
    .await;
    assert!(
        matches!(
            err,
            StrategyError::InsufficientCandles {
                indicator: "pullback from high",
                available: 5,
                required: 20,
            }
        ),
        "unexpected error: {err}"
    );

    let mut stale = highs_series(&[11.0, 16.0, 13.0, 12.0, 11.0]);
    for candle in &mut stale {
        candle.timestamp -= 60 * MINUTE;
    }
    let ctx = context_with_candles(12.0, "1m", bundle_with("1m", stale));
    let err = eval_err(&PullbackFromHighCondition, &pullback(5, 10.0, 30.0), &ctx).await;
    assert!(
        matches!(err, StrategyError::StaleCandleData { .. }),
        "unexpected error: {err}"
    );
}

#[test]
fn pullback_validation_bounds() {
    let validate = |cond: Condition| PullbackFromHighCondition.validate(&cond);
    assert!(validate(pullback(20, 10.0, 30.0)).is_ok());
    assert!(validate(pullback(2, 10.0, 30.0)).is_err());
    assert!(validate(pullback(100, 10.0, 30.0)).is_ok());
    assert!(validate(pullback(101, 10.0, 30.0)).is_err());
    assert!(validate(pullback(20, 0.5, 30.0)).is_err());
    assert!(validate(pullback(20, 10.0, 96.0)).is_err());
    // The band needs a minimum below its maximum.
    for (min, max) in [(30.0, 30.0), (40.0, 30.0)] {
        assert!(
            matches!(
                validate(pullback(20, min, max)),
                Err(StrategyError::InvalidConditionValue {
                    field: "maximum drop",
                    ..
                })
            ),
            "{min}..{max}"
        );
    }
}

// ==================== ProfitGiveback ====================

fn giveback(arm_after_profit_pct: f64, giveback_pct: f64) -> Condition {
    condition(
        "ProfitGiveback",
        &[
            ("arm_after_profit_pct", json!(arm_after_profit_pct)),
            ("giveback_pct", json!(giveback_pct)),
        ],
    )
}

#[tokio::test]
async fn profit_giveback_fires_at_the_share_given_back() {
    // Entry 1, best 2 (+100%), now 1.5 (+50%): half the best gain is gone.
    let ctx = peak_context(1.0, Some(2.0), Some(1.5));
    assert!(eval(&ProfitGivebackCondition, &giveback(30.0, 50.0), &ctx).await);
    assert!(!eval(&ProfitGivebackCondition, &giveback(30.0, 51.0), &ctx).await);
    let held = peak_context(1.0, Some(2.0), Some(1.51));
    assert!(!eval(&ProfitGivebackCondition, &giveback(30.0, 50.0), &held).await);
}

#[tokio::test]
async fn profit_giveback_arms_once_the_best_gain_is_large_enough() {
    let ctx = peak_context(1.0, Some(2.0), Some(1.0));
    assert!(eval(&ProfitGivebackCondition, &giveback(100.0, 50.0), &ctx).await);
    assert!(!eval(&ProfitGivebackCondition, &giveback(101.0, 50.0), &ctx).await);
}

#[tokio::test]
async fn profit_giveback_of_everything_is_back_at_the_entry_price() {
    let at_entry = peak_context(1.0, Some(2.0), Some(1.0));
    assert!(eval(&ProfitGivebackCondition, &giveback(30.0, 100.0), &at_entry).await);
    let above = peak_context(1.0, Some(2.0), Some(1.01));
    assert!(!eval(&ProfitGivebackCondition, &giveback(30.0, 100.0), &above).await);
}

#[tokio::test]
async fn profit_giveback_measures_from_the_average_after_an_add() {
    // An add lowered the average to 0.8 and restarted the peak at 1.0 (+25%); the gain
    // before the add belongs to an older average and is not counted.
    let ctx = peak_context(0.8, Some(1.0), Some(0.9));
    assert!(eval(&ProfitGivebackCondition, &giveback(25.0, 50.0), &ctx).await);
    assert!(!eval(&ProfitGivebackCondition, &giveback(26.0, 50.0), &ctx).await);
}

#[tokio::test]
async fn profit_giveback_never_fires_without_a_gain() {
    let below_entry = peak_context(2.0, Some(1.5), Some(1.0));
    assert!(!eval(&ProfitGivebackCondition, &giveback(1.0, 5.0), &below_entry).await);
}

#[tokio::test]
async fn profit_giveback_grows_as_the_price_falls() {
    let cond = giveback(30.0, 40.0);
    let mut fired = false;
    for step in 0..=20 {
        let price = 2.0 - f64::from(step) * 0.05;
        let now = eval(
            &ProfitGivebackCondition,
            &cond,
            &peak_context(1.0, Some(2.0), Some(price)),
        )
        .await;
        assert!(!fired || now, "fired, then stopped at {price}");
        fired = now;
    }
    assert!(fired);
}

#[tokio::test]
async fn profit_giveback_refuses_missing_data() {
    let cond = giveback(30.0, 50.0);
    let err = eval_err(&ProfitGivebackCondition, &cond, &context_bare(Some(1.0))).await;
    assert!(matches!(
        err,
        StrategyError::MissingContextData {
            data: "position data"
        }
    ));
    let err = eval_err(
        &ProfitGivebackCondition,
        &cond,
        &peak_context(1.0, None, Some(1.0)),
    )
    .await;
    assert!(matches!(
        err,
        StrategyError::MissingContextData { data: "peak price" }
    ));
}

#[test]
fn profit_giveback_validation_bounds() {
    let validate = |cond: Condition| ProfitGivebackCondition.validate(&cond);
    assert!(validate(giveback(30.0, 50.0)).is_ok());
    assert!(validate(giveback(0.5, 50.0)).is_err());
    assert!(validate(giveback(10_001.0, 50.0)).is_err());
    assert!(validate(giveback(30.0, 4.0)).is_err());
    assert!(validate(giveback(30.0, 101.0)).is_err());
}

// ==================== TokenAge ====================

fn token_age(comparison: &str, time_value: f64, time_unit: &str) -> Condition {
    condition(
        "TokenAge",
        &[
            ("comparison", json!(comparison)),
            ("time_value", json!(time_value)),
            ("time_unit", json!(time_unit)),
        ],
    )
}

fn created_ago(age: chrono::Duration) -> EvaluationContext {
    let mut ctx = market_context(None);
    ctx.market_data
        .as_mut()
        .expect("market data")
        .token_created_at = Some(common::anchor_time() - age);
    ctx
}

#[tokio::test]
async fn token_age_boundaries_split_older_from_younger() {
    let ctx = created_ago(chrono::Duration::minutes(30));
    assert!(
        eval(
            &TokenAgeCondition,
            &token_age("OLDER_THAN", 30.0, "MINUTES"),
            &ctx
        )
        .await
    );
    assert!(
        !eval(
            &TokenAgeCondition,
            &token_age("OLDER_THAN", 31.0, "MINUTES"),
            &ctx
        )
        .await
    );
    assert!(
        !eval(
            &TokenAgeCondition,
            &token_age("YOUNGER_THAN", 30.0, "MINUTES"),
            &ctx
        )
        .await
    );
    assert!(
        eval(
            &TokenAgeCondition,
            &token_age("YOUNGER_THAN", 31.0, "MINUTES"),
            &ctx
        )
        .await
    );
}

#[tokio::test]
async fn token_age_converts_hours_and_days() {
    let two_hours = created_ago(chrono::Duration::hours(2));
    assert!(
        eval(
            &TokenAgeCondition,
            &token_age("OLDER_THAN", 2.0, "HOURS"),
            &two_hours
        )
        .await
    );
    assert!(
        !eval(
            &TokenAgeCondition,
            &token_age("OLDER_THAN", 121.0, "MINUTES"),
            &two_hours
        )
        .await
    );
    let three_days = created_ago(chrono::Duration::days(3));
    assert!(
        eval(
            &TokenAgeCondition,
            &token_age("OLDER_THAN", 3.0, "DAYS"),
            &three_days
        )
        .await
    );
    assert!(
        eval(
            &TokenAgeCondition,
            &token_age("YOUNGER_THAN", 4.0, "DAYS"),
            &three_days
        )
        .await
    );
    assert!(
        !eval(
            &TokenAgeCondition,
            &token_age("OLDER_THAN", 73.0, "HOURS"),
            &three_days
        )
        .await
    );
}

#[tokio::test]
async fn token_age_refuses_an_unknown_or_future_creation_time() {
    let cond = token_age("OLDER_THAN", 30.0, "MINUTES");
    let err = eval_err(&TokenAgeCondition, &cond, &market_context(None)).await;
    assert!(
        matches!(
            err,
            StrategyError::MissingContextData {
                data: "token creation time"
            }
        ),
        "unexpected error: {err}"
    );
    let err = eval_err(&TokenAgeCondition, &cond, &context_bare(Some(1.0))).await;
    assert!(matches!(
        err,
        StrategyError::MissingContextData {
            data: "market data"
        }
    ));
    let err = eval_err(
        &TokenAgeCondition,
        &cond,
        &created_ago(chrono::Duration::minutes(-5)),
    )
    .await;
    assert!(
        matches!(
            err,
            StrategyError::UnusableData {
                field: "token creation time",
                ..
            }
        ),
        "unexpected error: {err}"
    );
    assert!(err.is_data_unavailable());
}

#[test]
fn token_age_is_an_entry_condition_with_validated_bounds() {
    assert_eq!(TokenAgeCondition.applies_to(), [StrategyType::Entry]);
    let validate = |cond: Condition| TokenAgeCondition.validate(&cond);
    assert!(validate(token_age("OLDER_THAN", 30.0, "DAYS")).is_ok());
    assert!(validate(token_age("OLDER_THAN", 0.0, "MINUTES")).is_err());
    assert!(validate(token_age("OLDER_THAN", 10_001.0, "MINUTES")).is_err());
    assert!(validate(token_age("OLDER_THAN", 30.0, "SECONDS")).is_err());
    assert!(validate(token_age("SAME_AS", 30.0, "MINUTES")).is_err());
}

// ==================== Vwap ====================

fn vwap(lookback: i64, position: &str, distance_pct: f64) -> Condition {
    condition(
        "Vwap",
        &[
            ("lookback", json!(lookback)),
            ("position", json!(position)),
            ("distance_pct", json!(distance_pct)),
        ],
    )
}

/// Five candles with typical prices 9, 9, 9, 18, 18 and volumes 1, 1, 1, 1, 2: a VWAP of
/// 81 / 6 = 13.5. `prefix` candles of a far price and volume come before them.
fn vwap_series(prefix: usize, volume_scale: f64) -> Vec<screenerbot::ohlcvs::Candle> {
    let mut rows: Vec<(f64, f64, f64, f64)> = vec![(1000.0, 1000.0, 1000.0, 1000.0); prefix];
    rows.extend([
        (12.0, 6.0, 9.0, 1.0),
        (12.0, 6.0, 9.0, 1.0),
        (12.0, 6.0, 9.0, 1.0),
        (24.0, 12.0, 18.0, 1.0),
        (24.0, 12.0, 18.0, 2.0),
    ]);
    let n = rows.len() as i64;
    rows.into_iter()
        .enumerate()
        .map(|(i, (high, low, close, volume))| {
            let ts = anchor_ts() - (n - 1 - i as i64) * MINUTE;
            candle(ts, close, high, low, close, volume * volume_scale)
        })
        .collect()
}

fn vwap_context(price: f64, series: Vec<screenerbot::ohlcvs::Candle>) -> EvaluationContext {
    context_with_candles(price, "1m", bundle_with("1m", series))
}

#[tokio::test]
async fn vwap_matches_a_hand_computed_value() {
    let at = vwap_context(13.5, vwap_series(0, 1.0));
    assert!(eval(&VwapCondition, &vwap(5, "ABOVE", 0.0), &at).await);
    assert!(eval(&VwapCondition, &vwap(5, "BELOW", 0.0), &at).await);
    let under = vwap_context(13.4, vwap_series(0, 1.0));
    assert!(!eval(&VwapCondition, &vwap(5, "ABOVE", 0.0), &under).await);
    assert!(eval(&VwapCondition, &vwap(5, "BELOW", 0.0), &under).await);
}

#[tokio::test]
async fn vwap_distance_and_window() {
    // 10% above 13.5 is 14.85.
    assert!(
        eval(
            &VwapCondition,
            &vwap(5, "ABOVE", 10.0),
            &vwap_context(15.0, vwap_series(0, 1.0))
        )
        .await
    );
    assert!(
        !eval(
            &VwapCondition,
            &vwap(5, "ABOVE", 10.0),
            &vwap_context(14.8, vwap_series(0, 1.0))
        )
        .await
    );
    // Candles before the lookback do not move the average.
    assert!(
        eval(
            &VwapCondition,
            &vwap(5, "ABOVE", 0.0),
            &vwap_context(13.5, vwap_series(3, 1.0))
        )
        .await
    );
}

#[tokio::test]
async fn vwap_refuses_no_volume_or_too_few_candles() {
    let err = eval_err(
        &VwapCondition,
        &vwap(5, "ABOVE", 0.0),
        &vwap_context(13.5, vwap_series(0, 0.0)),
    )
    .await;
    assert!(
        matches!(
            err,
            StrategyError::UnusableData {
                field: "total volume",
                ..
            }
        ),
        "unexpected error: {err}"
    );
    assert!(err.is_data_unavailable());
    let err = eval_err(
        &VwapCondition,
        &vwap(6, "ABOVE", 0.0),
        &vwap_context(13.5, vwap_series(0, 1.0)),
    )
    .await;
    assert!(
        matches!(
            err,
            StrategyError::InsufficientCandles {
                indicator: "vwap",
                available: 5,
                required: 6,
            }
        ),
        "unexpected error: {err}"
    );
}

#[test]
fn vwap_validation_bounds() {
    let validate = |cond: Condition| VwapCondition.validate(&cond);
    assert!(validate(vwap(30, "ABOVE", 0.0)).is_ok());
    assert!(validate(vwap(4, "ABOVE", 0.0)).is_err());
    assert!(validate(vwap(100, "ABOVE", 0.0)).is_ok());
    assert!(validate(vwap(101, "ABOVE", 0.0)).is_err());
    assert!(validate(vwap(30, "ABOVE", 101.0)).is_err());
    assert!(validate(vwap(30, "WITHIN", 1.0)).is_err());
}

// ==================== VolatilityRange ====================

fn volatility(period: i64, comparison: &str, threshold_pct: f64) -> Condition {
    condition(
        "VolatilityRange",
        &[
            ("period", json!(period)),
            ("comparison", json!(comparison)),
            ("threshold_pct", json!(threshold_pct)),
        ],
    )
}

/// `count` one-minute candles, each ranging 9 to 11 and closing at 10: a true range of 2.
fn steady_range(count: usize) -> Vec<screenerbot::ohlcvs::Candle> {
    (0..count)
        .map(|i| {
            let ts = anchor_ts() - (count - 1 - i) as i64 * MINUTE;
            candle(ts, 10.0, 11.0, 9.0, 10.0, 1.0)
        })
        .collect()
}

#[tokio::test]
async fn volatility_range_is_the_true_range_as_a_share_of_the_price() {
    // An ATR of 2 against a price of 8 is 25%.
    let ctx = context_with_candles(8.0, "1m", bundle_with("1m", steady_range(15)));
    assert!(
        eval(
            &VolatilityRangeCondition,
            &volatility(14, "ABOVE", 25.0),
            &ctx
        )
        .await
    );
    assert!(
        !eval(
            &VolatilityRangeCondition,
            &volatility(14, "ABOVE", 25.5),
            &ctx
        )
        .await
    );
    assert!(
        eval(
            &VolatilityRangeCondition,
            &volatility(14, "BELOW", 25.0),
            &ctx
        )
        .await
    );
    assert!(
        !eval(
            &VolatilityRangeCondition,
            &volatility(14, "BELOW", 24.5),
            &ctx
        )
        .await
    );
}

#[tokio::test]
async fn volatility_range_counts_a_gap_from_the_previous_close() {
    // Ranges are 2 wide, but each candle opens 4 above the last close: a true range of 5.
    let candles: Vec<_> = (0..4)
        .map(|i| {
            let base = 10.0 + 4.0 * f64::from(i);
            let ts = anchor_ts() - (3 - i64::from(i)) * MINUTE;
            candle(ts, base, base + 1.0, base - 1.0, base, 1.0)
        })
        .collect();
    let ctx = context_with_candles(10.0, "1m", bundle_with("1m", candles));
    assert!(
        eval(
            &VolatilityRangeCondition,
            &volatility(3, "ABOVE", 50.0),
            &ctx
        )
        .await
    );
}

#[tokio::test]
async fn volatility_range_needs_one_more_candle_than_its_period() {
    let ctx = context_with_candles(8.0, "1m", bundle_with("1m", steady_range(14)));
    let err = eval_err(
        &VolatilityRangeCondition,
        &volatility(14, "ABOVE", 5.0),
        &ctx,
    )
    .await;
    assert!(
        matches!(
            err,
            StrategyError::InsufficientCandles {
                indicator: "volatility range",
                available: 14,
                required: 15,
            }
        ),
        "unexpected error: {err}"
    );
}

#[test]
fn volatility_range_validation_bounds() {
    let validate = |cond: Condition| VolatilityRangeCondition.validate(&cond);
    assert!(validate(volatility(14, "ABOVE", 5.0)).is_ok());
    assert!(validate(volatility(1, "ABOVE", 5.0)).is_err());
    assert!(validate(volatility(99, "ABOVE", 5.0)).is_ok());
    assert!(validate(volatility(100, "ABOVE", 5.0)).is_err());
    assert!(validate(volatility(14, "ABOVE", 0.05)).is_err());
    assert!(validate(volatility(14, "ABOVE", 501.0)).is_err());
}

// ==================== BollingerBands ====================

fn bands(period: i64, std_dev: f64, signal: &str, squeeze_width_pct: f64) -> Condition {
    condition(
        "BollingerBands",
        &[
            ("period", json!(period)),
            ("std_dev", json!(std_dev)),
            ("signal", json!(signal)),
            ("squeeze_width_pct", json!(squeeze_width_pct)),
        ],
    )
}

/// Closes with an average of 5 and a population deviation of 2: bands at 1 and 9 for two
/// deviations.
fn bands_context(price: f64) -> EvaluationContext {
    closes_at(price, &[2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0])
}

fn closes_at(price: f64, closes: &[f64]) -> EvaluationContext {
    context_with_candles(
        price,
        "1m",
        bundle_with("1m", candle_series(anchor_ts(), MINUTE, closes)),
    )
}

#[tokio::test]
async fn bollinger_signals_compare_the_price_with_the_bands() {
    let below = bands(8, 2.0, "BELOW_LOWER", 10.0);
    let above = bands(8, 2.0, "ABOVE_UPPER", 10.0);
    assert!(eval(&BollingerBandsCondition, &below, &bands_context(1.0)).await);
    assert!(!eval(&BollingerBandsCondition, &below, &bands_context(1.01)).await);
    assert!(eval(&BollingerBandsCondition, &above, &bands_context(9.0)).await);
    assert!(!eval(&BollingerBandsCondition, &above, &bands_context(8.99)).await);
}

#[tokio::test]
async fn bollinger_squeeze_reads_the_band_width() {
    // Bands 1..9 around 5 are 160% wide; a flat series has no width at all.
    assert!(
        !eval(
            &BollingerBandsCondition,
            &bands(8, 2.0, "SQUEEZE", 100.0),
            &bands_context(5.0)
        )
        .await
    );
    assert!(
        eval(
            &BollingerBandsCondition,
            &bands(8, 1.0, "SQUEEZE", 80.0),
            &bands_context(5.0)
        )
        .await
    );
    let flat = closes_at(5.0, &[5.0; 8]);
    assert!(
        eval(
            &BollingerBandsCondition,
            &bands(8, 2.0, "SQUEEZE", 0.5),
            &flat
        )
        .await
    );
}

#[tokio::test]
async fn bollinger_refuses_a_zero_average_or_a_short_series() {
    let zero = closes_at(1.0, &[0.0; 8]);
    let err = eval_err(
        &BollingerBandsCondition,
        &bands(8, 2.0, "SQUEEZE", 10.0),
        &zero,
    )
    .await;
    assert!(
        matches!(
            err,
            StrategyError::UnusableData {
                field: "moving average",
                ..
            }
        ),
        "unexpected error: {err}"
    );
    let err = eval_err(
        &BollingerBandsCondition,
        &bands(20, 2.0, "BELOW_LOWER", 10.0),
        &bands_context(1.0),
    )
    .await;
    assert!(
        matches!(
            err,
            StrategyError::InsufficientCandles {
                indicator: "bollinger bands",
                available: 8,
                required: 20,
            }
        ),
        "unexpected error: {err}"
    );
}

#[test]
fn bollinger_validation_bounds() {
    let validate = |cond: Condition| BollingerBandsCondition.validate(&cond);
    assert!(validate(bands(20, 2.0, "BELOW_LOWER", 10.0)).is_ok());
    assert!(validate(bands(4, 2.0, "BELOW_LOWER", 10.0)).is_err());
    assert!(validate(bands(100, 2.0, "BELOW_LOWER", 10.0)).is_ok());
    assert!(validate(bands(101, 2.0, "BELOW_LOWER", 10.0)).is_err());
    assert!(validate(bands(20, 0.4, "BELOW_LOWER", 10.0)).is_err());
    assert!(validate(bands(20, 5.1, "BELOW_LOWER", 10.0)).is_err());
    assert!(validate(bands(20, 2.0, "INSIDE", 10.0)).is_err());
    assert!(validate(bands(20, 2.0, "SQUEEZE", 0.4)).is_err());
}

// ==================== LiquidityToSize ====================

fn liquidity_to_size(comparison: &str, multiple: f64) -> Condition {
    condition(
        "LiquidityToSize",
        &[
            ("comparison", json!(comparison)),
            ("multiple", json!(multiple)),
        ],
    )
}

/// An entry context: 100 in the pool, a configured trade size of 2.
fn entry_sizing(liquidity: Option<f64>, size: Option<f64>) -> EvaluationContext {
    let mut ctx = market_context(liquidity);
    ctx.market_data
        .as_mut()
        .expect("market data")
        .reference_size_native = size;
    ctx
}

/// An exit context: 100 in the pool, a position whose remaining tokens are worth `value`.
fn exit_sizing(value: Option<f64>) -> EvaluationContext {
    let mut ctx = market_context(Some(100.0));
    ctx.position_data = position_context(1.0).position_data;
    ctx.position_data
        .as_mut()
        .expect("position")
        .remaining_value_native = value;
    ctx
}

#[tokio::test]
async fn liquidity_to_size_on_entry_uses_the_trade_size() {
    let ctx = entry_sizing(Some(100.0), Some(2.0));
    assert!(
        eval(
            &LiquidityToSizeCondition,
            &liquidity_to_size("AT_LEAST", 50.0),
            &ctx
        )
        .await
    );
    assert!(
        !eval(
            &LiquidityToSizeCondition,
            &liquidity_to_size("AT_LEAST", 51.0),
            &ctx
        )
        .await
    );
    assert!(
        eval(
            &LiquidityToSizeCondition,
            &liquidity_to_size("BELOW", 51.0),
            &ctx
        )
        .await
    );
    assert!(
        !eval(
            &LiquidityToSizeCondition,
            &liquidity_to_size("BELOW", 50.0),
            &ctx
        )
        .await
    );
}

#[tokio::test]
async fn liquidity_to_size_on_exit_uses_the_value_still_held() {
    // After a partial sale the position is worth 4: a multiple of 25, whatever trade size
    // an entry would spend.
    let mut ctx = exit_sizing(Some(4.0));
    ctx.market_data
        .as_mut()
        .expect("market data")
        .reference_size_native = Some(1.0);
    assert!(
        eval(
            &LiquidityToSizeCondition,
            &liquidity_to_size("AT_LEAST", 25.0),
            &ctx
        )
        .await
    );
    assert!(
        !eval(
            &LiquidityToSizeCondition,
            &liquidity_to_size("AT_LEAST", 26.0),
            &ctx
        )
        .await
    );
}

#[tokio::test]
async fn liquidity_to_size_refuses_unknown_inputs() {
    let cond = liquidity_to_size("AT_LEAST", 50.0);
    for (ctx, data) in [
        (entry_sizing(None, Some(2.0)), "liquidity data"),
        (entry_sizing(Some(100.0), None), "trade size"),
        (exit_sizing(None), "remaining position value"),
        (context_bare(Some(1.0)), "market data"),
    ] {
        let err = eval_err(&LiquidityToSizeCondition, &cond, &ctx).await;
        assert!(
            matches!(&err, StrategyError::MissingContextData { data: missing } if *missing == data),
            "{data}: unexpected error: {err}"
        );
    }
    let err = eval_err(
        &LiquidityToSizeCondition,
        &cond,
        &entry_sizing(Some(100.0), Some(0.0)),
    )
    .await;
    assert!(
        matches!(
            err,
            StrategyError::UnusableData {
                field: "trade size",
                ..
            }
        ),
        "unexpected error: {err}"
    );
}

#[test]
fn liquidity_to_size_validation_bounds() {
    let validate = |cond: Condition| LiquidityToSizeCondition.validate(&cond);
    assert!(validate(liquidity_to_size("AT_LEAST", 50.0)).is_ok());
    assert!(validate(liquidity_to_size("AT_LEAST", 0.5)).is_err());
    assert!(validate(liquidity_to_size("AT_LEAST", 10_001.0)).is_err());
    assert!(validate(liquidity_to_size("ABOVE", 50.0)).is_err());
}

// ==================== PositionAdds ====================

fn adds(subject: &str, comparison: &str, count: i64) -> Condition {
    condition(
        "PositionAdds",
        &[
            ("subject", json!(subject)),
            ("comparison", json!(comparison)),
            ("count", json!(count)),
        ],
    )
}

fn booked(dca_count: u32, partial_exit_count: u32) -> EvaluationContext {
    let mut ctx = position_context(1.0);
    let position = ctx.position_data.as_mut().expect("position");
    position.dca_count = dca_count;
    position.partial_exit_count = partial_exit_count;
    ctx
}

#[tokio::test]
async fn position_adds_counts_each_subject_inclusively() {
    let ctx = booked(2, 1);
    assert!(eval(&PositionAddsCondition, &adds("ADDS", "AT_LEAST", 2), &ctx).await);
    assert!(!eval(&PositionAddsCondition, &adds("ADDS", "AT_LEAST", 3), &ctx).await);
    assert!(eval(&PositionAddsCondition, &adds("ADDS", "AT_MOST", 2), &ctx).await);
    assert!(!eval(&PositionAddsCondition, &adds("ADDS", "AT_MOST", 1), &ctx).await);
    assert!(
        eval(
            &PositionAddsCondition,
            &adds("PARTIAL_SELLS", "AT_LEAST", 1),
            &ctx
        )
        .await
    );
    assert!(
        !eval(
            &PositionAddsCondition,
            &adds("PARTIAL_SELLS", "AT_LEAST", 2),
            &ctx
        )
        .await
    );
    assert!(
        eval(
            &PositionAddsCondition,
            &adds("ADDS", "AT_MOST", 0),
            &booked(0, 3)
        )
        .await
    );
}

#[tokio::test]
async fn position_adds_without_a_position_errors() {
    let err = eval_err(
        &PositionAddsCondition,
        &adds("ADDS", "AT_LEAST", 1),
        &context_bare(Some(1.0)),
    )
    .await;
    assert!(matches!(
        err,
        StrategyError::MissingContextData {
            data: "position data"
        }
    ));
}

#[test]
fn position_adds_validation_bounds() {
    let validate = |cond: Condition| PositionAddsCondition.validate(&cond);
    assert!(validate(adds("ADDS", "AT_LEAST", 0)).is_ok());
    assert!(validate(adds("ADDS", "AT_LEAST", 21)).is_err());
    assert!(validate(adds("FILLS", "AT_LEAST", 1)).is_err());
    assert!(validate(adds("ADDS", "ABOVE", 1)).is_err());
}

// ==================== TimeOfDay ====================

fn hours(start_hour: f64, end_hour: f64) -> Condition {
    condition(
        "TimeOfDay",
        &[
            ("start_hour", json!(start_hour)),
            ("end_hour", json!(end_hour)),
        ],
    )
}

/// A context evaluated `hour` hours and 30 minutes after a UTC midnight.
fn at_hour(hour: i64) -> EvaluationContext {
    let mut ctx = context_bare(Some(1.0));
    ctx.evaluated_at =
        common::anchor_time() + chrono::Duration::hours(hour) + chrono::Duration::minutes(30);
    ctx
}

#[tokio::test]
async fn time_of_day_window_includes_its_start_and_excludes_its_end() {
    let cond = hours(13.0, 22.0);
    for (hour, inside) in [(12, false), (13, true), (21, true), (22, false), (0, false)] {
        assert_eq!(
            eval(&TimeOfDayCondition, &cond, &at_hour(hour)).await,
            inside,
            "hour {hour}"
        );
    }
}

#[tokio::test]
async fn time_of_day_window_wraps_past_midnight() {
    let cond = hours(22.0, 2.0);
    for (hour, inside) in [
        (21, false),
        (22, true),
        (23, true),
        (0, true),
        (1, true),
        (2, false),
    ] {
        assert_eq!(
            eval(&TimeOfDayCondition, &cond, &at_hour(hour)).await,
            inside,
            "hour {hour}"
        );
    }
    let whole_day = hours(0.0, 24.0);
    for hour in 0..24 {
        assert!(
            eval(&TimeOfDayCondition, &whole_day, &at_hour(hour)).await,
            "hour {hour}"
        );
    }
}

#[test]
fn time_of_day_validation_bounds() {
    let validate = |cond: Condition| TimeOfDayCondition.validate(&cond);
    assert!(validate(hours(13.0, 22.0)).is_ok());
    assert!(matches!(
        validate(hours(13.0, 13.0)),
        Err(StrategyError::InvalidConditionValue {
            field: "end hour",
            ..
        })
    ));
    assert!(validate(hours(13.5, 22.0)).is_err());
    assert!(validate(hours(24.0, 2.0)).is_err());
    assert!(validate(hours(13.0, 0.0)).is_err());
    assert!(validate(hours(13.0, 25.0)).is_err());
}

// ==================== LiquidityChangeSinceEntry ====================

fn liquidity_change(comparison: &str, change_pct: f64) -> Condition {
    condition(
        "LiquidityChangeSinceEntry",
        &[
            ("comparison", json!(comparison)),
            ("change_pct", json!(change_pct)),
        ],
    )
}

const ENTRY_POOL: &str = "entry-pool";

/// A position that entered with `entry` in `ENTRY_POOL`, whose live price now comes from
/// the same pool, holding `now` real reserves.
fn liquidity_since(entry: Option<f64>, now: Option<f64>) -> EvaluationContext {
    let mut ctx = market_context(now);
    ctx.market_data.as_mut().expect("market").liquidity_pool = Some(LiquidityPool {
        address: ENTRY_POOL.to_owned(),
        virtual_reserves: Some(false),
    });
    ctx.position_data = position_context(1.0).position_data;
    let position = ctx.position_data.as_mut().expect("position");
    position.entry_liquidity_native = entry;
    position.entry_liquidity_pool = entry.map(|_| ENTRY_POOL.to_owned());
    ctx
}

fn live_pool(ctx: &mut EvaluationContext) -> &mut LiquidityPool {
    ctx.market_data
        .as_mut()
        .and_then(|market| market.liquidity_pool.as_mut())
        .expect("live pool")
}

#[tokio::test]
async fn liquidity_change_measures_both_directions_from_the_entry() {
    let drained = liquidity_since(Some(64.0), Some(48.0));
    let cond = |comparison, pct| liquidity_change(comparison, pct);
    assert!(
        eval(
            &LiquidityChangeSinceEntryCondition,
            &cond("DROPPED_BY", 25.0),
            &drained
        )
        .await
    );
    assert!(
        !eval(
            &LiquidityChangeSinceEntryCondition,
            &cond("DROPPED_BY", 26.0),
            &drained
        )
        .await
    );
    assert!(
        !eval(
            &LiquidityChangeSinceEntryCondition,
            &cond("ROSE_BY", 25.0),
            &drained
        )
        .await
    );
    let deepened = liquidity_since(Some(64.0), Some(80.0));
    assert!(
        eval(
            &LiquidityChangeSinceEntryCondition,
            &cond("ROSE_BY", 25.0),
            &deepened
        )
        .await
    );
    assert!(
        !eval(
            &LiquidityChangeSinceEntryCondition,
            &cond("DROPPED_BY", 1.0),
            &deepened
        )
        .await
    );
}

#[tokio::test]
async fn liquidity_change_of_a_row_without_an_entry_reading_is_unknown() {
    // Rows opened before the reading was recorded, imported or adopted.
    let cond = liquidity_change("DROPPED_BY", 30.0);
    let err = eval_err(
        &LiquidityChangeSinceEntryCondition,
        &cond,
        &liquidity_since(None, Some(48.0)),
    )
    .await;
    assert!(
        matches!(
            err,
            StrategyError::MissingContextData {
                data: "entry liquidity"
            }
        ),
        "unexpected error: {err}"
    );
    assert!(err.is_data_unavailable());
    let err = eval_err(
        &LiquidityChangeSinceEntryCondition,
        &cond,
        &liquidity_since(Some(64.0), None),
    )
    .await;
    assert!(matches!(
        err,
        StrategyError::MissingContextData {
            data: "liquidity data"
        }
    ));
    let err = eval_err(
        &LiquidityChangeSinceEntryCondition,
        &cond,
        &market_context(Some(48.0)),
    )
    .await;
    assert!(matches!(
        err,
        StrategyError::MissingContextData {
            data: "position data"
        }
    ));
}

#[test]
fn liquidity_change_validation_bounds() {
    let validate = |cond: Condition| LiquidityChangeSinceEntryCondition.validate(&cond);
    assert!(validate(liquidity_change("DROPPED_BY", 30.0)).is_ok());
    assert!(validate(liquidity_change("DROPPED_BY", 0.5)).is_err());
    assert!(validate(liquidity_change("DROPPED_BY", 101.0)).is_err());
    assert!(validate(liquidity_change("FELL", 30.0)).is_err());
}

#[test]
fn position_conditions_apply_to_exit_strategies_only() {
    for evaluator in [
        &ProfitGivebackCondition as &dyn ConditionEvaluator,
        &PositionAddsCondition,
        &LiquidityChangeSinceEntryCondition,
    ] {
        assert_eq!(
            evaluator.applies_to(),
            [StrategyType::Exit],
            "{}",
            evaluator.condition_type()
        );
    }
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
    assert_eq!(RsiCondition.condition_type(), "Rsi");
    assert_eq!(MaCrossCondition.condition_type(), "MaCross");
    assert_eq!(PositionProfitCondition.condition_type(), "PositionProfit");
    assert_eq!(
        DrawdownFromPeakCondition.condition_type(),
        "DrawdownFromPeak"
    );
    assert_eq!(BuyPressureCondition.condition_type(), "BuyPressure");
    assert_eq!(
        TransactionVelocityCondition.condition_type(),
        "TransactionVelocity"
    );
    assert_eq!(
        PullbackFromHighCondition.condition_type(),
        "PullbackFromHigh"
    );
    assert_eq!(ProfitGivebackCondition.condition_type(), "ProfitGiveback");
    assert_eq!(TokenAgeCondition.condition_type(), "TokenAge");
    assert_eq!(VwapCondition.condition_type(), "Vwap");
    assert_eq!(VolatilityRangeCondition.condition_type(), "VolatilityRange");
    assert_eq!(BollingerBandsCondition.condition_type(), "BollingerBands");
    assert_eq!(LiquidityToSizeCondition.condition_type(), "LiquidityToSize");
    assert_eq!(PositionAddsCondition.condition_type(), "PositionAdds");
    assert_eq!(TimeOfDayCondition.condition_type(), "TimeOfDay");
    assert_eq!(
        LiquidityChangeSinceEntryCondition.condition_type(),
        "LiquidityChangeSinceEntry"
    );
}

/// Every candle-count parameter of every registered condition, with the candles its
/// evaluation reads beyond the parameter's own value at the other parameters' maxima.
const CANDLE_WINDOW_PARAMETERS: &[(&str, &str, usize)] = &[
    ("BollingerBands", "period", 0),
    ("ConsecutiveCandles", "count", 0),
    ("MaCross", "fast_period", 0),
    ("MaCross", "slow_period", 20),
    ("MaCross", "within_candles", 200),
    ("PriceBreakout", "lookback", 1),
    ("PriceToMA", "period", 0),
    ("PullbackFromHigh", "lookback", 0),
    ("Rsi", "period", 1),
    ("VolatilityRange", "period", 1),
    ("VolumeSpike", "lookback", 1),
    ("Vwap", "lookback", 0),
];

/// Windows that can still ask for more candles than the strategy bundle holds. Stored
/// strategies may hold those values, so each needs its own decision; the list only
/// shrinks.
const CANDLE_WINDOWS_ABOVE_THE_BUNDLE: &[(&str, &str)] = &[
    ("MaCross", "slow_period"),
    ("MaCross", "within_candles"),
    ("PriceBreakout", "lookback"),
    ("PriceToMA", "period"),
    ("Rsi", "period"),
    ("VolumeSpike", "lookback"),
];

#[test]
fn no_condition_window_asks_for_more_candles_than_the_bundle_holds() {
    let schemas = screenerbot::strategies::conditions::ConditionRegistry::new().get_all_schemas();
    let bundle = screenerbot::ohlcvs::BUNDLE_CANDLE_COUNT;
    let mut listed: Vec<(String, String)> = Vec::new();
    for (condition, schema) in schemas.as_object().expect("schemas are an object") {
        for (parameter, spec) in schema["parameters"]
            .as_object()
            .expect("parameters are an object")
        {
            if spec["unit"] == "candles" {
                listed.push((condition.clone(), parameter.clone()));
            }
        }
    }
    listed.sort();
    let mut known: Vec<(String, String)> = CANDLE_WINDOW_PARAMETERS
        .iter()
        .map(|(condition, parameter, _)| ((*condition).to_owned(), (*parameter).to_owned()))
        .collect();
    known.sort();
    assert_eq!(
        listed, known,
        "every candle-count parameter is listed with the candles it reads"
    );

    for &(condition, parameter, extra) in CANDLE_WINDOW_PARAMETERS {
        let max = schemas[condition]["parameters"][parameter]["max"]
            .as_u64()
            .unwrap_or_else(|| panic!("{condition}.{parameter} has a whole maximum"))
            as usize;
        let fits = max + extra <= bundle;
        let allowed = CANDLE_WINDOWS_ABOVE_THE_BUNDLE.contains(&(condition, parameter));
        assert!(
            fits || allowed,
            "{condition}.{parameter} reads up to {} candles; the bundle holds {bundle}",
            max + extra
        );
        assert!(
            !(fits && allowed),
            "{condition}.{parameter} now fits the bundle: remove it from the allowlist"
        );
    }
}
