# Config Module — Architecture

> ScreenerBot TOML configuration + UI metadata system — February 2026

---

## Table of Contents

1. [Overview](#1-overview)
2. [File Structure](#2-file-structure)
3. [Schema Definition Model (`config_struct!`)](#3-schema-definition-model-config_struct)
4. [Root `Config` Composition](#4-root-config-composition)
5. [Global Storage + Access Patterns](#5-global-storage--access-patterns)
6. [Load / Reload / Persistence Flows](#6-load--reload--persistence-flows)
7. [Validation Model](#7-validation-model)
8. [UI Metadata System](#8-ui-metadata-system)
9. [GUI Migration: Navigation Tabs](#9-gui-migration-navigation-tabs)
10. [Wallet Credentials + Keypair Access](#10-wallet-credentials--keypair-access)
11. [Webserver Integration](#11-webserver-integration)
12. [Module Connections](#12-module-connections)

---

## 1. Overview

The `config` module is ScreenerBot's **single source of truth** for runtime settings.

It provides:

- A strongly typed Rust `Config` struct (plus many nested section structs)
- Serialization/deserialization to a user-editable **TOML file** (`data/config.toml`)
- A global, thread-safe in-memory instance (`OnceLock<RwLock<Config>>`)
- A **metadata system** so the web dashboard can render config forms automatically
- Hot reload helpers (reload + validate + atomic replace)

Safety-sensitive execution remains opt-in: a newly generated `TraderConfig` sets the Auto Trader
master switch (`trader.enabled`) to `false`. Entry and exit monitor preferences may be configured
independently, but they cannot execute until the user explicitly enables the master switch.

### Design goals

- **Zero repetition**: default values, serde defaults, and UI metadata are declared once in schema
  definitions and code is generated via macros.
- **Stable, typed reads**: most modules should read config via `with_config(|cfg| ...)`.
- **Hot reload**: config changes can be applied without restarting the process.
- **UI-driven editing**: the backend can expose field metadata (labels, hints, min/max, categories)
  to the dashboard renderer.

### Non-goals (current implementation)

- No automatic file watcher inside the `config` module.
  - Reload happens only when explicitly triggered (e.g. webserver route calls `reload_config()`).

---

## 2. File Structure

```text
src/config/
├── mod.rs              Public API + re-exports
├── macros.rs           `config_struct!` macro (struct + Default + metadata generator)
├── metadata.rs         Field metadata types + `field_metadata!` helper + metadata collection
├── utils.rs            Load/reload/save + access helpers + validation + wallet key helpers
└── schemas/
    ├── mod.rs          Root `Config` struct + section module wiring
    ├── trader.rs       Trading settings (large; most validation targets this)
    ├── filtering.rs    Filtering settings (large; many UI fields)
    ├── rpc.rs          RPC provider selection + URLs + rate limiting
    ├── gui.rs          Dashboard settings + navigation tabs migration logic
    ├── webserver.rs    Webserver/auth config
    ├── telegram.rs     Telegram bot settings
    ├── tokens.rs       Token tracking settings
    ├── pools.rs        Pool discovery settings
    ├── positions.rs    Position management settings
    ├── ohlcv.rs        OHLCV retention/fetch settings
    ├── sol_price.rs    SOL price service config
    ├── ai.rs           AI providers + cache toggles
    ├── services.rs     Service toggles
    ├── events.rs       Event recording toggles
    ├── connectivity.rs Connectivity monitor config
    ├── wallet.rs       Wallet UX-related config (not key material)
    ├── ...             (other smaller schema files)
```

---

## 3. Schema Definition Model (`config_struct!`)

**Core macro:** `src/config/macros.rs`

All config section structs (including root `Config`) are defined with:

```rust
config_struct! {
  pub struct TraderConfig {
    // Optional doc comments are captured as metadata docs
    /// Maximum simultaneous positions
    #[metadata(field_metadata! {
      label: "Max Open Positions",
      hint: "Maximum simultaneous positions",
      min: 1, max: 100,
      category: "Core Trading",
    })]
    max_open_positions: u32 = 5,
  }
}
```

### What the macro generates

For every `config_struct! { struct X { ... } }`, the macro generates:

1. A struct with `#[derive(Debug, Clone, Serialize, Deserialize)]` and `#[serde(default)]`
2. An `impl Default for X` using the inline defaults
3. `FieldTypeInfo` and `NestedMetadata` trait impls used by the metadata system
4. An `X::field_metadata() -> SectionMetadata` constructor that:
   - captures default values
   - captures doc comments as `docs`
   - applies optional `#[metadata(...)]` extras (via `field_metadata!`)
   - recursively attaches `children` metadata for nested object fields

### Serde defaults behavior

Because the generated struct has `#[serde(default)]`:

- Missing fields in TOML will be filled using `Default::default()` for the struct.
- This is how the system supports forward-compatible schema growth without breaking older configs.

---

## 4. Root `Config` Composition

**File:** `src/config/schemas/mod.rs`

The root struct is also generated via `config_struct!` and contains:

- two top-level encrypted wallet credential fields:
  - `wallet_encrypted: String`
  - `wallet_nonce: String`
- plus nested section structs (RPC, trader, filtering, webserver, etc.)

At a high level, the root config looks like:

```text
Config
├── wallet_encrypted + wallet_nonce      (encrypted key material; base64 strings)
├── rpc                                 (RPC providers + limits)
├── trader                              (core trading)
├── positions                           (position lifecycle settings)
├── filtering                            (token filtering)
├── swaps                               (swap execution settings)
├── tokens                              (token tracking)
├── pools                               (pool discovery)
├── sol_price                           (SOL price service)
├── events                              (event recording)
├── services                            (service toggles)
├── monitoring                          (system monitoring)
├── connectivity                        (connectivity monitor)
├── ohlcv                               (OHLCV retention/fetch)
├── wallet                              (wallet UX; not key material)
├── strategies                          (strategy toggles/settings)
├── gui                                 (dashboard UI + navigation)
├── webserver                           (HTTP API/auth)
├── telegram                            (bot token/chat id/options)
├── holder_watch                        (tool settings)
├── ai                                  (LLM + AI tooling settings)
├── performance                         (cache/memory knobs)
└── maintenance                         (retention + vacuum scheduling)
```

---

## 5. Global Storage + Access Patterns

**Global instance:** `src/config/utils.rs`

```rust
pub static CONFIG: OnceLock<RwLock<Config>> = OnceLock::new();
```

Important details:

- `OnceLock` means config is initialized once via `load_config_*` and thereafter mutated via
  `reload_config_*` or `update_config_section`.
- The lock is a `std::sync::RwLock` (not `tokio::sync::RwLock`), so reads/writes are synchronous
  and should stay small/fast.

### Recommended read path: `with_config`

```rust
pub fn with_config<F, R>(f: F) -> R
where
  F: FnOnce(&Config) -> R
```

This acquires a read lock and runs the closure, returning its value.

### Async-safe read path: `get_config_clone`

```rust
pub fn get_config_clone() -> Config {
  with_config(|cfg| cfg.clone())
}
```

This is intended for holding config across `.await` points without keeping a lock.

### Write path: `update_config_section`

```rust
pub fn update_config_section<F>(update_fn: F, save_to_disk: bool) -> Result<(), String>
where
  F: FnOnce(&mut Config)
```

Key behavior:

- Acquires a write lock and runs `update_fn(&mut config)`
- Releases lock
- Optionally persists via `save_config(None)` (does not hold the lock while writing)

**Important:** `update_config_section` does not call `validate_config()` internally; validation is
the responsibility of the caller.

---

## 6. Load / Reload / Persistence Flows

All path defaults come from the `paths` module:

- `paths::get_config_path()` -> `.../data/config.toml`

### 6.1 Initial load: `load_config_from_path`

**File:** `src/config/utils.rs`

High-level behavior:

1. If TOML file exists:
   - `std::fs::read_to_string(path)`
   - `toml::from_str::<Config>(&contents)`
2. Else:
   - log warning
   - use `Config::default()`
3. Always run a GUI migration helper:
   - `ensure_all_tabs_present(cfg.gui.dashboard.navigation.tabs)`
4. Store into global `CONFIG` via `CONFIG.set(RwLock::new(config))`

Notes:

- `load_config_from_path` does **not** call `validate_config()`.
- It will fail if called twice (OnceLock already set); use reload APIs instead.

### 6.2 Hot reload: `reload_config_from_path`

Hot reload does:

1. Read TOML
2. Parse to `Config`
3. Run `ensure_all_tabs_present(...)`
4. `validate_config(&new_config)?`
5. Acquire write lock and atomically replace the entire `Config`

This means reload is "all-or-nothing": if parse or validation fails, the in-memory config is not
changed.

### 6.3 Persistence: `save_config` vs `save_config_to_file`

There are two main persistence helpers:

1. `save_config(path: Option<&str>)`
   - Serializes the in-memory config (`toml::to_string_pretty`)
   - Writes with `std::fs::write`
   - Does not create parent directories
   - Does not set file permissions
   - Does not validate

2. `save_config_to_file(config: &Config, path: &str, set_global: bool)`
   - Validates config before writing (`validate_config(config)?`)
   - Ensures parent directory exists
   - Writes TOML
   - On Unix: sets `0600` permissions (rw-------)
   - Optionally initializes or reloads the global CONFIG

---

## 7. Validation Model

Validation lives in `src/config/utils.rs` as:

```rust
pub fn validate_config(config: &Config) -> Result<(), String>
```

### 7.1 What is validated?

Validation is currently focused on:

- trader core invariants (e.g. `max_open_positions > 0`, trade size finite and > 0)
- conditional DCA constraints (thresholds, sizes, counts)
- ROI exit constraints (finite and > 0)
- time override constraints (unit must parse, max 30 days, loss threshold bounds)
- stop loss bounds
- positions constraints (cooldowns, partial exit bounds, trailing stop bounds, etc.)

### 7.2 When validation is applied

- Applied on reload (`reload_config_from_path`)
- Applied on save-to-file helper used during initialization (`save_config_to_file`)
- Not applied by default on:
  - initial `load_config_from_path`
  - `update_config_section` (unless caller validates)
  - `save_config`

This is important when designing webserver config mutations: callers should validate before
persisting/using config changes.

---

## 8. UI Metadata System

Metadata exists so the backend can describe how a config field should be rendered in the dashboard.

**Primary file:** `src/config/metadata.rs`

### 8.1 Core types

```rust
pub enum FieldType {
  Boolean,
  Number,
  Integer,
  Array,
  String,
  Object,
}

pub struct FieldMetadata {
  pub field_type: FieldType,
  pub item_type: Option<FieldType>,
  pub label: Option<&'static str>,
  pub hint: Option<&'static str>,
  pub unit: Option<&'static str>,
  pub impact: Option<&'static str>,
  pub category: Option<&'static str>,
  pub visibility: Option<&'static str>,   // derived ("primary" | "secondary" | "technical")
  pub min: Option<f64>,
  pub max: Option<f64>,
  pub step: Option<f64>,
  pub placeholder: Option<&'static str>,
  pub docs: Option<&'static str>,
  pub default: Option<serde_json::Value>,
  pub children: Option<SectionMetadata>,
  pub hidden: Option<bool>,
}
```

### 8.2 Metadata extras: `field_metadata!` macro

Schema definitions can attach metadata extras via:

```rust
#[metadata(field_metadata! {
  label: "Max Open Positions",
  hint: "Maximum simultaneous positions",
  min: 1, max: 100,
  category: "Core Trading",
})]
```

This expands to a `FieldMetadataExtras` value, which is consumed by the `config_struct!` macro when
building field metadata.

### 8.3 How metadata is collected for the UI

`collect_config_metadata() -> ConfigMetadata` builds a map of section metadata for a curated set of
sections (not every schema is rendered in the UI).

As of this code version it includes:

```text
rpc, trader, positions, filtering, swaps, tokens, sol_price, events,
services, monitoring, ohlcv, webserver, telegram, ai
```

During collection:

- fields marked `hidden` are filtered out
- `visibility` is derived from `category` (see `derive_visibility`)
- nested `children` fields are also filtered for hidden values

---

## 9. GUI Migration: Navigation Tabs

Both load and reload run a GUI-specific migration step:

```rust
config.gui.dashboard.navigation.tabs =
  ensure_all_tabs_present(config.gui.dashboard.navigation.tabs);
```

`ensure_all_tabs_present(...)` is defined in `src/config/schemas/gui.rs` and currently performs:

- migration: tab id `"wallet"` -> `"wallets"`
- forcing icons/labels from defaults (only order/enabled are user-controlled)
- adding missing default tabs (e.g. new "tools" tab) while preserving user order as much as
  possible

This keeps older configs compatible when UI tabs are renamed/added.

---

## 10. Wallet Credentials + Keypair Access

The root config stores wallet credentials as:

- `wallet_encrypted`
- `wallet_nonce`

These are produced by `secure_storage` encryption helpers (AES-256-GCM with machine-derived key).

### 10.1 Sync-friendly keypair access: `get_wallet_keypair()`

`config::get_wallet_keypair()` is a sync API that bridges into the multi-wallet system when
possible:

- If there is a running tokio runtime:
  - uses `tokio::task::block_in_place` and `handle.block_on(async { ... })`
  - prefers `wallets::get_main_keypair()` if `wallets::is_initialized().await`
  - else falls back to legacy config decrypt (`get_wallet_keypair_from_config()`)
- If there is no runtime (early startup):
  - falls back to legacy config decrypt

Related helpers:

- `get_wallet_pubkey() -> Pubkey`
- `get_wallet_pubkey_string() -> String`

### 10.2 Reset to defaults while preserving credentials

`reset_config_to_defaults_preserving_credentials()`:

- captures `(wallet_encrypted, wallet_nonce, rpc.urls)`
- constructs a fresh `Config::default()`
- restores the preserved values if present
- validates
- replaces config and forces save-to-disk

---

## 11. Webserver Integration

The config module is heavily used by the webserver:

- Most routes read settings via `with_config(|cfg| ...)`.
- Some routes mutate settings via `update_config_section(..., save_to_disk = true)`.
- Config UI endpoints use `collect_config_metadata()` to render forms dynamically.
- "Reload config" operations call `config::reload_config()` (parse + validate + atomic swap).

Primary webserver config route area:

- `src/webserver/routes/config/**`

---

## 12. Module Connections

```text
config/
├── paths/            config.toml path + data directory
├── secure_storage/   wallet credential encryption/decryption
├── wallets/          preferred main wallet key source (multi-wallet system)
└── webserver/        config read/write + metadata APIs
```

### Pitfalls / gotchas

- `load_config_from_path` does not validate; only reload/save-to-file does.
- `update_config_section` does not validate; callers should validate before applying changes.
- Some config values are secrets (wallet_encrypted, telegram bot token, API keys); protect
  `data/config.toml` accordingly (Unix permissions are set by `save_config_to_file`).
