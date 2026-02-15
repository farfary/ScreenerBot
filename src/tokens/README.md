<p align="center">
  <img src="../../public/logo.png" alt="ScreenerBot Logo" width="80">
</p>

<h1 align="center">Tokens Module</h1>

<p align="center">
  <strong>Unified Token Data Management for Native Solana Trading</strong>
</p>

<p align="center">
  Unified system for discovery, market data aggregation, security analysis, pool tracking, caching, and lifecycle management for Solana SPL tokens.
</p>

---

## Architecture Overview

<p align="center"><strong>Token Service Pipeline and Data Orchestration</strong></p>

```text
┌─────────────────────────────────────────────────────────────────────────────┐
│                         TokensService (tokens)                              │
│                       (service.rs — priority 40)                            │
│           Dependencies: Events, Transactions, Native Pool Service           │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌────────────────┐    ┌─────────────────┐    ┌───────────────────────────┐  │
│  │   Discovery    │    │   Update Loop   │    │      Cleanup Loop         │  │
│  │  (60s cycle)   │    │   (priority)    │    │       (hourly)            │  │
│  │  6 providers   │    │     7 tiers     │    │   authority monitoring    │  │
│  └──────┬─────────┘    └──────┬──────────┘    └──────────┬────────────────┘  │
│         │                     │                          │                  │
│  ┌──────▼─────────────────────▼──────────────────────────▼────────────────┐  │
│  │                 RateLimitCoordinator (refill 60s)                      │  │
│  │  DexScreener: 300/min   GeckoTerminal: 30/min   Rugcheck: 60/min       │  │
│  └──────┬─────────────────────┬──────────────────────────┬────────────────┘  │
│         │                     │                          │                  │
│  ┌──────▼─────────────────────▼──────────────────────────▼────────────────┐  │
│  │                       In-Memory Caches (store.rs)                      │  │
│  │  DexScreener: 30s TTL   GeckoTerminal: 60s TTL   Rugcheck: 1800s TTL   │  │
│  │  Token Snapshots: 30s   Pool Snapshots: 60s (with stale fallback)      │  │
│  │  Decimal Cache: Unlimited, preloaded from database at startup          │  │
│  └──────┬─────────────────────────────────────────────────────────────────┘  │
│         │                                                                    │
│  ┌──────▼─────────────────────────────────────────────────────────────────┐  │
│  │                TokenDatabase (SQLite — data/tokens.db)                 │  │
│  │  10 tables, 33 indexes, 94 data access methods                         │  │
│  │  High-performance async wrappers via tokio::spawn_blocking             │  │
│  └─────────────────────────────────────────────────────────────────────────┘  │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## File Structure

<p align="center"><strong>Core Files and Component Distribution</strong></p>

```text
src/tokens/
├── mod.rs              Module root — public API re-exports
├── types.rs            Core domain types (Token, errors, data bundles)
├── database.rs         Unified SQLite operations (94 methods)
├── schema.rs           Database schema (10 tables, 33 indexes)
├── updates.rs          Priority-based background update loops
├── discovery.rs        Multi-source token discovery pipeline
├── store.rs            In-memory TTL caches (4 cache types)
├── decimals.rs         Decimal lookup with 3-tier fallback
├── favorites.rs        User-managed token favorites with notes
├── search.rs           Unified cross-API token search
├── cleanup.rs          Authority-based auto-blacklisting
├── filtered.rs         Filtering engine result storage
├── service.rs          ServiceManager integration
├── priorities.rs       7-level priority enum
├── events.rs           Internal event bus
├── market/             Market data fetchers
│   ├── mod.rs          Re-exports
│   ├── dexscreener.rs  DexScreener fetcher implementation
│   └── geckoterminal.rs GeckoTerminal fetcher implementation
├── security/           Security analysis fetchers
│   ├── mod.rs          Re-exports
│   └── rugcheck.rs     Rugcheck analyst implementation
└── pools/              Advanced pool analysis engine
    ├── mod.rs          Re-exports
    ├── api.rs          Multi-source pool fetching
    ├── cache.rs        Pool snapshot caching (60s TTL, 8 prefetch workers)
    ├── conversion.rs   API → TokenPoolInfo mapping
    ├── operations.rs   Pool merging and deduplication
    └── utils.rs        Parsing and metrics helpers
