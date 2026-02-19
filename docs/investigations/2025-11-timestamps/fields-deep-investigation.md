# TIMESTAMP FIELDS DEEP INVESTIGATION - October 31, 2025

## Executive Summary

This investigation reveals **significant confusion and inconsistency** in timestamp field naming and usage across the entire token system. The current naming is vague, timestamps are being used interchangeably, and there's no clear separation between different update types (pool price updates vs. market data updates vs. metadata updates).

**Critical Issues Identified:**

1. Generic `updated_at` and `fetched_at` names without context
2. No distinction between pool service price calculations and API market data fetches
3. Conflation of multiple timestamp concepts into single fields
4. Inconsistent timestamp sources in database queries and joins
5. Dashboard displays incorrect timestamp types for different views

---

## Current Timestamp Fields Analysis

### 1. **tokens table (Core Metadata)**

#### Current Schema:

```sql
CREATE TABLE tokens (
    mint TEXT PRIMARY KEY,
    symbol TEXT,
    name TEXT,
    decimals INTEGER,
    created_at INTEGER NOT NULL,    -- ❌ CONFUSING: When was this? Bot discovery? Chain creation?
    updated_at INTEGER NOT NULL     -- ❌ CONFUSING: What was updated? Everything?
)
```

#### Problems:

- **`created_at`**: Unclear if this is:
  - When bot first discovered the token
  - When token was created on blockchain
  - When record was inserted into database
- **`updated_at`**: Unclear what triggered the update:
  - Metadata change (symbol/name/decimals)?
  - Any market data fetch?
  - Manual refresh?

#### Current Usage:

```rust
// database.rs:355 - INSERT
"INSERT INTO tokens (mint, symbol, name, decimals, created_at, updated_at)
 VALUES (?1, ?2, ?3, ?4, ?5, ?5)
 ON CONFLICT(mint) DO UPDATE SET
     symbol = excluded.symbol,
     name = excluded.name,
     decimals = excluded.decimals,
     updated_at = ?5"
```

**The same timestamp is used for both `created_at` AND `updated_at` on insert!** This makes `created_at` meaningless as it gets overwritten.

```rust
// database.rs:1339 - Generic update
"UPDATE tokens SET updated_at = ?1 WHERE mint = ?2"
```

Called from `mark_updated()` - but what exactly was updated? No context.

---

### 2. **market_dexscreener table**

#### Current Schema:

```sql
CREATE TABLE market_dexscreener (
    mint TEXT PRIMARY KEY,
    price_usd REAL,
    price_sol REAL,
    -- ... 20+ market data fields ...
    pair_created_at INTEGER,        -- ✅ CLEAR: When pair was created on-chain
    fetched_at INTEGER NOT NULL,    -- ❌ CONFUSING: API fetch or last update?
    FOREIGN KEY (mint) REFERENCES tokens(mint)
)
```

#### Problems:

- **`fetched_at`**: Is this when data was fetched from DexScreener API? Or any update?
- No distinction between "first fetch" vs "latest refresh"
- `pair_created_at` is good but only available from DexScreener

#### Current Usage:

```rust
// database.rs:526 - INSERT market data
"INSERT INTO market_dexscreener (..., fetched_at)
 VALUES (..., ?31)
 ON CONFLICT(mint) DO UPDATE SET
     price_usd = excluded.price_usd,
     -- ... all fields ...
     fetched_at = ?31"
```

`fetched_at` is updated every time market data is refreshed. But this loses historical context.

---

### 3. **market_geckoterminal table**

#### Current Schema:

```sql
CREATE TABLE market_geckoterminal (
    mint TEXT PRIMARY KEY,
    price_usd REAL,
    price_sol REAL,
    -- ... market data fields ...
    fetched_at INTEGER NOT NULL,    -- ❌ CONFUSING: Same problem as dexscreener
    FOREIGN KEY (mint) REFERENCES tokens(mint)
)
```

#### Problems:

- Same `fetched_at` ambiguity
- No `pair_created_at` equivalent
- No way to distinguish GeckoTerminal fetch time from DexScreener fetch time

---

### 4. **security_rugcheck table**

#### Current Schema:

```sql
CREATE TABLE security_rugcheck (
    mint TEXT PRIMARY KEY,
    score INTEGER,
    -- ... security fields ...
    fetched_at INTEGER NOT NULL,    -- ❌ CONFUSING: Rugcheck API fetch time
    FOREIGN KEY (mint) REFERENCES tokens(mint)
)
```

