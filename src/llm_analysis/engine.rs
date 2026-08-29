//! LLM analysis engine — orchestrates provider calls, prompt building, schema validation and decision caching for filter/entry/exit scoring.

use crate::apis::llm::{get_llm_manager, ChatMessage, ChatRequest, LlmError, Provider};
use crate::config::with_config;
use crate::errors::InternalError;
use crate::llm_analysis::cache::AnalysisCache;
use crate::llm_analysis::db::{record_decision, with_analysis_db};
use crate::llm_analysis::error::{Error, Result};
use crate::llm_analysis::prompts::{
    get_entry_analysis_prompt, get_exit_analysis_prompt, get_filter_prompt, PromptBuilder,
};
use crate::llm_analysis::schemas::{validate_json_response, FilterDecision, TradeDecision};
use crate::llm_analysis::types::{
    AnalysisDecision, DecisionRecord, EvaluationContext, EvaluationResult, Factor, Impact,
    Priority, RiskLevel,
};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::OnceCell;

/// Global AI engine singleton
static ANALYSIS_ENGINE: OnceCell<Arc<AnalysisEngine>> = OnceCell::const_new();

/// Initialize the global AI engine
pub async fn init_analysis_engine() -> Result<()> {
    let engine = AnalysisEngine::new();
    ANALYSIS_ENGINE.set(Arc::new(engine)).map_err(|_| {
        Error::Internal(InternalError::InvariantViolation {
            message: "AI engine already initialized".to_owned(),
        })
    })
}

/// Get the global AI engine
pub fn get_analysis_engine() -> Arc<AnalysisEngine> {
    ANALYSIS_ENGINE
        .get()
        .expect("AI engine not initialized - call init_analysis_engine() first")
        .clone()
}

/// Try to get the global AI engine (non-panicking version)
pub fn try_get_analysis_engine() -> Option<Arc<AnalysisEngine>> {
    ANALYSIS_ENGINE.get().cloned()
}

/// Main AI engine that orchestrates LLM calls, caching, and decision processing
pub struct AnalysisEngine {
    cache: Arc<AnalysisCache>,
}

impl AnalysisEngine {
    /// Create a new AI engine
    pub fn new() -> Self {
        let cache_ttl = with_config(|cfg| cfg.ai.cache_ttl_seconds);
        Self {
            cache: Arc::new(AnalysisCache::new(cache_ttl)),
        }
    }

    /// Evaluate a token for filtering
    pub async fn evaluate_filter(
        &self,
        context: EvaluationContext,
        priority: Priority,
    ) -> Result<EvaluationResult> {
        // Check if AI is enabled
        let (ai_enabled, filtering_enabled) =
            with_config(|cfg| (cfg.ai.enabled, cfg.ai.filtering_enabled));

        if !ai_enabled || !filtering_enabled {
            return Err(Error::Disabled);
        }

        // Check cache first
        if let Some(cached_decision) = self.cache.get(&context.mint, "filter", priority) {
            return Ok(EvaluationResult {
                decision: cached_decision,
                cached: true,
            });
        }

        // Get provider and model from config
        let (provider_name, bypass_cache) =
            with_config(|cfg| (cfg.ai.default_provider.clone(), cfg.ai.trading_bypass_cache));

        let provider =
            Provider::from_str(&provider_name).ok_or_else(|| Error::ProviderNotConfigured {
                provider: provider_name.clone(),
            })?;

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

        // Convert to AnalysisDecision
        let decision =
            self.convert_filter_decision(filter_decision, response, latency_ms, provider)?;

        // Cache the result (unless bypass cache is enabled for high priority)
        if !bypass_cache || priority != Priority::High {
            self.cache.insert(&context.mint, "filter", decision.clone());
        }

        // Record decision in history
        self.record_decision_history(&context.mint, None, &decision, false);

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
                Provider::Ollama => {
                    return cfg.ai.providers.ollama.model.clone();
                }
            };

