// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Strategy condition evaluators for rule-tree nodes (price, volume, candles, market, position, clock).
mod bollinger_bands;
mod buy_pressure;
mod candle_size;
mod catalog;
mod consecutive_candles;
mod drawdown_from_peak;
mod indicators;
mod liquidity_change_since_entry;
mod liquidity_level;
mod liquidity_to_size;
mod ma_cross;
mod position_adds;
mod position_holding_time;
mod position_profit;
mod price_breakout;
mod price_change_percent;
mod price_to_ma;
mod profit_giveback;
mod pullback_from_high;
mod rsi;
mod time_of_day;
mod token_age;
mod transaction_velocity;
mod volatility_range;
mod volume_spike;
mod vwap;

pub use bollinger_bands::BollingerBandsCondition;
pub use buy_pressure::BuyPressureCondition;
pub use candle_size::CandleSizeCondition;
pub use consecutive_candles::ConsecutiveCandlesCondition;
pub use drawdown_from_peak::DrawdownFromPeakCondition;
pub use liquidity_change_since_entry::LiquidityChangeSinceEntryCondition;
pub use liquidity_level::LiquidityLevelCondition;
pub use liquidity_to_size::LiquidityToSizeCondition;
pub use ma_cross::MaCrossCondition;
pub use position_adds::PositionAddsCondition;
pub use position_holding_time::PositionHoldingTimeCondition;
pub use position_profit::PositionProfitCondition;
pub use price_breakout::PriceBreakoutCondition;
pub use price_change_percent::PriceChangePercentCondition;
pub use price_to_ma::PriceToMaCondition;
pub use profit_giveback::ProfitGivebackCondition;
pub use pullback_from_high::PullbackFromHighCondition;
pub use rsi::RsiCondition;
pub use time_of_day::TimeOfDayCondition;
pub use token_age::TokenAgeCondition;
pub use transaction_velocity::TransactionVelocityCondition;
pub use volatility_range::VolatilityRangeCondition;
pub use volume_spike::VolumeSpikeCondition;
pub use vwap::VwapCondition;

use crate::ohlcvs::{Candle, Timeframe};
use crate::strategies::types::{
    Condition, ConditionData, EvaluationContext, RuleTree, StrategyType, TransactionCounts,
    WindowCounts,
};
use crate::strategies::{Error, Result};
use async_trait::async_trait;
use serde_json::json;
use std::collections::HashMap;

/// How far behind the evaluation instant a series may fall before it is refused.
///
/// A rule that compares the LIVE pool price against candle history is only meaningful
/// while the two describe the same moment. Candles are written only when a trade
/// happens — no-trade candles are never stored — so a token that stopped trading keeps
/// serving its last candles indefinitely while `current_price` is from this second.
/// Without this bound, a move that finished hours ago is measured as if it had just
/// happened and momentum rules fire on it.
///
/// Three buckets tolerates the ordinary quiet gap (a minute or two of no trades on a 1m
/// series) and refuses a series that has genuinely stopped tracking the price.
const MAX_SERIES_STALENESS_BUCKETS: i64 = 3;

/// The most candles a condition's window may span: the strategy bundle holds
/// [`BUNDLE_CANDLE_COUNT`](crate::ohlcvs::BUNDLE_CANDLE_COUNT) candles per timeframe, so a
/// wider window could never fill and its leaf would stay unknown.
pub const CANDLE_WINDOW_MAX: usize = crate::ohlcvs::BUNDLE_CANDLE_COUNT;

