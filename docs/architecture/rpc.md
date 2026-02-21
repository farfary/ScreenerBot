# RPC Module — Architecture

> ScreenerBot Multi-Provider Solana RPC Client — February 2026

---

## Table of Contents

1. [Overview](#1-overview)
2. [File Structure](#2-file-structure)
3. [Provider System](#3-provider-system)
4. [Rate Limiting](#4-rate-limiting)
5. [Circuit Breaker](#5-circuit-breaker)
6. [Selection Strategies](#6-selection-strategies)
7. [Retry & Error Handling](#7-retry--error-handling)
8. [RPC Methods](#8-rpc-methods)
9. [Statistics & Monitoring](#9-statistics--monitoring)
10. [Database Schema](#10-database-schema)
11. [Module Connections](#11-module-connections)

---

## 1. Overview

The RPC module manages Solana JSON-RPC connections across multiple providers with automatic failover, per-provider rate limiting, circuit breaking, and configurable selection strategies. It ensures reliable chain access despite individual provider failures.

**Key characteristics:**
- Multi-provider with configurable rate limits per provider
- GCRA rate limiting (Governor crate)
- Circuit breaker pattern (Closed → Open → HalfOpen)
- Selection strategies: RoundRobin, Priority, LatencyBased, Adaptive
- 3 retries with exponential backoff
- Per-call statistics stored in SQLite

**25 files, ~7,409 lines**

---

## 2. File Structure

```
src/rpc/
├── mod.rs              # Module exports, global RPC client
├── client.rs           # RpcManager — multi-provider orchestrator
├── provider.rs         # RpcProvider, ProviderKind, configuration
├── rate_limiter.rs     # GCRA rate limiting per provider
├── circuit_breaker.rs  # Circuit breaker pattern
├── selection.rs        # Provider selection strategies
├── retry.rs            # Retry logic with backoff
├── health.rs           # Provider health monitoring
├── stats.rs            # Call statistics collection
├── stats_db.rs         # SQLite stats persistence
├── methods/            # Typed RPC method wrappers
│   ├── account.rs      # getAccountInfo, getMultipleAccounts
│   ├── balance.rs      # getBalance, getTokenAccountBalance
│   ├── transaction.rs  # sendTransaction, getTransaction, getSignaturesForAddress
│   ├── slot.rs         # getSlot, getBlockTime
│   ├── token.rs        # getTokenAccountsByOwner, getTokenLargestAccounts
│   └── program.rs      # getProgramAccounts (with filters)
└── errors.rs           # RPC error types
```

---

## 3. Provider System

### ProviderKind

| Kind | Default Rate (rps) | Purpose |
|------|-------------------|---------|
| `Helius` | 50 | Primary (with DAS API) |
| `QuickNode` | 25 | Secondary |
| `Triton` | 100 | High-throughput |
| `Alchemy` | 25 | Alternative |
| `Shyft` | 25 | Alternative |
| `Public` | 4 | Fallback only |
| `Custom` | Configurable | User-defined |

### Provider Configuration

```rust
pub struct RpcProviderConfig {
    pub name: String,
    pub url: String,
    pub kind: ProviderKind,
    pub rate_limit_rps: Option<u32>,     // Override default
    pub priority: u8,                     // Lower = higher priority
    pub is_enabled: bool,
    pub supports_das: bool,               // Metaplex DAS API
    pub max_batch_size: Option<usize>,
}
```

---

## 4. Rate Limiting

**Implementation:** GCRA (Generic Cell Rate Algorithm) via `governor` crate.

Each provider has its own rate limiter:

```rust
pub struct ProviderRateLimiter {
    limiter: RateLimiter<NotKeyed, InMemoryState, DefaultClock>,
    max_rps: u32,
}
```

| Behavior | Detail |
|----------|--------|
| Algorithm | GCRA (token bucket variant) |
| Granularity | Per-provider |
| Burst | 1.5× sustained rate |
| On limit | Queues request (with timeout) |
| Configuration | From `ProviderKind` defaults or explicit override |

---

## 5. Circuit Breaker

**Pattern:** Closed → Open → HalfOpen → Closed

```rust
pub struct CircuitBreaker {
    state: CircuitState,
    failure_count: u32,
    failure_threshold: u32,          // Default: 5
    success_threshold: u32,          // Default: 3 (in half-open)
    open_timeout: Duration,          // Default: 30s
    half_open_successes: u32,
    last_failure_time: Option<Instant>,
}
```

| State | Behavior |
|-------|----------|
| `Closed` | Normal operation, counting failures |
| `Open` | All calls fail fast, skip provider |
| `HalfOpen` | Allow limited calls to test recovery |

**Transitions:**
- Closed → Open: `failure_count >= failure_threshold`
- Open → HalfOpen: `open_timeout` elapsed
- HalfOpen → Closed: `half_open_successes >= success_threshold`
- HalfOpen → Open: Any failure

---

## 6. Selection Strategies

| Strategy | Algorithm | Best For |
|----------|-----------|----------|
| `RoundRobin` | Cycle through providers | Even distribution |
| `Priority` | Always use highest priority, fallback on failure | Cost control |
| `LatencyBased` | Prefer lowest p50 latency | Performance |
| `Adaptive` | Combine health, latency, error rate | Production default |

### Adaptive Scoring

```
score = (health_weight × health_score)
      + (latency_weight × latency_score)
      + (error_weight × error_score)
```

Providers below health threshold are excluded.

---

## 7. Retry & Error Handling

**Retry policy:** 3 attempts with exponential backoff.

```
Attempt 1: immediate
Attempt 2: 500ms delay
Attempt 3: 1000ms delay
```

**Error classification:**

| Error Type | Retry? | Action |
|------------|--------|--------|
| Rate limited (429) | Yes | Switch provider |
| Timeout | Yes | Switch provider |
| Server error (5xx) | Yes | Record failure |
| Client error (4xx) | No | Return error |
| Connection refused | Yes | Circuit break provider |

---

## 8. RPC Methods

### Account Operations

| Function | Solana RPC | Purpose |
|----------|-----------|---------|
| `get_account_info(pubkey)` | `getAccountInfo` | Single account data |
| `get_multiple_accounts(pubkeys)` | `getMultipleAccounts` | Batch (up to 100) |

### Balance Operations

| Function | Solana RPC | Purpose |
|----------|-----------|---------|
| `get_balance(pubkey)` | `getBalance` | SOL balance |
| `get_token_account_balance(ata)` | `getTokenAccountBalance` | SPL token balance |

### Transaction Operations

| Function | Solana RPC | Purpose |
|----------|-----------|---------|
| `send_transaction(tx)` | `sendTransaction` | Submit signed transaction |
| `get_transaction(sig)` | `getTransaction` | Transaction details |
| `get_signatures(addr, opts)` | `getSignaturesForAddress` | Recent signatures |

### Token Operations

| Function | Solana RPC | Purpose |
|----------|-----------|---------|
| `get_token_accounts_by_owner(owner)` | `getTokenAccountsByOwner` | All token accounts |
| `get_token_largest_accounts(mint)` | `getTokenLargestAccounts` | Top holders |

### Program Operations

| Function | Solana RPC | Purpose |
|----------|-----------|---------|
| `get_program_accounts(program, filters)` | `getProgramAccounts` | Filtered program accounts |

### Slot & Block

| Function | Solana RPC | Purpose |
|----------|-----------|---------|
| `get_slot()` | `getSlot` | Current slot |
| `get_block_time(slot)` | `getBlockTime` | Slot timestamp |

---

## 9. Statistics & Monitoring

### In-Memory Metrics (per provider)

```rust
pub struct ProviderStats {
    pub total_calls: u64,
    pub success_count: u64,
    pub error_count: u64,
    pub timeout_count: u64,
    pub rate_limited_count: u64,
    pub avg_latency_ms: f64,
    pub p50_latency_ms: f64,
    pub p99_latency_ms: f64,
    pub circuit_state: CircuitState,
    pub last_error: Option<String>,
    pub last_success: Option<DateTime<Utc>>,
}
```

### Persisted Stats

Per-call records written to `rpc_stats.db` for historical analysis. 72-hour retention with auto-cleanup.

---

## 10. Database Schema

**Database:** `rpc_stats.db`

### rpc_call_stats table

| Column | Type | Purpose |
|--------|------|---------|
| `id` | INTEGER PK AUTO | Record ID |
| `provider` | TEXT | Provider name |
| `method` | TEXT | RPC method name |
| `success` | INTEGER | 1=success, 0=failure |
| `latency_ms` | INTEGER | Call latency |
| `error` | TEXT | Error message if failed |
| `timestamp` | TEXT | Call time |

**Retention:** 72 hours with periodic cleanup.

---

## 11. Module Connections

```
rpc/
├── config/          ← Provider configuration
├── database/        ← SQLite infrastructure
└── errors/          ← Error types
```

| Caller | Usage |
|--------|-------|
| tokens | Metadata fetching, account queries |
| pools | Pool state fetching |
| transactions | Transaction sending and monitoring |
| wallets | Balance queries |
| filtering (onchain) | Mint/metadata account fetching |
| swaps/routers | Quote and swap transaction submission |
| connectivity | Health monitoring |
