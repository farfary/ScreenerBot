# ScreenerBot Systematic Completion Plan

## Trader, Positions, and Swaps Module - Fundamental Solutions

**Status:** Investigation Complete - Ready for Implementation  
**Date:** October 23, 2025  
**Scope:** Complete partial sell support, DCA, and trader integration

---

## Executive Summary

This document provides a **systematic and fundamental** plan to complete the trader, positions, and swaps modules. The investigation revealed three critical gaps:

1. **Partial Sell Not Supported** - Positions and swaps only handle 100% exits
2. **DCA Not Supported** - Cannot accumulate into existing positions
3. **Trader Module Stubbed** - Entry/exit execution not implemented

The solution requires coordinated changes across 4 modules with database migrations and state machine enhancements.

---

## Part 1: Current State Analysis

### 1.1 Positions Module Assessment

**What Works:**

- ✅ Full position lifecycle (open → close → verify)
- ✅ State machine with transitions
- ✅ Global semaphore for max positions
- ✅ Verification queue with exponential backoff
- ✅ Price tracking (highs/lows)
- ✅ Database persistence with indexes
- ✅ Transaction signature mapping

**What's Missing:**

- ❌ **Partial exit support** (critical)
  - Position struct only tracks full amounts
  - `close_position_direct()` always sells 100%
  - No `remaining_token_amount` field
  - No partial exit state transitions
  - Verification expects full exit only

- ❌ **DCA support** (critical)
  - Cannot add to existing position
  - No average entry price calculation
  - No entry history tracking
  - `is_open_position()` blocks re-entry

- ❌ **Exit amount tracking**
  - No cumulative exit tracking
  - No partial P&L calculation
  - Cannot track multiple exits per position

**Current Position Struct:**

```rust
pub struct Position {
    pub token_amount: Option<u64>,        // FULL amount bought
    pub entry_size_sol: f64,              // FULL SOL spent on entry
    pub total_size_sol: f64,              // Same as entry_size_sol (no DCA)
    pub effective_entry_price: Option<f64>, // Single entry price
    // ... no remaining_token_amount field
    // ... no dca_count field
    // ... no entries history
}
```

### 1.2 Swaps Module Assessment

**What Works:**

- ✅ Multi-DEX routing (GMGN, Jupiter)
- ✅ Quote comparison (best output amount)
- ✅ Unified swap execution
- ✅ Sign and send helpers
- ✅ SwapResult with comprehensive data

**What's Missing:**

- ❌ **Partial amount parameter**
  - `execute_best_swap(input_amount)` is absolute only
  - No percentage-based selling
  - No helper to calculate partial amounts

**Current Swap API:**

```rust
pub async fn execute_best_swap(
    token: &Token,
    input_mint: &str,
    output_mint: &str,
    input_amount: u64,  // Absolute amount only, no percentage
    quote: UnifiedQuote,
) -> Result<SwapResult>
```

### 1.3 Transactions Module Assessment

**What Works:**

- ✅ Balance change extraction
- ✅ Swap detection and analysis
- ✅ Transaction verification
- ✅ Database caching

**What's Missing:**

- ❌ **Partial exit verification**
  - Assumes full position exit
  - Compares `token_amount` vs balance change
  - No "expected exit amount" tracking

**Current Verification Logic:**

```rust
// In verifier.rs
if let Some(token_amount) = position.token_amount {
    let dust_threshold = std::cmp::max(token_amount / 1_000, 10);
    if balance <= dust_threshold {
        // Ignores dust, assumes full exit
    }
}
```

### 1.4 Trader Module Assessment

**Status:** Architecture complete, execution STUBBED

**What's Ready:**

- ✅ Entry monitor loop structure
- ✅ Position monitor loop structure
- ✅ Safety systems (blacklist, limits, risk)
- ✅ Decision cache with retry logic
- ✅ Exit strategies (trailing stop, ROI, time override)
- ✅ Critical operation guards

**What's Stubbed:**

- ❌ `execution/buy.rs` - Returns TODO error
- ❌ `execution/sell.rs` - Returns TODO error
- ❌ `auto/strategy_manager.rs` - Returns empty decisions
- ❌ `auto/dca.rs` - Returns empty decisions

