# Infrastructure Modules — Architecture

> ScreenerBot Foundation: Database, Errors, Events, Logger, Connectivity, Actions — February 2026

---

## Table of Contents

1. [Overview](#1-overview)
2. [Database](#2-database)
3. [Errors](#3-errors)
4. [Events](#4-events)
5. [Logger](#5-logger)
6. [Connectivity](#6-connectivity)
7. [Actions](#7-actions)
8. [Initialization Order](#8-initialization-order)
9. [Cross-Module Dependencies](#9-cross-module-dependencies)

---

## 1. Overview

Six foundational modules that all other modules depend on. Grouped here because each is small (2-8 files) but essential.

| Module | Files | Lines | Purpose |
|--------|-------|-------|---------|
| database | 3 | 643 | SQLite PRAGMA config, vacuum, WAL |
| errors | 2 | 1,407 | Error type hierarchy |
| events | 4 | 2,604 | Persistent event recording |
| logger | 8 | 1,506 | Structured logging |
| connectivity | 13 | 1,678 | Endpoint health monitoring |
| actions | 5 | 2,073 | Trade progress tracking |

---

## 2. Database

**Files:** `mod.rs`, `configure.rs`, `maintenance.rs`

Provides SQLite connection configuration and maintenance — not a database itself, but the shared PRAGMA setup used by all 13 databases.

### DbPreset

| Preset | Cache Pages | mmap | Used By |
|--------|------------|------|---------|
| `Hot` | 5000 | 256 MB | tokens.db, transactions.db |
| `Standard` | 2000 | 0 | events.db, actions.db, positions.db, etc. |
| `Cold` | 500 | 0 | tools.db, strategies.db, rpc_stats.db |

### PRAGMA Configuration

Applied to every connection via r2d2 `with_init()`:

```sql
PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;
PRAGMA cache_size = -{preset_value};
PRAGMA mmap_size = {preset_value};
PRAGMA temp_store = MEMORY;
PRAGMA busy_timeout = 5000;
```

### Maintenance

| Task | Interval | Purpose |
|------|----------|---------|
| WAL checkpoint (TRUNCATE) | 1 hour | Prevent WAL growth |
| Incremental vacuum | 6 hours | Reclaim freed pages |
| Auto-vacuum mode | Once (migration) | Enable INCREMENTAL mode |

---

## 3. Errors

**Files:** `mod.rs` (456 lines), `blockchain.rs` (894 lines)

### ScreenerBotError (Top-Level)

```rust
pub enum ScreenerBotError {
    Blockchain(BlockchainError),
    Network(NetworkError),
    RpcProvider(RpcProviderError),
    Configuration(ConfigurationError),
    Data(DataError),
    Position(PositionError),
    RateLimit(RateLimitError),
}
```

### BlockchainError (44 variants)

Solana-specific errors: `BlockNotFound`, `SlotBehind`, `BlockhashExpired`, `AccountNotFound`, `InsufficientBalance`, `TransactionDropped`, `InstructionError`, `ContractViolation`, `ProgramError`, etc.

### Builder Functions

```rust
ScreenerBotError::invalid_amount(msg)
ScreenerBotError::network_error(msg)
ScreenerBotError::signing_error(msg)
ScreenerBotError::api_error(msg)
ScreenerBotError::insufficient_balance(msg)
```

All implement `Display` for human-readable logging.

---

## 4. Events

**Files:** `mod.rs`, `database.rs`, `maintenance.rs`, `types.rs`

Persistent event recording system with 15 categories, broadcast channel for real-time SSE, and 30-day retention.

### Event Structure

```rust
pub struct Event {
    id: Option<i64>,
    event_time: DateTime<Utc>,
    category: EventCategory,
    subtype: Option<String>,
    severity: Severity,              // Info, Warn, Error, Debug
    mint: Option<String>,
    reference_id: Option<String>,    // tx sig, pool addr
    payload: Value,                  // JSON
}
```

### EventCategory (15 types)

`Swap`, `Transaction`, `Pool`, `Token`, `System`, `Position`, `Wallet`, `Trader`, `Ohlcv`, `Rpc`, `Api`, `Security`, `Connectivity`, `Filtering`, `ScheduledTask`

### Architecture

```
record(event) → mpsc::channel → Writer Task → batch(100 or 1s) → SQLite
                                     ↓
                              broadcast::channel → SSE subscribers
```

### Public API

| Function | Purpose |
|----------|---------|
| `init()` | Create DB, start writer task |
| `record(event)` | Non-blocking queue |
| `recent(category, limit)` | Query by category |
| `by_mint(mint, limit)` | Query by token |
| `subscribe()` | Broadcast receiver for SSE |
| `cleanup_old_events()` | Delete >30 days |

### Database

**Table:** `events` in `events.db`  
**Indexes:** `event_time`, `(category, event_time)`, `mint`, `reference_id`

---

## 5. Logger

**Files:** `mod.rs`, `core.rs`, `format.rs`, `config.rs`, `file.rs`, `tags.rs`, `levels.rs`, `special.rs`

Structured logging with 40+ module tags, CLI-configurable debug flags, and daily file rotation.

### Log Levels

```
Error → Warning → Info → Debug → Verbose
```

### Log Tags (40+)

`System`, `Api`, `Trader`, `Tokens`, `Pool`, `Connectivity`, `Rpc`, `Swap`, `Position`, `Wallet`, `Security`, `Filtering`, `Ohlcv`, `Tools`, etc.

### CLI Integration

```bash
screenerbot --debug-trader    # Enable debug for trader tag
screenerbot --verbose         # All verbose output
screenerbot --quiet           # Suppress warnings
```

### Public API

| Function | Purpose |
|----------|---------|
| `init()` | Parse CLI args, init file logging |
| `error(tag, msg)` | Always shown |
| `warning(tag, msg)` | Default shown |
| `info(tag, msg)` | Standard |
| `debug(tag, msg)` | Only with --debug-{tag} |
| `verbose(tag, msg)` | Only with --verbose |
| `flush()` | Force disk write |

### File Rotation

Daily logs → `logs/<YYYYMMDD>.log`

---

## 6. Connectivity

**Files:** `mod.rs`, `types.rs`, `state.rs`, `service.rs`, `monitor.rs`, `monitors/` (7 monitors)

Tracks health of external endpoints: RPC, DexScreener, GeckoTerminal, Jupiter, Rugcheck, GMGN, Internet.

### EndpointHealth

```rust
pub enum EndpointHealth {
    Healthy { latency_ms, last_check },
    Degraded { latency_ms, reason, last_check },
    Unhealthy { reason, last_check, consecutive_failures },
    Unknown,
}
```

### EndpointCriticality

| Level | Meaning | Examples |
|-------|---------|---------|
| `Critical` | Bot cannot trade without | RPC |
| `Important` | Degraded experience | DexScreener, Jupiter |
| `Optional` | Nice to have | GeckoTerminal, GMGN |

### Monitors

7 endpoint-specific monitors polling every 30s. Each implements `EndpointMonitor` trait.

### Public API

| Function | Purpose |
|----------|---------|
| `is_endpoint_healthy(name)` | Boolean check |
| `are_critical_endpoints_healthy()` | All critical OK? |
| `get_fallback_strategy(name)` | UseCache/UseAlternative/Skip/Fail |

**No database** — all in-memory via `Arc<RwLock<HashMap>>`.

---

## 7. Actions

**Files:** `mod.rs`, `types.rs`, `state.rs`, `database.rs`, `broadcast.rs`

Tracks multi-step trade operations (buy/sell) with step-by-step progress for dashboard display.

### Action Structure

```rust
pub struct Action {
    id: ActionId,
    action_type: ActionType,        // SwapBuy, SwapSell, CloseLong, etc.
    entity_id: String,              // mint or position ID
    state: ActionState,
    steps: Vec<ActionStep>,
    current_step_index: usize,
    started_at: DateTime<Utc>,
    completed_at: Option<DateTime<Utc>>,
    metadata: Value,
}
```

### ActionState

```rust
pub enum ActionState {
    InProgress { current_step, progress_pct, total_steps },
    Completed { duration_ms },
    Failed { reason, step_index },
    Cancelled,
}
```

### Public API

| Function | Purpose |
|----------|---------|
| `register_action(action)` | Start tracking |
| `update_step(id, idx, status)` | Update progress |
| `complete_action_success(id)` | Mark done |
| `complete_action_failed(id, reason)` | Mark failed |
| `subscribe()` | Broadcast for SSE |

### Database

**Tables:** `actions` + `action_steps` in `actions.db`  
**Cleanup:** Delete >30 days old.

---

## 8. Initialization Order

```
1. logger::init()           — Before anything logs
2. database maintenance     — Background vacuum/WAL task
3. events::init()           — DB + writer task
4. actions::init_database() — DB + sync incomplete
5. ConnectivityService      — Spawn monitors
6. All services ready       — Can log, record, track
```

---

## 9. Cross-Module Dependencies

```
              LOGGER (foundation)
              ↓ used by all
    ┌─────────┼─────────┐
    ↓         ↓         ↓
 DATABASE   ERRORS   CONNECTIVITY
 (PRAGMAs)  (types)  (health)
    ↓         ↓
  EVENTS    ACTIONS
  (record)  (tracking)
```

- **Logger**: All modules call `logger::{error,info,debug}(tag, msg)`
- **Database**: All DB modules call `configure_connection()` in r2d2 pools
- **Errors**: All services return `Result<T, ScreenerBotError>`
- **Events**: Dashboard subscribes for real-time SSE
- **Actions**: Dashboard subscribes for progress updates
- **Connectivity**: Services check `is_endpoint_healthy()` before operations
