# Trader Module Reorganization

**Date:** November 17, 2025  
**Status:** Ready to Implement

---

## Current Structure Problems

**Mixed responsibilities:** `auto/entry_monitor.rs` (311 lines) and `auto/exit_monitor.rs` (497 lines) mix orchestration + evaluation + execution in single files.

**Confusing layout:** Exit strategies in `exit/` separate from `auto/exit_monitor.rs`. DCA split across 2 files.

**Inconsistent naming:** `entry_monitor.rs`, `strategy_manager.rs`, `decision_cache.rs` use different patterns.

**Code duplication:** Connectivity checks repeated in 4+ files. Constants scattered across multiple files.

---

## New Structure

```
trader/
├── mod.rs              # Module root with architecture docs
├── types.rs            # All trader types (unchanged)
├── constants.rs        # NEW: All constants consolidated
├── config.rs           # Config accessors (unchanged)
├── controller.rs       # Start/stop trader (unchanged)
├── service.rs          # Service implementation (updated imports)
│
├── monitors/           # NEW: Orchestration loops only
│   ├── mod.rs
│   ├── entry.rs        # Entry monitoring (~150 lines, extracted)
│   └── exit.rs         # Exit monitoring (~200 lines, extracted)
│
├── evaluators/         # NEW: Business logic
│   ├── mod.rs
│   ├── entry.rs        # Entry evaluation (extracted)
│   ├── exit.rs         # Exit coordinator (extracted)
│   ├── exit_roi.rs     # Moved from exit/roi.rs
│   ├── exit_trailing.rs # Moved from exit/trailing_stop.rs
│   ├── exit_time.rs    # Moved from exit/time_override.rs
│   ├── dca.rs          # Merged from auto/dca.rs + auto/dca_evaluation.rs
│   └── strategies.rs   # Renamed from auto/strategy_manager.rs
│
├── executors/          # RENAMED from execution/
│   ├── mod.rs
│   ├── core.rs         # NEW: Main execute_trade coordinator
│   ├── buy.rs          # Unchanged
│   ├── sell.rs         # Unchanged
│   ├── retry.rs        # Unchanged
│   └── cache.rs        # Renamed from decision_cache.rs
│
├── manual/             # Reorganized
│   ├── mod.rs
│   ├── api.rs          # NEW: Split from orders.rs
│   ├── force.rs        # NEW: Split from orders.rs
│   └── tracking.rs     # Unchanged
│
└── safety/             # Expanded
    ├── mod.rs          # Clean exports only
    ├── limits.rs       # Simplified
    ├── risk.rs         # Unchanged
    ├── blacklist.rs    # NEW: Extracted from mod.rs
    └── cooldown.rs     # NEW: Extracted from limits.rs
```

**Flow:** Monitor → Safety → Evaluator → Executor → Result

**DELETE:**

- `auto/` directory (all files moved)
- `exit/` directory (all files moved)
- `execution/` directory (renamed to executors/)
- `manual/orders.rs` (split into api.rs + force.rs)

---

## Implementation Steps

### 1. Create New Files

Create these new files with content from attached code blocks below:

**New constants file:**

- `src/trader/constants.rs`

**New monitor files:**

- `src/trader/monitors/mod.rs`
- `src/trader/monitors/entry.rs` (extract orchestration from `auto/entry_monitor.rs`)
- `src/trader/monitors/exit.rs` (extract orchestration from `auto/exit_monitor.rs`)

**New evaluator files:**

- `src/trader/evaluators/mod.rs`
- `src/trader/evaluators/entry.rs` (extract evaluation from `auto/entry_monitor.rs`)
- `src/trader/evaluators/exit.rs` (consolidate all exit checks)
- `src/trader/evaluators/dca.rs` (merge `auto/dca.rs` + `auto/dca_evaluation.rs`)
- `src/trader/evaluators/strategies.rs` (rename from `auto/strategy_manager.rs`)

**New executor file:**

- `src/trader/executors/core.rs`

**New manual files:**

- `src/trader/manual/api.rs` (split from `orders.rs`)
- `src/trader/manual/force.rs` (split from `orders.rs`)

**New safety files:**

- `src/trader/safety/blacklist.rs` (extract from `safety/mod.rs`)
- `src/trader/safety/cooldown.rs` (extract from `safety/limits.rs`)

---

### 2. Move Files

