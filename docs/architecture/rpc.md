# RPC Module — Architecture

> ScreenerBot multi-provider Solana JSON-RPC client with failover, rate limiting, retries, and SQLite stats — February 2026

---

## Table of Contents

1. [Overview](#1-overview)
2. [File Structure](#2-file-structure)
3. [Global Access Layer (`RpcClient`)](#3-global-access-layer-rpcclient)
4. [Provider Model + Configuration](#4-provider-model--configuration)
5. [`RpcManager` Request Execution Pipeline](#5-rpcmanager-request-execution-pipeline)
6. [Provider Selection Strategies](#6-provider-selection-strategies)
7. [Rate Limiting (Governor / GCRA)](#7-rate-limiting-governor--gcra)
8. [Circuit Breaker](#8-circuit-breaker)
9. [Error Model + Retry Policy](#9-error-model--retry-policy)
10. [Typed RPC Client Methods (`RpcClientMethods`)](#10-typed-rpc-client-methods-rpcclientmethods)
11. [Statistics System (`rpc_stats.db`)](#11-statistics-system-rpc_statsdb)
12. [WebSocket Utilities](#12-websocket-utilities)
13. [Module Connections](#13-module-connections)

---

## 1. Overview

The `rpc` module provides ScreenerBot's Solana chain access layer.

At a high level, it offers:

- **Multiple RPC endpoints** (from config) with per-call provider selection
- **Rate limiting per provider** using Governor (GCRA algorithm) + method cost weighting
- **Circuit breaker** failover (Closed -> Open -> HalfOpen)
- **Retries with exponential backoff** and provider failover
- **SQLite-backed call statistics** (`rpc_stats.db`) with background buffering
- A typed, Solana-friendly API (`RpcClientMethods`) used by most modules

### Design goals

- Keep chain access reliable even when a provider is unstable or rate limiting.
- Centralize rate limiting + failover logic so callers do not implement ad-hoc retry loops.
- Keep the rest of the codebase using a single entrypoint: `rpc::get_rpc_client()`.

### Non-goals (current implementation)

- No long-lived WebSocket client here; `rpc/websocket.rs` only provides URL/payload helpers.

---

## 2. File Structure

```text
src/rpc/
├── mod.rs                      Module root + many re-exports
├── global.rs                   Global `RpcClient` singleton (OnceLock<RpcClient>)
├── manager.rs                  `RpcManager` (orchestrator; retries, selection, stats, HTTP)
├── client/
│   ├── mod.rs                  `RpcClient` wrapper around `Arc<RpcManager>`
│   └── methods.rs              `RpcClientMethods` trait + implementation (typed Solana RPC API)
├── provider/
│   ├── mod.rs                  Provider module exports
│   ├── config.rs               `ProviderConfig` + helpers
│   └── detection.rs            ProviderKind detection + provider_id + websocket URL derivation
├── rate_limiter/
│   ├── mod.rs                  `RateLimiterManager` (per-provider limiter registry)
│   ├── provider.rs             `ProviderRateLimiter` (Governor / GCRA + 429 tracking)
│   └── adaptive.rs             Shared helpers (ExponentialBackoff, SlidingWindowTracker)
├── circuit_breaker/
│   ├── mod.rs                  `CircuitBreakerManager`
│   ├── config.rs               `CircuitBreakerConfig`
│   └── state.rs                `ProviderCircuitBreaker` state machine
├── selector.rs                 Strategy implementations (RoundRobin/Priority/Latency/Adaptive)
├── stats/
│   ├── mod.rs                  `StatsManager` orchestration + re-exports
│   ├── collector.rs            Buffered background writer (channel + flush loop)
│   ├── database.rs             `RpcStatsDatabase` (rpc_stats.db schema + queries)
│   ├── helpers.rs              Helper snapshots + monitoring/cleanup task
│   └── types.rs                Stats types (RpcCallRecord, ProviderStats, etc.)
├── websocket.rs                WS URL + subscription payload helpers
├── errors.rs                   `RpcError` enum + retryability classification
├── types.rs                    Core RPC types (ProviderKind, RpcMethod, ProviderState, ...)
├── utils.rs                    Misc helpers (pubkey parsing, ATA rent, SOL/lamports, ...)
├── testing.rs                  RPC endpoint tests (validate mainnet, version, etc.)
└── client/                     Typed RPC API wrappers (Solana SDK integration)
```

---

## 3. Global Access Layer (`RpcClient`)

Most of the codebase uses the sync function:

- `rpc::get_rpc_client() -> &'static RpcClient`

### 3.1 `RpcClient` global singleton

**File:** `src/rpc/global.rs`

```rust
static RPC_CLIENT: OnceLock<RpcClient> = OnceLock::new();
```

`get_rpc_client()`:

- returns an already-initialized client if present
- otherwise attempts `init_rpc_client()`
- panics if initialization fails (this is intentional: RPC is a hard dependency)

### 3.2 Initialization bridge (sync -> async)

`init_rpc_client()` uses:

```rust
tokio::task::block_in_place(|| {
  tokio::runtime::Handle::current().block_on(init_rpc_manager())
})
```

This allows the rest of the codebase to call `get_rpc_client()` from both sync and async contexts
while still building the internal async `RpcManager`.

### 3.3 `RpcManager` global singleton

**File:** `src/rpc/manager.rs`

```rust
static RPC_MANAGER: OnceCell<Arc<RpcManager>> = OnceCell::const_new();
```

`init_rpc_manager()`:

- constructs `RpcManager::new().await?`
- starts background services (`StatsManager::start()`)
- returns `Arc<RpcManager>`

---

## 4. Provider Model + Configuration

### 4.1 `RpcConfig` (TOML schema)

**File:** `src/config/schemas/rpc.rs`

Key fields (grouped):

- Endpoints:
  - `rpc.urls: Vec<String>` (ordered list; first is "primary")
  - `rpc.selection_strategy: String` (`adaptive`, `round_robin`, `priority`, `latency`)

- Timeouts + pooling (used by reqwest client builder):
  - `request_timeout_secs`
  - `connection_timeout_secs`
  - `pool_connections_per_host`
  - `pool_idle_timeout_secs`

- Retry policy:
  - `max_retries`
  - `retry_base_delay_ms`
  - `retry_max_delay_ms`

- Circuit breaker:
  - `circuit_breaker_enabled`
  - `circuit_breaker_failure_threshold`
  - `circuit_breaker_success_threshold`
  - `circuit_breaker_open_duration_secs`
  - `circuit_breaker_half_open_requests`

- Stats:
  - `stats_enabled`
  - (Some schema fields like `stats_retention_days` / `stats_minute_buckets` exist but are not
    currently wired into the stats implementation; see stats section.)

- Rate limiting:
  - Provider-specific limits exist in schema (helius/quicknode/triton/public/default), but the
    currently constructed providers use `ProviderKind::default_rate_limit()` unless a provider
    explicitly sets `ProviderConfig.rate_limit > 0` (see 7.4 caveats).

### 4.2 Provider identity + detection

**File:** `src/rpc/provider/detection.rs`

- `detect_provider_kind(url) -> ProviderKind`
- `generate_provider_id(url) -> String` (hash-based, includes provider kind)
- `derive_websocket_url(http_url) -> Option<String>` (https -> wss, http -> ws)

### 4.3 `ProviderConfig`

**File:** `src/rpc/provider/config.rs`

```rust
pub struct ProviderConfig {
  pub id: String,
  pub url: String,
  pub kind: ProviderKind,
  pub priority: u8,
  pub rate_limit: u32,   // 0 => ProviderKind default
  pub enabled: bool,
  pub timeout_secs: u64,
  pub max_retries: u32,
  pub weight: u32,       // reserved (not currently used in selection)
}
```

Provider configs are currently built from `rpc.urls` in `RpcManager::from_urls(...)` using:

- `ProviderConfig::from_url_with_priority(url, (index * 10) as u8)`

So the URLs list order becomes the default priority order.

---

## 5. `RpcManager` Request Execution Pipeline

The orchestrator is `RpcManager` in `src/rpc/manager.rs`.

The core method used by `RpcClientMethods` is:

- `RpcManager::execute_raw(method: &str, params: Value) -> Result<Value, RpcError>`

### 5.1 Step-by-step flow

For each call:

1. Parse the method string into an `RpcMethod` enum:
   - `RpcMethod::from_str(method)`
2. Maintain:
   - `tried_providers: Vec<String>` (each provider tried at most once per `execute_raw` call)
   - `last_error: Option<RpcError>`
3. Retry loop:
   - `for retry in 0..=self.max_retries` (attempts = max_retries + 1)
4. Select a provider (`select_provider`) excluding `tried_providers`
5. Circuit breaker gate:
   - `breaker.can_execute().await` (skip provider if Open)
6. Rate limiter gate:
   - `limiter.acquire(&rpc_method).await`
7. Execute HTTP JSON-RPC POST (`execute_single`)
8. Classify and record:
   - on success:
     - `breaker.record_success()`
     - `limiter.record_success()`
     - update `ProviderState` latency/error counters
     - record an `RpcCallResult` into stats
   - on error:
     - if rate limited:
       - `limiter.record_429(retry_after).await`
       - (circuit breaker is not updated in this path)
     - else:
       - `breaker.record_failure(...)`
     - update ProviderState error counters
     - record stats
9. If error is not retryable, return immediately; else exponential backoff and retry.

---

## 6. Provider Selection Strategies

The strategy is represented by:

- `SelectionStrategy` (`src/rpc/types.rs`)

and is set from config via:

- `SelectionStrategy::from_str(&cfg.rpc.selection_strategy)`

### 6.1 Selection in `RpcManager`

`RpcManager::select_provider(excluded)`:

- filters providers by:
  - `p.enabled`
  - not excluded
  - `ProviderState::is_healthy()` (currently checks `enabled && circuit_state == Closed`)
- if no "available" providers:
  - falls back to any enabled provider not excluded

**Note:** circuit breaker state is checked after selection; selection does not consult
`ProviderCircuitBreaker` directly.

### 6.2 Strategy behaviors

- RoundRobin
  - `round_robin_index: AtomicUsize`
  - selects `available[idx % available.len()]`

- Priority
  - picks `available.first()` (providers are constructed in URL order, so this is effectively
    "first URL wins" unless provider list ordering changes)

- LatencyBased
  - picks the provider with the smallest `ProviderState.avg_latency_ms`
  - no latency data defaults to `f64::MAX`

- Adaptive (default)
  - selects the provider with highest score:

```text
success_score  = success_rate_percent * 0.7            // 0..70
latency_score  = min(1000 / avg_latency_ms, 20.0)      // 0..20 (inverse)
priority_score = 10.0 - (priority / 25.5)              // 0..10 (lower priority is better)

total_score = success_score + latency_score + priority_score
```

Where `success_rate_percent` is `ProviderState::success_rate()` (a percentage in the range
0..=100).

### 6.3 `selector.rs` (alternate selector abstraction)

`src/rpc/selector.rs` provides `ProviderSelector` implementations that mirror the same selection
logic, plus a `create_selector(strategy)` factory.

Current `RpcManager` uses its own `select_provider` implementation rather than calling
`create_selector`, but the algorithms match.

---

## 7. Rate Limiting (Governor / GCRA)

### 7.1 Rate limiter manager

**File:** `src/rpc/rate_limiter/mod.rs`

`RateLimiterManager` maintains:

- a map of `provider_id -> ProviderRateLimiter`
- default rates per `ProviderKind`

### 7.2 Per-provider limiter (GCRA)

**File:** `src/rpc/rate_limiter/provider.rs`

`ProviderRateLimiter` wraps a Governor limiter:

```rust
limiter: governor::RateLimiter<NotKeyed, InMemoryState, DefaultClock, NoOpMiddleware>
```

### 7.3 Method cost weighting

`RpcMethod::cost()` (`src/rpc/types.rs`) returns a weight (1..5) and the limiter enforces it by:

```rust
for _ in 0..cost {
  self.limiter.until_ready().await;
}
```

This is the primary mechanism that makes heavier methods (e.g. `getProgramAccounts`) consume more
quota.

### 7.4 Adaptive backoff on 429 (current behavior)

`ProviderRateLimiter::record_429(retry_after)`:

- increments `consecutive_429s`
- computes a reduced `current_rate` using exponential reduction
- resets recovery progress
- optionally sleeps for a bounded `retry_after` duration

**Important:** the Governor quota itself is fixed at construction time (`Quota::per_second(rate)`).
Today, `current_rate` is primarily a **tracked metric** (and the optional `retry_after` sleep is
the real throttle).

### 7.5 Config caveats (wiring)

`RateLimiterManager::from_config()` reads provider-kind overrides from `RpcConfig`, but
`RpcManager::execute_raw` currently calls:

```rust
get_limiter(provider_id, Some(provider.effective_rate_limit()), provider.kind)
```

`ProviderConfig::effective_rate_limit()` returns:

- `provider.rate_limit` if set (> 0), else
- `provider.kind.default_rate_limit()` (hard-coded in `ProviderKind`)

So config fields like `cfg.rpc.helius_rate_limit` do not affect providers constructed via
`ProviderConfig::from_url_with_priority(...)` unless provider configs are extended to set
`rate_limit`.

In other words: the override values are loaded into `RateLimiterManager.default_rates`, but the
current `execute_raw` call path bypasses them by always providing an explicit override rate.

---

## 8. Circuit Breaker

### 8.1 Core types

**Files:**

- `src/rpc/circuit_breaker/config.rs` (`CircuitBreakerConfig`)
- `src/rpc/circuit_breaker/state.rs` (`ProviderCircuitBreaker`)
- `src/rpc/circuit_breaker/mod.rs` (`CircuitBreakerManager`)

States are represented by:

- `CircuitState` (`src/rpc/types.rs`): `Closed | Open | HalfOpen`

### 8.2 Transitions (high level)

- Closed -> Open: consecutive failures >= `failure_threshold` AND `min_state_duration` elapsed
- Open -> HalfOpen: `open_duration` elapsed
- HalfOpen -> Closed: consecutive successes >= `success_threshold`
- HalfOpen -> Open: any failure

### 8.3 Half-open probing

In half-open state, `can_execute()` allows only a limited number of probe requests:

- `half_open_max_requests` (from config via `RpcManager::from_urls`)

If exceeded, `can_execute()` returns `Err(Duration::from_millis(100))`.

### 8.4 Rate-limit failures

`ProviderCircuitBreaker::record_failure(error, is_rate_limit)` supports ignoring rate limits via:

- `CircuitBreakerConfig.ignore_rate_limits`

However, `RpcManager::execute_raw` does not call `record_failure` for 429 errors; it routes them to
`ProviderRateLimiter::record_429`. So circuit breakers are currently tripped by non-429 failures.

### 8.5 Config fields not currently enforced

`CircuitBreakerConfig.half_open_timeout` exists but is not used by the state machine today.

---

## 9. Error Model + Retry Policy

### 9.1 RpcError

**File:** `src/rpc/errors.rs`

Key variants:

- `RateLimited { provider_id, retry_after }`
- `Network { message, is_timeout }`
- `ProviderError { code, message, data }` (JSON-RPC error object)
- `CircuitOpen { provider_id, retry_after }`
- `NoProvidersAvailable { last_error }`
- `AccountNotFound { pubkey }`
- `InvalidResponse { message }`
- `Configuration { message }`

### 9.2 Retryability rules

`RpcError::is_retryable()` is the single source of truth for retry decisions:

- retryable:
  - `RateLimited`
  - `Network`
  - `Timeout`
  - `ProviderError` for JSON-RPC server error code range `-32099..=-32000`
- not retryable:
  - `CircuitOpen`
  - `NoProvidersAvailable`
  - `AccountNotFound`
  - `InvalidResponse`
  - `Configuration`

### 9.3 Backoff schedule

Backoff is applied only when:

- `retry < max_retries`, and
- error is retryable

Delay formula:

```text
delay = min(retry_delay_base * (2 ^ retry), retry_delay_max)
```

Note: in the implementation, the loop variable is an attempt index that starts at 0, and the sleep
is applied only after a failure. So the *first* backoff delay uses `2 ^ 0` and equals
`retry_delay_base * 1`.

---

## 10. Typed RPC Client Methods (`RpcClientMethods`)

Most modules do not call `RpcManager::execute_raw` directly.

Instead they call methods on `RpcClient`, via the trait:

- `RpcClientMethods` (`src/rpc/client/methods.rs`)

This layer:

- maps typed inputs/outputs to/from JSON-RPC
- parses responses into Solana SDK types (e.g. `Account`, `Hash`, `Signature`)
- provides convenience helpers for common patterns (token accounts, confirmations, batching)

Key method families:

- account reads (`get_account`, `get_multiple_accounts`, commitments)
- balance reads (`get_sol_balance`, token balances)
- token account utilities (`get_all_token_accounts`, `get_associated_token_account`, Token-2022 checks)
- program scans (`get_program_accounts`, filters, data slicing)
- transaction send + confirm flows (`sign_and_send_transaction`, `confirm_transaction`, batching)
- stats/health (`get_stats`, `get_provider_health`, reset circuit breakers)

Provider-specific methods are represented in `RpcMethod` as well (e.g. DAS-related methods).

---

## 11. Statistics System (`rpc_stats.db`)

Stats live in `src/rpc/stats/**` and persist to:

- `paths::get_data_directory().join("rpc_stats.db")`

### 11.1 Writer architecture

- `StatsManager` creates a new session on startup and (if enabled) spawns a channel-based
  `StatsCollector` background writer.
- `StatsCollector` buffers call records and flushes periodically (5s) or when the buffer is full.

### 11.2 Database schema

**File:** `src/rpc/stats/database.rs`

Tables:

- `sessions`
  - session id, started_at, ended_at, totals, is_current
- `providers`
  - provider id, masked url, kind, priority, enabled
- `calls`
  - per-call time series records (method, success, latency, retry_count, was_rate_limited, timestamp)
- `minute_buckets`
  - schema exists, but is not currently written by the recorder
- `provider_health`
  - schema exists; `StatsManager::update_provider_health` exists, but `RpcManager` does not
    currently call it

### 11.3 Monitoring + cleanup task

`start_rpc_stats_auto_save_service` (`src/rpc/stats/helpers.rs`) is run as a service:

- `services/implementations/rpc_stats_service.rs` (service name: `rpc_stats`)

Behavior:

- polls stats every 60s
- logs a warning if success_rate < 90%
- runs cleanup every ~60 ticks with a fixed retention window of 72 hours

---

## 12. WebSocket Utilities

**File:** `src/rpc/websocket.rs`

This module only provides helpers:

- convert HTTP RPC URL -> WS URL (`get_websocket_url`, `get_websocket_url_from_http`)
- build subscription payloads:
  - `accountSubscribe`
  - `logsSubscribe` with mentions filter
- lightweight log message helpers:
  - `logs_contains_initialize_mint`
  - `logs_contains_initialize_account` (also checks InitializeAccount3)

There is no WebSocket connection lifecycle management here.

---

## 13. Module Connections

```text
rpc/
├── config/          RPC urls + retry/timeouts/strategy toggles
├── database/        central SQLite PRAGMA configuration (rpc_stats.db)
├── services/        rpc_stats monitoring service
└── (callers)        tokens, pools, filtering, wallets, transactions, swaps, connectivity, ...
```

### Pitfalls / gotchas

- Some `RpcConfig` fields exist in schema but are not currently wired (e.g. minute buckets,
  retention days, burst factor).
- Provider selection consults `ProviderState::is_healthy`, but circuit breaker state is checked as
  a separate gate (`breaker.can_execute()`); keep that in mind when interpreting health telemetry.
