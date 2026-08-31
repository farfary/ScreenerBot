//! Model-scored analysis status, stats, configuration, cache and testing handlers (`/api/llm-analysis`).

use axum::{extract::State, http::StatusCode, response::Response, Json};
use std::sync::Arc;

use crate::config::{update_config_section, with_config};
use crate::llm_analysis::types::{EvaluationContext, Priority};
use crate::logger::{self, LogTag};
use crate::webserver::state::AppState;
use crate::webserver::utils::{error_response, success_response};

use super::types::*;

/// GET /api/llm-analysis/status - Get analysis module status
pub async fn get_analysis_status(State(state): State<Arc<AppState>>) -> Response {
    if crate::webserver::promo::are_promo_fixtures_enabled() {
        return success_response(crate::webserver::promo::get_promo_analysis_status());
    }

    let (llm, config) = with_config(|cfg| (cfg.llm.clone(), cfg.llm_analysis.clone()));
    let providers_cfg = &llm.providers;

    // Get cache stats
    let (total_entries, fresh_entries) = if let Some(engine) = &state.analysis_engine {
        engine.cache_stats()
    } else {
        (0, 0)
    };

    // Build providers list
    let mut providers = Vec::new();

    // Check each API-based provider
    let provider_checks = [
        ("openai", "OpenAI", &providers_cfg.openai),
        ("anthropic", "Anthropic", &providers_cfg.anthropic),
        ("groq", "Groq", &providers_cfg.groq),
        ("deepseek", "DeepSeek", &providers_cfg.deepseek),
        ("gemini", "Gemini", &providers_cfg.gemini),
        ("together", "Together AI", &providers_cfg.together),
        ("openrouter", "OpenRouter", &providers_cfg.openrouter),
        ("mistral", "Mistral AI", &providers_cfg.mistral),
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
        id: "ollama".to_owned(),
        name: "Ollama (Local)".to_owned(),
        enabled: providers_cfg.ollama.enabled,
        has_api_key: true, // Ollama doesn't need API key
        model: providers_cfg.ollama.model.clone(),
        rate_limit_per_minute: providers_cfg.ollama.rate_limit_per_minute,
    });

    let active_providers = providers
        .iter()
        .filter(|p| p.enabled && p.has_api_key)
        .count() as u32;
    let total_providers = providers.len() as u32;

    let response = AnalysisStatusResponse {
        enabled: llm.enabled,
        filtering_enabled: config.filtering_enabled,
        entry_analysis_enabled: config.entry_analysis_enabled,
        exit_analysis_enabled: config.exit_analysis_enabled,
        default_provider: llm.default_provider.clone(),
        configured_providers: providers,
        total_evaluations: 0, // TODO: Add metrics tracking
        cache_entries: total_entries,
        cache_fresh_entries: fresh_entries,
        metrics: AnalysisMetrics {
            total_evaluations: 0, // TODO: Add metrics tracking
            cache_hit_rate: 0.0,
            avg_response_time_ms: 0.0,
            active_providers,
            total_providers,
        },
        recent_decisions: Vec::new(),
    };

    success_response(response)
}

/// GET /api/llm-analysis/stats - Get LLM-analysis usage statistics
pub async fn get_analysis_stats(State(_state): State<Arc<AppState>>) -> Response {
    // Return promotional fixtures only for owner-initiated media capture.
    if crate::webserver::promo::are_promo_fixtures_enabled() {
        return success_response(crate::webserver::promo::get_promo_analysis_stats());
    }

    // TODO: Implement proper metrics tracking
    let response = AnalysisStatsResponse {
        total_requests: 0,
        successful_requests: 0,
        failed_requests: 0,
        avg_latency_ms: 0.0,
        cache_hit_rate: 0.0,
    };

    success_response(response)
}

/// GET /api/llm-analysis/config - Get analysis configuration
pub async fn get_analysis_config(State(_state): State<Arc<AppState>>) -> Response {
    let response = with_config(|cfg| AnalysisConfigResponse {
        filtering_enabled: cfg.llm_analysis.filtering_enabled,
        filtering_min_confidence: cfg.llm_analysis.min_confidence,
        filtering_fallback_pass: cfg.llm_analysis.fallback_pass,
        filtering_use_cache: cfg.llm_analysis.use_cache,
        entry_analysis_enabled: cfg.llm_analysis.entry_analysis_enabled,
        exit_analysis_enabled: cfg.llm_analysis.exit_analysis_enabled,
        ai_trailing_stop_enabled: cfg.llm_analysis.trailing_stop_enabled,
        trading_bypass_cache: cfg.llm_analysis.trading_bypass_cache,
        auto_blacklist_enabled: cfg.llm_analysis.auto_blacklist_enabled,
        auto_blacklist_min_confidence: cfg.llm_analysis.auto_blacklist_min_confidence,
        background_check_enabled: cfg.llm_analysis.background_check_enabled,
        background_check_interval_seconds: cfg.llm_analysis.background_check_interval_seconds,
        background_batch_size: cfg.llm_analysis.background_batch_size,
        max_evaluations_per_minute: cfg.llm_analysis.max_evaluations_per_minute,
        cache_ttl_seconds: cfg.llm_analysis.cache_ttl_seconds,
    });

    success_response(response)
}

