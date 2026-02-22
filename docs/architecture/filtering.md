# Filtering Module Architecture

## 1. Overview

The Filtering module evaluates discovered Solana tokens against configurable quality and security criteria before allowing them into the trading pipeline. It implements a sequential filter chain with early rejection, caching for query performance, and batch database operations for efficiency.

**Key Capabilities:**

- Multi-stage validation: Meta → OnChain → DexScreener → GeckoTerminal → Rugcheck → AI
- Config-driven thresholds with hot-reload support
- Cached snapshots with 180-second staleness threshold
- Non-blocking queries with background refresh
- Historical decision tracking (1000 recent pass/reject per category)
- Real-time pool price overlay for sorting/display
- Batch database operations (reduces 260k+ tasks to 4 per refresh)

## 2. Module Structure

```
src/filtering/
├── mod.rs              # Public API: refresh(), query_tokens(), get_filtered_mints()
├── engine.rs           # Core pipeline: compute_snapshot(), apply_all_filters()
├── store.rs            # Caching layer: FilteringStore with RwLock<Arc<Snapshot>>
├── types.rs            # Data structures: FilteringSnapshot, FilteringQuery, enums
└── sources/            # Filter implementations
    ├── mod.rs          # FilterSource & FilterRejectionReason enums (~100 variants)
    ├── meta.rs         # Pre-filters: age, cooldown, decimals (runs FIRST)
    ├── onchain.rs      # Scam detection: symbol analysis, authority checks, risk scoring
    ├── dexscreener.rs  # Market data: liquidity, volume, txns, price changes (43 checks)
    ├── geckoterminal.rs# Market data: alternative source validation (24 checks)
    ├── rugcheck.rs     # Security: authorities, holders, LP lock, transfer fee (28 checks)
    └── ai.rs           # LLM evaluation with confidence threshold (async)
```

## 3. Core Types

### FilteringSnapshot
```rust
use chrono::{DateTime, Utc};
use std::collections::HashMap;

pub struct FilteringSnapshot {
    pub updated_at: DateTime<Utc>,                              // Snapshot timestamp
    pub filtered_mints: Vec<String>,                            // All passing tokens
    pub passed_tokens: Vec<PassedToken>,                        // Top 1000 by time
    pub rejected_mints: Vec<String>,                            // All failing tokens
    pub rejected_tokens: Vec<RejectedToken>,                    // Top 1000 by time
    pub tokens: HashMap<String, TokenEntry>,                    // Full token data + flags
    pub blacklist_reasons: HashMap<String, Vec<BlacklistReasonInfo>>, // Multi-source blacklist
}
```

### TokenEntry
```rust
use chrono::{DateTime, Utc};
use std::sync::Arc;

pub struct TokenEntry {
    pub token: Arc<Token>,              // Shared reference (avoids cloning large Token structs per snapshot)
    pub has_pool_price: bool,           // Pre-computed flag
    pub has_open_position: bool,        // Pre-computed flag
    pub has_ohlcv: bool,                // Pre-computed flag
    pub pair_created_at: Option<i64>,   // Blockchain or discovery timestamp
    pub last_updated: DateTime<Utc>,    // Last data update (snapshot-derived)
}
```

### FilteringQuery
```rust
pub struct FilteringQuery {
    pub view: FilteringView,            // Pool, All, Passed, Rejected, Blacklisted, etc.
    pub search: Option<String>,         // Symbol/mint/name search
    pub sort_key: TokenSortKey,         // PriceSol, LiquidityUsd, Volume24h, etc.
    pub sort_direction: SortDirection,  // Asc or Desc
    pub page: usize,                    // Page number (1-indexed)
    pub page_size: usize,               // Items per page (max 200)
    
    // Range filters
    pub min_liquidity: Option<f64>,
    pub max_liquidity: Option<f64>,
    pub min_volume_24h: Option<f64>,
    pub max_volume_24h: Option<f64>,
    pub max_risk_score: Option<i32>,
    pub min_unique_holders: Option<i32>,
    
    // Boolean filters
    pub has_pool_price: Option<bool>,
    pub has_open_position: Option<bool>,
    pub has_ohlcv: Option<bool>,
    pub blacklisted: Option<bool>,
    pub rejection_reason: Option<String>, // Filter by specific rejection
}
```

### FilteringView Enum
```rust
pub enum FilteringView {
    Pool,           // Tokens with pool prices
    All,            // All tokens (bypasses snapshot, queries DB directly)
    Passed,         // Passed filtering
    Rejected,       // Failed filtering
    Blacklisted,    // Manually blacklisted
    Positions,      // With open positions
    Recent,         // Recently updated
    NoMarketData,   // No DexScreener/GeckoTerminal data
}
```

### FilterSource Enum
```rust
pub enum FilterSource {
    Core,           // Meta filters (age, cooldown, decimals)
    OnChain,        // Symbol/authority analysis
    DexScreener,    // DexScreener market data
    GeckoTerminal,  // GeckoTerminal market data
    Rugcheck,       // Security checks
    Ai,             // LLM evaluation
}
```

