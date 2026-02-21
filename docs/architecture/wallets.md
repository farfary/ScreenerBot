# Wallets Module — Architecture

> ScreenerBot Secure Multi-Wallet Management — February 2026

---

## Table of Contents

1. [Overview](#1-overview)
2. [File Structure](#2-file-structure)
3. [Core Types](#3-core-types)
4. [Wallet Manager](#4-wallet-manager)
5. [Cryptography](#5-cryptography)
6. [Balance Monitoring](#6-balance-monitoring)
7. [Database Schema](#7-database-schema)
8. [Module Connections](#8-module-connections)

---

## 1. Overview

The Wallets module provides secure multi-wallet management with AES-256-GCM encryption, keypair import/export, balance tracking, and bulk operations. All private keys are encrypted at rest.

**Key characteristics:**
- AES-256-GCM encryption for all private keys
- Multi-wallet with role system (Main, Secondary, Archive)
- Cached main wallet keypair for fast signing
- Token balance tracking per wallet
- Bulk import (CSV/Excel)
- Database-backed with foreign key cascading

**16 files, ~6,422 lines**

---

## 2. File Structure

```
src/wallets/
├── mod.rs              # Module exports
├── types.rs            # Wallet, TokenBalance, WalletRole, WalletType
├── manager.rs          # CRUD, caching, main wallet fast path
├── database.rs         # SQLite schema (2 tables)
├── crypto.rs           # Keypair generation, encryption, import/export
├── validation.rs       # Wallet consistency validation
├── balance_monitor/    # Balance tracking service
└── bulk/               # CSV/Excel bulk import
```

---

## 3. Core Types

### Wallet

```rust
pub struct Wallet {
    pub id: i64,
    pub name: String,
    pub address: String,                    // base58
    pub role: WalletRole,                   // Main | Secondary | Archive
    pub wallet_type: WalletType,            // Generated | Imported | Migrated
    pub created_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub notes: Option<String>,
    pub is_active: bool,
}
```

### WalletWithKey

```rust
pub struct WalletWithKey {
    pub wallet: Wallet,
    pub keypair: Keypair,                   // Decrypted for signing
}
```

### TokenBalance

```rust
pub struct TokenBalance {
    pub wallet_id: i64,
    pub mint: String,
    pub balance: u64,                       // Raw (smallest units)
    pub ui_amount: f64,                     // Human-readable with decimals
    pub decimals: u8,
    pub symbol: Option<String>,
    pub name: Option<String>,
    pub is_token_2022: bool,
    pub updated_at: DateTime<Utc>,
}
```

### Summary Types

```rust
pub struct WalletBalanceSummary {
    pub wallet_id: i64,
    pub wallet_name: String,
    pub address: String,
    pub sol_balance: f64,
    pub token_count: u32,
    pub tokens: Vec<SimpleTokenBalance>,
    pub empty_ata_count: u32,
    pub reclaimable_sol: f64,
}

pub struct WalletsSummary {
    pub total_count: u32,
    pub active_count: u32,
    pub main_wallet: Option<String>,
    pub main_wallet_name: Option<String>,
    pub total_sol: f64,
}
```

### Enums

| Enum | Variants |
|------|----------|
| `WalletRole` | `Main`, `Secondary`, `Archive` |
| `WalletType` | `Generated`, `Imported`, `Migrated` |

---

## 4. Wallet Manager

### Global State

```rust
static WALLETS_DB: LazyLock<Arc<RwLock<Option<WalletsDatabase>>>>
static MAIN_WALLET_CACHE: LazyLock<Arc<RwLock<Option<CachedMainWallet>>>>
```

### Public API

| Function | Purpose |
|----------|---------|
| `initialize()` | Init DB, load main wallet cache |
| `is_initialized()` | Check ready state |
| **Main wallet (fast path)** | |
| `get_main_keypair()` | Cached keypair for signing |
| `get_main_wallet()` | Main wallet metadata |
| `get_main_address()` | Main wallet address |
| `has_main_wallet()` | Check if main exists |
| `set_main_wallet(id)` | Designate main |
| **CRUD** | |
| `create_wallet(req)` | Create new wallet |
| `create_wallets_batch(reqs)` | Bulk create |
| `get_wallet(id)` | Get by ID |
| `get_wallet_by_address(addr)` | Get by address |
| `get_wallet_keypair(id)` | Decrypt keypair for signing |
| `update_wallet(id, req)` | Update metadata |
| `list_wallets(include_inactive)` | List all |
| `list_active_wallets()` | Active only |
| `delete_wallet(id)` | Delete (cascades balances) |
| `archive_wallet(id)` | Soft archive |
| `restore_wallet(id)` | Restore from archive |
| **Import/Export** | |
| `import_wallet(req)` | Import from private key |
| `export_wallet(id)` | Export (decrypted key) |
| `bulk_import_wallets(req)` | CSV/Excel import |
| **Balance tracking** | |
| `upsert_token_balance(...)` | Update token balance |
| `get_token_balances(wallet_id)` | Get wallet's tokens |
| `get_all_token_balances()` | All wallets' tokens |
| `get_wallets_summary()` | Summary stats |
| `get_wallet_balances(id)` | Balance details |

---

## 5. Cryptography

**File:** `crypto.rs`

| Function | Purpose |
|----------|---------|
| `generate_keypair()` | Secure random keypair |
| `generate_and_encrypt_keypair()` | Generate + AES-256-GCM encrypt |
| `parse_private_key(key)` | Parse base58 or JSON array format |
| `import_and_encrypt(key)` | Parse + encrypt |
| `export_private_key(encrypted, nonce)` | Decrypt to base58 |
| `decrypt_to_keypair(encrypted, nonce)` | Decrypt to Keypair |
| `validate_address(address)` | Validate base58 |
| `keypair_to_address(keypair)` | Keypair → base58 address |

**Encryption:** AES-256-GCM with unique nonce per key, stored as separate `encrypted_key` and `nonce` columns.

---

## 6. Balance Monitoring

**Directory:** `balance_monitor/`

Service that periodically fetches SOL and token balances for all active wallets via RPC calls.

---

## 7. Database Schema

**Database:** `wallets.db`

### wallets table

| Column | Type | Purpose |
|--------|------|---------|
| `id` | INTEGER PK AUTO | Wallet ID |
| `name` | TEXT | Display name |
| `address` | TEXT UNIQUE | Base58 address |
| `encrypted_key` | TEXT | AES-256-GCM encrypted private key |
| `nonce` | TEXT | Encryption IV |
| `role` | TEXT | main/secondary/archive |
| `wallet_type` | TEXT | generated/imported/migrated |
| `created_at` | TEXT | Timestamp |
| `last_used_at` | TEXT | Last trading use |
| `notes` | TEXT | User notes |
| `is_active` | INTEGER | Active flag |

**Indexes:** `address`, `role`, `active+role`

### wallet_token_balances table

| Column | Type | Purpose |
|--------|------|---------|
| `wallet_id` | INTEGER FK | References wallets(id) CASCADE |
| `mint` | TEXT | Token mint address |
| `balance` | INTEGER | Raw amount |
| `ui_amount` | REAL | Human-readable |
| `decimals` | INTEGER | Token decimals |
| `symbol` | TEXT | Token symbol |
| `name` | TEXT | Token name |
| `is_token_2022` | INTEGER | Token2022 flag |
| `updated_at` | TEXT | Last update |

**Primary key:** `(wallet_id, mint)`

---

## 8. Module Connections

```
wallets/
├── rpc/           ← Balance fetching, account queries
├── config/        ← Encryption key derivation source
├── database/      ← SQLite infrastructure
└── errors/        ← Error types
```

| Caller | Usage |
|--------|-------|
| trader/executors | `get_main_keypair()` for transaction signing |
| swaps/routers | Wallet address for swap requests |
| webserver/wallets | Full CRUD API |
| positions | Track which wallet opened position |
| tools | ATA scanning, consolidation, multi-buy/sell |
