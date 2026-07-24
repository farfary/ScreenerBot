<p align="center">
  <img src="../../public/logo.png" alt="ScreenerBot Logo" width="80">
</p>

<h1 align="center">APIs Module</h1>

<p align="center">
  <strong>Centralized External Data & AI Provider Integration</strong>
</p>

<p align="center">
  Unified HTTP client layer managing all external API connections — market data providers, security analysis, token discovery, and LLM-powered intelligence.
</p>

---

## Architecture Overview

<p align="center"><strong>Client Management and Request Orchestration</strong></p>

```text
┌─────────────────────────────────────────────────────────────────────────────┐
│                        ApiManager (Singleton via LazyLock)                   │
│                     get_api_manager() → Arc<ApiManager>                      │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌──────────────────┐  │
│  │ DexScreener │  │GeckoTerminal│  │  Rugcheck   │  │    Jupiter       │  │
│  │  14 methods │  │  16 methods │  │  5 methods  │  │   4 methods      │  │
│  │  300 rpm    │  │  30 rpm     │  │  60 rpm     │  │   No limit       │  │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘  └──────┬───────────┘  │
│         │                │                │                 │              │
│  ┌──────┴────┐     ┌─────┴─────┐    ┌─────┴─────┐    ┌─────┴──────┐      │
│  │ CoinGecko │     │ DefiLlama │    │    LLM    │    │   Stats    │      │
│  │ 1 method  │     │ 2 methods │    │9 providers │    │  Tracker   │      │
│  └───────────┘     └───────────┘    └───────────┘    └────────────┘      │
│                                                                             │
│  ┌──────────────────────────────────────────────────────────────────────┐   │
│  │                     Shared Infrastructure                            │   │
│  │  HttpClient (reqwest) + RateLimiter (semaphore) + ApiStatsTracker   │   │
│  └──────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## File Structure

<p align="center"><strong>Module Layout</strong></p>

```text
src/apis/
├── mod.rs                 Module root — re-exports all clients and types
├── manager.rs             ApiManager singleton (LazyLock global)
├── client.rs              HttpClient + RateLimiter + RateLimitGuard
├── stats.rs               ApiStatsTracker (atomic counters, latency)
├── dexscreener/
│   ├── mod.rs             DexScreener client (11 endpoints, 624 lines)
│   └── types.rs           DexScreenerPool, TokenProfile, etc. (372 lines)
├── geckoterminal/
│   ├── mod.rs             GeckoTerminal client (13 endpoints, 801 lines)
│   └── types.rs           GeckoTerminalPool, OhlcvResponse, etc. (507 lines)
├── rugcheck/
│   ├── mod.rs             Rugcheck client (5 endpoints, 296 lines)
│   └── types.rs           RugcheckInfo, SecurityRisk, etc. (326 lines)
├── jupiter/
│   ├── mod.rs             Jupiter client (4 endpoints, 264 lines)
│   └── types.rs           JupiterToken, JupiterAudit, etc. (125 lines)
├── coingecko/
│   ├── mod.rs             CoinGecko client (1 endpoint, 140 lines)
│   └── types.rs           CoinGeckoCoin (18 lines)
├── defillama/
│   ├── mod.rs             DefiLlama client (2 endpoints, 214 lines)
│   └── types.rs           DefiLlamaProtocol, Price (45 lines)
└── llm/
    ├── mod.rs             LlmManager singleton, LlmClient trait (423 lines)
    ├── types.rs           ChatMessage, ChatRequest, ChatResponse (303 lines)
    ├── openai/            OpenAI provider (gpt-4o-mini)
    ├── anthropic/         Anthropic provider (claude-3-haiku)
    ├── groq/              Groq provider (llama-3.1-8b-instant)
    ├── deepseek/          DeepSeek provider (deepseek-chat)
    ├── gemini/            Gemini provider (gemini-1.5-flash)
    ├── ollama/            Ollama local provider (llama3.2)
    ├── together/          Together AI provider (Meta-Llama-3.1-8B)
    ├── openrouter/        OpenRouter gateway (llama-3.1-8b-instruct:free)
    └── mistral/           Mistral provider (mistral-small-latest)
