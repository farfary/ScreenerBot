//! AI status, stats, configuration, cache, and testing handlers

use axum::{extract::State, http::StatusCode, response::Response, Json};
use std::sync::Arc;

use crate::ai::engine::AiEngine;
use crate::ai::types::{EvaluationContext, Priority};
use crate::apis::llm::Provider;
use crate::config::{update_config_section, with_config};
use crate::logger::{self, LogTag};
use crate::webserver::state::AppState;
use crate::webserver::utils::{error_response, success_response};

use super::types::*;

/// GET /api/ai/status - Get AI module status
pub async fn get_ai_status(State(state): State<Arc<AppState>>) -> Response {
    let config = with_config(|cfg| cfg.ai.clone());

    // Get cache stats
    let (total_entries, fresh_entries) = if let Some(engine) = &state.ai_engine {
        engine.cache_stats()
    } else {
        (0, 0)
    };

    // Build providers list
    let mut providers = Vec::new();

    // Check each API-based provider
    let provider_checks = [
        ("openai", "OpenAI", &config.providers.openai),
        ("anthropic", "Anthropic", &config.providers.anthropic),
        ("groq", "Groq", &config.providers.groq),
        ("deepseek", "DeepSeek", &config.providers.deepseek),
        ("gemini", "Gemini", &config.providers.gemini),
        ("together", "Together AI", &config.providers.together),
        ("openrouter", "OpenRouter", &config.providers.openrouter),
        ("mistral", "Mistral AI", &config.providers.mistral),
    ];

    for (id, name, provider_cfg) in provider_checks {
        providers.push(ProviderStatus {
            id: id.to_string(),
            name: name.to_string(),
            enabled: provider_cfg.enabled,
            has_api_key: !provider_cfg.api_key.is_empty(),
            model: provider_cfg.model.clone(),
            rate_limit_per_minute: provider_cfg.rate_limit_per_minute,
        });
    }

    // Add Ollama separately (different config type)
    providers.push(ProviderStatus {
        id: "ollama".to_string(),
        name: "Ollama (Local)".to_string(),
        enabled: config.providers.ollama.enabled,
        has_api_key: true, // Ollama doesn't need API key
        model: config.providers.ollama.model.clone(),
        rate_limit_per_minute: config.providers.ollama.rate_limit_per_minute,
    });

    // Add Assistant (OAuth-based, no API key)
    providers.push(ProviderStatus {
        id: "Assistant".to_string(),
        name: "an LLM provider".to_string(),
        enabled: config.providers.Assistant.enabled,
        has_api_key: crate::apis::llm::Assistant::is_authenticated(),
        model: config.providers.Assistant.model.clone(),
        rate_limit_per_minute: config.providers.Assistant.rate_limit_per_minute,
    });

    let response = AiStatusResponse {
        enabled: config.enabled,
        filtering_enabled: config.filtering_enabled,
        entry_analysis_enabled: config.entry_analysis_enabled,
        exit_analysis_enabled: config.exit_analysis_enabled,
        default_provider: config.default_provider.clone(),
        configured_providers: providers,
        total_evaluations: 0, // TODO: Add metrics tracking
        cache_entries: total_entries,
        cache_fresh_entries: fresh_entries,
    };

    success_response(response)
}

/// GET /api/ai/stats - Get AI usage statistics
pub async fn get_ai_stats(State(_state): State<Arc<AppState>>) -> Response {
    // TODO: Implement proper metrics tracking
    let response = AiStatsResponse {
        total_requests: 0,
        successful_requests: 0,
        failed_requests: 0,
        avg_latency_ms: 0.0,
        cache_hit_rate: 0.0,
    };

    success_response(response)
}

