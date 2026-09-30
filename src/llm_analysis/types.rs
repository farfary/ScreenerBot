//! LLM analysis data types — decisions, factors, evaluation context and instruction records.

use crate::i18n::MessageId;
use serde::{Deserialize, Serialize};

/// LLM-analysis evaluation priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Priority {
    High,   // Trading decisions - bypass cache
    Medium, // Trailing stop - use recent cache
    Low,    // Filtering/background - always use cache
}

/// Analysis decision produced from a validated LLM response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisDecision {
    pub decision: String, // "pass", "reject", "buy", "sell", etc.
    pub confidence: u8,   // 0-100
    pub reasoning: String,
    pub risk_level: RiskLevel,
    pub factors: Vec<Factor>,
    pub provider: String,
    pub model: String,
    pub tokens_used: u32,
    pub latency_ms: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Factor {
    pub name: String,
    pub impact: Impact,
    pub weight: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Impact {
    Positive,
    Negative,
    Neutral,
}

/// Evaluation context with multi-source data
#[derive(Debug, Clone, Default)]
pub struct EvaluationContext {
    pub mint: String,
    pub dexscreener_data: Option<serde_json::Value>,
    pub geckoterminal_data: Option<serde_json::Value>,
    pub rugcheck_data: Option<serde_json::Value>,
    pub pool_data: Option<serde_json::Value>,
    pub opening_snapshot: Option<serde_json::Value>,
    pub price_history: Option<Vec<f64>>,
}

/// Evaluation result (generic container for any decision type)
#[derive(Debug, Clone)]
pub struct EvaluationResult {
    pub decision: AnalysisDecision,
    pub cached: bool,
}

/// User-created model-analysis instruction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Instruction {
    pub id: i64,
    pub name: String,
    pub content: String,
    pub category: String,
    pub priority: i32,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

/// LLM-analysis decision history record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionRecord {
    pub id: i64,
    pub mint: String,
    pub symbol: Option<String>,
    pub decision: String,
    pub confidence: u8,
    pub reasoning: Option<String>,
    pub risk_level: Option<String>,
    pub provider: String,
    pub model: Option<String>,
    pub tokens_used: u32,
    pub latency_ms: f64,
    pub cached: bool,
    pub created_at: String,
}

/// Built-in instruction template
#[derive(Debug, Clone)]
pub struct InstructionTemplate {
    pub id: &'static str,
    pub name: MessageId,
    pub description: MessageId,
    pub category: &'static str,
    pub content: &'static str,
    pub tags: &'static [&'static str],
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::format_en;
    use crate::llm_analysis::schemas::{FilterAction, TradeAction};

    fn assert_catalog(key: String) {
        assert_ne!(format_en(&key, None), key, "missing {key}");
    }

    /// Mirrors `ANALYSIS_RISK_LABELS` in pages/assistant.js.
    #[test]
    fn every_risk_level_has_catalog_text() {
        for level in [
            RiskLevel::Low,
            RiskLevel::Medium,
            RiskLevel::High,
            RiskLevel::Critical,
        ] {
            let id = match level {
                RiskLevel::Low => "low",
                RiskLevel::Medium => "medium",
                RiskLevel::High => "high",
                RiskLevel::Critical => "critical",
            };
            assert_catalog(format!("assistant-risk-{id}"));
        }
    }

    /// The verdict ids the engine records, mirrored by `ANALYSIS_DECISION_LABELS`
    /// in pages/assistant.js.
    #[test]
    fn every_recorded_decision_has_catalog_text() {
        let filter = |action: FilterAction| match action {
            FilterAction::Pass => "pass",
            FilterAction::Reject => "reject",
        };
        let trade = |action: TradeAction| match action {
            TradeAction::Buy => "buy",
            TradeAction::Sell => "sell",
            TradeAction::Hold => "hold",
        };
        let ids = [
            filter(FilterAction::Pass),
            filter(FilterAction::Reject),
            trade(TradeAction::Buy),
            trade(TradeAction::Sell),
            trade(TradeAction::Hold),
        ];
        for id in ids {
            assert_catalog(format!("assistant-decision-{id}"));
        }
    }
}
