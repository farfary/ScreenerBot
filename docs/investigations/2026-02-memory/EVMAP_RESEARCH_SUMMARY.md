# Concurrent Data Structures in ScreenerBot — evmap Research Summary

> **Date**: June 2025
> **Scope**: Analysis of concurrent map implementations across the ScreenerBot Rust codebase
> **Key Finding**: ScreenerBot does **not** use `evmap`. All concurrent map access is handled by **DashMap 5.5.3**.

---

## 1. Executive Summary

An exhaustive search of the ScreenerBot codebase confirms that **`evmap` is not used anywhere** — not as a direct dependency, not as a transitive dependency, and not referenced in any source file.

**What's used instead:** [`DashMap`](https://crates.io/crates/dashmap) version **5.5.3** (locked), a concurrent hash map with fine-grained per-shard locking.

**Why this matters:** Understanding the actual concurrency primitives in the codebase is essential for reasoning about data races, lock contention, and performance characteristics. DashMap's per-shard locking model differs fundamentally from evmap's left-right (lock-free read) model, which affects how code is written, reviewed, and optimized.

---

## 2. Concurrent Data Structure Dependencies

### Direct Dependencies (Cargo.toml)

| Crate | Version Spec | Locked Version | Purpose |
|-------|-------------|----------------|---------|
| `dashmap` | `"5.5"` | 5.5.3 | Concurrent hash map |

### Transitive Dependencies

| Crate | Role |
|-------|------|
| `parking_lot` | Efficient mutex/rwlock (used by DashMap internally) |
| `crossbeam-*` | Lock-free utilities, epoch-based reclamation |

### NOT Used

