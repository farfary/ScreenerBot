//! Outbound LLM provider handlers (`/api/llm`).

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Response,
    Json,
};
use std::sync::Arc;

use crate::apis::llm::{try_get_llm_manager, ChatMessage, ChatRequest, Provider};
use crate::config::{update_config_section, with_config, Config};
use crate::logger::{self, LogTag};
use crate::webserver::state::AppState;
use crate::webserver::utils::{error_response, success_response};

use super::types::*;

/// GET /api/llm/providers - List all providers with status
pub async fn list_providers(State(_state): State<Arc<AppState>>) -> Response {
    // Return promotional fixtures only for owner-initiated media capture.
    if crate::webserver::promo::are_promo_fixtures_enabled() {
        return success_response(crate::webserver::promo::get_promo_providers());
    }

    let config = with_config(|cfg| cfg.llm.clone());

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
        id: "ollama".to_owned(),
        name: "Ollama (Local)".to_owned(),
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

/// GET /api/llm/config - Master LLM configuration (enable switch + default provider).
///
/// Returns only the two master fields. Provider credentials are never included;
/// they are read through `GET /api/llm/providers`.
pub async fn get_config(State(_state): State<Arc<AppState>>) -> Response {
    let response = with_config(|cfg| LlmConfigResponse {
        enabled: cfg.llm.enabled,
        default_provider: cfg.llm.default_provider.clone(),
    });
    success_response(response)
}

/// PATCH /api/llm/config - Update the master LLM enable switch and/or default provider.
///
/// This owner touches `cfg.llm.enabled` and `cfg.llm.default_provider` only.
/// Analysis behaviour lives at `/api/llm-analysis/config`; provider credentials
/// at `/api/llm/providers/:provider`.
pub async fn update_config(
    State(_state): State<Arc<AppState>>,
    Json(req): Json<UpdateLlmConfigRequest>,
) -> Response {
    match update_config_section(|cfg| apply_llm_config_update(cfg, &req), true) {
        Ok(()) => {
            logger::info(LogTag::Api, "Master LLM configuration updated via API");
            success_response(serde_json::json!({
                "message": "LLM configuration updated successfully"
            }))
        }
        Err(e) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "CONFIG_UPDATE_FAILED",
            &format!("Failed to update LLM config: {e}"),
            None,
        ),
    }
}

/// Apply a master-LLM patch in place. Split out so the ownership boundary — this
/// mutates `cfg.llm` master fields and nothing else — is unit-testable without a
/// running server. An unknown `default_provider` is ignored, matching the
/// per-provider update handler.
fn apply_llm_config_update(cfg: &mut Config, req: &UpdateLlmConfigRequest) {
    if let Some(enabled) = req.enabled {
        cfg.llm.enabled = enabled;
    }
    if let Some(provider) = req.default_provider.as_deref() {
        if Provider::from_str(provider).is_some() {
            cfg.llm.default_provider = provider.to_owned();
        }
    }
}

/// POST /api/llm/providers/:provider/test - Test a specific provider
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
                &format!("Unknown provider: {provider_name}"),
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
                &format!("Provider '{provider_name}' is not configured or disabled"),
                None,
            );
        }
    };

    // Get model from config
    let model = with_config(|cfg| {
        let provider_cfg = match provider {
            Provider::OpenAi => &cfg.llm.providers.openai,
            Provider::Anthropic => &cfg.llm.providers.anthropic,
            Provider::Groq => &cfg.llm.providers.groq,
            Provider::DeepSeek => &cfg.llm.providers.deepseek,
            Provider::Gemini => &cfg.llm.providers.gemini,
            Provider::Together => &cfg.llm.providers.together,
            Provider::OpenRouter => &cfg.llm.providers.openrouter,
            Provider::Mistral => &cfg.llm.providers.mistral,
            Provider::Ollama => {
                return cfg.llm.providers.ollama.model.clone();
            }
        };

        if !provider_cfg.model.is_empty() {
            provider_cfg.model.clone()
        } else {
            // Default models
            match provider {
                Provider::OpenAi => "gpt-4".to_owned(),
                Provider::Anthropic => "claude-3-5-sonnet-20241022".to_owned(),
                Provider::Groq => "llama-3.1-70b-versatile".to_owned(),
                Provider::DeepSeek => "deepseek-chat".to_owned(),
                Provider::Gemini => "gemini-pro".to_owned(),
                Provider::Together => "meta-llama/Llama-3-70b-chat-hf".to_owned(),
                Provider::OpenRouter => "openai/gpt-4".to_owned(),
                Provider::Mistral => "mistral-large-latest".to_owned(),
                Provider::Ollama => "llama3.2".to_owned(),
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
                format!(
                    "{}...",
                    response.content.chars().take(100).collect::<String>()
                )
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
                &format!("AI provider '{provider_name}' test failed: {e}"),
            );

            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "PROVIDER_TEST_FAILED",
                &format!("Provider test failed: {e}"),
                None,
            )
        }
    }
}

