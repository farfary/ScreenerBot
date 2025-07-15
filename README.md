# 🤖 ScreenerBot - Advanced Solana DEX Trading Bot

[![Rust](https://img.shields.io/badge/rust-2021-orange.svg)](https://www.rust-lang.org/)
[![Solana](https://img.shields.io/badge/solana-mainnet-purple.svg)](https://solana.com/)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

A sophisticated, multi-source token discovery and trading bot for the Solana ecosystem with real-time monitoring, intelligent filtering, and automated trading capabilities.

## 🔥 Key Features

### 🔍 **Multi-Source Token Discovery**
- **DexScreener Integration**: Latest token profiles, boosted tokens, and top trending tokens
- **RugCheck Integration**: New tokens, recent activity, trending tokens, and verified tokens  
- **Real-time Discovery**: Continuous background scanning with configurable intervals
- **Smart Filtering**: Liquidity, volume, market cap, and custom blacklist filtering
- **Duplicate Detection**: Automatic deduplication across multiple sources

### 💼 **Advanced Wallet Tracking**
- **Real-time Balance Monitoring**: SOL and SPL token balance tracking
- **Position Management**: Track all token positions with P&L calculations
- **Portfolio Analytics**: Total portfolio value and performance metrics
- **Transaction History**: Complete transaction logging and analysis

### 📊 **Intelligent Trading Engine**
- **Signal Generation**: Advanced market analysis and trading signals
- **Risk Management**: Configurable stop-loss and take-profit levels
- **Confidence Scoring**: AI-driven confidence metrics for each trade signal
- **Safety Features**: Trading disabled by default with comprehensive safeguards

### 🎯 **Professional Console Interface**
- **Real-time Dashboard**: Live status updates with emoji indicators
- **Discovery Alerts**: Immediate notifications when new tokens are found
- **Color-coded Logging**: Easy-to-read status messages with timestamps
- **Performance Metrics**: Discovery rates, portfolio performance, and system health

## 🚀 Quick Start

### Prerequisites
- Rust 1.70+ installed
- Solana CLI tools (optional, for advanced features)
- Active internet connection for API access

### Installation

1. **Clone the repository:**
   ```bash
   git clone <repository-url>
   cd ScreenerBot
   ```

2. **Configure your settings:**
   Edit `configs.json` with your wallet private key and preferences:
   ```json
   {
     "main_wallet_private": "YOUR_SOLANA_PRIVATE_KEY_HERE",
     "rpc_url": "https://api.mainnet-beta.solana.com",
     "discovery": {
       "enabled": true,
       "interval_seconds": 60,
       "sources": ["dexscreener", "rugcheck"]
     }
   }
   ```

3. **Build and run:**
   ```bash
   cargo build --release
   cargo run
   ```

## 📋 Configuration Guide

### Discovery Configuration
```json
"discovery": {
  "enabled": true,
  "interval_seconds": 60,           // Discovery frequency (seconds)
  "min_liquidity": 10000.0,         // Minimum liquidity filter ($)
  "min_volume_24h": 50000.0,        // Minimum 24h volume ($)
  "max_market_cap": 1000000.0,      // Maximum market cap ($)
  "min_market_cap": 10000.0,        // Minimum market cap ($)
  "blacklisted_tokens": [],         // Tokens to ignore
  "sources": ["dexscreener", "rugcheck"]  // Active discovery sources
}
```

### Trading Configuration
```json
"trader": {
  "enabled": false,                 // KEEP FALSE for safety
  "max_position_size": 0.1,         // Max position size (SOL)
  "stop_loss_percentage": 5.0,      // Stop loss %
  "take_profit_percentage": 20.0,   // Take profit %
  "max_slippage": 1.0,             // Max slippage %
  "min_confidence_score": 0.7       // Min confidence for trades
}
```

## 🔌 API Sources

### DexScreener APIs
- **Token Profiles**: `https://api.dexscreener.com/token-profiles/latest/v1`
- **Latest Boosts**: `https://api.dexscreener.com/token-boosts/latest/v1`
- **Top Boosts**: `https://api.dexscreener.com/token-boosts/top/v1`
- **Rate Limit**: 60 requests/minute

### RugCheck APIs
- **New Tokens**: `https://api.rugcheck.xyz/v1/stats/new_tokens`
- **Recent Activity**: `https://api.rugcheck.xyz/v1/stats/recent`
- **Trending**: `https://api.rugcheck.xyz/v1/stats/trending`
- **Verified**: `https://api.rugcheck.xyz/v1/stats/verified`

## 🏗️ Architecture

### Module Structure
```
src/
├── discovery/              # Multi-source token discovery
│   ├── mod.rs             # Main discovery orchestrator
│   └── sources/           # Individual API source implementations
│       ├── dexscreener.rs # DexScreener API integration
│       ├── rugcheck.rs    # RugCheck API integration
│       └── mod.rs         # Source trait definition
├── wallet.rs              # Wallet tracking and portfolio management
├── trader.rs              # Trading signal generation and execution
├── database.rs            # SQLite persistence layer
├── logger.rs              # Console UI and logging system
├── config.rs              # Configuration management
├── types.rs               # Core data structures
└── main.rs                # Application orchestration
```

### Key Components

**Discovery Engine**
- Pluggable source architecture using traits
- Concurrent API fetching with rate limiting
- Real-time filtering and deduplication
- Persistent caching with SQLite

**Wallet Tracker**
- SPL token account monitoring
- Real-time balance calculations
- Position tracking with P&L
- Portfolio performance analytics

**Trading System**
- Market signal generation
- Risk management algorithms
- Confidence-based decision making
- Safety-first approach

## 📈 Sample Output

```
════════════════════════════════════════════════════════════
🤖 SCREENER BOT - SOLANA DEX TRADER BOT
════════════════════════════════════════════════════════════
ℹ️  [12:09:32] Starting ScreenerBot...
✅ [12:09:32] Configuration loaded successfully
✅ [12:09:32] Database initialized successfully
🔎 [12:09:47] 🆕 NEW TOKEN FOUND via RUGCHECK: PILLWAR (Token 6ywvC36Q) | 💰 MC: $0 | 💧 Liq: $0 | 📊 Vol: $0
🔎 [12:09:47] 🆕 NEW TOKEN FOUND via RUGCHECK: DeepSeekAI (Token Dbt8kXUE) | 💰 MC: $0 | 💧 Liq: $0 | 📊 Vol: $0
🔎 [12:09:47] ✅ DISCOVERY found 35 new tokens from RugCheck
💼 [12:09:48] WALLET: SOL Balance: 0.5767 SOL
📈 [12:09:48] TRADER: DISABLED (for safety)
```

## 🛡️ Safety Features

- **Trading Disabled by Default**: Prevents accidental trades
- **Read-only Wallet Operations**: Only monitors, doesn't execute
- **Comprehensive Error Handling**: Robust error recovery
- **Rate Limiting**: Respects API limits to prevent bans
- **Data Validation**: Strict input validation and sanitization

## 🔧 Development

### Adding New Discovery Sources

1. **Create source file**: `src/discovery/sources/newsource.rs`
2. **Implement SourceTrait**:
   ```rust
   use super::SourceTrait;
   use async_trait::async_trait;
   
   pub struct NewSource {
       client: Client,
   }
   
   #[async_trait]
   impl SourceTrait for NewSource {
       fn name(&self) -> &str { "NewSource" }
       async fn discover(&self) -> Result<Vec<TokenInfo>> {
           // Implementation
       }
   }
   ```
3. **Register in mod.rs**: Add to sources module
4. **Add to config**: Include in `configs.json` sources array

### Database Schema
```sql
-- Tokens table
CREATE TABLE tokens (
    mint TEXT PRIMARY KEY,
    symbol TEXT NOT NULL,
    name TEXT NOT NULL,
    decimals INTEGER NOT NULL,
    -- ... additional fields
);

-- Positions table  
CREATE TABLE positions (
    wallet_address TEXT NOT NULL,
    mint TEXT NOT NULL,
    balance REAL NOT NULL,
    -- ... additional fields
);
```

## 🤝 Contributing

1. Fork the repository
2. Create a feature branch: `git checkout -b feature-name`
3. Make your changes and add tests
4. Commit: `git commit -m 'Add feature-name'`
5. Push: `git push origin feature-name`
6. Submit a pull request

## ⚠️ Disclaimer

This software is for educational and research purposes only. Trading cryptocurrencies involves significant risk and may result in financial loss. Always:

- Test thoroughly on devnet/testnet first
- Start with small amounts
- Never trade more than you can afford to lose
- Understand the risks of automated trading
- Comply with local regulations

## 📝 License

This project is licensed under the MIT License - see the LICENSE file for details.

## 🆘 Support

- Create an issue for bug reports
- Join our Discord for community support
- Check the Wiki for advanced configuration
- Review the API documentation for integration details