            if !provider_config.model.is_empty() {
                provider_config.model.clone()
            } else {
                // Default models for each provider
                match provider {
                    Provider::OpenAi => "gpt-4".to_owned(),
                    Provider::Anthropic => "claude-3-5-sonnet-20241022".to_owned(),
                    Provider::Groq => "llama-3.1-70b-versatile".to_owned(),
                    Provider::DeepSeek => "deepseek-chat".to_owned(),
                    Provider::Gemini => "gemini-pro".to_owned(),
                    Provider::Ollama => "llama3.2".to_owned(),
                    Provider::Together => "meta-llama/Llama-3-70b-chat-hf".to_owned(),
                    Provider::OpenRouter => "openai/gpt-4".to_owned(),
                    Provider::Mistral => "mistral-large-latest".to_owned(),
                }
            }
        })
    }

    /// Convert FilterDecision to AnalysisDecision
    fn convert_filter_decision(
        &self,
        filter: FilterDecision,
        response: crate::apis::llm::ChatResponse,
        latency_ms: f64,
        provider: Provider,
    ) -> Result<AnalysisDecision> {
        use crate::llm_analysis::schemas::FilterAction;

        let decision = match filter.decision {
            FilterAction::Pass => "pass".to_owned(),
            FilterAction::Reject => "reject".to_owned(),
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

        Ok(AnalysisDecision {
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
    fn map_llm_error(&self, error: LlmError) -> Error {
        match error {
            LlmError::ProviderDisabled { provider } => Error::ProviderNotConfigured { provider },
            LlmError::RateLimited { retry_after_ms, .. } => Error::RateLimited {
                retry_after_secs: retry_after_ms.map(|ms| ms / 1000),
            },
            LlmError::Timeout { timeout_ms, .. } => Error::Timeout {
                waited_ms: timeout_ms,
            },
            other => Error::Apis(crate::apis::Error::from(other)),
        }
    }

    /// Evaluate a token for entry (trading decision)
    pub async fn evaluate_entry(
        &self,
        context: &EvaluationContext,
        priority: Priority,
    ) -> Result<EvaluationResult> {
        // Check if AI is enabled
        let (ai_enabled, entry_enabled) =
            with_config(|cfg| (cfg.ai.enabled, cfg.ai.entry_analysis_enabled));

        if !ai_enabled || !entry_enabled {
            return Err(Error::Disabled);
        }

        // Check cache first (unless high priority)
        if priority != Priority::High {
            if let Some(cached_decision) = self.cache.get(&context.mint, "entry", priority) {
                return Ok(EvaluationResult {
                    decision: cached_decision,
                    cached: true,
                });
            }
        }

        // Get provider and model from config
        let provider_name = with_config(|cfg| cfg.ai.default_provider.clone());

        let provider =
            Provider::from_str(&provider_name).ok_or_else(|| Error::ProviderNotConfigured {
                provider: provider_name.clone(),
            })?;

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

        // Convert to AnalysisDecision
        let decision =
            self.convert_trade_decision(trade_decision, response, latency_ms, provider)?;

        // Cache the result (unless bypass cache is enabled for high priority)
        let bypass_cache = with_config(|cfg| cfg.ai.trading_bypass_cache);
        if !bypass_cache || priority != Priority::High {
            self.cache.insert(&context.mint, "entry", decision.clone());
        }

        // Record decision in history
        self.record_decision_history(&context.mint, None, &decision, false);

        Ok(EvaluationResult {
            decision,
            cached: false,
        })
    }

    /// Evaluate a position for exit
    pub async fn evaluate_exit(
        &self,
        context: &EvaluationContext,
        _priority: Priority,
    ) -> Result<EvaluationResult> {
        // Check if AI is enabled
        let (ai_enabled, exit_enabled) =
            with_config(|cfg| (cfg.ai.enabled, cfg.ai.exit_analysis_enabled));

        if !ai_enabled || !exit_enabled {
            return Err(Error::Disabled);
        }

        // Exit analysis should always be fresh (no cache for exit decisions)
        let provider_name = with_config(|cfg| cfg.ai.default_provider.clone());

        let provider =
            Provider::from_str(&provider_name).ok_or_else(|| Error::ProviderNotConfigured {
                provider: provider_name.clone(),
            })?;

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

        // Convert to AnalysisDecision
        let decision =
            self.convert_trade_decision(trade_decision, response, latency_ms, provider)?;

        // Record decision in history
        self.record_decision_history(&context.mint, None, &decision, false);

        Ok(EvaluationResult {
            decision,
            cached: false,
        })
    }

    /// Convert TradeDecision to AnalysisDecision
    fn convert_trade_decision(
        &self,
        trade: TradeDecision,
        response: crate::apis::llm::ChatResponse,
        latency_ms: f64,
        provider: Provider,
    ) -> Result<AnalysisDecision> {
        use crate::llm_analysis::schemas::TradeAction;

        let decision = match trade.decision {
            TradeAction::Buy => "buy".to_owned(),
            TradeAction::Sell => "sell".to_owned(),
            TradeAction::Hold => "hold".to_owned(),
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

        Ok(AnalysisDecision {
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

    /// Record a decision in history database
    fn record_decision_history(
        &self,
        mint: &str,
        symbol: Option<&str>,
        decision: &AnalysisDecision,
        cached: bool,
    ) {
        let risk_level = match decision.risk_level {
            RiskLevel::Low => "low",
            RiskLevel::Medium => "medium",
            RiskLevel::High => "high",
            RiskLevel::Critical => "critical",
        };

        let record = DecisionRecord {
            id: 0, // Will be set by database
            mint: mint.to_string(),
            symbol: symbol.map(|s| s.to_string()),
            decision: decision.decision.clone(),
            confidence: decision.confidence,
            reasoning: Some(decision.reasoning.clone()),
            risk_level: Some(risk_level.to_string()),
            provider: decision.provider.clone(),
            model: Some(decision.model.clone()),
            tokens_used: decision.tokens_used,
            latency_ms: decision.latency_ms,
            cached,
            created_at: String::new(), // Will be set by database
        };

        // Record in background to not block the response
        if let Err(e) = with_analysis_db(|db| record_decision(db, &record)) {
            // Log but don't fail the operation
            crate::logger::debug(
                crate::logger::LogTag::Filtering,
                &format!("Failed to record AI decision in history: {e}"),
            );
        }
    }
}

impl Default for AnalysisEngine {
    fn default() -> Self {
        Self::new()
    }
}
