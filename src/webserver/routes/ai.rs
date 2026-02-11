//! AI Module API Routes
//!
//! Endpoints for AI analysis, provider management, chat, and testing.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Response,
    routing::{delete, get, patch, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::ai::chat_db;
use crate::ai::db;
use crate::ai::engine::AiEngine;
use crate::ai::permissions::ToolPermissions;
use crate::ai::tools::ToolDefinition;
use crate::ai::types::{EvaluationContext, Priority};
use crate::ai::{
    get_chat_engine, try_get_chat_engine, ChatContext, ChatRequest as ChatEngineRequest,
    ChatResponse as ChatEngineResponse, ChatSession,
};
use crate::apis::llm::Assistant;
use crate::apis::llm::{try_get_llm_manager, ChatMessage, ChatRequest, Provider};
use crate::config::{update_config_section, with_config};
use crate::logger::{self, LogTag};
use crate::webserver::state::AppState;
use crate::webserver::utils::{error_response, success_response};
use std::sync::RwLock;

// ============================================================================
// DEVICE CODE STORAGE
// ============================================================================

/// In-memory storage for device code during OAuth flow
/// This is stored globally so the poll endpoint can access the device_code
static DEVICE_CODE_STORAGE: once_cell::sync::Lazy<RwLock<Option<String>>> =
    once_cell::sync::Lazy::new(|| RwLock::new(None));

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
        .route("/providers/:provider", patch(update_provider))
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
        // Chat Routes
        .route("/chat", post(send_chat_message))
        .route("/chat/sessions", get(list_chat_sessions))
        .route("/chat/sessions", post(create_chat_session))
        .route("/chat/sessions/:id", get(get_chat_session))
        .route("/chat/sessions/:id", delete(delete_chat_session))
        .route("/chat/sessions/:id/summarize", post(summarize_chat_session))
        .route(
            "/chat/sessions/:id/generate-title",
            post(generate_session_title),
        )
        .route(
            "/chat/confirm/:confirmation_id",
            post(confirm_tool_execution),
        )
        // Tools & Permissions
        .route("/tools", get(list_tools))
        .route("/permissions", get(get_permissions))
        .route("/permissions", patch(update_permissions))
        // Assistant Authentication
        .route("/Assistant/auth/status", get(Assistant_auth_status))
        .route("/Assistant/auth/start", post(Assistant_auth_start))
        .route("/Assistant/auth/poll", post(Assistant_auth_poll))
        .route("/Assistant/auth/logout", post(Assistant_auth_logout))
        .route("/Assistant/auth/test", post(Assistant_auth_test))
        // Automation routes
        .route(
            "/automation",
            get(list_automation_tasks).post(create_automation_task),
        )
        .route("/automation/runs", get(get_automation_recent_runs))
        .route("/automation/stats", get(get_automation_stats_handler))
        .route(
            "/automation/:id",
            get(get_automation_task)
                .patch(update_automation_task)
                .delete(delete_automation_task),
        )
        .route("/automation/:id/toggle", post(toggle_automation_task))
        .route("/automation/:id/run", post(run_automation_task))
        .route("/automation/:id/runs", get(get_automation_task_runs))
        .route("/automation/runs/:id", get(get_automation_run_detail))
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
pub struct UpdateProviderRequest {
    pub enabled: Option<bool>,
    pub api_key: Option<String>,
    pub model: Option<String>,
    pub rate_limit_per_minute: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct HistoryQuery {
    pub page: Option<usize>,
    pub per_page: Option<usize>,
    pub mint: Option<String>,
}

// ============================================================================
// CHAT REQUEST/RESPONSE TYPES
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct SendChatMessageRequest {
    pub session_id: i64,
    pub message: String,
    pub context: Option<ChatContext>,
}

#[derive(Debug, Deserialize)]
pub struct CreateChatSessionRequest {
    pub title: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CreateChatSessionResponse {
    pub session_id: i64,
}

#[derive(Debug, Serialize)]
pub struct GetChatSessionResponse {
    pub session: ChatSession,
    pub messages: Vec<chat_db::ChatMessage>,
}

#[derive(Debug, Deserialize)]
pub struct ConfirmToolExecutionRequest {
    pub approved: bool,
    pub session_id: Option<i64>,
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

    // Assistant - OAuth based (no API key)
    providers.push(ProviderStatus {
        id: "Assistant".to_string(),
        name: "an LLM provider".to_string(),
        enabled: config.providers.Assistant.enabled,
        has_api_key: crate::apis::llm::Assistant::is_authenticated(),
        model: config.providers.Assistant.model.clone(),
        rate_limit_per_minute: config.providers.Assistant.rate_limit_per_minute,
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
            Provider::Assistant => &cfg.ai.providers.Assistant,
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
                Provider::Assistant => "gpt-4o".to_string(),
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

/// PATCH /api/ai/providers/:provider - Update a specific provider's configuration
async fn update_provider(
    State(_state): State<Arc<AppState>>,
    Path(provider_name): Path<String>,
    Json(req): Json<UpdateProviderRequest>,
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

    match update_config_section(
        |cfg| {
            // Get a mutable reference to the provider config
            match provider {
                Provider::OpenAi => {
                    if let Some(enabled) = req.enabled {
                        cfg.ai.providers.openai.enabled = enabled;
                    }
                    if let Some(ref api_key) = req.api_key {
                        if !api_key.is_empty() {
                            cfg.ai.providers.openai.api_key = api_key.clone();
                        }
                    }
                    if let Some(ref model) = req.model {
                        cfg.ai.providers.openai.model = model.clone();
                    }
                    if let Some(rate_limit) = req.rate_limit_per_minute {
                        cfg.ai.providers.openai.rate_limit_per_minute = rate_limit;
                    }
                }
                Provider::Anthropic => {
                    if let Some(enabled) = req.enabled {
                        cfg.ai.providers.anthropic.enabled = enabled;
                    }
                    if let Some(ref api_key) = req.api_key {
                        if !api_key.is_empty() {
                            cfg.ai.providers.anthropic.api_key = api_key.clone();
                        }
                    }
                    if let Some(ref model) = req.model {
                        cfg.ai.providers.anthropic.model = model.clone();
                    }
                    if let Some(rate_limit) = req.rate_limit_per_minute {
                        cfg.ai.providers.anthropic.rate_limit_per_minute = rate_limit;
                    }
                }
                Provider::Groq => {
                    if let Some(enabled) = req.enabled {
                        cfg.ai.providers.groq.enabled = enabled;
                    }
                    if let Some(ref api_key) = req.api_key {
                        if !api_key.is_empty() {
                            cfg.ai.providers.groq.api_key = api_key.clone();
                        }
                    }
                    if let Some(ref model) = req.model {
                        cfg.ai.providers.groq.model = model.clone();
                    }
                    if let Some(rate_limit) = req.rate_limit_per_minute {
                        cfg.ai.providers.groq.rate_limit_per_minute = rate_limit;
                    }
                }
                Provider::DeepSeek => {
                    if let Some(enabled) = req.enabled {
                        cfg.ai.providers.deepseek.enabled = enabled;
                    }
                    if let Some(ref api_key) = req.api_key {
                        if !api_key.is_empty() {
                            cfg.ai.providers.deepseek.api_key = api_key.clone();
                        }
                    }
                    if let Some(ref model) = req.model {
                        cfg.ai.providers.deepseek.model = model.clone();
                    }
                    if let Some(rate_limit) = req.rate_limit_per_minute {
                        cfg.ai.providers.deepseek.rate_limit_per_minute = rate_limit;
                    }
                }
                Provider::Gemini => {
                    if let Some(enabled) = req.enabled {
                        cfg.ai.providers.gemini.enabled = enabled;
                    }
                    if let Some(ref api_key) = req.api_key {
                        if !api_key.is_empty() {
                            cfg.ai.providers.gemini.api_key = api_key.clone();
                        }
                    }
                    if let Some(ref model) = req.model {
                        cfg.ai.providers.gemini.model = model.clone();
                    }
                    if let Some(rate_limit) = req.rate_limit_per_minute {
                        cfg.ai.providers.gemini.rate_limit_per_minute = rate_limit;
                    }
                }
                Provider::Together => {
                    if let Some(enabled) = req.enabled {
                        cfg.ai.providers.together.enabled = enabled;
                    }
                    if let Some(ref api_key) = req.api_key {
                        if !api_key.is_empty() {
                            cfg.ai.providers.together.api_key = api_key.clone();
                        }
                    }
                    if let Some(ref model) = req.model {
                        cfg.ai.providers.together.model = model.clone();
                    }
                    if let Some(rate_limit) = req.rate_limit_per_minute {
                        cfg.ai.providers.together.rate_limit_per_minute = rate_limit;
                    }
                }
                Provider::OpenRouter => {
                    if let Some(enabled) = req.enabled {
                        cfg.ai.providers.openrouter.enabled = enabled;
                    }
                    if let Some(ref api_key) = req.api_key {
                        if !api_key.is_empty() {
                            cfg.ai.providers.openrouter.api_key = api_key.clone();
                        }
                    }
                    if let Some(ref model) = req.model {
                        cfg.ai.providers.openrouter.model = model.clone();
                    }
                    if let Some(rate_limit) = req.rate_limit_per_minute {
                        cfg.ai.providers.openrouter.rate_limit_per_minute = rate_limit;
                    }
                }
                Provider::Mistral => {
                    if let Some(enabled) = req.enabled {
                        cfg.ai.providers.mistral.enabled = enabled;
                    }
                    if let Some(ref api_key) = req.api_key {
                        if !api_key.is_empty() {
                            cfg.ai.providers.mistral.api_key = api_key.clone();
                        }
                    }
                    if let Some(ref model) = req.model {
                        cfg.ai.providers.mistral.model = model.clone();
                    }
                    if let Some(rate_limit) = req.rate_limit_per_minute {
                        cfg.ai.providers.mistral.rate_limit_per_minute = rate_limit;
                    }
                }
                Provider::Assistant => {
                    // Assistant doesn't use API key - it uses OAuth
                    if let Some(enabled) = req.enabled {
                        cfg.ai.providers.Assistant.enabled = enabled;
                    }
                    if let Some(ref model) = req.model {
                        cfg.ai.providers.Assistant.model = model.clone();
                    }
                    if let Some(rate_limit) = req.rate_limit_per_minute {
                        cfg.ai.providers.Assistant.rate_limit_per_minute = rate_limit;
                    }
                }
                Provider::Ollama => {
                    if let Some(enabled) = req.enabled {
                        cfg.ai.providers.ollama.enabled = enabled;
                    }
                    if let Some(ref model) = req.model {
                        cfg.ai.providers.ollama.model = model.clone();
                    }
                    if let Some(rate_limit) = req.rate_limit_per_minute {
                        cfg.ai.providers.ollama.rate_limit_per_minute = rate_limit;
                    }
                    // Ollama can also have a base_url but we're not updating it here
                }
            }
        },
        true, // save_to_disk
    ) {
        Ok(_) => {
            logger::info(
                LogTag::Api,
                &format!("Updated AI provider '{}' configuration", provider_name),
            );
            success_response(serde_json::json!({
                "provider": provider_name,
                "updated": true
            }))
        }
        Err(e) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "CONFIG_UPDATE_FAILED",
            &format!("Failed to update provider config: {}", e),
            None,
        ),
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
                if min_conf >= 0 && min_conf <= 100 {
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
                if min_conf >= 0 && min_conf <= 100 {
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
async fn get_instruction(State(_state): State<Arc<AppState>>, Path(id): Path<i64>) -> Response {
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
async fn delete_instruction(State(_state): State<Arc<AppState>>, Path(id): Path<i64>) -> Response {
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
        // For specific mint, use list_decisions_for_mint_paginated with offset
        db::with_ai_db(|conn| db::list_decisions_for_mint_paginated(conn, &mint, per_page, offset))
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
async fn get_history_detail(State(_state): State<Arc<AppState>>, Path(id): Path<i64>) -> Response {
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

// ============================================================================
// CHAT HANDLERS
// ============================================================================

/// POST /api/ai/chat - Send a message to AI chat
async fn send_chat_message(
    State(_state): State<Arc<AppState>>,
    Json(req): Json<SendChatMessageRequest>,
) -> Response {
    // Validate message
    if req.message.trim().is_empty() {
        return error_response(
            StatusCode::BAD_REQUEST,
            "INVALID_MESSAGE",
            "Message cannot be empty",
            None,
        );
    }

    if req.message.len() > 10000 {
        return error_response(
            StatusCode::BAD_REQUEST,
            "MESSAGE_TOO_LONG",
            "Message exceeds maximum length of 10,000 characters",
            None,
        );
    }

    // Validate session exists
    let pool = match chat_db::get_chat_pool() {
        Some(p) => p,
        None => {
            return error_response(
                StatusCode::SERVICE_UNAVAILABLE,
                "CHAT_DB_NOT_INITIALIZED",
                "Chat database not initialized",
                None,
            )
        }
    };

    match chat_db::get_session(&pool, req.session_id) {
        Ok(Some(_)) => {
            // Session exists, continue
        }
        Ok(None) => {
            return error_response(
                StatusCode::NOT_FOUND,
                "SESSION_NOT_FOUND",
                &format!("Chat session {} not found", req.session_id),
                None,
            )
        }
        Err(e) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "DB_ERROR",
                &format!("Failed to validate session: {}", e),
                None,
            )
        }
    }

    // Get chat engine
    let engine = match try_get_chat_engine() {
        Some(e) => e,
        None => {
            return error_response(
                StatusCode::SERVICE_UNAVAILABLE,
                "CHAT_NOT_INITIALIZED",
                "Chat engine not initialized",
                None,
            )
        }
    };

    // Create chat request
    let chat_request = ChatEngineRequest {
        session_id: req.session_id,
        message: req.message,
        context: req.context,
        headless: false,
        tool_mode: Default::default(),
    };

    // Process message
    match engine.process_message(chat_request).await {
        Ok(response) => {
            logger::info(
                LogTag::Api,
                &format!(
                    "Chat message processed for session {} (message {})",
                    req.session_id, response.message_id
                ),
            );
            success_response(response)
        }
        Err(e) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "CHAT_ERROR",
            &format!("Failed to process chat message: {}", e),
            None,
        ),
    }
}

/// GET /api/ai/chat/sessions - List all chat sessions
async fn list_chat_sessions(State(_state): State<Arc<AppState>>) -> Response {
    let pool = match chat_db::get_chat_pool() {
        Some(p) => p,
        None => {
            return error_response(
                StatusCode::SERVICE_UNAVAILABLE,
                "CHAT_DB_NOT_INITIALIZED",
                "Chat database not initialized",
                None,
            )
        }
    };

    match chat_db::get_sessions(&pool) {
        Ok(sessions) => success_response(sessions),
        Err(e) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DB_ERROR",
            &format!("Failed to list chat sessions: {}", e),
            None,
        ),
    }
}

/// POST /api/ai/chat/sessions - Create new chat session
async fn create_chat_session(
    State(_state): State<Arc<AppState>>,
    Json(req): Json<CreateChatSessionRequest>,
) -> Response {
    let pool = match chat_db::get_chat_pool() {
        Some(p) => p,
        None => {
            return error_response(
                StatusCode::SERVICE_UNAVAILABLE,
                "CHAT_DB_NOT_INITIALIZED",
                "Chat database not initialized",
                None,
            )
        }
    };

    let title = req.title.unwrap_or_else(|| {
        let now = chrono::Utc::now();
        format!("Chat {}", now.format("%Y-%m-%d %H:%M"))
    });

    match chat_db::create_session(&pool, &title) {
        Ok(session_id) => {
            logger::info(
                LogTag::Api,
                &format!("Created chat session: {}", session_id),
            );
            success_response(CreateChatSessionResponse { session_id })
        }
        Err(e) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DB_ERROR",
            &format!("Failed to create chat session: {}", e),
            None,
        ),
    }
}

/// GET /api/ai/chat/sessions/:id - Get session with messages
async fn get_chat_session(State(_state): State<Arc<AppState>>, Path(id): Path<i64>) -> Response {
    let pool = match chat_db::get_chat_pool() {
        Some(p) => p,
        None => {
            return error_response(
                StatusCode::SERVICE_UNAVAILABLE,
                "CHAT_DB_NOT_INITIALIZED",
                "Chat database not initialized",
                None,
            )
        }
    };

    // Get session
    let session = match chat_db::get_session(&pool, id) {
        Ok(Some(s)) => s,
        Ok(None) => {
            return error_response(
                StatusCode::NOT_FOUND,
                "NOT_FOUND",
                &format!("Chat session {} not found", id),
                None,
            )
        }
        Err(e) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "DB_ERROR",
                &format!("Failed to get chat session: {}", e),
                None,
            )
        }
    };

    // Get messages
    match chat_db::get_messages(&pool, id) {
        Ok(messages) => success_response(GetChatSessionResponse { session, messages }),
        Err(e) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DB_ERROR",
            &format!("Failed to get chat messages: {}", e),
            None,
        ),
    }
}

/// DELETE /api/ai/chat/sessions/:id - Delete session
async fn delete_chat_session(State(_state): State<Arc<AppState>>, Path(id): Path<i64>) -> Response {
    let pool = match chat_db::get_chat_pool() {
        Some(p) => p,
        None => {
            return error_response(
                StatusCode::SERVICE_UNAVAILABLE,
                "CHAT_DB_NOT_INITIALIZED",
                "Chat database not initialized",
                None,
            )
        }
    };

    match chat_db::delete_session(&pool, id) {
        Ok(()) => {
            logger::info(LogTag::Api, &format!("Deleted chat session: {}", id));
            success_response(serde_json::json!({
                "message": "Chat session deleted successfully"
            }))
        }
        Err(e) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DB_ERROR",
            &format!("Failed to delete chat session: {}", e),
            None,
        ),
    }
}

/// POST /api/ai/chat/sessions/:id/summarize - Summarize session
async fn summarize_chat_session(
    State(_state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Response {
    let pool = match chat_db::get_chat_pool() {
        Some(p) => p,
        None => {
            return error_response(
                StatusCode::SERVICE_UNAVAILABLE,
                "CHAT_DB_NOT_INITIALIZED",
                "Chat database not initialized",
                None,
            )
        }
    };

    // Get messages for this session
    let messages = match chat_db::get_messages(&pool, id) {
        Ok(m) => m,
        Err(e) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "DB_ERROR",
                &format!("Failed to get messages: {}", e),
                None,
            )
        }
    };

    if messages.is_empty() {
        return error_response(
            StatusCode::BAD_REQUEST,
            "EMPTY_SESSION",
            "Cannot summarize empty chat session",
            None,
        );
    }

    // Build conversation text
    let conversation: Vec<String> = messages
        .iter()
        .map(|m| format!("{}: {}", m.role, m.content))
        .collect();
    let conversation_text = conversation.join("\n");

    // Ask LLM to summarize
    let llm_manager = match try_get_llm_manager() {
        Some(m) => m,
        None => {
            return error_response(
                StatusCode::SERVICE_UNAVAILABLE,
                "LLM_NOT_CONFIGURED",
                "LLM manager not initialized",
                None,
            )
        }
    };

    let provider_name = with_config(|cfg| cfg.ai.default_provider.clone());
    let provider = match Provider::from_str(&provider_name) {
        Some(p) => p,
        None => {
            return error_response(
                StatusCode::BAD_REQUEST,
                "INVALID_PROVIDER",
                &format!("Invalid provider: {}", provider_name),
                None,
            )
        }
    };

    // Get the model for the configured provider
    let model = get_model_for_provider(provider);

    let request = ChatRequest::new(
        model,
        vec![
            ChatMessage::system(
                "You are a helpful assistant that creates concise summaries of chat conversations."
                    .to_string(),
            ),
            ChatMessage::user(format!(
                "Please provide a brief 1-2 sentence summary of this conversation:\n\n{}",
                conversation_text
            )),
        ],
    )
    .with_temperature(0.5)
    .with_max_tokens(150);

    match llm_manager.call(provider, request).await {
        Ok(response) => {
            let summary = response.content.trim().to_string();

            // Save summary to session
            if let Err(e) = chat_db::update_session_summary(&pool, id, &summary) {
                return error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "DB_ERROR",
                    &format!("Failed to save summary: {}", e),
                    None,
                );
            }

            logger::info(LogTag::Api, &format!("Summarized chat session: {}", id));
            success_response(serde_json::json!({
                "summary": summary
            }))
        }
        Err(e) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "LLM_ERROR",
            &format!("Failed to generate summary: {}", e),
            None,
        ),
    }
}

/// POST /api/ai/chat/sessions/:id/generate-title - Generate AI title for session
async fn generate_session_title(
    State(_state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Response {
    let pool = match chat_db::get_chat_pool() {
        Some(p) => p,
        None => {
            return error_response(
                StatusCode::SERVICE_UNAVAILABLE,
                "CHAT_DB_NOT_INITIALIZED",
                "Chat database not initialized",
                None,
            )
        }
    };

    // Get messages for this session
    let messages = match chat_db::get_messages(&pool, id) {
        Ok(m) => m,
        Err(e) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "DB_ERROR",
                &format!("Failed to get messages: {}", e),
                None,
            )
        }
    };

    if messages.is_empty() {
        return error_response(
            StatusCode::BAD_REQUEST,
            "EMPTY_SESSION",
            "Cannot generate title for empty chat session",
            None,
        );
    }

    // Get the first 2-3 messages (user + assistant exchanges)
    let mut first_user_msg = String::new();
    let mut first_assistant_msg = String::new();

    for msg in messages.iter().take(5) {
        if msg.role == "user" && first_user_msg.is_empty() {
            first_user_msg = msg.content.clone();
        } else if msg.role == "assistant"
            && first_assistant_msg.is_empty()
            && !first_user_msg.is_empty()
        {
            first_assistant_msg = msg.content.clone();
            break; // We have enough context
        }
    }

    if first_user_msg.is_empty() {
        return error_response(
            StatusCode::BAD_REQUEST,
            "NO_USER_MESSAGE",
            "No user messages found in session",
            None,
        );
    }

    // Build the title generation prompt
    let assistant_part = if !first_assistant_msg.is_empty() {
        format!("\nAssistant: {}", first_assistant_msg)
    } else {
        String::new()
    };

    let prompt = format!(
        "Generate a short, descriptive title (3-8 words) for this conversation. Output only the title, no quotes or formatting.\n\nUser: {}{}

Rules:
- Keep it concise (3-8 words max)
- Focus on the main topic or intent
- Match the language of the conversation
- If it's a generic greeting, use something like \"Quick Chat\" or \"General Question\"",
        first_user_msg, assistant_part
    );

    // Call LLM to generate title
    let llm_manager = match try_get_llm_manager() {
        Some(m) => m,
        None => {
            return error_response(
                StatusCode::SERVICE_UNAVAILABLE,
                "LLM_NOT_CONFIGURED",
                "LLM manager not initialized",
                None,
            )
        }
    };

    let provider_name = with_config(|cfg| cfg.ai.default_provider.clone());
    let provider = match Provider::from_str(&provider_name) {
        Some(p) => p,
        None => {
            return error_response(
                StatusCode::BAD_REQUEST,
                "INVALID_PROVIDER",
                &format!("Invalid provider: {}", provider_name),
                None,
            )
        }
    };

    // Get the model for the configured provider
    let model = get_model_for_provider(provider);

    let request = ChatRequest::new(model, vec![ChatMessage::user(prompt)])
        .with_temperature(0.7)
        .with_max_tokens(50);

    let title = match llm_manager.call(provider, request).await {
        Ok(response) => {
            let raw_title = response.content.trim();

            // Remove quotes if present
            let cleaned_title = raw_title.trim_matches('"').trim_matches('\'').trim();

            // Ensure title is within 50 characters
            if cleaned_title.len() > 50 {
                cleaned_title.chars().take(47).collect::<String>() + "..."
            } else {
                cleaned_title.to_string()
            }
        }
        Err(e) => {
            logger::warning(
                LogTag::Api,
                &format!("Failed to generate title with LLM: {}", e),
            );
            // Fallback: use first few words of user message
            let words: Vec<&str> = first_user_msg.split_whitespace().take(5).collect();
            let fallback = words.join(" ");
            if fallback.len() > 50 {
                fallback.chars().take(47).collect::<String>() + "..."
            } else {
                fallback
            }
        }
    };

    // Update session title in database
    if let Err(e) = chat_db::update_session_title(&pool, id, &title) {
        return error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DB_ERROR",
            &format!("Failed to update session title: {}", e),
            None,
        );
    }

    logger::info(
        LogTag::Api,
        &format!("Generated title for session {}: {}", id, title),
    );

    #[derive(Serialize)]
    pub struct GenerateTitleResponse {
        pub title: String,
    }

    success_response(GenerateTitleResponse { title })
}