```bash
# Rename execution to executors
mv src/trader/execution src/trader/executors

# Move exit strategies to evaluators
mv src/trader/exit/roi.rs src/trader/evaluators/exit_roi.rs
mv src/trader/exit/trailing_stop.rs src/trader/evaluators/exit_trailing.rs
mv src/trader/exit/time_override.rs src/trader/evaluators/exit_time.rs

# Rename decision_cache to cache
mv src/trader/executors/decision_cache.rs src/trader/executors/cache.rs
```

---

### 3. Update Existing Files

**Update `src/trader/mod.rs`:**

- Add architecture documentation
- Change imports: `auto` → `monitors`, `execution` → `executors`
- Add `pub mod evaluators;`
- Export `constants::*`
- Remove old constant definitions

**Update `src/trader/service.rs`:**

- Change `use crate::trader::auto` → `use crate::trader::monitors`
- Update function calls

**Update `src/trader/safety/mod.rs`:**

- Remove all implementation code
- Add exports for new modules: `blacklist`, `cooldown`

**Update `src/trader/safety/limits.rs`:**

- Remove `is_in_reentry_cooldown` function (moved to `cooldown.rs`)

**Update `src/trader/executors/mod.rs`:**

- Add `mod core;`
- Change `decision_cache` → `cache`
- Export `execute_trade` from `core`

---

### 4. Update External References

Find and update imports:

```bash
rg "trader::(auto|execution|exit)" --type rust -l
```

Replace:

- `trader::auto::monitor_entries` → `trader::monitors::monitor_entries`
- `trader::auto::monitor_positions` → `trader::monitors::monitor_positions`
- `trader::auto::process_dca_opportunities` → `trader::evaluators::dca::evaluate_dca_for_position`
- `trader::auto::StrategyManager` → `trader::evaluators::StrategyEvaluator`
- `trader::execution::execute_trade` → `trader::executors::execute_trade`
- `trader::execution::*` → `trader::executors::*`
- `trader::exit::*` → `trader::evaluators::exit_*::*`

---

### 5. Delete Old Files

```bash
# Delete entire auto directory
rm -rf src/trader/auto/

# Delete old exit directory (now empty)
rm -rf src/trader/exit/

# Delete old orders file (split into api + force)
rm src/trader/manual/orders.rs
```

---

### 6. Verify

```bash
cargo fmt
cargo clippy
cargo build
cargo run --bin screenerbot -- --run --dry-run
```

---

## Code Templates

### `constants.rs`

```rust
//! Trader module constants

pub const ENTRY_MONITOR_INTERVAL_SECS: u64 = 3;
pub const POSITION_MONITOR_INTERVAL_SECS: u64 = 5;
pub const ENTRY_CYCLE_MIN_WAIT_MS: u64 = 100;
pub const POSITION_CYCLE_MIN_WAIT_MS: u64 = 200;
pub const ENTRY_CHECK_ACQUIRE_TIMEOUT_SECS: u64 = 30;
pub const MAX_CONCURRENT_POSITION_EVALUATIONS: usize = 5;
pub const MAX_RETRIES: u32 = 3;
pub const RETRY_DELAY_MS: u64 = 2000;
pub const DECISION_CACHE_RETRY_MINUTES: i64 = 5;
pub const DECISION_CACHE_MAX_RETRIES: u32 = 5;
pub const DECISION_CACHE_CLEANUP_HOURS: i64 = 24;
pub const ENTRY_RESERVATION_TIMEOUT_SECS: u64 = 30;
pub const DEBUG_FORCE_SELL_MODE: bool = false;
pub const DEBUG_FORCE_BUY_MODE: bool = false;
```

### `monitors/mod.rs`

