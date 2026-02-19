# Swaps Module Redesign - Trait-Based Router Architecture

## Executive Summary

**This redesign introduces:**

1. **Trait-based router system** - Add routers by implementing a trait (1 file vs 7 files)
2. **Jupiter Ultra API migration** - Switch from deprecated Lite API to Ultra API with API key
3. **Hardcoded 0.5% referral fee** - Revenue generation (NOT configurable by users)
4. **Priority-based fallback** - Automatic failover without hardcoded pairs
5. **Future-proof architecture** - Plugin support, runtime discovery, multi-chain ready

**Current Status (Dec 3, 2025):**

- ⏰ **28 days until Lite API shutdown** (December 31, 2025)
- ✅ Bot currently works with `lite-api.jup.ag` (no API key needed)
- ⚠️ Must migrate to `api.jup.ag` before Dec 31
- 🆓 Free tier available: 60 requests/minute with API key

**Critical Actions Required:**

- ✅ Migrate Jupiter from `lite-api.jup.ag` to `api.jup.ag` (deadline: Dec 31 2025)
- ✅ Get API key from https://portal.jup.ag (free tier: 60 req/min)
- ✅ Setup referral account to collect 0.5% fees on all swaps
- ✅ Initialize referral token accounts for SOL/USDC/USDT mints

**API Key Requirement Clarification:**

- **Current (until Dec 31):** Lite API works WITHOUT API key
- **After Dec 31:** API key REQUIRED for all Jupiter APIs
- **Free tier:** Available to everyone (just needs email signup)
- **Rate limits:** 60 req/min free, paid tiers up to 5000 req/10sec

---

## Current Problems

### 1. **Hardcoded Router Logic**

- Adding/removing routers requires changes in 7+ locations
- Match statements everywhere (`RouterType::Jupiter`, `RouterType::GMGN`)
- Duplicate code for quote fetching, execution, and fallback
- No dynamic router discovery

### 2. **Rigid Fallback System**

- Hardcoded pairs (Jupiter ↔ GMGN)
- No priority-based fallback chain
- Cannot configure fallback order
- Adding third router breaks logic

### 3. **Non-Extensible**

- Cannot add routers without modifying core logic
- No plugin system
- Tightly coupled to specific router implementations

---

## Solution: Trait-Based Router Architecture

### Core Principle

**One interface, many implementations. Adding a router = implementing a trait.**

---

## New Architecture

### 1. **Router Trait** (`src/swaps/router.rs`)

```rust
use async_trait::async_trait;
use crate::errors::ScreenerBotError;
use crate::tokens::Token;

/// Unified swap router interface
/// All routers must implement this trait
#[async_trait]
pub trait SwapRouter: Send + Sync {
    /// Router identifier (e.g., "jupiter", "gmgn", "raydium")
    fn id(&self) -> &'static str;

    /// Display name for logging/UI
    fn name(&self) -> &'static str;

    /// Check if router is enabled in config
    fn is_enabled(&self) -> bool;

    /// Fallback priority (lower = higher priority, 0 = primary)
    /// Used to determine fallback order when primary fails
    fn priority(&self) -> u8;

    /// Get quote from this router
    async fn get_quote(
        &self,
        request: &QuoteRequest,
    ) -> Result<Quote, ScreenerBotError>;

    /// Execute swap using quote from this router
    async fn execute_swap(
        &self,
        token: &Token,
        quote: &Quote,
    ) -> Result<SwapResult, ScreenerBotError>;
}

/// Quote request parameters (immutable, passed to all routers)
#[derive(Debug, Clone)]
pub struct QuoteRequest {
    pub input_mint: String,
    pub output_mint: String,
    pub input_amount: u64,
    pub wallet_address: String,
    pub slippage_pct: f64,
    pub swap_mode: SwapMode,
}

/// Swap mode enum
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SwapMode {
    ExactIn,
    ExactOut,
}

/// Unified quote response (router-agnostic)
#[derive(Debug, Clone)]
pub struct Quote {
    pub router_id: String,
    pub router_name: String,
    pub output_amount: u64,
    pub price_impact_pct: f64,
    pub fee_lamports: u64,
    pub slippage_bps: u16,
    pub route_plan: String,
    pub execution_data: Vec<u8>, // Serialized router-specific data
}

/// Swap execution result (router-agnostic)
#[derive(Debug)]
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

---

### 2. **Router Registry** (`src/swaps/registry.rs`)

```rust
use crate::swaps::router::SwapRouter;
use std::sync::Arc;

/// Global router registry
/// Manages all available swap routers
pub struct RouterRegistry {
    routers: Vec<Arc<dyn SwapRouter>>,
}

impl RouterRegistry {
    /// Create registry with all routers
    pub fn new() -> Self {
        Self {
            routers: vec![
                Arc::new(crate::swaps::routers::JupiterRouter::new()),
                Arc::new(crate::swaps::routers::GmgnRouter::new()),
                Arc::new(crate::swaps::routers::RaydiumRouter::new()),
                // Add new routers here - ONLY change needed
            ],
        }
    }

    /// Get all enabled routers
    pub fn enabled_routers(&self) -> Vec<Arc<dyn SwapRouter>> {
        self.routers.iter()
            .filter(|r| r.is_enabled())
            .cloned()
            .collect()
    }

    /// Get router by ID
    pub fn get_router(&self, id: &str) -> Option<Arc<dyn SwapRouter>> {
        self.routers.iter()
            .find(|r| r.id() == id)
            .cloned()
    }

