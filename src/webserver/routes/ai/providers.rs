//! AI provider management handlers

use axum::{extract::{Path, State}, http::StatusCode, response::Response, Json};
use std::sync::Arc;

use crate::apis::llm::{try_get_llm_manager, ChatMessage, ChatRequest, Provider};
use crate::config::{update_config_section, with_config};
use crate::logger::{self, LogTag};
use crate::webserver::state::AppState;
use crate::webserver::utils::{error_response, success_response};

use super::types::*;

/// GET /api/ai/providers - List all providers with status
pub async fn list_providers(State(_state): State<Arc<AppState>>) -> Response {
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
pub async fn test_provider(
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

            let preview = if response.content.chars().count() > 100 {
                format!("{}...", response.content.chars().take(100).collect::<String>())
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
pub async fn update_provider(
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
