//! Assistant chat engine internals.
//!
//! Private implementation methods for ChatEngine.
//! Separated from chat_engine.rs for maintainability.

use super::database;
use super::engine::ChatEngine;
use super::types::{
    ChatContext, PendingConfirmation, ToolCall, ToolCallInfo, ToolCallStatus, ToolMode,
};
use crate::agent_control::tools::{ToolDefinition, ToolResult};
use crate::apis::llm::{
    get_llm_manager, ChatMessage as LlmChatMessage, ChatRequest as LlmChatRequest, MessageRole,
    Provider,
};
use crate::assistant::error::{Error, Result};
use crate::logger::{self, LogTag};
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use std::time::Duration;

/// Few-shot tool calls shown in the system prompt: (user request, tool, arguments
/// JSON). Rendered in the protocol the active provider uses.
const TOOL_CALL_EXAMPLES: [(&str, &str, &str); 5] = [
    ("What is my balance?", "get_balance", "{}"),
    (
        "Analyze token 7xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgAsU",
        "analyze_token",
        r#"{"mint_address": "7xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgAsU"}"#,
    ),
    (
        "Use the analyze_token tool to analyze this token: DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263",
        "analyze_token",
        r#"{"mint_address": "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263"}"#,
    ),
    ("Show position 5", "get_position", r#"{"position_id": 5}"#),
    ("Check my open positions", "get_positions", "{}"),
];

/// Full tool reference for the text protocol: names, confirmation gates and
/// parameter schemas the model cannot otherwise see.
fn push_tool_reference(prompt: &mut String, definitions: &[ToolDefinition]) {
    prompt.push_str("## AVAILABLE TOOLS\n\n");
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
}

/// One text-protocol tool call, or `None` when it names no tool or carries
/// arguments that are not a JSON object.
fn tool_call_from_value(call: &serde_json::Value) -> Option<ToolCall> {
    let call = call.get("function").unwrap_or(call);
    let name = call.get("name")?.as_str()?.trim();
    if name.is_empty() {
        return None;
    }
    let arguments = match call.get("arguments").or_else(|| call.get("parameters")) {
        None | Some(serde_json::Value::Null) => serde_json::json!({}),
        Some(serde_json::Value::String(encoded)) if encoded.trim().is_empty() => {
            serde_json::json!({})
        }
        Some(serde_json::Value::String(encoded)) => serde_json::from_str(encoded).ok()?,
        Some(value) => value.clone(),
    };
    arguments.is_object().then(|| ToolCall {
        name: name.to_owned(),
        arguments,
    })
}

impl ChatEngine {
    // =========================================================================
    // PRIVATE METHODS
    // =========================================================================

    /// Build messages for LLM including system prompt and history
    pub(super) fn build_messages(
        &self,
        history: &[database::ChatMessage],
        context: &Option<ChatContext>,
        native_tools: bool,
    ) -> Result<Vec<LlmChatMessage>> {
        let mut messages = Vec::new();

        // Add system prompt
        let system_prompt = self.build_system_prompt(context, native_tools);
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

    /// Whether the model behind this engine receives tool definitions natively.
    /// Decides which tool-call protocol the system prompt teaches.
    pub(super) fn native_tools_supported(&self) -> bool {
        if self.completion.is_some() {
            return true;
        }
        let provider_name = crate::config::with_config(|cfg| cfg.llm.default_provider.clone());
        Provider::from_str(&provider_name)
            .and_then(|provider| get_llm_manager().get_client(provider))
            .is_some_and(|client| client.supports_native_tools())
    }

    /// Build the system prompt. With `native_tools` the model is told to call tools
    /// only through function calling, because their definitions travel in the
    /// request; otherwise it is taught the JSON text protocol `parse_tool_calls` reads.
    pub(super) fn build_system_prompt(
        &self,
        context: &Option<ChatContext>,
        native_tools: bool,
    ) -> String {
        let mut prompt = String::with_capacity(8192);
        prompt.push_str(
            "You are the ScreenerBot Assistant for a Solana trading bot. \
             You help users analyze tokens, manage positions, and configure the bot.\n\n",
        );
        prompt.push_str(&super::reply_language::reply_language_line());

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

        if native_tools {
            prompt.push_str("Call tools only through the function-calling interface. Never write a tool call, a tool name or JSON in your reply text. Do not narrate your plan or expose private reasoning.\n\n");
        } else {
            prompt.push_str("Call a tool by replying with only this JSON block and nothing else. Do not narrate your plan or expose private reasoning.\n\n");
            prompt.push_str("Format:\n");
            prompt.push_str("```json\n{\"tool_calls\": [{\"name\": \"tool_name\", \"arguments\": {\"param1\": \"value1\", \"param2\": 123}}]}\n```\n\n");
        }

        prompt.push_str("### Examples (FOLLOW THESE EXACTLY):\n\n");
        for (request, tool, arguments) in TOOL_CALL_EXAMPLES {
            prompt.push_str(&format!("**User:** \"{request}\"\n"));
            if native_tools {
                prompt.push_str(&format!(
                    "**Assistant:** function call `{tool}` with arguments `{arguments}`\n\n"
                ));
            } else {
                prompt.push_str(&format!(
                    "**Assistant:**\n```json\n{{\"tool_calls\": [{{\"name\": \"{tool}\", \"arguments\": {arguments}}}]}}\n```\n\n"
                ));
            }
        }

        prompt.push_str("**User:** \"How does the bot work?\"\n");
        prompt.push_str("**Assistant:** ScreenerBot is a Solana trading bot that monitors tokens and executes trades based on your configured strategies. It can automatically buy and sell tokens based on market conditions.\n\n");

        prompt.push_str("**User:** \"Hello!\"\n");
        prompt.push_str("**Assistant:** Hello! I'm your ScreenerBot assistant. I can help you analyze tokens, check positions, manage trades, and configure settings. What would you like to do?\n\n");

        let definitions = self.tool_registry.list_definitions();
        if native_tools {
            // Names, descriptions and schemas already travel in the request.
            let confirmed: Vec<String> = definitions
                .iter()
                .filter(|def| def.requires_confirmation)
                .map(|def| format!("`{}`", def.name))
                .collect();
            if !confirmed.is_empty() {
                prompt.push_str(&format!(
                    "## Tools that require user confirmation\n{}\n\n",
                    confirmed.join(", ")
                ));
            }
        } else {
            push_tool_reference(&mut prompt, &definitions);
        }

        prompt.push_str("\n## Rules\n");
        prompt.push_str("1. DEFAULT ACTION: When in doubt, CALL A TOOL. Tool calling is preferred over natural responses.\n");
        if native_tools {
            prompt.push_str("2. NEVER write tool calls as text - use function calling only\n");
        } else {
            prompt.push_str(
                "2. NEVER add explanatory text with tool calls - ONLY output the JSON code block\n",
            );
        }
        prompt.push_str(
            "3. NEVER refuse a tool call - if user mentions ANY data or action, call the tool\n",
        );
        prompt.push_str("4. ALWAYS extract token addresses, position IDs, and other parameters from user messages\n");
        prompt.push_str("5. For confirmation-required tools: Call them anyway - the system handles confirmations\n");
        if native_tools {
            prompt.push_str("6. Multiple tools: Make several function calls in one reply\n");
        } else {
            prompt.push_str(
                "6. Multiple tools: Add multiple objects to tool_calls array in a single JSON block\n",
            );
        }
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
        let provider_name = crate::config::with_config(|cfg| cfg.llm.default_provider.clone());
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

    /// Parse text-protocol tool calls from a model reply.
    ///
    /// Every `{"tool_calls": [...]}` object in the reply is read with a streaming JSON
    /// parser, so code fences, surrounding prose and nested argument objects do not
    /// matter. Each call accepts the arguments under `arguments` or `parameters`, as an
    /// object or a JSON-encoded string, optionally wrapped in an OpenAI-style
    /// `function` object.
    pub(super) fn parse_tool_calls(&self, response: &str) -> Vec<ToolCall> {
        let mut tool_calls = Vec::new();

        for (key_offset, _) in response.match_indices("\"tool_calls\"") {
            let Some(start) = response[..key_offset].rfind('{') else {
                continue;
            };
            let Some(Ok(envelope)) = serde_json::Deserializer::from_str(&response[start..])
                .into_iter::<serde_json::Value>()
                .next()
            else {
                continue;
            };
            let Some(calls) = envelope.get("tool_calls").and_then(|v| v.as_array()) else {
                continue;
            };
            tool_calls.extend(calls.iter().filter_map(tool_call_from_value));
        }

        if tool_calls.is_empty() {
            logger::debug(LogTag::Api, "No tool calls found in response");
        } else {
            logger::debug(
                LogTag::Api,
                &format!("Parsed {} text tool calls", tool_calls.len()),
            );
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
        let Some(_active_tool) = crate::global::begin_tool() else {
            return ToolCallInfo {
                tool_name: tool_call.name.clone(),
                input: tool_call.arguments.clone(),
                output: Some(serde_json::json!({
                    "error": "An application update is restarting the tool runtime."
                })),
                status: ToolCallStatus::Failed,
            };
        };
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

        let prompt = engine.build_system_prompt(&None, false);
        assert!(prompt.contains("ScreenerBot"));
        assert!(prompt.contains("AVAILABLE TOOLS"));
        assert!(prompt.contains("tool_calls"));
        assert!(prompt.contains("the user's interface language"));

        // With context
        let context = Some(ChatContext {
            current_token: Some("So11111111111111111111111111111111111111112".to_owned()),
            current_position: Some(42),
        });

        let prompt_with_context = engine.build_system_prompt(&context, false);
        assert!(prompt_with_context.contains("So11111111111111111111111111111111111111112"));
        assert!(prompt_with_context.contains("42"));
    }

    #[test]
    fn native_prompt_never_teaches_the_text_protocol() {
        let engine = ChatEngine::new();

        let prompt = engine.build_system_prompt(&None, true);
        assert!(!prompt.contains("tool_calls"));
        assert!(!prompt.contains("```json"));
        assert!(!prompt.contains("AVAILABLE TOOLS"));
        assert!(prompt.contains("function-calling interface"));
        assert!(prompt.contains("function call `get_positions`"));
    }

    #[test]
    fn test_parse_parameters_key_without_code_block() {
        let engine = ChatEngine::new();

        let response = r#"{"tool_calls": [{"name": "get_positions", "parameters": {}}]}"#;

        let calls = engine.parse_tool_calls(response);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "get_positions");
        assert_eq!(calls[0].arguments, serde_json::json!({}));
    }

    #[test]
    fn test_parse_function_wrapper_with_encoded_arguments() {
        let engine = ChatEngine::new();

        let response = r#"Checking now.
{"tool_calls": [{"type": "function", "function": {"name": "get_position", "arguments": "{\"position_id\": 5}"}}]} done"#;

        let calls = engine.parse_tool_calls(response);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "get_position");
        assert_eq!(calls[0].arguments, serde_json::json!({"position_id": 5}));
    }

    #[test]
    fn test_parse_nested_arguments_in_prose() {
        let engine = ChatEngine::new();

        let response = r#"Sure: {"tool_calls": [{"name": "update_config", "arguments": {"values": [1, 2], "nested": {"a": [3]}}}]} and that is all."#;

        let calls = engine.parse_tool_calls(response);
        assert_eq!(calls.len(), 1);
        assert_eq!(
            calls[0].arguments,
            serde_json::json!({"values": [1, 2], "nested": {"a": [3]}})
        );
    }

    #[test]
    fn test_parse_rejects_calls_without_a_usable_name_or_object_arguments() {
        let engine = ChatEngine::new();

        let response = r#"{"tool_calls": [{"tool": "get_balance"}, {"name": "get_position", "arguments": "not json"}, {"name": "get_balance", "arguments": [1]}]}"#;

        assert!(engine.parse_tool_calls(response).is_empty());
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
