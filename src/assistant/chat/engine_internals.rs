//! Assistant chat engine internals.
//!
//! Private implementation methods for ChatEngine.
//! Separated from chat_engine.rs for maintainability.

use super::database;
use super::engine::{ChatEngine, JSON_CODE_BLOCK_PATTERN, LOOSE_JSON_PATTERN};
use super::types::{
    ChatContext, PendingConfirmation, ToolCall, ToolCallInfo, ToolCallStatus, ToolMode,
};
use crate::agent_control::tools::ToolResult;
use crate::apis::llm::{
    get_llm_manager, ChatMessage as LlmChatMessage, ChatRequest as LlmChatRequest, MessageRole,
    Provider,
};
use crate::assistant::error::{Error, Result};
use crate::logger::{self, LogTag};
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use std::time::Duration;

impl ChatEngine {
    // =========================================================================
    // PRIVATE METHODS
    // =========================================================================

    /// Build messages for LLM including system prompt and history
    pub(super) fn build_messages(
        &self,
        history: &[database::ChatMessage],
        context: &Option<ChatContext>,
    ) -> Result<Vec<LlmChatMessage>> {
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
    pub(super) fn build_system_prompt(&self, context: &Option<ChatContext>) -> String {
        let mut prompt = String::with_capacity(8192);
        prompt.push_str(
            "You are an AI assistant for ScreenerBot, a Solana trading bot. \
             You help users analyze tokens, manage positions, and configure the bot.\n\n",
        );

        // Add context if available
        if let Some(ctx) = context {
            if let Some(token) = &ctx.current_token {
                prompt.push_str(&format!("Current token context: {token}\n"));
            }
            if let Some(position_id) = ctx.current_position {
                prompt.push_str(&format!("Current position context: {position_id}\n"));
            }
            prompt.push('\n');
        }

        prompt.push_str("## Tool usage\n\n");
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

        prompt.push_str("Use the provider's function-calling interface whenever it is available. Do not narrate your plan or expose private reasoning. If native function calling is unavailable, use this JSON fallback and output nothing else with it:\n\n");
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
                " [REQUIRES USER CONFIRMATION]"
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
                                .unwrap_or_default();
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

        prompt.push_str("\n## Rules\n");
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

    /// Call the configured LLM with native tool definitions.
    pub(super) async fn call_llm(
        &self,
        messages: &[LlmChatMessage],
    ) -> Result<crate::apis::llm::ChatResponse> {
        let tools = self
            .tool_registry
            .list_definitions()
            .into_iter()
            .map(|definition| crate::apis::llm::ToolDefinition {
                name: definition.name,
                description: definition.description,
                parameters: definition.parameters,
            })
            .collect();
        if let Some(completion) = &self.completion {
            let request = LlmChatRequest::new("test-model", messages.to_vec())
                .with_temperature(0.2)
                .with_max_tokens(4000)
                .with_tools(tools);
            return match tokio::time::timeout(Duration::from_secs(60), completion.complete(request))
                .await
            {
                Ok(result) => result.map_err(|error| Error::Apis(crate::apis::Error::from(error))),
                Err(_) => Err(Error::Timeout { waited_ms: 60_000 }),
            };
        }

        let llm_manager = get_llm_manager();
        let provider_name = crate::config::with_config(|cfg| cfg.ai.default_provider.clone());
        let provider =
            Provider::from_str(&provider_name).ok_or_else(|| Error::ProviderNotConfigured {
                provider: provider_name.clone(),
            })?;
        let model = self.get_model_for_provider(provider);
        let request = LlmChatRequest::new(model, messages.to_vec())
            .with_temperature(0.2)
            .with_max_tokens(4000)
            .with_tools(tools);
        match tokio::time::timeout(Duration::from_secs(60), llm_manager.call(provider, request))
            .await
        {
            Ok(result) => result.map_err(|e| Error::Apis(crate::apis::Error::from(e))),
            Err(_) => Err(Error::Timeout { waited_ms: 60_000 }),
        }
    }

    /// Get the appropriate model for a provider
    pub(super) fn get_model_for_provider(&self, provider: Provider) -> String {
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

    /// Parse tool calls from LLM response
    pub(super) fn parse_tool_calls(&self, response: &str) -> Vec<ToolCall> {
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
                            &format!("Failed to parse JSON from code block: {e}"),
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
    pub(super) async fn execute_tools(
        &self,
        tool_calls: Vec<ToolCall>,
        session_id: i64,
        message_id: i64,
        pool: &Pool<SqliteConnectionManager>,
        headless: bool,
        tool_mode: &ToolMode,
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

            // In headless read-only mode, explicitly deny known trading tools regardless of confirmation flag
            if headless && matches!(tool_mode, ToolMode::ReadOnly) {
                let trading_tools = [
                    "buy_token",
                    "sell_token",
                    "close_position",
                    "update_config",
                    "force_stop",
                ];
                if trading_tools.contains(&tool_call.name.as_str()) {
                    results.push(ToolCallInfo {
                        tool_name: tool_call.name.clone(),
                        input: tool_call.arguments.clone(),
                        output: Some(serde_json::json!({"error": "This tool is not allowed in read-only automation mode"})),
                        status: ToolCallStatus::Denied,
                    });
                    continue;
                }
            }

            let source = if headless {
                match tool_mode {
                    ToolMode::ReadOnly => crate::agent_control::InvocationSource::ScheduledReadOnly,
                    ToolMode::Full => crate::agent_control::InvocationSource::ScheduledFull,
                }
            } else {
                crate::agent_control::InvocationSource::Assistant
            };

            match crate::agent_control::decide(&definition, source) {
                crate::agent_control::Decision::Deny => {
                    results.push(ToolCallInfo {
                        tool_name: tool_call.name.clone(), input: tool_call.arguments.clone(),
                        output: Some(serde_json::json!({"error": "This tool is denied by the configured agent-control policy"})),
                        status: ToolCallStatus::Denied,
                    });
                    continue;
                }
                crate::agent_control::Decision::Execute => {}
                crate::agent_control::Decision::RequireApproval => {
                    if headless {
                        // In headless mode, check tool_mode
                        match tool_mode {
                            ToolMode::ReadOnly => {
                                // Skip trading tools in read-only mode
                                results.push(ToolCallInfo {
                                tool_name: tool_call.name.clone(),
                                input: tool_call.arguments.clone(),
                                output: Some(serde_json::json!({"error": "Trading tools are not allowed in scheduled task read-only mode"})),
                                status: ToolCallStatus::Denied,
                            });
                                continue;
                            }
                            ToolMode::Full => {
                                // Auto-approve in full mode - execute directly
                            }
                        }
                    } else {
                        // Normal mode - create pending confirmation
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
                }
            }

            // Execute tool directly
            let result = self.execute_single_tool(tool_call, message_id, pool).await;
            results.push(result);
        }

        (results, pending_confirmations)
    }

    /// Execute a single tool
    pub(super) async fn execute_single_tool(
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
        let execution_timeout = Duration::from_secs(30);
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
                    &format!("Failed to serialize tool result: {e}"),
                );
                serde_json::json!({"error": "Failed to serialize result"}).to_string()
            }
        };

        if let Err(e) = database::add_tool_execution(
            pool,
            message_id,
            &tool_call.name,
            &serde_json::to_string(&tool_call.arguments).unwrap_or_else(|_| "{}".to_owned()),
            &output_json,
            status,
        ) {
            logger::warning(
                LogTag::Api,
                &format!("Failed to record tool execution: {e}"),
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
    pub(super) fn format_tool_results(&self, results: &[ToolCallInfo]) -> String {
        let mut output = String::new();

        for result in results {
            output.push_str(&format!("\n**{}**:\n", result.tool_name));

            match &result.status {
                ToolCallStatus::Executed => {
                    if let Some(data) = &result.output {
                        output.push_str(&format!(
                            "Success\n{}\n",
                            serde_json::to_string_pretty(data).unwrap_or_else(|_| data.to_string())
                        ));
                    }
                }
                ToolCallStatus::Failed => {
                    if let Some(data) = &result.output {
                        output.push_str(&format!(
                            "Failed\n{}\n",
                            serde_json::to_string_pretty(data).unwrap_or_else(|_| data.to_string())
                        ));
                    }
                }
                ToolCallStatus::PendingConfirmation => {
                    output.push_str("Pending user confirmation\n");
                }
                ToolCallStatus::Denied => {
                    output.push_str("Denied by user\n");
                }
            }
        }

        output
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::super::engine::{ChatContext, ChatEngine, ToolCallInfo, ToolCallStatus};

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
        assert!(prompt.contains("AVAILABLE TOOLS"));
        assert!(prompt.contains("tool_calls"));

        // With context
        let context = Some(ChatContext {
            current_token: Some("So11111111111111111111111111111111111111112".to_owned()),
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
                tool_name: "get_balance".to_owned(),
                input: serde_json::json!({}),
                output: Some(serde_json::json!({"balance": 10.5})),
                status: ToolCallStatus::Executed,
            },
            ToolCallInfo {
                tool_name: "invalid_tool".to_owned(),
                input: serde_json::json!({}),
                output: Some(serde_json::json!({"error": "Tool not found"})),
                status: ToolCallStatus::Failed,
            },
        ];

        let formatted = engine.format_tool_results(&results);
        assert!(formatted.contains("get_balance"));
        assert!(formatted.contains("invalid_tool"));
        // Outcome is stated in words, never a glyph: this string is fed back to the
        // LLM, and the formatter dropped its emoji when the no-emoji rule landed —
        // the assertions did not, leaving the suite permanently red.
        assert!(formatted.contains("Success"));
        assert!(formatted.contains("Failed"));
    }
}