### FilterRejectionReason (~100 variants)

**Core (6 reasons):**
- `NoDecimalsInDatabase` - Token decimals not cached
- `TokenTooNew` - Age < min_token_age_minutes
- `CooldownFiltered` - In position cooldown period
- `DexScreenerDataMissing` - No DexScreener data available
- `GeckoTerminalDataMissing` - No GeckoTerminal data available
- `RugcheckDataMissing` - No Rugcheck data available

**OnChain (6 reasons):**
- `OnChainNumericSymbol` - Symbol is all digits
- `OnChainEmptySymbol` - Symbol empty or whitespace
- `OnChainSuspiciousSymbol` - Single non-alphabetic char
- `OnChainKnownScamAuthority` - Freeze/update/mint authority in blocked set
- `OnChainImmutableWithFreeze` - Immutable metadata + freeze authority
- `OnChainHighRiskScore` - Combined risk score too high

**AI (1 reason):**
- `AiRejected { reason: String, confidence: u8, provider: String }` - LLM rejection

**DexScreener:**
- Token info: `DexScreenerEmptyName`, `DexScreenerEmptySymbol`, `DexScreenerEmptyLogoUrl`, `DexScreenerEmptyWebsiteUrl`
- Transactions: `DexScreenerInsufficientTransactions5Min`, `DexScreenerInsufficientTransactions1H`
- Liquidity: `DexScreenerZeroLiquidity`, `DexScreenerInsufficientLiquidity`, `DexScreenerLiquidityTooHigh`
- Market cap: `DexScreenerMarketCapTooLow`, `DexScreenerMarketCapTooHigh`
- FDV: `DexScreenerFdvMissing`, `DexScreenerFdvTooLow`, `DexScreenerFdvTooHigh`
- Volume: `DexScreenerVolumeMissing`, `DexScreenerVolumeTooLow`, plus timeframe-specific `DexScreenerVolume{5m,1h,6h}*`
- Price change: `DexScreenerPriceChange*` (timeframe-specific TooLow/TooHigh/Missing variants)

**GeckoTerminal:**
- Liquidity: `GeckoTerminalLiquidityMissing`, `GeckoTerminalLiquidityTooLow`, `GeckoTerminalLiquidityTooHigh`
- Market cap: `GeckoTerminalMarketCapMissing`, `GeckoTerminalMarketCapTooLow`, `GeckoTerminalMarketCapTooHigh`
- Volume: `GeckoTerminalVolume{5m,1h,24h}Missing/TooLow`
- Price change: `GeckoTerminalPriceChange{5m,1h,24h}TooLow/TooHigh/Missing`
- Pool metrics: `GeckoTerminalPoolCountTooLow`, `GeckoTerminalPoolCountTooHigh`, `GeckoTerminalReserveTooLow`

**Rugcheck:**
- Status: `RugcheckRuggedToken`, `RugcheckRiskScoreTooHigh`, `RugcheckRiskLevelDanger`
- Authorities: `RugcheckMintAuthorityBlocked`, `RugcheckFreezeAuthorityBlocked`
- Holders: `RugcheckNotEnoughHolders`, `RugcheckTopHolderTooHigh`, `RugcheckTop3HoldersTooHigh`
- Insiders: `RugcheckInsiderHolderCount`, `RugcheckInsiderTotalPct`, `RugcheckGraphInsidersTooHigh`
- Creator: `RugcheckCreatorBalanceTooHigh`
- Transfer: `RugcheckTransferFeePresent`, `RugcheckTransferFeeTooHigh`, `RugcheckTransferFeeMissing`
- LP: `RugcheckLpProvidersTooLow`, `RugcheckLpProvidersMissing`, `RugcheckLpLockTooLow`

## 4. Filtering Pipeline

