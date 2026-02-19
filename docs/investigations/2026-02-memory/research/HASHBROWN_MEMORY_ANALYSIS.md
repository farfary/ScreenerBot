# Hashbrown Memory Behavior Analysis

## Overview

DashMap uses `hashbrown::HashMap` as its underlying concurrent hash map implementation. Hashbrown is a Rust port of Google's high-performance SwissTable hash algorithm, and since Rust 1.36, it's been adopted in the Rust standard library.

## Key Memory Characteristics

### 1. **Memory Efficiency**
- **Low overhead**: Only 1 byte of overhead per entry (compared to 8 bytes in previous implementations)
- **Empty maps don't allocate**: `HashMap::new()` with capacity 0 allocates no memory
- **Capacity management**: Maps only allocate when first inserted into

### 2. **Clear Behavior (IMPORTANT FOR DASHMAP)**

```rust
/// Clears the map, removing all key-value pairs. Keeps the allocated memory
/// for potential reuse.
pub fn clear(&mut self) {
    self.table.clear();
}
```

**Key Point**: `clear()` **DOES NOT** deallocate memory. It retains the allocated capacity.

Example:
```rust
let capacity_before_clear = a.capacity();
a.clear();
assert_eq!(a.capacity(), capacity_before_clear);  // Capacity unchanged!
```

### 3. **Shrinking Methods**

#### `shrink_to_fit()`
- Shrinks the map's capacity to the absolute minimum needed for current elements
- Deallocates unused memory
- Implementation: Calls `shrink_to(0, ...)`

#### `shrink_to(min_capacity)`
- Shrinks capacity with a lower limit
- Won't shrink below the specified minimum capacity
- Maintains internal rules and may leave some space per the resize policy
- **Important**: Doesn't shrink if current capacity is already smaller than min_capacity

Example:
```rust
let mut map: HashMap<i32, i32> = HashMap::with_capacity(100);
map.insert(1, 2);
map.insert(3, 4);
assert!(map.capacity() >= 100);
map.shrink_to(10);
assert!(map.capacity() >= 10);  // Won't drop below 10
map.shrink_to(0);
assert!(map.capacity() >= 2);   // Won't drop below 2 (resize policy)
```

## Memory Allocation Structure

From `raw.rs`:
- RawTable tracks memory allocation using the allocator API
- Provides `allocation_size()` method to get total allocated memory
- Follows allocator-api2 for flexible allocator support
- Uses quadratic probing with SIMD lookups (memory efficient)

## Implications for DashMap

### Problem Areas Identified

1. **Lack of Automatic Shrinking**: 
   - Items removed via `remove()` or `clear()` don't trigger automatic deallocation
   - Memory accumulates in long-running applications where items are frequently removed

2. **`remove()` Behavior**:
   - Removes items and drops them
   - **Does NOT shrink the underlying capacity**
   - The bucket slot remains allocated but marked as deleted

3. **Drain Operations**:
   - `drain()` iterator keeps allocated memory
   - Capacity remains unchanged even when draining large portions
   - As documented: "Keeps the allocated memory for potential reuse"

### Why This Matters for Dashmap

DashMap wraps hashbrown::HashMap for each shard. In scenarios with:
- Frequent insertions and deletions
- Cyclical workloads (insert many, delete many, repeat)
- Long-running services with changing data volumes

Memory will **not be automatically reclaimed**. The capacity grows to accommodate peaks but shrinks back down only if explicitly called.

## Recommended Solutions for Dashmap

1. **Manual Shrinking**:
   ```rust
   // Call periodically for maps with churn
   dashmap.clear();  // Empty all data
   dashmap.shrink_to_fit();  // Reclaim memory
   ```

2. **Periodic Maintenance**:
   - Add optional `shrink_to_fit()` method to DashMap
   - Provide configuration for automatic shrinking policies

3. **Monitoring**:
   - Track allocation_size() to detect growing memory footprint
   - Alert when capacity significantly exceeds element count

4. **Alternative: Rebuild**:
   - For major cleanup operations, reconstruct the DashMap
   - This forces a fresh allocation

## Summary

**Hashbrown's behavior is intentional**: It prioritizes speed and reuse of allocated memory (avoiding reallocations during insertion-heavy workloads). However, this means **applications must explicitly call `shrink_to_fit()` to reclaim memory** from removed items.

This is documented and not a bug, but applications using DashMap need to be aware of this behavior, especially in memory-constrained environments or where the data volume fluctuates significantly.
