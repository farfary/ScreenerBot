# Tokens Module Architecture

**Status:** Production  
**Scale:** 281K+ tokens tracked, 11 database tables, 40+ source files  
**Last Updated:** February 2026

---

## 1. Overview

The Tokens module is the data foundation of ScreenerBot. It discovers tokens from multiple external sources, enriches them with market data (DexScreener + optional GeckoTerminal on-demand), security analysis from Rugcheck, and aggregated pool snapshot data. It maintains state-based scheduling for continuous updates, implements multi-layer caching, and integrates with the filtering system to provide clean, validated token datasets for trading decisions.

**Key Capabilities:**
- Multi-source token discovery (DexScreener, GeckoTerminal, Rugcheck, Jupiter, CoinGecko, DefiLlama)
- Market data from DexScreener + GeckoTerminal (assembly selects preferred source + fallback; GeckoTerminal mainly via discovery/force-update due to rate limits)
- Security assessment via Rugcheck with authority reputation tracking
- State-based update loops (5s to 30s for market data; plus a separate security loop)
- Multi-tier caching (memory + DB + API) with negative caching and stale fallback
- Authority reputation system that auto-discovers and blocks scam factories
- Integration with filtering module for rejection tracking and analytics
- Batch operations to prevent N+1 queries and reduce tokio::spawn overhead

---

## 2. Module Structure

**Base Path:** `ScreenerBot/src/tokens/`

```
src/tokens/
├── mod.rs                       # Module exports, global database ref, system ready flag
├── types.rs                     # Token struct (70+ fields), Priority enum, error types
├── service.rs                   # TokensServiceNew: initialize/start/stop, background tasks
├── decimals.rs                  # Decimals cache (100K, infinite TTL), RPC fetch, negative caching
├── authority_cache.rs           # Authority cache (100K), blocked set (ArcSwap), O(1) checks
├── store.rs                     # Token snapshot cache (30s TTL), metrics
├── events.rs                    # Pub/sub event system, TokenEvent enum
├── search.rs                    # Token search API (mint detection, DexScreener fallback)
├── favorites.rs                 # User favorites CRUD
├── filtered.rs                  # Global storage for filtering results
├── cleanup.rs                   # Hourly blacklist enforcement (authority-only)
├── priorities.rs                # Priority enum (100→10), update interval mapping
│
├── database/
│   ├── schema.rs                # 11 tables + 30+ indexes, schema initialization
│   ├── metadata.rs              # Token identity CRUD (mint, symbol, name, decimals)
│   ├── market.rs                # Market data CRUD (DexScreener + GeckoTerminal)
│   ├── security.rs              # Security data CRUD (Rugcheck)
│   ├── pool_data.rs             # Aggregated pool snapshots
│   ├── blacklist.rs             # Permanent blacklist management
│   ├── priority.rs              # Priority tracking
│   ├── tracking.rs              # Update tracking (error counts, timestamps)
│   ├── rejections.rs            # Rejection history + stats + batch operations
│   ├── authority.rs             # Authority reputation, auto-discovery
│   ├── assembly.rs              # Token reconstruction from multiple sources
│   └── async_api.rs             # 50+ async wrappers, spawn_blocking pattern
│
├── market/
│   ├── mod.rs                   # Market data orchestration
│   ├── dexscreener.rs           # DexScreener API (batch 30, 300/min, txns data)
│   └── geckoterminal.rs         # GeckoTerminal API (30/min, pool_count data)
│
├── pool_data/
│   ├── mod.rs                   # Pool data orchestration
│   ├── api.rs                   # Dual-source pool fetching
│   ├── operations.rs            # Pool merge, canonicalization (scoring algorithm)
│   └── cache.rs                 # Pool cache (60s TTL), stale fallback
│
├── security/
│   ├── mod.rs                   # Security orchestration
│   └── rugcheck.rs              # Rugcheck API (60/min, 5min cache, score interpretation)
│
├── updates/
│   ├── mod.rs                   # Update orchestration
│   ├── core.rs                  # update_token(), update_batch(), force_update()
│   ├── rate_limiter.rs          # RateLimitCoordinator (separate semaphores per API)
│   └── loops.rs                 # State-based update loops (security/uninitialized/pool_sync/open_position/pool_tracked/filter_passed/background)
│
└── discovery.rs                 # Discovery loop (60s), 8+ sources, normalization
```

---

## 3. Core Types

Canonical type definitions live in:

- `src/tokens/types.rs` (Token, DataSource, MarketDataBundle, TokenPoolsSnapshot/Info/Sources, RugcheckData, TokenError, ApiError, ...)
- `src/tokens/priorities.rs` (Priority enum and its integer mapping)

### 3.1 `Token` (primary snapshot)

`Token` is the **primary assembled token snapshot** served to the filtering pipeline, webserver, trader, and tools.

Important notes:

- In Rust, timestamps are `chrono::DateTime<Utc>`.
- In SQLite, timestamps are stored as `INTEGER` (Unix seconds) and converted during assembly.
- Token-2022 detection is **not stored on `Token`**; it is cached in `tokens/decimals.rs` (`TOKEN_2022_CACHE`).

Key fields (non-exhaustive, but type-accurate):

```rust
use chrono::{DateTime, Utc};

pub struct Token {
    pub mint: String,
    pub symbol: String,
    pub name: String,
    pub decimals: u8,
    pub data_source: DataSource,

    pub first_discovered_at: DateTime<Utc>,
    pub blockchain_created_at: Option<DateTime<Utc>>,
    pub metadata_last_fetched_at: DateTime<Utc>,
    pub decimals_last_fetched_at: DateTime<Utc>,
    pub market_data_last_fetched_at: DateTime<Utc>,
    pub security_data_last_fetched_at: Option<DateTime<Utc>>,
    pub pool_price_last_calculated_at: DateTime<Utc>,
    pub pool_price_last_used_pool: Option<String>,

    pub price_usd: f64,
    pub price_sol: f64,
    pub price_native: String,

    // Rugcheck normalized risk score (0-100; HIGHER = MORE RISKY)
    pub security_score_normalised: Option<i32>,
    pub is_rugged: bool,

    pub is_blacklisted: bool,
    pub priority: Priority,
}
```

Other notable `Token` fields (see `src/tokens/types.rs` for the canonical list):

- Market metrics: `market_cap`, `fdv`, `liquidity_usd`
- Timeframed changes/volumes: `price_change_m5/h1/h6/h24`, `volume_m5/h1/h6/h24`
- Activity: `txns_*` buy/sell counts (DexScreener)
- Pool metrics: `pool_count`, `reserve_in_usd`
- Links: `websites`, `socials`
- Security: authorities, Rugcheck scores, `security_risks`, holders, transfer-fee fields
- Bot state: `is_blacklisted`, `priority`, plus last filtering rejection metadata (`last_rejection_*`)

### 3.2 `Priority` (update scheduling)

`Priority` is a state-based enum used for token update scheduling:

- `OpenPosition` (100), `PoolTracked` (75), `FilterPassed` (60), `Uninitialized` (55),
  `Stale` (40), `Standard` (25), `Background` (10)

See: `src/tokens/priorities.rs`.

### 3.3 `DataSource` (market selection)

`DataSource` indicates which API’s market data is currently selected during assembly:

- `DexScreener`, `GeckoTerminal`, `Rugcheck`, `Unknown`

### 3.4 Market bundles

```rust
pub struct MarketDataBundle {
    pub dexscreener: Option<DexScreenerData>,
    pub geckoterminal: Option<GeckoTerminalData>,
}
```

### 3.5 Pool snapshots

Pool snapshots are normalized and aggregated across sources, and keep raw payloads for debugging/UI.

```rust
use chrono::{DateTime, Utc};
use serde_json::Value;

pub struct TokenPoolSources {
    pub dexscreener: Option<Value>,
    pub geckoterminal: Option<Value>,
}

pub struct TokenPoolInfo {
    pub pool_address: String,
    pub dex: Option<String>,
    pub base_mint: String,
    pub quote_mint: String,
    pub is_sol_pair: bool,
    pub liquidity_usd: Option<f64>,
    pub liquidity_token: Option<f64>,
    pub liquidity_sol: Option<f64>,
    pub volume_h24: Option<f64>,
    pub price_usd: Option<f64>,
    pub price_sol: Option<f64>,
    pub price_native: Option<String>,
    pub sources: TokenPoolSources,
    pub pool_data_last_fetched_at: DateTime<Utc>,
    pub pool_data_first_seen_at: DateTime<Utc>,
}

pub struct TokenPoolsSnapshot {
    pub mint: String,
    pub pools: Vec<TokenPoolInfo>,
    pub canonical_pool_address: Option<String>,
    pub pool_data_last_fetched_at: DateTime<Utc>,
}
```

