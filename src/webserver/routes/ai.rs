//! AI Module API Routes
//!
//! Endpoints for AI analysis, provider management, and testing.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Response,
    routing::{delete, get, patch, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::ai::db;
use crate::ai::engine::AiEngine;
use crate::ai::types::{EvaluationContext, Priority};
use crate::apis::llm::{try_get_llm_manager, ChatMessage, ChatRequest, Provider};
use crate::config::{update_config_section, with_config};
use crate::logger::{self, LogTag};
use crate::webserver::state::AppState;
use crate::webserver::utils::{error_response, success_response};

// ============================================================================
// ROUTES
// ============================================================================

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        // Status & Stats
        .route("/status", get(get_ai_status))
        .route("/stats", get(get_ai_stats))
        // Provider Management
        .route("/providers", get(list_providers))
        .route("/providers/:provider/test", post(test_provider))
        // Configuration
        .route("/config", get(get_ai_config))
        .route("/config", patch(update_ai_config))
        // Cache
        .route("/cache/clear", post(clear_cache))
        .route("/cache/stats", get(get_cache_stats))
        // Testing
        .route("/test/evaluate", post(test_evaluate))
        // Instructions
        .route("/instructions", get(list_instructions))
        .route("/instructions", post(create_instruction))
        .route("/instructions/:id", get(get_instruction))
        .route("/instructions/:id", patch(update_instruction))
        .route("/instructions/:id", delete(delete_instruction))
        .route("/instructions/reorder", post(reorder_instructions))
        // Templates
        .route("/templates", get(list_templates))
        // History
        .route("/history", get(list_history))
        .route("/history/:id", get(get_history_detail))
}

// ============================================================================
// RESPONSE TYPES
// ============================================================================

#[derive(Debug, Serialize)]
pub struct AiStatusResponse {
    pub enabled: bool,
    pub filtering_enabled: bool,
    pub entry_analysis_enabled: bool,
    pub exit_analysis_enabled: bool,
    pub default_provider: String,
    pub configured_providers: Vec<ProviderStatus>,
    pub total_evaluations: u64,
    pub cache_entries: usize,
    pub cache_fresh_entries: usize,
}

#[derive(Debug, Serialize)]
pub struct ProviderStatus {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub has_api_key: bool,
    pub model: String,
    pub rate_limit_per_minute: u32,
}

#[derive(Debug, Serialize)]
pub struct AiStatsResponse {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub avg_latency_ms: f64,
    pub cache_hit_rate: f64,
}

#[derive(Debug, Serialize)]
pub struct CacheStatsResponse {
    pub total_entries: usize,
    pub fresh_entries: usize,
    pub ttl_seconds: u64,
}

#[derive(Debug, Serialize)]
pub struct AiConfigResponse {
    // Master Control
    pub enabled: bool,
    pub default_provider: String,
    // Filtering
    pub filtering_enabled: bool,
    pub filtering_min_confidence: u8,
    pub filtering_fallback_pass: bool,
    pub filtering_use_cache: bool,
    // Trading
    pub entry_analysis_enabled: bool,
    pub exit_analysis_enabled: bool,
    pub ai_trailing_stop_enabled: bool,
    pub trading_bypass_cache: bool,
    // Auto Blacklist
    pub auto_blacklist_enabled: bool,
    pub auto_blacklist_min_confidence: u8,
    // Background Check
    pub background_check_enabled: bool,
    pub background_check_interval_seconds: u64,
    pub background_batch_size: u32,
    // Rate Limits
    pub max_evaluations_per_minute: u32,
    // Performance
    pub cache_ttl_seconds: u64,
}

#[derive(Debug, Serialize)]
pub struct ProvidersListResponse {
    pub providers: Vec<ProviderStatus>,
    pub default_provider: String,
}

#[derive(Debug, Serialize)]
pub struct TestProviderResponse {
    pub provider: String,
    pub success: bool,
    pub model: String,
    pub latency_ms: f64,
    pub tokens_used: u32,
    pub response_preview: String,
}

#[derive(Debug, Serialize)]
pub struct TestEvaluateResponse {
    pub decision: String,
    pub confidence: u8,
    pub reasoning: String,
    pub risk_level: String,
    pub factors: Vec<FactorResponse>,
    pub provider: String,
    pub model: String,
    pub tokens_used: u32,
    pub latency_ms: f64,
    pub cached: bool,
}

#[derive(Debug, Serialize)]
pub struct FactorResponse {
    pub name: String,
    pub impact: String,
    pub weight: u8,
}

