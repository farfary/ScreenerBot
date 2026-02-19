# Debug Manual Trading Tool

A comprehensive debug tool for manual position management and testing trading operations.

## Overview

This tool initializes the full bot system (identical to `run.rs`) but without auto-trading or webserver components. It provides manual control over trading operations for testing, debugging, and verification purposes.

## Features

- ✅ **Full System Initialization** - All services (RPC, pools, tokens, positions, transactions, etc.)
- ✅ **Manual Position Management** - Open, close, inspect positions
- ✅ **DCA Support** - Execute additional buys on existing positions
- ✅ **Partial Exits** - Sell portions of positions
- ✅ **Position Reconciliation** - Verify chain state matches DB
- ✅ **Quote Testing** - Test swap quotes without executing
- ✅ **Dry-Run Mode** - Test operations without real transactions
- ✅ **Debug Logging** - Enable detailed logging per module

## Installation

The tool is built as part of the main project:

```bash
cargo build --bin debug_manual_trading
```

## Usage

### Open a Position

```bash
# Open position with config trade size
cargo run --bin debug_manual_trading -- open --mint <TOKEN_MINT>

# Open position with custom size
cargo run --bin debug_manual_trading -- open --mint <TOKEN_MINT> --size-sol 0.5

# Open position with strategy tracking
cargo run --bin debug_manual_trading -- open --mint <TOKEN_MINT> --strategy my_strategy
```

### Close a Position

```bash
# Close position (manual reason)
cargo run --bin debug_manual_trading -- close --mint <TOKEN_MINT>

# Close with specific reason
cargo run --bin debug_manual_trading -- close --mint <TOKEN_MINT> --reason stop_loss

# Force close (skip verification)
cargo run --bin debug_manual_trading -- close --mint <TOKEN_MINT> --force
```

### DCA (Dollar Cost Averaging)

```bash
# Execute DCA buy with 0.5 SOL
cargo run --bin debug_manual_trading -- dca --mint <TOKEN_MINT> --amount 0.5

# DCA with max limit check
cargo run --bin debug_manual_trading -- dca --mint <TOKEN_MINT> --amount 0.3 --max-dca 5
```

### Partial Exit

```bash
# Exit 50% of position
cargo run --bin debug_manual_trading -- partial-exit --mint <TOKEN_MINT> --percentage 50

# Exit with specific reason
cargo run --bin debug_manual_trading -- partial-exit --mint <TOKEN_MINT> --percentage 25 --reason take_profit
```

### List Positions

```bash
# List all open positions
cargo run --bin debug_manual_trading -- list

# Detailed view with full data
cargo run --bin debug_manual_trading -- list --detailed

# Filter by strategy
cargo run --bin debug_manual_trading -- list --strategy scalping
```

### Inspect Position

```bash
# Basic inspection
cargo run --bin debug_manual_trading -- inspect --mint <TOKEN_MINT>

# Show all details
cargo run --bin debug_manual_trading -- inspect --mint <TOKEN_MINT> \
  --show-transactions --show-dca --show-exits
```

### Reconcile Positions

```bash
# Reconcile all positions
cargo run --bin debug_manual_trading -- reconcile

# Reconcile specific position
cargo run --bin debug_manual_trading -- reconcile --mint <TOKEN_MINT>

# Auto-fix discrepancies
cargo run --bin debug_manual_trading -- reconcile --auto-fix
```

### Test Quote

```bash
# Test buy quote
cargo run --bin debug_manual_trading -- test-quote --mint <TOKEN_MINT> --amount 1.0 --operation buy

# Test sell quote
cargo run --bin debug_manual_trading -- test-quote --mint <TOKEN_MINT> --amount 0.5 --operation sell
```

### System Initialization Test

```bash
# Just initialize and wait (useful for testing service startup)
cargo run --bin debug_manual_trading -- init

# Wait for specific duration
cargo run --bin debug_manual_trading -- init --wait-seconds 60
```

## Global Options

These options work with any command:

### Dry-Run Mode

Test operations without executing real transactions:

```bash
cargo run --bin debug_manual_trading -- open --mint <TOKEN_MINT> --dry-run
```

### Debug Logging

Enable detailed logging for specific modules:

```bash
# Enable positions debug logging
cargo run --bin debug_manual_trading -- open --mint <TOKEN_MINT> --debug-positions

# Enable trader debug logging
cargo run --bin debug_manual_trading -- open --mint <TOKEN_MINT> --debug-trader

# Enable swaps debug logging
cargo run --bin debug_manual_trading -- open --mint <TOKEN_MINT> --debug-swaps

# Combine multiple debug flags
cargo run --bin debug_manual_trading -- open --mint <TOKEN_MINT> \
  --debug-positions --debug-trader --debug-swaps
```

