/// LLM Provider Module
///
/// Unified interface for multiple LLM providers:
/// - OpenAI (GPT-3.5, GPT-4, GPT-4-turbo)
/// - Anthropic (Claude 3 family)
/// - Groq (Fast inference)
/// - DeepSeek (Reasoning models)
/// - Google Gemini
/// - Ollama (Local models)
/// - Together AI
/// - OpenRouter (Multi-provider gateway)
/// - Mistral AI
/// - Fireworks AI
///
/// All providers use raw HTTP via reqwest with shared rate limiting and stats.
pub mod anthropic;
pub mod deepseek;
pub mod fireworks;
pub mod gemini;
pub mod groq;
pub mod mistral;
pub mod ollama;
pub mod openai;
pub mod openrouter;
pub mod together;
pub mod types;

// Re-export core types
pub use types::{
    ChatMessage, ChatRequest, ChatResponse, LlmError, MessageRole, ResponseFormat, Usage,
};

use crate::apis::client::RateLimiter;
use crate::apis::stats::ApiStatsTracker;
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::OnceCell;

// ============================================================================
// PROVIDER ENUM
// ============================================================================

/// Supported LLM providers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Provider {
    OpenAi,
    Anthropic,
    Groq,
    DeepSeek,
    Gemini,
    Ollama,
    Together,
    OpenRouter,
    Mistral,
    Fireworks,
}

impl Provider {
    /// Get provider name as string
    pub fn as_str(&self) -> &'static str {
        match self {
            Provider::OpenAi => "openai",
            Provider::Anthropic => "anthropic",
            Provider::Groq => "groq",
            Provider::DeepSeek => "deepseek",
            Provider::Gemini => "gemini",
            Provider::Ollama => "ollama",
            Provider::Together => "together",
            Provider::OpenRouter => "openrouter",
            Provider::Mistral => "mistral",
            Provider::Fireworks => "fireworks",
        }
    }

    /// Parse provider from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "openai" => Some(Provider::OpenAi),
            "anthropic" => Some(Provider::Anthropic),
            "groq" => Some(Provider::Groq),
            "deepseek" => Some(Provider::DeepSeek),
            "gemini" => Some(Provider::Gemini),
            "ollama" => Some(Provider::Ollama),
            "together" => Some(Provider::Together),
            "openrouter" => Some(Provider::OpenRouter),
            "mistral" => Some(Provider::Mistral),
            "fireworks" => Some(Provider::Fireworks),
            _ => None,
        }
    }

    /// Get all providers
    pub fn all() -> Vec<Self> {
        vec![
            Provider::OpenAi,
            Provider::Anthropic,
            Provider::Groq,
            Provider::DeepSeek,
            Provider::Gemini,
            Provider::Ollama,
            Provider::Together,
            Provider::OpenRouter,
            Provider::Mistral,
            Provider::Fireworks,
        ]
    }
}

impl std::fmt::Display for Provider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// ============================================================================
// LLM CLIENT TRAIT
// ============================================================================

/// Unified interface for all LLM providers
#[async_trait]
pub trait LlmClient: Send + Sync {
    /// Provider name
    fn provider(&self) -> Provider;

    /// Check if client is enabled
    fn is_enabled(&self) -> bool;

    /// Make a chat completion request
    async fn call(&self, request: ChatRequest) -> Result<ChatResponse, LlmError>;

    /// Get API statistics
    async fn get_stats(&self) -> crate::apis::stats::ApiStats;

    /// Get rate limiter info
    fn rate_limit_info(&self) -> (usize, std::time::Duration);
}

// ============================================================================
// LLM MANAGER
// ============================================================================

