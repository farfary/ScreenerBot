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
            "name": "Price Change in Window",
            "category": "Price Patterns",
            "tags": ["momentum", "volatility"],
            "icon": "📈",
            "origin": "strategy",
            "description": "Check if price moved by percentage within timeframe",
            "parameters": {
                "timeframe": {
                    "type": "string",
                    "description": "Timeframe for price movement (e.g., '5m', '1h')",
                    "default": "5m",
                    "options": ["1m", "5m", "15m", "30m", "1h", "4h", "1d"]
                },
                "percentage": {
                    "type": "number",
                    "description": "Minimum price change percentage",
                    "default": 5.0,
                    "min": 0.0
                },
                "direction": {
                    "type": "string",
                    "description": "Direction of price movement",
                    "default": "UP",
                    "options": ["UP", "DOWN", "ANY"]
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
