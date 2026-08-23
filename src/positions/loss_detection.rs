//! Loss detection — identifies positions hitting stop-loss or trailing-stop thresholds.

use super::{
    lib::calculate_position_pnl,
    types::{Position, PositionOrigin},
};
use crate::config::with_config;
use crate::logger::{self, LogTag};
use crate::tokens::cleanup;
use crate::tokens::database::get_global_database;

use super::error::{Error, Result};

// =============================================================================
// LOSS DETECTION AND BLACKLISTING FUNCTIONS
// =============================================================================

/// Process a closed position for loss detection and potential blacklisting
///
/// This function:
/// 1. Calculates the final P&L for the closed position
/// 2. Evaluates if the loss meets blacklisting criteria
/// 3. Adds the token to blacklist if thresholds are exceeded
/// 4. Logs all actions for monitoring and debugging
///
/// # Arguments
/// * `position` - The closed position to analyze
///
/// # Returns
/// * `Ok(())` - Processing completed successfully
/// * `Err(String)` - Error during P&L calculation or blacklisting
pub async fn process_position_loss_detection(position: &Position) -> Result<()> {
    // An imported/wallet-history loss is not the bot's loss and must not blacklist a
    // token the user happened to already be holding at a loss.
    if matches!(position.origin, PositionOrigin::External) {
        return Ok(());
    }

    // Check if loss-based blacklisting is enabled via config
    let loss_blacklist_enabled = with_config(|cfg| cfg.positions.loss_blacklist_enabled);
    if !loss_blacklist_enabled {
        return Ok(());
    }
    if matches!(position.origin, PositionOrigin::Copy { .. })
        && !with_config(|cfg| cfg.positions.loss_blacklist_copy_origins)
    {
        logger::info(
            LogTag::Positions,
            &format!(
                "Copy-origin loss for {} will not alter the global blacklist",
                position.mint
            ),
        );
        return Ok(());
    }

    // Calculate final P&L for loss detection
    let (net_pnl_sol, net_pnl_percent) = calculate_position_pnl(position, None).await;

    // Process loss detection
    if net_pnl_sol < 0.0 {
        let loss_sol = net_pnl_sol.abs();
        logger::warning(
            LogTag::Positions,
            &format!(
                "Loss detected for {} ({}): -{:.3} SOL ({:.1}%)",
                position.symbol, &position.mint, loss_sol, net_pnl_percent
            ),
        );

        // Only blacklist for significant losses to avoid being too aggressive
        let threshold = with_config(|cfg| cfg.positions.loss_blacklist_threshold_pct);
        if net_pnl_percent <= threshold {
            // Add to database-backed blacklist
            if let Some(db) = get_global_database() {
                let reason = match &position.origin {
                    PositionOrigin::Copy {
                        task_id,
                        source_wallet,
                    } => {
                        format!("PoorPerformance:copy:{task_id}:{source_wallet}")
                    }
                    PositionOrigin::Auto { strategy_id } => format!(
                        "PoorPerformance:auto:{}",
                        strategy_id.as_deref().unwrap_or("unattributed")
                    ),
                    PositionOrigin::Manual => "PoorPerformance:manual".to_owned(),
                    // Unreachable: External returns Ok(()) above before this branch.
                    PositionOrigin::External => "PoorPerformance:external".to_owned(),
                };
                match cleanup::blacklist_token(&position.mint, &reason, &db) {
                    Ok(_) => {
                        logger::info(
                            LogTag::Positions,
                            &format!(
                                "Auto-blacklisted {} due to significant loss: -{:.3} SOL ({:.1}%)",
                                position.symbol, loss_sol, net_pnl_percent
                            ),
                        );
                    }
                    Err(e) => {
                        logger::warning(
                            LogTag::Positions,
                            &format!(
                                "Failed to blacklist {} after significant loss: {}",
                                position.symbol, e
                            ),
                        );
                    }
                }
            } else {
                logger::warning(
                    LogTag::Positions,
                    &format!(
                        "Failed to blacklist {} - database not initialized",
                        position.symbol
                    ),
                );
                return Err(Error::NotInitialised);
            }
        } else {
            logger::info(
                LogTag::Positions,
                &format!(
                    "Minor loss for {} not blacklisted: -{:.3} SOL ({:.1}%)",
                    position.symbol, loss_sol, net_pnl_percent
                ),
            );
        }
    } else if net_pnl_sol > 0.0 {
        logger::info(
            LogTag::Positions,
            &format!(
                "Profit recorded for {} ({}): +{:.3} SOL ({:.1}%)",
                position.symbol, &position.mint, net_pnl_sol, net_pnl_percent
            ),
        );
    }

    Ok(())
}

/// Get current loss detection threshold (for debugging/monitoring)
///
/// # Returns
/// * `percent_threshold` - Current percentage threshold from config
pub fn get_loss_thresholds() -> f64 {
    with_config(|cfg| cfg.positions.loss_blacklist_threshold_pct)
}

/// Check if loss-based blacklisting is currently enabled
///
/// # Returns
/// * `true` - Feature is enabled
/// * `false` - Feature is disabled
pub fn is_loss_blacklisting_enabled() -> bool {
    with_config(|cfg| cfg.positions.loss_blacklist_enabled)
}
