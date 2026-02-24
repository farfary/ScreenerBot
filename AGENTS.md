# AI Agent Instructions for ScreenerBot

> **This file is for AI coding agents** (an LLM provider, Cursor, Windsurf, Cline, etc.) that assist contributors working on ScreenerBot. If you are an AI agent, follow these instructions before taking any action.

> **IMPORTANT: This file contains INSTRUCTIONS ONLY — not development history.** Do not write implementation logs, phase completion notes, commit histories, or "what was done" narratives here. This file tells agents HOW to work, not WHAT was done. Development history belongs in `docs/investigations/` or session files. When updating this file, add only: rules, pitfalls, file locations, patterns, and architectural guidance.

## Before You Do Anything

**MANDATORY: Read the docs first.**

Before modifying code, creating files, researching, reviewing pull requests, writing issues, or taking any action in this repository:

1. **Read [`docs/README.md`](docs/README.md)** — Understand the documentation structure and what's available.
2. **Read the relevant architecture doc** in `docs/architecture/` for the system you're working on.
3. **Check `docs/investigations/`** for any past deep-dives related to your task.
4. **Read [`CONTRIBUTING.md`](CONTRIBUTING.md)** for code style and contribution guidelines.

Only after reading the relevant docs should you look at source code files.

## Start + End Doc Review (MANDATORY)

For **every task** (bug fix, feature, refactor, performance work), treat architecture docs as a first-class artifact.

### At the start of the task

1. Identify which modules are affected (tokens/filtering/pools/trader/positions/transactions/webserver/etc).
2. Read the relevant `docs/architecture/*.md` docs for those modules (and `docs/architecture/overview.md` if it is a cross-cutting change).
3. Decide what *must* change in docs if the code changes (capture this mentally; do not start coding blind).

### At the end of the task (before saying "done")

1. Re-read the same architecture docs and update them to match the new behavior (no guessing).
2. Update `docs/README.md` line counts for every modified/added architecture doc:
   ```bash
   wc -l docs/architecture/*.md | sort -n
   ```
3. Enforce the hard rule: **max 2000 lines per architecture doc** (split if necessary).
4. Never use `...` placeholders in architecture docs (file trees, module lists, route module lists) to hide undocumented modules — always enumerate every entry or split docs.

## Repository Structure

```
ScreenerBot/
├── src/                    # Rust source code (trading engine)
│   ├── config/             # Macro-driven configuration system
│   ├── pools/              # DEX pool discovery, decoding, pricing
│   ├── tokens/             # Token database, market data, security
│   ├── filtering/          # Token quality filtering pipeline
│   ├── trader/             # Automated + manual trading
│   ├── swaps/              # Multi-DEX swap routing
│   ├── positions/          # Position management, P&L tracking
│   ├── strategies/         # Condition-based trading strategies
│   ├── transactions/       # Real-time transaction monitoring
│   ├── ohlcvs/             # OHLCV candlestick data
│   ├── wallets/            # Wallet balance monitoring
│   ├── ai/                 # LLM-powered analysis + chat
│   ├── telegram/           # Telegram bot integration
│   ├── webserver/          # Axum REST API + embedded dashboard
│   ├── services/           # ServiceManager with dependency resolution
│   ├── apis/               # External API clients
│   ├── rpc/                # Solana RPC with multi-provider support
│   ├── connectivity/       # Endpoint health monitoring
│   ├── events/             # Structured event logging
│   ├── errors/             # Error types
│   └── global.rs           # Global state, startup flags, constants
├── electron/               # Electron shell (desktop app wrapper)
├── crates/                 # Local Rust crate dependencies
├── docs/                   # Technical documentation
│   ├── architecture/       # Living system docs (kept current)
│   ├── development/        # Build guides, CLI reference
│   └── investigations/     # Historical deep-dive analyses
├── assets/                 # Build assets
├── icons/                  # App icons
├── Cargo.toml              # Rust dependencies
├── package.json            # Electron dependencies
├── screenerbot.sh          # Linux/macOS installer script
└── AGENTS.md               # This file
```

## Data Folder Paths

ScreenerBot stores runtime data (config, databases, logs, cache) in a platform-specific data directory.

### Resolution Order (from `src/paths.rs`)
1. `dirs::data_local_dir()` — primary
2. `dirs::data_dir()` — fallback
3. `dirs::home_dir()` — final fallback

### Platform Paths
| Platform | Path |
|----------|------|
| macOS | `~/Library/Application Support/ScreenerBot/` |
| Windows | `%LOCALAPPDATA%\ScreenerBot\` |
| Linux | `$XDG_DATA_HOME/ScreenerBot/` (fallback: `~/.local/share/ScreenerBot/`) |

### Directory Structure
```
ScreenerBot/
├── data/                          # Config + databases
│   ├── config.toml               # Main configuration
│   ├── tokens.db                 # Token metadata + cache (largest DB, ~264MB)
│   ├── ohlcvs.db                 # OHLCV price data (90-day retention)
│   ├── rpc_stats.db              # RPC call statistics (72h retention)
│   ├── pools.db                  # Liquidity pool data
│   ├── transactions.db           # Transaction history
│   ├── positions.db              # Trading positions
│   ├── wallet.db                 # Wallet state
│   ├── events.db                 # System events
│   ├── strategies.db             # Trading strategies
│   ├── actions.db                # Action history
│   ├── tools.db                  # Tool state
│   ├── ai.db                     # AI analysis data
│   ├── ai_chat.db                # AI chat history
│   └── cache_pool/               # Pool discovery cache files
├── logs/
│   └── latest.log                # Current session log
└── analysis-exports/             # Exported analysis data
```

### Accessing in Code
```rust
// Base data directory
let base = screenerbot::paths::get_data_directory();

// Specific database paths
let tokens_db = screenerbot::paths::get_tokens_db_path();
let pools_db = screenerbot::paths::get_pools_db_path();
// ... etc for each database