#### Problems:

- Generic `fetched_at` without source context
- Security updates are infrequent but critical - need clear timestamp

---

### 5. **token_pools table**

#### Current Schema:

```sql
CREATE TABLE token_pools (
    mint TEXT NOT NULL,
    pool_address TEXT NOT NULL,
    -- ... pool data ...
    fetched_at INTEGER NOT NULL,    -- ❌ CONFUSING: From which source?
    PRIMARY KEY (mint, pool_address)
)
```

#### Problems:

- Multiple sources can report same pool
- `fetched_at` doesn't indicate which source last updated
- No distinction between pool metadata fetch vs pool price calculation

---

### 6. **update_tracking table**

#### Current Schema:

```sql
CREATE TABLE update_tracking (
    mint TEXT PRIMARY KEY,
    priority INTEGER NOT NULL DEFAULT 10,
    last_market_update INTEGER,         -- ⚠️ SEMI-CLEAR: Market API update
    last_security_update INTEGER,       -- ⚠️ SEMI-CLEAR: Rugcheck update
    last_decimals_update INTEGER,       -- ⚠️ SEMI-CLEAR: Decimals fetch
    market_update_count INTEGER DEFAULT 0,
    security_update_count INTEGER DEFAULT 0,
    -- ... error tracking ...
)
```

#### Better than others, but still issues:

- **`last_market_update`**: Is this DexScreener, GeckoTerminal, or both?
- **`last_decimals_update`**: Also used as proxy for "metadata updated" - wrong!
- No tracking for:
  - Pool service price calculations
  - OHLCV data updates
  - Token discovery time

---

### 7. **Token Type (Runtime)**

#### Current Fields:

```rust
pub struct Token {
    // ... identity fields ...

    pub data_source: DataSource,            // ✅ CLEAR: Which API for market data
    pub fetched_at: DateTime<Utc>,         // ❌ CONFUSING: What was fetched?
    pub updated_at: DateTime<Utc>,         // ❌ CONFUSING: What was updated?
    pub created_at: DateTime<Utc>,         // ❌ CONFUSING: Created where?
    pub metadata_updated_at: Option<DateTime<Utc>>,  // ⚠️ SEMI-CLEAR but not used correctly
    pub token_birth_at: Option<DateTime<Utc>>,       // ⚠️ SEMI-CLEAR: On-chain creation

    // ... market/security fields ...

    pub first_seen_at: DateTime<Utc>,      // ⚠️ SEMI-CLEAR: Bot discovery
    pub last_price_update: DateTime<Utc>,  // ❌ CONFUSING: Pool or market API?
}
```

#### Critical Confusion:

```rust
// database.rs:2361-2362 - Assignment during Token construction
first_seen_at: created_dt,           // ✅ OK: Maps created_at to first_seen
last_price_update: fetched_at_dt,    // ❌ WRONG: Uses market API fetch, not pool price!
```

**`last_price_update` should be pool service calculation time, NOT market API fetch time!**

---

## Database Query Confusion

### Query 1: Token Sorting (database.rs:204-210)

```rust
let sort_column_sql = match query.sort_by.as_deref() {
    Some("updated_at") => "COALESCE(ut.last_market_update, t.created_at)",  // Uses market update
    Some("first_seen_at") => "t.created_at",                                 // Uses created_at
    Some("metadata_updated_at") => "COALESCE(ut.last_decimals_update, t.updated_at)",  // Wrong!
    Some("token_birth_at") => "COALESCE(ut.last_decimals_update, t.created_at)",       // Wrong!
    _ => "COALESCE(ut.last_market_update, t.created_at)",
};
```

**Problems:**

