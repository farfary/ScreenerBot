//! Close position operations — full position exit with swap execution and verification.

use crate::config::with_config;
use crate::constants::SOL_MINT;
use crate::logger::{self, LogTag};
use crate::positions::price_resolution::get_price_with_api_fallback;
use crate::positions::queue::{enqueue_verification, VerificationItem};
use crate::positions::state::{acquire_position_lock, add_signature_to_index};
use crate::positions::types::VerificationKind;
use crate::positions::PENDING_VERIFICATION_SUFFIX;
use crate::rpc::{get_rpc_client, RpcClientMethods};
use crate::swaps::{execute_swap_with_fallback, get_best_quote, QuoteRequest, SwapMode};
use crate::utils::{get_token_balance, get_total_token_balance, get_wallet_address};
use serde_json::json;
use tokio::time::{sleep, Duration};

/// Close an existing position
pub async fn close_position_direct(
    token_mint: &str,
    exit_reason: String,
    slippage_pct: Option<f64>,
) -> Result<String, String> {
    let api_token = crate::tokens::get_full_token_async(token_mint)
        .await
        .map_err(|e| format!("Failed to get token: {e}"))?
        .ok_or_else(|| format!("Token not found: {token_mint}"))?;

    // Get price for the exit record. Price is used only for historical purposes —
    // the actual swap determines SOL received. Fall back to 0.0 if unavailable
    // (pool drained by rug, stale API data) so the close is never blocked by price.
    let exit_price = match get_price_with_api_fallback(token_mint).await {
        Some((pr, source)) if pr.price_sol > 0.0 && pr.price_sol.is_finite() => {
            logger::debug(
                LogTag::Positions,
                &format!(
                    "Closing position for {} at {} SOL (source: {:?})",
                    api_token.symbol, pr.price_sol, source
                ),
            );
            pr.price_sol
        }
        _ => {
            logger::warning(
                LogTag::Positions,
                &format!(
                    "No valid price data for {} — using 0.0 as exit price for record",
                    api_token.symbol
                ),
            );
            0.0
        }
    };

    let _lock = acquire_position_lock(token_mint).await;

    // A pending partial exit does NOT block a full close, and must not: this is the path a
    // stop-loss, a manual close and a force sell all take, so refusing it while a partial
    // confirms would block the user's emergency exit for as long as that verification runs
    // — which, if the partial's tx is slow to index, can be minutes.
    //
    // It is also unnecessary. A full close is sized from the WALLET BALANCE, read fresh from
    // chain below, so whatever a pending partial already sold is simply not there to sell
    // again. Only a second PARTIAL can oversell, because that sizes a percentage off the
    // position's `remaining_token_amount`, which the pending partial has not yet decremented
    // — and `partial_close_position` refuses exactly that case.
    if crate::positions::state::is_partial_exit_pending(token_mint).await {
        logger::warning(
            LogTag::Positions,
            &format!(
                "Closing {} while a partial exit is still confirming - the close is sized from the on-chain balance, so it cannot double-sell",
                api_token.symbol
            ),
        );
    }

    // Only block if a FULL exit is already pending for this position.
    if let Some(existing_position) = crate::positions::state::get_position_by_mint(token_mint).await
    {
        if let Some(pending_sig) = &existing_position.exit_transaction_signature {
            // Never slice a signature blindly: `&sig[..8]` panics on anything shorter (a
            // truncated or malformed value in the DB would take the process down here).
            let short_sig: String = pending_sig.chars().take(8).collect();
            logger::warning(
                LogTag::Positions,
                &format!(
                    "Position {} already has pending exit transaction: {}",
                    api_token.symbol, &short_sig
                ),
            );
            crate::events::record_position_event_flexible(
                "exit_blocked_pending_sig",
                crate::events::Severity::Warn,
                Some(&api_token.mint),
                Some(pending_sig),
                json!({
                  "reason": "pending_exit_tx_present"
                }),
            )
            .await;

            // Record structured position event
            crate::events::record_position_event(
                &existing_position.id.unwrap_or_default().to_string(),
                &api_token.mint,
                "exit_blocked",
                existing_position.entry_transaction_signature.as_deref(),
                Some(pending_sig),
                0.0,
                0,
                None,
                None,
            )
            .await;

            return Err(format!(
                "Position already has pending exit transaction: {}",
                &short_sig
            ));
        }
    }

    // Get TOTAL token balance across ALL accounts (CRITICAL FOR COMPLETE LIQUIDATION)
    let wallet_address =
        get_wallet_address().map_err(|e| format!("Failed to get wallet address: {e}"))?;

    let total_token_balance = get_total_token_balance(&wallet_address, token_mint)
        .await
        .map_err(|e| format!("Failed to get total token balance: {e}"))?;

    // Fetch primary (associated) token account balance separately. This is the balance most
    // swap routes will actually spend from. When multiple token accounts exist, passing the
    // aggregated total to a router that only sources a single ATA causes an "insufficient funds"
    // simulation failure (observed in logs). We therefore cap the sell amount to the primary
    // balance when it is lower than the aggregate, and log the discrepancy.
    let primary_token_balance = get_token_balance(&wallet_address, token_mint)
        .await
        .unwrap_or_default();

    let (sell_amount, multi_account_note) = if primary_token_balance == 0 && total_token_balance > 0
    {
        // We have tokens but not in the primary ATA (likely split or token-2022 alt). Use total but
        // expect potential router failure; still attempt but log.
        (
            total_token_balance,
            Some("primary_ata_empty_using_total".to_owned()),
        )
    } else if total_token_balance > primary_token_balance && primary_token_balance > 0 {
        (
            primary_token_balance,
            Some(format!(
                "multi_account_total={} primary={} shortfall={}, limiting_to_primary",
                total_token_balance,
                primary_token_balance,
                total_token_balance - primary_token_balance
            )),
        )
    } else {
        (total_token_balance, None)
    };

    if sell_amount == 0 {
        return Err(format!(
            "No tokens to sell: wallet balance is 0 for {}",
            api_token.symbol
        ));
    }

    logger::info(
        LogTag::Positions,
        &format!(
            "Selling tokens for {}: {} units (wallet total across all accounts: {})",
            api_token.symbol, sell_amount, total_token_balance
        ),
    );

    if let Some(note) = &multi_account_note {
        logger::warning(
            LogTag::Positions,
            &format!("Sell amount adjusted due to account distribution: {note}"),
        );
    }

    // Execute swap
    // IMPORTANT: Use ExactIn here. For exits we want to spend the exact token amount we actually have
    // (often restricted to a single ATA). Using ExactOut with `sell_amount` (token units) makes routers
    // treat it as desired SOL out, causing them to require more tokens than reside in the spending ATA,
    // which leads to SPL Token "insufficient funds"during Transfer. ExactIn avoids that.
    // Manual override starts the ladder; configured steps above it still escalate.
    let slippage_exit_retry_steps = super::slippage::exit_slippage_ladder(slippage_pct);
    // Slippage retry loop for exit
    let mut last_err: Option<String> = None;
    let mut swap_result = None;
    // A swap that was SUBMITTED but whose confirmation timed out: the sell may still land,
    // so retrying the ladder would sell twice. We stop and let verification reconcile it.
    let mut submitted_signature: Option<String> = None;
    for (i, slippage) in slippage_exit_retry_steps.iter().enumerate() {
        let quote_request = QuoteRequest {
            input_mint: token_mint.to_string(),
            output_mint: SOL_MINT.to_string(),
            input_amount: sell_amount,
            wallet_address: wallet_address.clone(),
            slippage_pct: *slippage,
            swap_mode: SwapMode::ExactIn,
            exclude_dexes: None,
        };

        let quote = match get_best_quote(quote_request.clone()).await {
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

        match execute_swap_with_fallback(&api_token, quote).await {
            Ok(res) => {
                swap_result = Some(res);
                last_err = None;
                break;
            }
            Err(e) => {
                // Submitted but unconfirmed — the sell may already be on chain. Stop the
                // ladder: another rung would be a second real sell.
                if let Some(signature) = crate::swaps::unconfirmed_swap_signature(&e) {
                    logger::warning(
                        LogTag::Positions,
                        &format!(
                            "Exit swap {} for {} was submitted but not confirmed in time - not retrying; verification will settle it",
                            signature, api_token.symbol
                        ),
                    );
                    submitted_signature = Some(signature);
                    last_err = None;
                    break;
                }

                // Check for pump.fun bonding curve error (graduated token routed through closed curve)
                let msg = e.to_string();
                let msg_lower = msg.to_lowercase();

                if msg.contains("0x1787") || msg.contains("6023") {
                    logger::warning(
                        LogTag::Positions,
                        &format!(
                            "Pump.fun bonding curve error detected for {}, retrying with alternative DEX route",
                            token_mint
                        ),
                    );
                    // Retry once with Pump.fun Amm excluded
                    let mut retry_request = quote_request.clone();
                    retry_request.exclude_dexes = Some(vec!["Pump.fun Amm".to_string()]);
                    let retry_quote = match get_best_quote(retry_request).await {
                        Ok(q) => q,
                        Err(e2) => {
                            last_err = Some(format!(
                                "Retry without Pump.fun also failed (quote): {e2} (step {} slippage {}%)",
                                i + 1, slippage
                            ));
                            continue;
                        }
                    };
                    match execute_swap_with_fallback(&api_token, retry_quote).await {
                        Ok(res) => {
                            swap_result = Some(res);
                            last_err = None;
                            break;
                        }
                        Err(e2) => {
                            if let Some(signature) = crate::swaps::unconfirmed_swap_signature(&e2) {
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

                // If we attempted to sell the aggregated total and failed with insufficient funds,
                // hint at likely multi-account cause for easier diagnosis.
                let enriched = if msg_lower.contains("insufficient funds")
                    && multi_account_note.is_none()
                    && total_token_balance > sell_amount
                {
                    format!("Swap failed (insufficient funds) - aggregated balance mismatch; consider consolidating ATAs: {msg}")
                } else {
                    format!("Swap failed: {msg}")
                };
                last_err = Some(format!(
                    "{} (step {} slippage {}%)",
                    enriched,
                    i + 1,
                    slippage
                ));
                if msg_lower.contains("429") || msg_lower.contains("rate limit") {
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

    let transaction_signature = match (swap_result, submitted_signature) {
        (Some(result), _) => {
            let transaction_signature = result.transaction_signature.clone();

            // CRITICAL: Log execution vs requested amounts to detect partial execution
            let executed_amount = result.input_amount;
            if executed_amount < sell_amount {
                logger::warning(
                    LogTag::Positions,
                    &format!(
 "PARTIAL SWAP DETECTED for {}: Requested {} tokens, executed {} tokens, shortfall: {}",
        api_token.symbol,
        sell_amount,
        executed_amount,
        sell_amount - executed_amount
      ),
                );
            } else {
                logger::info(
                    LogTag::Positions,
                    &format!(
                        "Full swap executed for {}: {} tokens",
                        api_token.symbol, executed_amount
                    ),
                );
            }

            transaction_signature
        }
        // Submitted, confirmation timed out. Treat it exactly like a confirmed submission:
        // record the signature and enqueue verification, which reads the chain and either
        // settles the exit or (if the transaction never landed) clears it for a retry. What
        // we must NOT do is send it again.
        (None, Some(signature)) => signature,
        (None, None) => {
            return Err(last_err.unwrap_or_else(|| "Exit swap failed".to_owned()));
        }
    };

    // Update position with exit signature and market exit price
    crate::positions::state::update_position_state(token_mint, |pos| {
        pos.exit_transaction_signature = Some(transaction_signature.clone());
        pos.exit_price = Some(exit_price); // Store pool/market price at exit decision time
        pos.closed_reason = Some(format!("{exit_reason}{PENDING_VERIFICATION_SUFFIX}"));
    })
    .await;

    add_signature_to_index(&transaction_signature, token_mint).await;

    // Get position ID (needed for event recording). Keep it an Option: defaulting to 0 on a
    // lookup race enqueued a verification item pointing at position 0, which resolves to
    // nothing — the exit would then never be applied to the real position.
    let position_id = crate::positions::state::get_position_by_mint(token_mint)
        .await
        .and_then(|p| p.id);

    // Record a position closing event (pending verification)
    crate::events::record_position_event(
        &position_id.unwrap_or_default().to_string(),
        token_mint,
        "closing_submitted",
        None,
        Some(&transaction_signature),
        0.0,
        sell_amount,
        None,
        None,
    )
    .await;

    // Get block height for expiration
    let expiry_height = get_rpc_client()
        .get_block_height()
        .await
        .map(|h| h + super::SOLANA_BLOCKHASH_VALIDITY_SLOTS)
        .ok();

    // Enqueue for verification
    let verification_item = VerificationItem::new(
        transaction_signature.clone(),
        token_mint.to_string(),
        position_id,
        VerificationKind::Exit,
        expiry_height,
    );

    enqueue_verification(verification_item).await;

    logger::info(
        LogTag::Positions,
        &format!(
            "Position closing: {} | TX: {} | Reason: {}",
            api_token.symbol, transaction_signature, exit_reason
        ),
    );

    Ok(transaction_signature)
}