// Ensure all directories exist
screenerbot::paths::ensure_all_directories().ok();
```

### For Debug Binaries
Always init paths before accessing data:
```rust
screenerbot::paths::ensure_all_directories().ok();
screenerbot::config::load_config().expect("Failed to load config");
```

## Architecture Quick Reference

| System | Entry Point | Doc |
|--------|------------|-----|
| Configuration | `src/config/` — `config_struct!` macro | [docs/architecture/config.md](docs/architecture/config.md) |
| Tokens | `src/tokens/service.rs` — lifecycle, caching, DB | [docs/architecture/tokens.md](docs/architecture/tokens.md) |
| Token Filtering | `src/filtering/engine.rs` | [docs/architecture/filtering.md](docs/architecture/filtering.md) |
| Pool Discovery | `src/pools/service.rs` — pricing, decoders, swap | [docs/architecture/pools.md](docs/architecture/pools.md) |
| Swap Execution | `src/swaps/router.rs` | [docs/architecture/swaps.md](docs/architecture/swaps.md) |
| Trading Engine | `src/trader/monitors/` (entry.rs, exit.rs) | [docs/architecture/trader.md](docs/architecture/trader.md) |
| Strategies | `src/strategies/engine.rs` | [docs/architecture/strategies.md](docs/architecture/strategies.md) |
| OHLCV Data | `src/ohlcvs/monitor.rs` | [docs/architecture/ohlcvs.md](docs/architecture/ohlcvs.md) |
| Positions | `src/positions/state.rs` | [docs/architecture/positions.md](docs/architecture/positions.md) |
| Transactions | `src/transactions/` — send, confirm, parse | [docs/architecture/transactions.md](docs/architecture/transactions.md) |
| RPC | `src/rpc/client.rs` — multi-provider, circuit breaker | [docs/architecture/rpc.md](docs/architecture/rpc.md) |
| External APIs | `src/apis/manager.rs` — DexScreener, GeckoTerminal, etc. | [docs/architecture/apis.md](docs/architecture/apis.md) |
| Wallets | `src/wallets/manager.rs` — encryption, multi-wallet | [docs/architecture/wallets.md](docs/architecture/wallets.md) |
| Service Manager | `src/services/mod.rs` — lifecycle, dependencies | [docs/architecture/services.md](docs/architecture/services.md) |
| Dashboard | `src/webserver/` — Axum, 200+ endpoints | [docs/architecture/webserver.md](docs/architecture/webserver.md) |
| Telegram | `src/telegram/` — notifications, commands | [docs/architecture/telegram.md](docs/architecture/telegram.md) |
| Infrastructure | database, errors, events, logger, connectivity, actions | [docs/architecture/infrastructure.md](docs/architecture/infrastructure.md) |
| System Overview | Full system reference | [docs/architecture/overview.md](docs/architecture/overview.md) |

## Coding Conventions

### Rust

- **Config access**: Always use `with_config()` or `get_config_clone()` — never hardcode values.
- **Config sections**: Must use `config_struct!` macro (see `src/config/macros.rs`).
- **Database**: SQLite via rusqlite + r2d2. Use `with_init()` for PRAGMA settings.
- **Error handling**: Use `crate::Error` + `crate::Result<T>` from `src/errors/`. All errors must use explicit domain variants (no implicit String/&str conversions).
- **Logging**: Use `error!()`, `warning!()`, `info!()`, `debug!()`, `verbose!()` with `LogTag`.
- **Services**: ALL Service trait implementations MUST live in `src/services/implementations/`. Business logic belongs in domain modules (e.g., `connectivity/checker.rs`, `filtering/background.rs`, `ai/scheduled_worker.rs`).
- **Global state**: Check `src/global.rs` for startup flags before accessing services.

### Dashboard (Embedded HTML/CSS/JS)

- **Templates**: `src/webserver/templates/` — HTML with embedded Rust data.
- **Assets**: Embedded via `include_str!`/`include_bytes!` in `src/webserver/embeds.rs`.
- **New JS/CSS files** require: entry in `embeds.rs` + match arm in `asset_serving.rs` + style array in `templates.rs`.
- **Toggles**: Use `.toggle` class from `form_controls.css` — never create custom toggle CSS.
- **Z-index**: Use CSS variables from `floating.css` (`--z-dropdown`, `--z-dialog`, etc.) — never hardcode.
- **Icon buttons**: Use `.btn-icon` class from `components.css`.

### Git

- Small, focused commits with descriptive messages.
- Run `cargo build` before committing Rust changes.
- Run `cargo clippy` to check for warnings.

## Issue Reporting Standards

When creating issues, include:

1. **Environment**: OS, ScreenerBot version, Rust version.
2. **Steps to reproduce**: Exact sequence of actions.
3. **Expected behavior**: What should happen.
4. **Actual behavior**: What actually happens.
5. **Logs**: Relevant log output (from `~/Library/Application Support/screenerbot/logs/latest.log` on macOS, or `~/.local/share/screenerbot/logs/latest.log` on Linux).
6. **Related docs**: Link to relevant docs in `docs/` if applicable.

## Pull Request Standards

When submitting PRs:

1. **Reference the relevant architecture doc** that describes the system you're changing.
2. **Describe what changed and why** — not just "fixed bug".
3. **If adding a new system**: Create an architecture doc in `docs/architecture/`.
4. **Architecture doc max length**: 2000 lines per file. No exceptions.
5. **If doing deep analysis**: Create an investigation folder in `docs/investigations/YYYY-MM-topic/`.
5. **Update `docs/README.md`** index if you add new docs.
6. **No sensitive content**: No API keys, tokens, passwords, server IPs, or deployment configs.
7. **Build passes**: `cargo build --release` must succeed.

## Security Rules

- **Never commit**: API keys, private keys, wallet seeds, server IPs, deployment configs, passwords.
- **Public constants are OK**: Jupiter referral account, donation address (these are intentionally public).
- **Scan before commit**: `git grep -i 'api_key\|secret\|password\|private_key\|seed_phrase'`

## Documentation Rules

| Category | Location | When to Update |
|----------|----------|---------------|
| How a system works | `docs/architecture/` | When system behavior changes |
| How to build/develop | `docs/development/` | When build process or tools change |
| Deep analysis/investigation | `docs/investigations/YYYY-MM-topic/` | After completing an investigation |
| This index | `docs/README.md` | When adding any new doc |

### Architecture docs are living documents

If you change how a system works, **update the architecture doc**. Outdated docs are worse than no docs.

### Investigation docs are immutable

Investigation folders are historical records. They describe what was found at that point in time. **Never modify** past investigations — create a new one if doing follow-up work.

### Naming conventions

- Folders: `kebab-case` (e.g., `2026-02-memory`)
- Files: `kebab-case.md` (e.g., `filtering.md`, `cli-reference.md`)
- Investigation folders: `YYYY-MM-topic` format

---

## Architecture Overview

This section provides detailed descriptions of every major module in the ScreenerBot codebase. Use this as your primary reference when working on any part of the system.

### Config (src/config/)

Macro-driven via `config_struct!`. Schema in `schemas/` with embedded defaults; values live in `data/config.toml`. Access: `with_config()` (sync) or `get_config_clone()` (async). Hot-reloadable. Metadata system for UI generation. Never hardcode. Files: `macros.rs` (config_struct! macro), `schemas/` (all config structures), `utils.rs` (load/reload/access), `metadata.rs` (UI metadata). Key schemas: `trader.rs`, `positions.rs`, `filtering.rs`, `swaps.rs`, `tokens.rs`, `rpc.rs`, `sol_price.rs`, `summary.rs`, `events.rs`, `webserver.rs`, `services.rs`, `monitoring.rs`, `ohlcv.rs`, `ai.rs`, `telegram.rs`, `gui.rs` (GuiConfig: zoom_level, DashboardConfig with interface/startup/navigation/lockscreen), `holder_watch.rs` (HolderWatchConfig: holder monitoring with thresholds and notifications), `wallet.rs` (dashboard metrics intervals, API cache TTL), `performance.rs` (PerformanceConfig: memory_profile, cache sizing, filter limits — Phase A structural only, Phase D wiring), `maintenance.rs` (MaintenanceConfig: retention periods, vacuum/checkpoint intervals — Phase A structural only, Phase D wiring).

### Pools (src/pools/)

Pipeline: discover → fetch (≤50 accounts/RPC) → decode (12+ DEX decoders) → calculate SOL price → cache. SOL pair only. Files: `discovery.rs` (DexScreener/GeckoTerminal/Raydium sources), `fetcher.rs` (batched RPC with AccountData), `decoders/` (Raydium CLMM/CPMM/Legacy, Orca Whirlpool, Meteora DAMM/DBC/DLMM, Pumpfun AMM/Legacy, Fluxbeam, Moonit), `calculator.rs` (price computation), `analyzer.rs` (pool classification), `cache.rs` (price history), `database/` (types.rs, operations.rs, writer.rs, blacklist.rs, global.rs — SQLite persistence), `blacklist.rs` (pool filtering), `swap/` (swap-related operations), `service.rs` (background service), `api.rs` (public price/pool query API), `utils.rs` (SOL mint detection, vault pairing helpers), `types.rs` (PoolDescriptor, PriceResult, ProgramKind).

### Tokens (src/tokens/)

Unified token database (12 tables: tokens, market_dexscreener, market_geckoterminal, token_pools, security_rugcheck, blacklist, update_tracking, token_favorites, rejection_history, rejection_stats, authority_reputation + indexes). Priority-based background updates with rate limiting. Files: `database/` (split into 11 submodules: mod.rs core/globals, metadata.rs token CRUD, market.rs DexScreener/GeckoTerminal, security.rs rugcheck, pool_data.rs pool snapshots, rejections.rs rejection tracking/history/stats, blacklist.rs blacklist CRUD, priority.rs priority management, tracking.rs update tracking/counts, assembly.rs complex token assembly, authority.rs authority reputation + auto-discovery, async_api.rs async wrappers), `schema.rs` (table definitions), `market/` (dexscreener.rs, geckoterminal.rs fetchers), `security/` (rugcheck.rs fetcher), `updates/` (split into 5 submodules: mod.rs re-exports, helpers.rs in-flight tracking/error classification, rate_limiter.rs RateLimitCoordinator with per-endpoint semaphores, core.rs update_token/batch/security + PoolPriorityManager, loops.rs start_update_loop + 5 priority-specific loops + force_update_token), `priorities.rs` (Priority enum), `filtered.rs` (passed/rejected/blacklisted token lists), `decimals.rs` (on-chain + cache), `discovery.rs` (new token detection), `cleanup.rs` (blacklist management), `store.rs` (snapshot cache), `events.rs` (token events), `service.rs` (ServiceManager integration), `favorites.rs` (user-managed token favorites with notes), `search.rs` (unified search across DexScreener/GeckoTerminal), `pools/` (token pool analysis submodule: api.rs, cache.rs, conversion.rs, operations.rs, utils.rs), `types.rs` (Token, DataSource, SecurityRisk, etc.).

### Filtering (src/filtering/)

Token filtering engine with multiple criteria sources. Cached snapshots with query API. Tracks passed/rejected tokens with detailed reasons. Files: `engine.rs` (compute_snapshot with concurrent processing), `sources/` (dexscreener.rs, geckoterminal.rs, rugcheck.rs, meta.rs, onchain.rs, ai.rs filters), `store.rs` (global FilteringStore with snapshot cache), `types.rs` (FilteringSnapshot, PassedToken, RejectedToken, FilteringQuery).

#### Filtering Pipeline Order

The filtering engine evaluates tokens in a specific order optimized for efficiency and cost reduction:

1. **Meta Filter** (`sources/meta.rs`) — Core checks: decimals validation, token age, cooldown periods
2. **On-Chain Filter** (`sources/onchain.rs`) — Fast scam detection using on-chain data (NO external APIs)
3. **DexScreener** (`sources/dexscreener.rs`) — Market data validation (liquidity, volume, price changes)
4. **GeckoTerminal** (`sources/geckoterminal.rs`) — Alternative market data source
5. **Rugcheck** (`sources/rugcheck.rs`) — Security analysis (authorities, holder distribution)
6. **AI Filter** (`sources/ai.rs`) — LLM-powered analysis (runs LAST, only on tokens that passed all other filters)

**Rationale:** On-chain filter runs early (after meta, before external APIs) to catch obvious scams without wasting API calls or credits on DexScreener/GeckoTerminal/Rugcheck/AI.

#### FilterSource Enum

All filter rejections are attributed to a source via the `FilterSource` enum (`sources/mod.rs`):

```rust
pub enum FilterSource {
    Core,        // Meta filters (age, decimals, cooldown)
    OnChain,     // On-chain scam detection
    DexScreener, // DexScreener market data filters
    GeckoTerminal, // GeckoTerminal market data filters
    Rugcheck,    // Rugcheck security filters
    Ai,          // AI-powered filtering
}
```

Each source has a corresponding rejection reason enum variant and config section.

#### On-Chain Scam Filter

**Purpose:** Detects scam tokens using ONLY on-chain data (Metaplex metadata + SPL mint authorities) — no external APIs, zero cost, instant results.

**Pipeline Position:** Runs AFTER meta filter, BEFORE DexScreener/GeckoTerminal/Rugcheck — catches obvious scams early before wasting API calls on external sources.

**Heuristics:**

1. **H1: Numeric-only symbols** — Rejects symbols like "00", "123", "420", "999" (classic scam pattern)
2. **H2: Empty/whitespace symbols** — Rejects tokens with empty or whitespace-only symbols
3. **H3: Single-char suspicious symbols** — Single-character symbols (configurable, disabled by default via `allow_single_char_symbols`)
4. **H4: Auto-discovered scam authorities** — DB-backed auto-growing list that learns from rejection patterns (no hardcoded seeds)
5. **H5: Immutable metadata + freeze authority** — Dangerous combo: can't update metadata but CAN freeze tokens
6. **H6: Combined risk scoring** — Weighted scoring of multiple weak signals (numeric symbol + questionable authorities + metadata issues) — rejects when score ≥ threshold

**Configuration:** `OnChainFilters` in `filtering.rs` config schema:

```rust
OnChainFilters {
    enabled: true,              // Master switch
    reject_numeric_symbols: true,
    reject_empty_symbols: true,
    allow_single_char_symbols: true,  // false = H3 enabled
    check_scam_authorities: true,
    risk_score_threshold: 60,   // 0-100 scale for H6
    // ... scam authority lists
}
```

**Rejection Reasons:** Six variants in `FilterRejectionReason` enum:

- `OnChainNumericSymbol` (H1)
- `OnChainEmptySymbol` (H2)
- `OnChainSuspiciousSymbol` (H3)
- `OnChainKnownScamAuthority` (H4)
- `OnChainImmutableWithFreeze` (H5)
- `OnChainHighRiskScore` (H6)

**Dashboard UI:** Filtering page → On-Chain tab with 3 analysis categories:

1. **Symbol Analysis** — H1, H2, H3 stats with examples
2. **Authority Analysis** — H4 stats with known scam wallet detection
3. **Risk Scoring** — H6 score distribution histogram + high-risk token examples

**Files:**

- `src/filtering/sources/onchain.rs` — Core filter logic with `evaluate()` function
- `src/config/schemas/filtering.rs` — `OnChainFilters` config struct
- `src/filtering/sources/mod.rs` — `FilterSource::OnChain` enum variant + rejection reasons
- `src/debug_bins/debug_onchain_filter.rs` — Debug binary for testing filter standalone: `cargo run --bin debug_onchain_filter <MINT_ADDRESS>`

**Performance:** Extremely fast (microseconds per token) — uses cached Metaplex metadata already in database, no RPC calls or external API requests. Typical rejection rate: 5-15% of tokens filtered before expensive API calls.

##### Authority Reputation System

Auto-growing scam authority detection. Starts empty, learns from rejection patterns — NO hardcoded scam addresses.

**How it works:** Background task (every 5min) queries `authority_reputation` SQLite table, groups tokens by freeze/mint/update authority, cross-references with rejection data. Blocks authority if confidence ≥ 0.8 AND total_tokens ≥ 5.

**Key files:**
- `src/tokens/authority_cache.rs` — In-memory blocked set (`ArcSwap<DashSet>`, O(1) lookup)
- `src/tokens/database/authority.rs` — DB persistence + discovery SQL
- `src/tokens/decimals.rs` — Extracts authority data from SPL Mint during existing fetch (zero extra RPC)
- `src/filtering/sources/onchain.rs` — Calls `is_blocked_authority()` during filtering

**Pitfalls:**
- Use `ArcSwap::store()` for atomic set replacement — never `DashSet.clear()` + insert (race condition)
- Authority cache populated as side effect of decimals fetch — do not add separate RPC calls
- Token assembly falls back to authority_cache when Rugcheck data unavailable

### Swaps (src/swaps/)

Trait-based router architecture supporting multiple DEX routers (GMGN, Jupiter, Raydium). Router trait in `router.rs` with registry pattern in `registry.rs`. Concurrent quote fetching, unified comparison, automatic best-route selection. Files: `router.rs` (Router trait), `registry.rs` (RouterRegistry), `routers/` (gmgn.rs, jupiter.rs, raydium.rs), `types.rs` (UnifiedQuote, SwapRequest, SwapResult), `operations.rs` (execute_swap, get_best_quote).

### Wallet (src/wallets/)

Balance monitoring with historical snapshots in SQLite (`data/wallet.db`). Tracks SOL + token balances with delayed RPC calls. Background service checks every minute. Files: `balance_monitor/` (types.rs, cache.rs, dashboard.rs, database.rs, service.rs), `manager.rs` (wallet management entry point — mod declarations and init only). ATA operations in `src/ata_operations.rs` (close/cleanup ATAs, get balances). Connection pooling with r2d2. Export to CSV. Service integration.

Code organization: `src/wallets/manager.rs` is a thin orchestration file (~100 LOC) — only global state, initialization, mod declarations, and re-exports. All logic lives in submodules under `src/wallets/manager/`. Similarly, `src/wallets/database.rs` is a thin struct definition (~75 LOC) — all wallet query methods are in `database/wallet_queries.rs`.

Current wallet submodules:
- `src/wallets/manager/` — crud, access, main_wallet, cache, bulk_ops, balance_ops, balance_queries, tools, migration
- `src/wallets/database/` — schema, token_balances, wallet_queries
- `src/wallets/balance_monitor/database/` — schema, metrics, flow_cache, dashboard_metrics, snapshots
- `src/wallets/balance_monitor/dashboard/` — token_metadata, flow_metrics
- `balance_monitor/types.rs` uses `SnapshotTokenBalance` (not `TokenBalance`) to avoid collision with `wallets/types.rs`

### Transactions (src/transactions/)

Real-time monitoring via WebSocket + bootstrap. Comprehensive DEX transaction analysis. Files: `manager.rs` (TransactionsManager lifecycle), `service/` (config.rs, lifecycle.rs, bootstrap.rs, processing.rs, websocket.rs, health.rs), `analyzer/` (classification, swap detection, P&L calculation, patterns.rs for pattern detection and risk assessment — 6-step pipeline), `processor/` (core.rs, extraction.rs, analysis.rs, helpers.rs), `fetcher.rs` (RPC batching with 50-account limit), `verifier.rs` (position integration), `websocket.rs` (real-time streaming), `database/` (types.rs, schema.rs, operations.rs, maintenance.rs, global.rs), `debug.rs` (diagnostics), `types.rs` (Transaction, TransactionType, SwapPnLInfo, AtaOperation), `utils.rs` (helpers), `program_ids.rs` (DEX program ID constants).

### Database (src/database/)

Shared SQLite configuration system for ALL databases. Files: `configure.rs` — Centralized PRAGMA configuration with DbPreset enum (Hot: 20MB cache + 256MB mmap, Standard: 8MB cache, Cold: 2MB cache), DbConfig struct with per-database constants (16 total), `configure_connection()` function that applies all PRAGMAs via `with_init()` on r2d2 pools. Every database MUST use this module — no ad-hoc PRAGMAs. Per-database constants: TOKENS_DB, TRANSACTIONS_DB, EVENTS_WRITE_DB, EVENTS_READ_DB, ACTIONS_WRITE_DB, ACTIONS_READ_DB, POSITIONS_DB, WALLET_MONITOR_DB, OHLCVS_DB, TOOLS_DB, STRATEGIES_DB, WALLETS_DB, RPC_STATS_DB, AI_CHAT_DB, AI_DB, POOLS_DB. Pool sizes: Hot=4-5, Standard=2-4, Cold=1-3. jemalloc is the global allocator on non-MSVC platforms (feature flag: "jemalloc", default on).

#### Database Maintenance (src/database/maintenance.rs)

Automated SQLite maintenance module that prevents disk fragmentation. Runs on startup + configurable periodic intervals.

**Features:**
- **Auto-migration**: Detects databases with `auto_vacuum=NONE` and migrates them to `INCREMENTAL` mode on first run (requires full VACUUM to update file header).
- **Incremental vacuum**: Runs `incremental_vacuum(500)` to reclaim up to 500 freelist pages per maintenance cycle.
- **WAL checkpoint**: Runs `PRAGMA wal_checkpoint(TRUNCATE)` to prevent WAL file growth by resetting it to zero bytes after flushing to main database file.
- **Multi-database coverage**: Maintains 13 databases: tokens, transactions, positions, wallet, events, pools, strategies, ohlcvs, actions, tools, ai, ai_chat, rpc_stats.
- **Configurable intervals**: `maintenance.vacuum_interval_secs` (default: 6h, minimum: 1h) and `maintenance.wal_checkpoint_interval_secs` (default: 30min, minimum: 5min).
- **Interleaved execution**: Uses `tokio::select!` to run vacuum and WAL checkpoint timers concurrently without blocking.

**Phase C Results:**
- `pools.db`: 729 MB → ~0 MB after VACUUM (99% freelist, 0 live rows).
- `ohlcvs.db`: 354 MB → 175 MB (49% reduction).
- All databases now use `auto_vacuum=INCREMENTAL` to prevent future fragmentation.

**Phase D Improvements:**
- WAL checkpoint prevents unbounded WAL file growth during high-write workloads.
- Configurable maintenance intervals allow tuning based on workload patterns.

**Phase E (SQLite Robustness):**
- `unlock_notify` feature added to rusqlite — concurrent connections now block/retry on BUSY instead of immediate failure.
- All r2d2 pools set `idle_timeout(None)` + `max_lifetime(None)` — prevents connection recycling that can lose PRAGMA state.
- `shrink_to_fit()` added after token loading — reclaims Vec over-allocation (~18 MB).

**jemalloc Tuning (optional):**
For production deployments, set the `MALLOC_CONF` environment variable before starting the bot:
```
MALLOC_CONF=dirty_decay_ms:5000,muzzy_decay_ms:5000,narenas:4 ./screenerbot
```
- `dirty_decay_ms:5000` — returns dirty pages to OS after 5s (default 10s)
- `muzzy_decay_ms:5000` — returns muzzy pages after 5s (default 10s)
- `narenas:4` — limits jemalloc arenas to 4 (reduces per-arena overhead)

### Trader (src/trader/)

Orchestrates automated and manual trading with monitors (`entry.rs`, `exit.rs`) handling orchestration; evaluators (entry/exit/DCA/strategies with priority safety gates — exit evaluators split into individual files: `exit_roi.rs`, `exit_stop_loss.rs`, `exit_time.rs`, `exit_trailing.rs`); executors (buy/sell/DCA); `safety/` (loss_limit.rs for period-based loss protection, blacklist.rs for auto-blacklisting, cooldown.rs for trade cooldowns, limits.rs for position/exposure limits, risk.rs for risk assessment), `manual/`, `config.rs` + `constants.rs` for runtime knobs, `controller.rs`, `service.rs` (depends on pools + tokens), and `types.rs`.

#### Trading Safety System (src/trader/safety/, src/global.rs)

- **FORCE_STOP**: Global flag that immediately halts all trading (entries, exits, swaps, manual trades). Set via API or dashboard emergency button. Webserver/services continue.
- **Loss Limit Protection**: Period-based (1-168h) cumulative loss tracking. When limit reached, entries pause but exits continue to protect positions. Auto-resume on period reset (configurable).
- **Separate Monitor Controls**: `entry_monitor_enabled` + `exit_monitor_enabled` config fields allow independent control. Common use: stop entries but let exit monitor manage existing positions.

Config fields: `loss_limit_enabled`, `loss_limit_sol`, `loss_limit_period_hours`, `loss_limit_auto_resume`, `entry_monitor_enabled`, `exit_monitor_enabled`.

API routes: `/api/trader/force-stop`, `/api/trader/resume`, `/api/trader/force-stop/status`, `/api/trader/monitors/status`, `/api/trader/monitors/entry/toggle`, `/api/trader/monitors/exit/toggle`, `/api/trader/loss-limit/status`, `/api/trader/loss-limit/resume`, `/api/trader/loss-limit/reset`.

### Positions (src/positions/)

Manages open/closed positions with DCA and partial exit support. Tracks entry/exit history with EntryRecord and ExitRecord. Files: `state.rs` (global POSITIONS state with locks), `operations.rs` (open/close/partial_close/add_to_position), `database/` (types.rs, operations.rs, global.rs, convenience.rs — SQLite persistence for positions/entries/exits), `helpers.rs` (P&L calculations, index management), `apply.rs` (position updates), `transitions.rs` (state transitions), `tracking.rs` (update_position_tracking), `price_updater.rs` (background price updates), `loss_detection.rs` (loss thresholds + blacklisting), `queue.rs` (verification queue), `verifier.rs` (chain verification), `worker.rs` (background worker), `metrics.rs` (ProceedsMetricsSnapshot), `types.rs` (Position with DCA/partial exit fields, EntryRecord, ExitRecord).

### Strategies (src/strategies/)

Condition-based trading strategy system with DB persistence. Entry/exit signal evaluation. Files: `engine.rs` (StrategyEngine with evaluation logic), `conditions/` (price, volume, indicator conditions), `database.rs` (strategies database with SQLite), `types.rs` (Strategy, StrategyType, EvaluationContext, EvaluationResult, MarketData, OhlcvData, Candle).

### OHLCV (src/ohlcvs/)

Multi-timeframe OHLCV data with priority-based monitoring. Syncs with Pool Service every 5 minutes. Priority auto-adjusts for open positions. Files: `monitor.rs` (OhlcvMonitor with token tracking), `fetcher.rs` (DexScreener/GeckoTerminal data fetch), `aggregator.rs` (timeframe aggregation), `database.rs` (SQLite with per-token tables), `cache.rs` (in-memory cache), `manager.rs` (PoolManager), `priorities.rs` (ActivityType, priority calculation), `gaps.rs` (gap detection), `service.rs` (OhlcvService), `types.rs` (OhlcvDataPoint, Timeframe, Priority, PoolConfig).

### AI Analysis / Assistant (src/ai/)

LLM-powered token analysis for intelligent filtering, entry/exit decisions, scam detection, and interactive chat with tool calling. ALL FEATURES DISABLED BY DEFAULT. Supports 10 providers: OpenAI, Anthropic, Groq, DeepSeek, Gemini, Ollama, Together, OpenRouter, Mistral, an LLM provider. Config in `[ai]` section. NOTE: User-facing UI calls this "Assistant" (tab bar, page titles, settings) while code uses "AI/ai" naming.

**Architecture:**
- `engine.rs` — AiEngine orchestrator with evaluate_filter(), evaluate_entry(), evaluate_exit(). Global singleton via init_ai_engine()/get_ai_engine()/try_get_ai_engine().
- `cache.rs` — AiCache with DashMap, TTL-based expiry. Priority::High bypasses cache (for trading decisions).
- `db.rs` — SQLite persistence (data/ai.db) for Instructions and DecisionRecord.
- `prompts/` — builder.rs (PromptBuilder), templates.rs (system prompts).
- `schemas/` — filter_decision.rs, trade_decision.rs, exit_suggestion.rs.
- `types.rs` — Priority (High/Medium/Low), AiDecision, AiError, EvaluationContext, EvaluationResult.

**AI Chat System (MCP-like tool calling):**
- `chat_engine.rs` — ChatEngine with process_message(), call_llm(), parse_tool_calls(). Tool loop with MAX_TOOL_ITERATIONS=5.
- `chat_db.rs` — SQLite persistence (data/ai_chat.db) for sessions and messages.
- `tools/` — Tool registry: analysis.rs, portfolio.rs, trading.rs (require confirmation), config.rs, system.rs.

**Tool Permissions** (per-category config fields):
- `allow` — Execute immediately without confirmation
- `ask_user` — Show confirmation dialog before execution
- `deny` — Block tool entirely with explanation

**AI Scheduled Tasks:**
- `scheduled_db.rs` — Scheduled AI task persistence with interval/daily/weekly schedules, retry logic, timeouts.
- Background service executes tasks headlessly via ChatEngine.
- API Routes: `/api/ai/automation` (CRUD), toggle, run, history, stats.

**LLM Clients** (`src/apis/llm/`): LlmManager singleton. LlmClient trait. Per-provider subdirectories with client.rs.

**Key Config Fields:** `enabled`, `default_provider`, `filtering_enabled`, `filtering_min_confidence`, `filtering_fallback_pass`, `entry_analysis_enabled`, `exit_analysis_enabled`, `ai_trailing_stop_enabled`, `trading_bypass_cache`, `auto_blacklist_enabled`, `cache_ttl_seconds`.

### Events (src/events/)

Structured JSON event logging with dedicated SQLite DB (`data/events.db`). Non-blocking async channels. Files: `database.rs` (EventsDatabase with connection pool), `maintenance.rs` (record helpers for each event type, cleanup task), `types.rs` (Event, EventCategory, Severity). Event types: API, Connectivity, Entry, Filtering, OHLCV, Pool, Position, Security, Swap, System, Token, Trader, Transaction.

### Telegram (src/telegram/)

Comprehensive Telegram bot integration providing notifications, commands, and chat discovery. Files: `types.rs`, `bot.rs`, `service.rs`, `notifier.rs`, `session.rs` (password/TOTP auth), `discovery.rs` (chat discovery without chat_id), `polling.rs`, `totp.rs`, `formatters.rs`, `keyboards.rs`, `pagination.rs`, `commands/` (mod.rs router, trading.rs, status.rs, menu.rs, callbacks.rs).

Bot States: Disconnected → Discovery (no chat_id) → Connected (full operation).

### APIs (src/apis/)

Centralized API clients with global singletons and rate limiting. Files: `manager.rs` (ApiManager), `client.rs` (HttpClient, RateLimiter), `stats.rs` (ApiStatsTracker), subdirectories: `coingecko/`, `defillama/`, `dexscreener/`, `geckoterminal/`, `jupiter/`, `rugcheck/` (each with client.rs, types.rs). LLM clients in `src/apis/llm/`.

### Connectivity (src/connectivity/)

Endpoint health monitoring with fallback strategies. Files: `monitor.rs` (EndpointMonitor trait), `monitors/` (specific endpoint monitors), `service.rs` (ConnectivityService), `state.rs` (global health state, `are_critical_endpoints_healthy()`), `types.rs` (EndpointHealth, EndpointCriticality, FallbackStrategy).

### SOL Price (src/sol_price.rs)

Real-time SOL/USD price from Jupiter API. Background service with 30s refresh. Cached price with expiry. Thread-safe access. Functions: `start_sol_price_service()`, `get_cached_sol_price()`.

### RPC (src/rpc/)

Modular RPC client with multi-provider support, rate limiting, circuit breaker, and SQLite stats. Files: `manager.rs` (RpcManager singleton), `selector.rs` (provider selection: RoundRobin, Priority, LatencyBased, Adaptive), `types.rs` (ProviderKind, RpcMethod, CircuitState, SelectionStrategy), `errors.rs`, `client/` (RpcClient wrapper with RpcClientMethods trait), `rate_limiter/` (Governor GCRA algorithm), `circuit_breaker/` (Closed→Open→HalfOpen state machine), `stats/` (SQLite persistence in data/rpc_stats.db), `provider/` (auto-detection for Helius, QuickNode, Triton, Alchemy, public endpoints). Access: `get_rpc_client()` + RpcClientMethods trait. Transaction encoding MUST be jsonParsed.

### Services (src/services/)

ServiceManager with dependency resolution, priority-based startup (topological sort), reverse-order shutdown, health/metrics monitoring. Files: `mod.rs` (Service trait, ServiceManager, GLOBAL_SERVICE_MANAGER), `health.rs`, `metrics.rs` (MetricsCollector with tokio_metrics TaskMonitor sampling), `implementations/` (23 services). ALL Service trait implementations live in `implementations/`. Service wrappers should be thin (<120 lines); extract business logic to domain modules.

### Webserver (src/webserver/)

Axum REST API + embedded dashboard. Files: `server.rs` (start/shutdown), `state.rs` (AppState with service accessors), `routes/` (31 route modules), `snapshot/` (data collectors), `utils.rs` (success_response, error_response), `embeds.rs` (all include_str! constants), `templates.rs` (rendering functions), `middleware.rs` (security_gate, initialization_gate, cache_control, auth_gate), `session.rs` (in-memory session tokens), `totp.rs` (TOTP 2FA), `demo.rs` + `demo_data.rs` (demo mode).

**Middleware Cache-Control Strategy:**
- Version-aware caching: URLs with `?v=` query parameter get `public, immutable, max-age=31536000` (1-year cache)
- Unversioned scripts/assets get `public, max-age=3600, must-revalidate` (1-hour revalidation)
- API endpoints: `no-cache, no-store, must-revalidate`
- HTML pages: `no-cache` (allows conditional requests with ETag/Last-Modified)

**Dashboard Pages (20):** dashboard, positions, tokens, filtering, transactions, strategies, ohlcv, config, ai, telegram, events, connectivity, wallets, tools, updates, about, lockscreen, splash, login, onboarding/setup.

**Dashboard CSS Architecture:**
- `base/scrollbar.css` — All scrollbar styling (.scrollbar-thin, .scrollbar-hidden, etc.)
- `base/floating.css` — Dropdowns, popovers, dialogs with z-index hierarchy
- `components/form_controls.css` — All form inputs, toggles (.toggle + .toggle-track)

Z-Index Hierarchy (from floating.css):
- `--z-dropdown`: 1000
- `--z-tooltip`: 1500
- `--z-popover`: 2000
- `--z-dialog`: 10000
- `--z-context-menu`: 10006
- `--z-toast`: 10010
- `--z-lockscreen`: 100000

**Dashboard JavaScript Architecture:**
Modular JS with ES6 imports. Page scripts in `scripts/pages/`, UI components in `scripts/ui/`. Split patterns: prototype mixin, factory function, pure module extraction.

New JS modules need: 1) File in correct subdirectory, 2) `pub(super) const` in embeds.rs, 3) Match arm in asset_serving.rs.

New CSS modules need: 1) File in correct subdirectory, 2) `pub(super) const` in embeds.rs, 3) Added to page-specific style array in templates.rs, 4) Added to COMBINED_STYLES array.

### Constants (src/constants.rs)

System-wide constants: SOL_MINT, SOL_DECIMALS, LAMPORTS_PER_SOL, SYSTEM_PROGRAM_ID, USDC_MINT, USDT_MINT, SPL_TOKEN_PROGRAM_ID, TOKEN_2022_PROGRAM_ID, ASSOCIATED_TOKEN_PROGRAM_ID, COMPUTE_BUDGET_PROGRAM_ID, MEMO_PROGRAM_ID, JUPITER_V6_PROGRAM_ID, JUPITER_V4_PROGRAM_ID, DEX program IDs (Raydium, Orca, Meteora, Pumpfun, etc.).

### ATA Cleanup (src/ata_cleanup.rs)

Background service for empty ATA cleanup. Runs every 5 minutes after a 30s startup delay. Failed ATA cache in `data/ata_failed_cache.json`. Stats tracking. Functions: `start_ata_cleanup_service()`, `get_ata_stats()`. Types: AtaCleanupStats, FailedAtaCache.

### Actions (src/actions/)

Operation progress tracking with real-time SSE broadcasting to dashboard. Tracks lifecycle of trading operations through discrete steps. Types: ActionType (SwapBuy/Sell/PositionOpen/Close/DCA/PartialExit/ManualOrder), ActionState (pending/running/completed/failed), ActionStep. Dual-write architecture: in-memory HashMap + SQLite DB. Broadcast channel for SSE streaming.

### Errors (src/errors/)

Structured error types with blockchain-aware parsing. Files:

- `mod.rs` (re-exports for `Error` and `Result<T>`)
- `error.rs` (`Error` enum + `Result<T>` alias + builder helpers)
- `database.rs` (`DatabaseError` for rusqlite/r2d2/schema/migrations)
- `network.rs` (`NetworkError`)
- `rpc_provider.rs` (`RpcProviderError`)
- `configuration.rs` (`ConfigurationError`)
- `data.rs` (`DataError`)
- `io.rs` (`IoError` for filesystem/OS errors)
- `internal.rs` (`InternalError` for invariants, task join failures, timeouts)
- `position.rs` (`PositionError`)
- `rate_limit.rs` (`RateLimitError`)
- `service.rs` (`ServiceError` for ServiceManager lifecycle/deps)
- `blockchain.rs` (`BlockchainError`, `parse_solana_error()`, `parse_structured_solana_error()`, `CommitmentLevel`, retry/severity helpers)

**Important:** No backwards-compatibility conversions exist. All errors must be constructed with explicit domain variants:
- ❌ `Err("error message".into())` - NOT ALLOWED
- ❌ `Err(format!("error").into())` - NOT ALLOWED
- ✅ `Err(Error::Network(NetworkError::Generic { message }))` - CORRECT
- ✅ `Err(Error::Data(DataError::ParseError { data_type, error }))` - CORRECT
- ✅ `Err(Error::Configuration(ConfigurationError::Generic { message }))` - CORRECT

### Global (src/global.rs)

Startup coordination flags (CONNECTIVITY_SYSTEM_READY, TOKENS_SYSTEM_READY, POSITIONS_SYSTEM_READY, POOL_SERVICE_READY, TRANSACTIONS_SYSTEM_READY). Check `are_core_services_ready()` before trading. File path constants (DATA_DIR, CONFIG_FILE, TOKENS_DATABASE, etc.). Trading control: FORCE_STOPPED with ForceStopStatus. Tool coordination: TOOLS_ACTIVE_COUNT, DASHBOARD_ACTIVE_TOKEN.

### Logger (src/logger/)

Level-based API: `error()`, `warning()`, `info()`, `debug()`, `verbose()`. Per-module debug control via `--debug-<module>`.

### Profiling

CPU profiling support via compile-time feature flags. `--features console` enables tokio-console. `--features flamegraph` enables pprof-based flamegraph generation. Profile during steady-state (5+ minutes after boot), not startup.

### Reset (src/reset.rs)

Bot state reset utility. Clears databases, caches, and verification state. Supports interactive mode (prompts for confirmation) and force mode (`--force` flag).

---

## Critical Patterns

### Price Data Architecture (Pool vs OHLCV)

Two independent price systems exist — understand the difference:

**Pool Price System** (`src/pools/`):
- Source: Direct RPC calls to Solana blockchain (~500ms intervals)
- Data: Real-time token/SOL prices from DEX pool reserves
- Use: Entry/exit decisions, P&L calculations, position tracking
- Latency: ~500ms, high precision, direct from chain

**OHLCV System** (`src/ohlcvs/`):
- Source: DexScreener/GeckoTerminal APIs (1min-24h candles)
- Data: Historical candlestick data for technical analysis
- Use: Strategy evaluation, indicators (RSI, MA, etc.)
- Latency: 1-5 min depending on timeframe

**Key rule:** Never mix these systems. Pool price for trading decisions, OHLCV for strategies.

### Axum Handler Patterns

Limitation: `Query<T>` extractors break with complex nested async. Solution: Use POST with `Json<T>` for complex ops; use GET with `Query<T>` only for simple calls.

### Service Metrics Collection

Services receive TaskMonitor in `start()`. Wrap spawned tasks with `monitor.instrument(async { ... })`. MetricsCollector samples `monitor.cumulative()` every 1s. View at `GET /api/services`. Never use `intervals()` API (blocking) — use periodic `cumulative()` sampling.

### Config System

Macro-driven via `config_struct!`. Values in `data/config.toml`. Access: `with_config(|cfg| cfg.section.field)` (sync), `get_config_clone()` (async). Hot reload: `reload_config()` or `POST /api/config/reload`. Never hardcode values or create `_v2` versions.

The config page currently has 20 sections: rpc, trader, positions, filtering, swaps, tokens, pools, wallet, sol_price, telegram, ai, strategies, holder_watch, events, webserver, services, monitoring, performance, maintenance, ohlcv.

**Adding New Config Sections:**

Each new config section requires 7 things wired:
- GET route + PATCH route in `routes/config/mod.rs`
- Getter function in `getters.rs`
- PATCH handler case with `type_name` matching
- Metadata registration in `src/config/metadata.rs`
- Sidebar entry in `utils.js` (add to `SECTION_DISPLAY_ORDER` + `SECTION_LABEL_OVERRIDES`)
- Icon in `field_renderers.js` (add to `SECTION_ICONS`)

Sidebar field counts use recursive leaf-field counting via `countRecursive()` in `utils.js`.

For nested config structs, add `#[metadata(field_metadata!{...})]` on ALL leaf fields to enable auto-rendering. Without field metadata, nested objects render as collapsed JSON blobs instead of proper form fields.

