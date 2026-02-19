# Trader Module Reorganization - Deep Review & Recommendations

**Date:** November 17, 2025  
**Status:** Ready for Implementation with Critical Fixes

---

## Executive Summary

The proposed reorganization is **architecturally sound** and addresses major structural issues. However, **critical fixes are required** before implementation to ensure:

1. No duplicate code remains
2. All safety checks are properly integrated
3. Constants are complete and accurate
4. Flow is systematic and clear

**Verdict:** APPROVE with mandatory fixes outlined below.

---

## Current Structure Analysis

### Confirmed Problems

#### 1. Mixed Responsibilities ✅ CORRECTLY IDENTIFIED

- **`auto/entry_monitor.rs` (311 lines):**
  - Orchestration: Loop, concurrency control, semaphore management
  - Inline evaluation: Strategy checks, safety validation
  - Execution calls: Direct `execute_trade()` invocations
  - **Issue:** Too many responsibilities in single file

- **`auto/exit_monitor.rs` (497 lines):**
  - Orchestration: Loop, concurrent task spawning, priority sorting
  - Inline evaluation: All exit checks (blacklist, trailing, ROI, time, strategy)
  - DCA processing: Separate `process_dca_opportunities()` call
  - Execution: Direct trade execution
  - **Issue:** Even worse - 497 lines doing everything

#### 2. Duplicate Logic ⚠️ CRITICAL ISSUE

**Blacklist Implementation:**

- **Location 1:** `safety/blacklist.rs` (111 lines)
  - Has cache system with `OnceLock<RwLock<HashSet>>`
  - Async `is_blacklisted()` function
  - Async `check_blacklist_exit()` function
  - Cache update mechanism

- **Location 2:** `safety/mod.rs` (62 lines)
  - Sync `is_blacklisted()` function
  - Sync `check_blacklist_exit()` function
  - Direct call to `tokens::get_blacklisted_tokens()`

**Problem:** TWO IMPLEMENTATIONS with different signatures (sync vs async). Current code uses sync version from mod.rs. Cached version in blacklist.rs is UNUSED and WRONG (tokens module is single source of truth).

**Constants Duplication:**

```rust
// In auto/entry_monitor.rs line 45
const ENTRY_MONITOR_INTERVAL_SECS: u64 = 3;

// In mod.rs line 25
pub const ENTRY_MONITOR_INTERVAL_SECS: u64 = 3;
```

#### 3. Confusing Layout ✅ CORRECTLY IDENTIFIED

- Exit strategies in `exit/` directory
- Exit monitor applying them in `auto/exit_monitor.rs`
- DCA split across `auto/dca.rs` + `auto/dca_evaluation.rs`
- No clear flow diagram

#### 4. Missing Integration ⚠️ NEWLY DISCOVERED

**Risk Check Not Used:**

- `safety/risk.rs` exists with `check_risk_limits()` function
- Exported in `safety/mod.rs`
- **BUT:** Never called in `auto/exit_monitor.rs`
- Function checks for >90% loss and triggers emergency exit
- **Issue:** Critical safety feature not integrated into exit flow

---

## Proposed Structure Review

### What's Excellent ✅

1. **Clear Architecture:** Monitor → Safety → Evaluator → Executor
2. **Single Responsibility:** Each module does ONE thing
3. **Consistent Naming:** monitors/, evaluators/, executors/, safety/
4. **Constants Consolidation:** All in `constants.rs`
5. **Manual Split:** api.rs (normal) + force.rs (bypass safety)

### Critical Issues Requiring Fixes 🔴

#### 1. Blacklist Duplication Not Resolved

**Plan says:**

> Create `safety/blacklist.rs` (extract from mod.rs)

**Reality:**

- `safety/blacklist.rs` ALREADY EXISTS with cache system
- `safety/mod.rs` ALSO has blacklist functions (no cache)
- Plan would create THIRD implementation
- Current code uses sync version from mod.rs

**REQUIRED FIX:**

