//! LLM-analysis token filtering
//!
//! Uses LLM analysis to determine if tokens pass filtering criteria.
//! Disabled by default. Provider clients are configured under `[llm]`; this
//! analysis stage is configured under `[llm_analysis]`.

use crate::config::with_config;
use crate::llm_analysis::types::{EvaluationContext, Priority};
use crate::tokens::types::Token;

use super::FilterRejectionReason;

/// Check token using LLM analysis
///
/// Returns `Err(FilterRejectionReason::LlmAnalysisRejected)` when analysis rejects the token.
/// Returns `Ok(())` when analysis passes or the feature is disabled.
pub async fn evaluate(token: &Token) -> Result<(), FilterRejectionReason> {
    // Check whether LLM features and filtering analysis are enabled.
    let (llm_enabled, filtering_enabled, min_confidence, fallback_pass) = with_config(|cfg| {
        (
            cfg.llm.enabled,
            cfg.llm_analysis.filtering_enabled,
            cfg.llm_analysis.min_confidence,
            cfg.llm_analysis.fallback_pass,
        )
    });

    if !llm_enabled || !filtering_enabled {
        return Ok(());
    }

    // Get the global model-analysis engine.
    let analysis_engine = match crate::llm_analysis::try_get_analysis_engine() {
        Some(engine) => engine,
        None => {
            // Model-backed features are enabled but the analysis engine is not ready.
            return Ok(());
        }
    };

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

    // Run the model-scored filtering stage.
    match analysis_engine.evaluate_filter(context, priority).await {
        Ok(result) => {
            let decision = result.decision;

            // Check confidence threshold
            if decision.confidence < min_confidence {
                // Low confidence - use fallback
                if fallback_pass {
                    return Ok(()); // Let token pass
                } else {
                    return Err(FilterRejectionReason::LlmAnalysisRejected {
                        reason: format!("Low confidence ({}%)", decision.confidence),
                        confidence: decision.confidence,
                        provider: decision.provider,
                    });
                }
            }

            // Check decision
            match decision.decision.as_str() {
                "pass" => Ok(()),
                "reject" => Err(FilterRejectionReason::LlmAnalysisRejected {
                    reason: decision.reasoning,
                    confidence: decision.confidence,
                    provider: decision.provider,
                }),
                _ => {
                    // Unknown decision - use fallback
                    if fallback_pass {
                        Ok(())
                    } else {
                        Err(FilterRejectionReason::LlmAnalysisRejected {
                            reason: format!("Unknown LLM analysis decision: {}", decision.decision),
                            confidence: decision.confidence,
                            provider: decision.provider,
                        })
                    }
                }
            }
        }
        Err(e) => {
            // Analysis error: apply the configured fallback posture.
            if fallback_pass {
                Ok(())
            } else {
                Err(FilterRejectionReason::LlmAnalysisRejected {
                    reason: format!("LLM analysis failed: {e}"),
                    confidence: 0,
                    provider: "unknown".to_owned(),
                })
            }
        }
    }
}
