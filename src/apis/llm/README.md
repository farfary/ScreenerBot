# LLM Provider Module

Unified interface for multiple LLM providers with raw HTTP, rate limiting, and stats tracking.

## Files Created

- `mod.rs` (424 lines) - Core module with trait, manager, and provider enum
- `types.rs` (303 lines) - Request/response types and error types

## Architecture

### Core Types (types.rs)

**Message Types:**

- `ChatMessage` - Role + content with builder methods
- `MessageRole` - System, User, Assistant
- `ChatRequest` - Model, messages, temperature, max_tokens, response_format
- `ChatResponse` - Content, usage, finish_reason, model, latency_ms
- `Usage` - Token counting (prompt, completion, total)
- `ResponseFormat` - JSON mode support

**Error Types:**

- `LlmError` - Comprehensive error enum covering:
  - RateLimited, Timeout, InvalidResponse
  - AuthError, NetworkError, ParseError
  - ApiError, ProviderDisabled

### Provider Interface (mod.rs)

**Provider Enum:**

```rust
pub enum Provider {
    OpenAi, Anthropic, Groq, DeepSeek, Gemini,
    Ollama, Together, OpenRouter, Mistral, Fireworks,
}
```

**LlmClient Trait:**

```rust
#[async_trait]
pub trait LlmClient: Send + Sync {
    fn provider(&self) -> Provider;
    fn is_enabled(&self) -> bool;
    async fn call(&self, request: ChatRequest) -> Result<ChatResponse, LlmError>;
    async fn get_stats(&self) -> ApiStats;
    fn rate_limit_info(&self) -> (usize, Duration);
}
```

**LlmManager:**

- Holds all provider clients as `Option<Arc<dyn LlmClient>>`
- `get_client(provider)` - Get specific provider
- `enabled_providers()` - List active providers
- `call(provider, request)` - Make request via specific provider
- Setter methods for each provider

**Global Singleton:**

```rust
init_llm_manager(manager) // Initialize once at startup
get_llm_manager()         // Get global instance
```

## Usage Patterns

### Basic Request

```rust
let request = ChatRequest::new(
    "gpt-4",
    vec![ChatMessage::user("Hello!")],
);
let response = manager.call(Provider::OpenAi, request).await?;
```

### With Options

```rust
let request = ChatRequest::new(model, messages)
    .with_temperature(0.7)
    .with_max_tokens(1000)
    .with_json_mode();
```

### Convenience Helpers

```rust
let req = simple_request("gpt-4", "Hello!");
let req = system_user_request("gpt-4", "You are...", "User message");
```

## Integration Points

- **Rate Limiting:** Uses `crate::apis::client::RateLimiter`
- **Stats Tracking:** Uses `crate::apis::stats::ApiStatsTracker`
- **HTTP Client:** Raw `reqwest::Client` (no external LLM libraries)

## Next Steps

Individual provider implementations will go in subdirectories:

- `src/apis/llm/openai/` - OpenAI client
- `src/apis/llm/anthropic/` - Anthropic client
- `src/apis/llm/groq/` - Groq client
- etc.

Each provider will implement the `LlmClient` trait and handle its specific API format.