/// POST /api/ai/chat/confirm/:confirmation_id - Confirm/deny tool execution
async fn confirm_tool_execution(
    State(_state): State<Arc<AppState>>,
    Path(confirmation_id): Path<String>,
    Json(req): Json<ConfirmToolExecutionRequest>,
) -> Response {
    // Get chat engine
    let engine = match try_get_chat_engine() {
        Some(e) => e,
        None => {
            return error_response(
                StatusCode::SERVICE_UNAVAILABLE,
                "CHAT_NOT_INITIALIZED",
                "Chat engine not initialized",
                None,
            )
        }
    };

    // Process confirmation with optional session_id validation
    match engine
        .process_confirmation(&confirmation_id, req.approved, req.session_id)
        .await
    {
        Ok(response) => {
            logger::info(
                LogTag::Api,
                &format!(
                    "Tool execution confirmation processed: {} (approved: {})",
                    confirmation_id, req.approved
                ),
            );
            success_response(response)
        }
        Err(e) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "CHAT_ERROR",
            &format!("Failed to process confirmation: {}", e),
            None,
        ),
    }
}

// ============================================================================
// TOOLS & PERMISSIONS HANDLERS
// ============================================================================

/// GET /api/ai/tools - List available tools
async fn list_tools(State(_state): State<Arc<AppState>>) -> Response {
    // Get chat engine to access tool registry
    let engine = match try_get_chat_engine() {
        Some(e) => e,
        None => {
            return error_response(
                StatusCode::SERVICE_UNAVAILABLE,
                "CHAT_NOT_INITIALIZED",
                "Chat engine not initialized",
                None,
            )
        }
    };

    // Use the tool registry from the engine (we'll need to expose this method)
    // For now, create a temporary registry
    let registry = crate::ai::create_tool_registry();
    let tools = registry.list_definitions();

    success_response(tools)
}

