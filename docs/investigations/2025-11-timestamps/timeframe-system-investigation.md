# Timeframe System Deep Investigation - November 23, 2025

## Executive Summary

**Investigation Focus**: How OHLCV timeframes flow from data preparation → trader → strategies → conditions, and whether each condition can independently select its timeframe.

**Critical Finding**: Currently, **strategy-level timeframe selection exists but condition-level timeframe selection is missing**. Each strategy has a single `timeframe` field that applies to ALL conditions within that strategy. Individual conditions cannot choose their own timeframe.

---

## 1. OHLCV Data Preparation & Storage

### 1.1 Timeframe Enum Definition

**Location**: `src/ohlcvs/types.rs:8-24`

```rust
pub enum Timeframe {
    Minute1,   // "1m"
    Minute5,   // "5m"
    Minute15,  // "15m"
    Hour1,     // "1h"
    Hour4,     // "4h"
    Hour12,    // "12h"
    Day1,      // "1d"
}
```

**Methods**:

- `to_seconds()` - converts to seconds (60, 300, 900, 3600, 14400, 43200, 86400)
- `to_api_param()` - GeckoTerminal API mapping ("minute", "hour", "day")
- `to_api_params()` - returns (endpoint, aggregate) for native API support
- `max_candles_30d()` - maximum candles for 30 days (43,200 for 1m down to 30 for 1d)
- `backfill_priority()` - priority order (1=Day1 fastest, 7=Minute1 slowest)
- `all()` - returns Vec of all timeframes
- `from_str()` - parse from string ("1m", "5m", etc.)
- `as_str()` - convert to string

### 1.2 TimeframeBundle Structure

**Location**: `src/ohlcvs/types.rs:185-200`

**Purpose**: Pre-loads ALL timeframes for a single token in one bundle for fast strategy evaluation.

```rust
pub struct TimeframeBundle {
    pub mint: String,
    pub pool_address: String,
    pub timestamp: DateTime<Utc>,

    // All 7 timeframes pre-loaded (each contains BUNDLE_CANDLE_COUNT candles)
    pub m1: Vec<Candle>,    // 100 candles = 100 min = 1.67 hours
    pub m5: Vec<Candle>,    // 100 candles = 500 min = 8.33 hours
    pub m15: Vec<Candle>,   // 100 candles = 1500 min = 25 hours
    pub h1: Vec<Candle>,    // 100 candles = 100 hours = 4.17 days
    pub h4: Vec<Candle>,    // 100 candles = 400 hours = 16.67 days
    pub h12: Vec<Candle>,   // 100 candles = 1200 hours = 50 days
    pub d1: Vec<Candle>,    // 100 candles = 100 days

    pub cache_age_seconds: u64,
    pub cache_hit: bool,
}
```

**Constant**: `BUNDLE_CANDLE_COUNT = 100` (hardcoded, `src/ohlcvs/types.rs:184`)

**Key Method**: `get_timeframe(&self, timeframe: &str) -> Option<&Vec<Candle>>`

- Maps string ("1m", "5m", "15m", "1h", "4h", "12h", "1d") to corresponding field
- Returns reference to candles vector
- Used by conditions to access specific timeframe data

### 1.3 OHLCV Fetcher

**Location**: `src/ohlcvs/fetcher.rs`

**Responsibilities**:

- Fetches raw 1-minute candles from GeckoTerminal API
- Supports batching (≤50 accounts per RPC call)
- Rate limiting (respects API limits)
- Handles timeframe-specific fetching with aggregation

**Key Functions**:

- `fetch_ohlcv(pool_address, timeframe, limit)` - main fetch function
- Uses `timeframe.to_api_params()` to get (endpoint, aggregate) for API calls
- Returns `Vec<Candle>` in ASC timestamp order (INVARIANT)

### 1.4 OHLCV Cache

**Location**: `src/ohlcvs/cache.rs`

