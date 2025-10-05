# Config System

**Clean, organized, zero-repetition configuration for ScreenerBot**

## 🎯 Overview

The new config system eliminates repetition and provides a single source of truth for all bot parameters.

### Before vs After

**Before (Old System):**
```rust
// Define in trader.rs
pub const MAX_OPEN_POSITIONS: usize = 2;

// Use in code
use crate::trader::MAX_OPEN_POSITIONS;
if positions.len() < MAX_OPEN_POSITIONS { ... }
```

**After (New System):**
```rust
// Define ONCE in schemas.rs
config_struct! {
    pub struct TraderConfig {
        max_open_positions: usize = 2,
    }
}

// Use ANYWHERE with one line
use screenerbot::config::with_config;
with_config(|cfg| {
    if positions.len() < cfg.trader.max_open_positions { ... }
});
```

## 📁 File Structure

```
src/config/
├── mod.rs       - Module documentation & exports
├── macros.rs    - config_struct! macro (zero repetition)
├── schemas.rs   - All config structs (SINGLE SOURCE)
└── utils.rs     - Loading & access utilities

data/
└── config.toml  - Single config file
```

## ✨ Features

- ✅ **Zero Repetition** - Define once, use everywhere
- ✅ **Type Safe** - Compile-time checking
- ✅ **Hot Reload** - Update without restart
- ✅ **Organized** - Grouped by module
- ✅ **One-Line Access** - Clean API
- ✅ **Embedded Defaults** - No fallback logic needed

## 🚀 Quick Start

### 1. Load at Startup

```rust
use screenerbot::config::load_config;

#[tokio::main]
async fn main() -> Result<(), String> {
    load_config()?;
    // Config ready!
    Ok(())
}
```

### 2. Read Values

```rust
use screenerbot::config::with_config;

// Single value
let max_pos = with_config(|cfg| cfg.trader.max_open_positions);

// Multiple values
with_config(|cfg| {
    println!("Max: {}", cfg.trader.max_open_positions);
    println!("Size: {}", cfg.trader.trade_size_sol);
});
```

### 3. Async Functions

```rust
use screenerbot::config::get_config_clone;

async fn process() {
    let cfg = get_config_clone();
    
    // Use across await points
    tokio::time::sleep(Duration::from_secs(1)).await;
    
    if value > cfg.filtering.min_liquidity_usd {
        // ...
    }
}
```

## 📝 Adding Parameters

**Only 1 step required:**

Edit `src/config/schemas.rs`:

```rust
config_struct! {
    pub struct TraderConfig {
        max_open_positions: usize = 2,
        new_param: bool = false,  // ← Add this
    }
}
```

**Optional:** Override in `data/config.toml`:

```toml
[trader]
new_param = true
```

**Use it:**

```rust
let value = with_config(|cfg| cfg.trader.new_param);
```

Done! No helper functions, no boilerplate.

## 🔄 Hot Reload

```rust
use screenerbot::config::reload_config;

// 1. Edit data/config.toml
// 2. Call reload:
reload_config()?;
// 3. New values active immediately!
```

## 📦 Configuration Sections

| Section | Purpose | Example |
|---------|---------|---------|
| `trader` | Trading parameters | `cfg.trader.max_open_positions` |
| `filtering` | Token filtering | `cfg.filtering.min_liquidity_usd` |
| `swaps` | Swap routers | `cfg.swaps.gmgn_enabled` |
| `tokens` | Token management | `cfg.tokens.max_tokens_per_api_call` |
| `positions` | Position tracking | `cfg.positions.position_open_cooldown_secs` |
| `sol_price` | Price service | `cfg.sol_price.price_refresh_interval_secs` |
| `summary` | Display settings | `cfg.summary.summary_display_interval_secs` |
| `events` | Event system | `cfg.events.batch_timeout_ms` |
| `webserver` | Web dashboard | `cfg.webserver.port` |
| `services` | Service manager | `cfg.services.trader.enabled` |
| `monitoring` | System monitoring | `cfg.monitoring.metrics_interval_secs` |
| `rpc` | RPC endpoints | `cfg.rpc.urls` |

## 🛠️ API Reference

### Loading

```rust
use screenerbot::config::{load_config, reload_config};

load_config()?;                              // Load from data/config.toml
load_config_from_path("path/to/config")?;    // Load from custom path
reload_config()?;                            // Reload current config
```

### Reading

```rust
use screenerbot::config::{with_config, get_config_clone};

// Read with closure (recommended)
let value = with_config(|cfg| cfg.trader.max_open_positions);

// Clone for async (when needed)
let cfg = get_config_clone();
```

### Saving

```rust
use screenerbot::config::save_config;

save_config(None)?;                    // Save to default path
save_config(Some("custom.toml"))?;     // Save to custom path
```

## 📚 Examples

Run the example:

```bash
cargo run --example config_example
```

See:
- `examples/config_example.rs` - Full usage examples
- `docs/config-system-migration.md` - Migration guide
- `data/config.toml` - Configuration template

## 🔧 Implementation Details

### The Macro

`config_struct!` generates:
- Struct with public fields
- Default implementation
- Serde serialization

```rust
config_struct! {
    pub struct TraderConfig {
        max_open_positions: usize = 2,
    }
}

// Expands to:
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TraderConfig {
    pub max_open_positions: usize,
}

impl Default for TraderConfig {
    fn default() -> Self {
        Self { max_open_positions: 2 }
    }
}
```

### Storage

- Global: `CONFIG` (OnceCell<RwLock<Config>>)
- Thread-safe: RwLock for concurrent reads
- Atomic: Hot reload replaces entire config

### TOML Parsing

- Uses `toml` crate
- Missing fields → defaults from schemas
- Invalid syntax → error on load
- Type mismatches → error on load

## 🎓 Best Practices

### DO ✅

```rust
// Read values with with_config
let value = with_config(|cfg| cfg.trader.max_open_positions);

// Clone for async functions
let cfg = get_config_clone();

// Group related reads
with_config(|cfg| {
    let max = cfg.trader.max_open_positions;
    let size = cfg.trader.trade_size_sol;
    // use max and size
});
```

### DON'T ❌

```rust
// Don't hold locks across await
with_config(|cfg| {
    some_async_fn().await; // ❌ DON'T
});

// Instead, clone first
let cfg = get_config_clone();
some_async_fn().await; // ✅ OK
```

## 🐛 Troubleshooting

### Config not found

```
⚠️ Config file 'data/config.toml' not found, using default values
```

**Solution:** Create `data/config.toml` or copy from example

### Parse error

```
Failed to parse config file: invalid type: string "abc", expected u64
```

**Solution:** Check types in TOML match schemas.rs

### Not initialized

```
Config not initialized. Call load_config() first.
```

**Solution:** Call `load_config()` at startup

## 📖 Further Reading

- [Migration Guide](../../docs/config-system-migration.md)
- [Example Code](../../examples/config_example.rs)
- [Schema Definitions](schemas.rs)

## 🤝 Contributing

When adding new parameters:

1. Add to appropriate struct in `schemas.rs`
2. Document with comment
3. Update `data/config.toml` template
4. Add example usage if complex

That's it!
