// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Partial close operations — sell a percentage of remaining tokens without closing the position.

use crate::chains::adapter;
use crate::chains::RawAmount;
use crate::logger::{self, LogTag};
use crate::positions::apply::apply_transition;
use crate::positions::queue::{enqueue_verification, VerificationItem};
use crate::positions::state::{
    acquire_position_lock, add_signature_to_index, register_pending_partial_exit,
};
use crate::positions::transitions::PositionTransition;
use crate::positions::types::{PartialExitHold, PendingPartialExit};
use crate::positions::{Error, Result};
use crate::swaps::{execute_swap_with_fallback, get_best_quote, QuoteRequest, SwapMode};
use crate::utils::get_wallet_address;
use chrono::Utc;

use super::exit_ladder::{run_exit_ladder, ExitSwap};
use super::exit_sizing::{read_exit_sale, ExitSale, ExitShare};

/// Partially close a position by selling a percentage of remaining tokens.
/// `take_profit_level` is the take-profit step the sale takes, reserved on the position
/// before the swap is sent and returned to the ladder only when the swap provably sold
/// nothing.
/// CRITICAL: This does NOT release the semaphore permit - position stays open
pub async fn partial_close_position(
    token_mint: &str,
    exit_percentage: f64,
    exit_reason: &str,
    slippage_pct: Option<f64>,
    take_profit_level: Option<u32>,
) -> Result<String> {
    super::refuse_until_reconciled("partial_exit", token_mint)?;
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
    // A held partial exit (sent with no proof either way) counts as in flight until the
    // chain proves its outcome.
    if crate::positions::state::is_partial_exit_pending(token_mint).await {
        return Err(Error::PartialExitPending {
            mint: token_mint.to_owned(),
        });
    }

    // The open round of the mint, archived or not: the row a manual sell targets.
    let position = crate::positions::state::get_open_round_by_mint(token_mint)
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
        .held_amount()
        .ok_or_else(|| Error::TransitionFailed {
            transition: "partial_exit",
            mint: token_mint.to_owned(),
            detail: "position has no token amount".to_owned(),
        })?;

    // Size against what the WALLET actually holds as well as what the position books
    // (`exit_sizing`): the lower of the two, so a drifted row never sells more than the
    // wallet has and a wallet never gives up more than the row owns. A wallet that cannot
    // be read fails the exit; a full close stays available and sizes from the wallet itself.
    // The holding read here is what a held sale's outcome is proven against.
    let wallet_address = get_wallet_address().map_err(|e| Error::WalletUnavailable {
        detail: e.to_string(),
    })?;

    let ExitSale {
        amount: exit_amount,
        wallet_holding: holding_before,
        ..
    } = read_exit_sale(
        &wallet_address,
        token_mint,
        Some(&position),
        ExitShare::Part(exit_percentage),
    )
    .await?;
    if holding_before != remaining_amount {
        logger::warning(
            LogTag::Positions,
            &format!(
                "Partial exit balance drift for {}: position={} on-chain={} — sizing against the lower",
                position.symbol, remaining_amount, holding_before
            ),
        );
    }

    if exit_amount == RawAmount::ZERO {
        return Err(Error::ZeroExitAmount {
            mint: token_mint.to_owned(),
        });
    }

    // Get API token for swap
    let api_token = crate::tokens::get_full_token_async(crate::chains::active_chain(), token_mint)
        .await
        .map_err(|_| Error::TokenNotFound {
            mint: token_mint.to_owned(),
        })?
        .ok_or_else(|| Error::TokenNotFound {
            mint: token_mint.to_owned(),
        })?;

    // Manual override starts the ladder; configured steps above it still escalate.
    let slippage_exit_retry_steps = super::slippage::exit_slippage_ladder(slippage_pct);

    if let Some(level) = take_profit_level {
        reserve_take_profit_step(token_mint, position_id, level).await?;
    }

    logger::info(
        LogTag::Positions,
        &format!(
            "Partial exit initiated: {} | {}% ({} of {} booked tokens) | Reason: {}",
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

    // Mark the partial pending BEFORE the first submission: the per-mint lock is released
    // as soon as this function returns, long before verification lands, so this registry is
    // what stops a second exit from being sized against tokens this one has already sold.
    crate::positions::state::mark_partial_exit_pending(token_mint).await;

    // ONE ladder: quote and swap at each slippage rung in turn, sending again
    // only after a failure that provably sold nothing.
    let base_request = QuoteRequest {
        chain: crate::chains::active_chain(),
        input_mint: token_mint.to_string(),
        output_mint: adapter().native_asset_address().to_string(),
        input_amount: exit_amount,
        wallet_address: wallet_address.clone(),
        // The ladder sets the slippage and any excluded venue per rung.
        slippage_pct: 0.0,
        swap_mode: SwapMode::ExactIn,
        exclude_dexes: None,
    };
    let token = &api_token;
    let exit = run_exit_ladder(
        "Partial exit",
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
            logger::info(
                LogTag::Positions,
                &format!(
                    "Partial exit quote ({}% slippage): {} tokens -> {} SOL",
                    f64::from(quote.slippage_bps) / 100.0,
                    exit_amount,
                    quote.output_amount.raw() as f64 / adapter().raw_units_per_native() as f64
                ),
            );
            execute_swap_with_fallback(token, quote, crate::swaps::SwapAmountLimit::Unrestricted)
        },
    )
    .await;

    let transaction_signature = match exit {
        ExitSwap::Executed(result) => result.transaction_signature,
        // May still land: record the signature and verify it like any other.
        ExitSwap::Submitted(signature) => signature,
        ExitSwap::Failed {
            detail,
            not_submitted,
        } => {
            // Nothing was sold, so the step returns to the ladder.
            if let Some(level) = take_profit_level {
                release_take_profit_step(token_mint, position_id, level).await;
            }
            return Err(PartialExitFailure {
                position: &position,
                position_id,
                exit_amount,
                exit_percentage,
            }
            .record(token_mint, detail, not_submitted)
            .await);
        }
        // Nothing proves whether the share was sold: the partial exit is held until the
        // chain proves its outcome, and a reserved step stays taken until the hold proves
        // nothing sold.
        ExitSwap::Unresolved { detail } => {
            return Err(PartialExitFailure {
                position: &position,
                position_id,
                exit_amount,
                exit_percentage,
            }
            .hold(token_mint, holding_before, take_profit_level, detail)
            .await);
        }
    };

    let expiry_height = crate::positions::settle::submission_expiry_bound().await;

    let pending_partial = PendingPartialExit {
        signature: transaction_signature.clone(),
        mint: token_mint.to_string(),
        position_id,
        expected_exit_amount: exit_amount,
        requested_exit_percentage: exit_percentage,
        expiry_height,
        created_at: Utc::now(),
        take_profit_level,
    };

    // The swap was sent, so from here nothing may drop its signature: every bookkeeping
    // failure below is logged and the verification is queued regardless. The pending
    // details make the partial exit durable across a restart.
    if let Err(e) = register_pending_partial_exit(pending_partial).await {
        logger::error(
            LogTag::Positions,
            &format!(
                "Pending partial exit {transaction_signature} for position {position_id} (mint {token_mint}) is held in memory only, not persisted: {e}"
            ),
        );
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
        logger::error(
            LogTag::Positions,
            &format!(
                "Partial exit {transaction_signature} for position {position_id} was not recorded as submitted: {e}"
            ),
        );
    }

    // Enqueue for verification with partial exit flag
    let verification_item = VerificationItem::new_partial_exit(
        transaction_signature.clone(),
        token_mint.to_string(),
        Some(position_id),
        exit_amount,
        exit_percentage,
        expiry_height,
    )
    .with_take_profit_level(take_profit_level);

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

/// The partial exit whose swap failed: what its failure record names.
struct PartialExitFailure<'a> {
    position: &'a crate::positions::types::Position,
    position_id: i64,
    exit_amount: RawAmount,
    exit_percentage: f64,
}

impl PartialExitFailure<'_> {
    /// Clears the pending partial of `token_mint`, records the failure event and returns
    /// the error the caller reports.
    async fn record(
        self,
        token_mint: &str,
        detail: String,
        not_submitted: Option<crate::swaps::NotSubmittedReason>,
    ) -> Error {
        crate::positions::state::clear_partial_exit_pending(token_mint).await;
        crate::events::record_position_event(
            &self.position_id.to_string(),
            token_mint,
            "partial_exit_swap_failed",
            self.position.entry_transaction_signature.as_deref(),
            None,
            0.0,
            self.exit_amount,
            None,
            Some(self.exit_percentage),
        )
        .await;
        Error::SwapFailed {
            mint: token_mint.to_owned(),
            detail,
            not_submitted,
        }
    }

    /// Holds the partial exit of `token_mint` whose last send failed with no signature,
    /// then clears its pending mark: the hold is registered first, so the mint never reads
    /// free of partial exits in between. Records the hold event and returns the error the
    /// caller reports.
    async fn hold(
        self,
        token_mint: &str,
        holding_before: RawAmount,
        take_profit_level: Option<u32>,
        detail: String,
    ) -> Error {
        let hold = PartialExitHold {
            mint: token_mint.to_owned(),
            position_id: self.position_id,
            exit_amount: self.exit_amount,
            requested_exit_percentage: self.exit_percentage,
            holding_before,
            take_profit_level,
            expiry_height: crate::positions::settle::submission_expiry_bound().await,
            created_at: Utc::now(),
        };
        crate::positions::swap_hold::hold_partial_exit(hold).await;
        crate::positions::state::clear_partial_exit_pending(token_mint).await;
        crate::events::record_position_event(
            &self.position_id.to_string(),
            token_mint,
            "partial_exit_held",
            self.position.entry_transaction_signature.as_deref(),
            None,
            0.0,
            self.exit_amount,
            None,
            Some(self.exit_percentage),
        )
        .await;
        logger::warning(
            LogTag::Positions,
            &format!(
                "Partial exit of position {} (mint {token_mint}) is held until the wallet shows whether it sold: {detail}",
                self.position_id
            ),
        );
        Error::PartialExitHeld {
            mint: token_mint.to_owned(),
            detail,
        }
    }
}

