# DeepSeek API Client

OpenAI-compatible LLM provider with very generous free tier and cheap pricing.

## Features

- **FREE TIER**: ~500K tokens/day
- **Cheap Pricing**: Very affordable paid rates
- **OpenAI Compatible**: Uses same API format as OpenAI
- **JSON Mode**: Supports structured output via `response_format`
- **Rate Limiting**: Built-in 60 requests/minute limit
- **Stats Tracking**: Request/error metrics via ApiStatsTracker

## Models

- `deepseek-chat` - General purpose chat model (default)
- `deepseek-reasoner` - Advanced reasoning capabilities

## Setup

1. Get API key from: https://platform.deepseek.com/api-keys

2. Create client:

```rust
use screenerbot::apis::llm::deepseek::DeepSeekClient;

let client = DeepSeekClient::new(
    api_key.to_string(),
    Some("deepseek-chat".to_string()), // Optional model override
    true, // enabled
)?;
```

3. Use via LlmClient trait:

```rust
use screenerbot::apis::llm::{ChatMessage, ChatRequest};

let request = ChatRequest::new(
    "deepseek-chat",
    vec![
        ChatMessage::system("You are a helpful assistant"),
        ChatMessage::user("Hello!"),
    ],
)
.with_temperature(0.7)
.with_max_tokens(1000);

let response = client.call(request).await?;
println!("Response: {}", response.content);
```

## API Configuration

- **Base URL**: `https://api.deepseek.com`
- **Endpoint**: `/chat/completions`
- **Auth**: Bearer token in `Authorization` header
- **Timeout**: 30 seconds
- **Rate Limit**: 60 requests/minute (default)

## Type Compatibility

Since DeepSeek uses OpenAI-compatible format, we re-export OpenAI types with DeepSeek aliases:

- `DeepSeekRequest` = `OpenAiRequest`
- `DeepSeekResponse` = `OpenAiResponse`
- `DeepSeekMessage` = `OpenAiMessage`
- etc.

## Testing

```bash
# Run unit tests
cargo test --lib apis::llm::deepseek

# Check compilation
cargo check --lib
```

## Documentation

- Official API Docs: https://api-docs.deepseek.com/
- Platform: https://platform.deepseek.com/
