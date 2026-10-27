// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Close position operations — full position exit with swap execution and verification.

use crate::chains::solana::assets::ata::{get_token_balance, get_total_token_balance};
use crate::chains::RawAmount;
use crate::errors::ErrorClass;
use crate::logger::{self, LogTag};
use crate::positions::db::{get_other_open_held, record_exit_submission, OtherOpenHeld};
use crate::positions::price_resolution::get_price_with_api_fallback;
use crate::positions::queue::{enqueue_verification, VerificationItem};
use crate::positions::state::{
    acquire_position_lock, add_signature_to_index, mark_swap_in_flight,
    set_exit_submission_in_memory, with_booking_lock,
};
use crate::positions::types::VerificationKind;
use crate::positions::PENDING_VERIFICATION_SUFFIX;
use crate::positions::{Error, Result};
use crate::swaps::{execute_swap_with_fallback, get_best_quote, QuoteRequest, SwapMode};
use crate::utils::get_wallet_address;
use serde_json::json;
use tokio::time::{sleep, Duration};

use super::exit_ladder::{run_exit_ladder, ExitSwap};

/// Close an existing position
pub async fn close_position_direct(
    token_mint: &str,
    exit_reason: String,
    slippage_pct: Option<f64>,
) -> Result<String> {
    let api_token = crate::tokens::get_full_token_async(crate::chains::active_chain(), token_mint)
        .await
        .map_err(|_| Error::TokenNotFound {
            mint: token_mint.to_owned(),
        })?
        .ok_or_else(|| Error::TokenNotFound {
            mint: token_mint.to_owned(),
        })?;

    // Get price for the exit record. Price is used only for historical purposes —
    // the actual swap determines SOL received. Fall back to 0.0 if unavailable
    // (pool drained by rug, stale API data) so the close is never blocked by price.
    let exit_price = match get_price_with_api_fallback(token_mint).await {
        Some((pr, source)) if pr.price_native > 0.0 && pr.price_native.is_finite() => {
            logger::debug(
                LogTag::Positions,
                &format!(
                    "Closing position for {} at {} SOL (source: {:?})",
                    api_token.symbol, pr.price_native, source
                ),
            );
            pr.price_native
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

    // The open round of the mint, archived or not: the row a manual sell targets, and the one
    // whose pending exit, holding and exit record this close reads and writes.
    let own_position = crate::positions::state::get_open_round_by_mint(token_mint).await;

    // Only block if a FULL exit is already pending for this position.
    if let Some(existing_position) = own_position.as_ref() {
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
                RawAmount::ZERO,
                None,
                None,
            )
            .await;

            return Err(Error::TransitionFailed {
                transition: "close",
                mint: api_token.mint.clone(),
                detail: format!("position already has pending exit transaction: {short_sig}"),
            });
        }
    }

    // Get TOTAL token balance across ALL accounts (CRITICAL FOR COMPLETE LIQUIDATION)
    let wallet_address = get_wallet_address().map_err(|e| Error::WalletUnavailable {
        detail: e.to_string(),
    })?;

    let total_token_balance = get_total_token_balance(&wallet_address, token_mint)
        .await
        .map_err(|e| Error::TransitionFailed {
            transition: "close",
            mint: api_token.mint.clone(),
            detail: format!("failed to get total token balance: {e}"),
        })?;

    // Fetch primary (associated) token account balance separately. This is the balance most
    // swap routes will actually spend from. When multiple token accounts exist, passing the
    // aggregated total to a router that only sources a single ATA causes an "insufficient funds"
    // simulation failure (observed in logs). We therefore cap the sell amount to the primary
    // balance when it is lower than the aggregate, and log the discrepancy.
    let primary_token_balance = get_token_balance(&wallet_address, token_mint)
        .await
        .unwrap_or_default();

    // What the other open positions of the mint hold is not this position's to sell.
    let others = get_other_open_held(token_mint, own_position.as_ref().and_then(|p| p.id))
        .await
        .map_err(|e| Error::TransitionFailed {
            transition: "close",
            mint: api_token.mint.clone(),
            detail: format!("failed to read the other open positions' holding: {e}"),
        })?;
    let own_remaining = own_position
        .as_ref()
        .and_then(|p| p.held_amount())
        .unwrap_or_default();
    let ExitSizing {
        amount: sell_amount,
        note: multi_account_note,
    } = exit_sell_amount(
        total_token_balance,
        primary_token_balance,
        own_remaining,
        others,
    );

    if sell_amount == 0 {
        return Err(Error::ZeroExitAmount {
            mint: api_token.mint.clone(),
        });
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
    // From before the first submission until the exit signature is recorded, the
    // wallet-history sync leaves the mint to the trader: a confirmed sell empties the wallet
    // before then, and the sync would otherwise close the position as sold outside the bot.
    let _in_flight =
        mark_swap_in_flight(crate::positions::db::get_store_chain().await?, token_mint);
    // The ladder sets the slippage and any excluded venue per rung.
    let base_request = QuoteRequest {
        chain: crate::chains::active_chain(),
        input_mint: token_mint.to_string(),
        output_mint: crate::chains::adapter().native_asset_address().to_string(),
        input_amount: sell_amount.into(),
        wallet_address: wallet_address.clone(),
        slippage_pct: 0.0,
        swap_mode: SwapMode::ExactIn,
        exclude_dexes: None,
    };
    let token = &api_token;
    let exit = run_exit_ladder(
        "Exit",
        &api_token.symbol,
        &slippage_exit_retry_steps,
        |slippage_pct, exclude_dexes| {
            get_best_quote(QuoteRequest {
                slippage_pct,
                exclude_dexes,
                ..base_request.clone()
            })
        },
        move |quote| {
            execute_swap_with_fallback(token, quote, crate::swaps::SwapAmountLimit::Unrestricted)
        },
    )
    .await;

    // A sell sized from the aggregated balance that the router could only source
    // from one account fails for want of funds.
    let with_balance_hint = |detail: String| {
        if detail.to_lowercase().contains("insufficient funds")
            && multi_account_note.is_none()
            && total_token_balance > sell_amount
        {
            format!(
                "Swap failed (insufficient funds) - aggregated balance mismatch; consider consolidating ATAs: {detail}"
            )
        } else {
            detail
        }
    };

    let transaction_signature = match exit {
        ExitSwap::Executed(result) => {
            let transaction_signature = result.transaction_signature.clone();

            // CRITICAL: Log execution vs requested amounts to detect partial execution
            let executed_amount = result.input_amount.raw();
            if executed_amount < u128::from(sell_amount) {
                logger::warning(
                    LogTag::Positions,
                    &format!(
 "PARTIAL SWAP DETECTED for {}: Requested {} tokens, executed {} tokens, shortfall: {}",
        api_token.symbol,
        sell_amount,
        executed_amount,
        u128::from(sell_amount) - executed_amount
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
        ExitSwap::Submitted(signature) => signature,
        // A full close sizes from the wallet balance, so a send whose outcome is unproven
        // fails like any other: the next attempt sells only what is still held.
        ExitSwap::Unresolved { detail } => {
            return Err(Error::SwapFailed {
                mint: api_token.mint.clone(),
                detail: with_balance_hint(detail),
                not_submitted: None,
            });
        }
        ExitSwap::Failed {
            detail,
            not_submitted,
        } => {
            return Err(Error::SwapFailed {
                mint: api_token.mint.clone(),
                detail: with_balance_hint(detail),
                not_submitted,
            });
        }
    };

    // Get position ID (needed for the exit record and event recording). Keep it an Option:
    // defaulting to 0 on a lookup race enqueued a verification item pointing at position 0,
    // which resolves to nothing — the exit would then never be applied to the real position.
    let position_id = own_position.as_ref().and_then(|p| p.id);

    // Record the exit signature and the market exit price on the row, then in memory.
    let closed_reason = format!("{exit_reason}{PENDING_VERIFICATION_SUFFIX}");
    match position_id {
        Some(id) => {
            match mark_exit_submitted(id, &transaction_signature, exit_price, &closed_reason).await
            {
                Ok(()) => {}
                Err(Error::AlreadyClosed { .. }) => logger::warning(
                    LogTag::Positions,
                    &format!(
                        "Exit {} was submitted after position {} was closed; the closed booking is kept",
                        transaction_signature, id
                    ),
                ),
                Err(error) => logger::error(
                    LogTag::Positions,
                    &format!(
                        "Exit {} for position {} was not written to the database ({}); memory holds it, and verification stores the signature with the close",
                        transaction_signature, id, error
                    ),
                ),
            }
        }
        None => logger::warning(
            LogTag::Positions,
            &format!(
                "No open position for {} when recording exit {}",
                api_token.symbol, transaction_signature
            ),
        ),
    }

    add_signature_to_index(&transaction_signature, token_mint).await;

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

    let expiry_height = crate::positions::settle::submission_expiry_bound().await;

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

/// Records a submitted full-exit swap on its position: the row first, then memory, both
/// under the position's booking lock so no booking publishes a row read before the write.
///
/// A retryable row failure is retried with the position-save backoff, up to
/// `POSITION_SAVE_MAX_RETRIES` attempts. The swap is already submitted, so memory takes
/// the exit even when the row write finally fails: without it the exit monitor would sell
/// again and verification could not match the exit to its position. The row then lacks
/// the submission until verification: a verified close stores the swap's signature with
/// the close, its exit record and its realized P&L, and a failed one clears the exit by
/// the swap's signature. The market exit price and the pending reason stay memory-only. A
/// position whose exit is already verified keeps its booking in the row and in memory,
/// and the call returns [`Error::AlreadyClosed`]. Returns the result of the row write.
pub async fn mark_exit_submitted(
    position_id: i64,
    exit_signature: &str,
    exit_price: f64,
    closed_reason: &str,
) -> Result<()> {
    with_booking_lock(position_id, async {
        let mut attempt = 1;
        let persisted = loop {
            match record_exit_submission(position_id, exit_signature, exit_price, closed_reason)
                .await
            {
                Err(error)
                    if error.is_retryable() && attempt < super::POSITION_SAVE_MAX_RETRIES =>
                {
                    logger::warning(
                        LogTag::Positions,
                        &format!(
                            "Recording exit {exit_signature} for position {position_id} failed on attempt {attempt}: {error}"
                        ),
                    );
                    sleep(Duration::from_millis(super::position_save_backoff_ms(
                        attempt,
                    )))
                    .await;
                    attempt += 1;
                }
                result => break result,
            }
        };
        if matches!(persisted, Err(Error::AlreadyClosed { .. })) {
            return persisted;
        }
        set_exit_submission_in_memory(position_id, exit_signature, exit_price, closed_reason)
            .await;
        persisted
    })
    .await
}

/// How many raw tokens a full close sells, and why it is below the wallet total when it is.
#[derive(Debug, PartialEq, Eq)]
struct ExitSizing {
    amount: u64,
    note: Option<String>,
}

/// Sizes a full close from the wallet's `total` holding across its token accounts and the
/// `primary` account's balance, the one most routes spend from.
///
/// With no other open position of the mint the close sweeps the wallet: the primary balance
/// when the total is split across accounts, the total when the primary account is empty.
/// Tokens the mint's other open positions hold are not this position's to sell: with
/// `others` booked, the sale is at most the total less their holding, and while their
/// holding cannot be attributed it is at most `own_remaining`.
fn exit_sell_amount(
    total: u64,
    primary: u64,
    own_remaining: RawAmount,
    others: OtherOpenHeld,
) -> ExitSizing {
    let (swept, note) = if primary == 0 && total > 0 {
        (total, Some("primary_ata_empty_using_total".to_owned()))
    } else if total > primary && primary > 0 {
        (
            primary,
            Some(format!(
                "multi_account_total={total} primary={primary} shortfall={}, limiting_to_primary",
                total - primary
            )),
        )
    } else {
        (total, None)
    };
    let own_cap = match others {
        OtherOpenHeld::Booked(held) if held == RawAmount::ZERO => {
            return ExitSizing {
                amount: swept,
                note,
            }
        }
        OtherOpenHeld::Booked(held) => {
            u64::try_from(held.raw()).map_or(0, |held| total.saturating_sub(held))
        }
        OtherOpenHeld::Unattributable { .. } => {
            u64::try_from(own_remaining.raw()).unwrap_or(u64::MAX)
        }
    };
    if own_cap >= swept {
        return ExitSizing {
            amount: swept,
            note,
        };
    }
    ExitSizing {
        amount: own_cap,
        note: Some(format!(
            "wallet_total={total} swept={swept} limiting_to_own_holding={own_cap}"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A full and a partial exit sell the open round of the mint, archived or not, the row a
    /// manual sell targets: a lookup that skips archived rows counts the closing row as
    /// another holder and sizes its own sale to nothing.
    #[test]
    fn exits_resolve_the_open_round_archived_or_not() {
        for (file, source) in [
            ("close.rs", include_str!("close.rs")),
            ("partial_close.rs", include_str!("partial_close.rs")),
        ] {
            let code = source.split("#[cfg(test)]").next().unwrap();
            assert!(code.contains("get_open_round_by_mint"), "{file}");
            assert!(!code.contains("get_position_by_mint"), "{file}");
        }
    }

    fn booked(raw: u128) -> OtherOpenHeld {
        OtherOpenHeld::Booked(RawAmount::new(raw))
    }

    const UNATTRIBUTABLE: OtherOpenHeld = OtherOpenHeld::Unattributable { position_id: 7 };

    fn amount(total: u64, primary: u64, own: u128, others: OtherOpenHeld) -> u64 {
        exit_sell_amount(total, primary, RawAmount::new(own), others).amount
    }

    #[test]
    fn a_close_with_no_other_open_position_sweeps_the_wallet_as_before() {
        assert_eq!(amount(2_000, 2_000, 1_000, booked(0)), 2_000);
        assert_eq!(amount(2_000, 1_500, 1_000, booked(0)), 1_500);
        assert_eq!(amount(2_000, 0, 1_000, booked(0)), 2_000);
        assert_eq!(amount(0, 0, 1_000, booked(0)), 0);
    }

    #[test]
    fn a_close_leaves_what_the_other_open_positions_hold() {
        assert_eq!(amount(2_000, 2_000, 1_000, booked(600)), 1_400);
        assert_eq!(amount(2_000, 1_200, 1_000, booked(600)), 1_200);
        assert_eq!(amount(2_000, 2_000, 1_000, booked(2_000)), 0);
        assert_eq!(amount(2_000, 2_000, 1_000, booked(5_000)), 0);
        assert_eq!(amount(2_000, 2_000, 1_000, booked(u128::MAX)), 0);
    }

    #[test]
    fn a_close_beside_a_holding_that_cannot_be_attributed_sells_at_most_its_own_remainder() {
        assert_eq!(amount(2_000, 2_000, 1_000, UNATTRIBUTABLE), 1_000);
        assert_eq!(amount(2_000, 800, 1_000, UNATTRIBUTABLE), 800);
        assert_eq!(amount(900, 900, 1_000, UNATTRIBUTABLE), 900);
        assert_eq!(amount(2_000, 2_000, 0, UNATTRIBUTABLE), 0);
    }

    #[test]
    fn a_capped_sale_says_why() {
        let sizing = exit_sell_amount(2_000, 2_000, RawAmount::new(1_000), booked(600));
        assert_eq!(sizing.amount, 1_400);
        assert!(sizing
            .note
            .is_some_and(|note| note.contains("limiting_to_own_holding=1400")));
        assert_eq!(
            exit_sell_amount(2_000, 2_000, RawAmount::new(1_000), booked(0)).note,
            None
        );
    }
}
