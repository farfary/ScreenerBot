use serde::{Deserialize, Serialize};

/// AI exit suggestion schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExitSuggestion {
    pub should_exit: bool,
    pub confidence: u8,
    pub reasoning: String,
    pub urgency: ExitUrgency,
    #[serde(default)]
    pub factors: Vec<ExitFactor>,
    #[serde(default)]
    pub suggested_exit_price: Option<f64>,
    #[serde(default)]
    pub alternative_action: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExitUrgency {
    Immediate, // Exit now
    Soon,      // Exit within minutes
    Normal,    // Exit when convenient
    Low,       // Consider exiting
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExitFactor {
    pub name: String,
    pub impact: String, // "positive", "negative", "neutral"
    #[serde(default)]
    pub weight: u8,
}
