//! Outbound LLM provider configuration.
//!
//! This section owns provider credentials, model selection and per-provider
//! rate limits only. Model-scored analysis lives in `llm_analysis`, the
//! dashboard assistant in `assistant`, and tool permissions in `agent_control`.

use crate::config_struct;
use crate::field_metadata;

config_struct! {
    /// Outbound LLM provider clients: the master enable switch, the default
    /// provider and every provider's credentials.
    pub struct LlmConfig {
        /// Legacy master switch for every model-backed feature.
        #[metadata(field_metadata! {
            label: "Enable LLM",
            hint: "Master switch for all model-backed features (analysis, assistant). Feature owners additionally check their own flags",
            category: "Master Control",
            impact: "critical",
        })]
        enabled: bool = false,

        /// Default provider used when a feature does not name one.
        #[metadata(field_metadata! {
            label: "Default Provider",
            hint: "Primary provider to use (openai, anthropic, groq, deepseek, gemini, ollama, together, openrouter, mistral)",
            placeholder: "openai",
            category: "Master Control",
        })]
        default_provider: String = "openai".to_owned(),

        /// Per-provider client configuration.
        #[metadata(field_metadata! {
            label: "Providers",
            hint: "Credentials, model and rate limit for every supported provider",
            category: "Providers",
        })]
        providers: LlmProvidersConfig = LlmProvidersConfig::default(),
    }
}

config_struct! {
    /// Per-provider client configuration.
    pub struct LlmProvidersConfig {
        /// OpenAI configuration (GPT-4, GPT-3.5-turbo, etc.)
        #[metadata(field_metadata! {
            label: "OpenAI",
            hint: "OpenAI API configuration (GPT-4, GPT-3.5-turbo)",
            category: "Providers",
        })]
        openai: LlmProviderConfig = LlmProviderConfig::default(),

        /// Anthropic configuration (Claude 3.5, Claude 3, etc.)
        #[metadata(field_metadata! {
            label: "Anthropic",
            hint: "Anthropic API configuration (Claude 3.5 Sonnet, Claude 3 Opus)",
            category: "Providers",
        })]
        anthropic: LlmProviderConfig = LlmProviderConfig::default(),

        /// Groq configuration (fast inference)
        #[metadata(field_metadata! {
            label: "Groq",
            hint: "Groq API configuration (ultra-fast inference, free tier available)",
            category: "Providers",
        })]
        groq: LlmProviderConfig = LlmProviderConfig::default(),

        /// DeepSeek configuration
        #[metadata(field_metadata! {
            label: "DeepSeek",
            hint: "DeepSeek API configuration (cost-effective option)",
            category: "Providers",
        })]
        deepseek: LlmProviderConfig = LlmProviderConfig::default(),

        /// Google Gemini configuration
        #[metadata(field_metadata! {
            label: "Gemini",
            hint: "Google Gemini API configuration (Gemini Pro, Gemini Ultra)",
            category: "Providers",
        })]
        gemini: LlmProviderConfig = LlmProviderConfig::default(),

        /// Ollama configuration (local models)
        #[metadata(field_metadata! {
            label: "Ollama",
            hint: "Ollama local configuration (run models locally, no API key needed)",
            category: "Providers",
        })]
        ollama: OllamaConfig = OllamaConfig::default(),

        /// Together AI configuration
        #[metadata(field_metadata! {
            label: "Together AI",
            hint: "Together AI API configuration (various open-source models)",
            category: "Providers",
        })]
        together: LlmProviderConfig = LlmProviderConfig::default(),

        /// OpenRouter configuration (access to multiple models)
        #[metadata(field_metadata! {
            label: "OpenRouter",
            hint: "OpenRouter API configuration (unified access to multiple providers)",
            category: "Providers",
        })]
        openrouter: LlmProviderConfig = LlmProviderConfig::default(),

        /// Mistral AI configuration
        #[metadata(field_metadata! {
            label: "Mistral",
            hint: "Mistral AI API configuration (Mistral Large, Mistral Medium)",
            category: "Providers",
        })]
        mistral: LlmProviderConfig = LlmProviderConfig::default(),
    }
}

config_struct! {
    /// Single API-keyed provider configuration.
    pub struct LlmProviderConfig {
        /// Enable this provider
        #[metadata(field_metadata! {
            label: "Enabled",
            hint: "Enable this provider",
            category: "Provider Settings",
        })]
        enabled: bool = false,

        /// API key for this provider
        #[metadata(field_metadata! {
            label: "API Key",
            hint: "API key for this provider. Leave empty if not using.",
            placeholder: "sk-...",
            category: "Provider Settings",
        })]
        api_key: String = String::new(),

        /// Model name to use (empty = provider default)
        #[metadata(field_metadata! {
            label: "Model",
            hint: "Specific model to use. Leave empty to use provider default (e.g., gpt-4, claude-3-5-sonnet-20241022)",
            placeholder: "auto",
            category: "Provider Settings",
        })]
        model: String = String::new(),

        /// Rate limit for this provider (requests per minute)
        #[metadata(field_metadata! {
            label: "Rate Limit",
            hint: "Maximum requests per minute for this provider",
            min: 1,
            max: 1000,
            step: 10,
            unit: "requests/min",
            category: "Provider Settings",
        })]
        rate_limit_per_minute: u32 = 60,
    }
}

config_struct! {
    /// Ollama-specific configuration (local models, no API key).
    pub struct OllamaConfig {
        /// Enable Ollama
        #[metadata(field_metadata! {
            label: "Enabled",
            hint: "Enable Ollama for local inference (no API key needed)",
            category: "Ollama Settings",
        })]
        enabled: bool = false,

        /// Model name to use
        #[metadata(field_metadata! {
            label: "Model",
            hint: "Ollama model to use (must be pulled locally first: ollama pull <model>)",
            placeholder: "llama3.2",
            category: "Ollama Settings",
        })]
        model: String = "llama3.2".to_owned(),

        /// Base URL for Ollama API
        #[metadata(field_metadata! {
            label: "Base URL",
            hint: "Ollama API endpoint (default: http://localhost:11434)",
            placeholder: "http://localhost:11434",
            category: "Ollama Settings",
        })]
        base_url: String = "http://localhost:11434".to_owned(),

        /// Rate limit for Ollama (higher since it's local)
        #[metadata(field_metadata! {
            label: "Rate Limit",
            hint: "Maximum requests per minute for Ollama (can be higher since it's local)",
            min: 1,
            max: 1000,
            step: 10,
            unit: "requests/min",
            category: "Ollama Settings",
        })]
        rate_limit_per_minute: u32 = 120,
    }
}
