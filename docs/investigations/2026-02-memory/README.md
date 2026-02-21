# Memory Optimization Investigation

**Date:** February 2026
**Status:** Phase A ✅ COMPLETED, Phase B ✅ COMPLETED, Phase C ✅ COMPLETED, Phase D ✅ COMPLETED — All 4 phases complete

## Problem

ScreenerBot uses 804MB+ RSS at startup and grows to gigabytes during 24/7 operation with ~275K tokens. This investigation identified all root causes and designed a comprehensive fix.

**Memory split:** ~60% DB-related (SQLite page caches ~450-500 MB), ~40% application logic (filtering snapshot, allocator, caches ~300-340 MB).

## Root Causes (ordered by impact)

| # | Cause | Impact | Phase |
|---|-------|--------|-------|
| 1 | SQLite page caches: 14 DBs × huge cache_size × pool | ~580MB at rest, up to 2.8GB | A |
| 2 | Filtering snapshot: 56K tokens with market data every 3 min | ~120MB steady, ~240MB peak | C |
| 3 | mmap_size 30GB on tokens.db & wallet_monitor.db | Up to 294MB RSS | A |
| 4 | macOS system allocator never returns pages | ~100-200MB fragmentation waste | A |
| 5 | 3 true leaks + 2 slow leaks + 9 unbounded caches | 20-100MB growing | B, D |
| 6 | Dashboard loads ALL positions on every poll | 20-60% overhead | D |
| 7 | Position cloning wastes 18.9 GB/year allocations | Allocator churn | B |
| 8 | Database disk waste: pools.db 729MB with 0 rows | Disk + mmap waste | E |

## Solution Architecture

10-component system across 5 phases:

- **Phase A** — SQLite standardization + jemalloc (~90% of benefit)
- **Phase B** — Cache management with moka + leak fixes
- **Phase C** — Lightweight filtering snapshot (TokenListEntry)
- **Phase D** — Maintenance service + periodic cleanup
- **Phase E** — Database compaction + monitoring

Expected result: 804MB → 150-250MB steady state, growth fully bounded.

## Implementation Progress

### Phase A: SQLite + Allocator ✅ COMPLETED
- **Implemented:** Standardized SQLite PRAGMA, jemalloc integration, mmap reduction
- **Result:** ~580MB memory reduction at startup (804MB → ~220MB baseline)
- **Documents:** 
  - [phase-a-plan.md](./phase-a-plan.md) — Phase A planning and analysis
  - [phase-a-summary.md](./phase-a-summary.md) — Implementation results and measurements

### Phase B: Bounded Caches ✅ COMPLETED
- **Implemented:** moka-based bounded caching with TTL for all unbounded caches
- **Result:** All 11 major caches migrated to moka (W-TinyLFU eviction), memory growth bounded
- **Impact:** Prevents unbounded memory growth, fixed cache-related leaks
- **Documents:** 
  - [phase-b-plan.md](./phase-b-plan.md) — Detailed implementation plan
  - [phase-b-summary.md](./phase-b-summary.md) — Implementation results and changes

### Phase C: Token Filtering Optimization ✅ COMPLETED
- **Implemented:** Unbounded cache fixes, DB maintenance tasks, stale token filtering
- **Result:** RSS reduced from 1011 MB avg → 371 MB avg (62% reduction)
- **Token count:** 172K tokens → 15.6K tokens cached (91% reduction)
- **Disk savings:** 945 MB reclaimed from databases through VACUUM operations
- **Target achieved:** ≤400 MB RSS target MET under production load
- **Documents:** 
  - [phase-c-summary.md](./phase-c-summary.md) — Implementation results and measurements

### Phase D: Hardening & Configurability ✅ COMPLETED
- **Implemented:** Configurable maintenance intervals and stale token cutoff
- **Result:** Made stale token cutoff configurable (maintenance.stale_token_days, default 7)
- **Added:** WAL checkpoint to maintenance (hourly TRUNCATE for disk space recovery)
- **Fixed:** PLAN.md accuracy (corrected false claims about TokenListEntry)
- **Stability:** 72-hour test passed (397 MB avg, 0 panics, 0 crashes)
- **Documents:** 
  - [phase-d-plan.md](./phase-d-plan.md) — Phase D planning and hardening strategy
  - [phase-d-summary.md](./phase-d-summary.md) — Implementation results and stability testing

### Phase E: Future Work
- Phase E: Enhanced monitoring and alerting

## Directory Structure

```
├── README.md              ← This file
├── PLAN.md                ← Master technical plan (5,900+ lines, v13)
├── AGENTS.md              ← Agent implementation strategy
├── phase-a-plan.md        ← Phase A planning and analysis
├── phase-a-summary.md     ← Phase A implementation results
├── phase-b-plan.md        ← Phase B bounded caches plan
├── phase-b-summary.md     ← Phase B implementation results
├── phase-c-summary.md     ← Phase C filtering optimization results
├── phase-d-plan.md        ← Phase D hardening and configurability plan
├── phase-d-summary.md     ← Phase D implementation and stability results
├── arc/                   ← Arc<T> memory research (5 files)
├── moka/                  ← Moka cache library research (4 files)
├── epoch/                 ← Epoch-based memory reclamation (3 files)
├── dashmap/               ← DashMap concurrent HashMap research (2 files)
├── async-rusqlite/        ← Async SQLite patterns research (5 files)
└── utilities/             ← Crossbeam, Hashbrown, Flurry, sources (5 files)
```

## Key Decisions

1. **TokenListEntry** (~550 bytes) replaces full Token (~2,200 bytes) in filtering snapshots — 75% reduction
2. **SQLite PRAGMA with_init()** pattern standardized across all 14 databases
3. **jemalloc** as optional allocator via cargo feature flag
4. **moka** cache library for bounded, TTL-based caching
5. ALL 275K tokens remain accessible — no data limitation
6. **MaintenanceService** for periodic VACUUM, cache cleanup, and leak mitigation

## Plan Reading Order

The plan document has been built iteratively (v1-v13). For implementation:

1. **Start with v10** (Line ~4461) — TokenListEntry architecture (Option D)
2. **Then v11** (Line ~4875) — 12 corrections to file paths and phases
3. **Then v12** (Line ~5236) — Flow details and scheduling patterns
4. **Then v13** (Line ~5594) — Non-DB memory deep-dive, corrected estimates

> v1-v9 contain historical analysis. Later versions supersede where they conflict.
