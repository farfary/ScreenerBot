# Transactions Module — Architecture

> ScreenerBot Solana Transaction Monitoring & Execution — February 2026

---

## Table of Contents

1. [Overview](#1-overview)
2. [File Structure](#2-file-structure)
3. [Core Types](#3-core-types)
4. [Transaction Monitor](#4-transaction-monitor)
5. [Transaction Builder](#5-transaction-builder)
6. [Signing & Sending](#6-signing--sending)
7. [Confirmation Tracking](#7-confirmation-tracking)
8. [Parser Pipeline](#8-parser-pipeline)
9. [Database Schema](#9-database-schema)
10. [Module Connections](#10-module-connections)

---

## 1. Overview

The Transactions module handles Solana transaction lifecycle: monitoring wallet activity, building swap/transfer transactions, signing, sending, and confirmation tracking. It also parses incoming DEX transactions to detect buys, sells, liquidity events, and transfers.

**Key characteristics:**
- WebSocket-based transaction monitoring (signatureSubscribe)
- Modular parser pipeline (Raydium, Orca, Jupiter, transfers)
- Priority fee estimation
- Retry with exponential backoff on send failure
- Confirmation tracking with timeout handling
- Transaction history stored in SQLite

**34 files, ~15,080 lines across 9 subdirectories**

---

## 2. File Structure

```
src/transactions/
├── mod.rs              # Module exports
├── types.rs            # TransactionRecord, TransactionType, etc.
├── service.rs          # Transaction monitoring service
├── monitor/            # WebSocket subscription management
│   ├── subscription.rs # signatureSubscribe handler
│   └── processor.rs    # Event processing
├── builder/            # Transaction construction
│   ├── swap.rs         # Swap transaction builder
│   ├── transfer.rs     # SOL/SPL transfer builder
│   ├── priority.rs     # Priority fee estimation
│   └── instructions.rs # Instruction helpers
├── signing/            # Transaction signing
│   └── signer.rs       # Keypair signing, versioned TX support
├── sender/             # Transaction submission
│   ├── sender.rs       # Send with retry
│   └── confirmation.rs # Confirmation polling
├── parsers/            # DEX transaction parsers
│   ├── router.rs       # Parser router (dispatch to correct parser)
│   ├── raydium.rs      # Raydium AMM/CLMM parser
│   ├── orca.rs         # Orca Whirlpool parser
│   ├── jupiter.rs      # Jupiter aggregator parser
│   ├── transfer.rs     # SOL/SPL transfer parser
│   └── common.rs       # Shared parsing utilities
├── database.rs         # SQLite schema and queries
└── pending.rs          # GLOBAL_PENDING_TRANSACTIONS tracking
```

---

## 3. Core Types

### TransactionRecord

```rust
pub struct TransactionRecord {
    pub id: i64,
    pub signature: String,
    pub transaction_type: TransactionType,
    pub status: TransactionStatus,
    pub token_mint: Option<String>,
    pub token_symbol: Option<String>,
    pub amount_sol: Option<f64>,
    pub amount_token: Option<f64>,
    pub price_per_token: Option<f64>,
    pub wallet_address: String,
    pub pool_address: Option<String>,
    pub dex: Option<String>,                  // raydium, orca, jupiter
    pub priority_fee_lamports: Option<u64>,
    pub compute_units: Option<u32>,
    pub error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub confirmed_at: Option<DateTime<Utc>>,
    pub slot: Option<u64>,
}
```

### Enums

| Enum | Variants |
|------|----------|
| `TransactionType` | `Buy`, `Sell`, `Transfer`, `AddLiquidity`, `RemoveLiquidity`, `CreatePool`, `Unknown` |
| `TransactionStatus` | `Pending`, `Confirmed`, `Failed`, `Expired`, `TimedOut` |
| `DexType` | `Raydium`, `Orca`, `Jupiter`, `Unknown` |

### Pending Transaction

```rust
pub struct PendingTransaction {
    pub signature: String,
    pub sent_at: Instant,
    pub transaction_type: TransactionType,
    pub token_mint: Option<String>,
    pub retry_count: u32,
}
```

Global tracking: `GLOBAL_PENDING_TRANSACTIONS: DashMap<String, PendingTransaction>` — bounded by transaction lifecycle (~100 entries max).

---

## 4. Transaction Monitor

Watches the main wallet for incoming/outgoing transactions:

```
Transaction Monitor
├─ Subscribe to wallet signature updates (WebSocket)
├─ On new signature:
│  ├─ Fetch full transaction (RPC)
│  ├─ Route to parser pipeline
│  ├─ Classify (buy/sell/transfer/liquidity)
│  ├─ Update positions if relevant
│  └─ Store in DB + emit event
└─ Reconnect on WebSocket drop
```

---

## 5. Transaction Builder

### Swap Builder (`builder/swap.rs`)

Constructs swap transactions from router responses:

```
SwapBuilder::build(params)
├─ Get recent blockhash
├─ Create transaction message
├─ Add priority fee instruction (if configured)
├─ Add compute budget instruction
├─ Add swap instructions (from router)
├─ Add cleanup instructions (close ATA if needed)
└─ Return unsigned VersionedTransaction
```

### Priority Fee Estimation (`builder/priority.rs`)

| Strategy | Source | Purpose |
|----------|--------|---------|
| `Fixed` | Config value | Predictable cost |
| `Recent` | `getRecentPrioritizationFees` RPC | Market-based |
| `Percentile` | Recent fees at configured percentile | Balanced |

---

## 6. Signing & Sending

### Signing

```rust
sign_transaction(tx: VersionedTransaction, keypair: &Keypair) -> VersionedTransaction
```

Supports both legacy and v0 (versioned) transactions.

### Sending

```
send_with_retry(signed_tx, config)
├─ Attempt 1: send_transaction(skipPreflight=true)
│  ├─ Success → track as pending
│  └─ Failure → check if retryable
├─ Attempt 2: 500ms delay, resend
├─ Attempt 3: 1000ms delay, resend
└─ Max retries exceeded → mark as failed
```

| Config | Default | Purpose |
|--------|---------|---------|
| `max_retries` | 3 | Send retry attempts |
| `skip_preflight` | true | Skip simulation (faster) |
| `commitment` | `confirmed` | Confirmation level |

---

## 7. Confirmation Tracking

```
confirm_transaction(signature, timeout)
├─ Poll getSignatureStatuses every 2s
├─ Status = confirmed/finalized → success
├─ Status = error → failed (parse error)
├─ Timeout (60s default) → mark timed out
└─ Update TransactionRecord + emit event
```

---

## 8. Parser Pipeline

### Router (`parsers/router.rs`)

```
parse_transaction(tx_data)
├─ Extract program IDs from instructions
├─ Match known DEX programs:
│  ├─ Raydium AMM (675kPX...) → raydium.rs
│  ├─ Raydium CLMM (CAMMCzo...) → raydium.rs
│  ├─ Orca Whirlpool (whirLb...) → orca.rs
│  ├─ Jupiter Aggregator (JUP...) → jupiter.rs
│  └─ System/Token Program → transfer.rs
└─ Return ParsedTransaction
```

### DEX Parsers

| Parser | Programs | Extracts |
|--------|----------|----------|
| Raydium | AMM, CLMM | Swap direction, amounts, pool, price |
| Orca | Whirlpool | Swap direction, amounts, pool, price |
| Jupiter | Aggregator v6 | Route, input/output amounts, price impact |
| Transfer | System, Token | Transfer direction, amount, recipient |

---

## 9. Database Schema

**Database:** `transactions.db`

### transactions table

| Column | Type | Purpose |
|--------|------|---------|
| `id` | INTEGER PK AUTO | Record ID |
| `signature` | TEXT UNIQUE | Transaction signature |
| `transaction_type` | TEXT | buy/sell/transfer/etc. |
| `status` | TEXT | pending/confirmed/failed/expired |
| `token_mint` | TEXT | Token involved |
| `token_symbol` | TEXT | Symbol for display |
| `amount_sol` | REAL | SOL amount |
| `amount_token` | REAL | Token amount |
| `price_per_token` | REAL | Execution price |
| `wallet_address` | TEXT | Wallet involved |
| `pool_address` | TEXT | DEX pool used |
| `dex` | TEXT | DEX name |
| `priority_fee_lamports` | INTEGER | Priority fee paid |
| `compute_units` | INTEGER | CU consumed |
| `error` | TEXT | Error details |
| `created_at` | TEXT | Submission time |
| `confirmed_at` | TEXT | Confirmation time |
| `slot` | INTEGER | Confirmed slot |

**Indexes:** `signature`, `wallet_address+created_at`, `token_mint+created_at`, `status`

---

## 10. Module Connections

```
transactions/
├── rpc/            ← Transaction sending, confirmation polling
├── wallets/        ← Keypair for signing
├── pools/          ← Pool address resolution
├── positions/      ← Position updates on confirmed trades
├── events/         ← Transaction events
├── config/         ← Priority fee settings, retry config
├── database/       ← SQLite infrastructure
└── errors/         ← Error types
```

| Caller | Usage |
|--------|-------|
| trader/executors | Build + sign + send swap transactions |
| swaps/routers | Transaction builder integration |
| positions | Position opening/closing confirmation |
| webserver | Transaction history API |
| tools | Manual transfers, ATA operations |