---

## Part 2: Architectural Design

### 2.1 Position Lifecycle States (Enhanced)

```
[Entry States]
Open (single entry)
Open + DCA (multiple entries, averaged price)

[Partial Exit States]
Open → PartialExitPending → Open (reduced amount)
Open → PartialExitVerified → Open (reduced amount)

[Full Exit States]
Open → ExitPending → Closed (current, works)
Open → ExitVerified → Closed (current, works)

[DCA States]
Open → DcaPending → Open (increased amount)
Open → DcaVerified → Open (averaged price)
```

### 2.2 Position Struct Extensions

**New Fields Required:**

```rust
pub struct Position {
    // Existing fields...
    pub token_amount: Option<u64>,  // INITIAL amount (first entry)
    pub entry_size_sol: f64,        // INITIAL SOL (first entry)
    pub total_size_sol: f64,        // CUMULATIVE SOL (all entries)

    // NEW: Partial exit support
    pub remaining_token_amount: Option<u64>,  // Current holdings after partial exits
    pub total_exited_amount: u64,             // Cumulative tokens sold
    pub average_exit_price: Option<f64>,      // Weighted average exit price
    pub partial_exit_count: u32,              // Number of partial exits

    // NEW: DCA support
    pub dca_count: u32,                       // Number of additional entries
    pub average_entry_price: f64,             // Weighted average entry price
    pub last_dca_time: Option<DateTime<Utc>>, // Last DCA timestamp

    // NEW: Exit history (for tracking)
    pub exit_history: Vec<ExitRecord>,        // All exits (partial + final)
    pub entry_history: Vec<EntryRecord>,      // All entries (initial + DCA)
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ExitRecord {
    pub timestamp: DateTime<Utc>,
    pub amount: u64,              // Tokens sold
    pub price: f64,               // Price at exit
    pub sol_received: f64,        // SOL received
    pub transaction_sig: String,
    pub is_partial: bool,         // true if partial, false if full
    pub percentage: f64,          // % of position sold
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct EntryRecord {
    pub timestamp: DateTime<Utc>,
    pub amount: u64,              // Tokens bought
    pub price: f64,               // Price at entry
    pub sol_spent: f64,           // SOL spent
    pub transaction_sig: String,
    pub is_dca: bool,             // true if DCA, false if initial
}
```

### 2.3 State Machine Transitions (New)

**Add to `src/positions/transitions.rs`:**

```rust
pub enum PositionTransition {
    // Existing transitions...
    EntryVerified { ... },
    ExitVerified { ... },

    // NEW: Partial exit transitions
    PartialExitSubmitted {
        position_id: i64,
        exit_signature: String,
        exit_amount: u64,           // Tokens to sell
        exit_percentage: f64,       // % of position
        market_price: f64,          // Price at submission
    },
    PartialExitVerified {
        position_id: i64,
        exit_amount: u64,           // Actual tokens sold
        sol_received: f64,          // Actual SOL received
        effective_exit_price: f64,  // Actual price
        fees: u64,                  // Transaction fee
        exit_time: DateTime<Utc>,
    },
    PartialExitFailed {
        position_id: i64,
        reason: String,
    },

    // NEW: DCA transitions
    DcaSubmitted {
        position_id: i64,
        dca_signature: String,
        dca_amount_sol: f64,        // Additional SOL invested
        market_price: f64,          // Price at DCA
    },
    DcaVerified {
        position_id: i64,
        tokens_bought: u64,         // Additional tokens
        sol_spent: f64,             // Actual SOL spent
        effective_price: f64,       // Actual price
        fees: u64,                  // Transaction fee
        dca_time: DateTime<Utc>,
    },
    DcaFailed {
        position_id: i64,
        reason: String,
    },
}
```

### 2.4 Database Schema Changes

**Migrations Required:**

**Migration 1: Add partial exit fields to positions table**

