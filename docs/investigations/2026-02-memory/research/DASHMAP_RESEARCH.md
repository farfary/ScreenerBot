# DashMap: Official Repository Research

## Executive Summary
DashMap is **a blazingly fast concurrent hashmap for Rust** designed to be a drop-in replacement for `RwLock<HashMap<K, V>>`. It uses sharded locking architecture for superior concurrent performance.

---

## 1. Performance Claims (From Official README)

### Key Claim
- **"Blazingly fast concurrent map in Rust"**
- **Goal**: To be "as fast as possible" - great effort put into performance optimization
- **Direct Replacement**: Aims to be a direct replacement for `RwLock<HashMap<K, V>>`

### Architecture for Performance
- Uses **sharded concurrent hashmap** (multiple internal locks instead of one global lock)
- Each key is hashed to a specific shard, reducing lock contention
- Allows multiple threads to access different shards simultaneously
- Uses **CachePadded** for preventing false sharing between shard locks

---

## 2. Benchmark Suite: conc-map-bench

### Benchmark Harness
- Based on **libcuckoo benchmark** (well-regarded in the community)
- Uses **bustle benchmarking framework**
- Location: https://github.com/xacrimon/conc-map-bench

### Three Workload Models

#### 2.1 Read Heavy (Cache/Page Cache Workload)
```
read:   98%
insert:  1%
remove:  1%
update:  0%
```
**Use Case**: Web server caches, disk page caches, data lookup heavy scenarios

#### 2.2 Exchange (Data Exchange Workload)
```
read:    10%
insert:  40%
remove:  40%
update:  10%
```
**Use Case**: Maps used for exchanging/rotating data, temporary storage

#### 2.3 Rapid Grow (Insert Heavy Workload)
```
read:     5%
insert:  80%
remove:   5%
update:  10%
```
**Use Case**: Gathering large amounts of data under short bursts, rapid accumulation

### Benchmark Results
- **Hardware**: Apple M1 Pro (2021 14-inch MacBook Pro)
- **OS**: macOS 14.5
- **Metrics Tracked**: 
  - Throughput (operations per second)
  - Latency (operation time distribution)
- **Hash Functions Tested**: 
  - Standard Rust hasher (std)
  - AHash (faster hasher)
- **Result Format**: Charts comparing multiple concurrent map implementations

---

## 3. When to Use DashMap vs Alternatives

### ✅ Use DashMap When...

#### 3.1 Multi-threaded Concurrent Access
- **Best for**: Applications with many threads accessing/modifying the map simultaneously
- **Why**: Sharded locking allows concurrent access to different keys without blocking
- **Example**: Web servers with thread pools, high-concurrency services

#### 3.2 High-Performance Read-Heavy Scenarios
- **Best for**: Caching layers, lookup tables, web server caches
- **Why**: Read locks don't block each other within different shards
- **API Note**: Takes `&self` instead of `&mut self` for all operations, enabling `Arc<DashMap<K, V>>` patterns

#### 3.3 Exchange/Rotate Workloads
- **Best for**: Message passing queues, rotating data structures
- **Why**: Balanced insert/remove under reasonable load
- **Pattern**: Share across threads with `Arc<DashMap>`

#### 3.4 Rapid Growth Scenarios
- **Best for**: Accumulating data from multiple sources
- **Why**: Sharding distributes write contention
- **Pattern**: Multiple producers inserting simultaneously

#### 3.5 Drop-in Replacement Needed
- **Best for**: Migrating from `RwLock<HashMap<K, V>>`
- **Why**: Similar API, takes `&self` for all operations
- **Benefit**: Better performance without rewriting code

### ❌ Don't Use DashMap When...

#### 3.6 Single-Threaded Access
- **Use instead**: `std::collections::HashMap`
- **Why**: DashMap has overhead from sharding/locks not needed for single thread
- **Performance**: Standard HashMap will be faster

#### 3.7 Need Ordered Keys
- **Use instead**: BTreeMap or equivalent concurrent variant
- **Why**: DashMap is unordered (hash-based), no iteration order guarantees

#### 3.8 Strong Consistency with Global Snapshots Required
- **Consider**: You may need explicit synchronization
- **Note**: DashMap provides per-operation atomicity, not global snapshots
- **API**: Has iteration methods but iteration isn't a point-in-time snapshot

#### 3.9 Very Small Maps or Rare Access
- **Use instead**: `RwLock<HashMap<K, V>>`
- **Why**: Sharding overhead not worth the complexity for tiny maps
- **Threshold**: Generally for maps where contention is the problem

---

## 4. API Design for Concurrency

### All Methods Take `&self`
```rust
// Rather than:
map.insert(key, value);  // Takes &mut self

// DashMap uses:
map.insert(key, value);  // Takes &self
```