/// Helper to extract candles for a specific timeframe from TimeframeBundle
///
/// Supports per-condition timeframe selection with fallback to the strategy timeframe,
/// and refuses data that cannot answer the question being asked: a missing bundle, an
/// unknown timeframe, an empty series, or a series too far behind
/// [`EvaluationContext::evaluated_at`] to stand next to a live price. Every refusal is
/// an `Err` naming what is wrong — a condition that returned `false` here would be
/// indistinguishable from one that simply did not fire.
pub fn get_candles_for_timeframe(
    context: &EvaluationContext,
    condition_timeframe: Option<&str>,
) -> Result<Vec<Candle>> {
    // Check if bundle exists
    let bundle = context
        .timeframe_bundle
        .as_ref()
        .ok_or_else(|| Error::MissingContextData { data: "OHLCV data" })?;

    // Use condition's timeframe if provided, otherwise fallback to strategy timeframe
    let timeframe = condition_timeframe.unwrap_or(&context.strategy_timeframe);

    // Validate timeframe value against the one enum that defines them
    let bucket_seconds = Timeframe::from_str(timeframe)
        .ok_or_else(|| Error::InvalidConditionValue {
            field: "timeframe",
            value: timeframe.to_owned(),
        })?
        .to_seconds();

    // Check if timeframe exists in bundle
    let candles = bundle
        .get_timeframe(timeframe)
        .ok_or_else(|| Error::NoCandleData {
            timeframe: timeframe.to_owned(),
        })?;

    // Check if timeframe has data
    if candles.is_empty() {
        return Err(Error::NoCandleData {
            timeframe: timeframe.to_owned(),
        });
    }

    // Check the series still describes the present
    let newest = candles
        .iter()
        .map(|c| c.timestamp)
        .max()
        .unwrap_or_default();
    let age_seconds = context.evaluated_at.timestamp() - newest;
    let max_age_seconds = bucket_seconds * MAX_SERIES_STALENESS_BUCKETS;
    if age_seconds > max_age_seconds {
        return Err(Error::StaleCandleData {
            timeframe: timeframe.to_owned(),
            age_seconds,
            max_age_seconds,
        });
    }

    Ok(candles.clone())
}

/// Transaction counts of one market window (`5m`, `1h`, `6h` or `24h`), refusing a
/// snapshot older than the window it describes.
///
/// The counts are a rolling window as of the moment the token snapshot was fetched. Once
/// the snapshot is older than the window, none of the trades it counts need still be
/// inside that window, so the figure describes a past market rather than this one. Age
/// is measured against [`EvaluationContext::evaluated_at`], never the wall clock.
pub fn market_window(context: &EvaluationContext, window: &str) -> Result<WindowCounts> {
    let (select, max_age_seconds): (fn(&TransactionCounts) -> Option<WindowCounts>, i64) =
        match window {
            "5m" => (|counts| counts.m5, 300),
            "1h" => (|counts| counts.h1, 3_600),
            "6h" => (|counts| counts.h6, 21_600),
            "24h" => (|counts| counts.h24, 86_400),
            _ => {
                return Err(Error::InvalidConditionValue {
                    field: "window",
                    value: window.to_owned(),
                })
            }
        };

    let missing = || Error::MissingContextData {
        data: "transaction counts",
    };
    let market = context.market_data.as_ref().ok_or_else(missing)?;
    let counts = market
        .transactions
        .as_ref()
        .and_then(select)
        .ok_or_else(missing)?;
    let snapshot_at = market.snapshot_at.ok_or(Error::MissingContextData {
        data: "market data",
    })?;

    let age_seconds = (context.evaluated_at - snapshot_at).num_seconds();
    if age_seconds > max_age_seconds {
        return Err(Error::StaleMarketData {
            window: window.to_owned(),
            age_seconds,
            max_age_seconds,
        });
    }

    Ok(counts)
}

/// The live price the whole context is built around, refusing a value no comparison can
/// use.
///
/// `NaN` is the dangerous one: every `>=`/`<=` against it is `false`, so a rule fed a
/// `NaN` price reports "no signal" forever instead of failing. An entry that never fires
/// is money not spent, but an EXIT that never fires is a position that cannot be closed
/// by strategy, so this is an `Err` and never a `false`. A price that is present but
/// unusable is data, not configuration: inside a rule tree it leaves the condition
/// unknown.
pub fn get_current_price(context: &EvaluationContext) -> Result<f64> {
    let price = context
        .current_price
        .ok_or_else(|| Error::MissingContextData {
            data: "current price",
        })?;

    if !price.is_finite() || price <= 0.0 {
        return Err(Error::UnusableData {
            field: "current price",
            value: price.to_string(),
        });
    }

    Ok(price)
}

/// Guard a value that is about to become the denominator of a percentage.
///
/// A zero or non-finite basis turns `(current - basis) / basis` into `+inf`, which
/// clears EVERY upward threshold a user can configure — the validator caps the
/// threshold at 1000%, and infinity is above that too. Refusing the basis is the only
/// way such a series produces no signal rather than a guaranteed one. The refusal names
/// the data, not the configuration, so a rule tree reads the condition as unknown.
pub fn usable_basis(label: &'static str, value: f64) -> Result<f64> {
    if !value.is_finite() || value <= 0.0 {
        return Err(Error::UnusableData {
            field: label,
            value: value.to_string(),
        });
    }
    Ok(value)
}

