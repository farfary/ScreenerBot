// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Entry submission: per-cycle token reservation and the pipeline that turns a
//! `TradeDecision` into an executed (or failed) trade.
//!
//! Moved out of `monitors::entry` so a second entry source can reserve a mint and submit
//! a decision through the identical pipeline instead of a copy of it.

use crate::actions::ActionFailure;
use crate::i18n::ids;
use crate::logger::{self, LogTag};
use crate::positions::{Book, PositionManagement, PositionOrigin};
use crate::trader::executors::{self, ExecutionContext};
use crate::trader::types::{TradeDecision, TradeResult};
use crate::trader::{actions, constants};
use std::collections::HashMap;
use std::sync::LazyLock;
use tokio::sync::RwLock;
use tokio::time::{Duration, Instant};

/// Entry cycle reservations to prevent duplicate concurrent entries of one token on one
/// book. Each book reserves on its own, so a paper entry never holds or releases a live
/// entry's reservation. Expires after ENTRY_RESERVATION_TIMEOUT_SECS to handle cases where
/// entry fails.
static ENTRY_CYCLE_RESERVATIONS: LazyLock<RwLock<HashMap<(Book, String), Instant>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

/// Try to reserve a token for entry processing on `book` in this cycle
/// Returns true if reservation successful, false if already reserved
pub async fn try_reserve_entry(book: Book, mint: &str) -> bool {
    let mut reservations = ENTRY_CYCLE_RESERVATIONS.write().await;

    // Clean expired reservations
    reservations.retain(|_, instant| {
        instant.elapsed() < Duration::from_secs(constants::ENTRY_RESERVATION_TIMEOUT_SECS)
    });

    // Try to reserve
    let key = (book, mint.to_string());
    if reservations.contains_key(&key) {
        return false; // Already reserved by another thread
    }
    reservations.insert(key, Instant::now());
    true
}

/// Clear the reservation of a token on `book` (called after entry attempt completes)
pub async fn clear_entry_reservation(book: Book, mint: &str) {
    let mut reservations = ENTRY_CYCLE_RESERVATIONS.write().await;
    reservations.remove(&(book, mint.to_string()));
}

/// The severity an unsuccessful entry is recorded at: a live entry that failed is an
/// error, while a paper entry that did not fill is an expected outcome of the simulation.
fn entry_failure_severity(book: Book) -> crate::events::Severity {
    match book {
        Book::Live => crate::events::Severity::Error,
        Book::Paper => crate::events::Severity::Info,
    }
}

/// Everything between "we have a decision" and "the attempt is recorded": the entry-signal
/// event, the dashboard `AutoOpenAction` of a live entry, `executors::execute_trade` on the
/// decision's book, the success/failure events including the capacity-guard branch, and
/// clearing the mint reservation on every path. Shared by the strategy monitor and any
/// other entry source.
pub async fn submit_entry(decision: TradeDecision) -> Option<TradeResult> {
    let context = ExecutionContext {
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
    context: ExecutionContext,
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
            "book": decision.book.as_str(),
        }),
    )
    .await;

    // The Actions panel lists real swaps in flight: a paper entry sends none.
    let action = match decision.book {
        Book::Live => {
            let symbol = match crate::chains::chain_for_address(&decision.mint) {
                Ok(chain) => crate::tokens::get_full_token_async(chain, &decision.mint)
                    .await
                    .ok()
                    .flatten(),
                Err(_) => None,
            }
            .map(|t| t.symbol);
            actions::AutoOpenAction::new(
                &decision.mint,
                symbol.as_deref(),
                decision.strategy_id.as_deref(),
                &format!("{:?}", decision.reason),
                context.origin.kind(),
            )
            .await
            .ok()
        }
        Book::Paper => None,
    };

    // Mark evaluation complete (we got a decision)
    if let Some(ref a) = action {
        a.complete_evaluation().await;
        a.start_quote().await;
    }

    // Execute the trade
    let mint_for_cleanup = decision.mint.clone();
    let book = decision.book;
    let execution = executors::execute_trade(&decision, context).await;
    match execution {
        Ok(result) => {
            // Clear reservation after execution attempt
            clear_entry_reservation(book, &mint_for_cleanup).await;

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
                        "book": decision.book.as_str(),
                    }),
                )
                .await;
            } else {
                let error_msg = result.error.clone().unwrap_or_default();

                // Fail action
                if let Some(ref a) = action {
                    a.fail(ActionFailure::with_details(
                        ids::ACTIONS_FAILURE_ENTRY,
                        error_msg.as_str(),
                    ))
                    .await;
                }

                if let Some(remaining) = result.capacity_guard_remaining {
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
                            "book": decision.book.as_str(),
                        }),
                    )
                    .await;
                } else {
                    let message = format!("Entry failed for {}: {}", decision.mint, error_msg);
                    match book {
                        Book::Live => logger::error(LogTag::Trader, &message),
                        Book::Paper => logger::info(LogTag::Trader, &message),
                    }

                    crate::events::record_trader_event(
                        "entry_failed",
                        entry_failure_severity(book),
                        Some(&decision.mint),
                        None,
                        serde_json::json!({
                            "success": false,
                            "mint": decision.mint,
                            "error": result.error,
                            "book": decision.book.as_str(),
                        }),
                    )
                    .await;
                }
            }

            Some(result)
        }
        Err(e) => {
            // Clear reservation on error
            clear_entry_reservation(book, &mint_for_cleanup).await;

            // Fail action
            if let Some(ref a) = action {
                a.fail(ActionFailure::with_details(
                    ids::ACTIONS_FAILURE_ENTRY,
                    e.to_string(),
                ))
                .await;
            }

            let message = format!("Failed to execute entry for {}: {}", decision.mint, e);
            match book {
                Book::Live => logger::error(LogTag::Trader, &message),
                Book::Paper => logger::warning(LogTag::Trader, &message),
            }

            // Record execution error event
            crate::events::record_trader_event(
                "entry_execution_error",
                entry_failure_severity(book),
                Some(&decision.mint),
                None,
                serde_json::json!({
                    "mint": decision.mint,
                    "error": e.to_string(),
                    "book": decision.book.as_str(),
                }),
            )
            .await;

            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_paper_entry_that_does_not_fill_is_recorded_as_info_and_a_live_one_as_an_error() {
        assert_eq!(
            entry_failure_severity(Book::Paper),
            crate::events::Severity::Info
        );
        assert_eq!(
            entry_failure_severity(Book::Live),
            crate::events::Severity::Error
        );
    }

    #[tokio::test]
    async fn each_book_reserves_and_releases_a_mint_on_its_own() {
        let mint = "ReservationMint1111111111111111111111111111";
        for (holder, other) in [(Book::Paper, Book::Live), (Book::Live, Book::Paper)] {
            assert!(try_reserve_entry(holder, mint).await);
            assert!(
                try_reserve_entry(other, mint).await,
                "a {holder:?} reservation blocked a {other:?} entry"
            );
            clear_entry_reservation(holder, mint).await;
            assert!(
                !try_reserve_entry(other, mint).await,
                "releasing the {holder:?} reservation released the {other:?} one"
            );
            assert!(try_reserve_entry(holder, mint).await);
            clear_entry_reservation(holder, mint).await;
            clear_entry_reservation(other, mint).await;
        }
    }
}