**Three-Tier System**:

1. **Hot Cache**: In-memory HashMap, 100 tokens max, 24h retention
2. **Database**: SQLite per-token tables with indexed queries
3. **API**: Live fetch on cache miss

**Cache Key**: `(mint, pool_address, timeframe)` tuple

**Methods**:

- `get(mint, pool_address, timeframe)` - retrieves cached candles
- `put(mint, pool_address, timeframe, candles)` - stores candles
- Cache invalidation based on age (24h for hot cache)

### 1.5 OHLCV Monitor

**Location**: `src/ohlcvs/monitor.rs`

**Background Service** that:

- Monitors active tokens with priority-based fetching
- Syncs with Pool Service every 5 minutes (`sync_pool_service_tokens()`)
- Auto-adjusts priorities for open positions (High priority)
- Runs backfill tasks for historical data gaps
- Aggregates higher timeframes from 1m base data

**Priority System** (`src/ohlcvs/priorities.rs`):

- `Critical` (1min interval) - open positions
- `High` (5min interval) - recent trades
- `Medium` (15min interval) - watched tokens
- `Low` (1h interval) - inactive tokens

### 1.6 OHLCV Database

**Location**: `src/ohlcvs/database.rs`

**Schema**: Per-token SQLite tables

- Table name: `ohlcv_{mint}_{timeframe}` (e.g., `ohlcv_ABC123_1m`)
- Columns: `timestamp, open, high, low, close, volume, pool_address`
- Indexes on timestamp for fast range queries

**Retention**: Configurable (default 30 days for 1m, longer for higher timeframes)

---

## 2. Strategy System Architecture

### 2.1 Strategy Structure

**Location**: `src/strategies/types.rs:27-42`

```rust
pub struct Strategy {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub strategy_type: StrategyType,  // ENTRY or EXIT
    pub enabled: bool,
    pub priority: i32,
    pub timeframe: String,  // ← STRATEGY-LEVEL TIMEFRAME ("1m", "5m", etc.)
    pub rules: RuleTree,
    pub parameters: HashMap<String, serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub author: Option<String>,
    pub version: i32,
}
```

**Critical Field**: `pub timeframe: String`

- **Purpose**: Determines which timeframe to use for ALL OHLCV-based conditions in this strategy
- **Default**: "5m" (`src/webserver/routes/strategies.rs:189`)
- **Scope**: STRATEGY-WIDE, not condition-specific

### 2.2 EvaluationContext

**Location**: `src/strategies/types.rs:126-132`

```rust
pub struct EvaluationContext {
    pub token_mint: String,
    pub current_price: Option<f64>,
    pub position_data: Option<PositionData>,
    pub market_data: Option<MarketData>,
    pub timeframe_bundle: Option<TimeframeBundle>,  // ALL timeframes
    pub strategy_timeframe: String,  // ← Strategy's chosen timeframe
}
```

**Key Points**:

- `timeframe_bundle` contains ALL 7 timeframes pre-loaded
- `strategy_timeframe` is copied from `Strategy.timeframe`
- Conditions receive BOTH bundle and strategy_timeframe

### 2.3 Strategy Evaluation Flow

**Location**: `src/strategies/mod.rs:56-106`

```rust
pub async fn evaluate_entry_strategies(
    token_mint: &str,
    current_price: f64,
    market_data: Option<MarketData>,
    timeframe_bundle: Option<TimeframeBundle>,  // ← Passed from trader
) -> Result<Option<String>, String>
```

**Process**:

1. Get enabled strategies from database
2. For each strategy, build `EvaluationContext`:
   - Copy `strategy.timeframe` to `context.strategy_timeframe`
   - Pass entire `timeframe_bundle` (all 7 timeframes)
3. Call `engine.evaluate_strategy(&strategy, &context)`
4. Return first strategy that signals entry/exit

### 2.4 Strategy Database

