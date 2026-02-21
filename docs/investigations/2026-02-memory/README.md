# Memory Optimization Investigation

**Date:** February 2026
**Status:** Phase A-E ✅ ALL COMPLETED — Target exceeded (1,011 → ~250 MB avg)

## Problem

ScreenerBot used 1,011 MB+ RSS during operation with ~275K tokens. This investigation identified root causes and implemented a multi-phase fix.

## Results Summary

| Phase | Technique | RSS Impact |
|-------|-----------|------------|
| A | SQLite standardization + jemalloc | -348 MB |
| B | 14 caches → moka (bounded, TTL) | Growth bounded |
| C | Stale token filter + DB maintenance | -429 MB (target met) |
| D | Configurable intervals + WAL checkpoint | Hardened |
| E | SQLite robustness (unlock_notify, r2d2) | Stability fix |

**Total: 1,011 → ~250 MB avg (75% reduction), 945+ MB disk reclaimed**

## Root Causes (ordered by impact)

| # | Cause | Impact | Fixed In |
|---|-------|--------|----------|
| 1 | SQLite page caches: 14 DBs × huge cache_size × pool | ~580 MB at rest | A |
| 2 | Filtering snapshot: 172K tokens loaded every cycle | ~120 MB steady | C |
| 3 | mmap_size 30GB on tokens.db & wallet_monitor.db | Up to 294 MB RSS | A |
| 4 | macOS system allocator never returns pages | ~100-200 MB fragmentation | A |
| 5 | 14 unbounded caches (HashMap, TimedCache) | 20-100 MB growing | B |
| 6 | pools.db 729 MB with 0 rows (99% freelist) | Disk + maintenance waste | C |
| 7 | r2d2 connection recycling losing PRAGMA state | WAL instability risk | E |
| 8 | No SQLITE_BUSY retry (unlock_notify missing) | Concurrent access errors | E |

## Implementation Progress

### Phase A: SQLite + Allocator ✅ COMPLETED
- Standardized SQLite PRAGMA across 14 databases (DbPreset system)
- jemalloc integration (optional, default on non-MSVC)
- Reduced mmap_size from 30GB to 256MB max
- **Result:** ~580 MB reduction at startup
- **Docs:** [phase-a-plan.md](./phase-a-plan.md), [phase-a-summary.md](./phase-a-summary.md)

### Phase B: Bounded Caches ✅ COMPLETED
- All 14 caches migrated to moka (W-TinyLFU eviction, TTL)
- TimedCache completely removed
- **Result:** Memory growth bounded, no more unbounded caches
- **Docs:** [phase-b-plan.md](./phase-b-plan.md), [phase-b-summary.md](./phase-b-summary.md)

### Phase C: Token Filtering + DB Maintenance ✅ COMPLETED
- Stale token filter: `WHERE market_data_last_fetched_at > cutoff` (7-day window)
- Token count reduced from 172K to 15.6K (91% reduction)
- Auto-vacuum maintenance for 13 databases
- **Result:** RSS 371 MB avg, ≤400 MB target MET
- **Disk:** pools.db 729→0 MB, ohlcvs.db 354→175 MB, rpc_stats.db 250→161 MB
- **Docs:** [phase-c-summary.md](./phase-c-summary.md)

### Phase D: Hardening & Configurability ✅ COMPLETED
- Configurable `stale_token_days` (default 7, 0 = disabled)
- WAL checkpoint (hourly TRUNCATE)
- Config-driven vacuum/checkpoint intervals with enforced minimums
- **Result:** 397 MB avg (10-min test, 0 panics, 0 crashes)
- **Docs:** [phase-d-plan.md](./phase-d-plan.md), [phase-d-summary.md](./phase-d-summary.md)

### Phase E: SQLite Robustness ✅ COMPLETED
- Added `unlock_notify` feature to rusqlite (BUSY retry instead of immediate failure)
- Fixed r2d2 pool recycling (`idle_timeout(None)`, `max_lifetime(None)` on all 13 pools)
- Added `shrink_to_fit()` after token loading (~18 MB reclaimed)
- **Result:** 246-253 MB, 0 BUSY errors, clean operation
- **Docs:** [phase-e-summary.md](./phase-e-summary.md)

## Directory Structure

```
├── README.md              ← This file
├── PLAN.md                ← Master technical plan (historical, v1-v13)
├── AGENTS.md              ← Agent implementation strategy
├── phase-a-plan.md        ← Phase A planning and analysis
├── phase-a-summary.md     ← Phase A implementation results
├── phase-b-plan.md        ← Phase B bounded caches plan
├── phase-b-summary.md     ← Phase B implementation results
├── phase-b-test-results.md ← Phase B detailed test data
├── phase-c-summary.md     ← Phase C filtering optimization results
├── phase-d-plan.md        ← Phase D hardening plan
├── phase-d-summary.md     ← Phase D stability results
├── phase-e-summary.md     ← Phase E SQLite robustness results
├── arc/                   ← Arc<T> memory research
├── moka/                  ← Moka cache library research
├── epoch/                 ← Epoch-based memory reclamation
├── dashmap/               ← DashMap concurrent HashMap research
├── async-rusqlite/        ← Async SQLite patterns research
└── utilities/             ← Crossbeam, Hashbrown, Flurry research
```

## Key Decisions

1. **Stale token filter** (SQL WHERE clause) replaced planned TokenListEntry refactor — 1 line vs weeks of work, same result
2. **SQLite PRAGMA with_init()** pattern standardized across all 14 databases
3. **jemalloc** as optional allocator via cargo feature flag
4. **moka** cache library for bounded, TTL-based caching
5. **r2d2 pools**: `idle_timeout(None)` + `max_lifetime(None)` for SQLite WAL stability
6. **unlock_notify**: Required for concurrent SQLite access without BUSY errors
7. TokenListEntry and incremental filtering deferred indefinitely (diminished ROI)
