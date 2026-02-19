# Filtering System Documentation

## Overview

The Filtering System is ScreenerBot's token quality control mechanism that evaluates every discovered Solana token against configurable criteria before allowing it to proceed to trading. It tracks which tokens pass or fail filtering, why they were rejected, and provides comprehensive analytics for optimizing filter settings.

**Key Capabilities:**

- Multi-source validation (DexScreener, GeckoTerminal, RugCheck, Core/Meta filters)
- Real-time rejection tracking with categorized reasons
- Time-range analytics for understanding filtering patterns
- Per-token rejection history with source attribution
- Configurable thresholds for liquidity, age, security, and data quality

## Architecture

### Backend (Rust)

#### Core Components

**1. Filtering Engine** (`src/filtering/`)

- `engine.rs` - Core filtering logic that evaluates tokens against all active sources
- `sources/` - Individual filter implementations:
  - `dexscreener.rs` - Token info, liquidity, volume validation
  - `geckoterminal.rs` - Market data validation
  - `rugcheck.rs` - Security checks (mint/freeze authority, top holder %)
  - `meta.rs` - Cross-source meta filters (age, cooldown, decimals)
- `store.rs` - Cached filtering snapshots with query API
- `types.rs` - FilteringSnapshot, PassedToken, RejectedToken data structures

**2. Database Layer** (`src/tokens/database.rs`)

Two critical tables with **different semantics**:

| Table             | Purpose                   | Semantic                                          | Use Case                                  |
| ----------------- | ------------------------- | ------------------------------------------------- | ----------------------------------------- |
| `update_tracking` | Current token state       | **Unique tokens** - one row per token             | "How many tokens are currently rejected?" |
| `rejection_stats` | Hourly aggregated buckets | **Cumulative events** - rejection count over time | Historical trend analysis                 |

**Schema: `update_tracking`**

```sql
CREATE TABLE update_tracking (
    mint TEXT PRIMARY KEY,
    last_rejection_reason TEXT,
    last_rejection_source TEXT,
    last_rejection_at INTEGER,
    -- ... other tracking fields
);
```

- Stores the **current state** of each token (rejected or passed)
- One row per unique token mint address
- Updated whenever a token's status changes

**Schema: `rejection_stats`**

```sql
CREATE TABLE rejection_stats (
    bucket_hour INTEGER NOT NULL,
    reason TEXT NOT NULL,
    source TEXT NOT NULL,
    rejection_count INTEGER NOT NULL DEFAULT 0,
    unique_tokens INTEGER NOT NULL DEFAULT 0,
    first_seen INTEGER NOT NULL,
    last_seen INTEGER NOT NULL,
    PRIMARY KEY (bucket_hour, reason, source)
);
```

- Aggregates rejection **events** into hourly buckets
- Same token rejected multiple times → multiple counts
- Used for historical trend analysis

**CRITICAL:** Never mix these tables! `update_tracking` for UI token counts, `rejection_stats` for historical events.

**3. Key Database Functions**

```rust
// Get unique tokens currently rejected (from update_tracking)
pub fn get_rejection_stats(&self) -> TokenResult<Vec<(String, String, i64)>>

// Get unique tokens rejected in time range (from update_tracking with filter)
pub fn get_rejection_stats_with_time_filter(
    &self,
    start_time: Option<i64>,
    end_time: Option<i64>,
) -> TokenResult<Vec<(String, String, i64)>>

// Get cumulative rejection events in time range (from rejection_stats)
pub fn get_rejection_stats_aggregated(
    &self,
    start_time: Option<i64>,
    end_time: Option<i64>,
) -> TokenResult<Vec<(String, String, i64)>>
```

**4. API Routes** (`src/webserver/routes/filtering.rs`)

| Endpoint                         | Method  | Purpose                                               |
| -------------------------------- | ------- | ----------------------------------------------------- |
| `/api/filtering/stats`           | GET     | Current filtering snapshot (passed/rejected counts)   |
| `/api/filtering/analytics`       | GET     | Comprehensive analytics with time range support       |
| `/api/filtering/rejection-stats` | GET     | Rejection breakdown by reason/source                  |
| `/api/filtering/rejected-tokens` | GET     | Paginated list of rejected tokens                     |
| `/api/config/filtering`          | GET/PUT | Filtering configuration (thresholds, enabled filters) |

