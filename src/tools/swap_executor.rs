//! Tool Swap Executor
//!
//! Execute swaps using the swaps module but with custom keypairs.
//! These functions are for tools (like Volume Aggregator) that need
//! to execute swaps WITHOUT creating positions in the position tracker.

use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;
use std::str::FromStr;

use crate::config::with_config;
use crate::constants::SOL_MINT;
use crate::logger::{self, LogTag};
use crate::rpc::{get_rpc_client, RpcClientMethods};
use crate::swaps::registry::get_registry;
use crate::swaps::router::{Quote, QuoteRequest, SwapMode};
use crate::wallets::WalletWithKey;

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
    wallet: &WalletWithKey,
    input_mint: &str,
    output_mint: &str,
    input_amount: u64,
    slippage_pct: Option<f64>,
) -> Result<ToolSwapResult, String> {
    let wallet_address = wallet.wallet.address.clone();
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
    };

    // Get quote from registry (uses best available router)
    let registry = get_registry();
    let enabled = registry.enabled_routers();

    if enabled.is_empty() {
        return Err("No swap routers enabled".to_string());
    }

    // Get quote from first enabled router (Jupiter preferred)
    let router = &enabled[0];
    let quote = router
        .get_quote(&quote_request)
        .await
        .map_err(|e| format!("Failed to get quote: {}", e))?;

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

    // Execute the swap with custom keypair
    let signature = execute_swap_with_keypair(&quote, &wallet.keypair).await?;

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
    wallet: &WalletWithKey,
    token_mint: &str,
    amount_sol: f64,
    slippage_pct: Option<f64>,
) -> Result<ToolSwapResult, String> {
    // Validate token mint
    Pubkey::from_str(token_mint).map_err(|e| format!("Invalid token mint: {}", e))?;

    // Convert SOL to lamports
    let lamports = (amount_sol * 1_000_000_000.0) as u64;

    if lamports < 1_000_000 {
        return Err("Amount too small (minimum 0.001 SOL)".to_string());
    }

    logger::info(
        LogTag::Tools,
        &format!(
            "Tool buy: {} SOL -> {} via wallet {}",
            amount_sol,
            &token_mint[..8],
            &wallet.wallet.address[..8]
        ),
    );

    execute_tool_swap(wallet, SOL_MINT, token_mint, lamports, slippage_pct).await
}

/// Sell token for SOL
///
/// Executes a Token -> SOL swap using the provided wallet.
/// Does NOT create positions - for tool use only.
pub async fn tool_sell(
    wallet: &WalletWithKey,
    token_mint: &str,
    token_amount: u64,
    slippage_pct: Option<f64>,
) -> Result<ToolSwapResult, String> {
    // Validate token mint
    Pubkey::from_str(token_mint).map_err(|e| format!("Invalid token mint: {}", e))?;

    if token_amount == 0 {
        return Err("Token amount cannot be zero".to_string());
    }

    logger::info(
        LogTag::Tools,
        &format!(
            "Tool sell: {} tokens of {} -> SOL via wallet {}",
            token_amount,
            &token_mint[..8],
            &wallet.wallet.address[..8]
        ),
    );

    execute_tool_swap(wallet, token_mint, SOL_MINT, token_amount, slippage_pct).await
}

/// Execute a swap transaction signed with a specific keypair
async fn execute_swap_with_keypair(quote: &Quote, keypair: &Keypair) -> Result<String, String> {
    // Deserialize quote response from execution_data
    let quote_response: serde_json::Value = serde_json::from_slice(&quote.execution_data)
        .map_err(|e| format!("Quote deserialization failed: {}", e))?;

    // Build swap request for Jupiter API
    let swap_req = serde_json::json!({
        "userPublicKey": keypair.pubkey().to_string(),
        "quoteResponse": quote_response,
        "dynamicComputeUnitLimit": true,
        "prioritizationFeeLamports": with_config(|cfg| cfg.swaps.jupiter.default_priority_fee),
    });

    // Call Jupiter swap endpoint
    let client = reqwest::Client::new();
    let response = client
        .post("https://api.jup.ag/swap/v1/swap")
        .header("x-api-key", "YOUR_JUPITER_API_KEY")
        .header("Content-Type", "application/json")
        .json(&swap_req)
        .send()
        .await
        .map_err(|e| format!("Jupiter swap request failed: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown".to_string());
        return Err(format!("Jupiter swap failed ({}): {}", status, error_text));
    }

    #[derive(serde::Deserialize)]
    struct JupiterSwapResponse {
        #[serde(rename = "swapTransaction")]
        swap_transaction: String,
    }

    let swap_response: JupiterSwapResponse = response
        .json()
        .await
        .map_err(|e| format!("Jupiter swap response parse failed: {}", e))?;

    // Sign and send using the provided keypair
    let rpc_client = get_rpc_client();
    let signature = rpc_client
        .sign_send_and_confirm_with_keypair(&swap_response.swap_transaction, keypair)
        .await
        .map_err(|e| format!("Transaction failed: {}", e))?;

    Ok(signature.to_string())
}
