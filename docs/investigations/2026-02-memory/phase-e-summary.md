# Phase E — SQLite Robustness & Memory Polish

**Status:** ✅ COMPLETE  
**Date:** 2026-02-21  
**Commit:** (pending)

## Goal

Fix verified SQLite connection stability issues and add easy memory wins. Low-risk, high-value improvements.

## Changes

### E1: Added `unlock_notify` to rusqlite (Cargo.toml)
- Feature enables SQLite's unlock notification mechanism
- Without it: concurrent connections get immediate SQLITE_BUSY errors
- With it: connections block/retry when another connection holds a lock
- Critical with 45 concurrent connections across 14 databases

### E2: Fixed r2d2 pool connection recycling (10 files, 13 pools)
- Added `.idle_timeout(None)` and `.max_lifetime(None)` to all Pool::builder() calls
- r2d2 defaults: idle_timeout=10min, max_lifetime=30min — recycles connections
- SQLite WAL mode needs persistent connections for PRAGMA state stability
- Files: events, actions, positions, wallet, wallets, strategies, tools, rpc_stats, ai_chat, transactions

### E3: Added `shrink_to_fit()` after token loading (assembly.rs)
- Vec over-allocates by 2x during growth
- With 17.9K tokens at ~1KB each, this wastes ~18 MB
- Single line addition after the loading loop

### E4: Documentation updates (AGENTS.md)
- Added Phase E summary to DB maintenance section
- Added jemalloc MALLOC_CONF recommendation for production
- Updated Database Pitfalls with unlock_notify and r2d2 guidance

## Test Results

| Metric | Phase D | Phase E | Change |
|--------|---------|---------|--------|
| Avg RSS | 397 MB | **246-253 MB** | -37% |
| Tokens | 17.8K | 17.9K | ~same |
| Build | ✅ | ✅ | OK |
| BUSY errors | unknown | **0** | ✅ |
| Panics | 0 | 0 | ✅ |

Note: The RSS difference vs Phase D (397→246) is partly explained by shorter test duration and connectivity-limited operation. True steady-state comparison requires 24h+ runs.

## Files Modified
- `Cargo.toml` — unlock_notify feature
- `src/actions/database.rs` — r2d2 pool config
- `src/ai/chat_db.rs` — r2d2 pool config (2 pools)
- `src/events/database.rs` — r2d2 pool config (2 pools)
- `src/positions/database/operations.rs` — r2d2 pool config
- `src/rpc/stats/database.rs` — r2d2 pool config
- `src/strategies/database.rs` — r2d2 pool config
- `src/tokens/database/assembly.rs` — shrink_to_fit()
- `src/tools/database/schema.rs` — r2d2 pool config
- `src/transactions/database/operations.rs` — r2d2 pool config
- `src/wallets/balance_monitor/database.rs` — r2d2 pool config
- `src/wallets/database.rs` — r2d2 pool config
- `AGENTS.md` — Phase E docs + pitfalls