/// GET /api/ai/permissions - Get tool permissions
async fn get_permissions(State(_state): State<Arc<AppState>>) -> Response {
    let permissions = with_config(|cfg| ToolPermissions {
        analysis: crate::ai::PermissionLevel::from_str(&cfg.ai.tool_permissions_analysis),
        portfolio: crate::ai::PermissionLevel::from_str(&cfg.ai.tool_permissions_portfolio),
        trading: crate::ai::PermissionLevel::from_str(&cfg.ai.tool_permissions_trading),
        config: crate::ai::PermissionLevel::from_str(&cfg.ai.tool_permissions_config),
        system: crate::ai::PermissionLevel::from_str(&cfg.ai.tool_permissions_system),
    });

    success_response(permissions)
}

/// PATCH /api/ai/permissions - Update permissions
async fn update_permissions(
    State(_state): State<Arc<AppState>>,
    Json(req): Json<ToolPermissions>,
) -> Response {
    match update_config_section(
        |cfg| {
            cfg.ai.tool_permissions_analysis = req.analysis.to_str().to_string();
            cfg.ai.tool_permissions_portfolio = req.portfolio.to_str().to_string();
            cfg.ai.tool_permissions_trading = req.trading.to_str().to_string();
            cfg.ai.tool_permissions_config = req.config.to_str().to_string();
            cfg.ai.tool_permissions_system = req.system.to_str().to_string();
        },
        true,
    ) {
        Ok(()) => {
            logger::info(LogTag::Api, "Updated AI tool permissions");
            success_response(serde_json::json!({
                "message": "Tool permissions updated successfully"
            }))
        }
        Err(e) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "CONFIG_ERROR",
            &format!("Failed to update permissions: {}", e),
            None,
        ),
    }
}

