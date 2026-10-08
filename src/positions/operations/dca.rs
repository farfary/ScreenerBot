// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! DCA (Dollar Cost Averaging) operations — add to an existing position.

use crate::chains::adapter;
use crate::chains::RawAmount;
use crate::config::with_config;
use crate::logger::{self, LogTag};
use crate::positions::price_resolution::get_price_with_api_fallback;
use crate::positions::queue::{enqueue_verification, VerificationItem};
use crate::positions::state::{
    acquire_position_lock, mark_swap_in_flight, register_pending_dca_swap,
};
use crate::positions::types::{PendingDcaSwap, TradeOrigin};
use crate::positions::{Error, Result};
use crate::swaps::{
    execute_swap_with_fallback, get_best_quote_for_opening, QuoteRequest, SwapMode,
};
use crate::utils::get_wallet_address;
use chrono::Utc;

/// Add to an existing position (Dollar Cost Averaging)
/// CRITICAL: This does NOT consume a new semaphore permit - same position
pub async fn add_to_position(
    token_mint: &str,
    dca_amount_native: f64,
    slippage_pct: Option<f64>,
    origin: TradeOrigin,
) -> Result<String> {
    // Serialize per-mint DCA operations
    let _lock = acquire_position_lock(token_mint).await;
    // The open position of the mint, archived or not: an add on an archived one unarchives
    // it when the add is booked.
    let position = crate::positions::state::get_open_round_by_mint(token_mint)
        .await
        .ok_or_else(|| Error::NotFound {
            mint: token_mint.to_owned(),
        })?;

    let position_id = position.id.ok_or_else(|| Error::TransitionFailed {
        transition: "dca",
        mint: token_mint.to_owned(),
        detail: "position has no id".to_owned(),
    })?;

    // The add brings an archived position back to active management; a store that still
    // holds another active open position of the mint would then hold two, so the add is
    // refused before anything is sent.
    if position.archived {
        crate::positions::db::refuse_reactivating_open_round(token_mint, position_id).await?;
    }

    // The DCA config governs what the AUTO-TRADER may do on its own — it is not a
    // capability switch for the user. A manual "Add to Position" from the dashboard is
    // the user overriding the bot, so `dca_enabled`, `dca_max_count` and the cooldown
    // do not apply to it. Gating manual adds on them meant anyone who simply did not
    // want automatic DCA got "DCA is disabled in configuration" when clicking Add, and
    // a user who did want to average down was told to wait out the BOT's cooldown.
    if origin == TradeOrigin::Auto {
        let dca_enabled = with_config(|cfg| cfg.trader.dca_enabled);
        if !dca_enabled {
            return Err(Error::DcaDisabled);
        }

        let max_dca_count = with_config(|cfg| cfg.trader.dca_max_count);
        if position.dca_count >= max_dca_count as u32 {
            return Err(Error::TransitionFailed {
                transition: "dca",
                mint: token_mint.to_owned(),
                detail: format!(
                    "maximum DCA count reached: {} (max: {})",
                    position.dca_count, max_dca_count
                ),
            });
        }

        // Check DCA cooldown
        if let Some(last_dca) = position.last_dca_time {
            let cooldown_minutes = with_config(|cfg| cfg.trader.dca_cooldown_minutes);
            let elapsed = Utc::now().signed_duration_since(last_dca).num_minutes();
            if elapsed < cooldown_minutes {
                return Err(Error::TransitionFailed {
                    transition: "dca",
                    mint: token_mint.to_owned(),
                    detail: format!(
                        "DCA cooldown active: {} minutes remaining",
                        cooldown_minutes - elapsed
                    ),
                });
            }
        }
    }

    logger::info(
        LogTag::Positions,
        &format!(
            "DCA entry initiated: {} | {} SOL | DCA #{} ",
            position.symbol,
            dca_amount_native,
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
        dca_amount_native,
        RawAmount::ZERO,
        None,
        None,
    )
    .await;

    // Get API token for swap
    let api_token = crate::tokens::get_full_token_async(crate::chains::active_chain(), token_mint)
        .await
        .map_err(|_| Error::TokenNotFound {
            mint: token_mint.to_owned(),
        })?
        .ok_or_else(|| Error::TokenNotFound {
            mint: token_mint.to_owned(),
        })?;

    // Get quote for DCA entry
    let wallet_address = get_wallet_address().map_err(|e| Error::WalletUnavailable {
        detail: e.to_string(),
    })?;
    // Manual override when the user set one in the trade dialog; config default otherwise.
    let slippage = super::slippage::entry_slippage(slippage_pct);
    let quote_request = QuoteRequest {
        chain: crate::chains::active_chain(),
        input_mint: adapter().native_asset_address().to_string(),
        output_mint: token_mint.to_string(),
        input_amount: adapter().native_to_raw(dca_amount_native).into(),
        wallet_address: wallet_address.clone(),
        slippage_pct: slippage,
        swap_mode: SwapMode::ExactIn,
        exclude_dexes: None,
    };
    let quote = get_best_quote_for_opening(quote_request, &api_token.symbol)
        .await
        .map_err(|e| Error::QuoteFailed {
            mint: token_mint.to_owned(),
            detail: e.to_string(),
        })?;

    // Only scale into a UI amount when the decimals are actually known; printing raw
    // units against an assumed 9 decimals misreports the quote by orders of magnitude.
    let quoted_tokens = match api_token.decimals {
        Some(decimals) => format!(
            "{}",
            quote.output_amount.raw() as f64 / 10_f64.powi(decimals as i32)
        ),
        None => format!("{} raw", quote.output_amount),
    };
    logger::info(
        LogTag::Positions,
        &format!("DCA quote: {dca_amount_native} SOL → {quoted_tokens} tokens"),
    );

    // From before the submission until its pending marker is registered, the wallet-history
    // sync leaves the mint to the trader: a confirmed swap is in the wallet before then.
    let _in_flight =
        mark_swap_in_flight(crate::positions::db::get_store_chain().await?, token_mint);

    // Execute swap. A swap that REACHED THE CHAIN is never discarded as a trade
    // that never happened: `unconfirmed_swap_signature` hands back the signature
    // of a confirmation that timed out or of a confirmed swap whose receipt could
    // not be measured, and both must be registered for verification exactly like a
    // clean fill. Returning `SwapFailed` here would leave the wallet holding
    // tokens the position never counted -- the same recovery `open.rs` performs on
    // an entry.
    let transaction_signature = match execute_swap_with_fallback(
        &api_token,
        quote,
        crate::swaps::SwapAmountLimit::Unrestricted,
    )
    .await
    {
        Ok(result) => result.transaction_signature,
        Err(error) => match crate::swaps::failed_swap(&error) {
            crate::swaps::FailedSwap::Reconcile { signature } => {
                logger::warning(
                    LogTag::Positions,
                    &format!(
                        "DCA swap {signature} for position {position_id} reached the chain but \
                         could not be confirmed here; registering it for verification instead of \
                         failing the DCA"
                    ),
                );
                signature
            }
            crate::swaps::FailedSwap::Resendable | crate::swaps::FailedSwap::Unresolved => {
                return Err(Error::SwapFailed {
                    mint: token_mint.to_owned(),
                    detail: format!("DCA swap failed: {error}"),
                    not_submitted: crate::swaps::not_submitted_reason(&error),
                })
            }
        },
    };

    // Pre-compute expiry height for verification + persistence
    let expiry_height = crate::positions::settle::submission_expiry_bound().await;

    // The swap was sent, so from here nothing may drop its signature: every bookkeeping
    // failure below is logged and the verification is queued regardless. The pending
    // marker makes the DCA durable across a restart and visible to the wallet-history sync.
    let pending_dca = PendingDcaSwap {
        signature: transaction_signature.clone(),
        mint: token_mint.to_string(),
        position_id,
        expiry_height,
        created_at: Utc::now(),
        size_sol: dca_amount_native,
    };
    if let Err(e) = register_pending_dca_swap(pending_dca).await {
        logger::error(
            LogTag::Positions,
            &format!(
                "Pending DCA {transaction_signature} for position {position_id} (mint {token_mint}) is held in memory only, not persisted: {e}"
            ),
        );
    }

    // The submitted event needs the market price; without one the DCA is still verified.
    match get_price_with_api_fallback(token_mint).await {
        Some((price_info, _price_source)) => {
            let transition = crate::positions::transitions::PositionTransition::DcaSubmitted {
                position_id,
                dca_signature: transaction_signature.clone(),
                dca_amount_native,
                market_price: price_info.price_native,
            };
            if let Err(e) = crate::positions::apply::apply_transition(transition).await {
                logger::error(
                    LogTag::Positions,
                    &format!(
                        "DCA {transaction_signature} for position {position_id} was not recorded as submitted: {e}"
                    ),
                );
            }
        }
        None => logger::warning(
            LogTag::Positions,
            &format!(
                "No price for {token_mint}: DCA {transaction_signature} is verified without a submitted event"
            ),
        ),
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
            dca_amount_native,
            transaction_signature,
            position.dca_count + 1
        ),
    );

    // CRITICAL: Do NOT consume a new semaphore permit - same position!

    Ok(transaction_signature)
}
