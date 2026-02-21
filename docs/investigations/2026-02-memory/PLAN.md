# ScreenerBot — Comprehensive Memory Investigation Report

> **STATUS: Phase A ✅ COMPLETED, Phase B ✅ COMPLETED, Phase C ✅ COMPLETED — See phase-a-summary.md, phase-b-summary.md, phase-b-test-results.md**
> Phase A: SQLite standardization + jemalloc + cleanup wiring.
> Phase B: All 14 unbounded caches bounded with moka. Memory now stable (not growing).
> Phase C: Unbounded caches bounded (API_RESPONSE, FAILED_CACHE), DB auto-vacuum maintenance, stale token SQL filter (7-day cutoff). RSS 1011→371 MB avg, 375 MB median, 483 MB peak. Tokens loaded 172K→15.6K (91% reduction). Target ≤400 MB ✅ MET.
> Phase D+ not yet started.
> This document serves as the master reference for the memory optimization project.

---

## ⚠️ READING GUIDE — Read This First

This plan was built iteratively (v1-v13). **Later versions supersede earlier ones where they conflict.**

### Authoritative Sections (start here for implementation):
| Section | Lines | What It Covers |
|---------|-------|----------------|
| **v10** | ~4461-4633 | TokenListEntry architecture (Option D) — **replaces all FilterToken references** |
| **v11** | ~4875-5226 | 12 corrections to file paths and phases |
| **v12** | ~5236-5574 | Flow details, preset workload justification, scheduling patterns |
| **v13** | ~5594-5906 | Non-DB memory deep-dive, corrected Token size, 14 new gaps |

### Known Stale Content in v1-v9 (do NOT implement as-is):
- **"FilterToken"** → replaced by **TokenListEntry** in v10. All FilterToken code/tests/phases are superseded. **UPDATE**: TokenListEntry was NOT implemented in Phase C. We used stale SQL filter instead (see line 1368).
- **Phase C "TokenListEntry + incremental filtering"** → NOT IMPLEMENTED. Actual Phase C: unbounded caches bounded, DB maintenance, stale SQL filter. See lines 1368-1407 for what was actually done.
- **Token size "~1,390 bytes"** → corrected to **~2,200 bytes** (78 fields) in v13. Filtering snapshot is ~120MB steady (not 238MB).
- **"171K tokens"** → corrected to **~56K tokens** loaded for filtering (only those with market data). **UPDATE**: After Phase C stale filter, now ~15.6K tokens (91% reduction).
- **Gap #7 "r2d2 has NO idle_timeout"** → corrected: r2d2 HAS idle_timeout (10min). Real fix: add with_init().
- **Phase A step A5** → cleanup_old_actions moved to Phase D (MaintenanceService) per v11.
- **Column "blacklisted_at"** → actual column is **"added_at"** (verified in schema.rs line 155).
- **Pressure levels** → standardize to 3 levels: Normal (<70%), Elevated (70-90%), Critical (>90%).

### Summary of Corrections Across Versions:
| Version | Key Changes |
|---------|-------------|
| v7 | Verified FETCH_LOCKS and COMPUTATION_FAILURES are BOUNDED (not leaks) |
| v9 | Column name correction (added_at not blacklisted_at) |
| v10 | FilterToken → TokenListEntry (Option D). ~550 bytes, 45 fields. NOT IMPLEMENTED. |
| v11 | 12 file/phase corrections. Phase C simplified to 7 files. |
| v12 | Preset workload justification. Scheduling pattern specified. |
| v13 | Token size 2,200 bytes. DB vs non-DB split (60/40). 14 new gaps. 31 cache inventory. |

---

---

## 📊 Executive Summary

**Problem**: Bot RSS starts at **804 MB** and grows unboundedly during 24/7 operation, reaching multiple GB.

**Root Causes Identified (in order of impact)**:

| # | Root Cause | Memory Impact | Category |
|---|-----------|--------------|----------|
| 1 | **SQLite page caches** — 14 databases × huge cache_size × connection pool multiplier | **580 MB at rest, up to 2.8 GB under load** | Configuration |
| 2 | **Filtering snapshot** — 171K full Token structs loaded every 3 min | **227 MB steady, 455 MB peak** | Architecture |
| 3 | **mmap_size 30 GB** on tokens.db & wallet_monitor.db | **Up to 294 MB in RSS** | Configuration |
| 4 | **Allocator fragmentation** — macOS system allocator never returns pages | **~200 MB wasted** | Runtime |
| 5 | **3 true leaks + 2 slow leaks + 8 bounded caches** — some grow with every token/transaction seen | **20-100 MB, some growing** | Code Leak |
| 6 | **Dashboard endpoint patterns** — loads ALL positions on every poll | **20-60% overhead when browsing** | Architecture |
| 7 | **Database files on disk** — pools.db 729 MB with 0 rows, rpc_stats 249 MB | **Disk waste, VACUUM needed** | Maintenance |

**Theoretical Maximum RSS**: With all connection pools fully active and dashboard polling:
- SQLite caches: ~1,500 MB
- Filtering: ~455 MB peak
- Allocator overhead: ~200 MB
- Global caches: ~100 MB
- Tokio/threads/embedded assets: ~100 MB
- **Total: ~2,355 MB (2.3 GB)** ← matches observed behavior

---

## 🔍 Detailed Investigation Results

### 1. SQLite Page Cache Analysis (THE #1 MEMORY CONSUMER)

Every SQLite connection maintains its own page cache. With r2d2 connection pools, this MULTIPLIES per connection.

**Formula**: `cache_size × page_size (4KB) × active_connections`

| Database | cache_size | Pool Max | Pool Min Idle | Cache/Conn | Worst Case | At Rest |
|----------|-----------|----------|---------------|------------|------------|---------|
| events.db (write) | 10,000 | 2 | 1 | 40 MB | 80 MB | 40 MB |
| events.db (read) | **20,000** | 10 | 1 | **80 MB** | **800 MB** | 80 MB |
| transactions.db | 10,000 | 10 | 0 | 40 MB | 400 MB | 40 MB |
| positions.db | 10,000 | 5 | 1 | 40 MB | 200 MB | 40 MB |
| tools.db | 10,000 | 10 | 2 | 40 MB | 400 MB | 80 MB |
| strategies.db | 10,000 | 10 | 0 | 40 MB | 400 MB | 40 MB |
| rpc_stats.db | 10,000 | 5 | 0 | 40 MB | 200 MB | 40 MB |
| wallets.db | 5,000 | 5 | 0 | 20 MB | 100 MB | 20 MB |
| wallet_monitor.db | 10,000 | 3 | 1 | 40 MB | 120 MB | 40 MB |
| tokens.db | 10,000 | 1 (Mutex) | 1 | 40 MB | 40 MB | 40 MB |
| ai/chat.db | 5,000 | 5 | 0 | 20 MB | 100 MB | 20 MB |
| ai/database.db | 5,000 | 1 | 0 | 20 MB | 20 MB | 20 MB |
| actions.db (write) | 10,000 | 2 | 1 | 40 MB | 80 MB | 40 MB |
| actions.db (read) | 10,000 | 10 | 1 | 40 MB | 400 MB | 40 MB |
| **TOTALS** | | **~79 conns** | | | **~2,860 MB** | **~580 MB** |

**Key Insights**:
- events.db read pool with 20,000 page cache × 10 connections = **800 MB alone!**
- Most databases use cache_size=10,000 (40MB/conn) — this is 5× the SQLite default of 2,000
- Under load (dashboard + filtering + trading), easily 30+ connections active = **1.2-1.5 GB**
- Idle connections still hold their page cache until the connection is dropped

### 2. mmap_size Configuration (Critical Misconfiguration)

| Database | mmap_size Setting | File Size | Impact |
|----------|------------------|-----------|--------|
| tokens.db | **30,000,000,000 (30 GB!)** | 294 MB | Maps entire file to RSS |
| wallet_monitor.db | **30,000,000,000 (30 GB!)** | ~5 MB | Excessive virtual space |
| transactions.db | 268,435,456 (256 MB) | ~50 MB | Reasonable |
| events.db | 268,435,456 (256 MB) | ~20 MB | Reasonable |
| All others | 0 (not set) | — | No mmap |

**Problem**: 30 GB mmap on tokens.db means the OS memory-maps the entire 294 MB file. These pages show up in RSS and compete with real memory. On a machine with limited RAM, this creates significant memory pressure.

### 3. Filtering Pipeline (227 MB steady, 455 MB peak)

**Flow**: Every 180 seconds, `compute_snapshot()` in `filtering/engine.rs`:
1. Calls `get_all_tokens_for_filtering_async()` → loads 171K Token structs
2. Each Token: ~1,390 bytes (with empty Vecs for security_risks, top_holders, etc.)
3. Total: 171K × 1,390 bytes = **~238 MB**
4. Wraps each in `Arc<Token>` (no clone, move semantics — good)
5. Creates `HashMap<String, Arc<Token>>` + `Vec<PassedToken>` + `Vec<RejectedToken>`
6. Old snapshot exists alongside new during compute → **2× memory = ~455 MB peak**
7. Old snapshot dropped after swap

**Fields actually used by filter sources**:
- dexscreener.rs: price, volume_h6/h24, txns, liquidity, mcap, fdv, pair_created_at (~15 fields)
- geckoterminal.rs: same price/volume/mcap subset (~12 fields)
- rugcheck.rs: score, mint_authority, freeze_authority, top_holders count, transfer_fees (~8 fields)
- meta.rs: mint, first_discovered_at (~2 fields)
- ai.rs: serializes ENTIRE Token to JSON (all 60+ fields!) — wasteful

**Optimization potential**: A `FilterToken` struct with only ~35 needed fields → ~300-400 bytes each
→ 171K × 350 bytes = **60 MB** (vs 238 MB) — **saves ~178 MB per snapshot**

### 4. Unbounded Global Caches (3 True Leaks + 2 Slow Leaks)

> **v7 CORRECTION**: Deep source code verification found that FETCH_LOCKS and COMPUTATION_FAILURES
> are NOT leaks. FETCH_LOCKS has per-use release + 10K safety cap. COMPUTATION_FAILURES uses
> static &str keys (fixed set of ~5-10 window names) with reset on success. TOKEN_POOLS_CACHE
> and POOL_PREFETCH_STATE are "slow leaks" (no periodic eviction, but have manual clear).

| Cache | Location | Size at Startup | Growth Rate | Max Potential | Status |
|-------|----------|----------------|-------------|---------------|--------|
| DECIMALS_CACHE | tokens/decimals.rs:30 | 242K entries (~20 MB) | +1 per new token | Unlimited | UNBOUNDED (has clear, no eviction) |
| FAILED_CACHE | tokens/decimals.rs:38 | 0 | +1 per failed lookup | Unlimited | UNBOUNDED (no eviction) |
| TOKEN_2022_CACHE | tokens/decimals.rs:43 | Small | +1 per check | Unlimited | **TRUE LEAK** (no removal) |
| GLOBAL_KNOWN_SIGNATURES | transactions/utils.rs:58 | 0 | +1 per transaction | ~88 bytes/sig | UNBOUNDED (no eviction) |
| GLOBAL_PENDING_TRANSACTIONS | transactions/utils.rs:62 | 0 | +1 per pending | 180s TTL (manual) | BOUNDED (manual TTL) |
| POSITION_LOCKS | positions/state.rs:22 | ~3 | +1 per mint traded | Unlimited | **TRUE LEAK** (no removal) |
| ACTIVE_ACTIONS | actions/state.rs:15 | ~recent incomplete | +1 per action created | Unlimited | **TRUE LEAK** (never removes completed) |
| LAST_TOKEN_ACCOUNTS_CHECK | positions/verifier.rs:22 | 0 | +1 per mint checked | Unlimited | UNBOUNDED (no eviction) |
| FETCH_LOCKS | tokens/decimals.rs:34 | 0 | +1/-1 per fetch | ≤10K (safety cap) | **BOUNDED** (per-use release + 10K cap) |
| TOKEN_POOLS_CACHE | tokens/pool_data/cache.rs:38 | 0 | +1 per pool lookup | Unlimited | **SLOW LEAK** (has TTL check but no eviction) |
| POOL_PREFETCH_STATE | tokens/pool_data/cache.rs:44 | 0 | +1 per prefetch | Unlimited | **SLOW LEAK** (only cleared with pool cache) |
| COMPUTATION_FAILURES | wallets/balance_monitor/cache.rs:118 | 0 | Fixed key set | ~5-10 entries | **BOUNDED** (static &str keys + reset on success) |

**After 24h with 275K tokens**: DECIMALS_CACHE alone could be 300K+ entries (30+ MB).
GLOBAL_KNOWN_SIGNATURES seeing ~1000 tx/hr = 24K entries/day × 88 bytes = 2 MB/day growth (slow but unbounded).

### 5. Dashboard Memory Overhead (20-60% additional)

**Heaviest endpoints** (called on dashboard polling):

| Endpoint | Data Loaded | Memory Per Call |
|----------|-------------|-----------------|
| `GET /api/dashboard/home` | 5 parallel position queries + ALL open positions + filtering stats | ~250 KB |
| `GET /api/dashboard/overview` | ALL open + ALL closed positions, mapped to detail structs | ~150 KB × position count |
| `GET /api/tokens/:mint` | Full Token via `get_full_token_async()` (all Vecs populated) | ~100-200 KB |

**Critical pattern**: `get_db_open_positions()` and `get_db_closed_positions()` load ALL positions with NO pagination on every dashboard poll (every 5-30 seconds).

With 50 open + 200 closed positions, each overview poll allocates ~12 MB in response serialization.

### 6. Database Disk Waste

| Database | File Size | Rows | Free Pages | Waste % | Action |
|----------|-----------|------|------------|---------|--------|
| pools.db | **729 MB** | **0 rows** | 186,566/186,589 | **99.99%** | VACUUM → ~94 KB |
| ohlcvs.db | 354 MB | 270K candles | 37,894 (42%) | 42% | VACUUM → ~200 MB |
| rpc_stats.db | 249 MB | 602K rows | 0 | 0% | Needs rotation + VACUUM |
| tokens.db | 294 MB | 275K tokens | 0 | 0% | OK (but no GC for old tokens) |

**Total recoverable disk**: ~880 MB from VACUUM + rotation

### 7. Allocator Fragmentation (~200 MB)

macOS system allocator (`malloc`) does not return freed pages to the OS after large allocations.
The filtering pipeline creates 238 MB → frees it → creates 238 MB → frees it... each cycle can fragment differently.
After several cycles, RSS stays at the peak allocation even though actual usage is lower.

**Solution**: Switch to jemalloc or mimalloc — both have better page return behavior for long-running processes.

---

## 🏗️ Product-Grade Memory Architecture (v2 — Critically Reviewed)

> **Design Principle**: These solutions must work universally for ALL users — from a $5/mo 2GB VPS 
> to a 64GB workstation — without manual configuration. The bot must be self-managing for 24/7 
> operation, self-healing under memory pressure, and prevent future developers from introducing leaks.
>
> **v2 CHANGES**: Each component now includes critical analysis, identified cons, and solutions 
> for those cons. Over-engineered parts simplified. New findings from pool utilization analysis 
> incorporated. Added SQL pre-filtering as a new component.

### The Core Problem

ScreenerBot has **no concept of how much memory it should use**. Every cache, database, and data 
structure grows independently with no coordination, no budget, and no cleanup. This is not a bug — 
it's a **missing architectural layer**.

A user running on a 2GB VPS gets the same 10,000-page SQLite cache as someone on a 64GB machine.
A bot running for 6 months accumulates 500K+ tokens in unbounded caches with zero eviction.
Database files grow to gigabytes with no maintenance.

---

### Architecture Component 1: Right-Sized SQLite Configuration

**REVISED FROM**: "Adaptive Memory Budget System" — the budget system was over-engineered.

**What changed and why**: The original plan calculated SQLite cache sizes from a percentage of system 
RAM, with profiles and dynamic budgets. After deeper analysis, this is unnecessary complexity because:

1. **r2d2 pools are LAZY** — connections are created on first checkout, not at pool creation. 
   `min_idle` only maintains minimum AFTER first use.
2. **Steady state is ~15 active connections**, not the theoretical max of 79.
3. **SQLite cache_size is a MAX, not a pre-allocation** — it grows only as data is queried.
4. **mmap already handles read caching** at the OS level when enabled — SQLite's own page cache 
   is partially redundant with mmap.
5. **Different databases have wildly different workloads** — tokens.db does bulk queries on 171K 
   rows, events.db mostly appends, positions.db has tens of rows.

**The simpler, correct approach**: Set cache sizes PER DATABASE based on actual workload, with a 
shared configuration function. No runtime detection, no profiles, no percentages.

```
Database Configurations (based on workload analysis):

HOT (heavy read queries, large datasets):
  tokens.db:        cache_size = 5,000 (20 MB) — 171K-row joins for filtering
  transactions.db:  cache_size = 3,000 (12 MB) — real-time monitoring queries

STANDARD (moderate read/write, medium datasets):
  positions.db:     cache_size = 1,000 (4 MB) — small dataset, frequent access
  events.db write:  cache_size = 1,000 (4 MB) — append-heavy
  events.db read:   cache_size = 2,000 (8 MB) — recent events only (was 20,000!)
  actions.db write: cache_size = 1,000 (4 MB)
  actions.db read:  cache_size = 2,000 (8 MB)

COLD (rarely queried, small datasets):
  rpc_stats.db:     cache_size = 500 (2 MB) — mostly append
  wallets.db:       cache_size = 500 (2 MB) — tiny dataset
  wallet_monitor.db: cache_size = 1,000 (4 MB)
  strategies.db:    cache_size = 500 (2 MB)
  tools.db:         cache_size = 500 (2 MB)
  ai/chat.db:       cache_size = 500 (2 MB)
  ai/database.db:   cache_size = 500 (2 MB)
  ohlcvs.db:        cache_size = 2,000 (8 MB) — chart data queries

TOTAL MAX (all connections at max):  ~84 MB (was ~2,860 MB!)
TYPICAL (steady-state ~15 conns):    ~40-60 MB
```

**Connection pool right-sizing** (SQLite is single-writer, most pools are oversized):
```
events.db read:     10 → 4 connections (events rarely read in parallel)
actions.db read:    10 → 4 connections
transactions.db:    10 → 5 connections
tools.db:           10 → 3 connections
strategies.db:      10 → 3 connections
```

**mmap_size fix**:
```
tokens.db:        30 GB → 256 MB (file is 294 MB)
wallet_monitor.db: 30 GB → 32 MB (file is ~5 MB)
transactions.db:   256 MB → keep
events.db:         256 MB → 128 MB
All others:        0 → keep at 0 (small files, mmap not needed)
```

**Shared initialization function**:
```rust
pub struct DbConnectionConfig {
    pub cache_size_pages: i32,
    pub mmap_size_bytes: i64,
    pub wal_enabled: bool,
    pub auto_vacuum_incremental: bool,
}

pub fn configure_sqlite_connection(conn: &Connection, config: &DbConnectionConfig) -> Result<()> {
    if config.wal_enabled {
        conn.pragma_update(None, "journal_mode", "WAL")?;
    }
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    conn.pragma_update(None, "busy_timeout", 30000)?;
    conn.pragma_update(None, "temp_store", "MEMORY")?;
    conn.pragma_update(None, "cache_size", config.cache_size_pages)?;
    conn.pragma_update(None, "mmap_size", config.mmap_size_bytes)?;
    if config.auto_vacuum_incremental {
        conn.pragma_update(None, "auto_vacuum", "INCREMENTAL")?;
    }
    Ok(())
}
```

✅ **PROS**:
- Simple and predictable — no runtime calculation, just correct values
- Every user gets the same optimized configuration
- Easy to test, easy to reason about
- Shared function prevents inconsistency between databases
- Massive reduction: ~2,860 MB theoretical → ~84 MB theoretical

❌ **CONS**:
- A user with 64GB RAM doesn't get bigger caches (but mmap + OS page cache handle this)
- Hardcoded values might not be optimal for extreme data volumes (1M+ tokens)

🔧 **CON MITIGATION**:
- Optional [performance] config section for power users: `sqlite_cache_multiplier = 2.0`
- Default 1.0 works for everyone; power users can increase if they have RAM to spare
- This is simpler than auto-detection and puts the user in control

---

### Architecture Component 2: Incremental Filtering (NOT full reload)

**What**: Replace the full 171K-token reload every 180 seconds with an incremental delta update 
that only fetches tokens that changed since the last refresh.

**Why this is the right solution**: Loading 171K tokens × 1,390 bytes = 238 MB every 3 minutes is 
the single biggest memory spike. It affects EVERY user regardless of hardware. Even on a 64GB machine, 
it's wasteful. The incremental approach reduces per-refresh memory from 238 MB to ~2-10 MB.

**Why it's feasible**: The tokens database already has `market_data_last_updated_at` and 
`security_data_last_updated_at` timestamps in the `update_tracking` table. The infrastructure exists.

**How it works** (REVISED — maintains immutable snapshot pattern):
```
Cold Start (first load):
  1. Load ALL tokens with market data (same as current)
  2. Store snapshot + record timestamp as `last_full_refresh`
  3. Memory: 238 MB (same as now, but only ONCE)

Incremental Refresh (every 180 seconds):
  1. Clone existing snapshot's HashMap structure (CHEAP: only Arc pointers + HashMap overhead)
     171K entries × ~48 bytes (key + Arc pointer) = ~8 MB
  2. Query delta: SELECT * WHERE update_tracking.market_data_last_updated_at > last_refresh
     Returns ~100-2000 tokens (those updated in last 3 min)
  3. Query new: SELECT * WHERE first_discovered_at > last_refresh
  4. Query blacklisted: SELECT mint WHERE blacklisted_at > last_refresh
  5. Apply deltas to cloned HashMap:
     - Updated tokens: re-evaluate all filters, insert if pass, remove if fail
     - New tokens: evaluate filters, insert if pass
     - Blacklisted: remove from snapshot
  6. Build new FilteringSnapshot from modified HashMap
  7. Swap atomically (existing immutable snapshot pattern preserved)

Memory per refresh:
  - HashMap clone: ~8 MB (Arc pointer copies, not data copies)
  - Delta tokens: ~100-2000 × 1,390 bytes = ~140 KB - 2.8 MB
  - Total per refresh: ~10 MB (not 238 MB!)
  
Peak memory (old + new coexist briefly):
  - 238 MB (old) + ~10 MB (new clone + deltas) = ~248 MB
  - vs current: 238 MB + 238 MB = 476 MB peak
  - Savings: ~228 MB peak reduction

Safety nets:
  - Full refresh every 30 minutes (catches any drift, blacklist changes, edge cases)
  - If delta count > 50% of total tokens, auto-trigger full refresh (something unusual happened)
  - Config change → trigger full refresh immediately
  - Log: "Incremental refresh: +{new} updated, +{added} new, -{removed} removed ({delta_count} total deltas)"
```

✅ **PROS**:
- 96-99% reduction in per-refresh DB query size
- ~228 MB peak memory reduction
- Less DB I/O = less SSD wear on VPS
- Eliminates the repeated 238 MB alloc/free cycle that causes allocator fragmentation
- Preserves existing immutable snapshot semantics (readers hold Arc independently)
- Existing update_tracking timestamps make this straightforward

❌ **CONS**:
- **Consistency risk**: Token deleted from DB between refreshes stays in snapshot until safety net
- **Merge complexity**: Must handle updates + inserts + removals correctly
- **Config change detection**: When user changes filter thresholds, old snapshot has wrong pass/fail decisions
- **Blacklist edge case**: Token blacklisted between refreshes needs explicit removal query
- **Testing complexity**: More states to test than simple full reload

🔧 **CON MITIGATIONS**:
- **Consistency**: 30-minute full refresh catches any drift. Also: if delta > 50% → full refresh
- **Merge complexity**: Well-defined 3-operation merge (update/insert/remove), testable independently
- **Config change**: Already handled — config changes trigger compute_snapshot() which we make do full refresh
- **Blacklist**: Explicit blacklist delta query (cheap: few entries per cycle)
- **Testing**: Add unit tests with controlled snapshots: test add, test update, test remove, test mixed

---

### Architecture Component 3: FilterToken Lightweight Struct

**What**: A minimal struct containing only the ~35 fields actually used by filter sources, 
instead of the full 60+ field Token struct.

**Why this is the right solution**: Even with incremental updates, the initial snapshot holds 171K 
tokens. At 1,390 bytes each, that's 238 MB. With only ~350 bytes of needed fields, it's 60 MB. 
This benefits every user's baseline memory usage, regardless of RAM.

**Why separate from Component 2**: Incremental updates reduce refresh cost; FilterToken reduces 
the size of the base snapshot. They MULTIPLY: 171K × 350 bytes = 60 MB baseline + ~3 MB refreshes.
Combined savings: from 238 MB baseline + 238 MB peak to 60 MB baseline + 63 MB peak.

**Fields needed by filter sources** (from code analysis of each source):
```rust
pub struct FilterToken {
    // Identity (2 fields) — used by meta.rs
    pub mint: String,                          // 24 + ~44 bytes (typical Solana address)
    pub first_discovered_at: Option<i64>,      // 16 bytes
    
    // DexScreener market data (15 fields) — used by dexscreener.rs
    pub ds_price_usd: Option<f64>,             // 16 bytes each
    pub ds_price_native: Option<f64>,
    pub ds_volume_h1: Option<f64>,
    pub ds_volume_h6: Option<f64>,
    pub ds_volume_h24: Option<f64>,
    pub ds_txns_h1_buys: Option<i64>,
    pub ds_txns_h1_sells: Option<i64>,
    pub ds_txns_h6_buys: Option<i64>,
    pub ds_txns_h6_sells: Option<i64>,
    pub ds_txns_h24_buys: Option<i64>,
    pub ds_txns_h24_sells: Option<i64>,
    pub ds_liquidity_usd: Option<f64>,
    pub ds_mcap: Option<f64>,
    pub ds_fdv: Option<f64>,
    pub ds_pair_created_at: Option<i64>,
    
    // GeckoTerminal market data (~10 fields) — used by geckoterminal.rs
    pub gt_price_usd: Option<f64>,
    pub gt_volume_h24: Option<f64>,
    pub gt_mcap: Option<f64>,
    pub gt_fdv: Option<f64>,
    pub gt_reserve_usd: Option<f64>,
    // ... (similar price/volume/mcap subset)
    
    // Rugcheck security (8 fields) — used by rugcheck.rs
    pub rc_score: Option<f64>,
    pub rc_mint_authority: Option<String>,
    pub rc_freeze_authority: Option<String>,
    pub rc_top_holders_count: Option<i64>,
    pub rc_top_holders_pct: Option<f64>,
    pub rc_transfer_fee_enabled: Option<bool>,
    pub rc_is_mintable: Option<bool>,
    pub rc_is_mutable: Option<bool>,
}
// ~350 bytes per instance (vs 1,390 for full Token with empty Vecs)
```