```
┌─────────────────────────────────────────────────────────────────┐
│  INPUT: Token Stream (56k tokens with market data)              │
└───────────────────────────────┬─────────────────────────────────┘
                                │
                    ┌───────────▼────────────┐
                    │  Batch Load Tokens     │
                    │  (Preferred + Fallback)│
                    └───────────┬────────────┘
                                │
                    ┌───────────▼────────────┐
                     │  Arc<Token> Wrapping   │
                     │  (Avoids Token clones) │
                    └───────────┬────────────┘
                                │
              ┌─────────────────┼─────────────────┐
              │                 │                 │
        ┌─────▼─────┐   ┌──────▼──────┐   ┌─────▼─────┐
        │Blacklist  │   │Priced Set   │   │Open       │
        │3 Sources  │   │(Pools)      │   │Positions  │
        └───────────┘   └─────────────┘   └───────────┘
                                │
                    ┌───────────▼────────────┐
                    │  FOR EACH TOKEN:       │
                    └───────────┬────────────┘
                                │
                    ┌───────────▼────────────┐
                    │  [1] Meta Filter       │
                    │  ✓ Decimals cached?    │
                    │  ✓ Age >= threshold?   │
                    │  ✓ Cooldown check      │
                    └───────────┬────────────┘
                                │ PASS
                    ┌───────────▼────────────┐
                    │  [2] OnChain Filter    │
                    │  ✓ Symbol valid?       │
                    │  ✓ Authority clean?    │
                    │  ✓ Risk score OK?      │
                    └───────────┬────────────┘
                                │ PASS
                    ┌───────────▼────────────┐
                    │  [3] DexScreener       │
                    │  (If data_source match)│
                    │  ✓ Token info complete?│
                    │  ✓ Liquidity in range? │
                    │  ✓ Volume sufficient?  │
                    │  ✓ Price change OK?    │
                    └───────────┬────────────┘
                                │ PASS
                    ┌───────────▼────────────┐
                    │  [4] GeckoTerminal     │
                    │  (If data_source match)│
                    │  ✓ Market data valid?  │
                    │  ✓ Pool metrics OK?    │
                    └───────────┬────────────┘
                                │ PASS
                    ┌───────────▼────────────┐
                    │  [5] Rugcheck Filter   │
                    │  ✓ Not rugged?         │
                    │  ✓ Authorities clean?  │
                    │  ✓ Holder distribution?│
                    │  ✓ LP lock sufficient? │
                    └───────────┬────────────┘
                                │ PASS
                    ┌───────────▼────────────┐
                    │  [6] AI Filter (async) │
                    │  ✓ LLM evaluation      │
                    │  ✓ Confidence check    │
                    │  ✓ Fallback on error   │
                    └───────────┬────────────┘
                                │
                  ┌─────────────┴─────────────┐
                  │                           │
              PASS│                           │REJECT
                  │                           │
          ┌───────▼────────┐         ┌────────▼────────┐
          │ passed_tokens  │         │rejected_tokens  │
          │ filtered_mints │         │rejected_mints   │
          └───────┬────────┘         └────────┬────────┘
                  │                           │
                  └───────────┬───────────────┘
                              │
                  ┌───────────▼────────────┐
                  │ Build FilteringSnapshot│
                  │ • Sort by time         │
                  │ • Truncate to 1000     │
                  │ • Build token entries  │
                  │ • Attach blacklist     │
                  └───────────┬────────────┘
                              │
              ┌───────────────┼───────────────┐
              │               │               │
    ┌─────────▼────┐  ┌──────▼──────┐  ┌────▼─────┐
    │Clear         │  │Update       │  │Upsert    │
    │Rejection     │  │Rejection    │  │Stats     │
    │Status (pass) │  │Status (fail)│  │(hourly)  │
    └──────────────┘  └─────────────┘  └──────────┘
                              │
                  ┌───────────▼────────────┐
                  │  OUTPUT: Snapshot      │
                  │  Cached in Store       │
                  └────────────────────────┘
```

**Sequential Execution:** Token must pass ALL enabled filters. First rejection stops processing (short-circuit).

**Data source awareness (and N+1 avoidance):**

- The batch load already assembles each `Token` using **preferred + fallback** market sources and sets `token.data_source`.
- Filtering does **not** perform per-token DB/API fetches to load the other market source (avoids N+1 queries).
- If a source filter is enabled but `token.data_source` does not match, the engine rejects with `*DataMissing`:
  - DexScreener enabled + `token.data_source != DexScreener` → `DexScreenerDataMissing`
  - GeckoTerminal enabled + `token.data_source != GeckoTerminal` → `GeckoTerminalDataMissing`

**Background Refresh:** Snapshot refresh spawns 4 batch database update tasks (fire-and-forget) and returns immediately. Tasks complete asynchronously.

## 5. Filter Sources

### Meta Filters (Core)
**Purpose:** Pre-filter checks that run before any external API calls

**Checks:**
- **Decimals cached:** Token must have decimals in database (avoids N RPC calls)
- **Token age:** Token age >= `min_token_age_minutes` (default: blocks tokens < 60 minutes old)
- **Cooldown:** Token not in position cooldown period (configurable hours after exit)

**Config Keys:**
```toml
[filtering]
age_enabled = true
min_token_age_minutes = 60
cooldown_enabled = true
check_cooldown = true
```

**Why First:** Meta checks are fast (database lookups) and eliminate tokens before expensive API validation.

### OnChain Filters
**Purpose:** Detect scams using on-chain metadata (no RPC calls required)

**Checks:**
1. **Numeric symbol:** Symbol is all digits (spam pattern)
2. **Empty symbol:** Symbol empty or whitespace
3. **Suspicious symbol:** Single non-alphabetic character
4. **Known scam authority:** Freeze/update/mint authority on discovered scam list
5. **Immutable + freeze:** Metadata immutable but freeze authority present (scam signal)
6. **Risk score:** Combined score from multiple signals (capped at 100)

**Risk Scoring:**
- Numeric symbol: +30 points
- Empty symbol: +25 points
- Freeze authority: +10 points
- Immutable metadata with other signals: +10 points
- Name == symbol: +15 points

