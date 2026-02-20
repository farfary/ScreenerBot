# Phase A Memory Optimization — Implementation Summary

**Date**: February 2025
**Status**: ✅ COMPLETED
**Build**: cargo build --release ✅ | cargo check --lib ✅

---

## What Phase A Did

Phase A standardized SQLite configuration across all 14 databases (16 pool endpoints), added jemalloc as the global allocator, wired orphaned cleanup functions, and added config structs for future phases.

### Core Deliverable: `src/database/configure.rs`

Central module that ALL databases now use:

- **DbPreset** enum: Hot (20MB cache, 256MB mmap), Standard (8MB cache), Cold (2MB cache)
- **DbConfig** struct with const builder pattern for per-database overrides
- **configure_connection()** applies 8 PRAGMAs: journal_mode=WAL, synchronous=NORMAL, cache_size, temp_store=MEMORY, mmap_size, foreign_keys, busy_timeout=5000, auto_vacuum=INCREMENTAL
- **16 per-database constants**: TOKENS_DB, TRANSACTIONS_DB, EVENTS_WRITE_DB, EVENTS_READ_DB, ACTIONS_WRITE_DB, ACTIONS_READ_DB, POSITIONS_DB, WALLET_MONITOR_DB, OHLCVS_DB, TOOLS_DB, STRATEGIES_DB, WALLETS_DB, RPC_STATS_DB, AI_CHAT_DB, AI_DB, POOLS_DB

---

## Critical Bugs Fixed

| Bug | Impact | Fix |
|-----|--------|-----|
| tokens.db: 30GB mmap_size | Mapped entire file to RAM | Reduced to 256 MB (Hot preset) |
| wallet_monitor.db: 30GB mmap_size | Same | Reduced to 32 MB |
| events.db: Triple duplicate PRAGMAs | 3× PRAGMA calls on every connection | Removed, single configure_connection |
| ohlcvs.db: No cache_size at all | Using SQLite defaults | Now Standard preset (2000 pages) |
| pools.db: No configuration at all | 729 MB DB with zero tuning | Now Standard preset with 3000 pages |
| 13/15 pool endpoints: No with_init() | PRAGMAs lost on connection recycle | All 16 endpoints now use with_init() |

---

## Pool Size Reductions

| Database | Before | After | Change |
|----------|--------|-------|--------|
| events read | 10 | 4 | -60% |
| actions read | 10 | 4 | -60% |
| transactions | 10 | 5 | -50% |
| tools | 10 | 3 | -70% |
| strategies | 10 | 3 | -70% |
| wallets | 5 | 3 | -40% |
| rpc_stats | 5 | 3 | -40% |
| ai_chat | 5 | 3 | -40% |
| **Total** | **79** | **42** | **-47%** |

---

## Additional Changes

### jemalloc (A3)
- Added `tikv-jemallocator` to Cargo.toml
- Feature flag: `jemalloc` (default on, excluded on MSVC/Windows)
- `#[global_allocator]` in main.rs

### Cleanup Tasks (A4, A5)
- **cleanup_stats()**: Runs every 60 minutes, removes RPC stats older than 72 hours
- **cleanup_old_actions()**: Runs every 24 hours, removes actions older than 30 days
- Both were previously defined but never called from any service loop

### Config Structs (A7, A8)
- **PerformanceConfig**: memory_profile, sqlite_cache_multiplier, max_filter_tokens, dashboard_poll_secs
- **MaintenanceConfig**: retention periods, vacuum/checkpoint intervals, maintenance window
- Structural only — Phase D will wire these to actual behavior

---

## Code Review Fixes (3 additional)

Found by code review subagent after initial implementation:

1. **src/reset.rs** — positions DB connection missing configure_connection
2. **src/webserver/routes/strategies/templates.rs** — strategies export connection missing
3. **src/ai/database.rs** — connection re-open path missing configure_connection

---

## RSS Measurements