---

## 4. Database Schema

**11 Tables, 30+ Indexes**

### tokens (core metadata)
```sql
CREATE TABLE tokens (
    mint TEXT PRIMARY KEY,
    symbol TEXT,
    name TEXT,
    decimals INTEGER,
    first_discovered_at INTEGER NOT NULL,
    blockchain_created_at INTEGER,
    metadata_last_fetched_at INTEGER NOT NULL,
    decimals_last_fetched_at INTEGER NOT NULL
);
-- Indexes: discovered, blockchain_created, metadata_fetched, 
--          symbol COLLATE NOCASE, name COLLATE NOCASE, discovery_mint composite
```

### market_dexscreener (DexScreener data)
```sql
-- Stores: price_usd, price_sol, price_native, market_cap, fdv, liquidity_usd
--         volume_5m/1h/6h/24h, price_change_*, txns_* (buys/sells)
--         pair_address, chain_id, dex_id, url, image_url, header_image_url, pair_blockchain_created_at
-- Indexes: last_fetch, first_fetch, liquidity DESC
```

### market_geckoterminal (GeckoTerminal data)
```sql
-- Stores: price_usd, price_sol, price_native, market_cap, fdv, liquidity_usd
--         volume_5m/1h/6h/24h, price_change_*
--         pool_count, top_pool_address, reserve_in_usd (UNIQUE fields)
-- Indexes: last_fetch, first_fetch, liquidity DESC
```

### token_pools (aggregated pools)
```sql
CREATE TABLE token_pools (
    mint TEXT NOT NULL,
    pool_address TEXT NOT NULL,
    dex TEXT,
    base_mint TEXT NOT NULL,
    quote_mint TEXT NOT NULL,
    is_sol_pair INTEGER NOT NULL,
    liquidity_usd REAL,
    liquidity_token REAL,
    liquidity_sol REAL,
    volume_h24 REAL,
    price_usd REAL,
    price_sol REAL,
    price_native TEXT,
    sources_json TEXT,
    pool_data_last_fetched_at INTEGER NOT NULL,
    pool_data_first_seen_at INTEGER NOT NULL,
    PRIMARY KEY (mint, pool_address)
);
-- No txns: Pool aggregation, not tracking
```

### security_rugcheck (Rugcheck data)
```sql
-- Stores: score, score_normalised, mint/freeze/update authorities, is_mutable
--         risks (JSON: [{name, value, description, score, level}])
--         holders (JSON: [{address, amount, pct, owner, insider}])
--         markets (raw JSON blob), token_type, liquidity, lp_providers
-- Indexes: last_fetch
```

### blacklist (permanent blocks)
```sql
CREATE TABLE blacklist (
    mint TEXT PRIMARY KEY,
    reason TEXT,
    source TEXT,
    added_at INTEGER
);
-- Criteria: mint_authority OR freeze_authority only
```

### update_tracking (priority + error tracking)
```sql
-- Stores: priority, market_data_last_updated, security_data_last_updated
--         market_error_count, market_error_type, last_error_at
--         security_error_count, last_security_error_type
--         last_rejection_at, last_rejection_reason, last_rejection_source
--         pool_price_last_calculated_at
-- Indexes: market_update ASC, security_update ASC, pool_calc DESC,
--          priority+market composite (for scheduling)
```

### token_favorites (user saved)
```sql
CREATE TABLE token_favorites (
    mint TEXT UNIQUE,
    name TEXT,
    symbol TEXT,
    logo_url TEXT,
    notes TEXT,
    added_at INTEGER
);
```

### rejection_history (time-range analytics)
```sql
CREATE TABLE rejection_history (
    mint TEXT,
    reason TEXT,
    source TEXT,
    rejected_at INTEGER
);
-- Indexes: time DESC, reason+time, mint, mint+source UNIQUE
```

### rejection_stats (hourly aggregates)
```sql
CREATE TABLE rejection_stats (
    bucket_hour INTEGER,  -- Unix timestamp rounded to hour
    reason TEXT,
    source TEXT,
    count INTEGER,
    PRIMARY KEY (bucket_hour, reason, source)
);
-- Optimization: Reduce 260k events to hourly snapshots
```

### authority_reputation (auto-discovery)
```sql
CREATE TABLE authority_reputation (
    authority_address TEXT PRIMARY KEY,
    authority_type TEXT,  -- mint|freeze|update
    total_tokens INTEGER,
    flagged_tokens INTEGER,
    confidence REAL,      -- flagged / total
    is_blocked INTEGER,   -- 1 if blocked
    last_analyzed_at INTEGER
);
-- Indexes: is_blocked (1), confidence DESC
```

**Relationships:**
- All tables keyed by `mint` (44-char base58 Solana address)
- tokens (1) : (0..1) market_dexscreener
- tokens (1) : (0..1) market_geckoterminal
- tokens (1) : (0..1) security_rugcheck
- tokens (1) : (0..N) token_pools
- tokens (1) : (0..1) update_tracking
- tokens (1) : (0..N) rejection_history

---

## 5. Token Lifecycle

```
┌──────────────────────────────────────────────────────────────────────────┐
│                         TOKEN LIFECYCLE                                  │
└──────────────────────────────────────────────────────────────────────────┘

  [1] DISCOVERY
      ├─ Sources: DexScreener, GeckoTerminal, Rugcheck, Jupiter, CoinGecko, DefiLlama
      ├─ Normalization: 44-char base58, not SOL/stablecoins
      ├─ Deduplication: Aggregate by mint
      ├─ Blacklist check: Skip if authority-blacklisted
      ├─ DB check: Skip if already known
      └─ Insert: upsert_token() → Priority: FilterPassed
           │
           ▼
  [2] ENRICHMENT
      ├─ Market Data:
      │   ├─ DexScreener: scheduled updates (batch <=30, 300/min) + txns fields
      │   ├─ GeckoTerminal: discovery + force_update_token (30/min)
      │   ├─ Assembly: chooses preferred source + fallback (cfg.tokens.preferred_market_data_source)
      │   ├─ Prices: USD, SOL, native
      │   ├─ Volumes: 5m, 1h, 6h, 24h
      │   ├─ Changes: 5m, 1h, 6h, 24h
      │   └─ Unique: txns (DexScreener), pool_count (GeckoTerminal)
      │
      ├─ Decimals: RPC fetch (SPL or Token-2022)
      │   ├─ Cache: DECIMALS_CACHE (100K, infinite TTL)
      │   ├─ Side effect: Cache authorities (zero cost)
      │   └─ Negative cache: Failed attempts (24h TTL)
      │
      ├─ Security: Rugcheck (60/min, 5min cache)
      │   ├─ Score: 0-100 (HIGHER = MORE RISKY)
      │   ├─ Risks: Vec<SecurityRisk>
      │   ├─ Authorities: mint/freeze/update
      │   └─ Holders: Top holders + concentration
      │
      └─ Pool Data: DexScreener + GeckoTerminal
          ├─ Fetch: All pools for token
          ├─ Canonicalization: Score-based "best" pool
          └─ Store: TokenPoolsSnapshot
               │
               ▼
  [3] FILTERING
      ├─ Input: get_all_tokens_for_filtering() → 56k tokens with market data
      ├─ Rules: 30+ filters (MCap, liquidity, security, authorities, activity)
      ├─ Authority check: is_blocked_authority() → O(1) from memory
      └─ Output: FilteredTokenLists { passed, rejected, blacklisted }
           │
           ▼
  [4] MONITORING
      ├─ Priority scheduling: OpenPosition (5s) → Background (30s)
      ├─ Update loops: 7 state-based loops (security, uninitialized, pool_sync, open_position, pool_tracked, filter_passed, background)
      ├─ Rate limiting: Separate semaphores per API
      ├─ Error tracking: permanent after 3 consecutive "not listed" market failures (others retry)
      └─ Force updates: API endpoint for on-demand refresh
           │
           ▼
  [5] CLEANUP
      ├─ Blacklist enforcement (hourly): mint_authority OR freeze_authority
      ├─ Authority discovery (5 mins): Auto-detect scam factories
      ├─ Rejection retention: Configurable history cleanup
      └─ Stale data: Exclude tokens without recent market data
```

**Phase Details:**

