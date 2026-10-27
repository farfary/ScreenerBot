// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Release of swap holds: what the settled holding proves about each held swap, what its position books before the hold leaves, and the event it leaves behind.

use chrono::{DateTime, Utc};

use crate::chains::{RawAmount, SettlementReader};
use crate::logger::{self, LogTag};

use super::{
    assign_expiry_bound, persist_swap_holds, take_hold, HeldSwap, SwapHold, SWAP_HOLDS,
    UNPROVEN_HOLD_LIMIT,
};

/// What the chain proves about a held swap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HoldOutcome {
    /// No settled reading yet, and the limit has not been reached.
    Wait,
    /// The settled holding equals the holding before the swap: the swap did not land.
    NotLanded,
    /// The settled holding moved by exactly what the held swap was sent to do: it landed.
    Landed,
    /// Any other settled holding, or no settled reading within the limit.
    Unknown,
}

/// A hold the settlement pass released, with the outcome that released it.
#[derive(Debug, Clone, PartialEq)]
pub struct ReleasedHold {
    pub hold: SwapHold,
    pub outcome: HoldOutcome,
}

impl HeldSwap {
    /// True when the holding moving from `before` to `after` is exactly what this swap was
    /// sent to do.
    fn landed(&self, before: RawAmount, after: RawAmount) -> bool {
        match self {
            HeldSwap::PartialExit { exit_amount, .. } => {
                before.checked_sub(after) == Some(*exit_amount)
            }
        }
    }
}

/// The outcome of `hold` given the `settled` holding read once no transaction sent before
/// its bound can still land, at `now`. Only an exact raw equality proves the swap did not
/// land, and only the exact movement the swap was sent to make proves it landed.
pub fn hold_outcome(
    hold: &SwapHold,
    settled: Option<RawAmount>,
    now: DateTime<Utc>,
) -> HoldOutcome {
    match settled {
        None => {
            let limit_reached = now
                .signed_duration_since(hold.created_at)
                .to_std()
                .is_ok_and(|held_for| held_for >= UNPROVEN_HOLD_LIMIT);
            if limit_reached {
                HoldOutcome::Unknown
            } else {
                HoldOutcome::Wait
            }
        }
        Some(after) if after == hold.holding_before => HoldOutcome::NotLanded,
        Some(after) if hold.swap.landed(hold.holding_before, after) => HoldOutcome::Landed,
        Some(_) => HoldOutcome::Unknown,
    }
}

/// Settles the holds through the settlement reader of the positions store's chain and the
/// trading wallet. Reads nothing when no swap is held.
pub async fn settle_swap_holds() {
    if SWAP_HOLDS.read().await.is_empty() {
        return;
    }
    let reader = match crate::positions::settle::reader().await {
        Ok(reader) => reader,
        Err(error) => {
            logger::warning(
                LogTag::Positions,
                &format!("Swap holds cannot be settled: no settlement reader: {error}"),
            );
            return;
        }
    };
    let wallet = match crate::utils::get_wallet_address() {
        Ok(wallet) => wallet,
        Err(error) => {
            logger::warning(
                LogTag::Positions,
                &format!("Swap holds cannot be settled: no trading wallet: {error}"),
            );
            return;
        }
    };
    settle_swap_holds_on(reader.as_ref(), &wallet, Utc::now()).await;
}

/// Settles each hold from `reader`'s reading of `wallet` at `now` and returns the holds it
/// released. A hold without a bound is given one first; a later bound is a safe upper bound.
/// A hold with no outcome yet, or whose mint's position lock is not free within the
/// settlement wait, is left for the next pass.
///
/// Under the lock, the position first books what the outcome requires, while the hold still
/// blocks the mint, and the hold is then removed; a hold whose booking failed stays for the
/// next pass. A hold whose swap landed or was released without proof is followed by the
/// wallet-history resync, which books whatever the wallet shows. The outcome is recorded
/// only when the removal returned the hold, so each hold leaves exactly once.
pub async fn settle_swap_holds_on(
    reader: &dyn SettlementReader,
    wallet: &str,
    now: DateTime<Utc>,
) -> Vec<ReleasedHold> {
    let holds = SWAP_HOLDS.read().await.clone();
    let mut released = Vec::new();
    for mut hold in holds {
        if hold.expiry_height.is_none() {
            match reader.expiry_bound().await {
                Ok(bound) => {
                    assign_expiry_bound(&hold, bound).await;
                    hold.expiry_height = Some(bound);
                }
                Err(error) => logger::debug(
                    LogTag::Positions,
                    &format!(
                        "Swap hold of position {} has no expiry bound yet: {error}",
                        hold.position_id
                    ),
                ),
            }
        }
        let settled = match hold.expiry_height {
            Some(bound) => reader
                .settled_holding(wallet, &hold.mint, bound)
                .await
                .ok()
                .flatten()
                .map(|holding| holding.amount),
            None => None,
        };
        let outcome = hold_outcome(&hold, settled, now);
        if outcome == HoldOutcome::Wait {
            continue;
        }
        let Some(_lock) = crate::positions::worker::settlement_lock(&hold.mint).await else {
            continue;
        };
        if !settle_position(&hold, outcome).await {
            continue;
        }
        let Some(removed) = take_hold(&hold).await else {
            continue;
        };
        if let Err(error) = persist_swap_holds().await {
            logger::error(
                LogTag::Positions,
                &format!(
                    "Released swap hold of position {} (mint {}) is still stored: {error}",
                    removed.position_id, removed.mint
                ),
            );
        }
        if outcome != HoldOutcome::NotLanded {
            crate::positions::ledger::schedule_resync();
        }
        record_outcome(&removed, outcome, settled).await;
        released.push(ReleasedHold {
            hold: removed,
            outcome,
        });
    }
    released
}

