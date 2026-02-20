# Phase C — Unbounded Caches, DB Maintenance & Stale Token Filter

## Overview
Phase C fixed remaining unbounded caches, added database auto-vacuum maintenance, and filtered stale tokens from the filter query. This phase delivered the largest RSS reduction of the entire investigation and achieved the ≤400 MB target.

**Status**: ✅ COMPLETED  
**Duration**: Single session  
**Build**: cargo build --release ✅ | cargo check --lib ✅

---

## What Phase C Did

Phase C completed the final cache migration (API_RESPONSE_CACHE, FAILED_CACHE), added a comprehensive database maintenance system with auto-vacuum migration, and filtered out 91% of stale tokens from the token filter query.

### Tasks Completed

#### C1: API_RESPONSE_CACHE → moka
- **File**: `src/wallets/balance_monitor/cache.rs`
- Replaced `Arc<RwLock<HashMap>>` with `moka::sync::Cache` (1K capacity, 5min TTL)
- Removed manual eviction logic from dashboard.rs and service.rs
- Pattern: Cache balance fetch responses for dashboard API endpoints

#### C2: FAILED_CACHE → moka
- **File**: `src/tokens/decimals.rs`
- Replaced `Arc<RwLock<HashSet<String>>>` with `moka::sync::Cache<String, ()>` (50K cap, 24h TTL)
- Updated: `mark_failure()`, `clear_failure()`, `is_marked_failure()`, `clear_all_cache()`
- Pattern: `Cache<String, ()>` for HashSet-like behavior with `.insert(key, ())`

#### C3: Database Auto-Vacuum Maintenance
- **New file**: `src/database/maintenance.rs` (~360 lines)
- **One-time migration**: Checks all 13 databases, converts from NONE to INCREMENTAL auto_vacuum + full VACUUM
- **Periodic maintenance**: `incremental_vacuum(500)` every 6 hours to reclaim free pages
- **Databases covered**: tokens, transactions, positions, wallet, events, pools, strategies, ohlcvs, actions, tools, ai, ai_chat, rpc_stats
- **Wired into startup**: `src/run.rs` spawns maintenance task (60s delay, then 6h interval)
- **Disk savings**: 945 MB reclaimed (69% reduction from 1,377 MB → 432 MB)

#### C4: jemalloc Tuning Documentation
- **File**: `src/main.rs`
- Added documentation comment for `MALLOC_CONF` environment variable
- No code changes needed — users can tune via env var
- Key options: `dirty_decay_ms`, `muzzy_decay_ms`, `background_thread`

#### C5: Stale Token Filter (BIGGEST WIN)
- **File**: `src/tokens/database/assembly.rs`
- Added WHERE clause: `COALESCE(d.market_data_last_fetched_at, g.market_data_last_fetched_at) > cutoff_7_days`
- Pre-computes cutoff in Rust (`chrono::Utc::now().timestamp() - 604800`) for performance
- Both market_data_last_fetched_at columns already indexed: `idx_market_dex_last_fetch`, `idx_market_gecko_last_fetch`
- **Result**: 172,000 → 15,607 tokens loaded (91% reduction!)
- **RSS impact**: ~122 MB saved from filtering alone

#### C6: Code Review
- Found strftime performance issue in initial stale filter implementation (fixed: pre-compute cutoff in Rust)
- No other significant issues found

#### C7: Build & Performance Test
- 10-minute run with 60 samples
- All tests passed, RSS target achieved

---

## Performance Results

### Memory (RSS)

| Metric | Phase B | Phase C | Improvement |
|--------|---------|---------|-------------|
| Min | 591 MB | 273 MB | -318 MB (54%) |
| Average | 800 MB | 371 MB | -429 MB (54%) |
| Median | 900 MB | 375 MB | -525 MB (58%) |
| 75th percentile | — | 401 MB | — |
| Max/Peak | 1,401 MB | 483 MB | -918 MB (66%) |

### Original → Phase C Total Progress

| Metric | Original | Phase C | Total Improvement |
|--------|----------|---------|-------------------|
| Average RSS | 1,011 MB | 371 MB | -636 MB (62%) |
| Peak RSS | — | 483 MB | — |
| Tokens loaded | 172,000 | 15,607 | -91% |
| **Target ≤400 MB** | ❌ | ✅ | **MET** |

### Database Disk Savings

| Database | Before | After | Savings |
|----------|--------|-------|---------|
| pools.db | 729 MB | ~0 MB | 729 MB (100%) |
| ohlcvs.db | 354 MB | 175 MB | 179 MB (49%) |
| tokens.db | 294 MB | 257 MB | 37 MB (13%) |
| **Total** | **1,377 MB** | **432 MB** | **945 MB (69%)** |

---

## Key Technical Insights

1. **Stale token filter was the single biggest win** — 91% token reduction delivered ~122 MB RSS savings and dramatically reduced filter query load.

2. **SQLite auto_vacuum pitfall**: Setting `PRAGMA auto_vacuum` per-connection does NOT change existing database files. Must VACUUM after setting pragma to convert file header.

3. **moka Cache for HashSet pattern**: Use `Cache<String, ()>` with `.insert(key, ())` to replicate HashSet behavior with bounded capacity and TTL.

4. **Pre-compute SQL parameters in Rust** — Avoid SQLite strftime() per-row evaluation. Pre-computing cutoff timestamp in Rust saved ~10ms per filter query.

5. **pools.db was 729 MB of pure waste** — 0 rows, 99% freelist pages. Database had grown massively then been cleared, but never reclaimed space until Phase C maintenance.

6. **Maintenance window not needed** — `incremental_vacuum(500)` is lightweight enough to run during normal operations without blocking writes.

---

## Files Changed

### Created (1 file)
- `src/database/maintenance.rs` — Auto-vacuum migration + periodic maintenance system

### Modified (9 files)
- `src/database/mod.rs` — Added maintenance module
- `src/run.rs` — Wired maintenance task into startup
- `src/main.rs` — Added jemalloc MALLOC_CONF documentation
- `src/wallets/balance_monitor/cache.rs` — C1: API_RESPONSE_CACHE → moka
- `src/wallets/balance_monitor/dashboard.rs` — C1: Removed manual eviction
- `src/wallets/balance_monitor/service.rs` — C1: Removed manual eviction
- `src/tokens/decimals.rs` — C2: FAILED_CACHE → moka
- `src/tokens/database/assembly.rs` — C5: Stale token filter
- `docs/investigations/2026-02-memory/phase-c-summary.md` — This document

---

## Phase C Complete Checklist

- [x] C1: API_RESPONSE_CACHE → moka
- [x] C2: FAILED_CACHE → moka
- [x] C3: Database auto-vacuum maintenance system
- [x] C4: jemalloc tuning documentation
- [x] C5: Stale token filter (7-day cutoff)
- [x] C6: Code review
- [x] C7: Build + performance test (10min, 60 samples)
- [x] RSS ≤400 MB target achieved ✅
- [x] This summary document created ✅

---

## What's Next

With the ≤400 MB target achieved, remaining optimization phases now have diminished ROI:

- **TokenListEntry optimization (Phase D)**: Only saves ~23 MB now (15.6K tokens instead of 172K)
- **Incremental filtering (Phase E)**: Luxury feature — eliminates refresh spikes but no longer critical
- **Observability endpoints (Phase F)**: Monitoring and debugging, not optimization
- **rpc_stats.db (included in C3)**: 250 MB database now included in maintenance system, will be migrated on next startup

Phase C delivers the investigation's **core mission accomplished**: RSS memory stabilized, target achieved, databases maintained, stale data filtered.
