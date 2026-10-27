// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Position lifecycle operations.
//!
//! High-level commands for opening, closing, partial-closing, and DCA-ing into
//! positions. Each swap operation acquires the position lock, executes the swap via
//! the configured router, and enqueues transaction verification. A force close books
//! an operator write-off without a swap. Position persistence with retry is also
//! handled here.

mod close;
mod dca;
mod exit_ladder;
mod exit_sizing;
mod force_close;
mod open;
mod partial_close;
mod slippage;

pub use close::{close_position_direct, mark_exit_submitted};
pub use dca::add_to_position;
pub(crate) use dca::check_auto_add;
pub(crate) use exit_sizing::{read_exit_sale, ExitSale, ExitShare};
pub use force_close::{force_close_position, ForceClosed};
pub use open::{open_position_direct, open_position_with_size};
pub use partial_close::partial_close_position;
pub(crate) use partial_close::release_take_profit_step;
pub(crate) use slippage::{entry_slippage, exit_slippage_ladder};

use crate::database::catalog::POSITIONS;
use crate::database::health::{self, StoreStatus};
use crate::logger::{self, LogTag};
use crate::positions::db::save_position;
use crate::positions::types::Position;
use crate::positions::{Error, Result};
use serde_json::json;
use tokio::time::{sleep, Duration};

/// Refuse `transition` of `mint` while the positions store holds restored or recreated
/// contents not yet reconciled with the wallet history. Only changes sized from the
/// stored position call this (an add, a partial exit, the full close of a quote asset); any
/// other full close sizes from the on-chain balance and stays available.
pub(crate) fn refuse_until_reconciled(transition: &'static str, mint: &str) -> Result<()> {
    reconciliation_refusal(positions_status().as_ref(), transition, mint)
}

/// Whether the positions store holds restored or recreated contents not yet reconciled,
/// so [`refuse_until_reconciled`] would refuse. The auto-trader asks this before it
/// proposes an add or a partial exit, instead of attempting one per tick.
pub fn awaiting_reconciliation() -> bool {
    reconciliation_refusal(positions_status().as_ref(), "", "").is_err()
}

fn positions_status() -> Option<StoreStatus> {
    POSITIONS
        .paths()
        .iter()
        .find_map(|path| health::status(path))
}

fn reconciliation_refusal(
    status: Option<&StoreStatus>,
    transition: &'static str,
    mint: &str,
) -> Result<()> {
    match status {
        Some(status) if status.health.is_replacement() && !status.reconciled => {
            Err(Error::StoreNotReconciled {
                transition,
                mint: mint.to_owned(),
                state: status.health.as_str(),
            })
        }
        _ => Ok(()),
    }
}

/// Wait before retrying the next slippage rung of an exit.
///
/// Being throttled needs a much longer pause than an ordinary failure, and the
/// error itself is the only reliable place to learn which happened — this asks
/// [`ErrorClass::is_rate_limited`] instead of searching the rendered message
/// for "429", which stops matching whenever a provider rewords its response.
/// `Retry-After` is honoured when the provider sent one.
pub(crate) async fn backoff_after<E: crate::errors::ErrorClass>(error: &E) {
    if error.is_rate_limited() {
        let wait = error.retry_after().unwrap_or(RATE_LIMIT_BACKOFF).max(
            // Never pause less than the ordinary rung wait just because a
            // provider asked for a token gesture of a delay.
            SLIPPAGE_RUNG_BACKOFF,
        );
        logger::warning(
            LogTag::Positions,
            &format!(
                "Router rate limit hit, backing off {:.0}s before retry",
                wait.as_secs_f64()
            ),
        );
        sleep(wait).await;
    } else {
        sleep(SLIPPAGE_RUNG_BACKOFF).await;
    }
}

/// Pause between slippage rungs after an ordinary failure.
const SLIPPAGE_RUNG_BACKOFF: Duration = Duration::from_secs(2);

