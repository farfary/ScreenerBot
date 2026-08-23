//! Partial close operations — sell a percentage of remaining tokens without closing the position.

use crate::chains::adapter;
use crate::chains::solana::assets::ata::get_total_token_balance;
use crate::chains::solana::rpc::{get_rpc_client, RpcClientMethods};
use crate::logger::{self, LogTag};
use crate::positions::error::{Error, Result};
use crate::positions::queue::{enqueue_verification, VerificationItem};
use crate::positions::state::{
    acquire_position_lock, add_signature_to_index, clear_pending_partial_exit,
    register_pending_partial_exit,
};
use crate::positions::types::PendingPartialExit;
use crate::swaps::{
    calculate_partial_amount, execute_swap_with_fallback, get_best_quote, QuoteRequest, SwapMode,
};
use crate::utils::get_wallet_address;
use chrono::Utc;
use tokio::time::{sleep, Duration};

/// Back off before the next slippage rung — longer when the router is rate-limiting us.
async fn backoff_after(error_message: &str) {
    let lowered = error_message.to_lowercase();
    if lowered.contains("429") || lowered.contains("rate limit") {
        logger::warning(
            LogTag::Positions,
            "Jupiter rate limit hit, backing off 10 seconds before retry",
        );
        sleep(Duration::from_secs(10)).await;
    } else {
        sleep(Duration::from_secs(2)).await;
    }
}