**Hardcoded Constants (NEVER make configurable):**
- Jupiter referral fee: `REFERRAL_FEE_BPS=50` and referral token accounts are hardcoded in `src/swaps/routers/jupiter.rs`

### RPC Client Rules

Use `get_rpc_client()` — never construct RpcClient directly. Transaction encoding must be jsonParsed. Features: multi-provider support, per-provider rate limiting (Governor GCRA), circuit breaker failover. Respect ≤50 accounts for `get_multiple_accounts`.

### Database Patterns

SQLite DBs in `data/*.db`. All append-only. Access via module helpers (e.g., `positions::db::get_open_positions()`). Never raw SQL in business logic. Performance: WAL mode, `busy_timeout(30000)`, `cache_size=10000+`, `temp_store=memory`. Async: use `tokio::task::spawn_blocking` for rusqlite sync ops. Debug with `src/bin/debug_*` tools — never hand-edit DBs.

### Webserver Architecture

Backend: Response types inline in route files — no separate models folder. Each route is self-contained. Use `success_response()`/`error_response()` from `webserver/utils.rs`. Config UI metadata-driven.

Frontend: Pages load as ES modules. Pure HTML in `pages/*.html`, no inline scripts/styles. All helpers in `core/utils.js` — never duplicate.

**ES Module Cache Busting:**
- `version_js_imports()` in `src/webserver/routes/asset_serving.rs` rewrites all JS `import from` paths to include `?v=VERSION-TIMESTAMP` at serve time
- `serve_js()` helper serves all JS files through the versioning pipeline
- `ASSET_VERSION_TS` is generated per-build in build.rs via SystemTime epoch seconds
- Cache-control strategy: URLs with `?v=` get `immutable, max-age=1yr`; without `?v=` get `must-revalidate, max-age=1hr`; API → `no-cache`; HTML → `no-cache`