**Note:** Recent rejections data is included in the `/api/filtering/analytics` response, not as a separate endpoint.

**Analytics Query Parameters:**

```
?start_time=<unix_timestamp_seconds>&end_time=<unix_timestamp_seconds>
```

- Omit both → All-time snapshot of current state
- Provide both → Unique tokens rejected in that time range

### Frontend (JavaScript)

#### Page Structure

**Main Module:** `src/webserver/templates/scripts/pages/filtering.js`

**State Management:**

```javascript
const state = {
  config: null, // Filter configuration
  draft: null, // Draft config (for editing)
  stats: null, // Current filtering statistics
  analytics: null, // Analytics data (with time range)
  isLoadingAnalytics: false, // Loading state for analytics
  analyticsRequestId: 0, // Race condition prevention
  timeRange: {
    // Time range filter
    preset: "all", // "1h", "6h", "24h", "7d", "all", "custom"
    startTime: null, // Unix timestamp (seconds)
    endTime: null, // Unix timestamp (seconds)
  },
  activeTab: "status", // Current tab
  // ... more fields
};
```

#### Views (Tabs)

**1. Status Tab** - Overview dashboard

- Total scanned, passed, rejected counts
- Pass/rejection rates
- Rejection breakdown by category (pie chart style)

**2. Analytics Tab** - Advanced analysis

- Time range filter (1H, 6H, 24H, 7D, All, Custom)
- Summary cards (scanned, passed, rejected)
- Rejection by category breakdown
- Rejection by source breakdown
- Top rejection reasons table

**3. Explorer Tab** - Tree view

- Hierarchical view: Categories → Reasons → Individual tokens
- Click a reason to see all tokens rejected for that reason
- Paginated token list with mint address, symbol, timestamp

**4. Config Tabs** (Core, DexScreener, GeckoTerminal, RugCheck)

- Category-grouped filter settings
- Enable/disable per source
- Configurable thresholds (liquidity, age, etc.)
- Import/export configuration

#### Time Range Filtering

**Presets:**

```javascript
const TIME_RANGE_PRESETS = {
  "1h": { label: "1H", seconds: 60 * 60 },
  "6h": { label: "6H", seconds: 6 * 60 * 60 },
  "24h": { label: "24H", seconds: 24 * 60 * 60 },
  "7d": { label: "7D", seconds: 7 * 24 * 60 * 60 },
};
```

**Custom Range:**

- Date/time pickers for start and end
- Validation: start < end, end ≤ now
- Persisted to `AppState` for session continuity

**Loading Pattern:**

```javascript
async function setTimeRangePreset(preset) {
  // Set time range based on preset
  state.timeRange.preset = preset;
  state.timeRange.startTime = preset === "all" ? null : now - seconds;
  state.timeRange.endTime = preset === "all" ? null : now;

  // Show loading state
  state.isLoadingAnalytics = true;
  render();

  try {
    await loadAnalytics();
  } finally {
    state.isLoadingAnalytics = false;
    render();
  }
}
```

## Data Flow

### Token Rejection Flow

```
1. Token discovered
   ↓
2. Filtering Engine evaluates token
   ↓
3a. PASSED → update_tracking: clear last_rejection_*
   ↓
4a. Add to passed tokens cache

3b. REJECTED → update_tracking: set last_rejection_reason/source/at
   ↓
4b. rejection_stats: increment bucket count (hourly aggregation)
   ↓
5b. Add to rejected tokens cache
```

### Analytics Request Flow

```
Frontend clicks time range preset
   ↓
JavaScript sets state.timeRange
   ↓
Fetch /api/filtering/analytics?start_time=X&end_time=Y
   ↓
Backend: get_rejection_stats_aggregated(start, end)
   ↓
Query rejection_stats WHERE bucket_hour BETWEEN start AND end
   ↓
Sum rejection counts per (reason, source)
   ↓
Return aggregated stats (Volume)
   ↓
Frontend renders analytics view
```

## Configuration

### Filter Categories

**1. Meta Requirements** (Core)

- `min_token_age_minutes` - Skip tokens newer than X minutes
- `check_cooldown` - Skip tokens recently exited (cooldown period)

**2. DexScreener Filters**