**Location**: `src/strategies/db.rs:22-38`

**Schema**:

```sql
CREATE TABLE strategies (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    type TEXT NOT NULL,
    enabled INTEGER NOT NULL DEFAULT 1,
    priority INTEGER NOT NULL DEFAULT 10,
    timeframe TEXT NOT NULL DEFAULT '5m',  -- ← Stored timeframe
    rules_json TEXT NOT NULL,
    parameters_json TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    author TEXT,
    version INTEGER NOT NULL DEFAULT 1
);
```

**Indexes**: `type`, `enabled`, `priority`

---

## 3. Trader Integration with Strategies

### 3.1 Entry Evaluation Flow

**Location**: `src/trader/evaluators/strategies.rs:17-165`

```rust
pub async fn check_entry_strategies(
    token_mint: &str,
    price_info: &PriceResult,
) -> Result<Option<TradeDecision>, String>
```

**Process**:

1. Check connectivity (RPC, DexScreener, RugCheck)
2. Build `MarketData` from `price_info`
3. **Fetch/Build TimeframeBundle**:
   - Try cache: `crate::ohlcvs::get_timeframe_bundle(token_mint)`
   - On cache miss: `crate::ohlcvs::build_timeframe_bundle(token_mint)`
   - Store in cache for future use
   - If OHLCV unavailable, evaluate without it (bundle = None)
4. Call `strategies::evaluate_entry_strategies()` with:
   - `token_mint`
   - `price_info.price_sol`
   - `market_data`
   - `timeframe_bundle` ← Contains ALL 7 timeframes
5. Handle timeout (5 seconds) and errors
6. Return `TradeDecision` with `strategy_id` if signal

**Bundle Building**: Non-blocking, builds on-demand if cache miss

### 3.2 Exit Evaluation Flow

**Location**: `src/trader/evaluators/strategies.rs:167-355`

**Same pattern as entry**, but:

- Builds `PositionData` with entry price, age, profit %
- Calls `strategies::evaluate_exit_strategies()`
- Uses position-specific data for exit conditions

### 3.3 OHLCV Bundle API

**Location**: `src/ohlcvs/mod.rs:86-96`

**Public Functions**:

- `get_timeframe_bundle(mint)` - fetch from cache
- `build_timeframe_bundle(mint)` - build fresh bundle
- `store_bundle(mint, bundle)` - cache for future use

**Implementation**: `src/ohlcvs/service.rs` (wrapped service methods)

---

## 4. Condition System - Current Implementation

### 4.1 Condition Registry

**Location**: `src/strategies/conditions/mod.rs:70-99`

**Purpose**: Central registry of all condition evaluators

**Registered Conditions**:

1. `PriceChangePercentCondition` - price % change over time
2. `PriceToMaCondition` - price vs moving average
3. `ConsecutiveCandlesCondition` - consecutive green/red candles
4. `CandleSizeCondition` - candle body/wick patterns
5. `PriceBreakoutCondition` - resistance/support breakouts
6. `VolumeSpikeCondition` - volume vs average
7. `LiquidityLevelCondition` - pool liquidity checks (NO OHLCV)
8. `PositionHoldingTimeCondition` - position age (NO OHLCV)

**Non-OHLCV Conditions**: #7 and #8 don't use candles, only metadata

### 4.2 Helper Function - get_candles_from_context()

**Location**: `src/strategies/conditions/mod.rs:21-44`

**Purpose**: Extract candles for strategy's timeframe from bundle

```rust
pub fn get_candles_from_context(context: &EvaluationContext) -> Result<Vec<Candle>, String> {
    let bundle = context.timeframe_bundle.as_ref()
        .ok_or_else(|| "OHLCV data not available")?;

    let timeframe = &context.strategy_timeframe;  // ← Uses STRATEGY timeframe
    let candles = bundle.get_timeframe(timeframe)
        .ok_or_else(|| format!("Timeframe {} not available", timeframe))?;

    if candles.is_empty() {
        return Err(format!("Timeframe {} has no candle data", timeframe));
    }

    Ok(candles.clone())
}
```