```

---

## Core Infrastructure

<p align="center"><strong>Shared Components</strong></p>

### ApiManager — Global Singleton

```rust
// Access the global API manager (LazyLock, created on first access)
let manager = get_api_manager();
let pools = manager.dexscreener.fetch_token_pools("mint_address", None).await?;
let stats = manager.get_all_stats().await;
```

| Field | Type | Purpose |
| :--- | :--- | :--- |
| `dexscreener` | `DexScreenerClient` | Market data, pool discovery |
| `geckoterminal` | `GeckoTerminalClient` | Pool data, OHLCV, trades |
| `rugcheck` | `RugcheckClient` | Security analysis |
| `jupiter` | `JupiterClient` | Token discovery, organic scores |
| `coingecko` | `CoinGeckoClient` | Coin list, Solana address extraction |
| `defillama` | `DefiLlamaClient` | Protocol data, token prices |

### HttpClient — Request Layer

- Built on `reqwest::Client` with configurable timeout
- Each API client creates its own `HttpClient` instance

### RateLimiter — Concurrency Control

- Semaphore-based with `min_interval` enforcement between requests
- Returns `RateLimitGuard` (RAII) that releases permit on drop
- Each endpoint can have its own independent rate limiter

### ApiStatsTracker — Per-Client Metrics

Atomic counters tracking per-client performance:

| Metric | Description |
| :--- | :--- |
| `total_requests` | Total API calls made |
| `successful_requests` | Requests returning valid data |
| `failed_requests` | Requests that errored |
| `cache_hits` / `cache_misses` | Cache effectiveness |
| `average_response_time_ms` | Rolling average latency |
| `last_error_message` | Most recent error for debugging |

Error events are sampled (1 in 10) to prevent log flooding.

---

## Market Data Providers

<p align="center"><strong>Real-Time Solana Market Intelligence</strong></p>

### DexScreener

| Property | Value |
| :--- | :--- |
| **Base URL** | `https://api.dexscreener.com` |
| **Default Chain** | `solana` |
| **Timeout** | 10 seconds |
| **Batch Limit** | 30 tokens per request |

**Endpoints (11):**

| Endpoint | Path | Purpose |
| :--- | :--- | :--- |
| Token Pools | `/token-pairs/v1/{chain}/{token}` | All pools for a single token |
| Batch Tokens | `/tokens/v1/{chain}/{addresses}` | Best pair per token (max 30) |
| Pair Lookup | `/latest/dex/pairs/{chain}/{pair}` | Single pair by address |
| Search | `/latest/dex/search?q={query}` | Search pairs by name/symbol |
| Latest Profiles | `/token-profiles/latest/v1` | Newest token listings |
| Latest Boosts | `/token-boosts/latest/v1` | Latest promoted tokens |
| Top Boosts | `/token-boosts/top/v1` | Most promoted tokens |
| Top Tokens | `/tokens/v1/{chain}?sort={field}` | Sorted by volume/liquidity/mcap |
| Token Info | `/token-profiles/{address}` | Social links, description |
| Token Orders | `/orders/v1/{chain}/{token}` | Paid promotions/ads |
| Supported Chains | `/chains/v1` | All supported blockchains |

**Rate Limits (per endpoint):**

| Tier | Limit | Endpoints |
| :--- | :--- | :--- |
| High | 300/min | Token Pools, Batch Tokens, Pair Lookup, Search |
| Standard | 60/min | Profiles, Boosts, Token Info, Token Orders, Chains |

**Key Type — `DexScreenerPool`:** Prices (USD, SOL, native), volumes (5m/1h/6h/24h), transaction counts (buys/sells across 4 timeframes), liquidity (USD/base/quote), FDV, market cap, pair metadata, images, labels.

---

### GeckoTerminal

| Property | Value |
| :--- | :--- |
| **Base URL** | `https://api.geckoterminal.com/api/v2` |
| **Default Network** | `solana` |
| **Timeout** | 10 seconds |
| **Rate Limit** | 30 requests/min |
| **429 Handling** | 5-second backoff |

**Endpoints (13):**

| Endpoint | Path | Purpose |
| :--- | :--- | :--- |
| Token Pools | `/networks/{net}/tokens/{token}/pools` | All pools for token |
| Top Pools (Token) | `/networks/{net}/tokens/{token}/pools` + sort | Sorted pool list |
| Trending Pools | `/networks/{net}/trending_pools` | Trending by network |
| Top Pools (Network) | `/networks/{net}/pools` | Top pools per network |
| Pool Detail | `/networks/{net}/pools/{address}` | Single pool details |
| Multi Pools | `/networks/{net}/pools/multi/{addresses}` | Batch pools (max 30) |
| OHLCV | `/networks/{net}/pools/{pool}/ohlcv/{tf}` | Candlestick data |
| DEX List | `/networks/{net}/dexes` | Supported DEXes |
| New Pools | `/networks/{net}/new_pools` | Newly created pools |
| Multi Tokens | `/networks/{net}/tokens/multi/{addresses}` | Batch token metadata |
| Token Info | `/networks/{net}/tokens/{address}/info` | Token details |
| Recently Updated | `/tokens/info_recently_updated` | Recently active tokens |
| Pool Trades | `/networks/{net}/pools/{pool}/trades` | Recent trades (24h) |