- **Discovery:** Tokens appear from external sources, deduplicated, blacklist-checked, inserted to DB
- **Enrichment:** Market data stored (DexScreener + optional GeckoTerminal), decimals resolved (RPC), security analyzed (Rugcheck), pools aggregated
- **Filtering:** External module applies 30+ rules, rejected tokens tracked, authority reputation updated
- **Monitoring:** Continuous updates via priority-based scheduling, rate-limited, error-tracked
- **Cleanup:** Authority-based blacklisting (hourly), scam factory auto-discovery (5 mins), history retention

---

## 6. Service Lifecycle

**TokensServiceNew Implementation:**

```rust
┌─────────────────────────────────────────────────────────────────────────┐
│                       SERVICE LIFECYCLE                                 │
└─────────────────────────────────────────────────────────────────────────┘

initialize()
  ├─ Create TokenDatabase (schema auto-initialized via schema.rs)
  ├─ Set global database reference: set_global_database()
  ├─ Preload decimals: Load all from DB → DECIMALS_CACHE (100K entries)
  └─ Load blocked authorities: DB → BLOCKED_AUTHORITIES (ArcSwap)

start()
  ├─ Create RateLimitCoordinator (shared across all loops)
  ├─ Store globally for force_update API
  ├─ Start update loops (updates::start_update_loop) — 7 tasks:
  │   ├─ [1] Security loop (cfg.tokens.update_intervals.security_seconds, default 60s)
  │   ├─ [2] Uninitialized seed loop (10s, fixed)
  │   ├─ [3] Pool priority sync loop (5s, fixed)
  │   ├─ [4] PoolTracked loop (cfg.tokens.update_intervals.pool_tracked_seconds, default 7s)
  │   ├─ [5] OpenPosition loop (cfg.tokens.update_intervals.open_position_seconds, default 5s)
  │   ├─ [6] FilterPassed loop (cfg.tokens.update_intervals.filter_passed_seconds, default 8s)
  │   └─ [7] Background loop (cfg.tokens.update_intervals.background_seconds, default 30s)
  ├─ [8] Rate Limiter Refill (60s interval)
  ├─ [9] Discovery Loop (60s interval, sources per config)
  ├─ [10] Cleanup Loop (3600s interval, blacklist enforcement)
  ├─ [11] Authority Discovery (300s interval, auto-block scams)
  └─ Mark TOKENS_SYSTEM_READY = true

stop()
  ├─ Mark TOKENS_SYSTEM_READY = false
  └─ Clear global database reference
```

**Background tasks (current):**

1. **Security loop** (Rugcheck one-time fetch)
   - Interval: `cfg.tokens.update_intervals.security_seconds` (default 60s)
   - Purpose: Fetch Rugcheck data for tokens that don’t have it yet (1 token per cycle)

2. **Uninitialized seed loop**
   - Interval: 10 seconds (fixed)
   - Purpose: Seed market data for tokens without any market data (batch updates)

3. **Pool priority sync loop**
   - Interval: 5 seconds (fixed)
   - Purpose: Sync Pool service tracked mints into `Priority::PoolTracked` and demote after timeout

4. **OpenPosition loop**
   - Interval: `cfg.tokens.update_intervals.open_position_seconds` (default 5s)
   - Purpose: Keep tokens with active positions fresh

5. **PoolTracked loop**
   - Interval: `cfg.tokens.update_intervals.pool_tracked_seconds` (default 7s)
   - Purpose: Keep pool-tracked tokens fresh

6. **FilterPassed loop**
   - Interval: `cfg.tokens.update_intervals.filter_passed_seconds` (default 8s)
   - Purpose: Keep filter-passed tokens fresh

7. **Background loop**
   - Interval: `cfg.tokens.update_intervals.background_seconds` (default 30s)
   - Purpose: Refresh oldest non-blacklisted tokens in background

8. **Rate limiter refill**
   - Interval: 60 seconds (fixed)
   - Purpose: Adds a new minute of API permits (`RateLimitCoordinator::refill_all()`)
   - Note: unused permits accumulate (token-bucket with effectively unbounded burst capacity)

9. **Discovery loop**
   - Interval: 60 seconds (fixed), initial delay 10 seconds
   - Sources: Driven by `cfg.tokens.discovery.*` per-provider/per-endpoint toggles
   - Skip Condition: Tools active (reducing RPC contention)

10. **Cleanup loop**
   - Interval: 3600 seconds (fixed)
   - Purpose: Enforce authority-based blacklist

11. **Authority discovery**
   - Interval: 300 seconds (fixed), warmup delay 60 seconds
   - Purpose: Auto-discover scam authorities and refresh the in-memory blocked set from DB

**Startup Preloading:**

- **Decimals Cache:** All token decimals loaded into memory (critical for sync pool decoders)
- **Blocked Authorities:** Loaded from DB, stored in ArcSwap for atomic updates
- **Impact:** Zero DB lookups during filtering (O(1) checks)

**Shutdown Sequence:**

- Set system ready flag to false (stops new requests)
- Background tasks self-terminate on next iteration
- Database reference cleared
- No explicit task cancellation (graceful shutdown)

---

## 7. Discovery System

**8+ Token Sources:**

```
┌─────────────────────────────────────────────────────────────────────────┐
│                        DISCOVERY SOURCES                                │
└─────────────────────────────────────────────────────────────────────────┘

DexScreener (3 endpoints)
  ├─ latest_profiles (60/min) → New token profiles
  ├─ latest_boosts (60/min) → Boosted tokens
  └─ top_boosts (60/min) → Top boosted tokens

GeckoTerminal (3 endpoints)
  ├─ new_pools → New liquidity pools (mint extraction)
  ├─ recently_updated → Updated tokens (mint + metadata)
  └─ trending → Trending tokens (mint extraction)

Rugcheck (4 endpoints)
  ├─ new_tokens → New verified tokens (mint + symbol + decimals)
  ├─ recent → Recently analyzed (mint + symbol + name)
  ├─ trending → Trending analysis (mint only)
  └─ verified → Verified tokens (mint + symbol + name)

Jupiter (4 endpoints)
  ├─ recent → Recent swaps (mint + symbol + name + decimals)
  ├─ top_organic → Organic volume (mint + symbol + name + decimals)
  ├─ top_traded → Most traded (mint + symbol + name + decimals)
  └─ top_trending → Trending (mint + symbol + name + decimals)

CoinGecko (1 endpoint)
  └─ markets → Solana markets (extract mint + name)

DefiLlama (1 endpoint)
  └─ protocols → Solana protocols (extract mint + name)
```

**Discovery Loop Flow:**

```rust
every 60 seconds:
  ├─ Check if tools running → skip (reduce RPC contention)
  ├─ Fetch from all enabled sources in parallel
  ├─ Normalize mints:
  │   ├─ Must be 44-char base58
  │   ├─ Exclude SOL, USDC, USDT (hardcoded stablecoins)
  │   └─ Exclude invalid addresses
  ├─ Aggregate by mint:
  │   ├─ CandidateAggregate { mint, symbol, name, decimals, sources[] }
  │   └─ Deduplicate: One record per unique mint
  ├─ Blacklist check → skip if blacklisted
  ├─ DB check → mark as "already_known" if exists
  ├─ Insert new tokens:
  │   ├─ upsert_token(mint, symbol, name, decimals)
  │   ├─ Set priority: FilterPassed (60)
  │   └─ Emit: TokenDiscovered event
  └─ Output: DiscoveryStats {
      total_candidates,
      unique_mints,
      newly_added,
      already_known,
      blacklisted,
      invalid,
      errors: Vec<(source, error)>
  }
```

**Normalization Rules:**

- **Mint validation:** Must be 44-character base58 (Solana address format)
- **Exclusions:** SOL, USDC, USDT, USDC-native, WSOL
- **Deduplication:** Aggregate metadata from multiple sources per mint
- **Symbol priority:** Use first non-empty symbol from sources
- **Name priority:** Use first non-empty name from sources
- **Decimals priority:** Use first non-null decimals from sources

**Blacklist Check:**

- Checked before insertion to avoid polluting DB
- Only authority-based blacklist (mint_authority OR freeze_authority)
- Not liquidity-based, not score-based

**DiscoveryStats Output:**

- Published to system events for dashboard
- Logged for monitoring
- Tracks discovery effectiveness per source

---

## 8. Market Data Pipeline

### DexScreener Pipeline

**Endpoints:**
- Batch (market data): `GET /tokens/v1/solana/{mint1},{mint2},...` (max 30 per request)
- Full pools for one token: `GET /token-pairs/v1/solana/{tokenAddress}`

**Rate Limits:**
- Batch: 300/min
- Profiles: 60/min (discovery)
- Boosts: 60/min (discovery)
- Pools: 300/min (pool data)

**Flow:**