/// What the position books for a hold leaving with `outcome`, under the mint's lock and
/// while the hold still blocks the mint; `false` keeps the hold for the next pass.
///
/// A partial exit proven not sold returns its take-profit step to the ladder of its open
/// row; every other partial-exit outcome books nothing here.
async fn settle_position(hold: &SwapHold, outcome: HoldOutcome) -> bool {
    match (&hold.swap, outcome) {
        (
            HeldSwap::PartialExit {
                take_profit_level, ..
            },
            HoldOutcome::NotLanded,
        ) => {
            if let Some(level) = *take_profit_level {
                if position_is_open(hold.position_id).await {
                    crate::positions::operations::release_take_profit_step(
                        &hold.mint,
                        hold.position_id,
                        level,
                    )
                    .await;
                }
            }
            true
        }
        (
            HeldSwap::PartialExit { .. },
            HoldOutcome::Landed | HoldOutcome::Unknown | HoldOutcome::Wait,
        ) => true,
    }
}

/// Records the event and the log line of a released hold.
async fn record_outcome(hold: &SwapHold, outcome: HoldOutcome, settled: Option<RawAmount>) {
    let (event, reason) = match (&hold.swap, outcome) {
        (HeldSwap::PartialExit { .. }, HoldOutcome::NotLanded) => (
            "partial_exit_hold_not_sold",
            "the settled holding equals the holding before the swap",
        ),
        (HeldSwap::PartialExit { .. }, HoldOutcome::Landed) => (
            "partial_exit_hold_sold",
            "the settled holding is lower by exactly the exit amount",
        ),
        (HeldSwap::PartialExit { .. }, HoldOutcome::Unknown) if settled.is_some() => (
            "partial_exit_hold_released",
            "the settled holding moved by another amount",
        ),
        (HeldSwap::PartialExit { .. }, HoldOutcome::Unknown | HoldOutcome::Wait) => (
            "partial_exit_hold_released",
            "no settled holding could be read within the hold limit",
        ),
    };
    let entry_signature = crate::positions::state::get_position_by_id(hold.position_id)
        .await
        .and_then(|position| position.entry_transaction_signature);
    let (token_amount, exit_percentage, swap_label, swap_detail) = match &hold.swap {
        HeldSwap::PartialExit {
            exit_amount,
            requested_exit_percentage,
            ..
        } => (
            *exit_amount,
            Some(*requested_exit_percentage),
            "Partial exit",
            format!("exit={exit_amount}"),
        ),
    };
    crate::events::record_position_event(
        &hold.position_id.to_string(),
        &hold.mint,
        event,
        entry_signature.as_deref(),
        None,
        0.0,
        token_amount,
        None,
        exit_percentage,
    )
    .await;
    let message = format!(
        "{swap_label} hold of position {} (mint {}) released as {outcome:?}: {reason} (before={} after={} {swap_detail})",
        hold.position_id,
        hold.mint,
        hold.holding_before,
        settled.map_or_else(|| "unread".to_owned(), |after| after.to_string()),
    );
    if outcome == HoldOutcome::Unknown {
        logger::warning(LogTag::Positions, &message);
    } else {
        logger::info(LogTag::Positions, &message);
    }
}

/// True when the position is in memory and not closed.
async fn position_is_open(position_id: i64) -> bool {
    crate::positions::state::get_position_by_id(position_id)
        .await
        .is_some_and(|position| !crate::positions::round_state::is_closed(&position))
}
