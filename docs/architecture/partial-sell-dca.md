# Partial Sell & DCA Flow Diagrams

## Visual Guide for Implementation

**Date:** October 23, 2025  
**Related:** SYSTEMATIC_COMPLETION_PLAN.md

---

## 1. Current vs Proposed Architecture

### Current Architecture (Full Exit Only)

```
┌─────────────────────────────────────────────────────────────┐
│                    CURRENT POSITION FLOW                    │
│                     (Full Exit Only)                        │
└─────────────────────────────────────────────────────────────┘

[Entry Phase]
positions::open_position_direct(mint)
    │
    ├─ Acquire semaphore permit (consume 1 slot)
    ├─ Execute swap (SOL → Token)
    ├─ Create Position {
    │    token_amount: 1000 tokens
    │    entry_size_sol: 0.1 SOL
    │    total_size_sol: 0.1 SOL
    │  }
    └─ "Forget" permit (slot consumed for position lifetime)

[Exit Phase]
positions::close_position_direct(mint)
    │
    ├─ Get ALL tokens from wallet
    ├─ Execute swap (Token → SOL)
    │    sell_amount = token_amount (100%)
    ├─ Mark position closed
    └─ Release semaphore permit (slot freed)

[Limitations]
❌ Cannot sell partial amounts (always 100%)
❌ Cannot add to position (DCA blocked)
❌ Cannot track multiple exits
❌ Single entry price only
```

### Proposed Architecture (Partial + DCA)

```
┌─────────────────────────────────────────────────────────────┐
│                    ENHANCED POSITION FLOW                   │
│                 (Partial Exit + DCA Support)                │
└─────────────────────────────────────────────────────────────┘

[Initial Entry]
positions::open_position_direct(mint)
    │
    ├─ Acquire semaphore permit (consume 1 slot)
    ├─ Execute swap (SOL → Token)
    ├─ Create Position {
    │    token_amount: 1000 tokens        (initial)
    │    remaining_token_amount: 1000     (current holdings)
    │    entry_size_sol: 0.1 SOL          (initial)
    │    total_size_sol: 0.1 SOL          (cumulative)
    │    dca_count: 0
    │    partial_exit_count: 0
    │    average_entry_price: 0.0001 SOL/token
    │    entry_history: [Entry1]
    │  }
    └─ "Forget" permit (slot consumed)

[DCA Entry] (Optional, repeatable)
positions::add_to_position(mint, 0.05 SOL)
    │
    ├─ NO semaphore permit needed (same slot)
    ├─ Execute swap (SOL → Token)
    ├─ Update Position {
    │    token_amount: 1000               (unchanged, initial)
    │    remaining_token_amount: 1500     (+500 from DCA)
    │    entry_size_sol: 0.1              (unchanged, initial)
    │    total_size_sol: 0.15 SOL         (+0.05 from DCA)
    │    dca_count: 1                     (+1)
    │    average_entry_price: 0.0001 SOL/token (recalculated)
    │    entry_history: [Entry1, DCA1]
    │  }
    └─ Permit still consumed (position still open)

[Partial Exit] (Optional, repeatable)
positions::partial_close_position(mint, 50%)
    │
    ├─ Calculate exit_amount = remaining * 0.5 = 750 tokens
    ├─ Execute swap (Token → SOL)
    ├─ Update Position {
    │    remaining_token_amount: 750      (1500 - 750)
    │    total_exited_amount: 750         (+750)
    │    partial_exit_count: 1            (+1)
    │    average_exit_price: 0.00015 SOL/token (weighted avg)
    │    exit_history: [Exit1{amount:750, pct:50%}]
    │  }
    └─ Permit still consumed (position STILL OPEN)

[Full Exit] (Final)
positions::close_position_direct(mint, Full)
    │
    ├─ Sell remaining 750 tokens
    ├─ Update Position {
    │    remaining_token_amount: 0
    │    total_exited_amount: 1500        (750 + 750)
    │    exit_history: [Exit1, FinalExit]
    │  }
    └─ Release semaphore permit (slot freed)

[Benefits]
✅ Partial profit taking
✅ Dollar cost averaging
✅ Risk management (staged exits)
✅ Complete history tracking
✅ Accurate P&L calculation
```