// ============================================================================
// Assistant AUTHENTICATION ROUTES
// ============================================================================

// Response Types

#[derive(Debug, Serialize)]
pub struct AssistantAuthStatusResponse {
    pub authenticated: bool,
    pub has_github_token: bool,
}

#[derive(Debug, Serialize)]
pub struct AssistantAuthStartResponse {
    pub user_code: String,
    pub verification_uri: String,
    pub device_code: String,
    pub expires_in: u64,
    pub interval: u64,
}

#[derive(Debug, Deserialize)]
pub struct AssistantAuthPollRequest {
    pub device_code: String,
}

#[derive(Debug, Serialize)]
pub struct AssistantAuthPollResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pending: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AssistantAuthLogoutResponse {
    pub success: bool,
}

#[derive(Debug, Serialize)]
pub struct AssistantAuthTestResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

// ─── Automation Types ────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct CreateAutomationTaskRequest {
    pub name: String,
    pub instruction: String,
    pub schedule_type: String,
    pub schedule_value: String,
    #[serde(default = "default_read_only")]
    pub tool_permissions: String,
    #[serde(default = "default_low")]
    pub priority: String,
    #[serde(default = "default_true")]
    pub notify_telegram: bool,
    #[serde(default = "default_true")]
    pub notify_on_success: bool,
    #[serde(default = "default_true")]
    pub notify_on_failure: bool,
    pub max_retries: Option<i32>,
    pub timeout_seconds: Option<i64>,
    pub instruction_ids: Option<String>,
}

