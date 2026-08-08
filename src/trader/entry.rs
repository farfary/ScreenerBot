//! Entry submission: per-cycle token reservation and the pipeline that turns a
//! `TradeDecision` into an executed (or failed) trade.
//!
//! Moved out of `monitors::entry` so a second entry source can reserve a mint and submit
//! a decision through the identical pipeline instead of a copy of it.

use crate::logger::{self, LogTag};
use crate::positions;
use crate::positions::{PositionManagement, PositionOrigin};
use crate::trader::types::{TradeDecision, TradeResult};
use crate::trader::{actions, constants, executors};
use std::collections::HashMap;
use std::sync::LazyLock;
use tokio::sync::RwLock;
use tokio::time::{Duration, Instant};

/// Entry cycle reservations to prevent duplicate concurrent entries for same token
/// Expires after ENTRY_RESERVATION_TIMEOUT_SECS to handle cases where entry fails
static ENTRY_CYCLE_RESERVATIONS: LazyLock<RwLock<HashMap<String, Instant>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntryContext {
    pub origin: PositionOrigin,
    pub management: PositionManagement,
}

/// Try to reserve a token for entry processing in this cycle
/// Returns true if reservation successful, false if already reserved
pub async fn try_reserve_entry(mint: &str) -> bool {
    let mut reservations = ENTRY_CYCLE_RESERVATIONS.write().await;

    // Clean expired reservations
    reservations.retain(|_, instant| {
        instant.elapsed() < Duration::from_secs(constants::ENTRY_RESERVATION_TIMEOUT_SECS)
    });

    // Try to reserve
    if reservations.contains_key(mint) {
        return false; // Already reserved by another thread
    }
    reservations.insert(mint.to_string(), Instant::now());
    true
}

/// Clear reservation for a token (called after entry attempt completes)
pub async fn clear_entry_reservation(mint: &str) {
    let mut reservations = ENTRY_CYCLE_RESERVATIONS.write().await;
    reservations.remove(mint);
}

/// Everything between "we have a decision" and "the attempt is recorded": the entry-signal
/// event, the dashboard `AutoOpenAction`, `executors::execute_trade`, the success/failure
/// events including the capacity-guard branch, and clearing the mint reservation on every
/// path. Shared by the strategy monitor and any other entry source.
pub async fn submit_entry(decision: TradeDecision) -> Option<TradeResult> {
    let context = EntryContext {
        origin: PositionOrigin::Auto {
            strategy_id: decision.strategy_id.clone(),
        },
        management: PositionManagement::AutoTrader,
    };
    submit_entry_with_context(decision, context).await
}

/// The shared submission lifecycle with explicit durable provenance. Copy entries use
/// this rather than teaching `TradeReason` to impersonate position ownership.
pub async fn submit_entry_with_context(
    decision: TradeDecision,
    context: EntryContext,
) -> Option<TradeResult> {
    // Record entry signal event
    crate::events::record_trader_event(
        "entry_signal_generated",
        crate::events::Severity::Info,
        Some(&decision.mint),
        None,
        serde_json::json!({
            "action": "entry_signal",
            "mint": decision.mint,
            "strategy_id": decision.strategy_id,
            "reason": format!("{:?}", decision.reason),
            "priority": format!("{:?}", decision.priority),
        }),
    )
    .await;

    // Create action for dashboard visibility
    let symbol = crate::tokens::get_full_token_async(&decision.mint)
        .await
        .ok()
        .flatten()
        .map(|t| t.symbol);

    let action = actions::AutoOpenAction::new(
        &decision.mint,
        symbol.as_deref(),
        decision.strategy_id.as_deref(),
        &format!("{:?}", decision.reason),
    )
    .await
    .ok();

    // Mark evaluation complete (we got a decision)
    if let Some(ref a) = action {
        a.complete_evaluation().await;
        a.start_quote().await;
    }

    // Execute the trade
    let mint_for_cleanup = decision.mint.clone();
    let execution = if matches!(decision.action, crate::trader::types::TradeAction::Buy) {
        executors::execute_buy_managed(&decision, context.origin, context.management).await
    } else {
        executors::execute_trade(&decision).await
    };
    match execution {
        Ok(result) => {
            // Clear reservation after execution attempt
            clear_entry_reservation(&mint_for_cleanup).await;

            if result.success {
                let tx_sig = result.tx_signature.clone();

                // Complete action
                if let Some(ref a) = action {
                    a.complete_quote().await;
                    a.start_swap().await;
                    a.complete_swap(tx_sig.as_deref().unwrap_or("unknown"))
                        .await;
                    a.complete(tx_sig.as_deref()).await;
                }

                logger::info(
                    LogTag::Trader,
                    &format!(
                        "Entry executed for {}: tx={}",
                        decision.mint,
                        tx_sig.clone().unwrap_or_default()
                    ),
                );

                // Record successful entry event
                crate::events::record_trader_event(
                    "entry_executed",
                    crate::events::Severity::Info,
                    Some(&decision.mint),
                    tx_sig.as_deref(),
                    serde_json::json!({
                        "success": true,
                        "mint": decision.mint,
                        "tx_signature": tx_sig,
                    }),
                )
                .await;
            } else {
                let error_msg = result.error.clone().unwrap_or_default();

                // Fail action
                if let Some(ref a) = action {
                    a.fail(&error_msg).await;
                }

                if let Some(remaining) = positions::parse_position_slot_error(&error_msg) {
                    logger::info(
                        LogTag::Trader,
                        &format!(
                            "Entry blocked for {} – capacity guard engaged (permits left: {})",
                            decision.mint, remaining
                        ),
                    );

                    crate::events::record_trader_event(
                        "entry_capacity_guard",
                        crate::events::Severity::Info,
                        Some(&decision.mint),
                        None,
                        serde_json::json!({
                            "mint": decision.mint,
                            "reason": error_msg,
                            "remaining_permits": remaining,
                        }),
                    )
                    .await;
                } else {
                    logger::error(
                        LogTag::Trader,
                        &format!("Entry failed for {}: {}", decision.mint, error_msg),
                    );

                    crate::events::record_trader_event(
                        "entry_failed",
                        crate::events::Severity::Error,
                        Some(&decision.mint),
                        None,
                        serde_json::json!({
                            "success": false,
                            "mint": decision.mint,
                            "error": result.error,
                        }),
                    )
                    .await;
                }
            }

            Some(result)
        }
        Err(e) => {
            // Clear reservation on error
            clear_entry_reservation(&mint_for_cleanup).await;

            // Fail action
            if let Some(ref a) = action {
                a.fail(&e).await;
            }

            logger::error(
                LogTag::Trader,
                &format!("Failed to execute entry for {}: {}", decision.mint, e),
            );

            // Record execution error event
            crate::events::record_trader_event(
                "entry_execution_error",
                crate::events::Severity::Error,
                Some(&decision.mint),
                None,
                serde_json::json!({
                    "mint": decision.mint,
                    "error": e.to_string(),
                }),
            )
            .await;

            None
        }
    }
}