**Config Keys:**
```toml
[filtering.onchain]
enabled = true
reject_numeric_symbols = true
reject_empty_symbols = true
reject_single_char_symbols = false
reject_known_scam_authorities = true
reject_immutable_with_freeze = true
combined_risk_enabled = true
max_combined_risk_score = 60
```

**Authority Cache:** Uses `tokens::authority_cache::is_blocked_authority()` - auto-discovered scam authorities from previous rejections.

### DexScreener Filters
**Purpose:** Validate market quality using DexScreener API data

**Checks (43 total):**

**Token Info:**
- Name, symbol, logo, website presence (configurable required fields)

**Transaction Activity:**
- 5-minute buy+sell totals
- 1-hour buy+sell totals
- (Only runs if `token.data_source == DataSource::DexScreener`)

**Liquidity:**
- Zero check (immediate reject)
- Min/max range (configurable USD thresholds)

**Market Cap:**
- Min/max range

**FDV (Fully Diluted Valuation):**
- Missing data handling
- Min/max range

**Volume:**
- 5m, 1h, 6h, 24h intervals
- Each configurable: threshold > 0 disables check, None = missing error, low = too low

**Price Change:**
- 5m, 1h, 6h, 24h intervals
- Min/max percentage ranges

**Config Keys:**
```toml
[filtering.dexscreener]
enabled = true
token_info_enabled = true
require_name_and_symbol = true
require_logo_url = false
require_website_url = false

transactions_enabled = true
min_transactions_5min = 10
min_transactions_1h = 50

liquidity_enabled = true
min_liquidity_usd = 1000.0
max_liquidity_usd = 500000.0

volume_enabled = true
min_volume_5m = 100.0
min_volume_1h = 500.0
min_volume_6h = 2000.0
min_volume_24h = 5000.0

price_change_enabled = true
min_price_change_m5 = -50.0
max_price_change_m5 = 200.0
# ... similar keys exist for h1/h6/h24
```

### GeckoTerminal Filters
**Purpose:** Alternative market data validation (24 checks)

**Checks:**
- **Liquidity:** Missing/low/high (max > 0 enables high check)
- **Market Cap:** Missing/low/high
- **Volume:** 5m, 1h, 24h
- **Price Change:** 5m, 1h, 24h
- **Pool Count:** Min/max thresholds
- **Reserve USD:** Minimum threshold

**Data Source Guard:** Only runs if `token.data_source == DataSource::GeckoTerminal`

**Config Keys:** Similar structure to DexScreener with `[filtering.geckoterminal]` section.

### Rugcheck Filters
**Purpose:** Security validation using Rugcheck API data (28 checks)

**Checks:**

**Status:**
- Rugged flag (immediate reject)
- Risk score threshold
- Danger-level risk present

**Authorities:**
- Mint authority presence (configurable allowance)
- Freeze authority presence (configurable allowance)

**Holder Distribution:**
- Minimum total holders
- Top holder percentage (single wallet dominance)
- Top 3 holders percentage (concentration risk)

**Insider Analysis:**
- Insider holders in top 10 (count + total %)
- Graph insiders detected count

**Creator:**
- Creator balance percentage (rug risk)

**Transfer Fee:**
- Fee presence check (blocks if any fee detected)
- Fee threshold (max allowed %)
- Missing data handling (configurable strict mode)

**LP (Liquidity Provider):**
- Minimum LP provider count
- Missing data error handling
- LP lock percentage:
  - Pump.fun tokens: lower threshold (50% default)
  - Regular tokens: higher threshold (80% default)

**Config Keys:**
```toml
[filtering.rugcheck]
enabled = true

# Risk score (raw Rugcheck score; lower = safer)
risk_score_enabled = true
max_risk_score = 10000

# Authorities
authority_checks_enabled = true
require_authorities_safe = true
allow_mint_authority = false
allow_freeze_authority = false

# Risk level
risk_level_enabled = true
block_danger_level = true

# Holders / concentration
holder_distribution_enabled = true
min_unique_holders = 50
max_top_holder_pct = 40.0
max_top_3_holders_pct = 60.0

# LP lock
lp_lock_enabled = true
min_pumpfun_lp_lock_pct = 50.0
min_regular_lp_lock_pct = 50.0

# Rugged flag
rugged_check_enabled = true
block_rugged_tokens = true

# Insiders
graph_insiders_enabled = true
max_graph_insiders = 3
insider_holder_checks_enabled = true
max_insider_holders_in_top_10 = 2
max_insider_total_pct = 20.0

# Creator
creator_balance_enabled = true
max_creator_balance_pct = 10.0

# LP providers
lp_providers_enabled = true
min_lp_providers = 3

# Transfer fee (Token-2022)
transfer_fee_enabled = true
block_transfer_fee_tokens = false
max_transfer_fee_pct = 5.0
```

**Pump.fun Detection:** Checks `token.token_type` field for "pump" string to apply alternative thresholds.