---

## 2. Position State Machine (Enhanced)

### Current State Machine (Simple)

```
┌────────┐
│  Open  │ ──entry_verified──►
└────────┘
     │
     │ close_position_direct()
     ▼
┌────────────┐
│ ExitPending│ ──exit_verified──►
└────────────┘
     │
     ▼
┌────────┐
│ Closed │
└────────┘
```

### Enhanced State Machine (Partial + DCA)

```
┌─────────────────────────────────────────────────────────────┐
│                    ENHANCED STATE MACHINE                   │
└─────────────────────────────────────────────────────────────┘

                    ┌───────────────┐
         ┌─────────►│  DcaPending   │──dca_verified──┐
         │          └───────────────┘                │
         │                                           ▼
    add_to_position()                      ┌────────────────┐
         │                                  │  Open (+ DCA)  │
         │                                  │ [dca_count++]  │
         │          ┌───────────────┐       └────────────────┘
         └──────────│     Open      │◄──entry_verified──────┬
                    │  [Initial]    │                        │
                    └───────────────┘                        │
                           │                                 │
                           │ partial_close_position(%)       │
                           ▼                                 │
                    ┌───────────────────┐                    │
                    │ PartialExitPending│──partial_verified──┤
                    └───────────────────┘                    │
                           │                                 │
                           ▼                                 │
                    ┌───────────────────┐                    │
                    │ Open (Reduced)    │                    │
                    │[remaining reduced]│◄───────────────────┘
                    └───────────────────┘
                           │
                           │ close_position_direct(Full)
                           ▼
                    ┌───────────────┐
                    │  ExitPending  │──exit_verified──►
                    └───────────────┘
                           │
                           ▼
                    ┌───────────────┐
                    │    Closed     │
                    │[permit freed] │
                    └───────────────┘
```

---

## 3. Verification Flow Comparison

### Current Verification (Full Exit Only)

```
[verify_exit_transaction(sig)]
    │
    ├─ Get transaction from blockchain
    ├─ Extract balance changes
    ├─ Get position.token_amount (e.g., 1000 tokens)
    │
    ├─ Compare: balance_change == token_amount?
    │    └─ If match: ✅ ExitVerified
    │    └─ If mismatch: ❌ Retry or fail
    │
    └─ Assumption: ALWAYS selling 100% of tokens
```

### Enhanced Verification (Partial Support)

```
[verify_exit_transaction(sig, expected_exit_amount)]
    │
    ├─ Get transaction from blockchain
    ├─ Extract balance changes
    │
    ├─ Get verification context:
    │    ├─ Is partial exit? (from VerificationItem)
    │    ├─ Expected amount (e.g., 500 tokens if 50% exit)
    │    └─ Remaining amount (e.g., 500 tokens left)
    │
    ├─ Compare: balance_change == expected_exit_amount?
    │    ├─ Full exit: expected = token_amount
    │    └─ Partial exit: expected = specified amount
    │
    ├─ If match:
    │    ├─ Full exit: ExitVerified → Closed
    │    └─ Partial exit: PartialExitVerified → Open (reduced)
    │
    └─ If mismatch: ❌ Retry or fail with detailed error
```

---

## 4. P&L Calculation (Enhanced)

### Current P&L (Simple)

```
[calculate_position_pnl(position, current_price)]
    │
    ├─ entry_price = effective_entry_price
    ├─ pnl_pct = (current_price - entry_price) / entry_price * 100
    │
    └─ Return: {
         unrealized_pnl_pct: 25.5%,
         realized_pnl: 0 (no partial exits),
         total_pnl: 25.5%
       }

Limitation: Cannot track realized gains from partial exits
```

### Enhanced P&L (Partial + DCA)

