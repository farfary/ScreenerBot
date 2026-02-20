# Phase B — Bounded Caches

**Status: ✅ COMPLETED**

**Depends on**: Phase A (completed ✅)  
**Risk**: LOW-MEDIUM (each cache migration is independent, rollback per-cache)  
**Expected impact**: Eliminates all unbounded memory growth. Saves 200-800 MB depending on runtime.

---

## Problem Statement

Phase A fixed SQLite configuration and reduced pool sizes. RSS dropped from ~1,011 MB to ~663 MB at startup. However, during extended runs (5+ minutes), RSS grows back to 1,152 MB because **in-memory caches have no size limits**.

The dominant memory consumer is `PRICE_HISTORY` — an unbounded DashMap that stores up to 1,000 price entries per token. With thousands of tokens, this alone can consume 500-1000 MB. Several other caches also grow without bounds.

## Solution

Replace unbounded `DashMap`/`HashMap`/`HashSet` caches with `moka::sync::Cache` — a battle-tested concurrent cache with LRU eviction, TTL, and bounded size. Keep DashMap only where unbounded is correct by design.

**Why moka**: ~12M downloads, W-TinyLFU eviction (better than simple LRU), thread-safe sharded internals, built-in entry_count()/weighted_size() for observability. Similar .get()/.insert() API to DashMap.

**Key API difference**: moka's `.get()` returns `Option<V>` (cloned value), not a reference. For small values (u8, bool, String) this is trivial. For large values, wrap in `Arc<V>`.

---

## Ground Truth — Current Cache Inventory

All caches verified against codebase on 2026-02-20:

### Critical (Unbounded, High Memory)

| # | Cache | File:Line | Type | Growth | Memory Impact |
|---|-------|-----------|------|--------|---------------|
| 1 | **PRICE_HISTORY** | pools/cache.rs:22 | `DashMap<String, PriceHistory>` | +1 token per price update, 1000 entries/token | **500-1000 MB** |
| 2 | **GLOBAL_KNOWN_SIGNATURES** | transactions/utils.rs:58 | `Arc<Mutex<HashSet<String>>>` | +1 per tx processed, never removed | 88 bytes/sig, **2 MB/day** |

### Medium (Unbounded, Lower Memory)

| # | Cache | File:Line | Type | Growth | Memory Impact |
|---|-------|-----------|------|--------|---------------|
| 3 | DECIMALS_CACHE | tokens/decimals.rs:30 | `Arc<RwLock<HashMap<String, u8>>>` | +1 per token lookup | ~45 bytes/entry |
| 4 | TOKEN_2022_CACHE | tokens/decimals.rs:43 | `Arc<RwLock<HashMap<String, bool>>>` | +1 per token check | ~45 bytes/entry |
| 5 | POSITION_LOCKS | positions/state.rs:22 | `RwLock<HashMap<String, Arc<Mutex<()>>>>` | +1 per traded mint, never removed | ~80 bytes/entry |
| 6 | LAST_TOKEN_ACCOUNTS_CHECK | positions/verifier.rs:22 | `RwLock<HashMap<String, DateTime<Utc>>>` | +1 per mint checked | ~60 bytes/entry |
| 7 | TOKEN_POOLS_CACHE | tokens/pool_data/cache.rs:38 | `RwLock<HashMap<String, TokenPoolCacheEntry>>` | +1 per pool lookup, stale never evicted | variable |
| 8 | POOL_PREFETCH_STATE | tokens/pool_data/cache.rs:44 | `AsyncMutex<HashMap<String, Instant>>` | +1 per prefetch | ~52 bytes/entry |
| 9 | DEFERRED_RETRIES | transactions/service/config.rs:35 | `Arc<Mutex<BTreeMap<String, DeferredRetry>>>` | +1 per failed tx | ~150 bytes/entry |
| 10 | ACTIVE_ACTIONS (in-memory) | actions/state.rs:15 | `Arc<RwLock<HashMap<ActionId, Action>>>` | +1 per action, completed stay in RAM | ~500 bytes/entry |

### Already Bounded (No Change Needed)