```rust
// DELETE from safety/mod.rs:
pub fn is_blacklisted(mint: &str) -> bool { ... }
pub fn check_blacklist_exit(...) -> Option<TradeDecision> { ... }

// DELETE safety/blacklist.rs entirely (cached version is wrong pattern)

// CREATE NEW safety/blacklist.rs with SYNC functions:
pub fn is_blacklisted(mint: &str) -> bool {
    crate::tokens::get_blacklisted_tokens().contains(&mint.to_string())
}

pub fn check_blacklist_exit(position: &Position, current_price: f64) -> Option<TradeDecision> {
    if crate::tokens::get_blacklisted_tokens().contains(&position.mint) {
        // Return emergency exit decision
    }
    None
}
```

**Rationale:** Tokens module is single source of truth. No cache needed. Direct sync call is faster and simpler.

#### 2. Risk Check Missing from Exit Flow

**Current code:** exit_monitor.rs checks blacklist → trailing → roi → time → strategy  
**Plan template:** blacklist → risk → trailing → time → roi → strategy

**ISSUE:** Plan adds risk check but current code doesn't have it!

**Investigation:**

- `safety/risk.rs` exists with `check_risk_limits()` (90% loss → emergency exit)
- Function is exported but NEVER called
- Grep search confirms: only 2 references (definition + export)

**REQUIRED FIX:**

```rust
// In evaluators/exit.rs:
pub async fn evaluate_exit_for_position(position: Position) -> Result<Option<TradeDecision>, String> {
    let current_price = ...;

    // Priority 1: Blacklist (emergency - sync)
    if let Some(decision) = safety::check_blacklist_exit(&position, current_price) {
        return Ok(Some(decision));
    }

    // Priority 2: Risk limits (>90% loss - emergency)
    if let Some(decision) = safety::check_risk_limits(&position, current_price).await? {
        return Ok(Some(decision));
    }

    // Priority 3: Trailing stop (high priority)
    if let Some(decision) = evaluators::exit_trailing::check_trailing_stop(&position, current_price).await? {
        return Ok(Some(decision));
    }

    // Priority 4: ROI target
    // Priority 5: Time override
    // Priority 6: Strategy exit
}
```

**Match Current Order:** blacklist → trailing → ROI → time → strategy  
**Add Risk After Blacklist:** blacklist → risk → trailing → ROI → time → strategy

#### 3. Entry Evaluator Too Thin

**Plan template (14 lines):**

```rust
pub async fn evaluate_entry_for_token(
    token_mint: &str,
    price_info: &PriceResult,
) -> Result<Option<TradeDecision>, String> {
    evaluators::check_evaluation_connectivity(&["rpc", "dexscreener", "rugcheck"]).await?;
    evaluators::StrategyEvaluator::check_entry_strategies(token_mint, price_info).await
}
```

**Missing from template:**

- Position limits check
- Existing position check
- Re-entry cooldown check
- Blacklist check
- Token reservation management

**Current entry_monitor.rs has:**

```rust
// Line 100: check_position_limits()
// Line 110: has_open_position()
// Line 115: is_in_reentry_cooldown()
// Line 121: try_reserve_token_for_cycle()
// Line 138: is_blacklisted()
```

**REQUIRED FIX:**

```rust
// evaluators/entry.rs should include ALL safety checks:
pub async fn evaluate_entry_for_token(
    token_mint: &str,
    price_info: &PriceResult,
) -> Result<Option<TradeDecision>, String> {
    // Connectivity check
    if let Some(unhealthy) = crate::connectivity::check_endpoints_healthy(&["rpc", "dexscreener", "rugcheck"]).await {
        return Err(format!("Unhealthy endpoints: {}", unhealthy));
    }

    // Position limits
    if !safety::check_position_limits().await? {
        return Ok(None);
    }

    // Existing position
    if safety::has_open_position(token_mint).await? {
        return Ok(None);
    }

    // Re-entry cooldown
    if safety::is_in_reentry_cooldown(token_mint).await? {
        return Ok(None);
    }

    // Blacklist
    if safety::is_blacklisted(token_mint) {
        return Ok(None);
    }

    // Strategy evaluation
    evaluators::StrategyEvaluator::check_entry_strategies(token_mint, price_info).await
}
```

