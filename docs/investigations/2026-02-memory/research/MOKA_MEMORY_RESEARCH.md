# Moka Cache Memory Management Research

Research on moka-rs/moka memory overhead, weigher functions, and size calculations.

**Repository:** https://github.com/moka-rs/moka  
**Documentation:** https://docs.rs/moka/  
**Date:** January 2025

---

## Table of Contents

1. [Overview](#overview)
2. [Memory Overhead](#memory-overhead)
3. [Weigher Functions](#weigher-functions)
4. [Size Calculation Examples](#size-calculation-examples)
5. [Key GitHub Issues](#key-github-issues)
6. [Best Practices](#best-practices)
7. [ScreenerBot Usage Notes](#screenerbot-usage-notes)

---

## Overview

**Moka** is a fast, concurrent cache library for Rust inspired by Java's Caffeine cache. It provides:

- Thread-safe, highly concurrent in-memory caching (sync and async variants)
- Support for bounding by max entries OR weighted size
- Advanced eviction using TinyLFU (Least Frequently Used admission + LRU eviction)
- Expiration policies: TTL, TTI, per-entry variable expiration
- Eviction listeners/callbacks

**Key Features for Memory Management:**
- Custom weigher functions for size-aware eviction
- Configurable capacity limits
- Low memory overhead for eviction tracking

---

## Memory Overhead

### Per-Entry Overhead (64-bit platforms)

Based on maintainer's response in [Issue #201](https://github.com/moka-rs/moka/issues/201):

| Configuration | Overhead per Entry |
|--------------|-------------------:|
| **Without write-order queue** | **152 bytes** |
| **With write-order queue** | **184 bytes** |

**Write-order queue is enabled when:**
- `.time_to_live()` is configured
- `.support_invalidation_closures()` is called

### LFU Filter (CountMin Sketch) Overhead

The LFU filter is **created when cache reaches 50% full** to track entry frequency.

**Size Calculation:**
```rust
let estimated_max_num_entries: u64 = ((current_num_entries as f64
    * (current_total_weighted_size as f64 / max_capacity as f64))
    as u64)
    .max(128);  // minimum 128 entries

let byte_size = 8 * estimated_max_num_entries
    .min(2u64.pow(30))
    .next_power_of_two();
```

**Examples:**

| Estimated Max Entries | LFU Filter Size | Memory (MiB) |
|----------------------:|----------------:|-------------:|
| 1,000,000 | 8,388,624 bytes | 8 MiB |
| 5,000,000 | 67,108,880 bytes | 64 MiB |
| 10,000,000 | 134,217,744 bytes | 128 MiB |

### Total Memory Formula

```
Total Memory = Σ(key_size + value_size + per_entry_overhead) + lfu_filter_overhead
```

Where:
- `key_size` = size of key in memory (including heap allocations)
- `value_size` = size of value in memory (including heap allocations)
- `per_entry_overhead` = 152-184 bytes depending on configuration
- `lfu_filter_overhead` = 8-128+ MiB depending on estimated capacity

---

## Weigher Functions

### API Documentation

**Method Signature:**
```rust
pub fn weigher(self, weigher: impl Fn(&K, &V) -> u32 + Send + Sync + 'static) -> Self
```

**Purpose:**  
Enables size-aware eviction by calculating the weight/size of each cache entry.

**Parameters:**
- `weigher`: Closure that takes `&K` (key reference) and `&V` (value reference)
- **Returns:** `u32` representing the relative size of the entry
- **Requirements:** Must be `Send + Sync + 'static` for thread safety

### How It Works

1. **Without weigher:** `max_capacity(n)` means cache holds up to `n` entries (count-based)
2. **With weigher:** `max_capacity(n)` means cache holds entries with total weight up to `n` (weight-based)

**Important Notes:**
- Weight is calculated on-demand, not stored per-entry
- The weigher function is stored as `Arc<dyn Fn>` internally for shared ownership
- Eviction is triggered when total weight exceeds `max_capacity`

### Basic Example

```rust
use moka::sync::Cache;

fn main() {
    let cache = Cache::builder()
        // Weigher closure calculates size based on String byte length
        .weigher(|_key, value: &String| -> u32 {
            value.len().try_into().unwrap_or(u32::MAX)
        })
        // Cache holds up to 32 MiB of values
        .max_capacity(32 * 1024 * 1024)
        .build();
    
    cache.insert(0, "zero".to_string());
}
```

### Advanced Example: Including Key Size

```rust
use moka::sync::Cache;

fn main() {
    let cache: Cache<String, Vec<u8>> = Cache::builder()
        .weigher(|key: &String, value: &Vec<u8>| -> u32 {
            let key_size = key.len();
            let value_size = value.len();
            let overhead = 152; // Without write-order queue
            
            (key_size + value_size + overhead)
                .try_into()
                .unwrap_or(u32::MAX)
        })
        .max_capacity(100 * 1024 * 1024) // 100 MiB limit
        .build();
    
    cache.insert("example".to_string(), vec![0u8; 1024]);
}
```

### Weigher for Complex Types (JSON)

From [Issue #201](https://github.com/moka-rs/moka/issues/201), example for `serde_json::Value`:

```rust
use serde_json::Value;
use std::mem;

fn json_value_size(value: &Value) -> usize {
    match value {
        Value::Null => mem::size_of::<Value>(),
        Value::Bool(_) => mem::size_of::<Value>(),
        Value::Number(_) => mem::size_of::<Value>(),
        Value::String(s) => mem::size_of::<Value>() + s.capacity(),
        Value::Array(arr) => {
            mem::size_of::<Value>()
                + arr.capacity() * mem::size_of::<Value>()
                + arr.iter().map(json_value_size).sum::<usize>()
        }
        Value::Object(obj) => {
            mem::size_of::<Value>()
                + obj.capacity() * (mem::size_of::<String>() + mem::size_of::<Value>())
                + obj.iter().map(|(k, v)| k.capacity() + json_value_size(v)).sum::<usize>()
        }
    }
}

// Usage
let cache = Cache::builder()
    .weigher(|_key, value: &Value| -> u32 {
        json_value_size(value).try_into().unwrap_or(u32::MAX)
    })
    .max_capacity(50 * 1024 * 1024) // 50 MiB
    .build();
```

---

## Size Calculation Examples

### Simple Types

```rust
// For String values
.weigher(|_key, value: &String| value.len() as u32)

// For Vec<u8> values
.weigher(|_key, value: &Vec<u8>| value.len() as u32)

// For both key and value (String -> String)
.weigher(|key: &String, value: &String| {
    (key.len() + value.len()) as u32
})
```

### With Overhead Calculation

```rust
// Including Moka's per-entry overhead
.weigher(|key: &String, value: &String| {
    const OVERHEAD: usize = 152; // Or 184 with write-order queue
    (key.len() + value.len() + OVERHEAD) as u32
})
```

### For Owned Types (Heap-Allocated Structures)

```rust
use std::mem;

#[derive(Clone)]
struct MyData {
    id: u64,
    name: String,
    tags: Vec<String>,
}

impl MyData {
    fn heap_size(&self) -> usize {
        self.name.capacity()
            + self.tags.capacity() * mem::size_of::<String>()
            + self.tags.iter().map(|s| s.capacity()).sum::<usize>()
    }
}

let cache = Cache::builder()
    .weigher(|_key: &u64, value: &MyData| {
        let stack_size = mem::size_of::<MyData>();
        let heap_size = value.heap_size();
        let overhead = 152;
        
        (stack_size + heap_size + overhead) as u32
    })
    .max_capacity(200 * 1024 * 1024) // 200 MiB
    .build();
```

---

## Key GitHub Issues

### 🔥 Critical Issues

1. **[#572 - Cache memory limit with weigher function](https://github.com/moka-rs/moka/issues/572)** (Open)
   - User asking about calculating overhead for weigher functions
   - Requests documentation and helper functions

2. **[#543 - Cache Not Evicting Despite Exceeding max_capacity](https://github.com/moka-rs/moka/issues/543)** (Resolved)
   - **Root Cause:** Eviction is "piggybacked" during cache access, not automatic
   - **Solution:** Use `run_pending_tasks()` or ensure cache is actively accessed
   - Important for understanding eviction behavior

3. **[#201 - Help with size-based eviction weighing bytes](https://github.com/moka-rs/moka/issues/201)** (Open)
   - **Most comprehensive documentation** on memory overhead
   - Includes per-entry overhead calculations (152-184 bytes)
   - LFU filter size calculations
   - Example for `serde_json::Value` weigher

### 📊 Other Relevant Issues

4. **[#540 - Online resize](https://github.com/moka-rs/moka/issues/540)**
   - Feature request for dynamic capacity adjustment

5. **[#473 - Keys occupy double memory after updating](https://github.com/moka-rs/moka/issues/473)**
   - Memory leak issue (resolved in later versions)

6. **[#424 - Size Aware Eviction: FIFO vs largest-first](https://github.com/moka-rs/moka/issues/424)**
   - Discussion on eviction ordering strategies

---

## Best Practices

### 1. Choose the Right Capacity Mode

**Count-based (no weigher):**
```rust
// Good for uniform-sized entries
let cache = Cache::builder()
    .max_capacity(10_000) // 10,000 entries
    .build();
```

**Weight-based (with weigher):**
```rust
// Good for variable-sized entries
let cache = Cache::builder()
    .max_capacity(100 * 1024 * 1024) // 100 MiB
    .weigher(|_k, v: &Vec<u8>| v.len() as u32)
    .build();
```

### 2. Account for Overhead in Weigher

```rust
// More accurate memory accounting
.weigher(|key: &String, value: &Vec<u8>| {
    const PER_ENTRY_OVERHEAD: usize = 152;
    (key.len() + value.len() + PER_ENTRY_OVERHEAD) as u32
})
```

### 3. Handle Large Values

```rust
// Prevent u32 overflow for large values
.weigher(|_key, value: &Vec<u8>| {
    value.len().try_into().unwrap_or(u32::MAX)
})
```

### 4. Consider LFU Filter Overhead

For caches with millions of entries, remember to account for LFU filter overhead:

```rust
// If you want 100 MiB of actual data
// and expect 1 million entries:
// - Per-entry: 1M * 152 bytes = 145 MiB
// - LFU filter: ~8 MiB
// - Total: 100 + 145 + 8 = 253 MiB

let cache = Cache::builder()
    .max_capacity(253 * 1024 * 1024)
    .weigher(|k, v| calculate_size(k, v))
    .build();
```

### 5. Understanding Eviction Behavior

From [Issue #543](https://github.com/moka-rs/moka/issues/543):

**Important:** Eviction is "piggybacked" during normal cache access, not automatic background process.

```rust
// If you need immediate eviction:
cache.run_pending_tasks(); // Forces pending evictions

// Or configure eviction listener to monitor:
let cache = Cache::builder()
    .max_capacity(1000)
    .eviction_listener(|key, value, cause| {
        println!("Evicted: {:?} (cause: {:?})", key, cause);
    })
    .build();
```

### 6. Async Caches with Tokio

For async applications:

```rust
use moka::future::Cache;

#[tokio::main]
async fn main() {
    let cache: Cache<String, Vec<u8>> = Cache::builder()
        .max_capacity(50 * 1024 * 1024)
        .weigher(|k: &String, v: &Vec<u8>| {
            (k.len() + v.len() + 152) as u32
        })
        .build();
    
    cache.insert("key".to_string(), vec![0u8; 1024]).await;
}
```

---

## ScreenerBot Usage Notes

### Current State

**Moka is NOT currently used in ScreenerBot.** The codebase uses:
- **dashmap (v5.5)** - Concurrent hash map for shared state
- **r2d2 + r2d2_sqlite** - Connection pooling for SQLite database

### Research Documents in Repository

The following research files exist indicating past investigation:
- `MOKA_BENCHMARKS.md`
- `MOKA_VS_MINI_MOKA_RESEARCH.md`
- `moka_api_summary.md`

### Potential Use Cases for Moka

If considering Moka for ScreenerBot:

1. **Token Metadata Caching**
   - Cache token info with size-aware eviction
   - Weigher based on metadata size

2. **Pool Data Caching**
   - Cache pool snapshots with TTL
   - Weight by pool data structure size

3. **Transaction History Caching**
   - Recent transaction lookups
   - Weight by number of transactions

**Example Integration:**

```rust
use moka::sync::Cache;
use std::time::Duration;

// Token metadata cache with 50 MiB limit
let token_cache = Cache::builder()
    .max_capacity(50 * 1024 * 1024)
    .time_to_live(Duration::from_secs(300)) // 5 minutes
    .weigher(|_addr: &String, metadata: &TokenMetadata| {
        // Calculate size of metadata struct
        let overhead = 184; // With write-order queue (TTL enabled)
        (metadata.heap_size() + overhead) as u32
    })
    .eviction_listener(|addr, _metadata, cause| {
        tracing::debug!("Evicted token {} (cause: {:?})", addr, cause);
    })
    .build();
```

---

## References

- **GitHub Repository:** https://github.com/moka-rs/moka
- **API Documentation:** https://docs.rs/moka/
- **Crate Page:** https://crates.io/crates/moka
- **Wiki:** https://github.com/moka-rs/moka/wiki

### Key Issues Referenced

- [#201 - Help with size-based eviction](https://github.com/moka-rs/moka/issues/201) - Overhead calculations
- [#543 - Cache not evicting](https://github.com/moka-rs/moka/issues/543) - Eviction behavior
- [#572 - Cache memory limit](https://github.com/moka-rs/moka/issues/572) - Documentation request

---

**Last Updated:** January 2025  
**Researcher:** ScreenerBot Research Agent  
**Status:** Complete