- `require_name_and_symbol` - Token must have name and symbol
- `require_logo_url` - Token must have logo URL
- `min_liquidity_usd` / `max_liquidity_usd` - Liquidity bounds
- `min_volume_24h_usd` - Minimum 24h trading volume
- `min_price_change_24h` / `max_price_change_24h` - Price volatility bounds

**3. GeckoTerminal Filters**

- Similar to DexScreener but from different data source
- Used as fallback/validation

**4. RugCheck Filters** (Security)

- `check_mint_authority` - Reject if mint authority present
- `check_freeze_authority` - Reject if freeze authority present
- `max_top_10_holder_percent` - Max % held by top 10 wallets

### Configuration File

**Location:** `data/config.toml`

**Example:**

```toml
[filtering]
enabled = true

[filtering.meta]
enabled = true
min_token_age_minutes = 60
check_cooldown = true
cooldown_hours = 24

[filtering.dexscreener]
enabled = true
require_name_and_symbol = true
require_logo_url = false
min_liquidity_usd = 1000.0
min_volume_24h_usd = 500.0

[filtering.rugcheck]
enabled = true
check_mint_authority = true
check_freeze_authority = true
max_top_10_holder_percent = 50.0
```

## Rejection Reasons

### Category: data_quality

| Reason               | Source      | Description                     |
| -------------------- | ----------- | ------------------------------- |
| `dex_data_missing`   | core        | No DexScreener data available   |
| `gecko_data_missing` | core        | No GeckoTerminal data available |
| `rug_data_missing`   | core        | No RugCheck data available      |
| `dex_empty_name`     | dexscreener | Name field is empty             |
| `dex_empty_symbol`   | dexscreener | Symbol field is empty           |
| `dex_empty_logo`     | dexscreener | Logo URL is empty               |
| `no_decimals`        | core        | Token decimals not in database  |

### Category: security

| Reason                 | Source   | Description                                      |
| ---------------------- | -------- | ------------------------------------------------ |
| `rug_mint_authority`   | rugcheck | Mint authority is present (can mint more tokens) |
| `rug_freeze_authority` | rugcheck | Freeze authority is present (can freeze wallets) |
| `rug_top_holders`      | rugcheck | Top 10 holders exceed threshold                  |

### Category: timing

| Reason              | Source | Description                         |
| ------------------- | ------ | ----------------------------------- |
| `token_too_new`     | core   | Token age < min_token_age_minutes   |
| `position_cooldown` | core   | Token in cooldown period after exit |

### Category: liquidity

| Reason                    | Source        | Description                   |
| ------------------------- | ------------- | ----------------------------- |
| `dex_liquidity_too_low`   | dexscreener   | Liquidity < min_liquidity_usd |
| `dex_liquidity_too_high`  | dexscreener   | Liquidity > max_liquidity_usd |
| `gecko_liquidity_too_low` | geckoterminal | Liquidity < min_liquidity_usd |

### Category: volume

| Reason                 | Source        | Description                     |
| ---------------------- | ------------- | ------------------------------- |
| `dex_volume_too_low`   | dexscreener   | 24h volume < min_volume_24h_usd |
| `gecko_volume_too_low` | geckoterminal | 24h volume < min_volume_24h_usd |

### Category: volatility

| Reason                      | Source      | Description                       |
| --------------------------- | ----------- | --------------------------------- |
| `dex_price_change_too_low`  | dexscreener | 24h change < min_price_change_24h |
| `dex_price_change_too_high` | dexscreener | 24h change > max_price_change_24h |

## Cleanup & Maintenance

### Automatic Cleanup (FilteringService)

**Schedule:** Every 10 minutes

**Tasks:**

1. `cleanup_rejection_stats_async(24)` - Keep last 24 hours of hourly buckets
2. `cleanup_rejection_history_async(24)` - Clean deprecated rejection_history table (if exists)

**Why:** The `rejection_stats` table grows rapidly (~1 bucket per hour per reason per source). With 50+ unique reasons, this is ~1200 rows/hour. Cleanup prevents unbounded growth.

### Manual Operations

**Reset all rejections:**

```sql
-- Clear current rejection state
UPDATE update_tracking SET
  last_rejection_reason = NULL,
  last_rejection_source = NULL,
  last_rejection_at = NULL;

-- Clear aggregated stats
DELETE FROM rejection_stats;
```

**Export rejection data:**

