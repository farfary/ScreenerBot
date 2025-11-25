# ScreenerBot

**Automated Solana DeFi Trading Bot**

A high-performance, fully automated trading bot for Solana DeFi with comprehensive token screening, real-time monitoring, and intelligent trade execution.

![Rust](https://img.shields.io/badge/Rust-1.75+-orange?logo=rust)
![Solana](https://img.shields.io/badge/Solana-Mainnet-purple?logo=solana)
![License](https://img.shields.io/badge/License-Proprietary-red)

---

## ✨ Features

### 🔍 Token Discovery & Filtering
- **Multi-Source Discovery**: Aggregates tokens from DexScreener, GeckoTerminal, and on-chain data
- **Advanced Filtering Engine**: Configurable rules for market cap, liquidity, holder distribution, and security
- **RugCheck Integration**: Automated security analysis with risk scoring
- **Real-time Updates**: Continuous monitoring with priority-based refresh rates

### 📊 12+ DEX Pool Decoders
Native on-chain price calculation from pool accounts:

| DEX | Pool Types |
|-----|------------|
| **Raydium** | CLMM, CPMM, Legacy AMM |
| **Orca** | Whirlpool |
| **Meteora** | DAMM, DBC, DLMM |
| **Pumpfun** | AMM, Legacy Bonding Curves |
| **Fluxbeam** | AMM |
| **Moonit** | AMM |

### 🔄 Multi-Router Swap Execution
- **Jupiter V6**: Optimal routing across 20+ DEXs
- **GMGN**: Alternative router for redundancy
- **Concurrent Quotes**: Fetches quotes in parallel, selects best rate
- **Auto Slippage**: Dynamic slippage based on liquidity

### 📈 Trading System

#### Entry Logic
- **Strategy-Based Entry**: Configurable entry strategies with conditions
- **Safety Gates**: Position limits, cooldowns, blacklist checks
- **Connectivity Checks**: Ensures RPC and API health before trading

#### Exit Logic (Priority Order)
1. **Emergency Blacklist Exit**: Immediate sell if token flagged
2. **Risk Limits**: Auto-exit on >90% loss
3. **Trailing Stop**: Dynamic stop-loss following price peaks
4. **ROI Target**: Take profit at configured percentage
5. **Time Override**: Exit after maximum hold time
6. **Strategy Exit**: Custom exit conditions

#### Advanced Features
- **DCA (Dollar Cost Averaging)**: Automated position averaging
- **Partial Exits**: Sell portions of positions
- **Manual Override**: Force buy/sell via dashboard

### 🎯 Strategy Engine
Condition-based trading strategies with rule trees:

| Condition Type | Description |
|----------------|-------------|
| `price_change_percent` | Price movement threshold |
| `volume_spike` | Unusual volume detection |
| `price_to_ma` | Distance from moving average |
| `price_breakout` | Support/resistance breaks |
| `candle_size` | Candle body/wick analysis |
| `consecutive_candles` | Pattern detection |
| `liquidity_level` | Pool liquidity thresholds |
| `position_holding_time` | Time-based exits |

### 📉 OHLCV Data System
- **Multi-Timeframe**: 1m, 5m, 15m, 1h, 4h, 1d candles
- **Priority-Based Monitoring**: Active positions get faster updates
- **Gap Detection**: Automatic backfill of missing data
- **Technical Analysis Ready**: MA, RSI, MACD calculations

### 🖥️ Web Dashboard
Real-time monitoring interface on `http://localhost:8080`:

| Page | Features |
|------|----------|
| **Home** | Portfolio overview, P&L summary, active positions |
| **Tokens** | Token database with filtering, search, details |
| **Positions** | Open/closed positions with live P&L |
| **Trader** | Trading controls, entry/exit settings |
| **Filtering** | Filter configuration and passed/rejected tokens |
| **Strategies** | Create and manage trading strategies |
| **Transactions** | Real-time transaction monitoring |
| **Events** | System event log with severity levels |
| **Wallet** | Balance tracking, SOL/token holdings |
| **Services** | Service health and metrics |
| **Config** | Runtime configuration editor |

### 🔐 License System
- **NFT-Based Licensing**: Solana NFT verification
- **Tier-Based Features**: Starter, Pro, Enterprise plans
- **Automatic Verification**: Background license checks

### 🏗️ Architecture

#### Service-Based Design
18 independent services with dependency management:
- Priority-based startup (topological sort)
- Health monitoring and metrics
- Graceful shutdown with reverse-order stop

#### Data Storage
- **SQLite Databases**: Positions, tokens, transactions, events, OHLCV
- **JSON Config**: `data/config.toml` with hot-reload support
- **In-Memory Caches**: Pool prices, token data, filtering snapshots

#### Real-Time Data
- **WebSocket Connections**: Transaction streaming
- **RPC Polling**: Account and balance updates
- **Rate Limiting**: Configurable per-endpoint limits

---

## 🚀 Quick Start

### Prerequisites
- Rust 1.75+
- Solana CLI (optional)
- Valid ScreenerBot license NFT

### Installation

```bash
# Clone the repository
git clone https://github.com/farfary/ScreenerBot.git
cd ScreenerBot

# Build the bot
cargo build --release

# Run (first launch opens initialization wizard)
cargo run --release
```

### First Launch
1. Bot starts webserver on `http://localhost:8080`
2. Open browser and complete initialization:
   - Enter wallet private key
   - Configure RPC endpoints
   - Verify license NFT
3. Bot automatically restarts with full services

### Configuration
All settings in `data/config.toml`:

```toml
[trader]
enabled = true
max_positions = 5
buy_amount_sol = 0.1
dry_run = false

[positions]
trailing_stop_enabled = true
trailing_stop_activation_pct = 10.0
trailing_stop_distance_pct = 5.0
roi_exit_threshold_pct = 50.0

[filtering]
min_liquidity_sol = 10.0
min_market_cap_usd = 50000
holder_distribution_enabled = true
```

---

## 🛠️ Development

### Build Commands

```bash
# Fast check (no codegen)
cargo check --lib

# Debug build
cargo build

# Release build
cargo build --release

# Run with debug logging
cargo run -- --debug-trader --debug-rpc
```

### Debug Flags
- `--debug-rpc`: RPC call details
- `--debug-trader`: Trading decisions
- `--debug-transactions`: Transaction analysis
- `--debug-websocket`: WebSocket events
- `--verbose`: All debug output

### Project Structure

```
src/
├── main.rs           # Entry point
├── run.rs            # Bot lifecycle
├── services/         # 18 service implementations
├── trader/           # Trading logic
│   ├── evaluators/   # Entry/exit evaluation
│   ├── executors/    # Buy/sell execution
│   ├── monitors/     # Position monitoring
│   └── safety/       # Risk management
├── pools/            # Pool management
│   └── decoders/     # 12 DEX decoders
├── tokens/           # Token database
├── filtering/        # Filter engine
├── strategies/       # Strategy system
│   └── conditions/   # 8 condition types
├── swaps/            # Jupiter & GMGN
├── webserver/        # Dashboard
│   ├── routes/       # API endpoints
│   └── templates/    # Frontend
└── config/           # Configuration system
```

---

## 📊 Performance

- **Startup Time**: ~15-20 seconds
- **Memory Usage**: ~200-400 MB
- **RPC Calls**: Rate-limited to 20/sec
- **Pool Decoding**: <10ms per pool
- **Price Updates**: Every 30 seconds (configurable)

---

## ⚠️ Disclaimer

This software is for educational and research purposes. Trading cryptocurrencies involves substantial risk of loss. Past performance does not guarantee future results. Always do your own research and never invest more than you can afford to lose.

---

## 📄 License

Proprietary software. Requires valid ScreenerBot license NFT for operation.

---

## 🔗 Links

- **Website**: [screenerbot.com](https://screenerbot.com)
- **Documentation**: [docs.screenerbot.com](https://docs.screenerbot.com)
- **Support**: support@screenerbot.com