```rust
//! Monitoring loops for automated trading

mod entry;
mod exit;

pub use entry::monitor_entries;
pub use exit::monitor_positions;

use std::time::Duration;
use tokio::time::Instant;

pub fn calculate_cycle_remaining(
    cycle_start: Instant,
    cycle_duration_ms: u64,
    min_wait_ms: u64,
) -> Duration {
    let elapsed = cycle_start.elapsed();
    let target = Duration::from_millis(cycle_duration_ms);
    let min_wait = Duration::from_millis(min_wait_ms);
    if elapsed >= target {
        min_wait
    } else {
        (target - elapsed).max(min_wait)
    }
}

pub async fn start_automated_trading(
    shutdown: tokio::sync::watch::Receiver<bool>,
) -> Result<(), String> {
    let entry_shutdown = shutdown.clone();
    let exit_shutdown = shutdown.clone();

    let entry_task = tokio::spawn(async move {
        if let Err(e) = monitor_entries(entry_shutdown).await {
            crate::logger::error(crate::logger::LogTag::Trader, &format!("Entry monitor error: {}", e));
        }
    });

    let exit_task = tokio::spawn(async move {
        if let Err(e) = monitor_positions(exit_shutdown).await {
            crate::logger::error(crate::logger::LogTag::Trader, &format!("Exit monitor error: {}", e));
        }
    });

    let _ = tokio::try_join!(entry_task, exit_task);
    Ok(())
}
```

### `evaluators/mod.rs`

```rust
//! Evaluation logic for trading decisions

pub mod dca;
pub mod entry;
pub mod exit;
pub mod exit_roi;
pub mod exit_time;
pub mod exit_trailing;
pub mod strategies;

pub use dca::{evaluate_dca_for_position, DcaEvaluation};
pub use entry::evaluate_entry_for_token;
pub use exit::evaluate_exit_for_position;
pub use strategies::StrategyEvaluator;

use std::time::Duration;

pub const STRATEGY_EVALUATION_TIMEOUT: Duration = Duration::from_secs(5);

pub async fn check_evaluation_connectivity(endpoints: &[&str]) -> Result<(), String> {
    if let Some(unhealthy) = crate::connectivity::check_endpoints_healthy(endpoints).await {
        return Err(format!("Unhealthy endpoints: {}", unhealthy));
    }
    Ok(())
}
```

### `evaluators/entry.rs`

```rust
//! Entry evaluation logic

use crate::pools::PriceResult;
use crate::trader::evaluators;
use crate::trader::types::TradeDecision;

pub async fn evaluate_entry_for_token(
    token_mint: &str,
    price_info: &PriceResult,
) -> Result<Option<TradeDecision>, String> {
    evaluators::check_evaluation_connectivity(&["rpc", "dexscreener", "rugcheck"]).await?;
    evaluators::StrategyEvaluator::check_entry_strategies(token_mint, price_info).await
}
```

### `evaluators/exit.rs`

```rust
//! Exit evaluation coordinator

use crate::pools;
use crate::positions::Position;
use crate::trader::evaluators;
use crate::trader::safety;
use crate::trader::types::TradeDecision;

pub async fn evaluate_exit_for_position(
    position: Position,
) -> Result<Option<TradeDecision>, String> {
    let current_price = match pools::get_pool_price(&position.mint) {
        Some(price_info) => price_info.price_sol,
        None => return Ok(None),
    };

    // Priority order: blacklist → risk → trailing → time → roi → strategy
    if let Some(decision) = safety::check_blacklist_exit(&position, current_price) {
        return Ok(Some(decision));
    }
    if let Some(decision) = safety::check_risk_limits(&position, current_price).await? {
        return Ok(Some(decision));
    }
    if let Some(decision) = evaluators::exit_trailing::check_trailing_stop(&position, current_price).await? {
        return Ok(Some(decision));
    }
    if let Some(decision) = evaluators::exit_time::check_time_override(&position, current_price).await? {
        return Ok(Some(decision));
    }
    if let Some(decision) = evaluators::exit_roi::check_roi_exit(&position, current_price).await? {
        return Ok(Some(decision));
    }
    if let Some(decision) = evaluators::StrategyEvaluator::check_exit_strategies(&position, current_price).await? {
        return Ok(Some(decision));
    }

    Ok(None)
}
```

### `executors/core.rs`

```rust
//! Core trade execution coordinator

use crate::trader::executors::{execute_buy, execute_dca, execute_sell};
use crate::trader::types::{TradeAction, TradeDecision, TradeResult};

pub async fn execute_trade(decision: &TradeDecision) -> Result<TradeResult, String> {
    match decision.action {
        TradeAction::Buy => execute_buy(decision).await,
        TradeAction::Sell => execute_sell(decision).await,
        TradeAction::DCA => execute_dca(decision).await,
    }
}
```

### `safety/blacklist.rs`

