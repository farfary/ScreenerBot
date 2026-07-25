//! DCA (Dollar Cost Averaging) operations — add to an existing position.

use crate::config::with_config;
use crate::constants::SOL_MINT;
use crate::logger::{self, LogTag};
use crate::positions::price_resolution::get_price_with_api_fallback;
use crate::positions::queue::{enqueue_verification, VerificationItem};
use crate::positions::state::{
    acquire_position_lock, clear_pending_dca_swap, register_pending_dca_swap,
};
use crate::positions::types::{PendingDcaSwap, TradeOrigin};
use crate::rpc::{get_rpc_client, RpcClientMethods};
use crate::swaps::{
    execute_swap_with_fallback, get_best_quote_for_opening, QuoteRequest, SwapMode,
};
use crate::utils::{get_wallet_address, sol_to_lamports};
use chrono::Utc;

/// Add to an existing position (Dollar Cost Averaging)
/// CRITICAL: This does NOT consume a new semaphore permit - same position
pub async fn add_to_position(
    token_mint: &str,
    dca_amount_sol: f64,
    slippage_pct: Option<f64>,
    origin: TradeOrigin,
) -> Result<String, String> {
    // Serialize per-mint DCA operations
    let _lock = acquire_position_lock(token_mint).await;
    // Get position
    let position = crate::positions::state::get_position_by_mint(token_mint)
        .await
        .ok_or_else(|| format!("No open position found for token: {token_mint}"))?;

    let position_id = position.id.ok_or_else(|| "Position has no ID".to_owned())?;

    // The DCA config governs what the AUTO-TRADER may do on its own — it is not a
    // capability switch for the user. A manual "Add to Position" from the dashboard is
    // the user overriding the bot, so `dca_enabled`, `dca_max_count` and the cooldown
    // do not apply to it. Gating manual adds on them meant anyone who simply did not
    // want automatic DCA got "DCA is disabled in configuration" when clicking Add, and
    // a user who did want to average down was told to wait out the BOT's cooldown.
    if origin == TradeOrigin::Auto {
        let dca_enabled = with_config(|cfg| cfg.trader.dca_enabled);
        if !dca_enabled {
            return Err("DCA is disabled in configuration".to_owned());
        }

        let max_dca_count = with_config(|cfg| cfg.trader.dca_max_count);
        if position.dca_count >= max_dca_count as u32 {
            return Err(format!(
                "Maximum DCA count reached: {} (max: {})",
                position.dca_count, max_dca_count
            ));
        }

        // Check DCA cooldown
        if let Some(last_dca) = position.last_dca_time {
            let cooldown_minutes = with_config(|cfg| cfg.trader.dca_cooldown_minutes);
            let elapsed = Utc::now().signed_duration_since(last_dca).num_minutes();
            if elapsed < cooldown_minutes {
                return Err(format!(
                    "DCA cooldown active: {} minutes remaining",
                    cooldown_minutes - elapsed
                ));
            }
        }
    }

    logger::info(
        LogTag::Positions,
        &format!(
            "DCA entry initiated: {} | {} SOL | DCA #{} ",
            position.symbol,
            dca_amount_sol,
            position.dca_count + 1
        ),
    );

    // Record DCA initiation
    crate::events::record_position_event(
        &position_id.to_string(),
        token_mint,
        "dca_initiated",
        position.entry_transaction_signature.as_deref(),
        None,
        dca_amount_sol,
        0,
        None,
        None,
    )
    .await;

    // Get API token for swap
    let api_token = crate::tokens::get_full_token_async(token_mint)
        .await
        .map_err(|e| format!("Failed to get token: {e}"))?
        .ok_or_else(|| format!("Token not found: {token_mint}"))?;

    // Get quote for DCA entry
    let wallet_address =
        get_wallet_address().map_err(|e| format!("Failed to get wallet address: {e}"))?;
    // Manual override when the user set one in the trade dialog; config default otherwise.
    let slippage = super::slippage::entry_slippage(slippage_pct);
    let quote_request = QuoteRequest {
        input_mint: SOL_MINT.to_string(),
        output_mint: token_mint.to_string(),
        input_amount: sol_to_lamports(dca_amount_sol),
        wallet_address: wallet_address.clone(),
        slippage_pct: slippage,
        swap_mode: SwapMode::ExactIn,
        exclude_dexes: None,
    };
    let quote = get_best_quote_for_opening(quote_request, &api_token.symbol)
        .await
        .map_err(|e| format!("Failed to get DCA quote: {e}"))?;

    // Only scale into a UI amount when the decimals are actually known; printing raw
    // units against an assumed 9 decimals misreports the quote by orders of magnitude.
    let quoted_tokens = match api_token.decimals {
        Some(decimals) => format!(
            "{}",
            quote.output_amount as f64 / 10_f64.powi(decimals as i32)
        ),
        None => format!("{} raw", quote.output_amount),
    };
    logger::info(
        LogTag::Positions,
        &format!("DCA quote: {dca_amount_sol} SOL → {quoted_tokens} tokens"),
    );

    // Execute swap
    let swap_result = execute_swap_with_fallback(&api_token, quote)
        .await
        .map_err(|e| format!("DCA swap failed: {e}"))?;

    let transaction_signature = swap_result.transaction_signature.clone();

    // Pre-compute expiry height for verification + persistence
    let expiry_height = get_rpc_client()
        .get_block_height()
        .await
        .ok()
        .map(|h| h + super::SOLANA_BLOCKHASH_VALIDITY_SLOTS);

    // Persist pending DCA metadata before queuing verification to survive restarts
    let pending_dca = PendingDcaSwap {
        signature: transaction_signature.clone(),
        mint: token_mint.to_string(),
        position_id,
        expiry_height,
        created_at: Utc::now(),
    };

    register_pending_dca_swap(pending_dca.clone())
        .await
        .map_err(|e| {
            logger::error(
                LogTag::Positions,
                &format!(
                    "Failed to persist pending DCA metadata for position {} (mint {}): {}",
                    position_id, token_mint, e
                ),
            );
            format!("Failed to persist pending DCA metadata: {e}")
        })?;

    // Get price with fallback to API for DCA transition
    let (price_info, _price_source) =
        get_price_with_api_fallback(token_mint)
            .await
            .ok_or_else(|| {
                format!(
                    "No price data for token: {} (checked pool + API)",
                    token_mint
                )
            })?;

    let transition = crate::positions::transitions::PositionTransition::DcaSubmitted {
        position_id,
        dca_signature: transaction_signature.clone(),
        dca_amount_sol,
        market_price: price_info.price_sol,
    };

    // Apply transition
    if let Err(e) = crate::positions::apply::apply_transition(transition).await {
        clear_pending_dca_swap(&pending_dca.signature)
            .await
            .map_err(|err| {
                logger::error(
                    LogTag::Positions,
                    &format!(
                        "Failed to rollback pending DCA {} after transition error: {}",
                        pending_dca.signature, err
                    ),
                );
                err
            })
            .ok();
        return Err(format!("Failed to apply DCA transition: {e}"));
    }

    // Enqueue for verification
    let verification_item = VerificationItem::new_dca(
        transaction_signature.clone(),
        token_mint.to_string(),
        Some(position_id),
        expiry_height,
    );

    enqueue_verification(verification_item).await;

    logger::info(
        LogTag::Positions,
        &format!(
            "DCA entry submitted: {} | {} SOL | TX: {} | DCA #{}",
            api_token.symbol,
            dca_amount_sol,
            transaction_signature,
            position.dca_count + 1
        ),
    );

    // CRITICAL: Do NOT consume a new semaphore permit - same position!

    Ok(transaction_signature)
}
