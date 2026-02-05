//! AI Chat Engine Module
//!
//! Main orchestrator for AI chat with MCP-like tool calling.
//! Handles conversation flow, tool execution, and permission management.

use crate::ai::chat_db;
use crate::ai::tools::{create_tool_registry, ToolRegistry, ToolResult};
use crate::ai::types::AiError;
use crate::apis::llm::{
    get_llm_manager, ChatMessage as LlmChatMessage, ChatRequest as LlmChatRequest, MessageRole,
    Provider,
};
use crate::logger::{self, LogTag};
use once_cell::sync::Lazy;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{OnceCell, RwLock};

// =============================================================================
// CONSTANTS
// =============================================================================

const MAX_TOOL_ITERATIONS: usize = 5;

// =============================================================================
// REGEX PATTERNS (Compiled once at startup)
// =============================================================================

/// Regex for JSON code blocks in LLM responses
static JSON_CODE_BLOCK_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?s)```json\s*(\{.+?\})\s*```").expect("Invalid JSON pattern regex"));

/// Regex for loose JSON tool calls without code blocks
static LOOSE_JSON_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"(?s)\{[^{}]*"tool_calls"[^{}]*\[.+?\]\s*\}"#)
        .expect("Invalid loose JSON pattern regex")
});

// =============================================================================
// GLOBAL INSTANCE
// =============================================================================

/// Global chat engine singleton
static CHAT_ENGINE: OnceCell<Arc<ChatEngine>> = OnceCell::const_new();

/// Initialize the global chat engine
pub async fn init_chat_engine() -> Result<(), String> {
    let engine = ChatEngine::new();
    CHAT_ENGINE
        .set(Arc::new(engine))
        .map_err(|_| "Chat engine already initialized".to_string())
}

/// Get the global chat engine
pub fn get_chat_engine() -> Arc<ChatEngine> {
    CHAT_ENGINE
        .get()
        .expect("Chat engine not initialized - call init_chat_engine() first")
        .clone()
}

/// Try to get the global chat engine (non-panicking version)
pub fn try_get_chat_engine() -> Option<Arc<ChatEngine>> {
    CHAT_ENGINE.get().cloned()
}

// =============================================================================
// TYPES
// =============================================================================

/// Chat request from user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    pub session_id: i64,
    pub message: String,
    pub context: Option<ChatContext>,
}

/// Optional context for chat
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatContext {
    pub current_token: Option<String>,
    pub current_position: Option<i64>,
}

/// Response to chat request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    pub message_id: i64,
    pub content: String,
    pub tool_calls: Vec<ToolCallInfo>,
    pub pending_confirmations: Vec<PendingConfirmation>,
    pub is_complete: bool,
}

/// Information about a tool call
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallInfo {
    pub tool_name: String,
    pub input: serde_json::Value,
    pub output: Option<serde_json::Value>,
    pub status: ToolCallStatus,
}

/// Status of a tool call
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ToolCallStatus {
    Executed,
    PendingConfirmation,
    Denied,
    Failed,
}

/// Pending confirmation for a tool call
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingConfirmation {
    pub confirmation_id: String,
    pub tool_name: String,
    pub description: String,
    pub input: serde_json::Value,
}

/// Parsed tool call from LLM response
#[derive(Debug, Clone)]
struct ToolCall {
    name: String,
    arguments: serde_json::Value,
}

/// Pending confirmation in memory
#[derive(Debug, Clone)]
struct ConfirmationState {
    session_id: i64,
    message_id: i64,
    tool_calls: Vec<ToolCall>,
    current_index: usize,
    created_at: std::time::Instant,
}

// =============================================================================
// CONFIRMATION MANAGER
// =============================================================================

/// Simple confirmation manager for tool calls requiring user approval
struct ConfirmationManager {
    pending: Arc<RwLock<HashMap<String, ConfirmationState>>>,
}