| Condition | RSS | Notes |
|-----------|-----|-------|
| Pre-Phase A baseline | ~1,011 MB | Historical measurement from investigation |
| After Phase A (20s startup) | 672 MB | All DBs initialized, tokens loading |
| After Phase A (stable ~2min) | 663 MB | Stable state |
| After Phase A (extended ~5min) | 1,152 MB | PRICE_HISTORY DashMap growth |

### Key Finding

**The dominant memory consumer is NOT SQLite — it's the PRICE_HISTORY DashMap** in `src/pools/cache.rs`. This unbounded cache stores up to 1,000 PriceResult entries per token with no eviction or size limit. With thousands of tracked tokens, it can consume 500-1000 MB.

Phase A's SQLite changes ARE correct and deliver their targeted savings (reduced page caches from ~120 MB to ~74 MB, eliminated 30 GB mmap bugs). But the in-memory cache dominance means total RSS reduction is less dramatic than the original plan projected.

**Phase B must address**: Bounded caches (PRICE_HISTORY → moka::Cache), token store cleanup, and additional pool size reductions.

---

## Files Changed (26 total)

### Created (4 files)
- `src/database/configure.rs` — Core Phase A module
- `src/database/mod.rs` — Module declaration
- `src/config/schemas/performance.rs` — PerformanceConfig struct
- `src/config/schemas/maintenance.rs` — MaintenanceConfig struct

### Modified (22 files)
- `Cargo.toml` — jemalloc dependency + feature
- `Cargo.lock` — Updated lockfile
- `src/lib.rs` — Added `pub mod database`
- `src/main.rs` — Added jemalloc `#[global_allocator]`
- `src/run.rs` — Wired actions cleanup task
- `src/reset.rs` — Added configure_connection (review fix)
- `src/config/schemas/mod.rs` — Added performance + maintenance modules
- `src/tokens/schema.rs` — Migrated, fixed 30GB mmap
- `src/events/database.rs` — Migrated dual pool, removed triple PRAGMAs
- `src/actions/database.rs` — Migrated dual pool
- `src/actions/state.rs` — Added spawn_cleanup_task()
- `src/actions/mod.rs` — Exported spawn_cleanup_task
- `src/positions/database/operations.rs` — Migrated, removed duplicate PRAGMA
- `src/pools/database/operations.rs` — Added configure_connection
- `src/tools/database/schema.rs` — Migrated, pool 10→3
- `src/strategies/database.rs` — Migrated, pool 10→3
- `src/wallets/database.rs` — Migrated, pool 5→3
- `src/wallets/balance_monitor/database.rs` — Migrated, fixed 30GB mmap
- `src/wallets/validation.rs` — Added documentation (review fix)
- `src/rpc/stats/database.rs` — Migrated, pool 5→3
- `src/rpc/stats/helpers.rs` — Added cleanup_stats() periodic call
- `src/ohlcvs/database.rs` — Migrated, added missing cache_size
- `src/ai/database.rs` — Migrated, fixed reopen path (review fix)
- `src/ai/chat_db.rs` — Migrated, pool 5→3
- `src/transactions/database/operations.rs` — Migrated, pool 10→5
- `src/webserver/routes/strategies/templates.rs` — Added configure_connection (review fix)

### Documentation Updated
- `AGENTS.md` — Added database module docs, pitfalls, conventions
- `docs/PHASE_A_MEMORY_OPTIMIZATION.md` — Marked all tasks complete, added results section

---

## Phase A Complete Checklist

- [x] A1: Create unified SQLite configuration
- [x] A2: Migrate all 14 databases (16 endpoints) to with_init()
- [x] A3: Add jemalloc allocator
- [x] A4: Wire cleanup_stats()
- [x] A5: Wire cleanup_old_actions()
- [x] A6: auto_vacuum=INCREMENTAL (in configure_connection)
- [x] A7: Add PerformanceConfig struct
- [x] A8: Add MaintenanceConfig struct
- [x] A9: Verification — build ✅, bot startup ✅, RSS measured ✅
- [x] Code review — 3 additional fixes applied ✅
- [x] AGENTS.md updated ✅
- [x] Phase A plan doc updated ✅
- [x] This summary document created ✅
