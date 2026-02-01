use serde::{Deserialize, Serialize};

/// AI trade decision schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeDecision {
    pub decision: TradeAction,
    pub confidence: u8,
    pub reasoning: String,
    pub risk_level: String,
    #[serde(default)]
    pub factors: Vec<TradeFactor>,
    #[serde(default)]
    pub suggested_entry_price: Option<f64>,
    #[serde(default)]
    pub suggested_stop_loss: Option<f64>,
    #[serde(default)]
    pub suggested_take_profit: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TradeAction {
    Buy,
    Sell,
    Hold,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeFactor {
    pub name: String,
    pub impact: String, // "positive", "negative", "neutral"
    #[serde(default)]
    pub weight: u8,
}