```rust
//! Blacklist-based safety checks

use crate::logger::{self, LogTag};
use crate::positions::Position;
use crate::trader::types::{TradeAction, TradeDecision, TradePriority, TradeReason};
use chrono::Utc;

pub fn is_blacklisted(mint: &str) -> bool {
    crate::tokens::get_blacklisted_tokens().contains(&mint.to_string())
}

pub fn check_blacklist_exit(position: &Position, current_price: f64) -> Option<TradeDecision> {
    if crate::tokens::get_blacklisted_tokens().contains(&position.mint) {
        logger::warning(
            LogTag::Trader,
            &format!("⛔ BLACKLISTED: {} - Emergency exit at {:.9} SOL", position.symbol, current_price),
        );
        return Some(TradeDecision {
            position_id: position.id.map(|id| id.to_string()),
            mint: position.mint.clone(),
            action: TradeAction::Sell,
            reason: TradeReason::Blacklisted,
            strategy_id: None,
            timestamp: Utc::now(),
            priority: TradePriority::Emergency,
            price_sol: Some(current_price),
            size_sol: None,
        });
    }
    None
}
```

### `safety/cooldown.rs`

```rust
//! Re-entry cooldown management

use crate::positions;
use crate::trader::config;
use chrono::Utc;

pub async fn is_in_reentry_cooldown(mint: &str) -> Result<bool, String> {
    let cooldown_minutes = config::get_position_close_cooldown_minutes();
    if cooldown_minutes == 0 {
        return Ok(false);
    }

    if let Ok(Some(position)) = positions::db::get_position_by_mint(mint).await {
        if let Some(exit_time) = position.exit_time {
            let elapsed = Utc::now().signed_duration_since(exit_time).num_minutes();
            if elapsed < cooldown_minutes as i64 {
                return Ok(true);
            }
        }
    }

    Ok(false)
}
```

### Updated `mod.rs`

````rust
//! Trader module - Core trading functionality orchestration
//!
//! ## Architecture
//!
//! ```text
//! Monitor → Safety → Evaluator → Executor → Result
//! ```
//!
//! **Monitors:** Orchestration loops (entry/exit)
//! **Safety:** Guards (limits, blacklist, cooldown, risk)
//! **Evaluators:** Business logic (strategies, exit conditions, DCA)
//! **Executors:** Trade execution (buy/sell/DCA, retry)

mod config;
mod constants;
mod controller;
pub mod evaluators;
pub mod executors;
pub mod manual;
pub mod monitors;
pub mod safety;
mod service;
mod types;

pub use config::*;
pub use constants::*;
pub use controller::{is_trader_running, start_trader, stop_trader_gracefully, TraderControlError};
pub use executors::execute_trade;
pub use service::TraderService;
pub use types::{TradeAction, TradeDecision, TradePriority, TradeReason, TradeResult};

use crate::logger::{self, LogTag};

pub async fn init_trader_system() -> Result<(), String> {
    logger::info(LogTag::Trader, "Initializing trader system...");
    executors::init_execution_system().await?;
    safety::init_safety_system().await?;
    logger::info(LogTag::Trader, "Trader system initialized");
    Ok(())
}
````

### Updated `service.rs`

```rust
//! Trader service implementation

use crate::trader::monitors;
// ... rest unchanged, just update import and function call:

async fn start(...) -> Result<Vec<JoinHandle<()>>, String> {
    // ...
    let handle = tokio::spawn(monitor.instrument(async move {
        if let Err(e) = monitors::start_automated_trading(watch_rx).await {
            logger::error(LogTag::Trader, &format!("Auto trading error: {}", e));
        }
    }));
    // ...
}
```

### Updated `safety/mod.rs`

```rust
//! Safety systems for trading

mod blacklist;
mod cooldown;
mod limits;
mod risk;

pub use blacklist::{check_blacklist_exit, is_blacklisted};
pub use cooldown::is_in_reentry_cooldown;
pub use limits::{check_position_limits, has_open_position};
pub use risk::check_risk_limits;

use crate::logger::{self, LogTag};

pub async fn init_safety_system() -> Result<(), String> {
    logger::info(LogTag::Trader, "Initializing safety system...");
    logger::info(LogTag::Trader, "Safety system initialized");
    Ok(())
}
```

### Updated `executors/mod.rs`