```rust
fetch_dexscreener_data_batch(mints: Vec<String>)
  ├─ Acquire permit from RateLimitCoordinator (300/min budget)
  ├─ Check cache (30s TTL) → return if hit
  ├─ Call API: GET /tokens/v1/solana/{ids_csv} (batch <=30)
  ├─ Parse response → Vec<DexScreenerPool>
  ├─ Extract per mint:
  │   ├─ Prices: price_usd, price_sol, price_native
  │   ├─ Changes: 5m, 1h, 6h, 24h
  │   ├─ Market: market_cap, fdv, liquidity_usd
  │   ├─ Volumes: 5m, 1h, 6h, 24h
  │   ├─ Txns: buys/sells for 5m, 1h, 6h, 24h (UNIQUE to DexScreener)
  │   ├─ Pair: pair_address, dex, quote_token, base_token
  │   └─ Images: image_url, header_image_url
  ├─ Create DexScreenerData struct
  ├─ Store to cache (30s TTL)
  ├─ Store to DB: upsert_dexscreener_data()
  └─ Update timestamp: market_data_last_fetched_at
```

**Unique Fields (DexScreener only):**
- `txns_5m/1h/6h/24h_buys: Option<i64>`
- `txns_5m/1h/6h/24h_sells: Option<i64>`
- `header_image_url: Option<String>`
- `pair_blockchain_created_at: Option<i64>`

**Caching Strategy:**
- **TTL:** 30 seconds (aggressive refresh for price volatility)
- **Capacity:** 2000 entries (Moka cache)
- **Metrics:** hits/misses/expirations/evictions
- **Negative caching:** Not implemented (NotFound = skip)

### GeckoTerminal Pipeline

**Endpoints:**
- Single: `GET /v1/networks/solana/tokens/{mint}`
- Batch: `GET /v1/networks/solana/tokens/multi/{mints}` (CSV)

**Rate Limits:**
- Market: 30/min (configurable via `geckoterminal_rate_limit_per_minute`)
- Discovery: Shared 30/min budget

**Flow:**

```rust
fetch_geckoterminal_data(mint: String)
  ├─ Acquire permit from RateLimitCoordinator (30/min budget)
  ├─ Check cache (60s TTL) → return if hit
  ├─ Call API: GET /v1/networks/solana/tokens/{mint}
  ├─ Parse response → GeckoTerminalToken
  ├─ Extract fields:
  │   ├─ Prices: price_usd, price_sol, price_native
  │   ├─ Changes: 5m, 1h, 6h, 24h
  │   ├─ Market: market_cap, fdv, liquidity_usd
  │   ├─ Volumes: 5m, 1h, 6h, 24h
  │   ├─ Pool metrics: pool_count, top_pool_address, reserve_in_usd (UNIQUE)
  │   └─ Image: image_url (no header_image_url)
  ├─ Create GeckoTerminalData struct
  ├─ Store to cache (60s TTL)
  ├─ Store to DB: upsert_geckoterminal_data()
  └─ Update timestamp: market_data_last_fetched_at
```

**Unique Fields (GeckoTerminal only):**
- `pool_count: Option<i64>` - Number of liquidity pools
- `top_pool_address: Option<String>` - Highest liquidity pool
- `reserve_in_usd: Option<f64>` - Total reserves across pools

**Caching Strategy:**
- **TTL:** 60 seconds (slower updates than DexScreener)
- **Capacity:** 2000 entries (Moka cache)
- **Rationale:** GeckoTerminal data updates less frequently

### Source Preference & Fallback

**Config:**
```rust
tokens.preferred_market_data_source: "dexscreener" | "geckoterminal"
```

**Assembly Strategy (database/assembly.rs):**

```rust
get_full_token(mint)
  ├─ Fetch token metadata
  ├─ Fetch both market sources (DexScreener + GeckoTerminal)
  ├─ Use preferred source:
  │   ├─ If DexScreener preferred → use DexScreener data
  │   └─ If GeckoTerminal preferred → use GeckoTerminal data
  ├─ Fallback if primary missing:
  │   ├─ If DexScreener preferred but missing → use GeckoTerminal
  │   └─ If GeckoTerminal preferred but missing → use DexScreener
  ├─ Image fallback:
  │   ├─ Use DexScreener images if available
  │   └─ Fallback to GeckoTerminal image_url
  └─ Assemble Token struct with chosen source
```

**Why Dual Sources?**
- **Redundancy:** If one API is down, fallback available
- **Coverage:** Different tokens may only exist on one source
- **Data richness:** Combine txn data (DexScreener) with pool metrics (GeckoTerminal)
- **User preference:** Some users trust one source over another

---

## 9. Security Pipeline

### Rugcheck Integration

**Endpoint:**
- `GET /v1/tokens/{mint}` - Comprehensive security analysis

**Rate Limit:**
- 60/min (configurable via `rugcheck_rate_limit_per_minute`)

**Flow:**

```rust
fetch_rugcheck_data(mint: String)
  ├─ Acquire permit from RateLimitCoordinator (60/min budget)
  ├─ Check cache (5min TTL) → return if hit
  ├─ Call API: GET /v1/tokens/{mint}
  ├─ Parse response → RugcheckInfo struct
  ├─ Extract fields:
  │   ├─ Scores:
  │   │   ├─ score: Raw risk score (0-150000+, HIGHER = MORE RISKY)
  │   │   └─ score_normalised: 0-100 (HIGHER = MORE RISKY)
  │   ├─ Authorities:
  │   │   ├─ mint_authority: Option<String>
  │   │   ├─ freeze_authority: Option<String>
  │   │   ├─ update_authority: Option<String>
  │   │   └─ is_mutable: bool
  │   ├─ Risks: Vec<SecurityRisk>
  │   │   └─ { name, value, description, score, level }
  │   ├─ Holders: Vec<TokenHolder>
  │   │   └─ { address, amount, pct, owner, insider }
  │   ├─ Markets: Raw JSON blob (full market data)
  │   ├─ Token metadata:
  │   │   ├─ token_type: standard|token2022|burn
  │   │   ├─ liquidity: Option<f64>
  │   │   ├─ lp_providers: Option<i64>
  │   │   ├─ creator_balance_pct: Option<f64>
  │   │   └─ transfer_fee_pct: Option<f64>
  │   └─ Timestamps: last_analyzed_at
  ├─ Create RugcheckData struct
  ├─ Store to cache (5min TTL)
  ├─ Store to DB: upsert_rugcheck_data()
  │   ├─ risks stored as JSON
  │   ├─ holders stored as JSON
  │   └─ markets stored as raw JSON blob
  └─ Update timestamp: security_data_last_fetched_at
```

**Score Interpretation:**

⚠️ **Confusing Naming Convention:**
- Rugcheck raw score: 0-150000+ (HIGHER = MORE RISKY)
- Rugcheck normalized: 0-100 (HIGHER = MORE RISKY)
- But SecurityLevel naming is inverted for UI display

```rust
// Actual mapping (confusing but intentional):
SecurityLevel::Dangerous  → score 0-19   (GOOD for filtering, low risk)
SecurityLevel::Risky      → score 20-39  (moderate risk)
SecurityLevel::Moderate   → score 40-59  (medium risk)
SecurityLevel::Good       → score 60-79  (high risk)
SecurityLevel::Safe       → score 80-100 (VERY RISKY)

// This means:
// - Lower score = safer token
// - Higher score = riskier token
// - "Dangerous" label = actually safe (low risk score)
// - "Safe" label = actually risky (high risk score)
```

**Caching Strategy:**
- **TTL:** 5 minutes (300 seconds)
- **Capacity:** 3000 entries (Moka cache)
- **Rationale:** Security data updates infrequently, longer cache acceptable
- **One-time fetch:** Most tokens fetched once, then cached indefinitely in DB

### Authority Reputation System

**Components:**

1. **Authority Cache (authority_cache.rs):**
   - **AUTHORITIES_CACHE:** Moka cache (100K entries), stores MintAuthorities
   - **BLOCKED_AUTHORITIES:** ArcSwap<DashSet>, atomic replacement
   - Population: Side effect of decimals.rs RPC calls (zero extra cost)

2. **Authority Discovery (database/authority.rs):**

```rust
run_authority_discovery()
  ├─ Query security_rugcheck for all authorities
  ├─ Group tokens by each authority address
  ├─ Cross-reference with rejection_history (last_rejection_at != NULL)
  ├─ Compute per authority:
  │   ├─ total_tokens: Count of tokens with this authority
  │   ├─ flagged_tokens: Count of rejected tokens
  │   ├─ confidence: flagged_tokens / total_tokens
  │   └─ is_blocked: confidence >= 0.8 AND total_tokens >= 5
  ├─ Upsert to authority_reputation table
  └─ Refresh in-memory cache:
      └─ BLOCKED_AUTHORITIES.store(new_set)  // Atomic swap
```