```sql
-- Add to positions table
ALTER TABLE positions ADD COLUMN remaining_token_amount INTEGER;
ALTER TABLE positions ADD COLUMN total_exited_amount INTEGER DEFAULT 0;
ALTER TABLE positions ADD COLUMN average_exit_price REAL;
ALTER TABLE positions ADD COLUMN partial_exit_count INTEGER DEFAULT 0;

-- Add DCA fields
ALTER TABLE positions ADD COLUMN dca_count INTEGER DEFAULT 0;
ALTER TABLE positions ADD COLUMN average_entry_price REAL;
ALTER TABLE positions ADD COLUMN last_dca_time TEXT;

-- Initialize remaining_token_amount from token_amount for existing positions
UPDATE positions
SET remaining_token_amount = token_amount
WHERE remaining_token_amount IS NULL AND token_amount IS NOT NULL;
```

**Migration 2: Create exit/entry history tables**

```sql
-- Exit history (all exits: partial + full)
CREATE TABLE IF NOT EXISTS position_exits (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    position_id INTEGER NOT NULL,
    timestamp TEXT NOT NULL,
    amount INTEGER NOT NULL,
    price REAL NOT NULL,
    sol_received REAL NOT NULL,
    transaction_signature TEXT NOT NULL,
    is_partial BOOLEAN NOT NULL,
    percentage REAL NOT NULL,
    fees_lamports INTEGER,
    FOREIGN KEY (position_id) REFERENCES positions(id) ON DELETE CASCADE
);
CREATE INDEX idx_position_exits_position_id ON position_exits(position_id);
CREATE INDEX idx_position_exits_timestamp ON position_exits(timestamp);

-- Entry history (initial + DCA)
CREATE TABLE IF NOT EXISTS position_entries (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    position_id INTEGER NOT NULL,
    timestamp TEXT NOT NULL,
    amount INTEGER NOT NULL,
    price REAL NOT NULL,
    sol_spent REAL NOT NULL,
    transaction_signature TEXT NOT NULL,
    is_dca BOOLEAN NOT NULL,
    fees_lamports INTEGER,
    FOREIGN KEY (position_id) REFERENCES positions(id) ON DELETE CASCADE
);
CREATE INDEX idx_position_entries_position_id ON position_entries(position_id);
CREATE INDEX idx_position_entries_timestamp ON position_entries(timestamp);
```

---

## Part 3: Implementation Plan

### Phase 1: Foundation (Positions Module)

**Step 1.1: Extend Position struct**

- File: `src/positions/types.rs`
- Add new fields (remaining_token_amount, dca_count, etc.)
- Add ExitRecord and EntryRecord structs
- Update serialization/deserialization

**Step 1.2: Database migrations**

- File: `src/positions/db.rs`
- Add migration functions
- Run migrations on startup
- Add get/set methods for new fields

**Step 1.3: Add partial exit function**

- File: `src/positions/operations.rs`
- Create `partial_close_position()` function:
  ```rust
  pub async fn partial_close_position(
      mint: &str,
      exit_percentage: f64,  // 0.0-100.0
      reason: &str
  ) -> Result<String, String>
  ```
- Calculate exit amount from remaining_token_amount
- Execute partial swap
- Update position state (keep open)
- DO NOT release semaphore permit

**Step 1.4: Add DCA function**

- File: `src/positions/operations.rs`
- Create `add_to_position()` function:
  ```rust
  pub async fn add_to_position(
      mint: &str,
      dca_amount_sol: f64
  ) -> Result<String, String>
  ```
- Check DCA limits (max count, cooldown)
- Execute buy swap
- Recalculate average entry price
- DO NOT consume another semaphore permit

**Step 1.5: Update state transitions**

- File: `src/positions/transitions.rs`
- Add PartialExitSubmitted, PartialExitVerified
- Add DcaSubmitted, DcaVerified
- Update apply_transition() logic

**Step 1.6: Update verifier**

- File: `src/positions/verifier.rs`
- Track expected_exit_amount for partial exits
- Verify partial exits correctly
- Update remaining_token_amount after verification

### Phase 2: Swaps Module Support

**Step 2.1: Add partial sell helper**

- File: `src/swaps/mod.rs`
- Create `calculate_partial_amount()`:
  ```rust
  pub fn calculate_partial_amount(
      total_amount: u64,
      percentage: f64
  ) -> u64
  ```