```

---

## Database Schema

<p align="center"><strong>SQLite Storage Layout (data/tokens.db)</strong></p>

### Tables (10)

| Table | Primary Key | Purpose | Foreign Key |
| :--- | :--- | :--- | :--- |
| **tokens** | `mint` | Core metadata (symbol, decimals, etc.) | — |
| **market_dexscreener** | `mint` | Price, volume (5m-24h), liquidity | → tokens |
| **market_geckoterminal**| `mint` | Prices, pools, reserve data | → tokens |
| **token_pools** | `(mint, address)` | Aggregated multi-source raw pool data | → tokens |
| **security_rugcheck** | `mint` | Scores, authorities, risk flags | → tokens |
| **update_tracking** | `mint` | Priorities, error counts, timestamps | → tokens |
| **blacklist** | `mint` | Authority-based and manual blocks | — |
| **token_favorites** | `id` | User-saved symbols with notes | — |
| **rejection_history** | `id` | Detailed event log of filtered tokens | — |
| **rejection_stats** | `(bucket, reason)` | Hourly aggregated filtering analytics | — |

### Indexes (33)

Optimized for: discovery time, blockchain creation, metadata fetch, symbol lookup (case-insensitive), liquidity ranking, fetch timestamps, priority+market composites, error types, favorites, rejection time ranges, and hourly aggregation buckets.

---

## Core Types

<p align="center"><strong>Domain Entities and Data Aggregations</strong></p>

### Token (Primary Entity)

The **Token** struct provides a unified view assembled from multiple high-performance storage tables.

| Category | Fields |
| :--- | :--- |
| **Identity** | `mint`, `symbol`, `name`, `decimals`, `description`, `image_url`, `header_image_url`, `supply` |
| **Pricing** | `price_usd`, `price_sol`, `price_native`, `price_change_m5/h1/h6/h24` |
| **Market** | `market_cap`, `fdv`, `liquidity_usd`, `reserve_in_usd`, `pool_count` |
| **Volume** | `volume_m5/h1/h6/h24` |
| **Transactions** | `txns_m5/h1/h6/h24_buys/sells` (8 fields with helper totals) |
| **Security** | `security_score`, `is_rugged`, `mint_authority`, `freeze_authority`, `security_risks`, `top_holders`, `transfer_fee_pct` |
| **Bot State**| `is_blacklisted`, `priority: Priority`, `data_source` |
| **Timestamps** | `first_discovered_at`, `market_data_last_fetched_at`, `pool_price_last_calculated_at` |

---

## Priority System

<p align="center"><strong>Refresh Intervals for Native Solana Market Dynamics</strong></p>

A 7-level priority system governs how frequently token data is synchronized from external APIs and on-chain sources:

| Priority | Level | Interval | Trigger |
| :--- | :--- | :--- | :--- |
| **OpenPosition** | 100 | **5s** | Active trading position is open |
| **PoolTracked** | 75 | **7s** | Token is being tracked by Pool Service |
| **FilterPassed** | 60 | **8s** | Token passed all filtering criteria |
| **Uninitialized** | 55 | **10s** | New discovery, no existing market data |
| **Stale** | 40 | **15s** | Market data exceeded TTL |
| **Standard** | 25 | **20s** | Default state for healthy tokens |
| **Background** | 10 | **30s** | Oldest tokens, rotation-based refresh |

---

## Update Loop Architecture

<p align="center"><strong>Multi-Threaded Priority Synchronization</strong></p>

The update system executes **7 concurrent background loops**, each optimized for specific data types and priority tiers:

```text
┌─────────────────────────────────────────────────────────────────────────────┐
│                            start_update_loop()                              │
│                      Returns Vec<JoinHandle<()>>                            │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  Loop 1: Security Data         (Configurable interval)                      │
│    ├─ 1 token/cycle, exponential backoff on errors                          │
│    └─ Persistent 404/400 handling for unknown tokens                        │
│                                                                             │
│  Loop 2: Uninitialized         (10s interval)                               │
│    └─ 30 tokens/batch, auto-exclude after 3 failures                        │
│                                                                             │
│  Loop 3: Pool-Synced           (7s interval)                                │
│    └─ 90 tokens max, auto-demote on inactivity                              │
│                                                                             │
│  Loop 4: Open Positions        (5s interval)                                │
│    └─ 200 tokens max, highest refresh priority                              │
│                                                                             │
│  Loop 5: Filter Passed         (8s interval)                                │
│    └─ 60 tokens max, critical for active monitoring                         │
│                                                                             │
│  Loop 6: Background            (30s interval)                               │
│    └─ Rotation of oldest tokens to prevent stale data                       │
│                                                                             │
│  Loop 7: Pool Priority Sync    (5m interval)                                │
│    └─ Synchronizes data with the core Pool Service                          │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Discovery Pipeline

