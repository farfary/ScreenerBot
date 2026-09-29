//! Outbound LLM provider configuration.
//!
//! This section owns provider credentials, model selection and per-provider
//! rate limits only. Model-scored analysis lives in `llm_analysis`, the
//! dashboard assistant in `assistant`, and tool permissions in `agent_control`.

use crate::config::metadata::{ConfigCategory, ConfigImpact};
use crate::config_struct;
use crate::field_metadata;

config_struct! {
    /// Outbound LLM provider clients: the master enable switch, the default
    /// provider and every provider's credentials.
    pub struct LlmConfig {
        /// Master switch for every model-backed feature.
        #[metadata(field_metadata! {
            category: ConfigCategory::MasterControl,
            impact: ConfigImpact::Critical,
        })]
        enabled: bool = false,

        /// Provider used by analysis, the Assistant, and scheduled automation.
        #[metadata(field_metadata! {
            category: ConfigCategory::MasterControl,
        })]
        default_provider: String = "openai".to_owned(),

        /// Per-provider client configuration.
        #[metadata(field_metadata! {
            category: ConfigCategory::Providers,
        })]
        providers: LlmProvidersConfig = LlmProvidersConfig::default(),
    }
}

config_struct! {
    /// Per-provider client configuration.
    pub struct LlmProvidersConfig {
        /// OpenAI configuration (GPT-4, GPT-3.5-turbo, etc.)
        #[metadata(field_metadata! {
            category: ConfigCategory::Providers,
        })]
        openai: LlmProviderConfig = LlmProviderConfig::default(),

        /// Anthropic configuration (Claude 3.5, Claude 3, etc.)
        #[metadata(field_metadata! {
            category: ConfigCategory::Providers,
        })]
        anthropic: LlmProviderConfig = LlmProviderConfig::default(),

        /// Groq configuration (fast inference)
        #[metadata(field_metadata! {
            category: ConfigCategory::Providers,
        })]
        groq: LlmProviderConfig = LlmProviderConfig::default(),

        /// DeepSeek configuration
        #[metadata(field_metadata! {
            category: ConfigCategory::Providers,
        })]
        deepseek: LlmProviderConfig = LlmProviderConfig::default(),

        /// Google Gemini configuration
        #[metadata(field_metadata! {
            category: ConfigCategory::Providers,
        })]
        gemini: LlmProviderConfig = LlmProviderConfig::default(),

        /// Ollama configuration (local models)
        #[metadata(field_metadata! {
            category: ConfigCategory::Providers,
        })]
        ollama: OllamaConfig = OllamaConfig::default(),

        /// Together AI configuration
        #[metadata(field_metadata! {
            category: ConfigCategory::Providers,
        })]
        together: LlmProviderConfig = LlmProviderConfig::default(),

        /// OpenRouter configuration (access to multiple models)
        #[metadata(field_metadata! {
            category: ConfigCategory::Providers,
        })]
        openrouter: LlmProviderConfig = LlmProviderConfig::default(),

        /// Mistral AI configuration
        #[metadata(field_metadata! {
            category: ConfigCategory::Providers,
        })]
        mistral: LlmProviderConfig = LlmProviderConfig::default(),
    }
}

config_struct! {
    /// Single API-keyed provider configuration.
    pub struct LlmProviderConfig {
        /// Enable this provider
        #[metadata(field_metadata! {
            category: ConfigCategory::ProviderSettings,
        })]
        enabled: bool = false,

        /// API key for this provider
        #[metadata(field_metadata! {
            category: ConfigCategory::ProviderSettings,
        })]
        api_key: String = String::new(),

        /// Model name to use (empty = provider default)
        #[metadata(field_metadata! {
            category: ConfigCategory::ProviderSettings,
        })]
        model: String = String::new(),

        /// Rate limit for this provider (requests per minute)
        #[metadata(field_metadata! {
            min: 1,
            max: 1000,
            step: 10,
            category: ConfigCategory::ProviderSettings,
        })]
        rate_limit_per_minute: u32 = 60,
    }
}

config_struct! {
    /// Ollama-specific configuration (local models, no API key).
    pub struct OllamaConfig {
        /// Enable Ollama
        #[metadata(field_metadata! {
            category: ConfigCategory::OllamaSettings,
        })]
        enabled: bool = false,

        /// Model name to use
        #[metadata(field_metadata! {
            category: ConfigCategory::OllamaSettings,
        })]
        model: String = "llama3.2".to_owned(),

        /// Base URL for Ollama API
        #[metadata(field_metadata! {
            category: ConfigCategory::OllamaSettings,
        })]
        base_url: String = "http://localhost:11434".to_owned(),

        /// Rate limit for Ollama (higher since it's local)
        #[metadata(field_metadata! {
            min: 1,
            max: 1000,
            step: 10,
            category: ConfigCategory::OllamaSettings,
        })]
        rate_limit_per_minute: u32 = 120,
    }
}
