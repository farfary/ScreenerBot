<p align="center">
  <img src="assets/banner.jpg" alt="ScreenerBot Banner" width="100%">
</p>

<h1 align="center">ScreenerBot</h1>

<p align="center">
  <strong>Native Solana Trading Engine</strong>
</p>

<p align="center">
  <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/Built%20with-Rust-000000?style=flat-square&logo=rust&logoColor=white" alt="Built with Rust"></a>
  <a href="https://solana.com/"><img src="https://img.shields.io/badge/Powered%20by-Solana-9945FF?style=flat-square&logo=solana&logoColor=white" alt="Powered by Solana"></a>
  <a href="https://tauri.app/"><img src="https://img.shields.io/badge/Desktop-Tauri-24C8D8?style=flat-square&logo=tauri&logoColor=white" alt="Tauri Desktop"></a>
  <a href="https://screenerbot.io/docs"><img src="https://img.shields.io/badge/Docs-screenerbot.io-blue?style=flat-square" alt="Documentation"></a>
  <a href="https://t.me/screenerbotio"><img src="https://img.shields.io/badge/Community-Telegram-26A5E4?style=flat-square&logo=telegram&logoColor=white" alt="Telegram"></a>
</p>

<p align="center">
  A high-performance, local-first automated trading system for Solana DeFi.<br>
  Built in Rust for native runtime performance and direct blockchain interaction.
</p>

<p align="center">
  <a href="https://screenerbot.io">Website</a> |
  <a href="https://screenerbot.io/docs">Documentation</a> |
  <a href="https://screenerbot.io/pricing">Get License</a>
</p>

---

<p align="center">
  <strong>Support Development</strong>
</p>

<p align="center">
  If this project helps you trade smarter, consider supporting the developer.
</p>

<p align="center">
  <a href="https://solscan.io/account/2Nr5M6TngMUPZRyW6yeN4GxLg53JwQSr5XTEMeeoTkMd">
    <img src="https://img.shields.io/badge/Donate-SOL-9945FF?style=for-the-badge&logo=solana&logoColor=white" alt="Donate SOL">
  </a>
</p>

<p align="center">
  <code>2Nr5M6TngMUPZRyW6yeN4GxLg53JwQSr5XTEMeeoTkMd</code>
</p>

---

## Why Rust?

ScreenerBot is written in **Rust** - the same language Solana itself is built with. This isn't a coincidence:

- **Native Performance**: Compiled to machine code, not interpreted. Executes as fast as C/C++.
- **Memory Safety**: No garbage collector pauses. Predictable, consistent execution times.
- **Concurrency**: Fearless parallelism with async/await. Handle thousands of tokens simultaneously.
- **Reliability**: If it compiles, it runs. Strong type system catches bugs at compile time.

Trading bots written in Python or JavaScript can't match the speed and reliability of native code. When milliseconds matter in DeFi, Rust delivers.

---

## Table of Contents