**AI filter special handling**: AI filtering runs LAST, only on tokens that already passed ALL other 
filters (confirmed: Meta → DexScreener → GeckoTerminal → Rugcheck → AI, with short-circuit on 
first rejection). Typically 2.8K-5.6K tokens reach AI. For these, AI calls `get_full_token()` 
individually to get complete data for LLM analysis. This is acceptable because:
- AI is rate-limited anyway (50 req/min for Assistant, similar for other providers)
- Single-row DB lookup by mint (indexed) = ~1-5ms per token
- Most users have AI filtering disabled (it's opt-in)

✅ **PROS**:
- 178 MB baseline reduction (238 MB → 60 MB) for ALL users
- Faster DB query (SELECT only 35 columns, skip 6 heavy joins for security/holder data)
- Cleaner separation: filtering data vs display data
- SQL query is simpler and faster (no LEFT JOINs for security_risks, top_holders, websites, socials)

❌ **CONS**:
- **Maintenance burden**: Two structs (Token + FilterToken) must stay in sync when adding new filter criteria
- **Future filter sources**: New filter source needing a field not in FilterToken requires updating both struct + SQL
- **Code duplication**: Two similar but different SQL queries for token loading

🔧 **CON MITIGATIONS**:
- **Maintenance**: Comment block at top of FilterToken lists which source uses which field
- **Future sources**: Document clearly in Assistant-instructions.md: "New filter sources → update FilterToken"
- **Code duplication**: The FilterToken SQL is a STRICT SUBSET of the Token SQL — extract shared query builder
- **Compile-time safety**: If filter source code references a field not in FilterToken, it won't compile

---

### Architecture Component 4: Bounded Caches via moka

**REVISED FROM**: "Managed Cache Registry with custom ManagedCache trait" — the trait system was 
over-engineered.

**What changed and why**: The original plan required every cache to implement a custom `ManagedCache` 
trait with a central `CacheRegistry`. After investigation:
- The bot already has a **custom TimedCache** (HashMap + VecDeque + Mutex) for TokenStore LRU caches
- DashMap is used for unbounded caches (DECIMALS, KNOWN_SIGNATURES, etc.)
- The `moka` crate provides all needed functionality: LRU, TTL, max entries, weighted size, 
  concurrent access, entry_count()/weighted_size() for metrics — OUT OF THE BOX
- A custom trait adds complexity without adding value over moka's built-in features

**The simpler approach**: Replace bare DashMap/HashMap with `moka::sync::Cache` or 
`moka::future::Cache` where bounded behavior is needed. Keep DashMap where unbounded is correct.

```
Cache Migration Plan:

REPLACE with moka (currently unbounded → bounded):
  DECIMALS_CACHE:                HashMap<String, u8>     → moka::sync::Cache (max: 150K, no TTL)
  FAILED_CACHE:                  HashSet<String>          → moka::sync::Cache (max: 10K, TTL: 5 min)
  TOKEN_2022_CACHE:              HashSet<String>          → moka::sync::Cache (max: 50K, no TTL)
  GLOBAL_KNOWN_SIGNATURES:       HashSet<String>          → moka::sync::Cache (max: 50K, no TTL)
  GLOBAL_PENDING_TRANSACTIONS:   HashMap<String, ...>     → moka::sync::Cache (max: 10K, TTL: 180s)
  POSITION_LOCKS:                HashMap<String, Mutex>   → clean up on position close (not moka)
  LAST_TOKEN_ACCOUNTS_CHECK:     HashMap<String, Instant> → moka::sync::Cache (max: 5K, TTL: 1 hour)
  DEFERRED_RETRIES:              HashMap<String, ...>     → moka::sync::Cache (max: 1K, TTL: 5 min)

REPLACE custom TimedCache with moka (better implementation):
  DEXSCREENER_CACHE:   TimedCache(2000, 30s)  → moka::sync::Cache(max: 2000, TTL: 30s)
  GECKOTERMINAL_CACHE: TimedCache(2000, 60s)  → moka::sync::Cache(max: 2000, TTL: 60s)
  RUGCHECK_CACHE:      TimedCache(3000, 300s) → moka::sync::Cache(max: 3000, TTL: 300s)

KEEP as DashMap (bounded by design or needs DashMap-specific features):
  PRICE_CACHE:   DashMap — bounded by pool count (cleanup_stale_entries exists)
  PRICE_HISTORY: DashMap — bounded by open position count + add cleanup on position close

SPECIAL: POSITION_LOCKS
  Not a cache — it's a concurrency primitive (per-mint Mutex)
  Fix: remove lock when position is closed (currently never removed)
  This is a code fix, not a cache migration
```

**For observability** (without a custom registry): The maintenance service polls each cache's 
`entry_count()` and `weighted_size()` (moka provides both). Reports to dashboard via 
`/api/system/memory` endpoint.

✅ **PROS**:
- moka is battle-tested (used by major Rust projects), W-TinyLFU eviction > simple LRU
- Drop-in replacement for DashMap/HashMap use patterns (similar .get()/.insert() API)
- Built-in TTL, max entries, weighted size — no custom code needed
- Removes 200+ lines of custom TimedCache implementation
- Prevents unbounded growth permanently for ALL users
- Thread-safe (sync::Cache uses sharded mutexes internally, like DashMap)

❌ **CONS**:
- **New dependency**: `moka` crate adds to compile time and binary size
- **API differences**: moka's get() returns Option<V> (cloned value), not a reference — this 
  matters for large values
- **DECIMALS_CACHE migration**: Currently 242K entries; if max set to 150K, 92K entries get evicted 
  at startup. Some cache misses → DB lookups (extra latency per miss: ~1-5ms)
- **POSITION_LOCKS**: Can't use moka — Mutex values aren't Clone. Need manual cleanup.

🔧 **CON MITIGATIONS**:
- **Dependency**: moka is widely used (~12M downloads), well-maintained, adds ~2s to compile
- **API**: For DECIMALS_CACHE (u8 values), clone is trivial. For larger values, use Arc wrapping
- **Eviction latency**: DECIMALS_CACHE at 150K covers the hot set. Cold tokens rarely needed for 
  decimals. And the fallback (DB lookup) is fast. Can tune max up if metrics show high miss rate
- **POSITION_LOCKS**: Simple fix — when position closes in operations.rs, also remove from POSITION_LOCKS

---

### Architecture Component 5: Automatic Maintenance Service

**What**: A new service in the ServiceManager that runs periodic database and cache maintenance. 
Zero user intervention required. Configurable intervals.

**Why this is the right solution**: Users don't know about SQLite VACUUM, WAL checkpoints, or 
RPC stats rotation. They shouldn't need to. The bot must maintain itself during 24/7 operation 
like any production database does.

**Service design**:
```rust
pub struct MaintenanceService {
    // Configurable via [maintenance] section
}

impl Service for MaintenanceService {
    fn name(&self) -> &'static str { "maintenance" }
    fn priority(&self) -> i32 { 90 }  // After core services, before webserver
    fn dependencies(&self) -> Vec<&'static str> { vec!["connectivity"] }
}
```

**Periodic tasks**:
| Task | Default Interval | What It Does |
|------|-----------------|--------------|
| WAL Checkpoint | 1 hour | `PRAGMA wal_checkpoint(TRUNCATE)` on all WAL databases |
| Incremental VACUUM | 6 hours | `PRAGMA incremental_vacuum(1000)` on databases with auto_vacuum=INCREMENTAL |
| RPC Stats Rotation | 24 hours | DELETE rpc_stats WHERE timestamp < (now - 48h), then VACUUM |
| Stale Token Marking | 24 hours | Mark tokens not updated in 30 days as inactive |
| Signature Pruning | 1 hour | Trim GLOBAL_KNOWN_SIGNATURES to max 50K entries |
| Cache Budget Check | 5 minutes | Report cache sizes to metrics |
| Memory Metrics | 1 minute | Report RSS, cache sizes, DB sizes to dashboard |
| Database Size Check | 6 hours | Log DB file sizes, warn if any > 500MB |

**Trading-aware maintenance** (CRITICAL — must NEVER interfere with trading):
```
Before any blocking DB operation (VACUUM, bulk DELETE):
  1. Check: is_force_stopped() → if stopped, skip maintenance (something more important happening)
  2. Check: are there in-flight trades? (via position semaphore)
  3. If trades active: SKIP this maintenance cycle, retry next interval
  4. If no trades: proceed, but with a 5-second timeout
  5. Use incremental VACUUM (fast, non-blocking) instead of full VACUUM
  6. Use batched DELETE (LIMIT 5000 per batch, yield between batches)
  
NEVER call FORCE_STOP for maintenance. Trading always has priority.
```

**Config section**:
```toml
[maintenance]
enabled = true
wal_checkpoint_interval_hours = 1
vacuum_interval_hours = 6
rpc_stats_retention_hours = 48
stale_token_days = 30
signature_max_count = 50000
```

**One-time migration** (adding auto_vacuum=INCREMENTAL to existing databases):
```
On bot startup (once per database):
  1. Check: PRAGMA auto_vacuum → if already INCREMENTAL, skip
  2. If not INCREMENTAL:
     a. Log: "First-time database optimization for {db_name} — this may take 30-60 seconds"
     b. Set PRAGMA auto_vacuum = INCREMENTAL
     c. Run full VACUUM (required to activate auto_vacuum change)
     d. Log completion time
  3. This happens ONCE per database, then never again
```

✅ **PROS**:
- Zero user intervention — everything automatic
- Prevents database bloat (pools.db 729MB → never happens again)
- Prevents RPC stats accumulation (602K rows → capped at 48h retention)
- Fits cleanly into existing ServiceManager architecture
- All intervals configurable for power users
- Trading-aware: never interferes with swaps

❌ **CONS**:
- **One-time migration VACUUM is slow**: Full VACUUM on 294MB tokens.db could take 30-60 seconds, 
  blocking all token queries during that time
- **Incremental VACUUM not as effective**: Doesn't defragment as well as full VACUUM
- **WAL checkpoint TRUNCATE blocks briefly**: Can delay writes by ~10-100ms
- **Stale token marking**: What if a token re-activates after 30 days? Need to un-mark it.
- **RPC stats deletion**: 602K rows DELETE takes seconds, blocks the database

🔧 **CON MITIGATIONS**:
- **One-time VACUUM**: Show progress in logs + dashboard. Do at startup before trading starts 
  (during service initialization phase, not after ServiceManager.start_all())
- **Incremental VACUUM**: Run more frequently (every 6h) to keep waste small. Full VACUUM only 
  during the one-time migration
- **WAL checkpoint**: Use PASSIVE (non-blocking) by default. TRUNCATE only when WAL file > 100MB
- **Stale tokens**: Don't DELETE — just mark as `inactive`. Any update to the token re-activates it
- **RPC stats**: Batch delete: `DELETE FROM calls WHERE timestamp < ? LIMIT 5000` in a loop, 
  yielding between batches to let other queries through

---

### Architecture Component 6: jemalloc Allocator

**What**: Replace the system allocator with jemalloc for all platforms except Windows (MSVC).

**Why this is the right solution**: This is industry standard for long-running Rust services. 
Firefox, TiKV (PingCAP), Cloudflare Workers, Discord all use jemalloc. The system allocator 
(glibc malloc on Linux, macOS malloc) is optimized for short-lived programs, not servers.

Key benefits for ALL users:
- **Returns freed pages to OS**: After filtering refresh frees 238 MB, jemalloc actually returns 
  those pages instead of keeping them mapped. RSS drops after peaks.
- **Thread-local caching**: Better performance for multi-threaded Rust async (tokio).
- **Built-in profiling**: `MALLOC_CONF=prof:true` gives heap profiles for debugging.
- **Reduces fragmentation**: Arena-based allocation prevents the fragmentation pattern we see 
  from repeated alloc/free cycles in filtering.

**Implementation**:
```toml
# Cargo.toml
[dependencies]
tikv-jemallocator = { version = "0.6", optional = true }

[features]
default = ["jemalloc"]
jemalloc = ["tikv-jemallocator"]
```

```rust
// main.rs
#[cfg(feature = "jemalloc")]
#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;
```

✅ **PROS**:
- 3 lines of code + 1 dependency
- 100-200 MB RSS reduction from fragmentation alone
- RSS now tracks actual usage (drops after freeing, unlike system allocator)
- Built-in profiling for future debugging (MALLOC_CONF=prof:true)
- Feature flag: users can disable if it causes issues on their platform

❌ **CONS**:
- **Cross-compilation complexity**: jemalloc compiles from C source, needs target C toolchain
- **Build time**: Adds ~30-60 seconds to compilation (compiles jemalloc from source)
- **Binary size**: Adds ~300-500 KB
- **musl targets**: jemalloc with musl-libc can have issues (Alpine Linux, static builds)
- **Windows**: Doesn't work with MSVC (handled by #[cfg], falls back to system allocator)

🔧 **CON MITIGATIONS**:
- **Cross-compilation**: The build.sh already handles cross-compilation toolchains. Test build 
  on all 3 platforms (macOS, Linux, Windows) before merging
- **Build time**: 30-60s is acceptable — builds already take minutes for Rust
- **Binary size**: 300-500 KB is negligible compared to 30 MB+ total binary
- **musl**: Feature flag allows disabling. Document in build instructions
- **Windows**: Gracefully falls back — #[cfg(not(target_env = "msvc"))] is compile-time, zero runtime cost

---

### Architecture Component 7: Memory Pressure Response

**What**: Active monitoring of the bot's own memory usage with automatic response at different 
pressure levels.

**Why this is the right solution**: Even with budgets and bounded caches, unexpected scenarios 
happen — large position counts, network issues causing retries, user browsing heavy dashboard 
pages. The bot needs a safety net that prevents OOM without human intervention.

**REVISED**: Monitor the BOT'S OWN RSS (not system available memory) to avoid false positives 
from other processes running on the same machine. Use 3 levels instead of 4 for simplicity.

**How it works**:
```
Memory check (every 30 seconds, via sysinfo — already a dependency):
  process_rss = sysinfo::Process::memory()
  memory_budget = configured or auto-detected budget

Pressure Levels:
  Level 0 - Normal (RSS < 70% of budget):
    → All systems at full speed
    → Full caching, normal refresh intervals
    → No action needed

  Level 1 - Elevated (RSS 70-90% of budget):
    → Log warning: "Memory usage elevated: {rss}MB / {budget}MB"
    → Increase filtering refresh interval (180s → 300s)
    → Trigger moka cache eviction on Disposable caches (FAILED_CACHE, TOKEN_2022_CACHE, etc.)
    → Report to dashboard status bar
    → If Telegram configured: one-time notification

  Level 2 - Critical (RSS > 90% of budget):
    → Log error: "Memory critical: {rss}MB / {budget}MB — reducing operations"
    → Force evict ALL non-essential caches (.invalidate_all() on moka caches)
    → Increase filtering interval to 600s
    → Trigger immediate WAL checkpoint on all databases
    → Dashboard: prominent warning banner
    → Telegram: urgent notification
    → DO NOT touch trading — trades must still execute normally

Self-healing:
  When RSS drops below 70% for 5 consecutive checks (2.5 minutes):
    → Restore normal intervals and cache behavior
    → Log: "Memory recovered: {rss}MB / {budget}MB — resuming normal operation"
```

**What the budget is** (simplified from Component 1):
```
Default budget:
  - If [performance] memory_target is set in config: use that value
  - Else: min(system_ram * 0.40, 1200 MB)
  - This gives: 2GB VPS → 800MB, 4GB → 1.2GB (capped), 8GB+ → 1.2GB (capped)
  - Cap at 1.2GB because the bot shouldn't use more regardless of system size
```

✅ **PROS**:
- Self-healing: bot recovers from memory spikes without user intervention
- OOM prevention: users on low-RAM VPS won't crash
- Observability: dashboard shows current memory pressure level
- Conservative: never touches trading, only reduces non-critical caching
- Simple: 3 levels, clear thresholds, deterministic behavior

❌ **CONS**:
- **RSS measurement includes mmap pages**: OS may report higher RSS than actual heap usage, 
  causing false elevated state
- **Filtering interval increase degrades freshness**: At 600s, tokens might be 10 minutes stale
- **Budget calculation**: 40% of system RAM is arbitrary. On a 2GB VPS running nginx+PM2+bot, 
  40% = 800MB might still be too much
- **Recovery oscillation**: If RSS hovers around 70%, might flip between Normal and Elevated

🔧 **CON MITIGATIONS**:
- **RSS includes mmap**: After fixing mmap_size (Component 1), mmap pages are reasonable (256MB max).
  Also: jemalloc (Component 6) provides `allocated` stat that's more accurate than RSS
- **Filtering freshness**: At Elevated, 300s is still acceptable for trading decisions. At Critical, 
  600s is a safety compromise — better stale data than OOM crash
- **Budget on 2GB VPS**: The cap at 1.2GB is generous. For 2GB VPS, actual target is ~800MB after 
  all optimizations. If the bot uses 560MB (70% of 800MB), that's fine with the other components
- **Oscillation**: Hysteresis — require 5 consecutive normal readings to restore (2.5 min debounce)

---

### Architecture Component 8: SQLite Configuration Standardization

**What**: Replace per-database ad-hoc PRAGMA configurations with a shared initialization function, 
fix the 30GB mmap misconfiguration, and standardize connection pool sizes.

**Why**: Currently each database file has its own copy-pasted PRAGMA setup with different (often 
excessive) values. tokens.db has `mmap_size = 30,000,000,000` (30 GB!), events.db read pool has 
`cache_size = 20,000`. There's no consistency and no central place to tune.

**This is the FOUNDATION for Component 1** (right-sized configs).

**Shared init function** (replaces 14 different PRAGMA setup blocks):
```rust
// src/database/common.rs (new file)
pub struct DbConnectionConfig {
    pub cache_size_pages: i32,    // SQLite pages (× 4KB = actual memory)
    pub mmap_size_bytes: i64,     // 0 = disabled, else bytes
    pub wal_enabled: bool,        // true for all databases
    pub auto_vacuum_incremental: bool,  // true for all databases
}

impl DbConnectionConfig {
    pub fn hot() -> Self {        // tokens, transactions
        Self { cache_size_pages: 5000, mmap_size_bytes: 256 * 1024 * 1024, 
               wal_enabled: true, auto_vacuum_incremental: true }
    }
    pub fn standard() -> Self {   // events, actions, positions
        Self { cache_size_pages: 2000, mmap_size_bytes: 128 * 1024 * 1024,
               wal_enabled: true, auto_vacuum_incremental: true }
    }
    pub fn cold() -> Self {       // strategies, wallets, ai, tools, rpc_stats
        Self { cache_size_pages: 500, mmap_size_bytes: 0,
               wal_enabled: true, auto_vacuum_incremental: true }
    }
}

pub fn configure_sqlite_connection(conn: &Connection, config: &DbConnectionConfig) -> Result<()> {
    if config.wal_enabled {
        conn.pragma_update(None, "journal_mode", "WAL")?;
    }
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    conn.pragma_update(None, "busy_timeout", 30000)?;
    conn.pragma_update(None, "temp_store", "MEMORY")?;
    conn.pragma_update(None, "cache_size", config.cache_size_pages)?;
    if config.mmap_size_bytes > 0 {
        conn.pragma_update(None, "mmap_size", config.mmap_size_bytes)?;
    }
    if config.auto_vacuum_incremental {
        // Only effective on new databases or after full VACUUM
        conn.pragma_update(None, "auto_vacuum", "INCREMENTAL")?;
    }
    conn.pragma_update(None, "foreign_keys", true)?;
    Ok(())
}
```

✅ **PROS**:
- Single source of truth for all database configuration
- Changing a value in one place affects all databases of that category
- Eliminates the 30GB mmap misconfiguration permanently
- New databases automatically get correct settings
- Easy to add new PRAGMAs (e.g., `page_size`) in one place

❌ **CONS**:
- **Migration effort**: 14 database init functions need updating
- **Per-database exceptions**: Some databases might need non-standard settings
- **Breaking change**: Reduced cache sizes could temporarily slow queries until working set adjusts

🔧 **CON MITIGATIONS**:
- **Migration**: Mechanical refactoring — replace PRAGMA blocks with configure_sqlite_connection() call
- **Exceptions**: DbConnectionConfig is a struct with overridable fields, not sealed
- **Performance**: Monitor query times for 1 week after change. If any DB is slower, bump its category

---

### Architecture Component 9: SQL Pre-Filtering (NEW — not in v1)

**What**: Push simple numeric filter criteria into the SQL WHERE clause, reducing the number of 
tokens loaded into Rust memory from 171K to potentially 10K-30K.

**Why this is a valuable addition**: Components 2 (incremental) and 3 (FilterToken) reduce refresh 
cost and per-token size. But the INITIAL cold-start load still brings 171K tokens. Many of these 
tokens will obviously fail filters (e.g., volume = 0, liquidity = 0, no market data). SQL can 
eliminate these BEFORE they reach Rust.

**How it works**:
```sql
-- Current query loads ALL tokens with market data:
SELECT ... FROM tokens JOIN market_dexscreener ON ...

-- Pre-filtered query adds WHERE clause from config values:
SELECT ... FROM tokens JOIN market_dexscreener ON ...
WHERE 
  COALESCE(ds_volume_h24, 0) >= {config.filtering.min_volume_h24}
  AND COALESCE(ds_liquidity_usd, 0) >= {config.filtering.min_liquidity}
  AND COALESCE(ds_mcap, 0) >= {config.filtering.min_mcap}
  -- Only include criteria that have simple numeric thresholds
```

**Example impact**: With typical filter settings (min volume $100, min liquidity $1000):
- Current: 171K tokens loaded
- Pre-filtered: maybe 20K-40K tokens loaded (tokens with zero volume/liquidity eliminated)
- Memory: 40K × 350 bytes (FilterToken) = 14 MB (vs 60 MB for 171K)

**Which filters can be pushed to SQL** (simple numeric comparisons only):
- ✅ min/max volume (h1, h6, h24) — numeric threshold
- ✅ min/max liquidity — numeric threshold
- ✅ min/max market cap — numeric threshold
- ✅ min/max FDV — numeric threshold
- ✅ min rugcheck score — numeric threshold
- ✅ token age (pair_created_at) — timestamp comparison
- ❌ Transaction buy/sell ratio — requires computation
- ❌ Cross-source comparisons — needs both data sources
- ❌ AI analysis — external API call
- ❌ Complex boolean logic — depends on config structure

✅ **PROS**:
- Further reduces cold-start memory (60 MB → ~14 MB with typical filters)
- SQLite is efficient at indexed WHERE clauses
- Reduces Rust processing time (fewer tokens to iterate)
- Works for ALL users — everyone has filter thresholds configured

❌ **CONS**:
- **Config coupling**: SQL query must read config values, creating coupling between filter config 
  and database query
- **Index requirements**: WHERE clauses need indexes on filtered columns to be efficient
- **Filter changes**: When user changes filter thresholds, the SQL query changes → need full 
  refresh (already handled by incremental design)
- **Not all filters expressible in SQL**: Complex filters still need Rust evaluation
- **Risk of inconsistency**: SQL pre-filter might exclude a token that Rust filter would have 
  included differently (e.g., if config has OR logic between sources)

🔧 **CON MITIGATIONS**:
- **Coupling**: Generate SQL WHERE clause from config at query time, not hardcoded
- **Indexes**: Most filtered columns (volume, liquidity, mcap) are likely already indexed or can 
  be cheaply added
- **Filter changes**: Incremental system already triggers full refresh on config change
- **Consistency**: Only push SAFE filters — strict numeric minimums that are ALWAYS AND conditions.
  If there's any doubt, don't push it to SQL
- **Phasing**: Implement AFTER Components 2+3 are stable. This is an optimization on top of the 
  base architecture, not a requirement

---

## 📊 Expected Results Summary (Revised)

### For a 2GB RAM VPS user:
| Component | Before | After | Confidence |
|-----------|--------|-------|------------|
| SQLite caches (steady state) | ~400 MB | ~40-60 MB | HIGH (just config) |
| Filtering snapshot (baseline) | 238 MB | 14-60 MB | HIGH (FilterToken+SQL prefilter) |
| Filtering snapshot (peak) | 476 MB | 70-120 MB | HIGH (incremental) |
| Global caches | 25 MB, growing | 15 MB, bounded | HIGH (moka caps) |
| Allocator waste | ~200 MB | ~50 MB | MEDIUM (jemalloc) |
| mmap overhead | ~100 MB | ~30 MB | HIGH (fix 30GB→256MB) |
| **Total RSS at startup** | **~800 MB** | **~150-250 MB** | |
| **Total RSS after 24h** | **~2+ GB** | **~200-300 MB** | |

### For any user (universal benefits):
| Metric | Before | After |
|--------|--------|-------|
| Memory growth rate | Unbounded | Bounded (capped caches) |
| Peak memory (filtering) | 2× baseline | ~1.1× baseline |
| Database disk waste | ~880 MB | ~0 (auto-maintained) |
| User intervention needed | Manual VACUUM/cleanup | Zero (automatic) |
| OOM risk on low-RAM VPS | High | Low (pressure response) |

---

## 🎛️ Tunability Architecture — Component 10: User & Automatic Tuning

> **Design Question**: How can users (and the system itself) tune the memory architecture — both 
> automatically for zero-config users and manually for power users?

### The 3-Layer Cascade Model

Every tunable parameter follows a 3-layer override cascade:

```
Layer 1: Auto-Detection (default)     ← Detects system RAM, picks optimal values
    ↓ overridden by
Layer 2: Memory Profile (simple)      ← User sets "low"/"medium"/"high" in config
    ↓ overridden by
Layer 3: Individual Override (expert)  ← User sets specific values per parameter
```

- **95% of users** never touch anything — Layer 1 auto-detects and picks the right values
- **Power users** pick a profile — Layer 2 overrides auto-detection
- **Expert users** fine-tune individual parameters — Layer 3 overrides everything

### Layer 1: Auto-Detection (Zero-Config)

At startup, the bot detects system RAM and calculates a memory budget:

```rust
fn detect_memory_profile(system: &System) -> MemoryProfile {
    let total_ram_mb = system.total_memory() / 1024 / 1024;
    match total_ram_mb {
        0..=3072     => MemoryProfile::Low,    // ≤3 GB (small VPS)
        3073..=12288 => MemoryProfile::Medium,  // 4-12 GB (typical VPS/laptop)
        _            => MemoryProfile::High,    // >12 GB (workstation)
    }
}
```

This uses the **existing `sysinfo` crate** (already a dependency, already called in 
`snapshot/collectors.rs` for dashboard metrics). No new dependencies needed.

**Key principle**: The auto-detected profile MUST be SAFE — the bot should never use more than 
~50% of system RAM at steady state without explicit user override.

### Layer 2: Memory Profile (Simple Manual Tuning)

```toml
[performance]
# "auto" (default) | "low" | "medium" | "high"
memory_profile = "auto"
```

Each profile is a **complete set of resolved values** for every tunable parameter:

| Parameter | Low (≤3GB) | Medium (4-12GB) | High (>12GB) |
|-----------|-----------|-----------------|-------------|
| **SQLite cache_size multiplier** | 0.5× | 1.0× (plan defaults) | 2.0× |
| **Max read pool connections** | 2 | 3-5 | 5-8 |
| **DECIMALS_CACHE max entries** | 50,000 | 150,000 | 300,000 |
| **SIGNATURES_CACHE max entries** | 10,000 | 50,000 | 100,000 |
| **TOKEN_2022_CACHE max entries** | 10,000 | 50,000 | 100,000 |
| **TokenStore max entries** | 3,000 | 10,000 | 25,000 |
| **PENDING_TX max entries** | 3,000 | 10,000 | 20,000 |
| **Filtering refresh interval** | 300s (5 min) | 180s (3 min) | 120s (2 min) |
| **Dashboard poll interval** | 15s | 10s | 5s |
| **Memory pressure Level 1** | 55% | 70% | 80% |
| **Memory pressure Level 2** | 70% | 85% | 90% |
| **Memory pressure Level 3** | 85% | 95% | 95% |

**Math for Low profile (2 GB VPS)**:
- SQLite caches: ~84 MB × 0.5 = ~42 MB
- FilterToken snapshot: ~60 MB (universal, not profile-dependent)
- Global caches: 50K+10K+10K+3K+3K ≈ 76K entries × ~100 bytes = ~8 MB
- jemalloc overhead: ~25 MB
- Application code: ~100 MB
- **Total estimated: ~235 MB** → ~12% of 2 GB ✅ safe

**Math for High profile (32 GB workstation)**:
- SQLite caches: ~84 MB × 2.0 = ~168 MB
- FilterToken snapshot: ~60 MB
- Global caches: 300K+100K+100K+25K+20K ≈ 545K entries × ~100 bytes = ~55 MB
- jemalloc overhead: ~50 MB
- Application code: ~100 MB
- **Total estimated: ~433 MB** → ~1.3% of 32 GB ✅ generous but controlled

### Layer 3: Individual Override (Expert Tuning)

Expert users can override ANY specific parameter. Value of `0` means "use profile default":

```toml
[performance]
memory_profile = "auto"

# SQLite tuning
sqlite_cache_multiplier = 0.0        # 0.0=profile default, 0.1-5.0=override
max_connections_per_read_pool = 0     # 0=profile default, 1-20=override

# Cache tuning
max_decimals_cache_entries = 0        # 0=profile default
max_signatures_cache_entries = 0      # 0=profile default
max_token_store_entries = 0           # 0=profile default

# Filtering tuning
filtering_refresh_interval_secs = 0   # 0=profile default (180s for medium)

# Dashboard tuning
dashboard_poll_interval_secs = 0      # 0=profile default (10s for medium)

[maintenance]
enabled = true

# Retention periods (days)
events_retention_days = 30
actions_retention_days = 30
rpc_stats_retention_days = 7
sol_flow_retention_days = 30
ohlcv_gap_retention_days = 90

# Maintenance timing
wal_checkpoint_interval_secs = 300
vacuum_interval_hours = 24

# Maintenance window — heavy operations (VACUUM) only run during this window
# Empty = anytime. Format: "HH:MM" in local time.
maintenance_window_start = ""
maintenance_window_end = ""

# Safety
skip_during_active_trades = true
```

**Resolution logic** (pseudocode):
```rust
fn resolve_parameter<T>(individual_override: T, profile_value: T, is_default: fn(T) -> bool) -> T {
    if !is_default(individual_override) {
        individual_override  // Layer 3 wins
    } else {
        profile_value        // Layer 2 (or Layer 1 auto-detected) wins
    }
}

// For numbers: 0 = "use default". For floats: 0.0 = "use default".
let max_decimals = resolve(config.max_decimals_cache_entries, profile.decimals_max, |v| v == 0);
```

### Hot-Reload vs Restart-Required

| Parameter | Hot-Reloadable? | Mechanism |
|-----------|----------------|-----------|
| moka cache capacities | ❌ No | max_capacity is fixed at creation — must rebuild cache to resize |
| Filtering refresh interval | ✅ Yes | Next cycle uses new interval |
| Dashboard poll interval | ✅ Yes | JS reads from `/api/config` |
| Maintenance settings | ✅ Yes | Service reads config each cycle |
| Pressure thresholds | ✅ Yes | Checked each pressure evaluation |
| Memory profile | ⚠️ Partial | Intervals/thresholds yes. Caches/SQLite/pools need restart (or cache swap) |
| SQLite cache_size | ❌ No | PRAGMA set on connection init |
| Connection pool sizes | ❌ No | r2d2 pool created at init |
| jemalloc | ❌ No | Compile-time feature flag |

**Dashboard UX**: When a restart-required setting changes, show a badge:
`⚠️ Some changes require restart to take effect`

### Observability (Users Must SEE Before They TUNE)

**New API endpoint**: `GET /api/system/memory`

```json
{
  "profile": "auto",
  "resolved_profile": "medium",
  "system_ram_mb": 8192,
  "process_rss_mb": 412,
  "memory_budget_mb": 4096,
  "usage_percent": 10.1,
  "pressure_level": "normal",
  "breakdown": {
    "sqlite_caches_mb": 52,
    "filtering_snapshot_mb": 61,
    "token_caches_mb": 28,
    "other_caches_mb": 8,
    "application_mb": 263
  },
  "caches": [
    { "name": "decimals", "entries": 148000, "max": 150000, "hit_rate": 0.973 },
    { "name": "dexscreener", "entries": 1850, "max": 2000, "hit_rate": 0.821 },
    { "name": "signatures", "entries": 24000, "max": 50000, "hit_rate": null },
    { "name": "token_store", "entries": 3200, "max": 10000, "hit_rate": 0.892 }
  ],
  "databases": [
    { "name": "tokens.db", "size_mb": 294, "rows": 275023, "free_percent": 0 },
    { "name": "ohlcvs.db", "size_mb": 200, "rows": 270578, "free_percent": 5 },
    { "name": "events.db", "size_mb": 45, "rows": 89000, "free_percent": 2 }
  ],
  "maintenance": {
    "last_run": "2026-02-19T02:00:00Z",
    "next_scheduled": "2026-02-20T02:00:00Z",
    "last_vacuum_db": "tokens.db",
    "total_reclaimed_mb": 880
  }
}
```

**Dashboard Panel** — new "Performance" card in header or dedicated tab section:
```
┌──────────────────────────────────────────────────────┐
│ Memory Profile: Auto (→Medium)     [Change ▾]        │
│ ────────────────────────────────────────────────────── │
│ Process: 412 MB / 8,192 MB system  │ Pressure: 🟢    │
│ ████████░░░░░░░░░░░░░░░░░░░ 5.0%   │                 │
│                                                       │
│ ┌─────────────┬──────────┬───────────┬──────────────┐ │
│ │ SQLite 52MB │ Filter   │ Caches    │ App 263 MB   │ │
│ │ 12.6%       │ 61 MB    │ 36 MB     │ 63.9%        │ │
│ │             │ 14.8%    │ 8.7%      │              │ │
│ └─────────────┴──────────┴───────────┴──────────────┘ │
│                                                       │
│ Top Caches:         Entries/Max    Hit Rate            │
│   decimals          148K / 150K   97.3%  ████████████ │
│   token_store       3.2K / 10K    89.2%  █████████░░  │
│   dexscreener       1.8K / 2.0K   82.1%  ████████░░░  │
│                                                       │
│ Databases:          Size    Rows    Free              │
│   tokens.db         294 MB  275K    0%                │
│   ohlcvs.db         200 MB  270K    5%                │
│   events.db          45 MB  89K     2%                │
│                                                       │
│ 🔧 Last maintenance: 2h ago  │  ⏭ Next: in 4h       │
└──────────────────────────────────────────────────────┘
```

### Self-Tuning / Adaptive Behaviors

Beyond static profiles, the system adapts at runtime:

**1. Pressure-Adaptive Profile Shifting**
When memory pressure hits Level 1, the system temporarily shifts cache parameters toward "Low" 
profile values:
```
Normal → L1 pressure: reduce moka capacities by 30%, increase filtering interval by 50%
L1 → L2 pressure: reduce moka capacities by 60%, pause non-critical background tasks
L2 → Normal: gradually restore original values over 5 minutes (hysteresis)
```
This is already described in Component 7, but the tunability angle is: the user can observe this 
happening in the dashboard and can override it by setting a manual profile.

**2. Cache Efficiency Monitoring (Observe, Don't Auto-Resize)**
Track hit rates over 1-hour windows for each moka cache:
- Hit rate > 95% at max capacity → log recommendation: "DECIMALS_CACHE hit rate 97.3% at capacity. 
  Consider increasing max_decimals_cache_entries for better performance."
- Hit rate < 30% → log recommendation: "SIGNATURES_CACHE hit rate 24%. Cache may be oversized."

**Why NOT auto-resize**: Changing cache size affects memory budget. If decimals auto-grows, it might 
push overall memory past the budget. Recommendations are safer — the user decides.

**3. Filtering Duration Tracking**
Track actual time per filtering refresh:
- If refresh takes <500ms consistently → log: "Filtering fast enough for shorter interval"
- If refresh takes >5s → log: "Filtering slow. Consider enabling SQL pre-filtering."

The maintenance service records these metrics. The dashboard shows them. The user decides.

### CLI Tuning (screenerbot-manager)

For VPS users who don't use the dashboard:
```bash
screenerbot-manager memory              # Current memory breakdown (RSS, caches, DBs)
screenerbot-manager performance         # Current profile + all resolved values
screenerbot-manager performance auto    # Set auto profile
screenerbot-manager performance low     # Set low profile
screenerbot-manager maintenance status  # Last run, next scheduled, DB sizes
screenerbot-manager maintenance run     # Force maintenance cycle now
screenerbot-manager caches              # Cache stats (entries, max, hit rates)
```

These commands call the same API endpoints (`/api/system/memory`, `/api/system/performance`, etc.) 
that the dashboard uses. Single source of truth.

### API Endpoints for Tunability

| Method | Endpoint | Purpose |
|--------|----------|---------|
| GET | `/api/system/memory` | Current memory breakdown, pressure, budget |
| GET | `/api/system/performance` | Active profile + all resolved parameter values |
| PATCH | `/api/system/performance` | Update performance config (hot-reload eligible) |
| GET | `/api/system/maintenance` | Maintenance status, history, next scheduled |
| POST | `/api/system/maintenance/trigger` | Force maintenance cycle now |
| GET | `/api/system/caches` | All cache names, entries, max, hit rates |
| POST | `/api/system/caches/:name/clear` | Clear a specific cache |

All behind existing auth middleware. Standard success/error response wrappers.

### Critical Design Decisions

1. **Defaults are the product** — 95% of users will never change a setting. The auto-detected 
   profile must be correct, safe, and performant for their system. If we need the user to tune 
   manually, we failed.

2. **Never break trading for tunability** — No profile change, cache eviction, or maintenance 
   operation should interrupt an active trade. Use existing FORCE_STOP/trade-in-flight checks.

3. **Observe first, tune second** — The dashboard memory panel ships BEFORE tunability knobs. 
   Users must understand what they're seeing before they can meaningfully adjust it.

4. **Config.toml is source of truth** — Profile changes via API/dashboard write to config.toml 
   via the existing config reload mechanism. No hidden state files or shadow configurations.

5. **Graceful degradation, never crash** — If the bot runs low on memory:
   - Level 1: Warnings + reduce non-critical caches (trading unaffected)
   - Level 2: Aggressive eviction + pause entries (exits continue to protect positions)
   - Level 3: Emergency mode + Telegram alert (user must intervene)
   - **NEVER**: OOM crash, data corruption, or silent failure

6. **Profiles are presets, not ceilings** — "Low" doesn't mean the bot is crippled. It means 
   caches are smaller and refresh intervals are longer. The bot still discovers tokens, filters, 
   and trades — just with a slightly larger latency budget on cache misses.

### Implementation Notes

- Component 10 spans multiple existing phases:
  - **Phase A**: Add `[performance]` + `[maintenance]` config sections, auto-detection
  - **Phase B**: moka caches read max from resolved profile
  - **Phase D**: MaintenanceService reads `[maintenance]` config
  - **Phase E**: Memory panel, pressure uses profile thresholds, API endpoints

- **Not a new phase** — it's a cross-cutting concern woven into existing phases
- The `PerformanceConfig` struct replaces the empty `MonitoringConfig` placeholder
- All profile resolution happens in a single `resolve_profile()` function that other modules call

✅ **PROS**:
- Zero-config works for everyone (auto-detection)
- Power users have full control without modifying code
- Cascading model is intuitive and prevents conflicts
- Observable — users can see exactly what's happening and why
- Works for ALL deployment scenarios (2GB VPS → 64GB workstation)
- Leverages existing infrastructure (sysinfo, config reload, API patterns)

❌ **CONS**:
- More config complexity (2 new sections with ~25 fields total)
- Dashboard panel is additional UI work
- Profile calculations must be tested across many RAM sizes
- Auto-detection could be wrong on containers (cgroups may lie about RAM)

🔧 **CON MITIGATIONS**:
- Config defaults to `memory_profile = "auto"` with 0-values for all overrides → zero config 
  needed, backward compatible
- Dashboard panel is Phase E (last) — core improvements work without it
- Test profiles at 1GB, 2GB, 4GB, 8GB, 16GB, 32GB boundaries
- For containers: detect cgroups memory limit as alternative to physical RAM:
  `let cgroup_limit = fs::read_to_string("/sys/fs/cgroup/memory.max")` → use min(physical, cgroup)

---

Phase A — Foundation (LOW RISK, enables everything):
  A1. SQLite standardization: shared configure_sqlite_connection() function [Component 8]
  A2. Fix mmap_size 30GB → 256MB on tokens.db and wallet_monitor.db [Component 1/8]
  A3. Right-size cache_size values per database [Component 1]
  A4. Right-size connection pool max_size (10→3-5 for most) [Component 1]
  A5. Migrate ALL SQLite pools to use with_init() for PRAGMA configuration [Gap 7 — CORRECTNESS]
      r2d2 already has idle_timeout=10min, max_lifetime=30min by DEFAULT — connections DO get 
      recycled. The fix is ensuring recycled connections get PRAGMAs via with_init(), not setting 
      timeouts. For cold databases (strategies, ai), optionally reduce idle_timeout to 5 min.
  A6. jemalloc allocator with feature flag [Component 6]
  A7. Add auto_vacuum=INCREMENTAL to all databases + one-time migration VACUUM [Component 5]
  A8. Add [performance] config section with PerformanceConfig [Component 10]
      - memory_profile field ("auto"/"low"/"medium"/"high")
      - Auto-detection via sysinfo (existing dep)
      - Profile resolution function: resolve_profile()
      - Individual override fields (all default to 0 = use profile)
  A9. Add [maintenance] config section with MaintenanceConfig [Component 10]
      - Retention periods for events/actions/rpc_stats/sol_flow/ohlcv
      - WAL checkpoint + VACUUM intervals
      - Maintenance window + trading safety toggle
  
  EXPECTED IMPACT: ~400-500 MB RSS reduction from cache_size + mmap fixes. with_init() ensures 
  recycled connections maintain correct PRAGMAs (correctness fix, not memory fix — r2d2 already 
  recycles connections every 10-30 min by default).
  VERIFICATION: Run bot, measure RSS at startup and after 30 min. Compare before/after.
  ROLLBACK: Revert cache_size/pool_size to previous values. Remove idle_timeout. Git revert commit.

Phase B — Bounded Caches (LOW-MEDIUM RISK):
  B1. Add moka crate dependency
  B2. Migrate DECIMALS_CACHE to moka — max from resolved profile (50K/150K/300K) [Component 10]
  B3. Migrate FAILED_CACHE to moka (10K max, 5 min TTL)
  B4. Migrate GLOBAL_KNOWN_SIGNATURES to moka — max from resolved profile (10K/50K/100K)
  B5. Migrate TOKEN_2022_CACHE to moka — max from resolved profile (10K/50K/100K)
  B6. Migrate GLOBAL_PENDING_TRANSACTIONS to moka — max from resolved profile (3K/10K/20K)
  B7. Migrate LAST_TOKEN_ACCOUNTS_CHECK to moka (5K max, 1h TTL)
  B8. Fix POSITION_LOCKS: cleanup on position close
  B9. Replace custom TimedCache with moka (DEXSCREENER/GECKO/RUGCHECK caches)
  B10. Add PRICE_HISTORY cleanup on position close
  B11. Migrate TokenStore HashMap to moka — max from resolved profile (3K/10K/25K) [Gap 3]
  B12. Migrate AI cache DashMap to moka (5K max, TTL from config) [Gap 8]
  B13. Migrate TOKEN_POOLS_CACHE to moka (5K max, TTL from pool_cache_ttl config) [Gap 9 — SLOW LEAK]
  B14. Migrate POOL_PREFETCH_STATE to moka (5K max, 5 min TTL) [Gap 9 — SLOW LEAK]
  B15. (REMOVED — v7 verified COMPUTATION_FAILURES is BOUNDED: fixed &'static str key set, ~5-10 entries max)
  B16. (REMOVED — v7 verified FETCH_LOCKS is BOUNDED: per-use release + 10K safety cap)
  B17. Add periodic remove for ACTIVE_ACTIONS (completed/failed actions) [Gap 9 — TRUE LEAK]
  
  EXPECTED IMPACT: Stops ALL unbounded memory growth. Saves ~50-100 MB after 24h.
  Profile-aware: Low profile gets smaller caches, High gets larger.
  VERIFICATION: Run bot for 24h, verify no cache exceeds configured max.
  ROLLBACK: Revert moka migration per-cache. Each cache is independent — can roll back individually.

Phase C — Unbounded Caches, DB Maintenance & Stale Token Filter ✅ COMPLETED:
  
  ⚠️ **ACTUAL VS PLANNED**: Phase C plan below (C1-C9) prescribed TokenListEntry + incremental filtering.
  These were NOT IMPLEMENTED. We took a fundamentally different (and better) approach:
  - Stale token SQL filter (7-day WHERE clause) achieved 91% reduction → BETTER than TokenListEntry
  - TokenListEntry would save ~23 MB (15.6K × 450 bytes). Stale filter saved ~122 MB.
  - C1/C2 below are now "DEFERRED INDEFINITELY — diminished ROI after stale filter"
  
  WHAT WE ACTUALLY DID:
  C1. API_RESPONSE_CACHE → moka (1K cap, 5min TTL) — bounded an unbounded cache ✅ DONE
  C2. FAILED_CACHE → moka (50K cap, 24h TTL) — bounded an unbounded cache ✅ DONE
  C3. Database auto-vacuum maintenance module (maintenance.rs) — one-time migration + periodic vacuum ✅ DONE
  C4. jemalloc tuning documentation (comment in main.rs) ✅ DONE
  C5. Stale token SQL WHERE filter (7-day cutoff on market_data_last_fetched_at) ✅ DONE
      → Reduces tokens from 172K to 15.6K (91% reduction, ~122 MB saved)
  
  ORIGINAL PLAN (NOT IMPLEMENTED):
  C1. Define FilterToken struct with only filter-needed fields [Component 3] ⏸️ DEFERRED
  C2. Create optimized SQL query for FilterToken loading ⏸️ DEFERRED
  C3. Update all filter sources (dexscreener, geckoterminal, rugcheck, meta) to use FilterToken ⏸️ DEFERRED
  C4. Update AI filter source: use FilterToken for pre-check, get_full_token() for analysis ⏸️ DEFERRED
  C5. Implement incremental delta query (WHERE updated_at > last_refresh) [Component 2] ⏸️ DEFERRED
  C6. Implement snapshot merge logic (update/insert/remove) ⏸️ DEFERRED
  C7. Filtering refresh interval from resolved profile (300s/180s/120s) [Component 10] ⏸️ DEFERRED
  C8. Add config change → full refresh trigger ⏸️ DEFERRED
  C9. Add SQL pre-filtering for simple numeric thresholds [Component 9] ⏸️ DEFERRED
  
  RESULTS ACHIEVED:
  - RSS Memory: 1011 MB → 371 MB avg, 375 MB median, 483 MB peak (63% reduction)
  - Tokens Loaded: 172K → 15.6K (91% reduction via stale SQL filter)
  - Target ≤400 MB: ✅ MET (375 MB median, 483 MB peak both under target)
  - Disk Space Reclaimed: pools.db 729→0 MB, ohlcvs.db 354→175 MB (945 MB total recovered)
  - All 5 actual tasks (C1-C5) completed successfully
  - Still using full Token struct with Arc<Token> (not TokenListEntry)
  - Still using full refresh every 3 min (not incremental filtering)
  
  EXPECTED IMPACT (from original plan): 238 MB → 14-60 MB baseline, 476 MB → 70-120 MB peak.
  ACTUAL IMPACT (from stale filter): Better than planned — stale filter alone saved ~122 MB.
  VERIFICATION: ✅ VERIFIED — 10-minute test run completed successfully. RSS target achieved.
  ROLLBACK: Revert stale SQL filter by removing WHERE clause in assembly.rs.

Phase D — Maintenance Service (MEDIUM RISK):
  D1. Create MaintenanceService implementing Service trait [Component 5]
  D2. Implement periodic WAL checkpoint (interval from [maintenance] config)
  D3. Wire up RPC stats rotation — method exists (cleanup_stats), just needs timer call [Gap 1]
  D4. Wire up actions cleanup — method exists (cleanup_old_actions), just needs timer call [Gap 1]
  D5. Add sol_flow_cache cleanup function + timer call (retention from config) [Gap 2]
  D6. Implement batched VACUUM (one DB per cycle, trading-aware)
  D7. Implement stale token marking (inactive after configurable days)
  D8. Add maintenance window support (heavy ops only during window) [Component 10]
  D9. Add trading-aware checks (skip_during_active_trades from config)
  D10. Add periodic cleanup_expired_sessions() call for webserver auth sessions [Gap 9]
  D11. Add periodic cleanup for IMPORT_SESSIONS and MULTI_WALLET_SESSIONS [Gap 9]
  
  EXPECTED IMPACT: Prevents database bloat, keeps disk usage stable long-term.
  Config-driven: all retention periods and intervals from [maintenance] section.
  VERIFICATION: Run for 1 week, verify DB file sizes stay stable.
  ROLLBACK: Disable MaintenanceService via config (enabled=false). No data loss risk.

Phase E — Observability + Pressure Response + Tunability UI (LOW-MEDIUM RISK):
  E1. Implement memory pressure detection using resolved profile thresholds [Component 7+10]
  E2. Add pressure levels with hysteresis (thresholds from profile)
  E3. Add /api/system/memory endpoint (breakdown, caches, DBs, pressure) [Component 10]
  E4. Add /api/system/performance endpoint (profile, resolved values) [Component 10]
  E5. Add /api/system/maintenance endpoint (status, history) [Component 10]
  E6. Add dashboard Performance panel (memory breakdown, cache stats, profile selector)
  E7. Add Telegram notifications for pressure events
  E8. Add dashboard poll interval from resolved profile (15s/10s/5s) [Component 10]
  E9. Add cache efficiency logging (hit rate recommendations)
  E10. Add screenerbot-manager CLI commands (memory, performance, maintenance, caches)
  
  EXPECTED IMPACT: Full observability, OOM prevention, user self-service tuning.
  Users can see exactly what's happening and adjust profile/individual values.
  VERIFICATION: Change profiles, verify parameters change. Trigger pressure, verify response.
  ROLLBACK: Pressure response can be disabled via config. Dashboard panel is additive (no regression risk).

---

## ⚠️ Additional Gaps Found (v5 — Complete Audit)

### Gap 1: Cleanup Functions That EXIST But Are NEVER CALLED

| Function | Location | Called? | Impact |
|----------|----------|--------|--------|
| `cleanup_old_events()` | events/maintenance.rs:94 | ✅ YES — events_service starts maintenance task | OK |
| `cleanup_rejection_history/stats` | filtering_service.rs:152-254 | ✅ YES — every 10 min | OK |
| `cleanup_old_snapshots_sync()` | wallet balance_monitor/service.rs:285 | ✅ YES — in service loop | OK |
| `cleanup_expired_metrics()` | wallet balance_monitor/service.rs:288 | ✅ YES — in service loop | OK |
| `cleanup_filled_gaps()` | ohlcvs/monitor.rs:1526 | ✅ YES — in monitor loop | OK |
| **`cleanup_old_actions()`** | **actions/database.rs:885** | **❌ NEVER CALLED** | **actions.db grows forever** |
| **`cleanup_stats()`** | **rpc/manager.rs:612** | **❌ NEVER CALLED** | **rpc_stats.db grows forever (602K rows!)** |

**FIX**: Both must be called by the new MaintenanceService (Phase D). The methods already exist — 
they just need to be invoked on a timer.

### Gap 2: sol_flow_cache — NO Cleanup At All

**Location**: `wallets/balance_monitor/database.rs:76`
- SQLite table that stores SOL flow records (transaction signature + sol_delta)
- INSERT happens on every SOL transaction detected
- **NO cleanup function exists** — unlike snapshots (capped at 1000) and metrics (cleanup exists)
- Over months, this table grows unboundedly

**FIX**: Add `cleanup_old_flow_cache(retention_days: i64)` method + call from MaintenanceService.
Suggested retention: 30 days (matches events/actions pattern).

### Gap 3: TokenStore HashMap — No Capacity Limit

**Location**: `tokens/store.rs:164`
- TokenStore uses `HashMap<String, (Token, Instant)>` with 30s TTL
- Lazy eviction: entries only removed when accessed and found expired
- **NO max capacity** — HashMap grows if tokens are inserted faster than they expire
- With 275K tokens in DB, if many are accessed simultaneously, HashMap could hold 50K+ entries × ~1.5KB = 75 MB

**FIX**: Add to moka migration list (Phase B). Replace TokenStore's internal HashMap with 
`moka::sync::Cache` with max_capacity=10,000 and TTL=30s. This also replaces the custom lazy 
eviction logic.

### Gap 4: Dashboard Polling Architecture

**Current behavior** (confirmed from code):
- Dashboard home polls every **5 seconds** (hardcoded in home.js)
- `/api/dashboard/home` executes **8 parallel operations** per poll:
  - 5 position stats queries (today/yesterday/week/month/all-time)
  - 1 wallet status query
  - 1 full open positions list (`get_db_open_positions()` — **NO LIMIT**)
  - 1 system metrics snapshot (cached)
- `/api/status` loads open positions for count (lighter)

**Memory per poll** (50 open positions): ~63 KB allocated and freed every 5 seconds
**Memory per poll** (500 open positions): ~600 KB allocated and freed every 5 seconds
**Allocation churn**: 12 polls/min × 63-600 KB = 756 KB - 7.2 MB/minute of alloc/free cycles
→ Contributes to allocator fragmentation (Root Cause #4)

**Not a Phase A-E priority** but should be addressed:
- Reduce polling interval to 10-15s (config-driven)
- Add LIMIT to position queries in home endpoint
- Consider delta/diff responses (only send changes)
- Consider server-sent events for real-time updates (future)

### Gap 5: File Structure for New Modules

New files/modules needed by the plan:

```
src/database/
  common.rs          ← NEW: shared configure_sqlite_connection() [Component 8]
  mod.rs             ← NEW: pub mod common

src/maintenance/
  mod.rs             ← NEW: MaintenanceService [Component 5]
  tasks.rs           ← NEW: individual task implementations

src/filtering/
  filter_token.rs    ← NEW: FilterToken struct + SQL query [Component 3]
  incremental.rs     ← NEW: delta query + merge logic [Component 2]
  (engine.rs         ← MODIFY: use FilterToken + incremental)
  (store.rs          ← MODIFY: incremental refresh)

src/services/implementations/
  maintenance_service.rs  ← NEW: Service impl wrapper [Component 5]
  (mod.rs                 ← MODIFY: add pub mod maintenance_service)

Cargo.toml:
  moka = { version = "0.12", features = ["sync"] }         [Component 4]
  tikv-jemallocator = { version = "0.6", optional = true }  [Component 6]

Config additions (config/schemas/):
  maintenance.rs     ← NEW: MaintenanceConfig [Component 5]
  performance.rs     ← NEW: PerformanceConfig [Component 7]
  (mod.rs            ← MODIFY: add both to Config struct)
```

### Gap 6: Comprehensive Rust Test Strategy

#### Current Test Infrastructure (baseline)
- **~160 unit tests** across 30 `#[cfg(test)]` modules (inline, no separate test files)
- **dev-dependencies**: only `tempfile = "3"` — minimal
- **No `tests/` directory** — zero integration tests
- **No `benches/` directory** — zero benchmarks
- **No mocking framework** — tests use real objects with test parameters
- **In-memory SQLite pattern** already established: `SqliteConnectionManager::memory()` (used in ai/chat_db.rs)
- **`#[tokio::test]` pattern** already established (used in rpc/circuit_breaker)
- **Test constructors**: Some types have `::default()` or `::empty()` (e.g., FilteringSnapshot::empty())

#### Test Strategy Per Component

**Component 1+8: SQLite Configuration & Standardization** — 8 tests

```rust
// src/database/common.rs — #[cfg(test)] module

#[test]
fn test_configure_sqlite_connection_sets_wal() {
    let conn = Connection::open_in_memory().unwrap();
    let config = DbConnectionConfig { wal_enabled: true, ..Default::default() };
    configure_sqlite_connection(&conn, &config).unwrap();
    let mode: String = conn.pragma_query_value(None, "journal_mode", |r| r.get(0)).unwrap();
    assert_eq!(mode, "wal");
}

#[test]
fn test_configure_sqlite_connection_sets_cache_size() {
    let conn = Connection::open_in_memory().unwrap();
    let config = DbConnectionConfig { cache_size_pages: 2000, ..Default::default() };
    configure_sqlite_connection(&conn, &config).unwrap();
    let size: i32 = conn.pragma_query_value(None, "cache_size", |r| r.get(0)).unwrap();
    assert_eq!(size, -2000); // SQLite stores negative = pages
}

#[test]
fn test_configure_sqlite_connection_sets_mmap() {
    // Verify mmap_size PRAGMA is set (value clamped by OS, check it's non-zero)
}

#[test]
fn test_configure_sqlite_connection_sets_auto_vacuum() {
    // Verify auto_vacuum = INCREMENTAL after configure
}

#[test]
fn test_all_database_configs_reasonable() {
    // Verify no database has cache_size > 10000 (our new max)
    // Verify no mmap_size > 256MB
    // Verify no pool has > 8 read connections
    // This is a "policy" test that catches regressions
}
```

Additional tests:
- `test_configure_sets_busy_timeout` — verify 30s timeout
- `test_configure_sets_synchronous_normal` — verify NORMAL (not FULL)
- `test_default_db_connection_config` — verify Default impl matches expected

**Component 2: Incremental Filtering** — 6 tests

```rust
// src/filtering/incremental.rs — #[cfg(test)] module

// Setup: in-memory tokens.db with schema, insert test tokens

#[test]
fn test_delta_query_returns_only_updated_tokens() {
    // Insert 100 tokens at T=0
    // Update 5 tokens at T=1 (touch updated_at)
    // Run delta query with last_refresh=T=0.5
    // Verify: exactly 5 tokens returned
}

#[test]
fn test_delta_query_includes_new_tokens() {
    // Insert 100 tokens at T=0
    // Insert 3 NEW tokens at T=1
    // Run delta query → expect 3 new tokens
}

#[test]
fn test_merge_updates_existing() {
    // Start with snapshot of 100 FilterTokens
    // Merge delta of 5 updated FilterTokens
    // Verify: 100 total, 5 have new values, 95 unchanged
}

#[test]
fn test_merge_adds_new() {
    // Start with snapshot of 100
    // Merge delta with 3 new mints
    // Verify: 103 total
}

#[test]
fn test_merge_removes_deleted() {
    // Start with 100
    // Mark 2 as blacklisted in DB
    // Merge with blacklist check → 98 remaining
}

#[test]
fn test_incremental_matches_full_refresh() {
    // Insert 1000 tokens
    // Do full snapshot (reference)
    // Do N incremental updates (add/update/remove)
    // Do another full snapshot
    // Verify incremental result == full refresh result (mint sets match, values match)
    // This is the CRITICAL correctness test
}
```

**Component 3: FilterToken** — 5 tests

```rust
// src/filtering/filter_token.rs — #[cfg(test)] module

#[test]
fn test_filter_token_size() {
    // Verify FilterToken is significantly smaller than Token
    assert!(std::mem::size_of::<FilterToken>() < 500);
    // Token has String/Vec fields so size_of isn't total, but validates stack portion
}

#[test]
fn test_filter_token_from_sql_row() {
    // Create in-memory DB with tokens schema
    // Insert a full token with market data
    // Query using FilterToken SQL
    // Verify all 35 fields map correctly
}

#[test]
fn test_filter_token_null_handling() {
    // Insert token with NULL market data
    // Query as FilterToken
    // Verify all Optional fields are None (no crash)
}

#[test]
fn test_filter_token_has_all_fields_for_dexscreener() {
    // Verify FilterToken has every field that dexscreener.rs filter source accesses
    // (compile-time check + runtime field access test)
}

#[test]
fn test_filter_token_has_all_fields_for_rugcheck() {
    // Same for rugcheck fields
}
```

**Component 4: moka Cache Migration** — 10 tests

```rust
// Can be placed in each cache's module, or in a shared test module

#[test]
fn test_moka_cache_respects_max_capacity() {
    let cache: Cache<String, u8> = Cache::builder()
        .max_capacity(100)
        .build();
    for i in 0..200 {
        cache.insert(format!("key_{}", i), (i % 256) as u8);
    }
    cache.run_pending_tasks(); // moka is async — force eviction
    assert!(cache.entry_count() <= 100);
}

#[test]
fn test_moka_cache_ttl_expiry() {
    let cache: Cache<String, u8> = Cache::builder()
        .max_capacity(1000)
        .time_to_live(Duration::from_millis(100))
        .build();
    cache.insert("key".into(), 42);
    assert_eq!(cache.get(&"key".into()), Some(42));
    std::thread::sleep(Duration::from_millis(150));
    cache.run_pending_tasks();
    assert_eq!(cache.get(&"key".into()), None);
}

#[test]
fn test_decimals_cache_migration_behavior() {
    // Simulate DECIMALS_CACHE: insert 200K entries, verify oldest evicted at 150K cap
    // Verify: get() still works for recent entries
    // Verify: cache.entry_count() <= 150_000
}

#[test]
fn test_token_store_moka_replaces_timed_cache() {
    // Verify moka TokenStore behaves same as old TimedCache:
    // - Insert token, get within TTL → Some
    // - Insert token, sleep past TTL → None
    // - Insert beyond capacity → oldest evicted
}

#[test]
fn test_cache_metrics_available() {
    // Verify we can read hit_rate, entry_count, etc. from moka
    // This ensures our dashboard /api/system/caches endpoint has data
}
```

Additional tests per cache: FAILED_CACHE, TOKEN_2022_CACHE, SIGNATURES, PENDING_TX, 
POSITION_LOCKS cleanup on close, PRICE_HISTORY cleanup on close.

**Component 5: Maintenance Service** — 8 tests

```rust
// src/maintenance/mod.rs — #[cfg(test)] module

#[tokio::test]
async fn test_cleanup_old_actions() {
    // Create in-memory actions.db
    // Insert 100 actions: 50 from 2 days ago, 50 from 40 days ago
    // Run cleanup_old_actions(30)
    // Verify: 50 remaining (the recent ones)
}

#[tokio::test]
async fn test_cleanup_rpc_stats() {
    // Create in-memory rpc_stats.db
    // Insert records: 1000 from 1h ago, 1000 from 10 days ago
    // Run cleanup(retention_hours=168) // 7 days
    // Verify: only 1000 recent records remain
}

#[test]
fn test_sol_flow_cache_cleanup() {
    // Create in-memory wallet_monitor.db
    // Insert flow records with timestamps spanning 60 days
    // Run cleanup_old_flow_cache(30)
    // Verify: only last 30 days remain
}

#[test]
fn test_wal_checkpoint_succeeds() {
    // Create WAL-mode database, write 1000 rows
    // Run PRAGMA wal_checkpoint(PASSIVE)
    // Verify: no error, WAL file reduced
}

#[test]
fn test_maintenance_skips_during_active_trade() {
    // Set up fake trade-in-progress state
    // Call should_skip_maintenance()
    // Verify: returns true
}

#[test]
fn test_maintenance_window_logic() {
    // Test is_in_maintenance_window("02:00", "04:00") at various times
    // Verify: correct at 01:59 (false), 02:00 (true), 03:30 (true), 04:01 (false)
}

#[test]
fn test_vacuum_batching() {
    // Verify VACUUM only runs on one DB per cycle (round-robin)
    // Call 5 times → verify each DB gets VACUUM'd once
}

#[test]
fn test_retention_config_defaults() {
    // Verify MaintenanceConfig::default() has reasonable values
    let config = MaintenanceConfig::default();
    assert_eq!(config.events_retention_days, 30);
    assert_eq!(config.rpc_stats_retention_days, 7);
    assert!(config.enabled);
}
```

**Component 7+10: Memory Pressure & Tunability** — 12 tests

```rust
// src/config/schemas/performance.rs — #[cfg(test)] module

#[test]
fn test_auto_detect_profile_low() {
    assert_eq!(detect_memory_profile_for_ram(2048), MemoryProfile::Low);
    assert_eq!(detect_memory_profile_for_ram(3072), MemoryProfile::Low);
}

#[test]
fn test_auto_detect_profile_medium() {
    assert_eq!(detect_memory_profile_for_ram(4096), MemoryProfile::Medium);
    assert_eq!(detect_memory_profile_for_ram(8192), MemoryProfile::Medium);
    assert_eq!(detect_memory_profile_for_ram(12288), MemoryProfile::Medium);
}

#[test]
fn test_auto_detect_profile_high() {
    assert_eq!(detect_memory_profile_for_ram(16384), MemoryProfile::High);
    assert_eq!(detect_memory_profile_for_ram(65536), MemoryProfile::High);
}

#[test]
fn test_cascade_layer1_auto() {
    // No config overrides → auto-detection
    let config = PerformanceConfig::default(); // memory_profile = "auto"
    let resolved = resolve_profile(&config, 8192); // 8GB RAM
    assert_eq!(resolved.decimals_max, 150_000); // Medium default
}

#[test]
fn test_cascade_layer2_profile_overrides_auto() {
    let mut config = PerformanceConfig::default();
    config.memory_profile = "low".into();
    let resolved = resolve_profile(&config, 8192); // 8GB RAM but "low" profile
    assert_eq!(resolved.decimals_max, 50_000); // Low, not Medium
}

#[test]
fn test_cascade_layer3_individual_overrides_profile() {
    let mut config = PerformanceConfig::default();
    config.memory_profile = "low".into();
    config.max_decimals_cache_entries = 200_000; // Individual override
    let resolved = resolve_profile(&config, 8192);
    assert_eq!(resolved.decimals_max, 200_000); // Override wins
}

#[test]
fn test_zero_means_use_default() {
    let mut config = PerformanceConfig::default();
    config.max_decimals_cache_entries = 0; // 0 = use profile default
    let resolved = resolve_profile(&config, 8192);
    assert_eq!(resolved.decimals_max, 150_000); // Medium default, not 0
}

#[test]
fn test_pressure_level_calculation() {
    let thresholds = PressureThresholds { l1: 70.0, l2: 85.0, l3: 95.0 };
    assert_eq!(calculate_pressure(50.0, &thresholds), PressureLevel::Normal);
    assert_eq!(calculate_pressure(75.0, &thresholds), PressureLevel::Warning);
    assert_eq!(calculate_pressure(90.0, &thresholds), PressureLevel::Critical);
    assert_eq!(calculate_pressure(97.0, &thresholds), PressureLevel::Emergency);
}

#[test]
fn test_pressure_hysteresis() {
    // Entering L1 at 70%, should not exit L1 until 65% (5% hysteresis)
    let mut state = PressureState::new(PressureThresholds::default());
    state.update(75.0); // → L1
    assert_eq!(state.level(), PressureLevel::Warning);
    state.update(68.0); // Still L1 (hysteresis)
    assert_eq!(state.level(), PressureLevel::Warning);
    state.update(64.0); // → Normal (below hysteresis)
    assert_eq!(state.level(), PressureLevel::Normal);
}

#[test]
fn test_config_deserialization_missing_performance_section() {
    // TOML with NO [performance] section → defaults work
    let toml = r#"
        [rpc]
        selection_strategy = "adaptive"
    "#;
    let config: Config = toml::from_str(toml).unwrap();
    assert_eq!(config.performance.memory_profile, "auto");
}

#[test]
fn test_config_deserialization_partial_performance() {
    // TOML with partial [performance] → missing fields use defaults
    let toml = r#"
        [performance]
        memory_profile = "low"
    "#;
    let config: Config = toml::from_str(toml).unwrap();
    assert_eq!(config.performance.memory_profile, "low");
    assert_eq!(config.performance.max_decimals_cache_entries, 0); // Default
}
```

**Component 9: SQL Pre-Filtering** — 4 tests

```rust
// src/filtering/filter_token.rs or engine.rs — #[cfg(test)] module

#[test]
fn test_sql_where_clause_generation() {
    let config = DexScreenerFilters {
        min_liquidity_usd: Some(1000.0),
        min_volume_h24: Some(500.0),
        ..Default::default()
    };
    let clause = generate_sql_prefilter(&config);
    assert!(clause.contains("ds_liquidity_usd >= 1000"));
    assert!(clause.contains("ds_volume_h24 >= 500"));
}

#[test]
fn test_sql_prefilter_subset_of_full() {
    // Insert 1000 tokens with varying liquidity/volume
    // Run SQL pre-filtered query → N results
    // Run full query + Rust filter → M results
    // Verify: N >= M (pre-filter never excludes something Rust would pass)
    // i.e., pre-filter may be LESS strict, never MORE strict
}

#[test]
fn test_sql_prefilter_with_zero_thresholds() {
    // All thresholds = 0 or None → empty WHERE clause → return all tokens
    let config = DexScreenerFilters::default();
    let clause = generate_sql_prefilter(&config);
    assert!(clause.is_empty() || clause == "1=1");
}

#[test]
fn test_sql_prefilter_only_safe_conditions() {
    // Verify we never push OR logic to SQL, only AND with strict minimums
    // No price_usd filter (could be stale), no string comparisons
}
```

#### Integration Tests (`tests/` directory — NEW)

```
tests/
  memory_budget.rs          ← Verify total memory stays within profile budget
  filtering_pipeline.rs     ← Full filtering with in-memory DB + FilterToken
  maintenance_cycle.rs      ← Full maintenance cycle with in-memory DBs
  config_backward_compat.rs ← Old config.toml files still deserialize correctly
```

```rust
// tests/memory_budget.rs

#[test]
fn test_low_profile_total_cache_memory() {
    let profile = resolve_profile_for("low");
    let total_cache_bytes = 
        profile.decimals_max * 100 +      // ~100 bytes per entry
        profile.signatures_max * 88 +      // ~88 bytes per sig
        profile.token_store_max * 1500 +   // ~1.5 KB per token
        profile.sqlite_total_cache_bytes;
    let total_mb = total_cache_bytes / (1024 * 1024);
    assert!(total_mb < 200, "Low profile cache budget {} MB exceeds 200 MB", total_mb);
}

// tests/config_backward_compat.rs

#[test]
fn test_old_config_without_performance_section() {
    // Read a fixture config.toml that predates the performance section
    let old_config = include_str!("fixtures/config_v110.toml");
    let config: Config = toml::from_str(old_config).unwrap();
    // Verify it deserializes without error and has defaults
    assert_eq!(config.performance.memory_profile, "auto");
    assert!(config.maintenance.enabled);
}
```

#### Benchmarks (`benches/` directory — NEW)

```
benches/
  filtering.rs    ← Benchmark filtering with N tokens (100, 1K, 10K, 100K)
  cache_ops.rs    ← Benchmark moka vs old TimedCache (get/insert/evict)
  sql_queries.rs  ← Benchmark FilterToken query vs full Token query
```

```rust
// benches/filtering.rs (using criterion)

fn bench_filter_token_load(c: &mut Criterion) {
    // Setup: in-memory DB with 10K tokens
    let db = setup_test_db(10_000);
    
    c.bench_function("load_10k_filter_tokens", |b| {
        b.iter(|| {
            let tokens = load_filter_tokens(&db);
            black_box(tokens);
        })
    });
}

fn bench_full_token_load(c: &mut Criterion) {
    let db = setup_test_db(10_000);
    
    c.bench_function("load_10k_full_tokens", |b| {
        b.iter(|| {
            let tokens = load_full_tokens(&db);
            black_box(tokens);
        })
    });
}
```

#### Dev Dependencies Needed

```toml
[dev-dependencies]
tempfile = "3"           # Already present
criterion = "0.5"        # For benchmarks (optional, add when needed)
# moka — already in regular dependencies, available in tests
# tokio — already in regular dependencies with test-util
```

No mocking framework needed — the codebase pattern is to use real objects with in-memory 
databases and test constructors. This is correct for our use case since the core logic 
being tested is SQLite configuration, cache behavior, and pure functions.

### Gap 7: r2d2 SQLite Pool Connection Lifecycle — CORRECTED (v6)

**⚠️ v6 CORRECTION**: The v5 analysis stated r2d2 had "NO idle_timeout" — this was **WRONG**.

**Verified fact** (from r2d2 0.8.10 source code, line 59-60):
```rust
// r2d2 Builder::default()
idle_timeout: Some(Duration::from_secs(10 * 60)),   // 10 minutes
max_lifetime: Some(Duration::from_secs(30 * 60)),   // 30 minutes
```

r2d2 **DOES** have idle_timeout (default 10 min) and max_lifetime (default 30 min). Since 
none of our SQLite pool builders explicitly set these, **the defaults apply** — connections 
ARE reaped after 10 min idle or 30 min total lifetime. Connections do NOT live forever.

**The REAL problem is different**: When r2d2 drops an idle/expired connection and creates 
a NEW one, that new connection **does not get PRAGMA configuration** because most pools 
(12 out of 14) don't use `with_init()` or `CustomizeConnection`. Three patterns exist:

| Pattern | Databases Using It | Behavior |
|---------|-------------------|----------|
| **`with_init()`** (CORRECT) | transactions.db, ai/chat.db (2 of 14) | Every new connection gets PRAGMAs ✅ |
| **PRAGMAs in `initialize_schema()`** | tools.db, strategies.db, wallets.db, rpc_stats.db, tokens.db, wallet_monitor.db (6 of 14) | Only first connection gets PRAGMAs ❌ Recycled connections use SQLite defaults! |
| **PRAGMAs on every `get_connection()`** | events.db, positions.db, actions.db (3 of 14 with both read+write pools) | Wasteful but works ✅ (re-sets PRAGMAs on checkout) |

**Impact of Pattern #2 (the 6 broken databases)**:
- After 30 min (max_lifetime), r2d2 replaces the connection
- New connection gets SQLite defaults: cache_size=2000 (8MB), no WAL, synchronous=FULL
- This is actually LESS memory than our configured 10000-page caches! But performance 
  suffers (no WAL = journal mode DELETE = slower writes, synchronous=FULL = fsync on every commit)
- This means ~6 databases have been running with MIXED configurations: initial connections 
  use our settings, recycled connections use SQLite defaults. Nobody noticed because the 
  defaults still work, just slower.

**REVISED FIX (Phase A — simpler than v5 proposed)**:
```rust
// Migrate ALL databases to use with_init() pattern:
let manager = SqliteConnectionManager::file(db_path)
    .with_init(|c| {
        configure_sqlite_connection(c, &DbConnectionConfig::standard())?;
        Ok(())
    });
let pool = Pool::builder()
    .max_size(N)
    .min_idle(Some(1))
    // Keep r2d2 defaults: idle_timeout=10min, max_lifetime=30min
    // These are GOOD values — connections get recycled AND re-initialized via with_init
    .build(manager)?;
```

This is no longer "the single most impactful fix" — it's a correctness fix that ensures 
recycled connections get proper PRAGMAs. The BIGGEST impact is still cache_size reduction.

**Optional tuning**: For databases accessed rarely (strategies.db, ai/chat.db), we could 
reduce idle_timeout to 5 min to free memory faster:
```rust
.idle_timeout(Some(Duration::from_secs(300))) // 5 min for cold DBs
```

### Gap 8: AI Cache (DashMap) — No Max Capacity

**Location**: `src/ai/cache.rs:13` — `cache: DashMap<String, CachedEntry>`

**Behavior**: Has TTL-based lazy eviction in `get()` and `stats()`, but:
- NO max capacity limit
- NO periodic cleanup — entries only evicted when accessed after TTL
- No `cleanup_expired()` method that runs on a timer
- If AI evaluates 171K tokens (filtering), 171K entries could accumulate in cache

**FIX**: Add to moka migration list in Phase B. Replace with:
```
moka::sync::Cache(max: 5000, TTL: cache_ttl_seconds from config)
```

### Gap 9: 15+ Additional Global LazyLock Stores NOT Audited in Plan

The plan identified 8 leak caches. Full audit found **30+ global LazyLock stores** total. 
Most are bounded or transient, but several were not analyzed:

| Store | Location | Type | Cleanup? | Risk |
|-------|----------|------|----------|------|
| FETCH_LOCKS | tokens/decimals.rs:34 | HashMap<String, Arc<AsyncMutex>> | ✅ Per-use release + 10K safety cap | **BOUNDED** (v7 verified) |
| TOKEN_POOLS_CACHE | tokens/pool_data/cache.rs:38 | HashMap<String, CacheEntry> | ⚠️ Has TTL check + manual clear() but no periodic eviction | SLOW LEAK — stale entries stay in HashMap |
| POOL_REFRESH_INFLIGHT | tokens/pool_data/cache.rs:41 | HashMap<String, Arc<Notify>> | ⚠️ Implicit (notify completes) | Low risk — short-lived |
| POOL_PREFETCH_STATE | tokens/pool_data/cache.rs:44 | HashMap<String, Instant> | ⚠️ Cleared only via clear_cache() with TOKEN_POOLS_CACHE | SLOW LEAK — no independent cleanup |
| COMPUTATION_FAILURES | wallets/balance_monitor/cache.rs:118 | HashMap<String, (u32, Instant)> | ✅ circuit_reset() removes on success. Keys are &'static str (fixed set ~5-10) | **BOUNDED** (v7 verified) |
| SESSIONS (webserver) | webserver/session.rs:23 | HashMap<String, Session> | ✅ cleanup_expired_sessions() | OK — BUT only called lazily on validate |
| IMPORT_SESSIONS | webserver/routes/wallets/types.rs:32 | HashMap<String, ImportSession> | ⚠️ Only on wallet import route | LEAK if imports abandoned |
| MULTI_WALLET_SESSIONS | webserver/routes/tools/multi_wallet/session.rs:38 | HashMap<String, MultiWalletSession> | ❓ Unknown | Needs audit |
| ACTIVE_ACTIONS | actions/state.rs:15 | HashMap<ActionId, Action> | ❌ No remove visible | LEAK — hot cache of actions |
| AI permissions pending | ai/permissions.rs:171 | DashMap<String, PendingConfirmation> | ✅ cleanup_expired() exists | OK — but is it called on timer? |
| Telegram pagination | telegram/pagination.rs:17 | DashMap<String, PaginationSession> | ✅ cleanup() on each access | OK — opportunistic cleanup |
| SIG_TO_MINT_INDEX | positions/state.rs:15 | HashMap<String, String> | ✅ remove() on position close | OK |
| ENTRY_CYCLE_RESERVATIONS | trader/monitors/entry.rs:23 | HashMap<String, Instant> | ✅ retain() on each access | OK |
| PENDING_OPEN/DCA/PARTIAL | positions/state.rs:27-60 | HashMap<String, ...> | ✅ Removed on swap completion | OK — transient |
| IN_FLIGHT_TOKENS | tokens/updates/helpers.rs:12 | HashSet<String> | ✅ Removed after update | OK — transient |

**TRUE CONFIRMED LEAKS (3 — reduced from 5 after v7 verification)**:
- **ACTIVE_ACTIONS**: Hot cache that never removes completed actions (DB is source of truth but HashMap grows)
- **POSITION_LOCKS**: Grows per unique mint traded. Never removes locks.
- **TOKEN_2022_CACHE**: Grows per unique token checked. No removal mechanism.

**SLOW LEAKS (2 — have TTL/clear but no periodic eviction)**:
- **TOKEN_POOLS_CACHE**: Has TTL freshness check + manual clear_cache(), but stale entries remain in HashMap
- **POOL_PREFETCH_STATE**: Cleared only via clear_cache() alongside TOKEN_POOLS_CACHE

**BOUNDED (corrected from leak in v7)**:
- **FETCH_LOCKS**: Per-use release_lock_if_idle() + 10K safety cap with map.clear() — NOT a leak
- **COMPUTATION_FAILURES**: Uses &'static str keys (fixed set of ~5-10 window names) + circuit_reset() on success — NOT a leak

**FIX**: 
- FETCH_LOCKS → **No fix needed** (already bounded via per-use release + 10K cap)
- TOKEN_POOLS_CACHE → Migrate to moka (max: 5000, TTL: pool_cache_ttl from config)
- POOL_PREFETCH_STATE → Migrate to moka (max: 5000, TTL: 5 min)  
- COMPUTATION_FAILURES → **No fix needed** (bounded by fixed &'static str key set, ~5-10 entries max)
- ACTIVE_ACTIONS → Add periodic remove for completed/failed actions

**Webserver session cleanup note**: `cleanup_expired_sessions()` exists in two places:
1. `webserver/session.rs:108` — for auth sessions (not called on timer!)
2. `webserver/routes/wallets/utils.rs:6` — for wallet import sessions (called per-import)
Auth session cleanup should be added to MaintenanceService timer (Phase D).

### Gap 10: OHLCV Cache — Analyzed, BOUNDED (Minor Note)

**Hot cache**: `HOT_CACHE_MAX_TOKENS = 100` tokens, `HOT_CACHE_RETENTION_HOURS = 24`, LRU eviction ✅
- Each entry: Vec<Candle> where Candle = 48 bytes. Typical: 500 candles × 48 = 24 KB per entry
- 100 entries × 24 KB = ~2.4 MB — negligible

**Bundle cache**: `BUNDLE_CACHE_MAX_SIZE = 150` entries, `BUNDLE_CACHE_TTL_SECONDS = 30`, LRU ✅
- Each TimeframeBundle: 7 timeframes × 100 candles × 48 bytes = ~33.6 KB per bundle
- 150 bundles × 33.6 KB = ~5 MB — manageable

**Verdict**: OHLCV caches are properly bounded. No action needed. Already has 
`cache_maintenance_loop()` that runs cleanup_expired(). ✅ This is how all caches should work.

### Gap 11: Embedded Assets — 15 MB Baked Into Binary

**Discovery**: 262 `include_str!` + `include_bytes!` calls in `webserver/embeds.rs` (368 lines).
**Total asset size**: 15 MB on disk, embedded into binary's .rodata section.

**Largest assets**:
| Asset | Size | Needed? |
|-------|------|---------|
| lucide.svg (icon font SVG) | 7.7 MB | ⚠️ Probably not served — woff2 used instead |
| lucide.symbol.svg | 3.9 MB | ⚠️ Probably not served |
| lucide.ttf | 724 KB | Fallback font format |
| lucide.eot | 724 KB | Fallback font format (IE only) |
| lightweight-charts.js | 160 KB | ✅ Essential |
| JetBrains Mono (3 weights) | 284 KB total | ✅ Essential |

**Impact on RSS**: These are memory-mapped from the binary. Only pages actually accessed 
count in RSS. The 7.7 MB lucide.svg is probably never accessed (woff2 is used), so it wastes 
binary size but not RSS. EOT format is for IE only — dead weight.

**FIX (LOW PRIORITY — not memory but binary size)**:
- Remove lucide.svg (7.7 MB), lucide.symbol.svg (3.9 MB), lucide.eot (724 KB) if not served
- Saves ~12 MB in binary size
- Not a memory fix per se, but reduces download size for users

### Gap 12: Channel Buffer Sizes — Bounded but Worth Documenting

All async channels in the codebase are bounded ✅:

| Channel | Buffer Size | Location | Risk |
|---------|------------|----------|------|
| Event mpsc | 10,000 | events/mod.rs:119 | ~2-5 MB if full (depends on Event size) |
| Event broadcast | 5,000 | events/mod.rs:115 | ~1-2.5 MB if full |
| RPC stats mpsc | 1,000 | rpc/stats/collector.rs:118 | ~100 KB |
| Actions broadcast | 1,000 | actions/broadcast.rs:10 | ~100 KB |
| Telegram mpsc | 100 | telegram/service.rs:133 | ~10 KB |

**Event channel at 10,000 capacity is the main concern**. If event processing is slow 
(e.g., database write contention), 10K events buffer up. Each Event contains category, 
severity, JSON data — could be 200-500 bytes each. Worst case: 10K × 500 bytes = 5 MB.

**Verdict**: All bounded, no action needed for memory architecture. But the event channel 
capacity (10K) could be reduced to 2K-5K in the Low memory profile (Component 10) if desired.

### Gap 13: DashMap Shrink Limitation (PRICE_CACHE, PRICE_HISTORY)

**Issue**: The plan keeps PRICE_CACHE and PRICE_HISTORY as DashMap (not migrating to moka) 
because they have existing cleanup via `cleanup_stale_entries()` and `PRICE_HISTORY_MAX_ENTRIES = 1000`.

**Hidden problem**: DashMap's internal hash table **never shrinks** after entries are removed.
- If a burst of pool discoveries creates 50K price cache entries
- Then cleanup removes 49K of them
- DashMap still holds the hash table sized for 50K (bucket memory not deallocated)
- This is a known DashMap limitation: `.retain()` removes values but doesn't shrink allocation

**Impact**: Minor (~1-5 MB), but worth noting. The fix would be to periodically replace the 
DashMap with a fresh one (clone active entries → swap), but this adds complexity.

**Verdict**: Document as known limitation. Not worth fixing in Phase A-E unless observed as 
significant in production. moka would solve this (its internal structures auto-compact).

---

### Summary of ALL Gaps (v5)

| Gap | Severity | Phase | Description |
|-----|----------|-------|-------------|
| 1 | HIGH | D | cleanup_old_actions() and cleanup_stats() never called |
| 2 | HIGH | D | sol_flow_cache has no cleanup at all |
| 3 | MEDIUM | B | TokenStore HashMap no capacity limit |
| 4 | MEDIUM | E | Dashboard polls all positions every 5s |
| 5 | INFO | A-E | File structure for new modules |
| 6 | INFO | A-E | Test strategy (61 tests planned) |
| **7** | **HIGH→MEDIUM** | **A** | **r2d2 pools DO have idle_timeout (10min default) — but 12/14 DBs don't use with_init(), so recycled connections lose PRAGMAs** |
| **8** | **MEDIUM** | **B** | **AI cache DashMap — no max capacity** |
| **9** | **HIGH→MEDIUM** | **B+D** | **3 true leaks (ACTIVE_ACTIONS, POSITION_LOCKS, TOKEN_2022_CACHE) + 2 slow leaks (TOKEN_POOLS_CACHE, POOL_PREFETCH_STATE). FETCH_LOCKS and COMPUTATION_FAILURES verified BOUNDED in v7.** |
| **10** | **NONE** | **—** | **OHLCV cache — already properly bounded ✅** |
| **11** | **LOW** | **Future** | **Embedded assets: 12 MB of dead weight (lucide SVGs, EOT font)** |
| **12** | **NONE** | **—** | **Channel buffers — all bounded ✅** |
| **13** | **LOW** | **Future** | **DashMap never shrinks after peak — known limitation** |

#### Test Count Summary

| Component | Unit Tests | Integration | Benchmarks | Total |
|-----------|-----------|-------------|------------|-------|
| 1+8 SQLite Config | 8 | 1 | 1 | 10 |
| 2 Incremental Filtering | 6 | 1 | 1 | 8 |
| 3 FilterToken | 5 | — | 1 | 6 |
| 4 moka Caches | 10 | — | 1 | 11 |
| 5 Maintenance | 8 | 1 | — | 9 |
| 7+10 Pressure+Tunability | 12 | 1 | — | 13 |
| 9 SQL Pre-Filtering | 4 | — | — | 4 |
| **Total** | **53** | **4** | **4** | **61** |

#### Test Placement Rules (aligns with existing codebase patterns)

1. **Unit tests** → `#[cfg(test)] mod tests` at bottom of the file being tested (existing pattern)
2. **Integration tests** → `tests/` directory (NEW — test cross-module behavior)
3. **Benchmarks** → `benches/` directory (NEW — measure performance, not correctness)
4. **Test fixtures** → `tests/fixtures/` (e.g., old config.toml files for backward compat)
5. **Test helpers** → if shared across modules, create `src/test_utils.rs` with `#[cfg(test)]`
   containing: `setup_in_memory_db()`, `create_test_tokens(n)`, `create_test_config(profile)`

#### When Tests Run During Implementation

- **Phase A**: Write SQLite config tests FIRST (TDD — tests define expected PRAGMAs before code)
- **Phase B**: Write moka behavior tests FIRST, then migrate caches, verify tests pass
- **Phase C**: Write `test_incremental_matches_full_refresh` FIRST — this is the safety net
- **Phase D**: Write cleanup tests with in-memory DBs, then wire up callers
- **Phase E**: Write pressure level + profile resolution tests FIRST (pure logic, easy TDD)

#### Critical "Never Regress" Tests

These tests MUST pass forever — they prevent re-introducing memory issues:

1. `test_all_database_configs_reasonable` — catches anyone re-adding cache_size=20000
2. `test_moka_cache_respects_max_capacity` — catches unbounded cache introduction
3. `test_incremental_matches_full_refresh` — catches filtering correctness drift
4. `test_low_profile_total_cache_memory` — catches total budget exceeding safe limits
5. `test_config_deserialization_missing_performance_section` — catches config upgrade breakage

---

## 🧠 Design Guidelines (for future development)

After implementing all phases, these rules prevent regression:

1. **Every global collection MUST have a max size** — use moka with configured max entries
2. **Every database MUST use configure_sqlite_connection()** — no ad-hoc PRAGMA setup
3. **Every background service MUST have cleanup logic** — no unbounded accumulation
4. **No collect::<Vec<_>>() on unbounded queries** — always add LIMIT or use streaming
5. **Filter sources operate on FilterToken, not Token** — full Token only for individual lookups
6. **New caches: default to moka** — DashMap only when you need unbounded AND can justify it
7. **Connection pools: max 5 for most databases** — SQLite is single-writer, 10 is almost always excessive
8. **Test with 250K+ tokens** — memory behavior at scale, not just with 100 test tokens
9. **Every cleanup function MUST be called on a timer** — not just defined (see Gap 1)
10. **New database tables MUST have retention policy** — define max age or max rows at creation time

---

## 📈 Expected Results After All Phases

| Metric | Current | After Phase A | After A+B | After A+B+C | After All |
|--------|---------|--------------|-----------|-------------|-----------|
| RSS at startup | 804 MB | ~400-500 MB | ~350-450 MB | ~200-300 MB | ~150-250 MB |
| RSS after 24h | 2+ GB | ~600-800 MB | ~500-600 MB | ~300-400 MB | ~200-350 MB |
| RSS peak (filtering) | ~1 GB | ~500-600 MB | ~450-550 MB | ~120-200 MB | ~120-200 MB |
| Disk usage | 1.63 GB | ~750 MB | ~750 MB | ~750 MB | ~750 MB |
| Memory growth/day | Unbounded | Bounded (SQLite) | Fully bounded | Fully bounded | Fully bounded |
| Maintenance needed | Manual | Manual | Manual | **Automatic** | **Automatic** |
| OOM risk (2GB VPS) | **HIGH** | Medium | Low | Low | **Very Low** |

---

## Phase 5: Memory/RAM Investigation (COMPLETED ✅ — Investigation Only, No Code Changes)

### Measured Baseline
- **Bot RSS at startup: 804 MB** (with 275K tokens in DB, 171K with market data)
- **Database on disk: 1.63 GB total** (pools.db 729MB is 99.99% empty — needs VACUUM)

## Phase 1: Security Audit (COMPLETED ✅)
All web security fixes applied, deployed, verified.

---

## Phase 2: screenerbot.sh Deep Review — INVESTIGATION FINDINGS

**Script**: `ScreenerBot/screenerbot.sh` (3065 lines, v1.1.2)
**VPS Test Copy**: `/tmp/screenerbot-test.sh` (ready for manual testing)
**Status**: Investigation complete. Big fixes deferred to next phase.

### 🔴 CRITICAL Issues (Must Fix Before Release)

1. **WALLET DATA DELETION POSSIBLE** (Lines 984-985, 1216)
   - `uninstall()` asks "Remove data directory?" then does `rm -rf "$data_dir"`
   - `restore_backup()` also does `rm -rf "$data_dir"` before restore
   - Data dir contains: wallet.db, wallets.db, config.toml (with private keys)
   - User requirement: "deleting wallets keys must not possible in script"
   - **FIX**: Remove all data dir deletion code entirely. Show info message instead.

2. **JSON INJECTION IN PASSWORD API** (Lines 1735-1739)
   - Passwords directly interpolated: `"{\"new_password\":\"$password\"}"`
   - Password like `test","admin":true,"` injects arbitrary JSON
   - **FIX**: Use jq for safe JSON construction

### 🟡 HIGH Issues

3. **Path Traversal in Backup Restore** (Lines 1096-1108, 1232)
   - tar extraction without `--no-absolute-names`
   - Malicious tarball can write anywhere on filesystem
   - **FIX**: Validate tarball contents, use safe tar flags

4. **Unsafe rm -rf with Potentially Empty Variables** (Lines 973, 985, 1216)
   - If `$data_dir` or `$INSTALL_DIR` is empty → catastrophic deletion
   - **FIX**: Guard all rm -rf with empty-string checks

5. **Unvalidated Variables in systemd Service** (Lines 1304-1319)
   - `$user`, `$home_dir` interpolated into unit file without sanitization
   - **FIX**: Validate username format before embedding

### 🟡 MEDIUM Issues

6. **No Binary Checksum Verification** (Lines 860-876)
7. **Telegram Token in systemd Journal Logs** (Lines 1592-1596)
8. **curl | bash Pattern Without Integrity Verification** (Lines 19-23)
9. **Symlink Attack in ln -sf** (Line 910)
10. **Trap Handler Conflicts** (Lines 850, 2048, 2234, 2256)

### 🟢 macOS Compatibility Issues (65 locations)

| Category | Count | Impact |
|----------|-------|--------|
| systemctl/systemd | 34 | Blocker - need launchd |
| /proc filesystem | 13 | Blocker - doesn't exist |
| GNU coreutils | 6 | Blocker - BSD variants differ |
| getent passwd | 5 | Blocker - use dscl |
| free command | 4 | Blocker - use vm_stat |
| ip command | 2 | Blocker - use ifconfig |
| Package managers | 1 | No Homebrew support |

Estimated effort: ~300 LOC abstraction layer, 2-3 days

### ✅ VERIFIED (No Issues)

- **`screenerbot-manager` command name**: Already correct at line 75
- **Italic banner**: Working with proper fallback (line 113)
- **Website/docs URLs**: All consistent and correct
  - install.sh redirect works (307 → GitHub raw)
  - VPS docs page comprehensive with 13 commands
  - ⚠️ Minor: download page uses `sudo bash` vs docs uses `bash`

### Next Phase: Fixes
- [ ] Remove all wallet deletion code (CRITICAL)
- [ ] Fix JSON injection with jq (CRITICAL)
- [ ] Add path traversal protection to restore
- [ ] Add rm -rf safety guards
- [ ] Add macOS compatibility layer (large task)
- [ ] Add binary checksum verification
- [ ] Unify install commands (sudo bash vs bash)

---

## 📦 Dependency Version Research (v6 — Complete Analysis)

> Research completed by examining official docs.rs documentation, crates.io, GitHub releases,
> and verified against actual r2d2 source code. All findings verified, not based on web search alone.

### Complete Dependency Version Matrix

| Crate | Current | Latest Stable | Breaking? | Upgrade Risk | Recommendation |
|-------|---------|--------------|-----------|-------------|----------------|
| **rusqlite** | 0.37.0 | 0.38.0 | YES | MEDIUM | Defer — not needed for memory work |
| **r2d2** | 0.8.10 | 0.8.10 | — | — | ✅ Already latest |
| **r2d2_sqlite** | 0.31.0 | 0.32.0 | YES (needs rusqlite 0.38) | MEDIUM | Defer — coupled with rusqlite upgrade |
| **dashmap** | 5.5.3 | 6.1.0 (stable) | YES (5→6) | LOW-MEDIUM | Optional — 5.5.3 works fine |
| **sysinfo** | 0.30.13 | 0.38.1 | YES (many) | HIGH | Defer — MSRV 1.88 required, 8 breaking versions |
| **moka** | (new dep) | 0.12.13 | N/A | LOW | Add — `features = ["sync"]` |
| **tikv-jemallocator** | (new dep) | 0.6.1+ | N/A | LOW | Add — optional feature, CVE-2025-62518 note |

### Detailed Analysis Per Dependency

#### rusqlite 0.37 → 0.38 (DEFER)

**Breaking changes in 0.38**:
- Statement cache now optional (feature flag)
- `u64`/`usize` `ToSql`/`FromSql` impls disabled by default
- Shell scripts removed from published package
- `libsqlite3-sys` is now `no_std`
- Minimum SQLite version bumped to 3.34.1

**Impact on us**: We use `features = ["bundled"]` so SQLite version is managed by the crate.
The `u64`/`usize` change could break our code if we pass `u64` values to SQLite. Need audit.

**Recommendation**: NOT needed for memory optimization. Defer to a separate upgrade PR.
Stay on 0.37 for Phase A-E. The `with_init()` API we need is available in 0.31/0.37.

#### r2d2 0.8.10 (ALREADY LATEST ✅)

**CRITICAL DISCOVERY** (v6): r2d2 0.8.10 has BOTH `idle_timeout` AND `max_lifetime` with 
sensible defaults (10 min / 30 min). The v5 claim that r2d2 "has NO idle_timeout" was **wrong**.

**Our pools do NOT override these defaults**, so connections ARE recycled. The real problem 
is that recycled connections don't get PRAGMAs re-applied (see corrected Gap 7).

#### r2d2_sqlite 0.31 → 0.32 (DEFER — coupled with rusqlite)

Requires rusqlite 0.38. The `with_init()` API we need exists in 0.31 already.
No benefit to upgrading for memory work.

#### dashmap 5.5.3 → 6.1.0 (OPTIONAL)

**Breaking changes (5→6)**:
- `equivalent` instead of `Borrow` for key equality
- MSRV bumped to 1.70
- `hashbrown` upgraded to 0.15
- `SharedValue` abstraction removed

**Impact on us**: We use DashMap extensively (PRICE_CACHE, PRICE_HISTORY, AI cache, Telegram).
The `Borrow→equivalent` change could require code adjustments.

**Recommendation**: NOT needed for memory work. moka replaces most DashMaps. The remaining 
DashMaps (PRICE_CACHE, PRICE_HISTORY) work fine on 5.5.3. Defer to future cleanup.

Note: dashmap 7.0.0-rc2 is in development — wait for stable 7.0 if upgrading.

#### sysinfo 0.30 → 0.38 (DEFER — HIGH RISK)

**8 breaking versions between 0.30 and 0.38**:
- 0.31: Removed `refresh_process` — must use `refresh_processes(ProcessesToUpdate::some(vec![pid]))`
- 0.32: API argument changes
- 0.34: `multithread` off by default
- 0.35: Open files APIs changed
- 0.37-0.38: MSRV bumped to **1.88** (!)

**Impact on us**: We use sysinfo for dashboard system metrics + plan to use for memory detection.
The API for `System::total_memory()` and `Process::memory()` (which we need) likely still works 
but method names may have changed.

**MSRV 1.88 is a concern**: Our current MSRV is likely lower. Need to verify.

**Recommendation**: Stay on 0.30 for Phase A-E. The basic `System::total_memory()` API works.
Upgrade as a separate effort when needed.

#### moka 0.12.13 (NEW — Add in Phase B)

**Critical API details for our architecture**:

```rust
// Cargo.toml
moka = { version = "0.12", features = ["sync"] }

// Basic usage
use moka::sync::Cache;
let cache: Cache<String, u8> = Cache::builder()
    .max_capacity(150_000)
    .time_to_live(Duration::from_secs(300))  // optional TTL
    .build();

cache.insert("key".to_string(), 42);
cache.get(&"key".to_string())  // Returns Option<u8> (cloned!)
cache.entry_count()             // Approximate count
cache.weighted_size()           // Approximate weighted size
cache.invalidate_all()          // Clear all entries
cache.run_pending_tasks()       // Force eviction processing
```

**⚠️ v6 CORRECTION — NO runtime resize**: 
The v5 plan claimed `.policy().set_max_capacity()` — this does NOT exist. 
max_capacity is **fixed at creation time**. To resize, you must create a new cache 
and migrate entries. This affects Component 10 (Tunability):

**Impact on tunability architecture**:
- Profile changes that affect cache sizes are now **restart-required**, not hot-reloadable
- Or: implement cache swap pattern — create new cache with new size, populate from old, swap
- Workaround for memory pressure: use `.invalidate_all()` to clear, but can't shrink max

**Other key facts**:
- No background threads (removed in 0.12) — must call `run_pending_tasks()` manually
- `.get()` returns cloned value, not reference — trivial for `u8`/`u64`, use `Arc<T>` for large values
- Eviction policy: TinyLFU (default, best for most workloads) or LRU (for streaming)
- Thread-safe — uses internal sharding like DashMap
- Feature `"sync"` required for `moka::sync::Cache`
- Feature `"future"` available for `moka::future::Cache` (async contexts)

#### tikv-jemallocator 0.6.1+ (NEW — Add in Phase A)

```toml
[dependencies]
tikv-jemallocator = { version = "0.6", optional = true }

[features]
default = ["jemalloc"]
jemalloc = ["tikv-jemallocator"]
```

**CVE-2025-62518**: Affects tikv-jemalloc-sys/tikv-jemallocator — PAX header 
desynchronization in TAR archives. **NOT relevant to us** — we don't parse TAR files.
The CVE was patched in downstream Fedora packages, likely via dependency updates.
Use `0.6` version range which resolves to latest patch.

**Build considerations**:
- Compiles jemalloc from C source — adds ~30-60s to build
- Needs C toolchain for target platform (already have via Rust build)
- `#[cfg(not(target_env = "msvc"))]` — graceful Windows fallback
- musl-libc: may need testing (static builds on Alpine)

### Dependency Upgrade Strategy

**Phase A-E (Memory Optimization)**: 
- ADD: moka 0.12 (features=["sync"]), tikv-jemallocator 0.6 (optional)
- KEEP: rusqlite 0.37, r2d2_sqlite 0.31, dashmap 5.5, sysinfo 0.30
- No existing dependency upgrades — minimize risk

**Future (post Phase E)**:
- Upgrade rusqlite 0.37→0.38 + r2d2_sqlite 0.31→0.32 (as a pair)
- Upgrade sysinfo 0.30→latest (when MSRV allows)
- Upgrade dashmap 5.5→6.x or 7.x (when stable)
- Consider: remove some DashMap dependencies in favor of moka

### v6 Critical Corrections Summary

| # | What Was Wrong (v5) | What Is Correct (v6) | Impact on Plan |
|---|--------------------|--------------------|---------------|
| 1 | "r2d2 has NO idle_timeout — connections live forever" | r2d2 HAS idle_timeout (10 min default) and max_lifetime (30 min default). Connections ARE recycled. | Gap 7 downgraded CRITICAL→HIGH/MEDIUM. A5 changed from "add timeouts" to "add with_init()" |
| 2 | "moka `.policy().set_max_capacity()` for hot-reload" | moka max_capacity is **fixed at creation**. NO runtime resize API exists. | Tunability table: moka capacities now ❌ restart-required, or need cache swap pattern |
| 3 | "PRAGMA Pattern #2 databases use configured cache_size" | Pattern #2 databases lose PRAGMAs when r2d2 recycles connections (every 30 min). They fall back to SQLite defaults (cache_size=2000). | Some databases use LESS memory than calculated! But also have worse performance (no WAL, synchronous=FULL) |

### Memory Estimate Revision (v6)

Given correction #3, the "at rest" memory for Pattern #2 databases is LOWER than v5 estimated:

**Pattern #2 databases (6 DBs) — recycled connections use SQLite defaults**:
- tools.db: cache_size=2000 (8 MB/conn, not 40 MB)
- strategies.db: cache_size=2000 (8 MB/conn)
- wallets.db: cache_size=2000 (8 MB/conn, not 20 MB)
- rpc_stats.db: cache_size=2000 (8 MB/conn)
- tokens.db: cache_size=2000 (8 MB/conn, not 40 MB) — initial connection had 10000 but recycled ones don't
- wallet_monitor.db: cache_size=2000 (8 MB/conn, not 40 MB)

**Revised "at rest" estimate**: Initial connections (first 30 min) use configured values.
After 30 min (max_lifetime), recycled connections use defaults:
- v5 estimate: ~580 MB at rest (all connections using configured cache_size)
- v6 estimate: ~350-400 MB at rest (mixed: Pattern #1/#3 use configured, Pattern #2 use defaults)
- After fix (all with_init()): ~84 MB at rest (all using our right-sized values)

The actual observed 804 MB at startup still makes sense because:
1. Initial connections DO get configured cache_sizes (haven't been recycled yet)
2. Filtering snapshot (238 MB) + allocator overhead (~200 MB) account for the rest
3. After 30+ min, some connections recycle and memory drops for Pattern #2 databases

This means the Phase A impact might be slightly different than projected:
- We're reducing FROM 350-400 MB (not 580 MB) TO 84 MB for SQLite caches
- Still a ~270-320 MB reduction — significant but not as dramatic as v5 claimed

### v7 Source Code Verification Corrections

Deep source code verification against ALL plan claims found these corrections:

| # | What Plan Claimed | What Source Code Shows (v7 verified) | Impact on Plan |
|---|-------------------|--------------------------------------|---------------|
| 1 | "FETCH_LOCKS: LEAK — one lock per unique token queried, NEVER cleaned" | **BOUNDED**: `release_lock_if_idle(mint)` removes lock after EVERY use (decimals.rs:445-448). Safety cap: `map.clear()` at 10K entries (decimals.rs:431-437). | Removed from Phase B moka migration (B16). No fix needed. |
| 2 | "COMPUTATION_FAILURES: LEAK — grows per failed computation" | **BOUNDED**: Keys are `&'static str` (fixed set of ~5-10 window names like "1h", "24h", "7d"). `circuit_reset(key)` removes on success (cache.rs:228-231). Max ~10 entries. | Removed from Phase B moka migration (B15). No fix needed. |
| 3 | "TOKEN_POOLS_CACHE: LEAK — grows per token pool lookup" | **SLOW LEAK**: Has TTL-based freshness check (`is_pool_entry_fresh`) + manual `clear_cache()`. But NO periodic eviction — stale entries stay in HashMap indefinitely. | Downgraded from "LEAK" to "SLOW LEAK". Still needs moka migration (B13). |
| 4 | "POOL_PREFETCH_STATE: LEAK — grows per prefetched token" | **SLOW LEAK**: Cleared only via `clear_cache()` alongside TOKEN_POOLS_CACHE. No independent cleanup. | Downgraded from "LEAK" to "SLOW LEAK". Still needs moka migration (B14). |
| 5 | "5+ additional leak caches" (Gap 9) | **3 true leaks** (POSITION_LOCKS, ACTIVE_ACTIONS, TOKEN_2022_CACHE) + **2 slow leaks** (TOKEN_POOLS_CACHE, POOL_PREFETCH_STATE). Two caches verified BOUNDED. | Gap 9 severity downgraded HIGH→MEDIUM. Phase B reduced by 2 tasks. |
| 6 | Token count: various references to "171K" in filtering | **171K CONFIRMED correct**: SQL `WHERE (d.mint IS NOT NULL OR g.mint IS NOT NULL)` filters 275K total → 171K with market data. Code comment says "reduces 144k -> ~56k" but that's from an older dataset. Our actual DB confirms 171K with market data. | No change needed — plan's 171K figure is correct. |
| 7 | "19 services" (various locations) | **19 CONFIRMED**: 19 `impl Service for` in implementations/ directory. Prior verification agent incorrectly claimed 20. | No change needed — plan's "19" is correct. |

**Net impact**: Phase B reduced from 17 tasks to 15 tasks. Two caches (FETCH_LOCKS, COMPUTATION_FAILURES) 
confirmed safe — no work needed. Overall leak count reduced from "8+ memory leaks" to "3 true leaks + 
2 slow leaks + several unbounded (but not leaking) caches".

---

## 🔄 Phase Review and Ordering Optimization (v8)

> Deep review of all 5 phases: ordering, dependencies, risks, missing steps, and improvements.
> Goal: minimize risk, maximize early impact, ensure correct dependency chains.

### Phase-Level Ordering Analysis

Current order: **A → B → C → D → E**

| Phase | Risk | Impact | Dependencies |
|-------|------|--------|-------------|
| A (SQLite foundation) | LOW | ~400-500 MB | None |
| B (Bounded caches) | LOW-MEDIUM | ~50-100 MB | A8 (profile system for moka capacities) |
| C (FilterToken + Incremental) | MEDIUM | ~178-228 MB | None (independent of B!) |
| D (Maintenance service) | MEDIUM | Disk stability | A9 (maintenance config) |
| E (Observability) | LOW | User-facing | A-D all done |

**Key insight: B and C are INDEPENDENT**. B needs A8 (profile config). C needs nothing from B. 
They could theoretically be done in parallel or either-first. Current order (B before C) is 
correct from a risk perspective: B is lower risk, so do it first to build confidence. If B 
goes well, C is safe to proceed. If B has problems, we fix them before touching the more 
complex C work.

**A → B → C → D → E ordering is CORRECT.** But steps WITHIN each phase need reordering.

---

### Phase A — Detailed Step Review

**Current (9 steps):**
```
A1. Create configure_sqlite_connection() shared function
A2. Fix mmap_size 30GB → 256MB
A3. Right-size cache_size per database
A4. Right-size pool max_size
A5. Migrate ALL pools to with_init()
A6. jemalloc with feature flag
A7. auto_vacuum=INCREMENTAL + one-time migration VACUUM
A8. Add [performance] config section
A9. Add [maintenance] config section
```

**Problems found:**

1. **A1, A2, A3 are inseparable**: You can't create the shared function (A1) without defining 
   its values (A2 mmap, A3 cache_size). These are ONE task: "create the function with correct values."

2. **A5 depends on A1 but is listed after A2-A4**: A5 is WHERE the function gets wired in. 
   Natural flow: create function → wire it into all pools. A4 (pool sizing) is a SEPARATE change 
   to the r2d2 builder, not the PRAGMA init function.

3. **A7 (one-time VACUUM) is the RISKIEST step in Phase A**: Full VACUUM on 294MB tokens.db takes 
   30-60 seconds, locks the database. If interrupted, could corrupt. This should be LAST in Phase A, 
   or even deferred to Phase D when the maintenance service exists.

4. **A6 (jemalloc) is 3 lines of code**: It's completely independent and gives instant benefit. 
   Should be done early.

5. **A8 and A9 are config-only work**: No behavioral changes, just struct definitions. Low risk, 
   but needed by Phase B (moka reads profile values) and Phase D (maintenance reads config). 
   Fine in current position.

6. **MISSING: Wire up existing cleanup functions (Gap 1)**: `cleanup_old_actions()` and 
   `cleanup_stats()` already exist but are NEVER CALLED. This is a one-line fix per function 
   (add a timer call in the existing service loop). The rpc_stats.db has 602K rows growing daily. 
   Every day we delay costs ~25K more rows. This trivial fix should be in Phase A, not Phase D.

**REVISED Phase A ordering (10 steps):**

```
Phase A — Foundation (LOW RISK)

  A1. Create src/database/common.rs with DbConnectionConfig + configure_sqlite_connection()
      - Define Hot/Standard/Cold presets with right-sized cache_size AND correct mmap_size
      - This is ONE task: function + values are inseparable
      - Include: cache_size (500-5000 pages), mmap_size (0-256MB), WAL, synchronous, busy_timeout

  A2. Migrate ALL 13 SQLite pools to use with_init() calling configure_sqlite_connection()
      - For each database: remove ad-hoc PRAGMA blocks, replace with with_init() + appropriate preset
      - Pick preset per DB: tokens=Hot, events=Standard, strategies=Cold, etc.
      - Test: bot starts, all DBs accessible, PRAGMAs applied correctly
      - This is the most mechanical but important task — touch 13 files

  A3. Right-size connection pool max_size (reduce oversized pools)
      - events.db read: 10→4, actions.db read: 10→4, transactions.db: 10→5
      - tools/strategies/wallets: 10→3
      - Separate from A2 because it changes r2d2 builder config, not PRAGMA init

  A4. jemalloc allocator with feature flag
      - 3 lines of code in Cargo.toml + main.rs. Instant ~100-200MB improvement.
      - Feature flag: default=["jemalloc"], disable with --no-default-features
      - Test: build on macOS + verify it links. Cross-compile check for Linux.

  A5. Wire up cleanup_old_actions() — add timer call in actions service loop
      - Function exists at actions/database.rs:885 — NEVER CALLED
      - One-line fix: add periodic call (every 24h) with configurable retention
      - Prevents actions.db from growing forever

  A6. Wire up cleanup_stats() — add timer call in rpc_stats service loop
      - Function exists at rpc/manager.rs:612 — NEVER CALLED
      - One-line fix: add periodic call (every 24h) with configurable retention
      - Prevents rpc_stats.db from growing forever (currently 602K rows!)

  A7. Add [performance] config section with PerformanceConfig
      - memory_profile: "auto"/"low"/"medium"/"high"
      - Auto-detection via sysinfo (existing dep)
      - resolve_profile() function: auto-detect → profile → individual overrides
      - Individual override fields (all default to 0 = use profile)
      - Replace empty MonitoringConfig placeholder

  A8. Add [maintenance] config section with MaintenanceConfig
      - Retention periods for events/actions/rpc_stats/sol_flow/ohlcv
      - WAL checkpoint + VACUUM intervals
      - Maintenance window + trading safety toggle
      - A5/A6 use hardcoded 24h for now; Phase D reads from this config

  A9. Add auto_vacuum=INCREMENTAL pragma to configure_sqlite_connection()
      - Just add the PRAGMA to the shared function — does NOT trigger VACUUM
      - auto_vacuum=INCREMENTAL only activates for NEW pages written after this point
      - Old bloat remains until Phase D does a full VACUUM (intentional — safer)
      - New databases created fresh get auto_vacuum from the start

  A10. VERIFY: Build bot, run it, measure RSS at startup and after 30 min
      - Compare before/after numbers
      - Verify all database PRAGMAs applied correctly (query with PRAGMA cache_size etc.)
      - Check logs for errors
      - Verify trading still works (manual test buy/sell if possible)

  EXPECTED IMPACT: ~400-500 MB RSS reduction + jemalloc (~100-200MB fragmentation reduction).
  ROLLBACK: Git revert commit. Old cache_size/pool_size values are in git history.
```

**Changes from v7:**
- A1+A2+A3 → A1 (create function with values = one task)
- Old A5 (with_init migration) → A2 (immediately after creating the function)
- Old A4 (pool sizing) → A3 (separate from PRAGMA work)
- Old A6 (jemalloc) → A4 (do early, 3 lines)
- **NEW A5+A6**: Wire up existing cleanup functions (moved from Phase D — trivially easy, immediate benefit)
- Old A8+A9 → A7+A8 (config work unchanged)
- **A7 split**: auto_vacuum PRAGMA goes into A9 (just the PRAGMA, no VACUUM). 
  One-time migration VACUUM moved to Phase D (safer when maintenance service exists).
- **NEW A10**: Explicit verification step

---

### Phase B — Detailed Step Review

**Current (15 active steps, 2 removed):**
```
B1-B14: Various moka migrations + code fixes
B15-B16: REMOVED (v7)
B17: ACTIVE_ACTIONS cleanup
```

**Problems found:**

1. **15 steps is too many for one phase**: Each moka migration is small but each needs testing. 
   This should be split into sub-phases for better progress tracking and earlier verification.

2. **Non-moka tasks mixed with moka migrations**: B8 (POSITION_LOCKS cleanup), B10 (PRICE_HISTORY 
   cleanup), B17 (ACTIVE_ACTIONS cleanup) are CODE FIXES, not moka migrations. They have different 
   risk profiles and should be grouped separately.

3. **B2 (DECIMALS_CACHE) is the highest-impact AND highest-risk migration**: 242K entries getting 
   capped to 150K. If the cap is too low, cache miss rate increases and decimals lookups slow down. 
   Needs its own verification checkpoint.

4. **B9 (TimedCache → moka) has wider blast radius**: Replacing the custom TimedCache affects 3 API 
   caches (DexScreener, GeckoTerminal, Rugcheck). These are on the critical path for token data 
   updates. Needs careful testing.

5. **B11 (TokenStore) needs design clarification**: TokenStore is a per-token detail lookup cache 
   (used by `get_token(mint)` for individual lookups). Token LIST is served from FilteringStore, 
   NOT TokenStore. So capping TokenStore to 10K is SAFE — it only affects individual detail 
   lookups, not the dashboard token list. Verified in code: list.rs uses `filtering::query_tokens()`.

**REVISED Phase B ordering (split into B-early + B-late):**

```
Phase B-early — High-Impact Cache Fixes (LOW RISK, 7 steps)

  B1. Add moka crate dependency (features=["sync"])

  B2. Migrate DECIMALS_CACHE to moka
      - Max from resolved profile: 50K (low) / 150K (medium) / 300K (high)
      - No TTL (decimal values don't change)
      - VERIFY: After startup, check entry_count(). If near max AND hit rate <90%, cap is too low.
      - This is the biggest cache (242K entries, ~20MB)

  B3. Migrate TOKEN_2022_CACHE to moka
      - Max from resolved profile: 10K / 50K / 100K
      - No TTL
      - TRUE LEAK fix — currently has no removal mechanism

  B4. Migrate GLOBAL_KNOWN_SIGNATURES to moka
      - Max from resolved profile: 10K / 50K / 100K
      - No TTL (signatures are immutable)
      - Prevents unbounded growth from transaction monitoring

  B5. Fix POSITION_LOCKS: remove lock when position closes
      - Code fix in positions/operations.rs (not a moka migration)
      - When close_position() called → also remove from POSITION_LOCKS HashMap
      - TRUE LEAK fix

  B6. Add periodic remove for ACTIVE_ACTIONS completed/failed entries
      - Code fix: after action reaches terminal state (success/failed/cancelled), remove from HashMap
      - OR: add periodic cleanup every 5 min that removes terminal actions older than 1 hour
      - TRUE LEAK fix

  B7. VERIFY: Run bot for 1h. Check DECIMALS hit rate. Check POSITION_LOCKS size. Check ACTIVE_ACTIONS.

Phase B-late — Remaining Caches (LOW-MEDIUM RISK, 8 steps)

  B8. Migrate FAILED_CACHE to moka (10K max, 5 min TTL)
  B9. Migrate GLOBAL_PENDING_TRANSACTIONS to moka (max from profile, TTL: 180s)
  B10. Migrate LAST_TOKEN_ACCOUNTS_CHECK to moka (5K max, 1h TTL)
  B11. Replace custom TimedCache with moka (DEXSCREENER_CACHE, GECKOTERMINAL_CACHE, RUGCHECK_CACHE)
       - Keep same max entries and TTL as current TimedCache config
       - Removes ~200 lines of custom TimedCache implementation
  B12. Add PRICE_HISTORY cleanup on position close
  B13. Migrate TokenStore HashMap to moka (max from profile: 3K/10K/25K)
       - Safe: TokenStore is per-token detail cache. Token list uses FilteringStore (DB-backed).
  B14. Migrate AI cache DashMap to moka (5K max, TTL from config)
  B15. Migrate TOKEN_POOLS_CACHE to moka (5K max, TTL from pool_cache_ttl config) [slow leak fix]
       - Also migrates POOL_PREFETCH_STATE (cleared alongside, can share lifecycle)

  VERIFY: Run bot for 24h. No cache exceeds max. All leak sources capped.
```

**Changes from v7:**
- Split into B-early (highest impact, true leaks) and B-late (remaining caches)
- B5 (POSITION_LOCKS) and B6 (ACTIVE_ACTIONS) moved to B-early (true leak fixes are urgent)
- B13+B14 merged conceptually (TOKEN_POOLS_CACHE + POOL_PREFETCH_STATE share lifecycle)
- Added explicit VERIFY steps
- Each sub-phase can be committed and tested independently

---

### Phase C — Detailed Step Review

**Current (9 steps):**
```
C1-C4: FilterToken struct + migration
C5-C6: Incremental delta updates
C7-C8: Config integration
C9: SQL pre-filtering
```

**Problems found:**

1. **C should be split into 3 sub-phases**: FilterToken, Incremental, and SQL pre-filter are 
   INDEPENDENT features that multiply together. Each can be developed and tested alone.

2. **C9 (SQL pre-filtering) is an optimization-of-an-optimization**: With FilterToken alone 
   (171K × 350 bytes = 60MB), memory is already reasonable. SQL pre-filtering reduces to ~14MB 
   which is nice but not critical. This should be marked OPTIONAL/DEFER.

3. **C5-C6 (incremental) is the MOST COMPLEX part of the entire plan**: The merge logic (update + 
   insert + remove) with the immutable snapshot pattern has many edge cases:
   - Token updated but already removed from snapshot (race condition)
   - Token blacklisted between refreshes (need explicit removal query)
   - update_tracking timestamp reset or missing (need graceful fallback)
   - Database schema migration during incremental refresh (extremely rare but possible)
   
   This needs its own detailed design document and test plan BEFORE implementation.

4. **C4 (AI filter → FilterToken)** has a subtle dependency: it needs `get_full_token()` to work 
   as a single-row lookup. This function likely already exists but needs verification that it's 
   fast enough for the AI path (2.8K-5.6K tokens × ~2ms each = 5-11 seconds total AI pre-lookup).

5. **MISSING step: C0 — Audit ALL filter source field access**: Before defining FilterToken, we 
   need a complete audit of every field access in dexscreener.rs, geckoterminal.rs, rugcheck.rs, 
   meta.rs, and ai.rs filter sources. The plan estimates ~35 fields but this wasn't mechanically 
   verified against code.

**REVISED Phase C ordering (3 sub-phases):**

⚠️ **NOTE**: The sections below (Phase C1, C2, C3) describe the ORIGINAL PLAN for TokenListEntry + incremental filtering. 
These were NOT IMPLEMENTED. See lines 1368-1410 for what was actually done in Phase C.

```
Phase C1 — FilterToken ⏸️ DEFERRED INDEFINITELY (MEDIUM RISK, 5 steps)

  ⚠️ **NOT IMPLEMENTED — bypassed by stale SQL filter (C5 actual)**
  
  The stale token SQL filter achieved BETTER results than FilterToken would have:
  - FilterToken would save ~23 MB (15.6K tokens × 450 bytes vs 2,200 bytes)
  - Stale filter saved ~122 MB by reducing tokens from 172K to 15.6K (91% reduction)
  - With only 15.6K tokens loaded, the ROI of TokenListEntry is now minimal
  
  ORIGINAL PLAN (not implemented):

  C1. Audit all filter source field access ⏸️ DEFERRED
      - Mechanically grep each filter source for token.field_name accesses
      - Produce definitive field list (not estimated ~35, exact count)
      - Document which source uses which field

  C2. Define FilterToken struct with audited fields ⏸️ DEFERRED
      - Comment block at top: field → source mapping
      - sizeof check: verify ~350 bytes or actual size
      - Implemented as TokenListEntry per v10 specification

  C3. Create optimized SQL query for FilterToken loading ⏸️ DEFERRED
      - SELECT only FilterToken columns (skip security_risks JSON, top_holders, websites, socials)
      - Simpler JOINs (skip tables not needed for FilterToken fields)
      - Benchmark: time for full load before/after

  C4. Update all filter sources (dexscreener, geckoterminal, rugcheck, meta) to use FilterToken ⏸️ DEFERRED
      - Compile-time safety: if a source references a missing field, build fails

  C5. Update AI filter source: FilterToken pre-check, get_full_token() for LLM analysis ⏸️ DEFERRED
      - AI runs LAST (after all other filters). Typically 2.8K-5.6K tokens reach AI.
      - For each: single-row indexed DB lookup (~2ms) to get full Token for LLM prompt
      - Acceptable: AI is rate-limited anyway (50 req/min)

  VERIFY: ⏸️ NOT VERIFIED — Phase not implemented.
  ROLLBACK: N/A — nothing to roll back.

Phase C2 — Incremental Filtering ⏸️ DEFERRED INDEFINITELY (MEDIUM-HIGH RISK, 5 steps)

  ⚠️ **NOT IMPLEMENTED — bypassed by stale SQL filter (C5 actual)**
  
  The stale token SQL filter eliminated the need for incremental filtering:
  - With 91% of tokens filtered out (172K → 15.6K), full refresh is now lightweight
  - Full refresh of 15.6K tokens every 3 min has negligible performance impact
  - Incremental filtering would add significant complexity for minimal gain
  
  ORIGINAL PLAN (not implemented):

  C6. Implement incremental delta query (WHERE updated_at > last_refresh) ⏸️ DEFERRED
      - Query tokens where update_tracking.market_data_last_updated_at > last_refresh_timestamp
      - Also query new tokens (first_discovered_at > last_refresh)
      - Also query newly blacklisted (blacklisted_at > last_refresh)

  C7. Implement snapshot merge logic (update/insert/remove) ⏸️ DEFERRED
      - Clone existing snapshot's HashMap (Arc pointers only, ~8MB)
      - Apply: updated tokens re-evaluated, new tokens added if pass, blacklisted removed
      - Build new FilteringSnapshot from modified HashMap
      - Swap atomically via existing Arc+RwLock pattern

  C8. Add safety nets: ⏸️ DEFERRED
      - Full refresh every 30 min (catches drift)
      - If delta > 50% of total → auto full refresh
      - Config change → immediate full refresh
      - Fallback: if incremental fails, do full refresh + log warning

  C9. Filtering refresh interval from resolved profile (300s/180s/120s) ⏸️ DEFERRED

  C10. Add config change → full refresh trigger ⏸️ DEFERRED

  VERIFY: ⏸️ NOT VERIFIED — Phase not implemented.
  ROLLBACK: N/A — nothing to roll back.

Phase C3 — SQL Pre-Filtering (LOW PRIORITY — OPTIONAL/DEFER)

  C11. Add SQL WHERE clause for simple numeric thresholds (volume, liquidity, mcap, fdv, rugcheck score)
       - Only push SAFE filters: strict numeric minimums that are always AND conditions
       - Generate WHERE clause from config values at query time
       - Requires: appropriate indexes on filtered columns

  EXPECTED ADDITIONAL IMPACT: 60MB → ~14MB (if typical filters eliminate 70%+ of tokens)
  DEFER IF: 60MB is acceptable for target users. This adds complexity for diminishing returns.
```

**Changes from v7:**
- Split into 3 clearly independent sub-phases (C1, C2, C3)
- Added C1 step: audit filter field access (was missing)
- Added C8: explicit safety nets step (was implicit)
- C9 (SQL pre-filter) → C3 sub-phase marked OPTIONAL/DEFER
- Each sub-phase has independent rollback — critical for risk management

---

### Phase D — Detailed Step Review

**Current (11 steps):**
```
D1-D2: Create service + WAL checkpoint
D3-D4: Wire up existing cleanups (moved to Phase A in v8!)
D5: sol_flow cleanup
D6: Batched VACUUM
D7: Stale token marking
D8-D9: Maintenance window + trading safety
D10-D11: Session cleanups
```

**Problems found:**

1. **D3-D4 moved to Phase A (v8)**: cleanup_old_actions and cleanup_stats are trivial one-liners 
   that shouldn't wait for a full Maintenance Service. Already moved.

2. **D6 (batched VACUUM) now includes the one-time migration VACUUM**: Moved from Phase A because:
   - One-time VACUUM is slow (30-60s per large DB) and risky (DB lock)
   - Better to have the maintenance service infrastructure first
   - The maintenance service can use trading-aware checks
   - By Phase D, Phases A/B/C already reduced memory significantly

3. **D7 (stale token marking) needs more design**: What column? How does it interact with filtering? 
   What if a "stale" token gets new market data — does it auto-reactivate?
   
   Design: Add `is_active` column to tokens table (default true). Mark false when no 
   update_tracking changes for 30 days. Any new market data update sets it back to true.
   FilterToken SQL adds `WHERE is_active = true` (or omit for now — stale tokens with no market 
   data are already filtered out by the DexScreener/GeckoTerminal data requirement).

4. **D8 (maintenance window) is over-scoped**: Crypto markets are 24/7. "Maintenance windows" 
   are a datacenter concept. For v1, just have `maintenance_enabled = true/false` and 
   `skip_during_active_trades = true`. Add window support in a future version.

5. **D10-D11 (session cleanups) are trivially easy**: Add periodic call to existing 
   cleanup_expired_sessions(). Should be one step, not two.

**REVISED Phase D ordering (9 steps):**

```
Phase D — Maintenance Service (MEDIUM RISK)

  D1. Create MaintenanceService implementing Service trait
      - Priority: 90 (after core services, before webserver)
      - Dependencies: ["connectivity"] (minimal — maintenance can run independently)
      - Main loop: tick every 60 seconds, check which tasks are due

  D2. Implement periodic WAL checkpoint
      - Default: every 1 hour (from [maintenance] config)
      - Use PRAGMA wal_checkpoint(PASSIVE) by default (non-blocking)
      - Switch to TRUNCATE only when WAL file > 100MB
      - Skip during active trades

  D3. Add sol_flow_cache cleanup function + timer (retention from config)
      - Currently NO cleanup exists [Gap 2]
      - Implement: delete entries older than retention period

  D4. Add periodic session cleanup
      - Call cleanup_expired_sessions() every 15 min
      - Also clean IMPORT_SESSIONS and MULTI_WALLET_SESSIONS

  D5. One-time migration: activate auto_vacuum=INCREMENTAL on existing databases
      - On first startup after update: check PRAGMA auto_vacuum for each DB
      - If not INCREMENTAL: set it + run full VACUUM (required to activate)
      - Log progress: "Optimizing {db_name}... this is one-time and may take 30-60 seconds"
      - Run BEFORE ServiceManager.start_all() (during init phase)
      - Skip if bot was interrupted mid-VACUUM (detect via sentinel file)
      - CRITICAL: This is the riskiest step in Phase D

  D6. Implement batched incremental VACUUM (one DB per cycle, trading-aware)
      - Every 6 hours (from config): PRAGMA incremental_vacuum(1000) on one database
      - Rotate through all databases (round-robin)
      - Skip during active trades

  D7. Implement stale token marking (inactive after configurable days)
      - Mark tokens with no update_tracking changes for N days as inactive
      - Any new market data update reactivates the token
      - NOTE: Currently stale tokens without market data are already excluded from filtering
        (WHERE d.mint IS NOT NULL OR g.mint IS NOT NULL). This step adds explicit tracking.
      - DEFER if filtering already handles this adequately

  D8. Add maintenance status tracking (for observability in Phase E)
      - Track: last run time, next scheduled time, last vacuum DB, total reclaimed bytes
      - Store in memory (not DB — maintenance metadata doesn't need persistence)

  D9. Add trading-aware checks throughout (skip_during_active_trades from config)
      - Before any blocking operation: check for in-flight trades
      - If trades active: skip this cycle, retry next interval
      - Log: "Maintenance deferred — trades in progress"

  VERIFY: Run for 1 week. Check DB file sizes stay stable or decrease.
  Check WAL files don't grow unbounded. Check no interference with trading.
  ROLLBACK: Set maintenance.enabled = false in config. No data loss risk.
```

**Changes from v7:**
- D3/D4 (cleanup_old_actions, cleanup_stats) moved to Phase A
- Old D8 (maintenance window) simplified → just skip_during_active_trades
- One-time migration VACUUM (from old A7) now D5 (done with maintenance infrastructure)
- Added D8 (maintenance status tracking) for Phase E observability
- Reduced from 11 → 9 steps

---

### Phase E — Detailed Step Review

**Current (10 steps) — mostly good as-is. Minor improvements:**

**Problems found:**

1. **E10 (CLI commands) is significant new work**: The screenerbot-manager binary needs new 
   subcommands. This is a nice-to-have, not essential for memory optimization. Mark as OPTIONAL.

2. **E8 (dashboard poll interval)** is trivial — just read config in JS. Could be done alongside 
   Phase A config work if desired.

3. **E6 (dashboard Performance panel)** is the biggest UI task. Needs its own design but the plan 
   already has a detailed mockup. Good.

4. **E1-E2 (pressure detection)** could have been done earlier (Phase B) to protect during 
   migrations. But doing it in Phase E means less code to maintain during active development. 
   Trade-off is acceptable — by Phase E, memory usage is already dramatically reduced.

**REVISED Phase E ordering (10 steps, E10 marked optional):**

```
Phase E — Observability + Pressure Response (LOW-MEDIUM RISK)

  E1. Implement memory pressure detection
      - Check process RSS every 30 seconds via sysinfo (existing dep)
      - Calculate usage_percent = rss / memory_budget
      - memory_budget from resolved profile or config override

  E2. Add pressure levels with hysteresis
      - Level 0 (Normal): RSS < 70% budget → full speed
      - Level 1 (Elevated): RSS 70-90% → reduce filtering interval, evict disposable caches
      - Level 2 (Critical): RSS > 90% → force evict all non-essential caches, urgent alerts
      - Recovery: 5 consecutive normal readings (2.5 min) → restore normal operation

  E3. Add /api/system/memory endpoint
      - RSS, budget, pressure_level, breakdown (sqlite/filtering/caches/app), cache stats, DB sizes

  E4. Add /api/system/performance endpoint
      - Active profile, resolved values for all parameters, system RAM

  E5. Add /api/system/maintenance endpoint
      - Last run, next scheduled, last vacuum DB, total reclaimed

  E6. Add dashboard Performance panel
      - Memory breakdown bar, pressure indicator, cache hit rates, DB sizes
      - Profile selector (auto/low/medium/high)
      - Uses E3/E4/E5 endpoints

  E7. Add Telegram notifications for pressure events
      - Level 1: one-time warning
      - Level 2: urgent notification
      - Recovery: "Memory recovered" notification

  E8. Dashboard poll interval from resolved profile (15s/10s/5s)
      - JS reads interval from /api/config response
      - Trivial change but improves UX

  E9. Add cache efficiency logging (hit rate recommendations)
      - Track hit rates per moka cache over 1-hour windows
      - Log recommendations: "DECIMALS_CACHE at capacity with 97% hit rate" etc.

  E10. (OPTIONAL) Add screenerbot-manager CLI commands
       - memory, performance, maintenance, caches subcommands
       - Call same API endpoints as dashboard
       - Useful for VPS users without GUI
       - DEFER if time-constrained — dashboard covers same functionality

  VERIFY: Change profiles, verify parameters change. Simulate pressure, verify response.
  ROLLBACK: Pressure response disabled via config. Dashboard panel is additive.
```

---

### Cross-Phase Dependency Map (Revised)

```
A1 (shared function) ──→ A2 (migrate pools)
A7 (performance config) ──→ B2-B14 (moka caches read profile values)
A8 (maintenance config) ──→ D1-D9 (maintenance service reads config)
A9 (auto_vacuum PRAGMA) ──→ D5 (one-time VACUUM to activate)
                           ──→ D6 (incremental VACUUM uses it)

B1 (add moka) ──→ B2-B15 (all moka migrations)

C1-C5 (FilterToken) ──→ independent (no Phase B dependency!)
C6-C10 (Incremental) ──→ independent (no Phase B dependency!)
C6-C10 ──→ benefits from C1-C5 (FilterToken reduces delta token size)

D1 (maintenance service) ──→ D2-D9 (all maintenance tasks)
D5 (one-time VACUUM) ──→ depends on A9 (auto_vacuum PRAGMA exists)

E1-E2 (pressure detection) ──→ E3-E10 (all observability)
E3-E5 (API endpoints) ──→ E6 (dashboard panel reads them)
E6 (dashboard) ──→ needs all of A-D completed for meaningful data
```

### Missing Cross-Cutting Concerns

**1. Verification Protocol (run after EACH phase)**
```
After every phase commit:
  1. cargo build --release (must succeed)
  2. Run bot for 5 minutes minimum
  3. Measure RSS at startup, after 1 min, after 5 min
  4. Check logs for errors/warnings
  5. Verify dashboard loads correctly
  6. If bot trades, verify trading still works
  7. Record measurements in a tracking table (for comparison across phases)
```

**2. Database Backup Recommendation**
```
Before Phase A (first implementation):
  - Copy data/ directory to data_backup/
  - User communication: "First run after update may take 30-60 seconds for database optimization"
Before Phase D5 (one-time VACUUM):
  - Automatic backup: copy {db}.db to {db}.db.bak before VACUUM
  - Delete backup after successful VACUUM
```

**3. Feature Flags for Gradual Rollout**
```
Consider config flags that let users opt-in/opt-out of major changes:
  [performance]
  use_incremental_filtering = true   # Phase C2 — can be disabled if issues found
  maintenance_enabled = true         # Phase D — can be disabled
  pressure_response_enabled = true   # Phase E — can be disabled
```

### Revised Phase Task Count Summary

| Phase | v7 Tasks | v8 Tasks | Change | Risk |
|-------|----------|----------|--------|------|
| A | 9 | 10 (includes verify) | +1 (added cleanup wiring from D, added verify step) | LOW |
| B-early | — | 7 (includes verify) | Split from B | LOW |
| B-late | — | 8 | Split from B | LOW-MEDIUM |
| B total | 15 | 15 | Same count, better grouping | |
| C1 (FilterToken) | — | 5 | Split from C | MEDIUM |
| C2 (Incremental) | — | 5 | Split from C | MEDIUM-HIGH |
| C3 (SQL prefilter) | — | 1 (OPTIONAL) | Split from C, marked optional | LOW |
| C total | 9 | 11 | +2 (added audit step + safety nets step) | |
| D | 11 | 9 | -2 (cleanups moved to A) | MEDIUM |
| E | 10 | 10 (E10 optional) | Same, E10 marked optional | LOW-MEDIUM |
| **TOTAL** | **54** | **55** | Net +1 (better decomposed) | |

### Risk-Ordered Implementation Priority

If we need to STOP at any point, here's what delivers the most value per phase:

```
Phase A alone:          ~500-700 MB reduction (SQLite + jemalloc)     ← HUGE win, LOW risk
Phase A + B-early:      + leak fixes, biggest caches bounded          ← essential correctness
Phase A + B-early + C1: + 178 MB from FilterToken                    ← significant memory win
Phase A + B:            + all caches bounded                          ← completeness
Phase A + B + C:        + incremental filtering                       ← eliminates peak spikes
Phase A + B + C + D:    + self-maintaining databases                  ← long-term stability
Phase A + B + C + D + E: + observability + OOM prevention             ← production-grade
```

**The first 3 phases (A + B-early + C1) deliver ~90% of the total memory benefit.**
Everything after that is about long-term stability, observability, and prevention.

---

## 🗺️ Complete File Change Flowchart (v8)

> Every file that will be CREATED, MODIFIED, REMOVED, or REVIEWED across all 5 phases.
> All paths relative to `ScreenerBot/` (the public repo root).
> Line counts verified from actual source. Actions: ✨=CREATE, ✏️=MODIFY, 🗑️=REMOVE, 🔍=REVIEW-ONLY

---

### 📦 Cargo.toml & Root Files

| File | Lines | Phase | Action | What Changes |
|------|-------|-------|--------|-------------|
| `Cargo.toml` | 226 | A | ✏️ MODIFY | Add `tikv-jemallocator = { version = "0.6", optional = true }`, `moka = { version = "0.12", features = ["sync"] }`. Add `[features] jemalloc = ["tikv-jemallocator"]` |
| `src/main.rs` | 217 | A | ✏️ MODIFY | Add 3-line `#[global_allocator]` jemalloc block with `#[cfg(feature="jemalloc")]` guard |

---

### 📁 NEW Files to CREATE

| File | Phase | Purpose | Est. Lines |
|------|-------|---------|-----------|
| ✨ `src/database/mod.rs` | A | Module declaration: `pub mod common;` | ~2 |
| ✨ `src/database/common.rs` | A | `DbConnectionConfig` struct + `configure_sqlite_connection()` shared function + `Hot/Standard/Cold` presets | ~80 |
| ✨ `src/config/schemas/performance.rs` | A | `PerformanceConfig`: memory_profile, sqlite_cache_multiplier, max_* cache entries, filtering_refresh_interval_secs | ~120 |
| ✨ `src/config/schemas/maintenance.rs` | A | `MaintenanceConfig`: enabled, retention periods (events/actions/rpc_stats/sol_flow/ohlcv), wal_checkpoint_interval, vacuum_interval, skip_during_active_trades | ~80 |
| ✨ `src/filtering/filter_token.rs` | C1 | `FilterToken` struct (~35 fields, ~350 bytes) + optimized SQL query builder | ~200 |
| ✨ `src/filtering/incremental.rs` | C2 | Delta query (WHERE updated_at > last_refresh), snapshot merge (update/insert/remove), safety nets | ~300 |
| ✨ `src/maintenance/mod.rs` | D | `MaintenanceService` implementing `Service` trait + task scheduler loop | ~150 |
| ✨ `src/maintenance/tasks.rs` | D | Individual task implementations: WAL checkpoint, incremental VACUUM, sol_flow cleanup, session cleanup, stale token marking | ~250 |
| ✨ `src/services/implementations/maintenance_service.rs` | D | Service wrapper: name, priority(90), dependencies, start/stop | ~50 |
| ✨ `tests/memory_budget.rs` | B+E | Integration test: verify profile cache totals stay within budgets | ~60 |
| ✨ `tests/config_backward_compat.rs` | A | Integration test: old config.toml files still deserialize without [performance]/[maintenance] | ~40 |
| ✨ `tests/filtering_pipeline.rs` | C | Integration test: full filtering with in-memory DB + FilterToken, incremental matches full refresh | ~100 |

**Total NEW files: 12 | Total NEW lines: ~1,432**

---

### 📁 PHASE A — Foundation (14 files MODIFIED)

#### A1: Shared SQLite Function (2 files CREATED + 1 MODIFIED)

| File | Lines | Action | What Changes |
|------|-------|--------|-------------|
| ✨ `src/database/mod.rs` | NEW | ✨ CREATE | Module declaration |
| ✨ `src/database/common.rs` | NEW | ✨ CREATE | DbConnectionConfig + configure_sqlite_connection() |
| `src/lib.rs` or top-level `mod` | — | ✏️ MODIFY | Add `pub mod database;` declaration |

#### A2: Migrate ALL 13 SQLite pools to with_init() (13 files MODIFIED)

These files currently have ad-hoc PRAGMA blocks. Each gets: remove PRAGMAs → add `with_init(|c| configure_sqlite_connection(c, &preset))`.

| File | Lines | Current Pattern | Change |
|------|-------|----------------|--------|
| `src/tokens/schema.rs` | 362 | PRAGMAs in `initialize_schema()` (Pattern #2) | Add `with_init()` with `Hot` preset. Remove inline PRAGMAs. |
| `src/events/database.rs` | 1038 | PRAGMAs on every `get_connection()` checkout (Pattern #3) | Switch to `with_init()` with `Standard` preset. Remove per-checkout PRAGMAs. |
| `src/actions/database.rs` | 977 | PRAGMAs in init (Pattern #2+#3 mixed) | Add `with_init()` with `Standard` preset. Remove duplicates. |
| `src/positions/database/operations.rs` | 1770 | PRAGMAs on checkout (Pattern #3) | Switch to `with_init()` with `Standard` preset. |
| `src/transactions/database/operations.rs` | 925 | ✅ Already uses `with_init()` (Pattern #1) | Update to use shared `configure_sqlite_connection()` preset. |
| `src/strategies/database.rs` | 710 | PRAGMAs in `initialize()` (Pattern #2) | Add `with_init()` with `Cold` preset. |
| `src/wallets/database.rs` | 762 | PRAGMAs in init (Pattern #2) | Add `with_init()` with `Cold` preset. |
| `src/wallets/balance_monitor/database.rs` | 1137 | PRAGMAs in init (Pattern #2) + **30GB mmap!** | Add `with_init()` with `Standard` preset. **Fix mmap 30GB→32MB**. |
| `src/tools/database/schema.rs` | 457 | PRAGMAs in init (Pattern #2) | Add `with_init()` with `Cold` preset. |
| `src/rpc/stats/database.rs` | 579 | PRAGMAs in init (Pattern #2) | Add `with_init()` with `Cold` preset. |
| `src/ai/chat_db.rs` | 795 | ✅ Already uses `with_init()` (Pattern #1) | Update to use shared preset. |
| `src/ai/database.rs` | 716 | PRAGMAs in init (Pattern #2) | Add `with_init()` with `Cold` preset. |
| `src/ohlcvs/database.rs` | 1237 | PRAGMAs in init (Pattern #2) | Add `with_init()` with `Standard` preset. |

**Bonus PRAGMA fix in A2**: `src/tokens/schema.rs` line ~40: `mmap_size=30,000,000,000` → `256MB`

#### A3: Right-size connection pool max_size (same 13 files as A2)

| File | Pool | Current max | New max |
|------|------|-------------|---------|
| `src/events/database.rs` | Read pool | 10 | 4 |
| `src/actions/database.rs` | Read pool | 10 | 4 |
| `src/transactions/database/operations.rs` | Pool | 10 | 5 |
| `src/tools/database/schema.rs` | Pool | 10 | 3 |
| `src/strategies/database.rs` | Pool | 10 | 3 |
| Others | Various | Keep | Keep |

#### A4: jemalloc (2 files)

| File | Lines | Action | What Changes |
|------|-------|--------|-------------|
| `Cargo.toml` | 226 | ✏️ MODIFY | Add tikv-jemallocator dep + jemalloc feature |
| `src/main.rs` | 217 | ✏️ MODIFY | Add 3-line #[global_allocator] block |

#### A5-A6: Wire up existing cleanup functions (2-4 files)

| File | Lines | Action | What Changes |
|------|-------|--------|-------------|
| `src/actions/database.rs` | 977 | 🔍 REVIEW | `cleanup_old_actions()` at line ~885 — function exists, verified |
| `src/rpc/manager.rs` | 687 | 🔍 REVIEW | `cleanup_stats()` at line ~612 — function exists, verified |
| `src/services/implementations/rpc_stats_service.rs` | 46 | ✏️ MODIFY | Add periodic call to `cleanup_stats(retention_hours)` |
| Service loop file for actions (TBD) | — | ✏️ MODIFY | Add periodic call to `cleanup_old_actions(retention_days)` |

#### A7-A8: Config sections (3 files CREATED + 2 MODIFIED)

| File | Lines | Action | What Changes |
|------|-------|--------|-------------|
| ✨ `src/config/schemas/performance.rs` | NEW | ✨ CREATE | PerformanceConfig struct + resolve_profile() |
| ✨ `src/config/schemas/maintenance.rs` | NEW | ✨ CREATE | MaintenanceConfig struct |
| `src/config/schemas/monitoring.rs` | 12 | ✏️ MODIFY | Replace empty MonitoringConfig → import PerformanceConfig (or keep as separate) |
| `src/config/schemas/mod.rs` | 120 | ✏️ MODIFY | Add `pub mod performance; pub mod maintenance;` + add to Config struct |
| `src/config/utils.rs` | 838 | ✏️ MODIFY | Register new config sections for serialization/deserialization |

**Phase A TOTAL: 2 files CREATED + ~18 unique files MODIFIED (some touched in multiple steps)**

---

### 📁 PHASE B-early — True Leaks + Biggest Caches (7 files MODIFIED)

| File | Lines | Action | What Changes |
|------|-------|--------|-------------|
| `Cargo.toml` | 226 | ✏️ MODIFY | Add `moka = { version = "0.12", features = ["sync"] }` (if not added in A4) |
| `src/tokens/decimals.rs` | 468 | ✏️ MODIFY | **DECIMALS_CACHE**: `HashMap<String,u8>` → `moka::sync::Cache` (150K max). **TOKEN_2022_CACHE**: `HashSet` → `moka::sync::Cache` (50K max). Remove unused manual eviction code. |
| `src/transactions/utils.rs` | 327 | ✏️ MODIFY | **GLOBAL_KNOWN_SIGNATURES**: `HashSet` → `moka::sync::Cache` (50K max). |
| `src/positions/state.rs` | 779 | ✏️ MODIFY | **POSITION_LOCKS**: Add `remove_lock(mint)` call. NOT moka — Mutex isn't Clone. |
| `src/positions/database/operations.rs` | 1770 | ✏️ MODIFY | Call `remove_position_lock(mint)` when position closes. |
| `src/actions/state.rs` | 481 | ✏️ MODIFY | **ACTIVE_ACTIONS**: Add periodic cleanup of completed/failed actions from HashMap. |
| 🔍 `src/tokens/decimals.rs` (FETCH_LOCKS) | 468 | 🔍 NO CHANGE | Verified BOUNDED (per-use release + 10K cap) — no fix needed. |

---

### 📁 PHASE B-late — Remaining Caches (9 files MODIFIED)

| File | Lines | Action | What Changes |
|------|-------|--------|-------------|
| `src/tokens/decimals.rs` | 468 | ✏️ MODIFY | **FAILED_CACHE**: `HashSet` → `moka::sync::Cache` (10K max, 5min TTL) |
| `src/transactions/utils.rs` | 327 | ✏️ MODIFY | **GLOBAL_PENDING_TRANSACTIONS**: `HashMap` → `moka::sync::Cache` (10K max, 180s TTL) |
| `src/positions/verifier.rs` | 774 | ✏️ MODIFY | **LAST_TOKEN_ACCOUNTS_CHECK**: `HashMap` → `moka::sync::Cache` (5K max, 1h TTL) |
| `src/tokens/store.rs` | 332 | ✏️ MODIFY | **TokenStore**: Replace custom `TimedCache` HashMap with `moka::sync::Cache` (10K max, 30s TTL). Remove ~200L TimedCache impl. |
| `src/ai/cache.rs` | 70 | ✏️ MODIFY | **AI cache DashMap** → `moka::sync::Cache` (5K max, TTL from config) |
| `src/tokens/pool_data/cache.rs` | 705 | ✏️ MODIFY | **TOKEN_POOLS_CACHE** + **POOL_PREFETCH_STATE**: `HashMap` → `moka::sync::Cache` (5K max, TTL). Slow leak fix. |
| `src/pools/cache.rs` | 355 | ✏️ MODIFY | **PRICE_HISTORY**: Add cleanup on position close (remove entries for closed position's mint) |
| `src/wallets/balance_monitor/cache.rs` | 404 | 🔍 NO CHANGE | COMPUTATION_FAILURES verified BOUNDED — no fix needed. |
| `src/tokens/store.rs` (TimedCache removal) | 332 | ✏️ MODIFY | Remove custom TimedCache struct + impl (~200 lines) |

**Phase B TOTAL: ~11 unique files MODIFIED across B-early + B-late**

---

### 📁 PHASE C1 — FilterToken (5 files CREATED/MODIFIED)

⚠️ **NOT IMPLEMENTED**: The file list below describes the ORIGINAL PLAN for Phase C1 (FilterToken/TokenListEntry).
This was NOT implemented. See phase-c-summary.md for what was actually done (C1-C5: cache migrations, DB maintenance, stale filter).

| File | Lines | Action | What Changes |
|------|-------|--------|-------------|
| ✨ `src/filtering/filter_token.rs` | NEW | ✨ CREATE | FilterToken struct (~35 fields) + SQL query builder |
| `src/filtering/mod.rs` | 49 | ✏️ MODIFY | Add `pub mod filter_token;` |
| `src/filtering/sources/dexscreener.rs` | 299 | ✏️ MODIFY | Change function signatures: `&Token` → `&FilterToken` |
| `src/filtering/sources/geckoterminal.rs` | 241 | ✏️ MODIFY | Change function signatures: `&Token` → `&FilterToken` |
| `src/filtering/sources/rugcheck.rs` | 247 | ✏️ MODIFY | Change function signatures: `&Token` → `&FilterToken` |
| `src/filtering/sources/meta.rs` | 50 | ✏️ MODIFY | Change function signatures: `&Token` → `&FilterToken` |
| `src/filtering/sources/ai.rs` | 110 | ✏️ MODIFY | Use FilterToken for pre-check, `get_full_token()` for LLM analysis |
| `src/filtering/sources/mod.rs` | 576 | ✏️ MODIFY | Update source dispatch to use FilterToken |
| `src/filtering/engine.rs` | 677 | ✏️ MODIFY | `compute_snapshot()`: load FilterTokens instead of full Tokens |
| `src/filtering/store.rs` | 948 | ✏️ MODIFY | Snapshot stores `Arc<FilterToken>` instead of `Arc<Token>` |
| `src/filtering/types.rs` | 292 | ✏️ MODIFY | Add FilterToken to type exports, update PassedToken/RejectedToken |
| `src/tokens/database/assembly.rs` | 1298 | ✏️ MODIFY | Add new optimized SQL query: `get_all_filter_tokens()` (SELECT only ~35 columns, fewer JOINs) |
| `src/tokens/database/async_api.rs` | 456 | ✏️ MODIFY | Add `get_all_filter_tokens_async()` wrapper |

**Phase C1 TOTAL: 1 file CREATED + 12 files MODIFIED**

---

### 📁 PHASE C2 — Incremental Filtering (3 files CREATED/MODIFIED)

⚠️ **NOT IMPLEMENTED**: The file list below describes the ORIGINAL PLAN for Phase C2 (incremental filtering).
This was NOT implemented. See phase-c-summary.md for what was actually done.

| File | Lines | Action | What Changes |
|------|-------|--------|-------------|
| ✨ `src/filtering/incremental.rs` | NEW | ✨ CREATE | Delta query + snapshot merge + safety nets |
| `src/filtering/mod.rs` | 49 | ✏️ MODIFY | Add `pub mod incremental;` |
| `src/filtering/engine.rs` | 677 | ✏️ MODIFY | `compute_snapshot()` → calls incremental logic, falls back to full on first run or config change |
| `src/filtering/store.rs` | 948 | ✏️ MODIFY | Track `last_refresh_timestamp`, switch between full/incremental |
| `src/tokens/database/assembly.rs` | 1298 | ✏️ MODIFY | Add delta query: `get_filter_tokens_updated_since(timestamp)` |
| `src/tokens/database/async_api.rs` | 456 | ✏️ MODIFY | Add async wrapper for delta query |

**Phase C2 TOTAL: 1 file CREATED + 5 files MODIFIED (some overlap with C1)**

---

### 📁 PHASE C3 — SQL Pre-Filtering (OPTIONAL — 2 files MODIFIED)

| File | Lines | Action | What Changes |
|------|-------|--------|-------------|
| `src/filtering/filter_token.rs` | (from C1) | ✏️ MODIFY | Add `generate_sql_prefilter()` from config thresholds |
| `src/tokens/database/assembly.rs` | 1298 | ✏️ MODIFY | Append WHERE clause from `generate_sql_prefilter()` to FilterToken query |

---

### 📁 PHASE D — Maintenance Service (8 files CREATED/MODIFIED)

| File | Lines | Action | What Changes |
|------|-------|--------|-------------|
| ✨ `src/maintenance/mod.rs` | NEW | ✨ CREATE | MaintenanceService struct + Service impl + task scheduler |
| ✨ `src/maintenance/tasks.rs` | NEW | ✨ CREATE | Task implementations: WAL checkpoint, incremental VACUUM, sol_flow cleanup, session cleanup, stale token marking, DB backup before VACUUM |
| ✨ `src/services/implementations/maintenance_service.rs` | NEW | ✨ CREATE | Service wrapper (name, priority, deps, start/stop) |
| `src/services/implementations/mod.rs` | 47 | ✏️ MODIFY | Add `pub mod maintenance_service;` |
| `src/services/mod.rs` | 891 | ✏️ MODIFY | Register MaintenanceService in ServiceManager startup sequence |
| `src/wallets/balance_monitor/database.rs` | 1137 | ✏️ MODIFY | Add `cleanup_old_flow_cache(retention_days)` function (Gap 2 fix) |
| `src/webserver/session.rs` | 129 | ✏️ MODIFY | Ensure `cleanup_expired_sessions()` is callable from maintenance service |
| `src/webserver/routes/tools/multi_wallet/session.rs` | 218 | ✏️ MODIFY | Add `cleanup_expired_sessions()` for MULTI_WALLET_SESSIONS |
| `src/webserver/routes/wallets/types.rs` | 133 | ✏️ MODIFY | Add `cleanup_expired_sessions()` for IMPORT_SESSIONS |
| `src/startup.rs` | 105 | ✏️ MODIFY | Add one-time migration VACUUM check during init (D5) |

**Phase D TOTAL: 3 files CREATED + 7 files MODIFIED**

---

### 📁 PHASE E — Observability + Pressure (10+ files MODIFIED)

| File | Lines | Action | What Changes |
|------|-------|--------|-------------|
| `src/webserver/routes/system.rs` | 545 | ✏️ MODIFY | Add `/api/system/memory`, `/api/system/performance`, `/api/system/maintenance` endpoints |
| `src/webserver/routes/mod.rs` | 262 | ✏️ MODIFY | Register new system routes |
| `src/webserver/snapshot/collectors.rs` | 666 | ✏️ MODIFY | Add memory/cache snapshot collector for dashboard |
| `src/webserver/snapshot/mod.rs` | 142 | ✏️ MODIFY | Wire new collector into snapshot orchestrator |
| `src/services/metrics.rs` | 469 | ✏️ MODIFY | Add cache hit rate + memory metrics collection |
| `src/global.rs` | 322 | ✏️ MODIFY | Add memory pressure level global state |
| `src/webserver/state.rs` | 129 | ✏️ MODIFY | Add memory/pressure data to AppState |
| `src/webserver/embeds.rs` | 368 | ✏️ MODIFY | Embed new Performance dashboard page HTML/JS/CSS |
| `src/webserver/templates.rs` | 433 | ✏️ MODIFY | Add Performance page template + styles |
| `src/telegram/notifier.rs` | (varies) | ✏️ MODIFY | Add memory pressure notification formatters |
| `src/config/schemas/performance.rs` | (from A) | ✏️ MODIFY | Add pressure threshold fields (if not done in Phase A) |

**Dashboard UI files (new, embedded):**
| File | Phase | Action | Purpose |
|------|-------|--------|---------|
| ✨ `templates/pages/performance.html` | E | ✨ CREATE | Performance dashboard page |
| ✨ `scripts/pages/performance.js` | E | ✨ CREATE | Performance page logic |
| ✨ `styles/pages/performance.css` | E | ✨ CREATE | Performance page styling |

**Phase E TOTAL: 3 dashboard files CREATED + ~11 files MODIFIED**

---

### 📁 TEST Files (ALL NEW)

| File | Phase | Purpose | Est. Lines |
|------|-------|---------|-----------|
| ✨ `tests/memory_budget.rs` | A+B | Verify profile cache totals within budget | ~60 |
| ✨ `tests/config_backward_compat.rs` | A | Old config.toml files still work | ~40 |
| ✨ `tests/filtering_pipeline.rs` | C | FilterToken + incremental correctness | ~100 |
| ✨ `tests/fixtures/config_v110.toml` | A | Fixture: pre-performance config | ~50 |

Plus **53 inline unit tests** in `#[cfg(test)]` modules within:
- `src/database/common.rs` (8 tests)
- `src/filtering/filter_token.rs` (5 tests)
- `src/filtering/incremental.rs` (6 tests)
- `src/maintenance/tasks.rs` (8 tests)
- `src/config/schemas/performance.rs` (12 tests)
- Various cache migration files (10 tests)
- `src/filtering/filter_token.rs` SQL prefilter (4 tests)

---

### 📊 MASTER SUMMARY BY ACTION TYPE

```
┌─────────────────────────────────────────────────────────────────┐
│                    FILE CHANGE SUMMARY                           │
├──────────────────┬──────────┬───────────────────────────────────┤
│ Action           │ Count    │ Details                           │
├──────────────────┼──────────┼───────────────────────────────────┤
│ ✨ CREATE (new)  │ 19 files │ 12 .rs source + 3 dashboard      │
│                  │          │ + 4 test files                    │
│ ✏️ MODIFY        │ ~42 files│ Core logic changes                │
│ 🗑️ REMOVE        │ 0 files  │ No files deleted                  │
│ 🔍 REVIEW-ONLY   │ 3 files  │ Verified correct, no changes      │
├──────────────────┼──────────┼───────────────────────────────────┤
│ TOTAL TOUCHED    │ ~61 files│                                   │
└──────────────────┴──────────┴───────────────────────────────────┘
```

### 📊 BY PHASE

```
┌──────────────┬─────────┬──────────┬──────────┬────────────────────────────┐
│ Phase        │ Create  │ Modify   │ Review   │ Key Files                  │
├──────────────┼─────────┼──────────┼──────────┼────────────────────────────┤
│ A Foundation │ 4 files │ 18 files │ 2 files  │ 13 SQLite pool files       │
│ B-early      │ 0       │ 6 files  │ 1 file   │ decimals, state, utils     │
│ B-late       │ 0       │ 8 files  │ 1 file   │ store, pool_data, ai cache │
│ C1 FilterTok │ 1 file  │ 12 files │ 0        │ filter sources, assembly   │
│ C2 Increment │ 1 file  │ 5 files  │ 0        │ engine, store, assembly    │
│ C3 SQL Pre   │ 0       │ 2 files  │ 0        │ filter_token, assembly     │
│ D Maintenanc │ 3 files │ 7 files  │ 0        │ NEW service + cleanups     │
│ E Observable │ 3 files │ 11 files │ 0        │ system routes, dashboard   │
│ Tests        │ 4 files │ 0        │ 0        │ integration + fixtures     │
├──────────────┼─────────┼──────────┼──────────┼────────────────────────────┤
│ **TOTAL**    │ **16**  │ **~42*** │ **3**    │ *some files in >1 phase    │
└──────────────┴─────────┴──────────┴──────────┴────────────────────────────┘
```

### 📊 BY DIRECTORY (unique files)

```
src/
├── database/                          ← NEW DIRECTORY (Phase A)
│   ├── mod.rs                         ✨ CREATE
│   └── common.rs                      ✨ CREATE  (DbConnectionConfig + shared PRAGMA fn)
│
├── maintenance/                       ← NEW DIRECTORY (Phase D)
│   ├── mod.rs                         ✨ CREATE  (MaintenanceService)
│   └── tasks.rs                       ✨ CREATE  (WAL, VACUUM, cleanup tasks)
│
├── config/
│   ├── schemas/
│   │   ├── mod.rs                     ✏️ MODIFY  (add performance + maintenance modules)
│   │   ├── monitoring.rs              ✏️ MODIFY  (12L → replace/extend with PerformanceConfig)
│   │   ├── performance.rs             ✨ CREATE  (PerformanceConfig + resolve_profile)
│   │   └── maintenance.rs             ✨ CREATE  (MaintenanceConfig)
│   ├── macros.rs                      🔍 REVIEW  (verify config_struct! supports new sections)
│   └── utils.rs                       ✏️ MODIFY  (838L — register new config sections)
│
├── tokens/
│   ├── schema.rs                      ✏️ MODIFY  (362L — with_init + fix 30GB mmap)
│   ├── decimals.rs                    ✏️ MODIFY  (468L — 3 caches → moka)
│   ├── store.rs                       ✏️ MODIFY  (332L — TimedCache → moka, remove ~200L)
│   ├── mod.rs                         🔍 REVIEW  (170L — verify module exports)
│   ├── pool_data/
│   │   └── cache.rs                   ✏️ MODIFY  (705L — 2 slow leak caches → moka)
│   └── database/
│       ├── assembly.rs                ✏️ MODIFY  (1298L — add FilterToken SQL + delta query)
│       └── async_api.rs               ✏️ MODIFY  (456L — add async wrappers)
│
├── filtering/
│   ├── mod.rs                         ✏️ MODIFY  (49L — add filter_token + incremental mods)
│   ├── engine.rs                      ✏️ MODIFY  (677L — FilterToken + incremental logic)
│   ├── store.rs                       ✏️ MODIFY  (948L — track last_refresh, Arc<FilterToken>)
│   ├── types.rs                       ✏️ MODIFY  (292L — update type defs)
│   ├── filter_token.rs                ✨ CREATE  (FilterToken struct + SQL)
│   ├── incremental.rs                 ✨ CREATE  (delta merge logic)
│   └── sources/
│       ├── mod.rs                     ✏️ MODIFY  (576L — dispatch with FilterToken)
│       ├── dexscreener.rs             ✏️ MODIFY  (299L — Token → FilterToken)
│       ├── geckoterminal.rs           ✏️ MODIFY  (241L — Token → FilterToken)
│       ├── rugcheck.rs                ✏️ MODIFY  (247L — Token → FilterToken)
│       ├── meta.rs                    ✏️ MODIFY  (50L — Token → FilterToken)
│       └── ai.rs                      ✏️ MODIFY  (110L — FilterToken + get_full_token)
│
├── events/
│   └── database.rs                    ✏️ MODIFY  (1038L — with_init + right-size pool)
│
├── actions/
│   ├── database.rs                    ✏️ MODIFY  (977L — with_init + right-size pool)
│   └── state.rs                       ✏️ MODIFY  (481L — ACTIVE_ACTIONS cleanup)
│
├── positions/
│   ├── state.rs                       ✏️ MODIFY  (779L — POSITION_LOCKS remove on close)
│   ├── verifier.rs                    ✏️ MODIFY  (774L — LAST_TOKEN_ACCOUNTS_CHECK → moka)
│   └── database/
│       └── operations.rs              ✏️ MODIFY  (1770L — with_init + call remove_lock)
│
├── transactions/
│   ├── utils.rs                       ✏️ MODIFY  (327L — 2 caches → moka)
│   └── database/
│       └── operations.rs              ✏️ MODIFY  (925L — update with_init to shared preset)
│
├── strategies/
│   └── database.rs                    ✏️ MODIFY  (710L — with_init + right-size pool)
│
├── wallets/
│   ├── database.rs                    ✏️ MODIFY  (762L — with_init + Cold preset)
│   └── balance_monitor/
│       ├── database.rs                ✏️ MODIFY  (1137L — with_init + fix 30GB mmap + add sol_flow cleanup)
│       └── cache.rs                   🔍 NO CHANGE (404L — COMPUTATION_FAILURES verified bounded)
│
├── pools/
│   └── cache.rs                       ✏️ MODIFY  (355L — PRICE_HISTORY cleanup on close)
│
├── ai/
│   ├── cache.rs                       ✏️ MODIFY  (70L — DashMap → moka)
│   ├── chat_db.rs                     ✏️ MODIFY  (795L — update with_init to shared preset)
│   ├── database.rs                    ✏️ MODIFY  (716L — with_init + Cold preset)
│   └── scheduled_db.rs               🔍 REVIEW  (1044L — verify pool setup)
│
├── rpc/
│   ├── stats/database.rs             ✏️ MODIFY  (579L — with_init + Cold preset)
│   └── manager.rs                     🔍 REVIEW  (687L — cleanup_stats exists, verify callable)
│
├── tools/
│   └── database/schema.rs            ✏️ MODIFY  (457L — with_init + Cold preset)
│
├── ohlcvs/
│   └── database.rs                    ✏️ MODIFY  (1237L — with_init + Standard preset)
│
├── services/
│   ├── mod.rs                         ✏️ MODIFY  (891L — register MaintenanceService)
│   ├── metrics.rs                     ✏️ MODIFY  (469L — add cache/memory metrics)
│   └── implementations/
│       ├── mod.rs                     ✏️ MODIFY  (47L — add maintenance_service mod)
│       ├── maintenance_service.rs     ✨ CREATE  (Service wrapper)
│       ├── rpc_stats_service.rs       ✏️ MODIFY  (46L — wire cleanup_stats timer)
│       └── filtering_service.rs       ✏️ MODIFY  (278L — incremental refresh integration)
│
├── webserver/
│   ├── server.rs                      🔍 REVIEW  (459L — verify route registration)
│   ├── state.rs                       ✏️ MODIFY  (129L — add memory/pressure to AppState)
│   ├── session.rs                     ✏️ MODIFY  (129L — cleanup called from maintenance)
│   ├── embeds.rs                      ✏️ MODIFY  (368L — embed performance page assets)
│   ├── templates.rs                   ✏️ MODIFY  (433L — add performance page template)
│   ├── snapshot/
│   │   ├── mod.rs                     ✏️ MODIFY  (142L — add memory collector)
│   │   └── collectors.rs             ✏️ MODIFY  (666L — add MemoryCollector)
│   └── routes/
│       ├── mod.rs                     ✏️ MODIFY  (262L — register new routes)
│       ├── system.rs                  ✏️ MODIFY  (545L — add memory/perf/maintenance APIs)
│       ├── wallets/types.rs           ✏️ MODIFY  (133L — add session cleanup)
│       └── tools/multi_wallet/
│           └── session.rs             ✏️ MODIFY  (218L — add session cleanup)
│
├── global.rs                          ✏️ MODIFY  (322L — add pressure level state)
├── startup.rs                         ✏️ MODIFY  (105L — one-time migration VACUUM check)
├── main.rs                            ✏️ MODIFY  (217L — jemalloc allocator)
└── lib.rs (or mod root)              ✏️ MODIFY  (add `pub mod database; pub mod maintenance;`)

tests/                                 ← NEW DIRECTORY
├── memory_budget.rs                   ✨ CREATE
├── config_backward_compat.rs          ✨ CREATE
├── filtering_pipeline.rs              ✨ CREATE
└── fixtures/
    └── config_v110.toml               ✨ CREATE

templates/pages/                       (embedded in binary)
└── performance.html                   ✨ CREATE

scripts/pages/
└── performance.js                     ✨ CREATE

styles/pages/
└── performance.css                    ✨ CREATE

Cargo.toml                             ✏️ MODIFY  (add moka + tikv-jemallocator deps)
```

### 📊 LINES OF CODE IMPACT

```
┌──────────────────────────────────────────────────────────────────┐
│                    LINES OF CODE SUMMARY                         │
├──────────────────────┬──────────────────────────────────────────┤
│ Files CREATED        │ ~1,932 new lines across 19 files         │
│ Files MODIFIED       │ ~22,864 lines in ~42 files (touched)     │
│ Lines REMOVED        │ ~200 (TimedCache custom impl in store.rs)│
│ Lines ADDED (est.)   │ ~2,500-3,500 net new lines               │
│ Net Change           │ ~+2,300-3,300 lines                      │
├──────────────────────┼──────────────────────────────────────────┤
│ Tests ADDED          │ 61 tests (53 unit + 4 integration + 4    │
│                      │ benchmark stubs)                          │
│ Test lines           │ ~500-700 new test lines                   │
├──────────────────────┼──────────────────────────────────────────┤
│ TOTAL ESTIMATED      │ ~3,000-4,000 net new lines               │
└──────────────────────┴──────────────────────────────────────────┘
```

### 🔗 FILE DEPENDENCY CHAIN (execution order)

```
START
  │
  ▼
PHASE A ─────────────────────────────────────────────────────
  │
  ├─ A1: CREATE database/mod.rs + database/common.rs
  │       (no dependencies)
  │
  ├─ A2: MODIFY 13 SQLite pool files → use with_init()
  │       (depends on A1: needs configure_sqlite_connection)
  │       │
  │       ├── tokens/schema.rs ──────────┐
  │       ├── events/database.rs          │
  │       ├── actions/database.rs         │
  │       ├── positions/database/ops.rs   │
  │       ├── transactions/database/ops.rs│  All call
  │       ├── strategies/database.rs      │  configure_sqlite_connection()
  │       ├── wallets/database.rs         │  from database/common.rs
  │       ├── wallets/balance_monitor/db  │
  │       ├── tools/database/schema.rs    │
  │       ├── rpc/stats/database.rs       │
  │       ├── ai/chat_db.rs              │
  │       ├── ai/database.rs             │
  │       └── ohlcvs/database.rs ────────┘
  │
  ├─ A3: MODIFY same 13 files (pool max_size adjustments)
  │       (can be done in same commit as A2)
  │
  ├─ A4: MODIFY Cargo.toml + main.rs (jemalloc)
  │       (independent of A1-A3)
  │
  ├─ A5-A6: MODIFY rpc_stats_service.rs + actions service loop
  │          (independent — wire up existing functions)
  │
  ├─ A7-A8: CREATE performance.rs + maintenance.rs
  │          MODIFY schemas/mod.rs + config/utils.rs
  │          (independent of A1-A6)
  │
  └─ A9: MODIFY database/common.rs (add auto_vacuum PRAGMA)
          (depends on A1)
  │
  ▼
PHASE B-early ───────────────────────────────────────────────
  │  (depends on: Cargo.toml has moka, A7 has PerformanceConfig)
  │
  ├─ B1: MODIFY Cargo.toml (add moka)
  │
  ├─ B2-B4: MODIFY tokens/decimals.rs, transactions/utils.rs
  │          (3 biggest caches → moka, independent of each other)
  │
  ├─ B5: MODIFY positions/state.rs + positions/database/operations.rs
  │       (POSITION_LOCKS fix — code change, not moka)
  │
  └─ B6: MODIFY actions/state.rs
          (ACTIVE_ACTIONS cleanup)
  │
  ▼
PHASE B-late ────────────────────────────────────────────────
  │  (depends on: B1 moka available)
  │
  ├─ B8-B10: MODIFY decimals.rs, transactions/utils.rs, verifier.rs
  │           (remaining cache migrations, independent)
  │
  ├─ B11: MODIFY tokens/store.rs (TimedCache → moka, -200L)
  │
  ├─ B12: MODIFY pools/cache.rs (PRICE_HISTORY cleanup)
  │
  ├─ B13-B14: MODIFY ai/cache.rs, pool_data/cache.rs
  │
  └─ B15: MODIFY tokens/pool_data/cache.rs (slow leak fixes)
  │
  ▼
PHASE C1 ────────────────────────────────────────────────────
  │  (INDEPENDENT of Phase B — can run in parallel!)
  │
  ├─ C1: CREATE filtering/filter_token.rs
  │
  ├─ C2-C3: MODIFY assembly.rs + async_api.rs (new SQL query)
  │
  └─ C4-C5: MODIFY 6 filter source files + engine.rs + store.rs
  │
  ▼
PHASE C2 ────────────────────────────────────────────────────
  │  (depends on: C1 FilterToken exists)
  │
  ├─ C6: CREATE filtering/incremental.rs
  │
  └─ C7-C10: MODIFY engine.rs + store.rs + assembly.rs
  │
  ▼
PHASE D ─────────────────────────────────────────────────────
  │  (depends on: A8 MaintenanceConfig, A9 auto_vacuum PRAGMA)
  │
  ├─ D1: CREATE maintenance/mod.rs + maintenance/tasks.rs
  │       CREATE services/implementations/maintenance_service.rs
  │       MODIFY services/implementations/mod.rs + services/mod.rs
  │
  ├─ D2-D4: MODIFY maintenance/tasks.rs (implement tasks)
  │          MODIFY balance_monitor/database.rs (sol_flow cleanup)
  │          MODIFY session.rs, wallets/types.rs, multi_wallet/session.rs
  │
  ├─ D5: MODIFY startup.rs (one-time VACUUM migration)
  │
  └─ D6-D9: MODIFY maintenance/tasks.rs (VACUUM, stale tokens, trading checks)
  │
  ▼
PHASE E ─────────────────────────────────────────────────────
  │  (depends on: ALL previous phases for meaningful data)
  │
  ├─ E1-E2: MODIFY global.rs (pressure state)
  │          MODIFY config/schemas/performance.rs (pressure thresholds)
  │
  ├─ E3-E5: MODIFY webserver/routes/system.rs (3 new API endpoints)
  │          MODIFY webserver/routes/mod.rs
  │
  ├─ E6: CREATE performance.html + performance.js + performance.css
  │       MODIFY embeds.rs + templates.rs
  │
  ├─ E7: MODIFY telegram/notifier.rs
  │
  └─ E8-E9: MODIFY various (poll interval, cache logging)
  │
  ▼
DONE ── All 61 tests pass, bot runs stable at 150-250MB RSS
```

---

## 🔬 Deep Plan Review — Confusing Parts, Logic Issues, and Missing Pros/Cons (v9)

> Full review of the 3,780-line plan after reading every section, verifying 5 critical assumptions
> against actual source code, and analyzing every major flow for edge cases. This section documents
> ALL confusing areas, logic gaps, and additional pros/cons discovered.

### Source Code Verification Results (New Findings)

Before analyzing the plan, I verified 5 critical assumptions:

| # | Plan Assumption | Actual Source Code | Impact |
|---|----------------|-------------------|--------|
| 1 | Blacklist has `blacklisted_at` column | Column is actually `added_at` (tokens/database/blacklist.rs + schema.rs line 155) | C2 delta query must use `added_at`, not `blacklisted_at` |
| 2 | Filtering might have OR logic between DexScreener/GeckoTerminal | **Strict AND logic** — engine.rs:553-639 uses `?` operator for short-circuit rejection. Both sources must pass if enabled | SQL pre-filtering (C3) IS safe with AND conditions ✅ |
| 3 | Config structs use plain derive macros | ALL config uses `config_struct!` macro (config/macros.rs:33-109) with auto-Default, metadata, serde | PerformanceConfig and MaintenanceConfig MUST use `config_struct!` macro |
| 4 | PassedToken holds Arc<Token> | PassedToken is lightweight: only mint, symbol, name, passed_time (4 fields). TokenEntry.token is Arc<Token> in the snapshot's HashMap | Snapshot has both: lightweight pass/reject lists + full TokenEntry with Arc<Token> |
| 5 | TokenStore stores Arc<Token> | Stores plain `Token` (not Arc), clones on every .get() call | moka migration should use Arc<Token> to avoid expensive clones |

---

### 🚨 CRITICAL ISSUE #1: FilterToken Breaks the Tokens List API

**The plan's biggest architectural gap.**

The plan says (Component 3, Phase C1):
> "Snapshot stores `Arc<FilterToken>` instead of `Arc<Token>`"

But the tokens list API (`GET /api/tokens/list`) reads **directly from the FilteringStore snapshot** 
and returns **full Token objects** to the dashboard:
```
store.rs → ensure_snapshot() → collect_entries() → returns Vec<&Token>
list.rs → query_tokens() → returns TokenListResponse { items: Vec<Token> }
```

**If we replace `Arc<Token>` with `Arc<FilterToken>` in the snapshot**, the tokens list page loses 
~25 fields (security_risks, top_holders, websites, socials, description, logo, etc.). The 
dashboard would break — it expects full Token data for the token list table.

**Impact on memory savings**: If we keep full Token in the snapshot to serve the API, the baseline 
stays at 238MB. FilterToken only helps during the filter evaluation step (a few seconds), not for 
the persistent snapshot. **The claimed 238MB → 60MB reduction would NOT materialize.**

**Solutions (3 options, each with trade-offs)**:

**Option A — Decouple API from Snapshot (RECOMMENDED)**:
```
Filtering snapshot stores: FilterToken (60MB) — for filter evaluation ONLY
Tokens list API reads: from DATABASE with pagination — not from snapshot
Snapshot provides: list of passed/rejected mints (Vec<String>, ~8MB)
API flow: GET /api/tokens/list → read passed_mints from snapshot → 
          SELECT * FROM tokens WHERE mint IN (page_mints) LIMIT 50
```
✅ PROS: Full 238→60MB savings. API always returns fresh data. Pagination is natural.
❌ CONS: Tokens list becomes a DB query (2-10ms) instead of memory read (~0.1ms). More complex query logic.
🔧 MITIGATION: 50 tokens × 1.5KB = 75KB per page. SQLite indexed by mint — fast. The DB is already open.

**Option B — Two-Tier Snapshot (FilterToken + Lazy Full Token)**:
```
Snapshot stores: HashMap<String, Arc<FilterToken>> for ALL tokens (60MB)
                + DashMap<String, Arc<Token>> as LRU page cache (moka, max 500)
API flow: check page cache first → miss → fetch from DB → cache
```
✅ PROS: Hot pages served from memory. Filtering uses FilterToken.
❌ CONS: Two caches to manage. Cache coherence between FilterToken and full Token.

**Option C — Keep Full Token in Snapshot, Use FilterToken Only During Filter Evaluation**:
```
Load FilterTokens from DB → evaluate filters → discard FilterTokens
Load full Tokens only for passed mints → store in snapshot
Memory: ~60MB during filtering + ~25MB for passed tokens (~5K × 5KB)
Peak: ~85MB (not 238MB, because we only load full Token for passed mints)
```
✅ PROS: Simplest change. API works unchanged. Still saves ~150MB.
❌ CONS: Two DB queries per refresh (FilterToken + full Token for passed). Less savings than plan claims.
🔧 The ~5K passed tokens × 5KB ≈ 25MB is much less than 171K × 1.4KB = 238MB.

**Recommendation**: Option A for maximum savings + clean architecture. Option C as a simpler 
alternative if Option A is too complex. The plan MUST be updated to address this — the current 
design silently breaks the tokens list page.

---

### 🚨 CRITICAL ISSUE #2: HashMap Clone Cost in Incremental Updates

The plan says (Component 2):
> "Clone existing snapshot's HashMap structure (CHEAP: only Arc pointers + HashMap overhead)
> 171K entries × ~48 bytes (key + Arc pointer) = ~8 MB"

**This math is WRONG.** HashMap<String, TokenEntry> clone behavior:

```
Per entry clone cost:
  String key clone: heap allocation + copy of ~44 bytes (Solana address)
  Arc<Token> clone: atomic increment (cheap, 8 bytes)
  bool fields × 3: trivial
  DateTime: trivial

Total per entry: ~44 bytes heap alloc + ~24 bytes String struct = ~68 bytes stack + ~44 bytes heap
171K entries: 171K × 112 bytes = ~19 MB
PLUS: 171K individual heap allocations (malloc overhead ~16 bytes each) = ~2.7 MB
PLUS: HashMap bucket array reallocation = ~8 MB

ACTUAL clone cost: ~30 MB + 171,000 heap allocations (takes ~5-15ms)
```

**The 171K String allocations are the real problem** — not memory but CPU/latency. 171K malloc 
calls in a tight loop takes measurable time and fragments the heap.

**Solution: Use `Arc<str>` for HashMap keys**:
```rust
// Instead of:
HashMap<String, TokenEntry>

// Use:
HashMap<Arc<str>, TokenEntry>
```
Arc<str> clone is just an atomic increment — zero heap allocation. HashMap clone becomes:
```
171K × (8 bytes Arc + ~40 bytes TokenEntry value) = ~8 MB actual
Zero heap allocations for keys
Clone time: ~1-3ms
```

This makes the incremental clone genuinely cheap as the plan intended.

**Additional benefit**: All mint string storage across the snapshot (filtered_mints, rejected_mints, 
HashMap keys) can share the same Arc<str>, saving ~7MB of duplicate string storage.

✅ PROS: True O(1) clone per key. Eliminates 171K allocations per refresh.
❌ CONS: Arc<str> is less ergonomic than String (can't mutate, needs conversion for API responses).
🔧 MITIGATION: Mint addresses are immutable by nature — Arc<str> is semantically correct.

---

### ⚠️ ISSUE #3: PerformanceConfig MUST Use config_struct! Macro

The plan shows PerformanceConfig as a plain struct with manual Default. But ALL config in the 
codebase uses the `config_struct!` macro which auto-generates:
- `#[derive(Debug, Clone, Serialize, Deserialize)]`
- `#[serde(default)]`
- `impl Default` from declared values
- `FieldTypeInfo` and `NestedMetadata` traits for dashboard UI generation

**If PerformanceConfig doesn't use config_struct!**:
- Dashboard config page can't auto-render its fields
- Hot-reload won't pick up changes
- Metadata system (labels, hints, categories, min/max) won't work
- Inconsistent with every other config section

**Fix**: Use config_struct! macro with metadata annotations:
```rust
config_struct! {
    /// Performance and memory management configuration
    pub struct PerformanceConfig {
        #[metadata(field_metadata! {
            label: "Memory Profile",
            hint: "auto detects system RAM. low/medium/high for manual override",
            impact: "critical",
            category: "Memory",
        })]
        memory_profile: String = "auto".to_string(),

        #[metadata(field_metadata! {
            label: "SQLite Cache Multiplier",
            hint: "0.0 = use profile default. 0.1-5.0 = manual override",
            min: 0.0,
            max: 5.0,
            category: "Advanced",
        })]
        sqlite_cache_multiplier: f64 = 0.0,
        
        // ... etc
    }
}
```

Same for MaintenanceConfig. This is a MUST-DO, not optional.

---

### ⚠️ ISSUE #4: Blacklist Delta Query Uses Wrong Column Name

Plan says (Component 2, incremental refresh):
> "Query blacklisted: SELECT mint WHERE blacklisted_at > last_refresh"

**Actual column is `added_at`** (verified in tokens/database/schema.rs line 155 and blacklist.rs).

Fix: `SELECT mint FROM blacklist WHERE added_at > ?` with `last_refresh` timestamp parameter.

Minor but would cause a runtime SQL error if implemented as-written.

---

### ⚠️ ISSUE #5: auto_vacuum=INCREMENTAL PRAGMA — Misleading Timing

Plan says (Phase A9):
> "Add auto_vacuum=INCREMENTAL pragma to configure_sqlite_connection()"
> "auto_vacuum=INCREMENTAL only activates for NEW pages written after this point"

**This is misleading.** From SQLite docs: "The auto_vacuum pragma can only change the auto-vacuum 
mode for newly created databases, or databases for which a VACUUM has been run."

Setting `PRAGMA auto_vacuum = INCREMENTAL` on an existing database with auto_vacuum=0 (NONE) 
via `with_init()` has **NO EFFECT** until a full VACUUM is run. The PRAGMA gets set on the 
connection but the database file's auto_vacuum mode doesn't change.

**What actually happens**:
1. Phase A9 adds the PRAGMA to the shared function → **no effect on existing databases**
2. Phase D5 runs full VACUUM → **auto_vacuum mode activates**
3. Between A9 and D5, the PRAGMA is silently ignored on existing databases

**The plan says "New pages use incremental" — this is WRONG for existing databases.** Only 
truly NEW databases (created fresh by a first-time user) would get auto_vacuum from A9.

**Fix**: A9 should be documented as "sets the PRAGMA for new databases only; Phase D5 activates 
it for existing databases via VACUUM." Not harmful, but developers shouldn't expect any 
behavior change from A9 alone on existing installations.

---

### ⚠️ ISSUE #6: Memory Pressure Response Has Limited Effect Post-Optimization

Plan says (Component 7, Phase E):
> "Level 1 - Elevated: Increase filtering refresh interval (180s → 300s)"
> "Level 2 - Critical: Increase filtering interval to 600s"

**After our optimizations, increasing the filtering interval has MINIMAL memory effect.** Why:

1. The filtering SNAPSHOT persists in memory regardless of refresh frequency
2. With FilterToken (60MB) + incremental (10MB delta), the refresh cost is tiny
3. With jemalloc, the alloc/free churn from refreshes doesn't fragment
4. The snapshot's memory footprint is constant whether refreshed every 60s or 600s

**What the pressure response ACTUALLY does after optimization**:
- Slightly reduces CPU usage (fewer filter evaluation cycles)
- Slightly reduces DB I/O (fewer delta queries)
- Does NOT reduce the 60MB FilterToken snapshot
- Does NOT reduce the moka cache memory (those are bounded by max_capacity)

**The pressure response is a safety net for unexpected future growth**, not for the 
current optimized architecture. This should be clearly stated in the plan to set 
correct expectations.

**More effective pressure responses**:
- `cache.invalidate_all()` on disposable moka caches → immediate reclaim
- Close idle r2d2 pool connections (if r2d2 supports manual drain)
- Reduce moka cache max_capacity (but moka can't resize at runtime!)

**Actual impact**: The only meaningful lever under pressure is cache invalidation. 
The rest is theater after our optimizations. Plan should be honest about this.

---

### ⚠️ ISSUE #7: One-Time VACUUM During Startup (D5) — UX Black Hole

Plan says:
> "Run BEFORE ServiceManager.start_all() (during init phase)"
> "Log progress: 'Optimizing {db_name}... this is one-time and may take 30-60 seconds'"

**Problem**: At this point in startup, the webserver isn't running. The dashboard shows nothing.
A VPS user running via PM2 sees the bot "starting" for 30-60+ seconds with no visible progress.
They might:
1. Think it crashed → restart it → interrupt the VACUUM → potential corruption
2. See PM2 status as "launching" with no logs (if pm2 doesn't stream stdout immediately)

**Solutions**:

**Option A — Start webserver first, show maintenance page**:
Complex — requires decoupling webserver from other service dependencies.

**Option B — Sentinel file + skip-on-interrupt** (plan already mentions this):
```
1. Create sentinel: data/.vacuum_in_progress
2. Run VACUUM
3. Delete sentinel
4. If bot starts and sentinel exists → previous VACUUM was interrupted
   → Skip VACUUM, log warning, try again next startup
```
This prevents corruption but still has the UX problem.

**Option C — Lazy one-time VACUUM (RECOMMENDED)**:
Don't VACUUM at startup. Instead:
```
1. MaintenanceService starts normally with other services
2. On first maintenance cycle (60 seconds after startup):
   a. Check: has one-time VACUUM been done? (sentinel file: data/.vacuum_complete)
   b. If not: schedule one-time VACUUMs, one DB per cycle (5-minute intervals)
   c. Each VACUUM: check for active trades, proceed if clear
   d. After all DBs done: create data/.vacuum_complete sentinel
3. Total time: ~9 DBs × 5-min intervals = 45 minutes spread across first hour
   (but each individual VACUUM is 10-30 seconds, done during idle moments)
```

✅ PROS: No startup delay. Trading-aware. Webserver running during VACUUM.
❌ CONS: First hour has slightly suboptimal database performance.
🔧 MITIGATION: The memory optimizations (Phase A cache_size + mmap) are IMMEDIATE.
VACUUM is about DISK reclamation, not memory — it can safely be deferred.

---

### ⚠️ ISSUE #8: TokenStore moka Migration Should Use Arc<Token>

Current TokenStore (store.rs:164-188):
```rust
// Stores plain Token, clones on every .get()
HashMap<String, TokenEntry> where TokenEntry { token: Token, ... }
fn get(&self, mint: &str) -> Option<Token> { entry.token.clone() }
```

Token is ~1,390 bytes with heap-allocated Strings/Vecs. Cloning it involves multiple heap 
allocations. With moka, every `.get()` call clones the stored value.

**Fix**: Store `Arc<Token>` in moka:
```rust
moka::sync::Cache<String, Arc<Token>>
// .get() clones the Arc (atomic increment, ~1ns)
// Not the Token (~100ns with heap allocs)
```

This should be specified in the plan for Phase B13.

---

### ⚠️ ISSUE #9: moka run_pending_tasks() — Not Specified in Plan

The plan correctly notes:
> "No background threads in 0.12 — eviction is lazy unless run_pending_tasks() called"

But NEVER specifies where/when `run_pending_tasks()` is called. Without it:
- Eviction happens lazily during get()/insert() operations
- For caches with high TTL but low access frequency, expired entries linger
- For the DECIMALS_CACHE (no TTL, 150K max), eviction happens on insert beyond capacity

**When run_pending_tasks() matters**:
1. **Memory pressure response (E2)**: After `cache.invalidate_all()`, entries aren't immediately 
   freed — must call `run_pending_tasks()` to process pending evictions
2. **Metrics accuracy**: `entry_count()` may be stale without run_pending_tasks()
3. **Testing**: Tests that assert `entry_count() <= max` need run_pending_tasks() first

**Where to call it**:
- In the MaintenanceService loop (every 60s): call run_pending_tasks() on all moka caches
- In the memory pressure response: call immediately after invalidate_all()
- In tests: always before assertions

**Fix**: Add explicit step to Phase D or B: "MaintenanceService calls run_pending_tasks() on 
all moka caches every 60 seconds to ensure timely eviction and accurate metrics."

---

### ⚠️ ISSUE #10: Filtering Snapshot Type Inconsistency

The plan describes FilteringSnapshot differently in different sections:

**Component 2** says:
> "Clone existing snapshot's HashMap structure... 171K entries × ~48 bytes (key + Arc pointer)"

Implies: `HashMap<String, Arc<FilterToken>>`

**Component 3** says:
> "Snapshot stores `Arc<FilterToken>` instead of `Arc<Token>`"

**But actual source code** (verified) shows:
```rust
pub struct FilteringSnapshot {
    pub tokens: HashMap<String, TokenEntry>,
    pub filtered_mints: Vec<String>,
    pub rejected_mints: Vec<String>,
    pub passed_tokens: Vec<PassedToken>,
    pub rejected_tokens: Vec<RejectedToken>,
    pub blacklist_reasons: HashMap<String, Vec<BlacklistReasonInfo>>,
}
```

Where `TokenEntry` has `token: Arc<Token>` plus 4 metadata fields.

The plan needs to be precise: are we replacing `TokenEntry { token: Arc<Token> }` with 
`Arc<FilterToken>`? Or keeping TokenEntry with `Arc<FilterToken>` inside? The metadata fields 
(`has_pool_price`, `has_open_position`, `has_ohlcv`, `pair_created_at`, `last_updated`) are 
used by the API. If we remove them, the API breaks further.

**Recommendation**: If going with Option A from Issue #1 (decouple API from snapshot):
```rust
// Filtering-only snapshot (lean)
pub struct FilteringSnapshot {
    pub filter_tokens: HashMap<Arc<str>, Arc<FilterToken>>,  // for evaluation
    pub passed_mints: Vec<Arc<str>>,      // mints that passed (for API)
    pub rejected_mints: Vec<Arc<str>>,    // mints rejected
    pub passed_tokens: Vec<PassedToken>,  // lightweight summaries
    pub rejected_tokens: Vec<RejectedToken>,
    pub blacklist_reasons: HashMap<Arc<str>, Vec<BlacklistReasonInfo>>,
    pub updated_at: DateTime<Utc>,
}
// Size: 171K × ~400 bytes = ~68MB (vs 238MB with full Token)
```

If going with Option C (keep full Token for passed only):
```rust
pub struct FilteringSnapshot {
    // Full tokens only for passed mints (~5K entries)
    pub tokens: HashMap<String, TokenEntry>,  // ~5K × 5KB = ~25MB
    pub passed_mints: Vec<String>,
    pub rejected_mints: Vec<String>,
    // ... rest unchanged
}
// Size: ~25MB + metadata
```

---

### 📋 Additional Pros/Cons Not Previously Documented

#### Incremental Filtering — Edge Cases

**PRO not documented**: Incremental refresh enables REAL-TIME token updates. Currently, 
a newly listed token with a huge pump takes up to 180 seconds to appear in passed tokens. 
With incremental, it appears on the next delta check (~10-30s depending on update_tracking).

**CON not documented — Stale rejection reasons**: When filtering config changes (e.g., user 
raises min_liquidity from $100 to $1000), tokens that previously passed now fail. But with 
incremental, only CHANGED tokens are re-evaluated. Tokens that DIDN'T change in the DB still 
appear as "passed" in the snapshot with their old evaluation. The safety net (full refresh 
every 30 min) catches this, but there's a 0-30 minute window of stale results.

🔧 Solution: Config change → immediate full refresh (already in the plan as C10). But we 
need to verify the config change detection is robust. Does the bot detect config changes 
during hot-reload? If the user edits config.toml while bot is running, does it trigger?
Answer: Yes — the config system has hot-reload via file watcher.

**CON not documented — Race condition in delta merge**: If a token is being updated in the 
DB (market data write) at the exact moment the delta query runs, we might get a partial update 
(new price but old volume). SQLite WAL mode gives snapshot isolation per-statement, so this 
is actually safe at the SQL level. But if the token update spans multiple INSERTs (e.g., 
update market_dexscreener then update update_tracking), the delta query might see the tracking 
update but miss the market data update if they're in separate transactions.

🔧 Solution: Ensure market data + tracking timestamp are updated in the same SQLite transaction.
Need to verify this in the token update code. If they're separate transactions, this is a bug 
that exists TODAY (not introduced by our changes) but incremental makes it more visible.

#### FilterToken Maintenance Burden — Longer Term

**CON not documented**: Over time, new filter criteria will be added. Each new filter field 
requires updating BOTH Token (for API/display) AND FilterToken (for filtering). If a developer 
adds a field to the filter source but forgets FilterToken, the filter silently has no data for 
that field (Option<> fields default to None).

🔧 Solution: Compile-time enforcement. The filter source functions take `&FilterToken`, so if 
they access a field that doesn't exist, the compiler catches it. BUT: if the filter source 
uses `if let Some(value) = token.new_field` and new_field is Option<> defaulting to None, the 
filter just skips the check — no compile error, no runtime error, just silent degradation.

Better solution: Add a `FilterToken::from_token(token: &Token) -> FilterToken` conversion 
function. When new fields are added to Token, the developer adds them to from_token(), and 
the FilterToken struct naturally grows. Add a test that compares FilterToken::from_token() 
output with the SQL query output to catch drift.

#### jemalloc — Platform-Specific Gotcha

**CON not documented**: On Apple Silicon (M1/M2/M3), jemalloc's performance characteristics 
differ from Intel. The bot currently builds for both via universal binary. jemalloc's 
transparent huge pages and arena configuration may need different tuning on ARM64 vs x86_64.

🔧 Solution: Use default jemalloc configuration (no custom MALLOC_CONF). Default works well 
on both architectures. Only tune if profiling shows issues. The feature flag allows disabling.

#### moka max_capacity Fixed at Creation — Impact on Tunability

The plan acknowledges this but doesn't fully analyze the impact:

**When user changes memory_profile from "medium" to "low"**:
1. Profile resolution changes (e.g., DECIMALS max 150K → 50K)
2. But the moka cache was created at startup with max_capacity=150K
3. It CANNOT be reduced to 50K without creating a new cache
4. New cache means: all existing entries lost, cold cache, potential miss storm

**Cache swap pattern** (if we implement it):
```rust
fn resize_cache(old: &Cache<K,V>, new_max: u64) -> Cache<K,V> {
    let new_cache = Cache::builder().max_capacity(new_max).build();
    // Migrate hot entries? Can't iterate moka efficiently.
    // Option: just start fresh. Cold cache for a few minutes.
    new_cache
}
```

**Recommendation**: Don't implement cache swap. Document that cache size changes require 
restart. This is acceptable because:
1. Profile changes are rare (set once, maybe adjust once more)
2. The bot restarts quickly (< 5 seconds)
3. Cold cache recovers in minutes (natural access patterns repopulate)

---

### 📋 Flow Logic Analysis — All Major Flows

#### Flow 1: SQLite Pool Migration (Phase A2)

```
FOR EACH of 13 database files:
  1. Open file
  2. Find current PRAGMA setup (Pattern #1, #2, or #3)
  3. If Pattern #1 (with_init): update to use shared function
  4. If Pattern #2 (in initialize_schema): move PRAGMAs to with_init, remove from schema fn
  5. If Pattern #3 (on every checkout): remove per-checkout PRAGMAs, add with_init
  6. Pick preset: Hot/Standard/Cold based on workload table
  7. Update pool builder to use with_init(|c| configure_sqlite_connection(c, &preset))
  8. Test: bot starts, PRAGMAs verified via SELECT on each DB
```

**Risk**: Pattern #3 (events, positions, actions) sets PRAGMAs on EVERY checkout as a safety net. 
Removing this and relying only on with_init means trusting that r2d2 always calls init for new 
connections. This IS correct behavior per r2d2 docs, but it's a behavior change.

**Verification**: After migration, connect to each DB and verify:
```sql
PRAGMA journal_mode;  -- should be "wal"
PRAGMA cache_size;    -- should match preset
PRAGMA mmap_size;     -- should match preset
```

#### Flow 2: moka Cache Migration (Phase B)

```
FOR EACH cache migration:
  1. Import moka::sync::Cache
  2. Replace LazyLock<HashMap/HashSet/DashMap> with LazyLock<Cache<K,V>>
  3. Update insert: map.insert(k,v) → cache.insert(k,v)
  4. Update get: map.get(k) → cache.get(k)  // returns cloned value, NOT reference!
  5. Remove manual cleanup code (moka handles eviction)
  6. For caches read by profile: get max from resolve_profile() at init
  7. Test: cache bounded, old entries evicted, functionality preserved
```

**Key gotcha at step 4**: `map.get(&key)` returns `Option<&V>` (reference), while 
`cache.get(&key)` returns `Option<V>` (cloned value). This changes ownership semantics.
Code that does `let val = map.get(&key)?; val.some_field` must change to 
`let val = cache.get(&key)?; val.some_field` — works for owned types but NOT for code 
that holds a reference to the value.

**Solution per cache**:
- DECIMALS_CACHE (u8 values): trivial, u8 is Copy
- TOKEN_2022_CACHE (unit/bool values): trivial
- KNOWN_SIGNATURES (unit/bool values): trivial
- PENDING_TRANSACTIONS (complex values): may need Arc wrapping
- TokenStore (Token values): use Arc<Token> as discussed in Issue #8

#### Flow 3: Incremental Filtering Merge (Phase C2)

```
Every 180 seconds:
  1. Get last_refresh_timestamp from snapshot metadata
  2. Query: SELECT filter_tokens WHERE market_data_last_updated_at > last_refresh
     → delta_updated: Vec<FilterToken>
  3. Query: SELECT filter_tokens WHERE first_discovered_at > last_refresh
     → delta_new: Vec<FilterToken>
  4. Query: SELECT mint FROM blacklist WHERE added_at > last_refresh
     → delta_blacklisted: Vec<String>
  5. Clone existing snapshot's HashMap (key: Arc<str>, cheap)
  6. For each in delta_updated:
     a. Re-evaluate ALL filters on this token
     b. If passes: upsert into HashMap
     c. If fails: remove from HashMap, add to rejected
  7. For each in delta_new:
     a. Evaluate ALL filters
     b. If passes: insert into HashMap
     c. If fails: add to rejected
  8. For each in delta_blacklisted:
     a. Remove from HashMap
     b. Add to rejected with reason "blacklisted"
  9. Rebuild passed_mints/rejected_mints Vec from HashMap
  10. Create new FilteringSnapshot, wrap in Arc
  11. Swap into RwLock<Option<Arc<FilteringSnapshot>>>
  12. Update last_refresh_timestamp
```

**Edge cases NOT in the plan**:

**A. Token deleted from DB** (not blacklisted, just deleted):
- No delta query catches deletions that aren't blacklist operations
- Token remains in snapshot until next full refresh (30 min)
- Impact: LOW — tokens are almost never deleted, only blacklisted

**B. Token's data source changes** (had DexScreener data, now only GeckoTerminal):
- Delta query picks it up (market_data_last_updated_at changed)
- Re-evaluation uses new data source → may change pass/fail
- Impact: Handled correctly ✅

**C. update_tracking timestamp precision**:
- If last_refresh is at T=100 and a token was updated at T=100 exactly
- `WHERE updated_at > 100` misses it (should be >=)
- But `>=` would re-process all tokens from the previous cycle
- Solution: Use `> last_refresh - 1` (1-second overlap, ~10 extra tokens, safe)

**D. Multiple delta queries are NOT atomic**:
- Between query 2 (delta_updated) and query 4 (delta_blacklisted), a token could be
  blacklisted, causing it to appear in delta_updated but ALSO in delta_blacklisted
- Step 8 handles this: blacklist removal happens AFTER upsert, so it correctly removes
- This ordering is critical and must be preserved ✅

---

### 📋 Confusing Terminology in the Plan

| Term | Used As | Actually Means | Recommendation |
|------|---------|---------------|----------------|
| "Clone snapshot" | Copy HashMap | Clone HashMap (171K String allocs) | Say "clone HashMap keys and Arc pointers" |
| "Arc pointers only" | Claims clone is cheap | Actually clones String keys too | Correct after Arc<str> fix |
| "FilterToken" | Lighter struct for filtering | Also changes API behavior | Rename to "FilterEvalToken" to distinguish from API token |
| "One-time VACUUM" | Happens once at startup | Actually happens once per DB | Say "one-time VACUUM per database" |
| "Profile" | memory_profile config | Also used for "CPU profile" elsewhere | Use "MemoryProfile" in code |
| "Pressure Level 3" | Component 7 mentions 3 levels | Phase E only has 2 levels (Elevated + Critical) | Reconcile: Component 7 says 3, Phase E says 2 |

**Pressure level inconsistency**: Component 7 defines 3 levels (Normal, Elevated, Critical) 
but the v8 Phase E review section says "Level 0 (Normal), Level 1 (Elevated), Level 2 (Critical)" 
— only 2 actionable levels. The original Component 7 text mentions "Level 3" in the 
Self-Tuning section: "L2 → Normal: gradually restore..." suggesting a 3rd level that isn't 
clearly defined. The plan should standardize on 3 clear levels or 2.

---

### 📋 Dependency Risks Not Yet Analyzed

#### moka 0.12 → 0.13 (Future Risk)

moka 0.12 is the latest stable, but the changelog shows active development. A 0.13 release 
could change APIs. Pin to `"0.12"` (not `"0.12.*"`) in Cargo.toml to avoid surprise upgrades.

#### tikv-jemallocator Compile Time on CI

The plan says jemalloc adds "~30-60 seconds to compilation." On GitHub Actions CI runners 
(which the bot likely uses for CI), this could be more. Also, jemalloc needs a C compiler 
for the target platform. Cross-compilation from macOS to Linux requires the right cross-compile 
toolchain.

**Fix**: The build.sh already handles cross-compilation. But the CI pipeline (if any) needs 
the jemalloc C dependency. Add to CI setup: `apt-get install -y build-essential` (likely 
already there for Rust builds).

---

### 📊 Summary of Issues by Severity

| # | Severity | Issue | Phase Affected | Fix Complexity |
|---|----------|-------|---------------|---------------|
| 1 | 🚨 CRITICAL | FilterToken breaks tokens list API | C1 | HIGH — needs arch decision |
| 2 | 🚨 CRITICAL | HashMap clone cost underestimated (171K String allocs) | C2 | MEDIUM — use Arc<str> |
| 3 | ⚠️ HIGH | PerformanceConfig must use config_struct! macro | A7 | LOW — use correct macro |
| 4 | ⚠️ MEDIUM | Blacklist column is `added_at`, not `blacklisted_at` | C2 | LOW — rename in query |
| 5 | ⚠️ MEDIUM | auto_vacuum PRAGMA has no effect until VACUUM | A9 | LOW — update docs only |
| 6 | ⚠️ MEDIUM | Memory pressure response is mostly theater post-optimization | E1-E2 | LOW — update expectations |
| 7 | ⚠️ MEDIUM | One-time VACUUM UX problem at startup | D5 | MEDIUM — use lazy approach |
| 8 | ⚠️ MEDIUM | TokenStore should use Arc<Token> with moka | B13 | LOW — wrap in Arc |
| 9 | ⚠️ LOW | moka run_pending_tasks() never specified | B+D | LOW — add to maintenance loop |
| 10 | ⚠️ LOW | FilteringSnapshot type inconsistency across plan sections | C1 | LOW — clarify struct |
| — | 📝 INFO | Pressure level count inconsistency (2 vs 3) | E | Clarify |
| — | 📝 INFO | Terminology confusion (several terms) | All | Clarify |

### 🎯 Recommended Plan Updates (v10)

1. **MUST**: Resolve Critical Issue #1 — decide Option A, B, or C for FilterToken + API coexistence
2. **MUST**: Add Arc<str> for HashMap keys (Issue #2) to Phase C specification
3. **MUST**: Specify config_struct! macro for PerformanceConfig/MaintenanceConfig (Issue #3)
4. **MUST**: Fix blacklist column name in incremental query spec (Issue #4)
5. **SHOULD**: Change D5 to lazy one-time VACUUM via MaintenanceService (Issue #7)
6. **SHOULD**: Specify Arc<Token> for TokenStore moka migration (Issue #8)
7. **SHOULD**: Add run_pending_tasks() to MaintenanceService loop (Issue #9)
8. **SHOULD**: Clarify A9 auto_vacuum PRAGMA limitations (Issue #5)
9. **NICE**: Update memory pressure response expectations (Issue #6)
10. **NICE**: Fix terminology inconsistencies (pressure levels, naming)

---

## 🔬 Deep Critical Issue Resolution — v10 Architecture Decision (NEW)

> After 4 parallel deep-dive investigations into actual source code (store.rs, engine.rs,
> types.rs, tokens.js, list.rs, all filter sources), this section resolves ALL critical issues
> with a unified architecture that does NOT limit the bot in any way.

### Investigation Findings (Source Code Verified)

#### What the Tokens List API Actually Does (store.rs:167-340)

```
1. Get snapshot (171K TokenEntry with Arc<Token>)
2. collect_entries() → filter by view flags (has_pool_price, etc.)
3. Extract &Token references → Vec<&Token>
4. apply_filters() → search, min/max liquidity/volume/risk/holders, flags
5. sort_tokens() → sort by any of 16 sort keys
6. Paginate → slice [start_idx..end_idx] (typically 50 items)
7. Clone ONLY the page (50 tokens) → Vec<Token>    ← ONLY 50 cloned!
8. overlay_pool_price_data() → mutate price_sol on 50 clones
9. Serialize to JSON and return
```

**KEY INSIGHT**: The API only clones 50 Token objects per request, not 171K.
The 238MB snapshot exists for SORTING and FILTERING — not for returning all data.

#### What the Dashboard Tokens Table Actually Renders (tokens.js verified)

The table columns access ONLY these token fields:
- `mint, symbol, name, logo_url/image_url` (identity)
- `price_sol, liquidity_usd, volume_24h, fdv, market_cap` (market)
- `price_change_h1, price_change_h24` (momentum)
- `txns_5m/1h/6h/24h buys+sells` (activity)
- `risk_score` (security — just the score number)
- `has_pool_price, has_ohlcv, has_open_position, blacklisted` (status flags)
- `pool_price_last_calculated_at, metadata_last_fetched_at, market_data_last_fetched_at` (timestamps)
- `blockchain_created_at, first_discovered_at` (timestamps)

**The table NEVER displays**: security_risks[], top_holders[], websites[], socials[],
description, mint_authority, freeze_authority, update_authority, transfer_fee fields,
graph_insiders_detected, lp_provider_count, creator_balance_pct, top_10_holders_pct,
price_native, supply, pool_count, reserve_in_usd, header_image_url, token_type, is_mutable.

These heavy fields are ONLY shown in the **Token Detail Dialog** which fetches from
`GET /api/tokens/:mint` (a separate endpoint that queries DB directly).

#### What Sort Keys Access (store.rs:781-851)

16 sort keys, ALL accessible from lightweight fields:
Symbol, PriceSol, LiquidityUsd, Volume24h, Fdv, MarketCap, PriceChangeH1, PriceChangeH24,
RiskScore, MarketDataLastFetchedAt, FirstDiscoveredAt, MetadataLastFetchedAt,
BlockchainCreatedAt, PoolPriceLastCalculatedAt, Mint, Txns5m/1h/6h/24h

#### What apply_filters() Accesses (store.rs:687-778)

- Search: symbol, mint, name
- Ranges: liquidity_usd, volume_h24, security_score, total_holders
- Flags: has_pool_price, has_open_position, has_ohlcv (from TokenEntry)
- Booleans: is_blacklisted, last_rejection_reason

#### What Filter Evaluation Sources Access (filtering/sources/*.rs)

- **dexscreener.rs**: name, symbol, data_source, txns, liquidity, market_cap, fdv, volumes, price_changes
- **geckoterminal.rs**: data_source, liquidity, market_cap, volumes, price_changes, pool_count, reserve_in_usd
- **meta.rs**: mint (decimals check), mint (cooldown), first_discovered_at
- **rugcheck.rs**: is_rugged, security_score, mint_authority.is_some(), freeze_authority.is_some(),
  graph_insiders_detected, creator_balance_pct, transfer_fee_pct, lp_provider_count,
  total_holders, **top_holders.clone()** ← clones entire Vec
- **ai.rs**: mint

IMPORTANT: Filter evaluation uses the TRANSIENT full Token from DB (during compute_snapshot),
NOT the persistent snapshot. The snapshot is the OUTPUT of evaluation.

#### All Consumers of FilteringStore/FilteringSnapshot

| Consumer | What It Reads | Would Break with Lighter Struct? |
|----------|--------------|----------------------------------|
| API `/api/tokens/list` | Full Token from snapshot for page | YES — but only needs display fields |
| `collect_entries()` | TokenEntry flags (has_pool_price etc.) | YES — needs flags |
| `apply_filters()` | Token fields + TokenEntry flags | YES — needs sort/filter fields |
| `sort_tokens()` | Token sort fields (price, volume, etc.) | YES — needs sort fields |
| Filtering Service | PassedToken list (mint, symbol, name) | NO — lightweight already |
| Pool Discovery | Just mint strings | NO |
| Dashboard stats | Counts from snapshot | NO — just counting |
| "All" view | Queries DB directly (NOT snapshot) | NO |
| "NoMarketData" view | Queries DB directly (NOT snapshot) | NO |
| Rejected Tokens API | Queries DB directly | NO |

### 🎯 RESOLUTION: Option D — TokenListEntry Architecture (NEW — replaces Options A/B/C)

**The v9 options A/B/C all had trade-offs. After deep source code analysis, I designed Option D
which has NO trade-offs — it gives full memory savings without ANY limitation.**

#### Core Insight

The snapshot serves TWO purposes:
1. **Sorting/Filtering/Pagination engine** — needs ~30 sortable/filterable fields per token
2. **API data source** — returns page of tokens to dashboard

Both purposes need the SAME lightweight fields. Neither needs the heavy detail fields
(security_risks Vec, top_holders Vec, socials Vec, websites Vec, description, authority strings).

The heavy fields are ONLY needed:
- During filter EVALUATION (transient, from DB) — specifically rugcheck accesses top_holders
- In Token Detail Dialog (separate API endpoint, fetches from DB on demand)

#### Solution: Replace Arc<Token> with TokenListEntry in Snapshot

```rust
/// Lightweight token representation for the snapshot.
/// Contains ALL fields needed by the tokens table, sorting, filtering, and pagination.
/// Heavy fields (security_risks, top_holders, socials, websites, description) are
/// only loaded during filter evaluation (transient) and token detail dialog (from DB).
#[derive(Debug, Clone, Serialize)]
pub struct TokenListEntry {
    // ── Identity (display + search) ──
    pub mint: Arc<str>,
    pub symbol: Arc<str>,           // Arc<str> for zero-alloc clone
    pub name: Arc<str>,
    pub image_url: Option<Arc<str>>,
    pub data_source: DataSource,
    pub decimals: u8,

    // ── Prices (sort + display) ──
    pub price_sol: f64,
    pub price_usd: f64,
    pub price_change_m5: Option<f64>,
    pub price_change_h1: Option<f64>,
    pub price_change_h6: Option<f64>,
    pub price_change_h24: Option<f64>,

    // ── Market Metrics (sort + filter + display) ──
    pub market_cap: Option<f64>,
    pub fdv: Option<f64>,
    pub liquidity_usd: Option<f64>,

    // ── Volume (sort + display) ──
    pub volume_m5: Option<f64>,
    pub volume_h1: Option<f64>,
    pub volume_h6: Option<f64>,
    pub volume_h24: Option<f64>,

    // ── Transactions (sort + display) ──
    pub txns_m5_buys: Option<i64>,
    pub txns_m5_sells: Option<i64>,
    pub txns_h1_buys: Option<i64>,
    pub txns_h1_sells: Option<i64>,
    pub txns_h6_buys: Option<i64>,
    pub txns_h6_sells: Option<i64>,
    pub txns_h24_buys: Option<i64>,
    pub txns_h24_sells: Option<i64>,

    // ── Security (sort + filter + display — just scores, not full detail) ──
    pub security_score: Option<i32>,       // Raw risk score for sorting
    pub total_holders: Option<i64>,        // For min_holders filter
    pub is_blacklisted: bool,              // For blacklisted view/filter
    pub is_rugged: bool,                   // For display

    // ── Filtering State ──
    pub last_rejection_reason: Option<Arc<str>>,

    // ── Timestamps (sort + display) ──
    pub first_discovered_at: DateTime<Utc>,
    pub blockchain_created_at: Option<DateTime<Utc>>,
    pub metadata_last_fetched_at: DateTime<Utc>,
    pub market_data_last_fetched_at: DateTime<Utc>,
    pub pool_price_last_calculated_at: DateTime<Utc>,

    // ── Status Flags (view filtering) ── (formerly in TokenEntry)
    pub has_pool_price: bool,
    pub has_open_position: bool,
    pub has_ohlcv: bool,
    pub pair_created_at: Option<i64>,
}
```

**Size per TokenListEntry**: ~550 bytes (all Arc<str> fields = 16 bytes each, no heap on clone)
**Total for 171K tokens**: ~94MB

**vs current full Token**: ~1,400 bytes average → 171K × 1,400 = ~240MB

**Savings**: ~146MB (61% reduction in snapshot)

#### Why This Does NOT Limit the Bot

| Concern | Answer |
|---------|--------|
| "Don't limit to 5K tokens" | ALL 171K tokens remain in snapshot — zero limitation |
| "Other tokens never operated" | Every token is evaluated, stored, and browsable |
| "Data stuck in last" | Same 180s refresh cycle — identical freshness |
| "UI performance important" | BETTER — API sends ~600 bytes/token instead of ~3KB JSON. 7× smaller responses |
| "Sorting works" | All 16 sort keys present in TokenListEntry ✅ |
| "Filtering works" | All filter fields present (liquidity, volume, risk, holders, search) ✅ |
| "All views work" | Pool, Passed, Rejected, Blacklisted, Positions, Recent — all flags present ✅ |
| "Token detail works" | Detail dialog already fetches from DB via `/api/tokens/:mint` ✅ |

#### FilterToken Struct Is NO LONGER NEEDED

In Options A/B/C, we proposed a separate "FilterToken" for evaluation. This is unnecessary because:

1. **Evaluation** uses FULL Token loaded from DB (transient during compute_snapshot)
2. **Snapshot** stores TokenListEntry (for API/sort/filter)
3. No third struct needed — two natural layers:
   - Full Token: transient during evaluation, loaded from DB, dropped after
   - TokenListEntry: persistent in snapshot, serves all API needs

This eliminates Critical Issue #1 entirely. There IS no "FilterToken breaks API" because
we don't put FilterToken in the snapshot.

#### Updated FilteringSnapshot Structure

```rust
pub struct FilteringSnapshot {
    pub updated_at: DateTime<Utc>,

    // Token data — ALL tokens with display+sort+filter fields
    pub tokens: HashMap<Arc<str>, TokenListEntry>,  // 171K × ~550B = ~94MB

    // Pre-computed view lists
    pub filtered_mints: Vec<Arc<str>>,         // Mints that passed all filters
    pub rejected_mints: Vec<Arc<str>>,         // Mints that were rejected

    // Lightweight summaries (unchanged)
    pub passed_tokens: Vec<PassedToken>,       // max 1000, for recent-pass display
    pub rejected_tokens: Vec<RejectedToken>,   // max 1000, for recent-reject display
    pub blacklist_reasons: HashMap<Arc<str>, Vec<BlacklistReasonInfo>>,
}
```

**Total snapshot size**: ~94MB (tokens) + ~8MB (mint lists with Arc<str>) + ~1MB (summaries) = **~103MB**
vs current **~240MB**. **137MB saved (57%).**

#### Arc<str> Throughout — Zero-Alloc Clone (Resolves Critical Issue #2)

ALL string fields use `Arc<str>` instead of `String`:
- HashMap keys: `Arc<str>` (clone = atomic increment, 0 allocs)
- symbol, name, image_url, rejection_reason: `Arc<str>` (clone = atomic increment)
- mint lists (filtered_mints, rejected_mints): `Vec<Arc<str>>` (shared with HashMap keys)

**Clone cost for incremental refresh**:
```
171K entries × ~550 bytes (all stack/Arc pointers, zero heap allocs) = ~94MB memcpy
+ 1 HashMap bucket array allocation = ~8MB
Total: ~102MB copy, 1 allocation, ~5-10ms
```

vs current String-based approach:
```
171K entries × 4 String fields × ~44 bytes each = 684K heap allocations
+ 171K × ~112 bytes (String structs) = ~19MB
+ HashMap bucket array = ~8MB
Total: ~27MB + 684,000 heap allocations, ~15-50ms
```

Arc<str> is **5-10× faster** for snapshot cloning and produces **zero heap fragmentation**.

#### Compute Flow (Updated)

```
compute_snapshot() — Phase C implementation:

1. Load Vec<Token> from DB (171K, ~240MB transient)
   └── get_all_tokens_for_filtering_async() — existing function, unchanged

2. Load metadata sets:
   └── priced_set, open_position_set, ohlcv_set — existing logic, unchanged

3. For each token in Vec<Token>:
   a. Evaluate apply_all_filters(&token, &config) — uses FULL Token (rugcheck needs top_holders)
   b. Build TokenListEntry::from(&token, &priced_set, &position_set, &ohlcv_set)
   c. Based on pass/fail: add to passed_mints/rejected_mints
   d. Insert TokenListEntry into HashMap<Arc<str>, TokenListEntry>

4. Vec<Token> drops at end of loop scope → 240MB freed

5. Build FilteringSnapshot → ~103MB persistent
6. Swap into RwLock (old snapshot drops when last Arc reference released)

Peak memory: old_snapshot(103MB) + DB_load(240MB) + new_snapshot_building(103MB) = 446MB
   vs current: old_snapshot(240MB) + DB_load(240MB) = 480MB  ← ACTUALLY WORSE CURRENTLY!
Steady state: 103MB  vs current 240MB  ← 57% BETTER
```

#### API Response Change

```rust
// BEFORE (returns full Token with 90+ fields, ~3-5KB JSON per item):
pub struct TokenListResponse {
    pub items: Vec<Token>,
    // ...
}

// AFTER (returns TokenListEntry with ~45 fields, ~500-800 bytes JSON per item):
pub struct TokenListResponse {
    pub items: Vec<TokenListEntry>,  // Same fields dashboard actually uses
    // ... (rest unchanged)
}
```

**Dashboard JavaScript**: NO changes needed. The JS only accesses fields that ARE in
TokenListEntry. Fields it never accessed (security_risks, socials, etc.) are simply absent
from JSON — JS doesn't notice.

**API response size**: ~200KB per page → ~30KB per page (7× smaller, faster loading,
especially important on slower connections)

#### Code Changes Required

| File | Change |
|------|--------|
| `filtering/types.rs` | Add `TokenListEntry` struct, update `FilteringSnapshot` |
| `filtering/engine.rs` | Build `TokenListEntry` instead of `Arc<Token>` in compute_snapshot |
| `filtering/store.rs` | Update `collect_entries`, `apply_filters`, `sort_tokens` to use `&TokenListEntry` |
| `webserver/routes/tokens/list.rs` | Return `Vec<TokenListEntry>` instead of `Vec<Token>` |
| `webserver/routes/tokens/types.rs` | Update `TokenListResponse` |
| Dashboard JS | **NO CHANGES** — fields are a superset of what JS uses |

#### What About `overlay_pool_price_data()`?

Currently mutates `Token.price_sol` on the 50-item page. With TokenListEntry,
we mutate `TokenListEntry.price_sol` instead. Same logic, different struct:

```rust
fn overlay_pool_price_data(items: &mut [TokenListEntry]) {
    for item in items.iter_mut() {
        if let Some(price_result) = pools::get_pool_price(&item.mint) {
            item.price_sol = price_result.price_sol;
            // ... update pool_price_last_calculated_at
        }
    }
}
```

#### Pros and Cons

**PROS:**
- ✅ ALL 171K tokens remain accessible — zero limitation
- ✅ ALL sort operations work (all 16 sort keys present)
- ✅ ALL view filters work (Pool, Passed, Rejected, Blacklisted, Positions, Recent)
- ✅ ALL query filters work (search, min/max liquidity/volume/risk/holders)
- ✅ API responses 7× smaller (faster dashboard loading)
- ✅ 137MB memory saved (57% reduction in snapshot)
- ✅ Zero-alloc snapshot clone with Arc<str> (5-10× faster incremental refresh)
- ✅ No FilterToken struct needed (simpler, one fewer struct to maintain)
- ✅ Token detail dialog unchanged (already fetches from DB)
- ✅ Filter evaluation unchanged (still uses full Token from DB)
- ✅ Lower peak memory during refresh (446MB vs 480MB current)
- ✅ No dashboard JavaScript changes required

**CONS:**
- ⚠️ New struct to maintain (`TokenListEntry`) — when adding new table columns, must add to both Token and TokenListEntry
- ⚠️ `TokenListEntry::from(&Token)` conversion function needed — potential for drift if fields added to Token but not to conversion
- ⚠️ API response format change — existing API consumers (if any) expecting full Token get fewer fields

**SOLUTIONS FOR CONS:**
- **Drift prevention**: Add compile-time test that verifies TokenListEntry::from() covers all display fields
- **API compatibility**: Version the API or add `?fields=full` parameter for backward compat (if needed)
- **Maintenance**: TokenListEntry fields match 1:1 with dashboard table columns — easy to audit visually

#### Comparison with Original Options

| Aspect | Option A (DB pagination) | Option B (Two-tier) | Option C (5K limit) | **Option D (TokenListEntry)** |
|--------|------------------------|--------------------|--------------------|------------------------------|
| Tokens in snapshot | Mint list only | FilterToken | ~5K passed only | **ALL 171K** |
| Memory savings | 238→8MB (96%) | 238→60MB (75%) | 238→25MB (90%) | **238→94MB (61%)** |
| Sorting | DB query (slower) | In-memory (fast) | Only passed tokens | **In-memory, all tokens (fast)** |
| API latency | 2-10ms per page | ~0.1ms | ~0.1ms | **~0.1ms** |
| Dashboard changes | YES (pagination logic) | Complex cache | May break views | **NONE** |
| Complexity | HIGH | VERY HIGH | MEDIUM | **LOW** |
| All tokens browsable | Via DB queries | Via cache miss→DB | NO (only 5K) | **YES (all 171K)** |
| Incremental refresh | Complex | Complex | Simple | **Same as current** |

**Option D wins on every axis except raw memory savings.** The 61% savings is slightly less than
Options A/C, but Options A/C have severe limitations (DB query latency, 5K token limit).
Combined with Phase A (SQLite PRAGMAs, ~500-700MB saved), Option D's 137MB is more than enough
to bring total memory to comfortable levels.

#### Total Memory Architecture (All Phases Combined)

| Component | Current | After All Phases | Savings |
|-----------|---------|-----------------|---------|
| SQLite page caches (Phase A) | ~580 MB | ~84 MB | **496 MB** |
| Filtering snapshot (Phase C - Option D) | ~240 MB | ~103 MB | **137 MB** |
| moka caches (Phase B) | unbounded | bounded ~50 MB max | **variable** |
| jemalloc fragmentation (Phase A) | ~100-200 MB | ~20-50 MB | **~100 MB** |
| DashMap leaks (Phase B) | ~30 MB growing | bounded | **~20 MB** |
| **TOTAL estimated** | **~800-1200 MB** | **~250-350 MB** | **~600-800 MB** |

Bot should run comfortably at **250-350 MB RSS** for typical usage (171K tokens, normal trading).

### Resolution of All 10 Issues

| # | Issue | Resolution |
|---|-------|-----------|
| 1 | 🚨 FilterToken breaks API | **RESOLVED** — Option D: TokenListEntry has all API fields, no breakage |
| 2 | 🚨 HashMap clone cost 171K allocs | **RESOLVED** — Arc<str> for all strings, zero heap allocs on clone |
| 3 | ⚠️ config_struct! macro | Unchanged — PerformanceConfig/MaintenanceConfig MUST use macro |
| 4 | ⚠️ Blacklist column name | Unchanged — use `added_at`, not `blacklisted_at` |
| 5 | ⚠️ auto_vacuum PRAGMA timing | Unchanged — document A9 is for new DBs only |
| 6 | ⚠️ Memory pressure is theater | Reframed — pressure system is a safety valve, not primary tool |
| 7 | ⚠️ VACUUM UX at startup | Unchanged — lazy approach via MaintenanceService |
| 8 | ⚠️ TokenStore moka needs Arc | Unchanged — use Arc<Token> for moka migration |
| 9 | ⚠️ run_pending_tasks() missing | Unchanged — add to MaintenanceService loop |
| 10 | ⚠️ Snapshot type inconsistency | **RESOLVED** — TokenListEntry is the single clear type |

### FilterToken Concept — DEPRECATED

The original plan's "FilterToken" (Component 3) is replaced by TokenListEntry (Option D).
FilterToken was designed to reduce snapshot memory by storing only ~35 evaluation fields.
But since evaluation uses TRANSIENT full Token from DB (not the snapshot), a separate
evaluation-only struct is unnecessary. The snapshot only needs display+sort+filter fields,
which is exactly what TokenListEntry provides.

---

## 🔄 Comprehensive Phase Review — v11 Corrections

> v11: Full phase-by-phase codebase verification. Cross-referenced every file, directory,
> function, and dependency in the plan against actual source code. Fixed inconsistencies
> between v8 flowchart (which still uses "FilterToken") and v10 (which replaces it with
> TokenListEntry). Found 12 corrections across all phases.

### Source Code Verification Results (v11)

| # | Plan Claim | Actual Codebase | Impact | Fix |
|---|-----------|----------------|--------|-----|
| 1 | `src/database/` directory will be created | ✅ Correct — does NOT exist yet | None | Plan accurate |
| 2 | `src/maintenance/` directory will be created | ✅ Correct — does NOT exist yet | None | Plan accurate |
| 3 | Module root is `src/lib.rs` or top-level mod | ✅ It's `src/lib.rs` (lines 3-42) with 38 modules | Clarify: add `pub mod database;` and `pub mod maintenance;` to lib.rs | Update flowchart |
| 4 | Phase C1 creates `filter_token.rs` with "FilterToken" | ❌ INCONSISTENT — v10 deprecated FilterToken, uses TokenListEntry | Phase C flowchart references wrong struct name | **CRITICAL: Update all C1 references** |
| 5 | Phase C1 store.rs change says `Arc<FilterToken>` | ❌ INCONSISTENT — v10 says `TokenListEntry` | Wrong type in flowchart | Update flowchart |
| 6 | Phase A5 wires cleanup_old_actions into "actions service loop (TBD)" | ❌ **No actions_service.rs exists**. No actions background loop. | A5 cannot wire into non-existent service | Wire into MaintenanceService (Phase D) OR create simple tokio::spawn in run.rs |
| 7 | All 12 cache locations (Phase B) | ✅ ALL verified — exact types, lines, variable names match | None | Plan accurate |
| 8 | cleanup_expired_sessions() is callable from maintenance | ✅ Exists at session.rs:108, is `pub fn` | But NEVER called anywhere — plan correctly notes this | Plan accurate |
| 9 | MULTI_WALLET_SESSIONS has cleanup | ✅ `cleanup_old_sessions()` at line 63, removes >1 hour old | Called from multi_buy.rs and multi_sell.rs already | Already handled — D4 still useful as periodic insurance |
| 10 | startup.rs can host one-time VACUUM check | ✅ Exists (service status tracking), but NOT the right place | startup.rs tracks service readiness, not DB migration | Move D5 to run.rs (where ServiceManager is started) |
| 11 | TokenListResponse.items is Vec<Token> | ✅ Confirmed at types.rs:22 — `items: Vec<crate::tokens::types::Token>` | Phase C changes this to Vec<TokenListEntry> | Need to update Telegram callbacks.rs too |
| 12 | Only webserver consumes query_tokens() | ❌ **Telegram callbacks.rs also calls it** (2 locations) | Phase C must update Telegram consumer too | Add telegram/callbacks.rs to C1 file list |

### Correction 1: Phase A5 — No Actions Service Exists

**Problem**: Plan says "Wire up cleanup_old_actions() — add timer call in actions service loop."
But there IS no `actions_service.rs` and no background loop for actions.

**Options**:
- A) Create a new actions_service.rs (overkill for one cleanup call)
- B) Wire it into the MaintenanceService (Phase D) — natural home for cleanup tasks
- C) Add a simple `tokio::spawn` in run.rs during startup

**Resolution**: Move A5 (cleanup_old_actions) from Phase A to Phase D (MaintenanceService).
This is the right architectural home — maintenance tasks belong in the maintenance service.
Keep A6 (cleanup_stats via rpc_stats_service) in Phase A since that service already exists.

**Updated Phase A**:
```
A5. Wire up cleanup_stats() — add timer call in rpc_stats_service.rs  [KEEP]
A6. (REMOVED — cleanup_old_actions() moves to Phase D, wired into MaintenanceService)
```

**Updated Phase D**:
```
D3. Wire cleanup_old_actions() into MaintenanceService loop
    - Call ActionsDatabase::cleanup_old_actions(retention_days) every 24h
    - Retention from [maintenance] config (default: 30 days)
```

### Correction 2: Phase C — Rename FilterToken → TokenListEntry Throughout

**Problem**: The v8 file change flowchart (Phase C1 section) still uses "FilterToken" in:
- Section title: "PHASE C1 — FilterToken"
- NEW file: `src/filtering/filter_token.rs` → "FilterToken struct"
- store.rs change: `Arc<FilterToken>`
- All filter source changes: `&Token → &FilterToken`

**v10 explicitly deprecated FilterToken** and replaced it with TokenListEntry. The flowchart
was written BEFORE v10 and was never updated.

**Resolution**: The v10 architecture is correct. Apply these renames:

| v8 Flowchart (WRONG) | v11 Corrected |
|-----------------------|---------------|
| Phase C1 title: "FilterToken" | Phase C1 title: "**TokenListEntry**" |
| CREATE `filter_token.rs` | CREATE: **add TokenListEntry to `filtering/types.rs`** (not a new file) |
| store.rs: `Arc<FilterToken>` | store.rs: **`TokenListEntry`** (no Arc needed — struct is ~550 bytes, cheaper to clone directly than Arc overhead for small structs) |
| Filter sources: `&Token → &FilterToken` | Filter sources: **NO CHANGE** — v10 clarified that filter evaluation uses TRANSIENT full Token from DB, NOT the snapshot. Filter sources keep using `&Token`. Only the SNAPSHOT output changes. |
| engine.rs: "load FilterTokens" | engine.rs: **load full Tokens (transient), evaluate, convert to TokenListEntry for snapshot** |

**Major simplification**: Filter sources (dexscreener.rs, geckoterminal.rs, rugcheck.rs, meta.rs, ai.rs)
do NOT need any changes! They continue to operate on `&Token` during the transient evaluation phase.
The TokenListEntry conversion happens AFTER evaluation, when building the snapshot.

**Updated Phase C1 file list** (replacing the v8 flowchart):

| File | Lines | Action | What Changes |
|------|-------|--------|-------------|
| `src/filtering/types.rs` | 292 | ✏️ MODIFY | Add `TokenListEntry` struct (~45 fields). Update `FilteringSnapshot` to use `HashMap<Arc<str>, TokenListEntry>`. Remove old `TokenEntry` wrapper. |
| `src/filtering/engine.rs` | 677 | ✏️ MODIFY | `compute_snapshot()`: after evaluating each token against filters using full Token, convert to TokenListEntry. Build new snapshot with TokenListEntry HashMap. |
| `src/filtering/store.rs` | 948 | ✏️ MODIFY | Update `collect_entries()`, `apply_filters()`, `sort_tokens()` to work with `&TokenListEntry` instead of `&Token`. Update `overlay_pool_price_data()` to mutate TokenListEntry.price_sol. |
| `src/filtering/mod.rs` | 49 | ✏️ MODIFY | Export TokenListEntry from types |
| `src/webserver/routes/tokens/types.rs` | varies | ✏️ MODIFY | TokenListResponse.items: `Vec<Token>` → `Vec<TokenListEntry>` |
| `src/webserver/routes/tokens/list.rs` | varies | ✏️ MODIFY | Update response building to use TokenListEntry |
| `src/telegram/commands/callbacks.rs` | varies | ✏️ MODIFY | Update query_tokens() consumers to use TokenListEntry fields |

**Files NO LONGER modified in Phase C1** (removed from v8 list):
- ❌ `src/filtering/sources/dexscreener.rs` — no change (uses transient Token)
- ❌ `src/filtering/sources/geckoterminal.rs` — no change
- ❌ `src/filtering/sources/rugcheck.rs` — no change
- ❌ `src/filtering/sources/meta.rs` — no change
- ❌ `src/filtering/sources/ai.rs` — no change
- ❌ `src/filtering/sources/mod.rs` — no change
- ❌ `src/tokens/database/assembly.rs` — no change (full Token query stays for evaluation)
- ❌ `src/tokens/database/async_api.rs` — no change

**Net impact**: Phase C1 touches **7 files** instead of 13. Dramatically simpler.
The filter evaluation pipeline is UNTOUCHED — only the snapshot output format changes.

### Correction 3: NEW File List Update

**v8 lists** `src/filtering/filter_token.rs` as a NEW file to create.
**v11**: This file is NOT created. Instead, TokenListEntry is added to the existing
`src/filtering/types.rs` file. This is better because:
- types.rs already has FilteringSnapshot, TokenEntry, PassedToken, RejectedToken
- TokenListEntry naturally lives alongside these types
- One fewer file to create and manage

**Updated NEW files list** (removing filter_token.rs, adding test adjustments):

| File | Phase | Purpose | Est. Lines |
|------|-------|---------|-----------|
| ✨ `src/database/mod.rs` | A | Module: `pub mod common;` | ~2 |
| ✨ `src/database/common.rs` | A | DbConnectionConfig + configure_sqlite_connection() | ~80 |
| ✨ `src/config/schemas/performance.rs` | A | PerformanceConfig via config_struct! macro | ~120 |
| ✨ `src/config/schemas/maintenance.rs` | A | MaintenanceConfig via config_struct! macro | ~80 |
| ~~✨ `src/filtering/filter_token.rs`~~ | ~~C1~~ | ~~REMOVED — TokenListEntry goes in types.rs~~ | ~~—~~ |
| ✨ `src/filtering/incremental.rs` | C2 | Delta query + snapshot merge + safety nets | ~300 |
| ✨ `src/maintenance/mod.rs` | D | MaintenanceService + task scheduler | ~150 |
| ✨ `src/maintenance/tasks.rs` | D | Task implementations | ~250 |
| ✨ `src/services/implementations/maintenance_service.rs` | D | Service wrapper | ~50 |
| ✨ `tests/memory_budget.rs` | B+E | Profile budget integration test | ~60 |
| ✨ `tests/config_backward_compat.rs` | A | Config backward compatibility test | ~40 |
| ✨ `tests/filtering_pipeline.rs` | C | Filtering correctness test | ~100 |

**Total NEW files: 11 (was 12)**

### Correction 4: Phase D5 — startup.rs Is Wrong Location

**Problem**: Plan says "MODIFY startup.rs — Add one-time migration VACUUM check during init (D5)."
But startup.rs tracks service readiness status — it's not where DB initialization happens.

**ServiceManager.start_all() is called in run.rs** (line 152 for pre-init, line 319 for normal).
The one-time VACUUM should happen in run.rs BEFORE start_all(), or better yet, as the first
task of MaintenanceService (as v9 Issue #7 already recommended).

**Resolution**: D5 uses the lazy approach — MaintenanceService handles it:
- D5 target file: `src/maintenance/tasks.rs` (the one-time VACUUM logic)
- D5 trigger: MaintenanceService first cycle (60s after startup)
- D5 sentinel: `data/.vacuum_complete` file
- startup.rs: NOT modified for D5

### Correction 5: Module Registration in lib.rs

**v8 flowchart says**: "MODIFY `src/lib.rs` or top-level `mod`"
**v11 verification**: It's definitively `src/lib.rs` (38 mod declarations, lines 3-42)

Add:
```rust
pub mod database;     // after line 8 (config)
pub mod maintenance;  // after line 15 (logger)
```

### Correction 6: Phase A Step Count Adjustment

Phase A originally had 10 steps (A1-A10 in v8). With A5 cleanup_old_actions moved to Phase D:

```
Phase A — Foundation (9 steps):
  A1. Create src/database/common.rs + mod.rs  [unchanged]
  A2. Migrate all 13 SQLite pools to with_init()  [unchanged]
  A3. Right-size connection pool max_size  [unchanged]
  A4. jemalloc with feature flag  [unchanged]
  A5. Wire up cleanup_stats() in rpc_stats_service.rs  [was A6]
  A6. Add [performance] config section (config_struct! macro)  [was A7]
  A7. Add [maintenance] config section (config_struct! macro)  [was A8]
  A8. Add auto_vacuum=INCREMENTAL to shared function  [was A9]
  A9. VERIFY: build, run, measure RSS  [was A10]
```

### Correction 7: Phase D Step Adjustment

Phase D gains cleanup_old_actions from Phase A:

```
Phase D — Maintenance Service (10 steps):
  D1.  Create MaintenanceService  [unchanged]
  D2.  WAL checkpoint  [unchanged]
  D3.  Wire cleanup_old_actions() (from actions/database.rs)  [NEW — moved from Phase A]
  D4.  sol_flow_cache cleanup  [was D3]
  D5.  Periodic session cleanup  [was D4]
  D6.  One-time VACUUM via MaintenanceService  [was D5, moved from startup.rs]
  D7.  Batched incremental VACUUM  [was D6]
  D8.  Stale token marking  [was D7]
  D9.  Maintenance status tracking  [was D8]
  D10. Trading-aware checks  [was D9]
```

### Correction 8: Telegram Consumer of query_tokens()

**Discovery**: `src/telegram/commands/callbacks.rs` calls `query_tokens()` in 2 locations.
This was NOT in the Phase C file list. If TokenListResponse changes from Vec<Token> to
Vec<TokenListEntry>, the Telegram callback code that formats token data for Telegram
messages also needs updating.

**Impact**: LOW — Telegram callbacks likely only use mint, symbol, name, price, volume
(fields that ARE in TokenListEntry). But the type change requires code updates.

**Added to Phase C1**: `src/telegram/commands/callbacks.rs` ✏️ MODIFY

### Correction 9: TokenListEntry Cloning Strategy

**v10 says**: Use `Arc<str>` for all string fields in TokenListEntry to enable zero-alloc clone.

**Consideration**: TokenListEntry at ~550 bytes is relatively small. The snapshot clone
during incremental refresh copies 171K entries. Two approaches:

1. **Direct clone** (no Arc): 171K × 550 bytes + 171K × ~80 bytes (4 String heap allocs each) = ~108MB + 684K allocs
2. **Arc<str> fields**: 171K × 550 bytes + 0 heap allocs = ~94MB copy, ~5ms

**Decision**: Use Arc<str> for the 4-5 string fields (mint, symbol, name, image_url, 
last_rejection_reason). The rest are numeric/bool/enum types that are Copy.
This gives the zero-alloc benefit with minimal API ergonomic cost.

**HashMap key**: `Arc<str>` (shared with TokenListEntry.mint field — same Arc instance).

### Correction 10: Arc vs Direct for Snapshot Storage

**v10 says**: Snapshot stores `HashMap<Arc<str>, TokenListEntry>` with direct TokenListEntry values.
**v8 used**: `HashMap<String, TokenEntry>` where TokenEntry had `token: Arc<Token>`.

**Analysis**: Should we wrap TokenListEntry in Arc?
- TokenListEntry: ~550 bytes (smaller than Token's ~1400)
- Snapshot has ONE owner (the FilteringStore behind RwLock)
- API clones only 50 per page request
- Incremental refresh clones the entire HashMap

**Decision**: Direct `TokenListEntry` (not Arc-wrapped) in the HashMap. Reasons:
- No shared ownership needed — snapshot is read-only after creation
- 550 bytes is small enough that clone is fast
- Arc overhead (16 bytes + atomic ops) for 171K entries = 2.7MB wasted
- The HashMap value clone during incremental refresh is a memcpy, not heap alloc

### Correction 11: Phase Order Final Verification

After all corrections, the phase dependency chain is:

```
Phase A (Foundation)
  ├─ A1: CREATE database/common.rs (no deps)
  ├─ A2: MODIFY 13 pool files → with_init() (depends on A1)
  ├─ A3: MODIFY same files → pool max_size (can merge with A2)
  ├─ A4: Cargo.toml + main.rs → jemalloc (independent)
  ├─ A5: rpc_stats_service.rs → wire cleanup (independent)
  ├─ A6: CREATE performance.rs config (independent)
  ├─ A7: CREATE maintenance.rs config (independent)
  ├─ A8: MODIFY database/common.rs → auto_vacuum (depends on A1)
  └─ A9: VERIFY (depends on all above)
  
Phase B-early (Critical caches — depends on A4 for moka dep, A6 for profile values)
  ├─ B1: Cargo.toml → add moka
  ├─ B2-B4: 3 biggest caches → moka (independent of each other)
  ├─ B5-B6: POSITION_LOCKS + ACTIVE_ACTIONS leak fixes (code changes, not moka)
  └─ B7: VERIFY

Phase B-late (Remaining caches — depends on B1)
  ├─ B8-B15: Remaining cache migrations (independent of each other)
  └─ VERIFY

Phase C1 (TokenListEntry — INDEPENDENT of Phase B!)
  ├─ C1: Add TokenListEntry to types.rs
  ├─ C2: Update engine.rs compute_snapshot → build TokenListEntry
  ├─ C3: Update store.rs → collect/filter/sort on &TokenListEntry
  ├─ C4: Update API response types
  └─ C5: VERIFY (snapshot size, API responses)

Phase C2 (Incremental — depends on C1 for TokenListEntry, benefits from Arc<str>)
  ├─ C6: Implement delta query
  ├─ C7: Implement merge logic
  ├─ C8: Safety nets
  ├─ C9: Profile-based refresh interval
  └─ C10: Config change → full refresh

Phase C3 (SQL pre-filter — OPTIONAL, depends on C1)
  └─ C11: Generate SQL WHERE from config

Phase D (Maintenance — depends on A7 for config, A8 for auto_vacuum)
  ├─ D1: CREATE MaintenanceService
  ├─ D2: WAL checkpoint
  ├─ D3: Wire cleanup_old_actions
  ├─ D4: sol_flow cleanup
  ├─ D5: Session cleanup
  ├─ D6: One-time VACUUM (lazy, via MaintenanceService)
  ├─ D7: Batched incremental VACUUM
  ├─ D8-D10: Stale tokens, tracking, trading safety
  └─ VERIFY

Phase E (Observability — depends on all above)
  ├─ E1-E2: Pressure detection
  ├─ E3-E5: API endpoints
  ├─ E6: Dashboard Performance panel
  ├─ E7-E10: Telegram, poll intervals, logging, CLI
  └─ VERIFY
```

**Ordering is CORRECT.** The key insight confirmed:
- B and C1 are INDEPENDENT (can be done in either order or parallel)
- Current order (B before C1) is better from risk perspective
- Phase A delivers biggest bang for buck and has lowest risk
- First 3 phases (A + B-early + C1) deliver ~90% of memory benefit

### Correction 12: File Count Summary Update

| Phase | v8 Files | v11 Files | Change | Reason |
|-------|----------|-----------|--------|--------|
| A | 2 CREATE + 18 MODIFY | 2 CREATE + 17 MODIFY | -1 MODIFY | cleanup_old_actions moved to D |
| B-early | 0 CREATE + 6 MODIFY | Same | — | — |
| B-late | 0 CREATE + 8 MODIFY | Same | — | — |
| C1 | 1 CREATE + 12 MODIFY | 0 CREATE + 7 MODIFY | **-1 CREATE, -5 MODIFY** | No filter_token.rs, no filter source changes |
| C2 | 1 CREATE + 5 MODIFY | Same | — | — |
| D | 3 CREATE + 7 MODIFY | 3 CREATE + 7 MODIFY | Same count, different targets | startup.rs → maintenance/tasks.rs |
| E | 3 CREATE + 11 MODIFY | Same | — | — |

**Net change**: 1 fewer CREATE (filter_token.rs removed), 6 fewer MODIFY (filter sources unchanged).
**Total: 10 CREATE + ~40 MODIFY + 0 REMOVE** (was 11 CREATE + ~42 MODIFY).

### Updated Risk-Ordered Implementation Priority

```
Phase A alone:           ~500-700 MB (SQLite + jemalloc)      ← HUGE win, LOW risk
Phase A + B-early:       + leak fixes + biggest caches        ← essential correctness
Phase A + B-early + C1:  + 137 MB from TokenListEntry         ← significant, SIMPLER now
Phase A + B:             + all caches bounded                  ← completeness
Phase A + B + C:         + incremental filtering               ← eliminates peak spikes
Phase A + B + C + D:     + self-maintaining databases          ← long-term stability
Phase A + B + C + D + E: + observability + OOM prevention      ← production-grade
```

**Phase C1 is now MUCH simpler** — 7 files instead of 13, no filter source changes.
This reduces its risk from MEDIUM to LOW-MEDIUM. The critical insight from v10 (filter
sources use transient Token, not snapshot) eliminated half the work.

### Final Consistency Check

| Plan Section | Struct Name | Consistent? |
|-------------|-------------|-------------|
| Component 3 (original) | FilterToken | ❌ OLD — deprecated by v10 |
| v8 Phase C specification | FilterToken | ❌ OLD — deprecated by v10 |
| v8 File change flowchart | FilterToken | ❌ OLD — deprecated by v10 |
| v10 Option D resolution | TokenListEntry | ✅ CURRENT |
| v11 corrections (this section) | TokenListEntry | ✅ CURRENT |

**Reading order for implementers**: Read v10 + v11 sections. The v8 Phase C and flowchart
sections contain outdated references to "FilterToken" which are superseded by TokenListEntry.
When implementing, follow v11 file lists and v10 architecture.

### Plan Status: READY FOR IMPLEMENTATION

All phases verified against codebase. All inconsistencies resolved. Phase ordering confirmed
correct. File lists updated and verified. The plan is comprehensive and implementation-ready.

**When user says "start" or "implement":**
1. Read v10 + v11 + v12 sections (authoritative)
2. Follow Phase A steps (9 steps, ~17 files)
3. Build and verify after each phase
4. Proceed through B → C1 → C2 → D → E

---

## 🔬 Deep Flow Analysis — v12 Corrections and Clarifications (NEW)

> v12: Deep analysis of 5 specific flows that were insufficiently documented.
> Verified against actual source code (engine.rs, types.rs, filtering_service.rs,
> wallet balance_monitor/service.rs, events maintenance.rs).
> All findings resolved with concrete implementation guidance.

### v12 Finding 1: Phase A2 — Database Preset Assignment Justification

The plan assigns Hot/Standard/Cold presets but lacked workload analysis. Here's the
justified assignment table based on actual code access patterns:

| Database | Preset | Reads/Min (est.) | Writes/Min | Why This Preset |
|----------|--------|-------------------|------------|-----------------|
| **tokens.db** | **Hot** | 100+ (filtering every 3min loads 171K rows) | 50 (market data updates) | Highest read volume, biggest table, filtering is critical path |
| **transactions.db** | **Hot** | 50+ (real-time monitoring + dashboard) | 20 (transaction recording) | Live WebSocket monitoring, real-time queries during trading |
| **events.db read** | **Standard** | 10-30 (dashboard events page) | — | Dashboard access only, not on critical path |
| **events.db write** | **Standard** | — | 30+ (events from all services) | High write volume, WAL essential, but cache doesn't help writes much |
| **positions.db** | **Standard** | 20 (dashboard home every 5s + trading) | 5 (position open/close/update) | Small dataset (~50-200 rows), frequent but lightweight reads |
| **actions.db read** | **Standard** | 5-15 (tool queries) | — | Moderate dashboard access |
| **actions.db write** | **Standard** | — | 10 (action recording) | Moderate write volume |
| **wallet_monitor.db** | **Standard** | 5-10 (wallet dashboard + balance checks) | 10 (snapshots every 60s) | Periodic snapshots, wallet page queries |
| **ohlcvs.db** | **Standard** | 20+ (chart data for dashboard + trading) | 10 (candle updates) | Chart queries can be heavy (multiple timeframes) |
| **tools.db** | **Cold** | 1-2 (only during active tool use) | 1-2 | Tools (multi-wallet, volume aggregator) used infrequently |
| **strategies.db** | **Cold** | 1-5 (strategy evaluation) | 0-1 | Small dataset, infrequent writes |
| **wallets.db** | **Cold** | 2-5 (wallet list, balance fetch) | 0-1 | Small dataset (~1-10 wallets), mostly read |
| **rpc_stats.db** | **Cold** | 0-1 (stats page rarely viewed) | 20+ (every RPC call logged) | Write-heavy but reads are rare — Cold cache is fine because writes don't benefit from read cache |
| **ai/chat.db** | **Cold** | 0-5 (chat session loading) | 0-5 | Used only when AI chat is active |
| **ai/database.db** | **Cold** | 0-2 (instruction loading) | 0-1 | Very small dataset, rare access |

**Edge cases**:
- **rpc_stats.db**: High WRITE volume but Cold preset is correct because cache_size helps READS, 
  not writes. RPC stats are append-only; the read path (stats dashboard page) is rarely accessed.
- **transactions.db**: Promoted to Hot (was Standard in some plan sections) because real-time 
  WebSocket monitoring does continuous reads during active trading.
- **ohlcvs.db**: Standard because chart queries can span multiple timeframes but the dataset 
  per token is small (~500 candles × 7 timeframes).

**Final preset-to-config mapping**:
```
Hot:      cache_size = 5,000 pages (20 MB/conn), mmap_size = 256 MB
Standard: cache_size = 2,000 pages (8 MB/conn),  mmap_size = 128 MB  
Cold:     cache_size = 500 pages (2 MB/conn),     mmap_size = 0
```

### v12 Finding 2: TokenListEntry Conversion — Complete Data Flow

The plan described the conversion but didn't explicitly show where metadata sets originate.
Here's the COMPLETE data flow verified from engine.rs source:

```
compute_snapshot() — Full Verified Flow (engine.rs lines 46-551):

Step 1 (line 46-48): Load all tokens with market data from DB
  → tokens: Vec<Token> (~171K entries, ~240MB transient)

Step 2 (line 231-244): Load metadata sets — THREE cheap queries:
  → priced_set: HashSet<String> = pools::get_available_tokens()
     (returns mints with active pool prices — in-memory lookup, ~0.1ms)
  → open_position_set: HashSet<String> = positions::get_open_mints()
     (returns mints with open positions — DB query on small table, ~1ms)
  → ohlcv_set: HashSet<String> = ohlcvs::get_mints_with_data(&candidate_mints)
     (returns mints with OHLCV data — cache check, ~5ms)

Step 3 (line 255-258): Pre-wrap all tokens in Arc<Token>
  → arc_tokens: HashMap<String, Arc<Token>>

Step 4 (line 279-303): Build TokenEntry for EVERY token:
  for (mint, token) in arc_tokens.iter() {
      let has_pool_price = priced_set.contains(&token.mint);      // O(1) set lookup
      let has_open_position = open_position_set.contains(&token.mint);
      let has_ohlcv = ohlcv_set.contains(&token.mint);
      
      token_entries.insert(mint.clone(), TokenEntry {
          token: Arc::clone(token),       // cheap Arc increment
          has_pool_price,                  // from Step 2
          has_open_position,               // from Step 2
          has_ohlcv,                       // from Step 2
          pair_created_at: Some(creation_timestamp),
          last_updated: token.market_data_last_fetched_at,
      });
  }

Step 5 (line 305-395): Evaluate filters on ALL tokens
  → passed_tokens, rejected_tokens lists built
  → uses full Token data (from Arc<Token>) for evaluation

Step 6 (line 489-513): Build FilteringSnapshot
  → FilteringSnapshot { tokens: token_entries, passed_tokens, rejected_tokens, ... }
```

**When we implement TokenListEntry (Phase C1)**, Step 4 changes to:
```rust
// Step 4 becomes: Build TokenListEntry for EVERY token
for (mint, token) in &tokens {  // tokens is Vec<Token>, not pre-Arc'd
    let has_pool_price = priced_set.contains(&token.mint);
    let has_open_position = open_position_set.contains(&token.mint);
    let has_ohlcv = ohlcv_set.contains(&token.mint);
    
    // TokenListEntry::from() has ALL inputs it needs:
    //   - &Token for all display/sort/filter fields
    //   - 3 boolean flags from metadata sets
    token_list_entries.insert(
        Arc::from(token.mint.as_str()),
        TokenListEntry::from_token(token, has_pool_price, has_open_position, has_ohlcv),
    );
}
```

**KEY ANSWER**: The `from_token()` conversion has ALL inputs available because:
1. Full Token is loaded from DB (Step 1) — all fields available
2. Metadata sets are queried BEFORE the loop (Step 2) — flags available
3. The conversion happens inside the same loop iteration — no race condition
4. After the loop, the Vec<Token> is dropped — 240MB freed

**Status flag staleness**: Between Step 2 (query sets) and Step 4 (use them), ~1-5 seconds
elapse. A position could open/close in that window. This is the SAME race that exists TODAY
in the current code. It's acceptable because:
- Flags are informational (UI badges), not trading decisions
- Next snapshot refresh (180s) corrects any staleness
- The race window is seconds, not minutes

### v12 Finding 3: Incremental Refresh + Metadata Sets Strategy

**Question**: When Phase C2 does incremental delta refresh, should metadata sets also be
refreshed incrementally or fully?

**Answer**: FULL refresh of metadata sets on every cycle. Here's why:

```
Metadata set query costs (measured from code):
  pools::get_available_tokens() → returns Vec<String> from in-memory PRICE_CACHE
    Cost: ~0.1ms, zero DB access, returns ~5K-20K mints
    
  positions::get_open_mints() → DB query on positions table
    Cost: ~1ms, tiny table (~50-200 rows)
    
  ohlcvs::get_mints_with_data() → checks in-memory OHLCV cache
    Cost: ~5ms, no DB access for cached data
    
  TOTAL: ~6ms for ALL THREE metadata sets

Compare with:
  Full token load: ~500ms-2s (171K rows from SQLite with JOINs)
  Incremental delta: ~10-50ms (100-2000 changed rows)
```

**The metadata set queries are 100× cheaper than token loading.** Making them incremental
would add complexity for ~6ms savings. Not worth it.

**Phase C2 incremental flow (UPDATED)**:
```
Every 180 seconds:
  1. Get last_refresh_timestamp from snapshot metadata
  2. Query delta tokens: WHERE updated_at > last_refresh (~10-50ms)
  3. Query new tokens: WHERE first_discovered_at > last_refresh (~5ms)
  4. Query newly blacklisted: WHERE added_at > last_refresh (~1ms)
  5. Refresh metadata sets FULLY (~6ms):
     → priced_set = pools::get_available_tokens()
     → position_set = positions::get_open_mints()
     → ohlcv_set = ohlcvs::get_mints_with_data(...)
  6. Clone existing snapshot HashMap (Arc<str> keys, ~94MB memcpy, ~5ms)
  7. For each delta token: re-evaluate + convert to TokenListEntry with FRESH flags
  8. For each new token: evaluate + convert to TokenListEntry with FRESH flags
  9. For blacklisted: remove from HashMap
  10. For ALL unchanged tokens in clone: update ONLY has_open_position flag
      (positions can open/close between refreshes — cheapest flag to update)
      → iterate 171K entries, 3 set lookups each: ~20ms
  11. Build new FilteringSnapshot, swap atomically
  
  Total per refresh: ~50-100ms (vs current ~500ms-2s)
```

**Step 10 is NEW and important**: Even if a token's market data hasn't changed, its
position/price/ohlcv status might have. The cheapest approach is refreshing all flags
for all tokens (171K × 3 lookups = ~20ms). This is still 10-50× faster than reloading
171K full Tokens from SQLite.

### v12 Finding 4: Phase B Cache Migration — Not a Runtime Race

**The explore agent flagged**: "DECIMALS_CACHE migration timing race condition with filtering"

**This is NOT a real issue.** Here's why:

The moka migration (B2) is a **code change**, not a runtime migration. The sequence:
1. Developer changes `tokens/decimals.rs`: replaces `LazyLock<HashMap>` with `LazyLock<moka::sync::Cache>`
2. Developer changes all `.insert()` and `.get()` call sites
3. Code is compiled into a new binary
4. Bot is restarted with new binary
5. On startup, the NEW moka cache starts empty and fills from DB lookups

There is **zero overlap** between old HashMap and new moka cache. The bot either runs the
old code (HashMap) or the new code (moka). Never both simultaneously.

**The only real concern**: Cold cache after restart. The DECIMALS_CACHE starts empty and
needs ~242K lookups to warm up. Each lookup falls through to DB (indexed, ~1ms each).
During warmup, token processing is slightly slower.

**Mitigation** (already implicit): DECIMALS_CACHE is populated lazily — each `get_decimals(mint)`
call that misses inserts the result. Within the first filtering cycle (~180s), most active
tokens' decimals are cached. Cold tokens are cached on first access.

**No additional constraints needed for B2.** The v12 finding downgrades this from HIGH to INFO.

### v12 Finding 5: Maintenance Service — Scheduling Pattern

The plan didn't specify how multiple periodic tasks coordinate. After reviewing 3 existing
service implementations, the **wallet balance_monitor pattern** (tokio::select! with
multiple intervals) is the correct choice.

**Maintenance Service implementation pattern** (verified against codebase conventions):

```rust
// src/maintenance/mod.rs

pub struct MaintenanceService {
    shutdown: Arc<Notify>,
}

impl Service for MaintenanceService {
    fn name(&self) -> &'static str { "maintenance" }
    fn priority(&self) -> i32 { 90 }  // After core, before webserver
    fn dependencies(&self) -> Vec<&'static str> { vec!["connectivity"] }
}

impl MaintenanceService {
    pub async fn run_loop(shutdown: Arc<Notify>) {
        // Read intervals from config
        let wal_secs = get_config_clone().maintenance.wal_checkpoint_interval_secs;
        let vacuum_hours = get_config_clone().maintenance.vacuum_interval_hours;
        
        let mut wal_interval = tokio::time::interval(
            Duration::from_secs(wal_secs.max(60))
        );
        let mut vacuum_interval = tokio::time::interval(
            Duration::from_secs(vacuum_hours.max(1) * 3600)
        );
        let mut cleanup_interval = tokio::time::interval(
            Duration::from_secs(24 * 3600)  // Daily cleanups
        );
        let mut session_interval = tokio::time::interval(
            Duration::from_secs(15 * 60)  // 15 min session cleanup
        );
        let mut metrics_interval = tokio::time::interval(
            Duration::from_secs(60)  // 1 min metrics
        );
        let mut moka_eviction_interval = tokio::time::interval(
            Duration::from_secs(60)  // 1 min — run_pending_tasks on all moka caches
        );
        
        // Skip first tick (fires immediately)
        wal_interval.tick().await;
        vacuum_interval.tick().await;
        cleanup_interval.tick().await;
        session_interval.tick().await;
        metrics_interval.tick().await;
        moka_eviction_interval.tick().await;
        
        // One-time VACUUM check (lazy, on first cycle)
        let mut one_time_vacuum_done = Self::check_vacuum_sentinel();
        let mut vacuum_db_index: usize = 0;
        
        loop {
            tokio::select! {
                _ = shutdown.notified() => {
                    logger::info(LogTag::System, "Maintenance service shutting down");
                    break;
                }
                
                _ = wal_interval.tick() => {
                    // WAL checkpoint — PASSIVE is non-blocking, safe anytime
                    Self::checkpoint_all_wal_databases().await;
                }
                
                _ = vacuum_interval.tick() => {
                    if !one_time_vacuum_done {
                        // First time: activate auto_vacuum + full VACUUM (one DB per cycle)
                        one_time_vacuum_done = Self::one_time_vacuum_next_db().await;
                    } else {
                        // Steady state: incremental VACUUM, one DB per cycle, round-robin
                        if Self::can_do_maintenance().await {
                            vacuum_db_index = Self::incremental_vacuum(vacuum_db_index).await;
                        }
                    }
                }
                
                _ = cleanup_interval.tick() => {
                    if Self::can_do_maintenance().await {
                        Self::cleanup_old_actions().await;
                        Self::cleanup_rpc_stats().await;
                        Self::cleanup_sol_flow_cache().await;
                        Self::mark_stale_tokens().await;
                    }
                }
                
                _ = session_interval.tick() => {
                    // Session cleanup is lightweight — no trade check needed
                    Self::cleanup_sessions().await;
                }
                
                _ = metrics_interval.tick() => {
                    // Collect memory metrics for dashboard
                    Self::publish_memory_metrics().await;
                }
                
                _ = moka_eviction_interval.tick() => {
                    // Force moka eviction processing for accurate metrics
                    Self::run_moka_pending_tasks();
                }
            }
        }
    }
    
    /// Check if maintenance operations are safe to run
    async fn can_do_maintenance() -> bool {
        let config = get_config_clone();
        if !config.maintenance.enabled { return false; }
        if config.maintenance.skip_during_active_trades {
            if crate::global::is_force_stopped() { return false; }
            // Check TOOLS_ACTIVE_COUNT — tools need DB access
            if crate::global::TOOLS_ACTIVE_COUNT.load(Ordering::Relaxed) > 0 { return false; }
        }
        true
    }
}
```

**Key design decisions**:
- **Single loop with tokio::select!** — prevents two heavy tasks from running simultaneously
  (only one branch executes per iteration)
- **WAL checkpoint always runs** — PASSIVE mode is non-blocking, doesn't interfere with reads
- **VACUUM and cleanups gated by can_do_maintenance()** — skips if trades active
- **moka run_pending_tasks() every 60s** — ensures accurate metrics and timely eviction
  (resolves v9 Issue #9)
- **One-time VACUUM handled lazily** — on first vacuum_interval tick, not at startup
  (resolves v9 Issue #7)
- **No concurrent heavy operations** — tokio::select! is exclusive; if VACUUM is running,
  no other heavy task starts until next loop iteration

**Concurrency guarantee**: `tokio::select!` picks ONE ready branch per iteration.
If both `vacuum_interval` and `cleanup_interval` fire at the same moment, only ONE executes
first, then the other on the next loop. This prevents two SQLite-heavy operations from
competing for locks.

### v12 Summary

| # | Finding | Severity | Resolution |
|---|---------|----------|-----------|
| 1 | Preset assignment lacks justification | ~~CRITICAL~~ → **RESOLVED** | Added full workload analysis table with reads/writes/min and rationale |
| 2 | TokenListEntry conversion data flow unclear | ~~CRITICAL~~ → **RESOLVED** | Documented complete flow: sets queried in Step 2, used in Step 4, same loop iteration |
| 3 | Metadata sets not specified for incremental | ~~HIGH~~ → **RESOLVED** | Full refresh (6ms total, 100× cheaper than token load). Added Step 10 for flag updates |
| 4 | DECIMALS_CACHE migration race condition | ~~HIGH~~ → **NOT AN ISSUE** | Code change, not runtime migration. Downgraded to INFO |
| 5 | Maintenance task scheduling unspecified | ~~HIGH~~ → **RESOLVED** | tokio::select! multi-interval pattern (matches wallet_monitor convention) |

**Reading order for implementers**: v10 (Option D architecture) → v11 (file corrections) → v12 (flow details)

---

## 🧠 v13 — Non-Database Memory Deep-Dive (DB vs Application Memory)

> v13 adds: Complete non-DB memory analysis. 31 global caches inventoried. 5 memory flows traced
> with exact allocation timelines. Token struct size corrected to ~2,200 bytes (plan estimated 1,390).
> Filtering architecture confirmed well-optimized (Arc pattern). Position cloning identified as
> biggest non-DB waste (18.9 GB/year of unnecessary allocations). 14 new gaps added to tracking.
> DB vs non-DB split quantified: ~60% DB-related, ~40% application logic.

### The Core Question: How Much Is NOT About the Database?

The v1-v12 plan correctly identified SQLite page caches as the #1 consumer. But this section
isolates and quantifies EVERY non-database memory issue to understand the full picture.

#### Memory Split at 804 MB Startup RSS

| Category | Estimated MB | % of 804 MB | Source |
|----------|-------------|-------------|--------|
| **SQLite page caches** (14 DBs warming up) | 250-350 | 31-44% | DB config |
| **SQLite mmap_size** (tokens.db + wallet.db) | 50-100 | 6-12% | DB config |
| **Filtering snapshot** (56K tokens in Arc) | 120 | 15% | Application |
| **Transient token load** (compute_snapshot peak) | 112 | 14% | Application |
| **Allocator fragmentation** (macOS system allocator) | 80-120 | 10-15% | Runtime |
| **Global in-memory caches** (31 caches combined) | 30-50 | 4-6% | Application |
| **Tokio runtime + threads + embedded assets** | 30-50 | 4-6% | Runtime |
| **Position state + indexes** | 1-5 | <1% | Application |
| **Channel buffers** | 5-10 | <1% | Application |
| **TOTAL** | ~680-920 | ~804 observed | — |

**Key insight**: ~40% of startup memory (300-340 MB) comes from application logic, NOT databases.
Even if we fix ALL SQLite issues (Phase A), we'd still see ~350-450 MB RSS from application code.

---

### Non-DB Memory Issue #1: Filtering Snapshot (120 MB steady, 232 MB peak)

**Status: Well-optimized, but large by design.**

The filtering pipeline loads 56K tokens (those with DexScreener/GeckoTerminal market data,
not all 144K tokens) every 3 minutes via `compute_snapshot()`.

**Architecture (confirmed via deep code review):**

```
compute_snapshot() flow:
  1. Load 56K Token structs from SQLite → Vec<Token> (112 MB transient)
  2. Wrap each in Arc → HashMap<String, Arc<Token>> (consumes Vec, ~112 MB)
  3. Build TokenEntry for each: { token: Arc<Token>, has_pool_price, has_open_position, has_ohlcv, ... }
  4. Evaluate filters on &Token references (zero-copy)
  5. Collect filtered_mints, passed_tokens, rejected_tokens, blacklist_reasons
  6. Build FilteringSnapshot with all the above
  7. Wrap in Arc<FilteringSnapshot> and atomically swap with old snapshot
  8. Old Arc<FilteringSnapshot> drops (refcount → 0 → freed)
```

**Memory timeline during refresh:**
- T0: Steady state — 1 Arc<FilteringSnapshot> = **120 MB**
- T1: Token load begins — new Vec<Token> = **+112 MB** (total: 232 MB)
- T2: Tokens wrapped in Arc, Vec consumed — still ~232 MB (Arc map replaces Vec)
- T3: New snapshot built, Arc wraps it — **2 snapshots briefly** (~240 MB)
- T4: Old snapshot swapped out, Arc dropped — back to **120 MB**
- Peak: **~240 MB** for ~2-3 seconds every 3 minutes

**Why it's well-optimized:**
- `Arc<Token>` in TokenEntry = 8 bytes per reference (not 2,200 byte clone)
- API endpoint `execute_query()` works with `&Token` references, only clones page items (100-200 max)
- Snapshot accessed via `Arc::clone()` (pointer increment, not deep copy)
- `passed_tokens` and `rejected_tokens` capped at `MAX_DECISION_HISTORY = 1000`

**What TokenListEntry (Phase C1) would change:**
- Replace `Arc<Token>` (2,200 bytes backing) with TokenListEntry (~550 bytes, owned)
- 56K × 550 = 30.5 MB vs 56K × 2,200 = 123 MB → **saves ~93 MB**
- PLUS eliminates the 112 MB transient token load (query only needed fields from DB)
- Net: 120 MB steady → **31 MB steady**, 240 MB peak → **62 MB peak**

---

### Non-DB Memory Issue #2: Position Cloning (18.9 GB/year wasted allocations)

**Status: Biggest non-DB WASTE — easy to fix.**

**Position struct: 42+ fields, ~600-750 bytes per instance** (incl. heap strings)

**Critical hot path — Price Updater (every 1 second):**
```rust
// price_updater.rs:38 — runs EVERY SECOND
let positions = get_open_positions().await;  // Clones ALL open positions
for position in positions {
    // Only uses position.mint for price lookup!
    match get_current_price(&position.mint).await { ... }
}
```

**Impact with 1000 open positions:**
- Clone size: 1000 × 700 bytes = **700 KB per second**
- Per day: 700 KB × 86,400 = **60.5 GB/day of allocation churn**
- Per year: **~18.9 TB** of wasteful allocation/deallocation

**Why this matters beyond raw allocation:**
- Allocator fragmentation: frequent alloc/dealloc of 700KB chunks = fragmented heap
- macOS system allocator NEVER returns pages to OS → grows RSS permanently
- jemalloc (Phase A) would mitigate but not eliminate

**Fix (simple, Phase B):**
```rust
// Instead of cloning all positions:
let mints: Vec<String> = {
    let guard = POSITIONS.read().await;
    guard.iter()
        .filter(|p| p.exit_time.is_none())
        .map(|p| p.mint.clone())
        .collect()
};
```
- Clones only mint strings: 1000 × 43 bytes = **43 KB** (16× smaller)
- Or add `get_open_mints()` function (already exists at line 610!)

**Other hot-path clone sites:**
| Caller | Frequency | What's cloned | Impact |
|--------|-----------|--------------|--------|
| `price_updater.rs:38` | Every 1s | ALL open positions | 700 KB/s ⚠️ |
| `list.rs:35` (status=all) | Per HTTP request | ALL positions | 700 KB/req |
| `list.rs:31` (status=open) | Per HTTP request | Open positions only | 350 KB/req |
| `header.rs:33` | Per dashboard refresh | Open positions | 350 KB/req |
| `exit.rs` (exit monitor) | Every check cycle | Open positions | 350 KB/cycle |
| `entry.rs` | Every check cycle | Open positions (count) | 350 KB/cycle |

---

### Non-DB Memory Issue #3: Global Cache Inventory (31 caches, 9 unbounded)

**Complete inventory of all 31 in-memory caches:**

#### 🔴 UNBOUNDED-GROWS (9 caches — never cleaned)

| # | Location | Type | Growth Rate | Est. Size After 24h |
|---|----------|------|-------------|---------------------|
| 1 | `transactions/utils.rs:58` GLOBAL_KNOWN_SIGNATURES | HashSet<String> | +1000 tx/hour × 88B | **2.1 MB/day** |
| 2 | `positions/state.rs:22` POSITION_LOCKS | HashMap<String, Arc<Mutex>> | +1 per unique mint | **~100 KB** (slow) |
| 3 | `positions/state.rs:27` PENDING_PARTIAL_EXITS | HashMap<String, u32> | Tied to swaps | **~10 KB** |
| 4 | `positions/state.rs:41` PENDING_PARTIAL_EXIT_DETAILS | HashMap<String, PendingPartialExit> | Tied to swaps | **~50 KB** |
| 5 | `positions/state.rs:47` PENDING_OPEN_SWAPS | HashMap<String, DateTime> | Tied to swaps | **~10 KB** |
| 6 | `positions/state.rs:60` PENDING_DCA_SWAPS | HashMap<String, PendingDcaSwap> | Tied to swaps | **~20 KB** |
| 7 | `tokens/decimals.rs:38` FAILED_CACHE | HashSet<String> | +failures/hour | **~500 KB** |
| 8 | `positions/verifier.rs:22` LAST_TOKEN_ACCOUNTS_CHECK | HashMap<String, DateTime> | +1 per unique mint | **~100 KB** |
| 9 | `ohlcvs/fetcher.rs:52` request_history | VecDeque<Instant> | +requests/hour | **~50 KB** |

**Total unbounded after 24h: ~3 MB** — small individually but accumulates over weeks.

#### 🟡 BOUNDED-TTL (9 caches — active cleanup)

| # | Location | TTL | Max Size |
|---|----------|-----|----------|
| 1 | `transactions/utils.rs:62` GLOBAL_PENDING_TRANSACTIONS | 180s | ~100 KB |
| 2 | `pools/cache.rs:19` PRICE_CACHE | configurable + hourly cleanup | ~5 MB |
| 3 | `pools/cache.rs:22` PRICE_HISTORY | max entries/token + gap cleanup | ~20-50 MB |
| 4 | `tokens/pool_data/cache.rs:38` TOKEN_POOLS_CACHE | 60s | ~5 MB |
| 5 | `tokens/pool_data/cache.rs:44` POOL_PREFETCH_STATE | 20s debounce | ~10 KB |
| 6 | `webserver/session.rs:23` SESSIONS | expiry + periodic cleanup | ~10 KB |
| 7 | `trader/monitors/entry.rs:23` ENTRY_CYCLE_RESERVATIONS | timeout-based | ~10 KB |
| 8 | `ohlcvs/service.rs:43` bundle_cache | 30s TTL | ~10 MB |
| 9 | `strategies/engine.rs:17` CachedEvaluation | 5s TTL | ~1 MB |

**Total bounded-TTL: ~40-70 MB** (dominated by PRICE_HISTORY and bundle_cache)

#### 🟢 BOUNDED-ACTIVE (8 caches) + TRANSIENT (2) + DB-BACKED (3)

These total ~5-15 MB combined and are well-managed.

**Grand total all non-DB caches: ~50-90 MB** (dominated by PRICE_HISTORY ~50 MB)

---

### Non-DB Memory Issue #4: Allocator Fragmentation (~100-200 MB wasted)

**Status: Root cause of "memory never goes down" on macOS.**

macOS system allocator (`malloc`) has these characteristics:
- Pages allocated from OS are NEVER returned (even after free)
- Fragmentation from mixed-size allocations grows RSS permanently
- Tokio's work-stealing creates allocation patterns across threads

**Evidence:**
- 700 KB position clones every 1 second → rapid alloc/free churn
- 112 MB token load every 3 minutes → massive alloc then free
- Multiple HashMap rehashes during startup → fragmented pages

**Fix: jemalloc (Phase A)**
- jemalloc actively returns pages to OS
- Thread-local caches reduce contention
- Configurable dirty page purging interval
- Expected savings: **80-150 MB** (conservative estimate)

---

### Non-DB Memory Issue #5: Channel Buffers (~7-15 MB)

| Channel | Buffer Size | Element Size | Max MB |
|---------|------------|--------------|--------|
| Events broadcast | 5,000 | ~500 bytes | 2.5 MB |
| Events writer mpsc | 10,000 | ~500 bytes | 5 MB |
| Actions broadcast | 1,000 | ~200 bytes | 0.2 MB |
| Telegram mpsc | 100 | ~500 bytes | 0.05 MB |
| RPC stats mpsc | 1,000 | ~100 bytes | 0.1 MB |
| Events cache VecDeque | 5,000 | ~500 bytes | 2.5 MB |

**Total: ~10 MB** — bounded and acceptable.

---

### Non-DB Memory Issue #6: Embedded Assets + Tokio Runtime (~30-50 MB)

**Embedded assets (via include_str!/include_bytes!):**
- HTML templates: ~20 templates × ~10 KB = 200 KB
- CSS files: ~15 files × ~5 KB = 75 KB
- JavaScript files: ~40 files × ~15 KB = 600 KB
- Font files (JetBrains Mono, Orbitron, Lucide): ~2 MB
- Images (logos, provider icons): ~500 KB
- lightweight-charts.js: ~200 KB
- **Total embedded: ~4 MB**

**Tokio runtime:**
- Thread pool: ~8 worker threads × ~8 KB stack = 64 KB
- Task queues, timers, IO driver: ~5-10 MB
- Pending futures (Services, WebSocket, etc.): ~5-10 MB
- **Total tokio: ~15-25 MB**

**Other runtime:**
- Rust standard library structures: ~5 MB
- Stack space for async tasks: ~5-10 MB
- **Total: ~30-50 MB** (fixed, not a problem)

---

### Summary: What Phase Fixes What (Non-DB Focus)

| Phase | What It Fixes (non-DB) | MB Saved |
|-------|----------------------|----------|
| **A (jemalloc)** | Allocator fragmentation | 80-150 MB |
| **B-early (true leaks)** | POSITION_LOCKS, ACTIVE_ACTIONS, TOKEN_2022_CACHE | ~5 MB |
| **B-early (position clone)** | Price updater 700KB/s waste → 43KB/s | Prevents ~200 MB fragmentation |
| **B-late (bounded caches)** | GLOBAL_KNOWN_SIGNATURES, FAILED_CACHE, session stores | ~5-10 MB |
| **B-late (pending swaps)** | PENDING_PARTIAL_EXIT/DCA_SWAPS eviction | ~1 MB |
| **C1 (TokenListEntry)** | Filtering snapshot 120 MB → 31 MB | **~90 MB** |
| **C1 (transient load)** | Filtering peak 240 MB → 62 MB | **~178 MB peak** |
| **D (MaintenanceService)** | Periodic cleanup of all bounded-TTL caches | ~10-20 MB |
| **D (PRICE_HISTORY cap)** | Cap per-token history entries globally | ~20-30 MB |

**Total non-DB savings: ~390-485 MB** (with all phases)

---

### 14 New Gaps Found in Gap Review (Assigned to Phases)

| Gap # | Description | Severity | Phase | Est. Impact |
|-------|-------------|----------|-------|-------------|
| 14 | Transaction struct bloat — 20+ fields, Vec<> always allocated | MEDIUM | Future | ~1 MB |
| 15 | Position deep-clone in list.rs:35 (status=all) | HIGH | B-early | 700 KB/req |
| 16 | PENDING_PARTIAL_EXIT + PENDING_DCA_SWAPS no eviction | HIGH | B-early | ~100 KB |
| 17 | Filtering batch Vec<String> (274-277) for 56K tokens | HIGH | C1 | ~5 MB transient |
| 18 | RPC stats VecDeque fragmentation (never shrinks) | MEDIUM | D | ~1 MB |
| 19 | OHLCV cache VecDeque (already bounded via LRU) | LOW | — | ~15 MB (OK) |
| 20 | Events cache VecDeque capacity (bounded 5000) | LOW | — | ~2.5 MB (OK) |
| 21 | Manual trade history Vec::drain (bounded by limit) | LOW | — | ~100 KB (OK) |
| 22 | Quote collection Vec (temporary, small) | LOW | — | ~10 KB (OK) |
| 23 | No streaming JSON for large API responses | MEDIUM | Future | ~1 MB/req |
| 24 | Telegram discovered chats Vec (typically tiny) | LOW | D | ~10 KB |
| 25 | API response text() full buffering | MEDIUM | Future | ~1 MB/call |
| 26 | Stream .collect::<Vec<_>>() in hot paths | MEDIUM | Future | Varies |
| 27 | Position list no server-side pagination | HIGH | C1/E | 700 KB/req |

---

### Corrected Token Struct Size

**Previous estimate (v1-v12):** ~1,390 bytes per Token

**Actual measurement (v13):** ~2,000-2,500 bytes per Token

The Token struct has **78 fields** including:
- ~30 String fields (24 bytes stack + heap allocation each)
- 7 Vec<> fields (24 bytes stack + heap allocation each)
- ~20 Option<f64> fields (16 bytes each)
- ~10 Option<String> fields (40 bytes each)
- Multiple nested structs (SecurityData, MarketData, etc.)

**Impact on memory estimates:**
| Metric | v12 Estimate | v13 Corrected | Delta |
|--------|-------------|---------------|-------|
| Single Token | 1,390 bytes | ~2,200 bytes | +58% |
| Snapshot (56K tokens) | 78 MB | 123 MB | +45 MB |
| Peak during refresh | 156 MB | 240 MB | +84 MB |
| TokenListEntry savings | 137 MB | **~93 MB** | Revised |
| After C1 steady state | ~42 MB | ~31 MB | Better |

Note: TokenListEntry savings are measured differently now because the baseline is higher
(123 MB → 31 MB = 92 MB saved) while the struct itself stays at ~550 bytes.

---

### v13 Summary

| # | Finding | Severity | Resolution |
|---|---------|----------|-----------|
| 1 | Non-DB memory is ~40% of total (300-340 MB) | **INFO** | Quantified — both DB and non-DB fixes needed |
| 2 | Filtering snapshot well-optimized (Arc pattern) | **GOOD NEWS** | No code change needed for current architecture |
| 3 | Position cloning wastes 18.9 GB/year allocations | **HIGH** | Fix: use get_open_mints() in price_updater (Phase B-early) |
| 4 | 9 unbounded caches total ~3 MB/day growth | **MEDIUM** | Phase B-late + D (MaintenanceService cleanup) |
| 5 | Token struct is 2,200 bytes not 1,390 | **CORRECTION** | Updated all estimates throughout plan |
| 6 | PRICE_HISTORY is largest non-DB cache (~50 MB) | **MEDIUM** | Phase D: add global cap on entries |
| 7 | 14 new gaps identified and assigned to phases | **INFO** | Tracked in gap table above |
| 8 | Allocator fragmentation confirmed ~100-200 MB | **HIGH** | Phase A: jemalloc deployment |

**Reading order**: v10 → v11 → v12 → v13 (non-DB deep-dive)