/// Global manager for all LLM provider clients
pub struct LlmManager {
    // Provider clients (initialized on first access)
    openai: Option<Arc<dyn LlmClient>>,
    anthropic: Option<Arc<dyn LlmClient>>,
    groq: Option<Arc<dyn LlmClient>>,
    deepseek: Option<Arc<dyn LlmClient>>,
    gemini: Option<Arc<dyn LlmClient>>,
    ollama: Option<Arc<dyn LlmClient>>,
    together: Option<Arc<dyn LlmClient>>,
    openrouter: Option<Arc<dyn LlmClient>>,
    mistral: Option<Arc<dyn LlmClient>>,
    fireworks: Option<Arc<dyn LlmClient>>,
}

impl LlmManager {
    /// Create a new LlmManager with all providers disabled
    pub fn new() -> Self {
        Self {
            openai: None,
            anthropic: None,
            groq: None,
            deepseek: None,
            gemini: None,
            ollama: None,
            together: None,
            openrouter: None,
            mistral: None,
            fireworks: None,
        }
    }

    /// Get a client for a specific provider
    pub fn get_client(&self, provider: Provider) -> Option<Arc<dyn LlmClient>> {
        match provider {
            Provider::OpenAi => self.openai.clone(),
            Provider::Anthropic => self.anthropic.clone(),
            Provider::Groq => self.groq.clone(),
            Provider::DeepSeek => self.deepseek.clone(),
            Provider::Gemini => self.gemini.clone(),
            Provider::Ollama => self.ollama.clone(),
            Provider::Together => self.together.clone(),
            Provider::OpenRouter => self.openrouter.clone(),
            Provider::Mistral => self.mistral.clone(),
            Provider::Fireworks => self.fireworks.clone(),
        }
    }

    /// Get all enabled providers
    pub fn enabled_providers(&self) -> Vec<Provider> {
        let mut providers = Vec::new();

        if let Some(client) = &self.openai {
            if client.is_enabled() {
                providers.push(Provider::OpenAi);
            }
        }
        if let Some(client) = &self.anthropic {
            if client.is_enabled() {
                providers.push(Provider::Anthropic);
            }
        }
        if let Some(client) = &self.groq {
            if client.is_enabled() {
                providers.push(Provider::Groq);
            }
        }
        if let Some(client) = &self.deepseek {
            if client.is_enabled() {
                providers.push(Provider::DeepSeek);
            }
        }
        if let Some(client) = &self.gemini {
            if client.is_enabled() {
                providers.push(Provider::Gemini);
            }
        }
        if let Some(client) = &self.ollama {
            if client.is_enabled() {
                providers.push(Provider::Ollama);
            }
        }
        if let Some(client) = &self.together {
            if client.is_enabled() {
                providers.push(Provider::Together);
            }
        }
        if let Some(client) = &self.openrouter {
            if client.is_enabled() {
                providers.push(Provider::OpenRouter);
            }
        }
        if let Some(client) = &self.mistral {
            if client.is_enabled() {
                providers.push(Provider::Mistral);
            }
        }
        if let Some(client) = &self.fireworks {
            if client.is_enabled() {
                providers.push(Provider::Fireworks);
            }
        }

        providers
    }

    /// Make a request using a specific provider
    pub async fn call(
        &self,
        provider: Provider,
        request: ChatRequest,
    ) -> Result<ChatResponse, LlmError> {
        let client = self
            .get_client(provider)
            .ok_or_else(|| LlmError::ProviderDisabled {
                provider: provider.to_string(),
            })?;

        client.call(request).await
    }

    /// Set OpenAI client
    pub fn set_openai(&mut self, client: Arc<dyn LlmClient>) {
        self.openai = Some(client);
    }

    /// Set Anthropic client
    pub fn set_anthropic(&mut self, client: Arc<dyn LlmClient>) {
        self.anthropic = Some(client);
    }

    /// Set Groq client
    pub fn set_groq(&mut self, client: Arc<dyn LlmClient>) {
        self.groq = Some(client);
    }

    /// Set DeepSeek client
    pub fn set_deepseek(&mut self, client: Arc<dyn LlmClient>) {
        self.deepseek = Some(client);
    }

    /// Set Gemini client
    pub fn set_gemini(&mut self, client: Arc<dyn LlmClient>) {
        self.gemini = Some(client);
    }

