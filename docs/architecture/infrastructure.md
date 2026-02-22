# Infrastructure Modules — Architecture

> ScreenerBot foundation: SQLite tuning + maintenance, structured errors, persistent events, logging, connectivity monitoring, and action progress tracking — February 2026

---

## Table of Contents

1. [Overview](#1-overview)
2. [Database (`database`)](#2-database-database)
3. [Errors (`errors`)](#3-errors-errors)
4. [Events (`events`)](#4-events-events)
5. [Logger (`logger`)](#5-logger-logger)
6. [Connectivity (`connectivity`)](#6-connectivity-connectivity)
7. [Actions (`actions`)](#7-actions-actions)
8. [Initialization Order (where these start)](#8-initialization-order-where-these-start)
9. [Cross-Module Dependencies](#9-cross-module-dependencies)

---

## 1. Overview

ScreenerBot has a set of "infrastructure" modules that everything else builds on.

These modules are not "business logic" (tokens, pools, trader, swaps, ...), but they are
critical because they define:

- how *every* SQLite database connection is configured (PRAGMAs, cache, WAL)
- how long-running processes keep databases healthy (vacuum/checkpoint cycles)
- how errors are represented and categorized
- how the bot persists an "audit trail" of important state changes (events)
- how logs are formatted, filtered, and written to disk
- how external endpoints are monitored (internet, RPC, APIs)
- how multi-step operations expose progress to the dashboard (actions + SSE)

This doc groups these foundations together because they frequently interact:

- Connectivity uses Events to record endpoint state transitions.
- Actions are stored in SQLite (via Database PRAGMAs) and streamed to Webserver via SSE.
- Logger is used by every other module (including infrastructure itself).

---

## 2. Database (`database`)

**Files:**

- `src/database/mod.rs`
- `src/database/configure.rs`
- `src/database/maintenance.rs`

The `database` module is *not* a database. It provides:

1. **Standardized SQLite connection PRAGMAs** (`configure_connection`)
2. **Per-database tuning presets** (`DbPreset`, `DbConfig`, and `*_DB` constants)
3. A **global background maintenance loop** that fixes and maintains databases on disk
   (auto-vacuum migration, incremental vacuum, WAL checkpoint).

### 2.1 Why `with_init()` matters (r2d2 + SQLite)

ScreenerBot uses `r2d2_sqlite::SqliteConnectionManager` pools for most databases.

SQLite PRAGMAs are **connection-local state**.
If you only run PRAGMAs once during DB initialization, recycled connections can
silently drift from the expected config.

Because of that, every pool is created as:

```rust
SqliteConnectionManager::file(&path)
  .with_init(|c| database::configure_connection(c, database::SOME_DB_CONST))
```

This guarantees that **every connection checkout** applies the same baseline settings.

### 2.2 Workload presets: `DbPreset` + `DbConfig`

**File:** `src/database/configure.rs`

`DbPreset` captures "how hot" a database is expected to be:

- `Hot` — high-frequency reads/writes (e.g., tokens, transactions)
- `Standard` — moderate traffic (events, actions, positions, wallet monitor, ohlcvs)
- `Cold` — infrequent access (tools, strategies, wallets list, rpc_stats, AI DBs)

`DbConfig` = `{ preset, cache_size override, mmap_size override }`:

- `cache_size` is expressed in **pages** (SQLite default page size is typically 4KB)
- `mmap_size` is expressed in **bytes**

### 2.3 Standard PRAGMAs applied to *every* connection

**File:** `src/database/configure.rs`

`configure_connection(conn, cfg)` applies:

```sql
PRAGMA journal_mode = WAL;
PRAGMA synchronous  = NORMAL;
PRAGMA cache_size   = <pages>;
PRAGMA temp_store   = MEMORY;
PRAGMA mmap_size    = <bytes>;
PRAGMA foreign_keys = 1;
PRAGMA busy_timeout = 5000;
PRAGMA auto_vacuum  = INCREMENTAL;
```

Notes:

- `cache_size` is set as a **page count** (not negative KiB form).
- `auto_vacuum=INCREMENTAL` here configures the connection, but the auto-vacuum mode
  is ultimately stored in the database file header; existing DBs can require a one-time
  conversion (see maintenance task).

### 2.4 Per-database configuration constants

**File:** `src/database/configure.rs`

The module defines DB-specific configs used by each database pool.
Examples:

- `TOKENS_DB: Hot`
- `TRANSACTIONS_DB: Hot` (+ cache override)
- `EVENTS_WRITE_DB` / `EVENTS_READ_DB` (split pools; read side enables mmap)
- `ACTIONS_WRITE_DB` / `ACTIONS_READ_DB`
- `WALLET_MONITOR_DB` (Standard + mmap)
- `RPC_STATS_DB`, `AI_DB`, `AI_CHAT_DB` (Cold)

This keeps tuning decisions centralized and consistent across the codebase.

### 2.5 Global database maintenance loop

**File:** `src/database/maintenance.rs`

`start_maintenance_task()` runs forever and performs two categories of work:

#### 2.5.1 Path discovery (only operate on DBs that exist)

`get_all_db_paths()` returns a fixed set of "known databases" and filters to those
whose files currently exist on disk:

- `tokens.db`
- `transactions.db`
- `positions.db`
- `wallet.db`
- `events.db`
- `pools.db`
- `strategies.db`
- `ohlcvs.db`
- `actions.db`
- `tools.db`
- `ai.db`
- `ai_chat.db`
- `rpc_stats.db` (resolved as `data_directory/rpc_stats.db`)

#### 2.5.2 One-time auto-vacuum migration

After a 60s startup delay, the task runs `ensure_auto_vacuum_mode(path)` on every DB:

1. Reads `PRAGMA auto_vacuum`:
   - `0 = NONE`, `1 = FULL`, `2 = INCREMENTAL`
2. If the DB is not already `INCREMENTAL`:
   - sets `PRAGMA auto_vacuum = 2`
   - runs a full `VACUUM;` rewrite to convert the database file header

This is intentionally heavy and runs only once per startup cycle.

#### 2.5.3 Periodic maintenance cycles (two independent timers)

After migration, two `tokio::time::interval(...)` loops run concurrently using
`tokio::select!`:

- **Incremental vacuum cycle**:
  - interval: `cfg.maintenance.vacuum_interval_secs` (min 1 hour enforced)
  - runs `run_incremental_vacuum(path, 500)` per DB
    - 500 pages ~= ~2MB (assuming 4KB pages)
- **WAL checkpoint cycle**:
  - interval: `cfg.maintenance.wal_checkpoint_interval_secs` (min 5 minutes enforced)
  - runs `run_wal_checkpoint(path)` per DB
    - only performs a checkpoint if `journal_mode == wal`
    - uses `wal_checkpoint = TRUNCATE` to reset the WAL file to ~0 bytes

All heavy SQLite work is run via `spawn_blocking` to avoid stalling the async runtime.

---

## 3. Errors (`errors`)

**Files:**

- `src/errors/mod.rs`
- `src/errors/error.rs`
- `src/errors/database.rs`
- `src/errors/network.rs`
- `src/errors/rpc_provider.rs`
- `src/errors/configuration.rs`
- `src/errors/data.rs`
- `src/errors/io.rs`
- `src/errors/internal.rs`
- `src/errors/position.rs`
- `src/errors/rate_limit.rs`
- `src/errors/service.rs`
- `src/errors/blockchain.rs`

ScreenerBot uses structured errors so that:

- logs can present human-friendly messages (`Display`)
- retry/handling decisions can be made based on *type*, not string matching
- Solana-specific failures (confirmation timeouts, blockhash expired, instruction error)
  are represented explicitly

### 3.1 Top-level error type: `crate::Error`

**Files:**

- `src/errors/error.rs` (definition)
- `src/errors/mod.rs` (re-exports + backwards-compat alias)

```rust
pub enum Error {
  Blockchain(BlockchainError),
  Network(NetworkError),
  RpcProvider(RpcProviderError),
  Database(DatabaseError),
  Service(ServiceError),
  Io(IoError),
  Internal(InternalError),
  Configuration(ConfigurationError),
  Data(DataError),
  Position(PositionError),
  RateLimit(RateLimitError),
}
```

For ergonomics, the crate also provides:

- `pub type Result<T> = std::result::Result<T, Error>;` (re-exported as `crate::Result<T>`)
- `pub type ScreenerBotError = Error;` (compat alias — prefer `crate::Error`)

All variants implement `Display` and the enum implements `std::error::Error`.

### 3.2 Error conversions (migration convenience)

**File:** `src/errors/error.rs`

To reduce boilerplate during migration from older string-based errors:

- `From<String>` and `From<&str>` map to `NetworkError::Generic`
- `From<reqwest::Error>` maps to `NetworkError::Generic { message: ... }`
- `From<serde_json::Error>` maps to `DataError::ParseError { data_type: "JSON", ... }`
- `From<std::io::Error>` maps to `IoError::{NotFound|PermissionDenied|...}`
- `From<rusqlite::Error>` and `From<r2d2::Error>` map to `DatabaseError`
- `From<tokio::task::JoinError>` and `From<tokio::time::error::Elapsed>` map to `InternalError`

### 3.3 Builder helpers (backward-compatible constructors)

**File:** `src/errors/error.rs`

`impl Error { ... }` includes helpers like:

- `invalid_amount(amount, reason)`
- `network_error(message)`
- `api_error(message)` (maps to `RpcProviderError::Generic`)
- `insufficient_balance(message)` (maps to `BlockchainError::InsufficientBalance` with placeholders)

These are migration helpers: they preserve old callsites while moving toward structured types.

### 3.4 Solana-specific structured failures (`blockchain.rs`)

**File:** `src/errors/blockchain.rs`

This file goes beyond a flat enum and includes a classification system:

- `FailureType`: `Permanent | Temporary | Uncertain`
- `SolanaTransactionError`: decoded, structured transaction error details
- `BlockchainError`: strongly typed Solana failure cases
- additional metadata models:
  - `CommitmentLevel`
  - `CongestionLevel`
  - `ErrorSeverity`
  - `RecoveryStrategy`

Key design point:

- `BlockchainError` has methods like `get_severity()` that allow higher-level systems
  (transaction verification, retries, swap execution) to make consistent decisions.

---

## 4. Events (`events`)

**Files:**

- `src/events/mod.rs`
- `src/events/types.rs`
- `src/events/database.rs`
- `src/events/maintenance.rs`

Events are an "audit log" of structured state changes (trades, transactions, filtering decisions,
connectivity transitions, system warnings, ...).

Events complement (but do not replace) `logger`:

- **Logger**: optimized for human reading and realtime debugging.
- **Events**: optimized for persistence, querying, metrics, and dashboard display.

### 4.1 Global event system: storage + fanout

**File:** `src/events/mod.rs`

Core global components:

- `EVENTS_DB: OnceLock<Arc<EventsDatabase>>`
- `EVENT_WRITER: LazyLock<Arc<Mutex<Option<EventWriter>>>>`
  - wraps an `mpsc::Sender<Event>` + join handle for the writer task
- `EVENTS_BROADCAST_TX: OnceLock<broadcast::Sender<Event>>`
- `EVENTS_CACHE: LazyLock<Arc<RwLock<VecDeque<Event>>>>` (recent-events ring buffer)

Capacity limits (important for memory safety under load):

- incoming channel: `EVENT_CHANNEL_CAPACITY = 10000`
- broadcast channel: `broadcast::channel::<Event>(5000)`
- in-memory cache: `EVENTS_CACHE_CAPACITY = 5000`

### 4.2 Initialization (`events::init`)

`events::init()`:

1. Creates `EventsDatabase::new()` (events.db with split pools)
2. Sets `EVENTS_DB`
3. Creates the broadcast channel and sets `EVENTS_BROADCAST_TX`
4. Creates `mpsc::channel(EVENT_CHANNEL_CAPACITY)`
5. Spawns `event_writer_task(receiver, db)`

If `events::init()` is called multiple times, it short-circuits (idempotent).

In the normal bot startup path, `events::init()` is invoked by `EventsService`
(`src/services/implementations/events_service.rs`). That service is only enabled when:

- `global::is_initialization_complete() == true`
- `cfg.events.enabled == true`

So in initialization mode (no configured bot yet), the events system is typically not started.

### 4.3 Recording API: `record` vs `record_safe`

**File:** `src/events/mod.rs`

- `record(event) -> Result<(), String>`
  - If `cfg.events.enabled == false`, it returns `Ok(())` silently.
  - Otherwise it sends to the writer channel (awaits send).
- `record_safe(event)`
  - Same config gating
  - Logs a warning if recording fails instead of propagating the error.

This is intentional: event recording should never bring down a trading bot.

### 4.4 Writer pipeline (batching + cache + broadcast)

**File:** `src/events/mod.rs`

The writer task batches:

- `BATCH_SIZE = 100`
- `BATCH_TIMEOUT_MS = 1000`

Pseudo-flow:

```text
record() -> mpsc::Sender<Event> -> event_writer_task()
  - collect until 100 events OR 1s timeout
  - db.insert_events(&mut batch)
  - push_to_cache_and_broadcast(batch)
```

Important nuance:

- `push_to_cache_and_broadcast` runs even if DB insertion fails.
  This keeps realtime UI/broadcast behavior consistent, but can cause divergence
  between the dashboard live stream and the persisted DB during failures.
  (The DB insertion error is logged by the writer task.)

### 4.5 Database schema and indexes (`events.db`)

**File:** `src/events/database.rs`

Table:

```sql
CREATE TABLE events (
  id            INTEGER PRIMARY KEY AUTOINCREMENT,
  event_time    TEXT NOT NULL,
  category      TEXT NOT NULL,
  subtype       TEXT,
  severity      TEXT NOT NULL,
  mint          TEXT,
  reference_id  TEXT,
  message_short TEXT,
  json_payload  TEXT NOT NULL,
  created_at    TEXT NOT NULL DEFAULT (datetime('now'))
);
```

Indexes include:

- category + time desc: `idx_events_category_time`
- severity + time desc: `idx_events_severity_time`
- `reference_id`, `mint`, `created_at`
- keyset helpers: `id desc`, plus composite indexes for pagination/filters

Insert behavior details:

- `json_payload` is stored as a JSON string.
- `message_short` is derived from `payload["message"]` and truncated to 240 chars.

Retention:

- DB cleanup uses `MAX_EVENT_AGE_DAYS = 30` (in `database.rs`).

### 4.6 Maintenance + category gating

**File:** `src/events/maintenance.rs`

The maintenance layer provides:

1. A background cleanup task (every 6 hours) that:
   - calls `db.cleanup_old_events()`
   - logs DB stats (total, last 24h, size)
2. Per-category recording functions:
   - `record_swap_event`, `record_transaction_event`, `record_connectivity_event`, ...
3. Per-category enable/disable:
   - `cfg.events.enabled` gate
   - and per-category flags like `cfg.events.record_swap`, `cfg.events.record_rpc`, ...

This allows reducing event volume in production while keeping the event system available.

---

## 5. Logger (`logger`)

**Files:**

- `src/logger/mod.rs`
- `src/logger/config.rs`
- `src/logger/core.rs`
- `src/logger/levels.rs`
- `src/logger/tags.rs`
- `src/logger/format.rs`
- `src/logger/file.rs`
- `src/logger/special.rs`

Logger is the realtime human-facing output system.

Key properties:

- per-tag Debug/Verbose gating from CLI flags
- structured prefix formatting (time + tag + level)
- word-wrapping for long lines
- dual output: console + log file

### 5.1 Logger initialization

**File:** `src/logger/mod.rs`

`logger::init()` does:

1. `config::init_from_args()` — parse CLI flags into the runtime config
2. `file::init_file_logging()` — create the log file and initialize the file logger

### 5.2 Levels and filtering rules

**Files:**

- `src/logger/levels.rs`
- `src/logger/core.rs`
- `src/logger/config.rs`

Log levels are ordered by increasing verbosity:

```text
Error (0) < Warning (1) < Info (2) < Debug (3) < Verbose (4)
```

Filtering is implemented in `core::should_log(tag, level)`:

- `Error` is always logged.
- Otherwise, a global threshold is applied:
  - default is `Info` (meaning Info/Warning/Error are shown)
  - `--quiet` sets threshold to `Warning`
  - `--verbose` sets threshold to `Verbose`
- `Debug` requires `--debug-<module>` for that tag.
- `Verbose` requires `--verbose` OR `--verbose-<module>` for that tag.

### 5.3 CLI flag parsing and tag mapping

**File:** `src/logger/config.rs`

Logger scans raw command-line args and maps flags like:

- `--debug-tokens`
- `--debug-pool-service`
- `--verbose-rpc`

into internal tag keys (strings like `tokens`, `pool_service`, `rpc`, ...).

This mapping is intentionally centralized to avoid every module needing to know about CLI parsing.

### 5.4 Formatting and stdout behavior

**File:** `src/logger/format.rs`

Formatting includes:

- optional time prefix (enabled), optional date prefix (disabled)
- fixed-width tag + level blocks for alignment
- word wrapping to `MAX_LINE_LENGTH = 155`
- continuation lines aligned under the message area

Broken pipe handling:

- if stdout write/flush returns `ErrorKind::BrokenPipe`, the process exits(0)
  (useful when piping output to tools like `head`).

### 5.5 File logging: rotation, retention, `latest.log`

**File:** `src/logger/file.rs`

On startup, the file logger creates a unique log file:

- `screenerbot_YYYY-MM-DD_HH-MM-SS.log`

It also creates/updates `latest.log`:

- Unix: symlink to the current run's log file
- Windows: hard link (or file copy fallback)

Retention policy:

- delete logs older than `LOG_RETENTION_HOURS = 24`
- also enforce `MAX_LOG_FILES = 7` (safety bound)

Concurrency model:

- uses a global `LazyLock<Arc<Mutex<Option<FileLogger>>>>`
- uses `try_lock()` on write:
  - if busy, the message is dropped and a drop counter increments

Write strategy:

- logs are written through a `BufWriter` (4KB buffer)
- `FLUSH_INTERVAL_WRITES = 1` currently flushes every write (debug-friendly, higher I/O)
- cleanup runs every `CLEANUP_INTERVAL_WRITES = 1000` writes via a spawned task

---

## 6. Connectivity (`connectivity`)

**Files:**

- `src/connectivity/mod.rs`
- `src/connectivity/types.rs`
- `src/connectivity/state.rs`
- `src/connectivity/monitor.rs`
- `src/connectivity/service.rs`
- `src/connectivity/monitors/*`

Connectivity is the "external dependency health" system.

It answers questions like:

- Is the internet reachable?
- Are any RPC providers currently healthy?
- Should we use cache / skip / fail when a third-party API is degraded?

Connectivity is a `ServiceManager` service:

- name: `connectivity`
- priority: 5 (starts early)
- enabled only after initialization completes (`global::is_initialization_complete()`)

### 6.1 Core types

**File:** `src/connectivity/types.rs`

- `EndpointCriticality`:
  - `Critical` (bot should pause if down; e.g. internet, rpc)
  - `Important` (warn + degraded mode; e.g. DexScreener, Jupiter)
  - `Optional` (fallback silently; e.g. Rugcheck)
- `EndpointHealth`:
  - `Healthy { latency_ms, last_check }`
  - `Degraded { latency_ms, reason, last_check }`
  - `Unhealthy { reason, last_check, last_success, consecutive_failures }`
  - `Unknown`
- `FallbackStrategy`:
  - `UseCache { max_age_secs }`
  - `UseAlternative { endpoint_name }`
  - `Skip`
  - `Fail`
- `HealthCheckResult`:
  - `success(latency_ms)`
  - `degraded(latency_ms, reason)`
  - `failure(error)`

### 6.2 Global state model (hysteresis: failure vs recovery threshold)

**File:** `src/connectivity/state.rs`

`ConnectivityState` stores:

- current health per endpoint
- criticality and fallback strategy per endpoint
- consecutive failures and consecutive successes per endpoint

Update logic (high level):

- On **success**:
  - increment successes
  - only once `successes >= recovery_threshold`:
    - reset failures to 0
    - set `Healthy` (or `Degraded` if a warning reason is present)
  - otherwise remain in the previous state (still "recovering")
- On **failure**:
  - increment failures
  - reset successes to 0
  - only once `failures >= failure_threshold`:
    - set `Unhealthy { ... }` and record last_success from previous health state

This prevents flapping when an endpoint alternates between success/failure.

### 6.3 Endpoint monitors (`EndpointMonitor` trait)

**File:** `src/connectivity/monitor.rs`

Each endpoint implements:

- `name() -> &'static str`
- `criticality() -> EndpointCriticality`
- `fallback_strategy() -> Option<FallbackStrategy>`
- `is_enabled() -> bool`
- `check_health() -> HealthCheckResult`

### 6.4 ConnectivityService loop and event recording

**File:** `src/connectivity/service.rs`

Startup:

- constructs a list of monitors (internet, rpc, dexscreener, geckoterminal, rugcheck, gmgn, jupiter)
- registers endpoint metadata in global state
- sets `global::CONNECTIVITY_SYSTEM_READY = true`

Run loop:

- interval: `cfg.connectivity.check_interval_secs`
- sequentially runs each monitor's `check_health()`
- calls `state::update_health(...)` using:
  - `cfg.connectivity.failure_threshold`
  - `cfg.connectivity.recovery_threshold`

Logging and events:

- The service compares the *previous* health kind vs the *new* health kind and only logs/records
  an event on **state transitions**.
- Events are emitted via `events::record_connectivity_event(...)` with `Severity` derived from
  endpoint criticality.

Critical endpoint failure handling:

- After each cycle, it checks `state::get_unhealthy_critical_endpoints()`.
- If any critical endpoints are unhealthy, it logs an error and records a system-level
  `"critical_endpoints_unhealthy"` connectivity event.

---

## 7. Actions (`actions`)

**Files:**

- `src/actions/mod.rs`
- `src/actions/types.rs`
- `src/actions/state.rs`
- `src/actions/database.rs`
- `src/actions/broadcast.rs`

Actions are the dashboard-facing "operation progress" system.

They exist because many bot operations are multi-step and asynchronous:

- swap buy/sell
- position open/close
- DCA (add to position)
- manual orders

The UI needs to show:

- what is currently happening
- which step is in progress
- step-level errors
- progress percentage

### 7.1 Core model: `Action` + steps

**File:** `src/actions/types.rs`

`Action` includes:

- `id: ActionId` (String)
- `action_type: ActionType`
- `entity_id: String` (mint, position id, etc.)
- `state: ActionState` (in_progress/completed/failed/cancelled)
- `steps: Vec<ActionStep>`
- timestamps: `started_at`, `completed_at`
- `metadata: serde_json::Value` (UI-friendly context)

Step updates update:

- step timestamps (`started_at`, `completed_at`)
- action-level `progress_pct` (derived from count of completed steps)

### 7.2 Storage architecture: DB is source of truth, HashMap is hot cache

**File:** `src/actions/state.rs`

Global state:

- `ACTIVE_ACTIONS: HashMap<ActionId, Action>` (in-memory hot cache)
- `ACTIONS_DB: RwLock<Option<ActionsDatabase>>` (initialized via `init_database()`)

Startup sync:

- `sync_from_db()` loads recent incomplete actions from SQLite and populates the HashMap.

### 7.3 Dual-write update pattern (DB → memory → broadcast)

Most state transitions follow this pattern:

1. write to DB (fail fast if DB write fails)
2. update in-memory HashMap
3. broadcast update for realtime UI

Examples:

- `register_action(action)`:
  - inserts into DB first; returns `Err` if DB insert fails
- `update_step(...)`:
  - DB update must succeed or it returns `false` (no broadcast)

Completion nuance:

- `complete_action_success/failed/cancel` update memory first, then update DB.
- If DB update fails, it logs an error but does not revert in-memory state.

This is a deliberate tradeoff: keep the UI responsive, but surface persistence failures.

### 7.4 Persistence: `actions.db` schema

**File:** `src/actions/database.rs`

Tables:

- `actions` (one row per action)
- `action_steps` (one row per step, unique by `(action_id, step_index)`)

The DB uses split pools (read/write) and is configured using `database::configure_connection`
like other SQLite databases.

Retention:

- DB cleanup uses a 30-day policy (`cleanup_old_actions(RETENTION_DAYS)`).

### 7.5 Broadcast channel + SSE integration

**File:** `src/actions/broadcast.rs`

- `broadcast::channel(1000)` fanout for action updates
- `subscribe()` returns a `broadcast::Receiver<ActionUpdate>`

Webserver integration:

- `src/webserver/routes/actions.rs` exposes `GET /api/actions/stream`
  which subscribes and streams updates via SSE.

### 7.6 Cleanup tasks (DB + memory)

**File:** `src/actions/state.rs`

`spawn_cleanup_task()`:

- waits 5 minutes after startup
- runs every 24 hours:
  - deletes DB rows older than 30 days
  - evicts completed/failed/cancelled actions from memory when `completed_at` is older than 24h

This prevents the in-memory HashMap from growing unbounded over a long-running bot.

---

## 8. Initialization Order (where these start)

The authoritative startup sequence is split between:

- `src/main.rs` (CLI entrypoint: banner, `logger::init()`, `config::load_config()`, panic hook)
- `src/run.rs` (service-based runtime orchestration)

Key points relevant to infrastructure:

- `logger::init()` is called in `src/main.rs` before `run_bot()` / `run.rs` executes.
- `actions::init_database()` is called in normal mode startup (before services start).
- `actions::sync_from_db()` runs at startup to restore incomplete actions into memory (requires `init_database()` first).
- `actions::spawn_cleanup_task()` runs regardless of whether actions are actively used.
- `tokio::spawn(database::start_db_maintenance_task())` starts centralized SQLite maintenance.
- `EventsService` will initialize `events::init()` (and start event DB maintenance) only when initialization is complete and `cfg.events.enabled == true`.
- `ConnectivityService` is a ServiceManager service, but it will only start after initialization
  completes and `cfg.connectivity.enabled == true`.

---

## 9. Cross-Module Dependencies

These infrastructure modules are widely depended on:

- `database` is used by every `*_database.rs` implementation that creates SQLite pools.
- `errors` is used across RPC, swapping, transaction verification, wallet parsing, and more.
- `events` is used by:
  - connectivity (endpoint transition events)
  - swaps/transactions (trade events)
  - security/filtering (risk and rejection events)
  - scheduled tasks (maintenance and background systems)
- `logger` is used by everything.
- `connectivity` is consulted by API clients and background services for fallback decisions.
- `actions` is used by:
  - swaps and positions to publish step-by-step progress
  - webserver (SSE stream + active/history tables)
