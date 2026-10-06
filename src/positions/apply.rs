// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Position transition effects.
//!
//! Applies verified or failed position transitions (entry, exit, DCA, partial-exit).
//! A transition that writes the position row books a candidate copy, commits the row, its
//! idempotence guard and its history record in one transaction, and only then replays the
//! same booking on the in-memory position. Side effects (slot release, loss accounting,
//! events, notifications, pending clears) run after the commit. A failed commit leaves
//! memory, the row and the records unchanged.

use super::booking::{CloseFill, DcaAverage, DcaFill, EntryFill, PartialExitFill};
use super::db::{
    commit_booking, force_database_sync, update_position_price_fields, BookingCommit, BookingGuard,
    BookingRecord,
};
use super::types::{EntryRecord, ExitRecord, Position};
use super::{
    loss_detection::process_position_loss_detection,
    state::{
        clear_pending_dca_swap, get_position_by_id, get_position_by_mint, release_position_slot,
        remove_position, remove_signature_from_index, update_position_state,
        update_position_state_by_id, POSITIONS,
    },
    transitions::PositionTransition,
};
use crate::chains::RawAmount;
use crate::config::with_config;
use crate::logger::{self, LogTag};
use crate::telegram::{queue_notification, Notification};
use chrono::Utc;

use super::error::{Error, Result};

#[derive(Debug)]
pub struct ApplyEffects {
    pub db_updated: bool,
    pub position_removed: bool,
    pub position_closed: bool,
}