    /// Set Ollama client
    pub fn set_ollama(&mut self, client: Arc<dyn LlmClient>) {
        self.ollama = Some(client);
    }

    /// Set Together client
    pub fn set_together(&mut self, client: Arc<dyn LlmClient>) {
        self.together = Some(client);
    }

    /// Set OpenRouter client
    pub fn set_openrouter(&mut self, client: Arc<dyn LlmClient>) {
        self.openrouter = Some(client);
    }

    /// Set Mistral client
    pub fn set_mistral(&mut self, client: Arc<dyn LlmClient>) {
        self.mistral = Some(client);
    }

    /// Set Fireworks client
    pub fn set_fireworks(&mut self, client: Arc<dyn LlmClient>) {
        self.fireworks = Some(client);
    }
}

impl Default for LlmManager {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// GLOBAL SINGLETON
// ============================================================================

static LLM_MANAGER: OnceCell<Arc<LlmManager>> = OnceCell::const_new();

/// Initialize the global LLM manager
///
/// Should be called once at startup with configured provider clients.
pub async fn init_llm_manager(manager: LlmManager) -> Result<(), String> {
    LLM_MANAGER
        .set(Arc::new(manager))
        .map_err(|_| "LLM manager already initialized".to_string())
}

/// Get the global LLM manager
///
/// # Panics
/// Panics if the manager hasn't been initialized with `init_llm_manager()`
pub fn get_llm_manager() -> Arc<LlmManager> {
    LLM_MANAGER
        .get()
        .expect("LLM manager not initialized - call init_llm_manager() first")
        .clone()
}

/// Try to get the global LLM manager (non-panicking version)
pub fn try_get_llm_manager() -> Option<Arc<LlmManager>> {
    LLM_MANAGER.get().cloned()
}

// ============================================================================
// UTILITY FUNCTIONS
// ============================================================================

/// Create a simple single-message request
pub fn simple_request(model: impl Into<String>, prompt: impl Into<String>) -> ChatRequest {
    ChatRequest::new(model, vec![ChatMessage::user(prompt)])
}

/// Create a request with system and user messages
pub fn system_user_request(
    model: impl Into<String>,
    system_prompt: impl Into<String>,
    user_prompt: impl Into<String>,
) -> ChatRequest {
    ChatRequest::new(
        model,
        vec![
            ChatMessage::system(system_prompt),
            ChatMessage::user(user_prompt),
        ],
    )
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_enum() {
        assert_eq!(Provider::OpenAi.as_str(), "openai");
        assert_eq!(Provider::Anthropic.as_str(), "anthropic");
        assert_eq!(Provider::from_str("openai"), Some(Provider::OpenAi));
        assert_eq!(Provider::from_str("ANTHROPIC"), Some(Provider::Anthropic));
        assert_eq!(Provider::from_str("invalid"), None);
    }

    #[test]
    fn test_message_builders() {
        let sys = ChatMessage::system("You are a helpful assistant");
        assert_eq!(sys.role, MessageRole::System);

        let user = ChatMessage::user("Hello");
        assert_eq!(user.role, MessageRole::User);

        let assistant = ChatMessage::assistant("Hi there!");
        assert_eq!(assistant.role, MessageRole::Assistant);
    }

    #[test]
    fn test_request_builder() {
        let req = ChatRequest::new("gpt-4", vec![ChatMessage::user("test")])
            .with_temperature(0.7)
            .with_max_tokens(1000)
            .with_json_mode();

        assert_eq!(req.model, "gpt-4");
        assert_eq!(req.temperature, Some(0.7));
        assert_eq!(req.max_tokens, Some(1000));
        assert!(req.response_format.is_some());
    }

    #[test]
    fn test_usage() {
        let usage = Usage::new(100, 50);
        assert_eq!(usage.prompt_tokens, 100);
        assert_eq!(usage.completion_tokens, 50);
        assert_eq!(usage.total_tokens, 150);
    }
}
