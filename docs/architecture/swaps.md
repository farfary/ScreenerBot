# Swaps Module — Architecture

> ScreenerBot multi-router quoting + swap execution with fallback (Jupiter primary, GMGN optional fallback, Raydium stub) — February 2026

---

## Table of Contents

1. [Overview](#1-overview)
2. [File Structure](#2-file-structure)
3. [Core Types](#3-core-types)
4. [Router Trait (`SwapRouter`)](#4-router-trait-swaprouter)
5. [Router Registry (`RouterRegistry`)](#5-router-registry-routerregistry)
6. [Quote Selection (`get_best_quote`)](#6-quote-selection-get_best_quote)
7. [Swap Execution + Fallback (`execute_swap_with_fallback`)](#7-swap-execution--fallback-execute_swap_with_fallback)
8. [Special Quote Flow: Opening Positions (`get_best_quote_for_opening`)](#8-special-quote-flow-opening-positions-get_best_quote_for_opening)
9. [Router Implementations](#9-router-implementations)
10. [Fee Collection (Jupiter Referral)](#10-fee-collection-jupiter-referral)
11. [Error Handling + Retryability](#11-error-handling--retryability)
12. [Module Connections](#12-module-connections)

---

## 1. Overview

The `swaps` module is the execution layer that turns:

- "I want to trade X SOL into token Y"

into:

- a best-route quote (aggregator-specific)
- a signed Solana transaction submission
- a confirmed signature (or an actionable structured error)

Key design points in the current implementation:

- Multiple routers implement the same `SwapRouter` trait.
- Quotes are fetched from all enabled routers concurrently and the best quote is selected by output amount.
- Swap execution starts with the router that produced the quote and can fall back to other enabled routers if the error is classified as retryable.
- Jupiter fee collection is intentionally hardcoded (0.5% referral) and is a revenue source.

---

## 2. File Structure

```text
src/swaps/
├── mod.rs                 module exports + `calculate_partial_amount()`
├── router.rs              `SwapRouter` trait + core quote/execute types
├── registry.rs            `RouterRegistry` + global OnceLock accessor
├── operations.rs          high-level orchestration (quote selection + fallback execution)
├── types.rs               shared enums + serde helpers used by routers
└── routers/
    ├── mod.rs             router exports
    ├── jupiter.rs         Jupiter router (primary; referral fee collection)
    ├── gmgn.rs            GMGN router (optional; anti-MEV, retry loop, connectivity gated)
    └── raydium.rs         Raydium stub (not implemented)
```

---

## 3. Core Types

**File:** `src/swaps/router.rs`

### 3.1 `QuoteRequest`

Immutable input passed to routers:

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

`slippage_pct` is expressed as a percentage (e.g. `1.0` = 1%).

### 3.2 `SwapMode`

```rust
pub enum SwapMode {
  ExactIn,
  ExactOut,
}
```

Routers typically serialize this as `"ExactIn"` / `"ExactOut"` via `SwapMode::as_str()`.

### 3.3 `Quote`

Router-agnostic quote representation:

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
  pub execution_data: Vec<u8>,
}
```

`execution_data` is intentionally opaque:

- Jupiter stores the raw JSON quote response bytes (to preserve all fields needed by `/swap`).
- GMGN stores a serialized `SwapData` struct (quote + raw tx + metadata).

`fee_lamports` is also router-specific in practice:

- Jupiter currently sets it to `0` (referral fee is applied via Jupiter's platform fee mechanism).
- GMGN sets it to `swap_data.raw_tx.prioritization_fee_lamports` (priority fee, not a DEX fee).

### 3.4 `SwapResult`

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

### 3.5 Shared swap enums + helpers

**File:** `src/swaps/types.rs`

- `RouterType`: currently `{ GMGN, Jupiter }` (used by other layers)
- `ExitType`: `{ Full, Partial { percentage } }` (used by positions/trader)
- custom deserializers used by GMGN:
  - `deserialize_string_or_number`
  - `deserialize_optional_string_or_number`

**File:** `src/swaps/mod.rs`

`calculate_partial_amount(total_amount, percentage)` is used for partial exits:

- returns 0 if `total_amount == 0` or `percentage <= 0`
- returns `total_amount` if `percentage >= 100`
- otherwise returns `min(total_amount, floor(total_amount * percentage/100))`

---

## 4. Router Trait (`SwapRouter`)

**File:** `src/swaps/router.rs`

All swap routers implement:

```rust
#[async_trait]
pub trait SwapRouter: Send + Sync {
  fn id(&self) -> &'static str;        // "jupiter", "gmgn", "raydium"
  fn name(&self) -> &'static str;      // UI/log name
  fn is_enabled(&self) -> bool;        // config gate
  fn priority(&self) -> u8;            // 0 = primary, higher = fallback order
  async fn get_quote(&self, request: &QuoteRequest) -> Result<Quote>;
  async fn execute_swap(&self, token: &Token, quote: &Quote) -> Result<SwapResult>;
}
```

Router IDs are used end-to-end:

- quoted router ID is stored in `Quote.router_id`
- execution uses `registry.get_router(&quote.router_id)` to find the router again

---

## 5. Router Registry (`RouterRegistry`)

**File:** `src/swaps/registry.rs`

The registry is the only place where routers are assembled into the system.

### 5.1 Construction

```rust
pub fn new() -> Self {
  routers: vec![
    Arc::new(JupiterRouter::new()),
    Arc::new(GmgnRouter::new()),
    Arc::new(RaydiumRouter::new()),
  ]
}
```

### 5.2 Global singleton

```rust
static REGISTRY: OnceLock<RouterRegistry> = OnceLock::new();

pub fn get_registry() -> &'static RouterRegistry {
  REGISTRY.get_or_init(|| RouterRegistry::new())
}
```

### 5.3 Enabled routers and fallback chain

Key methods:

- `enabled_routers()`: filters by `router.is_enabled()`
- `get_primary_router()`: min by `priority()`
- `get_router(id)`: lookup by router id string
- `get_fallback_chain(failed_router_id)`:
  - filters enabled routers excluding the failed id
  - sorts by `priority()` (lowest first)

---

## 6. Quote Selection (`get_best_quote`)

**File:** `src/swaps/operations.rs`

`get_best_quote(request)`:

1. Reads enabled routers from registry.
2. Spawns quote requests to each router concurrently using `future::join_all(...)`.
3. Drops failed quotes (logs warning) and keeps successful quotes.
4. Picks the quote with highest `output_amount`.

Important behavior:

- If no routers are enabled, it returns a configuration error.
- If all routers fail, it returns `Error::api_error("All routers failed to provide quotes")`.

---

## 7. Swap Execution + Fallback (`execute_swap_with_fallback`)

**File:** `src/swaps/operations.rs`

`execute_swap_with_fallback(token, quote)`:

### 7.1 Force-stop gate

Before doing anything:

- if `global::is_force_stopped()` is true, it returns an internal error:
  `"Trading halted - Force stop is active"`

### 7.2 Primary attempt

1. Resolve the router from the quote:
   - `primary = registry.get_router(&quote.router_id)`
2. Call:
   - `primary.execute_swap(token, &quote)`

If it succeeds:

- returns `SwapResult` with `execution_time_ms` filled in.

### 7.3 Fallback attempt (only for retryable errors)

If `primary.execute_swap(...)` returns an error:

- if `is_retryable_error(...)` is false → fail immediately (non-retryable)
- otherwise → attempt fallbacks:
  - `fallbacks = registry.get_fallback_chain(&quote.router_id)`

For each fallback router:

1. Build a new `QuoteRequest` based on the original quote.
   - reconstructed slippage uses `slippage_pct = (quote.slippage_bps as f64) / 100.0`
2. Fetch a fresh quote from the fallback router.
3. Execute the swap using the fallback router and its fresh quote.

If all fallbacks fail, `execute_swap_with_fallback` returns the original primary error.

---

## 8. Special Quote Flow: Opening Positions (`get_best_quote_for_opening`)

**File:** `src/swaps/operations.rs`

`get_best_quote_for_opening(request, token_symbol)` wraps `get_best_quote(...)` and adds
"no route" failure tracking:

- It detects "no route" by string matching on the error message (e.g. `"no route"`, `"Jupiter API error: 400"`).
- On detected no-route errors it blacklists the non-SOL mint via:
  - `tokens::cleanup::blacklist_token(output_mint, "NoRoute", &db)`

This is best-effort:

- it ignores blacklist errors (`let _ = blacklist_token(...)`) because quote failures should not crash the bot.

---

## 9. Router Implementations

Routers live under `src/swaps/routers/*`.

### 9.1 Jupiter router (primary)

**File:** `src/swaps/routers/jupiter.rs`

Enabling:

- `is_enabled()` reads `cfg.swaps.jupiter.enabled` (default true)

API key model (key is OPTIONAL):

- Default (no key): requests go to the FREE endpoint `https://lite-api.jup.ag`.
- If `cfg.swaps.jupiter.api_key` is set: requests go to `https://api.jup.ag` and the
  `x-api-key` header is added (higher rate-limit tier from portal.jup.ag).
- The API key ONLY affects rate limits. It does NOT affect swap fees or the
  referral revenue share — referral works fully keyless.

Rate-limit resilience (free tier limits hard, per-IP, across all endpoints):

- `jupiter_send_with_retry(...)` wraps both the quote (GET) and swap (POST) calls
  with retry + exponential backoff on HTTP 429, 5xx, and network errors. It honors
  a `Retry-After` header when present. 4xx (e.g. 400 = no route) is NOT retried.
  Retrying the `/swap` call is safe because it only BUILDS an unsigned transaction.
- A swap holds a `crate::apis::jupiter::throttle::swap_guard()` for the whole
  quote/swap so background Jupiter callers (SOL price fallback, token discovery,
  health monitor, multi-wallet tool) defer to it and don't steal the shared budget.
  See `src/apis/jupiter/throttle.rs`.

#### 9.1.1 Quote flow (GET `/swap/v1/quote`)

1. Acquire a `swap_guard()` (priority over background Jupiter traffic).
2. Compute slippage bps:
   - `slippage_bps = ((slippage_pct * 100.0).round() as u16).max(1)`
3. Build `JupiterQuoteRequest` and send (via `jupiter_send_with_retry`):
   - `GET {base}/swap/v1/quote` where base = lite-api (keyless) or api.jup.ag (keyed)
   - query params: `inputMint`, `outputMint`, `amount`, `slippageBps`, `swapMode`,
     `platformFeeBps` (always `REFERRAL_FEE_BPS` = 50), optional `excludeDexes`, and
     **`instructionVersion=V2`** (REQUIRED so the platform fee can be collected on
     Token2022 tokens — see §10).
4. Read the response body as text.
5. Parse a limited struct (`JupiterQuoteResponse`) for `outAmount`, `priceImpactPct`,
   `routePlan`.
6. Store the *raw* JSON bytes as `Quote.execution_data` (preserves `platformFee` etc.).

#### 9.1.2 Swap flow (POST `/swap/v1/swap`)

1. Acquire a `swap_guard()`.
2. Deserialize `Quote.execution_data` into `serde_json::Value` (quoteResponse).
3. Referral fee account selection via the shared `referral_fee_account(input, output)`
   helper: always returns the WSOL or USDC referral token account (we always trade
   against SOL/USDC, so one side matches). Token2022 is NOT skipped anymore.
4. Build `JupiterSwapRequest` including:
   - `userPublicKey`
   - `quoteResponse`
   - `dynamicComputeUnitLimit = cfg.swaps.jupiter.dynamic_compute_unit_limit`
   - `prioritizationFeeLamports = cfg.swaps.jupiter.default_priority_fee`
   - `feeAccount` (the referral token account)
5. Send (via `jupiter_send_with_retry`):
   - `POST {base}/swap/v1/swap`, JSON body, `Content-Type: application/json`,
     `x-api-key` header only when a key is configured
6. Parse `swapTransaction` (base64 string).
7. Submit via RPC:
   - `rpc_client.sign_send_and_confirm_transaction_simple(&swapTransaction)`

### 9.2 GMGN router (optional fallback)

**File:** `src/swaps/routers/gmgn.rs`

Enabling:

- `is_enabled()` reads `cfg.swaps.gmgn.enabled` (default false)

Connectivity gate:

- GMGN refuses to quote/execute when either `internet` or `rpc` endpoints are unhealthy:
  - `connectivity::check_endpoints_healthy(&["internet", "rpc"])`

#### 9.2.1 Quote flow (GET `GMGN_QUOTE_API`)

Endpoint:

- `GMGN_QUOTE_API = "https://gmgn.ai/defi/router/v1/sol/tx/get_swap_route"`

Query string includes:

- `token_in_address`
- `token_out_address`
- `in_amount`
- `from_address`
- `slippage`
- `swap_mode`
- `fee` (from `cfg.swaps.gmgn.fee_sol`)
- `is_anti_mev` (from `cfg.swaps.gmgn.anti_mev`)
- `partner` (from `cfg.swaps.gmgn.partner`)

Retry loop:

- attempts: 3
- request timeout: 15s
- delay between attempts: 1s, 2s, 3s (linear backoff)

No-route detection:

- if response JSON contains:
  - `code == 40000402`
  - or `msg` contains `"no route"`
- then it returns an API error immediately (no more retries).

The parsed output (`SwapData`) contains:

- `quote` (amounts, impact, route plan, ...)
- `raw_tx.swapTransaction` (base64 tx string)

The router stores `SwapData` as JSON in `Quote.execution_data`.

#### 9.2.2 Swap flow

1. Deserialize `Quote.execution_data` into `SwapData`.
2. Send transaction via RPC:
   - `sign_send_and_confirm_transaction_simple(&swap_data.raw_tx.swap_transaction)`
3. Record swap event:
   - `events::record_swap_event(...)`

### 9.3 Raydium router (stub)

**File:** `src/swaps/routers/raydium.rs`

- `get_quote` and `execute_swap` return `Error::internal_error("Raydium router not implemented yet")`.

---

## 10. Fee Collection (Jupiter Referral)

**File:** `src/swaps/routers/jupiter.rs`

Fee collection is intentionally hardcoded and not configurable (it is the revenue):

- `REFERRAL_FEE_BPS = 50` (0.5%)
- Referral token accounts (proper Jupiter Referral Program PDAs):
  - WSOL: `9yiZThTzanryu3mg1VVu6Qy4HiqKhydCAUqcasLHPxWB`
  - USDC: `3kmcF3DFGFRKXeC5v5AMzwpsdj2Uc3Z7a5KrojtWv2GW`
  - Both have token authority `FnTdf2xmCUXrW7PMRtsSiQsnCzGZ31HTY2Sb36vfWtVn`, which is
    owned by the Jupiter Referral Program `REFER4ZgmyYx9c6He5XfaTMiGfdLwRnkV4RPp9t9iF3`.

Jupiter fee mechanics used:

- Quote request: `platformFeeBps = 50` (always) + `instructionVersion = V2`.
- Swap request: `feeAccount` = the WSOL/USDC referral account (always).
- The fee is charged on the SOL/USDC side of the pair, so a single WSOL referral
  account collects on BOTH directions:
  - Buy (SOL→token): fee taken on input WSOL.
  - Sell (token→SOL): fee taken on output WSOL.
- The quote's `platformFee` field is displayed in the OUTPUT mint, but the actual
  collection side follows the `feeAccount` mint we pass (WSOL).

Token2022 handling (FIXED — was previously skipped):

- Older code skipped fees entirely for Token2022 to avoid `IncorrectTokenProgramID`
  (custom error `0x177e` / 6014). Since most pump.fun tokens are now Token2022, that
  leaked the majority of referral revenue.
- Adding `instructionVersion=V2` to the quote lets Jupiter collect the fee on the
  SOL/USDC side even when the other side is Token2022, with no `0x177e`. Verified by
  simulation (default → `0x177e`; V2 → ok) and by real on-chain swaps.

API access: referral fees work on the FREE `lite-api.jup.ag` (no API key). The key
only changes the rate-limit tier, never the fee/referral.

---

## 11. Error Handling + Retryability

**File:** `src/swaps/operations.rs`

Fallback is only attempted for errors classified as retryable:

```rust
matches!(
  error,
  Error::Network(_) | Error::RpcProvider(_) | Error::RateLimit(_)
)
```

Everything else is treated as non-retryable and fails immediately.

Routers also implement internal retry logic:

- Jupiter: `jupiter_send_with_retry` retries quote/swap on HTTP 429 / 5xx / network
  with exponential backoff (honors `Retry-After`). This matters because Jupiter is
  usually the only enabled router, so the cross-router fallback above can't help —
  the in-router retry is what survives lite-api rate limits.
- GMGN: a 3-attempt linear-backoff retry loop (see §9.2.1).

---

## 12. Module Connections

The swaps module depends on:

- `config` (`cfg.swaps.*` for router enablement + per-router parameters)
- `rpc` (`sign_send_and_confirm_transaction_simple`)
- `tokens`:
  - opening quote helper can blacklist tokens on no-route failures
- `apis::jupiter::throttle`: swap-priority gate shared with all Jupiter callers
- `connectivity`:
  - GMGN is gated on internet+rpc health
- `events`:
  - GMGN records swap events (`record_swap_event`)
- `global`:
  - force-stop gate prevents swap execution when trading is halted

The swaps module is used by:

- positions lifecycle (enter/exit)
- trader engine (automated entry/exit)
- manual trading endpoints (webserver / telegram)