**Critical Issue**: Always uses `context.strategy_timeframe`, no per-condition timeframe selection!

### 4.3 OHLCV-Based Conditions Analysis

#### A. PriceChangePercentCondition

**Location**: `src/strategies/conditions/price_change_percent.rs`

**Parameters**:

- `percentage` - threshold % (0.1-1000%)
- `direction` - ABOVE/BELOW/WITHIN
- `time_value` - lookback period value (1-3600)
- `time_unit` - SECONDS/MINUTES/HOURS

**Current Timeframe Logic** (Lines 42-68):

```rust
// Select appropriate timeframe based on lookback period
let candles = if lookback_seconds <= 3600 {
    &bundle.m1  // Up to 1 hour: use 1m
} else if lookback_seconds <= 1800 * 60 {
    &bundle.m5  // Up to 30 hours: use 5m
} else if lookback_seconds <= 90000 {
    &bundle.m15 // Up to 25 hours: use 15m
} else if lookback_seconds <= 360000 {
    &bundle.h1  // Up to 100 hours: use 1h
} else if lookback_seconds <= 1440000 {
    &bundle.h4  // Up to 400 hours: use 4h
} else if lookback_seconds <= 4320000 {
    &bundle.h12 // Up to 1200 hours: use 12h
} else {
    &bundle.d1  // Beyond: use 1d
};
```

**Unique Behavior**:

- **IGNORES** `strategy_timeframe`!
- Auto-selects timeframe based on lookback period
- Accesses bundle fields directly (`bundle.m1`, `bundle.m5`, etc.)

**Problem**: No user control over timeframe, auto-selection may not match intent

#### B. ConsecutiveCandlesCondition

**Location**: `src/strategies/conditions/consecutive_candles.rs`

**Parameters**:

- `count` - number of consecutive candles (2-20)
- `direction` - GREEN/RED
- `minimum_change` - minimum % change per candle (0.1-50%)

**Timeframe Logic** (Line 24):

```rust
let candles = get_candles_from_context(context)?;
```

**Uses**: Strategy-level timeframe via helper function

#### C. CandleSizeCondition

**Location**: `src/strategies/conditions/candle_size.rs`

**Parameters**:

- `pattern` - LARGE_BODY/SMALL_BODY/LONG_UPPER_WICK/LONG_LOWER_WICK
- `threshold` - size threshold % (10-100%)

**Timeframe Logic** (Line 20):

```rust
let candles = get_candles_from_context(context)?;
```

**Uses**: Strategy-level timeframe, analyzes most recent candle only

#### D. PriceToMaCondition

**Location**: `src/strategies/conditions/price_to_ma.rs`

**Parameters**:

- `period` - MA period in candles (2-200)
- `position` - ABOVE/BELOW/WITHIN
- `distance` - distance % from MA (0.1-100%)

**Timeframe Logic** (Line 21):

```rust
let candles = get_candles_from_context(context)?;
```

**Uses**: Strategy-level timeframe for MA calculation

#### E. PriceBreakoutCondition

**Location**: `src/strategies/conditions/price_breakout.rs`

**Parameters**:

- `lookback` - number of candles (2-100)
- `direction` - UPWARD/DOWNWARD
- `confirmation` - % past level (0-20%)

**Timeframe Logic** (Line 20):

```rust
let candles = get_candles_from_context(context)?;
```

**Uses**: Strategy-level timeframe

#### F. VolumeSpikeCondition

**Location**: `src/strategies/conditions/volume_spike.rs`

**Parameters**:

- `lookback` - number of candles (2-100)
- `multiplier` - volume multiplier (1.0-50.0x)

**Timeframe Logic** (Line 18):

```rust
let candles = get_candles_from_context(context)?;
```