```
[calculate_position_pnl(position, current_price)]
    │
    ├─ Get weighted averages:
    │    ├─ average_entry_price (from DCA history)
    │    └─ average_exit_price (from partial exit history)
    │
    ├─ Calculate unrealized P&L (on remaining tokens):
    │    ├─ remaining = remaining_token_amount (e.g., 500)
    │    ├─ cost_basis = remaining * average_entry_price
    │    ├─ current_value = remaining * current_price
    │    └─ unrealized = current_value - cost_basis
    │
    ├─ Calculate realized P&L (from exits):
    │    ├─ total_exited = 500 tokens
    │    ├─ exit_proceeds = total_exited * average_exit_price
    │    ├─ exit_cost = total_exited * average_entry_price
    │    └─ realized = exit_proceeds - exit_cost
    │
    ├─ Total P&L = unrealized + realized
    │
    └─ Return: {
         unrealized_pnl: 0.01 SOL (+10%),
         realized_pnl: 0.02 SOL (+20%),
         total_pnl: 0.03 SOL (+15% overall),
         remaining_tokens: 500,
         exited_tokens: 500,
         average_entry: 0.0001 SOL,
         average_exit: 0.00012 SOL
       }
```

---

## 5. DCA Flow (Dollar Cost Averaging)

### Scenario: Price Drops, Add to Position

```
┌─────────────────────────────────────────────────────────────┐
│                      DCA EXAMPLE FLOW                       │
└─────────────────────────────────────────────────────────────┘

Time T0: Initial Entry
─────────────────────
Token Price: 0.0001 SOL/token
Investment: 0.1 SOL
Tokens Bought: 1000 tokens

Position State:
  token_amount: 1000
  remaining_token_amount: 1000
  entry_size_sol: 0.1
  total_size_sol: 0.1
  average_entry_price: 0.0001
  dca_count: 0

─────────────────────────────────────────────────────────────

Time T1: Price Drops 20%, DCA Trigger
─────────────────────
Token Price: 0.00008 SOL/token (down 20%)
Current P&L: -20%

DCA Decision:
✅ dca_enabled: true
✅ dca_threshold_pct: -10% (triggered)
✅ dca_count: 0 < dca_max_count: 2
✅ dca_cooldown: elapsed

DCA Execution:
  Investment: 0.05 SOL (50% of initial)
  Tokens Bought: 625 tokens (0.05 / 0.00008)

Updated Position State:
  token_amount: 1000 (initial, unchanged)
  remaining_token_amount: 1625 (1000 + 625)
  entry_size_sol: 0.1 (initial, unchanged)
  total_size_sol: 0.15 (0.1 + 0.05)
  average_entry_price: 0.0000923 SOL
    └─ Calculation: 0.15 SOL / 1625 tokens
  dca_count: 1
  entry_history: [
    {timestamp: T0, amount: 1000, price: 0.0001, sol: 0.1, is_dca: false},
    {timestamp: T1, amount: 625, price: 0.00008, sol: 0.05, is_dca: true}
  ]

New Break-Even: 0.0000923 SOL/token (down 7.7% from initial)

─────────────────────────────────────────────────────────────

Time T2: Price Recovers, Partial Exit
─────────────────────
Token Price: 0.00012 SOL/token (up 50% from DCA entry)
Current P&L vs Average: +30%

Exit Decision:
  Exit 50% to lock in profits

Partial Exit Execution:
  Tokens to Sell: 812.5 (50% of 1625)
  SOL Received: 0.0975 SOL (812.5 * 0.00012)

Updated Position State:
  remaining_token_amount: 812.5 (1625 - 812.5)
  total_exited_amount: 812.5
  average_exit_price: 0.00012
  partial_exit_count: 1
  exit_history: [
    {timestamp: T2, amount: 812.5, price: 0.00012,
     sol_received: 0.0975, is_partial: true, percentage: 50%}
  ]

P&L Summary:
  Investment: 0.15 SOL
  Realized: 0.0975 SOL (from 50% exit)
  Remaining Value: 0.0975 SOL (812.5 tokens @ 0.00012)
  Total Value: 0.195 SOL
  Total P&L: +0.045 SOL (+30%)
```

---

## 6. Semaphore Management (Critical)

### Current Flow (Simple)

```
[Semaphore: Capacity = 5]

Open Position 1:
  ├─ Acquire permit (4 remaining)
  └─ Forget permit (consumed, 4 available)

Open Position 2:
  ├─ Acquire permit (3 remaining)
  └─ Forget permit (consumed, 3 available)

Close Position 1:
  ├─ Release permit (4 available)
  └─ Slot freed

Open Position 3:
  ├─ Acquire permit (3 remaining)
  └─ Forget permit (consumed, 3 available)

State: 3 permits consumed (Pos2, Pos3), 2 available
```

