# OHLCV Module Architecture

> ScreenerBot OHLCV Candle Data, Aggregation & Strategy Integration — February 2026

The OHLCV module provides candlestick (Open, High, Low, Close, Volume) data for tokens by fetching from external APIs (GeckoTerminal), aggregating across 7 timeframes, caching in a three-tier system, persisting to SQLite, and delivering data to strategies via `TimeframeBundle`. It manages pool selection with automatic failover, gap detection with backfill, priority-based fetch scheduling, and activity-driven resource allocation.

---

## Table of Contents

1. [Module Overview](#1-module-overview)
2. [Core Data Types](#2-core-data-types)
3. [Service Lifecycle](#3-service-lifecycle)
4. [Monitor](#4-monitor)
5. [Fetcher](#5-fetcher)
6. [Aggregator](#6-aggregator)
7. [Caching Layer](#7-caching-layer)
8. [Database Layer](#8-database-layer)
9. [Gap Detection & Backfill](#9-gap-detection--backfill)
10. [Pool Manager](#10-pool-manager)
11. [Priority System](#11-priority-system)
12. [API Surface](#12-api-surface)
13. [Configuration](#13-configuration)
14. [Integration Points](#14-integration-points)
15. [Performance Patterns](#15-performance-patterns)
16. [Error Handling](#16-error-handling)

---

## 1. Module Overview

### Purpose

The OHLCV module provides the historical and real-time price data that strategies need for signal generation. It:

- Fetches OHLCV candle data from GeckoTerminal API
- Supports 7 timeframes: 1m, 5m, 15m, 1h, 4h, 12h, 1d
- Aggregates base timeframe (1m) into higher timeframes
- Manages pool selection per token with failover and health tracking
- Detects gaps in data and schedules backfill
- Delivers data as `TimeframeBundle` (100 candles per timeframe per token)
- Priority-based scheduling: open positions fetched every 30s, inactive every 15min

### File Structure

```
src/ohlcvs/
├── mod.rs          — Public API re-exports (126 lines)
├── types.rs        — Candle, Timeframe, TimeframeBundle, Priority, errors (598 lines)
├── service.rs      — Service singleton, public API functions (772 lines)
├── monitor.rs      — Background monitoring loop, telemetry (1600+ lines)
├── fetcher.rs      — GeckoTerminal API client, rate limiting (460+ lines)
├── aggregator.rs   — Timeframe aggregation, resampling, VWAP (150 lines)
├── cache.rs        — Three-tier LRU cache (332 lines)
├── database.rs     — SQLite persistence (900+ lines)
├── gaps.rs         — Gap detection and filling (438 lines)
├── manager.rs      — Pool management and failover (440+ lines)
└── priorities.rs   — Priority calculation, activity scoring (192 lines)
```

**Total:** 11 Rust source files, ~6,600 lines of code.

### Key Capabilities

- **7 timeframes** — 1m, 5m, 15m, 1h, 4h, 12h, 1d
- **100-candle bundles** — Each timeframe delivers up to 100 candles (`BUNDLE_CANDLE_COUNT`)
- **Three-tier caching** — Hot (memory, 100 tokens) → Database → API fetch
- **Priority scheduling** — Critical (30s), High (60s), Medium (5m), Low (15m)
- **Activity-driven** — Position opens, chart views, data requests adjust priority
- **Pool failover** — Automatic pool rotation when API returns errors
- **Gap detection** — Identifies and fills data discontinuities
- **Backfill** — Historical data fetching with retry and completion tracking

---

## 2. Core Data Types

### Candle (`types.rs`)

```rust
pub struct Candle {
    pub timestamp: i64,   // Unix timestamp (start of candle)
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}
```

**Methods:**
- `new(timestamp, open, high, low, close, volume)` — Constructor
- `is_valid()` → bool — Validates: high ≥ low, open/close within [low, high]

### Timeframe (`types.rs`)

```rust
pub enum Timeframe {
    Minute1, Minute5, Minute15,
    Hour1, Hour4, Hour12, Day1,
}
```

**Methods:**
| Method | Purpose | Example |
|--------|---------|---------|
| `to_seconds()` | Duration in seconds | Minute5 → 300 |
| `to_api_param()` | GeckoTerminal endpoint | Minute1 → "minute" |
| `to_api_params()` | Endpoint + aggregate value | Minute5 → ("minute", 5) |
| `max_candles_30d()` | Max candles in 30 days | Minute1 → 43200 |
| `backfill_priority()` | Fetch order (1=first) | Minute1 → 1 |
| `all()` | All 7 variants | [Minute1, ..., Day1] |
| `from_str(s)` | Parse string | "5m" → Minute5 |
| `as_str()` | Display string | Minute5 → "5m" |

### TimeframeBundle (`types.rs`)

**The main data structure consumed by strategies:**

```rust
pub const BUNDLE_CANDLE_COUNT: usize = 100;

pub struct TimeframeBundle {
    pub mint: String,
    pub pool_address: String,
    pub timestamp: DateTime<Utc>,
    pub m1: Vec<Candle>,    // 100 most recent 1-minute candles
    pub m5: Vec<Candle>,    // 100 most recent 5-minute candles
    pub m15: Vec<Candle>,   // 100 most recent 15-minute candles
    pub h1: Vec<Candle>,    // 100 most recent 1-hour candles
    pub h4: Vec<Candle>,    // 100 most recent 4-hour candles
    pub h12: Vec<Candle>,   // 100 most recent 12-hour candles
    pub d1: Vec<Candle>,    // 100 most recent daily candles
}
```

**Methods:**
- `new(mint, pool_address)` — Empty bundle
- `get_timeframe(timeframe_str)` → `Option<&Vec<Candle>>` — Get candles by string key
- `is_complete()` → bool — All timeframes have data
- `is_fresh(max_age_seconds)` → bool — Check bundle age
- `total_candles()` → usize — Sum across all timeframes

### Priority (`types.rs`)

```rust
pub enum Priority {
    Critical,  // 30s interval — open positions
    High,      // 60s interval — user viewing
    Medium,    // 300s (5m) — watched tokens
    Low,       // 900s (15m) — inactive
}
```

### TokenOhlcvConfig (`types.rs`)

Per-token monitoring configuration:

```rust
pub struct TokenOhlcvConfig {
    pub mint: String,
    pub priority: Priority,
    pub fetch_interval_seconds: u64,
    pub source: String,
    pub is_active: bool,
    pub last_fetch: Option<DateTime<Utc>>,
    pub last_success: Option<DateTime<Utc>>,
    pub consecutive_failures: u32,
    pub consecutive_empty_fetches: u32,
    pub pool_discovery_failures: u32,
    pub last_pool_discovery: Option<DateTime<Utc>>,
    pub backfill_complete: bool,
    pub pools: Vec<PoolConfig>,
    pub default_pool: Option<String>,
}
```

**Methods:**
- `mark_fetch()` — Record fetch attempt
- `get_default_pool()` / `get_best_pool()` — Pool selection
- `mark_activity()` — Reset empty fetch counter
- `mark_empty_fetch()` — Increment empty counter
- `calculate_adjusted_interval()` — Dynamic interval based on failures
- `should_retry_pool_discovery()` — Exponential backoff for pool discovery

### PoolConfig (`types.rs`)

```rust
pub struct PoolConfig {
    pub address: String,
    pub dex: String,
    pub liquidity: f64,
    pub consecutive_failures: u32,
    pub is_default: bool,
    pub last_success: Option<DateTime<Utc>>,
}
```

### OhlcvError (`types.rs`)

```rust
pub enum OhlcvError {
    DatabaseError(String),
    ApiError(String),
    RateLimitExceeded,
    PoolNotFound(String),
    InvalidTimeframe(String),
    DataGap { start: i64, end: i64 },
    CacheError(String),
    NotFound(String),
}
```

---

## 3. Service Lifecycle

### Singleton Pattern (`service.rs`)

The OHLCV service uses `OnceCell` for singleton initialization:

```rust
pub struct OhlcvService;

impl OhlcvService {
    pub async fn initialize() -> OhlcvResult<()>
    pub async fn start(shutdown: watch::Receiver<bool>) -> OhlcvResult<()>
}
```

### Initialization Flow

```
OhlcvService::initialize()
  → Create data directory
  → Initialize OhlcvDatabase (SQLite)
  → Initialize OhlcvCache (three-tier)
  → Initialize OhlcvFetcher (API client)
  → Initialize PoolManager (pool selection)
  → Initialize GapManager (gap detection)
  → Store all in global OnceCell

OhlcvService::start(shutdown)
  → Create OhlcvMonitor with all components
  → Start monitor background task
  → Monitor runs fetch loops per priority level
```

---

## 4. Monitor

### OhlcvMonitor (`monitor.rs`)

The main orchestrator — 1600+ lines managing fetch scheduling, data flow, and telemetry.

```rust
pub struct OhlcvMonitor {
    db: Arc<OhlcvDatabase>,
    cache: Arc<OhlcvCache>,
    fetcher: Arc<OhlcvFetcher>,
    pool_manager: Arc<PoolManager>,
    gap_manager: Arc<GapManager>,
    configs: RwLock<HashMap<String, TokenOhlcvConfig>>,
    // ... metrics atomics
}
```

### Monitor Flow (per token)

```
┌──────────────────────────────────────────────┐
│         Monitor Fetch Cycle (per token)        │
├──────────────────────────────────────────────┤
│  1. Check priority → determine interval        │
│  2. Select pool (PoolManager)                  │
│  3. Fetch 1m candles (OhlcvFetcher)            │
│  4. Aggregate to higher timeframes             │
│  5. Store in database                          │
│  6. Update cache                               │
│  7. Detect gaps → schedule backfill            │
│  8. Update telemetry                           │
└──────────────────────────────────────────────┘
```

### Aggregated Timeframes

```rust
const AGGREGATED_TIMEFRAMES: [Timeframe; 6] = [
    Timeframe::Minute5, Timeframe::Minute15,
    Timeframe::Hour1, Timeframe::Hour4,
    Timeframe::Hour12, Timeframe::Day1,
];
```

1-minute is the base timeframe fetched from API. All higher timeframes are aggregated from 1m data.

### Token Management

| Method | Purpose |
|--------|---------|
| `add_token(mint, priority)` | Start monitoring a token |
| `remove_token(mint)` | Stop monitoring |
| `update_priority(mint, priority)` | Change fetch frequency |
| `record_activity(mint, activity_type)` | Adjust priority on user action |
| `force_refresh(mint)` | Immediate fetch |

### Telemetry

```rust
pub struct MonitorTelemetrySnapshot {
    pub active_tokens: usize,
    pub total_fetches: u64,
    pub total_errors: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub data_points_stored: u64,
    pub gaps_detected: u64,
    pub gaps_filled: u64,
    pub average_fetch_latency_ms: f64,
    // ... per-priority breakdowns
}
```

---

## 5. Fetcher

### OhlcvFetcher (`fetcher.rs`)

Fetches candle data from GeckoTerminal API with rate limiting.

```rust
const RATE_LIMIT_WINDOW: Duration = Duration::from_secs(60);
const MAX_CANDLES_PER_REQUEST: usize = 1000;

pub struct OhlcvFetcher {
    rate_limiter: Arc<RwLock<RateLimiter>>,
    metrics: Arc<FetcherMetrics>,
}
```

### Fetch Methods

| Method | Purpose |
|--------|---------|
| `fetch_ohlcv(pool, timeframe, limit)` | Standard candle fetch |
| `fetch_with_aggregate(pool, timeframe, aggregate, limit)` | Fetch with aggregation parameter |
| `fetch_immediate(pool, timeframe)` | Bypass queue, immediate fetch |
| `fetch_historical(pool, timeframe, before_timestamp)` | Historical backfill |

### Rate Limiting

- Per-minute rate limit window
- Tracks calls and respects GeckoTerminal API limits
- `average_latency_ms()` and `calls_per_minute()` metrics
- Historical fetch has `MAX_ATTEMPTS = 500` for deep backfill

### API Integration

GeckoTerminal OHLCV endpoint:
- Base timeframe: `/ohlcv/minute?pool={address}`
- Aggregated: `/ohlcv/{timeframe}?pool={address}&aggregate={value}`
- Returns up to 1000 candles per request

---

## 6. Aggregator

### OhlcvAggregator (`aggregator.rs`)

Stateless aggregation utility:

```rust
pub struct OhlcvAggregator;

impl OhlcvAggregator {
    pub fn aggregate(candles_1m: &[Candle], target: Timeframe) -> Vec<Candle>
    pub fn validate_aggregated(data: &[Candle]) -> bool
    pub fn expected_candles(from, to, timeframe) -> usize
    pub fn detect_gaps(data: &[Candle], timeframe) -> Vec<(i64, i64)>
    pub fn interpolate_gaps(data: &[Candle], timeframe) -> Vec<Candle>
    pub fn resample(data: &[Candle], from: Timeframe, to: Timeframe) -> Vec<Candle>
    pub fn calculate_vwap(data: &[Candle]) -> Option<f64>
}
```

### Aggregation Logic

`aggregate(candles_1m, target_timeframe)`:
1. Group 1m candles by target timeframe boundaries
2. For each group: open = first.open, close = last.close, high = max, low = min, volume = sum
3. Validate result ordering and OHLC constraints

### Additional Utilities

- **VWAP:** Volume-Weighted Average Price across candles
- **Gap detection:** Identifies missing candle periods
- **Interpolation:** Fills small gaps with interpolated candles
- **Resampling:** Convert between arbitrary timeframes

---

## 7. Caching Layer

### OhlcvCache (`cache.rs`)

Three-tier caching system:

```rust
const HOT_CACHE_MAX_TOKENS: usize = 100;
const HOT_CACHE_RETENTION_HOURS: i64 = 24;

pub struct OhlcvCache {
    hot: RwLock<HashMap<String, CacheEntry>>,
    hits: AtomicU64,
    misses: AtomicU64,
}
```

### Cache Tiers

```
Tier 1: Hot Cache (in-memory)
  → HashMap<mint, CacheEntry>
  → Max 100 tokens
  → 24-hour retention
  → LRU eviction when full

Tier 2: Database (SQLite)
  → ohlcv_candles table
  → Full history (retention_days config)
  → Queried on cache miss

Tier 3: API (GeckoTerminal)
  → Fetched on DB miss
  → Rate-limited
  → Results populate both Tier 1 and Tier 2
```

### Cache API

| Method | Purpose |
|--------|---------|
| `get(mint, timeframe, limit)` | Read from hot cache |
| `put(mint, timeframe, candles)` | Write to hot cache |
| `invalidate(mint)` | Remove token from cache |
| `clear()` | Clear entire cache |
| `hit_rate()` | Cache effectiveness metric |
| `size()` | Number of cached tokens |
| `cleanup_expired()` | Remove stale entries |

---

## 8. Database Layer

### Schema (`database.rs`)

**4 SQLite tables:**

**ohlcv_pools** — Pool configurations per token:
```sql
CREATE TABLE IF NOT EXISTS ohlcv_pools (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    mint TEXT NOT NULL,
    pool_address TEXT NOT NULL,
    dex TEXT NOT NULL,
    liquidity REAL NOT NULL DEFAULT 0.0,
    consecutive_failures INTEGER DEFAULT 0,
    is_default INTEGER DEFAULT 0,
    last_success TEXT,
    UNIQUE(mint, pool_address)
)
```

**ohlcv_candles** — Candle data (main storage):
```sql
CREATE TABLE IF NOT EXISTS ohlcv_candles (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    mint TEXT NOT NULL,
    pool_address TEXT NOT NULL,
    timeframe TEXT NOT NULL,
    timestamp INTEGER NOT NULL,
    open REAL NOT NULL,
    high REAL NOT NULL,
    low REAL NOT NULL,
    close REAL NOT NULL,
    volume REAL NOT NULL,
    UNIQUE(mint, pool_address, timeframe, timestamp)
)
```

**ohlcv_gaps** — Gap tracking per timeframe:
```sql
CREATE TABLE IF NOT EXISTS ohlcv_gaps (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    mint TEXT NOT NULL,
    pool_address TEXT NOT NULL,
    timeframe TEXT NOT NULL,
    start_timestamp INTEGER NOT NULL,
    end_timestamp INTEGER NOT NULL,
    filled INTEGER DEFAULT 0,
    detected_at TEXT, filled_at TEXT
)
```

**ohlcv_monitor_config** — Per-token monitoring config:
```sql
CREATE TABLE IF NOT EXISTS ohlcv_monitor_config (
    mint TEXT PRIMARY KEY,
    priority TEXT NOT NULL,
    fetch_interval_seconds INTEGER NOT NULL DEFAULT 60,
    source TEXT NOT NULL DEFAULT 'manual',
    is_active INTEGER NOT NULL DEFAULT 1,
    last_fetch TEXT, last_success TEXT,
    consecutive_failures INTEGER DEFAULT 0,
    backfill_complete INTEGER DEFAULT 0
)
```

### OhlcvDatabase API

| Category | Key Methods |
|----------|-------------|
| Pool CRUD | `upsert_pool`, `delete_pool`, `get_pools`, `mark_pool_failure/success` |
| Candle I/O | `insert_candles_batch`, `get_candles` (with time range, limit, order) |
| Gap tracking | `insert_gap`, `get_unfilled_gaps`, `mark_gap_filled`, `get_gap_aggregate` |
| Config | `upsert_monitor_config`, `get_monitor_config`, `get_all_active_configs` |
| Backfill | `is_backfill_complete`, `mark_backfill_complete`, `mark_all_backfills_complete` |
| Cleanup | `cleanup_old_data(retention_days)`, `cleanup_filled_gaps(retention_days)` |
| Stats | `get_data_point_count`, `has_data_for_mint`, `get_mints_with_data`, `get_pool_count`, `get_token_count` |

---

## 9. Gap Detection & Backfill

### GapManager (`gaps.rs`)

Detects and fills discontinuities in candle data:

```rust
pub struct GapManager {
    db: Arc<OhlcvDatabase>,
    fetcher: Arc<OhlcvFetcher>,
}
```

### Detection Flow

1. Load candles for token/pool/timeframe from DB
2. Walk through candles, check timestamp continuity
3. If gap > expected interval → record gap (start, end timestamps)
4. Store in `ohlcv_gaps` table

### Backfill Flow

1. Query `get_unfilled_gaps()` for pending gaps
2. For each gap: `fetch_historical(pool, timeframe, end_timestamp)`
3. Insert fetched candles via `insert_candles_batch()`
4. Mark gap as filled: `mark_gap_filled()`
5. If no data returned → mark gap filled anyway (data unavailable)

### Backfill Completion Tracking

Per-token, per-timeframe backfill tracking:
- `is_backfill_complete(mint, timeframe)` — Check if historical fill done
- `mark_backfill_complete(mint, timeframe)` — Mark one timeframe done
- `mark_all_backfills_complete(mint)` — Mark all timeframes done

---

## 10. Pool Manager

### PoolManager (`manager.rs`)

Manages pool selection and health for OHLCV fetching:

```rust
pub struct PoolManager {
    db: Arc<OhlcvDatabase>,
}
```

### Pool Selection

```rust
pub async fn select_pool_for_fetch(mint) -> OhlcvResult<Option<(String, bool)>>
```

Returns `(pool_address, is_default)`:
1. Try default pool first
2. If default unhealthy → try best pool (highest liquidity, fewest failures)
3. If no pools → trigger pool discovery

### Pool Discovery

```rust
pub async fn discover_pools(mint) -> OhlcvResult<Vec<PoolConfig>>
```

Discovers pools for a token from token module's cached pool data. Registers discovered pools in DB.

### Health Tracking

| Method | Purpose |
|--------|---------|
| `mark_failure(mint, pool)` | Increment failure count |
| `mark_success(mint, pool)` | Reset failure count, update last_success |
| `check_pool_health(mint)` | Get health status per pool |
| `reset_pool_failures(mint, pool)` | Manual reset |

**Health rule:** `is_healthy()` = `consecutive_failures < 5`

### Pool Stats

```rust
pub struct PoolStats {
    pub total_pools: usize,
    pub healthy_pools: usize,
    pub default_pool: Option<String>,
    pub best_pool: Option<String>,
}
```

---

## 11. Priority System

### PriorityManager (`priorities.rs`)

Stateless priority calculation:

```rust
pub struct PriorityManager;

impl PriorityManager {
    pub fn calculate_priority_score(config: &TokenOhlcvConfig) -> u32
    pub fn priority_from_score(score: u32) -> Priority
    pub fn calculate_fetch_interval(config: &TokenOhlcvConfig) -> Duration
    pub fn should_throttle(config: &TokenOhlcvConfig) -> bool
    pub fn throttle_multiplier(consecutive_empty_fetches: u32) -> f64
    pub fn get_recommended_action(config: &TokenOhlcvConfig) -> RecommendedAction
    pub fn update_priority_on_activity(config: &mut TokenOhlcvConfig, activity: ActivityType)
    pub fn calculate_batch_size(priority: Priority) -> usize
    pub fn get_fetch_timeout(priority: Priority) -> Duration
    pub fn should_retry(priority: Priority, attempt: u32) -> bool
    pub fn retry_delay(attempt: u32) -> Duration
}
```

### Activity Types

```rust
pub enum ActivityType {
    PositionOpened,   // → Critical priority
    PositionClosed,   // → High (downgrade from Critical)
    TokenViewed,      // → Medium (if was Low)
    ChartViewed,      // → Medium/High upgrade
    DataRequested,    // → High (immediate)
}
```

### Recommended Actions

```rust
pub enum RecommendedAction {
    FetchNow,           // Data needed immediately
    FetchSoon,          // Within next interval
    WaitForInterval,    // Normal scheduling
    ReduceFrequency,    // Too many empty fetches
    PauseAndRetry,      // Consecutive failures, back off
    RemoveToken,        // Persistent failures, remove from monitoring
}
```

### Throttling

- `should_throttle()` — Returns true if token has too many consecutive empty fetches
- `throttle_multiplier()` — 1.0× for 0 empty, 2.0× for 3+, 4.0× for 10+
- Prevents wasting API calls on tokens with no trading activity

---

## 12. API Surface

### Service API (`service.rs`)

**Data Access:**
| Function | Signature | Purpose |
|----------|-----------|---------|
| `get_ohlcv_data(mint, timeframe, limit)` | `async -> OhlcvResult<Vec<Candle>>` | Get candles |
| `get_timeframe_bundle(mint)` | `async -> OhlcvResult<Option<TimeframeBundle>>` | Get full bundle from cache |
| `build_timeframe_bundle(mint)` | `async -> OhlcvResult<TimeframeBundle>` | Build bundle from DB |
| `store_bundle(mint, bundle)` | `async -> OhlcvResult<()>` | Store bundle in cache |
| `get_available_pools(mint)` | `async -> OhlcvResult<Vec<PoolMetadata>>` | List pools |
| `get_data_gaps(mint, timeframe)` | `async -> OhlcvResult<Vec<(i64, i64)>>` | Get unfilled gaps |
| `has_data(mint)` | `async -> OhlcvResult<bool>` | Check data availability |
| `get_mints_with_data(mints)` | `async -> OhlcvResult<HashSet<String>>` | Batch availability check |
| `get_all_tokens_with_status()` | `async -> OhlcvResult<Vec<OhlcvTokenStatus>>` | All monitored tokens |

**Monitoring Control:**
| Function | Purpose |
|----------|---------|
| `add_token_monitoring(mint, priority)` | Start monitoring |
| `remove_token_monitoring(mint)` | Stop monitoring |
| `update_token_priority(mint, priority)` | Change priority |
| `record_activity(mint, activity_type)` | Record user activity |
| `request_refresh(mint)` | Force immediate fetch |

**Metrics:**
| Function | Purpose |
|----------|---------|
| `get_metrics()` | Overall OHLCV metrics |
| `get_monitor_stats()` | Monitor telemetry snapshot |

---

## 13. Configuration

OHLCV config options (`src/config/schemas/ohlcv.rs`):

| Field | Type | Default | Purpose |
|-------|------|---------|---------|
| `enabled` | bool | true | Master enable/disable of OHLCV monitoring |
| `max_monitored_tokens` | usize | 100 | Max tokens actively monitored (cap memory + API budget) |
| `retention_days` | i64 | 7 | Days to keep historical candle data |
| `max_empty_fetches` | u32 | 10 | Consecutive empty API responses before throttling |
| `auto_fill_gaps` | bool | true | Automatically fetch missing candles when gaps detected |
| `cache_size` | usize | 100 | Max tokens in hot memory cache |
| `cache_retention_hours` | i64 | 24 | Hours to keep tokens in hot cache |
| `pool_failover_enabled` | bool | true | Switch to alternative data source when primary fails |
| `max_pool_failures` | u32 | 5 | Consecutive failures before switching to backup source |
| `sources` | OhlcvSourcesConfig | (below) | OHLCV data sources — independent of `[tokens.discovery.*]` |

### Data Sources (`ohlcv.sources`)

The OHLCV fetcher's API sources are configured under `[ohlcv.sources.*]`,
**independent of token discovery**. Previously the GeckoTerminal client
lived in `[tokens.sources.geckoterminal]` + `[tokens.discovery.geckoterminal]`
and was ANDed with the discovery master switch — turning off discovery
silently disabled OHLCV fetches (264+ errors/min in the latest log).
SolanaTracker is OHLCV-only and used to live under `[tokens.sources.solana_tracker]`.

The shared `ApiManager.geckoterminal` client now stays on if EITHER side
needs it (discovery OR OHLCV), so the two can be enabled/disabled
independently. Endpoints are config-driven (no hardcoded URLs in code).

#### `[ohlcv.sources.geckoterminal]`

| Field | Type | Default | Purpose |
|-------|------|---------|---------|
| `enabled` | bool | true | Enable GeckoTerminal as an OHLCV data source |
| `endpoint` | String | `https://api.geckoterminal.com/api/v2` | API base URL |
| `rate_limit_per_minute` | u32 | 30 | Maximum API requests per minute |
| `timeout_seconds` | u64 | 10 | HTTP request timeout in seconds |

#### `[ohlcv.sources.solana_tracker]` (fallback source, credit-based)

| Field | Type | Default | Purpose |
|-------|------|---------|---------|
| `enabled` | bool | false | Enable SolanaTracker as OHLCV fallback |
| `endpoint` | String | `https://data.solanatracker.io` | API base URL |
| `api_key` | String | (empty) | SolanaTracker API key (required when enabled) |
| `rate_limit_per_minute` | u32 | 30 | Maximum API requests per minute |
| `timeout_seconds` | u64 | 15 | HTTP request timeout in seconds |

### Multi-source fallback chain (`fetcher.rs::fetch_multi_source`)

```
fetch_multi_source(mint, pool_address, api_endpoint, aggregate, limit)
  1. Try SolanaTracker  (uses mint, no pool needed) — if enabled + API key
  2. Fallback to GeckoTerminal (uses pool_address) — if enabled
  3. Return error
```

---

## 14. Integration Points

### Strategies → OHLCV

Strategies consume `TimeframeBundle` for signal generation:
- `get_timeframe_bundle(mint)` — Get cached bundle (fast)
- `build_timeframe_bundle(mint)` — Build fresh from DB
- Strategies access specific timeframes: `bundle.get_timeframe("5m")`

### Positions → OHLCV

Position lifecycle events adjust monitoring priority:
- Position opened → `record_activity(mint, PositionOpened)` → Critical priority
- Position closed → `record_activity(mint, PositionClosed)` → High priority

### Pools → OHLCV

Pool module provides:
- Pool data for OHLCV discovery (`discover_pools`)
- Real-time pool prices (separate from candle data)

### Dashboard → OHLCV

The webserver exposes OHLCV data:
- Chart data endpoints return `TimeframeBundle`
- Token detail views trigger `record_activity(DataRequested)`

---

## 15. Performance Patterns

### Three-Tier Cache Strategy

Hot cache (100 tokens, in-memory) handles repeated reads. DB handles historical queries. API is the fallback of last resort with rate limiting.

### Priority-Based Scheduling

Critical tokens (open positions) get 30s refresh. Low-priority tokens get 15m. This allocates API budget where it matters most.

### Batch Insert

`insert_candles_batch()` uses SQLite `INSERT OR REPLACE` in transactions for bulk efficiency.

### Throttle on Empty

Tokens with no trading activity get progressively longer intervals via `throttle_multiplier()`, preventing wasted API calls.

### Chunk-Based Queries

`get_mints_with_data()` uses `CHUNK_SIZE = 512` for SQL IN clauses to avoid SQLite parameter limits.

---

## 16. Error Handling

### OhlcvError Variants

| Variant | Cause | Recovery |
|---------|-------|----------|
| `DatabaseError` | SQLite failure | Log, continue from cache |
| `ApiError` | GeckoTerminal API failure | Retry with backoff |
| `RateLimitExceeded` | API rate limit hit | Wait, retry next interval |
| `PoolNotFound` | No pool for token | Trigger pool discovery |
| `InvalidTimeframe` | Bad timeframe string | Return error to caller |
| `DataGap` | Missing candle data | Schedule backfill |
| `CacheError` | Cache operation failed | Bypass cache, query DB |
| `NotFound` | Token not monitored | Return error to caller |

### Error Classification (`monitor.rs`)

```rust
fn classify_ohlcv_error(error: &OhlcvError) -> (&'static str, Severity)
```

Classifies errors by severity for logging and telemetry.

### Pool Failover

When a pool fails:
1. Increment `consecutive_failures`
2. If `>= 5` failures → pool marked unhealthy
3. Next fetch selects next best pool
4. If all pools unhealthy → trigger pool discovery
5. Success resets failure counter
