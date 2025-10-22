use crate::strategies::conditions::{get_param_f64, get_param_string, ConditionEvaluator};
use crate::strategies::types::{Condition, EvaluationContext};
use async_trait::async_trait;
use chrono::Utc;
use serde_json::json;

/// Price movement condition - check if price moved by percentage in timeframe
pub struct PriceMovementCondition;

#[async_trait]
impl ConditionEvaluator for PriceMovementCondition {
    fn condition_type(&self) -> &'static str {
        "PriceMovement"
    }

    async fn evaluate(
        &self,
        condition: &Condition,
        context: &EvaluationContext,
    ) -> Result<bool, String> {
        let timeframe = get_param_string(condition, "timeframe")?;
        let percentage = get_param_f64(condition, "percentage")?;
        let direction = get_param_string(condition, "direction")?;

        let ohlcv_data = context
            .ohlcv_data
            .as_ref()
            .ok_or_else(|| "OHLCV data not available".to_string())?;

        if ohlcv_data.candles.is_empty() {
            return Err("No candle data available".to_string());
        }

        // Parse timeframe to get lookback period in minutes
        let lookback_minutes = parse_timeframe(&timeframe)?;

        // Anchor the window to the latest available candle to avoid wall-clock drift
        let latest_ts = ohlcv_data
            .candles
            .last()
            .map(|c| c.timestamp)
            .unwrap_or_else(Utc::now);
        let target_time = latest_ts - chrono::Duration::minutes(lookback_minutes as i64);

        let historical_candle = ohlcv_data
            .candles
            .iter()
            .filter(|c| c.timestamp <= target_time)
            .max_by_key(|c| c.timestamp);

        let historical_price = match historical_candle {
            Some(candle) => candle.close,
            None => {
                // Not enough historical data
                return Ok(false);
            }
        };

        let current_price = context
            .current_price
            .ok_or_else(|| "Current price not available".to_string())?;

        // Calculate price change percentage
        let price_change_pct = ((current_price - historical_price) / historical_price) * 100.0;

        let result = match direction.as_str() {
            "UP" => price_change_pct >= percentage,
            "DOWN" => price_change_pct <= -percentage,
            "ANY" => price_change_pct.abs() >= percentage,
            _ => return Err(format!("Invalid direction: {}", direction)),
        };

        Ok(result)
    }

    fn validate(&self, condition: &Condition) -> Result<(), String> {
        let _timeframe = get_param_string(condition, "timeframe")?;
        let percentage = get_param_f64(condition, "percentage")?;
        if percentage < 0.0 {
            return Err("Percentage must be non-negative".to_string());
        }

        let direction = get_param_string(condition, "direction")?;
        if !["UP", "DOWN", "ANY"].contains(&direction.as_str()) {
            return Err(format!("Invalid direction: {}", direction));
        }

        Ok(())
    }

    fn parameter_schema(&self) -> serde_json::Value {
        json!({
            "type": "PriceMovement",
            "name": "Price Movement in Timeframe",
            "category": "Price Patterns",
            "tags": ["momentum", "volatility", "timeframe"],
            "icon": "📈",
            "origin": "strategy",
            "description": "Check if price moved by percentage within a specific timeframe",
            "parameters": {
                "timeframe": {
                    "type": "enum",
                    "name": "Timeframe",
                    "description": "Time window to measure price movement",
                    "default": "5m",
                    "options": [
                        { "value": "1m", "label": "1 minute" },
                        { "value": "5m", "label": "5 minutes" },
                        { "value": "15m", "label": "15 minutes" },
                        { "value": "30m", "label": "30 minutes" },
                        { "value": "1h", "label": "1 hour" },
                        { "value": "4h", "label": "4 hours" },
                        { "value": "1d", "label": "1 day" }
                    ]
                },
                "percentage": {
                    "type": "percent",
                    "name": "Price Change %",
                    "description": "Minimum percentage price movement",
                    "default": 5.0,
                    "min": 0.1,
                    "max": 100.0,
                    "step": 0.5
                },
                "direction": {
                    "type": "enum",
                    "name": "Direction",
                    "description": "Required movement direction",
                    "default": "UP",
                    "options": [
                        { "value": "UP", "label": "Upward (Gain)" },
                        { "value": "DOWN", "label": "Downward (Loss)" },
                        { "value": "ANY", "label": "Either Direction" }
                    ]
                }
            }
        })
    }
}

/// Parse timeframe string to minutes
fn parse_timeframe(timeframe: &str) -> Result<u32, String> {
    let timeframe = timeframe.to_lowercase();

    if let Some(minutes) = timeframe.strip_suffix('m') {
        minutes
            .parse::<u32>()
            .map_err(|_| format!("Invalid timeframe format: {}", timeframe))
    } else if let Some(hours) = timeframe.strip_suffix('h') {
        hours
            .parse::<u32>()
            .map(|h| h * 60)
            .map_err(|_| format!("Invalid timeframe format: {}", timeframe))
    } else if let Some(days) = timeframe.strip_suffix('d') {
        days.parse::<u32>()
            .map(|d| d * 24 * 60)
            .map_err(|_| format!("Invalid timeframe format: {}", timeframe))
    } else {
        Err(format!("Invalid timeframe format: {}", timeframe))
    }
}