/// Trait for condition evaluation
#[async_trait]
pub trait ConditionEvaluator: Send + Sync {
    /// Unique identifier for this condition type
    fn condition_type(&self) -> &'static str;

    /// Evaluate the condition against the context
    async fn evaluate(&self, condition: &Condition, context: &EvaluationContext) -> Result<bool>;

    /// Validate condition parameters
    fn validate(&self, condition: &Condition) -> Result<()>;

    /// Strategy types this condition can be used in. A condition that reads position data
    /// applies to exit strategies only: in an entry strategy it could never be evaluated.
    fn applies_to(&self) -> &'static [StrategyType] {
        &[StrategyType::Entry, StrategyType::Exit]
    }

    /// The data this condition reads, published in its schema as `reads`.
    fn reads(&self) -> ConditionData;

    /// Structure of the parameters for the UI. Carries no display text: the registry adds
    /// the localization keys (see `catalog`).
    fn parameter_schema(&self) -> serde_json::Value;
}

/// Registry for all condition evaluators
pub struct ConditionRegistry {
    evaluators: HashMap<String, Box<dyn ConditionEvaluator>>,
}

impl ConditionRegistry {
    /// Create a new registry with all built-in conditions
    pub fn new() -> Self {
        let mut registry = Self {
            evaluators: HashMap::new(),
        };

        // Register all built-in conditions
        registry.register(Box::new(PriceChangePercentCondition));
        registry.register(Box::new(PriceToMaCondition));
        registry.register(Box::new(ConsecutiveCandlesCondition));
        registry.register(Box::new(CandleSizeCondition));
        registry.register(Box::new(PriceBreakoutCondition));
        registry.register(Box::new(VolumeSpikeCondition));
        registry.register(Box::new(LiquidityLevelCondition));
        registry.register(Box::new(PositionHoldingTimeCondition));
        registry.register(Box::new(RsiCondition));
        registry.register(Box::new(MaCrossCondition));
        registry.register(Box::new(PositionProfitCondition));
        registry.register(Box::new(DrawdownFromPeakCondition));
        registry.register(Box::new(BuyPressureCondition));
        registry.register(Box::new(TransactionVelocityCondition));
        registry.register(Box::new(PullbackFromHighCondition));
        registry.register(Box::new(ProfitGivebackCondition));
        registry.register(Box::new(TokenAgeCondition));
        registry.register(Box::new(VwapCondition));
        registry.register(Box::new(VolatilityRangeCondition));
        registry.register(Box::new(BollingerBandsCondition));
        registry.register(Box::new(LiquidityToSizeCondition));
        registry.register(Box::new(PositionAddsCondition));
        registry.register(Box::new(TimeOfDayCondition));
        registry.register(Box::new(LiquidityChangeSinceEntryCondition));

        registry
    }

    /// Register a condition evaluator
    pub fn register(&mut self, evaluator: Box<dyn ConditionEvaluator>) {
        let condition_type = evaluator.condition_type().to_string();
        self.evaluators.insert(condition_type, evaluator);
    }

    /// Get an evaluator by condition type
    pub fn get(&self, condition_type: &str) -> Option<&Box<dyn ConditionEvaluator>> {
        self.evaluators.get(condition_type)
    }

    /// List all registered condition types
    pub fn list_types(&self) -> Vec<String> {
        self.evaluators.keys().cloned().collect()
    }

    /// The candle timeframes the switched-on rules of `rules` read, each once in rule order:
    /// a candle condition's own `timeframe`, else `strategy_timeframe`, as
    /// [`get_candles_for_timeframe`] resolves it. A rule of an unknown condition type or an
    /// invalid timeframe adds none; its evaluation reports that itself.
    pub fn candle_timeframes(&self, rules: &RuleTree, strategy_timeframe: &str) -> Vec<Timeframe> {
        let mut timeframes = Vec::new();
        self.collect_candle_timeframes(rules, strategy_timeframe, &mut timeframes);
        timeframes
    }