**Key Type — `GeckoTerminalPool`:** 48 fields spanning prices, volumes across 6 timeframes (m5/m15/m30/h1/h6/h24), FDV, market cap, reserves, transaction metrics.

---

### Rugcheck

| Property | Value |
| :--- | :--- |
| **Base URL** | `https://api.rugcheck.xyz/v1` |
| **Timeout** | 15 seconds |
| **Rate Limit** | 60 requests/min (configurable) |

**Endpoints (5):**

| Endpoint | Path | Purpose |
| :--- | :--- | :--- |
| Token Report | `/tokens/{mint}/report` | Full security analysis |
| New Tokens | `/stats/new_tokens` | Newly created tokens |
| Recent Tokens | `/stats/recent` | Most viewed tokens |
| Trending | `/stats/trending` | Trending tokens |
| Verified | `/stats/verified` | Verified tokens |

**Key Type — `RugcheckInfo`:** Security score (0-100 normalized), authorities (mint/freeze/update), holder analysis (top holders, creator balance, insiders detected), transfer fees, LP providers, risk flags with severity levels, rugged status.

---

## Token Discovery Providers

<p align="center"><strong>Multi-Source Solana Token Discovery</strong></p>

### Jupiter

| Property | Value |
| :--- | :--- |
| **Base URL** | `https://lite-api.jup.ag/tokens/v2` |
| **Timeout** | 15 seconds |
| **Rate Limit** | None (public API) |

**Endpoints (4):**

| Endpoint | Path | Purpose |
| :--- | :--- | :--- |
| Recent | `/recent` | Newest tokens |
| Top Organic | `/toporganicscore/{interval}` | Organic score ranking |
| Top Traded | `/toptraded/{interval}` | Most traded tokens |
| Top Trending | `/toptrending/{interval}` | Trending tokens |

Intervals: `5m`, `1h`, `6h`, `24h`. Default limit: 100 tokens.

**Key Type — `JupiterToken`:** Token identity, prices, FDV, market cap, liquidity, holder count, organic score, audit data (authorities, dev balance), multi-timeframe stats (price change, volume, buy/sell counts, trader counts).

---

### CoinGecko

| Property | Value |
| :--- | :--- |
| **Base URL** | `https://api.coingecko.com/api/v3` |
| **Timeout** | 20 seconds |
| **Auth** | Optional `COINGECKO_API_KEY` env var |

**Endpoints (1):**

| Endpoint | Path | Purpose |
| :--- | :--- | :--- |
| Coins List | `/coins/list?include_platform=true` | All coins with platform addresses |

**Static Helpers:**
- `extract_solana_addresses()` — Filter for Solana platform addresses
- `extract_solana_addresses_with_names()` — Addresses + token names

---

### DefiLlama

| Property | Value |
| :--- | :--- |
| **Base URLs** | `https://api.llama.fi`, `https://coins.llama.fi` |
| **Timeout** | 25 seconds |
| **Auth** | None (public API) |

**Endpoints (2):**

| Endpoint | Path | Purpose |
| :--- | :--- | :--- |
| Protocols | `/protocols` | All DeFi protocols |
| Token Price | `/prices/current/solana:{mint}` | SOL token price lookup |

**Static Helpers:**
- `extract_solana_addresses()` — Filter protocols with Solana addresses
- `extract_solana_addresses_with_names()` — Addresses + protocol names

---

## LLM Module

<p align="center"><strong>Multi-Provider AI Intelligence Layer</strong></p>

### Architecture