**Blocking Criteria:**
- **Minimum tokens:** 5+ tokens with this authority
- **Minimum confidence:** 80%+ of tokens rejected
- **Result:** Block entire authority (factory-level blocking)

**Integration with Filtering:**

```rust
// In filtering engine:
if is_blocked_authority(&token.mint_authority) 
   || is_blocked_authority(&token.freeze_authority)
   || is_blocked_authority(&token.update_authority) {
    reject_token("Blocked authority");
}

// O(1) lookup (no DB access):
fn is_blocked_authority(address: &Option<String>) -> bool {
    match address {
        Some(addr) => BLOCKED_AUTHORITIES.load().contains(addr),
        None => false,
    }
}
```

**Authority Cache Enrichment:**

- **Populated during:** decimals.rs RPC calls (get_account_info)
- **Zero cost:** Already fetching mint account, parse authorities
- **Fallback:** Rugcheck authorities if RPC data unavailable
- **Usage:** Filtering checks authorities without DB/RPC calls

---

## 10. Pool Data Pipeline

### Dual-Source Fetching

**Sources:**
1. **DexScreener:** `fetch_token_pools(mint)` - All pools for token
2. **GeckoTerminal:** `fetch_pools(mint)` - All pools for token

**Flow:**

```rust
get_token_pools_snapshot(mint)
  ├─ Fetch DexScreener pools (parallel)
  ├─ Fetch GeckoTerminal pools (parallel)
  ├─ Convert to standardized TokenPoolInfo:
  │   ├─ pool_address: String
  │   ├─ dex: String (raydium, orca, meteora, etc)
  │   ├─ liquidity_usd: Option<f64>
  │   ├─ volume_24h: Option<f64>
  │   └─ sources_json: Raw API payload for debugging
  ├─ Merge pools:
  │   ├─ Deduplicate by pool_address
  │   └─ Prefer higher liquidity if duplicate
  ├─ Choose canonical pool:
  │   └─ Scoring algorithm (see below)
  └─ Create TokenPoolsSnapshot:
      ├─ pools: Vec<TokenPoolInfo> (all pools)
      ├─ canonical_pool_address: Selected "best" pool
      └─ pool_data_last_fetched_at: now
```

### Canonicalization Algorithm

Canonical selection is implemented in `tokens/pool_data/operations.rs`:

- Pools are merged/deduplicated by `pool_address` (`ingest_pool_entry` + `merge_pool_info`).
- Canonical selection (`choose_canonical_pool`) considers **SOL-paired pools only**.
- Primary metric: `calculate_pool_metric(pool)` where:
  - `liquidity_sol` > `liquidity_usd` > `volume_h24` (first non-`None`)
- Tie-breaker when metrics are equal: higher `volume_h24`.

This ensures the canonical pool is the most liquid SOL pair, with volume as a tie-breaker.

```rust
// utils.rs
fn calculate_pool_metric(pool: &TokenPoolInfo) -> f64 {
    pool.liquidity_sol
        .or(pool.liquidity_usd)
        .or(pool.volume_h24)
        .unwrap_or(0.0)
}

// operations.rs (simplified)
fn choose_canonical_pool(pools: &[TokenPoolInfo]) -> Option<String> {
    pools.iter()
        .filter(|p| p.is_sol_pair)
        .max_by(|a, b| calculate_pool_metric(a).partial_cmp(&calculate_pool_metric(b)).unwrap())
        .map(|p| p.pool_address.clone())
}
```

### Storage & Caching

**Database Storage:**

```rust
replace_token_pools(snapshot: TokenPoolsSnapshot)
  ├─ Delete all existing pools for mint
  ├─ Insert all new pools:
  │   └─ (mint, pool_address, dex, liquidity_usd, volume_24h, sources_json)
  └─ Result: Complete replacement (not incremental)
```

**Cache Strategy (pool_data/cache.rs):**

```rust
TOKEN_POOLS_CACHE {
    type: Moka sync cache,
    max_capacity: 5000 entries,
    time_to_live: 120 seconds,         // cache retention
    freshness_ttl: 60 seconds,         // "fresh" check (TOKEN_POOLS_TTL_SECS)
    stale_fallback: true               // persisted snapshot + allow_stale path
}

POOL_REFRESH_INFLIGHT {
    type: AsyncMutex<HashMap<mint, Notify>>,
    purpose: single-flight refresh per mint (avoid duplicate refresh storms)
}

POOL_PREFETCH_STATE {
    type: Moka sync cache,
    max_capacity: 5000 entries,
    time_to_live: 60 seconds,
    purpose: debounce background refresh scheduling
}

get_snapshot(mint)
  ├─ Check cache (60s) → return if hit
  ├─ Fetch fresh from APIs
  ├─ Store to cache
  └─ Return snapshot

get_snapshot_allow_stale(mint)
  ├─ Check cache (60s) → return if hit
  ├─ Fetch fresh from APIs
  │   └─ If fetch fails:
  │       ├─ Check DB for stale data
  │       └─ Return stale if available (better than nothing)
  ├─ Store to cache
  └─ Return snapshot
```

**Stale Fallback Pattern:**
- Used by pool service when APIs are down
- Prevents complete failure due to transient network issues
- Stale data better than no data for pool tracking

---

## 11. Update System

### Core Update Functions

**update_token(mint, db, coordinator):** (scheduled market update)

```rust
update_token(mint: String, db: &TokenDatabase, coordinator: &RateLimitCoordinator)
  ├─ Acquire DexScreener batch permit (1 request budget)
  ├─ Fetch market data (DexScreener only)
  ├─ Mark market_data_updated on success
  └─ Return UpdateResult { mint, successes, failures }

Note: Security data (Rugcheck) is fetched by a separate loop (`update_security_data`) and is not part of every market update cycle.
```

**update_tokens_batch(mints, db, coordinator):**

```rust
update_tokens_batch(mints: Vec<String>, ...)
  ├─ Filter out in-flight mints (shared across loops)
  ├─ Fetch DexScreener market batch (single API call per <=30 mints)
  ├─ Mark market_data_updated for tokens with market data
  └─ Return Vec<UpdateResult> (one per mint)
```

**force_update_token(mint, db, coordinator):**

```rust
force_update_token(mint: String, ...)
  ├─ Bypass normal scheduling queue (user-initiated)
  ├─ Fetch in parallel (tokio::join!):
  │   ├─ DexScreener (market)
  │   ├─ GeckoTerminal (market)
  │   └─ Rugcheck (security)
  ├─ Consume permits only on success (permit.forget())
  ├─ Mark market_data_updated if any market source succeeds
  └─ Return UpdateResult { successes, failures }
  
// Used by:
// - Token detail page (user-triggered refresh)
// - Admin tools (manual updates)
// - API endpoints (/api/tokens/{mint}/force-update)
```

### Rate Limiter Architecture

**RateLimitCoordinator (updates/rate_limiter.rs):**

```rust
pub struct RateLimitCoordinator {
    // DexScreener endpoints (separate limits per endpoint)
    dexscreener_batch_sem: Arc<Semaphore>,     // 300/min
    dexscreener_profiles_sem: Arc<Semaphore>,  // 60/min
    dexscreener_boosts_sem: Arc<Semaphore>,    // 60/min
    dexscreener_pools_sem: Arc<Semaphore>,     // 300/min
    dexscreener_batch_budget: usize,
    dexscreener_profiles_budget: usize,
    dexscreener_boosts_budget: usize,
    dexscreener_pools_budget: usize,

    // Other API endpoints (budgets can be overridden by config)
    geckoterminal_sem: Arc<Semaphore>,         // default 30/min
    rugcheck_sem: Arc<Semaphore>,              // default 60/min
    geckoterminal_budget: usize,
    rugcheck_budget: usize,
}

// Separate semaphores prevent blocking:
// - Discovery (60/min) doesn't block market updates (300/min)
// - Rugcheck (60/min) doesn't block DexScreener (300/min)
// - Pool fetches (300/min) independent from market updates
```

**Refill Mechanism:**

```rust
refill_all()
  ├─ Add budgets back to each semaphore once per minute
  └─ Note: unused permits accumulate (carryover burst behavior)
```

Permits are only consumed when requests succeed (`permit.forget()`); on error the permit is dropped and returns to the semaphore.

### Update Loops

**State-based update loops (updates/loops.rs):**

