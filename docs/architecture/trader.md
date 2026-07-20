# Trader Module — Architecture

> ScreenerBot Trading Engine — Entry/Exit Monitoring, Evaluation & Execution — February 2026

---

## Table of Contents

1. [Overview](#1-overview)
2. [File Structure](#2-file-structure)
3. [Core Types](#3-core-types)
4. [Service Lifecycle](#4-service-lifecycle)
5. [Entry Monitor](#5-entry-monitor)
6. [Exit Monitor](#6-exit-monitor)
7. [Evaluators](#7-evaluators)
8. [Executors](#8-executors)
9. [Safety System](#9-safety-system)
10. [Manual Trading API](#10-manual-trading-api)
11. [AI Analysis](#11-ai-analysis)
12. [Action Tracking](#12-action-tracking)
13. [Configuration](#13-configuration)
14. [Module Connections](#14-module-connections)

---

## 1. Overview

The Trader module is the trading orchestration core of ScreenerBot. It runs two independent monitoring loops — entry monitor (new opportunities) and exit monitor (open positions) — supported by evaluators, executors, safety guards, and manual trading endpoints.

**Key characteristics:**
- Dual-loop architecture: entry (3s) + exit (5s) independent cycles
- Priority-based exit execution (Emergency > High > Normal)
- 9-step entry evaluation pipeline with safety checks
- Automatic DCA evaluation during exit monitoring
- AI-powered entry/exit analysis (optional)
- Full action tracking for dashboard visibility
- Manual buy/sell/add/force-buy/force-sell API

---

## 2. File Structure

```
src/trader/
├── mod.rs                # Module orchestrator, public API, init
├── types.rs              # TradeDecision, TradeAction, TradeReason, TradePriority, TradeResult
├── constants.rs          # Monitor intervals, timeouts, thresholds
├── config.rs             # Configuration accessors (max positions, trade size, DCA, exits)
├── service.rs            # TraderService (Service trait impl, priority 150)
├── controller.rs         # Start/stop trader, TraderControlError
├── actions.rs            # Action tracking structs for dashboard
├── ai_analysis.rs        # AI-powered entry/exit recommendations
├── monitors/
│   ├── mod.rs            # start_automated_trading() → spawns entry + exit
│   ├── entry.rs          # Entry monitor loop (3s), reservation system
│   └── exit.rs           # Exit monitor loop (5s), priority sorting
├── evaluators/
│   ├── mod.rs            # Evaluator exports
│   ├── entry.rs          # 9-step entry pipeline
│   ├── exit.rs           # Priority-ordered exit checks
│   ├── dca.rs            # DCA opportunity evaluation
│   ├── trailing_stop.rs  # Trailing stop loss logic
│   ├── roi.rs            # ROI target exit
│   ├── stop_loss.rs      # Stop loss exit
│   ├── time_override.rs  # Time-based forced exit
│   └── risk.rs           # Emergency risk management (>90% loss)
├── executors/
│   ├── mod.rs            # execute_trade() dispatcher
│   ├── buy.rs            # Buy execution (open position)
│   └── sell.rs           # Sell execution (close/partial)
├── safety/
│   ├── mod.rs            # Safety system init
│   ├── checks.rs         # Pre-trade safety validation
│   ├── cooldown.rs       # Re-entry cooldown tracking
│   ├── loss_limit.rs     # Rolling loss limit protection
│   └── blacklist.rs      # Token blacklist enforcement
└── manual/
    ├── mod.rs            # Manual trading exports
    ├── api.rs            # manual_buy, manual_sell, manual_add
    ├── force.rs          # force_buy, force_sell (bypass safety)
    └── tracking.rs       # Manual trade history
```

**33 files, ~5,596 lines**

---

## 3. Core Types

### TradeDecision (`types.rs`)

```rust
pub struct TradeDecision {
    pub position_id: Option<String>,
    pub mint: String,
    pub action: TradeAction,
    pub reason: TradeReason,
    pub strategy_id: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub priority: TradePriority,
    pub price_sol: Option<f64>,
    pub size_sol: Option<f64>,       // Trade size or exit percentage
}
```

### TradeAction

| Variant | Purpose |
|---------|---------|
| `Buy` | Open new position |
| `Sell` | Close or partial exit |
| `DCA` | Add to existing position |

### TradeReason

**Entry reasons:** `StrategySignal`, `ManualEntry`, `ForceBuy`, `DCAScheduled`

**Exit reasons:** `TakeProfit`, `StopLoss`, `TrailingStop`, `TimeOverride`, `StrategyExit`, `AiExit`, `ManualExit`, `RiskManagement`, `Blacklisted`, `ForceSell`

### TradePriority

| Priority | When Used |
|----------|-----------|
| `Emergency` | Stop loss trigger, blacklist, >90% loss |
| `High` | AI exit (urgent), manual trades |
| `Normal` | ROI target, time override, strategy exit |
| `Low` | Delayed operations |

### TradeResult (`types.rs`)

```rust
pub struct TradeResult {
    pub decision: TradeDecision,
    pub success: bool,
    pub tx_signature: Option<String>,
    pub executed_price_sol: Option<f64>,
    pub executed_size_sol: Option<f64>,
    pub error: Option<String>,
    pub position_id: Option<String>,
    pub execution_timestamp: DateTime<Utc>,
    pub retry_count: u32,
}
```

---

## 4. Service Lifecycle

### TraderService (`service.rs`)

Implements the `Service` trait:

| Property | Value |
|----------|-------|
| Name | `"trader"` |
| Priority | `150` |
| Dependencies | `positions`, `pool_*`, `tokens`, `filtering` |

Fresh configurations default `trader.enabled` to `false`; starting in either preview or full mode
never opts the user into automated execution. The dashboard must explicitly enable the master
switch before either monitor can trade.

### Startup Sequence

```
ServiceManager::start_all()
  └─ TraderService::start()
       ├─ init_trader_system()
       │   ├─ executors::init_execution_system()
       │   └─ safety::init_safety_system()
       │       └─ loss_limit::initialize_from_history()  // Recover from DB
       ├─ Spawn monitors::start_automated_trading()
       │   ├─ Spawn entry monitor (if enabled)
       │   └─ Spawn exit monitor (if enabled)
       └─ Return task handle
```

### Shutdown

```
Shutdown signal received
  ├─ Set shutdown watch → true
  ├─ Entry monitor breaks loop
  ├─ Exit monitor breaks loop
  └─ Wait 2 seconds for graceful termination
```

### Controller (`controller.rs`)

| Function | Purpose |
|----------|---------|
| `is_trader_running()` | Check config `trader.enabled` |
| `start_trader()` | Enable trader via config update |
| `stop_trader_gracefully()` | Disable trader, send shutdown |

---

## 5. Entry Monitor

**File:** `monitors/entry.rs`
**Interval:** 3 seconds (`ENTRY_MONITOR_INTERVAL_SECS`)

### Loop

```
Every 3 seconds:
  1. Get available tokens from pools module
  2. For each token:
       ├─ Try reserve (ENTRY_CYCLE_RESERVATIONS HashMap)
       │   └─ Skip if already reserved (TTL: 120s)
       ├─ Get current price from pools
       └─ Spawn concurrent task (limited by semaphore):
            ├─ evaluate_entry_for_token(mint, price)
            ├─ If signal → execute_trade(decision)
            └─ Clear reservation after attempt
  3. Wait for next cycle
```

### Reservation System

- **Static:** `ENTRY_CYCLE_RESERVATIONS: LazyLock<DashMap<String, Instant>>`
- **TTL:** 120 seconds (`ENTRY_RESERVATION_TIMEOUT_SECS`)
- **Purpose:** Prevent duplicate concurrent evaluations for same token
- **Cleanup:** Expired entries removed on check

---

## 6. Exit Monitor

**File:** `monitors/exit.rs`
**Interval:** 5 seconds (`POSITION_MONITOR_INTERVAL_SECS`)

### Loop

```
Every 5 seconds:
  1. Get all open positions
  2. Spawn concurrent evaluation tasks (sell_concurrency semaphore):
       └─ evaluate_exit_for_position(position) → Option<TradeDecision>
  3. Collect all evaluations
  4. Sort by priority: Emergency > High > Normal
  5. Execute sequentially in priority order
  6. Process DCA opportunities (separate pass)
  7. Wait for next cycle
```

### Priority-Based Execution

Exit decisions are sorted by `TradePriority` before execution. Emergency exits (blacklist, >90% loss) always execute first, ensuring critical risk management isn't blocked by slower normal exits.

---

## 7. Evaluators

### Entry Evaluator (`evaluators/entry.rs`)

```rust
pub async fn evaluate_entry_for_token(
    token_mint: &str,
    price_info: &PriceResult,
) -> Result<Option<TradeDecision>, String>
```

**9-step pipeline (all must pass):**

| Step | Check | Failure |
|------|-------|---------|
| 1 | Force stop flag | Skip silently |
| 2 | Connectivity (internet + RPC) | Return Err |
| 3 | Blacklist check | Skip token |
| 4 | Re-entry cooldown | Skip token |
| 5 | Max positions limit | Skip (capacity guard) |
| 6 | Loss limit check | Skip all entries |
| 7 | Get full token data | Skip on failure |
| 8 | AI entry analysis (if enabled) | Skip if AI rejects |
| 9 | Strategy evaluation | Signal if strategy matches |

### Exit Evaluator (`evaluators/exit.rs`)

```rust
pub async fn evaluate_exit_for_position(
    position: &Position,
) -> Result<Option<TradeDecision>, String>
```

**Priority-ordered checks (first match wins):**

| Priority | Check | Reason |
|----------|-------|--------|
| Emergency | Blacklisted token | `Blacklisted` |
| Emergency | >90% loss (`EMERGENCY_LOSS_THRESHOLD_PCT`) | `RiskManagement` |
| High | AI exit (Immediate urgency) | `AiExit` |
| High | Stop loss triggered | `StopLoss` |
| High | Trailing stop triggered | `TrailingStop` |
| Normal | ROI target reached | `TakeProfit` |
| Normal | Time override expired | `TimeOverride` |
| Normal | Strategy exit signal | `StrategyExit` |

### Specialized Evaluators

| File | Function | Logic |
|------|----------|-------|
| `trailing_stop.rs` | `check_trailing_stop(position)` | Activates at `activation_pct`, triggers at `distance_pct` from `price_highest` |
| `roi.rs` | `check_roi_target(position)` | Triggers when unrealized PnL ≥ `target_profit_pct` |
| `stop_loss.rs` | `check_stop_loss(position)` | Triggers when loss ≥ `threshold_pct` after `min_hold_seconds` |
| `time_override.rs` | `check_time_override(position)` | Triggers when age > `duration_seconds` AND loss > `loss_threshold_pct` |
| `dca.rs` | `evaluate_dca(position)` | Triggers when price drop ≥ `threshold_pct`, count < max, cooldown elapsed |
| `risk.rs` | `check_risk_limits(position)` | Emergency exit at >90% loss |

---

## 8. Executors

### Trade Dispatcher (`executors/mod.rs`)

```rust
pub async fn execute_trade(decision: TradeDecision) -> TradeResult
```

Dispatches to `execute_buy()` or `execute_sell()` based on `TradeAction`.

### Buy Executor (`executors/buy.rs`)

```
execute_buy(decision):
  1. Build QuoteRequest (input: SOL, output: token)
  2. get_best_quote_for_opening()     // With no-route blacklisting
  3. Create position via positions module
  4. execute_swap_with_fallback()      // With retry chain
  5. Update position with tx signature
  6. Return TradeResult
```

### Sell Executor (`executors/sell.rs`)

```
execute_sell(decision):
  1. Determine full vs partial exit
  2. Build QuoteRequest (input: token, output: SOL)
  3. get_best_quote()
  4. If partial → positions::partial_close_position()
     If full → positions::close_position_direct()
  5. execute_swap_with_fallback()
  6. Update position state
  7. Return TradeResult
```

---

## 9. Safety System

### Pre-Trade Checks (`safety/checks.rs`)

Called before any automated entry:

| Check | Purpose |
|-------|---------|
| Blacklist | Token not blacklisted |
| Cooldown | Re-entry cooldown not active for this token |
| Position limit | Under max open positions |
| Loss limit | Under rolling loss threshold |

### Cooldown Tracking (`safety/cooldown.rs`)

Prevents rapid re-entry to recently exited tokens. Cooldown period configurable.

### Loss Limit (`safety/loss_limit.rs`)

Rolling window loss protection:
- Tracks cumulative losses over `loss_limit_period_hours`
- Pauses entries when total loss exceeds `loss_limit_sol`
- `initialize_from_history()` recovers state from DB on startup
- Optional auto-resume when window rolls over

### Blacklist Enforcement (`safety/blacklist.rs`)

Tokens blacklisted via API or no-route detection are blocked from entry.

---

## 10. Manual Trading API

**Files:** `manual/api.rs`, `manual/force.rs`, `manual/tracking.rs`

| Function | Safety | Priority | Purpose |
|----------|--------|----------|---------|
| `manual_buy(mint, size_sol)` | Full checks | High | Manual entry |
| `manual_sell(mint, percentage)` | Full checks | High | Manual exit (full/partial) |
| `manual_add(mint, size_sol)` | Full checks | Normal | Manual DCA |
| `force_buy(mint, size_sol)` | **Bypassed** | High | Force entry, ignores all safety |
| `force_sell(mint, percentage)` | **Bypassed** | Emergency | Force exit, ignores partial config |

**Force operations** bypass position limits, blacklist, cooldown, and loss limits. Logged with "FORCE" marker.

**Trade history** maintained in memory (limit: `MANUAL_TRADE_HISTORY_LIMIT = 1000`).

---

## 11. AI Analysis

**File:** `ai_analysis.rs`

### Entry Analysis

```rust
pub async fn analyze_entry(token: &Token) -> Option<EntryAnalysisResult>

pub struct EntryAnalysisResult {
    pub should_enter: bool,
    pub confidence: u8,             // 0-100%
    pub reasoning: String,
    pub suggested_amount: Option<f64>,
    pub provider: String,
}
```

If enabled, AI rejection blocks entry (high filter).

### Exit Analysis

```rust
pub async fn analyze_exit(position: &Position, token: &Token) -> Option<ExitAnalysisResult>

pub struct ExitAnalysisResult {
    pub action: ExitAction,         // Hold | Exit | PartialExit
    pub confidence: u8,
    pub reasoning: String,
    pub suggested_percentage: Option<u8>,
    pub urgency: ExitUrgency,       // Low | Normal | High | Immediate
    pub provider: String,
}
```

**Urgency → Priority mapping:**
- `Immediate` → `Emergency`
- `High` → `High`
- `Normal`/`Low` → `Normal`

---

## 12. Action Tracking

**File:** `actions.rs`

All trades (manual and automated) create action objects streamed to the dashboard via SSE.

### Action Types

| Struct | Used By |
|--------|---------|
| `ManualBuyAction` | `manual_buy()` |
| `ManualSellAction` | `manual_sell()` |
| `ManualAddAction` | `manual_add()` |
| `AutoOpenAction` | Entry monitor |
| `AutoCloseAction` | Exit monitor |
| `AutoDcaAction` | DCA evaluator |

### Action Steps (each has start/complete/fail)

1. Validation → 2. Quote → 3. Swap → 4. Verify

### Preflight Failure Helpers

```rust
pub async fn create_failed_buy_action(mint, error)
pub async fn create_failed_sell_action(mint, error)
pub async fn create_failed_add_action(mint, error)
```

Captures errors that occur before trade execution.

---

## 13. Configuration

### Constants (`constants.rs`)

| Constant | Value | Purpose |
|----------|-------|---------|
| `ENTRY_MONITOR_INTERVAL_SECS` | 3 | Entry loop interval |
| `POSITION_MONITOR_INTERVAL_SECS` | 5 | Exit loop interval |
| `ENTRY_CYCLE_MIN_WAIT_MS` | 100 | Min wait between iterations |
| `POSITION_CYCLE_MIN_WAIT_MS` | 200 | Min wait between iterations |
| `ENTRY_CHECK_ACQUIRE_TIMEOUT_SECS` | 30 | Semaphore acquire timeout |
| `ENTRY_RESERVATION_TIMEOUT_SECS` | 120 | Token reservation TTL |
| `STRATEGY_EVALUATION_TIMEOUT_SECS` | 5 | Strategy eval timeout |
| `EMERGENCY_LOSS_THRESHOLD_PCT` | 90.0 | Auto-exit loss threshold |
| `MAX_TRADE_SIZE_MULTIPLIER` | 100.0 | Max trade size guard |
| `MIN_TRADE_SIZE_SOL` | 0.001 | Min trade size |
| `MANUAL_TRADE_HISTORY_LIMIT` | 1000 | In-memory history cap |
| `STRATEGY_CACHE_MAX_ENTRIES` | 1000 | Strategy eval cache cap |

### Runtime Config (`config.rs`)

All accessed via `with_config()` macro:

| Category | Parameters |
|----------|-----------|
| **Position Mgmt** | `max_open_positions`, `trade_size_sol`, `entry_check_concurrency`, `sell_concurrency` |
| **DCA** | `dca_enabled`, `dca_threshold_pct`, `dca_max_count`, `dca_cooldown_minutes`, `dca_size_percentage` |
| **Trailing Stop** | `trailing_stop_enabled`, `activation_pct`, `distance_pct` |
| **ROI Exit** | `roi_exit_enabled`, `target_profit_pct` |
| **Stop Loss** | `stop_loss_enabled`, `threshold_pct`, `min_hold_seconds` |
| **Time Override** | `time_override_enabled`, `duration_seconds`, `loss_threshold_pct` |
| **Loss Limit** | `loss_limit_enabled`, `loss_limit_sol`, `period_hours`, `auto_resume` |
| **Monitor Control** | `entry_monitor_enabled`, `exit_monitor_enabled` |

---

## 14. Module Connections

```
trader/
├── positions/     ← Open/close positions, partial exits, DCA
├── pools/         ← get_available_tokens(), get_pool_price()
├── tokens/        ← get_full_token(), blacklist checks
├── strategies/    ← evaluate_entry/exit_strategies()
├── ohlcvs/        ← get/build TimeframeBundle for strategy eval
├── swaps/         ← Indirectly via positions module
├── config/        ← All trader config parameters
├── connectivity/  ← Health checks before trading
├── events/        ← record_trader_event() for dashboard
├── actions/       ← register_action(), update_step() for progress tracking
├── ai/            ← AI entry/exit analysis
├── logger/        ← Structured logging
└── global/        ← is_force_stopped() for emergency halt
```

| Connection | What |
|-----------|------|
| trader → pools | `get_available_tokens()` for entry scanning |
| trader → strategies | `evaluate_entry/exit_strategies()` for signals |
| trader → positions | `open_position()`, `close_position()`, `partial_close()`, `add_to_position()` |
| trader → swaps (via positions) | Quote + execute swap for trades |
| trader → ohlcvs | `get_timeframe_bundle()` for strategy evaluation |
| trader → ai | `analyze_entry()`, `analyze_exit()` (optional) |
| trader → events | Record entry/exit events for dashboard |
| trader → actions | Live action tracking (validation → quote → swap → verify) |