**Build System:**
- `build.rs` recursively watches all files in `src/webserver/templates/` individually (directory-level `rerun-if-changed` only watches listing, not content changes)

**Performance Patterns:**
- Status endpoints: `/api/status/services` and `/api/status/metrics` use targeted collector functions, NOT `gather_status_snapshot()`. Only `/api/status` uses the full snapshot.
- Dashboard overview uses SQL aggregation (`get_period_trading_stats`) — never load all closed positions into memory.
- Trader stats uses `get_closed_positions_since()` for time-bounded DB queries.
- Position detail uses `tokio::join!` for parallel async calls.
- Transaction collector uses `tokio::join!` for parallel DB queries.
- Cache-control: static assets (`/scripts/*`, `/assets/*`, `/fonts/*`) get `immutable, max-age=31536000`. API endpoints get `no-store`. HTML pages get `no-cache`.

### Performance Patterns

**Rule: Always use `tokio::join!` for independent async calls** — Never sequential await. When fetching multiple pieces of data that don't depend on each other, run them in parallel:
```rust
let (data1, data2, data3) = tokio::join!(fetch1(), fetch2(), fetch3());
```

**Rule: Push filtering/aggregation to SQL** — Never load all records then filter in Rust. Use WHERE clauses, aggregation functions (COUNT, SUM, AVG), and LIMIT in queries. O(1) memory is better than O(n).