/// Takes take-profit step `level` of `position_id` before the swap that sells it is sent,
/// under the mint's position lock. Refuses unless `level` is the step after the last one
/// taken, so a second decision for the same step never sells and a decision made on a stale
/// level never skips a step: the step is booked once, ahead of its sale.
async fn reserve_take_profit_step(mint: &str, position_id: i64, level: u32) -> Result<()> {
    let effects =
        apply_transition(PositionTransition::TakeProfitLevelReserved { position_id, level })
            .await?;
    if effects.db_updated {
        Ok(())
    } else {
        Err(Error::TransitionFailed {
            transition: "partial_exit",
            mint: mint.to_owned(),
            detail: format!("take-profit step {level} is not the next step of the ladder"),
        })
    }
}

/// Returns take-profit step `level` of `position_id` to the ladder after its sale provably
/// sold nothing. A failed write keeps the step taken, which can skip it but never sells it
/// twice.
pub(crate) async fn release_take_profit_step(mint: &str, position_id: i64, level: u32) {
    if let Err(error) =
        apply_transition(PositionTransition::TakeProfitLevelReleased { position_id, level }).await
    {
        logger::error(
            LogTag::Positions,
            &format!(
                "Take-profit step {level} of position {position_id} (mint {mint}) stays taken after its sale sold nothing: {error}"
            ),
        );
    }
}
