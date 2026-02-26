//! Partial close operations — sell a percentage of remaining tokens without closing the position.

use crate::config::with_config;
use crate::constants::SOL_MINT;
use crate::logger::{self, LogTag};
use crate::positions::db::save_position;
use crate::positions::queue::{enqueue_verification, VerificationItem};
use crate::positions::state::{
    acquire_position_lock, add_signature_to_index, clear_pending_partial_exit,
    register_pending_partial_exit,
};
use crate::positions::types::PendingPartialExit;
use crate::rpc::{get_rpc_client, RpcClientMethods};
use crate::swaps::{
    calculate_partial_amount, execute_swap_with_fallback, get_best_quote, QuoteRequest, SwapMode,
};
use crate::utils::get_wallet_address;
use chrono::Utc;
use serde_json::json;
use tokio::time::{sleep, Duration};

/// Partially close a position by selling a percentage of remaining tokens
/// CRITICAL: This does NOT release the semaphore permit - position stays open
pub async fn partial_close_position(
    token_mint: &str,
    exit_percentage: f64,
    exit_reason: &str,
) -> Result<String, String> {
    // Serialize per-mint operations to avoid overlapping partials/full exits
    let _lock = acquire_position_lock(token_mint).await;

    // Validate percentage
    if exit_percentage <= 0.0 || exit_percentage >= 100.0 {
        return Err(format!(
            "Invalid exit percentage: {}. Must be between 0 and 100 (exclusive)",
            exit_percentage
        ));
    }

    // Get position
    let position = crate::positions::state::get_position_by_mint(token_mint)
        .await
        .ok_or_else(|| format!("No open position found for token: {token_mint}"))?;

    let position_id = position.id.ok_or_else(|| "Position has no ID".to_owned())?;

    // Get remaining token amount
    let remaining_amount = position
        .remaining_token_amount
        .or(position.token_amount)
        .ok_or_else(|| "Position has no token amount".to_owned())?;

    // NOTE: No wallet balance verification here to avoid RPC latency during critical exit.
    // The swap executor will fail gracefully if balance is insufficient.
    // Consider adding periodic wallet balance reconciliation in monitoring service.

    // Calculate partial exit amount
    let exit_amount = calculate_partial_amount(remaining_amount, exit_percentage);

    if exit_amount == 0 {
        return Err("Calculated exit amount is zero".to_owned());
    }

    logger::info(
        LogTag::Positions,
        &format!(
            "Partial exit initiated: {} | {}% ({} of {} tokens) | Reason: {}",
            position.symbol, exit_percentage, exit_amount, remaining_amount, exit_reason
        ),
    );

    // Record partial exit initiation
    crate::events::record_position_event(
        &position_id.to_string(),
        token_mint,
        "partial_exit_initiated",
        position.entry_transaction_signature.as_deref(),
        None,
        0.0,
        exit_amount,
        None,
        Some(exit_percentage),
    )
    .await;

    // Get API token for swap
    let api_token = crate::tokens::get_full_token_async(token_mint)
        .await
        .map_err(|e| format!("Failed to get token: {e}"))?
        .ok_or_else(|| format!("Token not found: {token_mint}"))?;

    // Get quote for partial exit
    let wallet_address =
        get_wallet_address().map_err(|e| format!("Failed to get wallet address: {e}"))?;
    let slippage_exit_retry_steps =
        with_config(|cfg| cfg.swaps.slippage.exit_retry_steps_pct.clone());
    // Slippage retry loop for partial exit
    let mut last_err: Option<String> = None;
    let mut quote_opt = None;
    for (i, slippage) in slippage_exit_retry_steps.iter().enumerate() {
        let quote_request = QuoteRequest {
            input_mint: token_mint.to_string(),
            output_mint: SOL_MINT.to_string(),
            input_amount: exit_amount,
            wallet_address: wallet_address.clone(),
            slippage_pct: *slippage,
            swap_mode: SwapMode::ExactIn,
        };
        match get_best_quote(quote_request).await {
            Ok(q) => {
                quote_opt = Some(q);
                last_err = None;
                break;
            }
            Err(e) => {
                let err_msg = e.to_string();
                last_err = Some(format!(
                    "Quote failed at step {} ({}%): {}",
                    i + 1,
                    slippage,
                    err_msg
                ));
                let err_lower = err_msg.to_lowercase();
                if err_lower.contains("429") || err_lower.contains("rate limit") {
                    logger::warning(
                        LogTag::Positions,
                        "Jupiter rate limit hit, backing off 10 seconds before retry",
                    );
                    sleep(Duration::from_secs(10)).await;
                } else {
                    sleep(Duration::from_secs(2)).await;
                }
                continue;
            }
        }
    }
    let quote = quote_opt
        .ok_or_else(|| last_err.unwrap_or_else(|| "Failed to get exit quote".to_owned()))?;

    logger::info(
        LogTag::Positions,
        &format!(
            "Partial exit quote: {} tokens → {} SOL",
            exit_amount,
            quote.output_amount as f64 / 1_000_000_000.0
        ),
    );

    // Mark pending partial BEFORE executing swap to serialize concurrent attempts
    crate::positions::state::mark_partial_exit_pending(token_mint).await;

    // Execute swap with retry on different slippage levels
    let mut swap_result = execute_swap_with_fallback(&api_token, quote)
        .await
        .map_err(|e| format!("Swap failed: {e}"));
    if swap_result.is_err() {
        for (i, slippage) in slippage_exit_retry_steps.iter().enumerate() {
            let quote_request = QuoteRequest {
                input_mint: token_mint.to_string(),
                output_mint: SOL_MINT.to_string(),
                input_amount: exit_amount,
                wallet_address: wallet_address.clone(),
                slippage_pct: *slippage,
                swap_mode: SwapMode::ExactIn,
            };
            let q = match get_best_quote(quote_request).await {
                Ok(q) => q,
                Err(e) => {
                    let err_msg = e.to_string();
                    last_err = Some(format!(
                        "Quote failed at step {} ({}%): {}",
                        i + 1,
                        slippage,
                        err_msg
                    ));
                    let err_lower = err_msg.to_lowercase();
                    if err_lower.contains("429") || err_lower.contains("rate limit") {
                        logger::warning(
                            LogTag::Positions,
                            "Jupiter rate limit hit, backing off 10 seconds before retry",
                        );
                        sleep(Duration::from_secs(10)).await;
                    } else {
                        sleep(Duration::from_secs(2)).await;
                    }
                    continue;
                }
            };
            match execute_swap_with_fallback(&api_token, q).await {
                Ok(res) => {
                    swap_result = Ok(res);
                    last_err = None;
                    break;
                }
                Err(e) => {
                    let err_msg = e.to_string();
                    last_err = Some(format!(
                        "Partial exit swap failed at step {} ({}%): {}",
                        i + 1,
                        slippage,
                        err_msg
                    ));
                    let err_lower = err_msg.to_lowercase();
                    if err_lower.contains("429") || err_lower.contains("rate limit") {
                        logger::warning(
                            LogTag::Positions,
                            "Jupiter rate limit hit, backing off 10 seconds before retry",
                        );
                        sleep(Duration::from_secs(10)).await;
                    } else {
                        sleep(Duration::from_secs(2)).await;
                    }
                }
            }
        }
    }
    let swap_result = match swap_result {
        Ok(res) => res,
        Err(e) => {
            crate::positions::state::clear_partial_exit_pending(token_mint).await;

            // Record partial exit failure
            crate::events::record_position_event(
                &position_id.to_string(),
                token_mint,
                "partial_exit_swap_failed",
                position.entry_transaction_signature.as_deref(),
                None,
                0.0,
                exit_amount,
                None,
                Some(exit_percentage),
            )
            .await;

            return Err(format!("Partial exit swap failed: {e}"));
        }
    };

    let transaction_signature = swap_result.transaction_signature.clone();

    let expiry_height = get_rpc_client()
        .get_block_height()
        .await
        .unwrap_or_default()
        + super::SOLANA_BLOCKHASH_VALIDITY_SLOTS;

    let pending_partial = PendingPartialExit {
        signature: transaction_signature.clone(),
        mint: token_mint.to_string(),
        position_id,
        expected_exit_amount: exit_amount,
        requested_exit_percentage: exit_percentage,
        expiry_height: Some(expiry_height),
        created_at: Utc::now(),
    };

    if let Err(e) = register_pending_partial_exit(pending_partial.clone()).await {
        crate::positions::state::clear_partial_exit_pending(token_mint).await;
        logger::error(
            LogTag::Positions,
            &format!(
                "Failed to persist pending partial exit metadata for position {} (mint {}): {}",
                position_id, token_mint, e
            ),
        );
        return Err(format!(
            "Failed to persist pending partial exit metadata: {}",
            e
        ));
    }

    // Update position state (mark as partial exit pending)
    crate::positions::state::update_position_state(token_mint, |pos| {
        pos.exit_transaction_signature = Some(transaction_signature.clone());
        // Do NOT set exit_time - position is still open!
    })
    .await;

    // Save updated position to DB
    if let Some(updated_pos) = crate::positions::state::get_position_by_mint(token_mint).await {
        save_position(&updated_pos).await?;
    }

    // Add signature to index
    add_signature_to_index(&transaction_signature, token_mint).await;

    // Create partial exit transition
    let transition = crate::positions::transitions::PositionTransition::PartialExitSubmitted {
        position_id,
        exit_signature: transaction_signature.clone(),
        exit_amount,
        exit_percentage,
        market_price: position.current_price.unwrap_or(position.entry_price),
    };

    // Apply transition
    if let Err(e) = crate::positions::apply::apply_transition(transition).await {
        if let Err(err) = clear_pending_partial_exit(&pending_partial.signature).await {
            logger::error(
                LogTag::Positions,
                &format!(
                    "Failed to rollback pending partial exit {} after transition error: {}",
                    pending_partial.signature, err
                ),
            );
        }
        crate::positions::state::clear_partial_exit_pending(token_mint).await;
        return Err(format!("Failed to apply partial exit transition: {e}"));
    }

    // Enqueue for verification with partial exit flag
    let verification_item = VerificationItem::new_partial_exit(
        transaction_signature.clone(),
        token_mint.to_string(),
        Some(position_id),
        exit_amount,
        exit_percentage,
        Some(expiry_height),
    );

    enqueue_verification(verification_item).await;

    logger::info(
        LogTag::Positions,
        &format!(
            "Partial exit submitted: {} | {}% | TX: {} | Reason: {}",
            api_token.symbol, exit_percentage, transaction_signature, exit_reason
        ),
    );

    // CRITICAL: Do NOT release semaphore permit - position still open!

    Ok(transaction_signature)
}
