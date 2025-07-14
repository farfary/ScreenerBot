# ScreenerBot 🤖

[![Rust](https://img.shields.io/badge/rust-2021-orange.svg)](https://www.rust-lang.org/)
[![Solana](https://img.shields.io/badge/solana-2.3.1-purple.svg)](https://solana.com/)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

**ScreenerBot** is an advanced, automated cryptocurrency trading bot specifically designed for the Solana ecosystem. It provides intelligent token discovery, risk assessment, and automated trading capabilities with comprehensive portfolio management.

## ✨ Features

### 🔍 **Multi-Source Token Screening**

- **DexScreener Integration**: Real-time token discovery from DEX aggregators
- **GeckoTerminal Support**: Professional-grade market data
- **Raydium Native**: Direct integration with Solana's premier DEX
- **RugCheck Analysis**: Automated security and rug-pull risk assessment

### 💱 **Advanced Trading Engine**

- **Jupiter Aggregator**: Optimal routing for best execution prices
- **GMGN Integration**: Professional trading interface support
- **Automated DCA**: Dollar-cost averaging for position building
- **Risk Management**: Stop-loss, take-profit, and position sizing
- **Slippage Protection**: Configurable slippage tolerance

### 📊 **Portfolio Management**

- **Real-time P&L Tracking**: Comprehensive profit/loss monitoring
- **Position Analytics**: Detailed performance metrics per token
- **Rebalancing Recommendations**: AI-driven portfolio optimization
- **Historical Performance**: Transaction history and trend analysis

### 🗄️ **Intelligent Caching**

- **SQLite Database**: Local caching for faster operations
- **Transaction History**: Persistent storage of all trades
- **Token Metadata**: Cached token information and metrics
- **Market Data**: Historical price and volume data

### 🛡️ **Security & Risk Management**

- **Wallet Integration**: Secure keypair management
- **Transaction Simulation**: Pre-execution validation
- **Risk Scoring**: Multi-factor risk assessment
- **Position Limits**: Configurable exposure controls

## 🚀 Quick Start

### Prerequisites

- **Rust** (2021 edition or later)
- **Solana CLI** tools
- **Git**

### Installation

1. **Clone the repository**

   ```bash
   git clone https://github.com/farfary/ScreenerBot.git
   cd ScreenerBot
   ```

2. **Build the project**

   ```bash
   cargo build --release
   ```

3. **Configure your settings**

   ```bash
   cp configs.json.example configs.json
   # Edit configs.json with your wallet and preferences
   ```

4. **Run the bot**
   ```bash
   cargo run --release
   ```

## ⚙️ Configuration

### Basic Configuration (`configs.json`)

```json
{
  "main_wallet_public": "YOUR_WALLET_PUBLIC_KEY",
  "main_wallet_private": "YOUR_WALLET_PRIVATE_KEY",

  "trading": {
    "entry_amount_sol": 0.001,
    "max_positions": 50,
    "dca_enabled": true
  },

  "screener_config": {
    "sources": {
      "dexscreener_enabled": true,
      "geckoterminal_enabled": true,
      "raydium_enabled": true,
      "rugcheck_enabled": true
    },
    "filters": {
      "min_volume_24h": 1000.0,
      "min_liquidity": 5000.0,
      "max_age_hours": 24,
      "require_verified": false
    }
  }
}
```

### Trading Parameters

| Parameter                | Description                  | Default |
| ------------------------ | ---------------------------- | ------- |
| `entry_amount_sol`       | SOL amount per trade         | 0.001   |
| `max_positions`          | Maximum concurrent positions | 50      |
| `dca_enabled`            | Enable dollar-cost averaging | true    |
| `take_profit_percentage` | Profit-taking threshold      | 20%     |
| `max_slippage`           | Maximum acceptable slippage  | 5%      |

### Screening Filters

| Filter             | Description                      | Default  |
| ------------------ | -------------------------------- | -------- |
| `min_volume_24h`   | Minimum 24h trading volume (USD) | 1000     |
| `min_liquidity`    | Minimum liquidity (USD)          | 5000     |
| `max_age_hours`    | Maximum token age                | 24 hours |
| `require_verified` | Only verified tokens             | false    |

## 🏗️ Architecture

```
ScreenerBot/
├── src/
│   ├── core/           # Core bot runtime and configuration
│   ├── wallet/         # Solana wallet management
│   ├── screener/       # Token discovery and analysis
│   ├── trader/         # Trading logic and execution
│   ├── portfolio/      # Portfolio tracking and analytics
│   ├── cache/          # Data persistence and caching
│   └── swap/           # DEX integration and routing
├── configs.json        # Bot configuration
└── cache.db           # SQLite database (auto-created)
```

### Core Components

#### 🧠 **BotRuntime** (`src/core/`)

- Central coordinator managing all bot components
- Configuration loading and validation
- Main execution loop with 60-second cycles
- Graceful shutdown handling

#### 👛 **WalletManager** (`src/wallet/`)

- Solana wallet integration and key management
- Balance tracking (SOL and SPL tokens)
- Transaction history and parsing
- Token account discovery

#### 🔍 **ScreenerManager** (`src/screener/`)

- Multi-source token discovery
- Risk assessment and scoring
- Opportunity filtering and ranking
- Market data aggregation

#### 💱 **TraderManager** (`src/trader/`)

- Trade signal generation
- Order execution and routing
- Position management
- Risk controls and limits

#### 📈 **PortfolioManager** (`src/portfolio/`)

- Real-time portfolio tracking
- Performance analytics
- P&L calculation
- Rebalancing recommendations

## 🔌 API Integrations

### Supported DEX Protocols

- **Jupiter Aggregator**: Optimal swap routing
- **Raydium**: Native Solana AMM
- **Orca**: Low-slippage trading
- **Serum**: Orderbook-based trading

### Data Sources

- **DexScreener**: Real-time DEX data
- **GeckoTerminal**: Professional market data
- **RugCheck**: Security analysis
- **Solana RPC**: On-chain data

## 📊 Monitoring & Analytics

### Real-time Metrics

- **Portfolio Value**: Current total value in SOL
- **Active Positions**: Number of open positions
- **Total P&L**: Unrealized + realized profits/losses
- **Success Rate**: Percentage of profitable trades

### Performance Analytics

- **Trade Frequency**: Average trades per day
- **Hold Duration**: Average position holding time
- **Risk-Adjusted Returns**: Sharpe ratio and volatility
- **Drawdown Analysis**: Maximum portfolio decline

## 🛠️ Development

### Building from Source

```bash
# Debug build
cargo build

# Release build (optimized)
cargo build --release

# Run tests
cargo test

# Check code formatting
cargo fmt

# Run linter
cargo clippy
```

### Project Structure

```rust
// Core modules
mod core;      // Bot runtime and configuration
mod wallet;    // Solana wallet management
mod screener;  // Token discovery
mod trader;    // Trading execution
mod portfolio; // Portfolio tracking
mod cache;     // Data persistence
mod swap;      // DEX integrations
```

### Key Dependencies

| Crate            | Version | Purpose                       |
| ---------------- | ------- | ----------------------------- |
| `solana-sdk`     | 2.3.1   | Solana blockchain integration |
| `spl-token-2022` | 8.0.1   | Token program support         |
| `tokio`          | 1.0     | Async runtime                 |
| `reqwest`        | 0.11    | HTTP client                   |
| `rusqlite`       | 0.29    | SQLite database               |
| `jup-ag`         | 0.8.0   | Jupiter aggregator            |

## 🔐 Security Considerations

### Private Key Management

- **Never commit private keys** to version control
- Use environment variables or secure config files
- Consider hardware wallet integration for production

### Risk Management

- **Start with small amounts** while testing
- **Monitor positions closely** during initial runs
- **Set appropriate stop-losses** to limit downside

### Network Security

- **Use secure RPC endpoints** (avoid public/free RPCs for production)
- **Implement rate limiting** to avoid API restrictions
- **Monitor for suspicious transactions**

## 📈 Trading Strategies

### Built-in Strategies

#### 1. **Momentum Trading**

- Detects tokens with high volume spikes
- Enters positions on confirmed breakouts
- Uses trailing stops for profit protection

#### 2. **Mean Reversion**

- Identifies oversold conditions
- Dollar-cost averages into positions
- Exits on return to mean price levels

#### 3. **Liquidity Farming**

- Targets newly listed tokens with growing liquidity
- Focuses on tokens with verified development teams
- Implements strict risk controls

### Custom Strategy Development

```rust
// Example custom strategy implementation
impl TradingStrategy {
    pub fn analyze_opportunity(&self, token: &TokenOpportunity) -> Option<TradeSignal> {
        // Your custom logic here
        if self.meets_criteria(token) {
            Some(TradeSignal::new(
                token.mint,
                TradeType::Buy,
                self.calculate_entry_amount(token),
                self.calculate_confidence(token)
            ))
        } else {
            None
        }
    }
}
```

## 🚨 Error Handling & Troubleshooting

### Common Issues

#### **Connection Errors**

```
Error: RPC connection failed
```

**Solution**: Check your Solana RPC endpoint configuration and network connectivity.

#### **Insufficient Balance**

```
Error: Insufficient SOL balance for transaction
```

**Solution**: Ensure your wallet has enough SOL for trading and transaction fees.

#### **API Rate Limits**

```
Warning: DexScreener API rate limit exceeded
```

**Solution**: The bot automatically handles rate limits with exponential backoff.

### Logging Configuration

```bash
# Set log level (debug, info, warn, error)
RUST_LOG=screenerbot=debug cargo run

# Log to file
cargo run 2>&1 | tee bot.log
```

## 🤝 Contributing

We welcome contributions! Please see our [Contributing Guidelines](CONTRIBUTING.md) for details.

### Development Workflow

1. **Fork** the repository
2. **Create** a feature branch
3. **Implement** your changes with tests
4. **Submit** a pull request

### Code Standards

- Follow Rust formatting (`cargo fmt`)
- Pass all lints (`cargo clippy`)
- Include comprehensive tests
- Document public APIs

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## ⚠️ Disclaimer

**IMPORTANT**: This software is for educational and research purposes only. Cryptocurrency trading involves substantial risk of loss. The authors are not responsible for any financial losses incurred through the use of this bot.

- **Use at your own risk**
- **Never invest more than you can afford to lose**
- **Thoroughly test with small amounts first**
- **Understand the risks involved in automated trading**

## 🙏 Acknowledgments

- **Solana Foundation** for the robust blockchain infrastructure
- **Jupiter Protocol** for excellent swap aggregation
- **Rust Community** for the amazing ecosystem
- **Contributors** who help improve this project

## 📞 Support

- **Issues**: [GitHub Issues](https://github.com/farfary/ScreenerBot/issues)
- **Discussions**: [GitHub Discussions](https://github.com/farfary/ScreenerBot/discussions)
- **Documentation**: [Wiki](https://github.com/farfary/ScreenerBot/wiki)

---

**Happy Trading! 🚀**

_Built with ❤️ for the Solana ecosystem_