**Rule: Use targeted data fetching** — If you only need services status, don't run a full system snapshot. Call the specific collector function you need (e.g., `get_services_status()` not `gather_status_snapshot()`).

**Rule: Cache immutable resources aggressively** — Compile-time embedded assets don't change between builds. Set `Cache-Control: public, immutable, max-age=31536000` for static assets. Only dynamic API responses need `no-store`.

### Dashboard Design System

Dark theme colors: `bg-primary=#0d1117`, `bg-secondary=#161b22`, `border-color=#30363d`, `text-primary=#e6edf3`, `success=#3fb950`, `error=#f85149`. Always use CSS variables from `foundation.css`.

Typography: JetBrains Mono with tabular numerals. Labels: 0.625rem uppercase. Values: 0.875rem bold. System sans-serif for UI text.

Spacing: 8px base scale (xs=4, sm=8, md=16, lg=24, xl=32). Border-radius: sm=4, md=6, lg=8, xl=12. Cards: 8px radius, 1px solid border-color.

Components: Tabs use 3px bottom-border active state. Dialogs: 16px radius, backdrop blur. Use CSS Grid with `minmax()`. Breakpoints at 1280/1100/800/500px.

### Startup & Readiness

Atomic flags in `src/global.rs`: TOKENS_SYSTEM_READY, POSITIONS_SYSTEM_READY, POOL_SERVICE_READY, TRANSACTIONS_SYSTEM_READY. Flow: Base init → TX bootstrap → Pool tasks → Positions verify → Trader starts when `are_core_services_ready()` is true.