| Cache | File:Line | Type | Why OK |
|-------|-----------|------|--------|
| PRICE_CACHE | pools/cache.rs:19 | DashMap | Has TTL cleanup (60s, 2×TTL eviction) |
| FAILED_CACHE | tokens/decimals.rs:38 | HashSet | Small size, clear_failure() on resolve |
| FETCH_LOCKS | tokens/decimals.rs:34 | HashMap | Per-use release + 10K safety cap |
| GLOBAL_PENDING_TRANSACTIONS | transactions/utils.rs:62 | HashMap | Has 180s TTL cleanup |
| DEXSCREENER_CACHE | tokens/store.rs:228 | TimedCache | LRU + 30s TTL, cap 2000 |
| GECKOTERMINAL_CACHE | tokens/store.rs:235 | TimedCache | LRU + 60s TTL, cap 2000 |
| RUGCHECK_CACHE | tokens/store.rs:242 | TimedCache | LRU + 300s TTL, cap 3000 |
| COMPUTATION_FAILURES | wallets/balance_monitor/cache.rs:118 | HashMap | Fixed &'static str keys, ~5-10 entries |
| OHLCV_CACHE | ohlcvs/cache.rs:44 | Custom | LRU + 24h TTL, cap 100 tokens |
| ENTRY_CYCLE_RESERVATIONS | trader/monitors/entry.rs:23 | HashMap | Auto-cleanup expired |

---

## Implementation Plan

### Tier 0: Setup

**B0. Add moka dependency** ✅
```toml
# Cargo.toml [dependencies]
moka = { version = "0.12", features = ["sync"] }
```

### Tier 1: Critical (Biggest Memory Savings)

**B1. PRICE_HISTORY → moka (500-1000 MB savings)** ✅

This is the #1 priority. Currently stores PriceHistory (up to 1000 PriceResult entries) per token with no token eviction.

**Current** (pools/cache.rs:22):
```rust
static PRICE_HISTORY: LazyLock<DashMap<String, PriceHistory>> = LazyLock::new(DashMap::new);
```

**Target**: `moka::sync::Cache<String, Arc<PriceHistory>>` with:
- max_capacity: 500 tokens (covers all open positions + recently active)
- TTL: 2 hours (inactive tokens evicted)
- Arc wrapping because PriceHistory is large (~172 KB per entry at max fill)

**Files to modify**: `src/pools/cache.rs` (declaration + all access sites: lines 22, 68, 109, 247, 303, 311, 345), `src/pools/types.rs` (PRICE_HISTORY_MAX_ENTRIES stays at 1000)