**Note:** Token reservation stays in monitor (orchestration concern, not evaluation).

#### 4. Constants Incomplete

**Plan lists:**

```rust
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

**Actual constants found:**

- `ENTRY_MONITOR_INTERVAL_SECS` ✅ (entry_monitor.rs:45)
- `POSITION_MONITOR_INTERVAL_SECS` ✅ (exit_monitor.rs:19)
- `ENTRY_CYCLE_MIN_WAIT_MS` ✅ (entry_monitor.rs:46)
- `POSITION_CYCLE_MIN_WAIT_MS` ✅ (exit_monitor.rs:20)
- `ENTRY_CHECK_ACQUIRE_TIMEOUT_SECS` ✅ (entry_monitor.rs:47)
- `MAX_RETRIES` ✅ (execution/retry.rs:8)
- `RETRY_DELAY_MS` ✅ (execution/retry.rs:9)
- `DEBUG_FORCE_SELL_MODE` ✅ (mod.rs:26)
- `DEBUG_FORCE_BUY_MODE` ✅ (mod.rs:27)

**Derived from code:**

- `MAX_CONCURRENT_POSITION_EVALUATIONS: usize = 5` ✅ (exit_monitor.rs:86 `Semaphore::new(5)`)
- `DECISION_CACHE_RETRY_MINUTES: i64 = 5` ✅ (decision_cache.rs:51 `Duration::minutes(5)`)
- `DECISION_CACHE_MAX_RETRIES: u32 = 5` ✅ (decision_cache.rs:56 `retry_count < 5`)
- `DECISION_CACHE_CLEANUP_HOURS: i64 = 24` ✅ (decision_cache.rs:81 `Duration::hours(24)`)
- `ENTRY_RESERVATION_TIMEOUT_SECS: u64 = 30` ✅ (entry_monitor.rs:28 `Duration::from_secs(30)`)

**REQUIRED FIX:** Constants list is actually COMPLETE. ✅

#### 5. Unnecessary Wrapper

**Plan creates:** `executors/core.rs` with `execute_trade()` function

**Current:** `execution/mod.rs` already has perfect implementation:

```rust
pub async fn execute_trade(decision: &TradeDecision) -> Result<TradeResult, String> {
    match decision.action {
        TradeAction::Buy => buy::execute_buy(decision).await,
        TradeAction::Sell => sell::execute_sell(decision).await,
        TradeAction::DCA => buy::execute_dca(decision).await,
    }
}
```

**ISSUE:** Creating `executors/core.rs` adds unnecessary layer:

- Rename execution → executors ✅
- Keep execute_trade in mod.rs ✅
- Don't create core.rs ❌

**REQUIRED FIX:** Remove `executors/core.rs` from plan. Keep execute_trade in `executors/mod.rs`.

#### 6. Service Integration Missing

**Plan mentions:**

> Updated `service.rs` imports

**But doesn't show:**

- Change `use crate::trader::auto;` → `use crate::trader::monitors;`
- Change `auto::start_auto_trading()` → `monitors::start_automated_trading()`

**REQUIRED FIX:**

```rust
// src/trader/service.rs line 6
use crate::trader::monitors;  // Changed from auto