impl ConfirmationManager {
    fn new() -> Self {
        Self {
            pending: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    async fn create_confirmation(
        &self,
        session_id: i64,
        message_id: i64,
        tool_calls: Vec<ToolCall>,
    ) -> String {
        let confirmation_id = uuid::Uuid::new_v4().to_string();
        let state = ConfirmationState {
            session_id,
            message_id,
            tool_calls,
            current_index: 0,
            created_at: std::time::Instant::now(),
        };

        let mut pending = self.pending.write().await;

        // Cleanup expired confirmations (older than 10 minutes)
        let timeout = std::time::Duration::from_secs(600);
        pending.retain(|_, v| v.created_at.elapsed() < timeout);

        // Limit max pending confirmations per session (prevent DoS)
        let session_count = pending
            .values()
            .filter(|v| v.session_id == session_id)
            .count();
        if session_count >= 10 {
            logger::warning(
                LogTag::Api,
                &format!("Session {} has too many pending confirmations", session_id),
            );
        }

        pending.insert(confirmation_id.clone(), state);

        confirmation_id
    }

    async fn get_confirmation(&self, confirmation_id: &str) -> Option<ConfirmationState> {
        let pending = self.pending.read().await;
        let state = pending.get(confirmation_id)?;

        // Check if confirmation has expired (10 minutes)
        if state.created_at.elapsed() > std::time::Duration::from_secs(600) {
            return None;
        }

        Some(state.clone())
    }

    async fn remove_confirmation(&self, confirmation_id: &str) {
        let mut pending = self.pending.write().await;
        pending.remove(confirmation_id);
    }
}

// =============================================================================
// CHAT ENGINE
// =============================================================================

/// Main chat engine that orchestrates conversation and tool calling
pub struct ChatEngine {
    tool_registry: Arc<ToolRegistry>,
    confirmation_manager: Arc<ConfirmationManager>,
}

impl ChatEngine {
    /// Create a new chat engine
    pub fn new() -> Self {
        let tool_registry = Arc::new(create_tool_registry());
        let confirmation_manager = Arc::new(ConfirmationManager::new());

        Self {
            tool_registry,
            confirmation_manager,
        }
    }

    /// Process a user message and generate response
    pub async fn process_message(&self, request: ChatRequest) -> Result<ChatResponse, AiError> {
        // Get database pool
        let pool = chat_db::get_chat_pool()
            .ok_or_else(|| AiError::ValidationError("Chat database not initialized".to_string()))?;

        // Add user message to history
        let user_message_id =
            chat_db::add_message(&pool, request.session_id, "user", &request.message, None)
                .map_err(|e| AiError::ParseError(format!("Failed to save user message: {}", e)))?;

        logger::debug(
            LogTag::Api,
            &format!(
                "Processing chat message for session {} (message {})",
                request.session_id, user_message_id
            ),
        );

        // Load conversation history
        let history = chat_db::get_messages(&pool, request.session_id)
            .map_err(|e| AiError::ParseError(format!("Failed to load history: {}", e)))?;

        // Build messages for LLM (system + history)
        let mut messages = self.build_messages(&history, &request.context)?;

        // Execute tool calling loop
        let mut tool_calls_info = Vec::new();
        let mut iteration = 0;

        let final_content = loop {
            if iteration >= MAX_TOOL_ITERATIONS {
                logger::warning(
                    LogTag::Api,
                    &format!("Max tool iterations ({}) reached", MAX_TOOL_ITERATIONS),
                );
                break "I've reached the maximum number of tool calls. Please try breaking this down into smaller requests.".to_string();
            }

            // Call LLM
            let llm_response = self.call_llm(&messages).await?;
            let content = llm_response.content.trim();

            logger::debug(LogTag::Api, &format!("LLM response: {}", content));

            // Parse tool calls from response
            let tool_calls = self.parse_tool_calls(content);

            if tool_calls.is_empty() {
                // No more tool calls, we're done
                break content.to_string();
            }

            logger::debug(
                LogTag::Api,
                &format!("Parsed {} tool calls", tool_calls.len()),
            );

            // Execute tools
            let (results, pending) = self
                .execute_tools(tool_calls, request.session_id, user_message_id, &pool)
                .await;

            // If there are pending confirmations, return early
            if !pending.is_empty() {
                logger::info(
                    LogTag::Api,
                    &format!("Waiting for {} confirmations", pending.len()),
                );

                return Ok(ChatResponse {
                    message_id: user_message_id,
                    content: content.to_string(),
                    tool_calls: results,
                    pending_confirmations: pending,
                    is_complete: false,
                });
            }

            // Add tool results to conversation
            tool_calls_info.extend(results.clone());

            // Build tool results message
            let tool_results_text = self.format_tool_results(&results);
            messages.push(LlmChatMessage::assistant(content));
            messages.push(LlmChatMessage::user(format!(
                "Tool execution results:\n{}",
                tool_results_text
            )));

            iteration += 1;
        };

        // Save assistant response
        let tool_calls_json = if tool_calls_info.is_empty() {
            None
        } else {
            match serde_json::to_string(&tool_calls_info) {
                Ok(json) => Some(json),
                Err(e) => {
                    logger::warning(
                        LogTag::Api,
                        &format!("Failed to serialize tool calls: {}", e),
                    );
                    None
                }
            }
        };

        let assistant_message_id = chat_db::add_message(
            &pool,
            request.session_id,
            "assistant",
            &final_content,
            tool_calls_json.as_deref(),
        )
        .map_err(|e| AiError::ParseError(format!("Failed to save assistant message: {}", e)))?;

        logger::info(
            LogTag::Api,
            &format!(
                "Chat response generated for session {} (message {})",
                request.session_id, assistant_message_id
            ),
        );

        Ok(ChatResponse {
            message_id: assistant_message_id,
            content: final_content,
            tool_calls: tool_calls_info,
            pending_confirmations: Vec::new(),
            is_complete: true,
        })
    }

    /// Process a confirmation response from user
    pub async fn process_confirmation(
        &self,
        confirmation_id: &str,
        approved: bool,
    ) -> Result<ChatResponse, AiError> {
        // Get confirmation state
        let state = self
            .confirmation_manager
            .get_confirmation(confirmation_id)
            .await
            .ok_or_else(|| {
                AiError::ValidationError("Confirmation not found or expired".to_string())
            })?;

        // Remove confirmation from pending
        self.confirmation_manager
            .remove_confirmation(confirmation_id)
            .await;

        if !approved {
            // User denied the tool call
            logger::info(LogTag::Api, "User denied tool execution");

            return Ok(ChatResponse {
                message_id: state.message_id,
                content: "Tool execution was denied by user.".to_string(),
                tool_calls: vec![ToolCallInfo {
                    tool_name: state.tool_calls[state.current_index].name.clone(),
                    input: state.tool_calls[state.current_index].arguments.clone(),
                    output: None,
                    status: ToolCallStatus::Denied,
                }],
                pending_confirmations: Vec::new(),
                is_complete: true,
            });
        }

        // Get database pool
        let pool = chat_db::get_chat_pool()
            .ok_or_else(|| AiError::ValidationError("Chat database not initialized".to_string()))?;

        // Execute the approved tool
        let tool_call = &state.tool_calls[state.current_index];
        let result = self
            .execute_single_tool(tool_call, state.message_id, &pool)
            .await;

        logger::info(
            LogTag::Api,
            &format!("Tool {} executed after approval", tool_call.name),
        );

        // Return continuation message
        Ok(ChatResponse {
            message_id: state.message_id,
            content: format!("Tool {} executed successfully.", tool_call.name),
            tool_calls: vec![result],
            pending_confirmations: Vec::new(),
            is_complete: true,
        })
    }

    // =========================================================================
    // PRIVATE METHODS
    // =========================================================================

    /// Build messages for LLM including system prompt and history
    fn build_messages(
        &self,
        history: &[chat_db::ChatMessage],
        context: &Option<ChatContext>,
    ) -> Result<Vec<LlmChatMessage>, AiError> {
        let mut messages = Vec::new();

        // Add system prompt
        let system_prompt = self.build_system_prompt(context);
        messages.push(LlmChatMessage::system(system_prompt));

        // Add conversation history (skip the last user message - it's the current request)
        // Note: history already includes the new user message we just saved to DB,
        // so we skip it to avoid duplication in the LLM context
        let history_to_process = if history.is_empty() {
            history
        } else {
            &history[..history.len() - 1]
        };

        for msg in history_to_process {
            let role = match msg.role.as_str() {
                "user" => MessageRole::User,
                "assistant" => MessageRole::Assistant,
                "system" => MessageRole::System,
                _ => continue,
            };

            messages.push(LlmChatMessage {
                role,
                content: msg.content.clone(),
            });
        }

        // Add the current user message
        if let Some(last_msg) = history.last() {
            if last_msg.role == "user" {
                messages.push(LlmChatMessage::user(last_msg.content.clone()));
            }
        }

        Ok(messages)
    }

    /// Build system prompt with tool definitions
    fn build_system_prompt(&self, context: &Option<ChatContext>) -> String {
        let mut prompt = String::from(
            "You are an AI assistant for ScreenerBot, a Solana trading bot. \
             You help users analyze tokens, manage positions, and configure the bot.\n\n",
        );

        // Add context if available
        if let Some(ctx) = context {
            if let Some(token) = &ctx.current_token {
                prompt.push_str(&format!("Current token context: {}\n", token));
            }
            if let Some(position_id) = ctx.current_position {
                prompt.push_str(&format!("Current position context: {}\n", position_id));
            }
            prompt.push('\n');
        }

        // Add tool calling instructions - CRITICAL: Be extremely explicit
        prompt.push_str("## CRITICAL INSTRUCTIONS - TOOL USAGE\n\n");
        prompt.push_str(
            "YOU MUST USE TOOLS FOR ALL DATA REQUESTS AND ACTIONS. This is not optional.\n\n",
        );

        prompt.push_str("### ALWAYS Use Tools For:\n");
        prompt.push_str("- ANY mention of: balance, positions, tokens, analysis, market data, trading, configuration\n");
        prompt.push_str("- ANY request containing token addresses or position IDs\n");
        prompt.push_str("- ANY action words: analyze, check, show, get, fetch, buy, sell, set, configure, list\n");
        prompt.push_str("- User explicitly mentions a tool name (e.g., 'use analyze_token')\n");
        prompt.push_str("- Even if the user is polite or indirect - call the tool anyway\n\n");

        prompt.push_str("### ONLY Respond Without Tools For:\n");
        prompt.push_str("- Purely conversational: greetings, thank you, goodbye\n");
        prompt.push_str("- Abstract questions: 'how does trading work?', 'what is Solana?'\n");
        prompt.push_str("- Requests for help/clarification that don't involve specific data\n\n");

        prompt.push_str("### REQUIRED Tool Call Format:\n");
        prompt.push_str("When calling tools, output ONLY the JSON code block - nothing else.\n");
        prompt.push_str("DO NOT add explanatory text before or after the JSON.\n\n");
        prompt.push_str("Format:\n");
        prompt.push_str("```json\n");
        prompt.push_str("{\n");
        prompt.push_str("  \"tool_calls\": [\n");
        prompt.push_str("    {\n");
        prompt.push_str("      \"name\": \"tool_name\",\n");
        prompt.push_str("      \"arguments\": {\n");
        prompt.push_str("        \"param1\": \"value1\",\n");
        prompt.push_str("        \"param2\": 123\n");
        prompt.push_str("      }\n");
        prompt.push_str("    }\n");
        prompt.push_str("  ]\n");
        prompt.push_str("}\n");
        prompt.push_str("```\n\n");

        prompt.push_str("### Examples (FOLLOW THESE EXACTLY):\n\n");

        prompt.push_str("**User:** \"What is my balance?\"\n");
        prompt.push_str("**Assistant:**\n");
        prompt.push_str(
            "```json\n{\"tool_calls\": [{\"name\": \"get_balance\", \"arguments\": {}}]}\n```\n\n",
        );

        prompt
            .push_str("**User:** \"Analyze token 7xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgAsU\"\n");
        prompt.push_str("**Assistant:**\n");
        prompt.push_str("```json\n{\"tool_calls\": [{\"name\": \"analyze_token\", \"arguments\": {\"mint_address\": \"7xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgAsU\"}}]}\n```\n\n");

        prompt.push_str("**User:** \"Use the analyze_token tool to analyze this token: DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263\"\n");
        prompt.push_str("**Assistant:**\n");
        prompt.push_str("```json\n{\"tool_calls\": [{\"name\": \"analyze_token\", \"arguments\": {\"mint_address\": \"DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263\"}}]}\n```\n\n");

        prompt.push_str("**User:** \"Show position 5\"\n");
        prompt.push_str("**Assistant:**\n");
        prompt.push_str("```json\n{\"tool_calls\": [{\"name\": \"get_position\", \"arguments\": {\"position_id\": 5}}]}\n```\n\n");

        prompt.push_str("**User:** \"Check my open positions\"\n");
        prompt.push_str("**Assistant:**\n");
        prompt.push_str("```json\n{\"tool_calls\": [{\"name\": \"get_positions\", \"arguments\": {}}]}\n```\n\n");

        prompt.push_str("**User:** \"How does the bot work?\"\n");
        prompt.push_str("**Assistant:** ScreenerBot is a Solana trading bot that monitors tokens and executes trades based on your configured strategies. It can automatically buy and sell tokens based on market conditions.\n\n");

        prompt.push_str("**User:** \"Hello!\"\n");
        prompt.push_str("**Assistant:** Hello! I'm your ScreenerBot assistant. I can help you analyze tokens, check positions, manage trades, and configure settings. What would you like to do?\n\n");

        // List all tools with full parameter schemas
        prompt.push_str("## AVAILABLE TOOLS\n\n");
        let definitions = self.tool_registry.list_definitions();
        for def in definitions {
            let confirmation_note = if def.requires_confirmation {
                " ⚠️ [REQUIRES USER CONFIRMATION]"
            } else {
                ""
            };

            prompt.push_str(&format!("### {}{}\n", def.name, confirmation_note));
            prompt.push_str(&format!("{}\n\n", def.description));

            // Add parameter schema
            if let Some(properties) = def.parameters.get("properties") {
                if let Some(obj) = properties.as_object() {
                    if !obj.is_empty() {
                        prompt.push_str("**Parameters:**\n");

                        let required = def
                            .parameters
                            .get("required")
                            .and_then(|r| r.as_array())
                            .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>())
                            .unwrap_or_default();

                        for (param_name, param_schema) in obj {
                            let param_type = param_schema
                                .get("type")
                                .and_then(|t| t.as_str())
                                .unwrap_or("any");
                            let param_desc = param_schema
                                .get("description")
                                .and_then(|d| d.as_str())
                                .unwrap_or("");
                            let is_required = required.contains(&param_name.as_str());
                            let required_marker = if is_required {
                                " (required)"
                            } else {
                                " (optional)"
                            };

                            prompt.push_str(&format!(
                                "- `{}`: {} - {}{}\n",
                                param_name, param_type, param_desc, required_marker
                            ));
                        }
                        prompt.push_str("\n");
                    } else {
                        prompt.push_str("**Parameters:** None\n\n");
                    }
                }
            } else {
                prompt.push_str("**Parameters:** None\n\n");
            }
        }

        prompt.push_str("\n## CRITICAL RULES - MUST FOLLOW\n");
        prompt.push_str("1. DEFAULT ACTION: When in doubt, CALL A TOOL. Tool calling is preferred over natural responses.\n");
        prompt.push_str(
            "2. NEVER add explanatory text with tool calls - ONLY output the JSON code block\n",
        );
        prompt.push_str(
            "3. NEVER refuse a tool call - if user mentions ANY data or action, call the tool\n",
        );
        prompt.push_str("4. ALWAYS extract token addresses, position IDs, and other parameters from user messages\n");
        prompt.push_str("5. For confirmation-required tools: Call them anyway - the system handles confirmations\n");
        prompt.push_str(
            "6. Multiple tools: Add multiple objects to tool_calls array in a single JSON block\n",
        );
        prompt.push_str("7. Parameter types: Match exactly (string, integer, boolean) as shown in tool schemas\n");
        prompt.push_str("8. Natural responses: Only for greetings, abstract questions, or when NO tool is relevant\n");

        prompt
    }

    /// Call LLM with messages
    ///
    /// TODO: Add native function calling support for providers that support it:
    /// - OpenAI: Use 'tools' parameter with function definitions
    /// - Anthropic: Use 'tools' parameter (Claude 3+)
    /// - This would be more reliable than text-based parsing
    /// Current approach uses text-based tool calling (JSON in markdown) which is provider-agnostic
    /// but less reliable. Consider adding provider-specific native tool calling in the future.
    async fn call_llm(
        &self,
        messages: &[LlmChatMessage],
    ) -> Result<crate::apis::llm::ChatResponse, AiError> {
        let llm_manager = get_llm_manager();

        // Get default provider and model from config
        let provider_name = crate::config::with_config(|cfg| cfg.ai.default_provider.clone());
        let provider = Provider::from_str(&provider_name)
            .ok_or_else(|| AiError::ProviderNotConfigured(provider_name.clone()))?;

        let model = self.get_model_for_provider(provider);

        let request = LlmChatRequest::new(model, messages.to_vec())
            .with_temperature(0.7)
            .with_max_tokens(2000);

        llm_manager
            .call(provider, request)
            .await
            .map_err(|e| AiError::LlmError(format!("LLM call failed: {}", e)))
    }

    /// Get the appropriate model for a provider
    fn get_model_for_provider(&self, provider: Provider) -> String {
        crate::config::with_config(|cfg| {
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
                    Provider::OpenAi => "gpt-4".to_string(),
                    Provider::Anthropic => "claude-3-5-sonnet-20241022".to_string(),
                    Provider::Groq => "llama-3.1-70b-versatile".to_string(),
                    Provider::DeepSeek => "deepseek-chat".to_string(),
                    Provider::Gemini => "gemini-pro".to_string(),
                    Provider::Ollama => "llama3.2".to_string(),
                    Provider::Together => "meta-llama/Llama-3-70b-chat-hf".to_string(),
                    Provider::OpenRouter => "openai/gpt-4".to_string(),
                    Provider::Mistral => "mistral-large-latest".to_string(),
                }
            }
        })
    }

    /// Parse tool calls from LLM response
    fn parse_tool_calls(&self, response: &str) -> Vec<ToolCall> {
        let mut tool_calls = Vec::new();

        // Strategy 1: Look for JSON code blocks with pre-compiled regex
        for cap in JSON_CODE_BLOCK_PATTERN.captures_iter(response) {
            if let Some(json_str) = cap.get(1) {
                logger::debug(
                    LogTag::Api,
                    &format!("Found JSON code block: {}", json_str.as_str()),
                );

                // Try to parse the JSON
                match serde_json::from_str::<serde_json::Value>(json_str.as_str()) {
                    Ok(json_value) => {
                        // Extract tool_calls array
                        if let Some(calls) = json_value.get("tool_calls").and_then(|v| v.as_array())
                        {
                            for call in calls {
                                if let (Some(name), Some(args)) = (
                                    call.get("name").and_then(|v| v.as_str()),
                                    call.get("arguments"),
                                ) {
                                    logger::debug(
                                        LogTag::Api,
                                        &format!(
                                            "Parsed tool call: {} with args: {:?}",
                                            name, args
                                        ),
                                    );
                                    tool_calls.push(ToolCall {
                                        name: name.to_string(),
                                        arguments: args.clone(),
                                    });
                                }
                            }
                        }
                    }
                    Err(e) => {
                        logger::warning(
                            LogTag::Api,
                            &format!("Failed to parse JSON from code block: {}", e),
                        );
                    }
                }
            }
        }

        // Strategy 2: Try parsing the entire response as JSON (for models that output raw JSON)
        if tool_calls.is_empty() {
            if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(response) {
                logger::debug(LogTag::Api, "Parsing entire response as JSON");

                if let Some(calls) = json_value.get("tool_calls").and_then(|v| v.as_array()) {
                    for call in calls {
                        if let (Some(name), Some(args)) = (
                            call.get("name").and_then(|v| v.as_str()),
                            call.get("arguments"),
                        ) {
                            tool_calls.push(ToolCall {
                                name: name.to_string(),
                                arguments: args.clone(),
                            });
                        }
                    }
                }
            }
        }

        // Strategy 3: Look for any JSON-like structure with tool_calls using pre-compiled regex
        if tool_calls.is_empty() {
            if let Some(cap) = LOOSE_JSON_PATTERN.find(response) {
                let potential_json = cap.as_str();
                logger::debug(
                    LogTag::Api,
                    &format!(
                        "Found potential JSON without code block: {}",
                        potential_json
                    ),
                );

                if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(potential_json) {
                    if let Some(calls) = json_value.get("tool_calls").and_then(|v| v.as_array()) {
                        for call in calls {
                            if let (Some(name), Some(args)) = (
                                call.get("name").and_then(|v| v.as_str()),
                                call.get("arguments"),
                            ) {
                                tool_calls.push(ToolCall {
                                    name: name.to_string(),
                                    arguments: args.clone(),
                                });
                            }
                        }
                    }
                }
            }
        }

        if tool_calls.is_empty() {
            logger::debug(LogTag::Api, "No tool calls found in response");
        }

        tool_calls
    }

    /// Execute tools and handle permissions
    async fn execute_tools(
        &self,
        tool_calls: Vec<ToolCall>,
        session_id: i64,
        message_id: i64,
        pool: &Pool<SqliteConnectionManager>,
    ) -> (Vec<ToolCallInfo>, Vec<PendingConfirmation>) {
        let mut results = Vec::new();
        let mut pending_confirmations = Vec::new();

        for tool_call in tool_calls.iter() {
            // Check if tool exists
            let tool = match self.tool_registry.get(&tool_call.name) {
                Some(t) => t,
                None => {
                    results.push(ToolCallInfo {
                        tool_name: tool_call.name.clone(),
                        input: tool_call.arguments.clone(),
                        output: Some(serde_json::json!({"error": "Tool not found"})),
                        status: ToolCallStatus::Failed,
                    });
                    continue;
                }
            };

            let definition = tool.definition();

            // Check if confirmation is required
            if definition.requires_confirmation {
                // Create pending confirmation for THIS tool only
                let single_tool_call = vec![tool_call.clone()];
                let confirmation_id = self
                    .confirmation_manager
                    .create_confirmation(session_id, message_id, single_tool_call)
                    .await;

                pending_confirmations.push(PendingConfirmation {
                    confirmation_id,
                    tool_name: tool_call.name.clone(),
                    description: definition.description.clone(),
                    input: tool_call.arguments.clone(),
                });

                results.push(ToolCallInfo {
                    tool_name: tool_call.name.clone(),
                    input: tool_call.arguments.clone(),
                    output: None,
                    status: ToolCallStatus::PendingConfirmation,
                });

                // Stop processing more tools - wait for confirmation
                break;
            }

            // Execute tool directly
            let result = self.execute_single_tool(tool_call, message_id, pool).await;
            results.push(result);
        }

        (results, pending_confirmations)
    }

    /// Execute a single tool
    async fn execute_single_tool(
        &self,
        tool_call: &ToolCall,
        message_id: i64,
        pool: &Pool<SqliteConnectionManager>,
    ) -> ToolCallInfo {
        let tool = match self.tool_registry.get(&tool_call.name) {
            Some(t) => t,
            None => {
                return ToolCallInfo {
                    tool_name: tool_call.name.clone(),
                    input: tool_call.arguments.clone(),
                    output: Some(serde_json::json!({"error": "Tool not found"})),
                    status: ToolCallStatus::Failed,
                };
            }
        };

        // Execute the tool with timeout (30 seconds)
        let execution_timeout = tokio::time::Duration::from_secs(30);
        let result = match tokio::time::timeout(
            execution_timeout,
            tool.execute(tool_call.arguments.clone()),
        )
        .await
        {
            Ok(r) => r,
            Err(_) => {
                logger::error(
                    LogTag::Api,
                    &format!("Tool {} execution timed out after 30s", tool_call.name),
                );
                ToolResult::error("Tool execution timed out after 30 seconds")
            }
        };

        // Record execution in database
        let status = if result.success { "success" } else { "error" };
        let output_json = match serde_json::to_string(&result) {
            Ok(json) => json,
            Err(e) => {
                logger::error(
                    LogTag::Api,
                    &format!("Failed to serialize tool result: {}", e),
                );
                serde_json::json!({"error": "Failed to serialize result"}).to_string()
            }
        };

        if let Err(e) = chat_db::add_tool_execution(
            pool,
            message_id,
            &tool_call.name,
            &serde_json::to_string(&tool_call.arguments).unwrap_or_default(),
            &output_json,
            status,
        ) {
            logger::warning(
                LogTag::Api,
                &format!("Failed to record tool execution: {}", e),
            );
        }

        ToolCallInfo {
            tool_name: tool_call.name.clone(),
            input: tool_call.arguments.clone(),
            output: if result.success {
                result.data
            } else {
                Some(serde_json::json!({"error": result.error.unwrap_or_default()}))
            },
            status: if result.success {
                ToolCallStatus::Executed
            } else {
                ToolCallStatus::Failed
            },
        }
    }

    /// Format tool results for LLM
    fn format_tool_results(&self, results: &[ToolCallInfo]) -> String {
        let mut output = String::new();

        for result in results {
            output.push_str(&format!("\n**{}**:\n", result.tool_name));

            match &result.status {
                ToolCallStatus::Executed => {
                    if let Some(data) = &result.output {
                        output.push_str(&format!(
                            "✅ Success\n{}\n",
                            serde_json::to_string_pretty(data).unwrap_or_else(|_| data.to_string())
                        ));
                    }
                }
                ToolCallStatus::Failed => {
                    if let Some(data) = &result.output {
                        output.push_str(&format!(
                            "❌ Failed\n{}\n",
                            serde_json::to_string_pretty(data).unwrap_or_else(|_| data.to_string())
                        ));
                    }
                }
                ToolCallStatus::PendingConfirmation => {
                    output.push_str("⏳ Pending user confirmation\n");
                }
                ToolCallStatus::Denied => {
                    output.push_str("🚫 Denied by user\n");
                }
            }
        }

        output
    }
}

