use crate::strategies::conditions::{get_candles_from_context, get_param_f64, get_param_string, ConditionEvaluator};
use crate::strategies::types::{Condition, EvaluationContext};
use async_trait::async_trait;
use serde_json::json;

/// Detect volume spikes compared to average volume
pub struct VolumeSpikeCondition;

#[async_trait]
impl ConditionEvaluator for VolumeSpikeCondition {
    fn condition_type(&self) -> &'static str {
        "VolumeSpike"
    }

    async fn evaluate(
        &self,
        condition: &Condition,
        context: &EvaluationContext,
    ) -> Result<bool, String> {
        let lookback = get_param_f64(condition, "lookback")? as usize;
        let multiplier = get_param_f64(condition, "multiplier")?;

        let candles = get_candles_from_context(context)?;

        if candles.len() < lookback + 1 {
            return Err(format!(
                "Not enough candles: {} < {}",
                candles.len(),
                lookback + 1
            ));
        }

        // Get current candle volume
        let current_candle = candles.last().unwrap();
        let current_volume = current_candle.volume;

        // Calculate average volume over lookback period (excluding current)
        let end_idx = candles.len().saturating_sub(1);
        let start_idx = end_idx.saturating_sub(lookback);
        let lookback_candles = &candles[start_idx..end_idx];

        if lookback_candles.is_empty() {
            return Err("No lookback candles for volume calculation".to_string());
        }

        let avg_volume: f64 =
            lookback_candles.iter().map(|c| c.volume).sum::<f64>() / lookback_candles.len() as f64;

        if avg_volume <= 0.0 {
            return Err("Average volume is zero or negative".to_string());
        }

        // Check if current volume is multiplier times the average
        let volume_ratio = current_volume / avg_volume;
        let result = volume_ratio >= multiplier;

        Ok(result)
    }

    fn validate(&self, condition: &Condition) -> Result<(), String> {
        let lookback = get_param_f64(condition, "lookback")?;
        if lookback < 2.0 || lookback > 100.0 {
            return Err("Lookback must be between 2 and 100".to_string());
        }

        let multiplier = get_param_f64(condition, "multiplier")?;
        if multiplier < 1.0 || multiplier > 50.0 {
            return Err("Multiplier must be between 1.0 and 50.0".to_string());
        }

        Ok(())
    }

    fn parameter_schema(&self) -> serde_json::Value {
        json!({
            "type": "VolumeSpike",
            "name": "Volume Spike",
            "category": "Volume Analysis",
            "tags": ["volume", "spike", "momentum", "interest"],
            "icon": "📊",
            "origin": "strategy",
            "description": "Detect volume spikes compared to average volume (indicates increased interest)",
            "parameters": {
                "lookback": {
                    "type": "number",
                    "name": "Lookback Period",
                    "description": "Number of candles to calculate average volume",
                    "default": 20,
                    "min": 2,
                    "max": 100,
                    "step": 1
                },
                "multiplier": {
                    "type": "number",
                    "name": "Volume Multiplier",
                    "description": "How many times above average (e.g., 2.0 = 200% of average)",
                    "default": 2.0,
                    "min": 1.0,
                    "max": 50.0,
                    "step": 0.5
                }
            }
        })
    }
}
