# ScreenerBot System Flow

This document provides a high-level overview of the system architecture and data flow.

## System Overview

ScreenerBot is an automated Solana DeFi trading bot that discovers tokens, analyzes them, and executes trades based on configured strategies.

### Website License Management (Nov 2025)

- The Next.js web admin now reads license plans exclusively from the `planConfig` table via `/api/plans`; all hardcoded tier constants were removed so pricing and durations derive from the database single source of truth.
- `POST /api/license/mint` pulls the plan definition before minting, verifies the USDC transfer amount and sender associated token account, records the payment in `Payment` with verification metadata, and links the row to the minted license upon success.
- License metadata uses dynamic plan data (name, duration, price paid) and embeds the actual USDC amount received so off-chain consumers see the same pricing found in the plan configuration.
- Admin-only license APIs now enforce `isAdmin()` authentication before revealing license lists or allowing revocations.
- Mint flow now wraps payment lookups in a serializable Prisma transaction, toggling new `processing` / `processingStartedAt` columns so concurrent `/api/license/mint` requests for the same signature block with a 2-minute timeout and roll back cleanly on failure.
- Payment verification prefers previously recorded `Payment.amount` or `PaymentIntent.amount` values before falling back to the current plan price, ensuring legitimate signatures remain valid after pricing updates while still logging mismatches.

## Core Components

```
┌─────────────────────────────────────────────────────────────────────────┐
│                            SCREENERBOT                                  │
│                         (Main Orchestrator)                             │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                    ┌───────────────┼───────────────┐
                    ▼               ▼               ▼
            ┌───────────┐   ┌──────────┐   ┌──────────────┐
            │  Config   │   │   RPC    │   │   Logger     │
            │  System   │   │  Client  │   │   System     │
            └───────────┘   └──────────┘   └──────────────┘
```

## Token System Architecture

The tokens module is the core data layer for token information, with clean separation of concerns:

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         TOKENS MODULE                                   │
│                    (Unified Token Data System)                          │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
        ┌───────────────────────────┼───────────────────────────┐
        ▼                           ▼                           ▼
┌──────────────┐          ┌──────────────┐          ┌──────────────┐
│  Database    │          │   Discovery  │          │   Updates    │
│  (tokens.db) │          │   (New       │          │  (Priority-  │
│              │          │   Tokens)    │          │   Based)     │
└──────────────┘          └──────────────┘          └──────────────┘
        │                           │                           │
        ▼                           ▼                           ▼
┌──────────────┐          ┌──────────────┐          ┌──────────────┐
│  Decimals    │          │   Market     │          │   Security   │
│  (Cache +    │          │   (DexScrn,  │          │  (Rugcheck)  │
│   Chain)     │          │   GeckoTerm) │          │              │
└──────────────┘          └──────────────┘          └──────────────┘
        │                           │                           │
        └───────────────────────────┴───────────────────────────┘
                                    │
                                    ▼
                         ┌──────────────────┐
                         │ Filtered Store   │
                         │ (Pass/Reject/    │
                         │  Blacklist)      │
                         └──────────────────┘
```

### Database Schema (tokens.db)

- **tokens**: Basic token metadata (mint, symbol, name, created_at)
- **market_dexscreener**: DexScreener market data (price, volume, liquidity, txns)
- **market_geckoterminal**: GeckoTerminal market data (price, volume, liquidity)
- **security_rugcheck**: Rugcheck security data (score, authorities, risks, holders)
- **tracking**: Update tracking (last_updated, had_errors, priority)
- **blacklist**: Blacklisted tokens (reason, source, added_at)

12 indexes for efficient queries by mint, priority, timestamp, blacklist status.

### Pool Snapshot Cache

- Tokens module now owns aggregated pool snapshots (`token_pools` table + in-memory cache) built from DexScreener `/token-pairs` and GeckoTerminal `/tokens/{mint}/pools` endpoints.
- `RateLimitCoordinator` gained a dedicated semaphore for full pool fetches (300/min) so DexScreener pools, batch market data, and discovery endpoints never starve each other; GeckoTerminal calls continue to respect the 30/min cap.
- `get_token_pools_snapshot()` merges both sources once, deduplicates by pool address, and selects the canonical SOL pair via highest liquidity/volume metric before persisting to SQLite and caching for 60s.
- Concurrent refreshes are coalesced with `POOL_REFRESH_INFLIGHT`; callers receive fresh data while background prefetch (debounced 20s per mint) warms snapshots for newly passed tokens.
- When all live sources fail, we fall back to the last persisted snapshot (if allowed) so downstream consumers stay functional while logging the degraded state.
- Pool discovery now consumes these centralized snapshots (via `prefetch_token_pools` and `get_token_pools_snapshot*`) to eliminate direct API calls, ensuring all rate limiting and caching stays inside the tokens module.
- Snapshot refreshes emit structured token events for success, fallback, and canonical changes so observability tooling can surface pool state transitions in real time.

## Service Layer

The service layer provides a unified framework for managing all bot subsystems with dependency resolution, priority-based startup, health monitoring, and per-service metrics instrumentation.

### Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         SERVICE MANAGER                                 │
│              (Global Singleton via GLOBAL_SERVICE_MANAGER)              │
│                                                                         │
│  Core Responsibilities:                                                 │
│  • Dependency resolution (topological sort + circular detection)        │
│  • Priority-based startup (lower = earlier, higher = later)             │
│  • Reverse-order shutdown (higher priority stops first)                 │
│  • Metrics collection via MetricsCollector                              │
│  • Cached health/metrics for non-blocking dashboard reads               │
│  • Background cache updater (every 5s with 3s timeout)                  │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
        ┌───────────────────────────┼───────────────────────────┐
        ▼                           ▼                           ▼
   Service Trait              Registration Flow           Metrics System
   (src/services/             (src/run.rs::               (src/services/
    mod.rs)                    register_all_services)      metrics.rs)
        │                           │                           │
        ▼                           ▼                           ▼
   - name()                    20 Services                 TaskMonitor
   - priority()                Registered                  (tokio_metrics)
   - dependencies()            (order doesn't                   │
   - is_enabled()              matter - manager            Samples every 1s
   - initialize()              handles it)                 via cumulative()
   - start(monitor)                                             │
   - stop()                                                Accumulates:
   - health()                                              - total_polls
   - metrics()                                             - poll_duration
                                                           - idle_duration
                                                           - cycles_per_sec
```

### Registered Services (20 Total)

**Priority-ordered startup sequence** (lower number = starts earlier):

```
Priority 10:  events          [No dependencies]
              └─ Events system initialization

Priority 30:  pool_helpers    [Depends: transactions]
              └─ Pool components (database, cache, RPC, global components)
              webserver        [Depends: filtering]
              └─ REST API + dashboard (starts last in priority 30 group)

Priority 40:  tokens          [Depends: events, transactions, pools]
              └─ Centralized token system (discovery, updates, security)

Priority 45:  ohlcv           [Depends: tokens, positions]
              └─ Multi-timeframe OHLCV data collection

Priority 50:  positions       [No dependencies]
              └─ Position tracking and reconciliation

Priority 80:  transactions    [No dependencies]
              └─ Transaction monitoring (WebSocket + bootstrap)

Priority 90:  filtering       [Depends: tokens_store, pool_helpers,
              │                         token_discovery, security]
              └─ Token filtering engine

              wallet          [No dependencies]
              └─ Balance monitoring and snapshots

Priority 100: pool_discovery  [Depends: transactions, pool_helpers, filtering]
              └─ Pool discovery from APIs

              rpc_stats       [No dependencies]
              └─ RPC statistics auto-save

Priority 101: pool_fetcher    [Depends: transactions, pool_helpers,
              │                         pool_discovery, filtering]
              └─ Batched RPC account fetching

Priority 102: pool_calculator [Depends: pool_helpers, pool_fetcher, filtering]
              └─ Price calculation from account data

Priority 103: pool_analyzer   [Depends: pool_helpers, pool_fetcher, filtering]
              └─ Pool analysis and classification

Priority 110: ata_cleanup     [No dependencies]
              └─ Associated Token Account cleanup

Priority 120: sol_price       [No dependencies]
              └─ SOL price tracking

Priority 150: trader          [Depends: positions, pool_helpers, pool_discovery,
              │                         pool_fetcher, pool_calculator, tokens,
              │                         filtering]
              └─ Trading logic (entry/exit monitoring)

### RPC System Architecture (Dec 2025)

The RPC layer is a modular multi-provider system with rate limiting, circuit breaker failover, and per-provider statistics.

```

┌─────────────────────────────────────────────────────────────────────────┐
│ RPC MANAGER │
│ (Global Singleton via init_rpc_manager()) │
│ │
│ Access: get_rpc_client() + RpcClientMethods trait │
│ Config: [rpc] section in config.toml │
└─────────────────────────────────────────────────────────────────────────┘
│
┌───────────────────────────┼───────────────────────────┐
▼ ▼ ▼
Provider Pool Selection Strategy Stats Database
(from config URLs) (selector.rs) (data/rpc_stats.db)
│ │ │
▼ ▼ ▼
Per-Provider: Strategies: Per-minute buckets

- Rate Limiter - Adaptive (default) 24h retention
  (Governor GCRA) - RoundRobin Session tracking
- Circuit Breaker - Priority
  (3-state FSM) - LatencyBased
- Latency tracking