impl Default for ChatEngine {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_tool_calls() {
        let engine = ChatEngine::new();

        // Test JSON code block
        let response = r#"
Let me check the market data for you.

```json
{
  "tool_calls": [
    {
      "name": "get_market_data",
      "arguments": {
        "mint_address": "So11111111111111111111111111111111111111112"
      }
    }
  ]
}
```

I'll fetch that information now.
        "#;

        let calls = engine.parse_tool_calls(response);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "get_market_data");
    }

    #[test]
    fn test_parse_multiple_tool_calls() {
        let engine = ChatEngine::new();

        let response = r#"
```json
{
  "tool_calls": [
    {
      "name": "get_balance",
      "arguments": {}
    },
    {
      "name": "get_positions",
      "arguments": {}
    }
  ]
}
```
        "#;

        let calls = engine.parse_tool_calls(response);
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].name, "get_balance");
        assert_eq!(calls[1].name, "get_positions");
    }

    #[test]
    fn test_parse_multiline_json() {
        let engine = ChatEngine::new();

        // Test multiline JSON that the old regex would fail on
        let response = r#"
```json
{
  "tool_calls": [
    {
      "name": "analyze_token",
      "arguments": {
        "mint_address": "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263"
      }
    }
  ]
}
```
        "#;

        let calls = engine.parse_tool_calls(response);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "analyze_token");
        assert_eq!(
            calls[0]
                .arguments
                .get("mint_address")
                .and_then(|v| v.as_str()),
            Some("DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263")
        );
    }

    #[test]
    fn test_parse_json_without_code_block() {
        let engine = ChatEngine::new();

        // Some models might output JSON without code blocks
        let response = r#"{"tool_calls": [{"name": "get_balance", "arguments": {}}]}"#;

        let calls = engine.parse_tool_calls(response);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "get_balance");
    }

    #[test]
    fn test_system_prompt_generation() {
        let engine = ChatEngine::new();

        let prompt = engine.build_system_prompt(&None);
        assert!(prompt.contains("ScreenerBot"));
        assert!(prompt.contains("Available Tools"));
        assert!(prompt.contains("tool_calls"));

        // With context
        let context = Some(ChatContext {
            current_token: Some("So11111111111111111111111111111111111111112".to_string()),
            current_position: Some(42),
        });

        let prompt_with_context = engine.build_system_prompt(&context);
        assert!(prompt_with_context.contains("So11111111111111111111111111111111111111112"));
        assert!(prompt_with_context.contains("42"));
    }

    #[test]
    fn test_format_tool_results() {
        let engine = ChatEngine::new();

        let results = vec![
            ToolCallInfo {
                tool_name: "get_balance".to_string(),
                input: serde_json::json!({}),
                output: Some(serde_json::json!({"balance": 10.5})),
                status: ToolCallStatus::Executed,
            },
            ToolCallInfo {
                tool_name: "invalid_tool".to_string(),
                input: serde_json::json!({}),
                output: Some(serde_json::json!({"error": "Tool not found"})),
                status: ToolCallStatus::Failed,
            },
        ];

        let formatted = engine.format_tool_results(&results);
        assert!(formatted.contains("get_balance"));
        assert!(formatted.contains("✅"));
        assert!(formatted.contains("❌"));
    }
}