/// PATCH /api/llm/providers/:provider - Update a specific provider's configuration
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
                &format!("Unknown provider: {provider_name}"),
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
                        cfg.llm.providers.openai.enabled = enabled;
                    }
                    if let Some(ref api_key) = req.api_key {
                        if !api_key.is_empty() {
                            cfg.llm.providers.openai.api_key = api_key.clone();
                        }
                    }
                    if let Some(ref model) = req.model {
                        cfg.llm.providers.openai.model = model.clone();
                    }
                    if let Some(rate_limit) = req.rate_limit_per_minute {
                        cfg.llm.providers.openai.rate_limit_per_minute = rate_limit;
                    }
                }
                Provider::Anthropic => {
                    if let Some(enabled) = req.enabled {
                        cfg.llm.providers.anthropic.enabled = enabled;
                    }
                    if let Some(ref api_key) = req.api_key {
                        if !api_key.is_empty() {
                            cfg.llm.providers.anthropic.api_key = api_key.clone();
                        }
                    }
                    if let Some(ref model) = req.model {
                        cfg.llm.providers.anthropic.model = model.clone();
                    }
                    if let Some(rate_limit) = req.rate_limit_per_minute {
                        cfg.llm.providers.anthropic.rate_limit_per_minute = rate_limit;
                    }
                }
                Provider::Groq => {
                    if let Some(enabled) = req.enabled {
                        cfg.llm.providers.groq.enabled = enabled;
                    }
                    if let Some(ref api_key) = req.api_key {
                        if !api_key.is_empty() {
                            cfg.llm.providers.groq.api_key = api_key.clone();
                        }
                    }
                    if let Some(ref model) = req.model {
                        cfg.llm.providers.groq.model = model.clone();
                    }
                    if let Some(rate_limit) = req.rate_limit_per_minute {
                        cfg.llm.providers.groq.rate_limit_per_minute = rate_limit;
                    }
                }
                Provider::DeepSeek => {
                    if let Some(enabled) = req.enabled {
                        cfg.llm.providers.deepseek.enabled = enabled;
                    }
                    if let Some(ref api_key) = req.api_key {
                        if !api_key.is_empty() {
                            cfg.llm.providers.deepseek.api_key = api_key.clone();
                        }
                    }
                    if let Some(ref model) = req.model {
                        cfg.llm.providers.deepseek.model = model.clone();
                    }
                    if let Some(rate_limit) = req.rate_limit_per_minute {
                        cfg.llm.providers.deepseek.rate_limit_per_minute = rate_limit;
                    }
                }
                Provider::Gemini => {
                    if let Some(enabled) = req.enabled {
                        cfg.llm.providers.gemini.enabled = enabled;
                    }
                    if let Some(ref api_key) = req.api_key {
                        if !api_key.is_empty() {
                            cfg.llm.providers.gemini.api_key = api_key.clone();
                        }
                    }
                    if let Some(ref model) = req.model {
                        cfg.llm.providers.gemini.model = model.clone();
                    }
                    if let Some(rate_limit) = req.rate_limit_per_minute {
                        cfg.llm.providers.gemini.rate_limit_per_minute = rate_limit;
                    }
                }
                Provider::Together => {
                    if let Some(enabled) = req.enabled {
                        cfg.llm.providers.together.enabled = enabled;
                    }
                    if let Some(ref api_key) = req.api_key {
                        if !api_key.is_empty() {
                            cfg.llm.providers.together.api_key = api_key.clone();
                        }
                    }
                    if let Some(ref model) = req.model {
                        cfg.llm.providers.together.model = model.clone();
                    }
                    if let Some(rate_limit) = req.rate_limit_per_minute {
                        cfg.llm.providers.together.rate_limit_per_minute = rate_limit;
                    }
                }
                Provider::OpenRouter => {
                    if let Some(enabled) = req.enabled {
                        cfg.llm.providers.openrouter.enabled = enabled;
                    }
                    if let Some(ref api_key) = req.api_key {
                        if !api_key.is_empty() {
                            cfg.llm.providers.openrouter.api_key = api_key.clone();
                        }
                    }
                    if let Some(ref model) = req.model {
                        cfg.llm.providers.openrouter.model = model.clone();
                    }
                    if let Some(rate_limit) = req.rate_limit_per_minute {
                        cfg.llm.providers.openrouter.rate_limit_per_minute = rate_limit;
                    }
                }
                Provider::Mistral => {
                    if let Some(enabled) = req.enabled {
                        cfg.llm.providers.mistral.enabled = enabled;
                    }
                    if let Some(ref api_key) = req.api_key {
                        if !api_key.is_empty() {
                            cfg.llm.providers.mistral.api_key = api_key.clone();
                        }
                    }
                    if let Some(ref model) = req.model {
                        cfg.llm.providers.mistral.model = model.clone();
                    }
                    if let Some(rate_limit) = req.rate_limit_per_minute {
                        cfg.llm.providers.mistral.rate_limit_per_minute = rate_limit;
                    }
                }
                Provider::Ollama => {
                    if let Some(enabled) = req.enabled {
                        cfg.llm.providers.ollama.enabled = enabled;
                    }
                    if let Some(ref model) = req.model {
                        cfg.llm.providers.ollama.model = model.clone();
                    }
                    if let Some(rate_limit) = req.rate_limit_per_minute {
                        cfg.llm.providers.ollama.rate_limit_per_minute = rate_limit;
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
                &format!("Updated AI provider '{provider_name}' configuration"),
            );
            success_response(serde_json::json!({
                "provider": provider_name,
                "updated": true
            }))
        }
        Err(e) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "CONFIG_UPDATE_FAILED",
            &format!("Failed to update provider config: {e}"),
            None,
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn master_patch_touches_only_llm_master_fields() {
        let mut cfg = Config::default();
        cfg.llm_analysis.filtering_enabled = true;
        cfg.llm.providers.openai.api_key = "sk-existing".to_owned();
        let before_analysis = serde_json::to_value(&cfg.llm_analysis).unwrap();
        let before_providers = serde_json::to_value(&cfg.llm.providers).unwrap();

        apply_llm_config_update(
            &mut cfg,
            &UpdateLlmConfigRequest {
                enabled: Some(true),
                default_provider: Some("anthropic".to_owned()),
            },
        );

        assert!(cfg.llm.enabled);
        assert_eq!(cfg.llm.default_provider, "anthropic");
        // The other owners' config is untouched.
        assert_eq!(
            serde_json::to_value(&cfg.llm_analysis).unwrap(),
            before_analysis
        );
        assert_eq!(
            serde_json::to_value(&cfg.llm.providers).unwrap(),
            before_providers
        );
    }

    #[test]
    fn master_patch_ignores_unknown_default_provider() {
        let mut cfg = Config::default();
        let original = cfg.llm.default_provider.clone();
        apply_llm_config_update(
            &mut cfg,
            &UpdateLlmConfigRequest {
                enabled: None,
                default_provider: Some("not-a-provider".to_owned()),
            },
        );
        assert_eq!(cfg.llm.default_provider, original);
    }

    #[test]
    fn config_response_carries_no_provider_or_secret_fields() {
        let json = serde_json::to_value(LlmConfigResponse {
            enabled: true,
            default_provider: "openai".to_owned(),
        })
        .unwrap();
        let obj = json.as_object().unwrap();
        assert_eq!(obj.len(), 2);
        assert!(obj.contains_key("enabled"));
        assert!(obj.contains_key("default_provider"));
        assert!(!obj.contains_key("providers"));
        assert!(!obj.contains_key("api_key"));
    }

    #[test]
    fn provider_status_never_serializes_an_api_key() {
        let json = serde_json::to_value(ProvidersListResponse {
            providers: vec![ProviderStatus {
                id: "openai".to_owned(),
                name: "OpenAI".to_owned(),
                enabled: true,
                has_api_key: true,
                model: "gpt-4o".to_owned(),
                rate_limit_per_minute: 60,
            }],
            default_provider: "openai".to_owned(),
        })
        .unwrap();
        let provider = json["providers"][0].as_object().unwrap();
        // The boolean presence flag is exposed; the secret itself never is.
        assert!(provider.contains_key("has_api_key"));
        assert!(!provider.contains_key("api_key"));
        assert!(!provider
            .keys()
            .any(|k| k.contains("key") && k != "has_api_key"));
    }
}