/// Apply a position transition to state and database
pub async fn apply_transition(transition: PositionTransition) -> Result<ApplyEffects> {
    let mut effects = ApplyEffects {
        db_updated: false,
        position_removed: false,
        position_closed: false,
    };

    // A verified swap means SOL and tokens have actually moved. Tell the wallet monitor
    // to re-read the balance now, so the header/hero worth reflects the trade within a
    // second or two instead of at the next snapshot interval.
    if transition.affects_wallet_balance() {
        crate::wallet::request_balance_refresh();
    }

    match transition {
        // =================================================================
        // ENTRY
        // =================================================================
        PositionTransition::EntryVerified {
            position_id,
            effective_entry_price,
            token_amount_units,
            fee_raw,
            native_size,
        } => {
            let Some(snapshot) = get_position_by_id(position_id).await else {
                log_missing_position(position_id, "entry verification");
                return Ok(effects);
            };
            let fill = EntryFill {
                effective_entry_price,
                token_amount: token_amount_units,
                fee_raw,
                native_size,
            };
            let mut candidate = snapshot;
            candidate.apply_entry_fill(&fill);
            let record = candidate
                .entry_transaction_signature
                .clone()
                .map(|signature| {
                    BookingRecord::Entry(EntryRecord {
                        id: None,
                        position_id,
                        timestamp: candidate.entry_time,
                        amount: token_amount_units,
                        price: effective_entry_price,
                        native_spent: native_size,
                        transaction_signature: signature,
                        is_dca: false,
                        fees_raw: Some(fee_raw),
                    })
                });

            if commit_booking(&candidate, BookingGuard::Unconditional, record.as_ref()).await?
                == BookingCommit::AlreadyBooked
            {
                return Ok(effects);
            }
            publish_booking(position_id, &candidate, |live| {
                live.apply_entry_fill(&fill);
                Ok(())
            })
            .await;

            effects.db_updated = true;
            let _ = force_database_sync().await;
            crate::events::record_position_event(
                &position_id.to_string(),
                &candidate.mint,
                "entry_verified",
                candidate.entry_transaction_signature.as_deref(),
                None,
                native_size,
                token_amount_units,
                None,
                None,
            )
            .await;

            // Queue Telegram notification for position opened
            if with_config(|c| c.telegram.enabled && c.telegram.notify_position_opened) {
                queue_notification(Notification::position_opened(
                    candidate.symbol.clone(),
                    candidate.mint.clone(),
                    native_size,
                    effective_entry_price,
                ));
            }
        }

        // =================================================================
        // FULL EXIT
        // =================================================================
        PositionTransition::ExitVerified {
            position_id,
            effective_exit_price,
            native_received,
            fee_raw,
            exit_time,
        } => {
            // IDEMPOTENCE: this transition ACCUMULATES (`native_received +=`,
            // `total_exited_amount +=`), so the same close must be booked at most once. The
            // queue dedupes by signature only while an item is IN it, so a re-enqueue can hand
            // the same exit back. The stored `transaction_exit_verified` flag, read inside the
            // booking transaction, decides whether this close is already booked.
            let Some(snapshot) = get_position_by_id(position_id).await else {
                log_missing_position(position_id, "exit verification");
                return Ok(effects);
            };

            // Closed P&L is computed from the position as booked, before anything is written.
            let mut fill = CloseFill {
                effective_exit_price,
                native_received,
                fee_raw,
                exit_time,
                pnl: None,
            };
            let mut probe = snapshot.clone();
            probe.book_close(&fill)?;
            let (pnl_native, pnl_pct) =
                crate::positions::calculate_position_pnl(&probe, None).await;
            fill.pnl = Some((pnl_native, pnl_pct));

            // A full close sells whatever is left: the amount moved is what THIS close sold.
            let mut candidate = snapshot;
            let closed_amount = candidate.book_close(&fill)?;
            // The exit record for the FULL close: the position-details History tab and the
            // chart's exit markers are built from these records.
            let record = candidate
                .exit_transaction_signature
                .clone()
                .map(|signature| {
                    BookingRecord::Exit(ExitRecord {
                        id: None,
                        position_id,
                        timestamp: exit_time,
                        amount: closed_amount,
                        price: effective_exit_price,
                        native_received,
                        transaction_signature: signature,
                        is_partial: false,
                        percentage: 100.0,
                        fees_raw: Some(fee_raw),
                    })
                });

            match commit_booking(&candidate, BookingGuard::ExitNotVerified, record.as_ref()).await?
            {
                BookingCommit::AlreadyBooked => {
                    logger::debug(
                        LogTag::Positions,
                        &format!("Exit for position {position_id} already verified - skipping"),
                    );
                    release_position_slot(position_id).await;
                    return Ok(effects);
                }
                BookingCommit::Committed => {}
            }
            publish_booking(position_id, &candidate, |live| {
                live.book_close(&fill).map(|_| ())
            })
            .await;

            effects.db_updated = true;
            effects.position_closed = true;
            let _ = force_database_sync().await;

            // Release the global position permit now the close is booked, so new positions
            // can be opened within max_open_positions.
            release_position_slot(position_id).await;

            if let Err(e) = process_position_loss_detection(&candidate).await {
                logger::error(
                    LogTag::Positions,
                    &format!(
                        "Failed to process loss detection for {}: {}",
                        candidate.symbol, e
                    ),
                );
            }

            // Record realized loss for loss limit tracking (full exit only). A wallet-derived
            // round is excluded: it is the user's own pre-existing holding, not risk the bot
            // took, and a loss on it must not pause the trader.
            if pnl_native < 0.0 && !candidate.is_wallet_derived() {
                crate::trader::safety::loss_limit::record_realized_loss(pnl_native.abs());
            }

            // Record an exit verified event with basic P&L if computable
            let event_pnl_native = candidate
                .native_received
                .map(|s| s - candidate.total_size_native);
            let event_pnl_pct = candidate.effective_entry_price.and_then(|ep| {
                candidate.effective_exit_price.map(|xp| {
                    if ep > 0.0 {
                        ((xp - ep) / ep) * 100.0
                    } else {
                        0.0
                    }
                })
            });
            crate::events::record_position_event(
                &position_id.to_string(),
                &candidate.mint,
                "exit_verified",
                candidate.entry_transaction_signature.as_deref(),
                candidate.exit_transaction_signature.as_deref(),
                candidate.total_size_native,
                candidate.token_amount.unwrap_or_default(),
                event_pnl_native,
                event_pnl_pct,
            )
            .await;

            logger::info(
                LogTag::Positions,
                &format!(
                    "Released position slot for verified exit (ID: {})",
                    position_id
                ),
            );

            // Reset token priority to Standard after the close, so a stale OpenPosition
            // priority does not outlive the position.
            if let Some(db) = crate::tokens::database::get_global_database() {
                let _ = db.update_priority(
                    &candidate.mint,
                    crate::tokens::priorities::Priority::Standard.to_value(),
                );
                logger::debug(
                    LogTag::Positions,
                    &format!(
                        "Reset token {} to Standard priority after close",
                        candidate.symbol
                    ),
                );
            }

            // Queue Telegram notification for position closed
            if with_config(|c| c.telegram.enabled && c.telegram.notify_position_closed) {
                let duration_secs = candidate
                    .exit_time
                    .map(|exit| (exit - candidate.entry_time).num_seconds().max(0) as u64)
                    .unwrap_or_default();
                queue_notification(Notification::position_closed(
                    candidate.symbol.clone(),
                    candidate.mint.clone(),
                    candidate.pnl.unwrap_or_default(),
                    candidate.pnl_percent.unwrap_or_default(),
                    candidate.closed_reason.clone(),
                    candidate.average_entry_price,
                    candidate.effective_exit_price.unwrap_or_default(),
                    candidate.total_size_native,
                    candidate.native_received.unwrap_or_default(),
                    duration_secs,
                ));
            }
        }

        // =================================================================
        // EXIT FAILURE / RETRY
        // =================================================================
        PositionTransition::ExitFailedClearForRetry { position_id } => {
            let Some(snapshot) = get_position_by_id(position_id).await else {
                log_missing_position(position_id, "exit retry clear");
                return Ok(effects);
            };
            let mut candidate = snapshot;
            // The old signature is purged from the index once the clear is stored, so no stale
            // sig->mint mapping remains.
            let old_sig = candidate.clear_failed_exit();

            // A failed sell must never reopen a close that is already verified (a force close
            // or a synthetic exit committed while the sell was still being verified). The
            // stored verified flag is read inside the clear's own transaction.
            match commit_booking(&candidate, BookingGuard::ExitNotVerified, None).await? {
                BookingCommit::AlreadyBooked => {
                    logger::warning(
                        LogTag::Positions,
                        &format!(
                            "Exit retry clear for position {position_id} refused - the exit is already verified"
                        ),
                    );
                    return Err(Error::AlreadyClosed { position_id });
                }
                BookingCommit::Committed => {}
            }
            publish_booking(position_id, &candidate, |live| {
                live.clear_failed_exit();
                Ok(())
            })
            .await;
            effects.db_updated = true;

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
        }

        PositionTransition::ExitPermanentFailureSynthetic {
            position_id,
            exit_time,
        } => {
            // A synthetic exit writes the position off: the tokens are gone (or the exit can
            // no longer be verified), and no SOL comes back for whatever was still held. The
            // realized P&L makes it visible to the period trading stats and the loss limiter.
            // Realized proceeds from earlier partial exits still stand; only the remainder is
            // written off. The stored `transaction_exit_verified` flag guards against booking
            // (and counting the loss) twice.
            let Some(snapshot) = get_position_by_id(position_id).await else {
                log_missing_position(position_id, "synthetic exit");
                return Ok(effects);
            };
            let mut candidate = snapshot;
            let realized_pnl = match candidate.book_synthetic_close(exit_time) {
                Ok(realized_pnl) => realized_pnl,
                Err(error) => {
                    logger::error(
                        LogTag::Positions,
                        &format!("Synthetic exit for position {position_id} not applied: {error}"),
                    );
                    return Err(error);
                }
            };

            match commit_booking(&candidate, BookingGuard::ExitNotVerified, None).await? {
                BookingCommit::AlreadyBooked => {
                    logger::debug(
                        LogTag::Positions,
                        &format!(
                            "Exit for position {position_id} already verified - synthetic exit skipped"
                        ),
                    );
                    release_position_slot(position_id).await;
                    return Ok(effects);
                }
                BookingCommit::Committed => {}
            }
            publish_booking(position_id, &candidate, |live| {
                live.book_synthetic_close(exit_time).map(|_| ())
            })
            .await;
            effects.db_updated = true;
            effects.position_closed = true;

            if realized_pnl < 0.0 {
                crate::trader::safety::loss_limit::record_realized_loss(realized_pnl.abs());
            }

            crate::events::record_position_event(
                &position_id.to_string(),
                &candidate.mint,
                "exit_synthetic",
                candidate.entry_transaction_signature.as_deref(),
                candidate.exit_transaction_signature.as_deref(),
                candidate.total_size_native,
                candidate.remaining_token_amount.unwrap_or_default(),
                None,
                None,
            )
            .await;

            // Release global slot for synthetic exits as well
            release_position_slot(position_id).await;
            logger::debug(
                LogTag::Positions,
                &format!(
                    "Released position slot for synthetic exit (ID: {})",
                    position_id
                ),
            );

            // Reset token priority after synthetic exit
            if let Some(db) = crate::tokens::database::get_global_database() {
                let _ = db.update_priority(
                    &candidate.mint,
                    crate::tokens::priorities::Priority::Standard.to_value(),
                );
                logger::debug(
                    LogTag::Positions,
                    &format!(
                        "Reset token {} to Standard priority after synthetic exit",
                        candidate.symbol
                    ),
                );
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

                    // Reset token priority after orphan removal
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
                let native_estimate = (exit_amount.raw() as f64 / 10_f64.powi(9)) * market_price;
                crate::events::record_position_event(
                    &position_id.to_string(),
                    &position.mint,
                    "partial_exit_submitted",
                    position.entry_transaction_signature.as_deref(),
                    Some(&exit_signature),
                    native_estimate,
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
            native_received,
            effective_exit_price,
            fee_raw,
            exit_time,
            exit_signature,
            exit_percentage,
        } => {
            // IDEMPOTENCE: the booking ACCUMULATES (remaining -=, total_exited +=,
            // native_received +=, partial_exit_count += 1). Applying the same partial twice
            // would sell the same tokens twice on paper. The exit record is the token: one
            // swap = one record, checked and written in the same transaction as the row.
            let Some(snapshot) = get_position_by_id(position_id).await else {
                log_missing_position(position_id, "partial exit verification");
                return Ok(effects);
            };

            // Unrealized P&L after the partial is computed from the position as booked, so it
            // is current at once instead of on the next price tick.
            let mut fill = PartialExitFill {
                exit_amount,
                native_received,
                effective_exit_price,
                unrealized_pnl: None,
            };
            let mut probe = snapshot.clone();
            probe.book_partial_exit(&fill)?;
            if let Some(current_price) = probe.current_price {
                fill.unrealized_pnl = Some(
                    crate::positions::calculate_position_pnl(&probe, Some(current_price)).await,
                );
            } else {
                logger::debug(
                    LogTag::Positions,
                    &format!(
                        "No current price available for {} after partial exit, PnL will update on next price tick",
                        probe.symbol
                    ),
                );
            }

            // CRITICAL: the booking sets neither exit_time nor the exit signature - the
            // position is still open.
            let mut candidate = snapshot;
            candidate.book_partial_exit(&fill)?;
            let record = BookingRecord::Exit(ExitRecord {
                id: None,
                position_id,
                timestamp: exit_time,
                amount: exit_amount,
                price: effective_exit_price,
                native_received,
                transaction_signature: exit_signature.clone(),
                is_partial: true,
                percentage: exit_percentage,
                fees_raw: Some(fee_raw),
            });

            match commit_booking(
                &candidate,
                BookingGuard::ExitRecordAbsent(&exit_signature),
                Some(&record),
            )
            .await?
            {
                BookingCommit::AlreadyBooked => {
                    logger::debug(
                        LogTag::Positions,
                        &format!(
                            "Partial exit {exit_signature} already recorded for position {position_id} - skipping"
                        ),
                    );
                    // Still drop the pending marks, or the mint stays flagged as "a partial
                    // exit is confirming" and every later exit for it is refused. The per-mint
                    // counter drops only once the detail is gone: it must fall exactly once per
                    // signature, or a later in-flight partial loses its serialization.
                    super::state::clear_pending_partial_exit(&exit_signature).await?;
                    super::state::clear_partial_exit_pending(&candidate.mint).await;
                    return Ok(effects);
                }
                BookingCommit::Committed => {}
            }
            publish_booking(position_id, &candidate, |live| {
                live.book_partial_exit(&fill).map(|_| ())
            })
            .await;

            effects.db_updated = true;
            let _ = force_database_sync().await;

            // A failed pending clear must not skip the remaining effects of a booking that is
            // already stored; it is returned last, and a retry clears it on the
            // already-booked path.
            let pending_cleared = super::state::clear_pending_partial_exit(&exit_signature).await;
            if let Err(err) = &pending_cleared {
                logger::error(
                    LogTag::Positions,
                    &format!(
                        "Failed to clear pending partial exit {exit_signature} for position {position_id}: {err}"
                    ),
                );
            }

            // Realized P&L for THIS partial: proceeds minus the cost basis of the tokens
            // sold, scaled by the token's real decimals.
            let sold_tokens =
                match crate::tokens::get_decimals(crate::chains::active_chain(), &candidate.mint)
                    .await
                {
                    Some(decimals) => exit_amount.to_whole_units(decimals),
                    None => 0.0,
                };
            let partial_pnl = if sold_tokens > 0.0 {
                Some(native_received - (sold_tokens * candidate.average_entry_price))
            } else {
                None
            };

            // NOTE: a partial exit must NOT be fed to the loss limiter.
            //
            // The limiter's unit of account is the CLOSED POSITION: its baseline
            // is rebuilt by `initialize_from_history` from
            // `get_period_trading_stats`, which sums `pnl` over positions with
            // `transaction_exit_verified = 1 AND exit_time IS NOT NULL`. Feeding
            // it a partial would (a) double-count, because the close then records
            // the position's TOTAL pnl — which already includes this partial's
            // proceeds — and (b) not survive a restart, since the rebuilt
            // baseline only sees closed positions. The loss lands, in full and
            // exactly once, when the position closes.

            crate::events::record_position_event(
                &position_id.to_string(),
                &candidate.mint,
                "partial_exit_verified",
                candidate.entry_transaction_signature.as_deref(),
                None,
                native_received,
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
                    candidate.remaining_token_amount.unwrap_or_default()
                ),
            );
            // The per-mint counter drops only with the detail it counts; after a failed
            // detail clear the retry's already-booked path performs the single decrement.
            if pending_cleared.is_ok() {
                super::state::clear_partial_exit_pending(&candidate.mint).await;
            }

            // Queue Telegram notification for partial exit
            if with_config(|c| c.telegram.enabled && c.telegram.notify_partial_exit) {
                // Remaining percentage against tokens ever ACQUIRED (still held + already
                // exited). `token_amount` is only the entry buy and does not grow on a DCA.
                let remaining_pct = if let Some(remaining) = candidate.remaining_token_amount {
                    match candidate.acquired_amount() {
                        Some(acquired) if acquired > RawAmount::ZERO => {
                            (remaining.raw() as f64 / acquired.raw() as f64) * 100.0
                        }
                        _ => 0.0,
                    }
                } else {
                    100.0 - exit_percentage
                };
                queue_notification(Notification::partial_exit(
                    candidate.symbol.clone(),
                    candidate.mint.clone(),
                    exit_percentage,
                    partial_pnl.unwrap_or_default(),
                    remaining_pct,
                ));
            }

            // IMPORTANT: Do NOT release semaphore permit - position still open!
            pending_cleared?;
        }

        PositionTransition::ExitResidualClearForRetry {
            position_id,
            exit_amount,
            native_received,
            effective_exit_price,
            fee_raw,
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
                    position_id, exit_amount, native_received
                ),
            );

            Box::pin(apply_transition(PositionTransition::PartialExitVerified {
                position_id,
                exit_amount,
                native_received,
                effective_exit_price,
                fee_raw,
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
                    position.total_size_native,
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
            dca_amount_native,
            market_price,
        } => {
            // Record DCA submitted event
            if let Some(position) = get_position_by_id(position_id).await {
                let token_estimate = (dca_amount_native / market_price) * 10_f64.powi(9);
                crate::events::record_position_event(
                    &position_id.to_string(),
                    &position.mint,
                    "dca_submitted",
                    position.entry_transaction_signature.as_deref(),
                    Some(&dca_signature),
                    dca_amount_native,
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
                    position_id, dca_amount_native, market_price
                ),
            );
            // No state update needed for submission - just logging
        }

        PositionTransition::DcaVerified {
            position_id,
            tokens_bought,
            native_spent,
            effective_price,
            fee_raw,
            dca_time,
            dca_signature,
        } => {
            // IDEMPOTENCE: the booking ACCUMULATES (tokens, invested SOL, dca_count). The
            // entry record is the token: one swap = one record, checked and written in the
            // same transaction as the row.
            let snapshot = get_position_by_id(position_id)
                .await
                .ok_or(Error::NotFoundById { position_id })?;

            // Get token decimals for accurate price calculation
            let decimals =
                crate::tokens::get_decimals(crate::chains::active_chain(), &snapshot.mint)
                    .await
                    .unwrap_or(9); // Default to 9 if not found

            let fill = DcaFill {
                tokens_bought,
                native_spent,
                dca_time,
                decimals,
            };
            let mut candidate = snapshot;
            let booking = candidate.book_dca(&fill)?;
            match booking.average {
                DcaAverage::Recomputed => {}
                DcaAverage::InvalidNormalization => logger::error(
                    LogTag::Positions,
                    &format!(
                        "DCA: Invalid token normalization for position {} (remaining={}, decimals={})",
                        position_id, booking.remaining, decimals
                    ),
                ),
                DcaAverage::InvalidState => logger::error(
                    LogTag::Positions,
                    &format!(
                        "DCA: Invalid position state for average price calculation - position_id={}, remaining_tokens={}, total_size_native={}",
                        position_id, booking.remaining, candidate.total_size_native
                    ),
                ),
            }
            let record = BookingRecord::Entry(EntryRecord {
                id: None,
                position_id,
                timestamp: dca_time,
                amount: tokens_bought,
                price: effective_price,
                native_spent,
                transaction_signature: dca_signature.clone(),
                is_dca: true,
                fees_raw: Some(fee_raw),
            });

            match commit_booking(
                &candidate,
                BookingGuard::EntryRecordAbsent(&dca_signature),
                Some(&record),
            )
            .await?
            {
                BookingCommit::AlreadyBooked => {
                    logger::debug(
                        LogTag::Positions,
                        &format!(
                            "DCA {dca_signature} already recorded for position {position_id} - skipping"
                        ),
                    );
                    clear_pending_dca_swap(&dca_signature).await?;
                    return Ok(effects);
                }
                BookingCommit::Committed => {}
            }
            publish_booking(position_id, &candidate, |live| {
                live.book_dca(&fill).map(|_| ())
            })
            .await;

            effects.db_updated = true;
            let _ = force_database_sync().await;

            // A failed pending clear must not skip the remaining effects of a booking that is
            // already stored; it is returned last, and a retry clears it on the
            // already-booked path.
            let pending_cleared = clear_pending_dca_swap(&dca_signature).await;
            if let Err(err) = &pending_cleared {
                logger::error(
                    LogTag::Positions,
                    &format!(
                        "Failed to clear pending DCA {dca_signature} for position {position_id}: {err}"
                    ),
                );
            }

            crate::events::record_position_event(
                &position_id.to_string(),
                &candidate.mint,
                "dca_verified",
                candidate.entry_transaction_signature.as_deref(),
                None,
                native_spent,
                tokens_bought,
                None,
                None,
            )
            .await;

            logger::info(
                LogTag::Positions,
                &format!(
                    "DCA verified for position {}: {} tokens bought, new average entry: {:.11}",
                    position_id, tokens_bought, candidate.average_entry_price
                ),
            );

            // Queue Telegram notification for DCA executed
            if with_config(|c| c.telegram.enabled && c.telegram.notify_dca_executed) {
                queue_notification(Notification::dca_executed(
                    candidate.symbol.clone(),
                    candidate.mint.clone(),
                    native_spent,
                    candidate.total_size_native,
                    candidate.dca_count,
                ));
            }

            // IMPORTANT: Do NOT consume another semaphore permit - same position!
            pending_cleared?;
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
                    position.total_size_native,
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
                logger::error(
                    LogTag::Positions,
                    &format!(
                        "Failed to clear pending DCA {dca_signature} after failure of position {position_id}: {err}"
                    ),
                );
                return Err(err);
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
                pos.current_price_source = None;
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

/// Publishes a committed booking to the in-memory position by replaying the same pure
/// booking on it. A replay that fails publishes the committed candidate instead.
pub(super) async fn publish_booking(
    position_id: i64,
    candidate: &Position,
    replay: impl FnOnce(&mut Position) -> Result<()>,
) {
    let mut replay_error = None;
    let published = update_position_state_by_id(position_id, |live| {
        if let Err(error) = replay(live) {
            *live = candidate.clone();
            replay_error = Some(error);
        }
    })
    .await;

    if let Some(error) = replay_error {
        logger::error(
            LogTag::Positions,
            &format!(
                "Replaying the committed booking on position {position_id} failed ({error}); published the committed state"
            ),
        );
    }
    if !published {
        logger::warning(
            LogTag::Positions,
            &format!(
                "Position {position_id} left memory before its committed booking was published"
            ),
        );
    }
}

fn log_missing_position(position_id: i64, transition: &str) {
    logger::warning(
        LogTag::Positions,
        &format!("Position {position_id} is not in memory - {transition} not applied"),
    );
}

async fn find_mint_by_position_id(position_id: i64) -> Result<String> {
    let positions = POSITIONS.read().await;
    positions
        .iter()
        .find(|p| p.id == Some(position_id))
        .map(|p| p.mint.clone())
        .ok_or(Error::NotFoundById { position_id })
}