### Logging Standards

Mission: Logs enable investigation without code/DB changes.

Levels:
- `error!()` — Critical failures. Always shown.
- `warning!()` — Issues needing attention.
- `info!()` — Important operational events.
- `debug!()` — Only with `--debug-<module>`.
- `verbose!()` — Only with `--verbose` or `--verbose-<module>`.

Required elements: Full identifiers (never truncate mint addresses), contextual metadata, quantitative data, actionable summaries. Price precision: CRITICAL — tokens can have 12+ decimal places. Never `{:.6}`. Use scientific notation for < 1e-6.

Debug flags: `--debug-rpc`, `--debug-transactions`, `--debug-pool-fetcher`, `--debug-websocket`, `--debug-webserver`, `--debug-wallet`, `--debug-ohlcv`.

### Running the Bot (headless / nohup)

```bash
# Standard headless run (survives terminal disconnect)
nohup ./target/release/screenerbot > /tmp/screenerbot.log 2>&1 &
echo $! > /tmp/screenerbot.pid

# With debug logging
nohup ./target/release/screenerbot --debug-webserver --debug-trader > /tmp/screenerbot.log 2>&1 &

# Custom port
nohup ./target/release/screenerbot --port 9000 --host 0.0.0.0 > /tmp/screenerbot.log 2>&1 &

# Stop gracefully (SIGTERM)
kill $(cat /tmp/screenerbot.pid)

# Check status
ps -p $(cat /tmp/screenerbot.pid) -o pid,rss,etime
```

