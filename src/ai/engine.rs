use crate::ai::cache::AiCache;
use crate::ai::prompts::{
    get_entry_analysis_prompt, get_exit_analysis_prompt, get_filter_prompt, PromptBuilder,
};
use crate::ai::schemas::{validate_json_response, FilterDecision, TradeDecision};
use crate::ai::types::{
    AiDecision, AiError, EvaluationContext, EvaluationResult, Factor, Impact, Priority, RiskLevel,
};
use crate::apis::llm::{get_llm_manager, ChatMessage, ChatRequest, LlmError, Provider};
use crate::config::with_config;
use std::sync::Arc;
use std::time::Instant;

/// Main AI engine that orchestrates LLM calls, caching, and decision processing
pub struct AiEngine {
    cache: Arc<AiCache>,
}

impl AiEngine {
    /// Create a new AI engine
    pub fn new() -> Self {
        let cache_ttl = with_config(|cfg| cfg.ai.cache_ttl_seconds);
        Self {
            cache: Arc::new(AiCache::new(cache_ttl)),
        }
    }

    /// Evaluate a token for filtering
    pub async fn evaluate_filter(
        &self,
        context: EvaluationContext,
        priority: Priority,
    ) -> Result<EvaluationResult, AiError> {
        // Check if AI is enabled
        let (ai_enabled, filtering_enabled) =
            with_config(|cfg| (cfg.ai.enabled, cfg.ai.filtering_enabled));

        if !ai_enabled || !filtering_enabled {
            return Err(AiError::Disabled);
        }

        // Check cache first
        if let Some(cached_decision) = self.cache.get(&context.mint, priority) {
            return Ok(EvaluationResult {
                decision: cached_decision,
                cached: true,
            });
        }

        // Get provider and model from config
        let (provider_name, bypass_cache) =
            with_config(|cfg| (cfg.ai.default_provider.clone(), cfg.ai.trading_bypass_cache));

        let provider = Provider::from_str(&provider_name)
            .ok_or_else(|| AiError::ProviderNotConfigured(provider_name.clone()))?;

        // Build prompt
        let system_prompt = get_filter_prompt();
        let user_prompt = PromptBuilder::build_user_prompt(&context);

        // Create LLM request
        let request = ChatRequest::new(
            self.get_model_for_provider(provider),
            vec![
                ChatMessage::system(system_prompt),
                ChatMessage::user(user_prompt),
            ],
        )
        .with_temperature(0.7)
        .with_max_tokens(1000)
        .with_json_mode();

        // Call LLM
        let start = Instant::now();
        let llm_manager = get_llm_manager();
        let response = llm_manager
            .call(provider, request)
            .await
            .map_err(|e| self.map_llm_error(e))?;

        let latency_ms = start.elapsed().as_secs_f64() * 1000.0;

        // Parse response
        let filter_decision: FilterDecision = validate_json_response(&response.content)?;

        // Convert to AiDecision
        let decision =
            self.convert_filter_decision(filter_decision, response, latency_ms, provider)?;

        // Cache the result (unless bypass cache is enabled for high priority)
        if !bypass_cache || priority != Priority::High {
            self.cache.insert(&context.mint, decision.clone());
        }

        Ok(EvaluationResult {
            decision,
            cached: false,
        })
    }

