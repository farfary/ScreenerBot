mod candle_size;
mod consecutive_candles;
mod liquidity_level;
mod position_holding_time;
mod price_breakout;
mod price_change_percent;
mod price_to_ma;
mod volume_spike;

pub use candle_size::CandleSizeCondition;
pub use consecutive_candles::ConsecutiveCandlesCondition;
pub use liquidity_level::LiquidityLevelCondition;
pub use position_holding_time::PositionHoldingTimeCondition;
pub use price_breakout::PriceBreakoutCondition;
pub use price_change_percent::PriceChangePercentCondition;
pub use price_to_ma::PriceToMaCondition;
pub use volume_spike::VolumeSpikeCondition;

use crate::ohlcvs::Candle;
use crate::strategies::types::{Condition, EvaluationContext};
use async_trait::async_trait;

/// Helper to extract candles from TimeframeBundle using strategy's configured timeframe
/// Returns detailed error messages for debugging
pub fn get_candles_from_context(context: &EvaluationContext) -> Result<Vec<Candle>, String> {
    // Check if bundle exists
    let bundle = context
        .timeframe_bundle
        .as_ref()
        .ok_or_else(|| "OHLCV data not available - bundle is None".to_string())?;
    
    // Check if timeframe exists in bundle
    let timeframe = &context.strategy_timeframe;
    let candles = bundle
        .get_timeframe(timeframe)
        .ok_or_else(|| format!("Timeframe {} not available in bundle (valid: 1m, 5m, 15m, 1h, 4h, 12h, 1d)", timeframe))?;
    
    // Check if timeframe has data
    if candles.is_empty() {
        return Err(format!("Timeframe {} has no candle data - OHLCV system may not have fetched historical data yet", timeframe));
    }
    
    Ok(candles.clone())
}

/// Helper to extract candles for a specific timeframe from TimeframeBundle
/// Supports per-condition timeframe selection with fallback to strategy timeframe
/// Returns detailed error messages for debugging
pub fn get_candles_for_timeframe(context: &EvaluationContext, condition_timeframe: Option<&str>) -> Result<Vec<Candle>, String> {
    // Check if bundle exists
    let bundle = context
        .timeframe_bundle
        .as_ref()
        .ok_or_else(|| "OHLCV data not available - bundle is None".to_string())?;
    
    // Use condition's timeframe if provided, otherwise fallback to strategy timeframe
    let timeframe = condition_timeframe.unwrap_or(&context.strategy_timeframe);
    
    // Validate timeframe value
    let valid_timeframes = ["1m", "5m", "15m", "1h", "4h", "12h", "1d"];
    if !valid_timeframes.contains(&timeframe) {
        return Err(format!("Invalid timeframe '{}' - valid options: {}", timeframe, valid_timeframes.join(", ")));
    }
    
    // Check if timeframe exists in bundle
    let candles = bundle
        .get_timeframe(timeframe)
        .ok_or_else(|| format!("Timeframe {} not available in bundle (valid: 1m, 5m, 15m, 1h, 4h, 12h, 1d)", timeframe))?;
    
    // Check if timeframe has data
    if candles.is_empty() {
        return Err(format!("Timeframe {} has no candle data - OHLCV system may not have fetched historical data yet", timeframe));
    }
    
    Ok(candles.clone())
}

/// Trait for condition evaluation
#[async_trait]
pub trait ConditionEvaluator: Send + Sync {
    /// Unique identifier for this condition type
    fn condition_type(&self) -> &'static str;

    /// Evaluate the condition against the context
    async fn evaluate(
        &self,
        condition: &Condition,
        context: &EvaluationContext,
    ) -> Result<bool, String>;

    /// Validate condition parameters
    fn validate(&self, condition: &Condition) -> Result<(), String>;

    /// Get parameter description for UI
    fn parameter_schema(&self) -> serde_json::Value;
}

/// Registry for all condition evaluators
pub struct ConditionRegistry {
    evaluators: std::collections::HashMap<String, Box<dyn ConditionEvaluator>>,
}

impl ConditionRegistry {
    /// Create a new registry with all built-in conditions
    pub fn new() -> Self {
        let mut registry = Self {
            evaluators: std::collections::HashMap::new(),
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

    /// Get all parameter schemas for UI
    pub fn get_all_schemas(&self) -> serde_json::Value {
        let mut schemas = serde_json::Map::new();
        for (name, evaluator) in &self.evaluators {
            schemas.insert(name.clone(), evaluator.parameter_schema());
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
pub fn get_param_f64(condition: &Condition, param_name: &str) -> Result<f64, String> {
    let param = condition
        .parameters
        .get(param_name)
        .ok_or_else(|| format!("Missing parameter: {}", param_name))?;

    param
        .value
        .as_f64()
        .ok_or_else(|| format!("Parameter {} must be a number", param_name))
}

/// Helper function to get parameter value as string
pub fn get_param_string(condition: &Condition, param_name: &str) -> Result<String, String> {
    let param = condition
        .parameters
        .get(param_name)
        .ok_or_else(|| format!("Missing parameter: {}", param_name))?;

    param
        .value
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| format!("Parameter {} must be a string", param_name))
}

/// Helper function to get parameter value as bool
pub fn get_param_bool(condition: &Condition, param_name: &str) -> Result<bool, String> {
    let param = condition
        .parameters
        .get(param_name)
        .ok_or_else(|| format!("Missing parameter: {}", param_name))?;

    param
        .value
        .as_bool()
        .ok_or_else(|| format!("Parameter {} must be a boolean", param_name))
}

/// Helper function to get optional parameter value as string
pub fn get_param_string_optional(condition: &Condition, param_name: &str) -> Option<String> {
    condition
        .parameters
        .get(param_name)
        .and_then(|param| param.value.as_str())
        .map(|s| s.to_string())
}

/// Helper function to validate optional timeframe parameter
pub fn validate_timeframe_param(condition: &Condition) -> Result<(), String> {
    if let Some(timeframe) = get_param_string_optional(condition, "timeframe") {
        let valid_timeframes = ["1m", "5m", "15m", "1h", "4h", "12h", "1d"];
        if !valid_timeframes.contains(&timeframe.as_str()) {
            return Err(format!(
                "Invalid timeframe '{}' - valid options: {}",
                timeframe,
                valid_timeframes.join(", ")
            ));
        }
    }
    Ok(())
}
