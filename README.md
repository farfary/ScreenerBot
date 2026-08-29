<p align="center">
  <img src="https://screenerbot.io/banner.jpg" alt="ScreenerBot Banner" width="100%">
</p>

<p align="center">
  <strong>Open Source Solana Trading Engine</strong>
</p>

<p align="center">
  <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/Built%20with-Rust-000000?style=flat-square&logo=rust&logoColor=white" alt="Built with Rust"></a>
  <a href="https://solana.com/"><img src="https://img.shields.io/badge/Powered%20by-Solana-9945FF?style=flat-square&logo=solana&logoColor=white" alt="Powered by Solana"></a>
  <a href="https://www.electronjs.org/"><img src="https://img.shields.io/badge/Desktop-Electron-47848F?style=flat-square&logo=electron&logoColor=white" alt="Electron Desktop"></a>
  <a href="https://github.com/farfary/ScreenerBot/blob/main/LICENSE"><img src="https://img.shields.io/badge/License-BSL--1.1-blue?style=flat-square" alt="BSL 1.1 License"></a>
  <a href="https://screenerbot.io/docs"><img src="https://img.shields.io/badge/Docs-screenerbot.io-blue?style=flat-square" alt="Documentation"></a>
  <a href="https://t.me/screenerbotio_talk"><img src="https://img.shields.io/badge/Community-Telegram-26A5E4?style=flat-square&logo=telegram&logoColor=white" alt="Telegram Community"></a>
  <a href="https://github.com/farfary/ScreenerBot"><img src="https://img.shields.io/github/stars/farfary/ScreenerBot?style=flat-square" alt="GitHub Stars"></a>
  <a href="https://screenerbot.io/download"><img src="https://img.shields.io/badge/Download-Latest-orange?style=flat-square" alt="Download"></a>
  <a href="https://x.com/screenerbotio"><img src="https://img.shields.io/badge/X-Follow-000000?style=flat-square&logo=x&logoColor=white" alt="X Follow"></a>
</p>

<p align="center">
  A high-performance, local-first trading system for Solana DeFi.<br>
  Automated strategies, manual execution, and paper or confirmation-gated live wallet copy.<br>
  Built in Rust for native runtime performance and direct blockchain interaction.<br>
  <strong>Runs entirely on your own machine — your keys never leave your computer.</strong>
</p>

<p align="center">
  <a href="https://screenerbot.io">Website</a> •
  <a href="https://screenerbot.io/docs">Documentation</a> •
  <a href="https://screenerbot.io/download">Download</a> •
  <a href="https://t.me/screenerbotio_talk">Join Community</a>
</p>

---

## Screenshots

<table>
  <tr>
    <td align="center"><strong>Dashboard Overview</strong></td>
    <td align="center"><strong>Transaction Monitor</strong></td>
  </tr>
  <tr>
    <td><a href="https://screenerbot.io/api/screenshots/current/home?full=1"><img src="https://screenerbot.io/api/screenshots/current/home" alt="Dashboard Overview" width="400"></a></td>
    <td><a href="https://screenerbot.io/api/screenshots/current/transactions?full=1"><img src="https://screenerbot.io/api/screenshots/current/transactions" alt="Transaction Monitor" width="400"></a></td>
  </tr>
  <tr>
    <td align="center"><strong>Open Positions</strong></td>
    <td align="center"><strong>Position History</strong></td>
  </tr>
  <tr>
    <td><a href="https://screenerbot.io/api/screenshots/current/positions-open?full=1"><img src="https://screenerbot.io/api/screenshots/current/positions-open" alt="Open Positions" width="400"></a></td>
    <td><a href="https://screenerbot.io/api/screenshots/current/positions-closed?full=1"><img src="https://screenerbot.io/api/screenshots/current/positions-closed" alt="Position History" width="400"></a></td>
  </tr>
  <tr>
    <td align="center"><strong>Trader Interface</strong></td>
    <td align="center"><strong>Token Details</strong></td>
  </tr>
  <tr>
    <td><a href="https://screenerbot.io/api/screenshots/current/trader?full=1"><img src="https://screenerbot.io/api/screenshots/current/trader" alt="Trader Interface" width="400"></a></td>
    <td><a href="https://screenerbot.io/api/screenshots/current/token-details?full=1"><img src="https://screenerbot.io/api/screenshots/current/token-details" alt="Token Details" width="400"></a></td>
  </tr>
