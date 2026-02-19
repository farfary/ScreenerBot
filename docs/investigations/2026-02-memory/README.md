# Memory Optimization Investigation

**Date:** February 2026
**Status:** Plan Complete — Implementation Pending

## Problem

ScreenerBot uses 804MB+ RSS at startup and grows to gigabytes during 24/7 operation with ~275K tokens. This investigation identified all root causes and designed a comprehensive fix.

## Root Causes (ordered by impact)

| # | Cause | Impact | Phase |
|---|-------|--------|-------|
| 1 | SQLite page caches: 14 DBs × huge cache_size × pool | ~580MB at rest, up to 2.8GB | A |
| 2 | Filtering snapshot: 171K full Token structs every 3 min | 238MB steady, 455MB peak | C |
| 3 | mmap_size 30GB on tokens.db & wallet_monitor.db | Up to 294MB RSS | A |
| 4 | macOS system allocator never returns pages | ~200MB fragmentation waste | A |
| 5 | 3 true leaks + 2 slow leaks + 8 bounded caches | 20-100MB growing | B, D |
| 6 | Dashboard loads ALL positions on every poll | 20-60% overhead | D |
| 7 | Database disk waste: pools.db 729MB with 0 rows | Disk + mmap waste | E |

## Solution Architecture

10-component system across 5 phases:

- **Phase A** — SQLite standardization + jemalloc (~90% of benefit)
- **Phase B** — Cache management with moka
- **Phase C** — Lightweight filtering snapshot (TokenListEntry)
- **Phase D** — Maintenance service + leak fixes
- **Phase E** — Database compaction + monitoring

Expected result: 804MB → 250-350MB steady state, growth fully bounded.

## Files

| File | Description |
|------|-------------|
| [PLAN.md](PLAN.md) | Full technical plan (5,500+ lines) — root causes, architecture, phases, tests, dependencies |

## Key Decisions

1. **TokenListEntry** (~550 bytes) replaces full Token (~1,400 bytes) in filtering snapshots — 57% reduction, zero functionality loss
2. **SQLite PRAGMA with_init()** pattern standardized across all 14 databases
3. **jemalloc** as optional allocator via cargo feature flag
4. **moka** cache library for bounded, TTL-based caching
5. ALL 275K tokens remain accessible — no data limitation