// src/trader/service.rs line 128
let handle = tokio::spawn(monitor.instrument(async move {
    if let Err(e) = monitors::start_automated_trading(watch_rx).await {  // Changed from auto::start_auto_trading
        logger::error(LogTag::Trader, &format!("Auto trading error: {}", e));
    }
}));
```

---

## Systematic Implementation Plan

### Phase 1: Create New Directory Structure

```bash
mkdir -p src/trader/monitors
mkdir -p src/trader/evaluators
```

### Phase 2: Create New Files (with corrections)

#### A. Constants

```bash
# Create constants.rs with ALL constants consolidated
touch src/trader/constants.rs
```

#### B. Monitors

```bash
# Create monitor orchestration files
touch src/trader/monitors/mod.rs
touch src/trader/monitors/entry.rs  # Extract from auto/entry_monitor.rs
touch src/trader/monitors/exit.rs   # Extract from auto/exit_monitor.rs
```

**Note:** Monitors should ONLY contain:

- Loop management
- Concurrency control (semaphores)
- Token reservation (entry only)
- Shutdown handling
- Calling evaluators
- Calling executors
- Priority sorting (exit only)

#### C. Evaluators

```bash
# Create evaluation logic files
touch src/trader/evaluators/mod.rs
touch src/trader/evaluators/entry.rs      # ENHANCED version with all safety checks
touch src/trader/evaluators/exit.rs       # Coordinator with risk check added
touch src/trader/evaluators/dca.rs        # Merge auto/dca.rs + auto/dca_evaluation.rs
touch src/trader/evaluators/strategies.rs # Rename from auto/strategy_manager.rs
```

**Note:** Exit strategies will be moved here from exit/ directory.

#### D. Safety Modules

```bash
# Create new safety modules
touch src/trader/safety/cooldown.rs   # Extract from limits.rs
# DELETE old blacklist.rs (cached version)
# CREATE NEW blacklist.rs (sync version)
touch src/trader/safety/blacklist.rs  # SYNC implementation using tokens module
```

#### E. Manual Trading Split

```bash
# Split manual/orders.rs into api.rs + force.rs
touch src/trader/manual/api.rs    # manual_buy, manual_sell
touch src/trader/manual/force.rs  # force_buy, force_sell
```

### Phase 3: Move Existing Files

```bash
# Rename execution → executors
mv src/trader/execution src/trader/executors

# Move exit strategies to evaluators
mv src/trader/exit/roi.rs src/trader/evaluators/exit_roi.rs
mv src/trader/exit/trailing_stop.rs src/trader/evaluators/exit_trailing.rs
mv src/trader/exit/time_override.rs src/trader/evaluators/exit_time.rs

# Rename decision_cache → cache
mv src/trader/executors/decision_cache.rs src/trader/executors/cache.rs
```

### Phase 4: Update Existing Files

#### A. mod.rs

```rust
// Add architecture documentation
// Change imports: auto → monitors
// Add: pub mod evaluators;
// Add: pub mod constants;
// Export: pub use constants::*;
// Remove: old constant definitions (move to constants.rs)
```

#### B. service.rs

```rust
// Change: use crate::trader::auto; → use crate::trader::monitors;
// Change: auto::start_auto_trading() → monitors::start_automated_trading()
```

#### C. safety/mod.rs

```rust
// DELETE all implementation code
// Keep ONLY exports:
pub use blacklist::{check_blacklist_exit, is_blacklisted};
pub use cooldown::is_in_reentry_cooldown;
pub use limits::{check_position_limits, has_open_position};
pub use risk::check_risk_limits;

// Update init_safety_system if needed
```

#### D. safety/limits.rs

```rust
// Remove is_in_reentry_cooldown function (moved to cooldown.rs)
// Keep: check_position_limits, has_open_position
```

#### E. executors/mod.rs

```rust
// Change: decision_cache → cache
// Add exports for all public functions
// Keep: execute_trade function here (don't create core.rs)
```

### Phase 5: Update External References

```bash
# Find all imports of old structure
rg "trader::(auto|execution|exit)" --type rust -l

# Update each file:
# - trader::auto::monitor_entries → trader::monitors::monitor_entries
# - trader::auto::monitor_positions → trader::monitors::monitor_positions
# - trader::auto::process_dca_opportunities → trader::evaluators::dca::evaluate_dca_for_position
# - trader::auto::StrategyManager → trader::evaluators::StrategyEvaluator
# - trader::execution::* → trader::executors::*
# - trader::exit::* → trader::evaluators::exit_*::*
```

**Files to update:**

- src/trader/service.rs (change auto → monitors)
- src/webserver/routes/trading.rs (constants)
- src/webserver/routes/dashboard.rs (constants)
- src/webserver/routes/trader.rs (manual operations)

### Phase 6: Delete Old Directories

```bash
# COMPLETE DELETION - no compatibility, no legacy
rm -rf src/trader/auto/
rm -rf src/trader/exit/
rm src/trader/manual/orders.rs  # Replaced by api.rs + force.rs
```

### Phase 7: Verification

```bash
# Format code
cargo fmt