```rust
┌─────────────────────────────────────────────────────────────────────────┐
│                         UPDATE LOOPS                                    │
└─────────────────────────────────────────────────────────────────────────┘

SECURITY LOOP (cfg.tokens.update_intervals.security_seconds, default 60s)
  ├─ Targets: tokens without Rugcheck data (1 token per cycle)
  └─ Purpose: one-time security fetch to avoid large backlogs

UNINITIALIZED SEED LOOP (10s fixed)
  ├─ Targets: tokens without market data (batch size 30)
  └─ Purpose: seed market data quickly after discovery

POOL PRIORITY SYNC LOOP (5s fixed)
  ├─ Targets: pools::get_available_tokens()
  └─ Purpose: promote/demote PoolTracked priority based on pool service state

OPEN POSITION LOOP (cfg.tokens.update_intervals.open_position_seconds, default 5s)
  ├─ Targets: Priority::OpenPosition (limit ~200)
  └─ Updates: chunks of 30 via update_tokens_batch (join_all for concurrency)

POOL TRACKED LOOP (cfg.tokens.update_intervals.pool_tracked_seconds, default 7s)
  ├─ Targets: Priority::PoolTracked (limit ~200; process up to ~90)
  └─ Updates: chunks of 30 via update_tokens_batch

FILTER PASSED LOOP (cfg.tokens.update_intervals.filter_passed_seconds, default 8s)
  ├─ Targets: Priority::FilterPassed (limit ~200; process up to ~60)
  └─ Updates: chunks of 30 via update_tokens_batch

BACKGROUND LOOP (cfg.tokens.update_intervals.background_seconds, default 30s)
  ├─ Targets: oldest non-blacklisted tokens (limit 30)
  └─ Updates: one batch via update_tokens_batch
```

All loops share the same invariants:

- They skip work while tools are active (`are_tools_active()`) to reduce RPC contention.
- They avoid duplicate fetches using a shared in-flight set across loops.
- They batch market updates in chunks of **30** for the DexScreener `/tokens/v1` limit.
- They avoid wasting resources on permanently-not-listed tokens via the market failure handler.

### Error Tracking

**Per-Token Error State (update_tracking table):**

```rust
market_error_count: INTEGER,
market_error_type: TEXT,
security_error_count: INTEGER,
last_security_error_type: TEXT,
last_error_at: INTEGER
```

**Error Threshold:**

**Permanent failure threshold (market data):**

Market failures are classified in `updates/helpers.rs`:

- If **all** failures are "not listed" style → `error_type="not_listed"`
- Otherwise → `error_type="temporary"`

Tokens are marked permanently failed for market updates when:

- `error_type == "not_listed"` AND `error_count >= 3` (`PERMANENT_FAILURE_THRESHOLD`)

This prevents wasting budget on mints that will never have market data.

---

## 12. Caching Architecture

**Core caches (by subsystem):**

### Memory Caches

**1. DECIMALS_CACHE (decimals.rs)**
```rust
Type: Moka sync cache
Capacity: 100,000 entries
TTL: Infinite (never expire)
Population: Startup preload + upsert_token() side effect
Usage: Sync pool decoders (critical hot path)
Eviction: LRU when capacity reached
```

**2. TOKEN_2022_CACHE (decimals.rs)**
```rust
Type: Moka sync cache
Capacity: 100,000 entries
TTL: Infinite
Population: Side effect of RPC fetch (SPL vs Token-2022 detection)
Usage: Filtering, trading logic
```

**3. FAILED_CACHE (decimals.rs - negative caching)**
```rust
Type: Moka sync cache
Capacity: 50,000 entries
TTL: 24 hours
Purpose: Track mints where decimals couldn't be resolved
Impact: Avoid repeated expensive RPC calls for invalid/burned tokens
```

**4. AUTHORITIES_CACHE (authority_cache.rs)**
```rust
Type: Moka sync cache
Capacity: 100,000 entries
TTL: Infinite
Population: Side effect of decimals.rs RPC calls (zero extra cost)
Usage: Filtering checks authorities without DB/RPC
```

**5. BLOCKED_AUTHORITIES (authority_cache.rs)**
```rust
Type: ArcSwap<DashSet<String>>
Capacity: ~1000 blocked addresses
TTL: Refreshed every 5 minutes from DB
Purpose: Factory-level blocking (entire authority blocked)
Pattern: Atomic replacement (ArcSwap prevents race conditions)
Usage: O(1) membership check during filtering
```

### API Response Caches

**6. DEXSCREENER_CACHE (store.rs)**
```rust
Type: Moka sync cache
Capacity: 2,000 entries
TTL: 30 seconds
Purpose: Cache market data API responses
Metrics: hits/misses/expirations/evictions
Impact: Reduce API calls by 70%+ for active tokens
```

**7. GECKOTERMINAL_CACHE (store.rs)**
```rust
Type: Moka sync cache
Capacity: 2,000 entries
TTL: 60 seconds (slower updates than DexScreener)
Purpose: Cache market data API responses
Impact: Reduce API calls, GeckoTerminal has lower rate limits
```

**8. RUGCHECK_CACHE (store.rs)**
```rust
Type: Moka sync cache
Capacity: 3,000 entries
TTL: 5 minutes (300 seconds)
Purpose: Cache security data (updates infrequently)
Rationale: Security assessments rarely change, longer cache acceptable
```

**9. TOKEN_POOLS_CACHE (pool_data/cache.rs)**
```rust
Type: Moka sync cache
Capacity: 5,000 entries
TTL (retention): 120 seconds
Freshness: 60 seconds (snapshot considered "fresh")
Stale Fallback: Yes (persisted snapshot + allow_stale path)
Purpose: Cache aggregated pool snapshots
Usage: Pool service, trading decisions
```

**10. TOKEN_SNAPSHOT_CACHE (store.rs)**
```rust
Type: RwLock<HashMap<String, TokenEntry>>
TTL: 30 seconds (manual expiration)
Purpose: Cache fully assembled Token structs
Entry: { token: Token, inserted_at: Instant }
Eviction: Lazy (checked on read)
```

### Cache Patterns

**Negative Caching Pattern:**
```rust
// Problem: Repeated RPC calls for invalid mints waste resources
// Solution: Cache failures for 24 hours

if FAILED_CACHE.contains(&mint) {
    return Err("Decimals previously failed");
}

match fetch_decimals_from_rpc(&mint).await {
    Ok(decimals) => {
        DECIMALS_CACHE.insert(mint, decimals);
        Ok(decimals)
    }
    Err(e) => {
        FAILED_CACHE.insert(mint, ()); // Cache failure
        Err(e)
    }
}
```

**Stale Fallback Pattern:**
```rust
// Problem: API downtime breaks pool snapshot retrieval
// Solution: Allow a stale persisted snapshot when a fresh refresh fails

// tokens::pool_data::cache.rs
get_snapshot_allow_stale(mint)
  ├─ use fresh in-memory cache if available
  ├─ otherwise attempt refresh from APIs
  │   └─ on refresh error: return persisted snapshot if available
  └─ store snapshot back into cache when possible
```

**ArcSwap Pattern (authority_cache.rs):**
```rust
// Problem: Refreshing blocked set while filtering runs = race condition
// Solution: Atomic replacement with ArcSwap

// Read (lockless):
fn is_blocked(address: &str) -> bool {
    BLOCKED_AUTHORITIES.load().contains(address)
}

// Write (atomic swap):
fn refresh_blocked_from_db() {
    let new_set = load_blocked_authorities_from_db();
    BLOCKED_AUTHORITIES.store(Arc::new(new_set)); // Atomic
}

// Old set dropped automatically when no readers remain
```

**Cache Hierarchy:**
```
Memory (instant)
  ├─ DECIMALS_CACHE (infinite TTL)
  ├─ AUTHORITIES_CACHE (infinite TTL)
  └─ BLOCKED_AUTHORITIES (atomic set)
      ↓ miss
Database (fast)
  ├─ tokens table
  ├─ market_* tables
  └─ security_rugcheck
      ↓ miss
API (slow, rate-limited)
  ├─ DexScreener (30s cache)
  ├─ GeckoTerminal (60s cache)
  └─ Rugcheck (5min cache)
```

---

## 13. Event System

**TokenEvent Enum (events.rs):**

```rust
pub enum TokenEvent {
    TokenDiscovered {
        mint: String,
        source: String,
        at: i64,
    },
    TokenUpdated {
        mint: String,
        at: i64,
    },
    DecimalsUpdated {
        mint: String,
        decimals: u8,
        at: i64,
    },
    TokenBlacklisted {
        mint: String,
        reason: String,
        at: i64,
    },
    TokenUnblacklisted {
        mint: String,
        at: i64,
    },
}
```