    fn collect_candle_timeframes(
        &self,
        node: &RuleTree,
        strategy_timeframe: &str,
        timeframes: &mut Vec<Timeframe>,
    ) {
        if node.disabled {
            return;
        }
        if let Some(condition) = &node.condition {
            let reads_candles = self
                .get(&condition.condition_type)
                .is_some_and(|evaluator| evaluator.reads() == ConditionData::Candles);
            let timeframe = get_param_string_optional(condition, "timeframe");
            let timeframe = Timeframe::from_str(timeframe.as_deref().unwrap_or(strategy_timeframe));
            if let Some(timeframe) = timeframe.filter(|_| reads_candles) {
                if !timeframes.contains(&timeframe) {
                    timeframes.push(timeframe);
                }
            }
        }
        for child in node.conditions.iter().flatten() {
            self.collect_candle_timeframes(child, strategy_timeframe, timeframes);
        }
    }

    /// Get all parameter schemas for UI
    pub fn get_all_schemas(&self) -> serde_json::Value {
        let mut schemas = serde_json::Map::new();
        for (name, evaluator) in &self.evaluators {
            let mut schema = evaluator.parameter_schema();
            schema["applies_to"] = json!(evaluator.applies_to());
            schema["reads"] = json!(evaluator.reads());
            catalog::attach_text_keys(&mut schema);
            schemas.insert(name.clone(), schema);
        }
        serde_json::Value::Object(schemas)
    }
}

impl Default for ConditionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper function to get parameter value with type checking
pub fn get_param_f64(condition: &Condition, param_name: &'static str) -> Result<f64> {
    let param = condition
        .parameters
        .get(param_name)
        .ok_or(Error::MissingConditionParameter { field: param_name })?;

    param.value.as_f64().ok_or(Error::ConditionParameterType {
        field: param_name,
        expected: "a number",
    })
}

/// Read a numeric parameter and refuse a value outside `min..=max`, naming `field` in the error.
pub fn get_param_f64_in_range(
    condition: &Condition,
    param_name: &'static str,
    field: &'static str,
    min: f64,
    max: f64,
) -> Result<f64> {
    let value = get_param_f64(condition, param_name)?;
    if !(min..=max).contains(&value) {
        return Err(Error::InvalidConditionValue {
            field,
            value: value.to_string(),
        });
    }
    Ok(value)
}

/// Read a string parameter and refuse a value outside `allowed`, naming `field` in the error.
pub fn get_param_choice(
    condition: &Condition,
    param_name: &'static str,
    field: &'static str,
    allowed: &[&str],
) -> Result<String> {
    let value = get_param_string(condition, param_name)?;
    if !allowed.contains(&value.as_str()) {
        return Err(Error::InvalidConditionValue { field, value });
    }
    Ok(value)
}

/// Helper function to get parameter value as string
pub fn get_param_string(condition: &Condition, param_name: &'static str) -> Result<String> {
    let param = condition
        .parameters
        .get(param_name)
        .ok_or(Error::MissingConditionParameter { field: param_name })?;

    param
        .value
        .as_str()
        .map(str::to_string)
        .ok_or(Error::ConditionParameterType {
            field: param_name,
            expected: "a string",
        })
}

/// Helper function to get parameter value as bool
pub fn get_param_bool(condition: &Condition, param_name: &'static str) -> Result<bool> {
    let param = condition
        .parameters
        .get(param_name)
        .ok_or(Error::MissingConditionParameter { field: param_name })?;

    param.value.as_bool().ok_or(Error::ConditionParameterType {
        field: param_name,
        expected: "a boolean",
    })
}

/// Helper function to get optional parameter value as string
pub fn get_param_string_optional(condition: &Condition, param_name: &str) -> Option<String> {
    condition
        .parameters
        .get(param_name)
        .and_then(|param| param.value.as_str())
        .map(str::to_string)
}

/// Helper function to validate optional timeframe parameter
pub fn validate_timeframe_param(condition: &Condition) -> Result<()> {
    if let Some(timeframe) = get_param_string_optional(condition, "timeframe") {
        let valid_timeframes = ["1m", "5m", "15m", "1h", "4h", "12h", "1d"];
        if !valid_timeframes.contains(&timeframe.as_str()) {
            return Err(Error::InvalidConditionValue {
                field: "timeframe",
                value: timeframe,
            });
        }
    }
    Ok(())
}