**Step 2.2: Add exit type parameter**

- Update `execute_best_swap()` signature:

  ```rust
  pub async fn execute_best_swap(
      token: &Token,
      input_mint: &str,
      output_mint: &str,
      input_amount: u64,
      quote: UnifiedQuote,
      exit_type: Option<ExitType>,  // NEW
  ) -> Result<SwapResult>

  pub enum ExitType {
      Full,
      Partial { percentage: f64 },
  }
  ```

**Step 2.3: Update quote functions**

- Pass exit_type through quote chain
- Log partial vs full exits clearly

### Phase 3: Config Schema Extensions

**Step 3.1: Add DCA config**

- File: `src/config/schemas/trader.rs`
- Add fields:
  ```rust
  dca_enabled: bool = false,
  dca_threshold_pct: f64 = -10.0,  // Enter DCA when down X%
  dca_max_count: u32 = 2,           // Max 2 additional entries
  dca_size_percentage: f64 = 50.0, // 50% of initial size
  dca_cooldown_minutes: i64 = 30,  // Wait 30min between DCAs
  ```

**Step 3.2: Add partial exit config**

- File: `src/config/schemas/positions.rs`
- Add fields:
  ```rust
  partial_exit_enabled: bool = false,
  partial_exit_default_pct: f64 = 50.0,  // Default to 50% exits
  partial_exit_min_pct: f64 = 10.0,      // Min 10% per exit
  partial_exit_max_pct: f64 = 90.0,      // Max 90% per exit
  ```

**Step 3.3: Add trailing stop config**

- File: `src/config/schemas/positions.rs`
- Add fields (if not already present):
  ```rust
  trailing_stop_enabled: bool = false,
  trailing_stop_activation_pct: f64 = 10.0,  // Activate after +10%
  trailing_stop_distance_pct: f64 = 5.0,     // Trail by 5%
  ```

### Phase 4: Trader Module Completion

**Step 4.1: Implement buy execution**

- File: `src/trader/execution/buy.rs`
- Replace TODO stub with real implementation:
  ```rust
  pub async fn execute_buy(decision: &TradeDecision) -> Result<TradeResult, String> {
      // 1. Get trade size from decision or config
      // 2. Call positions::open_position_direct()
      // 3. Handle success/failure
      // 4. Return TradeResult
  }
  ```

**Step 4.2: Implement sell execution**

- File: `src/trader/execution/sell.rs`
- Replace TODO stub:
  ```rust
  pub async fn execute_sell(decision: &TradeDecision) -> Result<TradeResult, String> {
      // 1. Check if partial or full exit
      // 2. Call positions::partial_close_position() or close_position_direct()
      // 3. Handle success/failure
      // 4. Return TradeResult
  }
  ```

**Step 4.3: Implement DCA execution**

- File: `src/trader/execution/buy.rs`
- Add DCA handler:
  ```rust
  pub async fn execute_dca(decision: &TradeDecision) -> Result<TradeResult, String> {
      // 1. Get DCA size from config
      // 2. Call positions::add_to_position()
      // 3. Handle success/failure
      // 4. Return TradeResult
  }
  ```

**Step 4.4: Implement strategy manager**

- File: `src/trader/auto/strategy_manager.rs`
- Remove TODO stub
- Integrate with strategies module (when ready)
- For now, use simple rule-based logic

**Step 4.5: Complete DCA monitor**

- File: `src/trader/auto/dca.rs`
- Implement DCA opportunity detection:
  ```rust
  pub async fn process_dca_opportunities() -> Result<Vec<TradeDecision>, String> {
      // 1. Get open positions
      // 2. Check each for DCA eligibility:
      //    - Current P&L below threshold
      //    - DCA count < max
      //    - Cooldown elapsed
      // 3. Create TradeDecision for eligible positions
  }
  ```

**Step 4.6: Update exit strategies**

- File: `src/trader/exit/trailing_stop.rs`
- Support partial exits (sell % on trailing stop trigger)
- File: `src/trader/exit/roi.rs`
- Support partial exits (sell % at profit targets)

### Phase 5: Testing & Validation

**Step 5.1: Unit tests**