/// GET /api/ai/config - Get AI configuration
pub async fn get_ai_config(State(_state): State<Arc<AppState>>) -> Response {
    let config = with_config(|cfg| cfg.ai.clone());

    let response = AiConfigResponse {
        enabled: config.enabled,
        default_provider: config.default_provider,
        filtering_enabled: config.filtering_enabled,
        filtering_min_confidence: config.filtering_min_confidence,
        filtering_fallback_pass: config.filtering_fallback_pass,
        filtering_use_cache: config.filtering_use_cache,
        entry_analysis_enabled: config.entry_analysis_enabled,
        exit_analysis_enabled: config.exit_analysis_enabled,
        ai_trailing_stop_enabled: config.ai_trailing_stop_enabled,
        trading_bypass_cache: config.trading_bypass_cache,
        auto_blacklist_enabled: config.auto_blacklist_enabled,
        auto_blacklist_min_confidence: config.auto_blacklist_min_confidence,
        background_check_enabled: config.background_check_enabled,
        background_check_interval_seconds: config.background_check_interval_seconds,
        background_batch_size: config.background_batch_size,
        max_evaluations_per_minute: config.max_evaluations_per_minute,
        cache_ttl_seconds: config.cache_ttl_seconds,
    };

    success_response(response)
}

/// PATCH /api/ai/config - Update AI configuration
pub async fn update_ai_config(
    State(_state): State<Arc<AppState>>,
    Json(req): Json<UpdateAiConfigRequest>,
) -> Response {
    match update_config_section(
        |cfg| {
            // Master Control
            if let Some(enabled) = req.enabled {
                cfg.ai.enabled = enabled;
            }
            if let Some(ref provider) = req.default_provider {
                // Validate provider name
                if Provider::from_str(provider).is_some() {
                    cfg.ai.default_provider = provider.clone();
                }
            }

            // Filtering
            if let Some(filtering_enabled) = req.filtering_enabled {
                cfg.ai.filtering_enabled = filtering_enabled;
            }
            if let Some(min_conf) = req.filtering_min_confidence {
                if min_conf <= 100 {
                    cfg.ai.filtering_min_confidence = min_conf;
                }
            }
            if let Some(fallback_pass) = req.filtering_fallback_pass {
                cfg.ai.filtering_fallback_pass = fallback_pass;
            }
            if let Some(use_cache) = req.filtering_use_cache {
                cfg.ai.filtering_use_cache = use_cache;
            }

            // Trading
            if let Some(entry_enabled) = req.entry_analysis_enabled {
                cfg.ai.entry_analysis_enabled = entry_enabled;
            }
            if let Some(exit_enabled) = req.exit_analysis_enabled {
                cfg.ai.exit_analysis_enabled = exit_enabled;
            }
            if let Some(trailing_enabled) = req.ai_trailing_stop_enabled {
                cfg.ai.ai_trailing_stop_enabled = trailing_enabled;
            }
            if let Some(bypass_cache) = req.trading_bypass_cache {
                cfg.ai.trading_bypass_cache = bypass_cache;
            }

            // Auto Blacklist
            if let Some(auto_blacklist) = req.auto_blacklist_enabled {
                cfg.ai.auto_blacklist_enabled = auto_blacklist;
            }
            if let Some(min_conf) = req.auto_blacklist_min_confidence {
                if min_conf <= 100 {
                    cfg.ai.auto_blacklist_min_confidence = min_conf;
                }
            }

            // Background Check
            if let Some(bg_enabled) = req.background_check_enabled {
                cfg.ai.background_check_enabled = bg_enabled;
            }
            if let Some(interval) = req.background_check_interval_seconds {
                if interval >= 60 && interval <= 3600 {
                    cfg.ai.background_check_interval_seconds = interval;
                }
            }
            if let Some(batch_size) = req.background_batch_size {
                if batch_size >= 1 && batch_size <= 20 {
                    cfg.ai.background_batch_size = batch_size;
                }
            }

            // Rate Limits
            if let Some(max_evals) = req.max_evaluations_per_minute {
                if max_evals >= 1 && max_evals <= 100 {
                    cfg.ai.max_evaluations_per_minute = max_evals;
                }
            }

            // Performance
            if let Some(ttl) = req.cache_ttl_seconds {
                if ttl >= 60 && ttl <= 3600 {
                    cfg.ai.cache_ttl_seconds = ttl;
                }
            }
        },
        true,
    ) {
        Ok(()) => {
            logger::info(LogTag::Api, "AI configuration updated via API");
            success_response(serde_json::json!({
                "message": "AI configuration updated successfully"
            }))
        }
        Err(e) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "CONFIG_ERROR",
            &format!("Failed to update AI config: {}", e),
            None,
        ),
    }
}