### Enables Arc Patterns
```rust
let map = Arc::new(DashMap::new());
let map_clone = Arc::clone(&map);

// Spawn thread and modify shared map
thread::spawn(move || {
    map_clone.insert("key", "value");
});
```

### Reference-Based Access
- `get()` returns `Option<Ref<'a, K, V>>` - immutable reference guard
- `get_mut()` returns `Option<RefMut<'a, K, V>>` - mutable reference guard
- These are smart pointers that release locks when dropped

---

## 5. Configuration Options

### Customization for Different Scenarios

```rust
// Default: auto-detects optimal shard count
let map = DashMap::new();

// Custom capacity pre-allocation
let map = DashMap::with_capacity(1000);

// Custom shard amount (power of 2)
let map = DashMap::with_shard_amount(32);

// Custom capacity + shard count
let map = DashMap::with_capacity_and_shard_amount(1000, 32);

// Custom hasher for domain-specific hashing
let map = DashMap::with_hasher(custom_hasher);
```

### Default Shard Amount Algorithm
```rust
(num_cpus * 4).next_power_of_two()
```
- Scales with CPU count
- Balances between contention reduction and overhead

---

## 6. Technical Details

### Sharded Locking Architecture
- Uses **per-shard RwLock** (read-write locks)
- Each key maps to exactly one shard via hash
- Multiple readers on **different shards** never block each other
- Readers on **same shard** DO block each other and writers
- Writers on **different shards** never block each other

### Lock Types
- Supports custom RwLock implementations
- Default: `parking_lot::RwLock` (optional, with default fallback)
- Raw API available (feature: `raw-api`) for advanced use cases

### Cache Optimization
- **CachePadded** used around each shard lock to prevent false sharing
- Reduces cache coherency protocol overhead on multi-CPU systems
- Important for performance on NUMA systems

---

## 7. Cargo Features

```toml
[dependencies]
dashmap = "5.5"  # Core functionality

# Optional features:
dashmap = { version = "5.5", features = ["serde"] }         # Serialization
dashmap = { version = "5.5", features = ["rayon"] }         # Parallel iteration
dashmap = { version = "5.5", features = ["raw-api"] }       # Internal shard access
dashmap = { version = "5.5", features = ["arbitrary"] }     # Fuzzing support
dashmap = { version = "5.5", features = ["inline-more"] }   # More inlining
```

### Key Feature: Rayon Support
- Parallel iteration capabilities
- `.par_iter()` for parallel reads
- Integrates with Rayon work-stealing scheduler

---

## 8. Stability and MSRV

- **Current MSRV (Minimum Supported Rust Version)**: 1.70
- **Policy**: Not changed in patch releases
- **Conservative Upgrade**: Always stays at least 1 year behind current stable Rust
- **Stability Promise**: Can pin minor version for perfect stability

---

## 9. Real-World Comparison Points

### vs `RwLock<HashMap<K, V>>`
| Aspect | RwLock<HashMap> | DashMap |
|--------|-----------------|---------|
| Lock Contention | High (single lock) | Low (sharded locks) |
| Read Throughput (high contention) | Poor | Excellent |
| Write Throughput (high contention) | Poor | Excellent |
| Memory Overhead | Lower | Higher (multiple locks) |
| Setup Complexity | Simple | Simple |
| Production Readiness | Yes | Yes |

### vs Concurrent HashMap Alternatives
- DashMap actively benchmarked against other concurrent hashmaps in `conc-map-bench`
- Competitive on read-heavy and exchange workloads
- Variable performance on rapid-grow (implementation dependent)

---

## 10. Use Case Examples

### ✅ Perfect For:
1. **Web Server Request Cache** - Read-heavy with occasional invalidation
2. **Connection Pool** - Insert/remove balanced workload  
3. **Rate Limiter** - High-throughput counter map
4. **DNS Cache** - Read-heavy with periodic updates
5. **Session Store** - Mixed reads/writes from multiple threads
6. **Metrics/Telemetry** - High-concurrency counter increments
7. **In-Memory Database Index** - Multiple reader threads with some writers

### ❌ Not Ideal For:
1. Single-threaded CLI tools
2. Ordered key requirements (use BTree variant)
3. Very small datasets (< 100 items, single thread)
4. Needing strict global consistency snapshots

---

## 11. Sources & Links

- **Official Repository**: https://github.com/xacrimon/dashmap
- **Benchmark Suite**: https://github.com/xacrimon/conc-map-bench
- **Crates.io**: https://crates.io/crates/dashmap
- **Docs.rs**: https://docs.rs/dashmap

---

## Key Takeaway

**DashMap is best described as:**
> A production-ready, blazing-fast concurrent hashmap optimized for multi-threaded scenarios with significant concurrent access. It trades slightly higher memory overhead and complexity for dramatically better performance under contention through intelligent sharded locking.

**Use it when you have concurrent access. Use `HashMap` when you don't.**