</table>

<p align="center">
  <a href="https://screenerbot.io/screenshots">View all screenshots</a>
</p>

---

<p align="center">
  <strong>⚠️ Risk Disclaimer</strong>
</p>

<p align="center">
  Cryptocurrency trading involves substantial risk of loss and is not suitable for every investor.<br>
  This software may contain bugs or issues that could result in financial losses.<br>
  The developers are not responsible for any financial losses incurred through use of this software.<br>
  Trade at your own risk. Never invest more than you can afford to lose.
</p>

---

## Why Rust?

ScreenerBot is written in **Rust** — the same language Solana itself is built with. This isn't a coincidence:

- **Native Performance**: Compiled to machine code, not interpreted. Executes as fast as C/C++.
- **Memory Safety**: No garbage collector pauses. Predictable, consistent execution times.
- **Concurrency**: Fearless parallelism with async/await. Handle thousands of tokens simultaneously.
- **Reliability**: If it compiles, it runs. Strong type system catches bugs at compile time.

Trading bots written in Python or JavaScript can't match the speed and reliability of native code. When milliseconds matter in DeFi, Rust delivers.

---

## Table of Contents

- [Overview](#overview)
- [Screenshots](#screenshots)
- [Architecture](#architecture)
- [Core Systems](#core-systems)
- [Supported DEXs](#supported-dexs)
- [Trading Features](#trading-features)
- [AI Assistant](#ai-assistant)
- [Dashboard](#dashboard)
- [Configuration](#configuration)
- [Data Sources](#data-sources)
- [Desktop Application](#desktop-application)
- [Building from Source](#building-from-source)
- [Project Structure](#project-structure)
- [Contributing](#contributing)
- [Community](#community)

---

## Overview

ScreenerBot is a professional-grade trading automation platform for Solana DeFi. Unlike cloud-based solutions, it runs entirely on your local machine:

| Feature              | Benefit                                              |
| -------------------- | ---------------------------------------------------- |
| **Self-Custody**     | Private keys never leave your computer               |
| **Native Speed**     | Rust performance with direct RPC connections         |
| **Real-Time Prices** | Direct pool reserve calculations, not delayed APIs   |
| **Trading Modes**    | Automated, manual, paper copy, and armed live copy   |
| **Risk Controls**    | Shared admission, position, and emergency-stop gates |
| **Full Control**     | Raw data access, custom strategies, no subscriptions |

---

## Architecture

Independent services orchestrated by a central ServiceManager with dependency resolution,
priority-based startup, readiness gates, health monitoring, and reverse-order shutdown:

```
┌─────────────────────────────────────────────────────────────────────────────────────┐
│                                  ServiceManager                                     │
│         Dependency Resolution • Priority Startup • Health Monitoring • Metrics      │
└─────────────────────────────────────────────────────────────────────────────────────┘
        │                    │                    │                    │
        ▼                    ▼                    ▼                    ▼
┌──────────────────┐ ┌──────────────────┐ ┌───────────────────┐ ┌──────────────────┐
│   Pool Service   │ │  Token Service   │ │Transaction Service│ │  Trader Service  │
├──────────────────┤ ├──────────────────┤ ├───────────────────┤ ├──────────────────┤
│ • Discovery      │ │ • Multi-source DB│ │ • Subject decode  │ │ • Entry eval     │
│ • Fetcher (batch)│ │ • Market data    │ │ • Batch processor │ │ • Exit eval      │
│ • Decoders (11)  │ │ • Security data  │ │ • DEX analyzer    │ │ • Executors      │
│ • Calculator     │ │ • Priority update│ │ • P&L calculation │ │ • Safety gates   │
│ • Analyzer       │ │ • Blacklist      │ │ • Subject SQLite  │ │ • DCA/Partial    │
└──────────────────┘ └──────────────────┘ └───────────────────┘ └──────────────────┘
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
│   Connectivity   │ │  Events System   │ │   Swap Router    │ │ Wallet Services  │
├──────────────────┤ ├──────────────────┤ ├──────────────────┤ ├──────────────────┤
│ • Endpoint health│ │ • Non-blocking   │ │ • Jupiter V6     │ │ • Balances       │
│ • Fallback logic │ │ • Categorized    │ │ • GMGN           │ │ • Multi-wallet   │
│ • Critical check │ │ • SQLite storage │ │ • Concurrent     │ │ • Shared watcher │
└──────────────────┘ └──────────────────┘ └──────────────────┘ └──────────────────┘
        │                    │                    │                    │
        ▼                    ▼                    ▼                    ▼
┌──────────────────┐ ┌──────────────────┐ ┌──────────────────┐ ┌──────────────────┐
│   AI Assistant   │ │ Telegram Service │ │   SOL Price      │ │  Update Checker  │
├──────────────────┤ ├──────────────────┤ ├──────────────────┤ ├──────────────────┤
│ • 9 LLM provid.  │ │ • Notifications  │ │ • Jupiter feed   │ │ • Version check  │
│ • Tool-calling   │ │ • Bot commands   │ │ • 30s refresh    │ │ • Auto-notify    │
│ • Scheduled tasks│ │ • Inline actions │ │ • USD conversion │ │ • Release notes  │
└──────────────────┘ └──────────────────┘ └──────────────────┘ └──────────────────┘
                                        │││
                                         ▼
┌─────────────────────────────────────────────────────────────────────────────────────┐
│                                 Web Dashboard                                       │
│         Axum REST API • Real-time Updates • Embedded Assets • Hot-reload Config     │
└─────────────────────────────────────────────────────────────────────────────────────┘
```

### Service Dependencies

```
Always available:
  - Webserver (first-run setup, Explore Mode, and full mode)

Explore tier (Explore Mode and full mode):
  - Connectivity, Events, Tokens, Filtering

Full trading tier:
  - Transactions initializes subject-scoped transaction storage
  - Wallet Watch starts after Connectivity + Transactions
  - Pools, Positions, Wallet, OHLCV, and Trader start in dependency order
  - Copy Trading starts after Wallet Watch + Filtering + Positions + Wallet + Pools

Control and automation tier:
  - AI, Scheduled AI Tasks, Telegram, Account, Updates, and supporting services
```

ScreenerBot has three customer boot states: **initialization** (webserver-only setup), **Explore Mode**
(market discovery and filtering without wallet-bound trading), and **full** (all enabled trading
services). Moving from Explore Mode to full persists validated wallet/RPC settings and performs a graceful
restart so the normal full boot path initializes every trading dependency cleanly.

### Main Data Flows

```text
Market APIs -> Tokens -> Filtering -> Strategies -> Trader admission -> Swap -> Position
Solana RPC -> Pool accounts -> Native decoders -> Live pool price -----^          |
OHLCV sources -> Candles -> Indicators and strategy conditions --------^          v
                                                                               Verification

Shared wallet watch -> Subject decode -> Activity broadcast -> Alerts / Own wallet / Copy tasks
Dashboard manual trade ------------------------------------------------> Shared swap + position path
```

Direct pool prices drive trading and P&L. OHLCV candles drive charts, indicators, and strategies;
the two price systems are deliberately separate.

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

Subject-scoped transaction decoding, persistence, and DEX analysis for the bot's wallet and watched
wallets.

- Consumes activity from the shared wallet observation service
- DEX classification (Jupiter, Raydium, Orca, Meteora, Pumpfun, GMGN, Fluxbeam, Moonshot)
- Swap detection and P&L calculation
- ATA operation tracking
- Position entry/exit verification
- Subject-keyed SQLite persistence with connection pooling

### Wallet Observation Service

One durable observation pipeline watches the bot wallet and user-selected external wallets.

- A single multiplexed WebSocket transport for every watched address
- HTTP cursor polling, reconnect gap-fill, and restart-safe recovery
- Durable signature deduplication before activity is broadcast
- Subject-relative swap and transfer classification
- Independent consumers for transaction history, wallet refresh, Telegram alerts, and copy tasks

### Position Manager

Complete position lifecycle with DCA and partial exit support.

- Multiple entries per position (DCA)
- Partial exits with individual P&L tracking
- Background price monitoring with peak tracking
- Loss detection with configurable auto-blacklist
- Typed provenance and management for automated, manual, and copy-originated positions

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

### Copy Trading Service

Task-based wallet copy built on the same observation, admission, swap, position, and verification
components used elsewhere in ScreenerBot.

- Every task begins in Paper mode; Live mode requires an explicit per-task confirmation
- Fixed-SOL or ratio-of-target sizing
- Per-trade, per-token, and total task limits
- Optional target-size filters, buy-once behavior, and filtering-pipeline requirement
- Buy-only, mirror-sell, and hybrid exit ownership modes
- Durable decisions, skip reasons, task spend, and activity history

---

## Supported DEXs

Native decoders for direct pool state interpretation:

| DEX          | Programs                    |
| ------------ | --------------------------- |
| **Raydium**  | CLMM, CPMM, Legacy AMM      |
| **Orca**     | Whirlpool                   |
| **Meteora**  | DAMM, DBC, DLMM             |
| **Pumpfun**  | AMM, Legacy (Bonding Curve) |
| **Fluxbeam** | AMM                         |
| **Moonit**   | AMM                         |

### Swap Routers

- **Jupiter V6**: Aggregation with route optimization
- **GMGN**: Alternative router for quote comparison
- **Raydium Direct**: Pool-specific execution components for supported Raydium pools

Enabled quote routers are queried concurrently with automatic best-output selection and retryable
fallback. Direct pool execution is separate from the quote-router registry.

---

## Trading Features

### Entry Evaluation

Safety checks in order:

1. Global force stop and period loss limit
2. Connectivity health
3. Position limits
4. Duplicate prevention
5. Re-entry cooldown
6. Blacklist check
7. Strategy signals

### Exit Evaluation

Priority-ordered conditions:

1. **Blacklist** (emergency): Immediate exit if token blacklisted
2. **Risk Limits** (emergency): >90% loss protection
3. **AI Analysis** (high, optional): Provider-backed exit decision support
4. **Stop Loss** (high): Fixed loss threshold from entry
5. **Trailing Stop** (high): Dynamic stop-loss following price peaks
6. **ROI Target** (normal): Fixed profit target exit
7. **Time Override** (normal): Maximum hold duration
8. **Strategy Exit** (normal): Strategy-defined exit signals

### DCA (Dollar Cost Averaging)

- Configurable DCA rounds with size multipliers
- Price drop thresholds for additional entries
- Per-round tracking with individual cost basis

### Partial Exits

- Multiple exit points per position
- Individual P&L calculation per exit
- Remaining position tracking

### Manual Trading

- Manual buys and sells from token, position, and trader surfaces
- User-controlled DCA and percentage-based partial exits
- Shared quote selection, force-stop gate, position transitions, and transaction verification
- Explicit manual position provenance so automated DCA and policy exits do not take ownership

### Wallet Copy Trading

- Watch multiple target wallets through the shared, restart-safe observation pipeline
- New tasks are Paper by default; Live execution is armed separately with confirmation
- Fixed or proportional sizing with per-trade, per-token, and total task limits
- Optional filtering, blacklist, target-size, self-copy, duplicate, cooldown, and position-capacity gates
- Buy-only, mirror-sell, or hybrid exit management
- Recent activity shows paper fills, live submissions, copied sells, failures, and typed skip reasons

---

## AI Assistant

Multi-provider LLM integration for intelligent analysis and automated tasks. All features disabled by default.

### Providers

Supports 9 providers: OpenAI, Anthropic, Groq, DeepSeek, Gemini, Ollama, Together AI, OpenRouter,
and Mistral.

### Features

- **Token Filtering**: AI evaluates tokens during the filtering pipeline with configurable confidence thresholds
- **Entry/Exit Analysis**: LLM-powered trade decision support with risk assessment
- **Interactive Chat**: Tool-calling chat interface with portfolio, trading, and system tools
- **Custom Instructions**: User-defined prompts injected into all AI evaluations
- **Automation**: Scheduled AI tasks with interval/daily/weekly schedules, headless tool execution, Telegram notifications, and run history tracking

### Automation

Create scheduled tasks that run AI instructions automatically:

- **Interval**: Run every N seconds (e.g., every 5 minutes)
- **Daily**: Run at a specific time UTC (e.g., 14:00)
- **Weekly**: Run on specific days at a time (e.g., mon,wed,fri:09:00)
- Configurable tool permissions (read-only or full access)
- Run history with tool call details and AI responses
- Telegram notifications on completion or failure

### MCP Bridge (External Agents)

An external MCP client (an AI coding agent, for example) can drive the same tool
registry over stdio. The `screenerbot mcp` subprocess holds no trading logic: it
discovers the already-running app from `agent-runtime.json` and calls an internal
loopback bridge, which resolves the client's scope, applies the permission
policy, and runs each tool in the live process.

- `screenerbot mcp serve` — run the stdio MCP server (JSON-RPC on stdout only).
- `screenerbot mcp doctor` — report whether the app is running and whether the
  pairing credential is valid. Never prints the secret.

Pairings are created, listed, and revoked through the dashboard-authenticated
`/api/agent-control/pairings` API. Creating one returns a one-time secret; only
its hash is stored. The integrated Agent Connections screen and guided install
flows are planned for the next product slice. The subprocess reads its credential
from the environment, never from command-line arguments:

- `SCREENERBOT_CLIENT_ID` — the pairing's client id.
- `SCREENERBOT_PAIRING_SECRET` — the one-time secret shown at creation.

Scopes are `read`, `operate`, `trade`. Unpaired, revoked, disabled or
app-not-running states expose zero capabilities. A tool that needs confirmation
(any trade, and anything the policy marks *ask*) is never executed by the agent
directly — it creates a request that a person approves or denies inside
ScreenerBot, and the approved request runs at most once.

---

## Dashboard

Embedded multi-page web interface (headless defaults to `http://localhost:8080`; Electron uses a
dynamic authenticated localhost port):

- **Dashboard**: Overview with positions, system health, and real-time stats
- **Positions**: Open/closed positions with P&L tracking and detailed analytics
- **Tokens**: Database browser with market data, security analysis, and pool info
- **Filtering**: Passed/rejected tokens with detailed rejection reasons
- **Trader**: Automated trading controls, wallet copy, monitors, safety gates, and loss limits
- **Transactions**: Own-wallet and watched-wallet history with DEX classification and P&L
- **Strategies**: Visual strategy builder with condition editor
- **OHLCV**: Candlestick charts with multi-timeframe analysis
- **Assistant**: AI chat, providers, instructions, automation, and testing
- **Wallets**: Multi-wallet management with balance monitoring
- **Tools**: Multi-wallet trading, ATA cleanup, burn tokens
- **Events**: System event log with filtering and search
- **Services**: Service health, metrics, and dependency status
- **Config**: Hot-reload configuration editor with metadata-driven UI
- **Updates**: Version checking and release notes
- **About**: System information and credits
- **Lockscreen**: Security screen with password/TOTP protection
- **Login**: Authentication flow
- **Setup**: First-run initialization wizard
- **Onboarding**: Guided setup for new users

---

## Configuration

Managed through `data/config.toml` in the platform app data directory with hot-reload support.
Core configuration sections:

| Section          | Purpose                                                        |
| ---------------- | -------------------------------------------------------------- |
| `[trader]`       | Position limits, sizing, ROI targets, DCA, trailing stop       |
| `[copy_trading]` | Global copy-task enablement, limits, slippage, filter policy   |
| `[positions]`    | Position tracking, partial exits, cooldowns                    |
| `[filtering]`    | Token filtering with nested DexScreener/GeckoTerminal/Rugcheck |
| `[swaps]`        | Router configuration (Jupiter, GMGN, Raydium)                  |
| `[tokens]`       | Token database, update intervals                               |
| `[pools]`        | Pool discovery, caching                                        |
| `[rpc]`          | RPC endpoints and rate limiting                                |
| `[ohlcv]`        | Candlestick data settings                                      |
| `[strategies]`   | Strategy engine configuration                                  |
| `[wallet]`       | Wallet monitoring                                              |
| `[holder_watch]` | Holder-monitoring tool behavior                                |
| `[events]`       | Event system settings                                          |
| `[services]`     | Service manager settings                                       |
| `[monitoring]`   | System metrics                                                 |
| `[connectivity]` | Endpoint health monitoring                                     |
| `[sol_price]`    | SOL/USD price service                                          |
| `[gui]`          | Desktop application settings                                   |
| `[webserver]`    | Headless host, port, sessions, and authentication              |
| `[llm]`          | LLM provider credentials, models, rate limits, master switch   |
| `[llm_analysis]` | Model-scored filtering and trading analysis                    |
| `[assistant]`    | Dashboard chat and scheduled automation                        |
| `[agent_control]` | Shared agent/MCP tool availability and permission policy      |
| `[telegram]`     | Telegram bot, notifications, commands                          |
| `[performance]`  | Cache and memory tuning                                        |
| `[maintenance]`  | Retention, vacuum, and checkpoint schedules                    |
| `[network]`      | Network proxy settings                                         |
| `[account]`      | Optional ScreenerBot account settings                          |

Access via `with_config(|cfg| cfg.trader.max_open_positions)`. Hot-reload with `reload_config()`.
Per-target copy task mode, sizing, budgets, and exit ownership live in `copy_trading.db`, not in
global TOML.

---

## Data Sources

| Source            | Usage                                 |
| ----------------- | ------------------------------------- |
| **Solana RPC**    | Pool reserves, balances, transactions |
| **DexScreener**   | Market data, pool discovery           |
| **GeckoTerminal** | Alternative market metrics            |
| **Rugcheck**      | Security analysis                     |
| **Jupiter**       | Swap routing and quotes               |
| **CoinGecko**     | Token metadata                        |
| **DefiLlama**     | Token prices, DeFi protocols          |

All data cached locally in SQLite databases.

### RPC Provider

A premium Solana RPC endpoint is **required** for reliable trading. ScreenerBot auto-detects your provider and applies optimal rate limits.

| Provider                                              | Compatibility      | Notes                                                                                             |
| ----------------------------------------------------- | ------------------ | ------------------------------------------------------------------------------------------------- |
| **[Helius](https://www.helius.dev/solana-rpc-nodes)** | ⭐ **Recommended** | Most compatible and tested. Solana-native APIs, DAS, staked connections. Free tier: 100k req/day. |
| [QuickNode](https://www.quicknode.com)                | ✅ Supported       | Fast global network. Good alternative.                                                            |
| [Triton](https://triton.one)                          | ✅ Supported       | Ultra-low latency, gRPC support.                                                                  |
| [Alchemy](https://www.alchemy.com)                    | ✅ Supported       | Developer-friendly, generous free tier.                                                           |

> **Tip:** Configure 2-3 endpoints from different providers for automatic failover. See the [Best RPC Providers Guide](https://screenerbot.io/blog/best-rpc-providers) for detailed comparisons.

---

## Desktop Application

Native desktop application built with **Electron** - the proven framework behind apps like VS Code, Slack, and Discord.

### Platform Support

| Platform    | Min Version         | Package Format       |
| ----------- | ------------------- | -------------------- |
| **macOS**   | 10.13 (High Sierra) | `.app` / `.dmg`      |
| **Windows** | Windows 10          | `.exe` / `.msi`      |
| **Linux**   | Ubuntu 18.04+       | `.deb` / `.AppImage` |

### Desktop Features

- **Native Window**: 1400x900 default, 1200x700 minimum, fully resizable
- **Embedded Dashboard**: Electron launches the Rust backend on a dynamic authenticated localhost port
- **Keyboard Shortcuts**: Zoom (Cmd/Ctrl +/-/0), Reload (Cmd/Ctrl + R)
- **System Integration**: Native title bar, notifications

---

## Quick Install (VPS/Linux)

Run ScreenerBot 24/7 on a Linux VPS with a single command:

```bash
curl -fsSL https://screenerbot.io/install.sh | bash
```

> See the [VPS Installation Guide](https://screenerbot.io/docs/getting-started/installation/vps) for detailed setup instructions including system requirements and management.

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

# Build the Rust engine
cargo build --release

# The headless binary is now at target/release/screenerbot
```

### Run

```bash
# Headless mode
./target/release/screenerbot

# Desktop application (requires the Rust build first)
cd electron
npm install
npm start

# With debug logging
cargo run --bin screenerbot -- --debug-rpc
```

### Build Artifacts

Electron packaging uses `npm run make` inside `electron/`. See [BUILD.md](BUILD.md) for the current
platform dependencies, package outputs, and cross-compilation instructions.

- **Rust debug binary**: `target/debug/screenerbot`
- **Rust release binary**: `target/release/screenerbot`
- **Desktop packages**: `electron/out/`

---

## Project Structure

```
src/
├── actions/        # Operation progress tracking with SSE broadcasting
├── ai/             # AI assistant (9 LLM providers, chat, automation)
├── apis/           # External API clients (DexScreener, Jupiter, Rugcheck, LLM)
├── config/         # Macro-driven configuration system with hot-reload
├── connectivity/   # Endpoint health monitoring with fallback strategies
├── errors/         # Structured error types with blockchain-aware parsing
├── events/         # Structured JSON event logging (SQLite)
├── filtering/      # Multi-criteria token evaluation engine
├── ohlcvs/         # OHLCV candlestick data (7 timeframes)
├── pools/          # Pool service with 11 native DEX decoders
├── positions/      # Position lifecycle (DCA, partial exits, P&L)
├── rpc/            # Multi-provider RPC with rate limiting & circuit breaker
├── run/            # Initialization, Explore Mode, and full-mode bootstrap
├── services/       # ServiceManager lifecycle, readiness, health, and metrics
├── strategies/     # Condition-based trading strategy engine
├── swaps/          # Quote-router registry and swap execution
├── telegram/       # Telegram bot (notifications, commands, inline actions)
├── tokens/         # Token database with multi-source aggregation
├── trader/         # Automated, manual, and wallet-copy trading logic
├── transactions/   # Subject-scoped transaction decode and persistence
├── wallets/        # Wallet management, balances, and shared observation
└── webserver/      # Axum REST API + embedded dashboard assets

electron/           # Electron desktop shell
docs/architecture/  # Living module architecture documentation
tests/              # Domain-organized integration tests
```

---

## Links & Resources

| Resource                  | Link                                                             |
| ------------------------- | ---------------------------------------------------------------- |
| 🌐 **Website**            | [screenerbot.io](https://screenerbot.io)                         |
| 📚 **Documentation**      | [screenerbot.io/docs](https://screenerbot.io/docs)               |
| ⬇️ **Download**           | [screenerbot.io/download](https://screenerbot.io/download)       |
| 💬 **Telegram Community** | [t.me/screenerbotio_talk](https://t.me/screenerbotio_talk)       |
| 📢 **Telegram Channel**   | [t.me/screenerbotio](https://t.me/screenerbotio)                 |
| 🆘 **Telegram Support**   | [t.me/screenerbotio_support](https://t.me/screenerbotio_support) |
| 𝕏 **X (Twitter)**         | [x.com/screenerbotio](https://x.com/screenerbotio)               |

---

## Contributing

We welcome contributions from the community! Whether you're fixing a bug, adding a feature, or improving documentation — every contribution matters.

### Getting Started

1. **Join the community** — Start by joining our [Telegram Community](https://t.me/screenerbotio_talk) to discuss ideas, ask questions, and coordinate with other contributors
2. **Read the docs** — Browse the [documentation](https://screenerbot.io/docs) for architecture details, coding patterns, and project structure
3. **Fork & branch** — Fork the repository and create a feature branch from `main`
4. **Follow patterns** — Match existing code style, naming conventions, and module structure
5. **Validate** — Ensure `cargo check --lib` passes before submitting
6. **Open a PR** — Submit a pull request with a clear description of your changes

### Areas for Contribution

- **DEX decoders** — Add support for new Solana DEX protocols
- **Strategy conditions** — Implement new technical indicators and conditions
- **Dashboard improvements** — UI/UX enhancements, new visualizations
- **Documentation** — Improve guides, add tutorials, translate docs
- **Bug reports** — Found an issue? [Open a GitHub issue](https://github.com/farfary/ScreenerBot/issues)

> 💬 **Not sure where to start?** Ask in our [Telegram Community](https://t.me/screenerbotio_talk) — we'll help you find something that matches your skills!

---

## Community

<p align="center">
  <strong>Join the ScreenerBot community — your gateway to Solana DeFi trading</strong>
</p>

<p align="center">
  <a href="https://t.me/screenerbotio_talk"><img src="https://img.shields.io/badge/💬_Telegram_Community-Join_Discussion-26A5E4?style=for-the-badge&logo=telegram&logoColor=white" alt="Telegram Community"></a>
</p>

<p align="center">
  Our <a href="https://t.me/screenerbotio_talk"><strong>Telegram Community</strong></a> is the main hub for everything ScreenerBot:<br>
  🗣️ Discuss trading strategies and share insights<br>
  🐛 Report bugs and request features<br>
  🤝 Find contributors and collaborate on code<br>
  📢 Get announcements and early access to new features<br>
  🆘 Get help from the community and the development team
</p>

<p align="center">
  <a href="https://t.me/screenerbotio"><img src="https://img.shields.io/badge/Telegram-Channel-26A5E4?style=for-the-badge&logo=telegram&logoColor=white" alt="Telegram Channel"></a>
  <a href="https://t.me/screenerbotio_talk"><img src="https://img.shields.io/badge/Telegram-Community-26A5E4?style=for-the-badge&logo=telegram&logoColor=white" alt="Telegram Community"></a>
  <a href="https://t.me/screenerbotio_support"><img src="https://img.shields.io/badge/Telegram-Support-26A5E4?style=for-the-badge&logo=telegram&logoColor=white" alt="Telegram Support"></a>
  <a href="https://x.com/screenerbotio"><img src="https://img.shields.io/badge/X-Follow-000000?style=for-the-badge&logo=x&logoColor=white" alt="X (Twitter)"></a>
  <a href="https://screenerbot.io"><img src="https://img.shields.io/badge/Website-screenerbot.io-9945FF?style=for-the-badge" alt="Website"></a>
</p>

---

<p align="center">
  <img src="https://img.shields.io/badge/Built%20with-Rust-000000?style=flat-square&logo=rust&logoColor=white" alt="Rust">
  <img src="https://img.shields.io/badge/Powered%20by-Solana-9945FF?style=flat-square&logo=solana&logoColor=white" alt="Solana">
  <img src="https://img.shields.io/badge/Desktop-Electron-47848F?style=flat-square&logo=electron&logoColor=white" alt="Electron">
</p>

---

## Author

**Farhad Arghavan**

Contact: [info@screenerbot.io](mailto:info@screenerbot.io)

## License

This project is licensed under the [Business Source License 1.1](LICENSE) (BSL 1.1).

- **Non-commercial use** is permitted
- **Commercial use** (competing products or paid services) requires a separate license
- Contact [info@screenerbot.io](mailto:info@screenerbot.io) for alternative licensing