### AI Filter
**Purpose:** LLM-based token evaluation (async, runs LAST)

**Checks:**
1. AI engine availability (`try_get_ai_engine()`)
2. AI filtering enabled in config
3. Confidence threshold met
4. Decision: "pass" vs "reject"
5. Fallback handling on error/low confidence

**Config Keys:**
```toml
[ai]
enabled = true
filtering_enabled = true
filtering_min_confidence = 0.7
filtering_fallback_pass = true  # Pass on error/low confidence if true
```

**Execution Model:**
- Async (doesn't block pipeline)
- Priority::Low (background processing)
- Token serialized to JSON for LLM context
- Fallback behavior configurable: pass or reject on uncertainty

**Cross-module:** Uses `ai::try_get_ai_engine()` and `ai::types::EvaluationContext`.

## 6. Caching & Storage

### FilteringStore Pattern
```rust
pub struct FilteringStore {
    snapshot: RwLock<Option<Arc<FilteringSnapshot>>>,  // Cached snapshot
    refresh_in_progress: AtomicBool,                   // Refresh flag
    refresh_lock: Mutex<()>,                           // Serialize refreshes
}
```

**RwLock Strategy:**
- Multiple concurrent readers (queries)
- Single exclusive writer (refresh)
- No reader blocking during reads

**Arc Wrapper:**
- Snapshot cloned cheaply (8-byte pointer copy)
- Multiple consumers share same data
- Inner Token also Arc-wrapped (prevents N×Token clones per query)

### Staleness Management
```rust
const FILTER_CACHE_STALE_SECS: u64 = 180;  // 3 minutes
```

**Refresh Behavior:**

1. **Query arrives** → `ensure_snapshot()` called
2. **Snapshot exists?**
   - Yes + fresh (< 180s old): Return immediately
   - Yes + stale: Return current, spawn background refresh (non-blocking)
   - No: Wait up to 30s for initial compute
3. **Background refresh:**
   - Check `refresh_in_progress` flag (fast atomic check)
   - Acquire `refresh_lock` (serializes concurrent refresh attempts)
   - Double-check flag (race condition guard)
   - Call `compute_snapshot()`
   - Swap new snapshot into RwLock (exclusive write)
   - Clear flag

**First Access:** Blocks up to 30 seconds with timeout on initial snapshot creation. Returns error if timeout exceeded.

### Historical Limits
```rust
const MAX_DECISION_HISTORY: usize = 1000;
```

**Applied to:**
- `passed_tokens`: Top 1000 by passed_time (descending)
- `rejected_tokens`: Top 1000 by rejection_time (descending)

**Rationale:** Prevents unbounded memory growth while maintaining recent audit trail. Full lists available via `filtered_mints` and `rejected_mints` (no limit).

### Database Persistence

**Batch Updates (4 concurrent tasks):**

1. **Clear rejection status** (passed tokens):
   ```sql
   UPDATE tokens SET 
     last_rejection_reason = NULL,
     last_rejection_source = NULL,
     last_rejection_at = NULL
   WHERE mint IN (...)
   ```

2. **Update priority** (passed tokens):
   ```sql
   UPDATE tokens SET priority = 60 WHERE mint IN (...)
   ```

3. **Update rejection status** (failed tokens):
   ```sql
   UPDATE tokens SET
     last_rejection_reason = ?,
     last_rejection_source = ?,
     last_rejection_at = ?
   WHERE mint = ?
   ```

4. **Upsert rejection stats** (hourly aggregation):
   ```sql
   INSERT INTO rejection_stats (bucket_hour, reason, source, rejection_count, ...)
   VALUES (?, ?, ?, ?)
   ON CONFLICT (bucket_hour, reason, source) DO UPDATE ...
   ```

**Fire-and-Forget:** Tasks spawned with `tokio::spawn`, no await in compute_snapshot(). Snapshot returns immediately after spawning.

**Stored Results:** `tokens::store_filtered_results()` persists snapshot metadata:
- `passed`: filtered_mints
- `rejected`: rejected_mints
- `blacklisted`: from token.is_blacklisted flag
- `with_pool_price`: has_pool_price entries
- `open_positions`: has_open_position entries
- `updated_at`: snapshot timestamp

## 7. Query System

### execute_query() Flow

**Input:** `FilteringQuery` (view, search, filters, sort, pagination)

**Output:** `FilteringQueryResult` (items, total_count, page, page_size, metadata)

### View-Specific Handling

**"All" View:**
```rust
// Bypasses snapshot, queries database directly with SQL pagination
let tokens = get_all_tokens_optional_market_async(page, page_size, sort_key, sort_dir).await?;
let total_count = count_tokens_async().await?;  // Fast count query
```
**Why:** "All" includes tokens with no market data (not in snapshot). Direct DB query ensures completeness.

**"NoMarketData" View:**
```rust
// Queries tokens without DexScreener/GeckoTerminal data
let tokens = get_tokens_no_market_async(page, page_size).await?;
```

**Other Views (Pool, Passed, Rejected, Blacklisted, Positions, Recent):**
```rust
// 1. Get snapshot
let snapshot = self.ensure_snapshot().await?;

// 2. Collect relevant tokens
let mut entries: Vec<TokenEntry> = match view {
    Pool => snapshot.tokens.values().filter(|e| e.has_pool_price).cloned().collect(),
    Passed => snapshot.tokens.values().filter(|e| snapshot.filtered_mints.contains(&e.token.mint)).cloned().collect(),
    Rejected => snapshot.tokens.values().filter(|e| snapshot.rejected_mints.contains(&e.token.mint)).cloned().collect(),
    // ... other views
};

// 3. Apply filters (search, liquidity, volume, risk, etc.)
apply_filters(&mut entries, &query, &snapshot);

// 4. Sort
sort_tokens(&mut entries, &query);

// 5. Paginate
let total_count = entries.len();
let start = (query.page - 1) * query.page_size;
let items = entries.into_iter().skip(start).take(query.page_size).collect();
```

### Filter Application

**Search (case-insensitive):**
- `token.symbol` contains query
- `token.mint` contains query
- `token.name` contains query

**Range Filters:**
- `min_liquidity` / `max_liquidity`: Uses `token.liquidity_usd`
- `min_volume_24h` / `max_volume_24h`: Uses `token.volume_h24`
- `max_risk_score`: Uses `token.security_score` (raw Rugcheck risk; higher = more risky)
- `min_unique_holders`: Uses `token.total_holders`

**Boolean Filters:**
- `has_pool_price`: Filters by `has_pool_price` flag
- `has_open_position`: Filters by `has_open_position` flag
- `has_ohlcv`: Filters by `has_ohlcv` flag
- `blacklisted`: Filters by `token.is_blacklisted`
- `rejection_reason`: Matches `token.last_rejection_reason`

### Sorting

**TokenSortKey variants:**
- `Symbol`, `Mint`: String comparison
- `PriceSol`: Real-time pool price via `pools::get_pool_price()` (fallback to token.price_sol)
- `LiquidityUsd`, `Volume24h`, `Fdv`, `MarketCap`: Numeric field comparison
- `PriceChangeH1`, `PriceChangeH24`: Numeric field comparison
- `MarketDataLastFetchedAt`, `FirstDiscoveredAt`, `MetadataLastFetchedAt`, `BlockchainCreatedAt`, `PoolPriceLastCalculatedAt`: Timestamp comparison
- `Txns5m`, `Txns1h`, `Txns6h`, `Txns24h`: Sum of buy_count + sell_count
- `RiskScore`: Security metric (Rugcheck risk score)

**Pool Price Overlay:** For Pool view sorts, calls `pools::get_pool_price()` to get real-time price instead of stale token.price_sol.

### Pagination

**Max Page Size:** 200 items

**Calculation:**
```rust
let start = (query.page - 1) * query.page_size;  // 0-indexed offset
let items: Vec<Token> = entries
    .into_iter()
    .skip(start)
    .take(query.page_size)
    .collect();
```

**Response Metadata:**
```rust
FilteringQueryResult {
    items,
    total_count,
    page: query.page,
    page_size: query.page_size,
    total_pages: (total_count + query.page_size - 1) / query.page_size,
    
    // Derived sets (for UI indicators)
    priced_mints: HashSet<String>,       // Tokens with pool prices
    open_position_mints: HashSet<String>,// Tokens with open positions
    ohlcv_mints: HashSet<String>,        // Tokens with OHLCV data
    
    // Rejection metadata (for filter dropdowns)
    rejection_reasons: Vec<String>,      // Unique rejection reasons from DB
    blacklist_reasons: HashMap<String, Vec<BlacklistReasonInfo>>,
}
```

## 8. Integration Points

### tokens Module
**Used by filtering:**
- `get_all_tokens_for_filtering_async()` - Batch load tokens with market data (preferred + fallback)
- `count_tokens_async()` - Fast count query for "All" view
- `get_tokens_no_market_async()` - Tokens without market data for "NoMarketData" view
- `list_blacklisted_tokens_async()` - Blacklist source #1
- `get_rejection_stats_async()` - Unique rejection reasons for filter dropdowns
- `store_filtered_results()` - Persist snapshot metadata
- `batch_clear_rejection_status_async()` - Clear rejection fields for passed tokens
- `batch_update_priority_async()` - Set priority=60 for passed tokens
- `batch_update_rejection_status_async()` - Set rejection fields for failed tokens
- `batch_upsert_rejection_stats_async()` - Aggregate hourly rejection stats
- `authority_cache::is_blocked_authority()` - Scam authority detection

**Data flow:**
- Filtering reads Token structs with market data
- Filtering writes rejection reasons, priority, filtered status back
- Token.last_rejection_reason used in query filtering

### pools Module
**Used by filtering:**
- `db::list_blacklisted_pools()` - Blacklist source #2 (by pool_id)
- `db::list_blacklisted_accounts()` - Blacklist source #3 (by account_pubkey)
- `get_available_tokens()` - Set of tokens with pool prices
- `get_pool_price()` - Real-time price overlay for sorting/display

**Data flow:**
- Filtering checks if token has pool price (has_pool_price flag)
- Filtering uses real-time pool price for accurate sorting

### positions Module
**Used by filtering:**
- `get_open_mints()` - Set of tokens with open positions

**Data flow:**
- Filtering sets has_open_position flag for UI indicators
- Positions view filters to tokens with open trades

### config Module
**Used by filtering:**
- `with_config(|cfg| cfg.filtering.clone())` - Get current filter config during snapshot compute

**Data flow:**
- Config changes picked up on next refresh (hot-reload)
- Each source reads its config section (meta, onchain, dexscreener, etc.)

### events Module
**Used by filtering:**
- `record_sampling_event()` - Logs 1 in 10 token decisions for analytics

**Data flow:**
- Sampling reduces log volume (56k tokens → 5.6k events per refresh)
- Events used for filter effectiveness analysis

### ai Module
**Used by filtering:**
- `try_get_ai_engine()` - Get global AI engine instance
- `ai::types::EvaluationContext` - Token evaluation request structure

**Data flow:**
- AI filter runs async at end of pipeline
- Token serialized to JSON for LLM analysis
- AI decision (pass/reject) with confidence score

## 9. Performance Optimizations

### Arc<Token> Wrapping
**Problem:** Each `Token` is ~2KB. Cloning `N` tokens is ~`N * 2KB` of transient memory per operation (e.g. 56k → ~112MB).

**Solution:**
```rust
// Before processing
let arc_tokens: Vec<Arc<Token>> = tokens.into_iter().map(Arc::new).collect();

// During filtering
let token_entry = TokenEntry {
    token: arc_token.clone(),  // 8-byte pointer copy, not 2KB clone
    // ... flags
};
```

**Impact:** Reduces per-operation memory from **O(N × size(Token))** to **O(N × size(Arc pointer))** (e.g. ~112MB → ~0.4MB for 56k tokens, plus container overhead).

### Batch Database Operations
**Problem:** Original code spawned 260k+ tokio tasks (56k tokens × 4 DB operations each + 5.6k sampling events).

**Solution:** Collect all updates in vectors during filtering, spawn 4 batch tasks at end:
```rust
// Collect updates during loop
let mut passed_mints = Vec::new();
let mut rejected_updates = Vec::new();
let mut stats = HashMap::new();

for token in tokens {
    match apply_all_filters(&token) {
        Ok(_) => passed_mints.push(token.mint.clone()),
        Err(reason) => {
            rejected_updates.push((token.mint.clone(), reason.clone()));
            *stats.entry(reason).or_insert(0) += 1;
        }
    }
}

// Spawn 4 batch tasks (fire-and-forget)
tokio::spawn(batch_clear_rejection_status_async(passed_mints.clone()));
tokio::spawn(batch_update_priority_async(passed_mints, 60));
tokio::spawn(batch_update_rejection_status_async(rejected_updates));
tokio::spawn(batch_upsert_rejection_stats_async(stats));
```

**Impact:** Reduces task scheduler overhead, improves refresh latency from 10s+ to 2-4s.

### Stale Threshold + Background Refresh
**Problem:** Refreshing 56k tokens takes 2-4 seconds. Blocking queries during refresh hurts UX.

**Solution:**
```rust
pub async fn ensure_snapshot(&self) -> Result<Arc<FilteringSnapshot>> {
    if let Some(snapshot) = self.snapshot.read().await.clone() {
        // Have snapshot (even if stale) - return immediately
        if self.is_snapshot_stale(&snapshot) && !self.refresh_in_progress.load(Ordering::Acquire) {
            // Stale + not refreshing - spawn background refresh (non-blocking)
            let store = self.clone();
            tokio::spawn(async move { store.try_refresh_background().await });
        }
        return Ok(snapshot);
    }
    
    // No snapshot - wait up to 30s for initial compute
    self.try_refresh_with_timeout(Duration::from_secs(30)).await
}
```

**Impact:** Queries return in <1ms (stale read), refresh happens in background. Users see slightly stale data (< 3 min) instead of blocking.

### Fast Count Queries
**Problem:** Counting 138k+ tokens in "All" view requires full scan.

**Solution:** Use database count query instead of loading all tokens:
```rust
FilteringView::All => {
    let total_count = count_tokens_async().await?;  // SELECT COUNT(*) - fast
    // Only load current page worth of tokens
}
```

**Impact:** Pagination loads 50-200 items instead of 138k full list.

### Snapshot Token HashMap
**Problem:** Finding token by mint in query results requires O(n) scan.

**Solution:** Pre-build HashMap during snapshot construction:
```rust
let mut token_entries = HashMap::with_capacity(tokens.len());
for token in tokens {
    token_entries.insert(token.mint.clone(), TokenEntry {
        token: Arc::new(token),
        has_pool_price: priced_set.contains(&token.mint),
        has_open_position: open_position_set.contains(&token.mint),
        has_ohlcv: ohlcv_set.contains(&token.mint),
        // ... other fields
    });
}
```

**Impact:** O(1) token lookups during query execution, rejection reason matching, blacklist overlay.

## 10. Key Patterns & Pitfalls

### Sequential Filter Chain
**Pattern:** Filters evaluated in fixed order with short-circuit rejection.

**Why:** Faster/cheaper filters run first:
1. Meta (database lookups) - ~1ms
2. OnChain (memory checks) - ~1μs
3. DexScreener/GeckoTerminal (cached data) - ~10μs
4. Rugcheck (cached data) - ~10μs
5. AI (async LLM call) - ~500ms

**Pitfall:** Order matters! Moving AI filter before Meta would cause 56k × 500ms = 28,000 seconds (7.7 hours) of LLM calls.

### Config-Driven Enable/Disable
**Pattern:** Each source and individual check has `enabled` flag:
```rust
if config.dexscreener.enabled {
    if config.dexscreener.liquidity_enabled {
        enforce_liquidity_threshold(token, config)?;
    }
}
```

**Why:** Allows granular control without code changes. Disable expensive checks during testing.

**Pitfall:** Forgetting `enabled` checks causes all filters to run even when disabled in config.

### Data Source Guards
**Pattern:** Validate data source before applying metrics:
```rust
// DexScreener checks
if token.data_source == DataSource::DexScreener {
    enforce_transactions(token, config)?;
}

// GeckoTerminal checks
if token.data_source == DataSource::GeckoTerminal {
    enforce_pool_count(token, config)?;
}
```

**Why:** Different sources provide different fields. GeckoTerminal has pool_count, DexScreener doesn't. Applying wrong checks causes false rejections.

**Pitfall:** Removing source guards causes "missing data" rejections for tokens with valid alternative source.

### Batch Collect vs Spawn Pattern
**Anti-pattern:**
```rust
// ❌ BAD: Spawns 260k+ tasks
for token in tokens {
    tokio::spawn(update_rejection_status(token.mint, reason));
}
```

**Correct Pattern:**
```rust
// ✅ GOOD: Collect updates, spawn 4 batch tasks
let mut updates = Vec::new();
for token in tokens {
    updates.push((token.mint, reason));
}
tokio::spawn(batch_update_rejection_status_async(updates));  // Single task
```

**Why:** Tokio task spawning has overhead. 260k small tasks overwhelm scheduler. 4 batch tasks process same work efficiently.

### RwLock Read-Then-Write Race
**Anti-pattern:**
```rust
// ❌ BAD: Race condition
if self.snapshot.read().await.is_none() {
    let snapshot = compute_snapshot().await?;
    *self.snapshot.write().await = Some(Arc::new(snapshot));  // Another thread may have written
}
```

**Correct Pattern:**
```rust
// ✅ GOOD: Atomic flag + mutex + double-check
if !self.refresh_in_progress.swap(true, Ordering::AcqRel) {
    let _lock = self.refresh_lock.lock().await;
    if self.snapshot.read().await.is_some() {
        self.refresh_in_progress.store(false, Ordering::Release);
        return Ok(self.snapshot.read().await.clone().unwrap());
    }
    // Compute snapshot...
}
```

**Why:** Multiple concurrent refresh attempts waste CPU. Atomic flag + mutex + double-check prevents redundant work.

### Snapshot Staleness vs Query Latency
**Trade-off:** Fresh data (block queries during refresh) vs low latency (serve stale data).

**ScreenerBot Choice:** Serve stale data up to 180 seconds, refresh in background.

**Rationale:** Token filtering changes slowly. 3-minute staleness acceptable for UX. Trading decisions use real-time pool prices (not snapshot data).

**Pitfall:** Setting staleness too low (e.g., 10s) causes frequent refreshes, high CPU usage. Too high (e.g., 600s) causes users to see very outdated rejection counts.

### Missing Data Handling
**Inconsistent Pattern:** Some filters treat missing data as error, others as pass.

**Examples:**
- `DexVolume5mMissing` - Error if volume field is None but check enabled
- `RugcheckTransferFeeMissing` - Only error if `transfer_fee_missing_strict` flag set

**Why:** Trade-off between strictness and false positives. Some metrics (volume) expected on all tokens. Others (transfer fee) may legitimately be missing.

**Config Control:**
```toml
[filtering.rugcheck]
transfer_fee_missing_strict = false  # Pass if transfer fee data unavailable
lp_missing_error = true              # Reject if LP data unavailable
```

**Pitfall:** Strict missing data checks reject too many tokens. Lenient checks allow risky tokens through. Requires per-metric tuning based on data source reliability.

---

**Document Version:** 1.0  
**Last Updated:** January 2025  
**Codebase Version:** 0.1.108+
