// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Position transition effects.
//!
//! Applies verified or failed position transitions (entry, exit, DCA, partial-exit).
//! A transition that writes the position row books onto the row read inside its own
//! transaction: the idempotence check, the booking, the row and its history record commit
//! together, and memory then adopts the committed row. Side effects (slot release, loss
//! accounting, events, notifications, pending clears) run after the commit. A failed
//! commit leaves memory, the row and the records unchanged.
//!
//! A verified swap is never refused or dropped. One that lands on a closed position books
//! its own leg there, and the position then follows its round (`round_state::follow_round`):
//! it reopens when the wallet still holds more than dust attributable to it, otherwise it
//! stays closed with its realized P&L restated.

use super::booking::{CloseFill, DcaAverage, DcaFill, EntryFill, PartialExitFill};
use super::db::{
    commit_booking, force_database_sync, get_store_chain, update_position_price_fields, Booking,
    BookingReads, BookingRecord, Committed,
};
use super::ledger::is_wallet_history_close_reason;
use super::pnl::position_pnl;
use super::round_state::{attributable_held, follow_round, is_closed, FollowOutcome};
use super::types::{EntryRecord, ExitRecord, Position};
use super::{
    loss_detection::process_position_loss_detection,
    state::{
        clear_pending_dca_swap, get_position_by_id, get_position_by_mint, other_swap_in_flight,
        position_has_pending_swap, publish_committed, register_position_slot,
        release_position_slot, remove_position_by_id, remove_signature_from_index,
        try_consume_global_position_permit, update_position_state, with_booking_lock,
    },
    transitions::PositionTransition,
};
use crate::chains::{ChainId, RawAmount};
use crate::config::with_config;
use crate::i18n::{ids, UiArg, UiText};
use crate::logger::{self, LogTag};
use crate::telegram::{queue_notification, Notification};
use chrono::{DateTime, Utc};

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
            held_after,
        } => {
            let snapshot = get_position_by_id(position_id)
                .await
                .ok_or(Error::NotFoundById { position_id })?;
            let reading = LateFillReading::of(
                &snapshot.mint,
                position_id,
                snapshot.entry_transaction_signature.as_deref(),
                held_after,
            )
            .await;
            let store_chain = get_store_chain().await?;
            let fill = EntryFill {
                effective_entry_price,
                token_amount: token_amount_units,
                fee_raw,
                native_size,
            };
            // IDEMPOTENCE: the entry fill OVERWRITES the held amount and the size, so booking
            // it again after a partial exit or a DCA would undo them. The verified flag and
            // the entry record, read inside the booking transaction, decide whether the entry
            // is already booked.
            let committed = book_position(position_id, |row, reads| {
                let booked = row.transaction_entry_verified
                    || match row.entry_transaction_signature.as_deref() {
                        Some(signature) => reads.entry_record_exists(signature)?,
                        None => false,
                    };
                if booked {
                    return Ok(Booking::Skip(None));
                }
                let after_write_off = is_closed(row).then_some(row.synthetic_exit);
                row.apply_entry_fill(&fill);
                let late = after_write_off
                    .map(|after_write_off| {
                        late_fill(row, reads, reading, store_chain, after_write_off)
                    })
                    .transpose()?;
                let record = row.entry_transaction_signature.clone().map(|signature| {
                    BookingRecord::Entry(EntryRecord {
                        id: None,
                        position_id,
                        timestamp: row.entry_time,
                        amount: token_amount_units,
                        price: effective_entry_price,
                        native_spent: native_size,
                        transaction_signature: signature,
                        is_dca: false,
                        fees_raw: Some(fee_raw),
                    })
                });
                Ok(Booking::Write {
                    record,
                    outcome: late,
                })
            })
            .await?;
            let Committed::Written {
                row: position,
                outcome: late,
            } = committed
            else {
                logger::debug(
                    LogTag::Positions,
                    &format!("Entry for position {position_id} already verified - skipping"),
                );
                return Ok(effects);
            };

            effects.db_updated = true;
            let _ = force_database_sync().await;
            crate::events::record_position_event(
                &position_id.to_string(),
                &position.mint,
                "entry_verified",
                position.entry_transaction_signature.as_deref(),
                None,
                native_size,
                token_amount_units,
                None,
                None,
            )
            .await;

            // A late entry on a row that stays closed opened nothing; the late-fill event
            // reports it.
            if !is_closed(&position)
                && with_config(|c| c.telegram.enabled && c.telegram.notify_position_opened)
            {
                queue_notification(Notification::position_opened(
                    position.symbol.clone(),
                    position.mint.clone(),
                    native_size,
                    effective_entry_price,
                ));
            }

            if let Some(late) = late {
                let signature = position
                    .entry_transaction_signature
                    .clone()
                    .unwrap_or_default();
                after_late_fill(
                    &position,
                    late,
                    LateFillLeg {
                        kind: "entry",
                        signature: &signature,
                        tokens: token_amount_units,
                        native: native_size,
                    },
                )
                .await;
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
            exit_signature,
            exit_amount,
            held_after,
        } => {
            // IDEMPOTENCE: this transition ACCUMULATES (`native_received +=`,
            // `total_exited_amount +=`), so the same close must be booked at most once. The
            // queue dedupes by signature only while an item is IN it, so a re-enqueue can hand
            // the same exit back. The exit record of the swap, read inside the booking
            // transaction, decides whether it is already booked.
            let snapshot = get_position_by_id(position_id)
                .await
                .ok_or(Error::NotFoundById { position_id })?;
            let reading = LateFillReading::of(
                &snapshot.mint,
                position_id,
                Some(&exit_signature),
                held_after,
            )
            .await;
            let store_chain = get_store_chain().await?;
            let fill = CloseFill {
                effective_exit_price,
                native_received,
                fee_raw,
                exit_time,
            };

            let committed = book_position(position_id, |row, reads| {
                if reads.exit_record_exists(&exit_signature)? {
                    return Ok(Booking::Skip(None));
                }
                let late = if is_closed(row) {
                    // A close verified through this very swap before exit records were
                    // written already holds it.
                    if !row.synthetic_exit
                        && row.exit_transaction_signature.as_deref()
                            == Some(exit_signature.as_str())
                    {
                        return Ok(Booking::Skip(None));
                    }
                    if ledger_close_counts(row, exit_time) {
                        logger::info(
                            LogTag::Positions,
                            &format!(
                                "Sale {exit_signature} of position {position_id} is already in the proceeds of its close from wallet history"
                            ),
                        );
                        return Ok(Booking::Skip(None));
                    }
                    let after_write_off = row.synthetic_exit;
                    row.book_late_close(&fill);
                    Some(late_fill(
                        row,
                        reads,
                        reading,
                        store_chain,
                        after_write_off,
                    )?)
                } else {
                    if row.transaction_exit_verified {
                        return Ok(Booking::Skip(None));
                    }
                    // A submission whose row write failed left the exit signature in memory
                    // only; the verified swap supplies it to the row.
                    if row.exit_transaction_signature.is_none() {
                        row.exit_transaction_signature = Some(exit_signature.clone());
                    }
                    row.book_close(&fill)?;
                    None
                };
                // The exit record for the FULL close, with what the swap sold: the
                // position-details History tab and the chart's exit markers are built from
                // these records.
                Ok(Booking::Write {
                    record: Some(BookingRecord::Exit(ExitRecord {
                        id: None,
                        position_id,
                        timestamp: exit_time,
                        amount: exit_amount,
                        price: effective_exit_price,
                        native_received,
                        transaction_signature: exit_signature.clone(),
                        is_partial: false,
                        percentage: 100.0,
                        fees_raw: Some(fee_raw),
                    })),
                    outcome: late,
                })
            })
            .await?;
            let (candidate, late) = match committed {
                Committed::Skipped(_) => {
                    logger::debug(
                        LogTag::Positions,
                        &format!(
                            "Exit {exit_signature} already booked for position {position_id} - skipping"
                        ),
                    );
                    return Ok(effects);
                }
                Committed::Written { row, outcome } => (row, outcome),
                Committed::Deleted { .. } => return Err(Error::NotFoundById { position_id }),
            };

            effects.db_updated = true;
            effects.position_closed = is_closed(&candidate);
            let _ = force_database_sync().await;

            if late.is_none() {
                // Release the global position permit now the close is booked, so new
                // positions can be opened within max_open_positions.
                release_position_slot(position_id).await;
                logger::info(
                    LogTag::Positions,
                    &format!("Released position slot for verified exit (ID: {position_id})"),
                );

                if let Err(e) = process_position_loss_detection(&candidate).await {
                    logger::error(
                        LogTag::Positions,
                        &format!(
                            "Failed to process loss detection for {}: {}",
                            candidate.symbol, e
                        ),
                    );
                }

                // The loss limiter follows the books, which now hold this close.
                crate::trader::safety::loss_limit::sync_from_books().await;

                // Reset token priority to Standard after the close, so a stale OpenPosition
                // priority does not outlive the position.
                if let Some(db) = crate::tokens::database::database(store_chain) {
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
                Some(&exit_signature),
                candidate.total_size_native,
                candidate.token_amount.unwrap_or_default(),
                event_pnl_native,
                event_pnl_pct,
            )
            .await;

            // A late sale on a row that was already closed announced its close then; the
            // late-fill event reports the sale.
            if effects.position_closed
                && late.is_none()
                && with_config(|c| c.telegram.enabled && c.telegram.notify_position_closed)
            {
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

            if let Some(late) = late {
                after_late_fill(
                    &candidate,
                    late,
                    LateFillLeg {
                        kind: "exit",
                        signature: &exit_signature,
                        tokens: exit_amount,
                        native: native_received,
                    },
                )
                .await;
            }
        }

        // =================================================================
        // EXIT FAILURE / RETRY
        // =================================================================
        PositionTransition::ExitFailedClearForRetry {
            position_id,
            exit_signature,
        } => {
            return clear_exit_for_retry(position_id, exit_signature, ExitClearCause::SaleFailed)
                .await;
        }

        // =================================================================
        // ORPHAN CLEANUP
        // =================================================================
        PositionTransition::RemoveOrphanEntry {
            position_id,
            signature,
            evidence,
        } => {
            // The row is deleted only while it still describes nothing but an entry that never
            // landed: its entry is `signature` and unverified, no full exit was submitted, no
            // DCA or partial exit is booked on it or in flight in the pending DCA and
            // partial-exit maps, and it carries no entry or exit record. The delete cascades to
            // the row's records, so any booked or in-flight fill must refuse it. The settlement
            // holds the mint's position lock, under which a DCA or partial exit registers its
            // pending swap, so no new one can be submitted between this check and the delete.
            let mint = match get_position_by_id(position_id).await {
                Some(row) => Some(row.mint),
                None => super::db::get_position_by_id(position_id)
                    .await?
                    .map(|row| row.mint),
            };
            let in_flight = match &mint {
                Some(mint) => position_has_pending_swap(mint, position_id).await,
                None => false,
            };
            if in_flight {
                return Err(Error::EntryLanded {
                    position_id,
                    signature: signature.clone(),
                });
            }
            let store_chain = get_store_chain().await?;
            let committed = book_position(position_id, |row, reads| {
                let unlanded = row.entry_transaction_signature.as_deref()
                    == Some(signature.as_str())
                    && !row.transaction_entry_verified
                    && !row.transaction_exit_verified
                    && row.exit_transaction_signature.is_none()
                    && row.dca_count == 0
                    && row.partial_exit_count == 0
                    && !reads.has_any_record()?;
                if !unlanded {
                    return Err(Error::EntryLanded {
                        position_id,
                        signature: signature.clone(),
                    });
                }
                Ok(Booking::Delete { outcome: () })
            })
            .await?;
            let Committed::Deleted { row: removed, .. } = committed else {
                return Ok(effects);
            };
            effects.db_updated = true;
            effects.position_removed = true;

            release_position_slot(position_id).await;

            if let Some(db) = crate::tokens::database::database(store_chain) {
                let _ = db.update_priority(
                    &removed.mint,
                    crate::tokens::priorities::Priority::Standard.to_value(),
                );
            }

            crate::events::record_position_event_flexible(
                "entry_not_landed",
                crate::events::Severity::Info,
                Some(&removed.mint),
                Some(&signature),
                crate::events::with_text(
                    serde_json::json!({
                        "position_id": position_id,
                        "signature": signature,
                        "evidence": evidence,
                    }),
                    &UiText::new(ids::EVENTS_POSITION_ENTRY_NOT_LANDED)
                        .arg("symbol", UiArg::Text(removed.symbol.clone())),
                ),
            )
            .await;

            logger::info(
                LogTag::Positions,
                &format!(
                    "Removed position {position_id}: its entry {signature} did not land ({evidence:?})"
                ),
            );
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
            held_after,
        } => {
            // IDEMPOTENCE: the booking ACCUMULATES (remaining -=, total_exited +=,
            // native_received +=, partial_exit_count += 1). Applying the same partial twice
            // would sell the same tokens twice on paper. The exit record is the token: one
            // swap = one record, checked and written in the same transaction as the row.
            let snapshot = get_position_by_id(position_id)
                .await
                .ok_or(Error::NotFoundById { position_id })?;
            // The live price is the price updater's, in memory; the token's decimals value
            // the tokens still held and the tokens sold.
            let live_price = snapshot.current_price;
            let decimals =
                crate::tokens::get_decimals(crate::chains::active_chain(), &snapshot.mint).await;
            if live_price.is_none() {
                logger::debug(
                    LogTag::Positions,
                    &format!(
                        "No current price available for {} after partial exit, PnL will update on next price tick",
                        snapshot.symbol
                    ),
                );
            }

            let reading = LateFillReading::of(
                &snapshot.mint,
                position_id,
                Some(&exit_signature),
                held_after,
            )
            .await;
            let store_chain = get_store_chain().await?;
            let committed = book_position(position_id, |row, reads| {
                if reads.exit_record_exists(&exit_signature)? {
                    return Ok(Booking::Skip(None));
                }
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
                if is_closed(row) {
                    let after_write_off = row.synthetic_exit;
                    row.book_late_partial_exit(native_received);
                    let late = late_fill(row, reads, reading, store_chain, after_write_off)?;
                    return Ok(Booking::Write {
                        record: Some(record),
                        outcome: Some(late),
                    });
                }
                // Unrealized P&L after the partial is computed from the position as booked,
                // so it is current at once instead of on the next price tick.
                let mut fill = PartialExitFill {
                    exit_amount,
                    native_received,
                    effective_exit_price,
                    unrealized_pnl: None,
                };
                let mut probe = row.clone();
                probe.book_partial_exit(&fill)?;
                fill.unrealized_pnl =
                    live_price.map(|price| position_pnl(&probe, Some(price), decimals));

                // CRITICAL: the booking sets neither exit_time nor the exit signature - the
                // position is still open.
                row.book_partial_exit(&fill)?;
                Ok(Booking::Write {
                    record: Some(record),
                    outcome: None,
                })
            })
            .await?;
            let Committed::Written {
                row: candidate,
                outcome: late,
            } = committed
            else {
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
                super::state::clear_partial_exit_pending(&snapshot.mint).await;
                return Ok(effects);
            };

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
            let sold_tokens = match decimals {
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

            if let Some(late) = late {
                after_late_fill(
                    &candidate,
                    late,
                    LateFillLeg {
                        kind: "partial_exit",
                        signature: &exit_signature,
                        tokens: exit_amount,
                        native: native_received,
                    },
                )
                .await;
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
            held_after,
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
                exit_signature: exit_signature.clone(),
                exit_percentage,
                held_after,
            }))
            .await?;

            // A sale that landed on a closed row leaves no exit to retry: its leg is booked,
            // and the residual is the round's, not this row's.
            match clear_exit_for_retry(position_id, exit_signature, ExitClearCause::ResidualBooked)
                .await
            {
                Ok(cleared) => effects.db_updated = cleared.db_updated,
                Err(Error::AlreadyClosed { .. }) => effects.db_updated = true,
                Err(error) => return Err(error),
            }
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
            held_after,
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
            };
            let reading = LateFillReading::of(
                &snapshot.mint,
                position_id,
                Some(&dca_signature),
                held_after,
            )
            .await;
            let store_chain = get_store_chain().await?;
            let committed = book_position(position_id, |row, reads| {
                if reads.entry_record_exists(&dca_signature)? {
                    return Ok(Booking::Skip(None));
                }
                let after_write_off = is_closed(row).then_some(row.synthetic_exit);
                row.book_dca(&fill)?;
                let late = after_write_off
                    .map(|after_write_off| {
                        late_fill(row, reads, reading, store_chain, after_write_off)
                    })
                    .transpose()?;
                // An open or reopened position averages its cost over what it now holds; a
                // row that stays closed keeps the average its round was priced at.
                if late.is_none_or(|late| late.follow == FollowOutcome::Reopened) {
                    let remaining = row.remaining_token_amount.unwrap_or_default();
                    match row.recompute_average_entry_price(decimals) {
                        DcaAverage::Recomputed => {}
                        DcaAverage::InvalidNormalization => logger::error(
                            LogTag::Positions,
                            &format!(
                                "DCA: Invalid token normalization for position {position_id} \
                                 (remaining={remaining}, decimals={decimals})"
                            ),
                        ),
                        DcaAverage::InvalidState => logger::error(
                            LogTag::Positions,
                            &format!(
                                "DCA: Invalid position state for average price calculation - \
                                 position_id={position_id}, remaining_tokens={remaining}, \
                                 total_size_native={}",
                                row.total_size_native
                            ),
                        ),
                    }
                }
                Ok(Booking::Write {
                    record: Some(BookingRecord::Entry(EntryRecord {
                        id: None,
                        position_id,
                        timestamp: dca_time,
                        amount: tokens_bought,
                        price: effective_price,
                        native_spent,
                        transaction_signature: dca_signature.clone(),
                        is_dca: true,
                        fees_raw: Some(fee_raw),
                    })),
                    outcome: late,
                })
            })
            .await?;
            let Committed::Written {
                row: candidate,
                outcome: late,
            } = committed
            else {
                logger::debug(
                    LogTag::Positions,
                    &format!(
                        "DCA {dca_signature} already recorded for position {position_id} - skipping"
                    ),
                );
                clear_pending_dca_swap(&dca_signature).await?;
                return Ok(effects);
            };

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

            if let Some(late) = late {
                after_late_fill(
                    &candidate,
                    late,
                    LateFillLeg {
                        kind: "dca",
                        signature: &dca_signature,
                        tokens: tokens_bought,
                        native: native_spent,
                    },
                )
                .await;
            }

            // A DCA never consumes a semaphore permit of its own: it is the same position.
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

/// Why a close's exit is cleared for a retry.
#[derive(Debug, Clone, Copy)]
enum ExitClearCause {
    /// The sale did not land.
    SaleFailed,
    /// The sale landed and is booked as a partial exit; a residual is left to close.
    ResidualBooked,
}

/// Clears the exit `exit_signature` of position `position_id` so the close can be retried.
async fn clear_exit_for_retry(
    position_id: i64,
    exit_signature: String,
    cause: ExitClearCause,
) -> Result<ApplyEffects> {
    let mut effects = ApplyEffects {
        db_updated: false,
        position_removed: false,
        position_closed: false,
    };
    if get_position_by_id(position_id).await.is_none() {
        log_missing_position(position_id, "exit retry clear");
        return Ok(effects);
    }
    // A failed sell must never reopen a close that is already verified (a force close
    // or a synthetic exit committed while the sell was still being verified), and
    // never clears an exit other than its own: a newer exit submitted after it stays.
    // On a written-off row the failed sell is the one the write-off left in flight;
    // it is dropped from the row, so it is not verified again, and the write-off
    // stands. The stored row is read inside the clear's own transaction. Once the
    // clear is stored, the failed swap's signature is purged from the index, so no
    // stale sig->mint mapping remains. The failed swap comes from the transition: a
    // submission whose row write failed never put it on the row.
    let committed = book_position(position_id, |row, _| {
        if row.transaction_exit_verified {
            if row.synthetic_exit
                && row.exit_transaction_signature.as_deref() == Some(exit_signature.as_str())
            {
                row.drop_failed_written_off_sale();
                return Ok(Booking::Write {
                    record: None,
                    outcome: ExitClear::WrittenOffSaleDropped,
                });
            }
            return Ok(Booking::Skip(ExitClear::ExitVerified));
        }
        if let Some(other) = row
            .exit_transaction_signature
            .as_deref()
            .filter(|signature| *signature != exit_signature)
        {
            return Ok(Booking::Skip(ExitClear::OtherExit(other.to_owned())));
        }
        row.clear_failed_exit();
        Ok(Booking::Write {
            record: None,
            outcome: ExitClear::Cleared,
        })
    })
    .await?;
    match committed {
        Committed::Skipped(ExitClear::ExitVerified) => {
            logger::warning(
                LogTag::Positions,
                &format!(
                    "Exit retry clear for position {position_id} refused - the exit is already verified"
                ),
            );
            return Err(Error::AlreadyClosed { position_id });
        }
        Committed::Skipped(ExitClear::OtherExit(other)) => {
            logger::warning(
                LogTag::Positions,
                &format!(
                    "Exit retry clear of {exit_signature} for position {position_id} skipped - its exit is now {other}"
                ),
            );
            remove_signature_from_index(&exit_signature).await;
            return Ok(effects);
        }
        Committed::Written {
            outcome: ExitClear::WrittenOffSaleDropped,
            ..
        } => {
            let message = match cause {
                ExitClearCause::SaleFailed => format!(
                    "Sale {exit_signature} of written-off position {position_id} did not land - the write-off stands"
                ),
                ExitClearCause::ResidualBooked => format!(
                    "Sale {exit_signature} of written-off position {position_id} landed with a residual and is booked - the write-off stands"
                ),
            };
            logger::warning(LogTag::Positions, &message);
            effects.db_updated = true;
            remove_signature_from_index(&exit_signature).await;
            return Ok(effects);
        }
        Committed::Skipped(ExitClear::Cleared | ExitClear::WrittenOffSaleDropped)
        | Committed::Written { .. } => {}
        Committed::Deleted { .. } => return Err(Error::NotFoundById { position_id }),
    }
    effects.db_updated = true;

    remove_signature_from_index(&exit_signature).await;
    crate::events::record_position_event_flexible(
        "exit_retry_cleared",
        crate::events::Severity::Warn,
        None,
        Some(&exit_signature),
        serde_json::json!({
          "position_id": position_id
        }),
    )
    .await;
    Ok(effects)
}

/// Books onto the stored row of `position_id` (see [`commit_booking`]) and publishes the
/// committed row to memory, or drops a deleted one from it, both under the position's
/// booking lock, so memory adopts the rows of one position in commit order.
pub(crate) async fn book_position<T>(
    position_id: i64,
    book: impl FnOnce(&mut Position, &BookingReads<'_>) -> Result<Booking<T>>,
) -> Result<Committed<T>> {
    with_booking_lock(position_id, async {
        let committed = commit_booking(position_id, book).await?;
        match &committed {
            Committed::Written { row, .. } => {
                if !publish_committed(row).await {
                    logger::debug(
                        LogTag::Positions,
                        &format!(
                            "Position {position_id} is not in memory; its committed row is not published"
                        ),
                    );
                }
            }
            Committed::Deleted { .. } => {
                remove_position_by_id(position_id).await;
            }
            Committed::Skipped(_) => {}
        }
        Ok(committed)
    })
    .await
}

/// A verified swap booked onto a position that was already closed.
#[derive(Debug, Clone, Copy)]
struct LateFill {
    /// The position was an operator write-off when the swap was booked.
    after_write_off: bool,
    /// What following the round did with the position.
    follow: FollowOutcome,
}

/// Whether the close of `row` came from the wallet history and its proceeds already count
/// a sale made at `sale_time`. The ledger books a round's close from the chain, with the
/// proceeds of every disposal up to the round's close time, and only when the history
/// reconciles; a sale at or before that time is in them.
fn ledger_close_counts(row: &Position, sale_time: DateTime<Utc>) -> bool {
    !row.synthetic_exit
        && row.history_complete
        && row.transaction_exit_verified
        && is_wallet_history_close_reason(row.closed_reason.as_deref())
        && row.exit_time.is_some_and(|closed| sale_time <= closed)
}

/// What an exit retry clear decided for the row it read.
enum ExitClear {
    Cleared,
    /// The row's exit is verified: a failed sell never reopens a close.
    ExitVerified,
    /// The row's exit is another, newer swap, which the clear leaves in place.
    OtherExit(String),
    /// The row was written off while this sale was in flight; the sale is dropped and the
    /// write-off stands.
    WrittenOffSaleDropped,
}

/// The wallet's holding of a mint after a swap, as a late fill of one position may use it.
#[derive(Debug, Clone, Copy)]
struct LateFillReading {
    /// The holding read after the swap, `None` when it could not be read.
    held_after: Option<RawAmount>,
    /// Another bot swap of the mint may still move the holding, so the reading may or may
    /// not hold its tokens.
    other_swap_in_flight: bool,
}

impl LateFillReading {
    /// The reading for the fill of `signature`, a swap of position `position_id`.
    async fn of(
        mint: &str,
        position_id: i64,
        signature: Option<&str>,
        held_after: Option<RawAmount>,
    ) -> Self {
        Self {
            held_after,
            other_swap_in_flight: other_swap_in_flight(mint, position_id, signature).await,
        }
    }
}

/// Makes a closed `row`, with a late swap's own leg already booked on it, follow its round.
/// The reading's holding is the wallet's holding of the mint after the swap, of which the
/// row owns what the other open positions of the mint do not hold, up to what it acquired.
/// Without that reading, or while another swap of the mint or another position's entry is
/// still unsettled, nothing can be decided, so the booking fails retryably and is read
/// again.
fn late_fill(
    row: &mut Position,
    reads: &BookingReads<'_>,
    reading: LateFillReading,
    chain: ChainId,
    after_write_off: bool,
) -> Result<LateFill> {
    if reading.other_swap_in_flight {
        return Err(Error::HoldingUnattributable {
            mint: row.mint.clone(),
            detail: format!(
                "another swap of the mint is in flight during a fill on closed position {:?}",
                row.id
            ),
        });
    }
    let Some(held_after) = reading.held_after else {
        return Err(crate::chains::Error::SettlementRead {
            chain,
            detail: format!(
                "the holding of {} after a fill on closed position {:?} was not read",
                row.mint, row.id
            ),
        }
        .into());
    };
    let Some(acquired) = row.acquired_amount() else {
        return Err(Error::AmountOverflow {
            mint: row.mint.clone(),
            operation: "following the round",
        });
    };
    let held = attributable_held(held_after, reads.other_open_held(&row.mint)?, acquired);
    Ok(LateFill {
        after_write_off,
        follow: follow_round(row, held)?,
    })
}

/// The leg of a late fill, as the warning event reports it.
struct LateFillLeg<'a> {
    kind: &'static str,
    signature: &'a str,
    tokens: RawAmount,
    native: f64,
}

/// The effects of a late fill once its booking is committed, after the normal effects of its
/// transition. A reopened position that is not archived takes a trading slot back when one
/// is free, the loss limiter follows the restated books, and a fill on a written-off
/// position is recorded as one warning event.
async fn after_late_fill(position: &Position, late: LateFill, leg: LateFillLeg<'_>) {
    let reopened = late.follow == FollowOutcome::Reopened;
    if let (true, false, Some(position_id)) = (reopened, position.archived, position.id) {
        if try_consume_global_position_permit() {
            register_position_slot(position_id).await;
        } else {
            logger::warning(
                LogTag::Positions,
                &format!(
                    "Position {position_id} ({}) reopened by a late fill, but no trading slot is free",
                    position.symbol
                ),
            );
        }
    }

    crate::trader::safety::loss_limit::sync_from_books().await;

    logger::warning(
        LogTag::Positions,
        &format!(
            "A {} fill {} landed on closed position {:?} ({}): {:?}",
            leg.kind, leg.signature, position.id, position.symbol, late.follow
        ),
    );
    if late.after_write_off {
        crate::events::record_position_event_flexible(
            "fill_after_force_close",
            crate::events::Severity::Warn,
            Some(&position.mint),
            Some(leg.signature),
            crate::events::with_text(
                serde_json::json!({
                    "position_id": position.id,
                    "kind": leg.kind,
                    "signature": leg.signature,
                    "tokens": leg.tokens,
                    "native": leg.native,
                    "restated_pnl": position.pnl,
                    "reopened": reopened,
                    "trigger": "verifier",
                }),
                &UiText::new(ids::EVENTS_POSITION_FILL_AFTER_FORCE_CLOSE)
                    .arg("symbol", UiArg::Text(position.symbol.clone())),
            ),
        )
        .await;
    }
}

fn log_missing_position(position_id: i64, transition: &str) {
    logger::warning(
        LogTag::Positions,
        &format!("Position {position_id} is not in memory - {transition} not applied"),
    );
}