### Enhanced Flow (Partial + DCA)

```
[Semaphore: Capacity = 5]

Open Position A:
  ├─ Acquire permit (4 remaining)
  └─ Forget permit (consumed, 4 available)
  State: 1 permit consumed

DCA into Position A:
  ├─ NO acquire (same position)
  └─ NO release (position still open)
  State: 1 permit consumed (unchanged)

Partial Exit Position A (50%):
  ├─ NO release (position STILL OPEN)
  └─ Remaining tokens tracked
  State: 1 permit consumed (unchanged)

Partial Exit Position A (remaining 50%):
  ├─ NOW release permit (position CLOSED)
  └─ Slot freed
  State: 0 permits consumed, 5 available

Critical Rules:
✅ 1 permit per position (regardless of DCA count)
✅ NO release on partial exits
✅ Release ONLY on full close
✅ Periodic audit: open_positions.count() == consumed_permits
```

---

## 7. Database Schema Evolution

### Before (Current)

```sql
positions table:
├─ token_amount INTEGER              -- Full amount bought
├─ entry_size_sol REAL                -- SOL spent
├─ total_size_sol REAL                -- Same as entry_size_sol
├─ effective_entry_price REAL         -- Single entry price
└─ (no remaining, no history, no DCA tracking)
```

### After (Enhanced)

```sql
positions table:
├─ token_amount INTEGER               -- Initial amount (first entry)
├─ remaining_token_amount INTEGER     -- 🆕 Current holdings
├─ total_exited_amount INTEGER        -- 🆕 Cumulative sold
├─ entry_size_sol REAL                -- Initial SOL (first entry)
├─ total_size_sol REAL                -- 🆕 Cumulative SOL invested
├─ effective_entry_price REAL         -- Initial price (deprecated)
├─ average_entry_price REAL           -- 🆕 Weighted average (DCA)
├─ average_exit_price REAL            -- 🆕 Weighted average (exits)
├─ partial_exit_count INTEGER         -- 🆕 Number of partial exits
├─ dca_count INTEGER                  -- 🆕 Number of DCA entries
└─ last_dca_time TEXT                 -- 🆕 Last DCA timestamp

position_exits table (NEW):
├─ id INTEGER PRIMARY KEY
├─ position_id INTEGER FK
├─ timestamp TEXT
├─ amount INTEGER                     -- Tokens sold
├─ price REAL                         -- Exit price
├─ sol_received REAL                  -- SOL received
├─ transaction_signature TEXT
├─ is_partial BOOLEAN                 -- True for partial, false for full
└─ percentage REAL                    -- % of position sold

position_entries table (NEW):
├─ id INTEGER PRIMARY KEY
├─ position_id INTEGER FK
├─ timestamp TEXT
├─ amount INTEGER                     -- Tokens bought
├─ price REAL                         -- Entry price
├─ sol_spent REAL                     -- SOL spent
├─ transaction_signature TEXT
├─ is_dca BOOLEAN                     -- True for DCA, false for initial
└─ fees_lamports INTEGER
```

---

## 8. API Usage Examples

### Current API (Full Exit Only)

```rust
// Open position
let sig = positions::open_position_direct("mint123").await?;

// Close position (always 100%)
let exit_sig = positions::close_position_direct(
    "mint123",
    "profit_target"
).await?;
```

### Enhanced API (Partial + DCA)

```rust
// 1. Initial Entry
let sig = positions::open_position_direct("mint123").await?;
// Position: 1000 tokens, 0.1 SOL, average: 0.0001

// 2. DCA Entry (optional, when price drops)
let dca_sig = positions::add_to_position(
    "mint123",
    0.05  // Additional SOL
).await?;
// Position: 1625 tokens, 0.15 SOL, average: 0.0000923

// 3. Partial Exit #1 (take 50% profit)
let partial_sig = positions::partial_close_position(
    "mint123",
    50.0,  // Exit 50%
    "take_profit_50pct"
).await?;
// Position: 812.5 tokens remaining, 0.15 SOL invested

// 4. Partial Exit #2 (take another 25%)
let partial_sig2 = positions::partial_close_position(
    "mint123",
    25.0,  // Exit 25% of REMAINING
    "trailing_stop"
).await?;
// Position: 609.375 tokens remaining (812.5 * 0.75)

// 5. Full Exit (close remaining)
let final_sig = positions::close_position_direct(
    "mint123",
    "final_exit",
    ExitType::Full
).await?;
// Position: CLOSED, semaphore permit released

// Query position state at any time
let pnl = positions::calculate_position_pnl("mint123", current_price).await?;
// Returns: {
//   unrealized_pnl: 0.01 SOL,
//   realized_pnl: 0.02 SOL,
//   total_pnl: 0.03 SOL,
//   remaining_tokens: 609,
//   exited_tokens: 1015
// }
```

