//! AI-powered trading analysis
//!
//! Uses LLM for intelligent entry/exit decisions.
//! All features disabled by default.

use crate::ai::schemas::{ExitSuggestion, TradeAction, TradeDecision};
use crate::ai::{AiEngine, EvaluationContext, Priority};
use crate::config::with_config;
use crate::positions::types::Position;
use crate::tokens::types::Token;
use serde_json::json;

/// Result of AI entry analysis
#[derive(Debug, Clone)]
pub struct EntryAnalysisResult {
    pub should_enter: bool,
    pub confidence: u8,
    pub reasoning: String,
    pub suggested_amount: Option<f64>,
    pub provider: String,
}

/// Result of AI exit analysis
#[derive(Debug, Clone)]
pub struct ExitAnalysisResult {
    pub action: ExitAction,
    pub confidence: u8,
    pub reasoning: String,
    pub suggested_percentage: Option<u8>,
    pub urgency: ExitUrgency,
    pub provider: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExitAction {
    Hold,
    Exit,
    PartialExit,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExitUrgency {
    Low,
    Normal,
    High,
    Immediate,
}

/// Check if AI entry analysis should be performed
pub fn should_analyze_entry() -> bool {
    with_config(|cfg| cfg.ai.enabled && cfg.ai.entry_analysis_enabled)
}

/// Check if AI exit analysis should be performed
pub fn should_analyze_exit() -> bool {
    with_config(|cfg| cfg.ai.enabled && cfg.ai.exit_analysis_enabled)
}

/// Perform AI entry analysis for a token
/// Returns None if AI is disabled or analysis fails
pub async fn analyze_entry(token: &Token) -> Option<EntryAnalysisResult> {
    if !should_analyze_entry() {
        return None;
    }

    let (min_confidence, bypass_cache) =
        with_config(|cfg| (cfg.ai.filtering_min_confidence, cfg.ai.trading_bypass_cache));

    // Get global AI engine
    let ai_engine = match crate::ai::try_get_ai_engine() {
        Some(engine) => engine,
        None => {
            crate::logger::warning(
                crate::logger::LogTag::Trader,
                "AI entry analysis requested but AI engine not initialized",
            );
            return None;
        }
    };

    // Build context with token data
    let context = EvaluationContext {
        mint: token.mint.clone(),
        dexscreener_data: Some(json!(token)), // Serialize token data
        ..Default::default()
    };

    // Use HIGH priority for trading (bypasses cache if configured)
    let priority = if bypass_cache {
        Priority::High
    } else {
        Priority::Medium
    };

    // Call AI for entry analysis
    match ai_engine.evaluate_entry(&context, priority).await {
        Ok(result) => {
            let decision = &result.decision;
            let should_enter = decision.decision == "buy" && decision.confidence >= min_confidence;

            Some(EntryAnalysisResult {
                should_enter,
                confidence: decision.confidence,
                reasoning: decision.reasoning.clone(),
                suggested_amount: None,
                provider: decision.provider.clone(),
            })
        }
        Err(e) => {
            crate::logger::warning(
                crate::logger::LogTag::Trader,
                &format!("AI entry analysis failed for {}: {}", token.symbol, e),
            );
            None
        }
    }
}

/// Perform AI exit analysis for a position
/// Returns None if AI is disabled or analysis fails
pub async fn analyze_exit(position: &Position, token: &Token) -> Option<ExitAnalysisResult> {
    if !should_analyze_exit() {
        return None;
    }

    let bypass_cache = with_config(|cfg| cfg.ai.trading_bypass_cache);

    // Get global AI engine
    let ai_engine = match crate::ai::try_get_ai_engine() {
        Some(engine) => engine,
        None => {
            crate::logger::warning(
                crate::logger::LogTag::Trader,
                "AI exit analysis requested but AI engine not initialized",
            );
            return None;
        }
    };

    // Build context with position and token data
    let context = EvaluationContext {
        mint: position.mint.clone(),
        dexscreener_data: Some(json!(token)),
        opening_snapshot: Some(json!({
            "entry_price": position.entry_price,
            "average_entry_price": position.average_entry_price,
            "entry_size_sol": position.entry_size_sol,
            "total_size_sol": position.total_size_sol,
            "entry_time": position.entry_time,
            "current_price": position.current_price,
            "unrealized_pnl": position.unrealized_pnl,
            "unrealized_pnl_percent": position.unrealized_pnl_percent,
            "price_highest": position.price_highest,
            "price_lowest": position.price_lowest,
        })),
        ..Default::default()
    };

    let priority = if bypass_cache {
        Priority::High
    } else {
        Priority::Medium
    };

    // Call AI for exit analysis
    match ai_engine.evaluate_exit(&context, priority).await {
        Ok(result) => {
            let suggestion = &result.decision;

            // Parse action from the decision string
            let action = if suggestion.decision.to_lowercase().contains("exit") {
                ExitAction::Exit
            } else if suggestion.decision.to_lowercase().contains("partial") {
                ExitAction::PartialExit
            } else {
                ExitAction::Hold
            };

            // Parse urgency from risk level
            let urgency = match suggestion.risk_level {
                crate::ai::types::RiskLevel::Critical => ExitUrgency::Immediate,
                crate::ai::types::RiskLevel::High => ExitUrgency::High,
                crate::ai::types::RiskLevel::Medium => ExitUrgency::Normal,
                crate::ai::types::RiskLevel::Low => ExitUrgency::Low,
            };

            Some(ExitAnalysisResult {
                action,
                confidence: suggestion.confidence,
                reasoning: suggestion.reasoning.clone(),
                suggested_percentage: None,
                urgency,
                provider: suggestion.provider.clone(),
            })
        }
        Err(e) => {
            crate::logger::warning(
                crate::logger::LogTag::Trader,
                &format!("AI exit analysis failed for {}: {}", position.symbol, e),
            );
            None
        }
    }
}