- Test partial amount calculation
- Test average entry price calculation
- Test remaining amount tracking

**Step 5.2: Integration tests**

- Test full position lifecycle with partial exits
- Test DCA flow (multiple entries)
- Test verification with partial exits

**Step 5.3: Dry-run testing**

- Enable `dry_run_mode: true` in config
- Test with real market data
- Verify all logs and state changes

---

## Part 4: Implementation Order (Critical Path)

### Week 1: Foundation

1. ✅ Database migrations (positions table)
2. ✅ Position struct extensions
3. ✅ State transitions (partial/DCA)

### Week 2: Core Logic

4. ✅ Partial exit function (positions)
5. ✅ DCA function (positions)
6. ✅ Verifier updates (partial support)
7. ✅ Swaps module helpers (partial amount)

### Week 3: Config & Trader

8. ✅ Config schema extensions
9. ✅ Trader execution/buy.rs (real implementation)
10. ✅ Trader execution/sell.rs (real implementation)
11. ✅ Trader auto/dca.rs (real implementation)

### Week 4: Exit Strategies & Testing

12. ✅ Exit strategies (partial support)
13. ✅ Strategy manager integration
14. ✅ Comprehensive testing
15. ✅ Dry-run validation

---

## Part 5: Critical Considerations

### 5.1 Semaphore Management (CRITICAL)

**Current Behavior:**

- Semaphore permit acquired BEFORE open
- Permit "forgotten" (consumed) after successful open
- Permit released on close (ExitVerified)

**New Behavior Required:**

- **Partial Exit:** DO NOT release permit (position still open)
- **DCA:** DO NOT consume another permit (same position)
- **Full Exit:** Release permit as before

**Implementation:**

```rust
// In partial_close_position()
// DO NOT call release_global_position_permit()

// In add_to_position()
// DO NOT call acquire_global_position_permit()

// Only release on full exit (existing behavior)
```

### 5.2 Verification System (CRITICAL)

**Current Issue:**

- Verifier expects `token_amount` to match balance change
- Fails for partial exits

**Solution:**

```rust
pub struct VerificationItem {
    // Existing fields...
    pub expected_exit_amount: Option<u64>,  // NEW: For partial exits
    pub is_partial_exit: bool,              // NEW: Flag for partial
}

// In verify_transaction()
let expected_amount = if item.is_partial_exit {
    item.expected_exit_amount.unwrap_or(position.token_amount.unwrap_or(0))
} else {
    position.token_amount.unwrap_or(0)
};

// Compare against expected_amount, not full token_amount
```

### 5.3 Average Price Calculation (CRITICAL)

**For DCA (Weighted Average Entry):**

```rust
pub fn calculate_average_entry_price(
    current_average: f64,
    current_total_sol: f64,
    new_tokens: u64,
    new_sol: f64,
    decimals: u8
) -> f64 {
    let new_total_sol = current_total_sol + new_sol;
    let scale = 10_f64.powi(decimals as i32);
    let new_tokens_float = (new_tokens as f64) / scale;

    // Weighted average: total_sol / total_tokens
    new_total_sol / (new_tokens_float + (current_total_sol / current_average))
}
```

**For Partial Exits (Weighted Average Exit):**

```rust
pub fn calculate_average_exit_price(
    current_average: Option<f64>,
    current_total_exited: u64,
    new_exited: u64,
    new_price: f64,
    decimals: u8
) -> f64 {
    let scale = 10_f64.powi(decimals as i32);

    match current_average {
        Some(avg) => {
            let total_tokens = current_total_exited + new_exited;
            let current_weight = (current_total_exited as f64) / (total_tokens as f64);
            let new_weight = (new_exited as f64) / (total_tokens as f64);
            (avg * current_weight) + (new_price * new_weight)
        },
        None => new_price,  // First exit
    }
}
```

### 5.4 P&L Calculation (CRITICAL)

**Current (Full Exit Only):**

```rust
pub fn calculate_position_pnl(position: &Position, current_price: f64) -> PositionPnL {
    let entry_price = position.effective_entry_price.unwrap_or(position.entry_price);
    let pnl_pct = ((current_price - entry_price) / entry_price) * 100.0;
    // ...
}
```