1. `updated_at` sort uses `last_market_update` - but dashboard shows different data
2. `metadata_updated_at` fallback to `t.updated_at` is wrong (that's not metadata specific)
3. `token_birth_at` fallback to `last_decimals_update` makes no sense!

### Query 2: Full Token Query (database.rs:1769-1773)

```rust
let sort_column_sql = match query.sort_by.as_deref() {
    Some("updated_at") =>
        "COALESCE(ut.last_market_update, COALESCE(d.fetched_at, g.fetched_at, t.updated_at))",
    Some("first_seen_at") => "t.created_at",
    Some("metadata_updated_at") => "COALESCE(ut.last_decimals_update, t.updated_at)",
    Some("token_birth_at") => "COALESCE(d.pair_created_at, t.created_at)",
    _ => "t.updated_at",
};
```

**Different fallback chain than Query 1!** No consistency.

---

## Pool Service vs. Token System Confusion

### Pool Service (src/pools/)

**Pool price calculation uses `PriceResult.timestamp` (Instant):**

```rust
pub struct PriceResult {
    pub price_sol: f64,
    pub timestamp: Instant,    // When price was CALCULATED from on-chain data
    pub pool_address: String,
    // ...
}
```

**Pool database stores price history:**

```sql
CREATE TABLE price_history (
    -- ...
    created_at TEXT NOT NULL DEFAULT (datetime('now')),  -- When record was inserted
)
```

**Problem:** No field for "when price was calculated" - only "when record was saved to DB"!

### Token System (src/tokens/)

**Token struct has:**

```rust
pub last_price_update: DateTime<Utc>,  // Currently set to market API fetch time!
```

**This is WRONG! Should be pool service calculation time, not API fetch time.**

### Current Assignment:

```rust
// database.rs:2362
last_price_update: fetched_at_dt,  // Uses DexScreener/GeckoTerminal fetched_at ❌
```

**Should be:**

```rust
last_price_update: pool_price_calculated_at,  // Pool service calculation time ✅
```

---

## Dashboard Display Issues

### Tokens Route (webserver/routes/tokens.rs)

```rust
pub struct TokenDetailResponse {
    pub created_at: Option<i64>,           // Currently: t.created_at (bot discovery?)
    pub last_updated: Option<i64>,         // Currently: ???
    pub pair_created_at: Option<i64>,      // Currently: d.pair_created_at ✅
    pub price_updated_at: Option<i64>,     // Currently: ??? (should be pool calculation)
    // ...
}
```

**Assignment:**

```rust
// database.rs:988-989
let created_at_ts = Some(token.first_seen_at.timestamp());
let token_birth_ts = token.token_birth_at.map(|dt| dt.timestamp());
```

Maps `first_seen_at` → `created_at` in response, but field name doesn't indicate it's bot discovery!

### Dashboard Expectations (from user request):

> "in dashboard tokens tab sub tab pool service for updated time column we must use pool service price calculated time and for other sub tabs we need updated be market data updated time"

**Current Implementation:**

- All views use same timestamps
- No distinction between pool price calculation time vs market API fetch time
- Dashboard cannot filter/sort correctly by data source

---

## Update Logic Confusion

### Where Timestamps Are Set

#### 1. **tokens.updated_at**

```rust
// database.rs:1339 - Generic update (called from mark_updated)
"UPDATE tokens SET updated_at = ?1 WHERE mint = ?2"
```

**Trigger:** After any market/security update (too broad!)

#### 2. **market\_\*.fetched_at**

```rust
// database.rs:550 - DexScreener
data.fetched_at.timestamp()

// database.rs:668 - GeckoTerminal
data.fetched_at.timestamp()
```

**Trigger:** When API returns data

#### 3. **update_tracking.last_market_update**

```rust
// database.rs:1317 - After market update
"UPDATE update_tracking SET
     last_market_update = ?1,
     market_update_count = market_update_count + 1
 WHERE mint = ?2"
```

**Trigger:** After successful market data fetch

#### 4. **update_tracking.last_security_update**

Similar pattern for Rugcheck updates.

#### 5. **update_tracking.last_decimals_update**

```rust
// Used as proxy for "metadata updated" - WRONG!
```

Decimals update ≠ metadata update (symbol/name can change independently)

---

## Token Discovery Flow (Current)

```
1. Token discovered (discovery.rs)
   └─> Inserts into tokens table
       └─> created_at = now()
       └─> updated_at = now()  ❌ Same value!

2. Market data fetch (dexscreener.rs / geckoterminal.rs)
   └─> Updates market_* table
       └─> fetched_at = now()
   └─> Updates tokens.updated_at = now()  ❌ Overwrites created_at meaning!
   └─> Updates update_tracking.last_market_update = now()

3. Security data fetch (rugcheck.rs)
   └─> Updates security_rugcheck table
       └─> fetched_at = now()
   └─> Updates tokens.updated_at = now()  ❌ Again!
   └─> Updates update_tracking.last_security_update = now()

4. Pool price calculation (pools/calculator.rs)
   └─> Returns PriceResult with timestamp (Instant)
   └─> Cached in pools/cache.rs
   └─> ❌ NOT tracked in tokens system at all!

5. Token struct construction (database.rs:2361-2362)
   └─> first_seen_at = tokens.created_at  ✅
   └─> last_price_update = market_*.fetched_at  ❌ Should be pool calculation time!
```

---

## Proposed Timestamp Taxonomy

### Core Principles:

1. **Specificity**: Each timestamp must clearly indicate WHAT was updated/created
2. **Source**: Distinguish between on-chain, API, and calculated data
3. **Granularity**: Separate timestamps for different update types
4. **Consistency**: Same naming patterns across all tables

### Proposed Field Names:

#### **tokens table (Core Metadata)**

```sql
CREATE TABLE tokens (
    mint TEXT PRIMARY KEY,
    symbol TEXT,
    name TEXT,
    decimals INTEGER,

    -- Discovery & Creation
    first_discovered_at INTEGER NOT NULL,       -- When bot first saw this token (immutable)
    blockchain_created_at INTEGER,              -- When token was created on-chain (if known)

    -- Metadata Updates
    metadata_last_fetched_at INTEGER NOT NULL,  -- When symbol/name/decimals last fetched
    decimals_last_fetched_at INTEGER NOT NULL   -- When decimals specifically fetched
)
```

#### **market_dexscreener table**

```sql
CREATE TABLE market_dexscreener (
    mint TEXT PRIMARY KEY,
    -- ... market fields ...
    pair_blockchain_created_at INTEGER,             -- On-chain pair creation time
    market_data_last_fetched_at INTEGER NOT NULL,   -- DexScreener API fetch time
    market_data_first_fetched_at INTEGER NOT NULL   -- First time we got data from DexScreener
)
```

#### **market_geckoterminal table**

```sql
CREATE TABLE market_geckoterminal (
    mint TEXT PRIMARY KEY,
    -- ... market fields ...
    market_data_last_fetched_at INTEGER NOT NULL,   -- GeckoTerminal API fetch time
    market_data_first_fetched_at INTEGER NOT NULL   -- First time we got data from GeckoTerminal
)
```

#### **security_rugcheck table**

```sql
CREATE TABLE security_rugcheck (
    mint TEXT PRIMARY KEY,
    -- ... security fields ...
    security_data_last_fetched_at INTEGER NOT NULL,   -- Rugcheck API fetch time
    security_data_first_fetched_at INTEGER NOT NULL   -- First time we got security data
)
```

#### **token_pools table**

```sql
CREATE TABLE token_pools (
    mint TEXT NOT NULL,
    pool_address TEXT NOT NULL,
    -- ... pool fields ...
    pool_data_last_fetched_at INTEGER NOT NULL,      -- Last API fetch for this pool
    pool_data_first_seen_at INTEGER NOT NULL,        -- When we first discovered this pool
    PRIMARY KEY (mint, pool_address)
)
```

#### **update_tracking table**

```sql
CREATE TABLE update_tracking (
    mint TEXT PRIMARY KEY,
    priority INTEGER NOT NULL DEFAULT 10,

    -- Market Data Updates (API fetches)
    market_data_last_updated_at INTEGER,      -- Last successful market API update (any source)
    market_data_update_count INTEGER DEFAULT 0,

    -- Security Data Updates (Rugcheck API)
    security_data_last_updated_at INTEGER,    -- Last successful Rugcheck update
    security_data_update_count INTEGER DEFAULT 0,

    -- Metadata Updates (on-chain fetches)
    metadata_last_updated_at INTEGER,         -- Last metadata fetch (symbol/name/decimals)
    decimals_last_updated_at INTEGER,         -- Last decimals-specific fetch

    -- Pool Price Updates (Pool Service calculations)
    pool_price_last_calculated_at INTEGER,    -- Last pool price calculation time
    pool_price_last_used_pool_address TEXT,   -- Which pool was used for calculation

    -- Error Tracking
    last_error TEXT,
    last_error_at INTEGER,
    market_error_count INTEGER DEFAULT 0,
    security_error_count INTEGER DEFAULT 0
)
```

#### **Token Runtime Type**

```rust
pub struct Token {
    // Identity
    pub mint: String,
    pub symbol: String,
    pub name: String,
    pub decimals: u8,

    // Data Source
    pub data_source: DataSource,  // Which API for market data

    // Discovery & Creation Timestamps
    pub first_discovered_at: DateTime<Utc>,        // When bot first saw token (immutable)
    pub blockchain_created_at: Option<DateTime<Utc>>,  // On-chain creation (if known)

    // Metadata Timestamps
    pub metadata_last_fetched_at: DateTime<Utc>,   // Symbol/name/decimals fetch
    pub decimals_last_fetched_at: DateTime<Utc>,   // Decimals-specific fetch

    // Market Data Timestamps
    pub market_data_last_fetched_at: DateTime<Utc>,  // API fetch (from data_source)
    pub market_data_source_fetched_at: Option<DateTime<Utc>>,  // Alternative source fetch

    // Security Data Timestamps
    pub security_data_last_fetched_at: Option<DateTime<Utc>>,  // Rugcheck fetch

    // Pool Price Timestamps
    pub pool_price_last_calculated_at: DateTime<Utc>,  // Pool service calculation
    pub pool_price_last_used_pool: Option<String>,     // Which pool calculated price

    // ... market/security fields ...
}
```

---

## Dashboard Display Mapping

### Tokens Tab Views:

#### **1. Pool Service View**

- **"Updated" column** → `pool_price_last_calculated_at`
- **"First Seen" column** → `first_discovered_at`
- **Sort by "updated"** → `update_tracking.pool_price_last_calculated_at`

#### **2. Market Data View (DexScreener)**

- **"Updated" column** → `market_dexscreener.market_data_last_fetched_at`
- **"First Seen" column** → `market_dexscreener.market_data_first_fetched_at`
- **"Pair Created" column** → `market_dexscreener.pair_blockchain_created_at`
- **Sort by "updated"** → `market_dexscreener.market_data_last_fetched_at`

#### **3. Market Data View (GeckoTerminal)**

- **"Updated" column** → `market_geckoterminal.market_data_last_fetched_at`
- **"First Seen" column** → `market_geckoterminal.market_data_first_fetched_at`
- **Sort by "updated"** → `market_geckoterminal.market_data_last_fetched_at`

#### **4. Security View (Rugcheck)**

- **"Updated" column** → `security_rugcheck.security_data_last_fetched_at`
- **"First Fetched" column** → `security_rugcheck.security_data_first_fetched_at`
- **Sort by "updated"** → `security_rugcheck.security_data_last_fetched_at`

#### **5. All Tokens View**

- **"First Seen" column** → `tokens.first_discovered_at`
- **"Blockchain Created" column** → `tokens.blockchain_created_at`
- **"Last Updated" column** → MAX of all data source update times
- **Sort by "updated"** → Latest of any update type

---

## Implementation Impact Analysis

### Files Requiring Changes:

#### **Schema & Database (HIGH IMPACT)**

1. `src/tokens/schema.rs` - Complete schema rewrite
2. `src/tokens/database.rs` - All INSERT/UPDATE/SELECT queries
3. `src/pools/db.rs` - Add pool price calculation timestamp

#### **Types (HIGH IMPACT)**

4. `src/tokens/types.rs` - Token struct fields
5. `src/tokens/types.rs` - TokenMetadata struct
6. `src/tokens/types.rs` - UpdateTrackingInfo struct
7. `src/pools/types.rs` - PriceResult storage

#### **Update Logic (HIGH IMPACT)**

8. `src/tokens/updates.rs` - mark_updated() → separate update functions
9. `src/tokens/market/dexscreener.rs` - Update timestamp handling
10. `src/tokens/market/geckoterminal.rs` - Update timestamp handling
11. `src/tokens/security/rugcheck.rs` - Update timestamp handling
12. `src/tokens/decimals.rs` - Decimals fetch timestamp
13. `src/pools/service.rs` - Track pool price calculation time

#### **Query & Display (MEDIUM IMPACT)**

14. `src/webserver/routes/tokens.rs` - All query/display logic
15. `src/webserver/routes/filtering_api.rs` - Filtering queries
16. `src/filtering/sources/*.rs` - Filter timestamp usage
17. `src/tokens/filtered.rs` - Snapshot timestamp handling

#### **Discovery & Initialization (MEDIUM IMPACT)**

18. `src/tokens/discovery.rs` - Set first_discovered_at (immutable)
19. `src/tokens/database.rs` - ensure_token_exists() logic

#### **Frontend (LOW IMPACT but critical for UX)**

20. `templates/pages/tokens.html` - Column headers
21. `scripts/pages/tokens.js` - Display formatting
22. `scripts/core/utils.js` - Timestamp formatting helpers

---

## Migration Strategy (No Migration!)

Per user requirements: **"we dont need migrations, we are remaking database"**

### Approach:

1. Drop and recreate all tables with new schema
2. Re-discover tokens (uses existing discovery logic)
3. Re-fetch all data (market, security, decimals)
4. Let pool service recalculate prices naturally

### Data Loss Acceptable:

- Historical `updated_at` values (meaningless anyway)
- Old `fetched_at` timestamps (will be refreshed)
- Update counts (will restart from 0)

### Data Preservation:

- Token mints (core identifiers)
- Blacklist (separate table, unchanged)
- Position tracking (separate system, uses mints only)

---

## Priority Update Mapping (Current → New)

### Current Priority Logic:

```rust
Priority::OpenPosition => 100,    // Update every 5s
Priority::PoolTracked => 75,      // Update every 7s
Priority::FilterPassed => 60,     // Update every 8s
Priority::Uninitialized => 55,    // Update every 10s
Priority::Stale => 40,            // Update every 15s
Priority::Standard => 25,         // Update every 20s
Priority::Background => 10,       // Update every 30s
```

### New Logic with Specific Timestamps:

#### **Market Data Updates** (uses `market_data_last_updated_at`)

- OpenPosition: Every 5s
- PoolTracked: Every 7s
- FilterPassed: Every 8s
- Uninitialized (no market data yet): Immediate (priority queue)
- Stale (>1h since last update): Every 15s
- Standard: Every 20s
- Background: Every 30s

#### **Security Data Updates** (uses `security_data_last_updated_at`)

- One token per 60s (configurable via `tokens.sources.rugcheck.update_interval`)
- Priority: Tokens without security data > Tokens with stale data > Regular refresh

#### **Pool Price Calculations** (uses `pool_price_last_calculated_at`)

- Handled entirely by Pool Service (separate priority system)
- Update tracking table records calculation times for filtering/sorting

#### **Metadata Updates** (uses `metadata_last_updated_at`)

- On-demand when symbol/name/decimals missing
- Lazy refresh (very infrequent, symbols rarely change)

---

## Recommended Timestamp Field Naming Patterns

### Pattern: `{what}_{when}_{action}_at`

- **{what}**: Specific data type (market_data, security_data, metadata, pool_price, etc.)
- **{when}**: last / first / blockchain
- **{action}**: fetched / calculated / updated / created / discovered
- **\_at**: Suffix for all timestamps (consistent)

### Examples:

✅ `market_data_last_fetched_at` - Clear: Market data, last fetch time
✅ `pool_price_last_calculated_at` - Clear: Pool price, last calculation
✅ `security_data_first_fetched_at` - Clear: Security data, first fetch
✅ `blockchain_created_at` - Clear: Created on blockchain
✅ `first_discovered_at` - Clear: First seen by bot

❌ `updated_at` - Vague: What was updated?
❌ `fetched_at` - Vague: Fetched from where? What data?
❌ `created_at` - Vague: Created where? Bot or blockchain?

---

## Query Performance Considerations

### Current Indexes:

```sql
CREATE INDEX idx_tokens_updated ON tokens(updated_at DESC);
CREATE INDEX idx_market_dex_fetched ON market_dexscreener(fetched_at DESC);
CREATE INDEX idx_market_gecko_fetched ON market_geckoterminal(fetched_at DESC);
CREATE INDEX idx_security_rug_fetched ON security_rugcheck(fetched_at DESC);
CREATE INDEX idx_tracking_market_update ON update_tracking(last_market_update ASC);
```

### New Indexes Needed:

```sql
-- Discovery & core metadata
CREATE INDEX idx_tokens_discovered ON tokens(first_discovered_at DESC);
CREATE INDEX idx_tokens_blockchain_created ON tokens(blockchain_created_at DESC);
CREATE INDEX idx_tokens_metadata_fetched ON tokens(metadata_last_fetched_at DESC);

-- Market data (per source)
CREATE INDEX idx_market_dex_last_fetch ON market_dexscreener(market_data_last_fetched_at DESC);
CREATE INDEX idx_market_gecko_last_fetch ON market_geckoterminal(market_data_last_fetched_at DESC);

-- Security data
CREATE INDEX idx_security_rug_last_fetch ON security_rugcheck(security_data_last_fetched_at DESC);

-- Update tracking (for priority queries)
CREATE INDEX idx_tracking_market_update ON update_tracking(market_data_last_updated_at ASC);
CREATE INDEX idx_tracking_security_update ON update_tracking(security_data_last_updated_at ASC);
CREATE INDEX idx_tracking_pool_calc ON update_tracking(pool_price_last_calculated_at DESC);
CREATE INDEX idx_tracking_priority_market ON update_tracking(priority DESC, market_data_last_updated_at ASC);

-- Composite indexes for sorting
CREATE INDEX idx_tokens_discovery_mint ON tokens(first_discovered_at DESC, mint);
CREATE INDEX idx_tracking_priority_calc ON update_tracking(priority DESC, pool_price_last_calculated_at DESC);
```

---

## Next Steps (For User Review)

This investigation reveals the scope of the timestamp naming problem. Before proceeding with code changes, please confirm:

1. ✅ **Proposed naming taxonomy** (specific, source-based, action-based)
2. ✅ **Separation of concerns** (pool price ≠ market data ≠ security data ≠ metadata)
3. ✅ **Dashboard display mapping** (different views show different timestamps)
4. ✅ **No migration strategy** (clean slate database rebuild)
5. ✅ **Index strategy** (separate indexes per timestamp type)

Once approved, systematic changes will be implemented across all affected files with **zero compatibility layers** (clean removal of old fields, no `_v2` or legacy support).

---

## Questions for User

1. **Pool Price Calculation Time**: Should we add a dedicated field to `price_history` table for calculation time separate from insertion time?

2. **First Discovery Immutability**: Should `first_discovered_at` be truly immutable (never updated even if token is re-added after blacklist removal)?

3. **Blockchain Creation Time**: How should we handle tokens where we can't determine on-chain creation time (not available from APIs)?

4. **Alternative Source Timestamps**: If DexScreener AND GeckoTerminal both have data, how should we represent both fetch times in Token struct?

5. **Dashboard Default View**: Which timestamp should be the default "Last Updated" in the all-tokens view?

6. **Sorting Ambiguity**: When sorting by "updated" in all-tokens view, should we use MAX of all update times or specific preference order?

---

## Appendix: Complete Current Usage Map

### All `updated_at` Occurrences:

1. `tokens.updated_at` (DB field) - Generic update time
2. `Token.updated_at` (Rust type) - Maps to DB field
3. `update_tracking.last_market_update` - Market data specific
4. `update_tracking.last_security_update` - Security data specific
5. `update_tracking.last_decimals_update` - Decimals specific (misused as metadata proxy)
6. Sorting queries: Used in 6+ different query contexts
7. Dashboard: Displayed in 3+ different views

### All `fetched_at` Occurrences:

1. `market_dexscreener.fetched_at` (DB field)
2. `market_geckoterminal.fetched_at` (DB field)
3. `security_rugcheck.fetched_at` (DB field)
4. `token_pools.fetched_at` (DB field)
5. `DexScreenerData.fetched_at` (Rust type)
6. `GeckoTerminalData.fetched_at` (Rust type)
7. `RugcheckData.fetched_at` (Rust type)
8. `TokenPoolInfo.fetched_at` (Rust type)
9. `Token.fetched_at` (Rust type) - Mapped from market data
10. Used in freshness checks, cache invalidation, sorting

### All `created_at` Occurrences:

1. `tokens.created_at` (DB field) - Overwritten on updates!
2. `Token.created_at` (Rust type) - Maps to DB
3. `Token.first_seen_at` (Rust type) - Maps to same DB field
4. `blacklist.added_at` - Separate, unrelated
5. `pools.price_history.created_at` - Pool price record insertion
6. Sorting queries: Used as fallback in multiple contexts

**Total: 30+ distinct timestamp field usages, 12+ naming variations, 5+ semantic meanings**

---

**END OF INVESTIGATION**
