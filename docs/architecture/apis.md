# APIs Module — Architecture

> ScreenerBot external HTTP integrations (market data, security analysis, token discovery, LLM providers) — February 2026

---

## Table of Contents

1. [Overview](#1-overview)
2. [File Structure](#2-file-structure)
3. [Core Primitives](#3-core-primitives)
4. [ApiManager (Global Singleton)](#4-apimanager-global-singleton)
5. [Market / Discovery Clients](#5-market--discovery-clients)
6. [Security Clients (Rugcheck)](#6-security-clients-rugcheck)
7. [SOL Price Background Service](#7-sol-price-background-service)
8. [LLM Providers Submodule](#8-llm-providers-submodule)
9. [Error Handling & Observability](#9-error-handling--observability)
10. [Module Connections](#10-module-connections)

---

## 1. Overview

The `apis` module is ScreenerBot's integration layer for **external HTTP APIs** (anything that is not Solana RPC):

* **Market data / pool discovery**: DexScreener, GeckoTerminal
* **SOL/USD price**: DexScreener → GeckoTerminal → Jupiter (cascade, see §7)
* **Security**: Rugcheck
* **Token discovery / trends**: Jupiter
* **Reference datasets**: CoinGecko, DefiLlama
* **AI providers**: multiple LLM backends (OpenAI, Anthropic, Groq, etc.)

### Design goals

* **One global instance** per API client (true global rate limiting and consistent stats).
* **Centralized rate limiting** implemented in-process (per client or per endpoint).
* **Per-client stats** (requests/success/failure/latency + last error) exposed to the dashboard.
* **Strict enable/disable gating** via config so discovery features can be turned off cleanly.

### Non-goals (by design)

* **No shared response cache inside `apis/`**.
  * Caching is done by caller modules (e.g. `tokens`, `filtering`, `pools`) using their own moka caches and DBs.

---

## 2. File Structure

```text
src/apis/
├── mod.rs                 Module root (re-exports, public entrypoints)
├── manager.rs             ApiManager singleton (LazyLock<Arc<ApiManager>>)
├── client.rs              HttpClient + RateLimiter primitives
├── stats.rs               ApiStats + ApiStatsTracker (atomic counters + timestamps)
├── sol_price.rs           SOL/USD price background task + global cache
├── dexscreener/
│   ├── mod.rs             DexScreener client (multi-endpoint rate limiting)
│   └── types.rs           Serde response types + conversions
├── geckoterminal/
│   ├── mod.rs             GeckoTerminal client (per-client limiter)
│   └── types.rs           Serde response types + conversions
├── rugcheck/
│   ├── mod.rs             Rugcheck client (returns ApiError)
│   └── types.rs           Serde response types + flexible deserializers
├── jupiter/
│   ├── mod.rs             Jupiter discovery client
│   └── types.rs           Serde types (JupiterToken, etc.)
├── coingecko/
│   ├── mod.rs             Coin list (platform addresses) client
│   └── types.rs
├── defillama/
│   ├── mod.rs             Protocols + prices/current client
│   └── types.rs
└── llm/
    ├── mod.rs             Provider enum + LlmClient trait + LlmManager singleton (OnceCell)
    ├── types.rs           Provider-agnostic request/response DTOs
    └── <provider>/...      Per-provider modules (openai/, anthropic/, groq/, ...)
```

---

## 3. Core Primitives

### 3.1 RateLimiter (requests per minute)

**File:** `src/apis/client.rs`

`RateLimiter` enforces **single-flight** external requests and a **minimum interval** derived from `max_per_minute`:

```rust
pub struct RateLimiter {
    semaphore: Arc<Semaphore>,               // 1 concurrent request
    last_request: Arc<Mutex<Option<Instant>>>,
    min_interval: Duration,                  // 60s / max_per_minute
    max_per_minute: usize,
}
```

Usage pattern:

```rust
let guard = limiter.acquire().await?;
let resp = reqwest_builder.send().await;
drop(guard); // releases permit
```

Notes:
* This is intentionally conservative (1 concurrent request) to avoid burst bans.
* Some clients use **one limiter per client**, others (DexScreener) use **one limiter per endpoint**.

### 3.2 HttpClient (reqwest wrapper)

**File:** `src/apis/client.rs`

`HttpClient` wraps a `reqwest::Client` configured with a timeout:

```rust
pub struct HttpClient {
    client: Client,
    timeout: Duration,
}
```

Clients either:
* Use `HttpClient` directly (`RugcheckClient`, `JupiterClient`, `CoinGeckoClient`, `DefiLlamaClient`), or
* Use `reqwest::Client` directly and apply per-request timeout (`DexScreenerClient`, `GeckoTerminalClient`).

### 3.3 ApiStatsTracker (per-client metrics)

**File:** `src/apis/stats.rs`

Each API client owns `Arc<ApiStatsTracker>` and records:

* total/success/failed requests (atomic)
* cache hit/miss counters (atomic; mostly used by callers, but network errors often record a miss)
* last request/success timestamps
* last error timestamp + message
* rolling average latency (stored behind `RwLock<f64>`)

```rust
pub struct ApiStatsTracker {
    total_requests: AtomicU64,
    successful_requests: AtomicU64,
    failed_requests: AtomicU64,
    cache_hits: AtomicU64,
    cache_misses: AtomicU64,
    last_request_time: Arc<RwLock<Option<DateTime<Utc>>>>,
    last_success_time: Arc<RwLock<Option<DateTime<Utc>>>>,
    last_error: Arc<RwLock<Option<(DateTime<Utc>, String)>>>,
    avg_response_time: Arc<RwLock<f64>>,
}
```

Important behavior:
* `record_error_with_event()` samples events (every ~10th failure) to avoid spamming the events DB.
* Public/dashboard-facing stats are returned as `ApiStats { last_error_time, last_error_message, ... }`.

---

## 4. ApiManager (Global Singleton)

**File:** `src/apis/manager.rs`

`ApiManager` is the global aggregator for the non-LLM API clients:

```rust
pub struct ApiManager {
    pub dexscreener: DexScreenerClient,
    pub geckoterminal: GeckoTerminalClient,
    pub rugcheck: RugcheckClient,
    pub jupiter: JupiterClient,
    pub coingecko: CoinGeckoClient,
    pub defillama: DefiLlamaClient,
}
```

### 4.1 Singleton access

```rust
static GLOBAL_API_MANAGER: LazyLock<Arc<ApiManager>> =
    LazyLock::new(|| Arc::new(ApiManager::new()));

pub fn get_api_manager() -> Arc<ApiManager> {
    GLOBAL_API_MANAGER.clone()
}
```

### 4.2 Enable/disable gating (config-driven)

`ApiManager::new()` reads config (`get_config_clone()`) and computes `enabled` flags based on:

* global discovery toggle: `cfg.tokens.discovery.enabled`
* per-source toggles: `cfg.tokens.sources.<api>.enabled`
* per-discovery toggles: `cfg.tokens.discovery.<api>.enabled`

Example (DexScreener / GeckoTerminal):

```rust
let discovery_enabled = cfg.tokens.discovery.enabled;
let dexscreener_enabled =
    cfg.tokens.sources.dexscreener.enabled && discovery_enabled && cfg.tokens.discovery.dexscreener.enabled;
```

### 4.3 Initialization resilience

Each client construction is guarded:

* Try to create enabled client (with configured timeout/rate).
* If construction fails, log a warning and create a **disabled** client with default constants.

This ensures the bot can boot even if a particular client fails to initialize.

### 4.4 Aggregated stats

`ApiManager::get_all_stats()` calls `get_stats()` on every client and returns `ApiManagerStats` (serde-serializable).

---

## 5. Market / Discovery Clients

### 5.1 DexScreener

**Files:** `src/apis/dexscreener/mod.rs`, `src/apis/dexscreener/types.rs`  
**Base URL:** `https://api.dexscreener.com`  
**Default chain:** `"solana"`

Key traits:
* Uses **multiple** `RateLimiter`s: one per endpoint category (`limiter_token_pools`, `limiter_search`, ...).
* Uses a shared `get_json()` helper that:
  * enforces enabled flag,
  * acquires limiter guard,
  * sends request with timeout,
  * records stats,
  * records sampled error events,
  * performs basic 429 backoff (`sleep(5s)`).

Primary high-traffic methods:
* `fetch_token_pools(token_address, chain_id)` — **ALL** pools for one token (`token-pairs/v1/...`)
* `fetch_token_batch(addresses, chain_id)` — **best pair per token** for up to 30 tokens (`tokens/v1/...`)

Other implemented endpoints (see module header in code for the authoritative list):
* pair lookup, search, token profiles/boosts, orders, token info, supported chains

### 5.2 GeckoTerminal

**Files:** `src/apis/geckoterminal/mod.rs`, `src/apis/geckoterminal/types.rs`  
**Base URL:** `https://api.geckoterminal.com/api/v2`  
**Default network:** `"solana"`

Key traits:
* One `RateLimiter` per client instance (default 30/min).
* `get_json()` helper records stats and performs simple 429 backoff (`sleep(10s)`).
* Implements pool discovery + OHLCV/trades endpoints (see module header in code).

### 5.3 Jupiter (discovery/trends)

**Files:** `src/apis/jupiter/mod.rs`, `src/apis/jupiter/types.rs`  
**Base URL:** `https://lite-api.jup.ag/tokens/v2`

Key traits:
* Uses `HttpClient`.
* Returns `Result<T, crate::tokens::types::ApiError>`.
* Records `record_cache_miss()` on request transport failures to reflect “no usable data”.

Implemented endpoints:
* `/recent`
* `/toporganicscore/{interval}`
* `/toptraded/{interval}`
* `/toptrending/{interval}`

#### Jupiter shared rate-budget throttle

**File:** `src/apis/jupiter/throttle.rs`

Jupiter's free `lite-api.jup.ag` rate-limits **per-IP across all endpoints**, so
background callers (SOL price fallback, this discovery client's 4 list fetches, the
health monitor, the multi-wallet tool) can starve swap quote/swap — the revenue path.
The throttle gives swaps priority:

* Swaps hold `throttle::swap_guard()` for the whole quote/swap (see swaps.md §9.1).
* Background Jupiter callers call `throttle::acquire_background().await` first: they
  defer while any swap is in flight (bounded) and are spaced by a minimum interval.
* The 4 discovery fetches each call `acquire_background()` before their request.

### 5.4 CoinGecko (reference dataset)

**Files:** `src/apis/coingecko/mod.rs`, `src/apis/coingecko/types.rs`  
**Endpoint:** `/coins/list?include_platform=true`

Key traits:
* Uses a demo-tier API key from `COINGECKO_API_KEY` env var via header `x-cg-demo-api-key`.
* Provides helpers to extract Solana addresses from the returned “platforms” map:
  * `extract_solana_addresses()`
  * `extract_solana_addresses_with_names()`

### 5.5 DefiLlama (reference dataset)

**Files:** `src/apis/defillama/mod.rs`, `src/apis/defillama/types.rs`  
**Endpoints:** `/protocols` and `/prices/current/solana:{mint}`

Provides helpers to:
* fetch raw data (`fetch_protocols`, `fetch_token_price`),
* extract candidate Solana addresses from protocol entries.

---

## 6. Security Clients (Rugcheck)

### 6.1 Rugcheck

**Files:** `src/apis/rugcheck/mod.rs`, `src/apis/rugcheck/types.rs`  
**Base URL:** `https://api.rugcheck.xyz`

Key traits:
* Uses `HttpClient` + `RateLimiter`.
* Returns domain-layer `ApiError` (same type used by token discovery pipeline).
* Explicitly handles “token not analyzed yet” cases:
  * `404 Not Found` => `ApiError::NotFound`
  * `400 Bad Request` with body containing `"not found"` => `ApiError::NotFound`
* Contains a defensive extraction strategy for authority fields (Token2022 response shape differences):
  * top-level authority fields may be objects; fallback to nested `token.*` fields.

---

## 7. SOL Price Background Service

**File:** `src/apis/sol_price.rs`

This is a special-case module living under `apis/` but acting as a **background service** that maintains a global SOL/USD cache.

**Source cascade (priority order) — `fetch_sol_price()`:**

1. **DexScreener** (primary) — `https://api.dexscreener.com/latest/dex/tokens/{WSOL}`;
   uses the `priceUsd` of the highest-liquidity pair whose BASE token is WSOL.
2. **GeckoTerminal** (secondary) —
   `https://api.geckoterminal.com/api/v2/simple/networks/solana/token_price/{WSOL}`.
3. **Jupiter** (last-resort) — `https://lite-api.jup.ag/price/v3?ids={WSOL}`.

Each source is tried in order; on failure the next is used, and the active source is
stored in `SolPriceData.source`. Keeping SOL price OFF Jupiter by default is
deliberate: Jupiter's free lite-api shares one per-IP rate budget that must be
reserved for swap quote/swap. The Jupiter fallback still routes through the
`apis::jupiter::throttle` (see §5.3) so it yields to in-flight swaps.

* Refresh interval: `PRICE_REFRESH_INTERVAL_SECS = 30`
* Cache expiry: `CACHE_EXPIRY_SECS = 300`
* Guardrails: rejects unrealistic changes (`MAX_PRICE_CHANGE_PERCENT = 50.0`)

Global state:

```rust
static SOL_PRICE_CACHE: LazyLock<Arc<std::sync::RwLock<SolPriceData>>> = ...;
static SERVICE_RUNNING: LazyLock<Arc<AtomicBool>> = ...;
```

Service lifecycle entrypoints:
* `start_sol_price_service(shutdown, monitor) -> JoinHandle`
* `stop_sol_price_service()`

Read APIs:
* `get_sol_price() -> f64` (returns 0.0 if stale/invalid)
* `get_sol_price_info() -> Option<SolPriceData>`

---

## 8. LLM Providers Submodule

**Directory:** `src/apis/llm/`

The LLM system is intentionally **not** part of `ApiManager`; it has its own singleton:

* `LlmManager` stored in `static LLM_MANAGER: OnceCell<Arc<LlmManager>>`
* Must be initialized once at startup via `init_llm_manager(manager)`
* Access via `get_llm_manager()` (panics if not initialized) or `try_get_llm_manager()`

### 8.1 Provider enum

**File:** `src/apis/llm/mod.rs`

```rust
pub enum Provider {
    OpenAi, Anthropic, Groq, DeepSeek, Gemini,
    Ollama, Together, OpenRouter, Mistral, Assistant,
}
```

### 8.2 Provider-agnostic DTOs

**File:** `src/apis/llm/types.rs`

* `ChatRequest { model, messages, temperature, max_tokens, response_format }`
* `ChatResponse { content, usage, finish_reason, model, latency_ms }`

### 8.3 LlmClient trait

All provider implementations conform to:

```rust
#[async_trait]
pub trait LlmClient {
    fn provider(&self) -> Provider;
    fn is_enabled(&self) -> bool;
    async fn call(&self, request: ChatRequest) -> Result<ChatResponse, LlmError>;
    async fn get_stats(&self) -> ApiStats;
    fn rate_limit_info(&self) -> (usize, Duration);
}
```

Providers use the same shared primitives (`RateLimiter`, `ApiStatsTracker`) and raw reqwest HTTP.

---

## 9. Error Handling & Observability

### 9.1 Error types are not fully unified

Today there are two patterns:

* `Result<T, String>` (DexScreener, GeckoTerminal) — internal helpers build rich strings + record events.
* `Result<T, ApiError>` (Rugcheck, Jupiter, CoinGecko, DefiLlama) — returns a shared domain error type.

This is architectural debt (useful to know when adding new API clients).

### 9.2 Event sampling

`ApiStatsTracker::record_error_with_event()` logs an API error event only on a sampled cadence (based on `failed_requests`) to prevent high-frequency APIs from flooding the events DB.

---

## 10. Module Connections

```text
tokens/      -> api_manager().{dexscreener,geckoterminal,jupiter,...}  (enrichment + discovery)
filtering/   -> api_manager().rugcheck + market lookups               (risk assessment)
pools/       -> api_manager().dexscreener/geckoterminal               (pool discovery + refresh)
ohlcvs/      -> api_manager().geckoterminal                           (candles)
webserver/   -> api_manager().get_all_stats()                         (dashboard API stats)
ai/          -> llm::{init_llm_manager,get_llm_manager}               (LLM calls)
services/    -> sol_price::{start_sol_price_service,...}              (background price cache)
```