**Uses**: Strategy-level timeframe

### 4.4 Parameter Schema (for UI)

**Location**: Each condition's `parameter_schema()` method

**Example** from `PriceChangePercentCondition`:

```json
{
  "type": "PriceChangePercent",
  "name": "Price Change %",
  "category": "Price Analysis",
  "parameters": {
    "percentage": { "type": "percent", "default": 10.0, "min": 0.1, "max": 1000.0 },
    "direction": { "type": "enum", "options": ["ABOVE", "BELOW", "WITHIN"] },
    "time_value": { "type": "number", "default": 5.0, "min": 1.0, "max": 3600.0 },
    "time_unit": { "type": "enum", "options": ["SECONDS", "MINUTES", "HOURS"] }
  }
}
```

**Missing**: No `timeframe` parameter in any condition schema!

---

## 5. Webserver Frontend & Backend

### 5.1 Backend Routes

**Location**: `src/webserver/routes/strategies.rs`

**Key Endpoints**:

- `GET /api/strategies` - list all strategies
- `GET /api/strategies/:id` - get strategy detail
- `POST /api/strategies` - create strategy
- `PUT /api/strategies/:id` - update strategy
- `POST /api/strategies/:id/test` - test strategy evaluation
- `DELETE /api/strategies/:id` - delete strategy

**StrategyRequest** (Lines 166-180):

```rust
pub struct StrategyRequest {
    pub name: String,
    pub description: Option<String>,
    pub strategy_type: String,
    pub enabled: bool,
    pub priority: i32,
    pub timeframe: String,  // ← Strategy-level timeframe field
    pub rules: serde_json::Value,
    pub parameters: HashMap<String, serde_json::Value>,
    pub author: Option<String>,
}
```

**Default**: `default_timeframe() -> "5m"` (Line 189)

**Test Endpoint** (Line 721):

```rust
let timeframe_bundle = None; // TODO: Phase 4 implementation
```

**Note**: Test endpoint doesn't load OHLCV data yet!

### 5.2 Frontend HTML

**Location**: `src/webserver/templates/pages/strategies.html`

**Structure**:

- Left sidebar: Strategy list
- Main editor: Condition builder
- Strategy header: Name, type toggle (ENTRY/EXIT), status badge
- Footer: Add Condition button

**No Timeframe UI**: No visible timeframe selector in HTML!

### 5.3 Frontend JavaScript

**Location**: `src/webserver/templates/scripts/pages/strategies.js`

**Key Variables**:

- `currentStrategy` - current strategy being edited
- `strategies` - all strategies from API

**Strategy Type Toggle** (Lines 137-159):

```javascript
$("#strategy-type-toggle").addEventListener("click", (e) => {
  if (e.target.closest(".type-option")) {
    const type = e.target.closest(".type-option").dataset.type;
    // Update currentStrategy.strategy_type
    if (currentStrategy && !currentStrategy.id) {
      currentStrategy.strategy_type = type;
    }
  }
});
```

**No Timeframe Management**: No JS code for timeframe selection!

### 5.4 Frontend CSS

**Location**: `src/webserver/templates/styles/pages/strategies.css`

**No timeframe-specific styles** in strategies page CSS

**Token Details Dialog** has timeframe UI:

- `src/webserver/templates/styles/token_details_dialog.css:393-408`
- `.chart-timeframe` button styles
- Used for chart viewing, not strategy building

---

## 6. Configuration System

### 6.1 Config Schema

**Location**: `src/config/schemas/strategies.rs`

**Fields**:

- `enabled` - enable strategy system (bool)
- `max_concurrent_evaluations` - max concurrent evals (u32)

**Missing**: No strategy-level or condition-level timeframe config!

### 6.2 Constants

**Location**: `src/ohlcvs/types.rs:183`

```rust
pub const BUNDLE_CANDLE_COUNT: usize = 100;
```