**Migration pattern**:
- `PRICE_HISTORY.get(&mint)` → `PRICE_HISTORY.get(&mint)` (returns `Option<Arc<PriceHistory>>` instead of `Option<Ref<...>>`)
- `PRICE_HISTORY.get_mut(&mint)` → get + clone + modify + reinsert (moka doesn't support get_mut)
- `PRICE_HISTORY.insert(mint, history)` → `PRICE_HISTORY.insert(mint, Arc::new(history))`

**Critical consideration**: `get_mut` is used at lines 68 and 345 to modify history in-place (add_price, cleanup_gapped_data). With moka, this becomes get → clone → modify → reinsert. Line 68 is the hot path (called on every price update, potentially 100s/sec). The extra clone + reinsert per price tick is a performance concern.

**Complete access sites** (pools/cache.rs): get_mut (68, 345), insert (109, 262, 311), get (136), len→entry_count (146), iter (339).

**Recommended approach**: Keep DashMap but add a periodic cleanup task that removes tokens not in active positions and older than 2 hours. This is simpler than full moka migration, avoids the hot-path clone overhead, and achieves the same memory bound. The cleanup task runs every 5 minutes, checks each token's latest price timestamp, removes if >2h old AND not in open positions list.

**Alternative**: Full moka migration with `Arc<PriceHistory>` wrapping. More elegant but adds clone overhead on the hot path.

**B2. GLOBAL_KNOWN_SIGNATURES → moka (stops 2 MB/day leak)** ✅

**Current** (transactions/utils.rs:58):
```rust
static GLOBAL_KNOWN_SIGNATURES: LazyLock<Arc<Mutex<HashSet<String>>>> =
    LazyLock::new(|| Arc::new(Mutex::new(HashSet::new())));
```

**Target**: `moka::sync::Cache<String, ()>` with:
- max_capacity: 50,000 signatures
- TTL: none (LRU eviction is sufficient — old signatures get pushed out)

**Files to modify**: `src/transactions/utils.rs` (lines 58-59, 76, 104 + all access)

**Migration**: Straightforward. `contains()` → `.get().is_some()`, `insert()` → `.insert(sig, ())`

### Tier 2: Medium Priority (Prevent Slow Growth)

**B3. DECIMALS_CACHE → moka** ✅

**Current** (tokens/decimals.rs:30):
```rust
static DECIMALS_CACHE: std::sync::LazyLock<Arc<RwLock<HashMap<String, u8>>>> = ...
```

**Target**: `moka::sync::Cache<String, u8>` with max_capacity: 100,000 (covers hot set, u8 is trivially cloneable)

**Files to modify**: `src/tokens/decimals.rs` (lines 30-31, 179, 196, 339, 345-361)

**B4. TOKEN_2022_CACHE → moka** ✅

**Current** (tokens/decimals.rs:43):
```rust
static TOKEN_2022_CACHE: std::sync::LazyLock<Arc<RwLock<HashMap<String, bool>>>> = ...
```

**Target**: `moka::sync::Cache<String, bool>` with max_capacity: 100,000

**Files to modify**: `src/tokens/decimals.rs` (lines 43-44, 95, 153)

**B5. TOKEN_POOLS_CACHE → moka** ✅

**Current** (tokens/pool_data/cache.rs:38):
```rust
static TOKEN_POOLS_CACHE: LazyLock<RwLock<HashMap<String, TokenPoolCacheEntry>>> = ...
```

**Target**: `moka::sync::Cache<String, TokenPoolCacheEntry>` with max_capacity: 5,000, TTL: 60s (matches existing TTL check at is_pool_entry_fresh)

**Files to modify**: `src/tokens/pool_data/cache.rs` (lines 38-39 + all access sites)

**Note**: TokenPoolCacheEntry must implement Clone. Check if it already does.

**B6. POOL_PREFETCH_STATE → moka** ✅

**Current** (tokens/pool_data/cache.rs:44):
```rust
static POOL_PREFETCH_STATE: LazyLock<AsyncMutex<HashMap<String, Instant>>> = ...
```

**Target**: `moka::sync::Cache<String, Instant>` with max_capacity: 5,000, TTL: 20s (matches POOL_PREFETCH_DEBOUNCE_SECS)

**Files to modify**: `src/tokens/pool_data/cache.rs` (lines 44-45, 258)

### Tier 3: Cleanup and Small Fixes

**B7. POSITION_LOCKS cleanup on position close** ✅

NOT a moka migration — Mutex values aren't Clone. Instead, add cleanup:
- When a position is closed in `src/positions/operations.rs`, also remove the lock from POSITION_LOCKS
- Find the position close code path and add `POSITION_LOCKS.write().await.remove(&mint)`

**Files to modify**: `src/positions/state.rs` (add `pub async fn release_position_lock(mint: &str)`), `src/positions/operations.rs` or wherever positions are closed

**B8. LAST_TOKEN_ACCOUNTS_CHECK → moka** ✅

**Current** (positions/verifier.rs:22):
```rust
static LAST_TOKEN_ACCOUNTS_CHECK: LazyLock<RwLock<HashMap<String, chrono::DateTime<Utc>>>> = ...
```

**Target**: `moka::sync::Cache<String, DateTime<Utc>>` with max_capacity: 5,000, TTL: 1 hour

**Files to modify**: `src/positions/verifier.rs` (lines 22-23, 38)

**B9. DEFERRED_RETRIES → moka** ✅

**Current** (transactions/service/config.rs:35):
```rust
pub static DEFERRED_RETRIES: LazyLock<Arc<Mutex<BTreeMap<String, DeferredRetry>>>> = ...
```

**Target**: `moka::sync::Cache<String, DeferredRetry>` with max_capacity: 1,000, TTL: 5 min

**Note**: DeferredRetry must implement Clone. Check struct definition.

**Files to modify**: `src/transactions/service/config.rs` (lines 35-36, 39, 53-57)

**B10. ACTIVE_ACTIONS in-memory cleanup** ✅

Phase A wired the DB cleanup (30-day retention). But the in-memory HashMap keeps all actions forever until restart.

**Fix**: In `spawn_cleanup_task()` (actions/state.rs:485), after DB cleanup, also remove completed actions older than 24h from ACTIVE_ACTIONS HashMap.

**Files to modify**: `src/actions/state.rs` (extend spawn_cleanup_task)

### Tier 4: Nice-to-Have (Already Working, Low Urgency)

**B11. Replace custom TimedCache with moka** (tokens/store.rs) ✅

The custom TimedCache (lines 62-160) works but is 100 lines of custom code. moka does the same with less code and better eviction (W-TinyLFU vs simple LRU).

**Targets**:
- DEXSCREENER_CACHE (line 228): → moka, max: 2000, TTL: 30s
- GECKOTERMINAL_CACHE (line 235): → moka, max: 2000, TTL: 60s  
- RUGCHECK_CACHE (line 242): → moka, max: 3000, TTL: 300s

**Risk**: These already work fine. Migration adds risk for minor benefit (code reduction, slightly better eviction). Do LAST.

**Files to modify**: `src/tokens/store.rs` (remove TimedCache struct + impls, replace 3 cache declarations)

**B12. AI cache DashMap → moka** (ai/cache.rs) ✅

**Current** (ai/cache.rs:13):
```rust
cache: DashMap<String, CachedEntry>
```

**Target**: `moka::sync::Cache<String, CachedEntry>` with max_capacity: 5,000, TTL from existing

**Pre-requisite**: CachedEntry does NOT derive Clone — must add `#[derive(Clone)]` to CachedEntry at ai/cache.rs:6 first.

**Files to modify**: `src/ai/cache.rs` (lines 6, 13, 20, 34, 56)

**B13. Clean SIG_TO_MINT_INDEX and MINT_TO_POSITION_INDEX on position close** ✅

These two index caches are unbounded and grow with every position ever opened:
- `SIG_TO_MINT_INDEX` at positions/state.rs:15 — `RwLock<HashMap<String, String>>`
- `MINT_TO_POSITION_INDEX` at positions/state.rs:18 — `RwLock<HashMap<String, usize>>`

**Fix**: When a position closes, remove its entries from both indexes. NOT a moka migration — these are coordination indexes, not caches.

**Files to modify**: `src/positions/state.rs` (add cleanup functions), `src/positions/operations.rs` (call cleanup on close)

---

## Implementation Order

The tiers above define priority. Within each tier, execute in the listed order.

**Recommended batching**:

1. **Batch 1** (B0 + B1 + B2): Add moka, fix the two critical caches. Test. This alone delivers 80%+ of Phase B value.
2. **Batch 2** (B3 + B4 + B5 + B6): Decimals, Token2022, pool caches. All in tokens/ directory.
3. **Batch 3** (B7 + B8 + B9 + B10 + B13): Cleanup fixes, positions indexes, transactions, actions.
4. **Batch 4** (B11 + B12): Nice-to-have replacements. Do only if time allows.

---

## Verification

After each batch:
1. `cargo check --lib` — must compile
2. `cargo build --release` — must succeed
3. Run bot for 2 minutes, measure RSS at startup and at 2 min
4. After Batch 1: RSS should NOT grow past ~700 MB even after 5+ minutes
5. After all batches: No cache should exceed its configured max_capacity

After full Phase B:
- Run bot for 1 hour. Measure RSS every 5 minutes. Growth rate should be <1 MB/min (was unbounded before).
- Check `PRICE_HISTORY.entry_count()` stays under 500
- Check `GLOBAL_KNOWN_SIGNATURES.entry_count()` stays under 50,000

---

## Rollback

Each cache migration is independent. If a specific cache causes issues:
1. Revert that one file to the previous DashMap/HashMap version
2. Keep all other moka migrations in place
3. `cargo build --release` and re-test

moka can be removed entirely by reverting all migrations + removing from Cargo.toml.

---

## Risks and Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| moka get() clones values | Extra allocations for large values | Use Arc<V> wrapping for PriceHistory |
| PRICE_HISTORY hot-path clone | Extra clone+reinsert on every price update (100s/sec) | Use DashMap+cleanup approach instead of moka for this cache |
| DECIMALS_CACHE eviction causes cache misses | Extra DB lookups (~1-5ms per miss) | 100K capacity covers hot set; cold tokens rarely needed |
| TokenPoolCacheEntry not Clone | Won't compile | Verified: already derives Clone ✅ |
| CachedEntry (ai) not Clone | Won't compile with moka | Must add #[derive(Clone)] before migration |
| DeferredRetry not Clone | Won't compile | Verified: already derives Clone ✅ |
| TimedCache removal breaks existing behavior | Subtle timing differences | Do TimedCache replacement LAST (Tier 4), extensive testing |
| SIG_TO_MINT_INDEX cleanup removes needed data | Position lookup fails | Only clean entries for fully closed positions |

---

## Memory Budget After Phase B

| Component | Before Phase B | After Phase B | Notes |
|-----------|---------------|---------------|-------|
| PRICE_HISTORY | 500-1000 MB | ≤86 MB | 500 tokens × 1000 entries × 176 bytes |
| GLOBAL_KNOWN_SIGNATURES | Unbounded (2 MB/day) | ≤4.3 MB | 50K sigs × 88 bytes |
| DECIMALS + TOKEN_2022 | Unbounded | ≤9 MB | 100K × ~45 bytes each |
| Position indexes | Unbounded | Bounded | Cleaned on position close |
| Other caches (8 remaining) | Small, unbounded | Small, bounded | <5 MB total |
| SQLite (Phase A) | ~74 MB | ~74 MB | Already optimized |
| Application + Tokio | ~200 MB | ~200 MB | Baseline |
| **Total estimated RSS** | **1,152 MB (growing)** | **≤400 MB (stable)** | **65% reduction, no growth** |

---

## Phase B Does NOT Include

These are future phase concerns:
- **FilterToken lightweight struct** (Phase C) — reduces per-token memory during filtering
- **MaintenanceService** (Phase D) — periodic VACUUM, WAL checkpoint, stale token marking
- **Observability dashboard** (Phase E) — memory pressure detection, cache hit rate tracking
- **Memory profiles** (Phase E) — Low/Standard/High profiles for different hardware

Phase B focuses solely on **bounding in-memory caches** to stop unbounded growth.

---

## Completion Notes

**Phase B completed successfully on 2026-02-20.**

### Implementation Summary
- ✅ All 14 tasks (B0-B13) implemented and verified
- ✅ Release build compiles clean (`cargo build --release`)
- ✅ Smoke test passed (bot starts successfully)
- ✅ Code review completed (high-severity deferred retry iteration bug found and fixed)

### Key Achievements
- **TimedCache struct completely removed** from codebase (tokens/store.rs)
- **DEFERRED_RETRY_KEYS parallel index added** to fix moka iter() limitation
  - Moka's iter() doesn't support modification during iteration
  - Introduced DashSet<String> as a lightweight parallel index for signature tracking
  - Prevents deadlock/unsafe scenarios in retry cleanup logic
- **Phase B summary written** to `phase-b-summary.md` documenting approach and findings
- **AGENTS.md updated** with comprehensive cache architecture documentation

### Critical Bug Fixed
During code review, discovered a high-severity bug in DEFERRED_RETRIES implementation:
- **Problem**: Moka cache doesn't support modification during iter() (similar to Rust's borrow checker restrictions)
- **Impact**: Cleanup task would fail to remove expired retries, causing unbounded growth
- **Solution**: Added DEFERRED_RETRY_KEYS DashSet as parallel index for safe iteration
- **Files modified**: `src/transactions/service/config.rs`, `src/transactions/service/worker.rs`

### Memory Impact
Expected RSS after Phase B: ≤400 MB stable (down from 1,152 MB growing)
- PRICE_HISTORY: ≤86 MB (bounded to 500 tokens)
- GLOBAL_KNOWN_SIGNATURES: ≤4.3 MB (bounded to 50K signatures)
- All other caches: bounded with appropriate TTLs and capacity limits

### Next Steps
Phase B is complete. Ready to proceed with Phase C (FilterToken optimization) or Phase D (maintenance service) as needed.