```

**Module Structure** (`src/rpc/*`):

| Module | Purpose |
|--------|---------|
| `manager.rs` | RpcManager singleton, `execute_raw()`, provider orchestration |
| `selector.rs` | Provider selection strategies (Adaptive, RoundRobin, Priority, Latency) |
| `client/` | RpcClient wrapper with `RpcClientMethods` trait in `methods.rs` |
| `provider/` | ProviderConfig, auto-detection (Helius, QuickNode, Triton, Alchemy), WebSocket derivation |
| `rate_limiter/` | Governor GCRA algorithm with per-provider limits and adaptive backoff |
| `circuit_breaker/` | Failover protection: Closed → Open → HalfOpen state machine |
| `stats/` | SQLite persistence for RPC metrics (`data/rpc_stats.db`) |
| `types.rs` | `ProviderKind`, `RpcMethod`, `CircuitState`, `SelectionStrategy` |
| `errors.rs` | `RpcError` enum with `is_retryable()` detection |
| `compat.rs` | Backward compatibility helpers for legacy code |

**Request Flow:**

```

[RPC Call via get_rpc_client()] → RpcManager
│
▼
Select Provider (strategy)
│
▼
Check Circuit Breaker
┌─────────┼─────────┐
│ Closed │ Open │
▼ ▼ │
Proceed Skip to │
│ next provider│
▼ │
Wait for Rate Limit Permit │
│ │
▼ │
Execute RPC Request ◄───────┘
│
┌─────────────┼─────────────┐
▼ Success │ ▼ Failure
Record latency │ Update circuit breaker
Reset failures │ Check if retryable
│ │ │
│ │ ┌──────┴──────┐
│ │ ▼ Retryable ▼ Fatal
│ │ Try next Return error
│ │ provider
└─────────────┴─────────────────────┘
│
▼
Return Result + Record Stats

```

**Circuit Breaker States:**

- **Closed**: Normal operation, requests pass through
- **Open**: Provider failed repeatedly, requests skip this provider (configurable timeout)
- **HalfOpen**: Testing recovery, single request allowed through

**Config Options** (`[rpc]` section):

- `selection_strategy`: "adaptive" | "round_robin" | "priority" | "latency_based"
- `failure_threshold`: Failures before circuit opens (default: 5)
- `recovery_timeout_secs`: Time before Open → HalfOpen (default: 30)
- Per-provider rate limits via URL configuration

**Critical Rules:**

- **Always use `get_rpc_client()`** from `src/rpc/*` - never construct `RpcClient` directly
- **Transaction encoding must be `jsonParsed`** to resolve LUT addresses for v0 transactions
- **Max 50 accounts** per `get_multiple_accounts` call

#### RPC Metrics Pipeline

- Every runtime boot spawns a fresh session with unique `session_id`
- Stats recorded into rolling deque of 1,440 one-minute buckets (24h retention)
- Dashboard reads last 5 minutes from bucket deque for accurate calls/sec display
- `RpcStatsSnapshot` for analytics“calls per minute / per second” even immediately after reboot rather than using lifetime totals divided by a few startup seconds.
```

### Connectivity Service

- Monitors internet, RPC, DexScreener, GeckoTerminal, Rugcheck, Jupiter, and GMGN endpoints with shared logging
- RPC health checks reuse the centralized `RpcClient::probe_get_health` helper so availability mirrors runtime behavior and respects rate limits
- Endpoint metadata auto-registers on first successful poll, keeping criticality/fallback settings in sync after hot-reload without recycling the service

### Service Trait Contract

Each service implements the `Service` trait from `src/services/mod.rs`:

```rust
#[async_trait]
pub trait Service: Send + Sync {
    fn name(&self) -> &'static str;                    // Unique identifier
    fn priority(&self) -> i32 { 100 }                  // Lower = starts earlier
    fn dependencies(&self) -> Vec<&'static str> { vec![] }
    fn is_enabled(&self) -> bool { true }

    async fn initialize(&mut self) -> Result<(), String> { Ok(()) }

    // CRITICAL: Must return Vec<JoinHandle<()>> for spawned tasks
    // All tasks MUST be instrumented: monitor.instrument(async { ... })
    async fn start(
        &mut self,
        shutdown: Arc<Notify>,
        monitor: tokio_metrics::TaskMonitor,
    ) -> Result<Vec<JoinHandle<()>>, String>;

    async fn stop(&mut self) -> Result<(), String> { Ok(()) }
    async fn health(&self) -> ServiceHealth { ServiceHealth::Healthy }
    async fn metrics(&self) -> ServiceMetrics { ServiceMetrics::default() }
}
```

### Startup Flow (Detailed)

```
[Application Start] → Initialize Profiling (if enabled)
                          │
                          ▼
                  Acquire Process Lock
                  (data/.screenerbot.lock)
                  - Prevents multiple instances
                  - RAII pattern: held until shutdown
                  - OS auto-releases on crash
                          │
                          ▼
                  Initialize File Logging
                          │
                          ▼
                  Load Configuration
                  (data/config.toml)
                          │
                          ▼
                  Initialize Strategy System
                          │
                          ▼
                  Create ServiceManager
                          │
                          ▼
                  Register All Services (20 total)
                  (order doesn't matter - manager resolves)
                          │
                          ▼
                  Filter Enabled Services
                  (check is_enabled() on each)
                          │
                          ▼
                  Resolve Startup Order:
                  1. Topological sort by dependencies
                  2. Detect circular dependencies (error if found)
                  3. Sort by priority (lower = earlier)
                          │
                          ▼
                  For Each Service (in priority order):
                          │
        ┌─────────────────┼─────────────────┬──────────────────┐
        ▼                 ▼                 ▼                  ▼
   Get TaskMonitor   Initialize()     Start(monitor)    Register with
   (creates if       (one-time        (spawn tasks)     MetricsCollector
   doesn't exist)    setup)           instrumented      (start sampling)
        │                 │                 │                  │
        └─────────────────┴─────────────────┴──────────────────┘
                                    │
                          ▼
        Log Timing (if > 100ms) and Handle Count
                          │
                          ▼
        Store Handles in ServiceManager
        (for graceful shutdown)
                          │
                          ▼
        Continue to Next Service
                          │
                          ▼
        [All Services Started]
                          │
                          ▼
        Initialize Global ServiceManager
        (GLOBAL_SERVICE_MANAGER singleton)
                          │
                          ▼
        Perform Initial Cache Update
        (health + metrics snapshot)
                          │
                          ▼
        Spawn Background Cache Updater
        (every 5s with 3s timeout)
                          │
                          ▼
        [System Ready]
```

> Data root (platform standard): macOS `~/Library/Application Support/ScreenerBot`, Windows `%LOCALAPPDATA%\ScreenerBot`, Linux `$XDG_DATA_HOME/ScreenerBot` (fallback `~/.local/share/ScreenerBot`). All `data/`, `logs/`, and `analysis-exports/` paths resolve under this root.

### Shutdown Flow

```
[Shutdown Signal] → ServiceManager.stop_all()
                          │
                          ▼
                  Signal All Services
                  (shutdown.notify_waiters())
                          │
                          ▼
                  Get Services in Reverse Priority Order
                  (higher priority = stops first)
                          │
                          ▼
                  For Each Service (reverse order):
                          │
        ┌─────────────────┼─────────────────┬──────────────────┐
        ▼                 ▼                 ▼                  ▼
   Call stop()      Wait for Handles   Log Status        Track Timeouts
   (cleanup)        (10s timeout       (clean/panic/     (warn if > 10s)
                     per task)          timeout)
        │                 │                 │                  │
        └─────────────────┴─────────────────┴──────────────────┘
                                    │
                          ▼
        Continue to Next Service
                          │
                          ▼
        [All Services Stopped]
```

### Metrics Collection System

Located in `src/services/metrics.rs` with integration in `src/services/mod.rs`:

**Architecture:**

- **MetricsCollector**: Manages per-service metric accumulation
- **TaskMonitor**: tokio_metrics monitor passed to each service's start()
- **Background Sampling**: Spawned task samples monitor.cumulative() every 1 second
- **Accumulated Storage**: HashMap<&'static str, AccumulatedTaskMetrics>

**Collected Metrics:**

```
Per-Service Task Metrics:
  • instrumented_count         - Number of instrumented tasks
  • total_polls                - Cumulative poll count (futures awakened)
  • total_poll_duration_ns     - Time spent polling (working)
  • mean_poll_duration_ns      - Average time per poll
  • total_idle_duration_ns     - Time spent idle (awaiting)
  • mean_idle_duration_ns      - Average idle time
  • last_cycle_duration_ns     - Last poll+idle cycle duration
  • avg_cycle_duration_ns      - Average cycle duration
  • cycles_per_second          - Polling frequency

Process-Wide Metrics (shared):
  • process_cpu_percent        - CPU usage (all services)
  • process_memory_bytes       - Memory usage (all services)

Service-Specific Metrics (from service.metrics()):
  • operations_total           - Service-defined operation count
  • operations_per_second      - Operation rate
  • errors_total               - Error count
  • custom_metrics             - HashMap<String, f64> for custom data
```

**Critical Implementation Details:**

1. **Sampling Strategy**: Uses `monitor.cumulative()` every 1s, NOT `intervals()` (blocking API)
2. **Instrumentation Requirement**: Services MUST wrap spawned tasks:
   ```rust
   let handle = tokio::spawn(monitor.instrument(async move {
       // Service logic here
   }));
   ```
3. **Cache Updates**: Background task updates health/metrics cache every 5s with 3s timeout
4. **Non-Blocking Reads**: Dashboard uses cached values (`get_health_cached()`, `get_metrics_cached()`)
5. **Live Reads**: Available via `get_health()` and `get_metrics()` (slower, blocks on collection)

### Dependency Resolution

**Algorithm** (in `ServiceManager::resolve_startup_order`):

1. Depth-first traversal with visited/visiting sets
2. Detect circular dependencies (error if visiting set contains current node)
3. Build ordered list (dependencies before dependents)
4. Sort by priority (lower = earlier, stable sort preserves dependency order)

**Example Resolution:**

```
trader (150) depends on [positions, pool_discovery, ...]
  └─ positions (50) depends on []
  └─ pool_discovery (100) depends on [transactions, pool_helpers, filtering]
      └─ transactions (80) depends on []
      └─ pool_helpers (30) depends on [transactions]
      └─ filtering (90) depends on [tokens_store, ...]

Resolved Order:
1. events (10)
2. pool_helpers (30)
3. webserver (30)
4. tokens (40)
5. ohlcv (45)
6. positions (50)
7. transactions (80)
8. filtering (90)
9. wallet (90)
10. pool_discovery (100)
11. ... (continues in priority order)
```

### Global Access

Services accessible globally via `src/services/mod.rs`:

```rust
// Initialize (called in run.rs after all services started)
init_global_service_manager(manager).await;

// Access from anywhere
if let Some(manager_arc) = get_service_manager().await {
    let manager = manager_arc.read().await;
    if let Some(mgr) = manager.as_ref() {
        // Use manager (get_health_cached(), get_metrics_cached(), etc.)
    }
}
```

**WebServer Integration:**

- `AppState` has `get_service_manager()` method
- Routes use cached metrics/health for fast dashboard updates
- No direct service imports - all access via ServiceManager

## Data Flow (High-Level)

### 1. Startup Sequence

```
[Config Load] → [RPC Init] → [Logger Init] → [Service Manager Init]
                                                      │
                                    ┌─────────────────┼─────────────────┐
                                    ▼                 ▼                 ▼
                          [Token Service]   [Pool Helpers]   [Transaction Service]
                          (Priority 20)     (Priority 30)     (Priority 80)
                                    │                 │                 │
                                    │                 ▼                 │
                                    │      [Pool Sub-Services]          │
                                    │      (Priorities 100-103)         │
                                    │                 │                 │
                                    └─────────────────┼─────────────────┘
                                                      ▼
                                          [Core Services Ready]
                                                      │
                                                      ▼
                                            [Trader Service Start]
```

### 2. Pool Service Flow (Real-Time Price Discovery & Calculation)

```
[Pool Service Architecture] → 4 Components + 5 Services
                                    │
        ┌───────────────────────────┼───────────────────────────┐
        ▼                           ▼                           ▼
   Components              Services (priority-ordered)      Database
   (Singletons)            (via ServiceManager)             (pools.db)
        │                           │                           │
        ├─ PoolDiscovery            ├─ pool_helpers (30)       └─ Price History
        ├─ PoolAnalyzer             ├─ pool_discovery (100)       Cache (TTL)
        ├─ AccountFetcher           ├─ pool_analyzer (101)
        └─ PriceCalculator          ├─ pool_fetcher (102)
                                    └─ pool_calculator (103)
```

#### Pool Discovery Loop (Every 5 seconds)

```
[Discovery Service] → Get Tokens to Monitor
                          │
        ┌─────────────────┼─────────────────┐
        ▼                                   ▼
   tokens::get_passed_tokens()    positions::get_open_mints()
   (from filtering)                (always include!)
        │                                   │
        └─────────────────┬─────────────────┘
                          │
                          ▼
                  Merge & Deduplicate
                          │
                          ▼
                  Filter Stablecoins
                          │
                          ▼
                  Cap to max_watched_tokens
                  (prioritize position tokens)
                          │
                          ▼
        ┌─────────────────┼─────────────────┬─────────────────┐
        ▼                 ▼                 ▼                 ▼
   DexScreener      GeckoTerminal      Raydium          Database
   (batch API)      (per-token)        (not impl)       (known pools)
        │                 │                 │                 │
        └─────────────────┴─────────────────┴─────────────────┘
                          │
                          ▼
                 Convert to PoolDescriptor
                 (pool_id, base/quote, liquidity)
                          │
                          ▼
                 Deduplicate by pool_id
                          │
                          ▼
            [If single_pool_mode enabled]
                          │
                          ▼
            Keep only highest liquidity per token
                          │
                          ▼
                 Stream to Analyzer (channel)
```

#### Pool Analysis (Event-Driven via Channel)

```
[Analyzer Service] → Receive PoolDescriptor from Discovery
                          │
                          ▼
                  Check failed_pairs cache
                  (skip re-analysis if failed this run)
                          │
                          ▼
                  Classify Program Type:
                          │
        ┌─────────────────┼─────────────────┬─────────────────┐
        ▼                 ▼                 ▼                 ▼
   Raydium          Orca            Meteora          PumpFun
   (CPMM/CLMM/      (Whirlpool)     (DLMM/DAMM/      (AMM/Legacy)
    Legacy)                          DBC)
        │                 │                 │                 │
        └─────────────────┴─────────────────┴─────────────────┘
                          │
                          ▼
              Fetch on-chain pool account data (RPC)
              to determine reserve accounts layout
                          │
                          ▼
              Store in pool_directory (HashMap)
                          │
                          ▼
              Send reserve accounts to Fetcher
```

#### Account Fetching (Every 500ms, Batched RPC)

```
[Fetcher Service] → Collect Pending Accounts
                          │
                          ▼
                  Check Staleness:
                  - Position tokens: 5s threshold
                  - Other tokens: 30s threshold
                          │
                          ▼
                  Batch by Pool (max 50 accounts/RPC)
                          │
                          ▼
                  Fetch via get_multiple_accounts (RPC)
                          │
                          ▼
                  Store in account_bundles (HashMap)
                          │
                          ▼
              Check if bundle complete (all accounts fetched)
                          │
                          ▼
              [If complete AND not yet calculated]
                          │
                          ▼
              Send to Calculator (channel)
              Mark calculation_requested = true
```

#### Price Calculation (Event-Driven via Channel)

```
[Calculator Service] → Receive Complete Account Bundle
                          │
                          ▼
                  Get Pool Descriptor from directory
                          │
                          ▼
                  Select Program-Specific Decoder:
                          │
        ┌─────────────────┼─────────────────┬─────────────────┐
        ▼                 ▼                 ▼                 ▼
   RaydiumCpmmDecoder  OrcaWhirlpoolDecoder  MeteoraDlmmDecoder  PumpFunAmmDecoder
        │                 │                 │                 │
        └─────────────────┴─────────────────┴─────────────────┘
                          │
                          ▼
                  Decode Pool State from Account Data
                  (extract reserve balances, fees, etc.)
                          │
                          ▼
                  Get Token Decimals (from tokens module)
                          │
                          ▼
                  Calculate SOL Price:
                  token_price_sol = sol_reserve / token_reserve
                  (adjusted for decimals)
                          │
                          ▼
                  Create PriceResult (price, liquidity, volume, timestamp)
                          │
                          ▼
        ┌─────────────────┼─────────────────┐
        ▼                                   ▼
   Update Cache (DashMap)          Update History (RwLock<DashMap>)
   + Gap Detection                  (max 1000 entries)
        │                                   │
        └─────────────────┬─────────────────┘
                          │
                          ▼
                  Queue for DB Storage (async, non-blocking)
```

#### Price Cache & Public API

```
[Public API] → pools::get_pool_price(mint)
                      │
                      ▼
                 Check if service running
                      │
                      ▼
                 Read from PRICE_CACHE (DashMap)
                      │
                      ▼
                 Check TTL (default: stale if > configured seconds)
                      │
                      ▼
                 Return Some(PriceResult) or None
```

```
[Public API] → pools::get_available_tokens()
                      │
                      ▼
                 Iterate PRICE_CACHE
                      │
                      ▼
                 Filter by TTL (fresh prices only)
                      │
                      ▼
                 Return Vec<String> (mints with fresh prices)
```

#### Helper Tasks (Background Maintenance)

```
[Health Monitor] → Every 30s
                      │
                      ▼
                 Emit cache stats
                 (total prices, fresh prices, history entries)
```

```
[DB Cleanup] → Every 6 hours
                      │
                      ▼
                 Delete old price history entries
                 (keep recent N days)
```

```
[Gap Cleanup] → Every 30 minutes
                      │
                      ▼
        ┌─────────────┴─────────────┐
        ▼                           ▼
   Memory Gaps                  Database Gaps
   (remove from cache)          (delete gapped rows)
        │                           │
        └─────────────┬─────────────┘
                      │
                      ▼
                 Log removal count
```

#### Key Configuration Options

- **enable_dexscreener_discovery**: Enable/disable DexScreener API (hot-reload)
- **enable_geckoterminal_discovery**: Enable/disable GeckoTerminal API (hot-reload)
- **enable_raydium_discovery**: Enable/disable Raydium API (not implemented)
- **enable_single_pool_mode**: Only track highest liquidity pool per token
- **max_watched_tokens**: Cap discovery to N tokens (prioritizes position tokens)
- **price_cache_ttl_seconds**: Price freshness threshold
- **fetch_interval_ms**: Account fetching frequency (default: 500ms)
- **discovery_tick_interval**: Pool discovery frequency (default: 5s)

#### Critical Data Sources

**Token Input (Discovery):**

- `tokens::get_passed_tokens()` - Tokens that passed filtering
- `positions::get_open_mints()` - Tokens with open positions (ALWAYS included)

**Pool APIs:**

- DexScreener batch API (up to 30 tokens per call)
- GeckoTerminal pools API (per-token)
- Raydium API (configuration exists, not implemented)

**On-Chain Data:**

- Pool account data via `get_multiple_accounts` (RPC, max 50 accounts)
- Token decimals via tokens module (cached + chain fallback)

**Output:**

- `pools::get_pool_price(mint)` - Current price for a token
- `pools::get_available_tokens()` - Tokens with fresh prices
- `pools::get_price_history(mint)` - Historical prices (up to 1000)

#### Important Implementation Details

1. **Position Token Priority**: Tokens with open positions are ALWAYS monitored, even if they fail filtering
2. **Failed Pool Tracking**: Pools that fail analysis are cached in-memory to avoid re-analysis within the same run
3. **Calculation Deduplication**: Account bundles track if calculation was requested to prevent duplicate price calculations
4. **Concurrent Data Structures**:
   - PRICE_CACHE uses DashMap (lock-free concurrent)
   - PRICE_HISTORY uses RwLock<DashMap> (batch operations need write lock)
5. **Gap Detection**: Automatically detects and removes gapped price data (>2x expected interval)
6. **Async DB Storage**: Price storage is queued asynchronously to avoid blocking the calculation pipeline
7. **Stale Thresholds**: Position tokens refresh every 5s, others every 30s
8. **Single Pool Mode**: When enabled, only the highest liquidity pool per token is tracked (reduces RPC load)
9. **Program-Specific Decoders**: Each DEX program (Raydium, Orca, Meteora, PumpFun) has a dedicated decoder
10. **SOL-Based Pricing**: All prices are calculated in SOL (no USD conversion in pool service)

### 3. Token Analysis Flow (Token System)

```
[Token Service Init] → Initialize Database (tokens.db)
                            │
                            ▼
                       Initialize Schema (6 tables, 12 indexes)
                            │
                            ▼
                       Set TOKENS_SYSTEM_READY flag
                            │
                            ▼
                       Start Background Tasks
                            │
        ┌───────────────────┼───────────────────┬──────────────────┐
        ▼                   ▼                   ▼                  ▼
   [Discovery]      [Priority Updates]   [Security Updates]  [Cleanup]
        │                   │                   │                  │
        ▼                   ▼                   ▼                  ▼
   Every 60s            Configurable       Configurable       Hourly
        │                   │                   │                  │
        ▼                   ▼                   ▼                  ▼
   New Tokens          Market Data          Rugcheck         Blacklist
   (DexScreener,      (DexScreener,         (One-time        (Mint/Freeze
   GeckoTerminal,     GeckoTerminal)         fetch per       Authority
   Rugcheck)                                  token)          Check)
        │                   │                   │                  │
        └───────────────────┴───────────────────┴──────────────────┘
                            │
                            ▼
                   Store in Database (tokens.db)
                            │
                            ▼
                   Cache in Memory (LRU)
                            │
                            ▼
              [Available via Async Accessors]
```

#### Token Discovery Loop (Every 60s)

```
[Discovery Task] → Check if enabled in config
                        │
                        ▼
                   Fetch from multiple sources in parallel:
                        │
        ┌───────────────┼───────────────┬────────────────┐
        ▼               ▼               ▼                ▼
   DexScreener    DexScreener     GeckoTerminal    Rugcheck
   (Profiles)     (Boosts)        (New/Trending)   (New Tokens)
        │               │               │                │
        └───────────────┴───────────────┴────────────────┘
                        │
                        ▼
                   Deduplicate mints
                        │
                        ▼
                   Filter out: SOL, stablecoins, invalid pubkeys
                        │
                        ▼
                   Check if already in database
                        │
                        ▼
                   Check if blacklisted
                        │
                        ▼
                   Insert new tokens with priority=10 (Low)
                        │
                        ▼
                   Seed loop immediately fetches market data for new tokens
```

#### Token Update Loops (Priority-Based)

All loops use shared **RateLimitCoordinator** with semaphores:

- DexScreener: 300/min (configurable)
- GeckoTerminal: 30/min (configurable)
- Rugcheck: 60/min (configurable)

**Seeding Loop** (Every 10s)

```
[Uninitialized Tokens] → Get tokens with no market data
                              │
                              ▼
                         Batch update (up to 30 tokens)
                              │
                              ▼
                         Fetch DexScreener + GeckoTerminal in parallel
                              │
                              ▼
                         Store in database
```

**Critical Priority Loop** (Configurable, default 5s)

```
[Priority = 100] → Get tokens with open positions (limit 200)
                        │
                        ▼
                   Batch update (chunks of 30)
                        │
                        ▼
                   Fetch DexScreener + GeckoTerminal in parallel
                        │
                        ▼
                   Store in database + update tracking
```

**Pool Priority Loop** (Configurable, default 7s; sources refreshed every 5s)

```
[Priority = 75] ← Pool priority sync (5s cadence)
           │             │
           │             └─ Reads pools::get_available_tokens()
           │                ▸ Promotes tokens with fresh pool prices
           │                ▸ Stores previous priority for rollback (≥60s absence)
           ▼
   Get pool-priority tokens (limit 90)
           │
           ▼
   Batch update (chunks of 30)
           │
           ▼
   Fetch DexScreener + GeckoTerminal in parallel
           │
           ▼
   Store in database + update tracking
```

**High Priority Loop** (Configurable, default 10s)

```
[Priority = 50] → Get tokens with priority=50 (limit 60)
                        │
                        ▼
                   Batch update (chunks of 30)
                        │
                        ▼
                   Fetch DexScreener + GeckoTerminal in parallel
                        │
                        ▼
                   Store in database + update tracking
```

**Low Priority Loop** (Configurable, default 30s)

```
[Priority = 10] → Get oldest 30 non-blacklisted tokens
                        │
                        ▼
                   Batch update (single batch of 30)
                        │
                        ▼
                   Fetch DexScreener + GeckoTerminal in parallel
                        │
                        ▼
                   Store in database + update tracking
```

**Security Data Loop** (Configurable, default 60s)

```
[Security Update] → Get 1 token without Rugcheck data
                        │
                        ▼
                   Fetch Rugcheck data
                        │
                        ▼
                   Store in database (security_rugcheck table)
                        │
                        ▼
                   One-time fetch (not updated again unless manually requested)
```

#### Data Caching Strategy

**Memory Caches (LRU with TTL):**

- DexScreener data: 30s TTL, 2000 capacity
- GeckoTerminal data: 60s TTL, 2000 capacity
- Rugcheck data: 30min TTL, 3000 capacity

**Fetch Flow (per source):**

```
[Request] → Check Memory Cache
                │
                ├─ Hit (fresh) → Return immediately
                │
                └─ Miss/Stale → Check Database
                                    │
                                    ├─ Hit (fresh) → Cache + Return
                                    │
                                    └─ Miss/Stale → Fetch from API
                                                        │
                                                        ▼
                                                   Store in DB + Cache
                                                        │
                                                        ▼
                                                   Return
```

#### Decimals System

Separate caching system for token decimals (needed for calculations):

```
[get_decimals(mint)] → Check Memory Cache
                            │
                            ├─ Hit → Return
                            │
                            └─ Miss → Check Failed Cache
                                        │
                                        ├─ Known Failure → Return None
                                        │
                                        └─ Try Database
                                                │
                                                ├─ Hit → Cache + Return
                                                │
                                                └─ Miss → Fetch from Chain (RPC)
                                                            │
                                                            ├─ Success → Store DB + Cache + Return
                                                            │
                                                            └─ Failure → Add to Failed Cache
```

**Single-flight pattern**: Prevents duplicate chain fetches for same mint using async mutexes.

### 4. Filtering Flow (Comprehensive Token Evaluation & Storage System)

The filtering service evaluates tokens from the token database, applies multi-source filters, and stores results in a centralized store for consumption by Pool Service, Trader, and Dashboard.

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         FILTERING SERVICE                                │
│                      (Priority 90, Every 30s)                            │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
        ┌───────────────────────────┼───────────────────────────┐
        ▼                           ▼                           ▼
┌──────────────┐          ┌──────────────┐          ┌──────────────┐
│   Engine     │          │    Store     │          │   Service    │
│  (Snapshot   │          │  (Caching &  │          │  (Background │
│   Compute)   │          │   Queries)   │          │   Loop)      │
└──────────────┘          └──────────────┘          └──────────────┘
        │                           │                           │
        ▼                           ▼                           ▼
┌──────────────┐          ┌──────────────┐          ┌──────────────┐
│   Sources    │          │   Types      │          │   Tokens     │
│  (4 Filter   │          │  (Query API) │          │  Filtered    │
│   Modules)   │          │              │          │   Store)     │
└──────────────┘          └──────────────┘          └──────────────┘
```

**Service Loop:**

- Immediate first refresh (non-blocking, async)
- Periodic refresh every 30 seconds (FILTER_CACHE_TTL_SECS)
- Dependencies: tokens_store, pool_helpers, token_discovery, security
- Health check: Snapshot age (healthy if ≤120s, degraded if older)

**Snapshot Computation:**

1. List tokens from tokens.db (up to `max_tokens_to_process`, default 1000)
2. Fetch full token data (24 concurrent fetches)
3. Get supplementary data (pool prices, open positions, OHLCV)
4. Apply filters sequentially (Meta → DexScreener → GeckoTerminal → Rugcheck)
5. Build FilteringSnapshot with passed/rejected decisions
6. Store results in tokens module for consumption by other services

**Filter Evaluation Process:**

Each token goes through **4 filter layers** in sequence. ANY failure rejects immediately:

1. **Meta Filters** (sources::meta, always run)
   - Check decimals available (optional, if `require_decimals_in_db`)
   - Check token age (`min_token_age_minutes`, default 5 minutes)
   - Check cooldown status (if `check_cooldown`, uses positions::is_token_in_cooldown)

2. **DexScreener Filters** (if `dexscreener.enabled`)
   - Data overlay: Fetch DexScreener data if token from other source
   - Token info: name, symbol, logo, website (optional checks)
   - Transactions: 5m and 1h thresholds
   - Liquidity: min/max USD range
   - Market cap: min/max USD range
   - Volume: 24h, 5m, 1h, 6h thresholds
   - FDV: fully diluted value checks
   - Price changes: 5m, 1h, 6h, 24h ranges

3. **GeckoTerminal Filters** (if `geckoterminal.enabled`)
   - Data overlay: Fetch GeckoTerminal data if token from other source
   - Liquidity: min/max range
   - Market cap: min/max range
   - Volume: 5m, 1h, 24h thresholds
   - Price changes: 5m, 1h, 24h ranges
   - Pool count: min/max pools
   - Reserve: minimum reserve amount

4. **Rugcheck Filters** (if `rugcheck.enabled`)
   - Security score: minimum threshold
   - Risk level: reject if "danger" level
   - Authorities: check mint/freeze authority (allow/block lists)
   - Top holders: max single holder % and top 3 holders %
   - Total holders: minimum unique holder count
   - Insider holdings: graph insiders detection and percentages
   - Creator balance: maximum creator holding %
   - Transfer fees: presence check and maximum fee %
   - LP providers: minimum provider count
   - LP lock: minimum lock percentage

**Data Source Overlay System:**

Tokens can have data from multiple sources. Filtering fetches missing data on-demand:

```
[Token with DataSource::DexScreener]
    │
    ├─ DexScreener filter? → Use token data directly
    ├─ GeckoTerminal filter? → Fetch overlay via get_full_token_for_source_async()
    └─ Rugcheck filter? → Check if security data present, otherwise reject
```

**Storage Handoff to Tokens Module:**

After computing snapshot, filtering stores results in tokens module:

```
[compute_snapshot] → Build FilteredTokenLists
                          │
                          ▼
        FilteredTokenLists {
            passed: Vec<String>,         // Mints that passed all filters
            rejected: Vec<String>,        // Mints that failed one or more filters
            blacklisted: Vec<String>,     // Permanently blacklisted mints
            with_pool_price: Vec<String>, // Mints with pool pricing data
            open_positions: Vec<String>,  // Mints with active positions
            updated_at: DateTime<Utc>,    // Snapshot timestamp
        }
                          │
                          ▼
        tokens::store_filtered_results(lists)
                          │
                          ▼
        [Tokens Filtered Store] (Global RwLock)
```

- Snapshot recomputation now reuses the previous `FilteringSnapshot`, keeping
  `PassedToken.passed_time` / `RejectedToken.rejection_time` stable until a
  token actually transitions.
- When market APIs return `blockchain_created_at` as `0`, filtering falls back to
  the discovery timestamp so the **Recent** view continues to surface new pools.

**Consumer Access:**

Pool Service and other consumers use tokens module (NOT filtering module):

```
[Pool Discovery] → tokens::get_passed_tokens()
                          │
                          ▼
                   Returns Vec<String> (passed mints)
                          │
                          ▼
            Used for monitoring and price tracking
```

**Query System:**

Filtering provides query API for dashboard with 8 views:

- **Pool**: Tokens with pool price (has_pool_price=true)
- **All**: All tokens (DB query, bypasses snapshot for performance)
- **Passed**: Tokens that passed filters
- **Rejected**: Tokens rejected by filters (with reasons)
- **Blacklisted**: Permanently blacklisted tokens
- **Positions**: Tokens with open positions
- **Recent**: Tokens created in last 24h
- **NoMarketData**: Tokens with no market API data (DB query)

Query supports filters (liquidity, volume, risk score, search) and sorting (14 keys including symbol, price, liquidity, volume, market cap, etc.)

**Key Implementation Details:**

1. **Non-Blocking Startup**: First refresh async, doesn't block service manager
2. **Concurrent Fetching**: 24 parallel token fetches during snapshot
3. **Overlay Fetching**: Missing source data fetched on-demand during filtering
4. **Rejection Tracking**: Last 1000 rejections kept with specific reason (MAX_DECISION_HISTORY)
5. **Target Stop**: Can stop early if `target_filtered_tokens` reached
6. **Stale Fallback**: Uses stale snapshot if refresh fails (90s threshold)
7. **DB Bypass**: "All" and "NoMarketData" views query DB directly for performance
8. **Storage Handoff**: Results stored in tokens module, NOT kept in filtering module
9. **Decision History**: Circular buffer for last 1000 passed/rejected tokens
10. **Health Monitoring**: Service health based on snapshot age (120s healthy threshold)

### 5. Trading Flow (Trader System - Core Orchestration)

The trader module serves as the main orchestrator for automated trading operations, coordinating entry monitoring, position monitoring, and system integration with sophisticated concurrency management and safety controls.

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         TRADER SERVICE                                   │
│                  (Priority 150, Highest Priority Service)                │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
        ┌──────────────────────────┼──────────────────────────┐
        ▼                           ▼                           ▼
┌──────────────┐          ┌──────────────┐          ┌──────────────┐
│   Entry      │          │  Position    │          │  Safety &    │
│  Monitor     │          │  Monitor     │          │  Control     │
│  (3s loop)   │          │  (2s loop)   │          │  Systems     │
└──────────────┘          └──────────────┘          └──────────────┘
```

#### Service Configuration & Dependencies

**Location:** `src/trader/service.rs` (Service trait implementation) + `src/trader/auto/` (monitor loops)

**Service Priority:** 150 (starts last, after all core systems ready)

**Dependencies (7 total):**

- positions: Position tracking and management
- pool_discovery: Token discovery from DEX APIs
- pool_fetcher: RPC account fetching for pools
- pool_calculator: Price calculation from pool data
- token_discovery: Token metadata and discovery
- token_monitoring: Token data updates
- filtering: Token filtering and eligibility

**Shutdown Mechanism:** Uses bridge pattern to convert ServiceManager's `Arc<Notify>` to internal `watch::Receiver<bool>` for monitor loops.

**TaskMonitor Integration:** Wraps `auto::start_auto_trading()` with TaskMonitor for metrics collection.

**Configuration Access:** All parameters loaded from `config.trader.*` via centralized config system (never hardcoded).

#### Entry Monitor Loop (Background Task)

**Function:** `auto::monitor_entries()` (in `src/trader/auto/entry_monitor.rs`)

```
[monitor_entries()] → Wait for Core Services Ready
                          │
                          ▼
                     Check if Trader Enabled (config.trader.enabled)
                          │
                          ▼
                     Get Available Tokens (pools::get_available_tokens())
                          │
                          ▼
                     Fetch Pool Prices (get_pool_price for each mint)
                          │
                          ▼
                     Parallel Token Processing (Limited by Semaphore)
                          │
        ┌─────────────────┼─────────────────┐
        ▼                 ▼                  ▼
[Token Check Task 1] [Token Check Task 2] ... [Token Check Task N]
        │                 │                  │
        ▼                 ▼                  ▼
    Validate Price → Check Cooldown → Entry Analysis
        │                 │                  │
        ▼                 ▼                  ▼
    Check Position Exists → Strategy Evaluation
        │                 │                  │
        ▼                 ▼                  ▼
    Approved? → Debug Force Buy? → Open Position
        │                 │                  │
        ▼                 ▼                  ▼
    positions::open_position_direct(mint)
        │                 │                  │
        ▼                 ▼                  ▼
    Add to OHLCV Monitoring (Priority::Critical)
        │                 │                  │
        └─────────────────┴──────────────────┘
                          │
                          ▼
                     Wait for Interval (3s)
                          │
                          ▼
                     Loop Back to Start
```

**Key Configuration Options:**

- `trader.enabled`: Master on/off switch (hot-reloadable)
- `trader.entry_check_concurrency`: Parallel token checks (default: varies by config)
- `trader.trade_size_sol`: Position size in SOL
- `trader.max_open_positions`: Maximum concurrent positions (enforced by global semaphore)
- `trader.position_close_cooldown_minutes`: Re-entry cooldown after closing position

**Critical Implementation Details:**

1. **Concurrency Control**: Semaphore limits parallel entry checks to prevent overwhelming services
2. **Task Timeouts**:
   - Individual token check: 20s (TOKEN_CHECK_TASK_TIMEOUT_SECS)
   - Semaphore acquire: 60s (SEMAPHORE_ACQUIRE_TIMEOUT_SECS)
   - Collection overall: 30s (TOKEN_CHECK_COLLECTION_TIMEOUT_SECS)
3. **Shutdown Handling**: Sticky shutdown future with responsive checks every 10-100ms
4. **Price Validation**: Checks price > 0 and is_finite() before processing
5. **Cooldown Filter**: Fetches recently closed positions from DB, excludes from entry checks
6. **Duplicate Prevention**: Triple guard system (in-memory + DB + pending-open check)
7. **Token Tracking**: In-memory tracking of check history, price drops, entry attempts
8. **Critical Operation Guards**: RAII guards prevent shutdown during buy operations
9. **Position Blocking**: Skips entry if position already open or pending unverified
10. **OHLCV Integration**: Adds token to OHLCV monitoring with Critical priority after opening position
11. **Capacity Guard**: If the global position semaphore denies a permit mid-flight, the executor tags the result with `position_slots_unavailable` so the monitor logs an info event instead of an error and skips swap execution.
12. **Persistence Backoff**: Successful swaps now persist positions with exponential backoff (5 attempts, max 3s per wait) and trigger a fatal halt if SQLite writes never succeed, preventing untracked on-chain entries.

**Debug Modes (DISABLED by default):**

- `DEBUG_FORCE_BUY_MODE`: Auto-buy on price drops ≥3% (testing only)
- `DEBUG_FORCE_SELL_MODE`: Auto-sell positions after 45s (testing only)

**Loop Timing:**

- Interval: 3 seconds (ENTRY_MONITOR_INTERVAL_SECS)
- Minimum wait: 100ms (ENTRY_CYCLE_MIN_WAIT_MS) if cycle runs long

#### Position Monitor Loop (Background Task)

**Function:** `auto::monitor_positions()` (in `src/trader/auto/exit_monitor.rs`)

```
[monitor_positions()] → Wait for Core Services Ready
                          │
                          ▼
                     Check if Trader Enabled
                          │
                          ▼
                     Get Open Positions (positions::get_open_positions())
                          │
                          ▼
                     Filter to Verified-Entry Only (skip unverified)
                          │
                          ▼
                     Parallel Price Fetching (get_pool_price for all)
                          │
                          ▼
                     Update Position Prices & Tracking
                          │
        ┌─────────────────┼─────────────────┐
        ▼                 ▼                  ▼
[Exit Strategy]   [DCA Evaluation]    [Hold Position]
        │                 │                  │
        ▼                 ▼                  ▼
    Calculate P&L → Check Exit Conditions → Update Tracking
        │                 │                  │
        ▼                 ▼                  ▼
    Trailing Stop → ROI Target → Time Override
        │                 │                  │
        ▼                 ▼                  ▼
    Exit Approved? → Execute Trade → Success/Retry
        │                 │                  │
        ▼                 ▼                  ▼
    positions::close_position_direct() or add_to_position()
        │                 │                  │
        └─────────────────┴──────────────────┘
                          │
                          ▼
                     Wait for Interval (2s)
                          │
                          ▼
                     Loop Back to Start
```

**Key Configuration Options:**

- `trader.min_profit_threshold_enabled`: Enable minimum profit gate (default: false)
- `trader.min_profit_threshold_percent`: Minimum P&L% to allow exit (default: 5%)
- `trader.time_override_duration_hours`: Hours before time override applies (default: 2h)
- `trader.time_override_loss_threshold_percent`: Loss% threshold for override (default: -10%)

**Sell Decision Cache System:**

The trader uses a sophisticated caching system for sell decisions with dynamic retry scheduling:

```
[SellDecisionInfo Structure]
  - position_id: Unique identifier
  - mint: Token mint address
  - symbol: Token symbol
  - decision_reason: Why sell was triggered
  - decision_time: When decision was made
  - attempt_count: Number of sell attempts
  - next_retry_time: When to retry next
  - max_retries: 15 (normal) or 20 (emergency)
  - is_emergency_sell: High priority flag
  - last_error: Last failure reason

[Dynamic Retry Strategy]
  Phase 1 (Attempts 1-10): FAST PHASE
    - 5-10 seconds per attempt (randomized)
    - Quick retries for transient failures

  Phase 2 (Attempts 11+): DYNAMIC BACKOFF
    - Normal sells: 30s → 300s progressive backoff
    - Emergency sells: 15s → 120s progressive backoff
    - Randomization: ±20% to avoid timing patterns
    - Progressive scaling over 8 backoff attempts

[Cache Management]
  - Entry: cache_sell_decision() - stores new sell decisions
  - Retry: get_positions_ready_for_sell_retry() - checks can_retry()
  - Success: remove_sell_decision() - removes from cache
  - Failure: mark_sell_attempt_failed() - updates retry timing
  - Cleanup: cleanup_stale_sell_decisions() - removes old entries
  - Staleness: Normal 30min, Emergency 60min
```

**Critical Implementation Details:**

1. **Verified-Entry Filter**: Only monitors positions with `transaction_entry_verified = true`
2. **Parallel Price Fetching**: Fetches all position prices concurrently using futures::join_all
3. **Price Tracking Updates**: Updates in-memory tracking (no DB write) for performance
4. **P&L Calculation**: Uses calculate_position_pnl() for consistent calculations
5. **Exit Decision Flow**:
   - Debug force sell (if enabled) → profit::should_sell() → Min threshold check → Time override
6. **Minimum Profit Threshold**: Optional gate that blocks exits below configured profit %
7. **Time Override**: Allows exits for old positions (≥2h) with significant losses (≤-10%)
8. **Sell Concurrency**: Semaphore capacity driven by `trader.sell_concurrency` (default 5)
9. **Critical Operation Guards**: Prevents shutdown during sell operations (RAII pattern)
10. **Pool Availability Check**: Verifies pool price available before caching sell decision
11. **Retry Prevention**: Checks for existing cached decision before creating new one
12. **Failed Sell Tracking**: Marks attempts failed with error details for debugging
13. **Emergency Prioritization**: Stop loss (≤-20%) and high profit (≥50%) are emergency sells
14. **Timeout Handling**:
    - Individual sell operation: 600s (SELL_OPERATION_SMART_TIMEOUT_SECS)
    - Semaphore acquire: 30s (SELL_SEMAPHORE_ACQUIRE_TIMEOUT_SECS)
    - Collection overall: 240s (SELL_OPERATIONS_COLLECTION_TIMEOUT_SECS)

**Loop Timing:**

- Interval: 2 seconds (POSITION_MONITOR_INTERVAL_SECS)

#### Entry Logic Integration (entry.rs)

The entry monitor delegates entry decisions to the `entry` module:

```
[entry::should_buy(price_result)] → Conservative Drop Detection
                          │
                          ▼
                     Fetch Price History (pools::get_price_history)
                          │
                          ▼
                     Quality Checks (check_price_history_quality)
                          │
                          ▼
                     Recent Exit Price Check (avoid re-entry at same level)
                          │
                          ▼
                     ATH Prevention (multi-timeframe analysis)
                          │
                          ▼
                     Drop Detection (30s-10min windows)
                          │
                          ▼
                     Confidence Scoring (database-driven with stability weighting)
                          │
                          ▼
                     Return (approved: bool, confidence: f64, reason: String)
```

**Key Features:**

- Conservative 30s-10min detection windows (balanced approach)
- ATH prevention using multi-timeframe OHLCV analysis
- Database-driven confidence scoring with stability weighting
- Higher confidence thresholds for quality entries
- Optimized for 15-35% profit targets
- Cache-based recent exit price checks (30s TTL)
- Lightweight liquidity snapshots for trend inference

#### Exit Logic Integration (profit.rs)

The position monitor delegates exit decisions to the `profit` module:

```
[profit::should_sell(position, current_price)] → Streamlined Exit Decision
                          │
                          ▼
                     Calculate P&L (calculate_position_pnl)
                          │
                          ▼
                     ATH Context Analysis (fetch_ath_context)
                          │
        ┌─────────────────┼─────────────────┐
        ▼                 ▼                  ▼
[Recent High Proximity] [Time Pressure] [Trailing Stop]
        │                 │                  │
        ▼                 ▼                  ▼
    OHLCV 15m/1h/6h → Position Age → Dynamic Trailing Gap
        │                 │                  │
        ▼                 ▼                  ▼
    Tighten Trailing → Aggressive Exit → Exit Score
        │                 │                  │
        └─────────────────┴──────────────────┘
                          │
                          ▼
                     Composite Exit Decision
                          │
                          ▼
                     Return (should_sell: bool)
```

**Key Features:**

- ATH (All-Time High) proximity adaptation using 1m OHLCV data
- Recent high detection: 15m / 1h / 6h lookback windows
- Proximity classification: Extreme (≤1.5%), High (≤3%), Elevated (≤5%)
- Trailing stop tightening near recent highs (30% tighter at extreme)
- Time pressure behavior: More aggressive as position nears max hold time
- Max profit target reduction near highs (25% reduction at extreme proximity)
- Exit score nudging for decision bias near recent highs
- Numeric stability and logging clarity
- Cache-based ATH fetching (20s TTL per mint)

#### Safety & Control Systems

**1. Critical Operation Protection:**

```
[CriticalOperationGuard (RAII)] → Increment CRITICAL_OPERATIONS_IN_PROGRESS
                                    │
                                    ▼
                              Execute Buy/Sell Operation
                                    │
                                    ▼
                              Auto-decrement on Drop
                                    │
                                    ▼
                              Prevent Shutdown During Operation
```

**2. Global Position Semaphore:**

- Enforces `max_open_positions` limit atomically
- Acquired BEFORE swap execution (prevents race conditions)
- Permit "forgotten" (not dropped) after successful position creation
- Consumed for position lifetime until close
- Released on position close or synthetic exit

**3. Token Check Tracking:**

- In-memory tracking of check history per token
- Tracks: last_check_time, last_price, check_count, entry_check_count
- Prioritizes tokens with recent drops or stale checks
- Cleanup: Removes entries older than 10 minutes every ~10 calls

**4. Cooldown System:**

- Per-token re-entry cooldown after closing position
- Separate from global position cooldown (5s between any opens)
- Cached with 60s TTL, loaded from DB closed positions
- Excludes tokens within cooldown window from entry checks

**5. Shutdown Handling:**

- Sticky shutdown future (Box::pin) avoids missing notifications
- Responsive checks every 10-100ms throughout processing
- Waits up to 30s for critical operations to complete
- Graceful loop termination with cleanup

**6. Force Stop System (Emergency Trading Halt):**

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         FORCE STOP SYSTEM                               │
│               (Global Emergency Trading Halt)                           │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
        ┌──────────────────────────┼──────────────────────────┐
        ▼                           ▼                           ▼
┌──────────────┐          ┌──────────────┐          ┌──────────────┐
│  Global State │          │  Checked By  │          │   API        │
│  FORCE_STOPPED│          │  All Trading │          │  Endpoints   │
│  (AtomicBool) │          │  Components  │          │              │
└──────────────┘          └──────────────┘          └──────────────┘
```

**Location:** `src/global.rs` (state) + `src/trader/safety/` (checks) + `src/webserver/routes/trader.rs` (API)

**Global State:**

- `FORCE_STOPPED: AtomicBool` - Global flag in `src/global.rs`
- `is_force_stopped()` - Check current state
- `set_force_stopped(bool)` - Set state (called by API)

**Checked By (Returns Early When Active):**

- Entry monitor (`src/trader/auto/entry_monitor.rs`)
- Exit monitor (`src/trader/auto/exit_monitor.rs`)
- Entry evaluator (`src/trader/evaluators/entry.rs`)
- Exit evaluator (`src/trader/evaluators/exit.rs`)
- Swap execution (`src/swaps/operations.rs`)
- Manual trading handlers (`src/webserver/routes/trader.rs`)

**API Endpoints:**

- `POST /api/trader/force-stop` - Activate emergency stop
- `POST /api/trader/resume` - Clear force stop flag
- `GET /api/trader/force-stop/status` - Check current state

**Behavior:**

- When active: All trading operations return early, swaps blocked, manual trades blocked
- Resume: Clears flag but does NOT auto-enable trader (master switch unchanged)
- Independent of trader.enabled config (works even when trader enabled)
- Immediate effect (no restart required)

**7. Independent Monitor Control:**

```
┌─────────────────────────────────────────────────────────────────────────┐
│                    MONITOR CONTROL SYSTEM                               │
│           (Independent Entry/Exit Monitor Toggles)                      │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
        ┌──────────────────────────┼──────────────────────────┐
        ▼                           ▼                           ▼
┌──────────────┐          ┌──────────────┐          ┌──────────────┐
│  Entry       │          │  Exit        │          │  Master      │
│  Monitor     │          │  Monitor     │          │  Switch      │
│  Toggle      │          │  Toggle      │          │  (enabled)   │
└──────────────┘          └──────────────┘          └──────────────┘
        │                           │                           │
        ▼                           ▼                           ▼
    entry_monitor_enabled     exit_monitor_enabled     trader.enabled
        │                           │                           │
        └───────────────────────────┴───────────────────────────┘
                                    │
                                    ▼
                          Combined Check: master AND individual
```

**Location:** `src/config/schemas/trader.rs` (config) + `src/trader/` (accessors)

**Config Fields (in `[trader]` section):**

- `entry_monitor_enabled: bool` - Enable/disable entry monitoring (default: true)
- `exit_monitor_enabled: bool` - Enable/disable exit monitoring (default: true)

**Accessor Functions:**

- `is_entry_monitor_enabled()` - Returns `trader.enabled AND entry_monitor_enabled`
- `is_exit_monitor_enabled()` - Returns `trader.enabled AND exit_monitor_enabled`

**API Endpoints:**

- `GET /api/trader/monitors/status` - Get both monitor states
- `POST /api/trader/monitors/entry/toggle` - Toggle entry monitor
- `POST /api/trader/monitors/exit/toggle` - Toggle exit monitor

**Use Cases:**

- Disable entry monitoring while keeping exit active (stop buying, let positions exit)
- Disable exit monitoring to hold positions (diamond hands mode)
- Independent control without affecting master trader switch

**8. Loss Limit System (Drawdown Protection):**

```
┌─────────────────────────────────────────────────────────────────────────┐
│                        LOSS LIMIT SYSTEM                                │
│              (Period-Based Drawdown Protection)                         │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
        ┌──────────────────────────┼──────────────────────────┐
        ▼                           ▼                           ▼
┌──────────────┐          ┌──────────────┐          ┌──────────────┐
│  Config      │          │  State       │          │  Behavior    │
│  Parameters  │          │  Tracking    │          │  Effects     │
└──────────────┘          └──────────────┘          └──────────────┘
        │                           │                           │
        ▼                           ▼                           ▼
┌──────────────┐          ┌──────────────┐          ┌──────────────┐
│loss_limit_sol│          │period_start  │          │Entry Monitor │
│period_hours  │          │cumulative_   │          │ PAUSED       │
│auto_resume   │          │loss_sol      │          │              │
│enabled       │          │is_limited    │          │Exit Monitor  │
└──────────────┘          └──────────────┘          │ CONTINUES    │
                                                    └──────────────┘
```

**Location:** `src/trader/safety/loss_limit.rs`

**Config Fields (in `[trader]` section):**

- `loss_limit_enabled: bool` - Enable loss limit system (default: false)
- `loss_limit_sol: f64` - Maximum cumulative loss in SOL for period (default: 1.0)
- `loss_limit_period_hours: u64` - Rolling period window in hours (default: 24)
- `loss_limit_auto_resume: bool` - Auto-resume when period resets (default: true)

**State Structure (LossLimitState):**

```rust
struct LossLimitState {
    period_start: Instant,         // Start of current tracking period
    cumulative_loss_sol: f64,      // Total realized losses this period
    is_limited: bool,              // Currently in limited state
}
```

**Core Functions:**

- `is_entry_blocked_by_loss_limit()` - Called by entry evaluator before entry checks
- `record_realized_loss(loss_sol: f64)` - Called when position closes with loss
- `get_loss_limit_status()` - Returns status for dashboard display
- `initialize_from_history()` - Uses `get_period_trading_stats()` on startup to restore state

**Flow:**

```
[Position Closes with Loss] → record_realized_loss(loss_sol)
                                    │
                                    ▼
                           Add to cumulative_loss_sol
                                    │
                                    ▼
                           cumulative >= loss_limit_sol?
                                    │
                    ┌───────────────┴───────────────┐
                    ▼                               ▼
               [YES: Limit]                    [NO: Continue]
                    │
                    ▼
               Set is_limited = true
                    │
                    ▼
               Entry monitor blocked
               Exit monitor continues
                    │
                    ▼
    [Period Expires OR Manual Resume OR Auto Resume]
                    │
                    ▼
               Reset state, resume entries
```

**API Endpoints:**

- `GET /api/trader/loss-limit/status` - Get current loss limit state
- `POST /api/trader/loss-limit/resume` - Manually resume (clear is_limited)
- `POST /api/trader/loss-limit/reset` - Reset period and cumulative loss

**Key Behavior:**

- Only pauses entry monitor (no new positions)
- Exit monitor continues (positions can still close)
- Period-based: resets when period_hours elapsed
- Auto-resume: If enabled, automatically clears is_limited when period resets
- Startup recovery: Loads realized losses from DB for current period
- Manual override: Can resume or reset via API anytime

#### Runtime Control Functions

**Start Trader:**

```rust
start_trader() → Check if Already Running
                    │
                    ▼
               Set config.trader.enabled = true
                    │
                    ▼
               Log "Trader operations enabled"
```

**Stop Trader:**

```rust
stop_trader_gracefully() → Check if Already Stopped
                    │
                    ▼
               Set config.trader.enabled = false
                    │
                    ▼
               Wait for Critical Operations (max 30s)
                    │
                    ▼
               Log "Trader operations disabled"
```

**Check Status:**

```rust
is_trader_running() → Return config.trader.enabled
```

#### Integration Points

**1. Pools Module Integration:**

```
[Trader] → pools::get_available_tokens()
        → pools::get_pool_price(mint)
        → pools::get_price_history(mint)
        → pools::check_price_history_quality()
```

**2. Positions Module Integration:**

```
[Trader] → positions::open_position_direct(mint)
        → positions::close_position_direct(mint, reason)
        → positions::get_open_positions()
        → positions::get_open_mints()
        → positions::is_open_position(mint)
        → positions::is_token_in_cooldown(mint)
        → positions::update_position_tracking(mint, price, price_result)
        → positions::calculate_position_pnl(position, current_price)
```

**3. Tokens Module Integration:**

```
[Trader] → tokens::get_full_token_async(mint)
```

**4. Entry Module Integration:**

```
[Trader] → entry::should_buy(price_result)
        → entry::get_profit_target()
```

**5. Profit Module Integration:**

```
[Trader] → profit::should_sell(position, current_price)
```

**6. OHLCV Module Integration:**

```
[Trader] → ohlcvs::add_token_monitoring(mint, Priority::Critical)
        → ohlcvs::record_activity(mint, ActivityType::PositionOpened)
        → ohlcvs::get_ohlcv_data() (via profit module)
```

**7. Config Module Integration:**

```
[Trader] → with_config(|cfg| cfg.trader.*)
        → update_config_section() (for start/stop)
```

**8. Events Module Integration:**

```
[Trader] → Via positions module (entry/exit events)
```

#### Timing Constants (Hardcoded for Optimal Performance)

All timing constants are hardcoded in `src/trader.rs` for optimal trader performance:

- **ENTRY_MONITOR_INTERVAL_SECS**: 3s (entry loop cycle)
- **POSITION_MONITOR_INTERVAL_SECS**: 2s (position loop cycle)
- **SEMAPHORE_ACQUIRE_TIMEOUT_SECS**: 60s (max wait for entry semaphore)
- **TOKEN_CHECK_TASK_TIMEOUT_SECS**: 20s (individual token check)
- **TOKEN_CHECK_COLLECTION_TIMEOUT_SECS**: 30s (all token checks)
- **SELL_OPERATIONS_COLLECTION_TIMEOUT_SECS**: 240s (all sell operations)
- **SELL_OPERATION_SMART_TIMEOUT_SECS**: 600s (individual sell operation)
- **ENTRY_CYCLE_MIN_WAIT_MS**: 100ms (minimum wait between cycles)
- Various shutdown check intervals: 10-100ms (responsive shutdown)

#### Key Implementation Notes

1. **Service Priority**: 150 (highest, starts last after all dependencies ready)
2. **Background Tasks**: 2 instrumented tasks (entry monitor, position monitor)
3. **Startup Wait**: MUST wait for `are_core_services_ready()` before operations
4. **Hot Reload**: Trader can be enabled/disabled via config without restart
5. **Concurrency**: Semaphore-based limiting for both entry checks and sell operations
6. **Timeout Handling**: Comprehensive timeouts for all operations (20s-600s range)
7. **Critical Operations**: RAII guards prevent shutdown during trades
8. **Shutdown**: Sticky future with responsive checks (10-100ms intervals)
9. **Retry System**: Dynamic retry scheduling with phase-based backoff strategy
10. **Cache Management**: Sell decision cache with staleness detection and cleanup
11. **Emergency Prioritization**: Different retry schedules for emergency sells
12. **Position Blocking**: Multiple guards prevent duplicate position opens
13. **Price Validation**: Strict validation (> 0, is_finite) before processing
14. **Cooldown Enforcement**: Per-token re-entry cooldown after close
15. **Token Tracking**: In-memory tracking for intelligent prioritization

### 6. Position Management Flow (Comprehensive State Machine & Verification)

The positions module manages the complete lifecycle of trading positions with a sophisticated verification system, state machine transitions, and automatic reconciliation.

```
┌─────────────────────────────────────────────────────────────────────────┐
│                    POSITIONS SERVICE ARCHITECTURE                       │
│                      (Priority 50, No Dependencies)                     │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
        ┌───────────────────────────┼───────────────────────────┐
        ▼                           ▼                           ▼
┌──────────────┐          ┌──────────────┐          ┌──────────────┐
│   Worker     │          │    State     │          │  Database    │
│  (Verif.     │          │  (In-Memory  │          │  (positions. │
│   Loop)      │          │   + Indexes) │          │   db)        │
└──────────────┘          └──────────────┘          └──────────────┘
        │                           │                           │
        ▼                           ▼                           ▼
┌──────────────┐          ┌──────────────┐          ┌──────────────┐
│  Verifier    │          │  Operations  │          │  Transitions │
│  (Chain      │          │  (Open/Close │          │  (State      │
│   Checks)    │          │   Entry)     │          │   Machine)   │
└──────────────┘          └──────────────┘          └──────────────┘
        │                           │                           │
        └───────────────────────────┴───────────────────────────┘
                                    │
                          ┌─────────┴─────────┐
                          ▼                   ▼
                    Queue System        Tracking System
                    (Exponential        (Price Updates,
                     Backoff)            Highs/Lows)
```

#### Service Startup & Initialization Flow

```
[Service Start] → initialize_positions_system()
                          │
                          ▼
                  Initialize Database (positions.db)
                  - positions table (core position data)
                  - position_states table (state history)
                  - position_tracking table (price tracking)
                  - position_metadata table (system metadata)
                  - token_snapshots table (market data at open/close)
                  - 14 performance indexes
                          │
                          ▼
                  Load All Positions from Database
                          │
        ┌─────────────────┼─────────────────┬──────────────────┐
        ▼                 ▼                 ▼                  ▼
   Populate         Rebuild Indexes    Enqueue Unverified   Initialize
   POSITIONS        - SIG_TO_MINT      Transactions         Semaphore
   (RwLock<Vec>)    - MINT_TO_POS      (Entry/Exit)         (Max Open)
        │                 │                 │                  │
        └─────────────────┴─────────────────┴──────────────────┘
                                    │
                                    ▼
                  Reconcile Global Position Semaphore
                  (Consume permits for existing open positions)
                          │
                          ▼
                  Set POSITIONS_SYSTEM_READY flag
                          │
                          ▼
                  Start Verification Worker
                  (Wait for TRANSACTIONS_SYSTEM_READY + POOL_SERVICE_READY)
```

#### Verification Worker Loop (Background Task)

```
[Verification Worker] → Wait for Dependencies
                          │
                          ▼
                  Check TRANSACTIONS_SYSTEM_READY
                  Check POOL_SERVICE_READY
                          │
                          ▼
                  Set POSITIONS_SYSTEM_READY
                          │
                          ▼
                  Main Loop (Adaptive Sleep)
                          │
        ┌─────────────────┼─────────────────┬──────────────────┬──────────────┐
        ▼                 ▼                 ▼                  ▼              ▼
   Re-enqueue       Process Batch     Cleanup Expired    Emit Summary   Shutdown
   Missing          (10 items)        Items              (Every 30s)    (Graceful)
   Verifications                      (Age/Height)
        │                 │                 │                  │              │
        ▼                 ▼                 ▼                  │              │
   Scan All         Verify Each       Remove Expired           │              │
   Positions        Transaction       + Remove Orphans         │              │
   Check Queue      (RPC + Chain)     (Entry failures)         │              │
        │                 │                 │                  │              │
        └─────────────────┴─────────────────┴──────────────────┘              │
                                    │                                         │
                                    ▼                                         │
                  Apply Transitions (State Machine)                           │
                  - EntryVerified                                             │
                  - ExitVerified                                              │
                  - ExitFailedClearForRetry                                   │
                  - ExitPermanentFailureSynthetic                             │
                  - RemoveOrphanEntry                                         │
                  - UpdatePriceTracking                                       │
                                    │                                         │
                                    ▼                                         │
                  Update Database + Emit Events                               │
                  (Events recorded for all state changes)                     │
                                    │◄────────────────────────────────────────┘
                                    ▼
                  Adaptive Sleep (based on queue size):
                  - First cycle: 3s
                  - Queue > 50: 500ms
                  - Queue > 0: 2s
                  - Queue empty: 5s
```

#### Position Opening Flow (Entry)

```
[open_position_direct(mint)] → Get Token & Price Data
                          │
                          ▼
                  GET PRICE (with API Fallback)
                  - Priority 1: Pool price (real-time on-chain)
                  - Priority 2: Token API price (DexScreener/GeckoTerminal)
                    if fresh (< 60s) and no pool price available
                  - This enables trading for tokens not yet
                    tracked by pool service
                          │
                          ▼
                  GUARD: Acquire Global Position Permit
                  (Enforces max_open_positions atomically)
                  - Try acquire from semaphore
                  - Fail if no slots available
                  - Permit "forgotten" after position created
                          │
                          ▼
                  GUARD: Acquire Per-Mint Lock
                  (Serialize opens for same token)
                          │
                          ▼
                  GUARD: Check In-Memory State
                  (Prevent duplicate opens)
                  - is_open_position(&mint)
                          │
                          ▼
                  GUARD: Check Database
                  (Catch edge cases across restarts)
                  - get_position_by_mint(&mint)
                  - Check exit_time.is_none()
                  - Check !transaction_exit_verified
                          │
                          ▼
                  GUARD: Global Cooldown Check
                  (Prevent rapid successive opens)
                  - LAST_OPEN_TIME vs cooldown_secs
                          │
                          ▼
                  DRY-RUN Check
                  (Skip swap if enabled)
                          │
                          ▼
                  Mark Pending Open (TTL: 120s)
                  (Prevent duplicate swap attempts)
                          │
                          ▼
                  Get Best Quote for Opening
                  (Multi-DEX: GMGN, Jupiter)
                          │
                          ▼
                  Execute Swap (SOL → Token)
                          │
                          ▼
                  Create Position Object
                  - Initial state: Open
                  - entry_transaction_signature set
                  - transaction_entry_verified = false
                  - Semaphore permit "forgotten" (consumed)
                          │
                          ▼
                  Save to Database (positions table)
                          │
                          ▼
                  Add to In-Memory State (POSITIONS)
                  - Update indexes (SIG_TO_MINT, MINT_TO_POS)
                  - Clear pending-open flag
                  - Update LAST_OPEN_TIME
                          │
                          ▼
                  Enqueue Entry Verification
                  - VerificationKind::Entry
                  - Expiry: current_height + 150 slots
                          │
                          ▼
                  Capture Token Snapshot (opening)
                  (Market data at entry time)
                          │
                          ▼
                  Record Entry Event
                  (Events system integration)
```

#### Position Closing Flow (Exit)

```
[close_position_direct(mint)] → Acquire Position Lock
                          │
                          ▼
                  Get Position from State
                  - Check exists
                  - Check is_open (no exit_time)
                  - Check not already verified closed
                          │
                          ▼
                  Get Current Market Price
                  (Pool service integration)
                          │
                          ▼
                  DRY-RUN Check
                  (Skip swap if enabled)
                          │
                          ▼
                  Get Best Quote for Closing
                  (Multi-DEX routing with proceeds-first logic)
                  - Compare GMGN vs Jupiter
                  - Metrics tracking (shortfall, rejection)
                          │
                          ▼
                  Execute Swap (Token → SOL)
                          │
                          ▼
                  Update Position State
                  - Set exit_transaction_signature
                  - Set exit_price (market price)
                  - transaction_exit_verified = false
                  - Update database
                          │
                          ▼
                  Enqueue Exit Verification
                  - VerificationKind::Exit
                  - Expiry: current_height + 150 slots
                          │
                          ▼
                  Capture Token Snapshot (closing)
                  (Market data at exit time)
                          │
                          ▼
                  Record Exit Event
                  (Events system integration)
```

#### Partial Exit Flow (NEW - Systematic Support)

```
[partial_close_position(mint, %, reason)] → Acquire Position Lock
                          │
                          ▼
                  Get Position from State
                  - Check exists & is_open
                  - Check partial_exit_enabled in config
                          │
                          ▼
                  Validate Exit Percentage
                  - Check bounds (min_pct to max_pct)
                  - Calculate partial token amount
                          │
                          ▼
                  Get Current Market Price
                  (Pool service integration)
                          │
                          ▼
                  Get Best Quote for Partial Amount
                  (Multi-DEX routing)
                          │
                          ▼
                  Execute Swap (Tokens → SOL)
                  - Use calculate_partial_amount()
                  - Only sell specified percentage
                          │
                          ▼
                  Update Position State
                  - Set exit_transaction_signature
                  - DO NOT set exit_time (still open!)
                  - transaction_exit_verified = false
                          │
                          ▼
                  Apply PartialExitSubmitted Transition
                  - position_id, exit_signature
                  - exit_amount, exit_percentage
                  - market_price
                          │
                          ▼
                  Enqueue Partial Exit Verification
                  - VerificationItem::new_partial_exit()
                  - is_partial_exit = true
                  - expected_exit_amount = calculated amount
                  - Expiry: current_height + 150 slots
                          │
                          ▼
                  CRITICAL: Do NOT release semaphore permit
                  (Position remains open)
                          │
                          ▼
                  Record Partial Exit Event
                  (Events system integration)
```

**Partial Exit Verification:**

```
[verify_transaction(partial_exit_item)]
                          │
                          ▼
                  Calculate Exit Amount from Transaction
                  (Using token decimals)
                          │
                          ▼
                  Verify Expected Amount Match
                  - Tolerance: 0.1% or 10 units
                  - If mismatch: RetryTransient
                          │
                          ▼
                  Check Remaining Balance
                  (Informational - expect remaining tokens)
                          │
                          ▼
                  Apply PartialExitVerified Transition
                  - exit_amount (actual tokens sold)
                  - sol_received
                  - effective_exit_price
                  - fee_lamports
                  - exit_time
                          │
                          ▼
                  Update Position State
                  - remaining_token_amount -= exit_amount
                  - total_exited_amount += exit_amount
                  - average_exit_price (weighted average)
                  - partial_exit_count++
                  - transaction_exit_verified = true (for this exit)
                          │
                          ▼
                  Save Exit Record to History
                  - position_exits table
                  - Track all partial exits
                          │
                          ▼
                  CRITICAL: Do NOT release semaphore permit
                  (Position still open with remaining tokens)
```

#### DCA (Dollar Cost Averaging) Flow (NEW - Systematic Support)

```
[add_to_position(mint, dca_amount_sol)] → Get Position from State
                          │
                          ▼
                  Check DCA Configuration
                  - dca_enabled
                  - dca_max_count (limit per position)
                  - dca_cooldown_minutes
                          │
                          ▼
                  Validate DCA Eligibility
                  - position.dca_count < max_count
                  - Time since last_dca_time > cooldown
                          │
                          ▼
                  Get Current Market Price
                  (Pool service integration)
                          │
                          ▼
                  Get Best Quote for DCA Amount
                  (Multi-DEX routing)
                          │
                          ▼
                  Execute Swap (SOL → Tokens)
                  - Buy additional tokens at current price
                          │
                          ▼
                  Persist Pending DCA Swap (positions::state)
                  - Store signature, mint, position_id, expiry in metadata
                  - Ensures verification survives restarts
                          │
                          ▼
                  Apply DcaSubmitted Transition
                  - position_id, dca_signature
                  - dca_amount_sol
                  - market_price
                          │
                          ▼
                  Enqueue DCA Verification
                  - Dedicated VerificationItem::new_dca (flagged is_dca=true)
                  - Uses VerificationKind::Entry but DCA-aware logic
                  - Expiry: block_height + 150 slots (from persistence metadata)
                          │
                          ▼
                  CRITICAL: Do NOT consume new semaphore permit
                  (Same position, not a new position)
                          │
                          ▼
                  Record DCA Entry Event
                  (Events system integration)
```

**DCA Verification:**

```
[verify_transaction(dca_item)]
                          │
                          ▼
                  Get Swap Analysis
                  (From transactions module)
                          │
                          ▼
                  Extract DCA Amounts (swap-only data)
                  - No wallet balance reconciliation (prevents double counting)
                  - tokens_bought, sol_spent, effective_price, fees
                          │
                          ▼
                  Apply DcaVerified Transition
                  - tokens_bought
                  - sol_spent
                  - effective_price
                  - fee_lamports
                  - dca_time (block_time fallback → now)
                  - dca_signature (clears pending metadata)
                          │
                          ▼
                  Update Position State
                  - remaining_token_amount += tokens_bought
                  - total_size_sol += sol_spent
                  - average_entry_price (recalculate weighted avg)
                  - dca_count++
                  - last_dca_time = now
                  - transaction_entry_verified = true (for DCA)
                          │
                          ▼
                  Save Entry Record to History
                  - position_entries table
                  - is_dca = true
                  - Track all DCA entries
                  - Persisted even on retry (idempotent via signature)
                          │
                          ▼
                  Clear Pending DCA Metadata
                  - Removes signature from in-memory + metadata store
                  - Prevents duplicate rehydration
                          │
                          ▼
                  CRITICAL: Do NOT consume new semaphore permit
                  (Still same position, permit already consumed)

**DCA Failure / Timeout Handling:**
- Verification abandonment or expiry produces `PositionTransition::DcaFailed`.
- Pending metadata entry is cleared, enabling manual reconciliation without stale state.
- Worker rehydrate flow reloads pending DCA swaps from metadata into the verification queue on startup.
```

**DCA Opportunity Detection (Trader Module):**

```
[process_dca_opportunities()]
                          │
                          ▼
                  Check dca_enabled in config
                          │
                          ▼
                  Get All Open Positions
                          │
                          ▼
                  For Each Position:
                          │
        ┌─────────────────┼─────────────────┬──────────────────┐
        ▼                 ▼                 ▼                  ▼
   Check DCA        Check DCA          Calculate P&L      Check DCA
   Count Limit      Cooldown           vs Entry Price     Threshold
   dca_count <      last_dca_time      (use average_      pnl_pct <
   max_count        + cooldown         entry_price)       threshold
        │                 │                 │                  │
        └─────────────────┴─────────────────┴──────────────────┘
                                    │
                                    ▼
                  Create TradeDecision
                  - action: TradeAction::DCA
                  - reason: TradeReason::DCAScheduled
                  - size_sol: initial_size * dca_size_percentage
                  - priority: Normal
                          │
                          ▼
                  Return DCA Opportunities
                  (For trader execution)
```

#### Transaction Verification Flow (Chain Validation)

```
[verify_transaction(item)] → Get Transaction from Cache/RPC
                          │
                          ▼
                  Check Transaction Success
                          │
        ┌─────────────────┼─────────────────┐
        ▼                                   ▼
   Success                            Failed
        │                                   │
        ▼                                   ▼
   Entry Verification              Check Error Type
        │                                   │
        ▼                           ┌───────┴───────┐
   Get Swap Analysis          ▼                   ▼
   (From transactions      [PERMANENT]      Transient
    module)                     │                │
        │                       ▼                ▼
        ▼                  Entry:           Retry
   Extract Amounts         RemoveOrphan     (Exponential
   - token_amount          Exit:            Backoff)
   - sol_spent             Synthetic
   - fees                  Close
        │
        ▼
   Create Transition:
   - EntryVerified
     (effective_entry_price, token_amount, fees, sol_size)
        │
        ▼
   Exit Verification
        │
        ▼
   Get Swap Analysis
   (Balance changes from transactions module)
        │
        ▼
   Extract Amounts
   - sol_received
   - fees
        │
        ▼
   Calculate Effective Exit Price
   (sol_received / token_amount)
        │
        ▼
   Create Transition:
   - ExitVerified
     (effective_exit_price, sol_received, fees, exit_time)
```

#### State Machine Transitions (Apply Logic)

**Standard Transitions:**

```
[apply_transition(transition)] → Match Transition Type
                          │
        ┌─────────────────┼─────────────────┬──────────────────┬──────────────┐
        ▼                 ▼                 ▼                  ▼              ▼
   EntryVerified    ExitVerified    ExitFailedClear   ExitPermanent    RemoveOrphan
        │                 │                 │                  │              │
        ▼                 ▼                 ▼                  ▼              ▼
   Update State     Update State     Clear Exit Sig    Set Synthetic    Remove from
   - Verified=true  - Verified=true  - Retry Pending   Exit             State
   - Effective$     - Effective$     - Clear Exit      - Mark Closed    - Delete DB
   - Token Amount   - SOL Received   - Update DB       - Update DB      - Remove Index
   - Fees           - Exit Time                        - Release Slot   - Release Slot
        │                 │                 │                  │              │
        ▼                 ▼                 ▼                  ▼              ▼
   Update DB        Process Loss     Update DB         Update DB        Record Event
   + Emit Event     Detection        + Emit Event      + Emit Event     (Orphan)
        │            (Blacklist?)          │                  │
        ▼                 │                 │                  │
   Record Event           ▼                 │                  │
   (Entry)           Update DB              │                  │
                     + Emit Event           │                  │
                     - Release Slot◄────────┴──────────────────┘
                          │
                          ▼
                     Record Event
                     (Exit + P&L)
```

**NEW: Partial Exit Transitions:**

```
[PartialExitSubmitted] → Update Position State
                          - exit_transaction_signature
                          - DO NOT set exit_time
                          - transaction_exit_verified = false
                          │
                          ▼
                     Update DB + Emit Event

[PartialExitVerified] → Update Position State
                          - remaining_token_amount -= exit_amount
                          - total_exited_amount += exit_amount
                          - average_exit_price (weighted avg)
                          - partial_exit_count++
                          - transaction_exit_verified = true
                          │
                          ▼
                     Save to position_exits History Table
                          │
                          ▼
                     CRITICAL: Do NOT release semaphore permit
                     (Position still open)
                          │
                          ▼
                     Update DB + Emit Event

[PartialExitFailed] → Clear exit_transaction_signature
                          │
                          ▼
                     Update DB + Emit Event
```

**NEW: DCA Transitions:**

```
[DcaSubmitted] → Update Position State
                          - Add to entry history
                          - transaction_entry_verified = false
                          │
                          ▼
                     Update DB + Emit Event

[DcaVerified] → Update Position State
                          - remaining_token_amount += tokens_bought
                          - total_size_sol += sol_spent
                          - average_entry_price (recalculate)
                          - dca_count++
                          - last_dca_time = now
                          - transaction_entry_verified = true
                          │
                          ▼
                     Save to position_entries History Table
                     (is_dca = true)
                          │
                          ▼
                     CRITICAL: Do NOT consume new semaphore permit
                     (Same position)
                          │
                          ▼
                     Update DB + Emit Event

[DcaFailed] → Log failure reason
                          │
                          ▼
                     Update DB + Emit Event
```

#### P&L Calculation (NEW - DCA & Partial Exit Support)

**Standard P&L Calculation (calculate_position_pnl):**

```
[calculate_position_pnl(position, current_price)]
                          │
                          ▼
                  Determine Entry Price
                  - Use average_entry_price (if > 0)
                  - Fallback: effective_entry_price or entry_price
                          │
                          ▼
                  Validate Entry Price (> 0 && finite)
                          │
        ┌─────────────────┼─────────────────┬──────────────────┐
        ▼                 ▼                 ▼                  ▼
   Closed w/         Open Position      Closing Pending   No Price
   sol_received      (has current)      (has current)     Available
        │                 │                 │                  │
        ▼                 ▼                 ▼                  ▼
   Use Actual        Use Remaining      Estimate Based    Return
   SOL Received      Token Amount       on Current        (0.0, 0.0)
   vs Invested       (partial exits)    Price
        │                 │                 │
        ▼                 ▼                 ▼
   sol_received     current_value      current_value
   - total_size_sol - total_size_sol   - total_size_sol
   - total_fees     - total_fees       - total_fees
        │                 │                 │
        └─────────────────┴─────────────────┘
                          │
                          ▼
                  Calculate net_pnl_sol and net_pnl_percent
                          │
                          ▼
                  Return (net_pnl_sol, net_pnl_percent)
```

**NEW: Split P&L Calculation (calculate_split_pnl):**

```
[calculate_split_pnl(position, current_price)]
                          │
                          ▼
                  Determine Entry Price
                  - Use average_entry_price (weighted avg)
                          │
                          ▼
                  Calculate Realized P&L
                  (From partial exits)
                          │
                          ▼
                  Check total_exited_amount > 0
                          │
                  ┌───────┴───────┐
                  ▼               ▼
              Has Exits       No Exits Yet
                  │               │
                  ▼               ▼
          Use average_     realized = 0.0
          exit_price
          Calculate:
          - sol_received
          - invested_in_exited
            (total_size_sol * exit_portion)
          - exit_fees
          realized = sol_received
                   - invested_in_exited
                   - exit_fees
                  │               │
                  └───────┬───────┘
                          ▼
                  Calculate Unrealized P&L
                  (From remaining holdings)
                          │
                  Check remaining_token_amount
                          │
                  ┌───────┴───────┐
                  ▼               ▼
           Has Remaining    Fully Exited
           Tokens           (or closed)
                  │               │
                  ▼               ▼
          Get current_price   unrealized = 0.0
          Calculate:
          - current_value
            (remaining * price)
          - invested_in_remaining
            (total_size_sol * remaining_portion)
          - entry_fees_portion
          unrealized = current_value
                     - invested_in_remaining
                     - entry_fees_portion
                  │               │
                  └───────┬───────┘
                          ▼
                  Calculate Totals
                  - total_pnl_sol = realized + unrealized
                  - total_pnl_percent = (total_pnl_sol / total_size_sol) * 100
                          │
                          ▼
                  Return (realized_pnl_sol,
                          unrealized_pnl_sol,
                          total_pnl_sol,
                          total_pnl_percent)
```

**Key P&L Improvements:**

- **average_entry_price**: Weighted average across initial entry + all DCA entries
- **average_exit_price**: Weighted average across all partial exits
- **total_size_sol**: Cumulative SOL invested (includes DCA additions)
- **remaining_token_amount**: Current holdings after partial exits
- **Realized vs Unrealized**: Split P&L for positions with partial exits

#### Price Tracking Updates (Real-Time)

```
[update_position_tracking(mint, price)] → Acquire Position Lock (100ms timeout)
                          │
                          ▼
                  Get Position from State
                          │
                          ▼
                  Update Price Metrics
                  - current_price
                  - price_highest (if new high)
                  - price_lowest (if new low)
                  - current_price_updated
                          │
                          ▼
                  Apply UpdatePriceTracking Transition
                  (Updates in-memory state and persists price fields in DB)
                          │
                          ▼
                  Log if New High/Low
```

#### Database Schema (positions.db)

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         DATABASE TABLES                                 │
└─────────────────────────────────────────────────────────────────────────┘
        │
        ├─ positions (Core Position Data - EXTENDED FOR PARTIAL EXIT & DCA)
        │  - id, mint, symbol, name
        │  - entry_price, entry_time, exit_price, exit_time
        │  - position_type ('buy' or 'sell')
        │  - entry_size_sol (initial SOL), total_size_sol (includes DCA)
        │  - price_highest, price_lowest
        │  - entry_transaction_signature, exit_transaction_signature
        │  - token_amount (initial amount), effective_entry_price, effective_exit_price
        │  - sol_received (actual from chain)
        │  - profit_target_min, profit_target_max, liquidity_tier
        │  - transaction_entry_verified, transaction_exit_verified
        │  - entry_fee_lamports, exit_fee_lamports
        │  - current_price, current_price_updated
        │  - phantom_confirmations, phantom_first_seen, synthetic_exit
        │  - closed_reason, created_at, updated_at
        │  **NEW FIELDS:**
        │  - remaining_token_amount (current holdings after partial exits)
        │  - total_exited_amount (cumulative tokens sold)
        │  - average_exit_price (weighted average exit price)
        │  - partial_exit_count (number of partial exits)
        │  - dca_count (number of DCA entries)
        │  - average_entry_price (weighted average entry price)
        │  - last_dca_time (for cooldown enforcement)
        │
        ├─ position_exits (NEW - Exit History for Partial Exits)
        │  - id, position_id, timestamp
        │  - amount (tokens sold)
        │  - price (exit price per token)
        │  - sol_received
        │  - transaction_signature
        │  - is_partial (true/false)
        │  - percentage (% of position sold)
        │  - fees_lamports
        │
        ├─ position_entries (NEW - Entry History for DCA)
        │  - id, position_id, timestamp
        │  - amount (tokens bought)
        │  - price (entry price per token)
        │  - sol_spent
        │  - transaction_signature
        │  - is_dca (true/false - initial entry vs DCA)
        │  - fees_lamports
        │
        ├─ position_states (State History)
        │  - id, position_id, state, changed_at, reason
        │  States: Open, Closing, Closed, ExitPending, ExitFailed, Phantom, Reconciling
        │
        ├─ position_tracking (Price Tracking History)
        │  - id, position_id, price, price_source, pool_type, pool_address
        │  - api_price, tracked_at
        │
        ├─ position_metadata (System Metadata)
        │  - key, value, updated_at
        │
        └─ token_snapshots (Market Data Snapshots)
           - id, position_id, snapshot_type ('opening' or 'closing')
           - mint, symbol, name, price_sol, price_usd
           - liquidity_usd, liquidity_base, liquidity_quote
           - volume_h24, volume_h6, volume_h1, volume_m5
           - txns (buys/sells for each timeframe)
           - price_change_h24, h6, h1, m5
           - token metadata (uri, description, image, socials)
           - snapshot_time, api_fetch_time, data_freshness_score

Indexes (14+ total):
- positions: mint, entry_time, exit_time, mint+exit_time, entry_sig, exit_sig, state
- position_states: position_id+changed_at, state+changed_at
- position_tracking: position_id+tracked_at, price+tracked_at
- token_snapshots: position_id+snapshot_type, mint+snapshot_time, snapshot_type+snapshot_time
- **NEW** position_exits: position_id (for history queries)
- **NEW** position_entries: position_id (for DCA history)
```

#### Verification Queue System

```
[Verification Queue] → VecDeque with Priority Sorting
                          │
                          ▼
                  VerificationItem Structure:
                  - signature, mint, position_id
                  - kind (Entry/Exit)
                  - created_at, last_attempt_at, next_retry_at
                  - attempts, expiry_height
                          │
                          ▼
                  Exponential Backoff Schedule:
                  - Attempt 1: 5s
                  - Attempt 2: 10s
                  - Attempt 3: 20s
                  - Attempt 4: 40s
                  - Attempt 5: 60s
                  - Attempt 6: 90s
                  - Attempt 7: 120s
                  - Attempt 8: 150s
                  - Attempt 9: 180s
                  - Attempt 10: 210s
                  - Attempt 11+: 300s
                  (With ±10% jitter to prevent thundering herd)
                          │
                          ▼
                  Priority Sorting:
                  1. Due items (past next_retry_at)
                  2. Recent items (age < 60s)
                  3. Oldest items
                          │
                          ▼
                  Expiry Handling:
                  - Height-based: current_height > expiry_height
                  - Time-based fallback:
                    - Entry: 10 minutes
                    - Exit: 3 minutes
                          │
                          ▼
                  Expired Entry → RemoveOrphanEntry
                  Expired Exit → Keep retrying (user action required)
```

#### Global Position Semaphore (Capacity Enforcement)

```
[Global Semaphore] → Initialized with max_open_positions
                          │
                          ▼
                  acquire_global_position_permit()
                  - Called BEFORE position creation
                  - Try acquire (non-blocking)
                  - Fail immediately if no slots
                          │
                          ▼
                  Permit "Forgotten" After Success
                  - Position added to state
                  - Semaphore capacity consumed
                  - Slot held until position closed
                          │
                          ▼
                  release_global_position_permit()
                  - Called on ExitVerified
                  - Called on ExitPermanentFailureSynthetic
                  - Called on RemoveOrphanEntry
                  - Adds 1 permit back to semaphore
                          │
                          ▼
                  Reconciliation at Startup
                  - Count open positions from DB
                  - Consume N permits
                  - Align semaphore with actual state
```

#### Integration Points

**1. Trader Module Integration:**

```
[Trader] → positions::open_position_direct(mint)
         → positions::close_position_direct(mint)
         → positions::get_open_mints() (monitoring)
         → positions::get_open_positions() (exit checks)
         → positions::get_open_positions_count() (capacity check)
```

**2. Pool Service Integration:**

```
[Pool Discovery] → positions::get_open_mints()
                   (Always include tokens with open positions for price monitoring)
```

**3. OHLCV Service Integration:**

```
[OHLCV Monitor] → positions::state::get_open_positions()
                  (Set priority=100 for tokens with open positions)
```

**4. Tokens Module Integration:**

```
[Positions] → tokens::get_full_token_async(mint)
            → tokens::get_decimals(mint)
            → tokens::cleanup::blacklist_token(mint, reason)
              (Loss detection → auto-blacklist on significant losses)
```

**5. Transactions Module Integration:**

```
[Verifier] → transactions::get_transaction(signature)
             (Retrieve verified transaction with balance changes)
           → Transaction.swap_analysis (extract amounts, fees, prices)
```

**6. Swaps Module Integration:**

```
[Operations] → swaps::get_best_quote_for_opening(...)
             → swaps::get_best_quote(...)
             → swaps::execute_best_swap(...)
```

**7. Events System Integration:**

```
[Positions] → events::record_position_event(...)
              - entry_verified
              - exit_verified
              - exit_retry_cleared
              - orphan_entry_removed
              - verification_started
              - verification_finished
```

#### Loss Detection & Auto-Blacklisting

```
[process_position_loss_detection(position)] → Calculate Final P&L
                          │
                          ▼
                  Check if Loss
                          │
                          ▼
                  Check Threshold (≤ -15%)
                          │
        ┌─────────────────┼─────────────────┐
        ▼                                   ▼
   Significant Loss                  Minor Loss
   (≤ -15%)                          (> -15%)
        │                                   │
        ▼                                   ▼
   Blacklist Token                    Log Only
   - Add to tokens.db                 (No Action)
   - Reason: "PoorPerformance"
   - Log auto-blacklist event
        │
        ▼
   Token excluded from future filtering
```

#### Key Configuration Options

- **max_open_positions**: Maximum concurrent open positions (enforced by semaphore)
- **position_open_cooldown_secs**: Minimum time between position opens (global cooldown)
- **verification batch size**: 10 items per worker cycle
- **expiry heights**: 150 slots (~60s) from current height
- **verification expiry fallback**: Entry 10min, Exit 3min (time-based)
- **loss blacklist threshold**: -15% or worse (configurable constant)

#### Critical Implementation Details

1. **Semaphore Atomicity**: Global position permit acquired BEFORE any swap execution to prevent race conditions
2. **Permit Forgetting**: Permit is "forgotten" (not dropped) after successful position creation, consuming slot for position lifetime
3. **Triple Guard System**: In-memory check → Database check → Pending-open check (prevents duplicates)
4. **Per-Mint Locking**: Mutex per token mint serializes concurrent opens for same token
5. **Verification Worker Dependencies**: Waits for TRANSACTIONS_SYSTEM_READY + POOL_SERVICE_READY before starting
6. **Exponential Backoff**: Tiered retry schedule (5s → 300s) with jitter to reduce RPC pressure
7. **Re-enqueue Guard**: Every cycle scans all positions and re-enqueues missing verifications
8. **State Machine Transitions**: All state changes go through `apply_transition()` for consistency
9. **Database Sync**: Forced sync after critical updates (entry/exit verified) for durability
10. **Loss Detection**: Automatic blacklisting on close if loss ≤ -15% (configurable)
11. **Price Tracking**: UpdatePriceTracking now persists price/high/low/current timestamps via partial DB update
12. **Token Snapshots**: Market data captured at both opening and closing for post-analysis
13. **Phantom Detection**: Track positions with zero wallet balance (reconciliation logic exists but not actively used)
14. **Synthetic Exits**: Permanent failure exits still release semaphore permits to avoid slot leaks
15. **Orphan Removal**: Expired entry verifications automatically remove position and release permit

### 7. Transaction Processing Flow (Real-Time Monitoring & Analysis)

The transactions module provides comprehensive transaction monitoring, analysis, and verification with real-time WebSocket integration and intelligent caching.

```
┌─────────────────────────────────────────────────────────────────────────┐
│                    TRANSACTIONS SERVICE ARCHITECTURE                    │
│                      (Priority 80, No Dependencies)                     │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
        ┌───────────────────────────┼───────────────────────────┐
        ▼                           ▼                           ▼
┌──────────────┐          ┌──────────────┐          ┌──────────────┐
│   Manager    │          │   Service    │          │  Database    │
│  (Lifecycle) │          │  (Coordin-   │          │  (SQLite     │
│              │          │   ation)     │          │   Cache)     │
└──────────────┘          └──────────────┘          └──────────────┘
        │                           │                           │
        ▼                           ▼                           ▼
┌──────────────┐          ┌──────────────┐          ┌──────────────┐
│  Processor   │          │  Fetcher     │          │  Analyzer    │
│  (Pipeline)  │          │  (RPC Batch) │          │  (Balance,   │
│              │          │              │          │   DEX, PnL)  │
└──────────────┘          └──────────────┘          └──────────────┘
        │                           │                           │
        └───────────────────────────┴───────────────────────────┘
                                    │
                                    ▼
                         ┌──────────────────┐
                         │   WebSocket      │
                         │   (Real-Time     │
                         │   Monitoring)    │
                         └──────────────────┘
```

#### Service Startup & Bootstrap Flow

```
[Service Start] → Initialize TransactionsManager
                          │
                          ▼
                  Register in GLOBAL_TRANSACTION_MANAGER
                  (before bootstrap for on-demand access)
                          │
                          ▼
                  Perform Initial Bootstrap
                          │
        ┌─────────────────┼─────────────────┬──────────────────┐
        ▼                 ▼                 ▼                  ▼
   Check DB State   Load Cursor      Determine Mode      Check Newest
   (bootstrap_      (backfill_       (FULL vs            Known Signature
    state table)     before_cursor)   INCREMENTAL)       (checkpoint)
        │                 │                 │                  │
        └─────────────────┴─────────────────┴──────────────────┘
                          │
                          ▼
                  ┌───────────────────┐
                  │ Bootstrap Mode    │
                  ├───────────────────┤
                  │ FULL: Complete    │
                  │  history backfill │
                  │  from latest to   │
                  │  chain end        │
                  │                   │
                  │ INCREMENTAL:      │
                  │  Only fetch newer │
                  │  than newest-known│
                  └───────────────────┘
                          │
                          ▼
        ═══════════════════════════════════════════════════════
        PHASE 1: COLLECT SIGNATURES (RPC Pages)
        ═══════════════════════════════════════════════════════
                          │
                          ▼
        ┌─────────────────┼─────────────────┬──────────────────┐
        ▼                 ▼                 ▼                  ▼
   Start from       Fetch in Pages    Check if Hit      Persist Cursor
   Latest          (batch_limit=100)  Checkpoint        (FULL mode)
   (before=None)                       (if set)          (per page)
        │                 │                 │                  │
        └─────────────────┴─────────────────┴──────────────────┘
                          │
                          ▼
                  Stop Conditions:
                  - INCREMENTAL: Hit checkpoint OR single page done
                  - FULL: Reached chain end (page < batch_limit)
                          │
                          ▼
        ═══════════════════════════════════════════════════════
        PHASE 2: FILTER & PROCESS (Parallel Batches)
        ═══════════════════════════════════════════════════════
                          │
                          ▼
        Filter out already known signatures
        (check global cache + DB known_signatures)
                          │
                          ▼
        Process in parallel batches (size=10)
        with timeout (15s per transaction)
                          │
        ┌─────────────────┼─────────────────┬──────────────────┐
        ▼                 ▼                 ▼                  ▼
   Fetch from       Analyze          Store in DB        Add to Known
   RPC (fetcher)    (analyzer)       (processed_tx)     (global cache)
        │                 │                 │                  │
        └─────────────────┴─────────────────┴──────────────────┘
                          │
                          ▼
        Track Failed Signatures
        (timeouts, RPC errors, indexing delays)
                          │
                          ▼
        ═══════════════════════════════════════════════════════
        PHASE 3: RETRY FAILED (Exponential Backoff)
        ═══════════════════════════════════════════════════════
                          │
                          ▼
        Retry up to MAX_RETRY_ATTEMPTS (3)
        with exponential backoff (2^n * base_delay)
                          │
        ┌─────────────────┼─────────────────┬──────────────────┐
        ▼                 ▼                 ▼                  ▼
   Delay Between    Process Failed    Track Success     Mark Permanent
   Retries          in Batches        Recoveries        Failures
   (2s, 4s, 8s)     (size=10)         (retry_success)   (errors)
        │                 │                 │                  │
        └─────────────────┴─────────────────┴──────────────────┘
                          │
                          ▼
        Update Bootstrap State:
        - FULL + reached_chain_end → mark full_history_completed
        - FULL + not done → persist backfill cursor for resume
        - INCREMENTAL → no state update needed
                          │
                          ▼
        Set TRANSACTIONS_SYSTEM_READY flag
        (signals other services to proceed)
                          │
                          ▼
        Start Main Service Loop
```

#### Main Service Loop (Real-Time Coordination)

```
[Main Service Loop] → Tokio Select Loop (Multiple Event Sources)
                          │
        ┌─────────────────┼─────────────────┬──────────────────┬──────────────┐
        ▼                 ▼                 ▼                  ▼              ▼
   Periodic Check    WebSocket Rx    WS Health Check    Shutdown Signal  Health Check
   (every 3s)        (real-time)     (every 15s)        (SHUTDOWN_       (every 5min)
                                                         NOTIFY)
        │                 │                 │                  │              │
        ▼                 ▼                 ▼                  │              ▼
   - Cleanup         - Process          - Check WS           │         - Log Metrics
   Expired           Signature          Connection           │         - Update Stats
   Pending           - Update           - Reconnect          │         - Check Service
   - Process         Activity           if Dropped           │           Health
   Deferred          Timestamp                               │
   Retries                                                    │
   - Fallback                                                 │
   Check (if WS                                               │
   inactive >60s)                                             │
        │                 │                 │                  │              │
        └─────────────────┴─────────────────┴──────────────────┴──────────────┘
                                    │
                                    ▼
                         Return on Shutdown Signal
```

#### Transaction Processing Pipeline (Core Flow)

```
[process_transaction(sig)] → Entry Point (Called by Service, Positions, or On-Demand)
                          │
                          ▼
                  Check Cache-Only Mode
                  (used by debug tools)
                          │
                          ├─ Cache-Only: Try DB Only
                          └─ Normal: Continue
                          │
                          ▼
        ═══════════════════════════════════════════════════════
        STEP 1: FETCH TRANSACTION DATA (RPC)
        ═══════════════════════════════════════════════════════
                          │
                          ▼
        Fetcher.get_transaction_details(signature)
        - Uses jsonParsed encoding (for LUT resolution)
        - Returns TransactionDetails with meta, logs, accounts
                          │
                          ▼
        ═══════════════════════════════════════════════════════
        STEP 2: CREATE TRANSACTION STRUCT
        ═══════════════════════════════════════════════════════
                          │
                          ▼
        Extract Base Data:
        - signature, slot, block_time, status
        - fee_lamports, compute_units_consumed
        - success, error_message
        - raw_transaction_data (full JSON blob)
        - log_messages, accounts_count
                          │
                          ▼
        ═══════════════════════════════════════════════════════
        STEP 3: COMPREHENSIVE ANALYSIS (Analyzer)
        ═══════════════════════════════════════════════════════
                          │
                          ▼
        Analyzer.analyze_transaction(tx, tx_data)
                          │
        ┌─────────────────┼─────────────────┬──────────────────┬──────────────┐
        ▼                 ▼                 ▼                  ▼              ▼
   Balance          DEX Analysis     ATA Analysis      Pattern          PnL Analysis
   Analysis         (Swap Detection) (Account Ops)     Detection        (Profit Calc)
   (SOL/Token)      (Router ID)      (Create/Close)    (Classification)
        │                 │                 │                  │              │
        ▼                 ▼                 ▼                  ▼              ▼
   Extract          Identify         Track ATA         Classify         Calculate
   preBalances,     Jupiter/GMGN/    Operations        Transaction      Swap P&L
   postBalances,    PumpFun/Orca/    (rent costs)      Type & Direction if Buy/Sell
   preToken,        Raydium/         Filter noise      (Incoming/       detected
   postToken        Meteora          (tips, fees)      Outgoing)
   balances         routers
        │                 │                 │                  │              │
        └─────────────────┴─────────────────┴──────────────────┴──────────────┘
                          │
                          ▼
        Return CompleteAnalysis:
        - transaction_type (Buy/Sell/Transfer/ATA/etc)
        - direction (Incoming/Outgoing/Internal)
        - balance_analysis (SOL/token changes)
        - dex_analysis (router, token info)
        - ata_analysis (operations, rent)
        - pnl_analysis (profit/loss if swap)
        - confidence score (data quality)
                          │
                          ▼
        ═══════════════════════════════════════════════════════
        STEP 4: MAP ANALYSIS TO TRANSACTION
        ═══════════════════════════════════════════════════════
                          │
                          ▼
        Populate Transaction Fields:
        - transaction_type, direction
        - sol_balance_changes, token_balance_changes
        - sol_balance_change (net delta)
        - token_swap_info (mint, amount, router)
        - swap_pnl_info (profit/loss)
        - ata_operations (list of ops)
        - token_symbol, token_decimals
        - fee_sol (lamports to SOL)
                          │
                          ▼
        ═══════════════════════════════════════════════════════
        STEP 5: STORE IN DATABASE
        ═══════════════════════════════════════════════════════
                          │
                          ▼
        Database.store_processed_transaction(tx)
        - Stores in processed_transactions table
        - Saves analysis snapshot (cached_analysis)
        - Indexes by signature, type, direction, mint
        - Links to raw_transactions (FOREIGN KEY)
                          │
                          ▼
        Record Transaction Event
        (events module integration)
                          │
                          ▼
        Return Transaction Object
```

#### Balance Analysis Details (Industry-Standard Approach)

```
[Balance Analysis] → Used by DexScreener, GMGN, Birdeye Pattern
                          │
        ┌─────────────────┼─────────────────┬──────────────────┐
        ▼                 ▼                 ▼                  ▼
   SOL Balance      Token Balance      Filter Noise       Calculate
   Changes          Changes            (Tips, Rent,       Confidence
   (preBalances/    (preTokenBalances/ ATA Creation)      (Data Quality)
    postBalances)    postTokenBalances)
        │                 │                 │                  │
        ▼                 ▼                 ▼                  ▼
   Extract by       Extract by         Exclude:           Based on:
   Account Index    Owner & Mint       - MEV/Jito tips    - Balance data
   (wallet vs       (token accounts)   (~0.01 SOL max)      completeness
   others)                             - ATA rent           - Token data
                                       (~0.00204 SOL)         availability
                                       - Compute fees       - Decimals loaded
        │                 │                 │                  │
        └─────────────────┴─────────────────┴──────────────────┘
                          │
                          ▼
        Return BalanceAnalysis:
        - sol_changes (HashMap by account)
        - token_changes (HashMap by account+mint)
        - clean_transfers (filtered list)
        - total_tips, total_rent (excluded amounts)
        - confidence (0.0-1.0)
```

#### WebSocket Integration (Real-Time Monitoring)

```
[WebSocket System] → Real-Time Transaction Notifications
                          │
                          ▼
        Determine WS URL:
        - Prefer Helius if API key in config (wss://...)
        - Fallback to default Solana WS (wss://api.mainnet-beta.solana.com)
                          │
                          ▼
        Start WebSocket Monitoring Loop
        (spawned in background, managed by service)
                          │
        ┌─────────────────┼─────────────────┬──────────────────┐
        ▼                 ▼                 ▼                  ▼
   Subscribe to     Handle Ping/Pong   Receive Signature  Send to Channel
   logsSubscribe    (30s heartbeat)    Notifications      (mpsc unbounded)
   (wallet filter)  (keep-alive)       (confirmed txs)    (to service)
        │                 │                 │                  │
        └─────────────────┴─────────────────┴──────────────────┘
                          │
                          ▼
        On Connection Drop:
        - Close channel (None sent to service)
        - Service detects on next health check (15s)
        - Auto-reconnect with exponential backoff
                          │
                          ▼
        Service Receives Signature:
        - Update activity timestamp
        - Process via Processor.process_transaction()
        - Add to known signatures
        - Available immediately for positions verification
```

#### Fallback Check (When WebSocket Inactive)

```
[Fallback Trigger] → WebSocket inactive > 60s
                          │
                          ▼
        Fetch Recent Signatures via RPC
        (up to 100 most recent)
                          │
                          ▼
        Filter out known signatures
        (check global cache + DB)
                          │
                          ▼
        Process new signatures
        (same pipeline as bootstrap)
                          │
                          ▼
        Log new transaction count
        (INFO if found, DEBUG if none)
```

#### Database Schema (transactions.db)

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         DATABASE TABLES                                 │
└─────────────────────────────────────────────────────────────────────────┘
        │
        ├─ raw_transactions
        │  - signature (PK)
        │  - slot, block_time, timestamp
        │  - status (Pending/Confirmed/Finalized/Failed)
        │  - success, error_message
        │  - fee_lamports, compute_units_consumed
        │  - instructions_count, accounts_count
        │  - raw_transaction_data (JSON blob)
        │
        ├─ processed_transactions
        │  - signature (PK, FK to raw_transactions)
        │  - transaction_type (enum as string)
        │  - direction (Incoming/Outgoing/Internal/Unknown)
        │  - sol_balance_change (JSON blob)
        │  - token_balance_changes (JSON array)
        │  - token_swap_info (JSON blob)
        │  - swap_pnl_info (JSON blob)
        │  - ata_operations (JSON array)
        │  - cached_analysis (JSON blob, optional)
        │  - analysis_version, analysis_duration_ms
        │  - fee_sol, sol_delta (scalar fields for queries)
        │
        ├─ known_signatures
        │  - signature (PK)
        │  - status (known/processed)
        │  - added_at (timestamp)
        │
        ├─ deferred_retries
        │  - signature (PK)
        │  - next_retry_at (timestamp)
        │  - remaining_attempts, current_delay_secs
        │  - last_error
        │
        ├─ pending_transactions
        │  - signature (PK)
        │  - added_at, last_checked_at
        │  - check_count
        │
        └─ db_metadata
           - key (PK)
           - value (JSON blob)
           - Stores: schema_version, bootstrap_state
```

#### Integration Points

**1. Positions Module Integration:**

```
[Positions Verifier] → get_transaction(signature)
                          │
                          ▼
                  Check DB Cache First
                          │
                          ├─ Hit (fresh): Return immediately
                          └─ Miss: On-demand processing
                          │
                          ▼
                  Processor.process_transaction(sig)
                  with retry for indexing delays
                  (3 attempts, 300ms → 540ms → 972ms)
                          │
                          ▼
                  Return Transaction or None
                  (used for entry/exit verification)
```

**2. Wallet Service Integration:**

```
[Wallet Service] → Query transactions.db for flow metrics
                          │
                          ▼
                  Aggregate from processed_transactions:
                  - Total SOL in/out
                  - Swap counts (buy/sell)
                  - Token transfers
                  - Fee totals
                          │
                          ▼
                  Cache results (avoid DB queries on every snapshot)
                          │
                          ▼
                  Return WalletFlowMetrics
                  (used by dashboard and reports)
```

**3. Events System Integration:**

```
[Transaction Processing] → Record transaction events
                          │
                          ▼
                  record_transaction_event(
                    signature, action, success, fee, slot, error
                  )
                          │
                          ▼
                  Stored in events.db
                  (queryable via dashboard)
```

#### Key Configuration Options

- **check_interval_secs**: Periodic check frequency (default: 3s from NORMAL_CHECK_INTERVAL_SECS)
- **enable_websocket**: Enable real-time WebSocket monitoring (default: true)
- **max_concurrent_processing**: Parallel transaction processing (default: 10)
- **max_retry_attempts**: Retry failed transactions (default: 3)
- **retry_base_delay_secs**: Base delay between retries (default: 30s)
- **CONCURRENT_BATCH_SIZE**: Bootstrap parallel batch size (default: 10)
- **TRANSACTION_TIMEOUT_SECS**: Processing timeout per transaction (default: 15s)
- **RPC_BATCH_SIZE**: Signatures per RPC page (default: 100)

#### Critical Implementation Details

1. **Bootstrap Modes**:
   - FULL: Backfill complete history from latest to chain end (persists cursor for resume)
   - INCREMENTAL: Only fetch newer than newest-known signature (after full history completed)

2. **Global Transaction Manager**:
   - Registered BEFORE bootstrap (allows on-demand access during bootstrap)
   - Single source of truth for transaction processing

3. **Known Signatures Tracking**:
   - Global cache (in-memory HashSet) + persistent DB (known_signatures table)
   - Prevents duplicate processing across restarts

4. **WebSocket Health**:
   - Health check every 15s (checks if receiver exists)
   - Auto-reconnect only if connection doesn't exist
   - Activity timestamp tracks last message (used for fallback trigger)

5. **Fallback Strategy**:
   - Triggered only when WebSocket inactive > 60s
   - Fetches recent 100 signatures via RPC
   - Supplements WebSocket, doesn't replace it

6. **Analysis Caching**:
   - Analysis results cached in processed_transactions
   - Includes cached_analysis snapshot (optional, for finalized txs)
   - Fresh analysis if cache stale or missing

7. **jsonParsed Encoding**:
   - Required for LUT (Lookup Table) resolution in v0 transactions
   - Set globally in RPC client configuration

8. **Balance Change Extraction**:
   - Industry-standard approach (DexScreener, GMGN, Birdeye pattern)
   - Uses meta.preBalances/postBalances for SOL
   - Uses meta.preTokenBalances/postTokenBalances for SPL tokens
   - Filters noise: MEV tips, ATA rent, compute fees

9. **DEX Router Detection**:
   - Identifies Jupiter, GMGN, PumpFun, Orca, Raydium, Meteora
   - Uses program IDs and log pattern matching
   - Centralized in program_ids.rs

10. **Error Handling**:
    - Retries with exponential backoff (2^n \* base_delay)
    - Tracks permanent failures (max attempts exceeded)
    - Handles indexing delays with short retries (300ms → 540ms → 972ms)

#### Public API Summary

**Service Lifecycle:**

- `start_global_transaction_service(wallet_pubkey, monitor)` - Start service with instrumentation
- `stop_global_transaction_service()` - Graceful shutdown
- `is_global_transaction_service_running()` - Check service status
- `get_global_transaction_manager()` - Access manager instance

**Transaction Access:**

- `get_transaction(signature)` - Cache-first with on-demand processing
- `is_signature_known_globally(signature)` - Check if processed
- `add_signature_to_known_globally(signature)` - Mark as processed

**Database Access:**

- `get_transaction_database()` - Access database instance
- `init_transaction_database(db_path)` - Initialize database

**Analysis & Verification:**

- `verify_entry_transaction(signature, mint, sol_amount)` - Verify entry
- `verify_exit_transaction(signature, mint, token_amount)` - Verify exit
- `TransactionAnalyzer::analyze_transaction()` - Complete analysis

### 8. Wallet Management Flow (Balance Monitoring & Dashboard with 3-Tier Caching)

The wallet module provides comprehensive wallet balance monitoring with sophisticated 3-tier caching, SOL flow tracking, and pre-computed dashboard metrics for fast API responses.

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         WALLET SERVICE                                   │
│                  (Priority 90, No Dependencies)                          │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
        ┌───────────────────────────┼───────────────────────────┐
        ▼                           ▼                           ▼
┌──────────────┐          ┌──────────────┐          ┌──────────────┐
│  Database    │          │   Service    │          │   Caching    │
│  (wallet.db) │          │  (5 Loops)   │          │  (3 Tiers)   │
└──────────────┘          └──────────────┘          └──────────────┘
        │                           │                           │
        ▼                           ▼                           ▼
┌──────────────┐          ┌──────────────┐          ┌──────────────┐
│  5 Tables:   │          │  Snapshot    │          │  Memory      │
│  snapshots,  │          │  Flow Sync   │          │  Database    │
│  balances,   │          │  Metrics 24h │          │  Realtime    │
│  metadata,   │          │  Metrics 7d  │          │              │
│  flow_cache, │          │  Metrics 30d │          │              │
│  metrics     │          │  Metrics All │          │              │
└──────────────┘          └──────────────┘          └──────────────┘
```

#### Service Startup & Database Initialization

```
[Service Start] → initialize_wallet_database()
                          │
                          ├─ Create/Connect to data/wallet.db
                          ├─ Set WAL mode (concurrent reads)
                          ├─ Create 5 tables + indexes
                          ├─ Connection pool (max 3, min idle 1)
                          └─ Set schema version (v2)
                          │
                          ▼
              [warmup_dashboard_metrics()]
                          │
                          ├─ Compute initial 24h metrics (async)
                          ├─ Compute initial 7d metrics (async)
                          ├─ Compute initial 30d metrics (async)
                          └─ Compute initial all-time metrics (async)
                          │
                          ▼
                  [Service Loop Ready]
```

#### Background Service Loops (5 Concurrent Tasks)

**Loop 1: Snapshot Collection** (Every 60s default, configurable 15-600s)

```
[Snapshot Loop] → get_wallet_address()
                          │
                          ├─ 500ms delay (avoid RPC overload)
                          ▼
                  get_rpc_client().get_sol_balance()
                          │
                          ├─ 500ms delay (avoid RPC overload)
                          ▼
                  get_rpc_client().get_all_token_accounts()
                          │
                          ├─ Filter zero-balance accounts
                          ├─ Get decimals from tokens module cache
                          └─ Calculate balance_ui for each token
                          │
                          ▼
              save_wallet_snapshot_sync()
                          │
                          ├─ INSERT INTO wallet_snapshots
                          ├─ INSERT INTO token_balances (batch)
                          └─ Return snapshot_id
                          │
                          ▼
              [Cleanup Counter] (Every 60 intervals ~1 hour)
                          │
                          ├─ cleanup_old_snapshots_sync()
                          └─ cleanup_expired_metrics()
```

**Loop 2: Flow Cache Sync** (Every 5s default, configurable 1-60s)

```
[Flow Sync Loop] → get_flow_cache_max_ts_sync()
                          │
                          ├─ Get latest cached timestamp (wallet DB)
                          ├─ Subtract lookback window (3600s safety)
                          └─ Start timestamp = max_ts - lookback
                          │
                          ▼
              get_transaction_database()
                          │
                          ▼
              export_processed_for_wallet_flow(start_ts, batch_size)
                          │
                          ├─ Read from transactions.processed_transactions
                          ├─ Extract: signature, timestamp, sol_delta
                          ├─ Limit: 2000 rows (configurable 100-20000)
                          └─ Return Vec<(sig, ts, delta)>
                          │
                          ▼
              upsert_flow_rows_sync()
                          │
                          ├─ INSERT OR REPLACE INTO sol_flow_cache
                          ├─ Batch transaction (all rows)
                          └─ Fast indexed upsert
```

**Loop 3-6: Dashboard Metrics Pre-computation** (4 Separate Intervals)

```
[Metrics Loop 24h] → Every 60s
                          │
                          ▼
              compute_and_cache_metrics_internal("24h", 24)
                          │
                          ├─ Check circuit breaker (skip if failing repeatedly)
                          ├─ Check transactions DB ready
                          └─ Proceed to computation
                          │
                          ▼
              compute_and_cache_metrics("24h", 24)
                          │
                          ├─ Check for existing valid cache (skip if fresh)
                          └─ Trigger computation
                          │
                          ▼
              compute_dashboard_payload_realtime(24, 2880, 100)
                          │
                          ├─ Get recent snapshots (2880 limit)
                          ├─ Filter to 24h window
                          ├─ Compute flow metrics (cached or DB)
                          ├─ Compute daily flows (time-series chart)
                          ├─ Enrich token overview (tokens module)
                          └─ Build WalletDashboardData
                          │
                          ▼
              serialize_dashboard_payload()
                          │
                          ├─ Strip cache metadata
                          ├─ Serialize to JSON
                          └─ Gzip compress (Compression::fast())
                          │
                          ▼
              store_dashboard_metrics()
                          │
                          ├─ INSERT OR REPLACE INTO wallet_dashboard_metrics
                          ├─ Store gzipped blob + metadata
                          ├─ Set valid_until (now + interval)
                          └─ Track computation time, snapshot/flow counts

[Metrics Loop 7d]  → Every 300s (5 min)  → Same flow, window=168h
[Metrics Loop 30d] → Every 900s (15 min) → Same flow, window=720h
[Metrics Loop All] → Every 1800s (30 min) → Same flow, window=0 (all-time)
```

#### Dashboard Data Request Flow (3-Tier Caching)

```
[Dashboard API Request] → get_wallet_dashboard_data(window, snapshot_limit, token_limit)
                          │
                          ├─ Clamp parameters to valid ranges
                          ├─ Build request key (window, limit, limit)
                          └─ Start latency timer
                          │
                          ▼
              [Tier 1: Memory Cache Check]
                          │
                          ├─ Check API_RESPONSE_CACHE (HashMap)
                          ├─ TTL: 30s (configurable 10-300s)
                          └─ Cache hit? Return immediately
                          │
                          ▼ (Cache miss)
              [Tier 2: Database Cache Check]
                          │
                          ├─ Map window to canonical key (24h/7d/30d/all_time)
                          ├─ get_dashboard_metrics(window_key) from DB
                          ├─ Check if covers requested snapshot/token limits
                          ├─ Check if within valid_until timestamp
                          └─ Valid? Decompress and return
                          │
                          ▼ (Cache miss/expired)
              [Tier 3: Real-time Computation]
                          │
                          ▼
              compute_dashboard_payload_realtime()
                          │
                          ├─ Try initialize DB if not ready
                          ├─ Get recent snapshots (with retry logic)
                          ├─ Empty fallback if no snapshots
                          └─ Build payload from scratch
                          │
                          ▼
              [Cache Metrics Recording]
                          │
                          ├─ Track source: Memory/Database/Realtime
                          ├─ Track latency (milliseconds)
                          ├─ Track staleness flag
                          ├─ Increment hit counters
                          └─ Store in CACHE_METRICS (global)
```

#### SOL Flow Computation (Cache-First Strategy)

```
[compute_flow_metrics(window_hours)] → All-time mode (window=0)?
                          │
                          ├─ Yes: Get min_ts from flow cache
                          │       Aggregate from min_ts to now
                          │       Fallback to epoch if cache empty
                          │
                          └─ No: Calculate window_start (now - hours)
                          │
                          ▼
              [Try Cached Aggregation First]
                          │
                          ├─ aggregate_cached_flows_sync(start, end)
                          ├─ SELECT SUM from sol_flow_cache (indexed)
                          ├─ Fast: Pre-aggregated deltas
                          └─ Success with tx_count > 0? Return
                          │
                          ▼ (Cache miss/empty)
              [Fallback to Transactions DB]
                          │
                          ├─ get_transaction_database()
                          ├─ aggregate_sol_flows_since(start, end)
                          ├─ Slower: Full scan of processed_transactions
                          └─ Return (inflow, outflow, tx_count)
                          │
                          ▼
              Return WalletFlowMetrics
                          ├─ window_hours
                          ├─ inflow_sol
                          ├─ outflow_sol
                          ├─ net_sol (inflow - outflow)
                          └─ transactions_analyzed
```

#### Daily Flows Computation (Time-Series Chart Data)

```
[compute_daily_flows(window_hours)] → Calculate window_start
                          │
                          ├─ All-time (0): From epoch
                          └─ Windowed: now - hours
                          │
                          ▼
              get_transaction_database()
                          │
                          ▼
              aggregate_daily_flows(window_start, None)
                          │
                          ├─ GROUP BY DATE(timestamp)
                          ├─ SUM inflows, SUM outflows, COUNT txs
                          └─ Return Vec<(date, inflow, outflow, count)>
                          │
                          ▼
              [Convert to DailyFlowPoint]
                          │
                          ├─ Parse date string → timestamp
                          ├─ Calculate net = inflow - outflow
                          └─ Build points with date + timestamp
                          │
                          ▼
              [Apply Payload Cap & Decimation]
                          │
                          ├─ Cap: 730 days max (configurable)
                          ├─ Decimation threshold: 365 days
                          ├─ Keep recent quarter in full resolution
                          ├─ Decimate older half with adaptive stride
                          └─ Prevent huge API responses
                          │
                          ▼
              Return Vec<DailyFlowPoint>
                          ├─ date: "YYYY-MM-DD"
                          ├─ timestamp: Unix timestamp
                          ├─ inflow: SOL
                          ├─ outflow: SOL
                          ├─ net: inflow - outflow
                          └─ tx_count: Number of transactions
```

#### Token Enrichment (Balance + Market Data)

```
[enrich_token_overview(balances, max_tokens)] → Get unique mints
                          │
                          ├─ Deduplicate balances by mint
                          └─ Build unique_mints list
                          │
                          ▼
              [Fetch Token Metadata (Parallel)]
                          │
                          ├─ For each mint: tokens::get_full_token_async()
                          ├─ Builds HashMap<mint, Token>
                          └─ Concurrent fetches
                          │
                          ▼
              [Build WalletTokenOverview for each balance]
                          │
                          ├─ Extract: symbol, name, price_sol, price_usd
                          ├─ Extract: liquidity_usd, volume_24h, updated_at
                          ├─ Extract: data_source (as dex_id)
                          ├─ Calculate: value_sol = price_sol × balance_ui
                          ├─ Fallback: short_mint_label if no metadata
                          └─ Build WalletTokenOverview struct
                          │
                          ▼
              [Sort by Value (Descending)]
                          │
                          ├─ Sort by value_sol (or balance_ui if no price)
                          └─ Highest value tokens first
                          │
                          ▼
              [Truncate to Limit]
                          │
                          ├─ Clamp max_tokens (10-1000 range)
                          └─ Keep top N tokens
                          │
                          ▼
              Return Vec<WalletTokenOverview>
                          ├─ mint, symbol, name
                          ├─ balance_ui, balance_raw, decimals
                          ├─ is_token_2022
                          ├─ price_sol, price_usd, value_sol
                          ├─ liquidity_usd, volume_24h
                          └─ last_updated, dex_id
```

#### Database Schema (wallet.db)

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         DATABASE TABLES                                  │
└─────────────────────────────────────────────────────────────────────────┘

wallet_snapshots:
  - Historical wallet balance snapshots
  - Columns: id, wallet_address, snapshot_time, sol_balance,
             sol_balance_lamports, total_tokens_count, created_at
  - Indexes: (wallet_address, snapshot_time)
  - Purpose: Track SOL balance over time

token_balances:
  - Token holdings per snapshot (one-to-many relationship)
  - Columns: id, snapshot_id, mint, balance, balance_ui, decimals,
             is_token_2022, created_at
  - Foreign Key: snapshot_id → wallet_snapshots(id) ON DELETE CASCADE
  - Indexes: snapshot_id, (snapshot_id, mint)
  - Purpose: Track token balances per snapshot

wallet_metadata:
  - Key-value store for system metadata
  - Columns: key, value, updated_at
  - Primary Key: key
  - Purpose: Store schema version, migration flags

sol_flow_cache:
  - Pre-aggregated SOL flow data (one row per transaction)
  - Columns: signature, timestamp, sol_delta, created_at
  - Primary Key: signature
  - Indexes: timestamp
  - Purpose: Fast SOL flow aggregation (avoid full transactions DB scan)
  - Source: Synced from transactions.processed_transactions every 5s

wallet_dashboard_metrics:
  - Pre-computed dashboard payloads (gzipped JSON blobs)
  - Columns: window_key, window_hours, snapshot_limit, token_limit,
             payload_blob, payload_format, computed_at, valid_until,
             computation_duration_ms, snapshot_count, flow_cache_rows,
             last_processed_timestamp, last_processed_signature, window_start
  - Primary Key: window_key
  - Indexes: valid_until
  - Purpose: Cache expensive dashboard computations
  - Windows: 24h (60s), 7d (300s), 30d (900s), all_time (1800s)
```

#### Integration Points

**1. Transactions Module Integration:**

```
[Wallet] → get_transaction_database()
         ├─ export_processed_for_wallet_flow(start_ts, limit)
         │  └─ Read processed_transactions with balance_changes
         │     Extract SOL deltas for flow cache sync
         │
         ├─ aggregate_sol_flows_since(start, end)
         │  └─ Full aggregation when cache unavailable
         │     Slower fallback for flow metrics
         │
         └─ aggregate_daily_flows(start, end)
            └─ GROUP BY DATE aggregation
               Time-series data for charts
```

**2. Tokens Module Integration:**

```
[Wallet] → tokens::get_full_token_async(mint)
         ├─ Fetch complete token metadata
         ├─ Get: symbol, name, price_sol, price_usd
         ├─ Get: liquidity_usd, volume_24h, data_source
         └─ Used for token enrichment in dashboard

         → tokens::get_cached_decimals(mint)
         └─ Fast decimals lookup for balance calculations
```

**3. RPC Client Integration:**

```
[Wallet] → get_rpc_client()
         ├─ get_sol_balance(wallet_address)
         │  └─ Lamports → SOL conversion
         │
         └─ get_all_token_accounts(wallet_address)
            └─ Returns Vec<TokenAccountInfo>
               Includes: mint, balance, decimals, is_token_2022
```

**4. Config Integration:**

```
[Wallet] → with_config(|cfg| cfg.wallet.*)
         ├─ All intervals are hot-reloadable
         ├─ All limits are hot-reloadable
         └─ No hardcoded values
```

**5. Webserver Integration:**

```
[Webserver Routes] → wallet::get_wallet_dashboard_data()
                   │  └─ POST /api/wallet/dashboard
                   │     Returns WalletDashboardData (JSON)
                   │
                   ├─ wallet::get_current_wallet_status()
                   │  └─ GET /api/wallet/current
                   │     Returns latest snapshot
                   │
                   ├─ wallet::get_flow_cache_stats()
                   │  └─ GET /api/wallet/flow-cache
                   │     Returns cache diagnostics
                   │
                   ├─ wallet::refresh_dashboard_cache()
                   │  └─ POST /api/wallet/refresh-cache
                   │     Force recompute metrics
                   │
                   └─ wallet::get_dashboard_cache_metrics()
                      └─ GET /api/wallet/cache-metrics
                         Returns cache performance stats
```

#### Key Configuration Options

All configuration in `config.wallet.*` section:

- **snapshot_interval_secs**: 60s (15-600s)
  - Frequency of wallet balance snapshots
  - Delayed RPC calls: 500ms between operations

- **flow_cache_update_secs**: 5s (1-60s)
  - Frequency of SOL flow cache sync from transactions DB
  - Critical for fast flow metrics

- **flow_cache_backfill_batch**: 2000 rows (100-20000)
  - Max new transactions per sync cycle
  - Prevents overwhelming database

- **flow_cache_lookback_secs**: 3600s (0-86400s)
  - Safety lookback window when resuming sync
  - Prevents gaps from missed transactions

- **max_daily_flow_days**: 730 days (30-1825)
  - Hard cap on daily flow data points
  - Prevents huge API responses

- **daily_flow_decimate_threshold_days**: 365 days (30-730)
  - Days threshold for data decimation
  - Older data sampled at lower resolution

- **dashboard_metrics_24h_interval_secs**: 60s (30-300s)
  - Pre-compute frequency for 24h metrics
  - Most frequently accessed window

- **dashboard_metrics_7d_interval_secs**: 300s (60-600s)
  - Pre-compute frequency for 7d metrics
  - Moderate update frequency

- **dashboard_metrics_30d_interval_secs**: 900s (300-1800s)
  - Pre-compute frequency for 30d metrics
  - Lower update frequency

- **dashboard_metrics_alltime_interval_secs**: 1800s (600-3600s)
  - Pre-compute frequency for all-time metrics
  - Least frequently accessed

- **api_response_cache_ttl_secs**: 30s (10-300s)
  - Memory cache TTL for dashboard responses
  - Tier 1 cache (fastest)

#### Critical Implementation Details

1. **Delayed RPC Calls**: 500ms delay between `get_sol_balance()` and `get_all_token_accounts()` to avoid overwhelming global RPC client (not rate-limited, best-effort)

2. **Connection Pooling**: r2d2 pool with max 3 connections, min idle 1, designed for low-concurrency monitoring workload

3. **WAL Mode**: Write-Ahead Logging enabled for concurrent reads during snapshot writes

4. **Circuit Breaker**: After 3 consecutive computation failures, skip window for 300s cooldown to prevent repeated expensive operations

5. **Gzip Compression**: Dashboard payloads serialized to JSON then gzipped with `Compression::fast()` before DB storage (reduces storage 5-10x)

6. **Decimation Strategy**:
   - Keep last 25% of data at full resolution
   - Decimate older 75% with adaptive stride
   - Target: ~50% of threshold after decimation

7. **Staleness Handling**: Can serve stale cache with warning if recomputation fails (better than error response)

8. **Metrics Tracking**: Records cache performance (hit rates, latency, source) in global CACHE_METRICS for monitoring

9. **Zero-Balance Filtering**: Token accounts with balance=0 are excluded from snapshots to reduce storage

10. **Cleanup Strategy**: Old snapshots cleaned every ~1 hour (60 snapshot intervals), expired metrics cleaned via valid_until checks

11. **Warmup on Startup**: Triggers async pre-computation of all 4 metric windows on service start (non-blocking)

12. **Fallback Chain**: Memory → Database → Real-time computation with graceful degradation at each level

13. **Token Limit Clamping**: 10-1000 range with sort-by-value to ensure most important tokens included

14. **Snapshot Limit Clamping**: 16-2880 range (16 = ~4 hours at 15s interval, 2880 = 2 days at 1min interval)

15. **Window Clamping**: 1 hour to 2 years for windowed views, 0 = all-time (no filter)

#### Public API Summary

**Service Lifecycle:**

- `start_wallet_monitoring_service(shutdown, monitor)` - Start service with instrumentation
- `initialize_wallet_database()` - Initialize DB connection pool and schema

**Data Access:**

- `get_wallet_dashboard_data(window, snapshot_limit, token_limit)` - Get dashboard data (3-tier cache)
- `get_current_wallet_status()` - Get latest snapshot
- `get_recent_wallet_snapshots(limit)` - Get historical snapshots
- `get_snapshot_token_balances(snapshot_id)` - Get token balances for specific snapshot
- `get_wallet_monitor_stats()` - Get monitoring statistics

**Flow Metrics:**

- `get_flow_cache_stats()` - Get flow cache diagnostics (row count, latest timestamp)

**Cache Management:**

- `refresh_dashboard_cache(window_hours)` - Force recompute specific window
- `get_dashboard_cache_metrics()` - Get cache performance metrics (hit rates, latency)
- `clear_dashboard_api_cache()` - Clear memory cache (Tier 1)

**Dashboard Components:**

- `WalletDashboardData` - Complete dashboard payload
  - `summary`: Current vs previous balance, change %, token count
  - `flows`: Inflow/outflow/net SOL for window
  - `balance_trend`: Time-series SOL balance points
  - `daily_flows`: Time-series daily flow chart data
  - `tokens`: Enriched token overview (balance + market data)
  - `cache_metadata`: Cache source, freshness, computation stats

### 9. OHLCV Data Flow (Multi-Timeframe Candles with Priority-Based Monitoring)

The OHLCV service provides comprehensive multi-timeframe candlestick data with intelligent priority-based monitoring, gap detection, and automatic backfilling.

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         OHLCV SERVICE                                    │
│                  (Priority 45, Depends: tokens, positions)               │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
        ┌───────────────────────────┼───────────────────────────┐
        ▼                           ▼                           ▼
┌──────────────┐          ┌──────────────┐          ┌──────────────┐
│  Database    │          │   Monitor    │          │  Components  │
│  (ohlcvs.db) │          │  (5 Loops)   │          │  (5 Parts)   │
└──────────────┘          └──────────────┘          └──────────────┘
        │                           │                           │
        ▼                           ▼                           ▼
  5 Tables:             1. monitor_loop (5s)        - Fetcher
  - ohlcv_pools         2. gap_fill_loop (5min)     - Cache
  - ohlcv_1m            3. cleanup_loop (1h)        - PoolManager
  - ohlcv_aggregated    4. cache_maint (10min)      - GapManager
  - ohlcv_gaps          5. sync_pool_svc (30s)      - Aggregator
  - monitor_config
```

#### Service Startup Flow

```
[Service Start] → Initialize Database (ohlcvs.db)
                          │
                          ▼
                  Create Components
                          │
                          ├─ OhlcvDatabase (SQLite with WAL)
                          ├─ OhlcvFetcher (GeckoTerminal API)
                          ├─ OhlcvCache (LRU with TTL)
                          ├─ PoolManager (Pool discovery & health)
                          ├─ GapManager (Gap detection & filling)
                          └─ OhlcvMonitor (Orchestrates all)
                          │
                          ▼
                  Load Active Tokens from DB
                          │
                          ▼
                  Start 5 Background Loops
                          │
                          ├─ monitor_loop (data fetching)
                          ├─ gap_fill_loop (gap backfilling)
                          ├─ cleanup_loop (old data removal)
                          ├─ cache_maintenance_loop (cache cleanup)
                          └─ sync_pool_service_tokens (token sync)
                          │
                          ▼
                  Wait 5 Seconds (Allow system stabilization)
                          │
                          ▼
                  Auto-Populate Open Positions
                          │
                          ├─ Get all open positions
                          ├─ Add each to monitoring (Priority::Critical)
                          └─ Record PositionOpened activity
                          │
                          ▼
                  [Service Ready]
```

#### Token Sync Loop (Every 30 seconds)

```
[sync_pool_service_tokens] → Get Available Tokens (Pool Service)
                                    │
                                    ├─ pools::get_available_tokens()
                                    └─ Returns tokens with fresh prices
                          │
                          ▼
                  Get Open Positions
                          │
                          └─ positions::state::get_open_positions()
                          │
                          ▼
                  Process Each Token
                          │
                          ├─ If already monitored:
                          │    └─ Upgrade to Critical if has open position
                          │
                          ├─ If new token:
                          │    ├─ Check max_monitored_tokens limit (skip if full)
                          │    ├─ Determine priority:
                          │    │    ├─ Critical if open position
                          │    │    └─ Low otherwise
                          │    └─ Add to monitoring (discover pools)
                          │
                          └─ If not in Pool Service and no position:
                               └─ Remove from monitoring (cleanup)
                          │
                          ▼
                  Log Summary (added, upgraded, removed)
```

#### Monitor Loop (Every 5 seconds, Priority-Based Processing)

```
[monitor_loop] → Get All Active Tokens
                          │
                          ▼
                  For Each Token (with rate-limit delay):
                          │
                          ├─ Get Recommended Action (PriorityManager)
                          │    │
                          │    ├─ Critical → FetchNow (always)
                          │    ├─ High/Medium/Low → Based on last fetch time
                          │    ├─ Too many empty fetches (≥10) → Pause
                          │    └─ Very inactive (>1 week) → Pause
                          │
                          ├─ If FetchNow:
                          │    │
                          │    ├─ Check if pools available
                          │    │    │
                          │    │    ├─ No pools + backoff elapsed:
                          │    │    │    └─ Try pool discovery (exponential backoff)
                          │    │    │         ├─ Success → Reset failure counter
                          │    │    │         └─ Failed → Increment counter (log throttled)
                          │    │    │
                          │    │    └─ No pools + in backoff period:
                          │    │         └─ Skip silently (waiting for retry)
                          │    │
                          │    ├─ Get best pool (highest liquidity, lowest failures)
                          │    │
                          │    ├─ Calculate batch size by priority:
                          │    │    ├─ Critical → 1000 candles
                          │    │    ├─ High → 500 candles
                          │    │    ├─ Medium → 200 candles
                          │    │    └─ Low → 100 candles
                          │    │
                          │    ├─ Fetch 1m data (base timeframe)
                          │    │    └─ fetcher.fetch_immediate(pool, 1m, batch_size)
                          │    │
                          │    ├─ If empty fetch:
                          │    │    ├─ Increment consecutive_empty_fetches
                          │    │    └─ Log (throttled after 3 attempts)
                          │    │
                          │    └─ If success:
                          │         ├─ Store in database (ohlcv_1m table)
                          │         ├─ Mark pool success (reset failure count)
                          │         ├─ Reset consecutive_empty_fetches
                          │         ├─ Ensure retention window (backfill if needed)
                          │         ├─ Detect gaps (best-effort)
                          │         └─ Record fetch_success event
                          │
                          ├─ If Throttle:
                          │    └─ Skip this cycle (will check next tick)
                          │
                          └─ If Pause:
                               └─ Skip (token inactive or too many failures)
                          │
                          ▼
                  Delay Between Tokens (rate-limit aware)
                          │
                          └─ (60,000ms / rate_limit_per_min) + 100ms buffer
```

#### Gap Fill Loop (Every 5 minutes)

```
[gap_fill_loop] → For Each Active Token:
                          │
                          ├─ Auto-fill recent gaps (last 24h)
                          │    │
                          │    ├─ Query unfilled gaps from database
                          │    ├─ Sort by priority (recent first)
                          │    ├─ Attempt to fetch missing data
                          │    └─ Mark as filled if successful
                          │
                          └─ 1 second delay between tokens
```

#### Cleanup Loop (Every 1 hour)

```
[cleanup_loop] → Check retention_days config
                          │
                          ├─ If retention_days > 0:
                          │    └─ Delete data older than N days
                          │         ├─ ohlcv_1m (by created_at)
                          │         ├─ ohlcv_aggregated (by created_at)
                          │         └─ ohlcv_gaps (if filled)
                          │
                          └─ Log cleanup results
```

#### Cache Maintenance Loop (Every 10 minutes)

```
[cache_maintenance_loop] → Clean Expired Cache Entries
                                    │
                                    ├─ Evict entries older than TTL
                                    ├─ Respect LRU eviction policy
                                    └─ Log cache hit rate
```

#### Priority System (Activity-Based)

```
[Priority Levels] → 4 Tiers with Different Intervals
                          │
                          ├─ Critical (Priority=4)
                          │    ├─ Interval: 30 seconds
                          │    ├─ Batch: 1000 candles
                          │    ├─ Always fetches (no throttling)
                          │    └─ Triggers: PositionOpened
                          │
                          ├─ High (Priority=3)
                          │    ├─ Interval: 60 seconds
                          │    ├─ Batch: 500 candles
                          │    └─ Triggers: PositionClosed, ChartViewed, DataRequested
                          │
                          ├─ Medium (Priority=2)
                          │    ├─ Interval: 300 seconds (5min)
                          │    ├─ Batch: 200 candles
                          │    └─ Triggers: TokenViewed (upgrade from Low)
                          │
                          └─ Low (Priority=1)
                               ├─ Interval: 900 seconds (15min)
                               ├─ Batch: 100 candles
                               └─ Default for new tokens (no activity)
```

#### Activity Types (Priority Adjustments)

```
[Activity Events] → Automatic Priority Changes
                          │
                          ├─ PositionOpened → Critical
                          │    └─ Maximum priority, 30s interval
                          │
                          ├─ PositionClosed → High
                          │    └─ Downgrade from Critical
                          │
                          ├─ ChartViewed → High
                          │    └─ Upgrade from Low/Medium
                          │
                          ├─ TokenViewed → Medium
                          │    └─ Upgrade from Low only
                          │
                          └─ DataRequested → High
                               └─ Manual refresh request
```

#### Database Schema (ohlcvs.db)

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         DATABASE TABLES                                  │
└─────────────────────────────────────────────────────────────────────────┘

ohlcv_pools:
  - Pool configurations for each token
  - Columns: mint, pool_address, dex, liquidity, is_default, last_success, failure_count
  - Indexes: mint, (mint + is_default)

ohlcv_1m:
  - Raw 1-minute candlestick data (base timeframe)
  - Columns: mint, pool_address, timestamp, open, high, low, close, volume
  - Indexes: (mint + timestamp DESC), created_at, (pool_address + timestamp DESC)
  - Retention: Controlled by retention_days config

ohlcv_aggregated:
  - Pre-aggregated higher timeframes (5m, 15m, 1h, 4h, 12h, 1d)
  - Columns: mint, pool_address, timeframe, timestamp, OHLCV data
  - Indexes: (mint + timeframe + timestamp DESC)
  - Populated: On-demand during queries (cache in DB)

ohlcv_gaps:
  - Tracks detected gaps for backfilling
  - Columns: mint, pool_address, timeframe, start_timestamp, end_timestamp, attempts, filled
  - Indexes: (filled + mint)
  - Lifecycle: Created by gap detection, filled by gap_fill_loop

ohlcv_monitor_config:
  - Per-token monitoring configuration
  - Columns: mint (PK), priority, fetch_interval_seconds, last_fetch, last_activity,
             consecutive_empty_fetches, is_active, last_pool_discovery_attempt,
             consecutive_pool_failures
  - Indexes: (is_active + priority)
  - Updated: By monitor loops and activity events
```

#### Pool Discovery & Health

```
[Pool Discovery] → Triggered When Token Has No Pools
                          │
                          ├─ Check if backoff period elapsed
                          │    └─ Exponential backoff: 2^(failures-1) minutes
                          │
                          ├─ If retry allowed:
                          │    │
                          │    ├─ Request snapshot from tokens module
                          │    │    ├─ tokens::prefetch_token_pools([mint])
                          │    │    └─ tokens::get_token_pools_snapshot(mint)
                          │    │
                          │    ├─ Snapshot handling
                          │    │    ├─ Fresh data → proceed to merge
                          │    │    └─ Missing/Error → tokens::get_token_pools_snapshot_allow_stale(mint)
                          │    │         └─ If still None → treat as failure
                          │    │
                          │    ├─ Success:
                          │    │    ├─ Merge TokenPoolInfo → PoolConfig (preserve failure_count)
                          │    │    ├─ Set canonical pool as default & drop stale entries
                          │    │    ├─ Store in ohlcv_pools table
                          │    │    └─ Reset consecutive_pool_failures
                          │    │
                          │    └─ Failure:
                          │         ├─ Increment consecutive_pool_failures
                          │         ├─ Set next retry timestamp
                          │         └─ Log (throttled: only first 3 + every 5th)
                          │
                          └─ If in backoff:
                               └─ Skip silently (wait for next attempt)
                          │
                          ▼
[Pool Health] → Track Per-Pool Success/Failure
                          │
                          ├─ Success:
                          │    ├─ Set last_successful_fetch timestamp
                          │    └─ Reset failure_count to 0
                          │
                          └─ Failure:
                               ├─ Increment failure_count
                               └─ Unhealthy if failure_count ≥ 5
```

#### Data Fetching & Storage

```
[Data Fetch] → GeckoTerminal API Integration
                          │
                          ├─ Endpoint: /networks/solana/pools/:address/ohlcv/:timeframe
                          │
                          ├─ Timeframes:
                          │    ├─ "minute" → 1m candles (base)
                          │    ├─ "hour" → 1h candles (for aggregation)
                          │    └─ "day" → 1d candles (for aggregation)
                          │
                          ├─ Rate Limiting:
                          │    ├─ Global limit: 30 requests/minute (configurable)
                          │    ├─ Delay between tokens: ~2100ms
                          │    └─ Coordinated with tokens module rate limiter
                          │
                          └─ Response Processing:
                               ├─ Parse JSON array of OHLCV points
                               ├─ Validate data (high ≥ low, OHLC within range)
                               ├─ Store in ohlcv_1m table (UNIQUE constraint)
                               └─ Return data points
                          │
                          ▼
[Storage Strategy] → Layered Persistence
                          │
                          ├─ Raw 1m data → ohlcv_1m table
                          │    └─ All fetched candles stored here
                          │
                          ├─ On-demand aggregation → ohlcv_aggregated table
                          │    └─ Computed from 1m data, cached in DB
                          │
                          └─ Memory cache → OhlcvCache (LRU)
                               ├─ TTL: cache_retention_hours config
                               ├─ Capacity: cache_size config (tokens)
                               └─ Eviction: LRU + TTL expiry
```

#### Gap Detection & Filling

```
[Gap Detection] → Automatic After Successful Fetch
                          │
                          ├─ Query latest N candles (e.g., last 24h)
                          │
                          ├─ Calculate expected intervals (based on timeframe)
                          │
                          ├─ Identify missing timestamps
                          │    └─ Gap = interval > (expected * 1.5)
                          │
                          ├─ Store in ohlcv_gaps table
                          │    └─ UNIQUE(mint, pool, timeframe, start, end)
                          │
                          └─ Log gap summary (throttled: first 5 only)
                          │
                          ▼
[Gap Filling] → Backfill Missing Data
                          │
                          ├─ Query unfilled gaps (sorted by priority)
                          │
                          ├─ For each gap:
                          │    │
                          │    ├─ Fetch data for gap range
                          │    │    └─ API call with from/to timestamps
                          │    │
                          │    ├─ Store fetched data
                          │    │
                          │    ├─ Mark gap as filled
                          │    │    └─ UPDATE ohlcv_gaps SET filled=1
                          │    │
                          │    └─ Increment attempts counter
                          │
                          └─ Respect rate limits (pause between attempts)
```

#### Data Retrieval Flow (API Access)

```
[get_ohlcv_data()] → Request with Timeframe & Filters
                          │
                          ├─ Determine Pool:
                          │    ├─ Use specified pool_address, or
                          │    ├─ Get default pool, or
                          │    └─ Get best pool (highest liquidity)
                          │
                          ├─ Try Memory Cache First
                          │    └─ If hit: Return cached data (ASC sorted)
                          │
                          ├─ If timeframe != 1m:
                          │    │
                          │    ├─ Try ohlcv_aggregated table
                          │    │    └─ If hit: Cache & return (ASC sorted)
                          │    │
                          │    └─ Fallback: Aggregate from ohlcv_1m
                          │         ├─ Fetch raw 1m data
                          │         ├─ Aggregate using OhlcvAggregator
                          │         ├─ Cache result
                          │         └─ Return (ASC sorted)
                          │
                          └─ If timeframe == 1m:
                               ├─ Query ohlcv_1m table directly
                               ├─ Filter by timestamp range
                               ├─ Limit to last N candles
                               └─ Return (ASC sorted)
```

#### Integration Points

**1. Pool Service Integration:**

```
[OHLCV] → pools::get_available_tokens()
        → Returns tokens with fresh prices (updated every 5-10s)
        → OHLCV syncs every 30s and adds/upgrades tokens
```

**2. Positions Module Integration:**

```
[OHLCV] → positions::state::get_open_positions()
        → Returns open positions (for Critical priority)
        → Called by sync loop (30s) and startup (auto-populate)
```

**3. Webserver Integration:**

```
[Webserver] → ohlcvs::record_activity(mint, ChartViewed)
            → Upgrades priority when user views chart
            → ohlcvs::get_ohlcv_data(mint, timeframe, limit)
            → Returns candlestick data for frontend charts
```

**4. Tokens Module Integration:**

```
[OHLCV] → Depends on tokens module (service dependency)
        → Uses token metadata for validation
```

#### Key Configuration Options

- **enabled**: Enable/disable OHLCV collection (default: true)
- **max_monitored_tokens**: Max concurrent tokens (default: 100, 0=unlimited)
- **retention_days**: Data retention period (default: 7 days)
- **max_empty_fetches**: Pause threshold (default: 10 consecutive)
- **auto_fill_gaps**: Enable automatic backfilling (default: true)
- **cache_size**: Hot cache capacity in tokens (default: 100)
- **cache_retention_hours**: Cache TTL (default: 24 hours)
- **pool_failover_enabled**: Enable pool health tracking (default: true)
- **max_pool_failures**: Unhealthy threshold (default: 5 failures)

#### Critical Implementation Details

1. **Service Priority**: 45 (starts after tokens=40, before positions=50)
2. **Dependencies**: ["tokens", "positions"] (ensures they're ready first)
3. **Auto-Populate**: Open positions added to monitoring on startup (5s delay)
4. **Token Sync**: Every 30 seconds (NOT 5 minutes as documented elsewhere)
5. **Monitor Cycle**: Every 5 seconds, processes all tokens with rate-limit delays
6. **Gap Filling**: Every 5 minutes, backfills recent gaps (last 24h)
7. **Cleanup**: Every hour, removes old data (controlled by retention_days)
8. **Cache Maintenance**: Every 10 minutes, evicts expired entries
9. **Priority-Based Batching**: Batch size varies by priority (100-1000 candles)
10. **Pool Discovery**: Exponential backoff on failures (2^(n-1) minutes)
11. **Activity Tracking**: User interactions upgrade priority (chart views, data requests)
12. **Database**: SQLite with WAL mode, 30s busy timeout, multiple indexes
13. **Rate Limiting**: Coordinated with GeckoTerminal API limits (30/min default)
14. **Data Aggregation**: On-demand from 1m data, cached in DB and memory
15. **Gap Detection**: Automatic after successful fetches, best-effort backfilling

### 10. API Integration Flow (Centralized External API Clients)

The APIs module provides a centralized, singleton-based architecture for all external API integrations with unified rate limiting, statistics tracking, and error handling.

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         API MANAGER (GLOBAL SINGLETON)                   │
│                    (LazyLock - Initialized on First Access)              │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
        ┌───────────────────────────┼───────────────────────────┐
        ▼                           ▼                           ▼
┌──────────────┐          ┌──────────────┐          ┌──────────────┐
│ DexScreener  │          │ GeckoTerminal│          │   Rugcheck   │
│   Client     │          │    Client    │          │    Client    │
│              │          │              │          │              │
│ Token pools, │          │ Token pools, │          │ Security     │
│ market data, │          │ OHLCV data,  │          │ analysis,    │
│ profiles,    │          │ trending,    │          │ risk score,  │
│ boosts       │          │ new pools    │          │ holders      │
│              │          │              │          │              │
│ Rate: 300/min│          │ Rate: 30/min │          │ Rate: 60/min │
│ Timeout: 10s │          │ Timeout: 10s │          │ Timeout: 15s │
└──────────────┘          └──────────────┘          └──────────────┘
        │                           │                           │
        ▼                           ▼                           ▼
┌──────────────┐          ┌──────────────┐          ┌──────────────┐
│   Jupiter    │          │  CoinGecko   │          │  DefiLlama   │
│    Client    │          │    Client    │          │    Client    │
│              │          │              │          │              │
│ Token        │          │ All coins    │          │ Protocols    │
│ discovery:   │          │ with         │          │ list,        │
│ recent,      │          │ platform     │          │ token        │
│ top organic, │          │ addresses    │          │ prices       │
│ top traded,  │          │ (Solana)     │          │              │
│ trending     │          │              │          │              │
│              │          │ API Key:     │          │              │
│ No rate      │          │ Demo tier    │          │ No rate      │
│ limit        │          │              │          │ limit        │
│ Timeout: 15s │          │ Timeout: 20s │          │ Timeout: 25s │
└──────────────┘          └──────────────┘          └──────────────┘
        │                           │                           │
        └───────────────────────────┴───────────────────────────┘
                                    │
                    ┌───────────────┴───────────────┐
                    ▼                               ▼
        ┌──────────────────────┐        ┌──────────────────────┐
        │  Rate Limiter        │        │  Stats Tracker       │
        │  (Per Client)        │        │  (Per Client)        │
        │                      │        │                      │
        │ - Semaphore (1)      │        │ - Total requests     │
        │ - Min interval       │        │ - Success/failures   │
        │ - Last request time  │        │ - Cache hits/misses  │
        │ - Auto-throttling    │        │ - Response times     │
        │                      │        │ - Last error         │
        └──────────────────────┘        └──────────────────────┘
```

#### Architecture Overview

**Location:** `src/apis/`

**Core Components:**

- `manager.rs` - Global singleton ApiManager (LazyLock pattern)
- `client.rs` - Base HTTP client + RateLimiter implementation
- `stats.rs` - ApiStatsTracker for metrics collection
- `{api}/mod.rs` - Individual API client implementations
- `{api}/types.rs` - API-specific response types

**Global Access Pattern:**

```rust
use crate::apis::get_api_manager;

let apis = get_api_manager();
apis.dexscreener.fetch_token_pools(mint).await?;
apis.geckoterminal.fetch_token_pools(token, network).await?;
apis.rugcheck.get_token_report(mint).await?;
```

**Key Principle:** Single instance per API client across entire bot → True global rate limiting and centralized stats.

#### API Client Configurations

**1. DexScreener Client** (`src/apis/dexscreener/`)

```
Base URL: https://api.dexscreener.com
Default Chain: solana
Max Batch Size: 30 tokens per request
Rate Limit: 300/min (configurable via tokens.sources.dexscreener.rate_limit_per_minute)
Timeout: 10s (configurable via tokens.sources.dexscreener.timeout_seconds)
Enabled: tokens.sources.dexscreener.enabled && tokens.discovery.dexscreener.enabled

Endpoints Implemented (8 total):
  1. /token-pairs/v1/{chainId}/{tokenAddress} - Get all pools for token
  2. /tokens/v1/{chainId}/{tokenAddresses} - Batch pools (up to 30 tokens)
  3. /latest/dex/pairs/{chainId}/{pairId} - Single pair by address
  4. /latest/dex/search?q={query} - Search pairs
  5. /token-profiles/latest/v1 - Latest token profiles
  6. /token-boosts/latest/v1 - Latest boosted tokens
  7. /token-boosts/top/v1 - Top boosted tokens
  8. /orders/v1/{chainId}/{tokenAddress} - Token orders

Primary Use Cases:
  - Token discovery (profiles, boosts)
  - Market data (price, volume, liquidity, transactions)
  - Pool discovery (for Pool Service)
  - Batch token queries (up to 30 at once)
```

**2. GeckoTerminal Client** (`src/apis/geckoterminal/`)

```
Base URL: https://api.geckoterminal.com/api/v2
Default Network: solana
Max Trending Page: 10
Rate Limit: 30/min (configurable via tokens.sources.geckoterminal.rate_limit_per_minute)
Timeout: 10s (configurable via tokens.sources.geckoterminal.timeout_seconds)
Enabled: tokens.sources.geckoterminal.enabled && tokens.discovery.geckoterminal.enabled

Endpoints Implemented (12 total):
  1. /networks/{network}/tokens/{token}/pools - All pools for token (primary)
  2. /networks/{network}/trending_pools - Trending pools
  3. /networks/{network}/pools - Top pools
  4. /networks/{network}/pools/{address} - Pool details
  5. /networks/{network}/pools/multi/{addresses} - Multiple pools
  6. /networks/{network}/pools/{pool}/ohlcv/{timeframe} - OHLCV data
  7. /networks/{network}/dexes - Supported DEX list
  8. /networks/{network}/new_pools - Newly listed pools
  9. /networks/{network}/tokens/multi/{addresses} - Multiple token metadata
  10. /networks/{network}/tokens/{address}/info - Token metadata
  11. /tokens/info_recently_updated - Recent token updates (global)
  12. /networks/{network}/pools/{pool_address}/trades - Pool trades

Primary Use Cases:
  - Market data (price, volume, liquidity)
  - OHLCV candle data (for OHLCV service)
  - Pool discovery and monitoring
  - Token metadata and recent updates
```

**3. Rugcheck Client** (`src/apis/rugcheck/`)

```
Base URL: https://api.rugcheck.xyz/v1/tokens
Stats URL: https://api.rugcheck.xyz/v1/stats
Rate Limit: 60/min (configurable)
Timeout: 15s (security analysis can be slow)
Enabled: tokens.sources.rugcheck.enabled && tokens.discovery.rugcheck.enabled

Endpoints Implemented (4 total):
  1. /v1/tokens/{mint}/report - Full security report
  2. /v1/tokens/{mint}/report/summary - Summary report
  3. /v1/stats/summary - Platform statistics
  4. /v1/tokens/{mints}/batch - Batch reports (multiple tokens)

Primary Use Cases:
  - Security analysis (risk score, authorities, holder distribution)
  - Token blacklisting decisions (via security filters)
  - One-time security fetch per token (not updated)
```

**4. Jupiter Client** (`src/apis/jupiter/`)

```
Base URL: https://lite-api.jup.ag/tokens/v2
Timeout: 15s
Default Limit: 100 results
Enabled: tokens.discovery.jupiter.enabled

NOTE: Jupiter swap quotes/execution NOT handled by this client - see swaps module.

Endpoints Implemented (4 total):
  1. /tokens/v2/recent - Recent tokens
  2. /tokens/v2/toporganicscore/{interval} - Top organic score
  3. /tokens/v2/toptraded/{interval} - Top traded
  4. /tokens/v2/toptrending/{interval} - Top trending

Intervals: 5m, 1h, 6h, 24h

Primary Use Cases:
  - Token discovery (recent, trending, high-volume)
  - Market sentiment analysis
```

**5. CoinGecko Client** (`src/apis/coingecko/`)

```
Base URL: https://api.coingecko.com/api/v3
API Key: Demo tier (CG-tTkh1qKggtaa3zAt22G7bz63)
Timeout: 20s (large datasets, can be slow)
Enabled: tokens.discovery.coingecko.enabled &&
         tokens.discovery.coingecko.markets_enabled

Endpoints Implemented (1 total):
  1. /api/v3/coins/list?include_platform=true - All coins with platform addresses

Primary Use Cases:
  - Discover Solana tokens from global coin list
  - Extract platform addresses for token database
```

**6. DefiLlama Client** (`src/apis/defillama/`)

```
Base URL: https://api.llama.fi
Prices URL: https://coins.llama.fi/prices/current
Timeout: 25s (protocols endpoint has 6000+ entries)
Enabled: tokens.discovery.defillama.enabled &&
         tokens.discovery.defillama.protocols_enabled

Endpoints Implemented (2 total):
  1. /protocols - All DeFi protocols
  2. /prices/current/solana:{mint} - Current token price

Primary Use Cases:
  - Protocol discovery (DeFi ecosystem mapping)
  - Token price lookups (alternative to DEX APIs)
```

#### Rate Limiting Implementation

**Per-Client Rate Limiter** (in `client.rs`):

```
Mechanism:
  1. Semaphore with capacity 1 (serializes all requests)
  2. Minimum interval calculation: 60 seconds / max_per_minute
  3. Last request timestamp tracking
  4. Automatic sleep if interval not elapsed

Example:
  max_per_minute = 60  →  min_interval = 1s
  max_per_minute = 300 →  min_interval = 0.2s (200ms)

Flow:
  [Request] → Acquire Semaphore Permit
           → Check Last Request Time
           → Sleep if interval < min_interval
           → Update Last Request Time
           → Execute Request
           → Release Permit (via RAII guard)
```

**Configuration-Driven Limits:**

- DexScreener: `tokens.sources.dexscreener.rate_limit_per_minute` (default: 300)
- GeckoTerminal: `tokens.sources.geckoterminal.rate_limit_per_minute` (default: 30)
- Rugcheck: Hardcoded 60/min (no config override currently)
- Jupiter, CoinGecko, DefiLlama: No rate limiting (no semaphore)

**Why Per-Client Serialization:**

- Prevents parallel requests from exceeding API limits
- Simple and reliable (no complex token bucket)
- Works across all usages of the same client (true global limiting)

#### Statistics Tracking

**Per-Client Metrics** (in `stats.rs`):

```
ApiStats {
  total_requests: u64,           // Total API calls made
  successful_requests: u64,      // HTTP 2xx responses
  failed_requests: u64,          // HTTP errors, timeouts, parse failures
  cache_hits: u64,               // Local cache hits (before API call)
  cache_misses: u64,             // Cache misses (triggered API call)
  last_request_time: DateTime,   // Most recent request timestamp
  last_success_time: DateTime,   // Most recent successful response
  last_error_time: DateTime,     // Most recent error timestamp
  last_error_message: String,    // Last error message
  average_response_time_ms: f64  // Rolling average response time
}

Derived Metrics:
  - success_rate() → (successful / total) * 100
  - cache_hit_rate() → (hits / (hits + misses)) * 100

Collection:
  - Atomic counters for high-frequency metrics (requests, cache)
  - RwLock for low-frequency metadata (timestamps, errors)
  - Elapsed time tracked per request, averaged into cumulative
```

**Aggregated Stats Access:**

```rust
let apis = get_api_manager();
let all_stats = apis.get_all_stats().await;
// Returns ApiManagerStats with stats from all 6 clients

// Individual client stats
let dex_stats = apis.dexscreener.get_stats().await;
```

**Dashboard Integration:**

- Exposed via webserver at `/api/status` (as part of system status)
- Used for monitoring API health and rate limit compliance

#### Client Lifecycle & Initialization

**Singleton Initialization Flow:**

```
[First get_api_manager() Call] → LazyLock::new()
                                      │
                                      ▼
                        [Read Config] (get_config_clone())
                                      │
        ┌─────────────────────────────┼─────────────────────────────┐
        ▼                             ▼                             ▼
[Check Enabled Flags]     [Extract Rate Limits]       [Extract Timeouts]
        │                             │                             │
        └─────────────────────────────┴─────────────────────────────┘
                                      │
                                      ▼
                    [Create All 6 Clients with Config]
                                      │
                    ┌─────────────────┴─────────────────┐
                    ▼                                   ▼
        [Client Creation Fails?]                [Client Creation Succeeds]
                    │                                   │
                    ▼                                   │
        [Log Warning]                                   │
                    │                                   │
                    ▼                                   │
        [Create Disabled Fallback Client]              │
                    │                                   │
                    └───────────────┬───────────────────┘
                                    ▼
                    [Store in Global Arc<ApiManager>]
                                    │
                                    ▼
                            [Return Arc Clone]
```

**Fallback Strategy:**

- If any client fails to initialize (e.g., invalid config), a **disabled fallback client** is created
- Fallback client returns `ApiError::Disabled` for all requests
- Ensures bot doesn't crash if API config is invalid
- Logged as WARN with error details

**Enabled/Disabled Logic:**

Each client has its own enabled flag based on config:

```
DexScreener:   dexscreener.enabled && discovery.enabled && discovery.dexscreener.enabled
GeckoTerminal: geckoterminal.enabled && discovery.enabled && discovery.geckoterminal.enabled
Rugcheck:      rugcheck.enabled && discovery.enabled && discovery.rugcheck.enabled
Jupiter:       discovery.enabled && discovery.jupiter.enabled
CoinGecko:     discovery.enabled && discovery.coingecko.enabled && discovery.coingecko.markets_enabled
DefiLlama:     discovery.enabled && discovery.defillama.enabled && discovery.defillama.protocols_enabled
```

**Client Method Pattern:**

```rust
pub async fn fetch_something(&self) -> Result<T, ApiError> {
    if !self.enabled {
        return Err(ApiError::Disabled);
    }

    // Rate limiting
    let guard = self.rate_limiter.acquire().await?;

    // HTTP request
    let start = Instant::now();
    let response = self.http_client.client().get(url).send().await?;
    drop(guard);
    let elapsed = start.elapsed().as_millis() as f64;

    // Stats tracking
    if response.status().is_success() {
        self.stats.record_request(true, elapsed).await;
        Ok(response.json().await?)
    } else {
        self.stats.record_request(false, elapsed).await;
        self.stats.record_error(format!("HTTP {}", response.status())).await;
        Err(ApiError::InvalidResponse(...))
    }
}
```

#### Integration Points

**1. Token System Integration** (Primary Consumer):

```
[Tokens Discovery] → apis.dexscreener.fetch_token_profiles()
                  → apis.dexscreener.fetch_token_boosts()
                  → apis.jupiter.fetch_recent_tokens()
                  → apis.jupiter.fetch_top_trending()
                  → apis.coingecko.fetch_coins_list()
                  → apis.defillama.fetch_protocols()

[Token Market Data] → apis.dexscreener.fetch_token_batch(mints)
                    → apis.geckoterminal.fetch_token_pools(token)

[Token Security] → apis.rugcheck.get_token_report(mint)
```

**2. Pool Service Integration**:

```
[Pool Discovery] → tokens::prefetch_token_pools(mints)
                → tokens::get_token_pools_snapshot(mint)
                → tokens::get_token_pools_snapshot_allow_stale(mint)
```

**3. OHLCV Service Integration**:

```
[Candle Data] → apis.geckoterminal.fetch_ohlcv(pool, timeframe)
```

**4. Filtering System Integration**:

```
[Data Overlay] → Fetches missing API data during filter evaluation
               → Uses tokens module (which uses APIs internally)
```

**Note on Swaps:**

- Jupiter swap quotes/execution are handled in `src/swaps/jupiter.rs`
- GMGN swap quotes/execution are handled in `src/swaps/gmgn.rs`
- These are NOT part of the APIs module - they use direct HTTP calls
- Swaps module is a separate system focused on trade execution

#### Key Implementation Details

1. **Global Singleton Pattern**: LazyLock ensures ONE instance of ApiManager across entire bot
2. **Per-Client Serialization**: Semaphore (capacity=1) prevents concurrent requests to same API
3. **Automatic Rate Limiting**: Sleep-based throttling respects per-minute limits
4. **Config-Driven Behavior**: Rate limits, timeouts, and enabled flags read from config.toml
5. **Graceful Degradation**: Failed client initialization creates disabled fallback (no crash)
6. **Stats Collection**: Atomic counters + RwLock metadata for thread-safe tracking
7. **RAII Guards**: Rate limiter permits released automatically via drop
8. **Error Propagation**: Typed errors (ApiError) with source tracking
9. **Response Time Tracking**: Elapsed time computed per request, averaged cumulatively
10. **Cache Metrics**: Separate from request metrics (pre-API call logic)

#### Configuration Options

**Per-API Settings** (in `config.toml`):

```toml
[tokens.sources]
dexscreener.enabled = true
dexscreener.rate_limit_per_minute = 300
dexscreener.timeout_seconds = 10

geckoterminal.enabled = true
geckoterminal.rate_limit_per_minute = 30
geckoterminal.timeout_seconds = 10

rugcheck.enabled = true
# rate_limit and timeout hardcoded (60/min, 15s)

[tokens.discovery]
enabled = true

dexscreener.enabled = true
dexscreener.latest_profiles_enabled = true
dexscreener.latest_boosts_enabled = true

geckoterminal.enabled = true
geckoterminal.new_pools_enabled = true
geckoterminal.trending_enabled = true

jupiter.enabled = true
coingecko.enabled = true
coingecko.markets_enabled = true
defillama.enabled = true
defillama.protocols_enabled = true
rugcheck.enabled = true
```

**Timeout Defaults:**

- DexScreener: 10s (fast API)
- GeckoTerminal: 10s (moderate latency)
- Rugcheck: 15s (security analysis can be slow)
- Jupiter: 15s (token lists can be large)
- CoinGecko: 20s (large datasets)
- DefiLlama: 25s (protocols endpoint has 6000+ entries)

**Rate Limit Defaults:**

- DexScreener: 300/min (generous, conservative choice)
- GeckoTerminal: 30/min (moderate, official limit)
- Rugcheck: 60/min (reasonable for security checks)
- Jupiter, CoinGecko, DefiLlama: No rate limiting (unlimited or generous)

#### Critical Implementation Notes

1. **NOT for Swap Execution**: Jupiter/GMGN swap operations are in `src/swaps/`, NOT `src/apis/`
2. **Lazy Initialization**: ApiManager created on first `get_api_manager()` call (startup delay minimal)
3. **No Runtime Reconfiguration**: Rate limits/timeouts read at initialization, require restart to change
4. **Cache Hits Don't Count as Requests**: Only actual HTTP calls increment request counters
5. **Error Strings**: Last error stored as string for dashboard display (lossy, not typed)
6. **No Retry Logic in Clients**: Retries handled by consumers (e.g., tokens module has retry loops)
7. **Stats Thread-Safety**: Atomic counters + RwLock ensure safe concurrent access
8. **Response Time Averaging**: Cumulative average, NOT windowed (older requests have equal weight)
9. **Disabled State**: Checked at method entry, returns early without hitting rate limiter
10. **CoinGecko API Key**: Hardcoded demo tier key (CG-tTkh1qKggtaa3zAt22G7bz63)

### 11. Swaps Flow (Multi-Router DEX Execution with Fallback)

The swaps module provides a unified interface for executing token swaps across multiple DEX routers (GMGN and Jupiter) with intelligent quote comparison, automatic fallback, and comprehensive error handling.

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         SWAPS MODULE ARCHITECTURE                        │
│                    (NOT a Service - Direct Function Calls)               │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
        ┌───────────────────────────┼───────────────────────────┐
        ▼                           ▼                           ▼
┌──────────────┐          ┌──────────────┐          ┌──────────────┐
│    mod.rs    │          │   gmgn.rs    │          │ jupiter.rs   │
│  (Unified    │          │  (GMGN DEX   │          │  (Jupiter    │
│   Interface) │          │   Router)    │          │   Router)    │
│              │          │              │          │              │
│ - get_best_  │          │ - get_gmgn_  │          │ - get_       │
│   quote()    │          │   quote()    │          │   jupiter_   │
│ - get_best_  │          │ - execute_   │          │   quote()    │
│   quote_for_ │          │   gmgn_swap()│          │ - execute_   │
│   opening()  │          │ - gmgn_sign_ │          │   jupiter_   │
│ - execute_   │          │   and_send_  │          │   swap()     │
│   best_swap()│          │   transaction│          │ - jupiter_   │
│              │          │              │          │   sign_and_  │
│ Concurrent   │          │ MEV protect, │          │   send_trans │
│ quote fetch, │          │ 15s timeout, │          │              │
│ fallback on  │          │ retry x3     │          │ Fast quotes, │
│ failure      │          │              │          │ 15s timeout, │
│              │          │              │          │ retry x3     │
└──────────────┘          └──────────────┘          └──────────────┘
        │                           │                           │
        └───────────────────────────┴───────────────────────────┘
                                    │
                            ┌───────┴────────┐
                            ▼                ▼
                    ┌──────────────┐  ┌──────────────┐
                    │   types.rs   │  │   RPC Sign   │
                    │  (Common     │  │  & Send      │
                    │   Types)     │  │              │
                    │              │  │ - get_rpc_   │
                    │ - SwapResult │  │   client()   │
                    │ - SwapData   │  │ - sign_send_ │
                    │ - SwapQuote  │  │   and_       │
                    │ - RouterType │  │   confirm_   │
                    │ - Unified    │  │   transaction│
                    │   Quote      │  │              │
                    └──────────────┘  └──────────────┘
```

#### Architecture Overview

**Location:** `src/swaps/`

**Core Components:**

- `mod.rs` - Unified router interface, quote comparison, fallback logic
- `gmgn.rs` - GMGN router implementation with MEV protection
- `jupiter.rs` - Jupiter aggregator implementation
- `types.rs` - Shared types (SwapResult, SwapData, SwapQuote, RouterType, UnifiedQuote)

**Key Principle:** NOT a service - called directly by positions module during entry/exit operations. All functions are async and return typed results.

#### Quote Flow (Concurrent Multi-Router)

```
[get_best_quote()] → Concurrent Quote Requests
                          │
        ┌─────────────────┴─────────────────┐
        ▼                                   ▼
[GMGN Quote]                        [Jupiter Quote]
  (if enabled)                        (if enabled)
        │                                   │
        ▼                                   ▼
Build UnifiedQuote                  Build UnifiedQuote
  - router: GMGN                      - router: Jupiter
  - output_amount                     - output_amount
  - price_impact_pct                  - price_impact_pct
  - fee_lamports                      - fee_lamports
  - slippage_bps                      - slippage_bps
  - route_plan                        - route_plan
  - execution_data                    - execution_data
        │                                   │
        └─────────────────┬─────────────────┘
                          ▼
              future::join_all() - Wait for ALL
                          │
                          ▼
              Collect Successful Quotes
                          │
                          ▼
              If no quotes → Error("No routers available")
                          │
                          ▼
              Compare by output_amount (higher = better)
                          │
                          ▼
              Log comparison if multiple quotes
                          │
                          ▼
              Return Best Quote (UnifiedQuote)
```

**Critical Implementation:**

- **Concurrent Execution**: Quotes fetched in parallel via `future::join_all()`
- **True Comparison**: Both routers queried simultaneously, best selected by output amount
- **Enabled Check**: Only enabled routers queried (checked via `with_config()`)
- **Error Handling**: Failures logged but don't block other routers
- **Best Selection**: `max_by_key(|q| q.output_amount)` selects highest output

#### Execution Flow (With Automatic Fallback)

```
[execute_best_swap()] → Execute Primary Router
                          │
                          ▼
              Match quote.execution_data
                          │
        ┌─────────────────┴─────────────────┐
        ▼                                   ▼
[QuoteExecutionData::GMGN]      [QuoteExecutionData::Jupiter]
        │                                   │
        ▼                                   ▼
execute_gmgn_swap()                 execute_jupiter_swap()
  - Sign transaction                  - Sign transaction
  - Send to blockchain                - Send to blockchain
  - Confirm & verify                  - Confirm & verify
        │                                   │
        └─────────────────┬─────────────────┘
                          ▼
              Check Primary Result
                          │
        ┌─────────────────┴─────────────────┐
        ▼                                   ▼
    [Success]                          [Error]
        │                                   │
        ▼                                   │
Return SwapResult                           ▼
  - success: true                   Check Error Type:
  - router_used                     - TransactionDropped (not propagated)
  - transaction_signature           - TransactionDropped (dropped)
  - input/output_amount             - Network error
  - price_impact                           │
  - fee_lamports                           ▼
  - execution_time                  Should Fallback?
  - effective_price                        │
                                   ┌───────┴───────┐
                                   ▼               ▼
                               [Yes]            [No]
                                   │               │
                                   ▼               ▼
                    Get Fallback Quote    Return Primary Error
                           │
        ┌──────────────────┴──────────────────┐
        ▼                                     ▼
[Primary: Jupiter]                   [Primary: GMGN]
  Fallback: GMGN                       Fallback: Jupiter
        │                                     │
        ▼                                     ▼
get_gmgn_quote()                      get_jupiter_quote()
        │                                     │
        ▼                                     ▼
execute_gmgn_swap()                   execute_jupiter_swap()
        │                                     │
        └──────────────────┬──────────────────┘
                           ▼
              Check Fallback Result
                           │
        ┌──────────────────┴──────────────────┐
        ▼                                     ▼
    [Success]                             [Failed]
        │                                     │
        ▼                                     ▼
Return SwapResult                   Return Primary Error
  - success: true                   (not fallback error)
  - router_used: <fallback>
  - transaction_signature
```

**Fallback Strategy:**

- **Trigger Conditions**: TransactionDropped, Network errors (NOT all errors)
- **Automatic Alternative**: Jupiter fails → Try GMGN, GMGN fails → Try Jupiter
- **Quote Re-fetch**: New quote obtained from fallback router with same parameters
- **Error Precedence**: If fallback also fails, return original primary error (not fallback)
- **Smart Slippage**: Converts slippage_bps back to percentage for fallback quote

#### Router-Specific Implementations

**GMGN Router** (`gmgn.rs`):

```
Configuration (from config.toml):
  • enabled: bool = true
  • quote_api: "https://gmgn.ai/defi/router/v1/sol/tx/get_swap_route"
  • partner: "screenerbot"
  • anti_mev: bool = false
  • fee_sol: f64 = 0.0
  • default_swap_mode: "ExactIn"

Constants:
  • QUOTE_TIMEOUT_SECS: 15 (GMGN can be slower)
  • RETRY_ATTEMPTS: 3

Quote Flow:
  [get_gmgn_quote()] → Build URL with params
                          │
                          ▼
                  HTTP GET with timeout (15s)
                          │
                          ▼
                  Retry up to 3 times
                          │
                          ▼
                  Parse GMGNApiResponse
                          │
                          ▼
                  Check response.code
                          │
        ┌─────────────────┴─────────────────┐
        ▼                                   ▼
    [code=0]                           [code≠0]
        │                                   │
        ▼                                   │
Extract SwapData                            ▼
  - quote (amounts, slippage, impact)   [code=40000402]
  - raw_tx (transaction, blockhash)         │
  - amount_in_usd, amount_out_usd           ▼
  - jito_order_id                    "No route" error
  - sol_cost                         (terminal, no retry)
        │                                   │
        ▼                                   ▼
Return SwapData                     Return ScreenerBotError

Execution Flow:
  [execute_gmgn_swap()] → Extract swap_transaction from SwapData
                          │
                          ▼
                  gmgn_sign_and_send_transaction()
                          │
                          ▼
                  RPC: sign_send_and_confirm_transaction()
                          │
                          ▼
                  Return GMGNSwapResult
                    - success: true
                    - transaction_signature
                    - input/output amounts
                    - price_impact
                    - fee_lamports
                    - execution_time
                    - effective_price
                    - swap_data (full)
```

**Jupiter Router** (`jupiter.rs`):

```
Configuration (from config.toml):
  • enabled: bool = true
  • quote_api: "https://lite-api.jup.ag/swap/v1/quote"
  • swap_api: "https://lite-api.jup.ag/swap/v1/swap"
  • dynamic_compute_unit_limit: bool = false
  • default_priority_fee: u64 = 1000
  • default_swap_mode: "ExactIn"

Constants:
  • QUOTE_TIMEOUT_SECS: 15 (Jupiter is fast)
  • API_TIMEOUT_SECS: 20 (includes execution)
  • RETRY_ATTEMPTS: 3

Quote Flow:
  [get_jupiter_quote()] → Build query params
                          │
                          ▼
                  HTTP GET quote_api with timeout (15s)
                          │
                          ▼
                  Retry up to 3 times
                          │
                          ▼
                  Parse JupiterQuoteResponse
                          │
                          ▼
                  POST swap_api with quote
                          │
                          ▼
                  Parse JupiterSwapResponse
                          │
                          ▼
                  Fetch token decimals
                          │
                          ▼
                  Build SwapData
                    - quote (from quote response)
                    - raw_tx (from swap response)
                          │
                          ▼
                  Return SwapData

Execution Flow:
  [execute_jupiter_swap()] → Extract swap_transaction from SwapData
                          │
                          ▼
                  jupiter_sign_and_send_transaction()
                    (includes priority_fee_lamports, compute_unit_limit)
                          │
                          ▼
                  RPC: sign_send_and_confirm_transaction()
                          │
                          ▼
                  Return JupiterSwapResult
                    - success: true
                    - transaction_signature
                    - input/output amounts
                    - price_impact
                    - fee_lamports
                    - execution_time
                    - effective_price
                    - swap_data (full)
```

#### Special Quote Functions

**1. get_best_quote_for_opening()** (With Route Failure Tracking):

```
[get_best_quote_for_opening()] → Call get_best_quote()
                          │
                          ▼
              Check if error is "no route"
                          │
        ┌─────────────────┴─────────────────┐
        ▼                                   ▼
    [Success]                         [No Route Error]
        │                                   │
        ▼                                   │
Return UnifiedQuote                         ▼
                        Track in tokens::database::blacklist_token()
                                    │
                                    ▼
                        Add to blacklist with reason "NoRoute"
                                    │
                                    ▼
                        Log tracking event
                                    │
                                    ▼
                        Return error (propagate to caller)
```

**Error Detection Patterns:**

- `error.contains("no route")`
- `error.contains("No routers available for quote")`
- `error.contains("jupiter has no route")`
- `error.contains("Jupiter API error: 400")`
- `error.contains("400 Bad Request")`
- `error.contains("Jupiter") && error.contains("400")`

**Purpose:** Prevents repeated attempts to trade tokens with no liquidity/routes by auto-blacklisting after 5 failures.

**2. get_best_quote()** (Standard Multi-Router Quote):

- No blacklisting logic
- Used for closing positions (exits must always attempt)
- Supports custom swap_mode ("ExactIn" or "ExactOut")

#### Integration Points

**1. Positions Module Integration** (Primary Consumer):

```
[open_position_direct()] → get_best_quote_for_opening(
                              SOL_MINT,
                              token_mint,
                              sol_to_lamports(trade_size_sol),
                              wallet_address,
                              slippage_quote_default,
                              token_symbol
                           )
                          │
                          ▼
                     execute_best_swap(
                              token,
                              SOL_MINT,
                              token_mint,
                              sol_to_lamports(trade_size_sol),
                              quote
                           )
                          │
                          ▼
                     Extract transaction_signature
                          │
                          ▼
                     Save position with signature
```

```
[close_position_direct()] → get_best_quote(
                              token_mint,
                              SOL_MINT,
                              sell_amount,
                              wallet_address,
                              exit_retry_steps[0],  // 3.0% default
                              "ExactIn"  // Spend exact token amount
                           )
                          │
                          ▼
                     execute_best_swap(
                              token,
                              token_mint,
                              SOL_MINT,
                              sell_amount,
                              quote
                           )
                          │
                          ▼
                     Extract transaction_signature
                          │
                          ▼
                     Update position with exit signature
```

**2. RPC Integration**:

```
[GMGN/Jupiter] → gmgn_sign_and_send_transaction(base64_tx)
                 jupiter_sign_and_send_transaction(base64_tx, priority_fee, compute_limit)
                          │
                          ▼
                 get_rpc_client() (centralized RPC client)
                          │
                          ▼
                 sign_send_and_confirm_transaction()
                   - Deserialize base64 → Transaction
                   - Sign with wallet keypair
                   - Send to blockchain
                   - Confirm (wait for finalization)
                          │
                          ▼
                 Return signature string
```

**3. Tokens Module Integration**:

```
[Swaps] → tokens::get_decimals(mint)
            (needed for amount conversions)
          tokens::database::blacklist_token(mint, reason, db)
            (auto-blacklist on repeated no-route errors)
```

**4. Config Integration**:

```
[Swaps] → with_config(|cfg| cfg.swaps.gmgn.enabled)
          with_config(|cfg| cfg.swaps.jupiter.enabled)
          with_config(|cfg| cfg.swaps.slippage.quote_default_pct)
          with_config(|cfg| cfg.swaps.slippage.exit_retry_steps_pct)
          with_config(|cfg| cfg.trader.trade_size_sol)
```

#### Key Configuration Options

**GMGN Router:**

- `enabled`: Enable GMGN router (default: true)
- `quote_api`: API endpoint (default: gmgn.ai)
- `partner`: Partner identifier (default: "screenerbot")
- `anti_mev`: MEV protection (default: false)
- `fee_sol`: Platform fee (default: 0.0)
- `default_swap_mode`: ExactIn or ExactOut (default: "ExactIn")

**Jupiter Router:**

- `enabled`: Enable Jupiter router (default: true)
- `quote_api`: Quote endpoint (default: lite-api.jup.ag)
- `swap_api`: Swap endpoint (default: lite-api.jup.ag)
- `dynamic_compute_unit_limit`: Let Jupiter calculate CU (default: false)
- `default_priority_fee`: Fee in lamports (default: 1000)
- `default_swap_mode`: ExactIn or ExactOut (default: "ExactIn")

**Raydium Router:**

- `enabled`: Direct Raydium swaps (default: false, NOT IMPLEMENTED)

**Slippage:**

- `quote_default_pct`: Default slippage for quotes (default: 1.0%)
- `exit_profit_shortfall_pct`: Exit slippage for profit (default: 3.0%)
- `exit_loss_shortfall_pct`: Exit slippage for loss (default: 5.0%)
- `exit_retry_steps_pct`: Retry slippage steps (default: [3.0, 10.0, 25.0])

#### Critical Implementation Details

1. **NOT a Service**: Swaps module is a library of functions, NOT a managed service. No startup, no background tasks.
2. **Concurrent Quote Fetching**: Both routers queried in parallel via `future::join_all()` for true comparison.
3. **Best Route Selection**: Highest `output_amount` wins (more tokens received = better rate).
4. **Automatic Fallback**: Primary router failure triggers fallback to alternative (if enabled and error is retryable).
5. **Error-Specific Fallback**: Only TransactionDropped and Network errors trigger fallback (NOT all errors).
6. **Fallback Quote Re-fetch**: Fallback router gets fresh quote with same input parameters.
7. **Error Precedence**: Primary error returned if fallback also fails (preserves original failure context).
8. **No Route Blacklisting**: `get_best_quote_for_opening()` auto-blacklists tokens with repeated no-route errors.
9. **ExactIn vs ExactOut**: Opening uses ExactIn (spend exact SOL), closing uses ExactIn (sell exact tokens to avoid ATA mismatch).
10. **Retry Logic**: Both routers retry up to 3 times with no backoff (fast retries for transient API issues).
11. **Timeout Strategy**: GMGN 15s (slower), Jupiter 15s quote + 20s swap (includes execution).
12. **Terminal Errors**: "No route" errors (code 40000402 for GMGN, 400 for Jupiter) don't retry.
13. **RPC Centralization**: All signing/sending via `get_rpc_client()` (respects rate limits, round-robin, 429 backoff).
14. **Decimals Dependency**: Both routers fetch token decimals via tokens module for amount conversions.
15. **Comprehensive Logging**: Every step logged with LogTag::Swap for debugging and monitoring.

#### Data Types

**UnifiedQuote** (Comparison Structure):

```rust
pub struct UnifiedQuote {
    router: RouterType,              // GMGN or Jupiter
    input_mint: String,
    output_mint: String,
    input_amount: u64,
    output_amount: u64,              // Key comparison field
    price_impact_pct: f64,
    fee_lamports: u64,
    slippage_bps: u16,
    route_plan: String,
    execution_data: QuoteExecutionData,  // Router-specific data
    swap_mode: String,
}
```

**SwapResult** (Execution Result):

```rust
pub struct SwapResult {
    success: bool,
    router_used: Option<RouterType>,  // Which router executed
    transaction_signature: Option<String>,
    input_amount: String,
    output_amount: String,
    price_impact: String,
    fee_lamports: u64,
    execution_time: f64,
    effective_price: Option<f64>,    // Price per token in SOL
    swap_data: Option<SwapData>,     // Full response for reference
    error: Option<String>,
}
```

**SwapData** (API Response):

```rust
pub struct SwapData {
    quote: SwapQuote,                // Quote details
    raw_tx: RawTransaction,          // Transaction to sign
    amount_in_usd: Option<String>,
    amount_out_usd: Option<String>,
    jito_order_id: Option<String>,
    sol_cost: Option<String>,
}
```

#### Public API Summary

**Quote Functions:**

- `get_best_quote(input_mint, output_mint, amount, from, slippage, swap_mode)` → UnifiedQuote
- `get_best_quote_for_opening(input_mint, output_mint, amount, from, slippage, symbol)` → UnifiedQuote (with blacklisting)

**Execution Functions:**

- `execute_best_swap(token, input_mint, output_mint, amount, quote)` → SwapResult (with fallback)

**Router-Specific Functions:**

- `get_gmgn_quote(...)` → SwapData
- `execute_gmgn_swap(...)` → GMGNSwapResult
- `gmgn_sign_and_send_transaction(base64_tx)` → signature

- `get_jupiter_quote(...)` → SwapData
- `execute_jupiter_swap(...)` → JupiterSwapResult
- `jupiter_sign_and_send_transaction(base64_tx, priority_fee, compute_limit)` → signature

**Type Exports:**

- `RouterType`, `SwapResult`, `SwapData`, `SwapQuote`, `RawTransaction`, `UnifiedQuote`
- `GMGNApiResponse`, `JupiterQuoteResponse`, `JupiterSwapResponse`

### 12. Webserver Flow (Comprehensive REST API + Dashboard)

The webserver provides a complete web-based dashboard for monitoring and controlling the bot, built on Axum with a modular ES module frontend architecture.

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         WEBSERVER SERVICE                               │
│                (Priority 30, Depends: filtering)                        │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
        ┌───────────────────────────┼───────────────────────────┐
        ▼                           ▼                           ▼
┌──────────────┐          ┌──────────────┐          ┌──────────────┐
│   Service    │          │    Axum      │          │   AppState   │
│  (Priority   │          │   Server     │          │  (Minimal)   │
│    30)       │          │              │          │              │
│              │          │ - TCP        │          │ - startup_   │
│ Spawns 1     │          │   Listener   │          │   time       │
│ Task         │          │ - Graceful   │          │ - Service    │
│ (Monitored)  │          │   Shutdown   │          │   Manager    │
│              │          │ - Compress   │          │   Helpers    │
│ 200ms Init   │          │   Layer      │          │              │
│ Delay        │          │              │          │              │
└──────────────┘          └──────────────┘          └──────────────┘
        │                           │                           │
        └───────────────────────────┴───────────────────────────┘
                                    │
                              127.0.0.1:8080
```

#### Service Implementation Details

**Location:** `src/services/implementations/webserver_service.rs`

**Lifecycle:**

```
[Service Start] → Spawn Instrumented Task
                          │
                          ▼
                  Get CLI Overrides
                  (arguments::get_port_override())
                  (arguments::get_host_override())
                          │
                          ▼
          [PRE-FLIGHT CHECK: Test Port Binding]
          webserver::test_port_binding(port, host)
                          │
                ┌─────────┴─────────┐
                ▼                   ▼
            SUCCESS             FAILURE
         (Port Free)         (AddrInUse,
                           PermissionDenied)
                │                   │
                │                   ▼
                │           Return Err() → Bot Exits
                │
                ▼
          Drop Test Listener
                │
                ▼
          Spawn Background Task
                │
                ▼
          crate::webserver::start_server(port, host)
                │
                ▼
          Apply Precedence Logic:
          CLI --port > config.webserver.port > 8080
          CLI --host > config.webserver.host > "127.0.0.1"
                │
                ▼
          Create AppState (startup_time)
                │
                ▼
          Set Global AppState (once_cell)
                │
                ▼
          Build Router (routes + middleware)
                │
                ▼
          Bind TCP Listener (resolved address)
                │
                ▼
          Serve with Graceful Shutdown
          (await SHUTDOWN_NOTIFY signal)
                │
                ▼
        [200ms delay for initialization]
                │
                ▼
        [Service Ready - Log endpoint URL]
```

**Port/Host Configuration:**

- **Port Precedence**: CLI `--port` > `config.webserver.port` > 8080 default
- **Host Precedence**: CLI `--host` > `config.webserver.host` > "127.0.0.1" default
- **GUI Mode**: Pre-flight check skipped when no explicit CLI overrides; uses dynamic port (49152-65535)
- **CLI Arguments**: Validated early in `run.rs` before service initialization

**Error Handling:**

- **Invalid CLI Arguments** (`--port abc`, `--port 70000`): Caught in `run.rs` early validation, bot exits immediately with error
- **Port Already in Use**: Caught by pre-flight check (`test_port_binding()`), bot exits with "Address already in use" error
- **Permission Denied** (ports <1024): Caught by pre-flight check, bot exits with "Permission denied" error
- **Background Server Errors**: Logged but don't crash the bot (handled gracefully)

**Pre-Flight Check (Port Binding Test):**

1. Calls `webserver::test_port_binding(port_override, host_override)` BEFORE spawning background task
2. Attempts `TcpListener::bind()` with resolved address
3. If bind fails (AddrInUse, PermissionDenied, etc.): Returns `Err()` → ServiceManager stops initialization → Bot exits
4. If bind succeeds: Drops listener immediately, continues to background task spawn

**Logging:**

- **Early Validation**: `[SYSTEM] [ERROR] Argument validation failed: ...`
- **Pre-Flight Check**: `[SYSTEM] [DEBUG] [PRE-FLIGHT] Testing port binding: <host>:<port>`
- **Bind Test**: `[WEBSERVER] [DEBUG] [TEST-BIND] Successfully bound to <host>:<port>`
- **Server Start**: `[WEBSERVER] [INFO] Starting webserver on <host>:<port> [source: CLI/config/default]`
- **Service Ready**: `[SYSTEM] [INFO] service_notice service=webserver kind=ready endpoint=http://<host>:<port>`

**Shutdown Mechanism:**

- Global `Arc<Notify>` via `once_cell::sync::Lazy`
- `shutdown()` function triggers `notify_one()`
- Axum server awaits signal before stopping

**Middleware Stack:**

1. `CompressionLayer` (tower-http) - Compresses responses

**AppState Access Patterns:**

- Services accessed via `get_service_manager()` (no direct imports)
- Helper methods: `get_all_services()`, `get_service_health()`, `get_all_services_health()`, `get_service_metrics()`, `get_service_details()`
- Uptime tracking via `uptime_seconds()`

#### Router Architecture (Modular)

**Root Routes (HTML Pages - 7 Total):**

```
/                          → Services page (default)
/services                  → Services monitoring
/tokens                    → Token browser
/positions                 → Position tracking
/events                    → Event log viewer
/transactions              → Transaction history
/filtering                 → Filtering engine stats
/config                    → Configuration editor
```

**Script Routes (ES Modules):**

```
/scripts/core/:file        → Core framework scripts (7 files)
/scripts/pages/:file       → Page-specific scripts (7 files)
/scripts/ui/:file          → UI component scripts (4 files)
```

**API Routes (16 Modules Merged):**

```
/api/
  ├─ health                → Health check
  ├─ status                → System status snapshot
  ├─ status/services       → Service health
  ├─ status/metrics        → System metrics
  │
  ├─ tokens/list           → Token list (paginated)
  ├─ tokens/stats          → Token statistics
  ├─ tokens/filter         → Filter tokens (POST)
  ├─ tokens/:mint          → Token details
  ├─ tokens/:mint/ohlcv    → Token candlestick data
  │
  ├─ positions             → Position list
  ├─ positions/stats       → Position statistics
  ├─ positions/:key/details → Position details
  ├─ positions/:mint/debug → Position debug info
  │
  ├─ config                → Full config (all sections)
  ├─ config/rpc            → RPC config (GET)
  ├─ config/trader         → Trader config (GET/PATCH)
  ├─ config/positions      → Positions config (GET/PATCH)
  ├─ config/filtering      → Filtering config (GET/PATCH)
  ├─ config/swaps          → Swaps config (GET/PATCH)
  ├─ config/tokens         → Tokens config (GET/PATCH)
  ├─ config/sol_price      → SOL price config (GET/PATCH)
  ├─ config/summary        → Summary config (GET)
  ├─ config/events         → Events config (GET/PATCH)
  ├─ config/services       → Services config (GET/PATCH)
  ├─ config/monitoring     → Monitoring config (GET/PATCH)
  ├─ config/ohlcv          → OHLCV config (GET/PATCH)
  ├─ config/metadata       → UI metadata (for rendering)
  ├─ config/reload         → Reload from disk (POST)
  ├─ config/reset          → Reset to defaults (POST)
  │
  ├─ services              → Service list + overview
  ├─ services/:name/details → Service details
  │
  ├─ events/list           → Event list (paginated)
  ├─ events/stats          → Event statistics
  │
  ├─ filtering/refresh     → Trigger filtering refresh
  ├─ filtering/stats       → Filtering statistics
  │
  ├─ wallet/current        → Current wallet balances
  ├─ wallet/dashboard      → Wallet dashboard data
  ├─ wallet/flow/cache     → Wallet flow cache
  ├─ wallet/cache/metrics  → Wallet cache metrics
  │
  ├─ blacklist/stats       → Blacklist statistics
  │
  ├─ ohlcv/*               → OHLCV data endpoints
  │
  ├─ /trading/*            → Trading routes (nested)
  ├─ /trader/*             → Trader control (nested)
  ├─ /system/*             → System control (nested)
  ├─ /transactions/*       → Transaction routes (nested)
  │
  └─ pages/:page           → SPA page content (HTML fragments)
```

**Route Module Breakdown (16 Total):**

1. `status.rs` - Health, system status, metrics
2. `tokens.rs` - Token browsing, details, OHLCV
3. `positions.rs` - Position tracking, stats, details
4. `config.rs` - Configuration CRUD + metadata
5. `services.rs` - Service management
6. `events.rs` - Event log queries
7. `filtering_api.rs` - Filtering engine control
8. `wallet.rs` - Wallet balance/flow data
9. `blacklist.rs` - Blacklist statistics
10. `ohlcv.rs` - Candlestick data
11. `trading.rs` - Trading configuration
12. `trader.rs` - Trader control
13. `system.rs` - System reboot/control
14. `transactions.rs` - Transaction history
15. `dashboard.rs` - Dashboard overview
16. `mod.rs` - Router wiring + page handlers

**Response Type Convention:**

- All response types are **INLINE** in route files (no separate models folder)
- Use `success_response()` and `error_response()` from `webserver/utils.rs`
- Standard format: `{ "data": {...}, "timestamp": "..." }`

#### Frontend Architecture (ES Modules + Lifecycle Pattern)

**Structure Overview:**

```
templates/
├── base.html                    → Shell template (nav, content injection)
├── pages/                       → HTML structure only (7 pages)
│   ├── services.html
│   ├── tokens.html
│   ├── positions.html
│   ├── events.html
│   ├── transactions.html
│   ├── filtering.html
│   └── config.html
├── scripts/
│   ├── core/                    → Framework (7 modules)
│   │   ├── lifecycle.js         → Page lifecycle registry (init/activate/deactivate/dispose)
│   │   ├── app_state.js         → LocalStorage state wrapper
│   │   ├── poller.js            → Polling abstraction with start/stop/pause
│   │   ├── dom.js               → DOM utilities ($, $$, etc.)
│   │   ├── utils.js             → Shared helpers (formatters, validators, etc.)
│   │   ├── router.js            → SPA navigation (intercepts data-page links)
│   │   └── header.js            → Header component management
│   ├── pages/                   → Page logic (7 modules)
│   │   ├── services.js          → Services page lifecycle
│   │   ├── tokens.js            → Tokens page lifecycle
│   │   ├── positions.js         → Positions page lifecycle
│   │   ├── events.js            → Events page lifecycle
│   │   ├── transactions.js      → Transactions page lifecycle
│   │   ├── filtering.js         → Filtering page lifecycle
│   │   └── config.js            → Config page lifecycle
│   ├── ui/                      → Reusable components (4 modules)
│   │   ├── data_table.js        → Data table component
│   │   ├── table_toolbar.js     → Table toolbar component
│   │   ├── events_dialog.js     → Event details dialog
│   │   └── tab_bar.js           → Tab bar component
│   └── theme.js                 → Theme management
└── styles/
    ├── foundation.css           → CSS variables, resets
    ├── layout.css               → Grid, flex layouts
    ├── components.css           → Buttons, tables, chips
    ├── common.css               → Shared styles
    ├── pages/                   → Per-page styles (7 files)
    └── ui/                      → Per-component styles (4 files)
```

**Lifecycle Pattern (Core Contract):**

Each page module exports `createLifecycle()` returning:

```javascript
{
  init(ctx) {
    // One-time setup (called once per page load)
    // ctx: { pageName, data, isActive(), onDeactivate(), onDispose(), managePoller() }
  },

  activate(ctx) {
    // Start/resume (called on navigation to page)
    // Start pollers, attach event listeners
  },

  deactivate() {
    // Pause (called on navigation away)
    // Stop pollers, cleanup temporary state
  },

  dispose() {
    // Full cleanup (called on page unload)
    // Remove all resources
  }
}
```

**Router Flow (SPA Navigation):**

```
[User Clicks Link with data-page="tokens"]
                │
                ▼
        Router intercepts click
                │
                ▼
        Deactivate current page lifecycle
                │
                ▼
        Fetch HTML from /api/pages/tokens
                │
                ▼
        Load page module (scripts/pages/tokens.js)
                │
                ▼
        Initialize if first time (init hook)
                │
                ▼
        Inject HTML into #app-content
                │
                ▼
        Activate page lifecycle (activate hook)
                │
                ▼
        Update nav tabs (active state)
                │
                ▼
        [Page Ready - Pollers started, UI live]
```

#### Bootstrap Readiness Coordination (Jan 2026)

**Goal:** keep both the desktop shell and SPA from hammering API routes while the backend is still booting or waiting for the very first wallet snapshot.

**Backend Aggregation (`src/webserver/routes/system.rs`):**

- `/api/system/bootstrap` now surfaces a consolidated `BootStatusResponse` with `ready_for_requests`, `initialization_required`, `phase`, `message`, `pending_services`, `wallet_snapshot_ready`, optional `wallet_snapshot`, and a server-driven `retry_after_ms` poll hint.
- Status derives from `global::INITIALIZATION_COMPLETE`, `global::requires_initialization()`, `global::are_core_services_ready()`, and the cached ServiceManager health list so we can tell the UI exactly which services remain pending.
- Wallet readiness piggybacks on `wallet::get_current_wallet_status()`; once a snapshot lands, the UI stops showing “waiting for wallet data”.
- Middleware explicitly allows `/api/system/bootstrap` before the normal initialization gate so headless clients can always see progress.

**Desktop/Electron Handshake:**

- `wait_for_bootstrap_ready()` polls the bootstrap endpoint (200 ms cadence) before the legacy HTML polling loop. The main window remains hidden until either `ready_for_requests` turns true or onboarding (`initialization_required`) is triggered, eliminating “blank dashboard” flashes during heavy startups.
- Detailed logs (attempt count + server-provided message) land under `LogTag::System`, giving operators fast visibility into which subsystem is slowing startup.

**Frontend Bootstrap Module (`templates/scripts/core/bootstrap.js`):**

- New core module exported by `templates.rs` provides `waitForReady()` and `subscribeToBootstrap()`. It polls the backend using the server-provided `retry_after_ms`, caches the last-good payload, emits a `bootstrap-ready` DOM event once, and quietly retries on fetch errors.
- Consumers receive structured updates (phase, pending services, wallet readiness) so they can surface precise messaging in the UI.

**Current Consumers (Jan 2026 sweep):**

1. `router.js` waits for `waitForReady()` before bootstrapping the SPA or requesting `/api/pages/...`, guaranteeing that the first HTML fetch happens only after services are live.
2. `header.js` subscribes to bootstrap updates to drive the new “booting” badge state, disable trader controls until ready, and delay trader status/metrics pollers until the backend is responsive.
3. `notifications.js` defers establishing the `/api/actions/stream` SSE channel and the initial active-action sync until readiness to avoid 503 storms and reconnect loops.
4. Any other module (future manual trading panels, etc.) can now `await waitForReady()` instead of reinventing readiness checks.

This handshake makes startup deterministic: backend aggregates readiness, Electron blocks on the signal, and the SPA won’t issue heavy RPC-backed requests until the system (or onboarding wizard) is actually usable.

**Onboarding Override Flag (Dec 2025)**

- Operators can now pass `--dashboard-onboarding` when launching the bot to force the splash bootstrap into the onboarding tour even if the stored onboarding state is already complete.
- `SplashController` caches the backend’s `force_onboarding` indicator from `/api/initialization/status` and exposes `needsSetupAfterOnboarding()` so downstream controllers know whether the setup wizard should reopen.
- When the override flag is used after initialization, `OnboardingController` dismisses straight into the live dashboard instead of relaunching the setup flow, making repeat demos lightweight without touching config files.

**Poller Management:**

- Pages use `ctx.managePoller(poller)` for automatic start/stop
- Pollers pause on `deactivate()`, resume on `activate()`
- Cleanup on `dispose()` - no manual stop needed

**State Management:**

- `app_state.js` wraps `localStorage` with JSON serialization
- Methods: `get(key, default)`, `set(key, value)`, `remove(key)`, `clear()`
- Used for persisting UI preferences (pagination, filters, etc.)

#### Configuration UI (Metadata-Driven)

**System Overview:**

```
[Config Page Load] → GET /api/config/metadata
                          │
                          ▼
                  Receive ConfigMetadata (BTreeMap<section, fields>)
                          │
                          ▼
                  For Each Section:
                          │
        ┌─────────────────┼─────────────────┬─────────────────┐
        ▼                 ▼                 ▼                 ▼
   FieldType       FieldMetadata      Render Logic      Default Value
   (Boolean,       (label, hint,      (text input,      (from schema)
    Number,         unit, impact,      checkbox,
    Integer,        category,          JSON editor)
    String,         min/max/step,
    Array,          placeholder,
    Object)         docs)
        │                 │                 │                 │
        └─────────────────┴─────────────────┴─────────────────┘
                                    │
                          [Dynamic Form Rendered]
```

**Metadata Fields:**

- `type` - FieldType (boolean, number, integer, string, array, object)
- `item_type` - Type of array elements (optional)
- `label` - Display name
- `hint` - Tooltip/help text
- `unit` - Display unit (SOL, %, seconds, etc.)
- `impact` - Severity (critical, high, medium, low)
- `category` - Grouping (General, Developer, Advanced - normalized)
- `min/max/step` - Numeric constraints
- `placeholder` - Input placeholder
- `docs` - Inline documentation (from `///` comments)
- `default` - Default value (JSON serialized)
- `children` - Nested object metadata (recursive)

**Update Flow (PATCH Endpoints):**

```
[User Edits Field] → Validate in Frontend
                          │
                          ▼
                  PATCH /api/config/:section
                  (JSON with only changed fields)
                          │
                          ▼
                  Backend merges changes:
                  1. Get current section as JSON
                  2. Merge updates (only provided fields)
                  3. Deserialize to validate
                  4. Update config via update_config_section()
                  5. Save to disk (data/config.toml)
                          │
                          ▼
                  Return success response
                          │
                          ▼
                  [Config Hot-Reloaded - Next reads see new values]
```

**Exposed Config Sections (11 Total):**

- rpc, trader, positions, filtering, swaps, tokens, sol_price, events, services, monitoring, ohlcv
- **NOT exposed:** pools, wallet (internal service configs)

**Config Operations:**

- `GET /api/config/:section` - View section
- `PATCH /api/config/:section` - Partial update (JSON merge)
- `POST /api/config/reload` - Reload from disk
- `POST /api/config/reset` - Reset to defaults (dangerous!)
- `GET /api/config/metadata` - UI rendering metadata
- `GET /api/config/diff` - Diff from defaults

#### Integration Points

**1. Service Manager Integration:**

- AppState has helper methods to access ServiceManager
- No direct service imports - all via `get_service_manager().await`
- Cached health/metrics every 5s for fast dashboard reads
- Live reads available but slower

**2. Config System Integration:**

- Config UI reads metadata from `config::metadata::collect_config_metadata()`
- Updates go through `update_config_section()` with validation
- Hot reload via `reload_config()` - no restart needed

**3. Token System Integration:**

- Token list endpoint reads from `tokens::list_tokens_async()`
- Token details from `tokens::get_full_token_async()`
- OHLCV data from OHLCV service

**4. Position System Integration:**

- Position list from `positions::state::get_open_positions()`
- Position stats via database queries
- Debug info includes verification queue status

**5. Events System Integration:**

- Event list queries events.db
- Paginated with filters (type, category, time range)

**6. Wallet Integration:**

- Current balance from `wallet::get_balance_data()`
- Flow metrics from wallet.db queries

**7. Transaction System Integration:**

- Transaction history from transactions.db
- Analysis data (swap details, balance changes)

#### Critical Implementation Details

1. **Template Embedding:** All HTML/CSS/JS embedded at compile time via `include_str!`
2. **No WebSocket:** Pure REST polling - no real-time push (uses Poller abstraction)
3. **Global State:** AppState stored in `once_cell::sync::OnceCell` for non-handler access
4. **Shutdown Coordination:** Global `Arc<Notify>` allows external shutdown trigger
5. **Response Helpers:** `success_response()` and `error_response()` wrap JSON with timestamps
6. **Zero Hardcoded Forms:** Config UI is 100% metadata-driven from schema annotations
7. **Script Serving:** ES modules served via dynamic routes (not static file server)
8. **SPA Page Content:** `/api/pages/:page` endpoint returns HTML fragments for router
9. **XSS Protection:** Page name escaping in 404 handler prevents XSS
10. **Compression:** All responses compressed via `CompressionLayer` middleware
11. **Service Access Pattern:** AppState methods → ServiceManager → Individual services (never direct imports)
12. **Frontend Hygiene:** Zero inline JS/CSS in HTML, all in dedicated files
13. **Lifecycle Cleanup:** Pollers auto-managed via `ctx.managePoller()` - no memory leaks
14. **Router Cache:** Page HTML cached in memory to avoid re-fetching on navigation

#### Key Configuration Options

- **DEFAULT_HOST:** `127.0.0.1` (hardcoded constant)
- **DEFAULT_PORT:** `8080` (hardcoded constant)
- **Compression:** Always enabled (CompressionLayer)
- **Graceful Shutdown:** Awaits in-flight requests before stopping

**Note:** Webserver has no dedicated config section - host/port are constants. Future expansion would require adding a `webserver` config section with these as fields.

## Data Storage

```
┌─────────────────────────────────────────────────────────────────────────┐
│                          SQLITE DATABASES                                │
└─────────────────────────────────────────────────────────────────────────┘
        │
        ├─ transactions.db  (Transaction Stream + Cache)
        ├─ positions.db     (Open/Closed Positions)
        ├─ tokens.db        (Token Database + Decimals Cache)
        ├─ security.db      (Rugcheck Security Data)
        ├─ ohlcvs.db        (Multi-Timeframe Candles)
        ├─ events.db        (Structured Event Logs)
        └─ wallet.db        (Balance Snapshots)
```

## Events System (Persistent Event Recording)

The events system provides comprehensive, structured event recording across all bot activities with non-blocking async recording, categorized events, and queryable JSON payloads stored in SQLite.

### Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         EVENTS SERVICE                                  │
│                    (Priority 10, No Dependencies)                       │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
        ┌───────────────────────────┼───────────────────────────┐
        ▼                           ▼                           ▼
┌──────────────┐          ┌──────────────┐          ┌──────────────┐
│   Database   │          │   Channel    │          │   Service    │
│  (events.db) │          │  (Async      │          │  (Minimal)   │
│              │          │   Queue)     │          │              │
│ - Split Pools│          │              │          │ - Init Only  │
│   (Read/Write│          │ - Capacity:  │          │ - No Tasks   │
│    2/10)     │          │   10,000     │          │   Spawned    │
│              │          │              │          │              │
│ - WAL Mode   │          │ - Batched    │          │ - Waits for  │
│ - 30s Busy   │          │   Writes     │          │   Shutdown   │
│   Timeout    │          │   (100 or    │          │              │
│              │          │    1s)       │          │              │
└──────────────┘          └──────────────┘          └──────────────┘
        │                           │                           │
        ▼                           ▼                           ▼
┌──────────────┐          ┌──────────────┐          ┌──────────────┐
│   Schema     │          │  Broadcast   │          │  Helpers     │
│  (1 Table,   │          │  (Real-Time  │          │  (Recording) │
│   8 Indexes) │          │   Delivery)  │          │              │
│              │          │              │          │ - record_*   │
│ - events     │          │ - Capacity:  │          │   _event()   │
│   (id, time, │          │   5,000      │          │   Functions  │
│    category, │          │              │          │              │
│    subtype,  │          │ - Optional   │          │ - Macros:    │
│    severity, │          │   Subscribe  │          │   event_info!│
│    mint,     │          │              │          │   event_warn!│
│    ref_id,   │          │              │          │   event_error│
│    msg_short,│          │              │          │              │
│    payload,  │          │              │          │              │
│    created)  │          │              │          │              │
└──────────────┘          └──────────────┘          └──────────────┘
```

### Service Implementation Details

**Location:** `src/services/implementations/events_service.rs`

**Lifecycle:**

```
[Service Start] → Initialize Events System
                          │
                          ├─ Create Database (events.db)
                          │   └─ Split pools: Write (2 conns), Read (10 conns)
                          │
                          ├─ Create Async Channel (10,000 capacity)
                          │   └─ Spawn background writer task
                          │
                          ├─ Create Broadcast Channel (5,000 capacity)
                          │   └─ For real-time event delivery
                          │
                          └─ Create Ring Buffer Cache (5,000 events)
                              └─ In-memory recent events

        [Service Ready - Wait for Shutdown Signal]
```

**Critical Note:** Events service does NOT spawn maintenance tasks automatically. Maintenance must be started explicitly via `events::start_maintenance_task()` if needed (currently only used in `verify_transactions_from_csv.rs` bin).

### Event Categories (11 Total)

```
EventCategory Enum:
  ├─ Swap           → Jupiter/DEX swap execution
  ├─ Transaction    → Blockchain transaction events
  ├─ Pool           → Pool discovery, analysis, prices
  ├─ Token          → Token metadata, blacklist, discovery
  ├─ System         → Bot lifecycle, errors, config changes
  ├─ Position       → Position open/close, P&L updates
  ├─ Wallet         → Balance changes, ATA management
  ├─ Entry          → Entry signals, trading decisions
  ├─ Ohlcv          → OHLCV monitoring, gaps, backfills
  ├─ Rpc            → RPC and API interactions
  ├─ Security       → Security analysis, risk assessment
  └─ Other(String)  → Custom categories
```

### Severity Levels (4 Total)

```
Severity Enum:
  ├─ Info   → Normal operations
  ├─ Warn   → Potential issues, recoverable errors
  ├─ Error  → Failures, exceptions
  └─ Debug  → Detailed tracing (development)
```

### Event Structure

```rust
Event {
    id: Option<i64>,              // Auto-generated by DB
    event_time: DateTime<Utc>,    // Event timestamp
    category: EventCategory,       // Category enum
    subtype: Option<String>,      // Specific type (e.g., "JupiterQuote")
    severity: Severity,           // Severity level
    mint: Option<String>,         // Associated token mint
    reference_id: Option<String>, // Correlation ID (signature, pool address, etc.)
    payload: Value,               // JSON payload with event details
    created_at: Option<DateTime<Utc>> // DB insertion timestamp
}
```

### Recording Flow (Async, Non-Blocking)

```
[Business Logic] → events::record(event)
                          │
                          ├─ Lock EVENT_WRITER mutex
                          │
                          ├─ Send to mpsc channel (10,000 capacity)
                          │   └─ Non-blocking if channel has space
                          │
                          └─ Return immediately (Ok)

        [Background Writer Task] → Continuous Loop
                          │
                          ├─ Receive from channel (batch up to 100)
                          │
                          ├─ OR timeout after 1s (flush partial batch)
                          │
                          ├─ Write batch to database (single transaction)
                          │
                          ├─ Push to ring buffer cache (front = newest)
                          │   └─ Evict oldest if >5,000 events
                          │
                          └─ Broadcast to subscribers (real-time)
                              └─ Optional: subscribe() for live events
```

**Safe Variant:** `events::record_safe(event)` logs errors instead of propagating them to avoid disrupting main operations.

### Helper Functions (8 Recording Helpers)

Located in `src/events/maintenance.rs`:

```
record_transaction_event(signature, status, success, fee, slot, error)
record_swap_event(signature, input_mint, output_mint, amounts, success, error)
record_pool_event(pool_address, program_id, pool_type, mint, action, details)
record_position_event(position_id, mint, action, signatures, amounts, pnl)
record_entry_event(mint, signal_type, decision, price, timeframe, strength, reason)
record_system_event(component, action, severity, details)
record_token_event(mint, action, severity, details)
record_security_event(mint, analysis_type, risk_level, findings)
record_ohlcv_event(subtype, severity, mint, reference_id, payload)
```

**Convenience Macros:**

- `event_info!(category, subtype, mint, ref_id, payload)`
- `event_warn!(category, subtype, mint, ref_id, payload)`
- `event_error!(category, subtype, mint, ref_id, payload)`

### Database Schema (events.db)

**Table: events**

```
Columns:
  - id              INTEGER PRIMARY KEY AUTOINCREMENT
  - event_time      TEXT NOT NULL (RFC3339 timestamp)
  - category        TEXT NOT NULL (category enum string)
  - subtype         TEXT (optional specific type)
  - severity        TEXT NOT NULL (severity enum string)
  - mint            TEXT (optional token mint)
  - reference_id    TEXT (optional correlation ID)
  - message_short   TEXT (extracted from payload, max 240 chars)
  - json_payload    TEXT NOT NULL (full JSON event data)
  - created_at      TEXT NOT NULL DEFAULT (datetime('now'))

Indexes (8 total):
  1. idx_events_category_time      (category, event_time DESC)
  2. idx_events_reference_id       (reference_id)
  3. idx_events_mint               (mint)
  4. idx_events_severity_time      (severity, event_time DESC)
  5. idx_events_created_at         (created_at)
  6. idx_events_id_desc            (id DESC)
  7. idx_events_category_severity_id (category, severity, id DESC)
  8. idx_events_mint_id            (mint, id DESC)
```

**Performance Configuration:**

- Journal mode: WAL (Write-Ahead Logging)
- Synchronous: NORMAL (balanced durability/speed)
- Cache size: 10,000 pages (write), 20,000 pages (read)
- Temp store: MEMORY (in-memory temporary tables)
- Busy timeout: 30,000ms (30 seconds)
- Read pool: query_only=1, mmap_size=256MB (if supported)

### Query API (Database Methods)

**Keyset Pagination (Cursor-Based):**

- `get_events_head(limit, filters)` → Latest N events + max_id
- `get_events_since(after_id, limit, filters)` → Events after cursor (forward)
- `get_events_before(before_id, limit, filters)` → Events before cursor (backward)

**Filters (All Query Methods):**

- `category: Option<EventCategory>`
- `severity: Option<Severity>`
- `mint: Option<&str>`
- `reference_id: Option<&str>`
- `search: Option<&str>` (case-insensitive JSON payload search)

**Convenience Methods:**

- `recent(category, limit)` → Recent events by category
- `recent_all(limit)` → Recent events (all categories)
- `by_reference(ref_id, limit)` → Events for specific reference ID
- `by_mint(mint, limit)` → Events for specific token mint
- `get_event_counts_by_category(hours)` → Event counts per category for last N hours
- `get_stats()` → Database statistics (total events, 24h events, DB size)

### Maintenance System

**Background Task:** `start_maintenance_task()`

**Currently NOT Auto-Started** - Must be explicitly called if needed.

**Maintenance Schedule:**

```
[Maintenance Loop] → Every 6 Hours
                          │
                          ├─ Cleanup Old Events (>30 days)
                          │   └─ DELETE FROM events WHERE event_time < cutoff
                          │
                          ├─ Collect Database Statistics
                          │   └─ Total events, 24h events, DB size
                          │
                          └─ Log Summary
                              └─ STATS: Events DB status
```

**Configuration Constants:**

- `MAX_EVENT_AGE_DAYS: 30` (hardcoded in db.rs)
- `MAINTENANCE_INTERVAL: 6 hours` (hardcoded in maintenance.rs)

**Note:** Retention period is NOT configurable via `config.toml` (EventsConfig is empty, reserved for future use).

---

## Actions Progress Tracking System

The actions system provides real-time progress tracking for long-running operations (swaps, position management, manual orders) with Server-Sent Events (SSE) streaming to the dashboard for live updates.

### Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         ACTIONS MODULE                                  │
│              (Real-Time Operation Progress Tracking)                    │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
        ┌───────────────────────────┼───────────────────────────┐
        ▼                           ▼                           ▼
┌──────────────┐          ┌──────────────┐          ┌──────────────┐
│   State      │          │  Broadcast   │          │   Routes     │
│  (In-Memory) │          │  (Tokio)     │          │   (SSE)      │
│              │          │              │          │              │
│ - HashMap    │          │ - Capacity:  │          │ - GET /api/  │
│   <ActionId, │          │   1000       │          │   actions/   │
│    Action>   │          │              │          │   stream     │
│              │          │ - try_send   │          │              │
│ - Arc<RwLock>│          │   Pattern    │          │ - GET /api/  │
│              │          │              │          │   actions/   │
│ - Auto       │          │ - Drops if   │          │   active     │
│   Cleanup    │          │   No         │          │              │
│   (5 min)    │          │   Receivers  │          │ - GET /api/  │
│              │          │              │          │   actions/   │
│              │          │              │          │   all        │
│              │          │              │          │              │
│              │          │              │          │ - GET /api/  │
│              │          │              │          │   actions/   │
│              │          │              │          │   subscribers│
└──────────────┘          └──────────────┘          └──────────────┘
```

### Core Types

**Action:**

```rust
pub struct Action {
    pub id: ActionId,               // UUID
    pub action_type: ActionType,    // Operation type
    pub entity_id: String,          // Related entity (mint, position_id)
    pub state: ActionState,         // Current state
    pub steps: Vec<ActionStep>,     // All steps with status
    pub metadata: Value,            // Additional context
    pub created_at: String,         // RFC3339 timestamp
    pub updated_at: Option<String>, // Last update
    pub completed_at: Option<String>, // Completion time
}
```

**ActionType (7 Types):**

```rust
pub enum ActionType {
    SwapBuy,           // Buy swap via Jupiter/GMGN
    SwapSell,          // Sell swap via Jupiter/GMGN
    PositionOpen,      // Open new position
    PositionClose,     // Close existing position
    PositionDca,       // DCA into position
    PositionPartialExit, // Partial position exit
    ManualOrder,       // Manual trader order
}
```

**ActionState:**

```rust
pub enum ActionState {
    InProgress {
        current_step: usize,    // 1-based step index
        total_steps: usize,     // Total step count
        progress_pct: u8,       // 0-100 progress percentage
    },
    Completed,                  // Successfully finished
    Failed { error: String },   // Failed with error
    Cancelled,                  // User cancelled
}
```

**ActionStep:**

```rust
pub struct ActionStep {
    pub name: String,              // Step description
    pub status: StepStatus,        // Current status
    pub started_at: Option<String>, // When step started
    pub completed_at: Option<String>, // When step finished
    pub error: Option<String>,     // Error if failed
    pub metadata: Option<Value>,   // Step-specific data
}
```

**StepStatus:**

```rust
pub enum StepStatus {
    Pending,      // Not started
    InProgress,   // Currently executing
    Completed,    // Finished successfully
    Failed,       // Failed with error
    Skipped,      // Skipped (conditional step)
}
```

**ActionUpdate (Broadcast Event):**

```rust
pub struct ActionUpdate {
    pub action_id: ActionId,
    pub update_type: UpdateType,
    pub action: Action,
    pub timestamp: String,
}

pub enum UpdateType {
    ActionStarted,      // New action created
    StepProgress,       // Step status changed
    ActionCompleted,    // Action finished successfully
    ActionFailed,       // Action failed
}
```

### State Management (src/actions/state.rs)

**Global Registry:**

```rust
static ACTIVE_ACTIONS: Lazy<Arc<RwLock<HashMap<ActionId, Action>>>>
    = Lazy::new(|| Arc::new(RwLock::new(HashMap::new())));
```

**Key Functions:**

- `register_action(action: Action)` → Add new action, broadcast ActionStarted
- `get_action(action_id: &str)` → Get specific action
- `get_active_actions()` → Get all in-progress actions
- `get_all_actions()` → Get all actions (including completed/failed)
- `update_step(action_id, step_index, status, error, metadata)` → Update step, broadcast progress
- `complete_action_success(action_id)` → Mark complete, schedule removal (5s delay)
- `complete_action_failed(action_id, error)` → Mark failed, schedule removal (30s delay)
- `cleanup_old_actions()` → Remove completed/failed actions >5 minutes old

**Automatic Cleanup:**

- Completed actions: Removed 5 seconds after completion
- Failed actions: Removed 30 seconds after failure
- Old actions: Background cleanup removes actions >5 minutes old

### Broadcasting (src/actions/broadcast.rs)

**Channel:**

```rust
static ACTION_BROADCAST: Lazy<broadcast::Sender<ActionUpdate>>
    = Lazy::new(|| broadcast::channel(1000).0);
```

**Functions:**

- `broadcast_update(update: ActionUpdate)` → Send to all subscribers (try_send, non-blocking)
- `subscribe()` → Get Receiver<ActionUpdate> for listening
- `subscriber_count()` → Count active SSE connections

**Pattern:** Non-blocking try_send drops messages if no receivers (no backpressure on operations).

### SSE Endpoint (src/webserver/routes/actions.rs)

**GET /api/actions/stream**

```
Content-Type: text/event-stream
Connection: keep-alive
Cache-Control: no-cache

Event Stream Format:
  event: action-update
  data: { "action_id": "...", "update_type": "...", "action": {...} }

Keep-Alive: Every 15 seconds sends comment line
```

**Implementation:**

- Uses `async-stream` and `tokio-stream`
- Handles lagged clients (drops old messages)
- Auto-closes connection on channel close
- Wraps each ActionUpdate in SSE format

**REST Endpoints:**

- `GET /api/actions/active` → JSON array of in-progress actions
- `GET /api/actions/all` → JSON array of all actions (with completed/failed)
- `GET /api/actions/subscribers` → Count of active SSE connections

### Swap Integration (src/swaps/mod.rs)

**execute_best_swap() - Action Tracking:**

1. **Create Action:**

```rust
let action_id = Uuid::new_v4().to_string();
let steps = vec![
    "Validating quote",
    "Building transaction",
    "Signing transaction",
    "Submitting to blockchain",
    "Confirming transaction",
];
let action = Action::new(action_id.clone(), ActionType::SwapBuy, mint, steps, metadata);
register_action(action).await;
```

2. **Progress Updates:**

```rust
update_step(&action_id, 0, StepStatus::Completed, None, None).await; // Quote validated
update_step(&action_id, 1, StepStatus::InProgress, None, None).await; // Building tx
// ... etc for each step
```

3. **Completion:**

```rust
// Success path
complete_action_success(&action_id).await;

// Failure path
complete_action_failed(&action_id, error_message).await;
```

**Metadata Includes:**

- `symbol`: Token symbol
- `router`: Jupiter/GMGN
- `amount_in_sol`: Input amount
- `expected_amount_out`: Expected output
- `slippage_pct`: Configured slippage

### Frontend Integration

**Notification Manager (scripts/core/notifications.js):**

```javascript
class NotificationManager {
  - Connects to SSE endpoint on init
  - Listens for ActionUpdate events
  - Stores notifications in localStorage (max 100)
  - Auto-dismiss: Completed (10s), Failed (30s)
  - Provides query methods: getAll(), getActive(), getCompleted(), getFailed()
  - Emits events to subscribers for UI updates
}
```

- SSE payloads now always include an `action` snapshot so the UI can fully reconcile state with a single message.
- `update_type` values are lowercase snake_case (`action_started`, `step_progress`, `action_failed`, etc.).
- High-volume `step_*` updates are persisted with a short debounce to avoid thrashing localStorage.

**Notification Panel (scripts/ui/notification_panel.js):**

- Tabs: All / Active / Completed / Failed
- Real-time progress bars for in-progress actions
- Click to mark as read
- Dismiss button per notification
- Auto-updates on SSE events
- Persists to localStorage across page reloads
- Status detection relies on `action.state.status` (`in_progress`, `completed`, `failed`, `cancelled`) and uses `current_step_index` + `current_step` for progress labels.
- Completed/Failed tabs surface dismissed actions (including auto-dismissed) so historical runs remain reviewable while All/Active stay focused on live items.
- Rendering escapes token metadata, router names, and error messages before injecting HTML to block XSS payloads from on-chain data.

**Header Integration (scripts/core/header.js):**

- Badge shows unread count
- Toast notifications for new/completed/failed actions
- Panel toggle on notification button click
- Auto mark-all-as-read when opening panel

**Styles (styles/components/notifications.css):**

- Light/dark theme support
- Progress bar animations
- Badge pulse animation
- Smooth slide-in for new notifications
- Mobile responsive (full-width panel on small screens)

### Usage Pattern (For New Operations)

```rust
use crate::actions::{Action, ActionType, StepStatus, register_action, update_step, complete_action_success, complete_action_failed};
use uuid::Uuid;

pub async fn my_operation(params) -> Result<String, String> {
    // 1. Create action
    let action_id = Uuid::new_v4().to_string();
    let steps = vec!["Step 1".to_string(), "Step 2".to_string(), "Step 3".to_string()];
    let metadata = json!({ "param": value });
    let action = Action::new(action_id.clone(), ActionType::ManualOrder, entity_id, steps, metadata);
    register_action(action).await;

    // 2. Execute with progress updates
    update_step(&action_id, 0, StepStatus::InProgress, None, None).await;
    // ... do step 1 ...
    update_step(&action_id, 0, StepStatus::Completed, None, None).await;

    update_step(&action_id, 1, StepStatus::InProgress, None, None).await;
    // ... do step 2 ...
    update_step(&action_id, 1, StepStatus::Completed, None, None).await;

    // 3. Complete or fail
    match result {
        Ok(_) => {
            complete_action_success(&action_id).await;
            Ok(success_message)
        }
        Err(e) => {
            complete_action_failed(&action_id, e.to_string()).await;
            Err(error_message)
        }
    }
}
```

### Key Design Decisions

**In-Memory Only:** Actions are not persisted to database (transient progress tracking, not historical records).

**Non-Blocking Broadcast:** Uses try_send to avoid backpressure on critical operations.

**Auto-Cleanup:** Prevents memory leaks by auto-removing old actions (5 min retention for completed/failed).

**SSE Over WebSocket:** Simpler unidirectional streaming, browser auto-reconnect, no custom protocol.

**Entity-Based Grouping:** Each action tracks an entity_id (mint, position_id) for correlation.

**Step-Level Granularity:** Fine-grained progress (not just "in progress") for better UX.

**Metadata Flexibility:** JSON metadata allows operation-specific context without rigid schemas.

### Future Extensions

- Position operation tracking (following same pattern as swaps)
- Manual order tracking
- DCA operation tracking
- Partial exit tracking
- Historical action log (optional persistence)
- Action cancellation support (cancel in-flight operations)
- Action replay (retry failed actions)

---

### Webserver Integration (REST API)

**Location:** `src/webserver/routes/events.rs`

**Endpoints (4 total):**

```
GET /api/events/head         → Latest events with cursor
    Query Params:
      - limit: usize (default 200, max 1000)
      - category: String (optional)
      - severity: String (optional)
      - mint: String (optional)
      - reference: String (optional)
      - search: String (optional, JSON payload search)
    Response:
      - events: Vec<EventResponse>
      - count: usize
      - max_id: i64 (cursor for pagination)
      - timestamp: String

GET /api/events/since        → Events after cursor (forward pagination)
    Query Params:
      - after_id: i64 (required, cursor)
      - limit: usize (default 200, max 1000)
      - category, severity, mint, reference, search (same as head)
    Response: Same as head

GET /api/events/before       → Events before cursor (backward pagination)
    Query Params:
      - before_id: i64 (required, cursor)
      - limit: usize (default 200, max 1000)
      - category, severity, mint, reference, search (same as head)
    Response: Same as head

GET /api/events/categories   → List of available event categories
    Response:
      - categories: Vec<String> (all EventCategory variants)
```

**EventResponse Structure:**

```rust
{
    id: i64,
    event_time: String,        // RFC3339 timestamp
    category: String,
    subtype: Option<String>,
    severity: String,
    mint: Option<String>,
    reference_id: Option<String>,
    message: String,           // Extracted from payload["message"]
    payload: serde_json::Value, // Full JSON payload
    created_at: String         // RFC3339 timestamp
}
```

### Integration Points

**1. RPC Module:**

- Records transaction submission and confirmation events
- `record_transaction_event()` in `rpc.rs` (3 calls)

**2. Positions Module:**

- Records position lifecycle events (open, close, verification)
- Used via `record_position_event()` helper

**3. OHLCV Module:**

- Records monitoring events (discovery, fetch, gaps, backfills)
- Used via `record_ohlcv_event()` helper

**4. Swaps Module:**

- Records swap execution events
- Used via `record_swap_event()` helper

**5. Tokens Module:**

- Records token discovery, blacklist, metadata events
- Used via `record_token_event()` helper

**6. Security Module:**

- Records security analysis events
- Used via `record_security_event()` helper

**7. System-Wide:**

- All modules can record events via `events::record()` or macros
- Non-blocking async recording prevents impact on hot paths

### MCP Integration Helpers

Located in `src/events/maintenance.rs`:

```
get_events_summary(hours) → Summary statistics for MCP tools
    Returns:
      - counts_by_category: HashMap<String, u64>
      - database_stats: HashMap<String, i64>
      - recent_errors: Vec<Event> (last 10 errors)
      - time_range_hours: u64

search_events(category, mint, ref_id, since_hours, limit) → Multi-criteria search
    Returns: Vec<Event>
```

### Ring Buffer Cache (In-Memory)

```
EVENTS_CACHE: VecDeque<Event> (Capacity: 5,000)
    │
    ├─ Updated after batch writes (front = newest)
    ├─ Evicts oldest when >5,000 events
    ├─ Accessible via cached_events_head(limit)
    └─ Used for fast recent event access (no DB query)
```

### Broadcast System (Real-Time Delivery)

```
EVENTS_BROADCAST_TX: broadcast::Sender<Event> (Capacity: 5,000)
    │
    ├─ Optional subscription via events::subscribe()
    ├─ Events sent after successful DB write
    ├─ Lag occurs if receiver can't keep up (5,000 buffer)
    └─ Used for real-time event streaming (future websocket integration)
```

### Key Configuration Options

**No Configuration Available** - EventsConfig is empty (reserved for future use).

**Hardcoded Constants:**

- Channel capacity: 10,000 events
- Batch size: 100 events or 1s timeout
- Ring buffer: 5,000 events
- Broadcast buffer: 5,000 events
- Event retention: 30 days
- Maintenance interval: 6 hours
- Write pool: 2 connections
- Read pool: 10 connections
- Busy timeout: 30 seconds

### Critical Implementation Details

1. **Non-Blocking Recording**: mpsc channel with 10,000 capacity prevents backpressure on hot paths
2. **Batched Writes**: Events buffered up to 100 or 1s timeout for efficiency
3. **Split Connection Pools**: Separate read (10) and write (2) pools for concurrency
4. **Keyset Pagination**: ID-based cursors (not offset/limit) for efficient pagination
5. **WAL Mode**: Write-Ahead Logging for concurrent reads during writes
6. **Message Extraction**: Short message (240 chars) extracted from payload for quick display
7. **JSON Payload Search**: Full-text search via `LOWER(json_payload) LIKE` (case-insensitive)
8. **Auto-ID Generation**: Database auto-increments event IDs for ordering
9. **Timezone Handling**: All timestamps stored as RFC3339 (UTC), parsed as DateTime<Utc>
10. **Error Handling**: `record_safe()` logs errors instead of propagating to avoid disrupting operations
11. **No Maintenance Auto-Start**: Maintenance task must be started explicitly (not part of service lifecycle)
12. **Ring Buffer + Broadcast**: Dual caching (ring buffer for fast reads, broadcast for real-time delivery)
13. **Optional Subscription**: Broadcast channel is optional, receivers can subscribe if needed
14. **Reference ID Correlation**: Supports correlation via transaction signatures, pool addresses, position IDs, etc.
15. **Category-Based Queries**: All queries support optional category filtering for scoped searches

## Readiness Flags

```
[Global Flags] → TRANSACTIONS_SYSTEM_READY
                      │
                      ├→ POOL_SERVICE_READY
                      ├→ TOKENS_SYSTEM_READY
                      ├→ POSITIONS_SYSTEM_READY
                      └→ SECURITY_ANALYZER_READY

                [are_core_services_ready()] → Trader Start Gate
```

## Configuration System

The configuration system provides zero-repetition, type-safe, hot-reloadable configuration with metadata-driven UI rendering.

### Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────────┐
│                      CONFIGURATION SYSTEM                               │
│                    (Single Source of Truth)                             │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
        ┌───────────────────────────┼───────────────────────────┐
        ▼                           ▼                           ▼
┌──────────────┐          ┌──────────────┐          ┌──────────────┐
│   Schema     │          │   Storage    │          │   Metadata   │
│  (schemas/)  │          │  (Global     │          │  (UI Info)   │
│              │          │   RwLock)    │          │              │
│ - trader.rs  │          │              │          │ - FieldType  │
│ - filtering.rs│         │ OnceCell<    │          │ - Rendering  │
│ - tokens.rs  │          │  RwLock<     │          │   Hints      │
│ - swaps.rs   │          │   Config>>   │          │ - Category   │
│ - positions.rs│         │              │          │   Normalize  │
│ - rpc.rs     │          │              │          │              │
│ - (13 total) │          │              │          │              │
└──────────────┘          └──────────────┘          └──────────────┘
        │                           │                           │
        └───────────────────────────┴───────────────────────────┘
                                    │
                                    ▼
                         ┌──────────────────┐
                         │   Access Layer   │
                         │  (utils.rs)      │
                         │                  │
                         │ - with_config    │
                         │ - get_config_clone│
                         │ - reload_config  │
                         │ - update_config  │
                         └──────────────────┘
                                    │
        ┌───────────────────────────┼───────────────────────────┐
        ▼                           ▼                           ▼
┌──────────────┐          ┌──────────────┐          ┌──────────────┐
│  data/       │          │  Webserver   │          │  Business    │
│  config.toml │          │  Routes      │          │  Logic       │
│  (Overrides) │          │  (API)       │          │  (Usage)     │
└──────────────┘          └──────────────┘          └──────────────┘
```

### File Structure

```
src/config/
├── mod.rs              - Module exports & documentation
├── macros.rs           - config_struct! & field_metadata! macros
├── metadata.rs         - UI metadata system (FieldType, traits)
├── utils.rs            - Loading, access, hot reload utilities
└── schemas/
    ├── mod.rs          - Root Config struct (aggregates all sections)
    ├── trader.rs       - Trading configuration
    ├── filtering.rs    - Token filtering configuration
    ├── tokens.rs       - Token system configuration
    ├── swaps.rs        - Swap routing configuration
    ├── positions.rs    - Position management configuration
    ├── rpc.rs          - RPC client configuration
    ├── pools.rs        - Pool service configuration (no UI)
    ├── sol_price.rs    - SOL price service configuration
    ├── events.rs       - Events system configuration
    ├── services.rs     - Service manager configuration
    ├── monitoring.rs   - System monitoring configuration
    ├── ohlcv.rs        - OHLCV data configuration
    └── wallet.rs       - Wallet service configuration (no UI)

data/
└── config.toml         - Override file (optional, defaults from schemas)
```

### Configuration Sections

**Root Level:**

- `main_wallet_private` - Wallet private key (base58 or array format)

**Service Sections (14 total):**

- `rpc` - RPC endpoints, rate limiting, rotation
- `trader` - Core trading parameters, profit thresholds, timing
- `positions` - Position tracking, verification, reconciliation
- `filtering` - Token filtering criteria (DexScreener, GeckoTerminal, Rugcheck)
- `swaps` - Multi-DEX routing (GMGN, Jupiter), slippage, prioritization
- `tokens` - Discovery sources, update intervals, rate limits
- `pools` - Pool service config (discovery, fetcher, calculator) - **No UI**
- `sol_price` - SOL price updates, fallback sources
- `events` - Event logging, batching, persistence
- `services` - Service manager, priorities, dependencies
- `monitoring` - Metrics collection, health checks
- `ohlcv` - Multi-timeframe candles, priority monitoring
- `wallet` - Balance snapshots, flow metrics - **No UI**

**Note:** `pools` and `wallet` sections exist in schema but are NOT exposed in UI metadata (`collect_config_metadata()`). They're internal service configurations.

### Schema Definition Flow (Zero Repetition)

```
[Define in schemas/<section>.rs] → Use config_struct! macro
                                        │
        ┌───────────────────────────────┼───────────────────────────┐
        ▼                               ▼                           ▼
   Field Definition            Metadata Annotation          Default Value
   name: Type                  #[metadata(field_metadata!    = value
                               { label, hint, unit, ... })]
        │                               │                           │
        └───────────────────────────────┴───────────────────────────┘
                                        │
                                        ▼
                              Macro Expands to:
                                        │
        ┌───────────────────────────────┼───────────────────────────┐
        ▼                               ▼                           ▼
   Struct with pub fields     Default implementation      Serde support
   + FieldTypeInfo trait      + field_metadata() fn       (#[serde(default)])
        │                               │                           │
        └───────────────────────────────┴───────────────────────────┘
                                        │
                                        ▼
                              Aggregated in schemas/mod.rs
                                        │
                                        ▼
                              Root Config struct
```

**Example Schema Definition:**

```rust
// In schemas/trader.rs
config_struct! {
    pub struct TraderConfig {
        #[metadata(field_metadata! {
            label: "Max Open Positions",
            hint: "Max simultaneous positions (2-5 conservative)",
            min: 1,
            max: 100,
            unit: "positions",
            impact: "critical",
            category: "Core Trading",
        })]
        max_open_positions: usize = 2,

        #[metadata(field_metadata! {
            label: "Trade Size",
            hint: "SOL per position (0.005-0.01 for testing)",
            min: 0.001,
            max: 10,
            step: 0.001,
            unit: "SOL",
            impact: "critical",
            category: "Core Trading",
        })]
        trade_size_sol: f64 = 0.005,
    }
}
```

### Metadata System (UI Rendering)

```
[Metadata Collection] → collect_config_metadata()
                              │
        ┌─────────────────────┼─────────────────────┬────────────────┐
        ▼                     ▼                     ▼                ▼
   TraderConfig       FilteringConfig        TokensConfig    (11 sections)
   field_metadata()   field_metadata()       field_metadata()
        │                     │                     │                │
        └─────────────────────┴─────────────────────┴────────────────┘
                              │
                              ▼
                     ConfigMetadata (BTreeMap)
                              │
        ┌─────────────────────┼─────────────────────┬────────────────┐
        ▼                     ▼                     ▼                ▼
   FieldType           FieldMetadata         Category          Nested
   (Boolean, Number,   (label, hint,         Normalization     Metadata
    Integer, Array,     unit, impact,        (General,         (for Object
    String, Object)     min/max/step,        Developer,        types)
                        placeholder)         Advanced)
        │                     │                     │                │
        └─────────────────────┴─────────────────────┴────────────────┘
                              │
                              ▼
                     Sent to Frontend via GET /api/config/metadata
                              │
                              ▼
                     Dynamic UI Rendering (No Hardcoded Forms)
```

**FieldType Support:**

- `Boolean` - Checkbox
- `Number` - Input with step (f64, f32)
- `Integer` - Input (usize, u64, i64, u32, i32, u16, i16, u8, i8)
- `String` - Text input
- `Array` - JSON editor or multi-input
- `Object` - JSON editor with nested metadata

**Metadata Attributes:**

- `label` - Display name for UI
- `hint` - Tooltip/help text
- `unit` - Display unit (SOL, %, seconds, etc.)
- `impact` - Severity (critical, high, medium, low)
- `category` - Grouping (normalized to General, Developer, Advanced)
- `min/max/step` - Numeric constraints
- `placeholder` - Input placeholder text
- `docs` - Inline documentation (from `///` comments)
- `default` - Default value (JSON serialized)
- `children` - Nested object metadata

### Access Patterns

#### Loading (Startup)

```
[Application Start] → load_config()
                          │
                          ▼
                  Check if data/config.toml exists
                          │
        ┌─────────────────┼─────────────────┐
        ▼                                   ▼
   File Exists                         File Missing
   - Read TOML                         - Use defaults from schemas
   - Parse with serde                  - Print warning
   - Merge with defaults               - Continue with defaults
        │                                   │
        └─────────────────┬─────────────────┘
                          │
                          ▼
                  Initialize CONFIG (OnceCell)
                  Store in RwLock<Config>
                          │
                          ▼
                  Config ready for use
```

#### Synchronous Access (Closure-Based)

```
[Business Logic] → with_config(|cfg| cfg.trader.max_open_positions)
                          │
                          ▼
                  Acquire CONFIG.get()
                          │
                          ▼
                  Acquire RwLock::read()
                          │
                          ▼
                  Execute closure with &Config
                          │
                          ▼
                  Release lock automatically
                          │
                          ▼
                  Return closure result
```

**Critical Rule:** Never hold the lock across await points. This blocks hot reloads.

```rust
// ❌ WRONG - Blocks hot reload
with_config(|cfg| {
    some_async_fn().await; // DON'T DO THIS
});

// ✅ CORRECT - Clone first, then await
let cfg = get_config_clone();
some_async_fn().await;
```

#### Asynchronous Access (Full Clone)

```
[Async Function] → get_config_clone()
                          │
                          ▼
                  with_config(|cfg| cfg.clone())
                          │
                          ▼
                  Clone entire Config struct
                          │
                          ▼
                  Return owned Config
                          │
                          ▼
                  Use across await points safely
```

**Use Case:** When config values are needed across multiple await points or in spawned tasks.

#### Hot Reload

```
[Manual Trigger] → reload_config()
                          │
                          ▼
                  Read data/config.toml
                          │
                          ▼
                  Parse with serde
                          │
                          ▼
                  Acquire CONFIG.get()
                          │
                          ▼
                  Acquire RwLock::write()
                          │
                          ▼
                  Atomically replace *config
                          │
                          ▼
                  Release write lock
                          │
                          ▼
                  New values active immediately
                  (Next with_config() reads see new values)
```

**Trigger Methods:**

- Programmatic: `reload_config()`
- Webserver API: `POST /api/config/reload`
- Frontend: Config page "Reload" button

#### Programmatic Updates

```
[Update Request] → update_config_section(
                       |cfg| { cfg.trader.max_open_positions = 3; },
                       save_to_disk: true
                   )
                          │
                          ▼
                  Acquire CONFIG.get()
                          │
                          ▼
                  Acquire RwLock::write()
                          │
                          ▼
                  Execute update closure
                          │
                          ▼
                  Release write lock
                          │
                          ▼
                  If save_to_disk == true:
                      Serialize Config to TOML
                      Write to data/config.toml
                          │
                          ▼
                  Return Ok(()) or Err(String)
```

**Alternative with Diff Tracking:**

```
update_with_diff(
    |cfg| cfg.trader.clone(),           // Extract old value
    |cfg| { cfg.trader.enabled = false; }, // Update
    save_to_disk: true
)
    │
    ▼
Returns (old_value, new_value) for logging/auditing
```

### Webserver Integration (REST API)

#### GET Endpoints (Read Configuration)

```
GET /api/config                  → Full config (all sections)
GET /api/config/trader           → Single section
GET /api/config/filtering        → Single section
GET /api/config/tokens           → Single section
... (14 endpoints total)
GET /api/config/metadata         → UI metadata (for rendering)
GET /api/config/diff             → Diff from defaults

Response Format:
{
    "data": { ... },
    "timestamp": "2025-10-22T12:00:00Z"
}
```

#### PATCH Endpoints (Partial Updates)

```
PATCH /api/config/trader
PATCH /api/config/filtering
PATCH /api/config/tokens
... (11 endpoints total, no pools/wallet)

Request Body: Partial JSON
{
    "max_open_positions": 3,
    "trade_size_sol": 0.01
}

Processing Flow:
    │
    ▼
1. Get current section as JSON
    │
    ▼
2. Merge updates into current JSON (only fields provided)
    │
    ▼
3. Deserialize merged JSON to validate
    │
    ▼
4. Update config with merged values
    │
    ▼
5. Save to disk (if validation passes)
    │
    ▼
Response:
{
    "message": "TraderConfig updated successfully",
    "saved_to_disk": true,
    "timestamp": "2025-10-22T12:00:00Z"
}
```

**Key Behavior:** PATCH merges changes, doesn't replace entire section. Only provided fields are updated.

#### POST Endpoints (Utilities)

```
POST /api/config/reload          → Reload from disk
POST /api/config/reset           → Reset to defaults (dangerous!)
```

### Frontend Integration (Dashboard)

```
[Config Page Load] → GET /api/config/metadata
                          │
                          ▼
                  Receive ConfigMetadata
                          │
        ┌─────────────────┼─────────────────┬────────────────┐
        ▼                 ▼                 ▼                ▼
   Section Tabs      Field Rendering   Category Groups   Validation
   (Trader,          (Input type       (General,         (min/max,
    Filtering, ...)   from FieldType)   Developer, ...)   required)
        │                 │                 │                │
        └─────────────────┴─────────────────┴────────────────┘
                          │
                          ▼
                  User Edits Field
                          │
                          ▼
                  Frontend Validation
                          │
                          ▼
                  PATCH /api/config/<section>
                  (Only changed fields)
                          │
                          ▼
                  Check response.ok before processing
                          │
        ┌─────────────────┼─────────────────┐
        ▼                                   ▼
   Success                              Error
   - Show toast                         - Display error
   - Refresh UI                         - Rollback change
        │                                   │
        └─────────────────┬─────────────────┘
                          │
                          ▼
                  Config updated
```

**Frontend Features:**

- Metadata-driven rendering (zero hardcoded forms)
- JSON editors for complex objects (Array, Object types)
- Export/import configuration
- Validation before submit
- Category-based grouping
- Impact indicators (critical, high, medium, low)

### Storage & Persistence

```
[Global Storage] → static CONFIG: OnceCell<RwLock<Config>>
                          │
                          ▼
                  Thread-Safe Access
                          │
        ┌─────────────────┼─────────────────┐
        ▼                                   ▼
   Many Readers (RwLock::read)     Single Writer (RwLock::write)
   - with_config(|cfg| ...)         - reload_config()
   - get_config_clone()             - update_config_section()
        │                                   │
        └─────────────────┬─────────────────┘
                          │
                          ▼
                  No blocking between readers
                  Writers block all access (atomic)
```

**Persistence Flow:**

```
[In-Memory Config] → save_config(path)
                          │
                          ▼
                  with_config(|cfg| cfg.clone())
                          │
                          ▼
                  toml::to_string_pretty(cfg)
                          │
                          ▼
                  std::fs::write(path, toml_str)
                          │
                          ▼
                  data/config.toml updated
```

**File Format (data/config.toml):**

```toml
main_wallet_private = "base58_or_array_format"

[trader]
max_open_positions = 2
trade_size_sol = 0.005

[filtering]
min_liquidity_usd = 5000

[tokens]
# ... more sections
```

### Key Design Principles

1. **Single Source of Truth**: All defaults embedded in schemas, config.toml only for overrides
2. **Zero Repetition**: Define once (type + default + metadata), use everywhere
3. **Type Safety**: Compile-time checking, runtime validation
4. **Hot Reload**: Changes take effect immediately without restart
5. **Metadata-Driven UI**: No hardcoded forms, all generated from metadata
6. **Partial Updates**: PATCH merges changes, preserves unspecified fields
7. **Thread Safety**: RwLock allows concurrent reads, atomic writes
8. **Fail-Fast**: Invalid config prevents startup, invalid updates rejected

### Common Patterns

**Reading a single value:**

```rust
let max_positions = with_config(|cfg| cfg.trader.max_open_positions);
```

**Reading multiple values:**

```rust
let (max_pos, trade_size) = with_config(|cfg| {
    (cfg.trader.max_open_positions, cfg.trader.trade_size_sol)
});
```

**Async function usage:**

```rust
async fn trade() {
    let cfg = get_config_clone();
    // Use cfg across await points
    tokio::time::sleep(Duration::from_secs(1)).await;
    if positions.len() < cfg.trader.max_open_positions {
        // ...
    }
}
```

**Programmatic update:**

```rust
update_config_section(
    |cfg| {
        cfg.trader.max_open_positions = 3;
        cfg.trader.enabled = false;
    },
    true // Save to disk
)?;
```

**With diff tracking:**

```rust
let (old, new) = update_with_diff(
    |cfg| cfg.trader.clone(),
    |cfg| { cfg.trader.max_open_positions = 3; },
    true
)?;
log::info!("Changed from {} to {}", old.max_open_positions, new.max_open_positions);
```

## Token System Integration Points

The tokens module serves as the central data layer, consumed by multiple systems:

### 1. Filtering System Integration

```
[Filtering Engine] → tokens::list_tokens_async(limit)
                          │
                          ▼
                     tokens::get_full_token_async(mint) for each token
                          │
                          ▼
                     Apply 4-layer filters (Meta → DexScreener → GeckoTerminal → Rugcheck)
                          │
                          ▼
                     Build FilteredTokenLists (passed, rejected, blacklisted, with_pool_price, open_positions)
                          │
                          ▼
                     tokens::store_filtered_results(FilteredTokenLists)
                          │
                          ▼
                     [Tokens Filtered Store → Centralized Storage]
```

**Data Flow:**

- **Input**: Filtering reads tokens from tokens.db via `list_tokens_async()`
- **Processing**: Evaluates each token against configured filters
- **Output**: Stores results in tokens module via `store_filtered_results()`
- **Storage**: Results kept in tokens::filtered_store (Global RwLock)

**Consumer Access:**

- `tokens::get_passed_tokens()` → Pool service gets tradeable token mints
- `tokens::get_rejected_tokens()` → Dashboard shows rejected token mints
- `tokens::get_blacklisted_tokens()` → Dashboard shows blacklisted token mints
- `tokens::get_tokens_with_pool_price()` → Tokens with pricing data
- `tokens::get_tokens_with_open_positions()` → Tokens with active positions
- `tokens::get_counts()` → Dashboard stats (counts for all categories)

**Critical Note:** Consumers access filtered results from **tokens module**, NOT filtering module. The filtering module only provides query API for dashboard views.

### 2. Pool Service Integration

```
[Pool Discovery] → tokens::get_passed_tokens()
                      │
                      ▼
                 Get tokens that passed filtering (from tokens::filtered_store)
                      │
                      ▼
                 ALWAYS add positions::get_open_mints()
                 (position tokens monitored regardless of filtering)
                      │
                      ▼
                 Discover pools from APIs (DexScreener, GeckoTerminal)
                      │
                      ▼
                 Analyze → Fetch → Calculate → Cache prices
                      │
                      ▼
                 pools::get_available_tokens()
                 (returns tokens with fresh prices)
                      │
                      ▼
                 [Used by Trader and Dashboard]
```

**Key Implementation Details:**

- Pool service reads from `tokens::get_passed_tokens()` (NOT `filtering::get_filtered_token_mints()`)
- Position tokens ALWAYS included via `positions::get_open_mints()` (merged into token list)
- This ensures open positions continue to have price updates even if they no longer pass filters
- Pool discovery runs every 5 seconds (configurable via `discovery_tick_interval`)

### 3. Trader Integration

```
[Trader] → Get available tokens from pools
                │
                ▼
           tokens::get_full_token_async(mint)
                │
                ▼
           Extract market data (price, volume, liquidity)
                │
                ▼
           Make entry/exit decisions
                │
                ▼
           When position opens → Database updates priority to 100
                │
                ▼
           Token gets Critical priority updates (every 5s)
```

### 4. Position Management Integration

```
[Positions] → Open position created
                  │
                  ▼
              tokens::get_full_token_async(mint)
                  │
                  ▼
              Database priority updated to 100 (Critical)
                  │
                  ▼
              Position monitoring uses fresh market data
                  │
                  ▼
              On position close → Priority reverts to previous level
```

### 5. Webserver API Integration

```
[Dashboard] → GET /api/tokens/list
                  │
                  ▼
              tokens::list_tokens_async(limit)
                  │
                  ▼
              Returns paginated token list with all data sources
```

```
[Token Detail] → GET /api/tokens/:mint
                      │
                      ▼
                 tokens::get_full_token_async(mint)
                      │
                      ▼
                 Returns complete Token with:
                 - DexScreener data (if available)
                 - GeckoTerminal data (if available)
                 - Rugcheck data (if available)
                 - Priority, blacklist status, update tracking
```

### 6. Decimals Integration

All modules requiring token math use the decimals system:

```
[Any Module] → tokens::decimals::get(mint)
                    │
                    ▼
               Memory cache → DB → Chain
                    │
                    ▼
               Returns u8 decimals
                    │
                    ▼
               [Used for: price calculations, swap amounts, position sizing]
```

**Critical users:**

- Pool calculator: Needs decimals for price calculations
- Swap module: Needs decimals for amount conversions
- Positions: Needs decimals for value calculations
- Wallet: Needs decimals for balance display

### 7. Cleanup Integration

```
[Cleanup Loop] → Hourly scan of all tokens
                      │
                      ▼
                 Check Rugcheck data for authorities
                      │
                      ▼
                 If mint_authority OR freeze_authority present:
                      │
                      ▼
                 tokens::database::add_to_blacklist(mint, reason, "auto_cleanup")
                      │
                      ▼
                 Token excluded from filtering on next cycle
```

### Data Flow Summary

```
┌──────────────────────────────────────────────────────────────────┐
│                      TOKENS MODULE (Central)                     │
│  Database: tokens.db                                             │
│  Cache: Memory (LRU + TTL)                                       │
│  Sources: DexScreener, GeckoTerminal, Rugcheck                   │
└──────────────────────────────────────────────────────────────────┘
                            │
        ┌───────────────────┼───────────────────┬───────────────────┐
        ▼                   ▼                   ▼                   ▼
┌──────────────┐   ┌──────────────┐   ┌──────────────┐   ┌──────────────┐
│  Filtering   │   │    Pools     │   │    Trader    │   │  Positions   │
│  (Filter +   │   │  (Available  │   │  (Entry/Exit │   │  (Priority   │
│   Store)     │   │   Tokens)    │   │  Decisions)  │   │   Control)   │
└──────────────┘   └──────────────┘   └──────────────┘   └──────────────┘
        │                   │                   │                   │
        └───────────────────┴───────────────────┴───────────────────┘
                            │
                            ▼
                ┌───────────────────────┐
                │   Webserver (API)     │
                │   Dashboard (UI)      │
                └───────────────────────┘
```

**Key Design Principles:**

1. **Single Source of Truth**: tokens.db is the authoritative data source
2. **Layered Caching**: Memory → Database → API (minimize external calls)
3. **Priority-Based Updates**: Critical (positions) > High (filtered) > Low (all others)
4. **Rate Limiting**: Shared coordinator prevents API limit violations
5. **Async Access**: All public APIs are async to prevent blocking
6. **Separation of Concerns**: Discovery, updates, cleanup, decimals are independent tasks

## Notes

- **All pricing uses SOL as the monetary unit**
- **Price Fallback for Trading**: Position open/close/DCA operations use `get_price_with_api_fallback()` which:
  1. Tries pool price first (real-time on-chain)
  2. Falls back to Token API price (DexScreener/GeckoTerminal) if fresh (< 60s)
  3. If API price is stale, forces immediate market data fetch via `request_immediate_update()` and retries
  4. If token not in database, fetches it on-demand
     This enables trading for tokens not yet tracked by pool service.
- **Process Lock**: File-based lock (data/.screenerbot.lock) prevents multiple instances, acquired at startup before config load, OS auto-releases on crash
- **Service Manager**: Global singleton with dependency resolution, priority-based startup (lower = earlier), reverse-order shutdown (higher = first), and per-service metrics via TaskMonitor
- **Service Priorities**: events(10) → pool_helpers(30), webserver(30) → tokens(40) → ohlcv(45) → positions(50) → transactions(80) → filtering(90), wallet(90) → pool_discovery(100), rpc_stats(100) → pool_fetcher(101) → pool_calculator(102) → pool_analyzer(103) → ata_cleanup(110) → sol_price(120) → trader(150)
- **Service Instrumentation**: All spawned tasks MUST be wrapped with `monitor.instrument(async { ... })` for metrics collection
- **Metrics Collection**: Background task samples `monitor.cumulative()` every 1s (NOT intervals() - that's blocking), accumulates task metrics, cached every 5s for dashboard
- **Pool Service Architecture**: Split into 5 services (pool_helpers, pool_discovery, pool_analyzer, pool_fetcher, pool_calculator) coordinated via channels and global components
- **Token Input for Pool Discovery**: Reads from `tokens::get_passed_tokens()` + ALWAYS includes `positions::get_open_mints()`
- **Pool Discovery Sources**: DexScreener (batch API), GeckoTerminal (per-token), Raydium (config exists but not implemented)
- **Price Calculation Pipeline**: Discovery → Analysis → Fetching (batched RPC, ≤50 accounts) → Calculation (program-specific decoders) → Cache (DashMap) + DB (async)
- **Single Pool Mode**: When enabled, only highest liquidity pool per token is tracked (reduces RPC load)
- **Positions Service Priority**: 50 (starts after tokens, before transactions) with NO dependencies (loads from DB independently)
- **Positions Verification Dependencies**: Worker waits for TRANSACTIONS_SYSTEM_READY + POOL_SERVICE_READY before processing queue
- **Positions Global Semaphore**: Enforces max_open_positions atomically via permit acquisition BEFORE swap execution (prevents race conditions)
- **Positions Permit Lifecycle**: Acquired → "Forgotten" on success (consumed) → Released on verified close/synthetic exit/orphan removal
- **Positions Triple Guard**: In-memory check → Database check → Pending-open check (prevents duplicate opens across restarts)
- **Positions Verification Queue**: Exponential backoff (5s → 300s) with ±10% jitter, batch size 10, priority sorting (due → recent → oldest)
- **Positions State Machine**: All transitions via `apply_transition()` - EntryVerified, ExitVerified, ExitFailedClearForRetry, ExitPermanentFailureSynthetic, RemoveOrphanEntry, UpdatePriceTracking
- **Positions Loss Detection**: Auto-blacklist on close if loss ≤ -15% (adds to tokens.db blacklist with reason "PoorPerformance")
- **Positions Price Tracking**: UpdatePriceTracking persists current price/high/low to DB while keeping per-mint locks for consistency
- **Positions Token Snapshots**: Market data captured at both opening and closing (full DexScreener data stored in token_snapshots table)
- **Positions Database**: positions.db with 5 tables (positions, position_states, position_tracking, position_metadata, token_snapshots) + 14 indexes
- **Positions Integration**: Used by trader (open/close), pool discovery (get_open_mints), OHLCV (priority=Critical), tokens (blacklisting)
- **OHLCV Service Priority**: 45 (starts after tokens=40, before positions=50)
- **OHLCV Dependencies**: ["tokens", "positions"] (ensures they're ready before starting)
- **OHLCV Architecture**: 5 components (Database, Fetcher, Cache, PoolManager, GapManager) + Monitor orchestrator
- **OHLCV Database**: ohlcvs.db with 5 tables (ohlcv_pools, ohlcv_1m, ohlcv_aggregated, ohlcv_gaps, ohlcv_monitor_config) + WAL mode
- **OHLCV Background Loops**: 5 concurrent tasks - monitor_loop(5s), gap_fill_loop(5min), cleanup_loop(1h), cache_maintenance_loop(10min), sync_pool_service_tokens(30s)
- **OHLCV Token Sync**: Every 30 seconds with Pool Service (NOT 5 minutes), syncs available tokens + open positions
- **OHLCV Priority System**: Critical(30s/1000 candles), High(60s/500 candles), Medium(5min/200 candles), Low(15min/100 candles)
- **OHLCV Activity Types**: PositionOpened→Critical, PositionClosed→High, ChartViewed→High, TokenViewed→Medium, DataRequested→High
- **OHLCV Auto-Populate**: Adds all open positions to monitoring on startup (5s delay, Priority::Critical)
- **OHLCV Pool Discovery**: Uses tokens::prefetch_token_pools + tokens::get_token_pools_snapshot\* with exponential backoff (2^(n-1) minutes) and throttled logging (first 3 + every 5th)
- **OHLCV Gap Detection**: Automatic after successful fetches, identifies missing timestamps, backfills every 5 minutes
- **OHLCV Data Aggregation**: On-demand from 1m base data, cached in both DB (ohlcv_aggregated) and memory (LRU)
- **OHLCV Rate Limiting**: Coordinated with GeckoTerminal API (30/min default), delay between tokens ~2100ms
- **OHLCV Retention**: Configurable retention_days (default: 7), hourly cleanup removes old data
- **OHLCV Integration**: Syncs with Pool Service (available tokens), Positions (open positions), Webserver (chart views), Tokens (metadata)
- **API Manager**: Global singleton (LazyLock) with 6 clients - DexScreener, GeckoTerminal, Rugcheck, Jupiter, CoinGecko, DefiLlama
- **API Rate Limiting**: Per-client serialization via Semaphore (capacity=1) with minimum interval calculation (60s / max_per_minute)
- **API Statistics**: Per-client tracking (total/success/failed requests, cache hits/misses, response times, last error) via atomic counters + RwLock
- **API Enabled Flags**: Config-driven per-client (discovery + sources), disabled clients return ApiError::Disabled
- **API Timeouts**: DexScreener(10s), GeckoTerminal(10s), Rugcheck(15s), Jupiter(15s), CoinGecko(20s), DefiLlama(25s)
- **API Default Rate Limits**: DexScreener(300/min), GeckoTerminal(30/min), Rugcheck(60/min), Jupiter/CoinGecko/DefiLlama(unlimited)
- **API DexScreener Endpoints**: 8 total - token pairs, batch (30 max), pair details, search, profiles, boosts (latest/top), orders
- **API GeckoTerminal Endpoints**: 12 total - token pools, OHLCV, trending/top/new pools, pool details/trades, token metadata, dexes list
- **API Rugcheck Endpoints**: 4 total - full report, summary report, platform stats, batch reports
- **API Jupiter Endpoints**: 4 total - recent tokens, top organic score, top traded, top trending (intervals: 5m/1h/6h/24h)
- **API CoinGecko Endpoints**: 1 total - coins list with platform addresses (Solana filtering)
- **API DefiLlama Endpoints**: 2 total - protocols list, token price lookup (solana:{mint})
- **API Clients NOT for Swaps**: Jupiter/GMGN swap quotes/execution in src/swaps/, NOT src/apis/ (direct HTTP, no ApiManager)
- **API Fallback Strategy**: Failed initialization creates disabled client (no crash), logged as WARN
- **API Integration**: Primary consumer is tokens module (discovery + market data + security), also used by pools (discovery) and OHLCV (candles)
- **Transaction Service Priority**: 80 (starts after tokens and pool helpers, before most other services)
- **Transaction Bootstrap**: Two modes - FULL (complete history backfill) and INCREMENTAL (only newer than newest-known)
- **Transaction WebSocket**: Real-time monitoring via logsSubscribe with automatic reconnection and fallback to RPC polling
- **Transaction Analysis**: Industry-standard balance extraction (preBalances/postBalances) with noise filtering (tips, rent, fees)
- **Transaction Integration**: Used by positions (entry/exit verification), wallet (flow metrics), and events (logging)
- **Webserver Service Priority**: 30 (starts with pool_helpers, depends on filtering)
- **Webserver Framework**: Axum with CompressionLayer middleware, no WebSocket (REST polling only)
- **Webserver Binding**: 127.0.0.1:8080 (hardcoded constants, no config section)
- **Webserver AppState**: Minimal state (startup_time) + ServiceManager access helpers (no direct service imports)
- **Webserver Routes**: 16 route modules merged, 7 HTML pages, 3 script route categories (core/pages/ui)
- **Webserver Response Types**: All INLINE in route files (no separate models folder), use success_response()/error_response()
- **Webserver Templates**: All HTML/CSS/JS embedded at compile time via include_str! (no static file server)
- **Webserver Frontend**: Pure ES modules with lifecycle pattern (init/activate/deactivate/dispose), zero inline JS/CSS
- **Webserver Router**: Client-side SPA navigation via /api/pages/:page endpoint, page HTML cached in memory
- **Webserver Config UI**: 100% metadata-driven from config schema annotations, 11 sections exposed (not pools/wallet)
- **Webserver Shutdown**: Global Arc<Notify> with graceful shutdown (awaits in-flight requests)
- **Webserver Polling**: Frontend uses Poller abstraction with auto-managed lifecycle via ctx.managePoller()
- **Swaps Module Architecture**: NOT a service - library of functions for DEX execution (no startup, no background tasks)
- **Swaps Routers**: GMGN (MEV protection) and Jupiter (aggregator), both enabled by default
- **Swaps Quote Strategy**: Concurrent fetching from all enabled routers via `future::join_all()`, best selected by highest output_amount
- **Swaps Execution Flow**: Primary router → Fallback on specific errors (TransactionDropped, Network) → Return primary error if both fail
- **Swaps Fallback Logic**: Jupiter fails → Try GMGN, GMGN fails → Try Jupiter (automatic alternative with fresh quote)
- **Swaps Auto-Blacklisting**: `get_best_quote_for_opening()` tracks "no route" errors, auto-blacklists after repeated failures
- **Swaps Retry Strategy**: Both routers retry up to 3 times (no backoff) for transient API issues
- **Swaps Timeouts**: GMGN 15s quote, Jupiter 15s quote + 20s swap (includes execution)
- **Swaps Terminal Errors**: "No route" errors (GMGN code 40000402, Jupiter 400) don't retry
- **Swaps RPC Integration**: All signing/sending via `get_rpc_client()` (respects rate limits, round-robin, 429 backoff)
- **Swaps Configuration**: 4 sections - gmgn (partner, anti_mev, fee), jupiter (priority_fee, dynamic_cu), raydium (not implemented), slippage (default, exit steps)
- **Swaps ExactIn vs ExactOut**: Opening uses ExactIn (spend exact SOL), closing uses ExactIn (sell exact tokens to avoid ATA mismatch)
- **Swaps Integration**: Called by positions module during open/close operations, NOT invoked by trader directly
- **Swaps Data Types**: UnifiedQuote (comparison), SwapResult (execution), SwapData (API response), RouterType (enum)
- **Single source of truth: data/config.toml**
- **RPC encoding must be jsonParsed (for LUT resolution)**
- **All databases are append-only (no hand-editing)**
- **Service startup is dependency-based with priority ordering (topological sort + circular detection)**
- **Frontend uses ES modules with lifecycle pattern (init/activate/deactivate/dispose)**
- **Token system uses batch APIs where available (DexScreener, GeckoTerminal)**
- **Security data (Rugcheck) is one-time fetch per token, not continuously updated**
- **Decimals cache persists across restarts via database storage**
- **Priority changes are automatic based on open positions (managed by positions module)**

---

**Status:** Complete system flow documented and verified against actual code. Service layer completely reviewed and updated with accurate priorities, dependencies, startup sequence, metrics collection, and shutdown flow. Positions service comprehensively documented with state machine, verification system, semaphore enforcement, and all integration points. Webserver service fully documented with Axum architecture, 16 route modules, ES module frontend structure, metadata-driven config UI, and lifecycle pattern. Swaps module fully documented with multi-router architecture, concurrent quote fetching, automatic fallback, route failure tracking, and comprehensive integration points verified against actual implementation.
