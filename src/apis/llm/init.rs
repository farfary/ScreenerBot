//! Builds the LlmManager's providers from `[ai.providers]` config.

use crate::logger::{self, LogTag};

/// Initialize LLM providers based on configuration.
pub async fn init_providers_from_config() -> Result<(), String> {
    use crate::apis::llm::{init_llm_manager, LlmManager};
    use crate::config::with_config;

    let mut llm_manager = LlmManager::new();
    let mut enabled_providers = Vec::new();

    // Helper to get model option
    let get_model = |model_str: &str| -> Option<String> {
        if model_str.is_empty() || model_str == "auto" {
            None
        } else {
            Some(model_str.to_string())
        }
    };

    with_config(|cfg| {
        // OpenRouter (has extra parameters for site_url and site_name)
        if cfg.ai.providers.openrouter.enabled && !cfg.ai.providers.openrouter.api_key.is_empty() {
            use crate::apis::llm::openrouter::OpenRouterClient;
            let model = get_model(&cfg.ai.providers.openrouter.model);
            match OpenRouterClient::new(
                cfg.ai.providers.openrouter.api_key.clone(),
                model,
                cfg.ai.providers.openrouter.enabled,
                None, // site_url - would need to be added to config
                None, // site_name - would need to be added to config
            ) {
                Ok(client) => {
                    llm_manager.set_openrouter(std::sync::Arc::new(client));
                    enabled_providers.push("OpenRouter");
                }
                Err(e) => {
                    logger::warning(
                        LogTag::System,
                        &format!("Failed to initialize OpenRouter: {e}"),
                    );
                }
            }
        }

        // OpenAI
        if cfg.ai.providers.openai.enabled && !cfg.ai.providers.openai.api_key.is_empty() {
            use crate::apis::llm::openai::OpenAiClient;
            let model = get_model(&cfg.ai.providers.openai.model);
            match OpenAiClient::new(
                cfg.ai.providers.openai.api_key.clone(),
                model,
                cfg.ai.providers.openai.enabled,
            ) {
                Ok(client) => {
                    llm_manager.set_openai(std::sync::Arc::new(client));
                    enabled_providers.push("OpenAI");
                }
                Err(e) => {
                    logger::warning(LogTag::System, &format!("Failed to initialize OpenAI: {e}"));
                }
            }
        }

        // Anthropic
        if cfg.ai.providers.anthropic.enabled && !cfg.ai.providers.anthropic.api_key.is_empty() {
            use crate::apis::llm::anthropic::AnthropicClient;
            let model = get_model(&cfg.ai.providers.anthropic.model);
            match AnthropicClient::new(
                cfg.ai.providers.anthropic.api_key.clone(),
                model,
                cfg.ai.providers.anthropic.enabled,
            ) {
                Ok(client) => {
                    llm_manager.set_anthropic(std::sync::Arc::new(client));
                    enabled_providers.push("Anthropic");
                }
                Err(e) => {
                    logger::warning(
                        LogTag::System,
                        &format!("Failed to initialize Anthropic: {e}"),
                    );
                }
            }
        }

        // Groq
        if cfg.ai.providers.groq.enabled && !cfg.ai.providers.groq.api_key.is_empty() {
            use crate::apis::llm::groq::GroqClient;
            let model = get_model(&cfg.ai.providers.groq.model);
            match GroqClient::new(
                cfg.ai.providers.groq.api_key.clone(),
                model,
                cfg.ai.providers.groq.enabled,
            ) {
                Ok(client) => {
                    llm_manager.set_groq(std::sync::Arc::new(client));
                    enabled_providers.push("Groq");
                }
                Err(e) => {
                    logger::warning(LogTag::System, &format!("Failed to initialize Groq: {e}"));
                }
            }
        }

        // DeepSeek
        if cfg.ai.providers.deepseek.enabled && !cfg.ai.providers.deepseek.api_key.is_empty() {
            use crate::apis::llm::deepseek::DeepSeekClient;
            let model = get_model(&cfg.ai.providers.deepseek.model);
            match DeepSeekClient::new(
                cfg.ai.providers.deepseek.api_key.clone(),
                model,
                cfg.ai.providers.deepseek.enabled,
            ) {
                Ok(client) => {
                    llm_manager.set_deepseek(std::sync::Arc::new(client));
                    enabled_providers.push("DeepSeek");
                }
                Err(e) => {
                    logger::warning(
                        LogTag::System,
                        &format!("Failed to initialize DeepSeek: {e}"),
                    );
                }
            }
        }

        // Gemini
        if cfg.ai.providers.gemini.enabled && !cfg.ai.providers.gemini.api_key.is_empty() {
            use crate::apis::llm::gemini::GeminiClient;
            let model = get_model(&cfg.ai.providers.gemini.model);
            match GeminiClient::new(
                cfg.ai.providers.gemini.api_key.clone(),
                model,
                cfg.ai.providers.gemini.enabled,
            ) {
                Ok(client) => {
                    llm_manager.set_gemini(std::sync::Arc::new(client));
                    enabled_providers.push("Gemini");
                }
                Err(e) => {
                    logger::warning(LogTag::System, &format!("Failed to initialize Gemini: {e}"));
                }
            }
        }

        // Ollama (no API key, uses base_url instead)
        if cfg.ai.providers.ollama.enabled {
            use crate::apis::llm::ollama::OllamaClient;
            let base_url = if !cfg.ai.providers.ollama.base_url.is_empty() {
                Some(cfg.ai.providers.ollama.base_url.clone())
            } else {
                None
            };
            let model = get_model(&cfg.ai.providers.ollama.model);
            match OllamaClient::new(base_url, model, cfg.ai.providers.ollama.enabled) {
                Ok(client) => {
                    llm_manager.set_ollama(std::sync::Arc::new(client));
                    enabled_providers.push("Ollama");
                }
                Err(e) => {
                    logger::warning(LogTag::System, &format!("Failed to initialize Ollama: {e}"));
                }
            }
        }

        // Together
        if cfg.ai.providers.together.enabled && !cfg.ai.providers.together.api_key.is_empty() {
            use crate::apis::llm::together::TogetherClient;
            let model = get_model(&cfg.ai.providers.together.model);
            match TogetherClient::new(
                cfg.ai.providers.together.api_key.clone(),
                model,
                cfg.ai.providers.together.enabled,
            ) {
                Ok(client) => {
                    llm_manager.set_together(std::sync::Arc::new(client));
                    enabled_providers.push("Together");
                }
                Err(e) => {
                    logger::warning(
                        LogTag::System,
                        &format!("Failed to initialize Together: {e}"),
                    );
                }
            }
        }

        // Mistral
        if cfg.ai.providers.mistral.enabled && !cfg.ai.providers.mistral.api_key.is_empty() {
            use crate::apis::llm::mistral::MistralClient;
            let model = get_model(&cfg.ai.providers.mistral.model);
            match MistralClient::new(
                cfg.ai.providers.mistral.api_key.clone(),
                model,
                cfg.ai.providers.mistral.enabled,
            ) {
                Ok(client) => {
                    llm_manager.set_mistral(std::sync::Arc::new(client));
                    enabled_providers.push("Mistral");
                }
                Err(e) => {
                    logger::warning(
                        LogTag::System,
                        &format!("Failed to initialize Mistral: {e}"),
                    );
                }
            }
        }
    });

    init_llm_manager(llm_manager)
        .await
        .map_err(|e| format!("Failed to initialize LLM manager: {e}"))?;

    if enabled_providers.is_empty() {
        logger::info(
            LogTag::System,
            "LLM manager initialized (no providers enabled)",
        );
    } else {
        logger::info(
            LogTag::System,
            &format!(
                "LLM manager initialized with {} provider(s): {}",
                enabled_providers.len(),
                enabled_providers.join(", ")
            ),
        );
    }

    Ok(())
}
