# Pools Module Architecture

> ScreenerBot Pool Discovery, Pricing & Swap System — February 2026

The Pools module provides real-time token pricing by discovering DEX liquidity pools, fetching on-chain account data via Solana RPC, decoding pool state for 11 supported DEX programs, and calculating SOL-denominated prices. It maintains an in-memory price cache with history, a SQLite persistence layer, and a swap execution system for trade operations.

---

## Table of Contents

1. [Module Overview](#1-module-overview)
2. [Core Data Types](#2-core-data-types)
3. [Service Lifecycle](#3-service-lifecycle)
4. [Discovery Pipeline](#4-discovery-pipeline)
5. [Account Fetching](#5-account-fetching)
6. [DEX Decoders](#6-dex-decoders)
7. [Price Calculation](#7-price-calculation)
8. [Pool Analysis](#8-pool-analysis)
9. [Caching Layer](#9-caching-layer)
10. [Database Layer](#10-database-layer)
11. [Blacklisting](#11-blacklisting)
12. [Swap Integration](#12-swap-integration)
13. [API Surface](#13-api-surface)
14. [Configuration](#14-configuration)
15. [Integration Points](#15-integration-points)
16. [Performance Patterns](#16-performance-patterns)
17. [Error Handling](#17-error-handling)

---

## 1. Module Overview

### Purpose

The Pools module is the pricing backbone of ScreenerBot. Every trading decision depends on accurate, real-time token prices. The module:

- Discovers liquidity pools from multiple sources (DexScreener, GeckoTerminal, Raydium)
- Fetches raw on-chain account data via batched RPC calls (50 accounts per batch)
- Decodes pool state using program-specific decoders for 11 DEX programs
- Calculates SOL-denominated prices from pool reserves
- Maintains price history with gap detection for OHLCV candle generation
- Provides swap building and execution for trade operations

### File Structure

```
src/pools/
├── mod.rs                  — Public API exports and re-exports
├── types.rs                — PriceResult, PoolDescriptor, ProgramKind, PriceHistory, PoolError
├── api.rs                  — Public query functions (get_pool_price, get_price_history, etc.)
├── service.rs              — Service lifecycle (initialize, start, stop, health)
├── discovery.rs            — Pool discovery orchestration (5s tick interval)
├── analyzer.rs             — Pool classification and metadata extraction
├── fetcher.rs              — Batched RPC account fetching (500ms tick interval)
├── calculator.rs           — Price calculation from decoded pool data
├── cache.rs                — In-memory DashMap price cache with history and eviction
├── blacklist.rs             — (Empty — functionality lives in database/)
├── utils.rs                — SOL mint detection, token pair analysis, data readers
├── decoders/
│   ├── mod.rs              — PoolDecoder trait and decode_pool() dispatch function
│   ├── raydium_cpmm.rs     — Raydium CPMM (Constant Product) decoder
│   ├── raydium_clmm.rs     — Raydium CLMM (Concentrated Liquidity) decoder
│   ├── raydium_legacy_amm.rs — Raydium Legacy AMM v4 decoder
│   ├── orca_whirlpool.rs   — Orca Whirlpool decoder
│   ├── meteora_damm.rs     — Meteora DAMM v2 decoder
│   ├── meteora_dlmm.rs     — Meteora DLMM (Dynamic Liquidity) decoder
│   ├── meteora_dbc.rs      — Meteora DBC decoder
│   ├── pumpfun_amm.rs      — Pump.fun AMM decoder
│   ├── pumpfun_legacy.rs   — Pump.fun Legacy bonding curve decoder
│   ├── moonit_amm.rs       — Moonit AMM decoder
│   └── fluxbeam_amm.rs     — Fluxbeam AMM decoder
├── database/
│   ├── mod.rs              — Module exports
│   ├── operations.rs       — PoolsDatabase struct, initialization, CRUD
│   ├── global.rs           — Global singleton and public async DB API
│   ├── blacklist.rs        — Blacklist CRUD operations
│   ├── writer.rs           — Async batched write queue
│   └── types.rs            — DbPriceResult, BlacklistedAccountRecord, etc.
└── swap/
    ├── mod.rs              — Swap API exports
    ├── builder.rs          — SwapBuilder: transaction construction
    ├── executor.rs         — SwapExecutor: on-chain execution
    ├── types.rs            — SwapRequest, SwapResult, SwapError, SwapDirection
    └── programs/
        ├── mod.rs          — Program-specific swap dispatch
        ├── raydium_clmm.rs — Raydium CLMM swap instructions
        └── raydium_cpmm.rs — Raydium CPMM swap instructions
```

**Total:** 35 Rust source files, ~13,000 lines of code.

### Key Capabilities

- **11 DEX program decoders** — Raydium (3), Orca, Meteora (3), Pump.fun (2), Moonit, Fluxbeam
- **500ms price refresh** — Batched account fetching every 500ms
- **1000-entry price history** — Per-token ring buffer with gap detection
- **SOL-pair-only pricing** — All prices denominated in SOL (not USD)
- **Single-pool mode** — Optional mode to track only highest-liquidity pool per token
- **Canonical pool selection** — Automatic selection of best pool per token
- **Blacklisting** — Pool and account blacklisting with in-memory fast lookup

---

## 2. Core Data Types

### PriceResult (`types.rs`)

The primary data exchange format for prices throughout the system:

```rust
pub struct PriceResult {
    pub mint: String,              // Token mint address
    pub price_usd: f64,           // USD price (placeholder, not actively used)
    pub price_sol: f64,           // SOL price (PRIMARY — used for all trading)
    pub confidence: f32,          // 0.0–1.0 quality score
    pub source_pool: Option<String>, // Source pool identifier
    pub pool_address: String,     // Primary pool Pubkey
    pub slot: u64,                // Blockchain slot number
    pub timestamp: Instant,       // Monotonic clock (serialized as Unix timestamp)
    pub sol_reserves: f64,        // SOL amount in pool
    pub token_reserves: f64,      // Token amount in pool
}
```

**Methods:**
- `new(mint, price_usd, price_sol, sol_reserves, token_reserves, pool_address)` — Constructor
- `get_utc_timestamp()` — Convert monotonic Instant to chrono DateTime
- `is_fresh(max_age_seconds)` / `is_stale(max_age_seconds)` — Freshness check

**Serialization:** Custom `instant_serde` module handles Instant ↔ Unix timestamp conversion since `std::time::Instant` is a monotonic clock without a fixed epoch.

### PoolDescriptor (`types.rs`)

Analyzed pool metadata used for fetching and calculation:

```rust
pub struct PoolDescriptor {
    pub pool_id: Pubkey,
    pub program_kind: ProgramKind,
    pub base_mint: Pubkey,
    pub quote_mint: Pubkey,
    pub reserve_accounts: Vec<Pubkey>,  // Vault accounts to fetch
    pub liquidity_usd: f64,
    pub volume_h24_usd: f64,
    pub last_updated: Instant,
}
```

### ProgramKind (`types.rs`)

Enum representing all supported DEX programs:

```rust
pub enum ProgramKind {
    RaydiumCpmm,         // Raydium Constant Product Market Maker
    RaydiumLegacyAmm,    // Raydium Legacy AMM v4
    RaydiumClmm,         // Raydium Concentrated Liquidity
    OrcaWhirlpool,       // Orca Whirlpool
    MeteoraDamm,         // Meteora Dynamic AMM v2
    MeteoraDlmm,         // Meteora Dynamic Liquidity Market Maker
    MeteoraDbc,          // Meteora Dynamic Bonding Curve
    PumpFunAmm,          // Pump.fun AMM
    PumpFunLegacy,       // Pump.fun Legacy bonding curve
    Moonit,              // Moonit AMM
    FluxbeamAmm,         // Fluxbeam AMM
    Unknown,             // Unrecognized program
}
```

**Methods:**
- `program_id()` → `&'static str` — Solana program ID constant
- `display_name()` → `&'static str` — Human-readable name
- `from_program_id(id_str)` → Self — Classify by program ID
- `classify(pubkey)` → Self — Classify by Pubkey

### PriceHistory (`types.rs`)

Ring buffer for per-token price history:

```rust
pub struct PriceHistory {
    pub mint: String,
    pub prices: VecDeque<PriceResult>,
    pub max_entries: usize,  // = 1000
}
```

**Gap Detection Constants:**
- `MAX_PRICE_GAP_SECONDS = 60` — Maximum allowed gap between consecutive prices
- `detect_gap_before_price(&new_price)` → Option<gap_index>
- `cleanup_gapped_data()` → removed_count — Removes data before detected gaps
- `has_significant_gaps()` → bool

### PoolError (`types.rs`)

```rust
pub enum PoolError {
    InitializationFailed(String),
    ServiceNotRunning,
    PriceNotAvailable(String),
    RpcError(String),
    DecodeError(String),
}
```

---

## 3. Service Lifecycle

### Global State

The service uses a singleton pattern with static globals:

```rust
static SERVICE_RUNNING: AtomicBool                               // Running flag
static GLOBAL_SHUTDOWN_HANDLE: LazyLock<RwLock<Option<Arc<Notify>>>>  // Shutdown signal
static POOL_DISCOVERY: LazyLock<RwLock<Option<Arc<PoolDiscovery>>>>   // Discovery component
static POOL_ANALYZER: LazyLock<RwLock<Option<Arc<PoolAnalyzer>>>>     // Analyzer component
static ACCOUNT_FETCHER: LazyLock<RwLock<Option<Arc<AccountFetcher>>>> // Fetcher component
static PRICE_CALCULATOR: LazyLock<RwLock<Option<Arc<PriceCalculator>>>> // Calculator component
static DEBUG_TOKEN_OVERRIDE: LazyLock<RwLock<Option<Vec<String>>>>    // Debug mode tokens
```

### Initialization: `initialize_pool_components()`

```
Step 1: Atomic check SERVICE_RUNNING (singleton enforcement)
Step 2: Initialize SQLite database (db::initialize_database())
        → Creates tables: price_history, blacklist_accounts, blacklist_pools
        → Loads blacklists into in-memory HashSet
Step 3: Initialize cache (cache::initialize_cache())
        → Loads price history from DB for open positions
        → Starts 60s cleanup task
Step 4: Create shutdown Notify
Step 5: Initialize service components:
        → PoolDiscovery::new()
        → PoolAnalyzer::new(pool_directory)
        → AccountFetcher::new(pool_directory)
        → PriceCalculator::new(pool_directory)
Step 6: Warm cache for open positions
Step 7: Log configuration (single-pool mode, max tokens)
```

### Background Tasks: `start_helper_tasks()`

Returns `Vec<JoinHandle>` for:
- **Health monitor** — 30s interval, checks component health
- **DB cleanup** — 6h interval, removes old price history
- **Gap cleanup** — 30m interval, removes gapped data

### Shutdown: `stop_pool_service(timeout_seconds)`

1. Set `SERVICE_RUNNING = false`
2. Notify shutdown channel
3. Wait for tasks within timeout
4. Shutdown cache
5. Clear component references

---

## 4. Discovery Pipeline

### Overview

Pool discovery runs on a **5-second tick** cycle in `discovery.rs`:

```
┌─────────────────────────────────────────────────────────┐
│                    Discovery Tick (5s)                    │
├─────────────────────────────────────────────────────────┤
│  1. Build token list:                                    │
│     - Tokens from filtering (passed filter)              │
│     - Tokens with open positions                         │
│     - Debug override tokens (if set)                     │
│     Cap: max_watched_tokens (from config)                │
│                                                          │
│  2. Fetch pool snapshots from tokens module:             │
│     → prefetch_token_pools(mints)                        │
│     → get_token_pools_snapshot(mint)                     │
│     (tokens/ module handles caching, dedup, selection)   │
│                                                          │
│  3. Convert to PoolDescriptor format:                    │
│     → Classify by ProgramKind                            │
│     → Extract reserve accounts                           │
│     → Filter: SOL pairs only, no stablecoins             │
│                                                          │
│  4. Send to analyzer for registration:                   │
│     → AnalyzerMessage::AnalyzePool { ... }               │
└─────────────────────────────────────────────────────────┘
```

### PoolDiscovery State

```rust
pub struct PoolDiscovery {
    known_pools: HashMap<Pubkey, PoolDescriptor>,
    watched_tokens: Vec<String>,
    operations: Arc<AtomicU64>,
    errors: Arc<AtomicU64>,
    pools_discovered: Arc<AtomicU64>,
}
```

### Discovery Sources (configurable)

| Source | Config Flag | Method |
|--------|-----------|--------|
| DexScreener | `pools.enable_dexscreener_discovery` | Token pools from tokens module |
| GeckoTerminal | `pools.enable_geckoterminal_discovery` | Token pools from tokens module |
| Raydium | `pools.enable_raydium_discovery` | Direct RPC discovery |

**Important:** Pool data is fetched and cached by the **tokens module** (`src/tokens/pools/`). The pools discovery module consumes this cached data — it does not make external API calls directly.

### Token Selection

Tokens to watch are selected by:
1. **Filtered tokens** — Tokens that passed the filtering pipeline
2. **Position tokens** — Tokens with open trading positions (always included)
3. **Debug tokens** — Manual override via `set_debug_token_override()`
4. **Cap** — `max_watched_tokens` from config (prevents unbounded growth)

---

## 5. Account Fetching

### AccountFetcher (`fetcher.rs`)

The fetcher batches RPC calls to efficiently retrieve on-chain account data.

**Constants:**
```rust
const ACCOUNT_BATCH_SIZE: usize = 50;          // Max accounts per RPC call
const FETCH_INTERVAL_MS: u64 = 500;            // Fetch tick interval
const ACCOUNT_STALE_THRESHOLD_SECONDS: u64 = 30;     // Normal stale threshold
const OPEN_POSITION_ACCOUNT_STALE_THRESHOLD_SECONDS: u64 = 5;  // Position stale threshold
```

### Core Types

```rust
pub struct AccountData {
    pub pubkey: Pubkey,
    pub data: Vec<u8>,          // Raw account data bytes
    pub owner: Pubkey,          // Program owner
    pub lamports: u64,          // Account balance
    pub slot: u64,              // Slot when fetched
    pub fetched_at: Instant,    // Fetch timestamp
}

pub struct PoolAccountBundle {
    pub pool_id: Pubkey,
    pub accounts: HashMap<Pubkey, AccountData>,
    pub calculation_requested: bool,
    pub last_updated: Instant,
}
```

### Fetch Pipeline

```
┌────────────────────────────────────────────────────────┐
│              Account Fetcher (500ms ticks)               │
├────────────────────────────────────────────────────────┤
│  1. Collect accounts needing refresh:                    │
│     → Stale accounts (>30s for normal, >5s for positions)│
│     → New pools needing initial fetch                    │
│                                                          │
│  2. Batch into groups of 50:                             │
│     → getMultipleAccounts RPC call per batch             │
│                                                          │
│  3. Store in PoolAccountBundle:                          │
│     → Bundle tracks all accounts for a pool              │
│     → is_complete() checks all required accounts present │
│                                                          │
│  4. When bundle complete → request_calculation:          │
│     → Send CalculatorMessage to PriceCalculator          │
└────────────────────────────────────────────────────────┘
```

### Message-based Communication

```rust
pub enum FetcherMessage {
    FetchPool { pool_id: Pubkey, accounts: Vec<Pubkey> },
    FetchAccounts { accounts: Vec<Pubkey> },
    Shutdown,
}
```

The fetcher receives requests via `mpsc::UnboundedSender<FetcherMessage>`.

---

## 6. DEX Decoders

### PoolDecoder Trait (`decoders/mod.rs`)

```rust
pub trait PoolDecoder {
    fn supported_programs() -> Vec<ProgramKind>;
    fn decode_and_calculate(
        accounts: &HashMap<String, AccountData>,
        base_mint: &str,
        quote_mint: &str,
    ) -> Option<PriceResult>;
}
```

### Dispatch Function

`decode_pool(program_kind, accounts, base_mint, quote_mint)` dispatches to the correct decoder based on `ProgramKind`. Returns `None` if decoding fails.

### Supported Decoders

| Decoder | ProgramKind | DEX | Decode Method |
|---------|------------|-----|--------------|
| `RaydiumCpmmDecoder` | RaydiumCpmm | Raydium CPMM | Read vault balances, compute token/SOL ratio |
| `RaydiumClmmDecoder` | RaydiumClmm | Raydium CLMM | Read concentrated liquidity positions, compute from tick |
| `RaydiumLegacyAmmDecoder` | RaydiumLegacyAmm | Raydium AMM v4 | Read AMM state + open orders accounts |
| `OrcaWhirlpoolDecoder` | OrcaWhirlpool | Orca | Read whirlpool state, compute from sqrt_price |
| `MeteoraDammDecoder` | MeteoraDamm | Meteora DAMM v2 | Read vault balances |
| `MeteoraDlmmDecoder` | MeteoraDlmm | Meteora DLMM | Read bin arrays, active bin price |
| `MeteoraDbcDecoder` | MeteoraDbc | Meteora DBC | Read bonding curve state |
| `PumpFunAmmDecoder` | PumpFunAmm | Pump.fun AMM | Read AMM pool state |
| `PumpFunLegacyDecoder` | PumpFunLegacy | Pump.fun Legacy | Read bonding curve, virtual reserves |
| `MoonitAmmDecoder` | Moonit | Moonit | Read AMM pool state |
| `FluxbeamAmmDecoder` | FluxbeamAmm | Fluxbeam | Read pool state |

### Decoding Pattern

Each decoder:
1. Extracts required account data from the `HashMap<String, AccountData>` by Pubkey
2. Deserializes raw bytes into the DEX-specific pool struct
3. Reads reserve amounts (SOL and token)
4. Computes `price_sol = sol_reserves / token_reserves`
5. Constructs and returns a `PriceResult`

---

## 7. Price Calculation

### PriceCalculator (`calculator.rs`)

Orchestrates price computation for all discovered pools.

```rust
pub struct PriceCalculator {
    pool_directory: Arc<RwLock<HashMap<Pubkey, PoolDescriptor>>>,
    sender: mpsc::UnboundedSender<CalculatorMessage>,
    calculations: Arc<AtomicU64>,
    errors: Arc<AtomicU64>,
    updates: Arc<AtomicU64>,
    sol_reference_price: Arc<RwLock<f64>>,  // SOL/USD reference
}
```

### Calculation Flow

```
PoolAccountBundle (complete) → CalculatorMessage::Calculate
                                      ↓
                              decode_pool(program_kind, accounts, ...)
                                      ↓
                              PriceResult (raw from decoder)
                                      ↓
                              Confidence scoring
                                      ↓
                              Canonical pool selection (best pool per mint)
                                      ↓
                              cache::update_price(price)
                                      ↓
                              db::queue_price_for_storage(price)
```

### Canonical Pool Selection

When multiple pools exist for the same token, the calculator selects the **canonical pool** — the one used for official pricing:

- `get_canonical_pool(mint)` — Returns the best pool based on liquidity and confidence
- Higher liquidity pools are preferred
- SOL-paired pools only (stablecoin pairs excluded)

### Messages

```rust
pub enum CalculatorMessage {
    Calculate { pool_id: Pubkey, bundle: PoolAccountBundle },
    UpdateSolPrice(f64),
    Shutdown,
}
```

---

## 8. Pool Analysis

### PoolAnalyzer (`analyzer.rs`)

Classifies discovered pools and prepares them for fetching.

```rust
pub struct PoolAnalyzer {
    pool_directory: Arc<RwLock<HashMap<Pubkey, PoolDescriptor>>>,
    sender: mpsc::UnboundedSender<AnalyzerMessage>,
    analyzed: Arc<AtomicU64>,
    errors: Arc<AtomicU64>,
}
```

### Analysis Steps

For each discovered pool:
1. **Classify** — Determine `ProgramKind` from program ID
2. **Validate** — Check base/quote mints, ensure SOL-paired
3. **Extract** — Get reserve account Pubkeys from pool state
4. **Register** — Add to `pool_directory` for fetcher to use
5. **Request fetch** — Send to `AccountFetcher` for initial data

### Message Types

```rust
pub enum AnalyzerMessage {
    AnalyzePool {
        pool_id: Pubkey,
        program_id: Pubkey,
        base_mint: Pubkey,
        quote_mint: Pubkey,
        liquidity_usd: f64,
        volume_h24_usd: f64,
    },
    Shutdown,
}
```

---

## 9. Caching Layer

### In-Memory Cache (`cache.rs`)

Uses `DashMap` for concurrent access with per-token price history.

**Structure:**
```rust
// Global cache (lazy-initialized)
static PRICE_CACHE: LazyLock<DashMap<String, PriceHistory>>
static CACHE_CLEANUP_RUNNING: AtomicBool
```

**Constants:**
```rust
const HISTORY_EVICTION_SECS: u64 = 7200;  // 2-hour eviction for inactive tokens
```

### Public API

| Function | Purpose |
|----------|---------|
| `initialize_cache()` | Load history for open positions from DB |
| `get_price(mint)` → `Option<PriceResult>` | Latest price if available |
| `update_price(price)` | Insert into history ring buffer |
| `get_available_tokens()` → `Vec<String>` | All mints with prices |
| `get_price_history(mint)` → `Vec<PriceResult>` | Up to 1000 entries |
| `get_cache_stats()` → `CacheStats` | Total prices, fresh count, history entries |
| `load_token_history_from_database(mint)` | Load historical data from SQLite |
| `cleanup_all_memory_gaps()` → (cleaned, total) | Remove gapped data from all tokens |
| `shutdown_cache()` | Stop cleanup task |

### CacheStats

```rust
pub struct CacheStats {
    pub total_prices: usize,
    pub fresh_prices: usize,
    pub history_entries: usize,
}
```

### PriceHistory Ring Buffer

Each token maintains a `VecDeque<PriceResult>` capped at 1000 entries. When a new price arrives:
1. Check for gaps (`MAX_PRICE_GAP_SECONDS = 60`)
2. If gap detected → remove all data before gap (keep only continuous segment)
3. Push new price to back of deque
4. If over 1000 entries → pop from front

### Eviction

A background task runs every 60 seconds:
- Tokens with no price update for >2 hours are evicted from cache
- Tokens with open positions are never evicted

---

## 10. Database Layer

### Schema

Three SQLite tables in the pools database:

**price_history:**
```sql
CREATE TABLE IF NOT EXISTS price_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    mint TEXT NOT NULL,
    price_sol REAL NOT NULL,
    price_usd REAL NOT NULL,
    confidence REAL NOT NULL,
    pool_address TEXT NOT NULL,
    sol_reserves REAL NOT NULL,
    token_reserves REAL NOT NULL,
    slot INTEGER NOT NULL,
    timestamp INTEGER NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
)
```

**blacklist_accounts:**
```sql
CREATE TABLE IF NOT EXISTS blacklist_accounts (
    account TEXT PRIMARY KEY,
    reason TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
)
```

**blacklist_pools:**
```sql
CREATE TABLE IF NOT EXISTS blacklist_pools (
    pool_id TEXT PRIMARY KEY,
    reason TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
)
```

### PoolsDatabase (`database/operations.rs`)

Main database struct with connection pooling:

| Method | Purpose |
|--------|---------|
| `initialize()` | Create tables, indexes, load blacklists |
| `queue_price_for_storage(price)` | Async write to batch writer queue |
| `load_recent_price_history(mint, limit)` | Load recent prices |
| `get_price_history(mint, from, to)` | Time-range query |
| `cleanup_old_entries()` | Remove entries older than 7 days |
| `cleanup_gapped_data_for_token(mint)` | Remove data with >60s gaps |
| `cleanup_all_gapped_data()` | Cleanup all tokens |

### Batch Writer (`database/writer.rs`)

Asynchronous write batching to reduce SQLite contention:
- Prices queued via channel
- Batch written periodically
- Prevents write amplification from individual inserts

### Retention

`MAX_PRICE_HISTORY_AGE_DAYS = 7` — Prices older than 7 days are cleaned up by the 6-hour cleanup task.

---

## 11. Blacklisting

### Pool Blacklisting

Pools can be blacklisted by pool ID. Blacklisted pools are:
- Loaded into in-memory `HashSet` at startup
- Skipped during discovery and analysis
- Persisted in `blacklist_pools` table

### Account Blacklisting

Individual accounts can be blacklisted. Used for:
- Known scam pool accounts
- Broken pool state accounts
- Persisted in `blacklist_accounts` table

### API

Available through `database/blacklist.rs` and `database/global.rs`:
- `add_blacklisted_pool(pool_id, reason)`
- `remove_blacklisted_pool(pool_id)`
- `is_pool_blacklisted(pool_id)` — O(1) via in-memory HashSet
- `add_blacklisted_account(account, reason)`
- `is_account_blacklisted(account)` — O(1) via in-memory HashSet

---

## 12. Swap Integration

### Overview

The `swap/` submodule builds and executes on-chain swap transactions.

### SwapBuilder (`swap/builder.rs`)

Constructs Solana transactions for swap execution:
- Builds swap instructions for the target DEX program
- Adds compute budget instructions
- Handles slippage calculation
- Supports priority fees

### SwapExecutor (`swap/executor.rs`)

Executes constructed swap transactions:
- Sends transaction to RPC
- Handles transaction confirmation
- Retry logic for failed sends
- Returns SwapResult with signature and amounts

### Program-Specific Swaps (`swap/programs/`)

Currently supports direct swap instructions for:
- **Raydium CLMM** — `raydium_clmm.rs`
- **Raydium CPMM** — `raydium_cpmm.rs`

Other DEXes use Jupiter aggregator for swap routing.

### Core Types

```rust
pub struct SwapRequest {
    pub mint: String,
    pub direction: SwapDirection,  // Buy or Sell
    pub amount: u64,               // Lamports or token amount
    pub slippage_bps: u16,
}

pub enum SwapDirection { Buy, Sell }

pub struct SwapResult {
    pub signature: String,
    pub input_amount: u64,
    pub output_amount: u64,
    pub price: f64,
    pub pool_address: String,
}
```

---

## 13. API Surface

### Public Functions (from `mod.rs` and `api.rs`)

**Lifecycle:**
| Function | Signature | Purpose |
|----------|-----------|---------|
| `initialize_pool_components()` | `async -> Result<(), PoolError>` | Initialize all components |
| `start_helper_tasks(shutdown, monitor)` | `-> Vec<JoinHandle>` | Start background tasks |
| `stop_pool_service(timeout)` | `async -> Result<(), PoolError>` | Graceful shutdown |

**Queries:**
| Function | Signature | Purpose |
|----------|-----------|---------|
| `get_pool_price(mint)` | `-> Option<PriceResult>` | Latest price for token |
| `get_available_tokens()` | `-> Vec<String>` | All tokens with prices |
| `get_price_history(mint)` | `-> Vec<PriceResult>` | Up to 1000 history entries |
| `get_token_pools(mint)` | `-> Vec<PoolDescriptor>` | All pools for token |
| `get_cache_stats()` | `-> CacheStats` | Cache metrics |

**Discovery Config:**
| Function | Purpose |
|----------|---------|
| `is_dexscreener_discovery_enabled()` | Check DexScreener discovery |
| `is_geckoterminal_discovery_enabled()` | Check GeckoTerminal discovery |
| `is_raydium_discovery_enabled()` | Check Raydium discovery |

**Service State:**
| Function | Purpose |
|----------|---------|
| `is_pool_service_running()` | Check if service is active |
| `is_single_pool_mode_enabled()` | Check single-pool mode |
| `get_debug_token_override()` | Get debug token list |
| `set_debug_token_override(tokens)` | Set debug token list |

**Component Access:**
| Function | Purpose |
|----------|---------|
| `get_pool_discovery()` | Get Arc<PoolDiscovery> |
| `get_account_fetcher()` | Get Arc<AccountFetcher> |
| `get_price_calculator()` | Get Arc<PriceCalculator> |
| `get_pool_analyzer()` | Get Arc<PoolAnalyzer> |

---

## 14. Configuration

Pool-related configuration lives in `PoolsConfig` (via `config_struct!` macro):

| Field | Type | Default | Purpose |
|-------|------|---------|---------|
| `enable_single_pool_mode` | bool | false | Track only highest-liquidity pool |
| `enable_dexscreener_discovery` | bool | true | Enable DexScreener as pool source |
| `enable_geckoterminal_discovery` | bool | true | Enable GeckoTerminal as pool source |
| `enable_raydium_discovery` | bool | false | Enable Raydium RPC discovery |
| `max_watched_tokens` | usize | 200 | Maximum tokens to track |
| `price_freshness_threshold` | u64 | 30 | Seconds before price considered stale |

---

## 15. Integration Points

### Tokens Module → Pools

The tokens module provides pool discovery data:
- `prefetch_token_pools(mints)` — Triggers pool data fetching for batch of mints
- `get_token_pools_snapshot(mint)` — Returns cached pool data from DexScreener/GeckoTerminal
- Pools module converts this data to `PoolDescriptor` format

### Pools → Filtering

The filtering module uses pool prices for threshold checks:
- Liquidity thresholds (minimum SOL reserves)
- Price availability checks (must have pool price)

### Pools → Positions

Positions depend on real-time prices for:
- Entry/exit price recording
- Unrealized PnL calculation (via price_updater)
- Position tracking (high/low watermarks)

### Pools → OHLCV

OHLCV candles are built from pool price history:
- Base 1-minute candles from PriceResult data
- Gap detection ensures candle continuity
- PriceHistory ring buffer is the data source

### Pools → Strategies

Strategies consume price data for signal generation:
- Real-time price monitoring
- OHLCV candle data (derived from pool prices)

### Pools → Trader

The trader uses pool swap infrastructure:
- SwapBuilder for transaction construction
- SwapExecutor for on-chain execution
- Direct swaps for Raydium CLMM/CPMM pools

---

## 16. Performance Patterns

### Batched RPC Calls

Account fetching is batched at 50 accounts per `getMultipleAccounts` call. This is critical:
- Solana RPC has per-request limits
- Batching reduces round-trips and latency
- 500ms tick interval balances freshness vs RPC cost

### Message-Passing Architecture

All components communicate via `mpsc::UnboundedSender` channels:
- Discovery → Analyzer: `AnalyzerMessage`
- Analyzer → Fetcher: `FetcherMessage`
- Fetcher → Calculator: `CalculatorMessage`

This decouples components and prevents blocking.

### Stale Threshold Differentiation

Different staleness thresholds for different contexts:
- **Normal tokens:** 30s stale threshold
- **Open positions:** 5s stale threshold (need fresher prices for trading)

### Memory Efficiency

- PriceHistory capped at 1000 entries per token
- 2-hour eviction for inactive tokens
- Gap detection prevents accumulating discontinuous data

### Single-Pool Mode

When enabled, only tracks the highest-liquidity pool per token:
- Reduces RPC calls by ~60-80%
- Suitable for most trading scenarios
- Trades off multi-pool price comparison

---

## 17. Error Handling

### PoolError Variants

| Variant | Cause | Recovery |
|---------|-------|----------|
| `InitializationFailed` | DB init, component init | Fatal — service won't start |
| `ServiceNotRunning` | Query before init or after shutdown | Caller handles absence |
| `PriceNotAvailable` | No pool data for token | Return None to caller |
| `RpcError` | RPC call failed | Retry on next tick |
| `DecodeError` | Account data malformed | Skip pool, try next |

### Error Metrics

Each component tracks error counts via `Arc<AtomicU64>`:
- `PoolDiscovery::errors` — Discovery failures
- `AccountFetcher::errors` — Fetch failures
- `PriceCalculator::errors` — Calculation failures

### Recovery Patterns

- **RPC failures:** Automatic retry on next 500ms tick
- **Decode failures:** Skip pool, log error, continue with other pools
- **DB failures:** Log error, continue operating from cache only
- **Stale data:** Fresh data takes priority; stale data used as fallback until refreshed
