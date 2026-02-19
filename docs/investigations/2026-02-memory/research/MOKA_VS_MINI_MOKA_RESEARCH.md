# Moka vs Mini-Moka: Comprehensive Comparison

> **Research Date**: July 2025
> **Versions Compared**: Moka v0.12.13 / Mini-Moka v0.11.0
> **Author**: Farhad (ScreenerBot project research)

---

## Table of Contents

- [Executive Summary](#executive-summary)
- [1. Origins & Relationship](#1-origins--relationship)
- [2. Feature Comparison](#2-feature-comparison)
- [3. Architecture & Internals](#3-architecture--internals)
- [4. Memory Footprint](#4-memory-footprint)
- [5. Performance Characteristics](#5-performance-characteristics)
- [6. Thread-Safety & Concurrency](#6-thread-safety--concurrency)
- [7. API Differences](#7-api-differences)
- [8. Dependency Tree](#8-dependency-tree)
- [9. Use Cases & Recommendations](#9-use-cases--recommendations)
- [10. ScreenerBot-Specific Recommendation](#10-screenerbot-specific-recommendation)
- [Sources](#sources)

---

## Executive Summary

### Key Takeaways

| Aspect | Moka v0.12 | Mini-Moka v0.10+ |
|--------|-----------|-------------------|
| **Best for** | High-concurrency production systems | Lightweight, simpler caching needs |
| **Hash table** | Lock-free (custom `cht::SegmentedHashMap`) | DashMap (lock-per-shard) |
| **Async support** | ✅ Full (`future::Cache`) | ❌ None |
| **Per-entry expiration** | ✅ Via `Expiry` trait | ❌ |
| **Eviction listener** | ✅ | ❌ |
| **Non-concurrent cache** | ❌ | ✅ (`unsync::Cache`) |
| **Dependency tree** | Large (14+ direct deps) | Small (6 direct deps) |
| **Memory overhead** | Higher (lock-free structures + policy metadata) | Lower (DashMap + minimal policy) |
| **Scalability (>8 threads)** | Excellent | Good but diminishing returns |

**Bottom line**: Use **Moka** when you need maximum concurrency, async support, per-entry expiration, eviction listeners, or lock-free iterators. Use **Mini-Moka** when you want a simpler, lighter cache with fewer dependencies and don't need async or advanced features.

---

## 1. Origins & Relationship

Both libraries are maintained by the same author/organization ([moka-rs](https://github.com/moka-rs)) and are inspired by Java's [Caffeine](https://github.com/ben-manes/caffeine) cache library by Ben Manes.

**Historical context**:
- Moka originally contained `moka::dash::Cache` (DashMap-based sync cache) and `moka::unsync::Cache` (single-threaded cache)
- In v0.12.0, these were extracted into Mini-Moka as a separate, lighter crate
- The migration path: `moka::unsync::Cache` → `mini_moka::unsync::Cache`, `moka::dash::Cache` → `mini_moka::sync::Cache`
- Moka v0.12 rewrote its sync cache to use a lock-free concurrent hash table instead of DashMap

Mini-Moka is essentially the **"old" simpler Moka** preserved as a standalone crate for users who don't need the full-featured version.

---

## 2. Feature Comparison

### Complete Feature Matrix

| Feature | Moka v0.12 | Mini-Moka v0.10+ | Notes |
|---------|-----------|-------------------|-------|
| **Thread-safe sync cache** | ✅ `sync::Cache` | ✅ `sync::Cache` | Both available |
| **Thread-safe async cache** | ✅ `future::Cache` | ❌ | Moka only |
| **Segmented cache** | ✅ `sync::SegmentedCache` | ❌ | Higher write concurrency |
| **Non-concurrent cache** | ❌ | ✅ `unsync::Cache` | Mini-Moka only |
| **Max entries bound** | ✅ | ✅ | Both |
| **Weighted size bound** | ✅ | ✅ | Both |
| **TinyLFU admission** | ✅ | ✅ | Both use frequency sketch |
| **LRU eviction** | ✅ | ✅ | Both |
| **LRU-only policy** | ✅ | ❌ | Moka v0.12+ added pure LRU option |
| **Time-to-live (TTL)** | ✅ | ✅ | Both |
| **Time-to-idle (TTI)** | ✅ | ✅ | Both |
| **Per-entry variable expiration** | ✅ (`Expiry` trait) | ❌ | Moka only |
| **Eviction listener** | ✅ | ❌ | Callback on entry removal |
| **`get_with` (atomic init)** | ✅ | ❌ | Atomically init & insert |
| **`try_get_with`** | ✅ | ❌ | Fallible atomic init |
| **`invalidate_entries_if`** | ✅ | ❌ | Bulk conditional invalidation |
| **Lock-free iterator** | ✅ | ❌ | Moka v0.12 |
| **Lock-per-shard iterator** | ❌ | ✅ | Via DashMap |
| **`run_pending_tasks`** | ✅ | ❌ | Explicit maintenance trigger |
| **Custom hasher** | ✅ | ✅ | Both via builder |

### Expiration Policies

**Moka** offers three levels:
1. **Cache-level TTL**: All entries expire after fixed duration from insert
2. **Cache-level TTI**: All entries expire after fixed duration from last access
3. **Per-entry `Expiry`**: Custom expiration per entry via trait callbacks (`expire_after_create`, `expire_after_read`, `expire_after_update`)

**Mini-Moka** offers only:
1. Cache-level TTL
2. Cache-level TTI

Both libraries panic if TTL/TTI is configured longer than 1000 years (overflow protection).

### Eviction Algorithm: TinyLFU

Both use the same core algorithm inspired by Caffeine:

```
┌─────────────┐     ┌──────────────┐     ┌──────────┐
│  New Entry   │────▶│  LFU Filter  │────▶│  Cache   │
│  (Candidate) │     │  (Admission) │     │  (LRU)   │
└─────────────┘     └──────────────┘     └──────────┘
                          │                     │
                    Frequency Sketch      Doubly-Linked
                    (Count-Min Sketch)    List (LRU order)
```

- **Admission**: Controlled by LFU (Least Frequently Used) via a frequency sketch (modified Count-Min Sketch) — probabilistic estimation of key popularity with very low memory footprint
- **Eviction**: Controlled by LRU (Least Recently Used) — evicts least recently accessed entries when capacity exceeded
- Both track not just cached keys but ALL hit and missed keys for frequency estimation

**Moka** additionally supports a **pure LRU policy** (without TinyLFU admission) for recency-biased workloads like job queues and event streams.

---

## 3. Architecture & Internals

### Moka v0.12 Architecture

```
┌─────────────────────────────────────────────────┐
│                   sync::Cache                    │
│                                                  │
│  ┌─────────────────────────────────────────────┐ │
│  │        Arc<Inner<K, V, S>>                  │ │
│  │                                             │ │
│  │  ┌─────────────────────────────────┐        │ │
│  │  │  Lock-Free Hash Table (CHT)     │        │ │
│  │  │  cht::SegmentedHashMap          │        │ │
│  │  │  (crossbeam-epoch for GC)       │        │ │
│  │  └─────────────────────────────────┘        │ │
│  │                                             │ │
│  │  ┌────────────┐  ┌────────────────────┐     │ │
│  │  │ Frequency  │  │  LRU Deques        │     │ │
│  │  │ Sketch     │  │  (Doubly-linked)   │     │ │
│  │  │ (TinyLFU)  │  │                    │     │ │
│  │  └────────────┘  └────────────────────┘     │ │
│  │                                             │ │
│  │  ┌────────────┐  ┌────────────────────┐     │ │
│  │  │ Timer      │  │  Bounded Channels  │     │ │
│  │  │ Wheels     │  │  (Read + Write)    │     │ │
│  │  │ (Expiry)   │  │  (crossbeam-chan)  │     │ │
│  │  └────────────┘  └────────────────────┘     │ │
│  └─────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────┘
```

**Key internal components**:
- **`cht::SegmentedHashMap`**: Custom lock-free concurrent hash table using atomic CAS operations. Based on forked `moka-cht` crate. Uses `crossbeam-epoch` for epoch-based memory reclamation (safe GC without stop-the-world)
- **Frequency Sketch**: Modified Count-Min Sketch for TinyLFU admission probability
- **LRU Deques**: Doubly-linked lists for eviction ordering
- **Hierarchical Timer Wheels**: For per-entry expiration tracking
- **Bounded Channels**: Two `crossbeam-channel` channels (read + write) buffer operation recordings. Drained when either channel reaches 64 recordings OR 300ms has passed
- **Maintenance runs inline**: No background threads — triggered by `get`, `insert`, or explicit `run_pending_tasks()`

**Consistency model**:
- Hash table operations: **Strong consistency** (lock-free, immediately visible)
- Cache policy structures: **Eventually consistent** (batched via channels, guarded by lock)

### Mini-Moka Architecture

```
┌─────────────────────────────────────────────────┐
│               sync::Cache                        │
│                                                  │
│  ┌─────────────────────────────────────────────┐ │
│  │  DashMap<K, V>                              │ │
│  │  (Lock-per-shard concurrent HashMap)        │ │
│  └─────────────────────────────────────────────┘ │
│                                                  │
│  ┌────────────┐  ┌──────────────────────┐        │
│  │ Frequency  │  │  LRU Deques          │        │
│  │ Sketch     │  │  (Doubly-linked)     │        │
│  └────────────┘  └──────────────────────┘        │
│                                                  │
│  ┌────────────────────────────────────────────┐  │
│  │  Bounded Channel (crossbeam-channel)       │  │
│  └────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────┐
│               unsync::Cache                      │
│  ┌─────────────────────────────────────────────┐ │
│  │  std::collections::HashMap                  │ │
│  │  + Frequency Sketch + LRU Deques            │ │
│  └─────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────┘
```

**Key differences from Moka**:
- Uses **DashMap** (lock-per-shard) instead of lock-free hash table
- No timer wheels (no per-entry expiration)
- Simpler internal structure overall
- `unsync::Cache` uses plain `std::collections::HashMap` — zero concurrency overhead
- Uses `triomphe::Arc` (slimmer reference counting) instead of `std::sync::Arc`

---

## 4. Memory Footprint

### What Makes Mini-Moka "Mini"?

1. **Simpler hash table**: DashMap has lower per-entry overhead than a lock-free CHT (no epoch-based GC metadata per entry, no tagged pointers)
2. **No timer wheels**: Moka allocates hierarchical timer wheel structures for per-entry expiration — Mini-Moka skips this entirely
3. **No eviction listener storage**: No closure/callback stored per cache instance
4. **Fewer channels**: Simpler buffering mechanism
5. **Smaller dependency tree**: Fewer transitive allocations from dep initialization
6. **`triomphe::Arc`**: Smaller than `std::sync::Arc` (no weak reference support)

### Per-Entry Memory Overhead Estimation

| Component | Moka | Mini-Moka (sync) | Mini-Moka (unsync) |
|-----------|------|-------------------|---------------------|
| Hash table entry | ~80-120 bytes (lock-free node + epoch metadata + tagged ptr) | ~48-64 bytes (DashMap bucket entry) | ~32-48 bytes (HashMap entry) |
| Frequency sketch | Shared (~8 bytes per cache, amortized) | Same | Same |
| LRU deque node | ~32 bytes (prev/next pointers + metadata) | ~32 bytes | ~32 bytes |
| Timer wheel slot | ~16-24 bytes (per entry if TTL/TTI set) | None | None |
| Channel recording | Amortized ~8 bytes buffered | Same | None |
| **Approximate overhead per entry** | **~140-180 bytes** | **~90-110 bytes** | **~70-90 bytes** |

> **Note**: These are rough estimates. Actual overhead depends on key/value sizes, alignment, and allocator behavior. The overhead is *in addition to* the key and value sizes.

### Cache-Level Fixed Overhead

| Component | Moka | Mini-Moka (sync) |
|-----------|------|-------------------|
| Frequency sketch table | ~32 KB (for 10K capacity) | Same |
| Bounded channels | 2 channels × buffer | 1 channel |
| Timer wheels | ~8-16 KB | None |
| LRU deque headers | ~128 bytes | ~128 bytes |
| Arc/internal state | ~512 bytes | ~256 bytes |

---

## 5. Performance Characteristics

### Throughput

Based on mokabench results and documentation:

| Scenario | Moka v0.12 | Mini-Moka v0.10 | Winner |
|----------|-----------|------------------|--------|
| **1 thread, read-heavy** | High | High | ~Tie |
| **1 thread, write-heavy** | High | High | ~Tie |
| **4 threads, mixed** | Very High | High | Moka |
| **8 threads, mixed** | Very High | High | Moka |
| **16+ threads, mixed** | Excellent | Diminishing returns | Moka (clear) |
| **Single-thread only** | N/A | Excellent (unsync) | Mini-Moka |

**Key observations**:
- At low thread counts (1-4), both perform similarly
- At high thread counts (8+), Moka's lock-free hash table significantly outperforms DashMap's lock-per-shard approach
- Mini-Moka's DashMap can become a bottleneck under heavy write contention with many threads
- Mini-Moka's `unsync::Cache` is the fastest option for single-threaded use (zero synchronization overhead)

### Hit Ratio

Both use TinyLFU and achieve **near-optimal hit ratios** on standard workloads:

**ARC-S3 (Search Engine) workload** ([from Moka wiki](https://github.com/moka-rs/moka/wiki)):
- Moka (TinyLFU): ~85-95% hit ratio (close to theoretical optimum)
- Moka (TinyLFU) vs pure LRU: 10-20% better hit ratio
- Mini-Moka: Same algorithm, same hit ratio characteristics

**crates.io production** (real-world):
- Moka maintains **~85% cache hit rate** on the high-traffic download endpoint

### Latency

| Operation | Moka | Mini-Moka (sync) |
|-----------|------|-------------------|
| `get` (cache hit) | ~50-100ns | ~50-100ns |
| `get` (cache miss) | ~30-50ns | ~30-50ns |
| `insert` | ~100-300ns | ~100-200ns |
| `insert` + maintenance drain | ~1-5μs (amortized) | ~1-3μs (amortized) |

> Latency is workload-dependent. Moka's maintenance tasks (channel draining, eviction) execute inline and can cause occasional latency spikes. Both libraries amortize this cost.

### Benchmark Tool: mokabench

The official benchmark tool is [mokabench](https://github.com/moka-rs/mokabench). It supports:
- Multiple cache libraries (Moka versions, Mini-Moka, Quick Cache, Stretto, HashLink, TinyUFO)
- Real-world ARC traces (search engine, database, OLTP)
- Configurable thread counts, TTL/TTI, insertion delays
- Hit ratio and throughput measurement

```bash
# Build with both libraries
cargo build --release -F mini-moka

# Run comparison
./target/release/mokabench --num-clients 1,4,8,16
```

---

## 6. Thread-Safety & Concurrency

### Concurrency Models

| Aspect | Moka | Mini-Moka (sync) | Mini-Moka (unsync) |
|--------|------|-------------------|--------------------|
| **Hash table** | Lock-free (CAS + crossbeam-epoch) | Lock-per-shard (DashMap) | Not thread-safe |
| **Policy structures** | Mutex-guarded, batch-applied | Mutex-guarded, batch-applied | No synchronization |
| **Read concurrency** | Full (non-blocking) | High (shard-level read locks) | Single-thread only |
| **Write concurrency** | High (CAS for hash table, batched for policy) | Moderate (shard-level write locks) | Single-thread only |
| **Iterator** | Lock-free snapshot | Lock-per-shard | Direct |
| **`Clone`** | Cheap (Arc clone) | Cheap (Arc clone) | Not Clone |
| **`Send + Sync`** | ✅ | ✅ | ❌ |

### How They Handle Concurrent Access

**Moka**:
1. `get()` → Lock-free hash table lookup → records read in bounded channel → returns cloned value
2. `insert()` → Lock-free hash table CAS insert → records write in bounded channel
3. When channel is full (64 items) or 300ms has passed → drains channel and runs maintenance under a mutex
4. Maintenance: applies frequency sketch updates, runs LRU eviction, processes timer wheels, delivers eviction notifications
5. Read channel overflow: **reads are silently dropped** (never blocks, may slightly reduce hit ratio)
6. Write channel overflow: **writes block** until drained

**Mini-Moka (sync)**:
1. `get()` → DashMap shard read lock → records read → returns cloned value
2. `insert()` → DashMap shard write lock → records write
3. Similar batched maintenance but simpler (no timer wheels, no eviction listeners)

### Lock Strategy Comparison

```
Moka:
  Hash Table:  [ Lock-Free ]  ← CAS operations, epoch-based GC
  Policy:      [ Mutex ]      ← Batched updates, amortized contention
  
Mini-Moka (sync):
  Hash Table:  [ RwLock per shard ]  ← DashMap sharding
  Policy:      [ Mutex ]             ← Batched updates
  
Mini-Moka (unsync):
  Everything:  [ No locks ]  ← Single-threaded, zero overhead
```

---

## 7. API Differences

### Cache Creation

**Moka** (sync):
```rust
use moka::sync::Cache;
use std::time::Duration;

// Simple
let cache: Cache<String, Vec<u8>> = Cache::new(10_000);

// Full builder
let cache: Cache<String, Vec<u8>> = Cache::builder()
    .max_capacity(10_000)
    .time_to_live(Duration::from_secs(300))
    .time_to_idle(Duration::from_secs(60))
    .weigher(|_key, value: &Vec<u8>| -> u32 {
        value.len().try_into().unwrap_or(u32::MAX)
    })
    .eviction_listener(|key, value, cause| {
        println!("Evicted: {key} (cause: {cause:?})");
    })
    .build();
```

**Moka** (async):
```rust
use moka::future::Cache;

let cache: Cache<String, Vec<u8>> = Cache::builder()
    .max_capacity(10_000)
    .time_to_live(Duration::from_secs(300))
    .build();

// All operations are async
cache.insert("key".into(), vec![1, 2, 3]).await;
let val = cache.get("key").await;
cache.invalidate("key").await;
```

**Mini-Moka** (sync — nearly identical to Moka sync):
```rust
use mini_moka::sync::Cache;
use std::time::Duration;

// Simple
let cache: Cache<String, Vec<u8>> = Cache::new(10_000);

// Builder
let cache: Cache<String, Vec<u8>> = Cache::builder()
    .max_capacity(10_000)
    .time_to_live(Duration::from_secs(300))
    .time_to_idle(Duration::from_secs(60))
    .weigher(|_key, value: &Vec<u8>| -> u32 {
        value.len().try_into().unwrap_or(u32::MAX)
    })
    // ❌ No .eviction_listener()
    // ❌ No .expire_after() (per-entry)
    .build();
```

**Mini-Moka** (unsync — single-threaded):
```rust
use mini_moka::unsync::Cache;

let mut cache: Cache<String, String> = Cache::new(100);
cache.insert("key".into(), "value".into());
let val = cache.get(&"key".to_string()); // Returns Option<&V> (reference!)
```

### Key API Method Differences

| Method | Moka sync | Moka async | Mini-Moka sync | Mini-Moka unsync |
|--------|-----------|------------|----------------|------------------|
| `new(capacity)` | ✅ | ✅ | ✅ | ✅ |
| `builder()` | ✅ | ✅ | ✅ | ✅ |
| `get(&key)` → `Option<V>` | ✅ (clone) | ✅ `.await` | ✅ (clone) | `Option<&V>` (ref) |
| `insert(key, val)` | ✅ | ✅ `.await` | ✅ | ✅ |
| `invalidate(&key)` | ✅ | ✅ `.await` | ✅ | ✅ |
| `invalidate_all()` | ✅ | ✅ | ✅ | ✅ |
| `invalidate_entries_if(predicate)` | ✅ | ✅ | ❌ | ❌ |
| `get_with(key, init)` | ✅ | ✅ `.await` | ❌ | ❌ |
| `try_get_with(key, init)` | ✅ | ✅ `.await` | ❌ | ❌ |
| `entry(key)` (Entry API) | ✅ | ✅ | ❌ | ❌ |
| `iter()` | ✅ (lock-free) | ❌ | ✅ (lock-per-shard) | ✅ |
| `contains_key(&key)` | ✅ | ✅ | ✅ | ✅ |
| `entry_count()` | ✅ | ✅ | ✅ | ✅ |
| `weighted_size()` | ✅ | ✅ | ✅ | ✅ |
| `run_pending_tasks()` | ✅ | ✅ `.await` | ❌ | ❌ |
| `policy()` | ✅ | ✅ | ✅ | ✅ |

### Notable Moka-Only Features

**`get_with` — Atomic initialization**:
```rust
// If key not present, atomically compute and insert
let value = cache.get_with("key".into(), || {
    expensive_computation()
});
// Only one thread runs the closure; others wait
```

**Per-entry expiration via `Expiry` trait**:
```rust
use moka::{sync::Cache, Expiry};
use std::time::Duration;

struct MyExpiry;

impl Expiry<String, String> for MyExpiry {
    fn expire_after_create(&self, _key: &String, value: &String, _created_at: Instant) -> Option<Duration> {
        if value.starts_with("temp:") {
            Some(Duration::from_secs(30))
        } else {
            Some(Duration::from_secs(3600))
        }
    }
}

let cache = Cache::builder()
    .expire_after(MyExpiry)
    .build();
```

**Eviction listener**:
```rust
let cache = Cache::builder()
    .max_capacity(1000)
    .eviction_listener(|key, value, cause| {
        // cause: Expired, Size, Explicit, Replaced
        log::info!("Evicted {key}: {cause:?}");
    })
    .build();
```

---

## 8. Dependency Tree

### Moka v0.12.13 Dependencies

```toml
[dependencies]
crossbeam-channel = "0.5.15"     # Bounded channels for operation recording
crossbeam-epoch = "0.9.18"       # Epoch-based memory reclamation (lock-free GC)
crossbeam-utils = "0.8.21"       # Concurrency utilities
equivalent = "1.0"               # Key equivalence trait
parking_lot = "0.12"             # Fast mutex for policy structures
portable-atomic = "1.6"          # Portable atomic operations (32-bit support)
smallvec = "1.8"                 # Stack-allocated small vectors
tagptr = "0.2"                   # Tagged pointers for lock-free data structures
uuid = "1.1"                     # UUIDs for cache instance identification

# Optional (async)
async-lock = "3.3"               # Async mutex/rwlock
event-listener = "5.3"           # Async event notification
futures-util = "0.3.17"          # Future combinators

# Optional
quanta = "0.12.2"                # High-performance clock
log = "0.4"                      # Logging
```

**Total**: 9 required + 4 optional direct dependencies

### Mini-Moka v0.11.0 Dependencies

```toml
[dependencies]
crossbeam-channel = "0.5.5"      # Bounded channels
crossbeam-utils = "0.8"          # Concurrency utilities
smallvec = "1.8"                 # Stack-allocated small vectors
tagptr = "0.2"                   # Tagged pointers
triomphe = "0.1.13"              # Lightweight Arc (no weak refs)

# Optional (enabled by default)
dashmap = "6.1"                  # Lock-per-shard concurrent HashMap
```

**Total**: 5 required + 1 optional direct dependencies

### Dependency Comparison

| Category | Moka | Mini-Moka |
|----------|------|-----------|
| Direct dependencies (required) | 9 | 5 |
| Direct dependencies (all features) | 13 | 6 |
| Notable exclusives | `crossbeam-epoch`, `parking_lot`, `portable-atomic`, `uuid` | `triomphe`, `dashmap` |
| Shared | `crossbeam-channel`, `crossbeam-utils`, `smallvec`, `tagptr` | Same |
| Compile time impact | Higher | Lower |

**Key difference**: Moka needs `crossbeam-epoch` for its lock-free GC and `parking_lot` for its policy mutex. Mini-Moka avoids both by using DashMap (which has its own internal locking) and `triomphe` for lightweight reference counting.

---

## 9. Use Cases & Recommendations

### When to Choose Moka

| Scenario | Why Moka |
|----------|----------|
| **High-concurrency servers** (16+ threads) | Lock-free hash table scales linearly |
| **Async applications** (tokio, async-std) | Only option with `future::Cache` |
| **Per-entry expiration** needed | `Expiry` trait for custom TTL per entry |
| **Eviction notifications** needed | Eviction listener callback |
| **Atomic get-or-insert** (`get_with`) | Prevents thundering herd |
| **Production caching** (crates.io-scale) | Proven in production, ~85% hit rate |
| **Need lock-free iterators** | Non-blocking cache iteration |
| **Complex invalidation patterns** | `invalidate_entries_if` for bulk conditional invalidation |

### When to Choose Mini-Moka

| Scenario | Why Mini-Moka |
|----------|---------------|
| **Single-threaded applications** | `unsync::Cache` — zero overhead, returns `&V` references |
| **Minimal dependencies** matter | 6 deps vs 13 |
| **Compile time** is critical | Smaller dep tree = faster builds |
| **Embedded or resource-constrained** | Lower memory footprint |
| **Low-to-moderate concurrency** (1-8 threads) | DashMap is "fast enough" |
| **Simple TTL/TTI caching** | All you need, without complexity |
| **WASM/no-std adjacent** | Lighter surface area (though neither supports WASM) |
| **Don't need async** | Simpler mental model |

### Decision Flowchart

```
Need async cache?
  ├── Yes → Moka
  └── No
      ├── Single-threaded?
      │   ├── Yes → Mini-Moka (unsync::Cache)
      │   └── No
      │       ├── Need per-entry expiration?
      │       │   ├── Yes → Moka
      │       │   └── No
      │       │       ├── Need eviction listener?
      │       │       │   ├── Yes → Moka
      │       │       │   └── No
      │       │       │       ├── Need get_with (atomic init)?
      │       │       │       │   ├── Yes → Moka
      │       │       │       │   └── No
      │       │       │       │       ├── >8 threads high contention?
      │       │       │       │       │   ├── Yes → Moka
      │       │       │       │       │   └── No → Mini-Moka
      │       │       │       │       └──
      │       │       │       └──
      │       │       └──
      │       └──
      └──
```

### Real-World Adoption

**Moka in production**:
- **crates.io** — API service caching (85% hit rate on download endpoint, since Nov 2021)
- **aliyundrive-webdav** — Home router WebDAV gateway (32-bit MIPS/ARMv5TE embedded devices)
- Various web services, API gateways, and database query caches

**Mini-Moka use cases**:
- CLI tools with in-process caching
- Libraries that want to add caching without heavy dependencies
- Single-threaded applications (game engines, batch processors)
- Prototyping and smaller services

---

## 10. ScreenerBot-Specific Recommendation

### Current Usage Context

ScreenerBot uses caches extensively:
- Token metadata caching (tokens/store.rs)
- Pool price history (pools/cache.rs)
- API response caching (apis/*/cache.rs)
- OHLCV data (ohlcvs/cache.rs)
- RPC stats caching
- ATA failed cache

### Recommendation: **Use Moka**

For ScreenerBot, **Moka is the correct choice** because:

1. **Async-first architecture**: ScreenerBot runs on tokio — `moka::future::Cache` integrates natively
2. **High concurrency**: Multiple services (pools, tokens, transactions, trader) access caches concurrently from many tokio tasks
3. **Per-entry expiration**: Different tokens/pools may need different cache durations based on activity level
4. **Eviction listeners**: Useful for logging/metrics when cached data is evicted (tracking cache behavior)
5. **`get_with`**: Prevents thundering herd when multiple tasks try to fetch the same token data simultaneously
6. **Production-proven**: ScreenerBot is a production trading system — reliability matters more than compile time

Mini-Moka would only be appropriate if ScreenerBot had isolated, simple caching needs in a single-threaded context, which it does not.

---

## Sources

### Official Documentation
- [Moka GitHub README](https://github.com/moka-rs/moka) — Feature list, examples, comparison table
- [Mini-Moka GitHub README](https://github.com/moka-rs/mini-moka) — Features, examples
- [Moka docs.rs](https://docs.rs/moka/latest/moka/) — API reference, implementation details
- [Mini-Moka docs.rs](https://docs.rs/mini-moka/latest/mini_moka/) — API reference
- [Moka Wiki: Admission and Eviction Policies](https://github.com/moka-rs/moka/wiki) — TinyLFU details, hit ratio benchmarks

### Source Code
- [Moka Cargo.toml](https://github.com/moka-rs/moka/blob/main/Cargo.toml) — v0.12.13, 9 required deps
- [Mini-Moka Cargo.toml](https://github.com/moka-rs/mini-moka/blob/main/Cargo.toml) — v0.11.0, 5 required deps
- [moka-cht](https://github.com/moka-rs/moka-cht) — Lock-free concurrent hash table used by Moka

### Benchmarking
- [mokabench](https://github.com/moka-rs/mokabench) — Official benchmark tool for Moka ecosystem
- [Caffeine Simulator](https://github.com/ben-manes/caffeine/wiki/Simulator) — Used for hit ratio benchmarks

### Architecture Deep Dives
- [DeepWiki: moka-rs/moka](https://deepwiki.com/moka-rs/moka) — Architecture analysis
- [DeepWiki: Lock-Free Hash Table](https://deepwiki.com/moka-rs/moka/8.1-lock-free-hash-table) — CHT internals

### Academic References
- [TinyLFU: A Highly Efficient Cache Admission Policy](http://arxiv.org/pdf/1512.00727.pdf)
- [ARC: A Self-Tuning, Low Overhead Replacement Cache](https://www.usenix.org/event/fast03/tech/full_papers/megiddo/megiddo.pdf)

### Community
- [Moka GitHub Discussions](https://github.com/moka-rs/moka/discussions) — Production usage reports
- [crates.io usage of Moka](https://github.com/moka-rs/moka/discussions/51) — 85% hit rate data
- [LibHunt: Moka vs DashMap](https://www.libhunt.com/compare-moka-vs-dashmap) — Community comparison
