# Positions Module Architecture

> ScreenerBot Position Management & Trade Lifecycle — February 2026

The Positions module manages the complete lifecycle of trading positions: opening trades, tracking in-memory state, verifying on-chain transactions, calculating profit/loss, handling partial exits and DCA (dollar-cost averaging), detecting phantom positions, and persisting all state to SQLite. It coordinates with the Pools module for pricing, the Swaps system for execution, and the Trader module for automated entry/exit decisions.

---

## Table of Contents

1. [Module Overview](#1-module-overview)
2. [Core Data Types](#2-core-data-types)
3. [State Machine](#3-state-machine)
4. [Global State](#4-global-state)
5. [Position Lifecycle](#5-position-lifecycle)
6. [Entry Operations](#6-entry-operations)
7. [Exit Operations](#7-exit-operations)
8. [Transaction Verification](#8-transaction-verification)
9. [Price Tracking](#9-price-tracking)
10. [Phantom Detection](#10-phantom-detection)
11. [Loss Detection](#11-loss-detection)
12. [Database Layer](#12-database-layer)
13. [Metrics](#13-metrics)
14. [API Surface](#14-api-surface)
15. [Configuration](#15-configuration)
16. [Integration Points](#16-integration-points)
17. [Performance Patterns](#17-performance-patterns)
18. [Error Handling](#18-error-handling)

---

## 1. Module Overview

### Purpose

The Positions module is the trade execution and tracking core of ScreenerBot. It:

- Opens positions by executing swap transactions on Solana
- Tracks all position state in-memory with per-mint locking
- Verifies transactions on-chain with retry and backoff
- Calculates realized and unrealized PnL in SOL
- Supports partial exits (sell percentage) and DCA (add to position)
- Detects phantom positions (zero wallet balance)
- Persists all state to SQLite with token snapshots
- Provides real-time price updates for open positions (1s interval)

### File Structure

```
src/positions/
├── mod.rs              — Public API re-exports
├── types.rs            — Position, EntryRecord, ExitRecord structs
├── state.rs            — Global state containers, locking, semaphores
├── transitions.rs      — PositionTransition enum, state machine
├── operations.rs       — Entry/exit/DCA/partial operations
├── apply.rs            — Transition application logic
├── helpers.rs          — PnL calculation, indexing, snapshots
├── queue.rs            — Verification queue with backoff
├── verifier.rs         — On-chain transaction verification
├── worker.rs           — Background verification worker
├── price_updater.rs    — Real-time price updates (1s)
├── metrics.rs          — Verification and proceeds metrics
├── loss_detection.rs   — Loss-based token blacklisting
├── tracking.rs         — Position price tracking DB updates
└── database/
    ├── mod.rs          — Module exports, initialization
    ├── operations.rs   — SQLite CRUD operations (1751 lines)
    ├── types.rs        — Schema definitions, PositionState, TokenSnapshot
    ├── convenience.rs  — Convenience query wrappers
    └── global.rs       — Global singleton DB access
```

**Total:** 18 Rust source files, ~9,600 lines of code.

### Key Capabilities

- **Per-mint locking** — Concurrent position operations without races
- **Global position semaphore** — Hard limit on max simultaneous open positions
- **20-retry verification** — Transactions verified with exponential backoff
- **Phantom detection** — Automatic detection of positions with zero wallet balance
- **Partial exits** — Sell any percentage of a position
- **DCA support** — Add SOL to existing positions, recalculate average entry
- **Synthetic exits** — Graceful closure when exit transactions permanently fail
- **Token snapshots** — Market data snapshot at open and close time

---

## 2. Core Data Types

### Position (`types.rs`)

The central data structure (~40 fields):

```rust
pub struct Position {
    // Identity
    pub id: Option<i64>,              // Database ID
    pub mint: String,                 // Token mint address
    pub symbol: String,               // Token symbol
    pub name: String,                 // Token name
    pub position_type: String,        // "buy" or "sell"

    // Entry
    pub entry_price: f64,             // Initial entry price (SOL)
    pub entry_time: DateTime<Utc>,    // Entry timestamp
    pub entry_size_sol: f64,          // Initial SOL spent
    pub total_size_sol: f64,          // Total SOL including DCA
    pub entry_transaction_signature: Option<String>,
    pub entry_fee_lamports: u64,

    // Exit
    pub exit_price: Option<f64>,      // Final exit price
    pub exit_time: Option<DateTime<Utc>>,
    pub exit_transaction_signature: Option<String>,
    pub exit_fee_lamports: u64,
    pub sol_received: Option<f64>,    // Total SOL received from exits
    pub closed_reason: Option<String>,

    // Token accounting
    pub token_amount: Option<u64>,    // Initial tokens purchased (raw)
    pub remaining_token_amount: Option<u64>,  // After partial exits (raw)
    pub total_exited_amount: u64,     // Cumulative tokens sold (raw)

    // Prices and tracking
    pub effective_entry_price: Option<f64>,   // Verified on-chain price
    pub effective_exit_price: Option<f64>,    // Verified on-chain price
    pub current_price: f64,           // Live market price
    pub current_price_updated: Option<DateTime<Utc>>,
    pub price_highest: f64,           // All-time high since entry
    pub price_lowest: f64,            // All-time low since entry

    // PnL
    pub pnl: f64,                     // Realized PnL (SOL)
    pub pnl_percent: f64,             // Realized PnL %
    pub unrealized_pnl: f64,          // Mark-to-market PnL (SOL)
    pub unrealized_pnl_percent: f64,  // Mark-to-market PnL %

    // Verification
    pub transaction_entry_verified: bool,
    pub transaction_exit_verified: bool,

    // Phantom detection
    pub phantom_remove: bool,
    pub phantom_confirmations: i32,
    pub phantom_first_seen: Option<DateTime<Utc>>,
    pub synthetic_exit: bool,         // True if exit was synthetic

    // DCA
    pub dca_count: i32,               // Number of DCA entries
    pub average_entry_price: f64,     // Weighted average entry
    pub last_dca_time: Option<DateTime<Utc>>,

    // Partial exits
    pub partial_exit_count: i32,
    pub average_exit_price: f64,      // Weighted average exit

    // Targets
    pub profit_target_min: f64,       // Min ROI % target
    pub profit_target_max: f64,       // Max ROI % target
    pub liquidity_tier: String,       // Token liquidity classification
}
```

### EntryRecord (`types.rs`)

Individual entry transaction (initial + DCA):

```rust
pub struct EntryRecord {
    pub id: Option<i64>,
    pub position_id: i64,
    pub timestamp: DateTime<Utc>,
    pub amount: u64,                  // Tokens bought (raw)
    pub price: f64,                   // Entry price
    pub sol_spent: f64,
    pub transaction_signature: String,
    pub is_dca: bool,
    pub fees_lamports: u64,
}
```

### ExitRecord (`types.rs`)

Individual exit transaction:

```rust
pub struct ExitRecord {
    pub id: Option<i64>,
    pub position_id: i64,
    pub timestamp: DateTime<Utc>,
    pub amount: u64,                  // Tokens sold (raw)
    pub price: f64,                   // Exit price
    pub sol_received: f64,
    pub transaction_signature: String,
    pub is_partial: bool,
    pub percentage: f64,              // % of position sold
    pub fees_lamports: u64,
}
```

### PositionState (`database/types.rs`)

```rust
pub enum PositionState {
    Open,          // Active, no exit submitted
    Closing,       // Exit submitted, not verified
    Closed,        // Exit verified
    ExitPending,   // Exit in verification queue
    ExitFailed,    // Exit failed, needs retry
    Phantom,       // Zero wallet balance detected
    Reconciling,   // Auto-healing phantom
}
```

### TokenSnapshot (`database/types.rs`)

Market data captured at position open/close:

```rust
pub struct TokenSnapshot {
    pub position_id: i64,
    pub snapshot_type: String,  // "opening" or "closing"
    pub mint: String,
    pub symbol: String, pub name: String,
    pub price_sol: f64, pub price_usd: f64,
    pub fdv: f64, pub market_cap: f64,
    pub liquidity_usd: f64,
    pub volume_h24: f64, pub volume_h6: f64, pub volume_h1: f64, pub volume_m5: f64,
    pub txns_h24_buys: i64, pub txns_h24_sells: i64,
    pub txns_h6_buys: i64, pub txns_h6_sells: i64,
    pub txns_h1_buys: i64, pub txns_h1_sells: i64,
    pub txns_m5_buys: i64, pub txns_m5_sells: i64,
    pub price_change_h24: f64, pub price_change_h6: f64,
    pub price_change_h1: f64, pub price_change_m5: f64,
    pub description: String, pub image: String,
    pub website: String, pub twitter: String, pub telegram: String,
    pub freshness_score: f64,  // 0-100 data age quality
}
```

---

## 3. State Machine

### PositionTransition (`transitions.rs`)

```rust
pub enum PositionTransition {
    EntryVerified { mint, position_id, token_amount, effective_price, fees },
    ExitVerified { mint, position_id, sol_received, effective_price, fees },
    ExitFailedClearForRetry { mint, position_id },
    ExitPermanentFailureSynthetic { mint, position_id },
    RemoveOrphanEntry { mint, position_id },
    UpdatePriceTracking { mint, current_price },
    PartialExitSubmitted { mint, position_id, signature, percentage, amount },
    PartialExitVerified { mint, position_id, sol_received, price, fees },
    PartialExitFailed { mint, position_id, signature },
    DcaSubmitted { mint, position_id, signature, sol_amount },
    DcaVerified { mint, position_id, token_amount, price, fees },
    DcaFailed { mint, position_id, signature },
}
```

**Methods:**
- `position_id()` → `Option<i64>`
- `is_terminal()` → bool — Whether this transition closes the position
- `requires_db_update()` → bool — Whether DB persistence needed

### State Diagram

```
                    ┌──────────────┐
                    │  No Position │
                    └──────┬───────┘
                           │ open_position_direct()
                           ▼
                    ┌──────────────┐
              ┌─────│     Open     │─────┐
              │     └──────┬───────┘     │
              │            │             │
    DCA ◄─────┤     Price Updates       Partial Exit
    (add SOL) │     (1s interval)       (sell %)
              │            │             │
              │            │             │
              └─────►      │      ◄──────┘
                           │ close_position_direct()
                           ▼
                    ┌──────────────┐
                    │   Closing    │
                    └──────┬───────┘
                           │ verify_transaction()
                    ┌──────┴───────┐
                    │              │
                    ▼              ▼
             ┌──────────┐  ┌────────────┐
             │  Closed  │  │ ExitFailed │
             └──────────┘  └──────┬─────┘
                                  │ retry or synthetic
                                  ▼
                           ┌──────────────┐
                           │  Synthetic   │
                           │    Exit      │
                           └──────────────┘

    Phantom Detection (parallel path):
    ┌──────┐  zero balance  ┌─────────┐  confirm  ┌─────────────┐
    │ Open │ ─────────────► │ Phantom │ ────────► │ Reconciling │
    └──────┘                └─────────┘           └─────────────┘
```

---

## 4. Global State

### Static Containers (`state.rs`)

```rust
// Core position storage
pub static POSITIONS: LazyLock<RwLock<Vec<Position>>>
pub static SIG_TO_MINT_INDEX: LazyLock<RwLock<HashMap<String, String>>>
pub static MINT_TO_POSITION_INDEX: LazyLock<RwLock<HashMap<String, usize>>>

// Locking
static POSITION_LOCKS: LazyLock<RwLock<HashMap<String, Arc<Mutex<()>>>>>

// Pending operations
static PENDING_PARTIAL_EXITS: LazyLock<RwLock<HashMap<String, u32>>>
static PENDING_PARTIAL_EXIT_DETAILS: LazyLock<RwLock<HashMap<String, PendingPartialExit>>>
static PENDING_DCA_SWAPS: LazyLock<RwLock<HashMap<String, PendingDcaSwap>>>
static PENDING_OPEN_SWAPS: LazyLock<RwLock<HashMap<String, DateTime<Utc>>>>

// Capacity control
static GLOBAL_POSITION_SEMAPHORE: OnceLock<tokio::sync::Semaphore>
pub static LAST_OPEN_TIME: LazyLock<RwLock<Option<DateTime<Utc>>>>

// Constants
pub const PENDING_OPEN_TTL_SECS: i64 = 120;
```

### Index Strategy

Two auxiliary indexes avoid O(n) scans on the position vector:
- `SIG_TO_MINT_INDEX`: Transaction signature → Mint (for verification lookup)
- `MINT_TO_POSITION_INDEX`: Mint → Vec index (for position lookup by token)

Both are updated atomically with position mutations.

### Per-Mint Locking

```rust
pub struct PositionLockGuard {
    mint: String,
    _guard: tokio::sync::OwnedMutexGuard<()>,
}

pub async fn acquire_position_lock(mint: &str) -> PositionLockGuard
```

Each token mint has its own `Arc<Mutex<()>>`. Operations on a specific position acquire this lock first, allowing concurrent operations on different positions.

### Global Position Semaphore

```rust
pub fn init_global_position_semaphore(max_positions: usize)
pub async fn acquire_global_position_permit(mint: &str) -> Result<OwnedSemaphorePermit, String>
pub fn release_global_position_permit()
```

Enforces a hard limit on the number of simultaneous open positions (from config).

---

## 5. Position Lifecycle

```
1. OPEN:  open_position_direct(mint) / open_position_with_size(mint, sol)
   → Acquire global semaphore permit
   → Acquire per-mint lock
   → Execute buy swap
   → Create Position struct
   → Add to POSITIONS + indexes
   → Enqueue entry verification
   → Save to DB

2. VERIFY ENTRY:  verify_transaction(item) [background worker]
   → Check on-chain transaction status
   → Get actual token amount received
   → Apply EntryVerified transition
   → Update effective_entry_price

3. TRADE:  (position is open, price tracking active)
   → Price updates every 1s
   → High/low watermarks tracked
   → Unrealized PnL calculated

4. OPTIONAL - DCA:  add_to_position(mint, sol)
   → Execute additional buy swap
   → Recalculate average_entry_price
   → Update token_amount, dca_count

5. OPTIONAL - PARTIAL EXIT:  partial_close_position(mint, percentage)
   → Execute sell swap for percentage of tokens
   → Track remaining_token_amount
   → Record ExitRecord (is_partial=true)
   → Update average_exit_price

6. CLOSE:  close_position_direct(mint, reason)
   → Acquire per-mint lock
   → Execute sell swap for ALL remaining tokens
   → Set exit_price, exit_time
   → Enqueue exit verification
   → Save to DB

7. VERIFY EXIT:  verify_transaction(item) [background worker]
   → Check on-chain transaction status
   → Get actual SOL received
   → Apply ExitVerified transition
   → Calculate final PnL
   → Release global semaphore permit
```

---

## 6. Entry Operations

### `open_position_direct(token_mint)` / `open_position_with_size(token_mint, sol)`

1. Check no existing open position for mint
2. Acquire global position permit (respects max_positions limit)
3. Get current price via `get_price_with_api_fallback()`
4. Execute buy swap (via Jupiter or direct pool swap)
5. Create Position with initial values
6. Add to global state and indexes
7. Enqueue entry verification
8. Record EntryRecord to database
9. Record "opening" TokenSnapshot

### `add_to_position(token_mint, dca_amount_sol)` (DCA)

1. Verify position exists and is open
2. Execute additional buy swap
3. Update: `total_size_sol += dca_amount_sol`
4. Recalculate `average_entry_price` (weighted)
5. Increment `dca_count`
6. Record EntryRecord (is_dca=true)
7. Enqueue DCA verification

### Price Fallback

```rust
pub async fn get_price_with_api_fallback(token_mint: &str) -> Option<(PriceResult, PriceSource)>
```

Tries: Pool price → DexScreener API → GeckoTerminal API → None

---

## 7. Exit Operations

### `close_position_direct(token_mint, exit_reason)`

1. Acquire per-mint lock
2. Verify no pending exit transaction
3. Get all token accounts (associated + auxiliary)
4. Sell ALL tokens across all accounts
5. Record exit_price and exit_time
6. Enqueue exit verification
7. Record "closing" TokenSnapshot

### `partial_close_position(token_mint, sell_percentage)`

1. Calculate token amount: `remaining_token_amount * sell_percentage / 100`
2. Execute sell swap for calculated amount
3. Register pending partial exit metadata
4. Update remaining_token_amount
5. Increment partial_exit_count
6. Record ExitRecord (is_partial=true)

### Synthetic Exit

When exit transaction permanently fails after max retries:
- Position marked with `synthetic_exit = true`
- Estimated SOL received calculated from last known price
- Position state set to Closed
- Global semaphore permit released

---

## 8. Transaction Verification

### VerificationItem (`queue.rs`)

```rust
pub struct VerificationItem {
    pub signature: String,
    pub mint: String,
    pub position_id: Option<i64>,
    pub kind: VerificationKind,  // Entry or Exit
    pub created_at: DateTime<Utc>,
    pub last_attempt_at: Option<DateTime<Utc>>,
    pub next_retry_at: Option<DateTime<Utc>>,
    pub attempts: u32,           // Max 20
    pub expiry_height: Option<u64>,  // Slot-based expiry
    pub is_partial_exit: bool,
    pub expected_exit_amount: Option<f64>,
    pub requested_exit_percentage: Option<f64>,
    pub is_dca: bool,
}
```

### Verification Worker (`worker.rs`)

Background worker processes verification queue:

```
const VERIFICATION_BATCH_SIZE: usize = 10;

Loop:
  1. Take up to 10 items from queue
  2. For each item:
     → verify_transaction(item)
     → Match outcome: Confirmed/Failed/Pending/Expired
  3. Confirmed → apply_transition()
  4. Failed (< max retries) → re-enqueue with backoff
  5. Failed (>= 20 retries) → synthetic exit
  6. Pending → re-enqueue
  7. Sleep between batches
```

### Verifier (`verifier.rs`)

```rust
const TOKEN_ACCOUNTS_THROTTLE_SECS: i64 = 5;

pub async fn verify_transaction(item: &VerificationItem) -> VerificationOutcome
```

Verification checks:
1. Get transaction status from RPC (`getTransaction`)
2. Check if transaction succeeded (no error)
3. For entries: extract token balance changes (pre/post)
4. For exits: extract SOL balance changes
5. Return actual amounts received

### Retry Strategy

- Maximum 20 attempts
- Exponential backoff between retries
- Slot-based expiry for very old transactions
- After permanent failure → synthetic exit

---

## 9. Price Tracking

### Price Updater (`price_updater.rs`)

```rust
const UPDATE_INTERVAL_SECS: u64 = 1;
const API_PRICE_MAX_AGE_SECS: i64 = 5;

pub async fn start_price_updater(shutdown: watch::Receiver<bool>)
```

Runs every 1 second:
1. Get all open positions
2. For each position:
   - Get latest price from pool cache
   - If pool price unavailable or stale (>5s), try API fallback
   - Update `current_price` and `current_price_updated`
   - Update `price_highest` / `price_lowest` watermarks
   - Calculate `unrealized_pnl` and `unrealized_pnl_percent`

### Tracking (`tracking.rs`)

```rust
pub async fn update_position_tracking(mint: &str, current_price: f64) -> bool
```

Persists price tracking data to the `position_tracking` table for historical analysis.

---

## 10. Phantom Detection

Phantom positions are positions where the wallet has zero token balance despite the position being "open." This can happen when:
- Tokens were transferred out manually
- A DEX aggregated trades differently than expected
- Token program froze the account

**Detection flow:**
1. During price update, check token account balance
2. If balance is zero → set `phantom_remove = true`
3. Increment `phantom_confirmations` on each check
4. After threshold confirmations → transition to Reconciling
5. Attempt auto-recovery or create synthetic exit

**Fields:**
- `phantom_remove: bool` — Phantom detected
- `phantom_confirmations: i32` — Number of consecutive zero-balance checks
- `phantom_first_seen: Option<DateTime<Utc>>` — First detection time

---

## 11. Loss Detection

### `process_position_loss_detection(position)` (`loss_detection.rs`)

When a position is closed at a loss:
1. Check if loss exceeds threshold (`get_loss_thresholds()`)
2. If loss blacklisting enabled (`is_loss_blacklisting_enabled()`)
3. Blacklist the token to prevent re-entry
4. Log the blacklist action

This prevents the bot from repeatedly buying tokens that consistently lose money.

---

## 12. Database Layer

### Schema (`database/types.rs`)

**7 tables:**

| Table | Purpose |
|-------|---------|
| `positions` | Main position records |
| `position_states` | State change history |
| `position_exits` | Exit transaction records |
| `position_entries` | Entry transaction records (initial + DCA) |
| `position_tracking` | Price tracking snapshots |
| `position_metadata` | Key-value metadata store |
| `token_snapshots` | Market data at open/close |

**Indexes (12):**
- `positions`: wallet_address, mint, entry_time, exit_time, mint+exit_time, entry_signature, exit_signature, state composite
- `position_states`: position_id+changed_at, state+changed_at
- `position_tracking`: position_id+tracked_at, price+tracked_at
- `token_snapshots`: position_id+snapshot_type

### Operations (`database/operations.rs`)

1751 lines of SQLite CRUD operations:

| Category | Key Functions |
|----------|--------------|
| Position CRUD | insert, update, get_by_id, get_by_mint, get_open, get_closed |
| Entry records | insert_entry, get_entries_for_position |
| Exit records | insert_exit, get_exits_for_position |
| State history | record_state_change, get_state_history |
| Token snapshots | save_snapshot, get_snapshots |
| Metadata | set_metadata, get_metadata |
| Queries | get_total_pnl, get_win_rate, get_position_count |

### Convenience (`database/convenience.rs`)

High-level query wrappers for common patterns.

---

## 13. Metrics

### Verification Metrics (`metrics.rs`)

Tracks verification performance:
- Entries verified / failed
- Exits verified / failed
- Average verification time
- Queue depth

### Proceeds Tracking

Tracks total trading performance:
- Total SOL spent (entries)
- Total SOL received (exits)
- Net proceeds

---

## 14. API Surface

### Position Operations

| Function | Signature | Purpose |
|----------|-----------|---------|
| `open_position_direct(mint)` | `async -> Result<String, String>` | Open with default size |
| `open_position_with_size(mint, sol)` | `async -> Result<String, String>` | Open with custom size |
| `close_position_direct(mint, reason)` | `async -> Result<String, String>` | Close full position |
| `partial_close_position(mint, pct)` | `async -> Result<String, String>` | Partial exit |
| `add_to_position(mint, sol)` | `async -> Result<String, String>` | DCA entry |
| `update_position_price(mint, price)` | `async -> Result<(), String>` | Update current price |

### State Management

| Function | Purpose |
|----------|---------|
| `acquire_position_lock(mint)` | Per-mint mutex lock |
| `acquire_global_position_permit(mint)` | Global semaphore |
| `add_position(position)` | Add to global state |
| `update_position_state(mint, updater)` | Update position |
| `get_position_by_mint(mint)` | Read position |
| `get_open_positions()` | Open positions (excludes archived) |
| `get_closed_positions()` | Closed positions (excludes archived) |
| `get_archived_positions()` | Archived positions only |
| `get_all_positions()` | All positions |

### Archival & Removal

A position can be removed from the dashboard in two ways. **Archive** is a
reversible flag (`archived` / `archived_at` columns on `positions`) that hides the
row from the Open/Closed lists and surfaces it in the **Archived** sub-tab.
**Delete** is a permanent hard-delete that cascades (via `ON DELETE CASCADE`) only
to the position's own child tables — `position_states`, `position_exits`,
`position_entries`, `position_tracking`, `token_snapshots`. Transactions and tokens
live in **separate SQLite databases** and are never touched.

| Function / Endpoint | Purpose |
|---------------------|---------|
| `set_position_archived_db(id, bool)` / `set_position_archived_in_memory(id, bool)` | Toggle archive flag (DB + memory) |
| `delete_position_by_id(id)` / `remove_position_by_id(id)` | Hard delete (DB + memory) |
| `delete_archived_positions()` | Bulk hard-delete all archived |
| `POST /api/positions/:id/archive` | Archive a position |
| `POST /api/positions/:id/unarchive` | Restore a position |
| `DELETE /api/positions/:id` | Permanently delete a position |
| `DELETE /api/positions/archived` | Permanently delete all archived |
| `GET /api/positions?status=archived` | List archived positions |

**Trade-slot accounting:** archiving or deleting a position that is still *open*
frees its global semaphore permit (`release_global_position_permit`) so a new
position can be opened; unarchiving an open position reclaims a permit
(`try_consume_global_position_permit`). Removing does **not** sell — tokens stay
in the wallet.

### Token Activity History

`GET /api/positions/{key}/activity` lazily builds the selected token's complete
wallet history. It merges entry/exit records, pending position operations,
on-chain transaction details, position state changes, every historical position
for the mint, and wallet-only transactions that were not claimed by a position.
Token amounts are converted to whole-token UI units by the server.

The Position Details Activity tab presents this response as expandable trading
rounds. The current or newest round opens by default; events within each round
remain chronological, meaningful state changes share the same timeline, and
transactions outside ScreenerBot live in a separate wallet-only chapter. The
collapsed event row contains the human-readable action and outcome; accounting,
signature, routing, fees and transfer details use progressive disclosure.

### System

| Function | Purpose |
|----------|---------|
| `initialize_positions_system()` | Init DB, load positions, rehydrate pending |
| `start_positions_manager_service(shutdown)` | Start verification worker |
| `start_price_updater(shutdown)` | Start 1s price updater |

---

## 15. Configuration

Position-related config options:

| Field | Purpose |
|-------|---------|
| `max_open_positions` | Hard limit on simultaneous positions (semaphore) |
| `trade_size_sol` | Default SOL per trade |
| `slippage_bps` | Slippage tolerance (basis points) |
| `profit_target_min/max` | ROI % targets |
| `loss_blacklist_enabled` | Enable loss-based blacklisting |
| `loss_blacklist_threshold` | Loss % threshold for blacklisting |

---

## 16. Integration Points

### Pools → Positions

- Pool prices feed `price_updater` (1s interval)
- `get_price_with_api_fallback()` uses pool cache first

### Swaps → Positions

- `operations.rs` calls swap execution for buy/sell
- Jupiter aggregator for most trades
- Direct pool swaps for supported DEXes

### Trader → Positions

- Trader entry monitors call `open_position_direct()`
- Trader exit monitors call `close_position_direct()`
- Auto-sell strategies trigger `partial_close_position()`

### Tokens → Positions

- Token data provides symbol/name for position display
- Security data influences position decisions

### Dashboard → Positions

- WebSocket API exposes position state to frontend
- Open positions, closed history, PnL metrics

---

## 17. Performance Patterns

### Locking Granularity

Three-level locking prevents contention:
1. **Global semaphore** — Caps total positions (coarse)
2. **Per-mint mutex** — Serializes per-token operations (fine)
3. **RwLock on POSITIONS vec** — Short-held for read/write access

### Index Lookups

- `MINT_TO_POSITION_INDEX`: O(1) lookup by token mint
- `SIG_TO_MINT_INDEX`: O(1) lookup by transaction signature
- Avoids scanning the entire positions vector

### Pending Operation Persistence

Pending partial exits and DCA swaps are persisted to `position_metadata` table:
- On crash, `rehydrate_pending_dca_swaps()` and `rehydrate_pending_partial_exits()` restore state
- Constants: `PENDING_DCA_METADATA_KEY`, `PENDING_PARTIAL_EXIT_METADATA_KEY`

### Batch Verification

Verification worker processes up to 10 items per batch, preventing RPC spam while maintaining throughput.

---

## 18. Error Handling

### Error Patterns

- **Swap failure:** Return error to caller, release semaphore permit
- **Verification failure:** Re-enqueue with backoff (up to 20 retries)
- **Permanent failure:** Synthetic exit to close position gracefully
- **DB failure:** Log error, continue operating from memory
- **Price unavailable:** Skip update, retry on next 1s tick
- **Phantom detected:** Multi-confirmation before action (prevent false positives)

### Error Propagation

All public async functions return `Result<String, String>`:
- `Ok(signature)` — Transaction signature on success
- `Err(reason)` — Human-readable error message

Position operations never panic — all errors are captured and returned.