/// Pause after a rate-limited attempt when the provider named no `Retry-After`.
const RATE_LIMIT_BACKOFF: Duration = Duration::from_secs(10);

const POSITION_SAVE_MAX_RETRIES: usize = 5;
const POSITION_SAVE_BASE_BACKOFF_MS: u64 = 200;
const POSITION_SAVE_MAX_BACKOFF_MS: u64 = 3_000;

// =============================================================================
// PERSISTENCE HELPERS
// =============================================================================

fn position_save_backoff_ms(attempt: usize) -> u64 {
    let exp = attempt.saturating_sub(1) as u32;
    let growth = 1_u64 << exp.min(12);
    (POSITION_SAVE_BASE_BACKOFF_MS * growth).min(POSITION_SAVE_MAX_BACKOFF_MS)
}

/// Where a new open position was persisted.
enum Persisted {
    /// Stored as a new row with this id.
    Opened(i64),
    /// Refused because the mint already has the open position with this id, read in the
    /// refusing transaction: the swap that bought it is that position's add, never a second
    /// row.
    Added(i64),
    /// Not stored: every attempt failed. The swap was sent all the same, so its tokens are
    /// in the wallet's history, and the wallet-history sync books them as the mint's round
    /// once the swap no longer marks the mint busy.
    Unsaved,
}

