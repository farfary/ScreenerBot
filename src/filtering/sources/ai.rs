//! AI-powered token filtering
//!
//! Uses LLM analysis to determine if tokens pass filtering criteria.
//! Disabled by default. Configure in [ai] section of config.

use crate::ai::types::{EvaluationContext, Priority};
use crate::ai::AiEngine;
use crate::config::with_config;
use crate::tokens::types::Token;

use super::FilterRejectionReason;

/// Check token using AI analysis
///
/// Returns `Some(FilterRejectionReason::AiRejected)` if AI rejects the token.
/// Returns `None` if AI passes the token or if AI filtering is disabled.
pub async fn evaluate(token: &Token) -> Result<(), FilterRejectionReason> {
    // Check if AI filtering is enabled
    let (ai_enabled, filtering_enabled, min_confidence, fallback_pass) = with_config(|cfg| {
        (
            cfg.ai.enabled,
            cfg.ai.filtering_enabled,
            cfg.ai.filtering_min_confidence,
            cfg.ai.filtering_fallback_pass,
        )
    });

    if !ai_enabled || !filtering_enabled {
        return Ok(()); // AI filtering disabled, skip
    }

    // Create AI engine instance
    let ai_engine = AiEngine::new();

    // Build evaluation context with token data
    let context = EvaluationContext {
        mint: token.mint.clone(),
        dexscreener_data: Some(serde_json::to_value(token).unwrap_or_default()),
        geckoterminal_data: None,
        rugcheck_data: None,
        pool_data: None,
        opening_snapshot: None,
        price_history: None,
    };

    // Use Low priority for filtering (allows caching)
    let priority = Priority::Low;

    // Call AI engine
    match ai_engine.evaluate_filter(context, priority).await {
        Ok(result) => {
            let decision = result.decision;

            // Check confidence threshold
            if decision.confidence < min_confidence {
                // Low confidence - use fallback
                if fallback_pass {
                    return Ok(()); // Let token pass
                } else {
                    return Err(FilterRejectionReason::AiRejected {
                        reason: format!("Low confidence ({}%)", decision.confidence),
                        confidence: decision.confidence,
                        provider: decision.provider,
                    });
                }
            }

            // Check decision
            match decision.decision.as_str() {
                "pass" => Ok(()), // AI says pass
                "reject" => Err(FilterRejectionReason::AiRejected {
                    reason: decision.reasoning,
                    confidence: decision.confidence,
                    provider: decision.provider,
                }),
                _ => {
                    // Unknown decision - use fallback
                    if fallback_pass {
                        Ok(())
                    } else {
                        Err(FilterRejectionReason::AiRejected {
                            reason: format!("Unknown AI decision: {}", decision.decision),
                            confidence: decision.confidence,
                            provider: decision.provider,
                        })
                    }
                }
            }
        }
        Err(e) => {
            // AI error - use fallback
            if fallback_pass {
                Ok(())
            } else {
                Err(FilterRejectionReason::AiRejected {
                    reason: format!("AI analysis failed: {}", e),
                    confidence: 0,
                    provider: "unknown".to_string(),
                })
            }
        }
    }
}