<p align="center"><strong>Autonomous 60-second Multi-Source Discovery Loop (6 Providers, 16 Endpoints)</strong></p>

```text
Sources (Parallel Fetching):
  ├─ DexScreener: Latest Profiles, Latest Boosts, Top Boosts
  ├─ GeckoTerminal: New Pools, Recently Updated, Trending
  ├─ Rugcheck: New Tokens, Recent, Trending, Verified
  ├─ Jupiter: Recent, Top Organic, Top Traded, Top Trending
  ├─ CoinGecko: Solana Markets Extraction
  └─ DefiLlama: Protocol Address Extraction

Pipeline Logic:
  Raw Candidates
    │
    ├─ normalize_mint() ─────► Validate Base58 (32-44 chars)
    │
    ├─ filter_sol_only() ────► Strip stablecoins and non-SOL pairs
    │
    ├─ deduplicate() ────────► unique_mints only
    │
    ├─ skip_blacklist() ─────► O(1) database check
    │
    └─ finalize_pipeline() ──► Insert DB + Emit TokenDiscovered Event
```

---

## Caching Architecture

<p align="center"><strong>6-Tier High-Performance Memory Caching</strong></p>

| Cache | TTL | Backed By | Pre-loaded |
| :--- | :--- | :--- | :--- |
| **DexScreener** | 30s | `market_dexscreener` table | No |
| **GeckoTerminal** | 60s | `market_geckoterminal` table | No |
| **Rugcheck** | 30m | `security_rugcheck` table | No |
| **Token Snapshots** | 30s | Assembled from all tables | No |
| **Pool Snapshots** | 60s | `token_pools` table | No |
| **Decimals** | ∞ | `tokens` table | **Yes** (Startup) |

---

## Pool Data Pipeline

<p align="center"><strong>Real-time Native Solana Price Discovery</strong></p>

```text
get_snapshot(mint)
  │
  ├─ Check Pool Cache (60s TTL)
  │    ├─ Fresh Snapshot ─────► Return immediately
  │    └─ Stale/Miss ─────────► fetch_from_sources()
  │
  ├─ Parallel Source Fetching:
  │    ├─ DexScreener.fetch_token_pools()
  │    └─ GeckoTerminal.fetch_pools()
  │
  ├─ Data Normalization:
  │    ├─ ingest_pool_entry() ──► Merge by address
  │    ├─ sort_pools() ─────────► Native SOL pairs prioritized
  │    └─ choose_canonical() ───► Highest liquidity selection
  │
  └─ Finalization:
       └─ Persist to DB + Cache snapshots
```

---

## Market Data Fetchers

<p align="center"><strong>External Data API Integrations</strong></p>

| Provider | Endpoint | Batch Size | Rate Limit | Focus |
| :--- | :--- | :--- | :--- | :--- |
| **DexScreener** | `/tokens/v1` | 30 Tokens | 300/min | Live prices, multi-timeframe volume |
| **GeckoTerminal** | `/networks/solana/` | 30 Tokens | 30/min | Reserve monitoring, pool counting |
| **Rugcheck** | Report API | 1 Token | 60/min | Authority checks, risk score, fees |

---

## Token Lifecycle

<p align="center"><strong>From Native Discovery to Trading Execution</strong></p>

```text
       ┌──────────────┐
       │   Discovery  │ (60s cycle, 6 providers)
       └──────┬───────┘
              │
       ┌──────▼───────┐
       │   tokens DB  │ Priority = FilterPassed (60)
       │  + tracking  │
       └──────┬───────┘
              │
   ┌──────────┼──────────┐
   │          │          │
┌──▼───┐   ┌──▼───┐   ┌──▼─────┐
│ DEX  │   │ GECKO│   │ RUG    │
│ 30s  │   │ 60s  │   │ 30m    │
└──┬───┘   └──┬───┘   └──┬─────┘
   │          │          │
┌──▼──────────▼──────────▼───┐
│       get_full_token()     │
│ Assembles complete object  │
└─────────────┬──────────────┘
              │
   ┌──────────┼──────────┐
   │          │          │
┌──▼───┐   ┌──▼───┐   ┌──▼────┐
│Filter│   │ Dash │   │Trade  │
│Engine│   │Board │   │Engine │
└──────┘   └──────┘   └───────┘
```

