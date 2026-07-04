# Strategies Module — Architecture

> ScreenerBot Rule-Based Strategy Evaluation Engine — February 2026

---

## Table of Contents

1. [Overview](#1-overview)
2. [File Structure](#2-file-structure)
3. [Core Data Types](#3-core-data-types)
4. [Evaluation Engine](#4-evaluation-engine)
5. [Condition System](#5-condition-system)
6. [Built-in Conditions](#6-built-in-conditions)
7. [Database Schema](#7-database-schema)
8. [Web API](#8-web-api)
9. [Data Flow](#9-data-flow)
10. [Module Connections](#10-module-connections)
11. [Configuration](#11-configuration)

---

## 1. Overview

The Strategies module provides a composable, rule-based evaluation engine for trading entry and exit signals. Strategies are defined as recursive rule trees combining logical operators (AND/OR/NOT) with condition leaf nodes. The engine evaluates strategies against real-time market data, OHLCV candles, and position state.

**Key characteristics:**
- Recursive rule tree with short-circuit evaluation
- 8 built-in condition types (extensible via `ConditionEvaluator` trait)
- Per-evaluation caching with context fingerprinting
- Timeout protection (50ms default per strategy)
- Performance tracking in SQLite
- Entry and exit strategies evaluated independently

---

## 2. File Structure

```
src/strategies/
├── mod.rs                    # Public API, global engine lifecycle
├── types.rs                  # Strategy, RuleTree, Condition, Parameter, enums
├── engine.rs                 # StrategyEngine — evaluation, caching, validation
├── database.rs               # SQLite persistence (5 tables + schema_version, r2d2 pool)
└── conditions/
    ├── mod.rs                # ConditionEvaluator trait, ConditionRegistry, helpers
    ├── candle_size.rs        # CandleSize — body/wick pattern detection
    ├── consecutive_candles.rs # ConsecutiveCandles — momentum streak detection
    ├── liquidity_level.rs    # LiquidityLevel — pool safety threshold
    ├── position_holding_time.rs # PositionHoldingTime — time-based exit
    ├── price_breakout.rs     # PriceBreakout — support/resistance break
    ├── price_change_percent.rs # PriceChangePercent — historical price movement
    ├── price_to_ma.rs        # PriceToMA — moving average proximity
    └── volume_spike.rs       # VolumeSpike — trading activity detection
```

**13 files, ~2,889 lines**

---

## 3. Core Data Types

### Strategy (`types.rs`)

Top-level definition stored in database:

```rust
pub struct Strategy {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub strategy_type: StrategyType,        // Entry | Exit
    pub enabled: bool,
    pub priority: i32,                      // Lower = evaluated first
    pub timeframe: String,                  // "1m", "5m", "15m", "1h", "4h", "12h", "1d"
    pub rules: RuleTree,                    // Recursive condition tree
    pub parameters: HashMap<String, serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub author: Option<String>,
    pub version: i32,
}
```

### RuleTree (`types.rs`)

Recursive tree structure for composing conditions:

```rust
pub struct RuleTree {
    pub operator: Option<LogicalOperator>,  // AND | OR | NOT (branch nodes)
    pub conditions: Option<Vec<RuleTree>>,  // Children (branch nodes)
    pub condition: Option<Condition>,       // Leaf node
}
```

**Methods:** `leaf(condition)`, `branch(operator, children)`, `is_leaf()`, `is_branch()`

### Condition (`types.rs`)

```rust
pub struct Condition {
    pub condition_type: String,             // e.g., "PriceChangePercent"
    pub parameters: HashMap<String, Parameter>,
}
```

### Parameter (`types.rs`)

```rust
pub struct Parameter {
    pub value: serde_json::Value,
    pub default: serde_json::Value,
    pub constraints: Option<ParameterConstraints>,
}

pub struct ParameterConstraints {
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub options: Option<Vec<serde_json::Value>>,
    pub format: Option<String>,
}
```

### Enums

| Enum | Variants | Usage |
|------|----------|-------|
| `StrategyType` | `Entry`, `Exit` | Strategy classification |
| `LogicalOperator` | `And`, `Or`, `Not` | Rule tree branching |
| `RiskLevel` | `Low`, `Medium`, `High` | Template classification |

### Evaluation Types

```rust
pub struct EvaluationContext {
    pub token_mint: String,
    pub current_price: Option<f64>,
    pub position_data: Option<PositionData>,    // Exit strategies only
    pub market_data: Option<MarketData>,
    pub timeframe_bundle: Option<TimeframeBundle>,
    pub strategy_timeframe: String,
}

pub struct PositionData {
    pub entry_price: f64,
    pub entry_time: DateTime<Utc>,
    pub current_size_sol: f64,
    pub unrealized_profit_pct: Option<f64>,
    pub position_age_hours: f64,
}

pub struct MarketData {
    pub liquidity_sol: Option<f64>,
    pub volume_24h: Option<f64>,
    pub market_cap: Option<f64>,
    pub holder_count: Option<u32>,
    pub token_age_hours: Option<f64>,
}

pub struct EvaluationResult {
    pub strategy_id: String,
    pub result: bool,                   // true = trigger signal
    pub confidence: f64,
    pub execution_time_ms: u64,
    pub details: HashMap<String, serde_json::Value>,
}
```

---

## 4. Evaluation Engine

### Global State (`mod.rs`)

```rust
static STRATEGY_ENGINE: LazyLock<Arc<RwLock<Option<StrategyEngine>>>> = ...;
```

### Public API (`mod.rs`)

| Function | Purpose |
|----------|---------|
| `init_strategy_system(config)` | Initialize engine, DB, condition registry |
| `evaluate_entry_strategies(mint, price, market, bundle)` | Evaluate all enabled entry strategies, return first match |
| `evaluate_exit_strategies(mint, price, position, market, bundle)` | Evaluate all enabled exit strategies, return first match |
| `validate_strategy(strategy)` | Validate rule tree structure and condition parameters |
| `clear_evaluation_cache()` | Flush cached evaluations |
| `get_condition_schemas()` | Return JSON schemas for all registered conditions |

### StrategyEngine (`engine.rs`)

```rust
pub struct StrategyEngine {
    condition_registry: Arc<ConditionRegistry>,
    evaluation_cache: Arc<RwLock<HashMap<String, CachedEvaluation>>>,
    config: EngineConfig,
}

pub struct EngineConfig {
    pub evaluation_timeout_ms: u64,     // Default: 50ms
    pub cache_ttl_seconds: u64,         // Default: 5s
    pub max_concurrent_evaluations: usize, // Default: 10
}
```

### Evaluation Logic

1. Get enabled strategies ordered by priority (ascending)
2. For each strategy:
   - Build `EvaluationContext` from inputs
   - Compute cache key via `context_fingerprint()` (hash of token + price + market + position)
   - Check cache → return cached result if TTL valid
   - Evaluate rule tree recursively:
     - **Leaf node:** Look up `ConditionEvaluator` in registry → `evaluate(condition, context)`
     - **AND branch:** Short-circuit on first `false`
     - **OR branch:** Short-circuit on first `true`
     - **NOT branch:** Invert single child result
   - Cache result, record to `strategy_performance` table
3. Return first strategy that evaluates to `true`, or `None`

---

## 5. Condition System

### ConditionEvaluator Trait (`conditions/mod.rs`)

```rust
#[async_trait]
pub trait ConditionEvaluator: Send + Sync {
    fn condition_type(&self) -> &'static str;
    async fn evaluate(&self, condition: &Condition, context: &EvaluationContext) -> Result<bool, String>;
    fn validate(&self, condition: &Condition) -> Result<(), String>;
    fn parameter_schema(&self) -> serde_json::Value;
}
```

### ConditionRegistry (`conditions/mod.rs`)

```rust
pub struct ConditionRegistry {
    evaluators: HashMap<String, Box<dyn ConditionEvaluator>>,
}
```

**Methods:** `new()` (registers all 8 built-ins), `register()`, `get()`, `list_types()`, `get_all_schemas()`

### Helper Functions

| Function | Purpose |
|----------|---------|
| `get_candles_from_context(ctx)` | Extract candles from TimeframeBundle for strategy timeframe |
| `get_candles_for_timeframe(ctx, tf)` | Extract candles for specific or overridden timeframe |
| `get_param_f64(condition, name)` | Extract f64 parameter with error |
| `get_param_string(condition, name)` | Extract string parameter with error |
| `get_param_bool(condition, name)` | Extract bool parameter with error |
| `get_param_string_optional(condition, name)` | Extract optional string parameter |
| `validate_timeframe_param(condition)` | Validate timeframe parameter value |

---

## 6. Built-in Conditions

| # | Type | Parameters | Logic |
|---|------|-----------|-------|
| 1 | **CandleSize** | `pattern` (LARGE_BODY/SMALL_BODY/LONG_UPPER_WICK/LONG_LOWER_WICK), `threshold` (10-100%), `timeframe` | Candle body/wick ratio analysis |
| 2 | **ConsecutiveCandles** | `direction` (GREEN/RED), `count` (2-20), `minimum_change` (0.1-50%), `timeframe` | Count consecutive candles matching direction |
| 3 | **LiquidityLevel** | `threshold` (0-100,000 SOL), `comparison` (GT/GTE/LT/LTE) | Compare pool liquidity to threshold |
| 4 | **PositionHoldingTime** | `hours` (0-720), `comparison` (GT/LT/GTE/LTE) | Position age check (exit only) |
| 5 | **PriceBreakout** | `lookback` (2-100), `direction` (UPWARD/DOWNWARD), `confirmation` (0-20%), `timeframe` | Price above period high or below period low with confirmation |
| 6 | **PriceChangePercent** | `percentage` (0.1-1000%), `direction` (ABOVE/BELOW/WITHIN), `time_value`, `time_unit` (SEC/MIN/HR), `timeframe` | Historical price change comparison |
| 7 | **PriceToMA** | `period` (2-200), `position` (ABOVE/BELOW/WITHIN), `distance` (0.1-100%), `timeframe` | SMA proximity check |
| 8 | **VolumeSpike** | `lookback` (2-100), `multiplier` (1.0-50.0x), `timeframe` | Volume ratio vs average |

All conditions support optional `timeframe` parameter to override the strategy's default timeframe.

---

## 7. Database Schema

**Database:** `strategies.db` (SQLite, WAL mode, r2d2 pool, max 3 connections)

### Tables

| Table | Purpose | Key Columns |
|-------|---------|-------------|
| `strategies` | Strategy definitions | `id` PK, `name`, `type` (ENTRY/EXIT), `enabled`, `priority`, `timeframe`, `rules_json`, `parameters_json` |
| `strategy_performance` | Evaluation history | `strategy_id` FK, `result` (0/1), `execution_time_ms`, `token_mint`, `execution_timestamp` |
| `strategy_assignments` | Position → strategy mapping | `position_id` + `strategy_id` composite PK |
| `strategy_templates` | Reusable templates | `id` PK, `category`, `risk_level` (LOW/MEDIUM/HIGH), `rules_json` |
| `strategy_backtests` | Backtest results | `strategy_id` FK, `total_trades`, `win_trades`, `total_profit_sol` |
| `schema_version` | Migration tracking | `version` |

### Key DB Functions

| Function | Purpose |
|----------|---------|
| `init_strategies_db()` | Create tables and indices |
| `insert_strategy(strategy)` | Persist new strategy |
| `update_strategy(strategy)` | Update existing strategy |
| `delete_strategy(id)` | Remove strategy |
| `get_strategy(id)` | Fetch single strategy |
| `get_all_strategies()` | Fetch all strategies |
| `get_enabled_strategies(type)` | Fetch enabled strategies by type |
| `has_enabled_strategies(type)` | Check if any enabled strategies exist |
| `record_evaluation(result, mint)` | Log evaluation to performance table |
| `get_strategy_performance(id)` | Get aggregate performance stats |
| `assign_strategy_to_position(pos_id, strat_id)` | Link position to entry strategy |
| `get_position_strategies(pos_id)` | Get strategies linked to position |

---

## 8. Web API

Strategy CRUD is exposed under `src/webserver/routes/strategies/`:

| Route | Purpose |
|-------|---------|
| `GET /api/strategies` | List all strategies (`items`, `total`, `timestamp`) |
| `GET /api/strategies?type=ENTRY` | List entry strategies, filtered in the backend |
| `GET /api/strategies?type=EXIT` | List exit strategies, filtered in the backend |
| `GET /api/strategies?enabled=true` | List enabled strategies |
| `GET /api/strategies/:id` | Fetch full strategy detail |
| `POST /api/strategies` | Create a strategy |
| `PUT /api/strategies/:id` | Replace a full strategy definition |
| `PATCH /api/strategies/:id/enabled` | Toggle only enabled state; backend reads the full record, updates SQLite, increments version, and clears evaluation cache |
| `DELETE /api/strategies/:id` | Delete a strategy |
| `POST /api/strategies/:id/validate` | Validate a saved strategy |
| `POST /api/strategies/validate` | Validate an unsaved strategy payload |
| `POST /api/strategies/:id/test` | Test a strategy |

Dashboard Strategy Control must use backend-filtered list calls and the dedicated enabled-state endpoint. It must not reconstruct full strategy update payloads in the browser for a toggle.

---

## 9. Data Flow

### Entry Signal Flow

```
Trader Module (every 3s)
  └─ evaluate_entry_strategies(mint, price, market_data, timeframe_bundle)
       │
       ├─ get_enabled_strategies(StrategyType::Entry)  [from DB, ordered by priority]
       │
       └─ For each strategy (priority order):
            ├─ Build EvaluationContext
            │   ├─ token_mint, current_price
            │   ├─ market_data (liquidity, volume, market_cap, holders)
            │   ├─ timeframe_bundle (OHLCV candles for all timeframes)
            │   └─ strategy_timeframe (from Strategy.timeframe)
            │
            ├─ Check cache (context fingerprint hash)
            │   └─ Return cached if TTL < 5s
            │
            └─ evaluate_rule_tree(strategy.rules, context)
                 ├─ LEAF: registry.get(condition_type).evaluate(condition, context)
                 │        └─ get_candles_for_timeframe() → apply condition logic
                 │
                 └─ BRANCH: apply AND/OR/NOT with short-circuit
                      ├─ AND: false on first false
                      ├─ OR: true on first true
                      └─ NOT: invert child

Result: Some(strategy_id) → Trigger buy | None → Continue scanning
```

### Exit Signal Flow

Same as entry, but:
- Uses `StrategyType::Exit`
- `EvaluationContext` includes `PositionData` (entry_price, position_age, unrealized PnL)
- Conditions like `PositionHoldingTime` use position data

---

## 10. Module Connections

```
strategies/
├── ohlcvs/        ← TimeframeBundle, Candle (candle data for conditions)
├── trader/        ← STRATEGY_CACHE_MAX_ENTRIES constant; trader calls evaluate_*()
├── positions/     ← PositionData for exit evaluation; strategy_assignments table
└── config/        ← EngineConfig parameters
```

| Connection | Direction | What |
|-----------|-----------|------|
| trader → strategies | Caller | `evaluate_entry_strategies()`, `evaluate_exit_strategies()` |
| strategies → ohlcvs | Data | `TimeframeBundle.get_timeframe()` for candle data |
| strategies → positions DB | Write | `assign_strategy_to_position()` after entry |
| trader → strategies | Constant | `STRATEGY_CACHE_MAX_ENTRIES` for cache eviction |

---

## 11. Configuration

### Engine Constants

| Constant | Value | Purpose |
|----------|-------|---------|
| Evaluation timeout | 50ms | Max time per strategy evaluation |
| Cache TTL | 5s | Evaluation cache lifetime |
| Max concurrent evaluations | 10 | Parallelism limit |
| DB pool size | 3 | SQLite connection pool |
| Schema version | 1 | Migration tracking |
| Valid timeframes | 1m, 5m, 15m, 1h, 4h, 12h, 1d | Supported timeframe strings |
