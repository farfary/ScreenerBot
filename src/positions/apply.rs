use crate::{
    positions_types::Position,
    positions_db::{ update_position, force_database_sync },
    utils::safe_truncate,
    logger::{ log, LogTag },
    arguments::is_debug_positions_enabled,
};
use super::{
    state::{ update_position_state, remove_position, release_global_position_permit, POSITIONS },
    transitions::PositionTransition,
    loss_detection::process_position_loss_detection,
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
            let updated = update_position_state(
                &find_mint_by_position_id(position_id).await?,
                |pos| {
                    pos.transaction_entry_verified = true;
                    pos.effective_entry_price = Some(effective_entry_price);
                    pos.total_size_sol = sol_size;
                    pos.token_amount = Some(token_amount_units);
                    pos.entry_fee_lamports = Some(fee_lamports);
                }
            ).await;

            if updated && transition.requires_db_update() {
                if let Some(position) = get_position_by_id(position_id).await {
                    match update_position(&position).await {
                        Ok(_) => {
                            effects.db_updated = true;
                            let _ = force_database_sync().await;
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
            let updated = update_position_state(
                &find_mint_by_position_id(position_id).await?,
                |pos| {
                    pos.transaction_exit_verified = true;
                    pos.effective_exit_price = Some(effective_exit_price);
                    pos.sol_received = Some(sol_received);
                    pos.exit_fee_lamports = Some(fee_lamports);
                    pos.exit_time = Some(exit_time);
                    pos.exit_price = Some(effective_exit_price);
                }
            ).await;

            if updated && transition.requires_db_update() {
                if let Some(position) = get_position_by_id(position_id).await {
                    // Process loss detection and potential blacklisting
                    if let Err(e) = process_position_loss_detection(&position).await {
                        log(
                            LogTag::Positions,
                            "ERROR",
                            &format!(
                                "Failed to process loss detection for {}: {}",
                                position.symbol,
                                e
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

                            log(
                                LogTag::Positions,
                                "SUCCESS",
                                &format!("🔓 Released position slot for verified exit (ID: {})", position_id)
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
            let updated = update_position_state(
                &find_mint_by_position_id(position_id).await?,
                |pos| {
                    pos.exit_transaction_signature = None;
                    pos.transaction_exit_verified = false;
                    pos.closed_reason = Some("exit_retry_pending".to_string());
                }
            ).await;

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

        PositionTransition::ExitPermanentFailureSynthetic { position_id, exit_time } => {
            let updated = update_position_state(
                &find_mint_by_position_id(position_id).await?,
                |pos| {
                    pos.synthetic_exit = true;
                    pos.transaction_exit_verified = true;
                    pos.exit_time = Some(exit_time);
                    pos.closed_reason = Some("synthetic_exit_permanent_failure".to_string());
                }
            ).await;

            if updated && transition.requires_db_update() {
                if let Some(position) = get_position_by_id(position_id).await {
                    match update_position(&position).await {
                        Ok(_) => {
                            effects.db_updated = true;
                            effects.position_closed = true;
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

                    if is_debug_positions_enabled() {
                        log(
                            LogTag::Positions,
                            "DEBUG",
                            &format!("🗑️ Removed orphan entry position {}", position_id)
                        );
                    }
                }
            }
        }

        PositionTransition::UpdatePriceTracking { mint, current_price, highest, lowest } => {
            update_position_state(&mint, |pos| {
                pos.current_price = Some(current_price);
                pos.current_price_updated = Some(Utc::now());
                if let Some(high) = highest {
                    pos.price_highest = high;
                }
                if let Some(low) = lowest {
                    pos.price_lowest = low;
                }
            }).await;
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

async fn get_position_by_id(position_id: i64) -> Option<Position> {
    let positions = POSITIONS.read().await;
    positions
        .iter()
        .find(|p| p.id == Some(position_id))
        .cloned()
}