### Wait for Services

Control whether to wait for services to be ready (default: true):

```bash
# Skip waiting (faster for quick tests)
cargo run --bin debug_manual_trading -- list --wait-ready=false
```

## Service Initialization

The tool initializes the following services (same as `run.rs`):

- ✅ Events Service
- ✅ Transactions Service
- ✅ SOL Price Service
- ✅ Pool Discovery Service
- ✅ Pool Fetcher Service
- ✅ Pool Calculator Service
- ✅ Pool Analyzer Service
- ✅ Pools Service (coordinator)
- ✅ Tokens Service (centralized)
- ✅ Filtering Service
- ✅ OHLCV Service
- ✅ Positions Service
- ✅ Wallet Service
- ✅ RPC Stats Service
- ✅ ATA Cleanup Service

**Not Included:**

- ❌ Trader Service (auto-trading)
- ❌ Webserver Service (not needed for CLI)

## Implementation Status

### ✅ Implemented

- Full system initialization
- Service registration and startup
- Command-line argument parsing
- Debug flag support
- Service readiness checking

### 🚧 To Be Implemented

All command handlers are currently placeholders and will be implemented to:

- [ ] `open` - Call `positions::open_position_direct()` with validation
- [ ] `close` - Call `positions::close_position_direct()` with verification
- [ ] `dca` - Execute additional buy with DCA tracking
- [ ] `partial-exit` - Execute partial sell with tracking
- [ ] `list` - Query and display all open positions
- [ ] `inspect` - Show detailed position data with history
- [ ] `interactive` - REPL-style interactive mode
- [ ] `reconcile` - Compare DB vs chain state
- [ ] `test-quote` - Get quotes from all DEXes without executing

## Configuration

The tool uses the same `data/config.toml` as the main bot. Key settings:

```toml
[trader]
trade_size_sol = 0.1           # Default trade size
max_open_positions = 5          # Position limit

[positions]
enable_dca = true               # Allow DCA operations
max_dca_count = 3               # Max DCA per position
enable_partial_exits = true     # Allow partial exits
```

## Logs

All operations are logged to:

- Console (colored output)
- `logs/screenerbot_YYYY-MM-DD_HH-MM-SS.log` (persistent)

## Examples

### Complete Testing Workflow

```bash
# 1. Initialize system and verify readiness
cargo run --bin debug_manual_trading -- init --wait-seconds 30

# 2. Open a test position
cargo run --bin debug_manual_trading -- open --mint <TOKEN_MINT> --size-sol 0.1 --debug-positions

# 3. List positions to verify
cargo run --bin debug_manual_trading -- list --detailed

# 4. Execute DCA
cargo run --bin debug_manual_trading -- dca --mint <TOKEN_MINT> --amount 0.05

# 5. Partial exit (take some profit)
cargo run --bin debug_manual_trading -- partial-exit --mint <TOKEN_MINT> --percentage 30

# 6. Inspect position state
cargo run --bin debug_manual_trading -- inspect --mint <TOKEN_MINT> --show-dca --show-exits

# 7. Close remaining position
cargo run --bin debug_manual_trading -- close --mint <TOKEN_MINT> --reason manual
```

### Dry-Run Testing

```bash
# Test full workflow without real transactions
cargo run --bin debug_manual_trading -- open --mint <TOKEN_MINT> --dry-run
cargo run --bin debug_manual_trading -- dca --mint <TOKEN_MINT> --amount 0.1 --dry-run
cargo run --bin debug_manual_trading -- close --mint <TOKEN_MINT> --dry-run
```

## Safety Features

- **Dry-Run Mode**: Test without real transactions
- **Service Validation**: Ensures all services are ready before operations
- **Position Guards**: Prevents duplicate positions
- **Verification**: All transactions are verified on-chain
- **Debug Logging**: Detailed logs for troubleshooting

## Related Tools

- `debug_positions_semaphore.rs` - Check position limits
- `debug_tokens.rs` - Inspect token data
- `debug_pool_decoders.rs` - Test pool decoding
- `debug_events.rs` - Query event logs

## Notes

- Tool requires active RPC connection
- System initialization takes ~15-20 seconds
- All operations respect config limits (max positions, trade size, etc.)
- Transactions are queued for verification automatically
- Position reconciliation runs independently

## Future Enhancements

- Interactive REPL mode with command history
- Batch operations (multiple positions)
- Position templates (predefined strategies)
- Performance benchmarking mode
- Simulation mode (paper trading)
