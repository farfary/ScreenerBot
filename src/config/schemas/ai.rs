//! AI integration configuration for token filtering, trading decisions, and analysis

use crate::config_struct;
use crate::field_metadata;

// ============================================================================
// AI CONFIGURATION
// ============================================================================

config_struct! {
    /// Main AI configuration for filtering and trading
    pub struct AiConfig {
        // === Master Control ===
        /// Master switch for all AI features
        #[metadata(field_metadata! {
            label: "Enable AI",
            hint: "Master switch for all AI-powered features (filtering, trading analysis)",
            category: "Master Control",
            impact: "critical",
        })]
        enabled: bool = false,

        /// Default AI provider to use
        #[metadata(field_metadata! {
            label: "Default Provider",
            hint: "Primary AI provider to use (openai, anthropic, groq, deepseek, gemini, ollama, together, openrouter, mistral)",
            placeholder: "openai",
            category: "Master Control",
        })]
        default_provider: String = "openai".to_string(),

        // === Filtering Section ===
        /// Enable AI-powered token filtering
        #[metadata(field_metadata! {
            label: "AI Filtering Enabled",
            hint: "Use AI to analyze and filter tokens based on metadata, social signals, and risk factors",
            category: "Filtering",
        })]
        filtering_enabled: bool = false,

        /// Minimum confidence threshold for AI filtering (0-100%)
        #[metadata(field_metadata! {
            label: "Filtering Min Confidence",
            hint: "Minimum AI confidence score (0-100%) to pass filtering. Higher = stricter",
            min: 0,
            max: 100,
            step: 5,
            unit: "%",
            category: "Filtering",
        })]
        filtering_min_confidence: u8 = 70,

        /// Whether to pass tokens when AI filtering fails
        #[metadata(field_metadata! {
            label: "Fallback Pass on Failure",
            hint: "If true, tokens pass filtering when AI fails/errors. If false, tokens fail when AI unavailable",
            category: "Filtering",
        })]
        filtering_fallback_pass: bool = false,

        /// Use cache for filtering evaluations
        #[metadata(field_metadata! {
            label: "Cache Filtering Results",
            hint: "Cache AI filtering results to reduce API calls and costs",
            category: "Filtering",
        })]
        filtering_use_cache: bool = true,

        // === Trading Section ===
        /// Enable AI analysis for entry decisions
        #[metadata(field_metadata! {
            label: "AI Entry Analysis",
            hint: "Use AI to analyze tokens before opening positions",
            category: "Trading",
        })]
        entry_analysis_enabled: bool = false,

        /// Enable AI analysis for exit decisions
        #[metadata(field_metadata! {
            label: "AI Exit Analysis",
            hint: "Use AI to help decide when to exit positions",
            category: "Trading",
        })]
        exit_analysis_enabled: bool = false,

        /// Enable AI-powered dynamic trailing stop
        #[metadata(field_metadata! {
            label: "AI Trailing Stop",
            hint: "Use AI to dynamically adjust trailing stop levels based on market conditions",
            category: "Trading",
        })]
        ai_trailing_stop_enabled: bool = false,

        /// Bypass cache for trading decisions (always fresh analysis)
        #[metadata(field_metadata! {
            label: "Trading Bypass Cache",
            hint: "Always get fresh AI analysis for trading decisions (recommended for accuracy)",
            category: "Trading",
        })]
        trading_bypass_cache: bool = true,

        // === Auto Blacklist Section ===
        /// Enable automatic blacklisting based on AI analysis
        #[metadata(field_metadata! {
            label: "Auto Blacklist Enabled",
            hint: "Automatically blacklist tokens that AI identifies as high-risk scams",
            category: "Auto Blacklist",
        })]
        auto_blacklist_enabled: bool = false,

        /// Minimum confidence to auto-blacklist (0-100%)
        #[metadata(field_metadata! {
            label: "Auto Blacklist Min Confidence",
            hint: "Minimum AI confidence (0-100%) that token is a scam before auto-blacklisting. Higher = fewer false positives",
            min: 0,
            max: 100,
            step: 5,
            unit: "%",
            category: "Auto Blacklist",
        })]
        auto_blacklist_min_confidence: u8 = 90,

        // === Background Check Section ===
        /// Enable background checking of open positions
        #[metadata(field_metadata! {
            label: "Background Check Enabled",
            hint: "Periodically re-evaluate open positions with AI in background",
            category: "Background Check",
        })]
        background_check_enabled: bool = false,

        /// Interval between background checks
        #[metadata(field_metadata! {
            label: "Background Check Interval",
            hint: "How often to re-check open positions with AI",
            min: 60,
            max: 3600,
            step: 60,
            unit: "seconds",
            category: "Background Check",
        })]
        background_check_interval_seconds: u64 = 300,

        /// Number of positions to check per batch
        #[metadata(field_metadata! {
            label: "Background Batch Size",
            hint: "How many positions to check in each background batch",
            min: 1,
            max: 20,
            step: 1,
            category: "Background Check",
        })]
        background_batch_size: u32 = 5,

        // === Rate Limits Section ===
        /// Maximum AI evaluations per minute (global limit)
        #[metadata(field_metadata! {
            label: "Max Evaluations/Minute",
            hint: "Global rate limit for AI evaluations across all features",
            min: 1,
            max: 100,
            step: 5,
            unit: "requests/min",
            category: "Rate Limits",
        })]
        max_evaluations_per_minute: u32 = 10,

        // === Performance Section ===
        /// Cache TTL for AI results
        #[metadata(field_metadata! {
            label: "Cache TTL",
            hint: "How long to cache AI results before re-evaluating",
            min: 60,
            max: 3600,
            step: 60,
            unit: "seconds",
            category: "Performance",
        })]
        cache_ttl_seconds: u64 = 300,

        // === Providers Section ===
        /// AI provider configurations
        #[metadata(field_metadata! {
            label: "Providers",
            hint: "Configuration for all AI providers",
            category: "Providers",
        })]
        providers: AiProvidersConfig = AiProvidersConfig::default(),
    }
}