#[derive(Debug, Serialize)]
pub struct InstructionResponse {
    pub id: i64,
    pub name: String,
    pub content: String,
    pub category: String,
    pub priority: i32,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
pub struct InstructionsListResponse {
    pub instructions: Vec<InstructionResponse>,
    pub total: usize,
}

#[derive(Debug, Serialize)]
pub struct TemplateResponse {
    pub id: String,
    pub name: String,
    pub category: String,
    pub content: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct TemplatesListResponse {
    pub templates: Vec<TemplateResponse>,
}

#[derive(Debug, Serialize)]
pub struct DecisionHistoryResponse {
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

#[derive(Debug, Serialize)]
pub struct HistoryListResponse {
    pub decisions: Vec<DecisionHistoryResponse>,
    pub total: usize,
    pub page: usize,
    pub per_page: usize,
}

// ============================================================================
// REQUEST TYPES
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct UpdateAiConfigRequest {
    // Master Control
    pub enabled: Option<bool>,
    pub default_provider: Option<String>,
    // Filtering
    pub filtering_enabled: Option<bool>,
    pub filtering_min_confidence: Option<u8>,
    pub filtering_fallback_pass: Option<bool>,
    pub filtering_use_cache: Option<bool>,
    // Trading
    pub entry_analysis_enabled: Option<bool>,
    pub exit_analysis_enabled: Option<bool>,
    pub ai_trailing_stop_enabled: Option<bool>,
    pub trading_bypass_cache: Option<bool>,
    // Auto Blacklist
    pub auto_blacklist_enabled: Option<bool>,
    pub auto_blacklist_min_confidence: Option<u8>,
    // Background Check
    pub background_check_enabled: Option<bool>,
    pub background_check_interval_seconds: Option<u64>,
    pub background_batch_size: Option<u32>,
    // Rate Limits
    pub max_evaluations_per_minute: Option<u32>,
    // Performance
    pub cache_ttl_seconds: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct TestEvaluateRequest {
    pub mint: String,
    pub priority: Option<String>, // "high", "medium", "low"
}

#[derive(Debug, Deserialize)]
pub struct CreateInstructionRequest {
    pub name: String,
    pub content: String,
    pub category: Option<String>, // defaults to "general"
}

#[derive(Debug, Deserialize)]
pub struct UpdateInstructionRequest {
    pub name: Option<String>,
    pub content: Option<String>,
    pub category: Option<String>,
    pub priority: Option<i32>,
    pub enabled: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct ReorderInstructionsRequest {
    pub ids: Vec<i64>,
}

#[derive(Debug, Deserialize)]
pub struct HistoryQuery {
    pub page: Option<usize>,
    pub per_page: Option<usize>,
    pub mint: Option<String>,
}

// ============================================================================
// HANDLERS
// ============================================================================

/// GET /api/ai/status - Get AI module status
async fn get_ai_status(State(state): State<Arc<AppState>>) -> Response {
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
        ("fireworks", "Fireworks AI", &config.providers.fireworks),
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
async fn get_ai_stats(State(_state): State<Arc<AppState>>) -> Response {
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

/// GET /api/ai/providers - List all providers with status
async fn list_providers(State(_state): State<Arc<AppState>>) -> Response {
    let config = with_config(|cfg| cfg.ai.clone());

    let mut providers = Vec::new();

    // API-based providers
    let provider_checks = [
        ("openai", "OpenAI", &config.providers.openai),
        ("anthropic", "Anthropic", &config.providers.anthropic),
        ("groq", "Groq", &config.providers.groq),
        ("deepseek", "DeepSeek", &config.providers.deepseek),
        ("gemini", "Gemini", &config.providers.gemini),
        ("together", "Together AI", &config.providers.together),
        ("openrouter", "OpenRouter", &config.providers.openrouter),
        ("mistral", "Mistral AI", &config.providers.mistral),
        ("fireworks", "Fireworks AI", &config.providers.fireworks),
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

    // Ollama
    providers.push(ProviderStatus {
        id: "ollama".to_string(),
        name: "Ollama (Local)".to_string(),
        enabled: config.providers.ollama.enabled,
        has_api_key: true,
        model: config.providers.ollama.model.clone(),
        rate_limit_per_minute: config.providers.ollama.rate_limit_per_minute,
    });

    success_response(ProvidersListResponse {
        providers,
        default_provider: config.default_provider,
    })
}

/// POST /api/ai/providers/:provider/test - Test a specific provider
async fn test_provider(
    State(_state): State<Arc<AppState>>,
    Path(provider_name): Path<String>,
) -> Response {
    // Parse provider
    let provider = match Provider::from_str(&provider_name) {
        Some(p) => p,
        None => {
            return error_response(
                StatusCode::BAD_REQUEST,
                "INVALID_PROVIDER",
                &format!("Unknown provider: {}", provider_name),
                None,
            );
        }
    };

    // Get LLM manager
    let llm_manager = match try_get_llm_manager() {
        Some(m) => m,
        None => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "LLM_NOT_INITIALIZED",
                "LLM manager not initialized",
                None,
            );
        }
    };

    // Get client for provider
    let client = match llm_manager.get_client(provider) {
        Some(c) => c,
        None => {
            return error_response(
                StatusCode::BAD_REQUEST,
                "PROVIDER_DISABLED",
                &format!("Provider '{}' is not configured or disabled", provider_name),
                None,
            );
        }
    };

    // Get model from config
    let model = with_config(|cfg| {
        let provider_cfg = match provider {
            Provider::OpenAi => &cfg.ai.providers.openai,
            Provider::Anthropic => &cfg.ai.providers.anthropic,
            Provider::Groq => &cfg.ai.providers.groq,
            Provider::DeepSeek => &cfg.ai.providers.deepseek,
            Provider::Gemini => &cfg.ai.providers.gemini,
            Provider::Together => &cfg.ai.providers.together,
            Provider::OpenRouter => &cfg.ai.providers.openrouter,
            Provider::Mistral => &cfg.ai.providers.mistral,
            Provider::Fireworks => &cfg.ai.providers.fireworks,
            Provider::Ollama => {
                return cfg.ai.providers.ollama.model.clone();
            }
        };

        if !provider_cfg.model.is_empty() {
            provider_cfg.model.clone()
        } else {
            // Default models
            match provider {
                Provider::OpenAi => "gpt-4".to_string(),
                Provider::Anthropic => "claude-3-5-sonnet-20241022".to_string(),
                Provider::Groq => "llama-3.1-70b-versatile".to_string(),
                Provider::DeepSeek => "deepseek-chat".to_string(),
                Provider::Gemini => "gemini-pro".to_string(),
                Provider::Together => "meta-llama/Llama-3-70b-chat-hf".to_string(),
                Provider::OpenRouter => "openai/gpt-4".to_string(),
                Provider::Mistral => "mistral-large-latest".to_string(),
                Provider::Fireworks => {
                    "accounts/fireworks/models/llama-v3-70b-instruct".to_string()
                }
                Provider::Ollama => "llama3.2".to_string(),
            }
        }
    });

    // Create test request
    let request = ChatRequest::new(
        model.clone(),
        vec![
            ChatMessage::system("You are a helpful assistant testing API connectivity."),
            ChatMessage::user("Respond with 'OK' if you can read this message."),
        ],
    )
    .with_max_tokens(50);

    // Make request
    let start = std::time::Instant::now();
    match client.call(request).await {
        Ok(response) => {
            let latency_ms = start.elapsed().as_secs_f64() * 1000.0;

            logger::info(
                LogTag::Api,
                &format!(
                    "AI provider '{}' test successful - model: {}, latency: {:.0}ms",
                    provider_name, model, latency_ms
                ),
            );

            let preview = if response.content.len() > 100 {
                format!("{}...", &response.content[..100])
            } else {
                response.content.clone()
            };

            success_response(TestProviderResponse {
                provider: provider_name,
                success: true,
                model: response.model,
                latency_ms,
                tokens_used: response.usage.total_tokens,
                response_preview: preview,
            })
        }
        Err(e) => {
            logger::error(
                LogTag::Api,
                &format!("AI provider '{}' test failed: {}", provider_name, e),
            );

            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "PROVIDER_TEST_FAILED",
                &format!("Provider test failed: {}", e),
                None,
            )
        }
    }
}

/// GET /api/ai/config - Get AI configuration
async fn get_ai_config(State(_state): State<Arc<AppState>>) -> Response {
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
async fn update_ai_config(
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
async fn clear_cache(State(state): State<Arc<AppState>>) -> Response {
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
async fn get_cache_stats(State(state): State<Arc<AppState>>) -> Response {
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
async fn test_evaluate(
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

// ============================================================================
// INSTRUCTIONS HANDLERS
// ============================================================================

/// GET /api/ai/instructions - List all instructions
async fn list_instructions(State(_state): State<Arc<AppState>>) -> Response {
    match db::with_ai_db(|conn| db::list_instructions(conn)) {
        Ok(instructions) => {
            let total = instructions.len();
            let instructions: Vec<InstructionResponse> = instructions
                .into_iter()
                .map(|i| InstructionResponse {
                    id: i.id,
                    name: i.name,
                    content: i.content,
                    category: i.category,
                    priority: i.priority,
                    enabled: i.enabled,
                    created_at: i.created_at,
                    updated_at: i.updated_at,
                })
                .collect();

            success_response(InstructionsListResponse {
                instructions,
                total,
            })
        }
        Err(e) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DB_ERROR",
            &format!("Failed to list instructions: {}", e),
            None,
        ),
    }
}

/// GET /api/ai/instructions/:id - Get single instruction
async fn get_instruction(
    State(_state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Response {
    match db::with_ai_db(|conn| db::get_instruction(conn, id)) {
        Ok(Some(i)) => success_response(InstructionResponse {
            id: i.id,
            name: i.name,
            content: i.content,
            category: i.category,
            priority: i.priority,
            enabled: i.enabled,
            created_at: i.created_at,
            updated_at: i.updated_at,
        }),
        Ok(None) => error_response(
            StatusCode::NOT_FOUND,
            "NOT_FOUND",
            &format!("Instruction {} not found", id),
            None,
        ),
        Err(e) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DB_ERROR",
            &format!("Failed to get instruction: {}", e),
            None,
        ),
    }
}

/// POST /api/ai/instructions - Create new instruction
async fn create_instruction(
    State(_state): State<Arc<AppState>>,
    Json(req): Json<CreateInstructionRequest>,
) -> Response {
    let category = req.category.unwrap_or_else(|| "general".to_string());

    match db::with_ai_db(|conn| db::create_instruction(conn, &req.name, &req.content, &category)) {
        Ok(id) => {
            logger::info(
                LogTag::Api,
                &format!("Created AI instruction: {} ({})", req.name, category),
            );

            // Fetch the created instruction
            match db::with_ai_db(|conn| db::get_instruction(conn, id)) {
                Ok(Some(instruction)) => success_response(InstructionResponse {
                    id: instruction.id,
                    name: instruction.name,
                    content: instruction.content,
                    category: instruction.category,
                    priority: instruction.priority,
                    enabled: instruction.enabled,
                    created_at: instruction.created_at,
                    updated_at: instruction.updated_at,
                }),
                Ok(None) => error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "DB_ERROR",
                    "Failed to retrieve created instruction",
                    None,
                ),
                Err(e) => error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "DB_ERROR",
                    &format!("Failed to retrieve created instruction: {}", e),
                    None,
                ),
            }
        }
        Err(e) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DB_ERROR",
            &format!("Failed to create instruction: {}", e),
            None,
        ),
    }
}

/// PATCH /api/ai/instructions/:id - Update instruction
async fn update_instruction(
    State(_state): State<Arc<AppState>>,
    Path(id): Path<i64>,
    Json(req): Json<UpdateInstructionRequest>,
) -> Response {
    match db::with_ai_db(|conn| {
        db::update_instruction(
            conn,
            id,
            req.name.as_deref(),
            req.content.as_deref(),
            req.category.as_deref(),
            req.priority,
            req.enabled,
        )
    }) {
        Ok(()) => {
            logger::info(LogTag::Api, &format!("Updated AI instruction: {}", id));

            // Fetch the updated instruction
            match db::with_ai_db(|conn| db::get_instruction(conn, id)) {
                Ok(Some(instruction)) => success_response(InstructionResponse {
                    id: instruction.id,
                    name: instruction.name,
                    content: instruction.content,
                    category: instruction.category,
                    priority: instruction.priority,
                    enabled: instruction.enabled,
                    created_at: instruction.created_at,
                    updated_at: instruction.updated_at,
                }),
                Ok(None) => error_response(
                    StatusCode::NOT_FOUND,
                    "NOT_FOUND",
                    &format!("Instruction {} not found", id),
                    None,
                ),
                Err(e) => error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "DB_ERROR",
                    &format!("Failed to retrieve updated instruction: {}", e),
                    None,
                ),
            }
        }
        Err(e) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DB_ERROR",
            &format!("Failed to update instruction: {}", e),
            None,
        ),
    }
}

/// DELETE /api/ai/instructions/:id - Delete instruction
async fn delete_instruction(
    State(_state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Response {
    match db::with_ai_db(|conn| db::delete_instruction(conn, id)) {
        Ok(()) => {
            logger::info(LogTag::Api, &format!("Deleted AI instruction: {}", id));
            success_response(serde_json::json!({
                "message": "Instruction deleted successfully"
            }))
        }
        Err(e) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DB_ERROR",
            &format!("Failed to delete instruction: {}", e),
            None,
        ),
    }
}

/// POST /api/ai/instructions/reorder - Reorder instructions
async fn reorder_instructions(
    State(_state): State<Arc<AppState>>,
    Json(req): Json<ReorderInstructionsRequest>,
) -> Response {
    match db::with_ai_db(|conn| db::reorder_instructions(conn, &req.ids)) {
        Ok(()) => {
            logger::info(
                LogTag::Api,
                &format!("Reordered {} AI instructions", req.ids.len()),
            );
            success_response(serde_json::json!({
                "message": "Instructions reordered successfully"
            }))
        }
        Err(e) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DB_ERROR",
            &format!("Failed to reorder instructions: {}", e),
            None,
        ),
    }
}

// ============================================================================
// TEMPLATES HANDLERS
// ============================================================================

/// GET /api/ai/templates - List built-in templates
async fn list_templates(State(_state): State<Arc<AppState>>) -> Response {
    let templates = db::get_builtin_templates();
    let templates: Vec<TemplateResponse> = templates
        .into_iter()
        .map(|t| TemplateResponse {
            id: t.id.to_string(),
            name: t.name.to_string(),
            category: t.category.to_string(),
            content: t.content.to_string(),
            tags: t.tags.iter().map(|s| s.to_string()).collect(),
        })
        .collect();

    success_response(TemplatesListResponse { templates })
}

// ============================================================================
// HISTORY HANDLERS
// ============================================================================

/// GET /api/ai/history - List decision history with pagination
async fn list_history(
    State(_state): State<Arc<AppState>>,
    Query(query): Query<HistoryQuery>,
) -> Response {
    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(50).clamp(1, 100);

    // Calculate offset
    let offset = (page - 1) * per_page;

    // Fetch decisions based on whether mint filter is provided
    let result = if let Some(mint) = query.mint {
        // For specific mint, use list_decisions_for_mint
        db::with_ai_db(|conn| db::list_decisions_for_mint(conn, &mint, per_page))
    } else {
        // For all decisions, use list_decisions with pagination
        db::with_ai_db(|conn| db::list_decisions(conn, per_page, offset))
    };

    match result {
        Ok(decisions) => {
            // Get total count (simplified - in production, you'd want a separate count query)
            let total = decisions.len();

            let decisions: Vec<DecisionHistoryResponse> = decisions
                .into_iter()
                .map(|d| DecisionHistoryResponse {
                    id: d.id,
                    mint: d.mint,
                    symbol: d.symbol,
                    decision: d.decision,
                    confidence: d.confidence,
                    reasoning: d.reasoning,
                    risk_level: d.risk_level,
                    provider: d.provider,
                    model: d.model,
                    tokens_used: d.tokens_used,
                    latency_ms: d.latency_ms,
                    cached: d.cached,
                    created_at: d.created_at,
                })
                .collect();

            success_response(HistoryListResponse {
                decisions,
                total,
                page,
                per_page,
            })
        }
        Err(e) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DB_ERROR",
            &format!("Failed to list decision history: {}", e),
            None,
        ),
    }
}

/// GET /api/ai/history/:id - Get single decision details
async fn get_history_detail(
    State(_state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Response {
    match db::with_ai_db(|conn| db::get_decision(conn, id)) {
        Ok(Some(d)) => success_response(DecisionHistoryResponse {
            id: d.id,
            mint: d.mint,
            symbol: d.symbol,
            decision: d.decision,
            confidence: d.confidence,
            reasoning: d.reasoning,
            risk_level: d.risk_level,
            provider: d.provider,
            model: d.model,
            tokens_used: d.tokens_used,
            latency_ms: d.latency_ms,
            cached: d.cached,
            created_at: d.created_at,
        }),
        Ok(None) => error_response(
            StatusCode::NOT_FOUND,
            "NOT_FOUND",
            &format!("Decision {} not found", id),
            None,
        ),
        Err(e) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DB_ERROR",
            &format!("Failed to get decision: {}", e),
            None,
        ),
    }
}