fn default_read_only() -> String {
    "read_only".to_string()
}
fn default_low() -> String {
    "low".to_string()
}
fn default_true() -> bool {
    true
}

#[derive(Deserialize)]
pub struct UpdateAutomationTaskRequest {
    pub name: Option<String>,
    pub instruction: Option<String>,
    pub schedule_type: Option<String>,
    pub schedule_value: Option<String>,
    pub tool_permissions: Option<String>,
    pub priority: Option<String>,
    pub notify_telegram: Option<bool>,
    pub notify_on_success: Option<bool>,
    pub notify_on_failure: Option<bool>,
    pub max_retries: Option<i32>,
    pub timeout_seconds: Option<i64>,
    pub instruction_ids: Option<String>,
}

#[derive(Deserialize)]
pub struct ToggleTaskRequest {
    pub enabled: bool,
}

// Route Handlers

/// GET /api/ai/Assistant/auth/status - Check authentication status
async fn Assistant_auth_status(State(_state): State<Arc<AppState>>) -> Response {
    let has_github_token = Assistant::load_github_token().is_some();
    let has_valid_Assistant_token = Assistant::load_Assistant_token().is_some();

    // Authenticated if we have a valid Assistant token or a GitHub token that can be exchanged
    let authenticated = has_valid_Assistant_token || has_github_token;

    logger::debug(
        LogTag::Api,
        &format!(
            "[Assistant] Auth status: authenticated={}, has_github_token={}, has_Assistant_token={}",
            authenticated, has_github_token, has_valid_Assistant_token
        ),
    );

    success_response(AssistantAuthStatusResponse {
        authenticated,
        has_github_token,
    })
}

/// POST /api/ai/Assistant/auth/start - Start OAuth device flow
async fn Assistant_auth_start(State(_state): State<Arc<AppState>>) -> Response {
    logger::info(LogTag::Api, "[Assistant] Starting OAuth device flow");

    match Assistant::request_device_code().await {
        Ok(device_code_response) => {
            // Store device code for polling
            if let Ok(mut storage) = DEVICE_CODE_STORAGE.write() {
                *storage = Some(device_code_response.device_code.clone());
            }

            logger::info(
                LogTag::Api,
                &format!(
                    "[Assistant] Device code obtained. User code: {}",
                    device_code_response.user_code
                ),
            );

            success_response(AssistantAuthStartResponse {
                user_code: device_code_response.user_code,
                verification_uri: device_code_response.verification_uri,
                device_code: device_code_response.device_code,
                expires_in: device_code_response.expires_in,
                interval: device_code_response.interval,
            })
        }
        Err(e) => {
            logger::error(
                LogTag::Api,
                &format!("[Assistant] Failed to start OAuth flow: {}", e),
            );
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "OAUTH_START_FAILED",
                &e,
                None,
            )
        }
    }
}