/// Partially close a position by selling a percentage of remaining tokens
/// CRITICAL: This does NOT release the semaphore permit - position stays open
pub async fn partial_close_position(
    token_mint: &str,
    exit_percentage: f64,
    exit_reason: &str,
    slippage_pct: Option<f64>,
) -> Result<String> {
    // Serialize per-mint operations to avoid overlapping partials/full exits
    let _lock = acquire_position_lock(token_mint).await;

    // Validate percentage
    if exit_percentage <= 0.0 || exit_percentage >= 100.0 {
        return Err(Error::InvalidExitPercentage {
            percent: exit_percentage,
            reason: "must be between 0 and 100 (exclusive)".to_owned(),
        });
    }

    // Refuse to overlap two partial exits. The per-mint lock above only covers the
    // SUBMIT: it is released as soon as this function returns, while verification (which
    // is what decrements `remaining_token_amount`) is still seconds away. Without this
    // check a second partial — the exit monitor ticking again, or the user clicking sell
    // — sized its percentage against a remaining amount the first exit had already sold,
    // and the position was oversold.
    if crate::positions::state::is_partial_exit_pending(token_mint).await {
        return Err(Error::TransitionFailed {
            transition: "partial_exit",
            mint: token_mint.to_owned(),
            detail: "a partial exit is already in flight for this position".to_owned(),
        });
    }

    // Get position
    let position = crate::positions::state::get_position_by_mint(token_mint)
        .await
        .ok_or_else(|| Error::NotFound {
            mint: token_mint.to_owned(),
        })?;

    let position_id = position.id.ok_or_else(|| Error::TransitionFailed {
        transition: "partial_exit",
        mint: token_mint.to_owned(),
        detail: "position has no id".to_owned(),
    })?;

    // Get remaining token amount
    let remaining_amount = position
        .remaining_token_amount
        .or(position.token_amount)
        .ok_or_else(|| Error::TransitionFailed {
            transition: "partial_exit",
            mint: token_mint.to_owned(),
            detail: "position has no token amount".to_owned(),
        })?;

    // Size against what the WALLET actually holds, not only what the position believes.
    // The quote preview (`/api/trader/quote`) already prices the percentage against the
    // real on-chain balance, so sizing execution off the DB alone made the preview and
    // the trade disagree whenever the two drifted — and a DB amount larger than the real
    // balance simply failed the swap with insufficient funds. Take the LOWER of the two:
    // never sell more than the wallet has, never more than the position owns.
    let wallet_address = get_wallet_address().map_err(|e| Error::WalletUnavailable {
        detail: e.to_string(),
    })?;

    let sell_base = match get_total_token_balance(&wallet_address, token_mint).await {
        Ok(on_chain) if on_chain > 0 => {
            if on_chain != remaining_amount {
                logger::warning(
                    LogTag::Positions,
                    &format!(
                        "Partial exit balance drift for {}: position={} on-chain={} — sizing against the lower",
                        position.symbol, remaining_amount, on_chain
                    ),
                );
            }
            on_chain.min(remaining_amount)
        }
        // The balance lookup is best-effort: an RPC hiccup must not block an exit. Fall
        // back to the position's own accounting, which is what this always used.
        _ => remaining_amount,
    };

    // Calculate partial exit amount
    let exit_amount = calculate_partial_amount(sell_base, exit_percentage);

    if exit_amount == 0 {
        return Err(Error::ZeroExitAmount {
            mint: token_mint.to_owned(),
        });
    }

    logger::info(
        LogTag::Positions,
        &format!(
            "Partial exit initiated: {} | {}% ({} of {} tokens) | Reason: {}",
            position.symbol, exit_percentage, exit_amount, sell_base, exit_reason
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
        .map_err(|_| Error::TokenNotFound {
            mint: token_mint.to_owned(),
        })?
        .ok_or_else(|| Error::TokenNotFound {
            mint: token_mint.to_owned(),
        })?;

    // Manual override starts the ladder; configured steps above it still escalate.
    let slippage_exit_retry_steps = super::slippage::exit_slippage_ladder(slippage_pct);

    // Mark the partial pending BEFORE the first submission: the per-mint lock is released
    // as soon as this function returns, long before verification lands, so this registry is
    // what stops a second exit from being sized against tokens this one has already sold.
    crate::positions::state::mark_partial_exit_pending(token_mint).await;

    // ONE ladder: quote and swap at each slippage rung in turn.
    //
    // This used to be an initial swap attempt followed by a SECOND loop that started again
    // at rung 1 — re-submitting the very slippage that had just failed. And any swap error
    // led to another submission, including the "submitted but not confirmed in time" error,
    // which means the first sell may well be on chain: retrying it sells the same tokens
    // TWICE while the position records only one partial.
    let mut last_err: Option<String> = None;
    let mut swap_result = None;
    let mut submitted_signature: Option<String> = None;

    for (i, slippage) in slippage_exit_retry_steps.iter().enumerate() {
        let quote_request = QuoteRequest {
            chain: crate::chains::active_chain(),
            input_mint: token_mint.to_string(),
            output_mint: adapter().native_asset_address().to_string(),
            input_amount: exit_amount,
            wallet_address: wallet_address.clone(),
            slippage_pct: *slippage,
            swap_mode: SwapMode::ExactIn,
            exclude_dexes: None,
        };

        let quote = match get_best_quote(quote_request.clone()).await {
            Ok(quote) => quote,
            Err(e) => {
                let err_msg = e.to_string();
                last_err = Some(format!(
                    "Quote failed at step {} ({}%): {}",
                    i + 1,
                    slippage,
                    err_msg
                ));
                backoff_after(&err_msg).await;
                continue;
            }
        };

        logger::info(
            LogTag::Positions,
            &format!(
                "Partial exit quote ({}% slippage): {} tokens -> {} SOL",
                slippage,
                exit_amount,
                adapter().raw_to_native(quote.output_amount)
            ),
        );

        match execute_swap_with_fallback(&api_token, quote).await {
            Ok(res) => {
                swap_result = Some(res);
                last_err = None;
                break;
            }
            Err(e) => {
                // Submitted, confirmation timed out: the sell may still land. Stop here and
                // let verification settle it — another rung would be a second real sell.
                if let Some(signature) = crate::swaps::unconfirmed_swap_signature(&e) {
                    logger::warning(
                        LogTag::Positions,
                        &format!(
                            "Partial exit swap {} for {} was submitted but not confirmed in time - not retrying; verification will settle it",
                            signature, api_token.symbol
                        ),
                    );
                    submitted_signature = Some(signature);
                    last_err = None;
                    break;
                }

                let err_msg = e.to_string();

                // Pump.fun bonding curve error (graduated token): one retry with that AMM
                // excluded, at the same slippage.
                if err_msg.contains("0x1787") || err_msg.contains("6023") {
                    logger::warning(
                        LogTag::Positions,
                        &format!(
                            "Pump.fun bonding curve error detected for {}, retrying partial exit with alternative DEX route",
                            token_mint
                        ),
                    );

                    let mut retry_request = quote_request.clone();
                    retry_request.exclude_dexes = Some(vec!["Pump.fun Amm".to_string()]);

                    match get_best_quote(retry_request).await {
                        Ok(retry_quote) => {
                            match execute_swap_with_fallback(&api_token, retry_quote).await {
                                Ok(res) => {
                                    swap_result = Some(res);
                                    last_err = None;
                                    break;
                                }
                                Err(e2) => {
                                    if let Some(signature) =
                                        crate::swaps::unconfirmed_swap_signature(&e2)
                                    {
                                        submitted_signature = Some(signature);
                                        last_err = None;
                                        break;
                                    }
                                    last_err = Some(format!(
                                        "Retry swap without Pump.fun failed: {e2} (step {} slippage {}%)",
                                        i + 1,
                                        slippage
                                    ));
                                    continue;
                                }
                            }
                        }
                        Err(e2) => {
                            last_err = Some(format!(
                                "Retry without Pump.fun also failed (quote): {e2} (step {} slippage {}%)",
                                i + 1,
                                slippage
                            ));
                            continue;
                        }
                    }
                }

                last_err = Some(format!(
                    "Partial exit swap failed at step {} ({}%): {}",
                    i + 1,
                    slippage,
                    err_msg
                ));
                backoff_after(&err_msg).await;
            }
        }
    }

    let transaction_signature = match (swap_result, submitted_signature) {
        (Some(result), _) => result.transaction_signature.clone(),
        // Submitted but unconfirmed: record the signature and verify it like any other.
        (None, Some(signature)) => signature,
        (None, None) => {
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

            return Err(Error::SwapFailed {
                mint: token_mint.to_owned(),
                detail: last_err.unwrap_or_else(|| "no route".to_owned()),
            });
        }
    };

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
        return Err(e);
    }

    // A partial exit must NOT be written to `position.exit_transaction_signature`.
    //
    // That field means "a FULL exit is in flight / has happened", and the rest of the
    // system reads it that way. Stamping a partial's signature on it — which is what
    // this used to do, and which nothing ever cleared, because PartialExitVerified
    // deliberately leaves exit_signature/exit_time alone — permanently poisoned the
    // position:
    //   - `close_position_direct` refuses to run while it is set ("Position already has
    //     pending exit transaction"), so after ONE partial take-profit the position
    //     could never be fully closed again: not by stop-loss, not by take-profit, not
    //     by a manual or force close. It was stuck open with the remainder forever.
    //   - `transaction_exit_verified` is never set for a partial, so on every restart
    //     the worker saw an "unverified exit signature" and re-enqueued the partial's
    //     signature as a FULL-exit verification — which closes the position and releases
    //     the semaphore permit while the tokens are still in the wallet.
    //   - P&L took its "closing in progress" branch forever after (see pnl.rs).
    //
    // The partial exit is already durable without it: PENDING_PARTIAL_EXIT_DETAILS is
    // persisted to metadata and rehydrated into a partial verification item at startup,
    // and the queue item carries the mint, position id, expected amount and percentage.
    // The signature index below is all the position state a partial needs.

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
        return Err(Error::TransitionFailed {
            transition: "partial_exit",
            mint: token_mint.to_owned(),
            detail: e.to_string(),
        });
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
