use super::db::{force_database_sync, update_position};
use super::{
    loss_detection::process_position_loss_detection,
    state::{
        get_position_by_id, release_global_position_permit, remove_position,
        remove_signature_from_index, update_position_state, POSITIONS,
    },
    transitions::PositionTransition,
};
use crate::{
    arguments::is_debug_positions_enabled,
    learner,
    logger::{log, LogTag},
};
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

    match transition {
        PositionTransition::EntryVerified {
            position_id,
            effective_entry_price,
            token_amount_units,
            fee_lamports,
            sol_size,
        } => {
            let updated =
                update_position_state(&find_mint_by_position_id(position_id).await?, |pos| {
                    pos.transaction_entry_verified = true;
                    pos.effective_entry_price = Some(effective_entry_price);
                    pos.total_size_sol = sol_size;
                    pos.token_amount = Some(token_amount_units);
                    pos.entry_fee_lamports = Some(fee_lamports);
                })
                .await;

            if updated && transition.requires_db_update() {
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
                        }
                        Err(e) => {
                            return Err(format!("Failed to update database: {}", e));
                        }
                    }
                }
            }
        }

        PositionTransition::ExitVerified {
            position_id,
            effective_exit_price,
            sol_received,
            fee_lamports,
            exit_time,
        } => {
            let updated =
                update_position_state(&find_mint_by_position_id(position_id).await?, |pos| {
                    pos.transaction_exit_verified = true;
                    pos.effective_exit_price = Some(effective_exit_price);
                    pos.sol_received = Some(sol_received);
                    pos.exit_fee_lamports = Some(fee_lamports);
                    pos.exit_time = Some(exit_time);
                    // Note: exit_price is already set by close_position_direct to market price
                })
                .await;

            if updated && transition.requires_db_update() {
                if let Some(position) = get_position_by_id(position_id).await {
                    // Process loss detection and potential blacklisting
                    if let Err(e) = process_position_loss_detection(&position).await {
                        log(
                            LogTag::Positions,
                            "ERROR",
                            &format!(
                                "Failed to process loss detection for {}: {}",
                                position.symbol, e
                            ),
                        );
                    }

                    // Record completed trade for learning system
                    let entry_price = position
                        .effective_entry_price
                        .unwrap_or(position.entry_price);
                    let max_up_pct = if entry_price > 0.0 {
                        ((position.price_highest - entry_price) / entry_price) * 100.0
                    } else {
                        0.0
                    };
                    let max_down_pct = if entry_price > 0.0 {
                        ((entry_price - position.price_lowest) / entry_price) * 100.0
                    } else {
                        0.0
                    };

                    if let Err(e) =
                        learner::record_completed_trade(&position, max_up_pct, max_down_pct).await
                    {
                        log(
                            LogTag::Positions,
                            "WARN",
                            &format!(
                                "Failed to record trade for learning: {} ({})",
                                position.symbol, e
                            ),
                        );
                    } else if is_debug_positions_enabled() {
                        let current_pnl = if let (Some(exit_price), entry_price) =
                            (position.exit_price, entry_price)
                        {
                            if entry_price > 0.0 {
                                ((exit_price - entry_price) / entry_price) * 100.0
                            } else {
                                0.0
                            }
                        } else {
                            0.0
                        };
                        log(
                            LogTag::Positions,
                            "LEARNER",
                            &format!(
                                "🧠 Recorded completed trade for learning: {} (profit: {:.2}%, peak: {:.2}%)",
                                position.symbol,
                                current_pnl,
                                max_up_pct
                            )
                        );
                    }

                    match update_position(&position).await {
                        Ok(_) => {
                            effects.db_updated = true;
                            effects.position_closed = true;
                            let _ = force_database_sync().await;

                            // CRITICAL: Release global position permit when position is verified closed
                            // This allows new positions to be opened, fixing the MAX_OPEN_POSITIONS limit
                            release_global_position_permit();

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
                                position.token_amount.unwrap_or(0),
                                pnl_sol,
                                pnl_pct,
                            )
                            .await;

                            log(
                                LogTag::Positions,
                                "SUCCESS",
                                &format!(
                                    "🔓 Released position slot for verified exit (ID: {})",
                                    position_id
                                ),
                            );
                        }
                        Err(e) => {
                            return Err(format!("Failed to update database: {}", e));
                        }
                    }
                }
            }
        }

        PositionTransition::ExitFailedClearForRetry { position_id } => {
            let mint = find_mint_by_position_id(position_id).await?;
            // Capture old signature to purge index entry (prevent stale sig->mint mapping)
            let mut old_sig: Option<String> = None;
            let updated = update_position_state(&mint, |pos| {
                if let Some(sig) = pos.exit_transaction_signature.clone() {
                    old_sig = Some(sig);
                }
                pos.exit_transaction_signature = None;
                pos.transaction_exit_verified = false;
                pos.closed_reason = Some("exit_retry_pending".to_string());
            })
            .await;

            if let Some(sig) = old_sig {
                remove_signature_from_index(&sig).await;
                crate::events::record_safe(crate::events::Event::new(
                    crate::events::EventCategory::Position,
                    Some("exit_retry_cleared".to_string()),
                    crate::events::Severity::Warn,
                    None,
                    Some(sig),
                    serde_json::json!({
                        "position_id": position_id
                    }),
                ))
                .await;
            }

            if updated && transition.requires_db_update() {
                if let Some(position) = get_position_by_id(position_id).await {
                    match update_position(&position).await {
                        Ok(_) => {
                            effects.db_updated = true;
                        }
                        Err(e) => {
                            return Err(format!("Failed to update database: {}", e));
                        }
                    }
                }
            }
        }

        PositionTransition::ExitPermanentFailureSynthetic {
            position_id,
            exit_time,
        } => {
            let updated =
                update_position_state(&find_mint_by_position_id(position_id).await?, |pos| {
                    pos.synthetic_exit = true;
                    pos.transaction_exit_verified = true;
                    pos.exit_time = Some(exit_time);
                    pos.closed_reason = Some("synthetic_exit_permanent_failure".to_string());
                })
                .await;

            if updated && transition.requires_db_update() {
                if let Some(position) = get_position_by_id(position_id).await {
                    match update_position(&position).await {
                        Ok(_) => {
                            effects.db_updated = true;
                            effects.position_closed = true;
                            // Release global slot for synthetic exits as well
                            release_global_position_permit();
                            if is_debug_positions_enabled() {
                                log(
                                    LogTag::Positions,
                                    "DEBUG",
                                    &format!(
                                        "🔓 Released position slot for synthetic exit (ID: {})",
                                        position_id
                                    ),
                                );
                            }
                        }
                        Err(e) => {
                            return Err(format!("Failed to update database: {}", e));
                        }
                    }
                }
            }
        }

        PositionTransition::RemoveOrphanEntry { position_id } => {
            if let Ok(mint) = find_mint_by_position_id(position_id).await {
                if let Some(_) = remove_position(&mint).await {
                    effects.position_removed = true;
                    crate::events::record_safe(crate::events::Event::new(
                        crate::events::EventCategory::Position,
                        Some("orphan_entry_removed".to_string()),
                        crate::events::Severity::Warn,
                        Some(mint.clone()),
                        None,
                        serde_json::json!({
                            "position_id": position_id
                        }),
                    ))
                    .await;

                    if is_debug_positions_enabled() {
                        log(
                            LogTag::Positions,
                            "DEBUG",
                            &format!("🗑️ Removed orphan entry position {}", position_id),
                        );
                    }

                    // Orphan entries also occupied a slot originally; free it now
                    release_global_position_permit();
                    if is_debug_positions_enabled() {
                        log(
                            LogTag::Positions,
                            "DEBUG",
                            &format!(
                                "🔓 Released position slot after orphan removal (ID: {})",
                                position_id
                            ),
                        );
                    }
                    // NOTE: position removal already purged signature indexes. Optionally we could
                    // attempt to prune per-mint lock map here if implemented in state.
                }
            }
        }

        PositionTransition::UpdatePriceTracking {
            mint,
            current_price,
            highest,
            lowest,
        } => {
            update_position_state(&mint, |pos| {
                pos.current_price = Some(current_price);
                pos.current_price_updated = Some(Utc::now());
                if let Some(high) = highest {
                    pos.price_highest = high;
                }
                if let Some(low) = lowest {
                    pos.price_lowest = low;
                }
            })
            .await;
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
        .ok_or_else(|| format!("Position not found: {}", position_id))
}
