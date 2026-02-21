# Swaps Module — Architecture

> ScreenerBot Multi-Router Swap Execution System — February 2026

---

## Table of Contents

1. [Overview](#1-overview)
2. [File Structure](#2-file-structure)
3. [Core Types](#3-core-types)
4. [Router Trait](#4-router-trait)
5. [Router Registry](#5-router-registry)
6. [Router Implementations](#6-router-implementations)
7. [Operations — Quote & Execute](#7-operations--quote--execute)
8. [Fee Collection](#8-fee-collection)
9. [Error Handling & Fallback](#9-error-handling--fallback)
10. [Module Connections](#10-module-connections)

---

## 1. Overview

The Swaps module provides a trait-based, multi-router swap architecture with concurrent quote fetching and automatic fallback chains. It abstracts over multiple DEX aggregators (Jupiter, GMGN, Raydium stub) behind a unified `SwapRouter` trait.

**Key characteristics:**
- Concurrent quote fetching from all enabled routers
- Best-quote selection (highest output amount)
- Automatic fallback on retryable errors
- Hardcoded 0.5% referral fee (Jupiter only) — **revenue source**
- Token2022 detection to skip fees when incompatible
- GMGN retry with exponential backoff (3 attempts)

---

## 2. File Structure

```
src/swaps/
├── mod.rs           # Re-exports, calculate_partial_amount()
├── types.rs         # RouterType enum, ExitType enum, custom deserializers
├── router.rs        # SwapRouter trait, QuoteRequest, Quote, SwapResult, SwapMode
├── operations.rs    # get_best_quote(), execute_swap_with_fallback()
├── registry.rs      # RouterRegistry singleton, fallback chain logic
└── routers/
    ├── mod.rs       # Router sub-module exports
    ├── jupiter.rs   # Jupiter DEX router (priority 0, primary)
    ├── gmgn.rs      # GMGN router (priority 1, secondary)
    └── raydium.rs   # Raydium router (priority 2, stub/disabled)
```

**9 files, ~1,688 lines**

---

## 3. Core Types

### QuoteRequest (`router.rs`)

Immutable request passed to all routers:

```rust
pub struct QuoteRequest {
    pub input_mint: String,
    pub output_mint: String,
    pub input_amount: u64,
    pub wallet_address: String,
    pub slippage_pct: f64,
    pub swap_mode: SwapMode,
}
```

### Quote (`router.rs`)

Router-agnostic quote response:

```rust
pub struct Quote {
    pub router_id: String,
    pub router_name: String,
    pub input_mint: String,
    pub output_mint: String,
    pub input_amount: u64,
    pub output_amount: u64,
    pub price_impact_pct: f64,
    pub fee_lamports: u64,
    pub slippage_bps: u16,
    pub route_plan: String,
    pub swap_mode: SwapMode,
    pub wallet_address: String,
    pub execution_data: Vec<u8>,     // Serialized router-specific data for swap step
}
```

### SwapResult (`router.rs`)

Execution outcome:

```rust
pub struct SwapResult {
    pub success: bool,
    pub router_id: String,
    pub router_name: String,
    pub transaction_signature: String,
    pub input_amount: u64,
    pub output_amount: u64,
    pub price_impact_pct: f64,
    pub fee_lamports: u64,
    pub execution_time_ms: u64,
    pub effective_price_sol: Option<f64>,
}
```

### Enums

| Enum | Variants | Location |
|------|----------|----------|
| `SwapMode` | `ExactIn`, `ExactOut` | `router.rs` |
| `RouterType` | `GMGN`, `Jupiter` | `types.rs` |
| `ExitType` | `Full`, `Partial { percentage: f64 }` | `types.rs` |

---

## 4. Router Trait

```rust
#[async_trait]
pub trait SwapRouter: Send + Sync {
    fn id(&self) -> &'static str;
    fn name(&self) -> &'static str;
    fn is_enabled(&self) -> bool;           // Reads from config
    fn priority(&self) -> u8;               // Lower = higher priority
    async fn get_quote(&self, request: &QuoteRequest) -> Result<Quote, ScreenerBotError>;
    async fn execute_swap(&self, token: &Token, quote: &Quote) -> Result<SwapResult, ScreenerBotError>;
}
```

---

## 5. Router Registry

### Singleton (`registry.rs`)

```rust
static REGISTRY: OnceLock<RouterRegistry> = OnceLock::new();

pub struct RouterRegistry {
    routers: Vec<Arc<dyn SwapRouter>>,
}
```

**Created with:** Jupiter (priority 0), GMGN (priority 1), Raydium (priority 2)

### Methods

| Method | Purpose |
|--------|---------|
| `get_registry()` | Singleton accessor |
| `enabled_routers()` | Filter to enabled only |
| `get_router(id)` | Lookup by ID string |
| `get_primary_router()` | Lowest priority enabled router |
| `get_fallback_chain(failed_id)` | Enabled routers except failed, sorted by priority |
| `has_enabled_routers()` | Boolean check |
| `all_routers()` | All routers including disabled |

---

## 6. Router Implementations

### Jupiter Router (`routers/jupiter.rs`) — Priority 0

**Primary router.** Uses Jupiter Swap API v1.

#### Constants (HARDCODED — revenue source, NOT configurable)

```
JUPITER_API_BASE        = "https://api.jup.ag"
REFERRAL_FEE_BPS        = 50                    // 0.5%
REFERRAL_TOKEN_ACCOUNT_WSOL = "9yiZThTzanryu3mg1VVu6Qy4HiqKhydCAUqcasLHPxWB"
REFERRAL_TOKEN_ACCOUNT_USDC = "3kmcF3DFGFRKXeC5v5AMzwpsdj2Uc3Z7a5KrojtWv2GW"
```

#### Quote Flow

1. Check input/output mints for Token2022 via RPC
2. Build `JupiterQuoteRequest`:
   - `platformFeeBps = 50` (or None if Token2022)
3. POST `https://api.jup.ag/swap/v1/quote` with API key header
4. Parse response: `outAmount`, `priceImpactPct`, `routePlan`
5. Store raw JSON response as `execution_data`

#### Swap Flow

1. Deserialize stored JSON quote response
2. Select fee account (WSOL or USDC based on output mint)
3. Build `JupiterSwapRequest`:
   - `quoteResponse` = stored JSON
   - `feeAccount` = referral account (None if Token2022)
   - `prioritizationFeeLamports` from config
4. POST `https://api.jup.ag/swap/v1/swap`
5. Extract base64 `swapTransaction`
6. `sign_send_and_confirm_transaction_simple(base64_tx)`

#### Token2022 Handling

If either input or output mint is Token2022:
- Skip `platformFeeBps` in quote (prevents `IncorrectTokenProgramID`)
- Skip `feeAccount` in swap request

### GMGN Router (`routers/gmgn.rs`) — Priority 1

**Secondary/fallback router.** Uses GMGN API.

#### Constants

```
GMGN_QUOTE_API    = "https://gmgn.ai/defi/router/v1/sol/tx/get_swap_route"
QUOTE_TIMEOUT_SECS = 15
RETRY_ATTEMPTS      = 3
```

#### Quote Flow

1. Check connectivity (internet + RPC)
2. Build URL with query params: `token_in`, `token_out`, `in_amount`, `from_address`, `slippage`, `fee`, `is_anti_mev`, `partner`
3. **Retry logic:** 3 attempts, exponential backoff (1s, 2s, 3s)
4. Detect "no route" (code `40000402`) — fail immediately, no retry
5. Parse `SwapData` (includes quote + pre-built raw transaction)

#### Swap Flow

1. Deserialize `SwapData` from `execution_data`
2. `sign_send_and_confirm_transaction_simple(swap_data.raw_tx.swap_transaction)`
3. Record swap event

### Raydium Router (`routers/raydium.rs`) — Priority 2

**Stub.** Returns `Err("Raydium router not implemented yet")` for both quote and swap.

---

## 7. Operations — Quote & Execute

### `get_best_quote(request)` → `operations.rs`

1. Get all enabled routers from registry
2. Fetch quotes from ALL routers **concurrently** (`futures::join_all`)
3. Log timing and comparison for each router
4. Select quote with **highest `output_amount`**

### `get_best_quote_for_opening(request, symbol)` → `operations.rs`

Wraps `get_best_quote()` with no-route detection:
- If "no route" error → blacklist token
- Detects: "no route", "400 Bad Request", Jupiter-specific errors

### `execute_swap_with_fallback(token, quote)` → `operations.rs`

1. Try primary router's `execute_swap()`
2. On **retryable** error → enter fallback chain:
   - Get fallback routers (sorted by priority, excluding failed)
   - For each: fetch fresh quote → execute swap
   - Return first success or original error
3. Check force-stop flag before each attempt

---

## 8. Fee Collection

**Jupiter only.** GMGN and Raydium do not collect fees.

| Parameter | Value |
|-----------|-------|
| Fee rate | 50 BPS (0.5%) |
| WSOL fee account | `9yiZThTzanryu3mg1VVu6Qy4HiqKhydCAUqcasLHPxWB` |
| USDC fee account | `3kmcF3DFGFRKXeC5v5AMzwpsdj2Uc3Z7a5KrojtWv2GW` |
| Jupiter's share | 20% |
| ScreenerBot's share | 80% |
| Token2022 | Fees skipped (prevents transaction failure) |

**Mechanism:**
- `platformFeeBps` in quote request → Jupiter deducts from output
- `feeAccount` in swap request → fees deposited to referral token account

---

## 9. Error Handling & Fallback

### Retryable Error Categories

| Error Type | Retryable | Action |
|-----------|-----------|--------|
| `Network` | ✅ | Fallback to next router |
| `RpcProvider` | ✅ | Fallback to next router |
| `RateLimit` | ✅ | Fallback to next router |
| All others | ❌ | Return error immediately |

### Fallback Chain Priority

| Priority | Router | Role |
|----------|--------|------|
| 0 | Jupiter | Primary |
| 1 | GMGN | First fallback |
| 2 | Raydium | Second fallback (stub) |

### GMGN-Specific Retry

- 3 attempts with exponential backoff (1s, 2s, 3s delays)
- "No route" errors (code `40000402`) fail immediately without retry

---

## 10. Module Connections

```
swaps/
├── config/          ← Router enable/disable, API keys, slippage, priority fees
├── rpc/             ← Token2022 detection, transaction signing/sending
├── tokens/          ← Token struct for execute_swap()
├── connectivity/    ← GMGN pre-checks (internet + RPC available)
├── events/          ← Record swap events for dashboard
├── errors/          ← ScreenerBotError types
└── actions/         ← calculate_partial_amount() used by positions
```

| Caller | Function | Purpose |
|--------|----------|---------|
| trader/executors | `get_best_quote()` → `execute_swap_with_fallback()` | Trade execution |
| trader/executors | `get_best_quote_for_opening()` | Entry with blacklist on no-route |
| positions | `calculate_partial_amount()` | Partial exit amount calculation |

### Utility Function

```rust
pub fn calculate_partial_amount(total_amount: u64, percentage: f64) -> u64
```

Calculates token amount for partial exits: `(total * percentage / 100.0) as u64`