    /// Get fallback chain for failed router
    /// Returns routers sorted by priority (excluding failed router)
    pub fn get_fallback_chain(&self, failed_router_id: &str) -> Vec<Arc<dyn SwapRouter>> {
        let mut fallbacks: Vec<_> = self.routers.iter()
            .filter(|r| r.is_enabled() && r.id() != failed_router_id)
            .cloned()
            .collect();

        fallbacks.sort_by_key(|r| r.priority());
        fallbacks
    }

    /// Check if any router is enabled
    pub fn has_enabled_routers(&self) -> bool {
        self.routers.iter().any(|r| r.is_enabled())
    }
}

/// Global registry instance
static REGISTRY: OnceCell<RouterRegistry> = OnceCell::new();

pub fn get_registry() -> &'static RouterRegistry {
    REGISTRY.get_or_init(|| RouterRegistry::new())
}
```

---

### 3. **Core Swap Operations** (`src/swaps/operations.rs`)

```rust
use crate::swaps::registry::get_registry;
use crate::swaps::router::{Quote, QuoteRequest, SwapResult};
use crate::errors::ScreenerBotError;
use crate::logger::{self, LogTag};
use crate::tokens::Token;
use futures::future;

/// Get best quote from all enabled routers (concurrent)
pub async fn get_best_quote(
    request: QuoteRequest,
) -> Result<Quote, ScreenerBotError> {
    let registry = get_registry();
    let enabled = registry.enabled_routers();

    if enabled.is_empty() {
        return Err(ScreenerBotError::configuration_error(
            "No swap routers enabled"
        ));
    }

    logger::info(
        LogTag::Swap,
        &format!("Fetching quotes from {} routers concurrently...", enabled.len())
    );

    // Fetch all quotes concurrently
    let futures: Vec<_> = enabled.iter()
        .map(|router| {
            let req = request.clone();
            let r = router.clone();
            async move {
                match r.get_quote(&req).await {
                    Ok(quote) => {
                        logger::info(
                            LogTag::Swap,
                            &format!("✅ {}: {} tokens out", r.name(), quote.output_amount)
                        );
                        Some(quote)
                    }
                    Err(e) => {
                        logger::warning(
                            LogTag::Swap,
                            &format!("❌ {} quote failed: {}", r.name(), e)
                        );
                        None
                    }
                }
            }
        })
        .collect();

    let results = future::join_all(futures).await;
    let quotes: Vec<Quote> = results.into_iter().flatten().collect();

    if quotes.is_empty() {
        return Err(ScreenerBotError::api_error(
            "All routers failed to provide quotes"
        ));
    }

    // Select best quote (highest output)
    let best = quotes.into_iter()
        .max_by_key(|q| q.output_amount)
        .unwrap();

    logger::info(
        LogTag::Swap,
        &format!("🏆 Best quote: {} with {} tokens out", best.router_name, best.output_amount)
    );

    Ok(best)
}

/// Execute swap with automatic fallback
pub async fn execute_swap_with_fallback(
    token: &Token,
    quote: Quote,
) -> Result<SwapResult, ScreenerBotError> {
    let registry = get_registry();

    // Get primary router
    let primary = registry.get_router(&quote.router_id)
        .ok_or_else(|| ScreenerBotError::internal_error(
            format!("Router {} not found", quote.router_id)
        ))?;

    logger::info(
        LogTag::Swap,
        &format!("🚀 Executing swap via {}", primary.name())
    );

    // Try primary router
    match primary.execute_swap(token, &quote).await {
        Ok(result) => {
            logger::info(
                LogTag::Swap,
                &format!("✅ Swap succeeded via {}", result.router_name)
            );
            return Ok(result);
        }
        Err(primary_error) => {
            // Check if error is retryable
            if !is_retryable_error(&primary_error) {
                return Err(primary_error);
            }

            logger::warning(
                LogTag::Swap,
                &format!("⚠️ {} failed: {} - trying fallback...", primary.name(), primary_error)
            );

            // Try fallback chain
            let fallbacks = registry.get_fallback_chain(&quote.router_id);

            for fallback_router in fallbacks {
                logger::info(
                    LogTag::Swap,
                    &format!("🔄 Attempting fallback to {}", fallback_router.name())
                );

                // Get fresh quote from fallback router
                let fallback_request = QuoteRequest {
                    input_mint: quote.input_mint.clone(),
                    output_mint: quote.output_mint.clone(),
                    input_amount: quote.input_amount,
                    wallet_address: quote.wallet_address.clone(),
                    slippage_pct: (quote.slippage_bps as f64) / 100.0,
                    swap_mode: quote.swap_mode,
                };

                let fallback_quote = match fallback_router.get_quote(&fallback_request).await {
                    Ok(q) => q,
                    Err(e) => {
                        logger::warning(
                            LogTag::Swap,
                            &format!("❌ {} quote failed: {}", fallback_router.name(), e)
                        );
                        continue;
                    }
                };

                // Execute fallback swap
                match fallback_router.execute_swap(token, &fallback_quote).await {
                    Ok(result) => {
                        logger::info(
                            LogTag::Swap,
                            &format!("✅ Fallback succeeded via {}", result.router_name)
                        );
                        return Ok(result);
                    }
                    Err(e) => {
                        logger::warning(
                            LogTag::Swap,
                            &format!("❌ {} execution failed: {}", fallback_router.name(), e)
                        );
                        continue;
                    }
                }
            }

            // All fallbacks failed - return original error
            logger::error(
                LogTag::Swap,
                "❌ All routers failed (primary + fallbacks)"
            );
            Err(primary_error)
        }
    }
}