**Pub/Sub Architecture:**

```rust
static SUBSCRIBERS: Lazy<Vec<Arc<dyn Fn(&TokenEvent) + Send + Sync>>> = ...;

pub fn subscribe(callback: impl Fn(&TokenEvent) + Send + Sync + 'static) {
    SUBSCRIBERS.push(Arc::new(callback));
}

pub fn emit(event: TokenEvent) {
    for subscriber in SUBSCRIBERS.iter() {
        subscriber(&event); // Synchronous broadcast
    }
}
```

**Publishers:**
- Discovery loop: `emit(TokenEvent::TokenDiscovered)`
- Update loops: `emit(TokenEvent::TokenUpdated)`
- Decimals fetch: `emit(TokenEvent::DecimalsUpdated)`
- Cleanup loop: `emit(TokenEvent::TokenBlacklisted)`
- Blacklist API: `emit(TokenEvent::TokenUnblacklisted)`

**Subscribers:**
- **Filtering module:** Track rejection events for authority discovery
- **Dashboard:** Display new token discoveries in real-time
- **System events:** Record to global event log for admin monitoring
- **Metrics:** Track discovery rate, update rate, blacklist rate

**Usage Example:**

```rust
// In filtering module initialization:
tokens::subscribe(|event| {
    match event {
        TokenEvent::TokenDiscovered { mint, source, .. } => {
            log::info!("New token discovered: {} from {}", mint, source);
        }
        TokenEvent::TokenBlacklisted { mint, reason, .. } => {
            log::warn!("Token blacklisted: {} - {}", mint, reason);
            remove_from_filtered_lists(mint);
        }
        _ => {}
    }
});
```

---

## 14. Integration Points

### Filtering Module

**Data Flow: Tokens → Filtering**

```rust
// Input (tokens module → filtering module):
get_all_tokens_for_filtering_async()
  ├─ Query: require_market_data=true
  ├─ Optimization: Exclude 88k tokens without recent market data
  ├─ Result: ~56k tokens with all fields
  └─ Return: Vec<Token> (full 70+ fields)

// Processing (filtering module):
apply_filters(tokens: Vec<Token>)
  ├─ Filter 1: market_cap >= min_market_cap
  ├─ Filter 2: liquidity_usd >= min_liquidity
  ├─ Filter 3: security_score <= max_security_score (remember: lower = safer)
  ├─ Filter 4: !is_blocked_authority(mint_authority)
  ├─ Filter 5: !is_blocked_authority(freeze_authority)
  ├─ Filter 6-30: ... additional filters ...
  └─ Result: FilteredTokenLists { passed, rejected, blacklisted }

// Output (filtering module → tokens module):
store_filtered_results(lists: FilteredTokenLists)
  ├─ Store globally in tokens module
  ├─ Consumed by pool service, trader, dashboard
  └─ Update rejection tracking:
      └─ batch_update_rejection_status_async(rejected_tokens)
```

**Authority Integration:**

```rust
// Filtering checks blocked authorities (O(1)):
if is_blocked_authority(&token.mint_authority) {
    reject_token("mint_authority blocked");
}

// Authority discovery learns from rejections:
run_authority_discovery()
  ├─ Group rejected tokens by authority
  ├─ Compute confidence: flagged / total
  ├─ Block if confidence >= 0.8
  └─ Refresh BLOCKED_AUTHORITIES cache
```

**Rejection Recording:**

```rust
// For each rejected token:
update_rejection_status_async(
    mint: token.mint,
    reason: rejection_reason,
    source: filter_name,
    timestamp: now,
)
  ├─ Store in update_tracking (last_rejection_*)
  ├─ Insert to rejection_history (time-range analytics)
  └─ Aggregate to rejection_stats (hourly buckets)
```

### Pools Module

**Data Flow: Tokens → Pools**

```rust
// Pool service needs token pool data:
let snapshot = pool_data::get_snapshot(&mint).await?;
  ├─ Fetches from cache (60s TTL)
  ├─ Fallback to stale if API down
  └─ Returns TokenPoolsSnapshot

// Pool service calculates pool prices:
calculate_pool_price(&snapshot)
  ├─ Uses canonical_pool_address
  ├─ Updates pool_price_last_calculated_at
  └─ Stores result in pools module
```

### Positions Module

**Data Flow: Positions → Tokens**

```rust
// When position opened:
update_priority(mint, Priority::OpenPosition).await;
  ├─ Sets priority to 100
  ├─ Token moves to critical update loop (5s interval)
  └─ Ensures real-time market data during active trade

// When position closed:
update_priority(mint, Priority::Standard).await;
  ├─ Reduces priority to 25
  └─ Token moves to low priority loop (20s interval)
```

### Config Module

**Configurable Values:**

```rust
tokens.preferred_market_data_source: "dexscreener" | "geckoterminal"
tokens.discovery.enabled: bool
tokens.discovery.dexscreener.enabled: bool
tokens.discovery.geckoterminal.enabled: bool
tokens.discovery.rugcheck.enabled: bool
tokens.discovery.jupiter.enabled: bool
tokens.discovery.coingecko.enabled: bool
tokens.discovery.defillama.enabled: bool
tokens.sources.geckoterminal_rate_limit_per_minute: u32
tokens.sources.rugcheck_rate_limit_per_minute: u32
maintenance.stale_token_days: u32
maintenance.stale_pool_data_days: u32
```

### Webserver Module

**API Endpoints (webserver/routes/tokens.rs):**

```rust
GET  /api/tokens                    → List tokens (paginated)
GET  /api/tokens/{mint}             → Get full token details
POST /api/tokens/{mint}/force-update → Force immediate update
GET  /api/tokens/search?q=...       → Search by name/symbol/mint
GET  /api/tokens/favorites          → List user favorites
POST /api/tokens/{mint}/favorite    → Add to favorites
DELETE /api/tokens/{mint}/favorite  → Remove from favorites
POST /api/tokens/{mint}/blacklist   → Blacklist token
DELETE /api/tokens/{mint}/blacklist → Unblacklist token
GET  /api/tokens/rejections         → Rejection analytics
GET  /api/tokens/stats              → Token system stats
```

---

## 15. Performance Optimizations

### Batch DB Operations (N+1 Prevention)

**Problem:**
```rust
// BAD: N+1 query pattern
for mint in mints {
    let token = get_token(&mint);           // 1 query per token
    let market = get_market_data(&mint);    // 1 query per token
    let security = get_security_data(&mint); // 1 query per token
}
// Total: 3N queries for N tokens (expensive)
```

**Solution:**
```rust
// GOOD: Single query with LEFT JOINs
get_all_tokens_optional_market_async() {
    query = "
        SELECT t.*, 
               m_dex.*, 
               m_gecko.*, 
               s.*
        FROM tokens t
        LEFT JOIN market_dexscreener m_dex ON t.mint = m_dex.mint
        LEFT JOIN market_geckoterminal m_gecko ON t.mint = m_gecko.mint
        LEFT JOIN security_rugcheck s ON t.mint = s.mint
        WHERE t.market_data_last_fetched_at > ?
    ";
    // Result: 144k tokens in single query (vs 432k queries)
}
```

**Impact:**
- Load time: 10+ minutes → 3 seconds
- Memory: 260MB → 50MB
- Database contention: Eliminated

### Async Batch Operations

**Problem:**
```rust
// BAD: 130k tokio::spawn calls
for mint in rejected_tokens {
    tokio::spawn(async move {
        update_rejection_status_async(&mint, reason, source, timestamp).await;
    });
}
// Result: Task scheduler overwhelmed, OOM risk
```

**Solution:**
```rust
// GOOD: Batch async with chunking
batch_update_rejection_status_async(updates: Vec<RejectionUpdate>) {
    // Single spawn_blocking call
    spawn_blocking(move || {
        // Single transaction for all
        db.transaction(|tx| {
            for update in updates {
                tx.execute("UPDATE update_tracking SET ...", params)?;
            }
            Ok(())
        })
    }).await
}
// Result: 260k tasks → 4 tasks
```

**Impact:**
- Task count: 260,000 → 4
- Memory usage: 80% reduction
- Update time: 45s → 2s

### Priority Scheduling

**Problem:**
```rust
// BAD: Update all tokens equally
for token in all_tokens {
    update_token(token);  // Same interval for all
}
// Result: Waste API calls on stale/unimportant tokens
```

**Solution:**
```rust
// GOOD: Priority-based scheduling
Priority::OpenPosition (100)  → 5s interval  (active trades)
Priority::FilterPassed (60)   → 8s interval  (good tokens)
Priority::Background (10)     → 30s interval (oldest tokens)

// Result: Focus API budget on important tokens
```

