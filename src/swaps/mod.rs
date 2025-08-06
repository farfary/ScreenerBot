/// Swap module for handling multiple DEX routers
/// Supports GMGN and Jupiter routers with unified interface

pub mod gmgn;
pub mod jupiter;
pub mod interface;
pub mod types;
pub mod transaction;
pub mod pricing;
pub mod execution;

#[cfg(test)]
pub mod tests;

use crate::tokens::Token;
use crate::rpc::SwapError;
use crate::logger::{log, LogTag};

// =============================================================================
// TRANSACTION CONFIRMATION DELAY CONFIGURATION
// =============================================================================

/// Initial delay before first transaction confirmation check
pub const INITIAL_CONFIRMATION_DELAY_MS: u64 = 500;

/// Maximum delay between confirmation checks (cap for exponential backoff)
pub const MAX_CONFIRMATION_DELAY_SECS: u64 = 8;

/// Exponential backoff multiplier for confirmation delays
pub const CONFIRMATION_BACKOFF_MULTIPLIER: f64 = 1.5;

/// Total timeout for transaction confirmation (in seconds)
pub const CONFIRMATION_TIMEOUT_SECS: u64 = 60;

/// Base delay for rate limit errors (in seconds)
pub const RATE_LIMIT_BASE_DELAY_SECS: u64 = 5;

/// Additional delay per consecutive rate limit error (in seconds)
pub const RATE_LIMIT_INCREMENT_SECS: u64 = 2;

/// Delay for first few attempts (in milliseconds)
pub const EARLY_ATTEMPT_DELAY_MS: u64 = 1000;

/// Number of attempts to use early delay before switching to exponential backoff
pub const EARLY_ATTEMPTS_COUNT: u32 = 3;

// =============================================================================
// RE-EXPORTS - Clean interface for external use
// =============================================================================

// Main swap functions
pub use interface::{buy_token, sell_token, SwapResult};

// Common types and structures
pub use types::{
    SwapData, SwapQuote, RawTransaction, SwapRequest, 
    GMGNApiResponse, JupiterQuoteResponse, JupiterSwapResponse, 
    SOL_MINT, ANTI_MEV, PARTNER
};

// Transaction utilities
pub use transaction::{
    check_and_reserve_transaction_slot, check_recent_transaction_attempt, 
    clear_recent_transaction_attempt, TransactionSlotGuard, get_wallet_address,
    sign_and_send_transaction, verify_transaction_and_get_actual_amounts
};

// Pricing functions
pub use pricing::{
    validate_price_near_expected, calculate_effective_price_buy, calculate_effective_price_sell,
    validate_quote_price, get_token_price_sol
};

// Execution functions
pub use execution::{get_swap_quote, execute_swap_with_quote};

// Router-specific functions
pub use gmgn::{get_gmgn_quote, execute_gmgn_swap, gmgn_sign_and_send_transaction, GMGNSwapResult};
pub use jupiter::{get_jupiter_quote, execute_jupiter_swap, jupiter_sign_and_send_transaction, JupiterSwapResult};

// =============================================================================
// UNIFIED ROUTER INTERFACE
// =============================================================================

/// Represents available swap routers
#[derive(Debug, Clone, PartialEq)]
pub enum RouterType {
    GMGN,
    Jupiter,
}

/// Unified quote structure for comparing routes
#[derive(Debug, Clone)]
pub struct UnifiedQuote {
    pub router: RouterType,
    pub input_mint: String,
    pub output_mint: String,
    pub input_amount: u64,
    pub output_amount: u64,
    pub price_impact_pct: f64,
    pub fee_lamports: u64,
    pub slippage_bps: u16,
    pub route_plan: String,
    pub execution_data: QuoteExecutionData,
}

/// Router-specific execution data
#[derive(Debug, Clone)]
pub enum QuoteExecutionData {
    GMGN(types::SwapData),
    Jupiter(types::SwapData),
}