```rust
//! Trade execution system

mod buy;
mod cache;
mod core;
mod retry;
mod sell;

pub use buy::{execute_buy, execute_dca};
pub use cache::{cache_sell_decision, get_pending_sell_decisions, mark_sell_complete};
pub use core::execute_trade;
pub use retry::retry_trade;
pub use sell::execute_sell;

use crate::logger::{self, LogTag};

pub async fn init_execution_system() -> Result<(), String> {
    logger::info(LogTag::Trader, "Initializing execution system...");
    cache::init_cache()?;
    logger::info(LogTag::Trader, "Execution system initialized");
    Ok(())
}
```

### `manual/api.rs`

```rust
//! Public manual trading API

use crate::trader::executors::execute_trade;
use crate::trader::types::{TradeAction, TradeDecision, TradePriority, TradeReason, TradeResult};
use chrono::Utc;

pub async fn manual_buy(mint: &str, size_sol: f64) -> Result<TradeResult, String> {
    let decision = TradeDecision {
        position_id: None,
        mint: mint.to_string(),
        action: TradeAction::Buy,
        reason: TradeReason::ManualEntry,
        strategy_id: None,
        timestamp: Utc::now(),
        priority: TradePriority::High,
        price_sol: None,
        size_sol: Some(size_sol),
    };

    let result = execute_trade(&decision).await?;
    super::tracking::record_manual_trade(&result).await?;
    Ok(result)
}

pub async fn manual_sell(position_id: &str) -> Result<TradeResult, String> {
    let decision = TradeDecision {
        position_id: Some(position_id.to_string()),
        mint: "STUB_MINT".to_string(),
        action: TradeAction::Sell,
        reason: TradeReason::ManualExit,
        strategy_id: None,
        timestamp: Utc::now(),
        priority: TradePriority::High,
        price_sol: None,
        size_sol: None,
    };

    let result = execute_trade(&decision).await?;
    super::tracking::record_manual_trade(&result).await?;
    Ok(result)
}
```

### `manual/force.rs`

```rust
//! Force operations (bypass safety checks)

use crate::trader::executors::execute_trade;
use crate::trader::types::{TradeAction, TradeDecision, TradePriority, TradeReason, TradeResult};
use chrono::Utc;

pub async fn force_buy(mint: &str, size_sol: f64) -> Result<TradeResult, String> {
    let decision = TradeDecision {
        position_id: None,
        mint: mint.to_string(),
        action: TradeAction::Buy,
        reason: TradeReason::ForceBuy,
        strategy_id: None,
        timestamp: Utc::now(),
        priority: TradePriority::High,
        price_sol: None,
        size_sol: Some(size_sol),
    };

    let result = execute_trade(&decision).await?;
    super::tracking::record_manual_trade(&result).await?;
    Ok(result)
}

pub async fn force_sell(position_id: &str) -> Result<TradeResult, String> {
    let decision = TradeDecision {
        position_id: Some(position_id.to_string()),
        mint: "STUB_MINT".to_string(),
        action: TradeAction::Sell,
        reason: TradeReason::ForceSell,
        strategy_id: None,
        timestamp: Utc::now(),
        priority: TradePriority::Emergency,
        price_sol: None,
        size_sol: None,
    };

    let result = execute_trade(&decision).await?;
    super::tracking::record_manual_trade(&result).await?;
    Ok(result)
}
```

### `manual/mod.rs`

```rust
//! Manual trading operations

mod api;
mod force;
mod tracking;

pub use api::{manual_buy, manual_sell};
pub use force::{force_buy, force_sell};
pub use tracking::{get_manual_trade_history, record_manual_trade};
```

---

## Notes

**For `monitors/entry.rs` and `monitors/exit.rs`:** Extract orchestration logic from `auto/entry_monitor.rs` and `auto/exit_monitor.rs`, keeping only loop management, concurrency control, and calling evaluators/executors. Remove all inline evaluation logic.

**For `evaluators/dca.rs`:** Merge `auto/dca.rs` + `auto/dca_evaluation.rs` into single file, keeping the `DcaEvaluation` structure.

**For `evaluators/strategies.rs`:** Rename `StrategyManager` struct to `StrategyEvaluator`, move from `auto/strategy_manager.rs`.

**For exit strategies:** Move `exit/*.rs` to `evaluators/exit_*.rs` with content unchanged.

**External imports:** Find with `rg "trader::(auto|execution|exit)" --type rust -l` and update all references.
