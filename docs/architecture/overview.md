# ScreenerBot — System Overview (Architecture Map)

This document is a **high-level map** of ScreenerBot’s architecture and runtime flow.
It intentionally avoids duplicating module internals — each major subsystem has a
dedicated architecture document under `docs/architecture/`.

If you’re new to the codebase, start here, then read the module docs in the order
suggested below.

---

## Table of Contents

1. [Goals + Scope](#1-goals--scope)
2. [Doc Map (Reading Order)](#2-doc-map-reading-order)
3. [Runtime Modes](#3-runtime-modes)
4. [Core Runtime Model](#4-core-runtime-model)
5. [Main Data Flows](#5-main-data-flows)
6. [Persistence + Caching](#6-persistence--caching)
7. [Control Plane (UI + APIs)](#7-control-plane-ui--apis)
8. [Startup + Shutdown (Lifecycle)](#8-startup--shutdown-lifecycle)
9. [Cross-Cutting Conventions](#9-cross-cutting-conventions)

---

## 1. Goals + Scope

ScreenerBot is a local-first Solana DeFi trading system that:

- Discovers tokens/pools
- Enriches and filters tokens (quality / risk / scam heuristics)
- Tracks positions and market state
- Executes swaps through router backends (Jupiter / optional fallbacks)
- Exposes everything via a dashboard + APIs (and optionally Telegram)

This document covers:

- How the modules connect
- The main runtime loops and “who calls who”
- Where data lives (SQLite + caches)

It does **not** attempt to fully describe every module — the module docs are the
source of truth for those details.

---

## 2. Doc Map (Reading Order)

Recommended reading order:

1. **System control plane + lifecycle**
   - [`services.md`](services.md) — service manager, lifecycle, health/metrics caching
   - [`infrastructure.md`](infrastructure.md) — SQLite tuning+maintenance, events, actions, logging, connectivity
   - [`config.md`](config.md) — config schema, hot reload, metadata

2. **Data sources**
   - [`rpc.md`](rpc.md) — Solana RPC providers, rate limiting, circuit breaker, stats DB
   - [`apis.md`](apis.md) — external HTTP clients (price providers, third-party token data, etc.)

3. **Core trading pipeline**
   - [`tokens.md`](tokens.md) — token store + enrichment + persistence
   - [`pools.md`](pools.md) — pool discovery/decoding + price calculation inputs
   - [`transactions.md`](transactions.md) — tx monitoring + DEX detection/analyzer pipeline
   - [`filtering.md`](filtering.md) — filter chain + sources + pass/reject persistence
   - [`strategies.md`](strategies.md) — strategy definitions + evaluation model
   - [`trader.md`](trader.md) — entry/exit monitoring, orchestration, safety controls
   - [`positions.md`](positions.md) — state machine + reconciliation + persistence
   - [`ohlcvs.md`](ohlcvs.md) — candle aggregation + caching + strategy inputs
   - [`swaps.md`](swaps.md) — quote selection + swap execution with fallback

4. **User interfaces**
   - [`webserver.md`](webserver.md) — dashboard server, middleware gates, auth models
   - [`telegram.md`](telegram.md) — optional control + notifications

5. **Wallets**
   - [`wallets.md`](wallets.md) — wallet storage + monitoring

---

## 3. Runtime Modes

At runtime ScreenerBot can be used in two broad modes:

1. **GUI / local dashboard mode**
   - Webserver binds to localhost.
   - The UI is secured via a **per-run token** gate for the embedded dashboard.

2. **Headless mode**
   - Webserver can run on a configured host/port (default is typically `127.0.0.1:8080`).
   - Optional session/password/TOTP auth can be enabled.

The detailed security model (middleware ordering, allowlists, headless auth flow)
is documented in [`webserver.md`](webserver.md).

---

## 4. Core Runtime Model

ScreenerBot is built around a service manager that provides:

- Dependency ordering (topological sort)
- Priority-based startup ordering
- Reverse-order shutdown
- Health + metrics snapshot caching for non-blocking UI reads

The details and the canonical “registered services list” are maintained in
[`services.md`](services.md).

At a very high level, the system looks like this:

```text
          +---------------------+
          |    Config System    |
          +----------+----------+
                     |
                     v
  +------------------+------------------+
  |        Service Manager              |
  +--+-----------+-----------+--------+-+
     |           |           |        |
     v           v           v        v
  Tokens       Pools     Filtering   Trader
     |           |           |        |
     +-----------+-----+-----+--------+
                       |
                       v
                    Positions
                       |
                       v
                     Swaps
```

“Infrastructure” services (events, logging, connectivity, DB maintenance, RPC stats)
run alongside the trading services and are described in [`infrastructure.md`](infrastructure.md).

### 4.1 First-run, preview, and full-mode boundary

The existence and contents of `config.toml` select the boot mode:

- no config: webserver-only initialization; onboarding and setup are visible;
- skipped wallet/RPC: preview discovery services and the dashboard are usable;
- configured wallet/RPC: the normal full runtime and all enabled services start.

Preview-to-full setup always crosses a graceful process boundary. Credentials are validated and
saved first, services stop in normal reverse order, the process lock is released, and the new
process enters the existing full boot path. Electron owns/relaunches GUI backends; headless mode
replaces or relaunches itself. Browser clients prove the new instance through `/api/health` before
reloading.

---

## 5. Main Data Flows

This section describes the **primary end-to-end flows** through the system.
Exact code paths and invariants live in the module docs.

### 5.1 Token lifecycle (discover → enrich → filter)

High level:

```text
Token discovery / ingestion
  -> Tokens module persists core token rows (tokens.db)
  -> Tokens module enriches (decimals, metadata, market/security sources)
  -> Filtering pipeline evaluates (multi-source) and persists pass/reject/blacklist
  -> UI consumes filtered token views and per-token details
```

Docs:

- Tokens store + enrichment: [`tokens.md`](tokens.md)
- Filtering chain + sources: [`filtering.md`](filtering.md)

### 5.2 Pool lifecycle (discover → fetch → calculate → analyze)

High level:

```text
Pool discovery (API + on-chain signals)
  -> pool_fetcher pulls accounts in batches from RPC
  -> pool_calculator derives prices + liquidity signals
  -> pool_analyzer classifies pools and emits structured signals
  -> tokens / trader consume pool-derived pricing
```

Docs:

- [`pools.md`](pools.md)
- [`rpc.md`](rpc.md)

### 5.3 Trading lifecycle (signal → entry → swap → position)

High level:

```text
Strategies produce entry/exit signals
  -> Trader selects candidates and applies risk/safety gates
  -> Swap router obtains best quote + executes swap
  -> Positions module persists position state + reconciliation
  -> UI shows positions + PnL + actions/progress
```

Docs:

- [`strategies.md`](strategies.md)
- [`trader.md`](trader.md)
- [`swaps.md`](swaps.md)
- [`positions.md`](positions.md)

---

## 6. Persistence + Caching

ScreenerBot uses **SQLite** as the durable persistence layer and **bounded in-memory
caches** for hot-path performance.

Key characteristics:

- Many modules own a dedicated `*.db` file in the app data directory.
- Each DB is configured with module-specific PRAGMAs via the shared DB configurator.
- A background maintenance task ensures WAL + incremental vacuum behavior stays stable.

Docs:

- DB PRAGMAs + maintenance: [`infrastructure.md`](infrastructure.md)

At a category level, the system persists:

- **Market and token state**: tokens, pools, OHLCVs
- **Execution and tracking**: actions, events, transactions, positions
- **Operations**: rpc_stats (provider health, rate limiting, call volume)
- **Wallet state**: wallets (keys) + wallet (balances/snapshots)

The canonical per-database list, schemas, and retention policies are maintained
in the owning module docs.

---

## 7. Control Plane (UI + APIs)

ScreenerBot exposes control and observability through:

- **Webserver**: dashboard UI + REST APIs + SSE streams
- **Telegram** (optional): commands + notifications
- **CLI**: run modes, debug flags, host/port overrides, etc.

Docs:

- [`webserver.md`](webserver.md)
- [`telegram.md`](telegram.md)

---

## 8. Startup + Shutdown (Lifecycle)

At a high level:

1. Parse CLI + load config (TOML)
2. Initialize infrastructure (logger, DB configurator, global singletons)
3. Register all enabled services with the service manager
4. Start services in dependency+priority order
5. Run until shutdown is requested (signal / UI / control-plane request)
6. Stop services in reverse order with timeouts and best-effort cleanup

Docs:

- Service manager behavior: [`services.md`](services.md)
- Shutdown caveats for webserver: [`webserver.md`](webserver.md)

---

## 9. Cross-Cutting Conventions

These are “project-wide” architectural rules that appear in multiple modules:

- **Configuration is the single source of truth**: read via config helpers; do not hardcode runtime behavior.
- **Bounded memory**: caches must have explicit caps/TTLs; avoid unbounded `HashMap` growth.
- **Persistence before convenience**: the DB is the durable source; in-memory caches are optimizations.
- **Best-effort observability**:
  - events and actions are recorded asynchronously
  - failure to record should not crash the bot (but should be logged)
- **Connectivity-aware behavior**: external API / RPC calls should be gated and retried sanely.

For concrete implementations and patterns, refer to:

- [`infrastructure.md`](infrastructure.md)
- [`services.md`](services.md)