```bash
sqlite3 data/tokens.db <<EOF
.mode csv
.headers on
.output rejections_export.csv
SELECT
  ut.mint,
  t.symbol,
  ut.last_rejection_reason,
  ut.last_rejection_source,
  datetime(ut.last_rejection_at, 'unixepoch') as rejected_at
FROM update_tracking ut
LEFT JOIN tokens t ON ut.mint = t.mint
WHERE ut.last_rejection_reason IS NOT NULL
ORDER BY ut.last_rejection_at DESC;
EOF
```

## Common Pitfalls & Best Practices

### 1. **NEVER Mix Data Sources**

❌ **WRONG:**

```rust
// Don't use rejection_stats for UI counts
let stats = get_rejection_stats_aggregated_async(None, None).await?;
// This shows cumulative events, not unique tokens!
```

✅ **CORRECT:**

```rust
// Use update_tracking for current unique token counts
let stats = get_rejection_stats_with_time_filter_async(None, None).await?;
// This shows unique tokens currently rejected
```

### 2. **Race Conditions in Async JS**

❌ **WRONG:**

```javascript
async function refresh() {
  const btn = document.querySelector("button");
  btn.disabled = true;
  await loadData();
  btn.disabled = false; // btn may be stale after await!
}
```

✅ **CORRECT:**

```javascript
async function refresh() {
  state.isLoading = true;
  render(); // Re-render disables button via state

  try {
    await loadData();
  } finally {
    state.isLoading = false;
    render(); // Re-render enables button
  }
}
```

### 3. **Time Range Persistence**

When persisting time range state, validate on restore:

```javascript
// On init, check for inconsistent state
if (
  state.timeRange.preset === "custom" &&
  (!state.timeRange.startTime || !state.timeRange.endTime)
) {
  // Reset to "all" if custom preset but no times
  state.timeRange.preset = "all";
  state.timeRange.startTime = null;
  state.timeRange.endTime = null;
}
```

### 4. **Price Precision**

Tokens can have 12+ decimals. Never use `{:.6}` formatting.

```javascript
// WRONG: May show 0.000000 for tiny prices
const price = 0.00000000123;
console.log(`Price: ${price.toFixed(6)}`); // "Price: 0.000000"

// CORRECT: Use scientific notation for small numbers
if (price < 1e-6) {
  console.log(`Price: ${price.toExponential(2)}`); // "Price: 1.23e-9"
} else {
  console.log(`Price: ${price.toFixed(9)}`);
}
```

### 5. **Loading States**

Always use try/finally for loading flags:

```javascript
state.isLoadingAnalytics = true;
render();

try {
  await loadAnalytics();
} finally {
  // CRITICAL: Always clear loading state, even on error
  state.isLoadingAnalytics = false;
  render();
}
```

## Performance Considerations

### Database Indexing

**Existing Indexes:**

```sql
CREATE INDEX idx_rejection_stats_hour ON rejection_stats(bucket_hour DESC);
```

**Query Performance:**

- `update_tracking` queries (unique tokens): ~10-50ms for 138K tokens
- `rejection_stats` aggregation: ~5-20ms for 24h of data
- Time range queries use `last_rejection_at` - no index needed (table scan is fast)

### Frontend Optimization

**Race Condition Prevention:**

```javascript
const state = {
  analyticsRequestId: 0, // Incremented on each request
};

async function loadAnalytics() {
  const thisRequestId = ++state.analyticsRequestId;

  const data = await fetchAnalytics();

  // Discard stale responses
  if (thisRequestId !== state.analyticsRequestId) {
    return; // A newer request was made
  }

  state.analytics = data;
  render();
}
```

**Debouncing:**

```javascript
// Explorer tree filter (150ms debounce)
filterExplorerTree: (query) => {
  if (!window.filteringPage.debouncedFilterExplorerTree) {
    window.filteringPage.debouncedFilterExplorerTree = Utils.debounce((q) => {
      /* filter logic */
    }, 150);
  }
  window.filteringPage.debouncedFilterExplorerTree(query);
};
```

## Troubleshooting

### "Numbers don't match between views"

**Symptom:** Analytics shows different counts than Explorer

**Cause:** Analytics may be using cached data with different time range

**Fix:** Click Refresh button in footer to reload all data

### "Loading spinner stuck"

**Symptom:** Analytics shows "Loading..." indefinitely

**Cause:** `isLoadingAnalytics` not cleared due to exception