| Crate | Status |
|-------|--------|
| `evmap` | ❌ Not a dependency, not referenced |
| `left-right` | ❌ Not a dependency (evmap's backing crate) |
| `flurry` | ❌ Not a dependency (Java ConcurrentHashMap port) |

---

## 3. DashMap Usage Patterns

ScreenerBot employs DashMap in three distinct patterns:

### Pattern A — Global Static (via `LazyLock`)

```rust
// pools/cache.rs:19
static PRICE_CACHE: LazyLock<DashMap<String, PriceResult>> = LazyLock::new(DashMap::new);
```

Used for process-wide caches that must be accessible from any thread or async task without passing references.

### Pattern B — Struct Instance Field

```rust
// ai/cache.rs:13-23
pub struct AiCache {
    cache: DashMap<String, CachedEntry>,
    ttl: Duration,
}
```

Used when the map's lifetime is tied to a specific subsystem and scoped access is preferred.

### Pattern C — Wrapped in `Arc`

```rust
// ai/permissions.rs:171-178
pub struct ConfirmationManager {
    pending: Arc<DashMap<String, PendingConfirmation>>,
}
```

Used when the map must be shared across independently-spawned async tasks or threads that outlive the creating scope.

---

## 4. All DashMap Usage Locations

| File | Lines | Pattern | Key Type → Value Type |
|------|-------|---------|----------------------|
| `src/pools/cache.rs` | 19, 22 | Global Static | `String → PriceResult` |
| `src/ai/cache.rs` | 13, 20 | Instance Field | `String → CachedEntry` |
| `src/ai/permissions.rs` | 171, 178 | Arc-wrapped | `String → PendingConfirmation` |
| `src/telegram/pagination.rs` | 21 | Instance/Global | Pagination state |

---

## 5. Read Operations

### Get with Clone

```rust
// pools/cache.rs:42
PRICE_CACHE.get(mint).map(|entry| entry.clone())
```

Returns an `Option<PriceResult>` by cloning the value out of the read guard. The per-shard lock is held only for the duration of the `.get()` call.

### Iterate with Filter

```rust
// pools/cache.rs:122-131
PRICE_CACHE.iter().filter_map(...)
```

Iterates over all shards. Each shard is locked and released as the iterator advances. Suitable for bulk reads where point-in-time consistency across shards is not required.

---

## 6. Write Operations

### Insert

```rust
// pools/cache.rs:52
PRICE_CACHE.insert(mint.clone(), price.clone());
```

Acquires the shard lock for the target bucket, inserts the entry, and releases.

### Mutable Access (Atomic Read-Modify-Write)

```rust
// pools/cache.rs:68
if let Some(mut history) = PRICE_HISTORY.get_mut(&mint) {
    history.cleanup_gapped_data();
    history.add_price(price);
}
```

`get_mut()` returns a `RefMut` guard that holds the per-shard lock exclusively for the duration of the block, ensuring the cleanup + add sequence is atomic with respect to other operations on the same key.

---

## 7. Cleanup Pattern

```rust
// pools/cache.rs:196-202
PRICE_CACHE.retain(|_key, price| {
    let is_fresh = now.duration_since(price.timestamp).as_secs() < ttl * 2;
    is_fresh
});
```

`retain()` iterates all shards, acquiring each shard lock in turn, and removes entries for which the predicate returns `false`. This is the idiomatic DashMap approach for bulk eviction.

---

## 8. Other Concurrent Patterns in ScreenerBot

Beyond DashMap, the codebase uses a layered set of synchronization primitives:

| Primitive | Count | Typical Use |
|-----------|-------|-------------|
| `LazyLock<T>` | 100+ instances | Global singletons (caches, managers, registries) |
| `RwLock<T>` | Moderate | Shared mutable state (`POSITIONS`, index maps) |
| `Mutex<T>` | Moderate | Exclusive access (`TELEGRAM_BOT`, database writers) |
| `OnceCell` | Several | One-time initialization (`AI_ENGINE`, `RPC_MANAGER`) |
| `AtomicBool` | Several | Boolean flags (`FORCE_STOPPED`, readiness gates) |
| `AtomicUsize` | Several | Counters (`TOOLS_ACTIVE_COUNT`) |

### Selection Rationale

- **DashMap** → concurrent read/write map with no global lock
- **RwLock** → data that is read frequently but written infrequently, where a single writer is acceptable
- **Mutex** → data that requires strict serialized access (e.g., Telegram bot instance)
- **AtomicBool/AtomicUsize** → lightweight flags and counters with no locking overhead

---

## 9. Eventual Consistency Model

The codebase explicitly documents and accepts eventual consistency between related caches:

```rust
// pools/cache.rs:49-51
// Race condition: PRICE_CACHE and PRICE_HISTORY can briefly be out of sync, but this is
// acceptable because cache is for latest-price queries while history is for trends.
```

This is a deliberate design choice: `PRICE_CACHE` serves point queries for the latest price, while `PRICE_HISTORY` serves trend analysis. A brief window where one is updated before the other has no observable impact on correctness.

---

## 10. Per-Shard Locking Safety

The codebase documents the safety guarantees of `get_mut()`:

```rust
// pools/cache.rs:65-67
// Safety: get_mut() holds a per-shard lock for this key's entry, ensuring atomicity
// of the cleanup + add_price sequence. No other thread can modify this entry concurrently.
```

This is important: `get_mut()` does **not** lock the entire map — only the shard containing the target key. Other shards remain accessible to concurrent readers and writers.

---

## 11. Best Practices Observed

Based on the codebase analysis, the following DashMap patterns are consistently applied:

1. **Use DashMap directly** for read-heavy workloads — no `RwLock<HashMap>` wrapper needed
2. **Use `get_mut()`** for atomic read-modify-write sequences (cleanup + insert)
3. **Accept eventual consistency** where appropriate (cache vs. history separation)
4. **Use `retain()`** for bulk cleanup instead of collecting keys then removing
5. **Drop read guards** before calling remove/insert to avoid deadlocks
6. **Use `Arc<DashMap>`** when sharing across independently-spawned async tasks
7. **Use `LazyLock<DashMap>`** for process-global singletons

---

## 12. Performance Characteristics

### Benchmarks

No formal DashMap benchmarks exist in the codebase. Ad-hoc timing uses `Instant::now()` in select hot paths.

### Documented Improvements

From project documentation:

| Metric | Before | After | Improvement |
|--------|--------|-------|-------------|
| Concurrent evaluation | 4–6s | 400ms | **10–15×** |
| Dashboard backend target | 8.0s | 0.25s | **32×** (target) |

These improvements are attributed to concurrent data access (DashMap + parallel evaluation) rather than sequential locked access.

---

## 13. Why DashMap Over Alternatives

From project documentation (`FLOW.md`):

- **Lock-free concurrent access** — readers and writers on different shards never block each other
- **Fine-grained per-shard locking** — contention is proportional to shard collision, not total operations
- **No global lock contention** — unlike `RwLock<HashMap>`, no single bottleneck
- **Suitable for high-frequency reads** — the dominant access pattern in ScreenerBot (price lookups, token queries)

---

## 14. Comparison: DashMap vs evmap

| Feature | DashMap (used) | evmap (not used) |
|---------|---------------|-----------------|
| **Usage in ScreenerBot** | ✅ Used extensively | ❌ Not used |
| **Locking model** | Per-shard locks | Lock-free reads via left-right |
| **Read consistency** | Immediate (latest value) | Eventual (requires `refresh()`) |
| **Read performance** | Very fast (shard lock) | Extremely fast (true lock-free) |
| **Write performance** | Fast (shard lock) | Slower (double-buffered, must `refresh()`) |
| **Memory overhead** | 1× data | 2× data (two copies) |
| **Best use case** | Balanced read/write | 99%+ reads, rare writes |
| **Learning curve** | Low (HashMap-like API) | Higher (write handle / read handle split) |
| **Dependencies** | Self-contained | Requires `left-right` crate |
| **API ergonomics** | Single `DashMap` type | Separate `ReadHandle` / `WriteHandle` |
| **Cleanup/eviction** | `retain()` | Manual via write handle |

### When Would evmap Be Better?

evmap would outperform DashMap only if:

- Read-to-write ratio exceeds 99:1
- The application can tolerate stale reads between `refresh()` calls
- Memory overhead of double-buffering is acceptable
- The complexity of split read/write handles is manageable

ScreenerBot's workload (frequent price updates + frequent price reads) makes DashMap the better fit.

---

## 15. Code Examples

### Complete Initialization

```rust
use dashmap::DashMap;
use std::sync::LazyLock;

// Global static with LazyLock
static PRICE_CACHE: LazyLock<DashMap<String, PriceResult>> = LazyLock::new(DashMap::new);

// Instance in struct
pub struct AiCache {
    cache: DashMap<String, CachedEntry>,
    ttl: Duration,
}

impl AiCache {
    pub fn new(ttl_seconds: u64) -> Self {
        Self {
            cache: DashMap::new(),
            ttl: Duration::from_secs(ttl_seconds),
        }
    }
}
```

### Complete Read

```rust
pub fn get_price(mint: &str) -> Option<PriceResult> {
    PRICE_CACHE.get(mint).map(|entry| entry.clone())
}
```

### Complete Write

```rust
pub fn update_price(price: PriceResult) {
    let mint = price.mint.clone();
    PRICE_CACHE.insert(mint.clone(), price.clone());
}
```

### Complete Atomic Update

```rust
if let Some(mut history) = PRICE_HISTORY.get_mut(&mint) {
    let removed_count = history.cleanup_gapped_data();
    history.add_price(price);
}
```

### Complete Cleanup

```rust
fn cleanup_stale_entries() {
    let now = Instant::now();
    let ttl = price_cache_ttl_seconds();
    let mut removed_count = 0;

    PRICE_CACHE.retain(|_key, price| {
        let is_fresh = now.duration_since(price.timestamp).as_secs() < ttl * 2;
        if !is_fresh {
            removed_count += 1;
        }
        is_fresh
    });
}
```

---

## 16. File Reference Index

| File | Purpose |
|------|---------|
| `Cargo.toml` | Declares `dashmap = "5.5"` dependency |
| `Cargo.lock` | Locks DashMap at 5.5.3 |
| `src/pools/cache.rs` | Price cache and history — primary DashMap usage site |
| `src/ai/cache.rs` | AI evaluation result cache |
| `src/ai/permissions.rs` | Pending tool confirmation tracking |
| `src/telegram/pagination.rs` | Telegram bot pagination state |
| `src/global.rs` | Atomic flags, `OnceCell` singletons, readiness gates |
| `src/positions/state.rs` | `RwLock`-based position state (not DashMap) |
| `docs/FLOW.md` | Documents concurrency design rationale |

---

## 17. Recommendations

### Continue Current Approach

The existing DashMap usage is **appropriate and well-implemented**:

- ✅ Correct use of per-shard locking for atomic updates
- ✅ Documented eventual consistency trade-offs
- ✅ Proper `Arc` wrapping for cross-task sharing
- ✅ Idiomatic cleanup via `retain()`

### Do NOT Migrate to evmap

There is **no compelling reason** to introduce evmap:

- ScreenerBot's read/write ratio does not justify the double-buffering overhead
- The codebase already handles eventual consistency explicitly where needed
- DashMap's API is simpler and the team is already proficient with it
- Adding evmap would introduce a second concurrent map paradigm, increasing cognitive load

### Consider evmap Only If

- A new subsystem emerges with **99%+ read** workload (e.g., a read-only configuration cache serving thousands of concurrent requests)
- **AND** the system cannot tolerate any shard-lock contention
- **AND** eventual consistency (stale reads) is acceptable for that subsystem
- **AND** memory overhead of double-buffering is within budget

### Potential Improvements (Not Urgent)

- Add DashMap capacity hints (`DashMap::with_capacity()`) for caches with predictable sizes
- Consider `DashMap::with_shard_amount()` if profiling reveals shard contention
- Add formal benchmarks for critical paths (price lookup, token query) if performance becomes a concern

---

*This document was generated from a comprehensive codebase analysis. All code references, line numbers, and patterns were verified against the ScreenerBot source at the time of writing.*