**New (Partial Exit Support):**

```rust
pub fn calculate_position_pnl(position: &Position, current_price: f64) -> PositionPnL {
    let entry_price = position.average_entry_price;  // Use weighted average
    let remaining = position.remaining_token_amount.unwrap_or(0);

    // Unrealized P&L (on remaining tokens)
    let unrealized_pnl_pct = if remaining > 0 {
        ((current_price - entry_price) / entry_price) * 100.0
    } else {
        0.0
    };

    // Realized P&L (from exits)
    let realized_pnl = if let Some(avg_exit) = position.average_exit_price {
        let exited_sol_value = /* calculate from history */;
        let cost_basis = /* calculate from entry history */;
        exited_sol_value - cost_basis
    } else {
        0.0
    };

    PositionPnL {
        unrealized_pnl,
        realized_pnl,
        total_pnl: unrealized_pnl + realized_pnl,
        // ...
    }
}
```

### 5.5 State Machine Consistency (CRITICAL)

**Ensure Atomic Transitions:**

```rust
// In apply_transition()
pub async fn apply_transition(transition: PositionTransition) -> Result<(), String> {
    match transition {
        PositionTransition::PartialExitVerified { position_id, exit_amount, ... } => {
            // 1. Update position state (atomic)
            update_position_state_atomic(|pos| {
                pos.remaining_token_amount = Some(
                    pos.remaining_token_amount.unwrap_or(pos.token_amount.unwrap_or(0)) - exit_amount
                );
                pos.total_exited_amount += exit_amount;
                pos.partial_exit_count += 1;
                pos.average_exit_price = calculate_average_exit_price(...);
            }).await;

            // 2. Add to exit history
            add_exit_record(position_id, exit_amount, ...).await?;

            // 3. Update database
            db::save_position(&position).await?;

            // 4. Record event
            events::record_position_event(...).await;

            // DO NOT release semaphore permit (position still open)
            Ok(())
        },
        // ... other transitions
    }
}
```

---

## Part 6: API Changes Summary

### New Public Functions (positions module)

```rust
// Partial exit
pub async fn partial_close_position(
    mint: &str,
    exit_percentage: f64,
    reason: &str
) -> Result<String, String>;

// DCA
pub async fn add_to_position(
    mint: &str,
    dca_amount_sol: f64
) -> Result<String, String>;

// Queries
pub async fn get_position_unrealized_pnl(mint: &str) -> Option<f64>;
pub async fn get_position_realized_pnl(mint: &str) -> Option<f64>;
pub async fn get_position_dca_count(mint: &str) -> u32;
pub async fn get_position_exit_history(mint: &str) -> Vec<ExitRecord>;
pub async fn get_position_entry_history(mint: &str) -> Vec<EntryRecord>;
```

### Updated Functions (positions module)

```rust
// Now supports partial exits
pub async fn close_position_direct(
    mint: &str,
    exit_reason: &str,
    exit_type: ExitType  // NEW parameter
) -> Result<String, String>;

pub enum ExitType {
    Full,
    Partial { percentage: f64 },
}
```

### New Helper Functions (swaps module)

```rust
pub fn calculate_partial_amount(total: u64, pct: f64) -> u64;
pub fn get_partial_exit_quote(...) -> Result<UnifiedQuote>;
```

---

## Part 7: Documentation Updates Required

1. **FLOW.md** - Add partial exit and DCA flows
2. **trader-plan.md** - Mark as implemented
3. **TRADER_MODULE_MIGRATION.md** - Update completion status
4. **API documentation** - Document new functions
5. **Config documentation** - Document new config fields

---

## Part 8: Risks & Mitigation

### Risk 1: Semaphore Permit Leaks

**Impact:** Max positions limit breaks  
**Mitigation:**

- Add debug logging for all permit operations
- Add periodic audit (compare open positions vs permits)
- Add recovery mechanism (resync on startup)

### Risk 2: Verification System Confusion

**Impact:** Partial exits might fail verification  
**Mitigation:**

- Clear logging of expected vs actual amounts
- Separate verification paths for partial vs full
- Add verification dry-run mode

### Risk 3: Average Price Calculation Errors