/// POST /api/ai/Assistant/auth/poll - Poll for OAuth authorization
async fn Assistant_auth_poll(
    State(_state): State<Arc<AppState>>,
    Json(req): Json<AssistantAuthPollRequest>,
) -> Response {
    logger::debug(LogTag::Api, "[Assistant] Polling for OAuth authorization");

    match Assistant::poll_for_access_token(&req.device_code).await {
        Ok(Some(access_token)) => {
            logger::info(LogTag::Api, "[Assistant] User authorized! Got access token");

            // Save GitHub token
            if let Err(e) = Assistant::save_github_token(&access_token) {
                logger::error(
                    LogTag::Api,
                    &format!("[Assistant] Failed to save GitHub token: {}", e),
                );
                return error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "TOKEN_SAVE_FAILED",
                    &e,
                    None,
                );
            }

            // Exchange for Assistant token
            match Assistant::exchange_for_Assistant_token(&access_token).await {
                Ok(Assistant_token) => {
                    // Save Assistant token
                    if let Err(e) = Assistant::save_Assistant_token(&Assistant_token) {
                        logger::error(
                            LogTag::Api,
                            &format!("[Assistant] Failed to save Assistant token: {}", e),
                        );
                        return error_response(
                            StatusCode::INTERNAL_SERVER_ERROR,
                            "TOKEN_SAVE_FAILED",
                            &e,
                            None,
                        );
                    }

                    // Clear stored device code
                    if let Ok(mut storage) = DEVICE_CODE_STORAGE.write() {
                        *storage = None;
                    }

                    logger::info(
                        LogTag::Api,
                        "[Assistant] OAuth flow complete! Assistant token saved",
                    );

                    success_response(AssistantAuthPollResponse {
                        success: true,
                        pending: None,
                        error: None,
                    })
                }
                Err(e) => {
                    logger::error(
                        LogTag::Api,
                        &format!("[Assistant] Failed to exchange for Assistant token: {}", e),
                    );
                    error_response(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "TOKEN_EXCHANGE_FAILED",
                        &e,
                        None,
                    )
                }
            }
        }
        Ok(None) => {
            // Still pending
            logger::debug(LogTag::Api, "[Assistant] Authorization still pending");
            success_response(AssistantAuthPollResponse {
                success: false,
                pending: Some(true),
                error: None,
            })
        }
        Err(e) => {
            logger::error(LogTag::Api, &format!("[Assistant] OAuth poll error: {}", e));
            success_response(AssistantAuthPollResponse {
                success: false,
                pending: None,
                error: Some(e),
            })
        }
    }
}

/// POST /api/ai/Assistant/auth/logout - Remove saved tokens
async fn Assistant_auth_logout(State(_state): State<Arc<AppState>>) -> Response {
    logger::info(LogTag::Api, "[Assistant] Logging out - removing tokens");

    let github_path = Assistant::get_github_token_path();
    let Assistant_path = Assistant::get_Assistant_token_path();

    // Remove GitHub token file
    if github_path.exists() {
        if let Err(e) = std::fs::remove_file(&github_path) {
            logger::error(
                LogTag::Api,
                &format!("[Assistant] Failed to remove GitHub token file: {}", e),
            );
        } else {
            logger::info(LogTag::Api, "[Assistant] Removed GitHub token file");
        }
    }

    // Remove Assistant token file
    if Assistant_path.exists() {
        if let Err(e) = std::fs::remove_file(&Assistant_path) {
            logger::error(
                LogTag::Api,
                &format!("[Assistant] Failed to remove Assistant token file: {}", e),
            );
        } else {
            logger::info(LogTag::Api, "[Assistant] Removed Assistant token file");
        }
    }

    // Clear stored device code
    if let Ok(mut storage) = DEVICE_CODE_STORAGE.write() {
        *storage = None;
    }

    logger::info(LogTag::Api, "[Assistant] Logout complete");

    success_response(AssistantAuthLogoutResponse { success: true })
}

/// POST /api/ai/Assistant/auth/test - Test if authentication works
async fn Assistant_auth_test(State(_state): State<Arc<AppState>>) -> Response {
    logger::info(LogTag::Api, "[Assistant] Testing authentication");

    match Assistant::get_valid_Assistant_token().await {
        Ok(token) => {
            logger::info(
                LogTag::Api,
                &format!(
                    "[Assistant] Authentication test successful. API base: {}",
                    token.api_base
                ),
            );
            success_response(AssistantAuthTestResponse {
                success: true,
                error: None,
            })
        }
        Err(e) => {
            logger::error(
                LogTag::Api,
                &format!("[Assistant] Authentication test failed: {}", e),
            );
            success_response(AssistantAuthTestResponse {
                success: false,
                error: Some(e),
            })
        }
    }
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Get the appropriate model for a provider from config
fn get_model_for_provider(provider: Provider) -> String {
    with_config(|cfg| {
        let provider_config = match provider {
            Provider::OpenAi => &cfg.ai.providers.openai,
            Provider::Anthropic => &cfg.ai.providers.anthropic,
            Provider::Groq => &cfg.ai.providers.groq,
            Provider::DeepSeek => &cfg.ai.providers.deepseek,
            Provider::Gemini => &cfg.ai.providers.gemini,
            Provider::Together => &cfg.ai.providers.together,
            Provider::OpenRouter => &cfg.ai.providers.openrouter,
            Provider::Mistral => &cfg.ai.providers.mistral,
            Provider::Assistant => &cfg.ai.providers.Assistant,
            Provider::Ollama => {
                return cfg.ai.providers.ollama.model.clone();
            }
        };

        if !provider_config.model.is_empty() {
            provider_config.model.clone()
        } else {
            // Default models for each provider
            match provider {
                Provider::OpenAi => "gpt-4".to_string(),
                Provider::Anthropic => "claude-3-5-sonnet-20241022".to_string(),
                Provider::Groq => "llama-3.1-70b-versatile".to_string(),
                Provider::DeepSeek => "deepseek-chat".to_string(),
                Provider::Gemini => "gemini-pro".to_string(),
                Provider::Ollama => "llama3.2".to_string(),
                Provider::Together => "meta-llama/Llama-3-70b-chat-hf".to_string(),
                Provider::OpenRouter => "openai/gpt-4".to_string(),
                Provider::Mistral => "mistral-large-latest".to_string(),
                Provider::Assistant => "gpt-4o".to_string(),
            }
        }
    })
}

// ============================================================================
// AUTOMATION HANDLERS
// ============================================================================

/// GET /api/ai/automation — List all scheduled tasks
async fn list_automation_tasks() -> Response {
    let pool = match crate::ai::chat_db::get_chat_pool() {
        Some(p) => p,
        None => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "DB_ERROR",
                "Database not initialized",
                None,
            )
        }
    };

    match crate::ai::scheduled_db::list_tasks(&pool) {
        Ok(tasks) => success_response(serde_json::json!({ "tasks": tasks })),
        Err(e) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DB_ERROR",
            &format!("Failed to list tasks: {}", e),
            None,
        ),
    }
}

