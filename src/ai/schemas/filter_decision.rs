use serde::{Deserialize, Serialize};

/// AI filter decision schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterDecision {
    pub decision: FilterAction,
    pub confidence: u8,
    pub reasoning: String,
    pub risk_level: String,
    #[serde(default)]
    pub factors: Vec<FilterFactor>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FilterAction {
    Pass,
    Reject,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterFactor {
    pub name: String,
    pub impact: String, // "positive", "negative", "neutral"
    #[serde(default)]
    pub weight: u8,
}
