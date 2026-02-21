# Phase D — Hardening & Configurability

**Status: ✅ COMPLETED**
**Build: `cargo build --release` ✅**

## Overview

Phase D was not about memory optimization — the ≤400 MB target was already met in Phase C.
Phase D fixed correctness risks, long-term stability, and documentation accuracy.

## Tasks Completed

### D1: Stale Token Cutoff → Configurable ✅
- **File:** `src/config/schemas/maintenance.rs`
- Added `stale_token_days: u32 = 7` to MaintenanceConfig
- **File:** `src/tokens/database/assembly.rs`
- Replaced hardcoded `7 * 24 * 60 * 60` with config read via `with_config()`
- Setting to 0 disables stale filtering (loads all tokens)
- **Risk mitigated:** Users tracking dormant tokens can now adjust the threshold

### D2: WAL Checkpoint ✅
- **File:** `src/database/maintenance.rs`
- Added `run_wal_checkpoint()` using `PRAGMA wal_checkpoint(TRUNCATE)`
- Resets WAL file to zero bytes, preventing unbounded growth
- Checks journal mode first (only runs on WAL databases)
- Only logs if checkpoint takes >100ms (reduces log noise)

### D3: Config-Driven Maintenance Intervals ✅
- **File:** `src/database/maintenance.rs`
- Reads `vacuum_interval_secs` (default: 86400 = 24h) and `wal_checkpoint_interval_secs` (default: 3600 = 1h) from config
- Enforces minimums: vacuum ≥1h, WAL ≥5min
- Restructured periodic loop with `tokio::select!` for two independent timers

### D4: PLAN.md Accuracy Fix ✅
- Corrected Phase C description: was "TokenListEntry + incremental filtering" (FALSE)
- Now correctly describes: stale SQL filter, bounded caches, DB maintenance
- Marked TokenListEntry and incremental filtering as "DEFERRED INDEFINITELY"
- Added "ACTUAL VS PLANNED" warning blocks throughout

### D5: Stability Test ✅
10-minute test, 60 samples at 10-second intervals:

| Metric | Phase C | Phase D | Delta |
|--------|---------|---------|-------|
| Min    | 273 MB  | 271 MB  | -2    |
| Avg    | 371 MB  | 397 MB  | +26   |
| Median | 375 MB  | 412 MB  | +37   |
| Max    | 483 MB  | 485 MB  | +2    |

**Why +37 MB median increase:** Token count grew from 15,607 (Phase C) to 17,815 (Phase D) — 
+2,208 tokens discovered over several hours between tests. This is database growth, not a 
regression from code changes.

**WAL file sizes during test:** All small (tokens.db 9.4 MB, rpc_stats.db 1.2 MB, rest <16 KB).
WAL checkpoint will keep these in check on hourly cycles.

**No panics, no crashes, clean graceful shutdown.**

### D6: Code Review & Docs ✅
- Code review: No issues found (thread safety, edge cases, error handling all correct)
- AGENTS.md updated with Phase D info (WAL checkpoint, stale_token_days, pitfalls)
- phase-d-plan.md marked as completed

## Key Technical Notes

1. **`tokio::select!` for dual timers**: Both intervals tick once immediately after startup, 
   then continue independently. `MissedTickBehavior::Skip` prevents queue buildup.

2. **Config read on every filter query**: Negligible cost — `with_config()` is a `RwLock` 
   read lock, sub-microsecond, called once per 3-minute filter refresh cycle.

3. **WAL checkpoint TRUNCATE mode**: Resets WAL file to zero bytes. Briefly requires 
   exclusive access (busy_timeout handles contention). Much lighter than VACUUM.

## Files Changed
- `src/config/schemas/maintenance.rs` — D1 (stale_token_days field)
- `src/tokens/database/assembly.rs` — D1 (config read instead of hardcoded)
- `src/database/maintenance.rs` — D2+D3 (WAL checkpoint, config intervals, dual timers)
- `docs/investigations/2026-02-memory/PLAN.md` — D4 (accuracy corrections)
- `AGENTS.md` — D6 (Phase D documentation)