- [Overview](#overview)
- [Architecture](#architecture)
- [Core Systems](#core-systems)
- [Supported DEXs](#supported-dexs)
- [Trading Features](#trading-features)
- [Dashboard](#dashboard)
- [Configuration](#configuration)
- [Data Sources](#data-sources)
- [Desktop Application](#desktop-application)
- [Building from Source](#building-from-source)
- [Project Structure](#project-structure)
- [Contributing](#contributing)
- [Community](#community)
- [License](#license)

---

## Overview

ScreenerBot is a professional-grade trading automation platform for Solana DeFi. Unlike cloud-based solutions, it runs entirely on your local machine:

| Feature | Benefit |
|---------|---------|
| **Self-Custody** | Private keys never leave your computer |
| **Native Speed** | Rust performance with direct RPC connections |
| **Real-Time Prices** | Direct pool reserve calculations, not delayed APIs |
| **Full Control** | Raw data access, custom strategies, no platform fees |

---

## Architecture

17 independent services orchestrated by a central ServiceManager:

```
┌─────────────────────────────────────────────────────────────────────────────────────┐
│                                  ServiceManager                                      │
│         Dependency Resolution • Priority Startup • Health Monitoring • Metrics       │
└─────────────────────────────────────────────────────────────────────────────────────┘
        │                    │                    │                    │
        ▼                    ▼                    ▼                    ▼
┌──────────────────┐ ┌──────────────────┐ ┌──────────────────┐ ┌──────────────────┐
│   Pool Service   │ │  Token Service   │ │Transaction Service│ │  Trader Service  │
├──────────────────┤ ├──────────────────┤ ├──────────────────┤ ├──────────────────┤
│ • Discovery      │ │ • Database (6tbl)│ │ • WebSocket stream│ │ • Entry eval     │
│ • Fetcher (batch)│ │ • Market data    │ │ • Batch processor │ │ • Exit eval      │
│ • Decoders (11)  │ │ • Security data  │ │ • DEX analyzer    │ │ • Executors      │
│ • Calculator     │ │ • Priority update│ │ • P&L calculation │ │ • Safety gates   │
│ • Cache          │ │ • Blacklist      │ │ • SQLite cache    │ │ • DCA/Partial    │
└──────────────────┘ └──────────────────┘ └──────────────────┘ └──────────────────┘
        │                    │                    │                    │
        ▼                    ▼                    ▼                    ▼
┌──────────────────┐ ┌──────────────────┐ ┌──────────────────┐ ┌──────────────────┐
│ Filtering Engine │ │  OHLCV Service   │ │ Position Manager │ │ Strategy Engine  │
├──────────────────┤ ├──────────────────┤ ├──────────────────┤ ├──────────────────┤
│ • Multi-source   │ │ • 7 timeframes   │ │ • State tracking │ │ • Conditions     │
│ • Configurable   │ │ • Gap detection  │ │ • DCA tracking   │ │ • Rule trees     │
│ • Pass/reject    │ │ • Priority-based │ │ • Partial exits  │ │ • Evaluation     │
│ • Blacklist aware│ │ • Bundle cache   │ │ • P&L calculation│ │ • Caching        │
└──────────────────┘ └──────────────────┘ └──────────────────┘ └──────────────────┘
        │                    │                    │                    │
        ▼                    ▼                    ▼                    ▼
┌──────────────────┐ ┌──────────────────┐ ┌──────────────────┐ ┌──────────────────┐
│   Connectivity   │ │  Events System   │ │   Swap Router    │ │  Wallet Monitor  │
├──────────────────┤ ├──────────────────┤ ├──────────────────┤ ├──────────────────┤
│ • Endpoint health│ │ • Non-blocking   │ │ • Jupiter V6     │ │ • SOL balance    │
│ • Fallback logic │ │ • Categorized    │ │ • GMGN           │ │ • Token holdings │
│ • Critical check │ │ • SQLite storage │ │ • Concurrent     │ │ • Snapshots      │
└──────────────────┘ └──────────────────┘ └──────────────────┘ └──────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────────────────┐
│                                 Web Dashboard                                        │
│              Axum REST API • Real-time Updates • 12 Pages • Hot-reload Config        │
└─────────────────────────────────────────────────────────────────────────────────────┘
```

### Service Dependencies

```
Level 0 (No dependencies):
  - Events, RPC Stats, SOL Price, Connectivity

Level 1:
  - Tokens (depends on Events)
  - Pools (depends on Events)

Level 2:
  - OHLCV (depends on Pools, Tokens)
  - Filtering (depends on Pools, Tokens)
  - Positions (depends on Pools, Tokens)
  - Transactions (depends on Tokens)

Level 3:
  - Trader (depends on Pools, Tokens, Positions, Filtering, Transactions)
  - Wallet (depends on Transactions)

Level 4:
  - Webserver (depends on all services)
```

---

## Core Systems

### Pool Service

Real-time price calculation directly from on-chain liquidity pool reserves.

- **Discovery**: Finds pools from DexScreener, GeckoTerminal, and Raydium APIs
- **Fetcher**: Batched RPC calls (50 accounts per request) with rate limiting
- **Analyzer**: Classifies pools by DEX type and extracts metadata
- **Decoders**: 11 native decoders for parsing pool state data
- **Calculator**: Derives prices from reserves (SOL-based pricing)
- **Cache**: In-memory price history with database persistence

### Token Service

Unified token database with multi-source data aggregation.

- Core metadata (mint, symbol, decimals)
- Market data from DexScreener and GeckoTerminal
- Security analysis from Rugcheck
- Priority-based background updates
- Blacklist management

### Transaction Service

Real-time wallet monitoring via WebSocket with comprehensive DEX analysis.

- WebSocket streaming for instant detection
- DEX classification (Jupiter, Raydium, Orca, Meteora, Pumpfun, GMGN, Fluxbeam, Moonshot)
- Swap detection and P&L calculation
- ATA operation tracking
- Position entry/exit verification
- SQLite caching with connection pooling

### Position Manager

Complete position lifecycle with DCA and partial exit support.

- Multiple entries per position (DCA)
- Partial exits with individual P&L tracking
- Background price monitoring with peak tracking
- Loss detection with configurable auto-blacklist

### Filtering Engine

Multi-criteria token evaluation from multiple data sources.

- DexScreener: Liquidity, volume, price change, transactions, FDV, market cap
- GeckoTerminal: Liquidity, volume, price change, market cap, reserve
- Rugcheck: Security risks, authorities, holder distribution, insider detection
- Meta: Token age, decimals validation, cooldown check

### Strategy Engine

Condition-based trading logic with configurable rules.

- Price conditions (change percent, breakout, MA)
- Volume conditions (spike, thresholds)
- Candle patterns and time-based conditions
- Rule tree evaluation with caching

---

## Supported DEXs

Native decoders for direct pool state interpretation:

| DEX | Programs |
|-----|----------|
| **Raydium** | CLMM, CPMM, Legacy AMM |
| **Orca** | Whirlpool |
| **Meteora** | DAMM, DBC, DLMM |
| **Pumpfun** | AMM, Legacy (Bonding Curve) |
| **Fluxbeam** | AMM |
| **Moonit** | AMM |

### Swap Routers

- **Jupiter V6**: Aggregation with route optimization
- **GMGN**: Alternative router for quote comparison

Concurrent quote fetching with automatic best-route selection.

---

## Trading Features

### Entry Evaluation

Safety checks in order:
1. Connectivity health
2. Position limits
3. Duplicate prevention
4. Re-entry cooldown
5. Blacklist check
6. Strategy signals

### Exit Evaluation

Priority-ordered conditions:
1. **Blacklist** (emergency): Immediate exit if token blacklisted
2. **Risk Limits** (emergency): >90% loss protection
3. **Trailing Stop** (high): Dynamic stop-loss following price peaks
4. **ROI Target** (normal): Fixed profit target exit
5. **Time Override** (normal): Maximum hold duration
6. **Strategy Exit** (normal): Strategy-defined exit signals

### DCA (Dollar Cost Averaging)

- Configurable DCA rounds with size multipliers
- Price drop thresholds for additional entries
- Per-round tracking with individual cost basis

### Partial Exits

- Multiple exit points per position
- Individual P&L calculation per exit
- Remaining position tracking

---

## Dashboard

Web interface at `http://localhost:8080` with 12 pages:

- **Home**: Overview, positions, system health
- **Positions**: Open/closed with P&L tracking
- **Tokens**: Database browser with market and security data
- **Filtering**: Passed/rejected tokens with reasons
- **Trader**: Trading controls and monitoring
- **Transactions**: Real-time stream with classification
- **Strategies**: Strategy builder
- **Wallet**: Balance and holdings
- **Events**: System event log
- **Services**: Health and metrics
- **Config**: Hot-reload editor
- **Initialization**: First-run setup wizard

---

## Configuration

Managed through `data/config.toml` with hot-reload support. 16 config sections:

| Section | Purpose |
|---------|---------|
| `[trader]` | Position limits, sizing, ROI targets, DCA, trailing stop |
| `[positions]` | Position tracking, partial exits, cooldowns |
| `[filtering]` | Token filtering with nested DexScreener/GeckoTerminal/Rugcheck |
| `[swaps]` | Router configuration (Jupiter, GMGN) |
| `[tokens]` | Token database, update intervals |
| `[pools]` | Pool discovery, caching |
| `[rpc]` | RPC endpoints and rate limiting |
| `[ohlcv]` | Candlestick data settings |
| `[strategies]` | Strategy engine configuration |
| `[wallet]` | Wallet monitoring |
| `[events]` | Event system settings |
| `[services]` | Service manager settings |
| `[monitoring]` | System metrics |
| `[connectivity]` | Endpoint health monitoring |
| `[sol_price]` | SOL/USD price service |
| `[gui]` | Desktop application settings |

Access via `with_config(|cfg| cfg.trader.max_open_positions)`. Hot-reload with `reload_config()`.

---

## Data Sources

| Source | Usage |
|--------|-------|
| **Solana RPC** | Pool reserves, balances, transactions |
| **DexScreener** | Market data, pool discovery |
| **GeckoTerminal** | Alternative market metrics |
| **Rugcheck** | Security analysis |
| **Jupiter** | Swap routing and quotes |
| **CoinGecko** | Token metadata |
| **DefiLlama** | Token prices, DeFi protocols |

All data cached locally in SQLite databases.

---

## Desktop Application

Native desktop application built with **Tauri 2.1** - the same framework behind apps like 1Password and Notion.

### Platform Support

| Platform | Min Version | Package Format |
|----------|-------------|----------------|
| **macOS** | 10.13 (High Sierra) | `.app` / `.dmg` |
| **Windows** | Windows 10 | `.exe` / `.msi` |
| **Linux** | Ubuntu 18.04+ | `.deb` / `.AppImage` |

### Desktop Features

- **Native Window**: 1400x900 default, 1200x700 minimum, fully resizable
- **Embedded Dashboard**: Webserver runs locally at `localhost:8080`
- **Keyboard Shortcuts**: Zoom (Cmd/Ctrl +/-/0), Reload (Cmd/Ctrl + R)
- **System Integration**: Native title bar, notifications

---

## Building from Source

### Prerequisites

- Rust 1.75+
- Node.js 18+ (for frontend validation tools)
- Platform-specific:
  - **macOS**: Xcode Command Line Tools
  - **Windows**: Visual Studio Build Tools, WebView2
  - **Linux**: `libwebkit2gtk-4.0-dev`, `libssl-dev`, `libgtk-3-dev`

### Build Options

```bash
git clone https://github.com/farfary/ScreenerBot.git
cd ScreenerBot

# Headless mode (server only)
cargo build --bin screenerbot

# Desktop application
cargo tauri build
```

### Run

```bash
# Headless mode (terminal)
cargo run --bin screenerbot

# Desktop application
cargo tauri dev

# With debug logging
cargo run --bin screenerbot -- --debug-rpc
```

### Build Artifacts

After `cargo tauri build`:

- **macOS**: `target/release/bundle/macos/ScreenerBot.app`
- **Windows**: `target/release/bundle/msi/ScreenerBot_*.msi`
- **Linux**: `target/release/bundle/deb/screenerbot_*.deb`

---

## Project Structure

```
src/
+-- apis/           # External API clients
+-- config/         # Configuration system
+-- connectivity/   # Endpoint health monitoring
+-- events/         # Event recording system
+-- filtering/      # Token filtering engine
+-- ohlcvs/         # OHLCV candlestick data (7 timeframes)
+-- pools/          # Pool service and DEX decoders
+-- positions/      # Position lifecycle management
+-- services/       # ServiceManager
+-- strategies/     # Strategy engine
+-- swaps/          # Swap router integration
+-- tokens/         # Token database
+-- trader/         # Trading logic
+-- transactions/   # Transaction monitoring
+-- webserver/      # Dashboard and REST API
```

---

## Contributing

Contributions welcome:

1. Fork the repository
2. Create a feature branch
3. Follow existing code patterns
4. Ensure `cargo check --lib` passes
5. Open a pull request

**Areas for contribution:** DEX decoders, strategy conditions, dashboard improvements, documentation.

---

## Community

<p align="center">
  <a href="https://t.me/screenerbotio"><img src="https://img.shields.io/badge/Telegram-Join%20Chat-26A5E4?style=for-the-badge&logo=telegram&logoColor=white" alt="Telegram"></a>
  <a href="https://x.com/screenerbotio"><img src="https://img.shields.io/badge/X-Follow-000000?style=for-the-badge&logo=x&logoColor=white" alt="X (Twitter)"></a>
  <a href="https://screenerbot.io"><img src="https://img.shields.io/badge/Website-screenerbot.io-9945FF?style=for-the-badge" alt="Website"></a>
</p>

---

## License

Proprietary software. License required for operation.

<p align="center">
  <a href="https://screenerbot.io/pricing"><img src="https://img.shields.io/badge/Get%20License-screenerbot.io-9945FF?style=for-the-badge" alt="Get License"></a>
</p>

---

<p align="center">
  <img src="https://img.shields.io/badge/Built%20with-Rust-000000?style=flat-square&logo=rust&logoColor=white" alt="Rust">
  <img src="https://img.shields.io/badge/Powered%20by-Solana-9945FF?style=flat-square&logo=solana&logoColor=white" alt="Solana">
  <img src="https://img.shields.io/badge/Desktop-Tauri-24C8D8?style=flat-square&logo=tauri&logoColor=white" alt="Tauri">
</p>