/// Unified swap result
#[derive(Debug)]
pub struct UnifiedSwapResult {
    pub success: bool,
    pub router_used: RouterType,
    pub transaction_signature: Option<String>,
    pub input_amount: String,
    pub output_amount: String,
    pub price_impact: String,
    pub fee_lamports: u64,
    pub execution_time: f64,
    pub effective_price: Option<f64>,
    pub error: Option<String>,
}

// =============================================================================
// SIMPLIFIED BEST QUOTE FUNCTIONS
// =============================================================================

/// Get the best quote from available routers (TRUE COMPARISON IMPLEMENTATION)
pub async fn get_best_quote(
    input_mint: &str,
    output_mint: &str,
    input_amount: u64,
    from_address: &str,
    slippage: f64,
    fee: f64,
    is_anti_mev: bool,
) -> Result<UnifiedQuote, SwapError> {
    log(
        LogTag::Swap,
        "BEST_QUOTE",
        &format!(
            "🔍 Finding best route: {} -> {} (amount: {})",
            if input_mint == types::SOL_MINT { "SOL" } else { &input_mint[..8] },
            if output_mint == types::SOL_MINT { "SOL" } else { &output_mint[..8] },
            input_amount
        )
    );

    let mut quotes = Vec::new();

    // Get GMGN quote
    log(LogTag::Swap, "QUOTE_GMGN", "🔵 Getting GMGN quote...");
    match gmgn::get_gmgn_quote(
        input_mint,
        output_mint,
        input_amount,
        from_address,
        slippage,
        fee,
        is_anti_mev,
    ).await {
        Ok(gmgn_data) => {
            let unified_quote = UnifiedQuote {
                router: RouterType::GMGN,
                input_mint: input_mint.to_string(),
                output_mint: output_mint.to_string(),
                input_amount,
                output_amount: gmgn_data.quote.out_amount.parse().unwrap_or(0),
                price_impact_pct: gmgn_data.quote.price_impact_pct.parse().unwrap_or(0.0),
                fee_lamports: gmgn_data.raw_tx.prioritization_fee_lamports,
                slippage_bps: gmgn_data.quote.slippage_bps.parse().unwrap_or(0),
                route_plan: format!("GMGN Route: {}", serde_json::to_string(&gmgn_data.quote.route_plan).unwrap_or_default()),
                execution_data: QuoteExecutionData::GMGN(gmgn_data),
            };
            
            log(
                LogTag::Swap,
                "QUOTE_GMGN_SUCCESS",
                &format!(
                    "✅ GMGN quote: {} tokens, impact: {:.2}%, fee: {} lamports",
                    unified_quote.output_amount,
                    unified_quote.price_impact_pct,
                    unified_quote.fee_lamports
                )
            );
            
            quotes.push(unified_quote);
        }
        Err(e) => {
            log(LogTag::Swap, "QUOTE_GMGN_ERROR", &format!("❌ GMGN quote failed: {}", e));
        }
    }

    // Get Jupiter quote
    log(LogTag::Swap, "QUOTE_JUPITER", "🟡 Getting Jupiter quote...");
    match jupiter::get_jupiter_quote(
        input_mint,
        output_mint,
        input_amount,
        from_address,
        slippage,
        fee,
        is_anti_mev,
    ).await {
        Ok(jupiter_data) => {
            let unified_quote = UnifiedQuote {
                router: RouterType::Jupiter,
                input_mint: input_mint.to_string(),
                output_mint: output_mint.to_string(),
                input_amount,
                output_amount: jupiter_data.quote.out_amount.parse().unwrap_or(0),
                price_impact_pct: jupiter_data.quote.price_impact_pct.parse().unwrap_or(0.0),
                fee_lamports: jupiter_data.raw_tx.prioritization_fee_lamports,
                slippage_bps: jupiter_data.quote.slippage_bps.parse().unwrap_or(0),
                route_plan: format!("Jupiter Route: {}", serde_json::to_string(&jupiter_data.quote.route_plan).unwrap_or_default()),
                execution_data: QuoteExecutionData::Jupiter(jupiter_data),
            };
            
            log(
                LogTag::Swap,
                "QUOTE_JUPITER_SUCCESS",
                &format!(
                    "✅ Jupiter quote: {} tokens, impact: {:.2}%, fee: {} lamports",
                    unified_quote.output_amount,
                    unified_quote.price_impact_pct,
                    unified_quote.fee_lamports
                )
            );
            
            quotes.push(unified_quote);
        }
        Err(e) => {
            log(LogTag::Swap, "QUOTE_JUPITER_ERROR", &format!("❌ Jupiter quote failed: {}", e));
        }
    }

    // Check if we have any quotes
    if quotes.is_empty() {
        let error_msg = "No routers available for quote - both GMGN and Jupiter failed";
        log(LogTag::Swap, "QUOTE_ERROR", &format!("❌ {}", error_msg));
        return Err(SwapError::ApiError(error_msg.to_string()));
    }

    // Compare quotes and select the best one (highest output amount = better rate)
    let best_quote = quotes.iter()
        .max_by_key(|q| q.output_amount)
        .cloned()
        .ok_or_else(|| SwapError::ApiError("Failed to select best quote".to_string()))?;

    // Log comparison results if we have multiple quotes
    if quotes.len() > 1 {
        log(
            LogTag::Swap,
            "QUOTE_COMPARISON",
            &format!(
                "⚖️ Quote comparison: GMGN vs Jupiter - Winner: {:?}",
                best_quote.router
            )
        );
        
        // Show detailed comparison
        for quote in &quotes {
            log(
                LogTag::Swap,
                "QUOTE_DETAILS",
                &format!(
                    "  • {:?}: {} tokens (impact: {:.2}%, fee: {} lamports)",
                    quote.router,
                    quote.output_amount,
                    quote.price_impact_pct,
                    quote.fee_lamports
                )
            );
        }
    }

    log(
        LogTag::Swap,
        "BEST_ROUTE",
        &format!(
            "🏆 Best route selected: {:?} with {} tokens (impact: {:.2}%, fee: {} lamports)",
            best_quote.router,
            best_quote.output_amount,
            best_quote.price_impact_pct,
            best_quote.fee_lamports
        )
    );

    Ok(best_quote)
}

