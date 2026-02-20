# Phase B — Bounded Caches Implementation Summary

## Overview
Phase B replaced all unbounded in-memory caches with moka bounded caches (W-TinyLFU eviction). This prevents memory from growing indefinitely as the bot runs.

## Changes Made

### Dependency Added
- `moka v0.12` (sync feature) — concurrent bounded cache with TTL support

### Caches Migrated to moka

- **GLOBAL_KNOWN_SIGNATURES** (transactions/utils.rs): HashSet → moka (50K cap)
- **DECIMALS_CACHE** (tokens/decimals.rs): RwLock<HashMap> → moka (100K cap)
- **TOKEN_2022_CACHE** (tokens/decimals.rs): RwLock<HashMap> → moka (100K cap)
- **TOKEN_POOLS_CACHE** (tokens/pool_data/cache.rs): RwLock<HashMap> → moka (5K cap, 120s TTL)
- **POOL_PREFETCH_STATE** (tokens/pool_data/cache.rs): AsyncMutex<HashMap> → moka (5K cap, 60s TTL)
- **LAST_TOKEN_ACCOUNTS_CHECK** (positions/verifier.rs): RwLock<HashMap> → moka (5K cap, 1h TTL)
- **DEFERRED_RETRIES** (transactions/service/config.rs): BTreeMap → moka (1K cap, 5min TTL) + DashSet key index
- **AI_CACHE** (ai/cache.rs): DashMap → moka (5K cap, configurable TTL)
- **DEXSCREENER_CACHE** (tokens/store.rs): TimedCache → moka (2K cap, 30s TTL)
- **GECKOTERMINAL_CACHE** (tokens/store.rs): TimedCache → moka (2K cap, 60s TTL)
- **RUGCHECK_CACHE** (tokens/store.rs): TimedCache → moka (3K cap, 5min TTL)

### Cleanup Tasks Added
- **PRICE_HISTORY**: Kept as DashMap (hot-path get_mut), added periodic cleanup that evicts tokens >2h old and not in open positions
- **POSITION_LOCKS**: Added cleanup in remove_position() to prevent lock accumulation
- **ACTIVE_ACTIONS**: Extended spawn_cleanup_task to evict completed/failed/cancelled actions >24h old

### Code Removed
- `TimedCache<K, V>` struct and `CacheEntry<V>` struct entirely removed from tokens/store.rs — replaced by moka

## Technical Decisions
- PRICE_HISTORY stays DashMap: hot-path `get_mut` called 100s/sec; moka would require clone+reinsert per tick
- DEFERRED_RETRIES uses parallel DashSet: moka 0.12 sync::Cache has no `.iter()`, so DEFERRED_RETRY_KEYS tracks keys separately
- Pool cache TTL set to 2× the configured TTL (120s) to keep stale entries briefly as fallback
- AI cache capacity at 5K to avoid storing too many large response objects

## Files Modified
- Cargo.toml (moka dependency)
- src/pools/cache.rs (PRICE_HISTORY cleanup)
- src/transactions/utils.rs (GLOBAL_KNOWN_SIGNATURES)
- src/tokens/decimals.rs (DECIMALS_CACHE, TOKEN_2022_CACHE)
- src/tokens/pool_data/cache.rs (TOKEN_POOLS_CACHE, POOL_PREFETCH_STATE)
- src/positions/state.rs (POSITION_LOCKS cleanup)
- src/positions/verifier.rs (LAST_TOKEN_ACCOUNTS_CHECK)
- src/transactions/service/config.rs (DEFERRED_RETRIES + DEFERRED_RETRY_KEYS)
- src/transactions/service/processing.rs (deferred retry iteration)
- src/actions/state.rs (ACTIVE_ACTIONS cleanup)
- src/ai/cache.rs (AI cache)
- src/tokens/store.rs (TimedCache → moka, 3 caches)

## Expected Impact
- RSS should stabilize (no more unbounded growth)
- Target: ≤400 MB stable (down from 1,152 MB growing)
- All caches now have hard capacity limits with W-TinyLFU eviction

## Remaining Unbounded Caches (Phase C candidates)
- ACTIVE_ACTIONS HashMap (bounded by cleanup, not by cap)
- POSITION_LOCKS, SIG_TO_MINT_INDEX, MINT_TO_POSITION_INDEX (bounded by position lifecycle)
- GLOBAL_PENDING_TRANSACTIONS (bounded by transaction lifecycle)
- Various webserver session caches (bounded by session lifecycle)
- OHLCV hot cache (needs investigation)
- Services metrics accumulators (needs investigation)