**Impact:** Incorrect P&L reporting  
**Mitigation:**

- Unit tests for all calculation functions
- Debug logging of all price calculations
- Cross-check with on-chain data

### Risk 4: Database Migration Failures

**Impact:** Existing positions corrupted  
**Mitigation:**

- Backup database before migration
- Test migrations on copy first
- Add rollback capability
- Initialize new fields safely (NULL handling)

### Risk 5: State Machine Deadlocks

**Impact:** Positions stuck in transition states  
**Mitigation:**

- Timeout on all state transitions
- Recovery mechanism for stuck positions
- Clear state transition logging
- Manual override capability

---

## Part 9: Success Metrics

### Phase 1 Complete When:

- ✅ Database migrations pass
- ✅ Position struct compiles with new fields
- ✅ State transitions defined

### Phase 2 Complete When:

- ✅ Partial exit creates correct swap
- ✅ Position stays open after partial exit
- ✅ Remaining amount calculated correctly

### Phase 3 Complete When:

- ✅ DCA creates second entry
- ✅ Average entry price calculated correctly
- ✅ Total size increases properly

### Phase 4 Complete When:

- ✅ Trader executes real buys
- ✅ Trader executes real sells (full and partial)
- ✅ Trader executes DCA when eligible

### Phase 5 Complete When:

- ✅ All integration tests pass
- ✅ Dry-run mode validates correctly
- ✅ Production deploy successful

---

## Part 10: Implementation Checklist

### Positions Module

- [ ] Extend Position struct with new fields
- [ ] Create database migrations
- [ ] Implement `partial_close_position()`
- [ ] Implement `add_to_position()`
- [ ] Add new state transitions
- [ ] Update verifier for partial exits
- [ ] Add exit/entry history tables
- [ ] Update P&L calculations
- [ ] Add query functions for new data

### Swaps Module

- [ ] Add `calculate_partial_amount()` helper
- [ ] Update `execute_best_swap()` signature
- [ ] Add exit_type parameter throughout
- [ ] Update logging for partial vs full

### Transactions Module

- [ ] Add expected_exit_amount to VerificationItem
- [ ] Update verification logic for partial exits
- [ ] Add is_partial_exit flag

### Config Module

- [ ] Add DCA config fields to trader.rs
- [ ] Add partial exit config to positions.rs
- [ ] Add trailing stop config (if missing)
- [ ] Update CONFIG_METADATA for UI

### Trader Module

- [ ] Implement `execution/buy.rs` (remove TODO)
- [ ] Implement `execution/sell.rs` (remove TODO)
- [ ] Implement `auto/dca.rs` (remove TODO)
- [ ] Update `auto/strategy_manager.rs` (remove TODO)
- [ ] Update exit strategies for partial support
- [ ] Add partial exit to decision cache

### Testing

- [ ] Unit tests for partial amount calculation
- [ ] Unit tests for average price calculation
- [ ] Integration test: full position lifecycle
- [ ] Integration test: partial exit flow
- [ ] Integration test: DCA flow
- [ ] Dry-run validation

### Documentation

- [ ] Update FLOW.md
- [ ] Update trader-plan.md
- [ ] Update TRADER_MODULE_MIGRATION.md
- [ ] Add API documentation
- [ ] Add config documentation

---

## Conclusion

This plan provides a **systematic and fundamental** approach to completing the trader, positions, and swaps modules. The solution is architected to:

1. **Maintain backward compatibility** - Existing full-exit flow unchanged
2. **Add new capabilities** - Partial exits and DCA as optional features
3. **Preserve system integrity** - Semaphore management, verification, state machine
4. **Enable future extensions** - Clean APIs, modular design, comprehensive history

**Estimated Implementation Time:** 3-4 weeks with systematic approach

**Priority Order:**

1. Positions module extensions (foundation)
2. Swaps module support (dependency)
3. Verification updates (critical)
4. Trader module completion (integration)
5. Testing and validation (safety)

**Next Steps:**

1. Review this plan for completeness
2. Create implementation tickets
3. Start with Phase 1 (database migrations)
4. Proceed systematically through phases
5. Test thoroughly at each phase
