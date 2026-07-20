# Transactions Module — Architecture

> ScreenerBot Solana Transaction Monitoring & Analysis — February 2026

---

## Table of Contents

1. [Overview](#1-overview)
2. [File Structure](#2-file-structure)
3. [Core Types](#3-core-types)
4. [WebSocket Monitor](#4-websocket-monitor)
5. [Transaction Processor](#5-transaction-processor)
6. [Analyzer Pipeline](#6-analyzer-pipeline)
7. [DEX Detection](#7-dex-detection)
8. [Fee Analysis](#8-fee-analysis)
9. [Pending Transaction Tracking](#9-pending-transaction-tracking)
10. [Retry & Recovery](#10-retry--recovery)
11. [Database Schema](#11-database-schema)
12. [Module Connections](#12-module-connections)

---

## 1. Overview

The Transactions module monitors wallet activity via WebSocket (`logsSubscribe`), fetches full transaction data, analyzes DEX swaps across 10+ protocols, tracks fees (including Jito MEV tips), and stores results in a multi-table SQLite database. It provides the data layer for position tracking, P&L calculations, and the dashboard transaction view.

The module is full-mode-only because it follows the configured wallet. Preview-mode dashboard
reads do not initialize it; token-detail transaction history therefore has a valid empty response
contract until wallet/RPC setup is complete.

**Key characteristics:**
- WebSocket monitoring via `logsSubscribe` (mentions wallet address)
- Multi-stage analyzer pipeline: classify → balance → DEX detect → P&L → ATA analysis
- 10+ DEX protocol recognition (Jupiter, Raydium, Orca, Meteora, PumpFun, etc.)
- Jito MEV tip detection (8 hardcoded tip addresses)
- Deferred retry queue (in-memory bounded cache) with backoff for temporary RPC/indexing failures
- 7-table SQLite schema with 14 performance indexes
- Pending transaction tracking via `Arc<Mutex<HashMap>>` + known signatures via bounded moka cache + DB persistence

**34 files across 4 subdirectories + root files**

---

## 2. File Structure

```
src/transactions/
├── mod.rs              # Module exports
├── types.rs            # Transaction, TransactionType, TokenTransfer, etc.
├── manager.rs          # Transaction manager coordination
├── fetcher.rs          # RPC fetch with retry (signatures, details)
├── verifier.rs         # Transaction verification
├── utils.rs            # GLOBAL_PENDING_TRANSACTIONS, helpers
├── websocket.rs        # WebSocket logsSubscribe connection
├── program_ids.rs      # DEX program IDs, Jito tip addresses
├── debug.rs            # Debug utilities
├── analyzer/           # Multi-stage transaction analysis
│   ├── mod.rs           # Analyzer orchestration
│   ├── classify.rs      # Transaction classification
│   ├── balance.rs       # SOL/token balance change extraction
│   ├── dex.rs           # DEX swap detection & parsing
│   ├── pnl.rs           # P&L calculation, fee breakdown
│   ├── patterns.rs      # Pattern recognition
│   └── ata.rs           # ATA (Associated Token Account) analysis
├── database/           # SQLite persistence
│   ├── schema.rs        # 7 CREATE TABLE statements + 14 indexes
│   ├── maintenance.rs   # Cleanup, retention
│   └── types.rs         # DB row types
├── processor/          # Transaction processing pipeline
│   ├── mod.rs           # Pipeline orchestration
│   ├── core.rs          # Core processing logic
│   ├── extraction.rs    # Data extraction from raw tx
│   ├── analysis.rs      # Analysis coordination
│   └── helpers.rs       # Processing helpers
└── service/            # TransactionsService (Service trait impl)
    ├── mod.rs           # Service definition
    ├── lifecycle.rs     # Start/stop lifecycle
    ├── websocket.rs     # WebSocket management
    ├── processing.rs    # Background processing loop (3s interval)
    ├── bootstrap.rs     # Historical backfill on startup
    ├── config.rs        # Service configuration
    └── health.rs        # Health reporting
```

---

## 3. Core Types

### Transaction (main struct)

```rust
pub struct Transaction {
    pub signature: String,
    pub slot: Option<u64>,
    pub block_time: Option<i64>,
    pub timestamp: DateTime<Utc>,
    pub status: TransactionStatus,
    pub transaction_type: TransactionType,
    pub direction: TransactionDirection,
    pub success: bool,
    pub error_message: Option<String>,
    pub fee_sol: f64,
    pub fee_lamports: Option<u64>,
    pub compute_units_consumed: Option<u64>,
    pub instructions_count: usize,
    pub accounts_count: usize,
    pub sol_balance_change: f64,
    pub wallet_lamport_change: i64,
    pub wallet_signed: bool,
    pub token_transfers: Vec<TokenTransfer>,
    pub raw_transaction_data: Option<serde_json::Value>,
    pub log_messages: Vec<String>,
    pub instructions: Vec<InstructionInfo>,
    pub instruction_info: Vec<InstructionInfo>,
    pub sol_balance_changes: Vec<SolBalanceChange>,
    pub token_balance_changes: Vec<TokenBalanceChange>,
    pub position_impact: Option<PositionImpact>,
    pub profit_calculation: Option<ProfitCalculation>,
    pub ata_analysis: Option<AtaAnalysis>,
    pub ata_operations: Vec<AtaOperation>,
    pub token_info: Option<TokenSwapInfo>,
    pub token_swap_info: Option<TokenSwapInfo>,
    pub calculated_token_price_sol: Option<f64>,
    pub token_symbol: Option<String>,
    pub token_decimals: Option<u8>,
    pub swap_pnl_info: Option<SwapPnLInfo>,
    pub analysis_duration_ms: Option<u64>,
    pub last_updated: DateTime<Utc>,
    pub cached_analysis: Option<CachedAnalysis>,
}
```

### TransactionType

```rust
pub enum TransactionType {
    Buy,
    Sell,
    Transfer,
    Compute,
    AtaOperation,
    Failed,
    Unknown,
    SwapSolToToken { token_mint: String, sol_amount: f64, token_amount: f64, router: String },
    SwapTokenToSol { token_mint: String, token_amount: f64, sol_amount: f64, router: String },
    SwapTokenToToken { from_mint: String, to_mint: String, from_amount: f64, to_amount: f64, router: String },
    SolTransfer { amount: f64, from: String, to: String },
    TokenTransfer { mint: String, amount: f64, from: String, to: String },
    AtaClose { recovered_sol: f64, token_mint: String },
    Other { description: String, details: String },
}
```

### TransactionStatus

```rust
pub enum TransactionStatus {
    Pending,
    Confirmed,
    Finalized,
    Failed(String),     // Contains error message
}
```

### Supporting Types

```rust
pub struct TokenTransfer {
    pub mint: String,
    pub amount: f64,
    pub from: String,
    pub to: String,
    pub program_id: String,
}

pub struct FeeBreakdown {
    pub base_fee: f64,
    pub priority_fee: f64,
    pub mev_tips: f64,          // Jito tips
    pub swap_fees: f64,
    pub rent_costs: f64,
    pub total_fees: f64,
}
```

---

## 4. WebSocket Monitor

**File:** `websocket.rs` + `service/websocket.rs`

Uses `logsSubscribe` (NOT `signatureSubscribe`) to watch all transactions mentioning the wallet:

```rust
// websocket.rs — subscription message
{
    "method": "logsSubscribe",
    "params": [
        { "mentions": [wallet_address] },
        { "commitment": "confirmed" }
    ]
}
```

**Connection lifecycle:**
- 5-second ping interval for keep-alive
- Automatic reconnection on WebSocket drop
- Parses log notifications for transaction signatures
- Feeds signatures to the processing pipeline

---

## 5. Transaction Processor

**Directory:** `processor/`

Processing pipeline for each detected signature:

```
New signature from WebSocket
├─ Fetch full transaction via RPC (with retry)
├─ Extract raw data (instructions, accounts, logs)
├─ Run analyzer pipeline:
│  ├─ classify.rs    → Determine TransactionType
│  ├─ balance.rs     → Extract SOL/token balance changes
│  ├─ dex.rs         → Detect DEX router, parse swap details
│  ├─ pnl.rs         → Calculate P&L, fee breakdown
│  ├─ patterns.rs    → Pattern recognition
│  └─ ata.rs         → ATA create/close operations
├─ Store in database (raw + processed tables)
├─ Emit event
└─ Update positions if swap detected
```

**Background loop** (`service/processing.rs`): Runs every 3 seconds (`NORMAL_CHECK_INTERVAL_SECS`), processes queued signatures, handles deferred retries, cleans expired pending transactions.

---

## 6. Analyzer Pipeline

**Directory:** `analyzer/`

### Classification (`classify.rs`)

Examines instruction program IDs and log messages to classify:
- Swap detection via DEX program IDs
- Transfer detection via System/Token programs
- ATA operations (create, close)
- Failed transaction handling

### Balance Analysis (`balance.rs`)

Extracts pre/post balance changes from transaction metadata:
- SOL balance changes per account
- Token balance changes per mint
- Wallet-specific impact calculation

### DEX Detection (`dex.rs`)

Identifies the DEX router used and extracts swap parameters.

### P&L Calculation (`pnl.rs`)

```rust
pub struct FeeBreakdown {
    pub base_fee: f64,
    pub priority_fee: f64,
    pub mev_tips: f64,
    pub swap_fees: f64,
    pub rent_costs: f64,
    pub total_fees: f64,
}
```

Separates base fees, priority fees, Jito MEV tips, DEX swap fees, and ATA rent costs.

---

## 7. DEX Detection

**File:** `program_ids.rs`

Recognizes 10+ DEX protocols by program ID:

| DEX | Variants |
|-----|----------|
| Jupiter | V6, V4, V3 |
| Raydium | CPMM, Legacy AMM, CLMM |
| Orca | Whirlpool, V1 |
| Meteora | DAMM, DLMM, DBC |
| PumpFun | AMM, Legacy |
| Moonshot | — |
| FluxBeam | — |
| GMGN | — |
| Lifinity | — |
| Aldrin | — |
| Serum | V1, V2 |
| OpenBook | — |
| Phoenix | — |

### Jito MEV Tip Detection

8 hardcoded Jito tip addresses in `program_ids.rs`:
- `is_mev_tip_address(pubkey) -> bool`
- Tips detected and excluded from swap amount calculations
- Tracked separately in `FeeBreakdown.mev_tips`

---

## 8. Fee Analysis

The analyzer separates transaction costs into categories:

| Fee Type | Source | Purpose |
|----------|--------|---------|
| Base fee | Transaction signature fee | Network cost |
| Priority fee | Compute budget instruction | Validator priority |
| MEV tips | Transfers to Jito tip addresses | MEV protection |
| Swap fees | DEX protocol fees | DEX cost |
| Rent costs | ATA creation rent | Account storage |

---

## 9. Pending Transaction Tracking

**File:** `utils.rs`

```rust
static GLOBAL_PENDING_TRANSACTIONS: LazyLock<Arc<Mutex<HashMap<String, DateTime<Utc>>>>> =
    LazyLock::new(|| Arc::new(Mutex::new(HashMap::new())));

const PENDING_MAX_AGE_SECS: u64 = 180;  // 3 minutes
```

Tracks signatures → timestamps. Expired entries cleaned up by the processing loop.

---

## 10. Retry & Recovery

### Signature Fetch Retry (`fetcher.rs`)

```
fetch_signatures_with_retry(address, options)
├─ Attempt 1: getSignaturesForAddress
├─ Attempt 2: 1000ms delay (configurable base)
├─ Attempt 3: 2000ms delay (exponential)
└─ Return error after max_retries (default: 3)
```

### Transaction Detail Retry (`fetcher.rs`)

```
fetch_transaction_details_with_retry(signature)
├─ Handles RPC indexing delays (tx not yet available)
├─ Exponential backoff between attempts
└─ Returns parsed transaction or error
```

### Deferred Retry Queue (`service/processing.rs`)

For transactions that fail analysis (e.g., missing data):
- Stored in-memory (`service/config.rs`):
  - `DEFERRED_RETRIES`: moka cache (max 1K entries, 5min TTL)
  - `DEFERRED_RETRY_KEYS`: DashSet for iteration (moka has no `iter()`)
- Backoff: linear (`current_delay_secs = base_delay_secs * attempts`)
  - Example for RPC indexing delays (base=5s): 5s → 10s → 15s
- Max attempts: 3

### Bootstrap Backfill (`service/bootstrap.rs`)

On startup, fetches recent historical transactions:
- Resume cursor stored in `bootstrap_state` table
- Fills gaps from any downtime

---

## 11. Database Schema

**Database:** `transactions.db`
**Schema version:** 4 (`database/schema.rs`)

### Tables (7)

| Table | Purpose |
|-------|---------|
| `raw_transactions` | Blockchain data (signature, slot, block_time, status, fee, raw JSON) |
| `processed_transactions` | Analysis results (type, direction, swap info, ATA ops, cached analysis) |
| `known_signatures` | Signature dedup tracking |
| `deferred_retries` | Retry metadata table (schema exists; runtime retry queue is in-memory) |
| `pending_transactions` | Pending tx tracking (signature, added_at, check_count) |
| `db_metadata` | Version/config tracking |
| `bootstrap_state` | Resume cursor for historical backfill |

**14 performance indexes** on: wallet_address, timestamp, status, type, signature, block_time.

---

## 12. Module Connections

```
transactions/
├── rpc/            ← getTransaction, getSignaturesForAddress, WebSocket
├── wallets/        ← Wallet address for logsSubscribe filter
├── positions/      ← Position updates on confirmed swaps
├── events/         ← Transaction event recording
├── config/         ← Service config, retry settings
├── database/       ← SQLite infrastructure (DbPreset::Hot)
├── tokens/         ← Token symbol/decimal lookup
└── errors/         ← Error types
```

| Caller | Usage |
|--------|-------|
| trader/executors | Position impact detection |
| positions | P&L recalculation from confirmed swaps |
| webserver | Transaction history + detail API |
| tools | Manual transaction lookup |
| events | Transaction event recording |