fn is_retryable_error(error: &ScreenerBotError) -> bool {
    matches!(
        error,
        ScreenerBotError::Network(_) |
        ScreenerBotError::Blockchain(BlockchainError::TransactionDropped { .. })
    )
}
```

---

### 4. **Router Implementations** (`src/swaps/routers/`)

#### Jupiter Router (`src/swaps/routers/jupiter.rs`)

**CRITICAL: Ultra API Migration Required**

- **Old API (Deprecated Dec 31 2025):** `lite-api.jup.ag/swap/v1/*`
- **New API (Required):** `api.jup.ag/ultra/v1/*` with API key
- **API Key:** Required in `x-api-key` header (get from https://portal.jup.ag)
- **Free Tier:** 60 requests/minute
- **Referral Fee:** HARDCODED 0.5% (50 basis points) - NOT CONFIGURABLE

```rust
use crate::swaps::router::{SwapRouter, Quote, QuoteRequest, SwapResult};
use crate::config::with_config;
use crate::errors::ScreenerBotError;
use crate::tokens::Token;
use crate::logger::{self, LogTag};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

// ============================================================================
// JUPITER ULTRA API CONSTANTS
// ============================================================================

/// Jupiter Ultra API base URL (NEW - migrated from lite-api.jup.ag)
const JUPITER_ULTRA_API_BASE: &str = "https://api.jup.ag/ultra/v1";

/// HARDCODED REFERRAL FEE: 0.5% (50 basis points)
/// This fee is MANDATORY for all swaps and CANNOT be changed via config.
/// Revenue share: 80% to ScreenerBot, 20% to Jupiter
const REFERRAL_FEE_BPS: u16 = 50;

/// Referral account public key (created via Jupiter Referral Dashboard)
/// Setup: https://dev.jup.ag/docs/ultra/add-fees-to-ultra
/// Must create referral account ONCE using @jup-ag/referral-sdk
const REFERRAL_ACCOUNT: &str = "YOUR_REFERRAL_ACCOUNT_PUBKEY_HERE";

// ============================================================================
// JUPITER ULTRA API TYPES
// ============================================================================

#[derive(Debug, Serialize)]
struct JupiterOrderRequest {
    #[serde(rename = "inputMint")]
    input_mint: String,
    #[serde(rename = "outputMint")]
    output_mint: String,
    amount: String,
    taker: String,
    #[serde(rename = "referralAccount")]
    referral_account: String,
    #[serde(rename = "referralFee")]
    referral_fee: u16, // In basis points (50-255)
}

#[derive(Debug, Deserialize)]
struct JupiterOrderResponse {
    #[serde(rename = "outputAmount", default)]
    output_amount: Option<String>,
    #[serde(rename = "priceImpact", default)]
    price_impact: Option<f64>,
    #[serde(rename = "feeBps", default)]
    fee_bps: Option<u16>,
    #[serde(rename = "feeMint", default)]
    fee_mint: Option<String>,
    #[serde(rename = "platformFee", default)]
    platform_fee: Option<PlatformFee>,
    transaction: Option<String>, // Base64 encoded unsigned transaction
    #[serde(rename = "requestId")]
    request_id: String,
    #[serde(rename = "errorCode", default)]
    error_code: Option<u8>,
    #[serde(rename = "errorMessage", default)]
    error_message: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PlatformFee {
    amount: String,
    #[serde(rename = "feeBps")]
    fee_bps: u16,
}

#[derive(Debug, Serialize)]
struct JupiterExecuteRequest {
    #[serde(rename = "signedTransaction")]
    signed_transaction: String, // Base64 encoded signed transaction
    #[serde(rename = "requestId")]
    request_id: String,
}

#[derive(Debug, Deserialize)]
struct JupiterExecuteResponse {
    status: String, // "Success" or error
    signature: Option<String>,
    #[serde(rename = "errorCode", default)]
    error_code: Option<u8>,
    #[serde(rename = "errorMessage", default)]
    error_message: Option<String>,
}

// ============================================================================
// JUPITER ROUTER IMPLEMENTATION
// ============================================================================

pub struct JupiterRouter {
    client: Client,
    api_key: String,
}

impl JupiterRouter {
    pub fn new() -> Self {
        // Get API key from config
        let api_key = with_config(|cfg| cfg.swaps.jupiter.api_key.clone())
            .expect("Jupiter API key not found in config");

        Self {
            client: Client::new(),
            api_key,
        }
    }

    /// Verify referral account is initialized for fee mint
    /// If not initialized, order will execute WITHOUT fees (silent failure)
    async fn verify_referral_token_account(&self, fee_mint: &str) -> Result<bool, ScreenerBotError> {
        // TODO: Implement check via RPC to verify referral token account exists
        // See: https://dev.jup.ag/docs/ultra/add-fees-to-ultra#create-referraltokenaccount
        // For now, log warning if feeBps doesn't match REFERRAL_FEE_BPS
        Ok(true)
    }
}

#[async_trait]
impl SwapRouter for JupiterRouter {
    fn id(&self) -> &'static str {
        "jupiter"
    }

    fn name(&self) -> &'static str {
        "Jupiter Ultra"
    }

    fn is_enabled(&self) -> bool {
        with_config(|cfg| cfg.swaps.jupiter.enabled)
    }

    fn priority(&self) -> u8 {
        0 // Highest priority (primary)
    }

    async fn get_quote(&self, request: &QuoteRequest) -> Result<Quote, ScreenerBotError> {
        // Build Ultra API order request
        let order_url = format!("{}/order", JUPITER_ULTRA_API_BASE);

        let order_request = JupiterOrderRequest {
            input_mint: request.input_mint.clone(),
            output_mint: request.output_mint.clone(),
            amount: request.input_amount.to_string(),
            taker: request.wallet_address.clone(),
            referral_account: REFERRAL_ACCOUNT.to_string(),
            referral_fee: REFERRAL_FEE_BPS, // HARDCODED 0.5%
        };

        logger::debug(
            LogTag::Swap,
            &format!(
                "Jupiter Ultra API /order request: input={} output={} amount={} referralFee={}bps",
                order_request.input_mint, order_request.output_mint,
                order_request.amount, REFERRAL_FEE_BPS
            )
        );

        // Send request with API key
        let response = self.client
            .get(&order_url)
            .header("x-api-key", &self.api_key)
            .query(&order_request)
            .send()
            .await
            .map_err(|e| ScreenerBotError::network_error(
                format!("Jupiter Ultra API request failed: {}", e)
            ))?;

        let order_response: JupiterOrderResponse = response
            .json()
            .await
            .map_err(|e| ScreenerBotError::parse_error(
                format!("Jupiter Ultra API response parse failed: {}", e)
            ))?;

        // Check for API errors
        if let Some(error_msg) = order_response.error_message {
            return Err(ScreenerBotError::api_error(
                format!("Jupiter Ultra API error: {}", error_msg)
            ));
        }

        // Validate transaction exists
        let transaction_base64 = order_response.transaction
            .ok_or_else(|| ScreenerBotError::api_error(
                "Jupiter Ultra API returned no transaction"
            ))?;

        let output_amount = order_response.output_amount
            .ok_or_else(|| ScreenerBotError::api_error(
                "Jupiter Ultra API returned no output amount"
            ))?
            .parse::<u64>()
            .map_err(|e| ScreenerBotError::parse_error(
                format!("Invalid output amount: {}", e)
            ))?;

        // CRITICAL: Verify referral fee was applied
        let actual_fee_bps = order_response.fee_bps.unwrap_or(0);
        if actual_fee_bps != REFERRAL_FEE_BPS {
            logger::warning(
                LogTag::Swap,
                &format!(
                    "⚠️ Jupiter referral fee mismatch! Expected {}bps, got {}bps. \
                    Referral token account for mint {} may not be initialized!",
                    REFERRAL_FEE_BPS, actual_fee_bps,
                    order_response.fee_mint.as_deref().unwrap_or("UNKNOWN")
                )
            );
        }

        // Log platform fee details
        if let Some(platform_fee) = &order_response.platform_fee {
            logger::info(
                LogTag::Swap,
                &format!(
                    "Jupiter platform fee: {} ({}bps) - mint: {}",
                    platform_fee.amount,
                    platform_fee.fee_bps,
                    order_response.fee_mint.as_deref().unwrap_or("UNKNOWN")
                )
            );
        }

        Ok(Quote {
            router_id: self.id().to_string(),
            router_name: self.name().to_string(),
            output_amount,
            price_impact_pct: order_response.price_impact.unwrap_or(0.0),
            fee_lamports: 0, // Fee taken from output amount
            slippage_bps: (request.slippage_pct * 100.0) as u16,
            route_plan: format!("Jupiter Ultra - Fee Mint: {}",
                order_response.fee_mint.as_deref().unwrap_or("UNKNOWN")),
            execution_data: serde_json::to_vec(&serde_json::json!({
                "transaction": transaction_base64,
                "requestId": order_response.request_id,
            })).unwrap(),
        })
    }

    async fn execute_swap(&self, token: &Token, quote: &Quote) -> Result<SwapResult, ScreenerBotError> {
        // Extract execution data
        let exec_data: serde_json::Value = serde_json::from_slice(&quote.execution_data)
            .map_err(|e| ScreenerBotError::parse_error(
                format!("Invalid quote execution data: {}", e)
            ))?;

        let transaction_base64 = exec_data["transaction"]
            .as_str()
            .ok_or_else(|| ScreenerBotError::internal_error(
                "Missing transaction in quote execution data"
            ))?;

        let request_id = exec_data["requestId"]
            .as_str()
            .ok_or_else(|| ScreenerBotError::internal_error(
                "Missing requestId in quote execution data"
            ))?;

        // Sign transaction
        let signed_transaction = self.sign_transaction(transaction_base64).await?;

        // Execute via Ultra API
        let execute_url = format!("{}/execute", JUPITER_ULTRA_API_BASE);

        let execute_request = JupiterExecuteRequest {
            signed_transaction,
            request_id: request_id.to_string(),
        };

        logger::debug(
            LogTag::Swap,
            &format!("Jupiter Ultra API /execute request: requestId={}", request_id)
        );

        let response = self.client
            .post(&execute_url)
            .header("x-api-key", &self.api_key)
            .header("Content-Type", "application/json")
            .json(&execute_request)
            .send()
            .await
            .map_err(|e| ScreenerBotError::network_error(
                format!("Jupiter Ultra API execute failed: {}", e)
            ))?;

        let execute_response: JupiterExecuteResponse = response
            .json()
            .await
            .map_err(|e| ScreenerBotError::parse_error(
                format!("Jupiter Ultra API execute response parse failed: {}", e)
            ))?;

        // Check execution status
        if execute_response.status != "Success" {
            return Err(ScreenerBotError::blockchain_error(
                format!(
                    "Jupiter swap execution failed: {}",
                    execute_response.error_message.unwrap_or_else(|| "Unknown error".to_string())
                )
            ));
        }

        let signature = execute_response.signature
            .ok_or_else(|| ScreenerBotError::internal_error(
                "Jupiter execute response missing signature"
            ))?;

        Ok(SwapResult {
            success: true,
            router_id: self.id().to_string(),
            router_name: self.name().to_string(),
            transaction_signature: signature,
            input_amount: quote.input_amount,
            output_amount: quote.output_amount,
            price_impact_pct: quote.price_impact_pct,
            fee_lamports: quote.fee_lamports,
            execution_time_ms: 0, // TODO: Track timing
            effective_price_sol: None, // TODO: Calculate
        })
    }

    async fn sign_transaction(&self, transaction_base64: &str) -> Result<String, ScreenerBotError> {
        // TODO: Implement transaction signing
        // 1. Deserialize base64 transaction
        // 2. Sign with wallet keypair
        // 3. Serialize to base64
        todo!("Implement transaction signing")
    }
}
```

**Jupiter Ultra API Setup (Required Before Use):**

1. **Get API Key:**
   - Visit https://portal.jup.ag
   - Register and get API key (free tier: 60 req/min)
   - Add to config: `[swaps.jupiter] api_key = "your-key-here"`

2. **Create Referral Account (ONE TIME):**

   ```bash
   npm install @jup-ag/referral-sdk @solana/web3.js@1
   ```

   ```javascript
   import { ReferralProvider } from "@jup-ag/referral-sdk";
   const projectPubKey = new PublicKey("DkiqsTrw1u1bYFumumC7sCG2S8K25qc2vemJFHyW2wJc"); // Jupiter Ultra
   const tx = await provider.initializeReferralAccountWithName({
     payerPubKey: wallet.publicKey,
     partnerPubKey: wallet.publicKey,
     projectPubKey: projectPubKey,
     name: "screenerbot",
   });
   // Save referralAccountPubKey and replace REFERRAL_ACCOUNT constant
   ```

3. **Create Referral Token Accounts (for SOL/USDC/USDT):**

   ```javascript
   // Do this for each token mint you want to collect fees in
   const mints = [
     "So11111111111111111111111111111111111111112", // SOL
     "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v", // USDC
     "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB", // USDT
   ];
   for (const mint of mints) {
     const tx = await provider.initializeReferralTokenAccountV2({
       payerPubKey: wallet.publicKey,
       referralAccountPubKey: new PublicKey("YOUR_REFERRAL_ACCOUNT"),
       mint: new PublicKey(mint),
     });
   }
   ```

4. **Claim Fees (Periodic):**
   ```javascript
   const txs = await provider.claimAllV2({
     payerPubKey: wallet.publicKey,
     referralAccountPubKey: new PublicKey("YOUR_REFERRAL_ACCOUNT"),
   });
   // Fee split: 80% to you, 20% to Jupiter
   ```

**Revenue Model:**

- **Fee:** 0.5% (50 bps) on all swaps
- **Split:** 80% ScreenerBot, 20% Jupiter
- **Example:** $1000 swap = $5 fee → $4 ScreenerBot, $1 Jupiter
- **NOT CONFIGURABLE:** Fee is hardcoded in `REFERRAL_FEE_BPS` constant

#### GMGN Router (`src/swaps/routers/gmgn.rs`)

```rust
pub struct GmgnRouter;

impl GmgnRouter {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl SwapRouter for GmgnRouter {
    fn id(&self) -> &'static str {
        "gmgn"
    }

    fn name(&self) -> &'static str {
        "GMGN"
    }

    fn is_enabled(&self) -> bool {
        with_config(|cfg| cfg.swaps.gmgn.enabled)
    }

    fn priority(&self) -> u8 {
        1 // Secondary priority
    }

    async fn get_quote(&self, request: &QuoteRequest) -> Result<Quote, ScreenerBotError> {
        todo!("Implement GMGN quote fetching")
    }

    async fn execute_swap(&self, token: &Token, quote: &Quote) -> Result<SwapResult, ScreenerBotError> {
        todo!("Implement GMGN swap execution")
    }
}
```

#### Raydium Router (`src/swaps/routers/raydium.rs`)

```rust
pub struct RaydiumRouter;

impl RaydiumRouter {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl SwapRouter for RaydiumRouter {
    fn id(&self) -> &'static str {
        "raydium"
    }

    fn name(&self) -> &'static str {
        "Raydium"
    }

    fn is_enabled(&self) -> bool {
        with_config(|cfg| cfg.swaps.raydium.enabled)
    }

    fn priority(&self) -> u8 {
        2 // Tertiary priority
    }

    async fn get_quote(&self, request: &QuoteRequest) -> Result<Quote, ScreenerBotError> {
        todo!("Implement Raydium quote fetching")
    }

    async fn execute_swap(&self, token: &Token, quote: &Quote) -> Result<SwapResult, ScreenerBotError> {
        todo!("Implement Raydium swap execution")
    }
}
```

---

### 5. **Public API** (`src/swaps/mod.rs`)

```rust
mod router;
mod registry;
mod operations;
mod routers;

// Re-export public API
pub use router::{SwapRouter, Quote, QuoteRequest, SwapResult, SwapMode};
pub use registry::{RouterRegistry, get_registry};
pub use operations::{get_best_quote, execute_swap_with_fallback};

// Backwards compatibility helpers (during migration)
pub use routers::{JupiterRouter, GmgnRouter, RaydiumRouter};
```

---

## Migration Strategy

### Phase 1: Add New Code (No Breaking Changes)

1. Create `router.rs` trait
2. Create `registry.rs`
3. Create `operations.rs`
4. Create `routers/jupiter.rs`, `routers/gmgn.rs`, `routers/raydium.rs`
5. Keep old code intact

### Phase 2: Implement Trait Methods

1. **Setup Jupiter Ultra API** (CRITICAL - do this first):
   - Get API key from https://portal.jup.ag
   - Create referral account via @jup-ag/referral-sdk
   - Initialize referral token accounts for SOL/USDC/USDT
   - Add API key to config: `[swaps.jupiter] api_key = "..."`
2. Move Jupiter quote/execute logic into `JupiterRouter::get_quote/execute_swap`
   - Replace `lite-api.jup.ag/swap/v1/*` with `api.jup.ag/ultra/v1/*`
   - Add `x-api-key` header to all requests
   - Add `referralAccount` and `referralFee=50` to /order requests
   - Verify `feeBps` in response matches `REFERRAL_FEE_BPS`
3. Move GMGN quote/execute logic into `GmgnRouter::get_quote/execute_swap`
4. Implement Raydium if needed
5. All existing functions delegate to trait implementations

### Phase 3: Update Call Sites

1. Replace `get_best_quote()` calls with `operations::get_best_quote()`
2. Replace `execute_best_swap()` calls with `operations::execute_swap_with_fallback()`
3. Update positions module
4. Update trader module

### Phase 4: Remove Old Code

1. Delete old `UnifiedQuote`, `QuoteExecutionData` enums
2. Delete old match-based logic
3. Delete old `get_best_quote()`, `execute_best_swap()` functions
4. Clean up imports

---

## Benefits

### 1. **Trivial Router Addition**

Add new router in **3 steps**:

1. Create `src/swaps/routers/newrouter.rs`
2. Implement `SwapRouter` trait
3. Add to registry: `Arc::new(NewRouter::new())`

**No other code changes needed.**

### 2. **Automatic Fallback Chain**

Priority-based fallback:

- Jupiter (priority 0) fails → try GMGN (priority 1) → try Raydium (priority 2)
- Configurable via `priority()` method
- No hardcoded pairs

### 3. **Easy Enable/Disable**

Change config:

```toml
[swaps.jupiter]
enabled = false  # Router automatically excluded from registry
```

### 4. **No Match Statement Sprawl**

Zero match statements on router types. All dispatch via trait methods.

### 5. **Testable**

Mock routers for testing:

```rust
struct MockRouter;
impl SwapRouter for MockRouter {
    // Test implementation
}
```

### 6. **Future-Proof**

- Add router plugins
- Runtime router discovery
- External router registration
- Multi-chain support (just add more routers)

---

## Code Comparison

### Current (Adding Raydium)

```
❌ Edit types.rs - add RouterType::Raydium
❌ Edit mod.rs line 59 - add QuoteExecutionData::Raydium
❌ Edit mod.rs line 99-110 - add Raydium quote future
❌ Edit mod.rs line 502-680 - add Raydium execution match
❌ Edit mod.rs line 755-840 - add Raydium fallback (2 places)
❌ Edit mod.rs line 845-900 - add Raydium fallback execution
✅ Add connectivity monitor (already done)
Total: ~120 lines changed across 7 locations
```

### New Design (Adding Raydium)

```
✅ Create routers/raydium.rs - implement SwapRouter trait
✅ Edit registry.rs line 12 - add Arc::new(RaydiumRouter::new())
Total: ~80 lines in NEW file, 1 line changed in existing code
```

---

## Referral Fee Monitoring & Collection

### Fee Tracking

- All Jupiter swaps automatically include `referralFee=50` (0.5%) parameter
- Response `feeBps` field confirms fee was applied
- Response `feeMint` field shows which token fees are collected in (SOL/USDC/USDT)
- `platformFee.amount` shows exact fee amount in that mint

### Warning System

If `feeBps` in response doesn't match `REFERRAL_FEE_BPS` (50):

- **Cause:** Referral token account for `feeMint` not initialized
- **Impact:** Swap executes WITHOUT your fee (lost revenue)
- **Action:** Initialize referral token account for that mint via `@jup-ag/referral-sdk`

### Fee Collection (Manual - Periodic)

```javascript
import { ReferralProvider } from "@jup-ag/referral-sdk";

const provider = new ReferralProvider(connection);
const txs = await provider.claimAllV2({
  payerPubKey: wallet.publicKey,
  referralAccountPubKey: new PublicKey("YOUR_REFERRAL_ACCOUNT"),
});

// Batch claims: 5 per transaction
// Fee split: 80% ScreenerBot, 20% Jupiter
```

### Revenue Estimation

- **Average swap:** $500 USD
- **Fee per swap:** $2.50 (0.5%)
- **ScreenerBot share:** $2.00 (80%)
- **Jupiter share:** $0.50 (20%)
- **100 swaps/day:** $200/day revenue
- **1000 swaps/day:** $2000/day revenue

### Dashboard Integration (Future)

- Track total fees collected per mint
- Show revenue in USD
- Auto-claim fees when threshold reached
- Alert when referral token account missing

---

## File Structure

```
src/swaps/
├── mod.rs                 # Public API, re-exports
├── router.rs              # SwapRouter trait, Quote/SwapResult types
├── registry.rs            # RouterRegistry, global instance
├── operations.rs          # get_best_quote, execute_swap_with_fallback
└── routers/
    ├── mod.rs             # Router re-exports
    ├── jupiter.rs         # JupiterRouter implementation
    ├── gmgn.rs            # GmgnRouter implementation
    └── raydium.rs         # RaydiumRouter implementation
```

---

## Config Changes

Config needs new field for Jupiter API key in `src/config/schemas/swaps.rs`:

```rust
config_struct! {
    pub struct JupiterConfig {
        #[metadata(field_metadata! {
            label: "Enabled",
            hint: "Enable Jupiter router (finds best routes across DEXes)",
            impact: "high",
            category: "Router",
        })]
        enabled: bool = true,

        // NEW: Jupiter Ultra API Key (REQUIRED)
        #[metadata(field_metadata! {
            label: "API Key",
            hint: "Jupiter Ultra API key from https://portal.jup.ag (required for all swaps)",
            impact: "critical",
            category: "API",
            secret: true, // Mark as sensitive
        })]
        api_key: String = "".to_string(),

        #[metadata(field_metadata! {
            label: "Dynamic CU Limit",
            hint: "Let Jupiter calculate compute units",
            impact: "medium",
            category: "Performance",
        })]
        dynamic_compute_unit_limit: bool = false,

        // ... rest of fields
    }
}
```

User's `data/config.toml`:

```toml
[swaps.jupiter]
enabled = true
api_key = "your-api-key-from-portal-jup-ag" # NEW: Get from https://portal.jup.ag
dynamic_compute_unit_limit = false
default_priority_fee = 1000
# ... existing fields

[swaps.gmgn]
enabled = false
partner = "screenerbot"
# ... existing fields

[swaps.raydium]
enabled = false
slippage_bps = 100
# ... existing fields
```

**Priority is defined in code** (`priority()` method), not config.  
This prevents user misconfiguration.

**Referral fee (0.5%) is HARDCODED** in `JupiterRouter::REFERRAL_FEE_BPS` constant.  
This cannot be changed via config - it is ScreenerBot's revenue source.

**API key is REQUIRED** - startup validation should fail if empty:

```rust
// In config validation
if config.swaps.jupiter.enabled && config.swaps.jupiter.api_key.is_empty() {
    return Err("Jupiter API key required when Jupiter router is enabled. Get key from https://portal.jup.ag".to_string());
}
```

---

## Performance

**No Performance Loss:**

- Trait dispatch via vtable (single pointer dereference)
- Same async execution as current code
- Registry lookup: O(n) where n = 3-5 routers (negligible)
- Quote fetching still fully concurrent

**Memory:**

- RouterRegistry: ~240 bytes (3 Arc pointers + Vec overhead)
- Per-quote overhead: ~0 bytes (same as current)

---

## Backwards Compatibility During Migration

During migration, keep both systems:

```rust
// Old API (deprecated)
#[deprecated(note = "Use operations::get_best_quote")]
pub async fn get_best_quote_old(...) -> Result<UnifiedQuote> {
    // Delegates to new system
    operations::get_best_quote(...).await
}
```

This allows gradual migration without breaking existing code.

---

## Troubleshooting

### "Jupiter API key required" Error

**Cause:** Missing or empty `api_key` in config  
**Fix:** Get API key from https://portal.jup.ag and add to `data/config.toml`:

```toml
[swaps.jupiter]
api_key = "your-api-key-here"
```

### "feeBps mismatch" Warning

**Cause:** Referral token account not initialized for `feeMint`  
**Impact:** Swap executes but you don't collect fees (lost revenue)  
**Fix:** Initialize referral token account:

```javascript
const provider = new ReferralProvider(connection);
const tx = await provider.initializeReferralTokenAccountV2({
  payerPubKey: wallet.publicKey,
  referralAccountPubKey: new PublicKey("YOUR_REFERRAL_ACCOUNT"),
  mint: new PublicKey("So11111111111111111111111111111111111111112"), // SOL
});
```

### "Rate limit exceeded" Error

**Cause:** Exceeded 60 requests/minute on free tier  
**Fix Options:**

1. Implement request queueing in `JupiterRouter`
2. Upgrade to paid tier at https://portal.jup.ag
3. Reduce swap frequency

### "Migration deadline" Warning

**Cause:** Still using deprecated Lite API after Dec 31 2025  
**Fix:** Migrate to Ultra API (this redesign does it automatically)

### Missing Referral Account

**Cause:** `REFERRAL_ACCOUNT` constant is placeholder  
**Fix:** Create referral account once:

```javascript
const tx = await provider.initializeReferralAccountWithName({
  payerPubKey: wallet.publicKey,
  partnerPubKey: wallet.publicKey,
  projectPubKey: new PublicKey("DkiqsTrw1u1bYFumumC7sCG2S8K25qc2vemJFHyW2wJc"),
  name: "screenerbot",
});
console.log("Referral Account:", tx.referralAccountPubKey.toBase58());
// Replace REFERRAL_ACCOUNT constant with this value
```

### Fees Not Being Collected

**Check:**

1. ✅ Referral account created?
2. ✅ Referral token accounts initialized for SOL/USDC/USDT?
3. ✅ `feeBps` in logs matches 50?
4. ✅ `feeMint` in logs is a mint you initialized?

**Debug:**

```bash
# Search logs for fee confirmation
grep "feeBps" logs/screenerbot_*.log
grep "platform fee" logs/screenerbot_*.log
```

---

## Summary

**Current System:**

- ❌ Hardcoded routers in match statements
- ❌ Manual fallback logic
- ❌ 7+ file changes to add router
- ❌ High bug risk
- ❌ Using deprecated Lite API (ends Dec 31 2025)
- ❌ No revenue from swaps

**New System:**

- ✅ Trait-based dispatch
- ✅ Automatic fallback via priority
- ✅ 1 file to add router
- ✅ Type-safe, compile-time checks
- ✅ Zero compatibility code
- ✅ Clean, maintainable, extensible
- ✅ Jupiter Ultra API (production-ready with API key)
- ✅ 0.5% referral fee revenue (80% ScreenerBot, 20% Jupiter)
- ✅ Fee collection infrastructure via Jupiter SDK

**Business Impact:**

- **Revenue Generation:** 0.5% on all swaps (hardcoded, not configurable)
- **API Compliance:** Migration to Ultra API required by Dec 31 2025
- **Scalability:** Free tier 60 req/min, paid tiers available
- **Professional:** Industry-standard referral program integration

**This is the fundamental, systematic solution for router management with revenue generation.**

---

## Quick Reference

### Jupiter Ultra API Migration Checklist

**1. Get API Key (5 minutes)**

- Visit https://portal.jup.ag
- Create account and generate API key
- Free tier: 60 requests/minute
- Add to config: `[swaps.jupiter] api_key = "..."`

**2. Create Referral Account (ONE TIME - 10 minutes)**

```bash
npm install @jup-ag/referral-sdk @solana/web3.js@1
```

```javascript
// Create referral account
const tx = await provider.initializeReferralAccountWithName({
  payerPubKey: wallet.publicKey,
  partnerPubKey: wallet.publicKey,
  projectPubKey: new PublicKey("DkiqsTrw1u1bYFumumC7sCG2S8K25qc2vemJFHyW2wJc"),
  name: "screenerbot",
});
console.log("SAVE THIS:", tx.referralAccountPubKey.toBase58());
```

**3. Initialize Token Accounts (ONE TIME - 15 minutes)**

```javascript
// SOL, USDC, USDT - most common fee mints
const mints = [
  "So11111111111111111111111111111111111111112",
  "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
  "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB",
];
for (const mint of mints) {
  await provider.initializeReferralTokenAccountV2({
    payerPubKey: wallet.publicKey,
    referralAccountPubKey: new PublicKey("YOUR_REFERRAL_ACCOUNT"),
    mint: new PublicKey(mint),
  });
}
```

**4. Update Code Constants**

```rust
// In src/swaps/routers/jupiter.rs
const REFERRAL_ACCOUNT: &str = "ABC123..."; // Replace with your account from step 2
const REFERRAL_FEE_BPS: u16 = 50; // 0.5% - DO NOT CHANGE
```

**5. Periodic Fee Collection (Weekly/Monthly)**

```javascript
const txs = await provider.claimAllV2({
  payerPubKey: wallet.publicKey,
  referralAccountPubKey: new PublicKey("YOUR_REFERRAL_ACCOUNT"),
});
// Auto-batches 5 claims per transaction
// 80% to you, 20% to Jupiter
```

### Key Constants

```rust
// Jupiter Ultra API
const JUPITER_ULTRA_API_BASE: &str = "https://api.jup.ag/ultra/v1";
const REFERRAL_FEE_BPS: u16 = 50; // 0.5% HARDCODED
const REFERRAL_ACCOUNT: &str = "YOUR_REFERRAL_ACCOUNT_PUBKEY_HERE";

// Endpoints
GET  /ultra/v1/order    // Get quote with fee parameters
POST /ultra/v1/execute  // Execute signed transaction

// Headers
x-api-key: "your-api-key-from-portal"
Content-Type: "application/json"

// Fee Split
80% ScreenerBot
20% Jupiter
```

### Validation Steps

**Before going live:**

1. ✅ API key added to config
2. ✅ Referral account created
3. ✅ Token accounts initialized (SOL/USDC/USDT minimum)
4. ✅ `REFERRAL_ACCOUNT` constant updated in code
5. ✅ Test swap shows `feeBps: 50` in logs
6. ✅ Test swap shows correct `feeMint` in logs

**Monitor in production:**

```bash
# Check fee collection
grep "feeBps" logs/screenerbot_*.log | tail -20

# Check fee warnings
grep "feeBps mismatch" logs/screenerbot_*.log

# Verify referral account
grep "referralAccount" logs/screenerbot_*.log | tail -5
```

### Revenue Tracking

```bash
# Count successful swaps with fees
grep "feeBps: 50" logs/screenerbot_*.log | wc -l

# List fee mints used
grep "feeMint" logs/screenerbot_*.log | grep -oE '"[A-Za-z0-9]{32,}"' | sort | uniq -c

# Estimate revenue (manual calculation)
# swaps_count * average_swap_usd * 0.005 * 0.80 = your_revenue
```