/// PATCH /api/llm-analysis/config - Update model-analysis configuration
pub async fn update_analysis_config(
    State(_state): State<Arc<AppState>>,
    Json(req): Json<UpdateAnalysisConfigRequest>,
) -> Response {
    match update_config_section(|cfg| apply_analysis_config_update(cfg, &req), true) {
        Ok(()) => {
            logger::info(LogTag::Api, "LLM analysis configuration updated via API");
            success_response(serde_json::json!({
                "message": "Analysis configuration updated successfully"
            }))
        }
        Err(e) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "CONFIG_ERROR",
            &format!("Failed to update analysis config: {e}"),
            None,
        ),
    }
}

/// Apply an analysis-config patch in place. Split out so the ownership boundary
/// is unit-testable: this mutates `cfg.llm_analysis` only and can never reach
/// `cfg.llm` master fields (owned by `/api/llm/config`).
fn apply_analysis_config_update(
    cfg: &mut crate::config::Config,
    req: &UpdateAnalysisConfigRequest,
) {
    // Filtering
    if let Some(filtering_enabled) = req.filtering_enabled {
        cfg.llm_analysis.filtering_enabled = filtering_enabled;
    }
    if let Some(min_conf) = req.filtering_min_confidence {
        if min_conf <= 100 {
            cfg.llm_analysis.min_confidence = min_conf;
        }
    }
    if let Some(fallback_pass) = req.filtering_fallback_pass {
        cfg.llm_analysis.fallback_pass = fallback_pass;
    }
    if let Some(use_cache) = req.filtering_use_cache {
        cfg.llm_analysis.use_cache = use_cache;
    }

    // Trading
    if let Some(entry_enabled) = req.entry_analysis_enabled {
        cfg.llm_analysis.entry_analysis_enabled = entry_enabled;
    }
    if let Some(exit_enabled) = req.exit_analysis_enabled {
        cfg.llm_analysis.exit_analysis_enabled = exit_enabled;
    }
    if let Some(trailing_enabled) = req.ai_trailing_stop_enabled {
        cfg.llm_analysis.trailing_stop_enabled = trailing_enabled;
    }
    if let Some(bypass_cache) = req.trading_bypass_cache {
        cfg.llm_analysis.trading_bypass_cache = bypass_cache;
    }

    // Auto Blacklist
    if let Some(auto_blacklist) = req.auto_blacklist_enabled {
        cfg.llm_analysis.auto_blacklist_enabled = auto_blacklist;
    }
    if let Some(min_conf) = req.auto_blacklist_min_confidence {
        if min_conf <= 100 {
            cfg.llm_analysis.auto_blacklist_min_confidence = min_conf;
        }
    }

    // Background Check
    if let Some(bg_enabled) = req.background_check_enabled {
        cfg.llm_analysis.background_check_enabled = bg_enabled;
    }
    if let Some(interval) = req.background_check_interval_seconds {
        if (60..=3600).contains(&interval) {
            cfg.llm_analysis.background_check_interval_seconds = interval;
        }
    }
    if let Some(batch_size) = req.background_batch_size {
        if (1..=20).contains(&batch_size) {
            cfg.llm_analysis.background_batch_size = batch_size;
        }
    }

    // Rate Limits
    if let Some(max_evals) = req.max_evaluations_per_minute {
        if (1..=100).contains(&max_evals) {
            cfg.llm_analysis.max_evaluations_per_minute = max_evals;
        }
    }

    // Performance
    if let Some(ttl) = req.cache_ttl_seconds {
        if (60..=3600).contains(&ttl) {
            cfg.llm_analysis.cache_ttl_seconds = ttl;
        }
    }
}

/// POST /api/llm-analysis/cache/clear - Clear the analysis cache
pub async fn clear_cache(State(state): State<Arc<AppState>>) -> Response {
    if let Some(engine) = &state.analysis_engine {
        engine.clear_cache();
        logger::info(LogTag::Api, "Analysis cache cleared via API");
        success_response(serde_json::json!({
            "message": "Cache cleared successfully"
        }))
    } else {
        error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "ANALYSIS_NOT_INITIALIZED",
            "Analysis engine not initialized",
            None,
        )
    }
}