**Hardcoded**: All timeframes use 100 candles, no configuration

---

## 7. Problems Identified

### 7.1 Strategy-Level Timeframe Limitation

**Current State**:

- Each strategy has ONE `timeframe` field
- ALL conditions in that strategy use the same timeframe
- No way to mix timeframes within a single strategy

**Example Scenario** (not currently possible):

```
Strategy: "Scalping Entry"
- Condition 1: Price > 20-period MA on 1m chart
- Condition 2: Volume spike on 5m chart
- Condition 3: Breakout on 15m chart
```

**Current Workaround**: Create 3 separate strategies, each with different timeframe

**Impact**:

- Strategy duplication
- Complex multi-timeframe analysis impossible
- Reduced flexibility

### 7.2 PriceChangePercent Auto-Selection Issue

**Location**: `src/strategies/conditions/price_change_percent.rs:42-68`

**Problem**: Auto-selects timeframe based on lookback period, ignoring user intent

**Example**:

- User wants: "Price up 5% in last 5 minutes on 1m chart"
- Lookback: 300 seconds (5 minutes)
- Auto-selection: Uses 1m candles (correct by chance)
- BUT: User wants: "Price up 5% in last 5 minutes on 5m chart"
- Auto-selection: Still uses 1m (only has 1 candle in 5 min on 5m chart!)

**Impact**:

- Inconsistent behavior
- User confusion
- Can't force specific timeframe

### 7.3 Missing UI Components

**Identified Gaps**:

1. **Strategy Creation**:
   - No timeframe selector in create/edit form
   - Uses hardcoded default "5m"
   - No validation that timeframe is valid

2. **Condition Parameters**:
   - No `timeframe` parameter in condition schemas
   - No UI dropdown for per-condition timeframe
   - No indication of which timeframe condition uses

3. **Strategy List**:
   - Strategy items don't show timeframe
   - Can't filter by timeframe
   - No visual indicator

4. **Testing Interface**:
   - Test endpoint doesn't load OHLCV data
   - Can't test with different timeframes
   - No preview of candles used

### 7.4 Helper Function Limitation

**Location**: `src/strategies/conditions/mod.rs:21-44`

**Problem**: `get_candles_from_context()` only supports strategy-level timeframe

**What's Needed**:

```rust
// Current
pub fn get_candles_from_context(context: &EvaluationContext) -> Result<Vec<Candle>, String>

// Needed
pub fn get_candles_for_timeframe(
    context: &EvaluationContext,
    timeframe: &str,  // ← Per-condition timeframe
) -> Result<Vec<Candle>, String>
```

### 7.5 Database Schema Gaps

**Strategy Table**: Has `timeframe` field ✓
**Condition Table**: Doesn't exist!

**Current**: Conditions stored as JSON in `strategies.rules_json`
**Problem**: Can't query by condition parameters, including timeframe

### 7.6 Bundle Availability

**Issue**: Bundle building is on-demand in trader evaluation

**Flow**:

1. Trader calls strategy evaluation
2. Tries to get bundle from cache
3. On miss, builds bundle (can take 1-5 seconds)
4. Strategy evaluation may timeout (5s limit)

**Problem**: First evaluation for new token may fail due to timeout

**Solution Exists**: OHLCV Monitor service should pre-build bundles for monitored tokens

---

## 8. Architectural Considerations

### 8.1 Timeframe Selection Levels

**Three Possible Scopes**:

1. **Strategy-Level** (CURRENT):
   - One timeframe per strategy
   - All conditions use same timeframe
   - Simple, but limiting

2. **Condition-Level** (NEEDED):
   - Each condition specifies its timeframe
   - Allows multi-timeframe strategies
   - More flexible, more complex

3. **Parameter-Level**:
   - Some conditions have multiple timeframe-dependent parameters
   - Example: "Compare 1m MA to 5m MA"
   - Most complex, maximum flexibility