config_struct! {
    /// AI provider configurations
    pub struct AiProvidersConfig {
        /// OpenAI configuration (GPT-4, GPT-3.5-turbo, etc.)
        #[metadata(field_metadata! {
            label: "OpenAI",
            hint: "OpenAI API configuration (GPT-4, GPT-3.5-turbo)",
            category: "Providers",
        })]
        openai: AiProviderConfig = AiProviderConfig::default(),

        /// Anthropic configuration (Claude 3.5, Claude 3, etc.)
        #[metadata(field_metadata! {
            label: "Anthropic",
            hint: "Anthropic API configuration (Claude 3.5 Sonnet, Claude 3 Opus)",
            category: "Providers",
        })]
        anthropic: AiProviderConfig = AiProviderConfig::default(),

        /// Groq configuration (fast inference)
        #[metadata(field_metadata! {
            label: "Groq",
            hint: "Groq API configuration (ultra-fast inference, free tier available)",
            category: "Providers",
        })]
        groq: AiProviderConfig = AiProviderConfig::default(),

        /// DeepSeek configuration
        #[metadata(field_metadata! {
            label: "DeepSeek",
            hint: "DeepSeek API configuration (cost-effective option)",
            category: "Providers",
        })]
        deepseek: AiProviderConfig = AiProviderConfig::default(),

        /// Google Gemini configuration
        #[metadata(field_metadata! {
            label: "Gemini",
            hint: "Google Gemini API configuration (Gemini Pro, Gemini Ultra)",
            category: "Providers",
        })]
        gemini: AiProviderConfig = AiProviderConfig::default(),

        /// Ollama configuration (local models)
        #[metadata(field_metadata! {
            label: "Ollama",
            hint: "Ollama local AI configuration (run models locally, no API key needed)",
            category: "Providers",
        })]
        ollama: AiOllamaConfig = AiOllamaConfig::default(),

        /// Together AI configuration
        #[metadata(field_metadata! {
            label: "Together AI",
            hint: "Together AI API configuration (various open-source models)",
            category: "Providers",
        })]
        together: AiProviderConfig = AiProviderConfig::default(),

        /// OpenRouter configuration (access to multiple models)
        #[metadata(field_metadata! {
            label: "OpenRouter",
            hint: "OpenRouter API configuration (unified access to multiple AI providers)",
            category: "Providers",
        })]
        openrouter: AiProviderConfig = AiProviderConfig::default(),

        /// Mistral AI configuration
        #[metadata(field_metadata! {
            label: "Mistral",
            hint: "Mistral AI API configuration (Mistral Large, Mistral Medium)",
            category: "Providers",
        })]
        mistral: AiProviderConfig = AiProviderConfig::default(),
    }
}

config_struct! {
    /// Single AI provider configuration
    pub struct AiProviderConfig {
        /// Enable this provider
        #[metadata(field_metadata! {
            label: "Enabled",
            hint: "Enable this AI provider",
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
    /// Ollama-specific configuration (local models)
    pub struct AiOllamaConfig {
        /// Enable Ollama
        #[metadata(field_metadata! {
            label: "Enabled",
            hint: "Enable Ollama for local AI inference (no API key needed)",
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
        model: String = "llama3.2".to_string(),

        /// Base URL for Ollama API
        #[metadata(field_metadata! {
            label: "Base URL",
            hint: "Ollama API endpoint (default: http://localhost:11434)",
            placeholder: "http://localhost:11434",
            category: "Ollama Settings",
        })]
        base_url: String = "http://localhost:11434".to_string(),

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