---

## 9. Trader Module Integration

### Current (Stubbed)

```rust
// src/trader/execution/sell.rs
pub async fn execute_sell(decision: &TradeDecision) -> Result<TradeResult, String> {
    // TODO: Integrate with positions/swaps modules
    Err("Not implemented".to_string())
}
```

### Enhanced (Real Implementation)

```rust
// src/trader/execution/sell.rs
pub async fn execute_sell(decision: &TradeDecision) -> Result<TradeResult, String> {
    let mint = &decision.mint;
    let reason = decision.reason.to_string();

    // Determine exit type from decision
    let exit_type = match decision.action {
        TradeAction::Sell => {
            // Check config for default partial exit behavior
            if with_config(|cfg| cfg.positions.partial_exit_enabled) {
                let default_pct = with_config(|cfg| cfg.positions.partial_exit_default_pct);
                ExitType::Partial { percentage: default_pct }
            } else {
                ExitType::Full
            }
        },
        TradeAction::PartialSell(pct) => ExitType::Partial { percentage: pct },
        TradeAction::ForceStop => ExitType::Full,  // Emergency exits are always full
        _ => return Err("Invalid action for sell execution".to_string()),
    };

    // Execute appropriate exit
    let result = match exit_type {
        ExitType::Full => {
            positions::close_position_direct(mint, &reason, exit_type).await
        },
        ExitType::Partial { percentage } => {
            positions::partial_close_position(mint, percentage, &reason).await
        },
    };

    // Build and return TradeResult
    match result {
        Ok(signature) => Ok(TradeResult::success(decision.clone(), signature)),
        Err(e) => Ok(TradeResult::failure(decision.clone(), e, 0)),
    }
}
```

---

## 10. Configuration Schema

### Current Config (Missing DCA/Partial)

```toml
[trader]
enabled = true
max_open_positions = 2
trade_size_sol = 0.005
min_profit_threshold_enabled = true
min_profit_threshold_percent = 2.0
```

### Enhanced Config (Full Support)

```toml
[trader]
enabled = true
max_open_positions = 2
trade_size_sol = 0.005

# DCA Configuration (NEW)
dca_enabled = false
dca_threshold_pct = -10.0      # Enter DCA when down 10%
dca_max_count = 2               # Max 2 additional entries
dca_size_percentage = 50.0      # 50% of initial size
dca_cooldown_minutes = 30       # Wait 30min between DCAs

[positions]
profit_extra_needed_sol = 0.0002
position_open_cooldown_secs = 5

# Partial Exit Configuration (NEW)
partial_exit_enabled = false
partial_exit_default_pct = 50.0  # Default to 50% exits
partial_exit_min_pct = 10.0      # Min 10% per exit
partial_exit_max_pct = 90.0      # Max 90% per exit

# Trailing Stop Configuration (NEW)
trailing_stop_enabled = false
trailing_stop_activation_pct = 10.0   # Activate after +10%
trailing_stop_distance_pct = 5.0      # Trail by 5%
```

---

## Conclusion

This document provides visual flows and examples for implementing partial sell and DCA support. Key takeaways:

1. **Semaphore Management** is CRITICAL - permits must NOT be released on partial exits
2. **Verification System** needs expected_exit_amount tracking
3. **Database Schema** requires new fields and history tables
4. **P&L Calculation** must handle weighted averages and split realized/unrealized
5. **API Changes** are backward compatible (full exit still works)

Next steps: Follow SYSTEMATIC_COMPLETION_PLAN.md for implementation order.
