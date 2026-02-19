# Flurry Documentation Summary

## 1. Main Description/Overview

A concurrent hash table based on Java's `ConcurrentHashMap`.

A hash table that supports full concurrency of retrievals and high expected concurrency for updates. This type is functionally very similar to `std::collections::HashMap`, and for the most part has a similar API. Even though all operations on the map are thread-safe and operate on shared references, retrieval operations do not entail locking, and there is not any support for locking the entire table in a way that prevents all access.

**Version**: 0.5.2  
**License**: MIT OR Apache-2.0

### Notable Limitations
Flurry currently suffers from performance and memory usage issues under load. Alternative libraries to consider: **papaya** or **dashmap**.

---

## 2. Module Structure and Key Types

### Modules
- **iter**: Iterator types for traversing the concurrent hash table

### Core Structs

1. **HashMap** - The main concurrent hash table structure
2. **HashMapRef** - A reference to a HashMap (constructed with `HashMap::pin` or `HashMap::with_guard`)
3. **Guard** - A guard that keeps the current thread marked as active, enabling protected loads of atomic pointers
4. **HashSet** - A concurrent hash set implemented as a HashMap where the value is ()
5. **HashSetRef** - A reference to a HashSet (constructed with `HashSet::pin` or `HashSet::with_guard`)
6. **DefaultHashBuilder** - Default hash builder for HashMap
7. **DefaultHasher** - Default hasher for HashMap
8. **TryInsertError** - The error type for the `HashMap::try_insert` method

### Instantiation Options
- `new()` - Create a new map
- `with_capacity()` - Create a map with pre-allocated capacity

---

## 3. Architecture Details

### Internal Structure
- **Binned hash table** with buckets that can contain different types of nodes:
  - **BinEntry::Node** - Regular nodes with hash, key, value, and next field
  - **BinEntry::TreeNodes** - Organized in balanced red-black trees for collision resolution
  - **BinEntry::Tree** - Container bins that hold the roots of sets of TreeNodes
  - **BinEntry::Moved** - "Forwarding nodes" placed at bin heads during table resizing

### Concurrency Model
1. **Retrieval operations** (get, iterators) do NOT require locking
2. **Update operations** (insert, delete, replace) use per-bin locks
3. The first node in a bin list serves as the lock for that entire bin
4. When locked, nodes must re-validate they are still first in the bin before applying changes
5. New nodes are always appended to lists (maintaining order guarantees)

### Insertion Strategy
- **First insertion in empty bin**: Simple atomic compare-and-swap (CAS), most common operation
- **Other updates**: Require locks to prevent conflicts
- **Lock placement**: Embedded in first node to conserve space (no separate lock objects)

### Collision Resolution
- **Linear lists** for normal cases (Poisson distribution with λ=0.5 under random hashes)
- **Balanced red-black trees** when nodes in a bin exceed a threshold
- Tree operations bounded to O(log N) time, preventing worst-case scenarios
- Secondary strategy ensures search time remains reasonable even with poor hash distributions

---

## 4. Performance Notes

### Design Goals (in order)
1. **Maintain concurrent readability** while minimizing update contention
2. **Space consumption** similar to or better than `java.util.HashMap`
3. **High initial insertion rates** on empty table by many threads

### Performance Characteristics
- Lock contention probability for two threads accessing distinct elements: ~1/(8 * #elements) under random hashes
- Average bin size follows Poisson distribution with expected value 0.5
- Expected distribution at various bin sizes:
  - Size 0: 60.65%
  - Size 1: 30.33%
  - Size 2: 7.58%
  - Size 3: 1.26%
  - Size 4+: <0.16%

### Memory Management
- **Load factor threshold**: 0.75 (resizing when occupancy exceeds 75%)
- **Expected average**: Roughly 2 bins per mapping
- **Resizing cost**: May be relatively slow; use `with_capacity` when size estimate is available
- **Guard overhead**: Creating/dropping a Guard has overhead; trade-off between re-use and garbage accumulation

### Performance Optimization Tips
1. Use `with_capacity()` to provide size estimates when possible
2. Avoid using many keys with identical Hash values (will slow down any hash table)
3. Consider alternative libraries if performance under high load is critical
4. Reuse `Guard` objects to amortize creation/destruction costs

---

## 5. Threading Model

### Thread Safety
- **All operations are thread-safe** on shared references
- **No locking required for reads** (retrieval operations)
- **Per-bin locks for updates** only (not table-wide)

### Guard Mechanism
The `Guard` is fundamental to the threading model:
- Obtained via `HashMap::guard()`
- References tied to Guard lifetime
- Prevents destruction of items associated with the Guard
- Creates happens-before relationships for memory safety

**Key implications:**
- Delays deallocating values until safe
- May batch deallocations for efficiency
- Trade-off: Creating/dropping Guard has cost; keeping Guard long accumulates garbage
- Use best judgment for Guard re-use strategy

### Consistency Guarantees

**Retrieval Operations:**
- Generally do not block
- May overlap with update operations
- Reflect results of most recently completed update operations
- Update operations have happens-before relationship with successful retrievals

**Aggregate Operations (e.g., len, iterators):**
- Operate on snapshot of underlying table
- Iterators return elements at some point since creation
- Not suitable for program control under concurrent updates
- Useful for monitoring/estimation purposes

**Cloning:**
- May not produce "perfect" clone if map is being concurrently modified

### Resizing Mechanics
1. **Lazy initialization** to power-of-two size on first insertion
2. **Dynamic expansion** when table occupancy exceeds threshold
3. **Concurrent assistance**: Threads can help resize instead of stalling
4. **Forwarding nodes** (BinEntry::Moved) guide operations to new table
5. **Generation stamping** (size_ctl field) prevents overlapping resizes
6. **Power-of-two expansion**: Elements either stay at same index or move by power-of-two offset
7. **Node reuse optimization**: ~5/6 of nodes reused when table doubles

### Garbage Collection Strategy
Unlike Java's runtime GC, Rust uses **seize** library:
- **Batch reference-counting** garbage collection scheme
- **Requires Guard arguments** to many methods
- **Wrapped return values** for safe value access
- **More efficient** than per-value atomic reference counting
- Old table bins become garbage collectable after node transfers complete

---

## Dependencies

### Normal Dependencies
- **ahash** ^0.8.4 - Hash function
- **num_cpus** ^1.12.0 - CPU count detection
- **parking_lot** ^0.12 - Lock primitives
- **seize** ^0.3.3 - Garbage collection
- **rayon** ^1.3 (optional) - Parallel iteration
- **serde** ^1.0.105 (optional) - Serialization

### Development Dependencies
- criterion ^0.5 - Benchmarking
- rand ^0.8 - Random number generation
- rayon ^1.3 - Parallel testing
- serde_json ^1.0.50 - JSON serialization for tests

---

## Key Insights

1. **Direct port from Java**: Much of the design and documentation comes from Doug Lea's JSR166 implementation
2. **Lock-free reads**: Core innovation allowing concurrent reads without any synchronization
3. **Per-bin locking**: Efficient lock granularity that scales better than single lock
4. **Tree bins for collisions**: Prevents O(n) degradation in degenerate hash distributions
5. **Guard-based memory safety**: Unique Rust adaptation for safe concurrent access without GC
6. **Snapshot consistency**: Appropriate for monitoring but not program control under concurrency