/// POST /api/ai/cache/clear - Clear AI cache
pub async fn clear_cache(State(state): State<Arc<AppState>>) -> Response {
    if let Some(engine) = &state.ai_engine {
        engine.clear_cache();
        logger::info(LogTag::Api, "AI cache cleared via API");
        success_response(serde_json::json!({
            "message": "Cache cleared successfully"
        }))
    } else {
        error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "AI_NOT_INITIALIZED",
            "AI engine not initialized",
            None,
        )
    }
}

/// GET /api/ai/cache/stats - Get cache statistics
pub async fn get_cache_stats(State(state): State<Arc<AppState>>) -> Response {
    if let Some(engine) = &state.ai_engine {
        let (total_entries, fresh_entries) = engine.cache_stats();
        let ttl_seconds = with_config(|cfg| cfg.ai.cache_ttl_seconds);

        success_response(CacheStatsResponse {
            total_entries,
            fresh_entries,
            ttl_seconds,
        })
    } else {
        error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "AI_NOT_INITIALIZED",
            "AI engine not initialized",
            None,
        )
    }
}

/// POST /api/ai/test/evaluate - Test AI evaluation with a mint address
pub async fn test_evaluate(
    State(state): State<Arc<AppState>>,
    Json(req): Json<TestEvaluateRequest>,
) -> Response {
    // Check if AI is enabled
    let ai_enabled = with_config(|cfg| cfg.ai.enabled);
    if !ai_enabled {
        return error_response(
            StatusCode::BAD_REQUEST,
            "AI_DISABLED",
            "AI module is disabled. Enable it in configuration first.",
            None,
        );
    }

    // Get AI engine
    let engine = match &state.ai_engine {
        Some(e) => e,
        None => {
            return error_response(
                StatusCode::SERVICE_UNAVAILABLE,
                "AI_NOT_INITIALIZED",
                "AI engine not initialized",
                None,
            );
        }
    };

    // Parse priority
    let priority = match req.priority.as_deref() {
        Some("high") => Priority::High,
        Some("medium") => Priority::Medium,
        Some("low") | None => Priority::Low,
        Some(p) => {
            return error_response(
                StatusCode::BAD_REQUEST,
                "INVALID_PRIORITY",
                &format!("Invalid priority: '{}'. Use 'high', 'medium', or 'low'.", p),
                None,
            );
        }
    };

    // Create minimal evaluation context
    let context = EvaluationContext {
        mint: req.mint.clone(),
        ..Default::default()
    };

    // Evaluate
    match engine.evaluate_filter(context, priority).await {
        Ok(result) => {
            let risk_level = match result.decision.risk_level {
                crate::ai::types::RiskLevel::Low => "low",
                crate::ai::types::RiskLevel::Medium => "medium",
                crate::ai::types::RiskLevel::High => "high",
                crate::ai::types::RiskLevel::Critical => "critical",
            };

            let factors: Vec<FactorResponse> = result
                .decision
                .factors
                .into_iter()
                .map(|f| {
                    let impact = match f.impact {
                        crate::ai::types::Impact::Positive => "positive",
                        crate::ai::types::Impact::Negative => "negative",
                        crate::ai::types::Impact::Neutral => "neutral",
                    };
                    FactorResponse {
                        name: f.name,
                        impact: impact.to_string(),
                        weight: f.weight,
                    }
                })
                .collect();

            success_response(TestEvaluateResponse {
                decision: result.decision.decision,
                confidence: result.decision.confidence,
                reasoning: result.decision.reasoning,
                risk_level: risk_level.to_string(),
                factors,
                provider: result.decision.provider,
                model: result.decision.model,
                tokens_used: result.decision.tokens_used,
                latency_ms: result.decision.latency_ms,
                cached: result.cached,
            })
        }
        Err(e) => {
            logger::error(
                LogTag::Api,
                &format!("AI test evaluation failed for {}: {}", req.mint, e),
            );

            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "EVALUATION_FAILED",
                &format!("AI evaluation failed: {}", e),
                None,
            )
        }
    }
}
