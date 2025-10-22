mod price_threshold;
mod price_movement;
mod relative_to_ma;
mod liquidity_depth;
mod position_age;

pub use price_threshold::PriceThresholdCondition;
pub use price_movement::PriceMovementCondition;
pub use relative_to_ma::RelativeToMaCondition;
pub use liquidity_depth::LiquidityDepthCondition;
pub use position_age::PositionAgeCondition;

use crate::strategies::types::{Condition, EvaluationContext};
use async_trait::async_trait;

/// Trait for condition evaluation
#[async_trait]
pub trait ConditionEvaluator: Send + Sync {
    /// Unique identifier for this condition type
    fn condition_type(&self) -> &'static str;

    /// Evaluate the condition against the context
    async fn evaluate(&self, condition: &Condition, context: &EvaluationContext) -> Result<bool, String>;

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
        registry.register(Box::new(PriceThresholdCondition));
        registry.register(Box::new(PriceMovementCondition));
        registry.register(Box::new(RelativeToMaCondition));
        registry.register(Box::new(LiquidityDepthCondition));
        registry.register(Box::new(PositionAgeCondition));

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
