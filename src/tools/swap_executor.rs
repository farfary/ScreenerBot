//! Tool Swap Executor
//!
//! Execute swaps for a specific wallet (identified by ID), without creating
//! positions in the position tracker. Resolving and signing with that
//! wallet's key happens inside `crate::chains::solana` — this module never
//! holds a `Keypair`.

use crate::chains::solana::constants::SOL_MINT;
use crate::config::with_config;
use crate::logger::{self, LogTag};
use crate::swaps::registry::get_registry;
use crate::swaps::types::{QuoteRequest, SwapMode};
use crate::wallets::Wallet;

/// Result of a tool swap execution
#[derive(Debug, Clone)]
pub struct ToolSwapResult {
    /// Transaction signature
    pub signature: String,
    /// Input amount (lamports for SOL, raw amount for tokens)
    pub input_amount: u64,
    /// Output amount (lamports for SOL, raw amount for tokens)
    pub output_amount: u64,
    /// Price impact percentage
    pub price_impact_pct: f64,
    /// Router used for the swap
    pub router_name: String,
}

/// Execute a tool swap with a custom keypair
///
/// This function gets a quote and executes the swap using the provided wallet.
/// Unlike regular swaps, this does NOT create positions or track in position system.
pub async fn execute_tool_swap(
    wallet: &Wallet,
    input_mint: &str,
    output_mint: &str,
    input_amount: u64,
    slippage_pct: Option<f64>,
) -> Result<ToolSwapResult, String> {
    let wallet_address = wallet.address.clone();
    let slippage =
        slippage_pct.unwrap_or_else(|| with_config(|cfg| cfg.swaps.slippage.quote_default_pct));

    // Create quote request
    let quote_request = QuoteRequest {
        input_mint: input_mint.to_string(),
        output_mint: output_mint.to_string(),
        input_amount,
        wallet_address: wallet_address.clone(),
        slippage_pct: slippage,
        swap_mode: SwapMode::ExactIn,
        exclude_dexes: None,
    };

    // Get quote from registry (uses best available router)
    let registry = get_registry();
    let enabled = registry.enabled_routers();

    if enabled.is_empty() {
        return Err("No swap routers enabled".to_owned());
    }

    // Get quote from first enabled router (Jupiter preferred)
    let router = &enabled[0];
    let quote = router
        .get_quote(&quote_request)
        .await
        .map_err(|e| format!("Failed to get quote: {e}"))?;

    logger::debug(
        LogTag::Tools,
        &format!(
            "Tool swap quote: {} -> {} (input={}, output={}, impact={:.2}%)",
            input_mint,
            output_mint,
            quote.input_amount,
            quote.output_amount,
            quote.price_impact_pct
        ),
    );

    // Execute the swap, resolving and signing with this wallet's key inside
    // crate::chains::solana — this function never sees the keypair itself.
    let signature =
        crate::chains::solana::swaps::routers::execute_for_wallet(&quote, wallet.id).await?;

    Ok(ToolSwapResult {
        signature,
        input_amount: quote.input_amount,
        output_amount: quote.output_amount,
        price_impact_pct: quote.price_impact_pct,
        router_name: quote.router_name,
    })
}

/// Buy token with SOL
///
/// Executes a SOL -> Token swap using the provided wallet.
/// Does NOT create positions - for tool use only.
pub async fn tool_buy(
    wallet: &Wallet,
    token_mint: &str,
    amount_sol: f64,
    slippage_pct: Option<f64>,
) -> Result<ToolSwapResult, String> {
    // Validate token mint
    crate::wallets::validate_address(token_mint).map_err(|e| format!("Invalid token mint: {e}"))?;

    // Convert SOL to lamports
    let lamports = (amount_sol * 1_000_000_000.0) as u64;

    if lamports < 1_000_000 {
        return Err("Amount too small (minimum 0.001 SOL)".to_owned());
    }

    logger::info(
        LogTag::Tools,
        &format!(
            "Tool buy: {} SOL -> {} via wallet {}",
            amount_sol,
            &token_mint[..8],
            &wallet.address[..8]
        ),
    );

    execute_tool_swap(wallet, SOL_MINT, token_mint, lamports, slippage_pct).await
}

/// Sell token for SOL
///
/// Executes a Token -> SOL swap using the provided wallet.
/// Does NOT create positions - for tool use only.
pub async fn tool_sell(
    wallet: &Wallet,
    token_mint: &str,
    token_amount: u64,
    slippage_pct: Option<f64>,
) -> Result<ToolSwapResult, String> {
    // Validate token mint
    crate::wallets::validate_address(token_mint).map_err(|e| format!("Invalid token mint: {e}"))?;

    if token_amount == 0 {
        return Err("Token amount cannot be zero".to_owned());
    }

    logger::info(
        LogTag::Tools,
        &format!(
            "Tool sell: {} tokens of {} -> SOL via wallet {}",
            token_amount,
            &token_mint[..8],
            &wallet.address[..8]
        ),
    );

    execute_tool_swap(wallet, token_mint, SOL_MINT, token_amount, slippage_pct).await
}
