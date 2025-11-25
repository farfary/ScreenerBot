<p align="center">
  <img src="assets/banner.jpg" alt="ScreenerBot Banner" width="100%">
</p>

# ScreenerBot

**Native Solana Trading Engine**

A high-performance, local-first automated trading system for Solana DeFi. Built in Rust for native runtime performance and direct blockchain interaction.

Website: [screenerbot.io](https://screenerbot.io) | Documentation: [screenerbot.io/docs](https://screenerbot.io/docs)

---

<p align="center">
  <strong>Support Open Source Development</strong>
</p>

<p align="center">
  ScreenerBot is built with thousands of hours of work.<br>
  If this project helps you trade smarter, consider buying the developer a coffee.
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

## Table of Contents

- [Overview](#overview)
- [Architecture](#architecture)
- [Core Systems](#core-systems)
- [Supported DEXs](#supported-dexs)
- [Trading Features](#trading-features)
- [Dashboard](#dashboard)
- [Configuration](#configuration)
- [Data Sources](#data-sources)
- [Building from Source](#building-from-source)
- [Project Structure](#project-structure)
- [Contributing](#contributing)
- [Community](#community)
- [License](#license)

---

## Overview

ScreenerBot is a professional-grade trading automation platform designed for serious Solana DeFi traders. Unlike cloud-based solutions, it runs entirely on your local machine, providing:

- **Self-Custody Security**: Private keys never leave your computer
- **Native Performance**: Rust implementation with direct RPC connections
- **Real-Time Pricing**: Direct pool reserve calculations, not delayed API data
- **Complete Control**: Full access to raw data, customizable strategies, no platform fees

The system monitors thousands of tokens, evaluates them against configurable criteria, and executes trades automatically based on your strategies.

---

## Architecture

ScreenerBot employs a service-oriented architecture with 17 independent services orchestrated by a central ServiceManager. Services communicate through well-defined interfaces and are started in dependency order.

```
                                    ScreenerBot Architecture
    
    +-----------------------------------------------------------------------------------+
    |                                   ServiceManager                                   |
    |  (Dependency resolution, priority-based startup, health monitoring, metrics)      |
    +-----------------------------------------------------------------------------------+
                |                    |                    |                    |
    +-----------v--------+ +---------v----------+ +-------v------------+ +----v---------+
    |   Pool Service     | |   Token Service    | | Transaction Service| |   Trader     |
    |                    | |                    | |                    | |   Service    |
    | - Discovery        | | - Database (6 tbl) | | - WebSocket stream | | - Entry eval |
    | - Fetcher (batch)  | | - Market data      | | - Batch processor  | | - Exit eval  |
    | - Decoders (12)    | | - Security data    | | - Analyzer         | | - Executors  |
    | - Calculator       | | - Priority updates | | - P&L calculation  | | - Safety     |
    | - Cache            | | - Blacklist        | | - SQLite cache     | | - DCA/Partial|
    +--------------------+ +--------------------+ +--------------------+ +--------------+
                |                    |                    |                    |
    +-----------v--------+ +---------v----------+ +-------v------------+ +----v---------+
    |  Filtering Engine  | |   OHLCV Service    | |  Position Manager  | | Strategy     |
    |                    | |                    | |                    | | Engine       |
    | - Multi-source     | | - Multi-timeframe  | | - State management | | - Conditions |
    | - Configurable     | | - Gap detection    | | - DCA tracking     | | - Rule trees |
    | - Pass/reject      | | - Priority-based   | | - Partial exits    | | - Evaluation |
    | - Blacklist aware  | | - Bundle cache     | | - P&L calculation  | | - Caching    |
    +--------------------+ +--------------------+ +--------------------+ +--------------+
                |                    |                    |                    |
    +-----------v--------+ +---------v----------+ +-------v------------+ +----v---------+
    | Connectivity       | |   Events System    | |   Swap Router      | |   Wallet     |
    | Monitor            | |                    | |                    | |   Monitor    |
    |                    | | - Non-blocking     | | - Jupiter V6       | |              |
    | - Endpoint health  | | - Categorized      | | - GMGN             | | - Balance    |
    | - Fallback logic   | | - SQLite storage   | | - Concurrent quote | | - Snapshots  |
    | - Critical check   | | - Ring buffer      | | - Best route       | | - History    |
    +--------------------+ +--------------------+ +--------------------+ +--------------+
                                        |
    +-----------------------------------v-------------------------------------------+
    |                              Web Dashboard                                    |
    |  Axum REST API + Real-time Updates | 12 Feature Pages | Hot-reload Config    |
    +---------------------------------------------------------------------------+
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

Real-time wallet monitoring via WebSocket with DEX classification.

- Instant transaction detection
- Automatic swap identification (Jupiter, Raydium, Orca, Meteora, Pumpfun)
- P&L calculation per transaction
- SQLite caching with connection pooling

### Position Manager

Complete position lifecycle with DCA and partial exit support.

- Multiple entries per position (DCA)
- Partial exits with individual P&L tracking
- Background price monitoring with peak tracking
- Loss detection with configurable auto-blacklist

### Filtering Engine

Multi-criteria token evaluation from multiple data sources.

- DexScreener: Liquidity, volume, price change
- GeckoTerminal: Volume, FDV, reserve ratio
- Rugcheck: Security risks, authorities, holder distribution
- Meta: Token age, name patterns

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

Web interface at `http://localhost:8080` with 12 feature pages:

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

---

## Configuration

Managed through `data/config.toml` with hot-reload support. Key sections:

- `[trader]` - Position limits, sizing, dry-run mode
- `[trader.entry]` - Entry criteria (liquidity, age)
- `[trader.exit]` - Exit strategies (trailing, ROI, time)
- `[trader.dca]` - DCA settings
- `[filtering]` - Token filtering criteria
- `[swaps]` - Router configuration (Jupiter, GMGN)
- `[rpc]` - RPC endpoints and rate limiting

See full documentation at [screenerbot.io/docs](https://screenerbot.io/docs).

---

## Data Sources

| Source | Usage |
|--------|-------|
| **Solana RPC** | Pool reserves, balances, transactions |
| **DexScreener** | Market data, pools |
| **GeckoTerminal** | Alternative market metrics |
| **Rugcheck** | Security analysis |
| **Jupiter** | Swap routing |

All data cached locally in SQLite databases.

---

## Building from Source

### Prerequisites

- Rust 1.75+
- Node.js 18+ (optional, for frontend tools)

### Build

```bash
git clone https://github.com/screenerbot/screenerbot.git
cd screenerbot
cargo build
```

### Run

```bash
# Normal mode
cargo run --bin screenerbot -- --run

# Dry-run (no trades)
cargo run --bin screenerbot -- --run --dry-run

# With debug logging
cargo run --bin screenerbot -- --run --debug-rpc
```

---

## Project Structure

```
src/
+-- apis/           # External API clients
+-- config/         # Configuration system
+-- connectivity/   # Endpoint health monitoring
+-- events/         # Event recording system
+-- filtering/      # Token filtering engine
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

Contributions welcome. See our guidelines:

1. Fork the repository
2. Create a feature branch
3. Follow existing code patterns
4. Ensure `cargo check --lib` passes
5. Open a pull request

**Areas for contribution:**
- DEX decoder implementations
- Strategy conditions
- Dashboard improvements
- Documentation

---

## Community

- **Telegram**: [t.me/screenerbotio](https://t.me/screenerbotio)
- **X (Twitter)**: [x.com/screenerbotio](https://x.com/screenerbotio)
- **Website**: [screenerbot.io](https://screenerbot.io)
- **Documentation**: [screenerbot.io/docs](https://screenerbot.io/docs)

---

## License

Proprietary software. License required for operation.

Visit [screenerbot.io/pricing](https://screenerbot.io/pricing) to purchase.

---

Built with Rust. Powered by Solana.