**Impact:**
- API usage: 60% reduction
- Data freshness: 3x improvement for active tokens
- Cost: 50% reduction in API costs

### Cache Hierarchy

**Problem:**
```rust
// BAD: Always hit DB/API
fn get_decimals(mint: &str) -> u8 {
    // Every call = DB query or RPC call
    fetch_from_db_or_rpc(mint)
}
```

**Solution:**
```rust
// GOOD: Memory → DB → API hierarchy
fn get_decimals(mint: &str) -> u8 {
    // 1. Memory cache (instant)
    if let Some(decimals) = DECIMALS_CACHE.get(mint) {
        return decimals;
    }
    
    // 2. DB (fast)
    if let Some(decimals) = db::get_token_decimals(mint) {
        DECIMALS_CACHE.insert(mint, decimals);
        return decimals;
    }
    
    // 3. RPC (slow)
    let decimals = fetch_from_rpc(mint).await?;
    DECIMALS_CACHE.insert(mint, decimals);
    db::upsert_token(mint, decimals);
    decimals
}
```

**Impact:**
- Cache hit rate: 95%+
- RPC calls: 95% reduction
- Latency: 100ms → 0.1ms

### Negative Caching

**Problem:**
```rust
// BAD: Repeated RPC calls for invalid mints
for _ in 0..1000 {
    match get_decimals("invalid_mint") {
        Err(_) => continue,  // Retry every time
    }
}
// Result: 1000 expensive RPC calls for known-invalid mint
```

**Solution:**
```rust
// GOOD: Cache failures
if FAILED_CACHE.contains("invalid_mint") {
    return Err("Previously failed");
}

match fetch_from_rpc("invalid_mint").await {
    Err(e) => {
        FAILED_CACHE.insert("invalid_mint", ()); // Cache for 24h
        Err(e)
    }
}
// Result: 1 RPC call, 999 cache hits
```

**Impact:**
- RPC calls for invalid mints: 99% reduction
- Faster rejection of invalid tokens

---

## 16. Key Patterns & Pitfalls

### Critical Patterns

**1. ArcSwap for Atomic Set Replacement**
```rust
// Pattern: Atomic replacement of shared data structure
static BLOCKED_AUTHORITIES: Lazy<ArcSwap<DashSet<String>>> = ...;

// Write: Atomic swap (no race condition)
fn refresh() {
    let new_set = load_from_db();
    BLOCKED_AUTHORITIES.store(Arc::new(new_set));
    // Old set dropped when no readers remain
}

// Read: Lockless (multiple readers)
fn is_blocked(address: &str) -> bool {
    BLOCKED_AUTHORITIES.load().contains(address)
}
```

**2. spawn_blocking for All DB Access**
```rust
// Pattern: Never block async executor with blocking DB calls
async fn get_token_async(mint: &str) -> Result<Token, String> {
    let mint = mint.to_string();
    tokio::task::spawn_blocking(move || {
        get_token(&mint)  // Blocking rusqlite call
    }).await
    .map_err(|e| format!("Task join error: {}", e))?
}

// PITFALL: Never do this
async fn bad_get_token(mint: &str) -> Result<Token, String> {
    get_token(mint)  // BLOCKS async executor!
}
```

**3. Authority Cache as Side Effect**
```rust
// Pattern: Zero-cost enrichment from required data
fn fetch_decimals_from_rpc(mint: &str) -> Result<u8, String> {
    let account = rpc.get_account_info(mint)?;
    
    // Primary goal: Extract decimals
    let decimals = parse_spl_mint(&account.data)?.decimals;
    DECIMALS_CACHE.insert(mint, decimals);
    
    // Side effect: Cache authorities (zero extra cost)
    let authorities = MintAuthorities {
        mint_authority: parse_spl_mint(&account.data)?.mint_authority,
        freeze_authority: parse_spl_mint(&account.data)?.freeze_authority,
        supply: parse_spl_mint(&account.data)?.supply,
    };
    AUTHORITIES_CACHE.insert(mint, authorities);
    
    Ok(decimals)
}
```

**4. Timestamp Naming Convention**
```rust
// Pattern: {what}_{when}_{action}_at
first_discovered_at              // Immutable, set once
metadata_last_fetched_at         // Updated every metadata fetch
market_data_last_fetched_at      // Per market data fetch
security_data_last_fetched_at    // Per security fetch
pool_price_last_calculated_at    // Pool service specific
last_rejection_at                // Filtering result
blockchain_created_at            // From on-chain data
```

**5. Permanent failure guard (market data)**

- Implemented in `updates/helpers.rs::handle_market_failure()`
- Only `"not_listed"` market failures contribute to permanent status
- Threshold: `PERMANENT_FAILURE_THRESHOLD = 3`

### Common Pitfalls

**1. Confusing Security Score Direction**
```rust
// PITFALL: SecurityLevel naming is inverted
SecurityLevel::Dangerous → score 0-19  (GOOD for filtering, low risk)
SecurityLevel::Safe      → score 80-100 (BAD for filtering, high risk)

// Why: Rugcheck score is 0-100 where HIGHER = MORE RISKY
// But UI displays "Safe" for low-risk tokens
// So level names are inverted from score direction

// CORRECT filtering:
if token.security_score > 40.0 {  // High score = risky
    reject("High risk");
}
```

**2. Forgetting spawn_blocking**
```rust
// PITFALL: Blocking async executor
async fn bad() {
    let token = get_token(&mint);  // Blocks executor!
}

// CORRECT:
async fn good() {
    let token = tokio::task::spawn_blocking(move || {
        get_token(&mint)
    }).await?;
}
```

**3. Not Using Global Database Reference**
```rust
// PITFALL: Creating new DB connection
fn update_token(mint: &str) {
    let db = TokenDatabase::new("data/tokens.db")?;  // Wrong!
    db.update(mint);
}

// CORRECT:
fn update_token(mint: &str) {
    with_token_database(|db| {
        db.update(mint)
    })
}
```

**4. Assuming Market Data Always Present**
```rust
// PITFALL: Unwrapping optional market data
let price = token.price_usd.unwrap();  // Panics if None!

// CORRECT:
let price = token.price_usd.unwrap_or(0.0);
// Or better: Handle None explicitly
match token.price_usd {
    Some(price) => use_price(price),
    None => log::warn!("No price for {}", token.mint),
}
```

**5. Ignoring Rate Limits**
```rust
// PITFALL: Calling API without acquiring permit
async fn bad_update(mint: &str) {
    let data = fetch_dexscreener_data(mint).await?;  // May exceed rate limit
}

// CORRECT:
async fn good_update(mint: &str, coordinator: &RateLimitCoordinator) {
    coordinator.acquire_dexscreener_batch().await?;
    let data = fetch_dexscreener_data(mint).await?;
}
```

**6. Not Handling Stale Cache Fallback**
```rust
// PITFALL: Failing hard on API error
let pools = fetch_fresh_pools(&mint).await?;  // Fails if API down

// CORRECT:
let pools = get_snapshot_allow_stale(&mint).await?;
// Returns stale cache if fresh fetch fails (better than nothing)
```

**7. Querying All Tokens Without Filtering**
```rust
// PITFALL: Loading all 281k tokens with market data
let tokens = get_all_tokens_optional_market_async(None).await?;
// Result: OOM, slow query, unnecessary data

// CORRECT:
let tokens = get_all_tokens_for_filtering_async().await?;
// Excludes 88k tokens without recent market data (56k returned)
```

---

## Architecture Summary

**Tokens Module = Data Foundation**

- **Discovery:** 8+ sources → 281K+ tokens tracked
- **Enrichment:** Market data (DexScreener + optional GeckoTerminal) + Rugcheck security + pool snapshot aggregation
- **Scheduling:** State-based update loops (5s-30s market loops + separate security loop)
- **Caching:** Multiple bounded caches (memory + DB + API) with negative caching and stale fallback
- **Integration:** Powers filtering, pool tracking, trading decisions
- **Performance:** Batch operations, negative caching, stale fallback, N+1 prevention
- **Resilience:** Error tracking, rate limiting, fallback strategies
- **Intelligence:** Authority reputation system auto-discovers scam factories

**Key Metrics:**
- 281,000+ tokens tracked
- 11 database tables, 30+ indexes
- 40+ source files
- 8+ discovery sources
- 7 update loops (state-based)
- 11 background tasks (current)
- Multiple bounded caches (decimals/market/security/pools/authorities + snapshot stores)
- 50+ async API wrappers

---

**End of Architecture Document**