/// Execute swap with unified quote (simplified implementation)
pub async fn execute_best_swap(
    token: &Token,
    input_mint: &str,
    output_mint: &str,
    input_amount: u64,
    quote: UnifiedQuote,
) -> Result<UnifiedSwapResult, SwapError> {
    log(
        LogTag::Swap,
        "EXECUTE",
        &format!(
            "🚀 Executing swap via {:?}: {} -> {} (amount: {})",
            quote.router,
            if input_mint == types::SOL_MINT { "SOL" } else { &input_mint[..8] },
            if output_mint == types::SOL_MINT { "SOL" } else { &output_mint[..8] },
            input_amount
        )
    );

    match quote.execution_data {
        QuoteExecutionData::GMGN(gmgn_data) => {
            match gmgn::execute_gmgn_swap(token, input_mint, output_mint, input_amount, gmgn_data).await {
                Ok(result) => Ok(UnifiedSwapResult {
                    success: result.success,
                    router_used: RouterType::GMGN,
                    transaction_signature: result.transaction_signature,
                    input_amount: result.input_amount,
                    output_amount: result.output_amount,
                    price_impact: result.price_impact,
                    fee_lamports: result.fee_lamports,
                    execution_time: result.execution_time,
                    effective_price: result.effective_price,
                    error: result.error,
                }),
                Err(e) => Err(e),
            }
        }
        QuoteExecutionData::Jupiter(jupiter_data) => {
            match jupiter::execute_jupiter_swap(token, input_mint, output_mint, input_amount, jupiter_data).await {
                Ok(result) => Ok(UnifiedSwapResult {
                    success: result.success,
                    router_used: RouterType::Jupiter,
                    transaction_signature: result.transaction_signature,
                    input_amount: result.input_amount,
                    output_amount: result.output_amount,
                    price_impact: result.price_impact,
                    fee_lamports: result.fee_lamports,
                    execution_time: result.execution_time,
                    effective_price: result.effective_price,
                    error: result.error,
                }),
                Err(e) => Err(e),
            }
        }
    }
}
