/// Swap module for handling multiple DEX routers
/// Supports GMGN and Jupiter routers with automatic best route selection

pub mod gmgn;
pub mod jupiter;

use crate::tokens::Token;
use crate::rpc::SwapError;
use crate::logger::{log, LogTag};

/// Represents a router type for swap operations
#[derive(Debug, Clone, PartialEq)]
pub enum RouterType {
    GMGN,
    Jupiter,
}

/// Unified swap quote structure for comparing routes across different routers
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
    pub route_plan: String, // JSON string of route information
    pub execution_data: QuoteExecutionData, // Router-specific execution data
}

/// Router-specific execution data for performing swaps
#[derive(Debug, Clone)]
pub enum QuoteExecutionData {
    GMGN(gmgn::SwapData),
    Jupiter(jupiter::JupiterSwapData), // Placeholder for Jupiter data
}

/// Unified swap result structure
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

/// Gets quotes from all available routers and returns the best one
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
            if input_mint == crate::wallet::SOL_MINT { "SOL" } else { &input_mint[..8] },
            if output_mint == crate::wallet::SOL_MINT { "SOL" } else { &output_mint[..8] },
            input_amount
        )
    );

    let mut quotes = Vec::new();

    // Get GMGN quote
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
            quotes.push(unified_quote);
        }
        Err(e) => {
            log(LogTag::Swap, "WARNING", &format!("GMGN quote failed: {}", e));
        }
    }

    // Get Jupiter quote (placeholder)
    match jupiter::get_jupiter_quote(
        input_mint,
        output_mint,
        input_amount,
        from_address,
        slippage,
    ).await {
        Ok(jupiter_data) => {
            let unified_quote = UnifiedQuote {
                router: RouterType::Jupiter,
                input_mint: input_mint.to_string(),
                output_mint: output_mint.to_string(),
                input_amount,
                output_amount: jupiter_data.out_amount,
                price_impact_pct: jupiter_data.price_impact_pct,
                fee_lamports: jupiter_data.prioritization_fee_lamports,
                slippage_bps: (slippage * 100.0) as u16,
                route_plan: jupiter_data.route_plan.clone(),
                execution_data: QuoteExecutionData::Jupiter(jupiter_data),
            };
            quotes.push(unified_quote);
        }
        Err(e) => {
            log(LogTag::Swap, "WARNING", &format!("Jupiter quote failed: {}", e));
        }
    }

    if quotes.is_empty() {
        return Err(SwapError::ApiError("No routers available for quote".to_string()));
    }

    // Select best quote based on output amount (more tokens = better)
    let best_quote = quotes.into_iter()
        .max_by_key(|q| q.output_amount)
        .unwrap();

    log(
        LogTag::Swap,
        "BEST_ROUTE",
        &format!(
            "✅ Best route found: {:?} (output: {}, impact: {:.2}%, fee: {} lamports)",
            best_quote.router,
            best_quote.output_amount,
            best_quote.price_impact_pct,
            best_quote.fee_lamports
        )
    );

    Ok(best_quote)
}

/// Executes a swap using the best available router
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
            if input_mint == crate::wallet::SOL_MINT { "SOL" } else { &input_mint[..8] },
            if output_mint == crate::wallet::SOL_MINT { "SOL" } else { &output_mint[..8] },
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