**Fix:**

1. Check browser console for errors
2. Reload page (state will reset)
3. If persists, check backend logs for API errors

### "Time range filter shows wrong data"

**Symptom:** Custom time range returns unexpected counts

**Cause 1:** Frontend sending milliseconds instead of seconds
**Fix:** Backend expects Unix seconds. JavaScript `Date.now()` returns milliseconds, so convert:

```javascript
const unixSeconds = Math.floor(Date.now() / 1000);
// NOT: Date.now() which gives milliseconds
```

**Cause 2:** Timezone issues with date picker
**Fix:** Always use UTC for timestamps, convert to local for display

### "Rejection count keeps growing"

**Symptom:** `rejection_stats` table grows too large

**Cause:** Cleanup task not running or disabled

**Fix:**

1. Check FilteringService is running: GET `/api/services`
2. Verify cleanup interval in logs
3. Manually cleanup: `DELETE FROM rejection_stats WHERE bucket_hour < ...`

## API Examples

### Get Current Filtering Snapshot

```bash
curl http://localhost:8080/api/filtering/analytics | jq
```

Response:

```json
{
  "total_tokens": 138417,
  "total_passed": 427,
  "total_rejected": 137990,
  "pass_rate": 0.3,
  "rejection_rate": 99.7,
  "by_category": [
    {
      "category": "data_quality",
      "label": "Missing Data",
      "count": 135006,
      "percentage": 97.8,
      "reasons": [...]
    }
  ]
}
```

### Get Time-Filtered Analytics

```bash
# Last 24 hours (timestamps in Unix seconds)
START=$(date -v-24H -u +%s)
END=$(date -u +%s)

curl "http://localhost:8080/api/filtering/analytics?start_time=$START&end_time=$END" | jq
```

### Get Rejected Tokens by Reason

```bash
curl "http://localhost:8080/api/filtering/rejected-tokens?reason=dex_data_missing&limit=50" | jq
```

### Update Configuration

```bash
curl -X PUT http://localhost:8080/api/config/filtering \
  -H "Content-Type: application/json" \
  -d '{
    "meta": {
      "enabled": true,
      "min_token_age_minutes": 120
    },
    "dexscreener": {
      "enabled": true,
      "min_liquidity_usd": 5000
    }
  }'
```

## Migration Notes

### From rejection_history to rejection_stats (v0.1.108)

**Old Schema (Deprecated):**

```sql
CREATE TABLE rejection_history (
    mint TEXT,
    reason TEXT,
    source TEXT,
    rejected_at INTEGER
);
-- One row per rejection event (unbounded growth)
```

**New Schema:**

```sql
CREATE TABLE rejection_stats (
    bucket_hour INTEGER,
    reason TEXT,
    source TEXT,
    rejection_count INTEGER,
    PRIMARY KEY (bucket_hour, reason, source)
);
-- One row per hour per reason (O(1) aggregation)
```

**Migration:** No migration needed. Old table is ignored. Cleanup task removes old data automatically.

### From Cumulative Events to Unique Tokens (v0.1.108)

**Breaking Change:** Analytics endpoint semantics changed.

**Before:**

```javascript
// Showed cumulative rejection events (2.5M+)
total_rejected: 2524593;
```

**After:**

```javascript
// Shows unique tokens currently rejected (~138K)
total_rejected: 137990;
```

**Impact:** Charts/dashboards show more realistic numbers now.

## Future Enhancements

### Planned Features

1. **Filter Effectiveness Metrics**
   - Track how many filtered tokens would have been profitable
   - A/B test different filter configurations
   - Suggest optimal thresholds based on historical performance

2. **Smart Filters**
   - ML-based token quality scoring
   - Pattern recognition for rug pulls
   - Community-sourced filter rules

3. **Advanced Analytics**
   - Rejection trends over time (line charts)
   - Filter impact analysis (which filters block the most?)
   - Correlation analysis (liquidity vs rejection rate)

4. **Export/Import**
   - ✅ Config export/import (implemented)
   - TODO: Analytics export to CSV/JSON
   - TODO: Filter preset library

### Performance Improvements

- Materialized views for common queries
- Background aggregation worker
- Redis cache for analytics data
- WebSocket push for real-time updates

---

**Last Updated:** January 7, 2026  
**Version:** 0.1.108  
**Author:** ScreenerBot Development Team