# Check for issues
cargo clippy

# Build
cargo build

# Test run
pkill -f screenerbot
cargo run --bin screenerbot -- --run --dry-run
```

### Phase 8: Documentation

```bash
# Update FLOW.md with new trader architecture
# Add trader flow diagram showing: Monitor → Safety → Evaluator → Executor
```

---

## Corrected Code Templates

### constants.rs (Complete)

```rust
//! Trader module constants

// Monitor intervals
pub const ENTRY_MONITOR_INTERVAL_SECS: u64 = 3;
pub const POSITION_MONITOR_INTERVAL_SECS: u64 = 5;

// Cycle timing
pub const ENTRY_CYCLE_MIN_WAIT_MS: u64 = 100;
pub const POSITION_CYCLE_MIN_WAIT_MS: u64 = 200;

// Timeouts and limits
pub const ENTRY_CHECK_ACQUIRE_TIMEOUT_SECS: u64 = 30;
pub const ENTRY_RESERVATION_TIMEOUT_SECS: u64 = 30;
pub const MAX_CONCURRENT_POSITION_EVALUATIONS: usize = 5;

// Retry configuration
pub const MAX_RETRIES: u32 = 3;
pub const RETRY_DELAY_MS: u64 = 2000;

// Decision cache configuration
pub const DECISION_CACHE_RETRY_MINUTES: i64 = 5;
pub const DECISION_CACHE_MAX_RETRIES: u32 = 5;
pub const DECISION_CACHE_CLEANUP_HOURS: i64 = 24;

// Debug flags
pub const DEBUG_FORCE_SELL_MODE: bool = false;
pub const DEBUG_FORCE_BUY_MODE: bool = false;
```

### safety/blacklist.rs (SYNC - No Cache)

```rust
//! Blacklist-based safety checks
//!
//! Uses tokens module as single source of truth.
//! No caching - direct synchronous calls for simplicity and correctness.

use crate::logger::{self, LogTag};
use crate::positions::Position;
use crate::trader::types::{TradeAction, TradeDecision, TradePriority, TradeReason};
use chrono::Utc;

/// Check if a token is blacklisted (sync - direct read from tokens module)
pub fn is_blacklisted(mint: &str) -> bool {
    crate::tokens::get_blacklisted_tokens().contains(&mint.to_string())
}