```text
┌─────────────────────────────────────────────────────────────────┐
│                   LlmManager (OnceCell Singleton)                │
│           init_llm_manager() / get_llm_manager()                 │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────────┐    │
│  │  OpenAI  │  │Anthropic │  │   Groq   │  │  DeepSeek    │    │
│  │ gpt-4o-  │  │ claude-3 │  │ llama-3  │  │ deepseek-    │    │
│  │  mini    │  │ -haiku   │  │ .1-8b    │  │  chat        │    │
│  └──────────┘  └──────────┘  └──────────┘  └──────────────┘    │
│                                                                  │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────────┐    │
│  │  Gemini  │  │  Ollama  │  │ Together │  │ OpenRouter   │    │
│  │  1.5-    │  │ llama3.2 │  │ Meta-    │  │ llama-3.1-   │    │
│  │  flash   │  │ (local)  │  │ Llama    │  │ 8b:free      │    │
│  └──────────┘  └──────────┘  └──────────┘  └──────────────┘    │
│                                                                  │
│  ┌──────────┐                                                   │
│  │ Mistral  │                                                   │
│  │ mistral- │                                                   │
│  │ small    │                                                   │
│  └──────────┘                                                   │
│                                                                  │
│  All implement LlmClient trait:                                  │
│  provider() | is_enabled() | call() | get_stats()               │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### LlmClient Trait

```rust
pub trait LlmClient: Send + Sync {
    fn provider(&self) -> Provider;
    fn is_enabled(&self) -> bool;
    async fn call(&self, request: ChatRequest) -> Result<ChatResponse, LlmError>;
    async fn get_stats(&self) -> ApiStats;
    fn rate_limit_info(&self) -> (usize, Duration);
}
```

### Provider Comparison

| Provider | Default Model | Endpoint | Rate Limit | Auth | Special |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **OpenAI** | `gpt-4o-mini` | `api.openai.com` | 60/min | API Key | JSON mode |
| **Anthropic** | `claude-3-haiku` | `api.anthropic.com` | 60/min | API Key | System prompt separated |
| **Groq** | `llama-3.1-8b-instant` | `api.groq.com` | 30/min | API Key | Free tier, ultra-fast |
| **DeepSeek** | `deepseek-chat` | `api.deepseek.com` | 60/min | API Key | Free 500K tokens/day |
| **Gemini** | `gemini-1.5-flash` | `googleapis.com` | 60/min | API Key (query param) | Content as array |
| **Ollama** | `llama3.2` | `localhost:11434` | None | None | Local only, 120s timeout |
| **Together** | `Meta-Llama-3.1-8B` | `api.together.xyz` | 60/min | API Key | $1 free credit |
| **OpenRouter** | `llama-3.1-8b:free` | `openrouter.ai` | 60/min | API Key | 100+ model gateway |
| **Mistral** | `mistral-small-latest` | `api.mistral.ai` | 60/min | API Key | Code models available |

### Core Types

| Type | Purpose |
| :--- | :--- |
| `ChatMessage` | Message with role (System/User/Assistant) |
| `ChatRequest` | Model + messages + temperature + max_tokens + response_format |
| `ChatResponse` | Content + usage + latency + model + finish_reason |
| `LlmError` | RateLimited, Timeout, AuthError, NetworkError, ProviderDisabled, etc. |
| `Provider` | Enum of 9 providers with string conversion |

---

## Provider Summary Table

<p align="center"><strong>All API Clients at a Glance</strong></p>

| Client | Endpoints | Rate Limit | Timeout | Batch | Primary Use |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **DexScreener** | 11 | 60-300/min | 10s | 30 tokens | Market data, pool discovery |
| **GeckoTerminal** | 13 | 30/min | 10s | 30 pools | Pool data, OHLCV, trades |
| **Rugcheck** | 5 | 60/min | 15s | Single | Security analysis |
| **Jupiter** | 4 | Unlimited | 15s | N/A | Token discovery, organic scores |
| **CoinGecko** | 1 | N/A | 20s | Full list | Solana address extraction |
| **DefiLlama** | 2 | N/A | 25s | N/A | Protocol data, prices |
| **LLM (9)** | 1 each | 30-60/min | 30-120s | N/A | AI analysis, chat |

**Total: 6 market/data providers + 9 LLM providers = 15 external integrations**

---

## Error Handling

<p align="center"><strong>Resilient Request Processing</strong></p>

| Strategy | Applied By | Behavior |
| :--- | :--- | :--- |
| **Rate Limiting** | All clients | Semaphore-based with async `acquire()` guard |
| **Timeout** | All clients | Per-client configurable (10-120 seconds) |
| **429 Backoff** | GeckoTerminal | 5-second sleep on rate limit response |
| **Disabled Guard** | All clients | Returns error immediately if `enabled=false` |
| **Stats Tracking** | All clients | Atomic counters for success/failure/latency |
| **Error Sampling** | Stats system | 1 in 10 errors generate event logs |
| **Cache Fallback** | Market clients | Return cached data when endpoint is unhealthy |

---

## Public API

<p align="center"><strong>Module Exports</strong></p>

```rust
// Global access
pub use get_api_manager() → Arc<ApiManager>

// Client types
pub use DexScreenerClient, GeckoTerminalClient, RugcheckClient
pub use JupiterClient, CoinGeckoClient, DefiLlamaClient

// Response types (via type aliases)
pub use dexscreener_types::{DexScreenerPool, TokenProfile, ...}
pub use geckoterminal_types::{GeckoTerminalPool, OhlcvResponse, ...}
pub use rugcheck_types::{RugcheckInfo, SecurityRisk, ...}
pub use jupiter_types::{JupiterToken, JupiterAudit, ...}

// Infrastructure
pub use HttpClient, RateLimiter, ApiStats, ApiStatsTracker

// LLM
pub use llm::{LlmManager, LlmClient, Provider, ChatMessage, ChatRequest, ChatResponse}
```

---

<p align="center">
  Built for <strong>ScreenerBot</strong> — The ultimate Native Solana trading experience.
</p>
