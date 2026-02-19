# Moka - Performance and Benchmarks

## Overview

Moka is a fast, concurrent cache library for Rust inspired by the [Caffeine][caffeine-git] library for Java.

## Performance Highlights

### Key Performance Characteristics

Moka provides a rich and flexible feature set while maintaining:
- **High hit ratio** through sophisticated eviction policies
- **High level of concurrency** for concurrent access
- **Full concurrency of retrievals** and high expected concurrency for updates

### Admission and Eviction Policies

Moka maintains near optimal hit ratio by using entry replacement algorithms inspired by Caffeine:
- **Admission Control**: Least Frequently Used (LFU) policy
- **Eviction Strategy**: Least Recently Used (LRU) policy
- [More details and benchmark results are available at the TinyLFU wiki](https://github.com/moka-rs/moka/wiki#admission-and-eviction-policies)

## Performance Comparison Table

The following table shows the trade-offs between different cache implementations:

### Feature Comparison

| Feature | Moka v0.12 | Mini Moka v0.10 | Quick Cache v0.6 |
|:------- |:---- |:--------- |:----------- |
| Thread-safe, sync cache | ✅ | ✅ | ✅ |
| Thread-safe, async cache | ✅ | ❌ | ✅ |
| Non-concurrent cache | ❌ | ✅ | ✅ |
| Bounded by the maximum number of entries | ✅ | ✅ | ✅ |
| Bounded by the total weighted size of entries | ✅ | ✅ | ✅ |
| Near optimal hit ratio | ✅ TinyLFU | ✅ TinyLFU | ✅ S3-FIFO |
| Per-key, atomic insertion (e.g. `get_with` method) | ✅ | ❌ | ✅ |
| Cache-level expiration policies (TTL and TTI) | ✅ | ✅ | ❌ |
| Per-entry variable expiration | ✅ | ❌ | ❌ |
| Eviction listener | ✅ | ❌ | ✅ (via lifecycle hook) |
| Lock-free, concurrent iterator | ✅ | ❌ | ❌ |
| Lock-per-shard, concurrent iterator | ❌ | ✅ | ✅ |

### Performance and Resource Characteristics

| Performance, etc. | Moka v0.12 | Mini Moka v0.10 | Quick Cache v0.6 |
|:------- |:---- |:--------- |:----------- |
| Small overhead compared to a concurrent hash table | ❌ | ❌ | ✅ |
| Does not use background threads | ❌ → ✅ Removed from v0.12 | ✅ | ✅ |
| Small dependency tree | ❌ | ✅ | ✅ |

## Production Performance Data

### crates.io (The Official Rust Crate Registry)

Moka has been powering crates.io's API service since November 2021 to reduce loads on PostgreSQL:
- **Cache Hit Rate**: ~85% for the high-traffic download endpoint
- **Use Case**: Caching to reduce database load
- **Status**: In production through present day

### aliyundrive-webdav (WebDAV Gateway)

This WebDAV gateway for cloud drives has been deployed in hundreds of home Wi-Fi routers, including inexpensive models with 32-bit MIPS or ARMv5TE-based SoCs:
- **Use Case**: Caching remote file metadata
- **Status**: In production since August 2021 through present day

## Implementation Details

### Concurrency Model

- **Thread-safe**: Supports full concurrency of retrievals and high expected concurrency for updates
- **Lock-free Iteration**: Moka v0.12 provides lock-free, concurrent iterators for cache traversal
- **Background Threads**: Removed in v0.12.0 (performance optimization)
- **Async Support**: Native support for async/await runtimes (Tokio, async-std, actix-rt)

### Memory Management

To handle expensive-to-clone values efficiently:
- Wrap values in `std::sync::Arc` (thread-safe reference-counted pointer)
- `Arc::clone()` is a cheap operation, enabling efficient value sharing
- Automatically handles concurrent updates and value lifetime management

### Eviction Efficiency

- **Size-Aware Eviction**: Support for weighted cache entries with different memory footprints
- **Variable Expiration**: Per-entry expiration times using hierarchical timer wheels
- **Entry Replacement Algorithm**: TinyLFU (Tiny Least Frequently Used) inspired by Caffeine

## Use Case Selection

No cache implementation is perfect for every use case. Consider:

- **Choose Moka if you need:**
  - Async cache support with comprehensive features
  - Per-entry variable expiration
  - Eviction listeners/callbacks
  - Near-optimal hit ratio with TinyLFU
  - Lock-free concurrent iterators

- **Consider Mini Moka if you need:**
  - Simpler implementation
  - Smaller dependency tree
  - Don't need async support

- **Consider Quick Cache if you need:**
  - Minimal overhead over a concurrent hash table
  - Simpler implementation
  - S3-FIFO eviction policy

## References

- **TinyLFU Details**: https://github.com/moka-rs/moka/wiki#admission-and-eviction-policies
- **Caffeine Inspiration**: https://github.com/ben-manes/caffeine
- **Mini Moka**: https://crates.io/crates/mini-moka
- **Quick Cache**: https://crates.io/crates/quick_cache

---

**Note**: This document was compiled from the Moka README.md. For the most up-to-date information, visit:
- GitHub Repository: https://github.com/moka-rs/moka
- Crates.io: https://crates.io/crates/moka
- Documentation: https://docs.rs/moka
