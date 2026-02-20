# Phase B — Performance Test Results

**Date:** 2026-02-21
**Build:** Release profile, v0.1.110
**System:** macOS, 16 GB RAM, i9 MBP

## Test Methodology

- Built release binary: `cargo build --release`
- Launched bot as detached background process
- Monitored RSS, CPU, threads every 10 seconds for 10 minutes (60 samples)
- Checked logs for errors, panics, and warnings
- Analyzed database sizes and fragmentation

## Results

### Memory (RSS)

| Metric | Value |
|--------|-------|
| Startup RSS | ~680 MB |
| Minimum RSS | 591 MB |
| Maximum RSS (peak) | 1,401 MB |
| Average RSS | ~800 MB |
| Steady-state RSS | ~900 MB |
| Growth pattern | Sawtooth (bounded, not growing) |

**Before Phase A+B:** 1,152 MB and growing indefinitely.
**After Phase B:** ~900 MB stable with periodic spikes to ~1,400 MB during filter runs.

### CPU

| Metric | Value |
|--------|-------|
| Idle | 0% |
| Filter processing | 60-165% (multi-core) |
| Average | ~12% |

### Memory Spike Root Cause

The filter engine (`filtering/engine.rs`) loads all 172,000 tokens with market data into a `Vec<Token>` every ~3.5 minutes (`FILTER_CACHE_STALE_SECS = 180s`).

- Token struct: ~67 fields, estimated 1,200-1,500 bytes each
- Single load: ~246 MB
- Two concurrent loads observed: ~492 MB spike
- This is a TEMPORARY allocation, freed after filter completes (~6 seconds)
- jemalloc retains freed pages, so RSS stays elevated

**This is NOT a cache problem** — all Phase B bounded caches are working correctly. The filter loading is a separate optimization target for future phases.

### Database Status

| Database | Size | Rows | Fragmentation | Issue |
|----------|------|------|---------------|-------|
| pools.db | 729 MB | 0 | 99% (728 MB reclaimable) | CRITICAL: all freelist pages |
| tokens.db | 294 MB | 275,845 | 0% | OK |
| ohlcvs.db | 354 MB | varies | 41% (148 MB reclaimable) | Needs auto-vacuum |
| rpc_stats.db | 249 MB | varies | 0% | OK |

### Logs Analysis

- **No crashes or panics** — bot starts and runs cleanly
- **No errors** related to Phase B cache changes
- **Warnings (benign):** Invalid decimals from on-chain data (expected)
- **Warning (bug):** "Actions cleanup failed: FOREIGN KEY constraint failed" — pre-existing issue, not Phase B related
- **All 22 services** register and start successfully

## Conclusions

### Phase B Achieved
- ✅ All 14 caches bounded (moka + cleanup tasks)
- ✅ Memory no longer grows indefinitely — sawtooth pattern is bounded
- ✅ Clean startup, no crashes, no Phase-B-related errors
- ✅ Code review completed with high-severity bug found and fixed

### Not Addressed (Future Phases)
- Token filter loads 172K tokens into RAM every 3 minutes (Phase C candidate)
- pools.db 99% fragmented / 728 MB waste (needs auto-vacuum in code, NOT manual DB fix)
- ohlcvs.db 41% fragmented / 148 MB reclaimable
- Actions cleanup FK constraint error (pre-existing)
- jemalloc page retention after large temp allocations
- ~15 remaining unbounded HashMap caches (mostly lifecycle-bounded)

## Phase C Recommendations

1. **Token filter optimization:** Use streaming/cursor-based loading or lighter struct instead of full Vec<Token>
2. **Database auto-vacuum:** Enable `PRAGMA auto_vacuum = INCREMENTAL` for pools.db, ohlcvs.db
3. **Remaining unbounded caches:** ACTIVE_ACTIONS, OHLCV hot cache, webserver sessions, metrics accumulators
4. **jemalloc tuning:** Configure `background_thread` and `dirty_decay_ms` for faster page return
