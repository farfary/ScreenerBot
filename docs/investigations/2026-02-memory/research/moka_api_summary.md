# Moka Crate API Documentation Summary

## Version: 0.12.13

**Description:** A fast, concurrent cache library for Rust inspired by the Caffeine library for Java.

Moka provides in-memory concurrent cache implementations on top of hash maps. They support full concurrency of retrievals and a high expected concurrency for updates. They utilize a lock-free concurrent hash table as the central key-value storage.

---

## Core Features

- **Thread-safe, highly concurrent in-memory cache implementations:**
  - Synchronous caches that can be shared across OS threads
  - An asynchronous (futures aware) cache

- **Cache Bounding Options:**
  - The maximum number of entries
  - Maximum weight of entries
  - Time-based eviction (TTL and idle time)

- **Cache Policies:**
  - Least Recently Used (LRU)
  - Least Frequently Used (LFU)
  - Time-based expiration
  - Admission and eviction policies

---

## Main Modules

### 1. `sync` module
Provides thread-safe, concurrent cache implementations for synchronous use.

### 2. `future` module  
Provides a thread-safe, concurrent asynchronous (futures aware) cache implementation.

### 3. `notification` module
Common data types for notifications (sync or future).

### 4. `ops` module
Cache operations (sync or future).

### 5. `policy` module
Cache policy implementations (sync or future).

---

## Key Structs

- **Entry**: A snapshot of a single entry in the cache (available in both sync and future)

---

## Key Enums

- **PredicateError**: The error type for the functionalities around Cache::invalidate_entries_if method

---

## Key Traits

- **Equivalent**: Key equivalence trait for flexible key matching

---

## Dependencies

### Build Dependencies:
- async-lock ^3.3 (optional)
- crossbeam-channel ^0.5.15
- crossbeam-epoch ^0.9.18
- crossbeam-utils ^0.8.21
- equivalent ^1.0
- event-listener ^5.3 (optional)
- futures-util ^0.3.17 (optional)
- log ^0.4 (optional)
- parking_lot ^0.12
- portable-atomic ^1.6
- quanta ^0.12.2 (optional)
- smallvec ^1.8

### Dev Dependencies:
- tokio ^1.19
- loom ^0.7
- actix-rt ^2.8
- ahash ^0.8.3
- anyhow ^1.0.19
- env_logger ^0.10.0
- getrandom ^0.2
- once_cell ^1.7
- rand ^0.8.5
- reqwest ^0.12
- trybuild ^1.0

---

## License

Dual licensed under MIT OR Apache-2.0

---

## Repository & Links

- **GitHub:** https://github.com/moka-rs/moka
- **Crates.io:** https://crates.io/crates/moka
- **Documentation Coverage:** 93.1% of the crate is documented

---

## Platform Support

- aarch64-apple-darwin
- aarch64-unknown-linux-gnu
- i686-pc-windows-msvc
- x86_64-pc-windows-msvc
- x86_64-unknown-linux-gnu

---

## Key Implementation Details

### Concurrency
The cache uses lock-free concurrent hash tables for the central key-value storage, enabling:
- Full concurrency of retrievals
- High expected concurrency for updates

### Maintenance Tasks
The cache performs best-effort bounding using an entry replacement algorithm to determine which entries to evict when the capacity is exceeded.

---

## Minimum Supported Rust Versions

The crate supports specific MSRV levels (check the documentation for details).

