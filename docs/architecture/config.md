# Config Module — Architecture

> ScreenerBot TOML Configuration System — February 2026

---

## Table of Contents

1. [Overview](#1-overview)
2. [File Structure](#2-file-structure)
3. [Core Types](#3-core-types)
4. [Config Macro System](#4-config-macro-system)
5. [Loading & Hot Reload](#5-loading--hot-reload)
6. [Config Sections](#6-config-sections)
7. [Access Patterns](#7-access-patterns)
8. [Metadata System](#8-metadata-system)
9. [Module Connections](#9-module-connections)

---

## 1. Overview

The Config module provides strongly-typed TOML-based configuration with macro-driven definitions, hot reload, and UI metadata. All 23 config sections are defined declaratively with defaults, validation, and rendering hints.

**Key characteristics:**
- TOML file storage (`data/config.toml`)
- `OnceLock<RwLock<Config>>` for thread-safe global access
- `config_struct!` macro auto-generates struct + Default + serde + metadata
- Hot reload with validation (atomic replacement)
- Field metadata for dashboard UI rendering (labels, hints, min/max, categories)
- Encrypted wallet credentials in config file

**27 files, ~5,805 lines**

---

## 2. File Structure

```
src/config/
├── mod.rs              # Module exports
├── macros.rs           # config_struct! macro (110 lines)
├── metadata.rs         # FieldMetadata, FieldType, visibility (417 lines)
├── utils.rs            # Loading, reload, access, wallet mgmt (839 lines)
└── schemas/            # 23 config section definitions
    ├── mod.rs           # Root Config struct
    ├── filtering.rs     # Token filtering rules (969 lines, largest)
    ├── ai.rs            # AI providers & analysis (439 lines)
    ├── trader.rs        # Trading parameters (340 lines)
    ├── rpc.rs           # RPC providers (256 lines)
    ├── tokens.rs        # Token tracking (254 lines)
    ├── gui.rs           # GUI/dashboard (238 lines)
    ├── swaps.rs         # Swap routing (203 lines)
    ├── telegram.rs      # Telegram bot (186 lines)
    ├── wallet.rs        # Wallet addresses (140 lines)
    ├── positions.rs     # Position management (122 lines)
    ├── events.rs        # Event recording (121 lines)
    ├── ohlcv.rs         # OHLCV data (104 lines)
    ├── webserver.rs     # Webserver/headless (103 lines)
    ├── pools.rs         # Pool discovery (102 lines)
    ├── connectivity.rs  # Connectivity monitoring (84 lines)
    ├── holder_watch.rs  # Holder watch tool (74 lines)
    ├── strategies.rs    # Strategies (50 lines)
    ├── maintenance.rs   # Data retention (33 lines)
    ├── performance.rs   # Memory/cache sizing (27 lines)
    ├── monitoring.rs    # System monitoring (10 lines)
    ├── sol_price.rs     # SOL price (10 lines)
    └── services.rs      # Services (10 lines)
```

---

## 3. Core Types

### Root Config

```rust
pub struct Config {
    wallet_encrypted: String,
    wallet_nonce: String,
    rpc: RpcConfig,
    trader: TraderConfig,
    positions: PositionsConfig,
    filtering: FilteringConfig,
    swaps: SwapsConfig,
    tokens: TokensConfig,
    pools: PoolsConfig,
    sol_price: SolPriceConfig,
    events: EventsConfig,
    services: ServicesConfig,
    monitoring: MonitoringConfig,
    connectivity: ConnectivityMonitoringConfig,
    ohlcv: OhlcvConfig,
    wallet: WalletConfig,
    strategies: StrategiesConfig,
    gui: GuiConfig,
    webserver: WebserverConfig,
    telegram: TelegramConfig,
    holder_watch: HolderWatchConfig,
    ai: AiConfig,
    performance: PerformanceConfig,
    maintenance: MaintenanceConfig,
}
```

### Global Access

```rust
pub static CONFIG: OnceLock<RwLock<Config>> = OnceLock::new();
```

---

## 4. Config Macro System

### `config_struct!` Macro

Defines a config section in one declaration — generates struct, Default, serde, and metadata:

```rust
config_struct! {
    pub struct TraderConfig {
        enabled: bool = false,
        max_open_positions: u32 = 5,
        trade_size_sol: f64 = 0.1,
        // ... more fields
    }
}
```

**Auto-generates:**
- `pub struct TraderConfig { ... }` with `#[serde(default)]`
- `impl Default for TraderConfig` with specified defaults
- `FieldTypeInfo` trait implementation
- `NestedMetadata` trait for UI rendering

### `field_metadata!` Macro

Adds UI metadata to fields:

```rust
#[metadata(field_metadata! {
    label: "Max Open Positions",
    hint: "Maximum simultaneous positions",
    min: 1, max: 100,
    unit: "positions",
    impact: "critical",
    category: "Core Trading",
})]
max_open_positions: u32 = 5,
```

---

## 5. Loading & Hot Reload

### Load Sequence

```
load_config()
  → get_config_path()              // data/config.toml
  → fs::read_to_string(path)      // Read TOML
  → toml::from_str::<Config>()    // Parse with serde
  → ensure_all_tabs_present()     // GUI migration
  → CONFIG.set(RwLock::new(cfg))  // Store global
```

### Hot Reload

```
reload_config()
  → Read + parse new TOML
  → validate_config(&new_cfg)     // Strict validation
  → CONFIG.write() = new_cfg      // Atomic replace
```

- Old config preserved on validation failure
- All readers block during write, see new values after
- Used by dashboard config editor

---

## 6. Config Sections

| Section | Fields | Key Settings |
|---------|--------|-------------|
| `rpc` | ~15 | Provider URLs, selection strategy, rate limits |
| `trader` | ~40+ | enabled, max_positions, trade_size, ROI, DCA, stop-loss, time-override |
| `positions` | ~20 | Cooldown, partial exits, trailing stop, loss blacklist |
| `filtering` | ~50+ | DexScreener rules, token age, holders, security, on-chain |
| `swaps` | ~25 | Jupiter/GMGN routers, slippage tiers, priority fees |
| `tokens` | ~15 | Cache TTL, blacklist, metadata settings |
| `pools` | ~10 | Discovery intervals, analysis settings |
| `ai` | ~30+ | Provider (OpenAI/Anthropic/Groq), filtering, analysis, cache |
| `gui` | ~20 | Zoom, dashboard theme, lockscreen, navigation tabs |
| `telegram` | ~25 | Bot token, chat_id, notifications, rate limiting |
| `events` | ~15 | Recording toggles per category |
| `webserver` | ~10 | Port, host, CORS |
| `maintenance` | ~5 | Retention periods, vacuum schedule |
| `performance` | ~5 | Memory profile, cache sizing |
| `connectivity` | ~8 | Health check intervals, thresholds |
| Others | ~5 each | ohlcv, sol_price, monitoring, services, strategies, wallet, holder_watch |

---

## 7. Access Patterns

### Read (Most Common)

```rust
use crate::config::with_config;

let max = with_config(|cfg| cfg.trader.max_open_positions);
```

### Read for Async (Clone)

```rust
let cfg = get_config_clone();
// Safe to use across .await points
tokio::time::sleep(dur).await;
let val = cfg.trader.trade_size_sol;
```

### Write

```rust
update_config_section(|cfg| {
    cfg.trader.max_open_positions = 10;
}, true /* save_to_disk */)?;
```

---

## 8. Metadata System

Each field carries rendering metadata for the dashboard:

```rust
pub struct FieldMetadata {
    pub label: String,
    pub hint: Option<String>,
    pub unit: Option<String>,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub step: Option<f64>,
    pub impact: Option<String>,       // critical/high/medium/low
    pub category: Option<String>,
    pub visibility: Visibility,       // primary/secondary/technical
    pub field_type: FieldType,
}
```

Used by webserver config routes to auto-render configuration UI.

---

## 9. Module Connections

```
config/
└── (standalone — no dependencies)
```

| Caller | Usage |
|--------|-------|
| All modules | `with_config()` for settings |
| webserver/config | Full CRUD API + metadata |
| services | Check `is_enabled()` flags |
| trader | Trading parameters |
| filtering | Filter rules |
| rpc | Provider configuration |