/// Check if a position should be exited due to blacklist (sync)
///
/// Returns an immediate exit decision if the position's token is blacklisted.
/// This is a critical safety check that overrides all other exit conditions.
///
/// Uses tokens module as single source of truth (no cache layer).
pub fn check_blacklist_exit(position: &Position, current_price: f64) -> Option<TradeDecision> {
    if is_blacklisted(&position.mint) {
        logger::warning(
            LogTag::Trader,
            &format!(
                "⛔ BLACKLISTED: {} (mint={}) - Emergency exit at {:.9} SOL",
                position.symbol, position.mint, current_price
            ),
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

### evaluators/entry.rs (ENHANCED)

```rust
//! Entry evaluation logic with integrated safety checks

use crate::pools::PriceResult;
use crate::trader::{evaluators, safety};
use crate::trader::types::TradeDecision;

/// Evaluate entry opportunity for a token
///
/// Performs all safety checks before strategy evaluation:
/// 1. Connectivity check
/// 2. Position limits
/// 3. Existing position check
/// 4. Re-entry cooldown
/// 5. Blacklist check
/// 6. Strategy evaluation
pub async fn evaluate_entry_for_token(
    token_mint: &str,
    price_info: &PriceResult,
) -> Result<Option<TradeDecision>, String> {
    // 1. Connectivity check
    if let Some(unhealthy) = crate::connectivity::check_endpoints_healthy(&["rpc", "dexscreener", "rugcheck"]).await {
        return Err(format!("Unhealthy endpoints: {}", unhealthy));
    }

    // 2. Position limits
    if !safety::check_position_limits().await? {
        return Ok(None);
    }

    // 3. Existing position check
    if safety::has_open_position(token_mint).await? {
        return Ok(None);
    }

    // 4. Re-entry cooldown
    if safety::is_in_reentry_cooldown(token_mint).await? {
        return Ok(None);
    }

    // 5. Blacklist check (sync)
    if safety::is_blacklisted(token_mint) {
        return Ok(None);
    }

    // 6. Strategy evaluation
    evaluators::StrategyEvaluator::check_entry_strategies(token_mint, price_info).await
}
```

### evaluators/exit.rs (WITH RISK CHECK)

```rust
//! Exit evaluation coordinator with priority-based checks

use crate::pools;
use crate::positions::Position;
use crate::trader::{evaluators, safety};
use crate::trader::types::TradeDecision;

/// Evaluate exit opportunity for a position
///
/// Priority order (matching current implementation + risk check):
/// 1. Blacklist (emergency - sync)
/// 2. Risk limits (>90% loss - emergency)
/// 3. Trailing stop (high priority)
/// 4. ROI target (normal priority)
/// 5. Time override (normal priority)
/// 6. Strategy exit (normal priority)
pub async fn evaluate_exit_for_position(
    position: Position,
) -> Result<Option<TradeDecision>, String> {
    // Get current price
    let current_price = match pools::get_pool_price(&position.mint) {
        Some(price_info) => {
            if price_info.price_sol > 0.0 && price_info.price_sol.is_finite() {
                price_info.price_sol
            } else {
                return Ok(None);
            }
        }
        None => return Ok(None),
    };

    // Priority 1: Blacklist (emergency - sync check)
    if let Some(decision) = safety::check_blacklist_exit(&position, current_price) {
        return Ok(Some(decision));
    }

    // Priority 2: Risk limits (>90% loss - emergency)
    if let Some(decision) = safety::check_risk_limits(&position, current_price).await? {
        return Ok(Some(decision));
    }

    // Priority 3: Trailing stop (high priority)
    if let Some(decision) = evaluators::exit_trailing::check_trailing_stop(&position, current_price).await? {
        return Ok(Some(decision));
    }

    // Priority 4: ROI target (normal priority)
    if let Some(decision) = evaluators::exit_roi::check_roi_exit(&position, current_price).await? {
        return Ok(Some(decision));
    }

    // Priority 5: Time override (normal priority)
    if let Some(decision) = evaluators::exit_time::check_time_override(&position, current_price).await? {
        return Ok(Some(decision));
    }

    // Priority 6: Strategy exit (normal priority)
    if let Some(decision) = evaluators::StrategyEvaluator::check_exit_strategies(&position, current_price).await? {
        return Ok(Some(decision));
    }

    // No exit signals
    Ok(None)
}
```

### executors/mod.rs (NO core.rs)

```rust
//! Trade execution system

mod buy;
mod cache;  // Renamed from decision_cache
mod retry;
mod sell;

pub use buy::{execute_buy, execute_dca};
pub use cache::{cache_sell_decision, get_pending_sell_decisions, mark_sell_complete};
pub use retry::retry_trade;
pub use sell::execute_sell;

use crate::logger::{self, LogTag};
use crate::trader::types::{TradeAction, TradeDecision, TradeResult};

/// Initialize the execution system
pub async fn init_execution_system() -> Result<(), String> {
    logger::info(LogTag::Trader, "Initializing execution system...");
    cache::init_cache()?;
    logger::info(LogTag::Trader, "Execution system initialized");
    Ok(())
}

/// Execute a trade decision
///
/// Routes the decision to the appropriate executor based on action type.
/// No need for separate core.rs - this is the coordinator.
pub async fn execute_trade(decision: &TradeDecision) -> Result<TradeResult, String> {
    match decision.action {
        TradeAction::Buy => buy::execute_buy(decision).await,
        TradeAction::Sell => sell::execute_sell(decision).await,
        TradeAction::DCA => buy::execute_dca(decision).await,
    }
}
```

### safety/mod.rs (CLEAN EXPORTS ONLY)

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

/// Initialize the safety system
pub async fn init_safety_system() -> Result<(), String> {
    logger::info(LogTag::Trader, "Initializing safety system...");
    logger::info(LogTag::Trader, "Safety system initialized");
    Ok(())
}
```

---

## Final Checklist Before Implementation

### Critical Fixes Required ✅

- [ ] Delete duplicate blacklist implementations
- [ ] Create sync-only blacklist.rs using tokens module
- [ ] Add risk check to exit evaluation flow
- [ ] Enhance entry evaluator with all safety checks
- [ ] Remove executors/core.rs from plan
- [ ] Update service.rs imports (auto → monitors)
- [ ] Verify all constants are accurate

### Files to Create ✅

- [ ] constants.rs
- [ ] monitors/mod.rs
- [ ] monitors/entry.rs
- [ ] monitors/exit.rs
- [ ] evaluators/mod.rs
- [ ] evaluators/entry.rs (enhanced)
- [ ] evaluators/exit.rs (with risk check)
- [ ] evaluators/dca.rs (merged)
- [ ] evaluators/strategies.rs (renamed)
- [ ] safety/blacklist.rs (NEW sync version)
- [ ] safety/cooldown.rs
- [ ] manual/api.rs
- [ ] manual/force.rs

### Files to Move ✅

- [ ] execution/ → executors/
- [ ] exit/roi.rs → evaluators/exit_roi.rs
- [ ] exit/trailing_stop.rs → evaluators/exit_trailing.rs
- [ ] exit/time_override.rs → evaluators/exit_time.rs
- [ ] executors/decision_cache.rs → executors/cache.rs

### Files to Update ✅

- [ ] mod.rs (architecture docs, imports)
- [ ] service.rs (auto → monitors)
- [ ] safety/mod.rs (clean exports only)
- [ ] safety/limits.rs (remove cooldown function)
- [ ] executors/mod.rs (cache import, keep execute_trade)
- [ ] webserver/routes/trading.rs (constants)
- [ ] webserver/routes/dashboard.rs (constants)
- [ ] webserver/routes/trader.rs (manual API)

### Files to Delete ✅

- [ ] auto/ directory (entire)
- [ ] exit/ directory (entire)
- [ ] manual/orders.rs
- [ ] OLD safety/blacklist.rs (cached version)

### Verification ✅

- [ ] cargo fmt
- [ ] cargo clippy
- [ ] cargo build
- [ ] cargo run (test startup)
- [ ] Update FLOW.md
- [ ] No \_old, \_legacy, or compatibility code remains

---

## Conclusion

**Recommendation: APPROVE with mandatory fixes**

The reorganization plan is architecturally sound and addresses real problems in the current codebase. However, **critical fixes are required** before implementation:

1. **Blacklist duplication must be resolved** (sync-only implementation)
2. **Risk check must be integrated** into exit flow
3. **Entry evaluator must be enhanced** with all safety checks
4. **No unnecessary wrappers** (remove core.rs from plan)
5. **Service integration** must be completed

With these fixes applied, the new structure will be:

- ✅ Systematic (no legacy code)
- ✅ Clear (single responsibility per module)
- ✅ Complete (all safety checks integrated)
- ✅ Maintainable (no duplication)
- ✅ Scalable (easy to add new strategies/exits)

**Priority:** HIGH - This should be implemented ASAP after fixes are applied.

**Risk:** LOW - Changes are well-defined and testable at each phase.

**Timeline:** 2-3 hours for complete implementation with verification.