/// Persists a new open position, retrying transient failures. Runs after the entry swap
/// was sent, so it never fails the open: storage that refuses every attempt is reported as
/// [`Persisted::Unsaved`].
async fn persist_position_with_retry(position: &Position) -> Persisted {
    let mut attempt = 1;
    loop {
        match save_position(position).await {
            Err(Error::AlreadyOpen {
                open_position_id, ..
            }) => return Persisted::Added(open_position_id),
            Ok(id) => {
                if attempt > 1 {
                    logger::info(
                        LogTag::Positions,
                        &format!(
                            "Recovered after {} persistence retries for mint {} (ID {})",
                            attempt - 1,
                            position.mint,
                            id
                        ),
                    );
                }
                return Persisted::Opened(id);
            }
            Err(err) => {
                logger::warning(
                    LogTag::Positions,
                    &format!(
                        "Persisting position for mint {} failed on attempt {}: {}",
                        position.mint, attempt, err
                    ),
                );

                if attempt >= POSITION_SAVE_MAX_RETRIES {
                    crate::events::record_position_event_flexible(
                        "open_persist_failed",
                        crate::events::Severity::Error,
                        Some(&position.mint),
                        None,
                        json!({
                          "mint": position.mint,
                          "attempts": attempt,
                          "error": err.to_string(),
                        }),
                    )
                    .await;

                    logger::error(
                        LogTag::Positions,
                        &format!(
                            "Position for mint {} was not persisted after {} attempts; its entry swap {} is left to the wallet-history sync",
                            position.mint,
                            attempt,
                            position.entry_transaction_signature.as_deref().unwrap_or("-")
                        ),
                    );
                    return Persisted::Unsaved;
                }

                let backoff = position_save_backoff_ms(attempt);
                sleep(Duration::from_millis(backoff)).await;
                attempt += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{add_to_position, partial_close_position, reconciliation_refusal};
    use crate::database::catalog::POSITIONS;
    use crate::database::health::{self, StoreHealth, StoreStatus};
    use crate::positions::types::TradeOrigin;
    use crate::positions::Error;

    fn replaced() -> [StoreHealth; 2] {
        let quarantine = PathBuf::from("/operations-test/quarantine");
        [
            StoreHealth::Restored {
                snapshot_at: None,
                quarantine: quarantine.clone(),
            },
            StoreHealth::Recovered { quarantine },
        ]
    }

    fn status(health: StoreHealth, reconciled: bool) -> StoreStatus {
        StoreStatus {
            store: "positions",
            path: PathBuf::from("/operations-test/positions.db"),
            health,
            changed_at: chrono::Utc::now(),
            reconciled,
        }
    }

    #[test]
    fn replaced_positions_refuse_changes_until_reconciled() {
        for health in replaced() {
            let state = health.as_str();
            let waiting = status(health.clone(), false);
            match reconciliation_refusal(Some(&waiting), "dca", "mint") {
                Err(Error::StoreNotReconciled {
                    transition: "dca",
                    state: refused,
                    ..
                }) => assert_eq!(refused, state),
                other => panic!("expected a refusal, got {other:?}"),
            }
            let reconciled = status(health, true);
            assert!(reconciliation_refusal(Some(&reconciled), "dca", "mint").is_ok());
        }
        let healthy = status(StoreHealth::Healthy, true);
        assert!(reconciliation_refusal(Some(&healthy), "partial_exit", "mint").is_ok());
        assert!(reconciliation_refusal(None, "partial_exit", "mint").is_ok());
    }

    #[tokio::test]
    async fn an_add_and_a_partial_exit_wait_for_a_restored_store() {
        let [restored, _] = replaced();
        let path = POSITIONS.paths().remove(0);
        health::publish(&POSITIONS, &path, restored);

        let add = add_to_position("mint", 0.1, None, TradeOrigin::Manual).await;
        assert!(
            matches!(
                add,
                Err(Error::StoreNotReconciled {
                    transition: "dca",
                    ..
                })
            ),
            "{add:?}"
        );
        let partial = partial_close_position("mint", 50.0, "manual_exit", None, None).await;
        assert!(
            matches!(
                partial,
                Err(Error::StoreNotReconciled {
                    transition: "partial_exit",
                    ..
                })
            ),
            "{partial:?}"
        );

        assert!(super::awaiting_reconciliation());
        health::mark_reconciled(&path);
        assert!(!super::awaiting_reconciliation());
        let add = add_to_position("mint", 0.1, None, TradeOrigin::Manual).await;
        assert!(matches!(add, Err(Error::NotFound { .. })), "{add:?}");
    }

    /// Only operations sized from the stored position wait for reconciliation; a full
    /// close sizes from the on-chain balance (a quote asset's from its booked remainder,
    /// which waits), and a force close books a write-off, so both stay available after a
    /// restore.
    #[test]
    fn only_position_sized_operations_wait_for_reconciliation() {
        let gate = "refuse_until_reconciled(";
        assert!(include_str!("dca.rs").contains(gate));
        assert!(include_str!("partial_close.rs").contains(gate));
        let close = include_str!("close.rs");
        assert_eq!(close.matches(gate).count(), 1, "close.rs gates once");
        let quote_branch = close
            .find("if quote_asset {")
            .expect("close.rs branches on the quote asset");
        let gated = close.find(gate).unwrap();
        assert!(
            gated > quote_branch && !close[quote_branch..gated].contains('}'),
            "close.rs waits for reconciliation only for a quote asset"
        );
        for (file, source) in [
            ("force_close.rs", include_str!("force_close.rs")),
            ("open.rs", include_str!("open.rs")),
            ("exit_ladder.rs", include_str!("exit_ladder.rs")),
        ] {
            assert!(
                !source.contains(gate),
                "{file} must not wait for reconciliation"
            );
        }
    }

    /// Once the entry's pending-open mark is kept, nothing before the swap may return
    /// early: the mark would outlive an entry that sent nothing and block the mint for
    /// the whole pending-open window.
    #[test]
    fn no_fallible_step_separates_the_kept_pending_open_from_the_swap() {
        let source = include_str!("open.rs");
        let kept = source
            .find("pending_open.keep();")
            .expect("the open path keeps its pending-open mark");
        let swap = kept
            + source[kept..]
                .find("execute_swap_with_fallback(")
                .expect("the swap follows the kept mark");
        let between = &source[kept..swap];
        assert!(
            !between.contains(".await?") && !between.contains("return Err"),
            "a fallible step between keep() and the swap leaves the mark held:\n{between}"
        );
    }
}