/// POST /api/ai/automation — Create a new scheduled task
async fn create_automation_task(Json(req): Json<CreateAutomationTaskRequest>) -> Response {
    let pool = match crate::ai::chat_db::get_chat_pool() {
        Some(p) => p,
        None => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "DB_ERROR",
                "Database not initialized",
                None,
            )
        }
    };

    // Validate name is not empty
    if req.name.trim().is_empty() {
        return error_response(
            StatusCode::BAD_REQUEST,
            "INVALID_NAME",
            "Task name cannot be empty",
            None,
        );
    }

    // Validate instruction is not empty
    if req.instruction.trim().is_empty() {
        return error_response(
            StatusCode::BAD_REQUEST,
            "INVALID_INSTRUCTION",
            "Task instruction cannot be empty",
            None,
        );
    }

    // Validate schedule type
    if !["interval", "daily", "weekly"].contains(&req.schedule_type.as_str()) {
        return error_response(
            StatusCode::BAD_REQUEST,
            "INVALID_SCHEDULE_TYPE",
            "Invalid schedule_type. Must be: interval, daily, or weekly",
            None,
        );
    }

    // Validate schedule value
    if let Err(e) =
        crate::ai::scheduled_db::calculate_next_run(&req.schedule_type, &req.schedule_value, None)
    {
        return error_response(
            StatusCode::BAD_REQUEST,
            "INVALID_SCHEDULE",
            &format!("Invalid schedule_value: {}", e),
            None,
        );
    }

    match crate::ai::scheduled_db::create_task(
        &pool,
        &req.name,
        &req.instruction,
        &req.schedule_type,
        &req.schedule_value,
        Some(&req.tool_permissions),
        Some(&req.priority),
    ) {
        Ok(id) => {
            // Update optional fields that aren't part of create_task
            if let Err(e) = crate::ai::scheduled_db::update_task(
                &pool,
                id,
                None,
                None,
                req.instruction_ids.as_ref().map(|s| Some(s.as_str())),
                None,
                None,
                None,
                None,
                Some(req.notify_telegram),
                Some(req.notify_on_success),
                Some(req.notify_on_failure),
                req.max_retries,
                req.timeout_seconds,
            ) {
                crate::logger::warning(
                    crate::logger::LogTag::System,
                    &format!("Failed to update optional fields for task {}: {}", id, e),
                );
            }

            match crate::ai::scheduled_db::get_task(&pool, id) {
                Ok(Some(task)) => success_response(serde_json::json!({ "task": task })),
                _ => success_response(serde_json::json!({ "id": id })),
            }
        }
        Err(e) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DB_ERROR",
            &format!("Failed to create task: {}", e),
            None,
        ),
    }
}

/// GET /api/ai/automation/:id — Get a specific task
async fn get_automation_task(Path(id): Path<i64>) -> Response {
    let pool = match crate::ai::chat_db::get_chat_pool() {
        Some(p) => p,
        None => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "DB_ERROR",
                "Database not initialized",
                None,
            )
        }
    };

    match crate::ai::scheduled_db::get_task(&pool, id) {
        Ok(Some(task)) => success_response(serde_json::json!({ "task": task })),
        Ok(None) => error_response(StatusCode::NOT_FOUND, "NOT_FOUND", "Task not found", None),
        Err(e) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DB_ERROR",
            &format!("Failed to get task: {}", e),
            None,
        ),
    }
}

/// PATCH /api/ai/automation/:id — Update a task
async fn update_automation_task(
    Path(id): Path<i64>,
    Json(req): Json<UpdateAutomationTaskRequest>,
) -> Response {
    let pool = match crate::ai::chat_db::get_chat_pool() {
        Some(p) => p,
        None => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "DB_ERROR",
                "Database not initialized",
                None,
            )
        }
    };

    // Validate schedule if provided
    if let (Some(st), Some(sv)) = (&req.schedule_type, &req.schedule_value) {
        if let Err(e) = crate::ai::scheduled_db::calculate_next_run(st, sv, None) {
            return error_response(
                StatusCode::BAD_REQUEST,
                "INVALID_SCHEDULE",
                &format!("Invalid schedule: {}", e),
                None,
            );
        }
    }

    // Validate tool_permissions if provided
    if let Some(tp) = &req.tool_permissions {
        if !["full", "readonly"].contains(&tp.as_str()) {
            return error_response(
                StatusCode::BAD_REQUEST,
                "INVALID_TOOL_PERMISSIONS",
                "tool_permissions must be 'full' or 'readonly'",
                None,
            );
        }
    }

    // Validate priority if provided
    if let Some(p) = &req.priority {
        if !["low", "medium", "high"].contains(&p.as_str()) {
            return error_response(
                StatusCode::BAD_REQUEST,
                "INVALID_PRIORITY",
                "priority must be 'low', 'medium', or 'high'",
                None,
            );
        }
    }

    match crate::ai::scheduled_db::update_task(
        &pool,
        id,
        req.name.as_deref(),
        req.instruction.as_deref(),
        req.instruction_ids.as_ref().map(|s| Some(s.as_str())),
        req.schedule_type.as_deref(),
        req.schedule_value.as_deref(),
        req.tool_permissions.as_deref(),
        req.priority.as_deref(),
        req.notify_telegram,
        req.notify_on_success,
        req.notify_on_failure,
        req.max_retries,
        req.timeout_seconds,
    ) {
        Ok(_) => match crate::ai::scheduled_db::get_task(&pool, id) {
            Ok(Some(task)) => success_response(serde_json::json!({ "task": task })),
            _ => success_response(serde_json::json!({ "updated": true })),
        },
        Err(e) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DB_ERROR",
            &format!("Failed to update task: {}", e),
            None,
        ),
    }
}

