# ScreenerBot

**Native Solana Trading Engine**

A high-performance, local-first automated trading system for Solana DeFi. Built in Rust for native runtime performance and direct blockchain interaction.

Website: [screenerbot.io](https://screenerbot.io) | Documentation: [screenerbot.io/docs](https://screenerbot.io/docs)

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

Real-time price calculation from on-chain liquidity pool data.

| Component | Description |
|-----------|-------------|
| Discovery | Multi-source pool discovery (DexScreener, GeckoTerminal, Raydium API) |
| Fetcher | Batched RPC calls (50 accounts/request) with rate limiting |
| Decoders | 12 DEX-specific decoders for pool state interpretation |
| Calculator | Price derivation from pool reserves using constant product formula |
| Cache | Price history with configurable retention |

### Token Service

Unified token intelligence with multi-source data aggregation.

| Table | Purpose |
|-------|---------|
| tokens | Core token metadata (mint, symbol, decimals) |
| market_dexscreener | DexScreener market data |
| market_geckoterminal | GeckoTerminal market data |
| security_rugcheck | Rugcheck security analysis |
| tracking | Update timestamps and priorities |
| blacklist | Blocked tokens with reasons |

### Transaction Service

Real-time wallet monitoring with comprehensive DEX analysis.

- WebSocket streaming for instant transaction detection
- Batch RPC fetching with 50-account limit compliance
- Automatic DEX classification (Jupiter, Raydium, Orca, Meteora, Pumpfun)
- P&L calculation per transaction
- ATA operation tracking and rent calculation

### Position Manager

Complete position lifecycle management.

| Feature | Description |
|---------|-------------|
| Entry Tracking | Multiple entries per position (DCA support) |
| Exit Tracking | Partial exits with individual P&L |
| Price Updates | Background price monitoring with peak tracking |
| Loss Detection | Configurable loss thresholds with auto-blacklist |
| Verification | Chain verification for entry/exit confirmation |

### Filtering Engine

Multi-criteria token evaluation with concurrent processing.

**Filter Sources:**
- DexScreener: Liquidity, volume, price change, market cap
- GeckoTerminal: Volume, FDV, reserve ratio
- Rugcheck: Security risks, mint/freeze authority, top holders
- Meta: Token age, name patterns, symbol validation

### Strategy Engine

Condition-based trading logic with rule tree evaluation.

**Condition Types:**
- Price: Change percent, breakout, MA crossover
- Volume: Spike detection, level thresholds
- Candle: Size, consecutive patterns
- Time: Position holding duration
- Liquidity: Level requirements

---

## Supported DEXs

ScreenerBot includes native decoders for direct pool state interpretation:

| DEX | Programs | Description |
|-----|----------|-------------|
| **Raydium** | CLMM, CPMM, Legacy AMM | Largest Solana DEX, concentrated and standard liquidity |
| **Orca** | Whirlpool | Concentrated liquidity with tick-based pricing |
| **Meteora** | DAMM, DBC, DLMM | Dynamic AMM, dynamic bonding curve, dynamic liquidity |
| **Pumpfun** | AMM, Legacy (Bonding Curve) | Bonding curve launches and graduated pools |
| **Fluxbeam** | AMM | Standard constant product AMM |
| **Moonit** | AMM | Emerging DEX support |

### Swap Routers

| Router | Features |
|--------|----------|
| **Jupiter V6** | Best-in-class aggregation, route optimization, slippage protection |
| **GMGN** | Alternative router, concurrent quote comparison |

Quotes are fetched concurrently from enabled routers, and the best route is selected automatically.

---

## Trading Features

### Entry Evaluation

Safety checks performed in order:
1. Connectivity health (RPC, DexScreener, Rugcheck)
2. Position limits (configurable max open positions)
3. Duplicate prevention (no existing position)
4. Re-entry cooldown (configurable delay after exit)
5. Blacklist check
6. Strategy signal evaluation

### Exit Evaluation

Priority-ordered exit conditions:
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

Web-based control interface accessible at `http://localhost:8080` after bot startup.

| Page | Purpose |
|------|---------|
| **Home** | Overview with active positions, recent activity, system health |
| **Positions** | Open/closed position management with P&L tracking |
| **Tokens** | Token database browser with market and security data |
| **Filtering** | Passed/rejected token lists with filter reasons |
| **Trader** | Trading controls, active monitoring, entry/exit settings |
| **Transactions** | Real-time transaction stream with classification |
| **Strategies** | Strategy builder with condition configuration |
| **Wallet** | Balance tracking, history, token holdings |
| **Events** | System event log with category filtering |
| **Services** | Service health, metrics, status monitoring |
| **Config** | Hot-reload configuration editor |
| **Initialization** | First-run setup wizard |

---

## Configuration

Configuration is managed through `data/config.toml` with hot-reload support.

### Key Sections

```toml
[trader]
enabled = true
dry_run = false
max_positions = 5
position_size_sol = 0.1

[trader.entry]
min_liquidity_sol = 10.0
max_token_age_hours = 24

[trader.exit]
trailing_stop_enabled = true
trailing_stop_activation_percent = 20.0
trailing_stop_distance_percent = 10.0
roi_target_enabled = true
roi_target_percent = 50.0
time_override_enabled = true
time_override_hours = 24

[trader.dca]
enabled = true
max_rounds = 3
price_drop_percent = 20.0

[filtering]
# Multi-source filtering criteria

[swaps]
[swaps.jupiter]
enabled = true
slippage_bps = 300

[swaps.gmgn]
enabled = true

[tokens]
# Token database settings

[rpc]
urls = ["https://api.mainnet-beta.solana.com"]
rate_limit_per_second = 20
```

---

## Data Sources

ScreenerBot aggregates data from multiple sources:

| Source | Data Type | Usage |
|--------|-----------|-------|
| **Solana RPC** | On-chain state | Pool reserves, balances, transactions |
| **DexScreener** | Market data | Price, volume, liquidity, pools |
| **GeckoTerminal** | Market data | Alternative market metrics |
| **Rugcheck** | Security | Risk analysis, holder distribution |
| **Jupiter** | Quotes | Swap routing and pricing |

All data is cached locally in SQLite databases under the `data/` directory.

---

## Building from Source

### Prerequisites

- Rust 1.75+ (stable)
- Node.js 18+ (for frontend validation)
- macOS, Linux, or Windows

### Build Commands

```bash
# Clone repository
git clone https://github.com/screenerbot/screenerbot.git
cd screenerbot

# Build (debug profile for development)
cargo build

# Build release
cargo build --release

# Run
cargo run --bin screenerbot -- --run

# Run with dry-run mode (no actual trades)
cargo run --bin screenerbot -- --run --dry-run

# Run with debug logging
cargo run --bin screenerbot -- --run --debug-rpc --debug-transactions
```

### Frontend Validation

```bash
# Install dependencies
npm install

# Validate all frontend code
npm run check

# Format all code
npm run format
```

---

## Project Structure

```
screenerbot/
+-- src/
|   +-- apis/              # External API clients (DexScreener, Jupiter, etc.)
|   +-- config/            # Configuration system with hot-reload
|   +-- connectivity/      # Endpoint health monitoring
|   +-- errors/            # Structured error types
|   +-- events/            # Event recording system
|   +-- filtering/         # Token filtering engine
|   +-- logger/            # Logging with per-module control
|   +-- ohlcvs/            # OHLCV data management
|   +-- pools/             # Pool service and DEX decoders
|   +-- positions/         # Position lifecycle management
|   +-- services/          # ServiceManager and implementations
|   +-- strategies/        # Strategy engine and conditions
|   +-- swaps/             # Swap router integration
|   +-- tokens/            # Token database and updates
|   +-- trader/            # Trading logic and evaluators
|   +-- transactions/      # Transaction monitoring
|   +-- wallet.rs          # Wallet balance tracking
|   +-- webserver/         # Dashboard and REST API
|   +-- main.rs            # Entry point
|   +-- lib.rs             # Library exports
|   +-- run.rs             # Bot lifecycle
|
+-- data/                  # Runtime data (databases, config)
+-- Cargo.toml             # Rust dependencies
+-- package.json           # Frontend tooling
```

---

## Contributing

We welcome contributions from the community. Whether you are fixing bugs, improving documentation, or proposing new features, your help is appreciated.

### How to Contribute

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/your-feature`)
3. Make your changes following existing code patterns
4. Ensure all checks pass (`cargo check --lib && npm run check`)
5. Commit with clear messages
6. Open a pull request with a detailed description

### Code Standards

- Follow existing Rust idioms and project patterns
- Use the logging system with appropriate tags and levels
- Add configuration options through the config system, not hardcoded values
- Keep services independent with clear interfaces
- Document public APIs

### Areas for Contribution

- Additional DEX decoder implementations
- New strategy conditions
- Dashboard improvements
- Documentation and guides
- Performance optimizations
- Test coverage

---

## Community

Join our community for support, updates, and discussions:

- **Telegram**: [t.me/screenerbotio](https://t.me/screenerbotio)
- **X (Twitter)**: [x.com/screenerbotio](https://x.com/screenerbotio)
- **GitHub**: [github.com/screenerbot](https://github.com/screenerbot)
- **Website**: [screenerbot.io](https://screenerbot.io)
- **Documentation**: [screenerbot.io/docs](https://screenerbot.io/docs)

### Support

For technical support and questions:
- Check the [documentation](https://screenerbot.io/docs)
- Join our [Telegram community](https://t.me/screenerbotio)
- Contact support at [screenerbot.io/support](https://screenerbot.io/support)

### Donations

If ScreenerBot has been valuable to you, consider supporting development:

**Solana**: `[Your Solana Address]`

---

## License

This project is proprietary software. A valid license is required for operation.

Visit [screenerbot.io/pricing](https://screenerbot.io/pricing) to purchase a license.

---

Built with Rust. Powered by Solana.
