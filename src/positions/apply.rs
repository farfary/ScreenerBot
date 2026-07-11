//! Position transition effects.
//!
//! Applies verified or failed position transitions (entry, exit, DCA, partial-exit)
//! to in-memory state and persists them to the database. Each transition variant
//! updates balances, sends notifications, and logs the event.

use super::db::{
    force_database_sync, save_entry_record, save_exit_record, update_position,
    update_position_price_fields,
};
use super::{
    loss_detection::process_position_loss_detection,
    state::{
        clear_pending_dca_swap, get_position_by_id, get_position_by_mint, release_position_slot,
        remove_position, remove_signature_from_index, update_position_state,
        update_position_state_by_id, POSITIONS,
    },
    transitions::PositionTransition,
};
use crate::config::with_config;
use crate::logger::{self, LogTag};
use crate::telegram::{queue_notification, Notification};
use chrono::Utc;

#[derive(Debug)]
pub struct ApplyEffects {
    pub db_updated: bool,
    pub position_removed: bool,
    pub position_closed: bool,
}

/// Apply a position transition to state and database
pub async fn apply_transition(transition: PositionTransition) -> Result<ApplyEffects, String> {
    let mut effects = ApplyEffects {
        db_updated: false,
        position_removed: false,
        position_closed: false,
    };

    let requires_db_update = transition.requires_db_update();

    match transition {
        // =================================================================
        // ENTRY
        // =================================================================
        PositionTransition::EntryVerified {
            position_id,
            effective_entry_price,
            token_amount_units,
            fee_lamports,
            sol_size,
        } => {
            let updated = update_position_state_by_id(position_id, |pos| {
                pos.transaction_entry_verified = true;
                pos.effective_entry_price = Some(effective_entry_price);
                pos.total_size_sol = sol_size;
                pos.token_amount = Some(token_amount_units);
                pos.entry_fee_lamports = Some(fee_lamports);
                pos.entry_size_sol = sol_size;
                pos.remaining_token_amount = Some(token_amount_units);
                pos.average_entry_price = effective_entry_price;
            })
            .await;

            if updated && requires_db_update {
                if let Some(position) = get_position_by_id(position_id).await {
                    match update_position(&position).await {
                        Ok(_) => {
                            effects.db_updated = true;
                            let _ = force_database_sync().await;
                            // Record an entry verified event
                            crate::events::record_position_event(
                                &position_id.to_string(),
                                &position.mint,
                                "entry_verified",
                                position.entry_transaction_signature.as_deref(),
                                None,
                                sol_size,
                                token_amount_units,
                                None,
                                None,
                            )
                            .await;

                            if let Some(entry_sig) = position.entry_transaction_signature.as_deref()
                            {
                                if let Err(err) = save_entry_record(
                                    position_id,
                                    position.entry_time,
                                    token_amount_units,
                                    effective_entry_price,
                                    sol_size,
                                    entry_sig,
                                    false,
                                    Some(fee_lamports),
                                )
                                .await
                                {
                                    logger::error(
                                        LogTag::Positions,
                                        &format!(
                                            "Failed to persist entry history for position {}: {}",
                                            position_id, err
                                        ),
                                    );
                                }
                            }

                            // Queue Telegram notification for position opened
                            if with_config(|c| {
                                c.telegram.enabled && c.telegram.notify_position_opened
                            }) {
                                queue_notification(Notification::position_opened(
                                    position.symbol.clone(),
                                    position.mint.clone(),
                                    sol_size,
                                    effective_entry_price,
                                ));
                            }
                        }
                        Err(e) => {
                            return Err(format!("Failed to update database: {e}"));
                        }
                    }
                }
            }
        }

        // =================================================================
        // FULL EXIT
        // =================================================================
        PositionTransition::ExitVerified {
            position_id,
            effective_exit_price,
            sol_received,
            fee_lamports,
            exit_time,
        } => {
            // Tokens sold by THIS close = whatever was still held. Captured before the
            // update zeroes it, so the exit record below can be written.
            let mut closed_amount: u64 = 0;

            let updated = update_position_state_by_id(position_id, |pos| {
                closed_amount = pos.remaining_token_amount.unwrap_or_default();
                pos.transaction_exit_verified = true;
                pos.effective_exit_price = Some(effective_exit_price);
                // ACCUMULATE: `sol_received` is the position's total proceeds, and partial
                // exits have already added theirs. Overwriting it here (as this did) threw
                // away every SOL taken off the table earlier, so a position that took 50%
                // profit and then closed reported only the final close's proceeds — closed
                // P&L, which is computed straight off this field, understated the profit by
                // the whole partial exit.
                pos.sol_received = Some(pos.sol_received.unwrap_or_default() + sol_received);
                pos.exit_fee_lamports = Some(fee_lamports);
                pos.exit_time = Some(exit_time);

                // A full close sells whatever is left, so nothing remains held. Roll it into
                // the exited total: the Holdings / "% exited" cards read these two fields and
                // otherwise kept showing a closed position's sold tokens as still held.
                if let Some(remaining) = pos.remaining_token_amount {
                    pos.total_exited_amount += remaining;
                    pos.remaining_token_amount = Some(0);
                }

                // CRITICAL FIX: Update closed_reason to remove pending verification suffix
                // This ensures database state matches verification status
                if let Some(reason) = &pos.closed_reason {
                    if reason.ends_with(super::PENDING_VERIFICATION_SUFFIX) {
                        pos.closed_reason = Some(
                            reason
                                .trim_end_matches(super::PENDING_VERIFICATION_SUFFIX)
                                .to_string(),
                        );
                    }
                }

                // Note: exit_price is already set by close_position_direct to market price
            })
            .await;

            if updated && requires_db_update {
                if let Some(position) = get_position_by_id(position_id).await {
                    // Calculate final P&L for closed position BEFORE any database operations
                    let (pnl_sol, pnl_pct) =
                        crate::positions::calculate_position_pnl(&position, None).await;

                    // Atomically update position with PnL in a single operation
                    let pnl_updated = update_position_state_by_id(position_id, |pos| {
                        pos.pnl = Some(pnl_sol);
                        pos.pnl_percent = Some(pnl_pct);
                        // Clear unrealized PnL (position is now closed)
                        pos.unrealized_pnl = None;
                        pos.unrealized_pnl_percent = None;
                    })
                    .await;

                    if !pnl_updated {
                        logger::error(
                            LogTag::Positions,
                            &format!(
                                "Failed to update PnL for closed position {}",
                                position.symbol
                            ),
                        );
                        // Continue anyway - position is closed, PnL is secondary
                    }

                    // Refresh position after PnL update for loss detection
                    if let Some(position) = get_position_by_id(position_id).await {
                        // Process loss detection and potential blacklisting
                        if let Err(e) = process_position_loss_detection(&position).await {
                            logger::error(
                                LogTag::Positions,
                                &format!(
                                    "Failed to process loss detection for {}: {}",
                                    position.symbol, e
                                ),
                            );
                        }

                        // Record realized loss for loss limit tracking (full exit only)
                        // pnl_sol was calculated above via calculate_position_pnl
                        if pnl_sol < 0.0 {
                            crate::trader::safety::loss_limit::record_realized_loss(pnl_sol.abs());
                        }

                        match update_position(&position).await {
                            Ok(_) => {
                                effects.db_updated = true;
                                effects.position_closed = true;
                                let _ = force_database_sync().await;

                                // Persist the exit record for the FULL close. Only the PARTIAL
                                // path used to write one, so a position's final — and largest —
                                // exit was missing from its own history: the position-details
                                // History tab and the chart's exit markers are built from these
                                // records, and showed every partial exit but never the close.
                                if let Some(exit_signature) =
                                    position.exit_transaction_signature.as_deref()
                                {
                                    if let Err(err) = save_exit_record(
                                        position_id,
                                        exit_time,
                                        closed_amount,
                                        effective_exit_price,
                                        sol_received,
                                        exit_signature,
                                        false,
                                        100.0,
                                        Some(fee_lamports),
                                    )
                                    .await
                                    {
                                        logger::error(
                                            LogTag::Positions,
                                            &format!(
                                                "Failed to persist exit record for position {}: {}",
                                                position_id, err
                                            ),
                                        );
                                    }
                                }

                                // CRITICAL: Release global position permit when position is verified closed
                                // This allows new positions to be opened, fixing the MAX_OPEN_POSITIONS limit
                                release_position_slot(position_id).await;

                                // Record an exit verified event with basic P&L if computable
                                let pnl_sol =
                                    position.sol_received.map(|s| s - position.total_size_sol);
                                let pnl_pct = position.effective_entry_price.and_then(|ep| {
                                    position.effective_exit_price.map(|xp| {
                                        if ep > 0.0 {
                                            ((xp - ep) / ep) * 100.0
                                        } else {
                                            0.0
                                        }
                                    })
                                });
                                crate::events::record_position_event(
                                    &position_id.to_string(),
                                    &position.mint,
                                    "exit_verified",
                                    position.entry_transaction_signature.as_deref(),
                                    position.exit_transaction_signature.as_deref(),
                                    position.total_size_sol,
                                    position.token_amount.unwrap_or_default(),
                                    pnl_sol,
                                    pnl_pct,
                                )
                                .await;

                                logger::info(
                                    LogTag::Positions,
                                    &format!(
                                        "Released position slot for verified exit (ID: {})",
                                        position_id
                                    ),
                                );

                                // Bug #25 fix: Reset token priority to Standard (25) after position close
                                // This prevents stale OpenPosition priority after trading ends
                                if let Some(db) = crate::tokens::database::get_global_database() {
                                    let _ = db.update_priority(
                                        &position.mint,
                                        crate::tokens::priorities::Priority::Standard.to_value(),
                                    );
                                    logger::debug(
                                        LogTag::Positions,
                                        &format!(
                                            "Reset token {} to Standard priority after close",
                                            position.symbol
                                        ),
                                    );
                                }

                                // Queue Telegram notification for position closed
                                if with_config(|c| {
                                    c.telegram.enabled && c.telegram.notify_position_closed
                                }) {
                                    let exit_reason = position
                                        .closed_reason
                                        .clone()
                                        .unwrap_or_else(|| "exit".to_owned());
                                    // Use position.pnl and position.pnl_percent which were set in the state update above
                                    let final_pnl_sol = position.pnl.unwrap_or_default();
                                    let final_pnl_pct = position.pnl_percent.unwrap_or_default();
                                    let entry_price = position.average_entry_price;
                                    let exit_price =
                                        position.effective_exit_price.unwrap_or_default();
                                    let invested = position.total_size_sol;
                                    let received = position.sol_received.unwrap_or_default();
                                    let duration_secs = position
                                        .exit_time
                                        .map(|exit| {
                                            (exit - position.entry_time).num_seconds().max(0) as u64
                                        })
                                        .unwrap_or_default();
                                    queue_notification(Notification::position_closed(
                                        position.symbol.clone(),
                                        position.mint.clone(),
                                        final_pnl_sol,
                                        final_pnl_pct,
                                        exit_reason,
                                        entry_price,
                                        exit_price,
                                        invested,
                                        received,
                                        duration_secs,
                                    ));
                                }
                            }
                            Err(e) => {
                                return Err(format!("Failed to update database: {e}"));
                            }
                        }
                    }
                }
            }
        }

        // =================================================================
        // EXIT FAILURE / RETRY
        // =================================================================
        PositionTransition::ExitFailedClearForRetry { position_id } => {
            // Capture old signature to purge index entry (prevent stale sig->mint mapping)
            let mut old_sig: Option<String> = None;
            let updated = update_position_state_by_id(position_id, |pos| {
                if let Some(sig) = pos.exit_transaction_signature.clone() {
                    old_sig = Some(sig);
                }
                pos.exit_transaction_signature = None;
                pos.transaction_exit_verified = false;
                pos.closed_reason = Some("exit_retry_pending".to_owned());
                // The close did not happen: drop the exit price it stamped on the way in.
                // Leaving it set marks a still-OPEN position with exit data, which every
                // "is this closed?" check that looks at exit_price gets wrong.
                pos.exit_price = None;
                pos.effective_exit_price = None;
            })
            .await;

            if let Some(sig) = old_sig {
                remove_signature_from_index(&sig).await;
                crate::events::record_position_event_flexible(
                    "exit_retry_cleared",
                    crate::events::Severity::Warn,
                    None,
                    Some(&sig),
                    serde_json::json!({
                      "position_id": position_id
                    }),
                )
                .await;
            }

            if updated && requires_db_update {
                if let Some(position) = get_position_by_id(position_id).await {
                    match update_position(&position).await {
                        Ok(_) => {
                            effects.db_updated = true;
                        }
                        Err(e) => {
                            return Err(format!("Failed to update database: {e}"));
                        }
                    }
                }
            }
        }

        PositionTransition::ExitPermanentFailureSynthetic {
            position_id,
            exit_time,
        } => {
            // A synthetic exit writes the position off: the tokens are gone (or the exit can
            // no longer be verified), and no SOL comes back for whatever was still held. It
            // recorded NO P&L at all — pnl stayed None — so these positions were invisible to
            // the period trading stats AND to the loss limiter: a rugged position closed this
            // way never counted as a loss anywhere. Realized proceeds from earlier partial
            // exits still stand; only the remainder is written off.
            let mut realized_pnl = 0.0;

            let updated = update_position_state_by_id(position_id, |pos| {
                pos.synthetic_exit = true;
                pos.transaction_exit_verified = true;
                pos.exit_time = Some(exit_time);
                pos.closed_reason = Some("synthetic_exit_permanent_failure".to_owned());

                realized_pnl = pos.sol_received.unwrap_or_default() - pos.total_size_sol;
                pos.pnl = Some(realized_pnl);
                pos.pnl_percent = Some(if pos.total_size_sol > 0.0 {
                    (realized_pnl / pos.total_size_sol) * 100.0
                } else {
                    0.0
                });
                pos.unrealized_pnl = None;
                pos.unrealized_pnl_percent = None;

                // Nothing is held any more — roll the remainder into the exited total.
                if let Some(remaining) = pos.remaining_token_amount {
                    pos.total_exited_amount += remaining;
                    pos.remaining_token_amount = Some(0);
                }
            })
            .await;

            if updated && realized_pnl < 0.0 {
                crate::trader::safety::loss_limit::record_realized_loss(realized_pnl.abs());
            }

            if updated && requires_db_update {
                if let Some(position) = get_position_by_id(position_id).await {
                    // Record synthetic exit event
                    crate::events::record_position_event(
                        &position_id.to_string(),
                        &position.mint,
                        "exit_synthetic",
                        position.entry_transaction_signature.as_deref(),
                        position.exit_transaction_signature.as_deref(),
                        position.total_size_sol,
                        position.remaining_token_amount.unwrap_or_default(),
                        None,
                        None,
                    )
                    .await;

                    match update_position(&position).await {
                        Ok(_) => {
                            effects.db_updated = true;
                            effects.position_closed = true;
                            // Release global slot for synthetic exits as well
                            release_position_slot(position_id).await;
                            logger::debug(
                                LogTag::Positions,
                                &format!(
                                    "Released position slot for synthetic exit (ID: {})",
                                    position_id
                                ),
                            );

                            // Bug #25 fix: Reset token priority after synthetic exit
                            if let Some(db) = crate::tokens::database::get_global_database() {
                                let _ = db.update_priority(
                                    &position.mint,
                                    crate::tokens::priorities::Priority::Standard.to_value(),
                                );
                                logger::debug(
                                    LogTag::Positions,
                                    &format!(
                                        "Reset token {} to Standard priority after synthetic exit",
                                        position.symbol
                                    ),
                                );
                            }
                        }
                        Err(e) => {
                            return Err(format!("Failed to update database: {e}"));
                        }
                    }
                }
            }
        }

        // =================================================================
        // ORPHAN CLEANUP
        // =================================================================
        PositionTransition::RemoveOrphanEntry { position_id } => {
            if let Ok(mint) = find_mint_by_position_id(position_id).await {
                if remove_position(&mint).await.is_some() {
                    effects.position_removed = true;
                    crate::events::record_position_event_flexible(
                        "orphan_entry_removed",
                        crate::events::Severity::Warn,
                        Some(&mint),
                        None,
                        serde_json::json!({
                          "position_id": position_id
                        }),
                    )
                    .await;

                    logger::debug(
                        LogTag::Positions,
                        &format!("Removed orphan entry position {position_id}"),
                    );

                    // Orphan entries also occupied a slot originally; free it now
                    release_position_slot(position_id).await;
                    logger::debug(
                        LogTag::Positions,
                        &format!(
                            "Released position slot after orphan removal (ID: {})",
                            position_id
                        ),
                    );

                    // Bug #25 fix: Reset token priority after orphan removal
                    if let Some(db) = crate::tokens::database::get_global_database() {
                        let _ = db.update_priority(
                            &mint,
                            crate::tokens::priorities::Priority::Standard.to_value(),
                        );
                        logger::debug(
                            LogTag::Positions,
                            &format!(
                                "Reset token priority to Standard after orphan removal (ID: {})",
                                position_id
                            ),
                        );
                    }

                    // NOTE: position removal already purged signature indexes. Optionally we could
                    // attempt to prune per-mint lock map here if implemented in state.
                }
            }
        }

        // ==================== PARTIAL EXIT TRANSITIONS ====================
        PositionTransition::PartialExitSubmitted {
            position_id,
            exit_signature,
            exit_amount,
            exit_percentage,
            market_price,
        } => {
            // Record partial exit submitted event
            if let Some(position) = get_position_by_id(position_id).await {
                let sol_estimate = (exit_amount as f64 / 10_f64.powi(9)) * market_price;
                crate::events::record_position_event(
                    &position_id.to_string(),
                    &position.mint,
                    "partial_exit_submitted",
                    position.entry_transaction_signature.as_deref(),
                    Some(&exit_signature),
                    sol_estimate,
                    exit_amount,
                    None,
                    Some(exit_percentage),
                )
                .await;
            }

            logger::info(
                LogTag::Positions,
                &format!(
                    "Partial exit submitted for position {}: {}% ({} tokens) at price {:.11}",
                    position_id, exit_percentage, exit_amount, market_price
                ),
            );
        }

        PositionTransition::PartialExitVerified {
            position_id,
            exit_amount,
            sol_received,
            effective_exit_price,
            fee_lamports,
            exit_time,
            exit_signature,
            exit_percentage,
        } => {
            let updated = update_position_state_by_id(position_id, |pos| {
                // Update remaining token amount
                if let Some(remaining) = pos.remaining_token_amount {
                    pos.remaining_token_amount = Some(remaining.saturating_sub(exit_amount));
                }

                // Update total exited amount
                pos.total_exited_amount += exit_amount;

                // Calculate new average exit price (weighted average)
                let total_exited = pos.total_exited_amount;
                if total_exited > 0 {
                    if let Some(prev_avg) = pos.average_exit_price {
                        let prev_weight = (total_exited - exit_amount) as f64 / total_exited as f64;
                        let new_weight = exit_amount as f64 / total_exited as f64;
                        pos.average_exit_price =
                            Some((prev_avg * prev_weight) + (effective_exit_price * new_weight));
                    } else {
                        pos.average_exit_price = Some(effective_exit_price);
                    }
                }

                // Increment partial exit count
                pos.partial_exit_count += 1;

                // Update SOL received (cumulative)
                pos.sol_received = Some(pos.sol_received.unwrap_or_default() + sol_received);

                // CRITICAL: Do NOT set exit_time or exit_signature - position still open!
            })
            .await;

            if updated && requires_db_update {
                if let Some(mut position) = get_position_by_id(position_id).await {
                    // Calculate unrealized PnL immediately after partial exit
                    // Don't wait for price updater (eliminates up to 1 second delay)
                    if let Some(current_price) = position.current_price {
                        let (pnl_sol, pnl_pct) = crate::positions::calculate_position_pnl(
                            &position,
                            Some(current_price),
                        )
                        .await;

                        // Update unrealized PnL in memory
                        update_position_state_by_id(position_id, |pos| {
                            pos.unrealized_pnl = Some(pnl_sol);
                            pos.unrealized_pnl_percent = Some(pnl_pct);
                        })
                        .await;

                        // Refresh position to get updated PnL
                        if let Some(updated_pos) = get_position_by_id(position_id).await {
                            position = updated_pos;
                        }
                    } else {
                        logger::debug(
              LogTag::Positions,
              &format!("No current price available for {} after partial exit, PnL will update on next price tick", position.symbol),
            );
                    }

                    match update_position(&position).await {
                        Ok(_) => {
                            effects.db_updated = true;
                            let _ = force_database_sync().await;

                            if let Err(err) = save_exit_record(
                                position_id,
                                exit_time,
                                exit_amount,
                                effective_exit_price,
                                sol_received,
                                &exit_signature,
                                true,
                                exit_percentage,
                                Some(fee_lamports),
                            )
                            .await
                            {
                                logger::error(
                                    LogTag::Positions,
                                    &format!(
                                        "Failed to persist partial exit record for position {}: {}",
                                        position_id, err
                                    ),
                                );
                            }

                            if let Err(err) =
                                super::state::clear_pending_partial_exit(&exit_signature).await
                            {
                                return Err(format!(
                                    "Failed to clear pending partial exit {} for position {}: {}",
                                    exit_signature, position_id, err
                                ));
                            }

                            // Realized P&L for THIS partial: proceeds minus the cost basis of
                            // the tokens sold. Scale by the token's REAL decimals — this was
                            // hardcoded to 10^9 (SOL's), so for any token that is not 9-decimal
                            // the cost basis was off by orders of magnitude, and the number went
                            // to the events log AND the Telegram notification.
                            let sold_tokens = match crate::tokens::get_decimals(&position.mint)
                                .await
                            {
                                Some(decimals) => exit_amount as f64 / 10_f64.powi(decimals as i32),
                                None => 0.0,
                            };
                            let partial_pnl = if sold_tokens > 0.0 {
                                Some(sol_received - (sold_tokens * position.average_entry_price))
                            } else {
                                None
                            };

                            // Feed the period loss limit. `record_realized_loss` was only ever
                            // called on a FULL exit, so losses realized by partial exits (a
                            // partial stop-loss, say) were invisible to the limiter and could
                            // never trip it.
                            if let Some(pnl) = partial_pnl {
                                if pnl < 0.0 {
                                    crate::trader::safety::loss_limit::record_realized_loss(
                                        pnl.abs(),
                                    );
                                }
                            }

                            crate::events::record_position_event(
                                &position_id.to_string(),
                                &position.mint,
                                "partial_exit_verified",
                                position.entry_transaction_signature.as_deref(),
                                None,
                                sol_received,
                                exit_amount,
                                partial_pnl,
                                None,
                            )
                            .await;

                            logger::info(
                                LogTag::Positions,
                                &format!(
 "Partial exit verified for position {}: {} tokens sold, {} remaining",
                  position_id,
                  exit_amount,
                  position.remaining_token_amount.unwrap_or_default()
                ),
                            );
                            // Clear pending mark
                            super::state::clear_partial_exit_pending(&position.mint).await;

                            // Queue Telegram notification for partial exit
                            if with_config(|c| c.telegram.enabled && c.telegram.notify_partial_exit)
                            {
                                // Calculate remaining percentage against tokens ever
                                // ACQUIRED (still held + already exited). `token_amount` is
                                // only the entry buy and does not grow on a DCA, so using it
                                // reported more than 100% still held for any averaged-in
                                // position.
                                let remaining_pct =
                                    if let Some(remaining) = position.remaining_token_amount {
                                        let acquired = remaining + position.total_exited_amount;
                                        if acquired > 0 {
                                            (remaining as f64 / acquired as f64) * 100.0
                                        } else {
                                            0.0
                                        }
                                    } else {
                                        100.0 - exit_percentage
                                    };
                                queue_notification(Notification::partial_exit(
                                    position.symbol.clone(),
                                    position.mint.clone(),
                                    exit_percentage,
                                    partial_pnl.unwrap_or_default(),
                                    remaining_pct,
                                ));
                            }

                            // IMPORTANT: Do NOT release semaphore permit - position still open!
                        }
                        Err(e) => {
                            return Err(format!("Failed to update database: {e}"));
                        }
                    }
                }
            }
        }

        PositionTransition::ExitResidualClearForRetry {
            position_id,
            exit_amount,
            sol_received,
            effective_exit_price,
            fee_lamports,
            exit_time,
            exit_signature,
            exit_percentage,
        } => {
            // The close swap DID sell tokens and DID receive SOL — it just did not empty the
            // wallet (tokens split across accounts; close_position_direct sells the primary
            // ATA only). Book the fill exactly as a partial exit, THEN clear the exit
            // signature so the residual can be closed on the next pass.
            //
            // Previously this was reported as a plain ExitFailedClearForRetry, which recorded
            // nothing: the SOL received vanished from the position's proceeds and the tokens
            // sold were still counted as held.
            logger::warning(
                LogTag::Positions,
                &format!(
                    "Exit for position {} filled only partially ({} tokens, {:.6} SOL) - recording the fill and retrying the residual",
                    position_id, exit_amount, sol_received
                ),
            );

            Box::pin(apply_transition(PositionTransition::PartialExitVerified {
                position_id,
                exit_amount,
                sol_received,
                effective_exit_price,
                fee_lamports,
                exit_time,
                exit_signature,
                exit_percentage,
            }))
            .await?;

            let cleared = Box::pin(apply_transition(
                PositionTransition::ExitFailedClearForRetry { position_id },
            ))
            .await?;

            effects.db_updated = cleared.db_updated;
        }

        PositionTransition::PartialExitFailed {
            position_id,
            reason,
        } => {
            // Record partial exit failure event
            if let Some(position) = get_position_by_id(position_id).await {
                crate::events::record_position_event(
                    &position_id.to_string(),
                    &position.mint,
                    "partial_exit_failed",
                    position.entry_transaction_signature.as_deref(),
                    position.exit_transaction_signature.as_deref(),
                    position.total_size_sol,
                    position.remaining_token_amount.unwrap_or_default(),
                    None,
                    None,
                )
                .await;
            }

            logger::error(
                LogTag::Positions,
                &format!(
                    "Partial exit failed for position {}: {}",
                    position_id, reason
                ),
            );
            // Clear the pending partial by MINT: a partial exit is not recorded on the
            // position (`exit_transaction_signature` means a FULL exit), so the signature
            // this used to read from there was either absent or, worse, some other exit's.
            if let Some(position) = get_position_by_id(position_id).await {
                if let Err(err) =
                    super::state::clear_pending_partial_exits_for_mint(&position.mint).await
                {
                    logger::error(
                        LogTag::Positions,
                        &format!(
                            "Failed to clear pending partial exits for {} during failure handling of position {}: {}",
                            position.mint, position_id, err
                        ),
                    );
                }
                super::state::clear_partial_exit_pending(&position.mint).await;
            }
            // TODO: Implement retry logic if needed
        }

        // ==================== DCA TRANSITIONS ====================
        PositionTransition::DcaSubmitted {
            position_id,
            dca_signature,
            dca_amount_sol,
            market_price,
        } => {
            // Record DCA submitted event
            if let Some(position) = get_position_by_id(position_id).await {
                let token_estimate = (dca_amount_sol / market_price) * 10_f64.powi(9);
                crate::events::record_position_event(
                    &position_id.to_string(),
                    &position.mint,
                    "dca_submitted",
                    position.entry_transaction_signature.as_deref(),
                    Some(&dca_signature),
                    dca_amount_sol,
                    token_estimate as u64,
                    None,
                    None,
                )
                .await;
            }

            logger::info(
                LogTag::Positions,
                &format!(
                    "DCA submitted for position {}: {} SOL at price {:.11}",
                    position_id, dca_amount_sol, market_price
                ),
            );
            // No state update needed for submission - just logging
        }

        PositionTransition::DcaVerified {
            position_id,
            tokens_bought,
            sol_spent,
            effective_price,
            fee_lamports,
            dca_time,
            dca_signature,
        } => {
            // Get mint for decimals lookup
            let mint = find_mint_by_position_id(position_id).await?;

            // Get token decimals for accurate price calculation
            let decimals = crate::tokens::get_decimals(&mint).await.unwrap_or(9); // Default to 9 if not found

            let updated =
        update_position_state_by_id(position_id, |pos| {
          // Update remaining token amount (add new tokens)
          if let Some(remaining) = pos.remaining_token_amount {
            pos.remaining_token_amount = Some(remaining + tokens_bought);
          } else {
            pos.remaining_token_amount = Some(tokens_bought);
          }

          // Update total SOL invested
          pos.total_size_sol += sol_spent;

          // Recalculate average entry price (weighted average) with actual decimals
          // CRITICAL: Validate all inputs to prevent division by zero or invalid calculations
          let remaining_tokens = pos.remaining_token_amount.unwrap_or_default();
          if remaining_tokens > 0 && pos.total_size_sol > 0.0 && pos.total_size_sol.is_finite() {
            let total_tokens_normalized = remaining_tokens as f64
              / 10_f64.powi(decimals as i32);
            if total_tokens_normalized > 0.0 && total_tokens_normalized.is_finite() {
              pos.average_entry_price = pos.total_size_sol / total_tokens_normalized;
            } else {
              logger::error(
                LogTag::Positions,
                &format!(
 "DCA: Invalid token normalization for position {} (remaining={}, decimals={})",
                  position_id, remaining_tokens, decimals
                ),
              );
            }
          } else {
            // Edge case: Invalid state for average price calculation
            logger::error(
              LogTag::Positions,
              &format!(
 "DCA: Invalid position state for average price calculation - position_id={}, remaining_tokens={}, total_size_sol={}",
                position_id, remaining_tokens, pos.total_size_sol
              ),
            );
          }

          // Increment DCA count
          pos.dca_count += 1;

          // Update last DCA time
          pos.last_dca_time = Some(dca_time);
        })
        .await;

            if updated && requires_db_update {
                if let Some(position) = get_position_by_id(position_id).await {
                    match update_position(&position).await {
                        Ok(_) => {
                            effects.db_updated = true;
                            let _ = force_database_sync().await;

                            if let Err(err) = save_entry_record(
                                position_id,
                                dca_time,
                                tokens_bought,
                                effective_price,
                                sol_spent,
                                &dca_signature,
                                true,
                                Some(fee_lamports),
                            )
                            .await
                            {
                                logger::error(
                                    LogTag::Positions,
                                    &format!(
                                        "Failed to persist DCA entry history for position {}: {}",
                                        position_id, err
                                    ),
                                );
                            }

                            if let Err(err) = clear_pending_dca_swap(&dca_signature).await {
                                return Err(format!(
                                    "Failed to clear pending DCA {} for position {}: {}",
                                    dca_signature, position_id, err
                                ));
                            }

                            crate::events::record_position_event(
                                &position_id.to_string(),
                                &position.mint,
                                "dca_verified",
                                position.entry_transaction_signature.as_deref(),
                                None,
                                sol_spent,
                                tokens_bought,
                                None,
                                None,
                            )
                            .await;

                            logger::info(
                                LogTag::Positions,
                                &format!(
 "DCA verified for position {}: {} tokens bought, new average entry: {:.11}",
                  position_id,
                  tokens_bought,
                  position.average_entry_price
                ),
                            );

                            // Queue Telegram notification for DCA executed
                            if with_config(|c| c.telegram.enabled && c.telegram.notify_dca_executed)
                            {
                                queue_notification(Notification::dca_executed(
                                    position.symbol.clone(),
                                    position.mint.clone(),
                                    sol_spent,
                                    position.total_size_sol,
                                    position.dca_count,
                                ));
                            }

                            // IMPORTANT: Do NOT consume another semaphore permit - same position!
                        }
                        Err(e) => {
                            return Err(format!("Failed to update database: {e}"));
                        }
                    }
                }
            }
        }

        PositionTransition::DcaFailed {
            position_id,
            dca_signature,
            reason,
        } => {
            // Record DCA failure event
            if let Some(position) = get_position_by_id(position_id).await {
                crate::events::record_position_event(
                    &position_id.to_string(),
                    &position.mint,
                    "dca_failed",
                    position.entry_transaction_signature.as_deref(),
                    Some(&dca_signature),
                    position.total_size_sol,
                    position.remaining_token_amount.unwrap_or_default(),
                    None,
                    None,
                )
                .await;
            }

            logger::error(
                LogTag::Positions,
                &format!("DCA failed for position {position_id}: {reason}"),
            );

            if let Err(err) = clear_pending_dca_swap(&dca_signature).await {
                return Err(format!(
                    "Failed to clear pending DCA {} after failure: {}",
                    dca_signature, err
                ));
            }
            // TODO: Implement retry logic if needed
        }

        // =================================================================
        // PRICE TRACKING
        // =================================================================
        PositionTransition::UpdatePriceTracking {
            mint,
            current_price,
            highest,
            lowest,
        } => {
            let updated = update_position_state(&mint, |pos| {
                let now = Utc::now();
                pos.current_price = Some(current_price);
                pos.current_price_updated = Some(now);
                if let Some(high) = highest {
                    pos.price_highest = high;
                }
                if let Some(low) = lowest {
                    pos.price_lowest = low;
                }
            })
            .await;

            if updated {
                if let Some(position) = get_position_by_mint(&mint).await {
                    match update_position_price_fields(&position).await {
                        Ok(_) => {
                            effects.db_updated = true;
                        }
                        Err(err) => {
                            logger::error(
                                LogTag::Positions,
                                &format!(
                                    "Failed to persist price update for mint {} (id={:?}): {}",
                                    mint, position.id, err
                                ),
                            );
                        }
                    }
                } else {
                    logger::debug(
                        LogTag::Positions,
                        &format!(
              "Price update transition applied but position missing from state (mint={})",
              mint
            ),
                    );
                }
            }
        }
    }

    Ok(effects)
}

async fn find_mint_by_position_id(position_id: i64) -> Result<String, String> {
    let positions = POSITIONS.read().await;
    positions
        .iter()
        .find(|p| p.id == Some(position_id))
        .map(|p| p.mint.clone())
        .ok_or_else(|| format!("Position not found: {position_id}"))
}