---

## Blacklist & Security

<p align="center"><strong>Automated Risk Mitigation for Solana Mints</strong></p>

- **Automatic Authority Checks**: Scans for Mint Authority (unlimited supply risk) and Freeze Authority (lock risk).
- **Manual Control**: Global `blacklist_token()` API for user-defined exclusions.
- **Persistence**: O(1) lookups during discovery and trading pipelines.

---

## Search & Favorites

<p align="center"><strong>Unified Data Access and User Management</strong></p>

- **Cross-API Search**: Unified interface for DexScreener, GeckoTerminal, and on-chain lookup.
- **Smart Persistence**: Discovered tokens are automatically indexed for future low-latency retrieval.
- **Favorites**: SQLite-backed user notes and personalized watchlists.

---

## Event Architecture

<p align="center"><strong>Internal Reactive Token Bus</strong></p>

Native events allow other modules (Filtering, Trading, Dashboard) to react to token changes in real-time:

- `TokenDiscovered`: New mint detected on-chain or via APIs.
- `TokenUpdated`: Significant price or volume shift recorded.
- `DecimalsUpdated`: On-chain decimal metadata synchronized.
- `TokenBlacklisted`: Risk threshold reached or manual block applied.
- `TokenUnblacklisted`: Token removed from blacklist (manual or re-evaluated).

---

## Service Integration

<p align="center"><strong>Service Architecture</strong></p>

The service uses a two-layer design:

- **`TokensService`** (in `services/implementations/tokens_service.rs`) — Registered with ServiceManager as `"tokens"`. Thin wrapper that delegates to the orchestrator.
- **`TokensServiceNew`** (in `tokens/service.rs`) — Internal orchestrator managing DB init, cache setup, update loops, discovery, and cleanup.

| Property | Value |
| :--- | :--- |
| **Name** | `tokens` |
| **Priority**| 40 (Post-Infra, Pre-Webserver) |
| **Depends** | `events`, `transactions`, `pools` |

---

## Rejection Analytics

<p align="center"><strong>Multi-Layer Filtering Insights</strong></p>

- **High-Resolution Log**: `rejection_history` records every individual filtering event for deep forensic analysis.
- **Aggregated Metrics**: `rejection_stats` provides hourly buckets for instant dashboard reporting (O(1) query time).
- **Auto-Cleanup**: Configurable retention policies prevent database bloat while maintaining historical records.

---

## Key Design Decisions

<p align="center"><strong>Architectural Principles for Performance and Reliability</strong></p>

1.  **Consolidated Storage**: All SQL operations are centralized in `database.rs`, ensuring a single source of truth for schema interactions.
2.  **Non-Blocking I/O**: Multi-threaded database access via `tokio::spawn_blocking` prevents thread starvation.
3.  **Atomic Batches**: Large-scale updates use SQLite transactions to minimize lock contention and maximize throughput.
4.  **Priority-First Scheduling**: Trading-critical tokens (open positions) are prioritized over background discovery tasks.
5.  **Failure Awareness**: Intelligent tracking of permanent errors (e.g., 404s) prevents wasted API calls on non-existent mints.
6.  **Global Synchronization**: All background tasks integrate with the `TOOLS_ACTIVE` flag to prevent resource contention during intensive tool execution.

---

## Public API Summary

<p align="center"><strong>Interface for Native Solana Token Operations</strong></p>

```rust
// Core Trading Operations
pub async fn request_immediate_update(mint) -> TokenResult<UpdateResult>

// Lifecycle & Persistence
pub use Token, DataSource, Priority, SecurityLevel
pub use get_cached_token, store_token_snapshot, get_full_token_async

// Advanced Data Pipelines
pub use get_snapshot, get_snapshot_allow_stale, prefetch, fetch_immediate
pub use get_passed_tokens, get_rejected_tokens, get_blacklisted_tokens

// User Interaction
pub use search_tokens, add_favorite_async, get_favorites_async
```

---

<p align="center">
  Built for <strong>ScreenerBot</strong> — The ultimate Native Solana trading experience.
</p>