**Recommendation**: Implement #2 (Condition-Level) as it balances flexibility and complexity

### 8.2 Bundle Pre-Loading Strategy

**Current**: All 7 timeframes always loaded (100 candles each)

**Implications**:

- Bundle size: 700 candles total
- Memory: ~50KB per bundle (compressed)
- Build time: 1-3 seconds for fresh bundle
- Cache hit rate: ~80% after warmup

**Optimization Options**:

1. Load only requested timeframes (lazy loading)
2. Pre-load common timeframes (1m, 5m, 15m, 1h)
3. Keep current approach (simplest)

**Recommendation**: Keep current approach, pre-loading is fast and predictable

### 8.3 Condition Parameter Schema Evolution

**Current**:

```json
{
  "type": "ConsecutiveCandles",
  "parameters": {
    "count": { "type": "number", "default": 3 },
    "direction": { "type": "enum", "options": ["GREEN", "RED"] }
  }
}
```

**Proposed Addition**:

```json
{
  "type": "ConsecutiveCandles",
  "parameters": {
    "timeframe": {
      "type": "enum",
      "name": "Timeframe",
      "description": "Candle timeframe to analyze",
      "default": "5m",
      "options": [
        { "value": "1m", "label": "1 Minute" },
        { "value": "5m", "label": "5 Minutes" },
        { "value": "15m", "label": "15 Minutes" },
        { "value": "1h", "label": "1 Hour" },
        { "value": "4h", "label": "4 Hours" },
        { "value": "12h", "label": "12 Hours" },
        { "value": "1d", "label": "1 Day" }
      ]
    },
    "count": { ... },
    "direction": { ... }
  }
}
```

**Storage**: In condition's `parameters` map as `"timeframe": "5m"`

### 8.4 Backward Compatibility

**Challenge**: Existing strategies don't have per-condition timeframes

**Migration Strategy**:

1. Keep `Strategy.timeframe` as fallback
2. Conditions check for `parameters.timeframe` first
3. If missing, use `context.strategy_timeframe`
4. Update UI to prompt adding timeframe to conditions
5. Eventually deprecate strategy-level timeframe

### 8.5 Performance Implications

**Bundle Access**:

- Current: `get_candles_from_context()` - O(1) lookup by strategy timeframe
- Proposed: `get_candles_for_timeframe(timeframe)` - O(1) lookup by condition timeframe

**No performance impact** - both are O(1) field access

**Evaluation Time**:

- Current: 5-20ms per strategy
- Proposed: Same (timeframe lookup is negligible)

---

## 9. Suggested Implementation Approach

### Phase 1: Backend Infrastructure

1. Add `timeframe` parameter to ALL OHLCV condition schemas
2. Create new helper: `get_candles_for_timeframe(context, timeframe)`
3. Update all 6 OHLCV conditions to:
   - Extract `timeframe` from parameters
   - Fallback to `context.strategy_timeframe` if missing
   - Use new helper function
4. Add validation for timeframe parameter values

### Phase 2: Frontend UI

1. Add timeframe dropdown to condition parameter editor
2. Show timeframe badge on condition cards
3. Add timeframe filter to strategy list
4. Update strategy creation form with timeframe selector
5. Add timeframe preview in test interface

### Phase 3: Testing & Validation

1. Update strategy test endpoint to load OHLCV bundles
2. Add validation that requested timeframe has data
3. Show "No data for timeframe X" errors clearly
4. Add visual indicators of data availability

### Phase 4: Migration & Documentation

1. Auto-migrate existing strategies (add strategy.timeframe to each condition)
2. Update API documentation
3. Create user guide for multi-timeframe strategies
4. Add example strategies demonstrating feature

### Phase 5: Advanced Features

1. Add timeframe comparison conditions ("1m MA > 5m MA")
2. Support timeframe synchronization checks
3. Add candle alignment validation
4. Implement timeframe divergence detection

