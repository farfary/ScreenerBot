use super::{lib::calculate_position_pnl, types::Position};
use crate::logger::{self, LogTag};
use crate::tokens::cleanup;
use crate::tokens::database::get_global_database;

// =============================================================================
// LOSS DETECTION CONFIGURATION
// =============================================================================

/// Enable automatic blacklisting of tokens that result in losses
/// When enabled, tokens that cause losses above configured thresholds will be automatically
/// added to the blacklist to prevent future trading
pub const ENABLE_LOSS_BASED_BLACKLISTING: bool = true;

/// Minimum percentage loss threshold for blacklisting
const MIN_LOSS_PERCENT_THRESHOLD: f64 = -15.0;

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
pub async fn process_position_loss_detection(position: &Position) -> Result<(), String> {
    // Check if loss-based blacklisting is enabled
    if !ENABLE_LOSS_BASED_BLACKLISTING {
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
                "💸 Loss detected for {} ({}): -{:.3} SOL ({:.1}%)",
                position.symbol, &position.mint, loss_sol, net_pnl_percent
            ),
        );

        // Only blacklist for significant losses to avoid being too aggressive
        if should_blacklist_for_loss(net_pnl_percent) {
            // Add to database-backed blacklist
            if let Some(db) = get_global_database() {
                match cleanup::blacklist_token(&position.mint, "PoorPerformance", &db) {
                    Ok(_) => {
                        logger::info(
                            LogTag::Positions,
                            &format!(
                                "🚫 Auto-blacklisted {} due to significant loss: -{:.3} SOL ({:.1}%)",
                                position.symbol, loss_sol, net_pnl_percent
                            ),
                        );
                    }
                    Err(e) => {
                        logger::warning(
                            LogTag::Positions,
                            &format!(
                                "⚠️ Failed to blacklist {} after significant loss: {}",
                                position.symbol, e
                            ),
                        );
                    }
                }
            } else {
                logger::warning(
                    LogTag::Positions,
                    &format!(
                        "⚠️ Failed to blacklist {} - database not initialized",
                        position.symbol
                    ),
                );
                return Err(format!(
                    "Failed to blacklist token {} after loss",
                    position.symbol
                ));
            }
        } else {
            logger::info(
                LogTag::Positions,
                &format!(
                    "📊 Minor loss for {} not blacklisted: -{:.3} SOL ({:.1}%)",
                    position.symbol, loss_sol, net_pnl_percent
                ),
            );
        }
    } else if net_pnl_sol > 0.0 {
        logger::info(
            LogTag::Positions,
            &format!(
                "💰 Profit recorded for {} ({}): +{:.3} SOL ({:.1}%)",
                position.symbol, &position.mint, net_pnl_sol, net_pnl_percent
            ),
        );
    }

    Ok(())
}

/// Determine if a position should be blacklisted based on loss threshold
///
/// # Arguments
/// * `loss_percent` - Loss percentage (negative value)
///
/// # Returns
/// * `true` - Should be blacklisted
/// * `false` - Should not be blacklisted
fn should_blacklist_for_loss(loss_percent: f64) -> bool {
    loss_percent <= MIN_LOSS_PERCENT_THRESHOLD
}

/// Get current loss detection threshold (for debugging/monitoring)
///
/// # Returns
/// * `percent_threshold` - Current percentage threshold
pub fn get_loss_thresholds() -> f64 {
    MIN_LOSS_PERCENT_THRESHOLD
}

/// Check if loss-based blacklisting is currently enabled
///
/// # Returns
/// * `true` - Feature is enabled
/// * `false` - Feature is disabled
pub fn is_loss_blacklisting_enabled() -> bool {
    ENABLE_LOSS_BASED_BLACKLISTING
}