    /// Get the appropriate model for a provider
    fn get_model_for_provider(&self, provider: Provider) -> String {
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
                Provider::Fireworks => &cfg.ai.providers.fireworks,
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
                    Provider::Fireworks => {
                        "accounts/fireworks/models/llama-v3-70b-instruct".to_string()
                    }
                }
            }
        })
    }

    /// Convert FilterDecision to AiDecision
    fn convert_filter_decision(
        &self,
        filter: FilterDecision,
        response: crate::apis::llm::ChatResponse,
        latency_ms: f64,
        provider: Provider,
    ) -> Result<AiDecision, AiError> {
        use crate::ai::schemas::FilterAction;

        let decision = match filter.decision {
            FilterAction::Pass => "pass".to_string(),
            FilterAction::Reject => "reject".to_string(),
        };

        let risk_level = match filter.risk_level.to_lowercase().as_str() {
            "low" => RiskLevel::Low,
            "medium" => RiskLevel::Medium,
            "high" => RiskLevel::High,
            "critical" => RiskLevel::Critical,
            _ => RiskLevel::Medium,
        };

        let factors = filter
            .factors
            .into_iter()
            .map(|f| {
                let impact = match f.impact.to_lowercase().as_str() {
                    "positive" => Impact::Positive,
                    "negative" => Impact::Negative,
                    _ => Impact::Neutral,
                };
                Factor {
                    name: f.name,
                    impact,
                    weight: f.weight,
                }
            })
            .collect();

        Ok(AiDecision {
            decision,
            confidence: filter.confidence,
            reasoning: filter.reasoning,
            risk_level,
            factors,
            provider: provider.to_string(),
            model: response.model,
            tokens_used: response.usage.total_tokens,
            latency_ms,
        })
    }

    /// Map LLM errors to AI errors
    fn map_llm_error(&self, error: LlmError) -> AiError {
        match error {
            LlmError::ProviderDisabled { provider } => AiError::ProviderNotConfigured(provider),
            LlmError::RateLimited { retry_after_ms, .. } => AiError::RateLimited {
                retry_after: retry_after_ms.map(|ms| ms / 1000),
            },
            LlmError::Timeout { timeout_ms, .. } => AiError::Timeout,
            _ => AiError::LlmError(error.to_string()),
        }
    }

    /// Evaluate a token for entry (trading decision)
    pub async fn evaluate_entry(
        &self,
        context: &EvaluationContext,
        priority: Priority,
    ) -> Result<EvaluationResult, AiError> {
        // Check if AI is enabled
        let (ai_enabled, entry_enabled) =
            with_config(|cfg| (cfg.ai.enabled, cfg.ai.entry_analysis_enabled));

        if !ai_enabled || !entry_enabled {
            return Err(AiError::Disabled);
        }

        // Check cache first (unless high priority)
        if priority != Priority::High {
            if let Some(cached_decision) = self.cache.get(&context.mint, priority) {
                return Ok(EvaluationResult {
                    decision: cached_decision,
                    cached: true,
                });
            }
        }

        // Get provider and model from config
        let provider_name = with_config(|cfg| cfg.ai.default_provider.clone());

        let provider = Provider::from_str(&provider_name)
            .ok_or_else(|| AiError::ProviderNotConfigured(provider_name.clone()))?;

        // Build prompt
        let system_prompt = get_entry_analysis_prompt();
        let user_prompt = PromptBuilder::build_user_prompt(context);

        // Create LLM request
        let request = ChatRequest::new(
            self.get_model_for_provider(provider),
            vec![
                ChatMessage::system(system_prompt.to_string()),
                ChatMessage::user(user_prompt),
            ],
        )
        .with_temperature(0.7)
        .with_max_tokens(1000)
        .with_json_mode();

        // Call LLM
        let start = Instant::now();
        let llm_manager = get_llm_manager();
        let response = llm_manager
            .call(provider, request)
            .await
            .map_err(|e| self.map_llm_error(e))?;

        let latency_ms = start.elapsed().as_secs_f64() * 1000.0;

        // Parse response
        let trade_decision: TradeDecision = validate_json_response(&response.content)?;

        // Convert to AiDecision
        let decision =
            self.convert_trade_decision(trade_decision, response, latency_ms, provider)?;

        // Cache the result (unless bypass cache is enabled for high priority)
        let bypass_cache = with_config(|cfg| cfg.ai.trading_bypass_cache);
        if !bypass_cache || priority != Priority::High {
            self.cache.insert(&context.mint, decision.clone());
        }

        Ok(EvaluationResult {
            decision,
            cached: false,
        })
    }

    /// Evaluate a position for exit
    pub async fn evaluate_exit(
        &self,
        context: &EvaluationContext,
        priority: Priority,
    ) -> Result<EvaluationResult, AiError> {
        // Check if AI is enabled
        let (ai_enabled, exit_enabled) =
            with_config(|cfg| (cfg.ai.enabled, cfg.ai.exit_analysis_enabled));

        if !ai_enabled || !exit_enabled {
            return Err(AiError::Disabled);
        }

        // Exit analysis should always be fresh (no cache for exit decisions)
        let provider_name = with_config(|cfg| cfg.ai.default_provider.clone());

        let provider = Provider::from_str(&provider_name)
            .ok_or_else(|| AiError::ProviderNotConfigured(provider_name.clone()))?;

        // Build prompt
        let system_prompt = get_exit_analysis_prompt();
        let user_prompt = PromptBuilder::build_user_prompt(context);

        // Create LLM request
        let request = ChatRequest::new(
            self.get_model_for_provider(provider),
            vec![
                ChatMessage::system(system_prompt.to_string()),
                ChatMessage::user(user_prompt),
            ],
        )
        .with_temperature(0.7)
        .with_max_tokens(1000)
        .with_json_mode();

        // Call LLM
        let start = Instant::now();
        let llm_manager = get_llm_manager();
        let response = llm_manager
            .call(provider, request)
            .await
            .map_err(|e| self.map_llm_error(e))?;

        let latency_ms = start.elapsed().as_secs_f64() * 1000.0;

        // Parse response as TradeDecision (reuse schema for exit suggestions)
        let trade_decision: TradeDecision = validate_json_response(&response.content)?;

        // Convert to AiDecision
        let decision =
            self.convert_trade_decision(trade_decision, response, latency_ms, provider)?;

        Ok(EvaluationResult {
            decision,
            cached: false,
        })
    }

    /// Convert TradeDecision to AiDecision
    fn convert_trade_decision(
        &self,
        trade: TradeDecision,
        response: crate::apis::llm::ChatResponse,
        latency_ms: f64,
        provider: Provider,
    ) -> Result<AiDecision, AiError> {
        use crate::ai::schemas::TradeAction;

        let decision = match trade.decision {
            TradeAction::Buy => "buy".to_string(),
            TradeAction::Sell => "sell".to_string(),
            TradeAction::Hold => "hold".to_string(),
        };

        let risk_level = match trade.risk_level.to_lowercase().as_str() {
            "low" => RiskLevel::Low,
            "medium" => RiskLevel::Medium,
            "high" => RiskLevel::High,
            "critical" => RiskLevel::Critical,
            _ => RiskLevel::Medium,
        };

        let factors = trade
            .factors
            .into_iter()
            .map(|f| {
                let impact = match f.impact.to_lowercase().as_str() {
                    "positive" => Impact::Positive,
                    "negative" => Impact::Negative,
                    _ => Impact::Neutral,
                };
                Factor {
                    name: f.name,
                    impact,
                    weight: f.weight,
                }
            })
            .collect();

        Ok(AiDecision {
            decision,
            confidence: trade.confidence,
            reasoning: trade.reasoning,
            risk_level,
            factors,
            provider: provider.to_string(),
            model: response.model,
            tokens_used: response.usage.total_tokens,
            latency_ms,
        })
    }

    /// Get cache statistics
    pub fn cache_stats(&self) -> (usize, usize) {
        self.cache.stats()
    }

    /// Clear the cache
    pub fn clear_cache(&self) {
        self.cache.clear();
    }
}

impl Default for AiEngine {
    fn default() -> Self {
        Self::new()
    }
}
