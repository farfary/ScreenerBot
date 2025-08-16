# ScreenerBot - Solana DeFi Trading Bot

A sophisticated Rust-based trading bot for Solana DeFi with AI-powered reinforcement learning, comprehensive transaction analysis, and multi-router swap execution.

## 🚀 Features

- **AI-Powered Trading**: Reinforcement learning for optimal entry/exit timing
- **Multi-Router Swaps**: Jupiter (primary) and Raydium integration
- **Advanced Analysis**: Comprehensive transaction and profit/loss tracking
- **Background Services**: 13+ automated monitoring and analysis services
- **ATA Management**: Automatic Account Token Address handling and rent optimization
- **Safety First**: Extensive dry-run capabilities and transaction verification
- **Comprehensive Debugging**: Rich debug tools for development and monitoring

## 🏗️ Architecture

### Core Systems

- **Transaction Management**: Priority verification, phantom position prevention
- **Token Discovery**: Real-time new token detection and monitoring
- **Profit Analysis**: Accurate PnL calculations excluding ATA rent and fees
- **AI Learning**: Reinforcement learning from trading data for strategy optimization
- **Background Services**: Automated portfolio monitoring and analysis

### Background Services (13+)

1. **Transaction Manager** - Wallet monitoring & verification
2. **Token Discovery** - New token detection via APIs
3. **Token Monitoring** - Price tracking for positions
4. **RPC Stats** - Performance monitoring
5. **ATA Cleanup** - Account management
6. **RL Learning** - AI model training from trading data
7. **RL Auto-save** - Persistent storage of learning data
8. **Profit Analysis** - Position performance calculation
9. **Entry Analysis** - Trade opportunity identification
10. **Summary** - Portfolio performance reporting
11. **Blacklist** - Token safety filtering
12. **OHLCV** - Price history collection
13. **Pool Monitoring** - On-chain price verification

## 🛠️ Setup & Installation

### Prerequisites

- Rust (latest stable)
- Solana CLI tools
- A Solana wallet with trading funds

### Installation

```bash
git clone <repository-url>
cd ScreenerBot
cargo build --release
```

### Configuration

1. Set up your wallet configuration in `data/configs.json`
2. Configure RPC endpoints and trading parameters
3. Review safety settings and enable dry-run mode for testing

## 🎯 Usage

### Quick Start

```bash
# Check wallet balance
cargo run --bin main_debug -- --check-balance

# Analyze recent swaps
cargo run --bin main_debug -- --analyze-swaps --count 20

# Test swap functionality (dry run)
cargo run --bin main_debug -- --test-swap --dry-run

# Monitor wallet in real-time
cargo run --bin main_debug -- --monitor --duration 300
```

### Analysis Commands

```bash
# Comprehensive swap analysis
cargo run --bin main_debug -- --analyze-swaps --count 50 --min-sol 0.003

# Position lifecycle analysis
cargo run --bin main_debug -- --analyze-positions

# ATA operations analysis
cargo run --bin main_debug -- --analyze-ata --count 100

# Fee breakdown analysis
cargo run --bin main_debug -- --analyze-fees --count 200

# Deep-dive specific transaction
cargo run --bin main_debug -- --signature TRANSACTION_SIGNATURE
```

### Trading Commands

```bash
# Safe testing (always use --dry-run first)
cargo run --bin main_debug -- --test-swap --dry-run
cargo run --bin main_debug -- --test-position --dry-run

# Live trading (remove --dry-run when ready)
cargo run --bin main_debug -- --test-swap --token-mint <MINT> --sol-amount 0.001
```

### Data Management

```bash
# Fetch new transactions
cargo run --bin main_debug -- --fetch-new --analyze

# Update cached analysis
cargo run --bin main_debug -- --update-cache --count 100

# Clean cache
cargo run --bin main_debug -- --clean-cache
```

## 🔍 Debug Tools

The main debug tool (`main_debug`) provides comprehensive analysis capabilities:

- **Transaction Analysis**: Deep-dive into individual transactions
- **Swap Analysis**: Comprehensive profit/loss calculations
- **ATA Analysis**: Account Token Address operations and costs
- **Position Tracking**: Entry/exit timing and performance
- **Fee Analysis**: Transaction cost breakdowns
- **Cache Management**: Efficient data storage and retrieval

For complete debug tool documentation, see [Debug Tools Guide](.github/instructions/debug-tools.instructions.md).

## 📊 Key Features

### ATA Rent Handling
- Automatic detection and exclusion of ATA rent from trading calculations
- Accurate profit/loss reporting that matches external tools
- Comprehensive ATA operations analysis

### Transaction Classification
- Accurate buy/sell detection for all DEX routers
- Support for Jupiter, Raydium, and other Solana DEXes
- Proper handling of complex transaction structures

### AI Integration
- Reinforcement learning model for entry/exit timing
- Continuous learning from trading performance
- Automatic model improvements based on historical data

### Safety Features
- Mandatory dry-run mode for development
- Transaction verification and phantom position prevention
- Comprehensive error handling and logging

## 📁 Project Structure

```
src/
├── bin/                 # Debug and utility tools
├── swaps/              # DEX integration modules
├── arguments.rs        # Centralized argument handling
├── configs.rs          # Configuration management
├── transactions.rs     # Transaction analysis core
├── rl_learning.rs      # AI/ML components
├── positions.rs        # Position management
└── main.rs            # Main trading bot

data/                   # Runtime data
├── positions.json      # Current positions
├── transactions/       # Transaction cache
├── configs.json        # Configuration
└── rl_learning_records.json  # AI training data

.github/instructions/   # Development guides
├── debug-tools.instructions.md
├── transactions-system.instructions.md
└── ...
```

## 🔒 Safety & Testing

⚠️ **This is a live financial trading system. Always prioritize safety:**

- Use `--dry-run` for all testing
- Start with small amounts
- Monitor logs continuously
- Verify transaction results
- Test thoroughly before live trading

## 📖 Documentation

- [Debug Tools Guide](.github/instructions/debug-tools.instructions.md) - Comprehensive debug tool usage
- [Transactions System](.github/instructions/transactions-system.instructions.md) - Transaction analysis architecture
- [Updates History](UPDATES.md) - Recent changes and improvements
- [AI Agent Guide](.github/Assistant-instructions.md) - Development guidelines

## 🤝 Contributing

1. Read the development guidelines in `.github/instructions/`
2. Follow the safety requirements
3. Test thoroughly with `--dry-run`
4. Update documentation for significant changes
5. Add entries to `UPDATES.md` for tracking

## 📄 License

[Add your license information here]

---

**⚠️ Disclaimer**: This software is for educational purposes. Trading cryptocurrencies involves substantial risk. Use at your own risk and never trade with more than you can afford to lose.