/// GET /api/llm-analysis/cache/stats - Get cache statistics
pub async fn get_cache_stats(State(state): State<Arc<AppState>>) -> Response {
    // Return promotional fixtures only for owner-initiated media capture.
    if crate::webserver::promo::are_promo_fixtures_enabled() {
        return success_response(crate::webserver::promo::get_promo_cache_stats());
    }

    if let Some(engine) = &state.analysis_engine {
        let (total_entries, fresh_entries) = engine.cache_stats();
        let ttl_seconds = with_config(|cfg| cfg.llm_analysis.cache_ttl_seconds);

        success_response(CacheStatsResponse {
            total_entries,
            fresh_entries,
            ttl_seconds,
        })
    } else {
        error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "ANALYSIS_NOT_INITIALIZED",
            "Analysis engine not initialized",
            None,
        )
    }
}

/// POST /api/llm-analysis/test/evaluate - Test model analysis with a mint address
pub async fn test_evaluate(
    State(state): State<Arc<AppState>>,
    Json(req): Json<TestEvaluateRequest>,
) -> Response {
    // Check whether model-backed features are enabled.
    let llm_enabled = with_config(|cfg| cfg.llm.enabled);
    if !llm_enabled {
        return error_response(
            StatusCode::BAD_REQUEST,
            "ANALYSIS_DISABLED",
            "LLM features are disabled. Enable [llm] first.",
            None,
        );
    }

    // Get the model-analysis engine.
    let engine = match &state.analysis_engine {
        Some(e) => e,
        None => {
            return error_response(
                StatusCode::SERVICE_UNAVAILABLE,
                "ANALYSIS_NOT_INITIALIZED",
                "Analysis engine not initialized",
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
                &format!("Invalid priority: '{p}'. Use 'high', 'medium', or 'low'."),
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
                crate::llm_analysis::types::RiskLevel::Low => "low",
                crate::llm_analysis::types::RiskLevel::Medium => "medium",
                crate::llm_analysis::types::RiskLevel::High => "high",
                crate::llm_analysis::types::RiskLevel::Critical => "critical",
            };

            let factors: Vec<FactorResponse> = result
                .decision
                .factors
                .into_iter()
                .map(|f| {
                    let impact = match f.impact {
                        crate::llm_analysis::types::Impact::Positive => "positive",
                        crate::llm_analysis::types::Impact::Negative => "negative",
                        crate::llm_analysis::types::Impact::Neutral => "neutral",
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
                &format!("LLM test evaluation failed for {}: {}", req.mint, e),
            );

            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "EVALUATION_FAILED",
                &format!("Model analysis failed: {e}"),
                None,
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    fn empty_request() -> UpdateAnalysisConfigRequest {
        // All fields optional; serde default gives every one `None`.
        serde_json::from_str("{}").unwrap()
    }

    #[test]
    fn analysis_patch_cannot_mutate_llm_master_fields() {
        let mut cfg = Config::default();
        cfg.llm.enabled = true;
        cfg.llm.default_provider = "anthropic".to_owned();
        let llm_before = serde_json::to_value(&cfg.llm).unwrap();

        let mut req = empty_request();
        req.filtering_enabled = Some(true);
        req.cache_ttl_seconds = Some(1200);
        apply_analysis_config_update(&mut cfg, &req);

        // Analysis fields moved…
        assert!(cfg.llm_analysis.filtering_enabled);
        assert_eq!(cfg.llm_analysis.cache_ttl_seconds, 1200);
        // …and the master LLM section is byte-for-byte unchanged.
        assert_eq!(serde_json::to_value(&cfg.llm).unwrap(), llm_before);
    }

    #[test]
    fn analysis_config_shape_has_no_master_fields() {
        let json = serde_json::to_value(AnalysisConfigResponse {
            filtering_enabled: false,
            filtering_min_confidence: 70,
            filtering_fallback_pass: false,
            filtering_use_cache: true,
            entry_analysis_enabled: false,
            exit_analysis_enabled: false,
            ai_trailing_stop_enabled: false,
            trading_bypass_cache: false,
            auto_blacklist_enabled: false,
            auto_blacklist_min_confidence: 80,
            background_check_enabled: false,
            background_check_interval_seconds: 300,
            background_batch_size: 5,
            max_evaluations_per_minute: 20,
            cache_ttl_seconds: 1800,
        })
        .unwrap();
        let obj = json.as_object().unwrap();
        assert!(!obj.contains_key("enabled"));
        assert!(!obj.contains_key("default_provider"));
    }

    #[test]
    fn update_request_rejects_master_fields_silently() {
        // `enabled` / `default_provider` are not part of the struct, so serde
        // ignores them — the analysis endpoint has no path to the master fields.
        let req: UpdateAnalysisConfigRequest = serde_json::from_str(
            r#"{"enabled": true, "default_provider": "groq", "filtering_enabled": true}"#,
        )
        .unwrap();
        assert_eq!(req.filtering_enabled, Some(true));
        let mut cfg = Config::default();
        apply_analysis_config_update(&mut cfg, &req);
        assert!(!cfg.llm.enabled);
        assert_eq!(
            cfg.llm.default_provider,
            Config::default().llm.default_provider
        );
    }
}