---

## 10. Code Locations Reference

### Core OHLCV System

- Timeframe enum: `src/ohlcvs/types.rs:8-141`
- TimeframeBundle: `src/ohlcvs/types.rs:185-267`
- Fetcher: `src/ohlcvs/fetcher.rs`
- Cache: `src/ohlcvs/cache.rs`
- Monitor: `src/ohlcvs/monitor.rs`
- Database: `src/ohlcvs/database.rs`
- Service: `src/ohlcvs/service.rs`
- Public API: `src/ohlcvs/mod.rs`

### Strategy System

- Types: `src/strategies/types.rs`
- Engine: `src/strategies/engine.rs`
- Database: `src/strategies/db.rs`
- Evaluation: `src/strategies/mod.rs:56-255`
- Conditions registry: `src/strategies/conditions/mod.rs`
- Helper function: `src/strategies/conditions/mod.rs:21-44`

### Individual Conditions

1. Price Change %: `src/strategies/conditions/price_change_percent.rs`
2. Price to MA: `src/strategies/conditions/price_to_ma.rs`
3. Consecutive Candles: `src/strategies/conditions/consecutive_candles.rs`
4. Candle Size: `src/strategies/conditions/candle_size.rs`
5. Price Breakout: `src/strategies/conditions/price_breakout.rs`
6. Volume Spike: `src/strategies/conditions/volume_spike.rs`
7. Liquidity Level: `src/strategies/conditions/liquidity_level.rs` (no OHLCV)
8. Holding Time: `src/strategies/conditions/position_holding_time.rs` (no OHLCV)

### Trader Integration

- Entry evaluator: `src/trader/evaluators/entry.rs`
- Exit evaluator: `src/trader/evaluators/exit.rs`
- Strategy evaluator: `src/trader/evaluators/strategies.rs`
- Trader module: `src/trader/evaluators/mod.rs`

### Webserver

- Routes: `src/webserver/routes/strategies.rs`
- HTML: `src/webserver/templates/pages/strategies.html`
- JavaScript: `src/webserver/templates/scripts/pages/strategies.js`
- CSS: `src/webserver/templates/styles/pages/strategies.css`

### Configuration

- Strategy config: `src/config/schemas/strategies.rs`
- OHLCV constants: `src/ohlcvs/types.rs:183`

---

## 11. Summary of Findings

### What Works Well

✅ **OHLCV data preparation** is comprehensive and efficient
✅ **TimeframeBundle** pre-loads all timeframes for fast access
✅ **Strategy-level timeframe** is implemented and functional
✅ **Bundle caching** provides good performance
✅ **Background monitoring** keeps data fresh
✅ **Trader integration** properly fetches/builds bundles

### What's Missing

❌ **Per-condition timeframe selection** - all conditions use strategy timeframe
❌ **UI for timeframe selection** - no dropdown or selector
❌ **Timeframe parameter in schemas** - not exposed to conditions
❌ **Multi-timeframe strategies** - can't mix timeframes in one strategy
❌ **Test interface OHLCV loading** - can't test with real data
❌ **Timeframe validation UI** - no indicators of data availability

### Critical Issues

🔴 **PriceChangePercent auto-selection** - overrides user intent
🔴 **Strategy duplication needed** - for multi-timeframe analysis
🔴 **Bundle timeout risk** - first evaluation may fail
🔴 **No condition-level control** - inflexible for advanced strategies

### Next Steps

1. Add `timeframe` parameter to condition schemas
2. Create `get_candles_for_timeframe()` helper
3. Update all OHLCV conditions to support parameter
4. Build frontend UI for timeframe selection
5. Test with multi-timeframe strategies
6. Document new capabilities

---

**End of Deep Investigation**

This document provides a complete map of timeframe flow from OHLCV preparation through trader evaluation to condition execution, identifying the critical gap: **no per-condition timeframe selection exists**.