**Signal handling:**
- SIGINT (Ctrl+C) → graceful shutdown
- SIGTERM (`kill <PID>`) → graceful shutdown
- SIGQUIT → graceful shutdown
- SIGHUP → **ignored** (bot survives terminal disconnect / nohup)
- Second Ctrl+C during shutdown → immediate force exit

**⚠️ Pitfall:** SIGHUP was previously treated as shutdown signal (fixed 2026-02-21). If running old builds, bot will die when terminal disconnects even with nohup.

Log files: Daily rotation in `logs/screenerbot_YYYY-MM-DD_HH-MM-SS.log`. 24h retention.

### Cache Architecture

All caches use `moka::sync::Cache` (v0.12, W-TinyLFU eviction algorithm, thread-safe, lock-free concurrent access). One exception: PRICE_HISTORY stays as DashMap with periodic cleanup (hot-path requires `get_mut` at 100s/sec which moka doesn't support).

**Key characteristics:**
- `.get()` returns `Option<V>` — clones the value, not a reference
- `.invalidate_all()` instead of `.clear()`
- `.run_pending_tasks()` before metrics/counts for accurate numbers (moka uses lazy eviction)
- `moka::sync::Cache` in v0.12 does NOT have `.iter()` — use parallel DashSet key index when iteration needed (see DEFERRED_RETRY_KEYS pattern)

**Cache inventory:**

| Cache | File | Type | Cap | TTL | Notes |
|-------|------|------|-----|-----|-------|
| PRICE_CACHE | pools/cache.rs | DashMap+TTL | ∞/TTL | 30s | Pool price snapshots |
| PRICE_HISTORY | pools/cache.rs | DashMap+cleanup | bounded | 2h cleanup | Price ticks, hot-path get_mut |
| GLOBAL_KNOWN_SIGNATURES | transactions/utils.rs | moka | 50K | - | Dedup processed sigs |
| DECIMALS_CACHE | tokens/decimals.rs | moka | 100K | - | Token decimal places |
| TOKEN_2022_CACHE | tokens/decimals.rs | moka | 100K | - | Token-2022 flag |
| FAILED_CACHE | tokens/decimals.rs | moka | 50K | 24h | Failed decimal fetches (Phase C) |
| TOKEN_POOLS_CACHE | tokens/pool_data/cache.rs | moka | 5K | 120s | Pool snapshots |
| POOL_PREFETCH_STATE | tokens/pool_data/cache.rs | moka | 5K | 60s | Prefetch debounce |
| LAST_TOKEN_ACCOUNTS_CHECK | positions/verifier.rs | moka | 5K | 1h | Verification throttle |
| DEFERRED_RETRIES | transactions/service/config.rs | moka+DashSet | 1K | 5min | Retry queue |
| API_RESPONSE_CACHE | tokens/store/cache.rs | moka | 1K | 5min | API dedup (Phase C) |
| AI_CACHE | ai/cache.rs | moka | 5K | configurable | AI response cache |
| DEXSCREENER_CACHE | tokens/store.rs | moka | 2K | 30s | Market data |
| GECKOTERMINAL_CACHE | tokens/store.rs | moka | 2K | 60s | Market data |
| RUGCHECK_CACHE | tokens/store.rs | moka | 3K | 5min | Security data |

**Design rationale:**
- All caches bounded to prevent memory exhaustion
- W-TinyLFU eviction balances recency + frequency automatically
- TTL-based expiry for time-sensitive data (market prices, API responses)
- DEFERRED_RETRIES uses parallel DEFERRED_RETRY_KEYS DashSet to track keys since moka has no `.iter()`

### Memory Hot Spots (Phase C Optimized)

**Phase C Results:**
- **Stale token filter** — Reduced from ~172K → ~15.6K tokens (91% reduction) via 7-day cutoff on `market_data_last_fetched_at` field in token filtering.
- **RSS Improvement** — Median RSS: 1011 MB → 375 MB (62% reduction from baseline). Target ≤400 MB: **MET**.
- **Database maintenance** — Automated VACUUM operations (see Database Maintenance below) prevent disk fragmentation.

**Stale token configuration:**
- `maintenance.stale_token_days` (default: 7) — Controls which tokens the filtering engine loads from database based on `market_data_last_fetched_at` timestamp.
- Set to `0` to disable filtering (loads all tokens, ~278K+, for testing or when fresh data not available).
- Reduces memory footprint and filtering compute time by excluding tokens with stale market data.

Remaining considerations:
- **Token filter load** — Now loads ~15.6K tokens every 180s (~22 MB per load, dramatically reduced from 246 MB). Temporary allocation freed after ~6s.
- **jemalloc page retention** — After large temporary allocations, jemalloc keeps freed pages mapped. RSS appears elevated but memory is available. Configure `dirty_decay_ms` for faster return.

---

## Conventions

Extend, don't duplicate; no `_v2`/`_compat` forks. Reuse constants (e.g., `SOL_MINT`, `WSOL_MINT` in `tokens/decimals.rs`); avoid magic numbers. Errors: return `Result<T, String>`; DB uses SqliteResult; typed errors in `src/errors/`. Pricing uses PriceResult and the single-pool invariant (highest-liquidity SOL pair only).

### Database Configuration Rules

ALL databases must use `database::configure_connection()` — never set PRAGMAs directly. For pool connections: `SqliteConnectionManager::file(path).with_init(|c| configure_connection(c, preset))`. For single connections (Mutex pattern): call `configure_connection()` once after `Connection::open()`. When re-opening a connection (e.g., ai/database.rs reopen), must re-apply configure_connection. Per-database constants: TOKENS_DB, TRANSACTIONS_DB, EVENTS_WRITE_DB, EVENTS_READ_DB, ACTIONS_WRITE_DB, ACTIONS_READ_DB, POSITIONS_DB, WALLET_MONITOR_DB, OHLCVS_DB, TOOLS_DB, STRATEGIES_DB, WALLETS_DB, RPC_STATS_DB, AI_CHAT_DB, AI_DB, POOLS_DB. Pool sizes: Hot=4-5, Standard=2-4, Cold=1-3. jemalloc is the global allocator on non-MSVC platforms (feature flag: "jemalloc", default on).

---

## Making Changes

**New config field:** Add to `config_struct!` in `schemas.rs` → update `data/config.toml` → add to webserver `CONFIG_METADATA` (if UI needed). One source, three touches.

**New DEX:** Add to `ProgramKind` (`pools/types.rs`) → implement decoder `pools/decoders/<dex>.rs` (see `raydium_cpmm.rs`) → wire in `pools/calculator.rs::calculate_price()`.

**New service:** Implement `Service` trait in `services/implementations/<name>_service.rs`, register in `run.rs::register_all_services()`. Set priority and dependencies. Rules: (1) Every service file MUST have a `//!` doc comment as the first line. (2) Unit structs (no fields) MUST NOT have `impl Default` — only structs with fields need it. (3) Service `name()` must match the conventional name used in dependency lists (e.g., "pools" not "pool_helpers").

**New webserver route:** Create `routes/<name>.rs` with inline response types → implement `routes()` function → merge in `routes/mod.rs::api_routes()`.

**New frontend page:** Create `pages/<name>.html` (pure HTML), `scripts/pages/<name>.js` (lifecycle module with `createLifecycle()` + `registerPage()`), `styles/pages/<name>.css` (optional) → embed in `templates.rs` → wire route in `routes/mod.rs` → add nav link in `base.html`.

**New AI instruction:** Use dashboard AI tab → Instructions → Create, or `POST /api/ai/instructions`.

**New LLM provider:** Implement `LlmClient` trait in `src/apis/llm/<provider>/client.rs` → add to Provider enum → wire in LlmManager → add config to AiProvidersConfig.

---

## Common Pitfalls

- Hardcoding config values instead of using `with_config()` accessor.
- Doing token math without loaded decimals.
- Creating `RpcClient` directly instead of using `get_rpc_client()`, or changing tx encoding off jsonParsed.
- Aggregating multiple pools; pricing must use a single highest-liquidity SOL pair.
- Creating separate models folder for webserver types (keep inline with routes).
- Hardcoding webserver UI forms instead of using metadata-driven rendering.
- Importing services directly instead of using `get_service_manager()`.
- Hand-editing `data/*.db` files (use debug bins or API endpoints).
- Forgetting to wrap service tasks with `monitor.instrument()` (metrics won't collect).
- Using `intervals()` API from tokio_metrics (blocking — use periodic `cumulative()` sampling instead).
- Config export/import without checking `response.ok` — always validate HTTP status.
- Creating duplicate helper functions instead of extending `utils.js`.
- Creating new files prematurely instead of extending existing patterns.
- NEVER leave "dust comments" in place of removed code — remove completely.
- `cargo:rerun-if-changed=directory/` only watches the directory listing (adds/removes), NOT file content changes inside. Must list individual files recursively.

### Cache Pitfalls

- moka `.get()` clones the value — use `Arc<T>` for large types to avoid expensive clones.
- moka sync Cache has no `.iter()` in v0.12 — use parallel DashSet key index if iteration is needed.
- Call `.run_pending_tasks()` before `.entry_count()` for accurate counts (moka uses lazy eviction).
- When migrating from HashMap to moka, replace `.clear()` with `.invalidate_all()`.
- DEFERRED_RETRIES uses parallel DEFERRED_RETRY_KEYS DashSet to track keys since moka has no iter().

### Database Pitfalls

- NEVER set mmap_size > 256MB — causes RSS bloat (tokens.db had 30GB mmap before Phase A fix).
- r2d2 pools: set `idle_timeout(None)` + `max_lifetime(None)` on ALL pools. SQLite WAL mode needs persistent connections — default r2d2 recycling (10min/30min) drops PRAGMA state. PRAGMAs set on pool creation via `with_init()` are re-applied on new connections, but recycling adds unnecessary overhead and risk.
- `auto_vacuum=INCREMENTAL` must be set BEFORE first write to take effect on new databases.
- WAL checkpoint `TRUNCATE` mode resets WAL file to zero bytes — prevents unbounded growth but runs exclusively (briefly blocks writers).
- Maintenance intervals enforced: `vacuum_interval_secs` ≥ 1 hour (3600s), `wal_checkpoint_interval_secs` ≥ 5 minutes (300s).
- `stale_token_days=0` disables stale filtering — loads all 278K+ tokens from database (high memory, long filter compute).
- **SQLite auto_vacuum pitfall**: Setting `PRAGMA auto_vacuum = INCREMENTAL` per-connection does NOT retroactively change existing databases. The database file header remains unchanged. Must run `VACUUM` after setting the pragma to convert the database to incremental mode. The maintenance module handles this migration automatically.
- events.db and actions.db have dual pools (read + write) — both need separate configure_connection constants.
- Cleanup functions exist but need periodic wiring: cleanup_stats() (72h retention), cleanup_old_actions() (30d retention).
- Never manually VACUUM or modify production databases — all DB maintenance must be done by the bot code itself.
- `FOREIGN KEY constraint failed` during actions cleanup is a known pre-existing issue (non-critical).
- **Stale token filter**: Uses pre-computed Rust timestamp (not SQLite strftime) for performance when filtering tokens with `market_data_last_fetched_at` older than 7 days.
- **rusqlite `unlock_notify`**: Feature MUST be enabled in Cargo.toml alongside `bundled`. Without it, concurrent connections get immediate SQLITE_BUSY errors instead of blocking/retrying.
- `DataTable` column definitions — use `id` property (not `key`) and `container` property (not `containerId`).
- Frontend style hardcoding — never use inline styles or hardcode colors; use CSS variables from `foundation.css`.
- Skipping lifecycle hooks — always implement proper `init/activate/deactivate/dispose` for page modules.
- Mixing Pool Price and OHLCV data — Pool prices (500ms RPC) for trading, OHLCV (API) for strategies only.
- Catch blocks with unused error variables — use `catch { }` (no variable) instead of `catch (err) { }`.
- Forgetting to initialize state on startup — `loss_limit.rs` must call `initialize_from_history()`.
- API routes without proper error responses — always use `success_response()`/`error_response()`.
- Adding new safety checks without integrating into entry/exit evaluators.
- Telegram callback authentication — all trading callbacks must be in `is_sensitive_callback` check.
- Telegram pagination — use 1-indexed pages for user-facing display.
- Token vs Position operations — need separate handlers. Don't route `token:blacklist` to position-based handler.
- Token struct field names — `volume_h24` not `volume_24h`, `security_score_normalised` not `risk_score`.
- Position state updates by mint vs ID — `update_position_state()` finds by MINT (first match). Use `update_position_state_by_id()` when multiple positions exist for same token.
- Context menu events need listeners registered at module load time.
- Electron kill timeout — ServiceManager has 10s per-task timeout. Electron must wait 30s+ before SIGKILL.
- Manual trading validation — must check Force Stop first, then Core Services Ready.
- Unix signal handling — handle SIGINT, SIGTERM, SIGHUP, and SIGQUIT.
- Task spawning in loops — Never spawn individual tokio tasks in loops with 100k+ iterations. Use batch tasks.
- Token struct cloning — Token is ~2KB. Use `Arc<Token>` instead of cloning.
- Filtering loads only tokens with market data (`only_with_market_data=true`).
- Market data permanent failure — 3 empty DexScreener responses marks token as permanently failed.
- Lucide icon naming — verify icon names exist in `lucide.css` (some were renamed in newer versions).
- Emoji policy — Dashboard and Rust logs must NOT use emojis. Only Telegram module may use emojis.
- `TradeActionDialog` quick mode — use `mode: "quick"` to show mint input step.
- External libraries — access as `window.LibraryName` for ESLint compatibility.
- AI initialization order — init after config loads but before filtering/trading services.
- AI engine singleton — `try_get_ai_engine()` returns Option (safe), `get_ai_engine()` panics if not initialized.
- AI response parsing — use `with_json_mode()` on ChatRequest and `validate_json_response()`.
- AI cache priority — High bypasses cache (trading), Medium uses recent (trailing stop), Low always uses cache (filtering).
- Loading all closed positions from DB is O(n) memory — use SQL aggregation for stats (`get_period_trading_stats`).
- `gather_status_snapshot()` runs 9+ parallel queries — only call it when you need ALL the data (e.g., `/api/status`). Use targeted collectors for specific needs (`get_services_status()`, `get_metrics_status()`).

---

## Build & Development

### Build Commands

```bash
cargo check --lib          # Fast type-checking (no binaries)
cargo build                # Debug build
cargo build --release      # Release build
cargo clippy               # Lint checks
```

### Frontend Validation

Templates are embedded at compile time — `cargo check` won't catch HTML/CSS/JS errors:

```bash
npm run check              # Validate all (ESLint + Stylelint + HTML-Validate)
npm run lint:js            # JavaScript only
npm run lint:css           # CSS only
npm run lint:html          # HTML only
```

### Formatting

```bash
npm run format             # Format all (JS/CSS/HTML/JSON/MD/TOML/Rust)
cargo fmt --check          # Check Rust formatting
```

### Debug Binaries

`src/bin/debug_*` for inspection. Support `--help`, `--cache-only`. Examples: `debug_pool_decoders`, `debug_decoder_validation`, `debug_events`.

---

## Reference

### Quick Patterns

- Config access: `with_config(|cfg| cfg.trader.max_positions)` (sync), `get_config_clone()` (async).
- RPC: `get_rpc_client()` + RpcClientMethods trait; keep jsonParsed.
- Events: Use helpers in `src/events/` with LogTag.
- Pricing: Follow `pools/calculator.rs` single-pool invariant.
- Error handling: `Result<T, String>` for simple flows; typed errors from `src/errors/` for complex async.
- AI: Use `try_get_ai_engine()` for safe access, `evaluate_filter()` with Low priority, `evaluate_entry()`/`evaluate_exit()` with High priority.

### Verification Checklist

- [ ] RPC via `get_rpc_client()`; encoding is jsonParsed
- [ ] No raw SQL; use module DB helpers
- [ ] Services via ServiceManager/AppState
- [ ] Logging uses LogTag with full identifiers
- [ ] UI changes are metadata-driven
- [ ] Frontend validated: `npm run check`
- [ ] Code formatted: `npm run format`
- [ ] Build passes: `cargo check --lib` then `cargo build`
- [ ] Dashboard uses CSS variables (no hardcoded colors/sizes)
- [ ] Page modules use lifecycle hooks with `ctx.managePoller()` for cleanup

### Frontend Quick Reference

**Utils:** formatSol, formatNumber, formatCompactNumber, formatCurrencyUSD, formatPercent, formatDuration, formatTimestamp, formatPnL, debounce, throttle, copyToClipboard, showToast

**DOM:** $(selector), $$(selector), el(id), on(el, event, handler), off(el, event, handler), cls(el, classMap), create(tag, attrs, content), show(el), hide(el)

**Lifecycle:** ctx.managePoller(poller), ctx.manageTabBar(tabBar), ctx.manageActionBar(actionBar), ctx.onDeactivate(callback), ctx.onDispose(callback), ctx.isActive()

**Keyboard Shortcuts:** Ctrl+B (Cmd+B on Mac) for Quick Buy, Ctrl+Shift+S (Cmd+Shift+S on Mac) for Quick Sell.

---

## Code Quality

- Always remove obsolete code: Delete unused functions, stale comments, and anything not serving current architecture.
- Never create compatibility layers, legacy wrappers, or `_v2` variants.
- Fix problems systematically and fundamentally at their source.
- NEVER leave "dust comments" — when deleting obsolete code, remove it completely.
- Always investigate deeply before making changes.
- Maintain a single `docs/` directory as the only location for documentation.
- Each feature must have exactly one dedicated document.
- Always look for duplicated files, logic, helpers, or documentation and consolidate.