/// DELETE /api/ai/automation/:id — Delete a task
async fn delete_automation_task(Path(id): Path<i64>) -> Response {
    let pool = match crate::ai::chat_db::get_chat_pool() {
        Some(p) => p,
        None => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "DB_ERROR",
                "Database not initialized",
                None,
            )
        }
    };

    // Check if task has a running execution
    match crate::ai::scheduled_db::list_runs_for_task(&pool, id, 1) {
        Ok(runs) if !runs.is_empty() && runs[0].status == "running" => {
            return error_response(
                StatusCode::CONFLICT,
                "TASK_RUNNING",
                "Cannot delete task while it is running",
                None,
            );
        }
        _ => {}
    }

    match crate::ai::scheduled_db::delete_task(&pool, id) {
        Ok(_) => success_response(serde_json::json!({ "deleted": true })),
        Err(e) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DB_ERROR",
            &format!("Failed to delete task: {}", e),
            None,
        ),
    }
}

/// POST /api/ai/automation/:id/toggle — Enable/disable a task
async fn toggle_automation_task(
    Path(id): Path<i64>,
    Json(req): Json<ToggleTaskRequest>,
) -> Response {
    let pool = match crate::ai::chat_db::get_chat_pool() {
        Some(p) => p,
        None => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "DB_ERROR",
                "Database not initialized",
                None,
            )
        }
    };

    match crate::ai::scheduled_db::toggle_task(&pool, id, req.enabled) {
        Ok(_) => match crate::ai::scheduled_db::get_task(&pool, id) {
            Ok(Some(task)) => success_response(serde_json::json!({ "task": task })),
            _ => success_response(serde_json::json!({ "toggled": true })),
        },
        Err(e) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DB_ERROR",
            &format!("Failed to toggle task: {}", e),
            None,
        ),
    }
}

/// POST /api/ai/automation/:id/run — Trigger immediate execution
async fn run_automation_task(Path(id): Path<i64>) -> Response {
    let pool = match crate::ai::chat_db::get_chat_pool() {
        Some(p) => p,
        None => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "DB_ERROR",
                "Database not initialized",
                None,
            )
        }
    };

    let task = match crate::ai::scheduled_db::get_task(&pool, id) {
        Ok(Some(t)) => t,
        Ok(None) => {
            return error_response(StatusCode::NOT_FOUND, "NOT_FOUND", "Task not found", None)
        }
        Err(e) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "DB_ERROR",
                &format!("Failed to get task: {}", e),
                None,
            )
        }
    };

    // Don't allow running disabled tasks
    if !task.enabled {
        return error_response(
            StatusCode::BAD_REQUEST,
            "TASK_DISABLED",
            "Cannot run a disabled task",
            None,
        );
    }

    // Check if task is already running
    match crate::ai::scheduled_db::list_runs_for_task(&pool, id, 1) {
        Ok(runs) if !runs.is_empty() && runs[0].status == "running" => {
            return error_response(
                StatusCode::CONFLICT,
                "TASK_RUNNING",
                "Task is already running",
                None,
            );
        }
        _ => {}
    }

    // Execute in background
    tokio::spawn(async move {
        let pool = match crate::ai::chat_db::get_chat_pool() {
            Some(p) => p,
            None => {
                crate::logger::warning(
                    crate::logger::LogTag::System,
                    "Failed to get DB pool for manual task execution",
                );
                return;
            }
        };
        let timeout = if task.timeout_seconds > 0 {
            task.timeout_seconds as u64
        } else {
            120
        };
        if let Err(e) = crate::services::implementations::scheduled_ai_tasks_service::execute_scheduled_task_public(
            &pool, &task, timeout
        ).await {
            crate::logger::warning(crate::logger::LogTag::System, &format!("Manual task execution failed for '{}': {}", task.name, e));
        }
    });

    success_response(serde_json::json!({ "triggered": true, "task_id": id }))
}

/// GET /api/ai/automation/:id/runs — Get run history for a task
async fn get_automation_task_runs(Path(id): Path<i64>) -> Response {
    let pool = match crate::ai::chat_db::get_chat_pool() {
        Some(p) => p,
        None => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "DB_ERROR",
                "Database not initialized",
                None,
            )
        }
    };

    match crate::ai::scheduled_db::list_runs_for_task(&pool, id, 50) {
        Ok(runs) => success_response(serde_json::json!({ "runs": runs })),
        Err(e) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DB_ERROR",
            &format!("Failed to list runs: {}", e),
            None,
        ),
    }
}

/// GET /api/ai/automation/runs — Get all recent runs
async fn get_automation_recent_runs() -> Response {
    let pool = match crate::ai::chat_db::get_chat_pool() {
        Some(p) => p,
        None => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "DB_ERROR",
                "Database not initialized",
                None,
            )
        }
    };

    match crate::ai::scheduled_db::list_recent_runs(&pool, 100) {
        Ok(runs) => success_response(serde_json::json!({ "runs": runs })),
        Err(e) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DB_ERROR",
            &format!("Failed to list recent runs: {}", e),
            None,
        ),
    }
}

/// GET /api/ai/automation/runs/:id — Get a specific run
async fn get_automation_run_detail(Path(run_id): Path<i64>) -> Response {
    let pool = match crate::ai::chat_db::get_chat_pool() {
        Some(p) => p,
        None => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "DB_ERROR",
                "Database not initialized",
                None,
            )
        }
    };

    match crate::ai::scheduled_db::get_run(&pool, run_id) {
        Ok(Some(run)) => success_response(serde_json::json!({ "run": run })),
        Ok(None) => error_response(StatusCode::NOT_FOUND, "NOT_FOUND", "Run not found", None),
        Err(e) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DB_ERROR",
            &format!("Failed to get run: {}", e),
            None,
        ),
    }
}

/// GET /api/ai/automation/stats — Aggregated automation statistics
async fn get_automation_stats_handler() -> Response {
    let pool = match crate::ai::chat_db::get_chat_pool() {
        Some(p) => p,
        None => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "DB_ERROR",
                "Database not initialized",
                None,
            )
        }
    };

    match crate::ai::scheduled_db::get_automation_stats(&pool) {
        Ok(stats) => success_response(serde_json::json!({ "stats": stats })),
        Err(e) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DB_ERROR",
            &format!("Failed to get stats: {}", e),
            None,
        ),
    }
}
