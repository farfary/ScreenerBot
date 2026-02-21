# Wallets Module — Architecture

> Secure multi-wallet key management (`wallets.db`) + main wallet balance history and dashboard cache (`wallet.db`) — February 2026

---

## Table of Contents

1. [Overview](#1-overview)
2. [Key Terminology](#2-key-terminology)
3. [File Structure](#3-file-structure)
4. [Multi-Wallet System (`crate::wallets`)](#4-multi-wallet-system-cratewallets)
5. [Wallets DB (`wallets.db`)](#5-wallets-db-walletsdb)
6. [Key Encryption (`secure_storage`)](#6-key-encryption-secure_storage)
7. [Wallet Consistency Validation (`wallet_validation`)](#7-wallet-consistency-validation-wallet_validation)
8. [Wallet Balance Monitor (`crate::wallet`)](#8-wallet-balance-monitor-cratewallet)
9. [Wallet Monitor DB (`wallet.db`)](#9-wallet-monitor-db-walletdb)
10. [Dashboard Metrics Caching](#10-dashboard-metrics-caching)
11. [Webserver + Tools Integration](#11-webserver--tools-integration)
12. [Module Connections](#12-module-connections)

---

## 1. Overview

ScreenerBot has **two wallet-related subsystems**:

1. **Multi-wallet key management** (`crate::wallets`)
   - Stores **multiple wallets + encrypted private keys** in `wallets.db`.
   - Provides a **main-wallet fast path** (`get_main_keypair()`) for signing.
   - Exposes CRUD + bulk import/export for UI/tools.

2. **Main wallet monitoring / history** (`crate::wallet` = `wallets::balance_monitor`)
   - Periodically snapshots the **current main wallet** (SOL, token balances, NFT balances) into
     `wallet.db`.
   - Computes and caches **wallet dashboard metrics** (flows, trends, top tokens, NFTs).

These are intentionally separate:

- `wallets.db` is "key storage + multi-wallet metadata".
- `wallet.db` is "time series history + dashboard cache for the current wallet".

### Design goals

- **Private keys encrypted at rest** (AES-256-GCM, machine-derived key).
- **No private keys in logs** (bulk import/export is explicit and warns).
- **Main wallet keypair retrieval is fast** (in-memory cache avoids repeated decrypt).
- **Wallet-dependent databases are not silently mixed**: wallet changes are detected early (wallet
  consistency validation).

### Non-goals (current implementation)

- No always-on "multi-wallet balance monitor" service for every wallet in `wallets.db`.
  - Balance caching for secondary wallets is **explicit / on-demand** via `wallets::manager` APIs.

---

## 2. Key Terminology

### "Main wallet"

A wallet with `WalletRole::Main` in `wallets.db`. Most runtime subsystems treat it as "the wallet".

### `wallets.db` vs `wallet.db` (critical)

- `wallets.db`
  - Created by: `src/wallets/database.rs`
  - Path: `paths::get_data_directory().join("wallets.db")`
  - Purpose: **multi-wallet registry + encrypted private keys** + cached per-wallet token balances.

- `wallet.db`
  - Created by: `src/wallets/balance_monitor/database.rs`
  - Path: `paths::get_wallet_db_path()` -> `.../data/wallet.db`
  - Purpose: **time-series snapshots** and **precomputed dashboard payloads** for the current
    wallet.

### `crate::wallet` and `crate::wallet_validation` re-exports

In `src/lib.rs`:

```rust
pub use wallets::balance_monitor as wallet;
pub use wallets::validation as wallet_validation;
```

So most call sites use:

- `crate::wallet::...` for monitoring/dashboard (uses `wallet.db`)
- `crate::wallets::...` for multi-wallet key management (uses `wallets.db`)
- `crate::wallet_validation::WalletValidator` for mismatch detection

---

## 3. File Structure

```text
src/wallets/
├── mod.rs                      Re-exports + public module docs
├── types.rs                    Multi-wallet types (Wallet, WalletRole, TokenBalance, ...)
├── crypto.rs                   Keypair generation + import/export glue (uses secure_storage)
├── database.rs                 wallets.db schema + r2d2 pool
├── manager.rs                  Multi-wallet manager (global DB + main wallet cache)
├── validation.rs               Wallet consistency validator (transactions/positions/wallet.db)
├── bulk/
│   ├── mod.rs                  CSV/Excel import/export API
│   ├── parser.rs               CSV/Excel parsing
│   ├── types.rs                Bulk import row/result types
│   └── validator.rs            Row validation + preview builder
└── balance_monitor/            "Main wallet monitoring" subsystem (exported as crate::wallet)
    ├── mod.rs
    ├── types.rs                Snapshot + dashboard payload types
    ├── service.rs              Background task (snapshots + flow cache + metrics recompute)
    ├── database.rs             wallet.db schema + r2d2 pool + global instance
    ├── dashboard.rs            WalletDashboardData computation (realtime + cached)
    ├── cache.rs                Memory cache + gzip payload + circuit breaker
    └── utils.rs                Helper utilities for dashboard computation
```

---

## 4. Multi-Wallet System (`crate::wallets`)

This is the "key management" layer used by:

- swap/trade execution (signing)
- tools (multi-buy/sell, consolidation)
- webserver wallet CRUD APIs

### 4.1 Global state + concurrency model

`src/wallets/manager.rs` uses two global singletons:

```rust
static WALLETS_DB: LazyLock<Arc<RwLock<Option<WalletsDatabase>>>>;
static MAIN_WALLET_CACHE: LazyLock<Arc<RwLock<Option<CachedMainWallet>>>>;
```

- `WALLETS_DB` owns the `WalletsDatabase` (r2d2 pool) after `wallets::initialize()`.
- `MAIN_WALLET_CACHE` stores:
  - `wallet: Wallet` (metadata)
  - `keypair: Keypair` (decrypted keypair)

This avoids repeatedly decrypting the main wallet key on every trade.

### 4.2 Startup / initialization flow

In normal mode (`src/run.rs`):

```text
config::load_config()
  -> wallets::initialize()
      -> WalletsDatabase::new()           // opens/creates wallets.db
      -> migrate_from_config()            // first-run bridge from legacy config
      -> refresh_main_wallet_cache()      // decrypt main wallet + cache it
  -> wallet_validation::validate_wallet_consistency()
```

#### migrate_from_config()

This is a one-time bridge for older installs:

- If `wallets.db` has **0 wallets**
- And `config.toml` contains legacy `wallet_encrypted` + `wallet_nonce`
- Then `wallets.db` inserts a new wallet:
  - `name`: `"Main Wallet"`
  - `role`: `WalletRole::Main`
  - `wallet_type`: `WalletType::Migrated`
  - `encrypted_key` + `nonce`: copied from config

It decrypts the legacy key only to compute the Solana address (base58 pubkey).

### 4.3 Main wallet "fast path"

The keypair is not `Clone`, so `get_main_keypair()` returns a new `Keypair` by cloning bytes:

```rust
let bytes = cached.keypair.to_bytes();
Keypair::from_bytes(&bytes)
```

Key APIs:

- `wallets::get_main_keypair() -> Result<Keypair, String>`
- `wallets::get_main_address() -> Result<String, String>`
- `wallets::get_main_wallet() -> Result<Option<Wallet>, String>`
- `wallets::has_main_wallet() -> bool`

Cache invalidation happens when:

- setting a wallet as main (`set_main_wallet`)
- updating a wallet with role change to main
- startup initialization (`refresh_main_wallet_cache`)

### 4.4 CRUD + role semantics

Core CRUD is database-backed and modeled as:

- active/inactive: `wallets.is_active` (0/1)
- "archive" role: `WalletRole::Archive`

Important manager entrypoints (non-exhaustive):

- Create:
  - `create_wallet(CreateWalletRequest)`
  - `create_wallets_batch(count, prefix, notes)` (hard-capped at 100)
- Read:
  - `get_wallet(id)`
  - `get_wallet_by_address(address)`
  - `list_wallets(include_inactive)`
  - `list_active_wallets()`
- Update:
  - `update_wallet(id, UpdateWalletRequest)`
  - `set_main_wallet(id)` (promotes wallet, demotes previous main)
- Lifecycle:
  - `archive_wallet(id)`
  - `restore_wallet(id)`
  - `delete_wallet(id)` (permanent; cascades token balances)

### 4.5 Import / export (sensitive)

All import/export code is explicit and treated as sensitive:

- `import_wallet(ImportWalletRequest)`
  - parses private key (base58 or `[1,2,...]`)
  - normalizes storage to base58
  - encrypts with machine-bound key
  - inserts wallet record
- `export_wallet(wallet_id) -> ExportWalletResponse`
  - decrypts private key and returns it as base58
- `export_wallets(include_inactive) -> Vec<WalletExportRow>`
  - iterates wallets and decrypts each key
  - logs a warning: `"Exported N wallets - SENSITIVE DATA"`
- Bulk import (`bulk_import_wallets`)
  - computes wallet address from keypair without logging the key
  - supports duplicate skipping via `ImportOptions`

### 4.6 Per-wallet token balance caching (on-demand)

`wallets.db` includes `wallet_token_balances` as a convenience cache for tools/UI.

Key points:

- Updates are triggered explicitly (no background service).
- Token accounts are fetched from RPC and stored in `wallets.db`.
- NFTs are excluded (`filter(|acc| !acc.is_nft)`).

Important APIs:

- `update_wallet_balances(wallet_id) -> Result<usize, String>`
- `update_all_wallet_balances() -> Result<HashMap<i64, usize>, String>`
- `get_token_balances(wallet_id) -> Result<Vec<TokenBalance>, String>`
- `upsert_token_balance(...)` for incremental updates

### 4.7 Wallet discovery helpers (tools/UI)

Example: `get_wallets_with_token(token_mint, min_balance)`:

- iterates active wallets
- for each wallet:
  - queries SOL balance (`rpc.get_sol_balance`)
  - fetches token accounts (`rpc.get_all_token_accounts`)
  - finds a matching mint and reports balance + whether SOL top-up is needed

---

## 5. Wallets DB (`wallets.db`)

**File:** `src/wallets/database.rs`  
**Path:** `paths::get_data_directory().join("wallets.db")`

### 5.1 Connection management

- r2d2 pool size: `max_size(3)`
- `idle_timeout(None)` + `max_lifetime(None)` (SQLite stability; no recycling)
- Connection init: `database::configure_connection(c, database::WALLETS_DB)`

### 5.2 Schema

`wallets` (stores encrypted key material):

| Column | Type | Notes |
|---|---|---|
| id | INTEGER PK AUTOINCREMENT | internal ID |
| name | TEXT | user-facing |
| address | TEXT UNIQUE | base58 pubkey |
| encrypted_key | TEXT | base64 ciphertext (AES-256-GCM) |
| nonce | TEXT | base64 12-byte nonce |
| role | TEXT | main / secondary / archive |
| wallet_type | TEXT | generated / imported / migrated |
| created_at | TEXT | RFC3339 string |
| last_used_at | TEXT | nullable |
| notes | TEXT | nullable |
| is_active | INTEGER | 0/1 |

`wallet_token_balances` (cache of holdings per wallet):

| Column | Type | Notes |
|---|---|---|
| wallet_id | INTEGER | FK wallets(id), cascade delete |
| mint | TEXT | token mint |
| balance | INTEGER | raw u64 |
| ui_amount | REAL | decimals applied |
| decimals | INTEGER | token decimals |
| symbol | TEXT | optional enrichment |
| name | TEXT | optional enrichment |
| is_token_2022 | INTEGER | 0/1 |
| updated_at | TEXT | RFC3339 |

Primary key: `(wallet_id, mint)`

### 5.3 Indexes

Created by `WALLETS_INDEXES`:

- `idx_wallets_address`
- `idx_wallets_role`
- `idx_wallets_active`
- `idx_token_balances_wallet`
- `idx_token_balances_mint`

---

## 6. Key Encryption (`secure_storage`)

**Primary implementation:** `src/secure_storage.rs`  
**Wallet glue:** `src/wallets/crypto.rs`

### 6.1 What is encrypted?

- The Solana `Keypair` is serialized as the standard **64-byte private key bytes**.
- Those bytes are stored as a **base58 string** (normalized) before encryption.

### 6.2 Encryption scheme

- Cipher: **AES-256-GCM**
- Nonce: random **12 bytes**
- Storage format:
  - `EncryptedData.ciphertext`: base64 encoded ciphertext (includes auth tag)
  - `EncryptedData.nonce`: base64 encoded nonce

### 6.3 Key derivation (machine-bound)

The encryption key is derived from:

- machine unique ID (`machine_uid::get()` on desktop platforms)
- app salt: `screenerbot-wallet-encryption-v1`
- hashed via BLAKE3 to a 32-byte key

**Implication:** encrypted keys can only be decrypted on the same machine (unless exported and
re-imported).

### 6.4 Import/export formats

`wallets::crypto::parse_private_key()` supports:

- base58 encoded 64-byte key
- JSON array format: `[1,2,3,...]` (length must be 64)

Export always returns base58 (normalized).

---

## 7. Wallet Consistency Validation (`wallet_validation`)

**File:** `src/wallets/validation.rs` (re-exported as `crate::wallet_validation`)

Goal: prevent mixing wallet-dependent datasets when the "current wallet" changes.

### 7.1 What is validated?

At startup, `WalletValidator::validate_wallet_consistency()` compares:

- current wallet address: `utils::get_wallet_address()`
  - -> `config::get_wallet_pubkey_string()`
  - -> `config::get_wallet_keypair()`
  - -> (prefer) `wallets::get_main_keypair()` if wallets module initialized

Against wallet address stored in metadata tables:

| Database | Path helper | Metadata table |
|---|---|---|
| transactions history | `paths::get_transactions_db_path()` | `db_metadata` |
| positions | `paths::get_positions_db_path()` | `position_metadata` |
| wallet monitor history | `paths::get_wallet_db_path()` | `wallet_metadata` |

It reads: `SELECT value FROM <table> WHERE key = 'current_wallet'`.

### 7.2 Results

- `Valid` -> proceed
- `FirstRun` -> no DBs exist yet
- `Mismatch { current, stored, affected_systems }` -> hard error
  - user must clean wallet-dependent DBs before continuing

### 7.3 Cleanup helper

`WalletValidator::clean_all_databases()` deletes:

- `transactions.db` (+ WAL/SHM)
- `positions.db` (+ WAL/SHM)
- `wallet.db` (+ WAL/SHM)

It intentionally does **not** delete `wallets.db` (keys registry).

---

## 8. Wallet Balance Monitor (`crate::wallet`)

This subsystem is a **background service** that maintains time-series wallet history and
dashboard-friendly aggregates.

It is exported as `crate::wallet` via `src/lib.rs` and started by:

- `src/services/implementations/wallet_service.rs` (`Service::name() == "wallet"`)

### 8.1 What wallet is monitored?

The monitor tracks the **current wallet address** returned by:

- `utils::get_wallet_address()`

Which resolves the configured wallet keypair via:

- multi-wallet system (preferred), or
- legacy config fallback

So: changing the main wallet changes which wallet gets monitored.

### 8.2 Background loop (`start_wallet_monitoring_service`)

**File:** `src/wallets/balance_monitor/service.rs`

The service runs a `tokio::select!` loop with multiple intervals:

1. **Snapshot interval** (`cfg.wallet.snapshot_interval_secs`, min 10s)
2. **SOL flow cache sync interval** (`cfg.wallet.flow_cache_update_secs`, min 1s)
3. **Dashboard recompute intervals** (24h / 7d / 30d / all-time)
4. Shutdown via `Notify`

#### Snapshot collection

`collect_wallet_snapshot()`:

- RPC calls (with small delays to reduce burstiness):
  - `rpc.get_sol_balance(wallet_address)`
  - `rpc.get_all_token_accounts_str(wallet_address)` (tokens + NFTs)
- Separates:
  - fungible tokens (`TokenBalance`)
  - NFTs (`NftBalance`) and fetches metadata via `nfts::fetch_nft_metadata_batch(...)`
- Saves to `wallet.db` with:
  - one `wallet_snapshots` row
  - N `token_balances` rows
  - M `nft_balances` rows
- Updates an in-memory "snapshot readiness" cache:
  - `cache::update_wallet_snapshot_status(snapshot_time)`

#### Periodic cleanup

Every 60 snapshot ticks (about 1 hour for default 60s snapshots):

- deletes old snapshots (`cleanup_old_snapshots_sync`)
- deletes expired cached dashboard payloads (`cleanup_expired_metrics`)

#### SOL flow cache sync

On each tick:

- reads current `max(timestamp)` from `wallet.db.sol_flow_cache`
- subtracts a configurable lookback (`cfg.wallet.flow_cache_lookback_secs`)
- exports processed transaction rows from transactions DB:
  - `transactions_db.export_processed_for_wallet_flow(start_ts, batch_size)`
- upserts them into `wallet.db.sol_flow_cache`

This lets dashboard flow metrics aggregate over `wallet.db` first and only fall back to full
transactions DB aggregation when needed.

### 8.3 Public APIs

Service / DB accessors include:

- snapshots:
  - `get_recent_wallet_snapshots(limit)`
  - `get_snapshot_token_balances(snapshot_id)`
  - `get_snapshot_nft_balances(snapshot_id)`
  - `get_current_wallet_status()`
- metrics:
  - `get_wallet_monitor_stats()`
  - `get_balance_at_time(target_time)`
  - `get_flow_cache_stats()`
- dashboard caching controls:
  - `refresh_dashboard_cache(window_hours)`
  - `get_dashboard_cache_metrics()`
  - `clear_dashboard_api_cache()`

---

## 9. Wallet Monitor DB (`wallet.db`)

**File:** `src/wallets/balance_monitor/database.rs`  
**Path:** `paths::get_wallet_db_path()` -> `.../data/wallet.db`

### 9.1 Connection management

- r2d2 pool size: `max_size(3)`
- `min_idle(Some(1))`
- `idle_timeout(None)` + `max_lifetime(None)`
- Connection init: `database::configure_connection(c, database::WALLET_MONITOR_DB)`

### 9.2 Schema (high level)

Schema version constant: `WALLET_SCHEMA_VERSION: u32 = 3`

Tables:

1. `wallet_snapshots`
   - one row per snapshot time
2. `token_balances`
   - fungible token balances for a snapshot (FK -> wallet_snapshots)
3. `nft_balances`
   - NFT holdings + lightweight metadata (FK -> wallet_snapshots)
4. `wallet_metadata`
   - generic key/value metadata table
   - includes `schema_version` and `current_wallet`
5. `sol_flow_cache`
   - pre-aggregated SOL delta per processed transaction signature
6. `wallet_dashboard_metrics`
   - precomputed, compressed dashboard payloads for canonical windows

Indexes exist for:

- snapshot by time/address
- token/nft by snapshot/mint
- flow cache by timestamp
- dashboard metrics by valid_until

---

## 10. Dashboard Metrics Caching

Dashboard computation lives in:

- `src/wallets/balance_monitor/dashboard.rs`
- `src/wallets/balance_monitor/cache.rs`

The endpoint-level API is:

- `get_wallet_dashboard_data(window_hours, snapshot_limit, max_tokens) -> WalletDashboardData`

### 10.1 Cache layers (fast -> slow)

1. **In-memory API response cache** (`moka`)
   - key: `(window_hours, snapshot_limit, max_tokens)`
   - stores a fully materialized `WalletDashboardData`
   - has a fixed moka TTL (300s) plus an additional config freshness gate:
     - `cfg.wallet.api_response_cache_ttl_secs` (min 5s)
     - effective "fresh hit" window is `min(cfg, 300s)` (moka may keep the entry longer than the
       config gate, but the code will stop using it)

2. **Database precompute cache** (`wallet_dashboard_metrics`)
   - only for canonical windows:
     - 24h (24), 7d (168), 30d (720), all_time (0)
   - stores a gzip-compressed JSON payload:
     - `payload_format = "json-gzip"`

3. **Realtime computation**
   - loads snapshots + balances, computes flow metrics, enriches tokens, etc.

### 10.2 Compression format

`cache.rs` implements:

- `serialize_dashboard_payload(payload) -> Vec<u8>`
  - clone payload, strip `cache_metadata`
  - `serde_json::to_vec`
  - gzip (`flate2`, `Compression::fast`)
- `deserialize_dashboard_payload(raw) -> WalletDashboardData`
  - gunzip
  - serde_json decode

### 10.3 Circuit breaker (recompute protection)

`cache.rs` tracks repeated failures per window key:

- threshold: 3 failures
- cooldown: 300s

After repeated failures, background recomputation is skipped temporarily to avoid hot-looping on an
expensive compute path.

### 10.4 Flow metrics + fallbacks

Flow metrics are computed from:

- cached aggregation over `wallet.db.sol_flow_cache` (preferred), else
- live aggregation from `transactions.db` via `aggregate_sol_flows_since(...)`

All-time mode (`window_hours <= 0`) attempts:

- cached aggregation from min cached timestamp, else
- full aggregation from epoch via transactions DB

### 10.5 Payload sizing controls

To keep responses reasonable:

- snapshot_limit is clamped (16..=2880)
- token_limit is clamped (10..=1000)
- daily flow series is capped/decimated via config:
  - `cfg.wallet.max_daily_flow_days`
  - `cfg.wallet.daily_flow_decimate_threshold_days`

---

## 11. Webserver + Tools Integration

### 11.1 Webserver routes

- Multi-wallet CRUD + bulk operations:
  - `src/webserver/routes/wallets/**`
  - calls `crate::wallets::*` manager APIs
- Wallet monitor endpoints (history + dashboard):
  - `src/webserver/routes/wallet.rs`
  - calls `crate::wallet::*` (balance monitor)

### 11.2 Tools

The tools layer uses multi-wallet APIs for operations involving multiple wallets:

- `src/tools/multi_wallet/buy.rs`
- `src/tools/multi_wallet/sell.rs`

These typically:

- pull main keypair via `wallets::get_main_keypair()`
- list other wallets + keypairs via `wallets::get_wallets_with_keys()`

### 11.3 Services

- `WalletService` (service manager)
  - name: `"wallet"`
  - enabled only after initialization complete
  - starts: `crate::wallet::start_wallet_monitoring_service(...)`
  - exposes metrics: operations/errors/snapshots_taken/flow_syncs

---

## 12. Module Connections

```text
wallets/
├── config/           current wallet resolution + legacy migration source
├── secure_storage/   encryption/decryption for key material
├── database/         centralized SQLite PRAGMA configuration
├── rpc/              token account + balance queries
├── nfts/             NFT metadata enrichment (wallet monitor)
├── transactions/     SOL flow aggregation + export_processed_for_wallet_flow()
├── services/         WalletService starts the monitor task
└── webserver/        CRUD + dashboard endpoints
```

### Pitfalls / gotchas

- **Do not confuse `wallets.db` with `wallet.db`.**
  - `wallets.db` is key registry; `wallet.db` is history/cache for the current wallet.
- **Keys are machine-bound.**
  - Copying `wallets.db` to another machine will not make keys decryptable.
  - Export + import is the supported transfer path.
- **Wallet mismatch is fatal (by design).**
  - If the configured/main wallet changes, you must clean wallet-dependent DBs
    (transactions/positions/wallet history) before continuing.
