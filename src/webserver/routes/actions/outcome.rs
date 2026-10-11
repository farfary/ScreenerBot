// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The settled result of an action's swap: the position entry or exit record booked under
//! the swap's signature, joined onto each listed action.

use std::collections::{HashMap, HashSet};

use serde::Serialize;

use chrono::{DateTime, Duration, Utc};

use crate::actions::{Action, ActionState};
use crate::chains::{adapter_for, ChainId};
use crate::logger::{self, LogTag};
use crate::positions::{self, EntryRecord, ExitRecord};

use super::types::ActionHistoryRow;

/// Which record the swap was booked as.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OutcomeKind {
    Entry,
    Dca,
    Exit,
    PartialExit,
}

/// What a verified swap did, as the positions system booked it.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ActionOutcome {
    pub kind: OutcomeKind,
    pub signature: String,
    pub position_id: i64,
    /// Tokens bought or sold in whole units; absent while the token's decimals are unknown.
    pub token_amount: Option<f64>,
    /// SOL spent on an entry or received from an exit.
    pub native_amount: f64,
    /// Execution price in SOL per whole token.
    pub price: f64,
    /// Network fee of the transaction.
    pub fees_native: Option<f64>,
    /// Exits only: the share of the position this swap sold.
    pub exit_percentage: Option<f64>,
    /// Full exits only: the realized P&L of the position round, as its row stores it.
    pub realized_pnl: Option<f64>,
    pub realized_pnl_percent: Option<f64>,
}

/// The realized P&L a closed position row can stand behind.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct RoundPnl {
    pub pnl: f64,
    pub pnl_percent: Option<f64>,
}

/// `chain` is the chain of the positions store the record was read from.
pub(super) fn entry_outcome(
    record: &EntryRecord,
    chain: ChainId,
    decimals: Option<u8>,
) -> ActionOutcome {
    ActionOutcome {
        kind: if record.is_dca {
            OutcomeKind::Dca
        } else {
            OutcomeKind::Entry
        },
        signature: record.transaction_signature.clone(),
        position_id: record.position_id,
        token_amount: decimals.map(|decimals| record.amount.to_whole_units(decimals)),
        native_amount: record.native_spent,
        price: record.price,
        fees_native: record
            .fees_raw
            .map(|raw| adapter_for(chain).raw_to_native(raw)),
        exit_percentage: None,
        realized_pnl: None,
        realized_pnl_percent: None,
    }
}

/// `chain` is the chain of the positions store the record was read from. `round` is the
/// position's stored P&L; it describes this swap only when the swap closed
/// the position, so a partial exit never carries it.
pub(super) fn exit_outcome(
    record: &ExitRecord,
    chain: ChainId,
    decimals: Option<u8>,
    round: Option<RoundPnl>,
) -> ActionOutcome {
    let round = round.filter(|_| !record.is_partial);
    ActionOutcome {
        kind: if record.is_partial {
            OutcomeKind::PartialExit
        } else {
            OutcomeKind::Exit
        },
        signature: record.transaction_signature.clone(),
        position_id: record.position_id,
        token_amount: decimals.map(|decimals| record.amount.to_whole_units(decimals)),
        native_amount: record.native_received,
        price: record.price,
        fees_native: record
            .fees_raw
            .map(|raw| adapter_for(chain).raw_to_native(raw)),
        exit_percentage: Some(record.percentage),
        realized_pnl: round.map(|round| round.pnl),
        realized_pnl_percent: round.and_then(|round| round.pnl_percent),
    }
}

/// Pair every action with the record booked under its swap's signature. Reads are
/// storage-only: a swap that is not booked yet, or a failed read, leaves the outcome empty.
pub(super) async fn with_outcomes(actions: Vec<Action>) -> Vec<ActionHistoryRow> {
    let signatures: Vec<String> = actions
        .iter()
        .filter_map(|action| action.swap_signature().map(str::to_owned))
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();

    let without_outcomes = |actions: Vec<Action>| -> Vec<ActionHistoryRow> {
        actions
            .into_iter()
            .map(|action| ActionHistoryRow {
                action,
                outcome: None,
                unrecorded: false,
            })
            .collect()
    };
    if signatures.is_empty() {
        return without_outcomes(actions);
    }
    let read = async {
        let chain = positions::db::get_store_chain().await?;
        let records = positions::get_swap_records_by_signatures(&signatures).await?;
        Ok::<_, positions::Error>((chain, records))
    };
    let (chain, (entries, exits)) = match read.await {
        Ok(read) => read,
        Err(err) => {
            logger::debug(
                LogTag::Webserver,
                &format!("Failed to load swap records for actions: {err}"),
            );
            return without_outcomes(actions);
        }
    };

    let entries: HashMap<&str, &EntryRecord> = entries
        .iter()
        .map(|record| (record.transaction_signature.as_str(), record))
        .collect();
    let exits: HashMap<&str, &ExitRecord> = exits
        .iter()
        .map(|record| (record.transaction_signature.as_str(), record))
        .collect();

    let booked = |action: &Action| {
        action.swap_signature().is_some_and(|signature| {
            entries.contains_key(signature) || exits.contains_key(signature)
        })
    };
    let mints: Vec<String> = actions
        .iter()
        .filter(|action| booked(action))
        .map(|action| action.entity_id.clone())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    let decimals = load_decimals(chain, mints).await;
    let closed: HashSet<i64> = exits
        .values()
        .filter(|record| !record.is_partial)
        .map(|record| record.position_id)
        .collect();
    let rounds = load_round_pnl(closed).await;

    actions
        .into_iter()
        .map(|action| {
            let token_decimals = decimals.get(&action.entity_id).copied();
            let outcome = action.swap_signature().and_then(|signature| {
                if let Some(record) = exits.get(signature) {
                    let round = rounds.get(&record.position_id).copied();
                    Some(exit_outcome(record, chain, token_decimals, round))
                } else {
                    entries
                        .get(signature)
                        .map(|record| entry_outcome(record, chain, token_decimals))
                }
            });
            let unrecorded = outcome.is_none() && is_unrecorded(&action, Utc::now());
            ActionHistoryRow {
                action,
                outcome,
                unrecorded,
            }
        })
        .collect()
}

/// A completed action whose swap no position booked, once the positions verifier can no
/// longer book it: it keeps verifying a swap for `MAX_VERIFICATION_AGE_HOURS`, so before
/// that a missing record may only be late. The caller has already found no record.
pub(super) fn is_unrecorded(action: &Action, now: DateTime<Utc>) -> bool {
    let window = Duration::hours(positions::MAX_VERIFICATION_AGE_HOURS);
    matches!(action.state, ActionState::Completed)
        && action.swap_signature().is_some()
        && action
            .completed_at
            .is_some_and(|completed| now - completed >= window)
}

async fn load_decimals(chain: ChainId, mints: Vec<String>) -> HashMap<String, u8> {
    if mints.is_empty() {
        return HashMap::new();
    }
    crate::tokens::database::get_token_decimals_batch_async(chain, mints)
        .await
        .unwrap_or_else(|err| {
            logger::debug(
                LogTag::Webserver,
                &format!("Failed to load token decimals for actions: {err}"),
            );
            HashMap::new()
        })
}

/// The stored P&L of each closed position in `ids`, only where the row can stand
/// behind it (`Position::has_trustworthy_pnl`).
async fn load_round_pnl(ids: HashSet<i64>) -> HashMap<i64, RoundPnl> {
    let mut rounds = HashMap::with_capacity(ids.len());
    for id in ids {
        let Ok(Some(position)) = positions::get_db_position_by_id(id).await else {
            continue;
        };
        if !position.has_trustworthy_pnl() {
            continue;
        }
        if let Some(pnl) = position.pnl {
            rounds.insert(
                id,
                RoundPnl {
                    pnl,
                    pnl_percent: position.pnl_percent,
                },
            );
        }
    }
    rounds
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chains::RawAmount;
    use chrono::Utc;

    fn entry(is_dca: bool) -> EntryRecord {
        EntryRecord {
            id: Some(1),
            position_id: 7,
            timestamp: Utc::now(),
            amount: RawAmount::new(12_345_000_000),
            price: 0.000_000_4,
            native_spent: 0.005,
            transaction_signature: "sig-entry".to_owned(),
            is_dca,
            fees_raw: Some(5_000),
        }
    }

    fn exit(is_partial: bool) -> ExitRecord {
        ExitRecord {
            id: Some(2),
            position_id: 7,
            timestamp: Utc::now(),
            amount: RawAmount::new(12_345_000_000),
            price: 0.000_000_5,
            native_received: 0.0061,
            transaction_signature: "sig-exit".to_owned(),
            is_partial,
            percentage: if is_partial { 50.0 } else { 100.0 },
            fees_raw: None,
        }
    }

    const ROUND: RoundPnl = RoundPnl {
        pnl: 0.0011,
        pnl_percent: Some(22.0),
    };

    fn completed_swap(completed: DateTime<Utc>, signature: &str) -> Action {
        let mut action = Action::new(
            "a".to_owned(),
            crate::actions::ActionType::PositionDca,
            "mint".to_owned(),
            vec![crate::actions::ActionStepCode::Swap],
            serde_json::json!({}),
        );
        action.update_step(
            0,
            crate::actions::StepStatus::Completed,
            None,
            Some(serde_json::json!({ "signature": signature })),
        );
        action.complete_success();
        action.completed_at = Some(completed);
        action
    }

    #[test]
    fn an_unbooked_swap_is_flagged_only_after_the_verifier_stops_trying() {
        let window = positions::MAX_VERIFICATION_AGE_HOURS;
        let now = Utc::now();
        assert!(
            !is_unrecorded(&completed_swap(now, "sig"), now),
            "may still be booked"
        );
        assert!(!is_unrecorded(
            &completed_swap(now - Duration::hours(window - 1), "sig"),
            now
        ));
        assert!(is_unrecorded(
            &completed_swap(now - Duration::hours(window), "sig"),
            now
        ));
        assert!(
            !is_unrecorded(
                &completed_swap(now - Duration::hours(window), "unknown"),
                now
            ),
            "no signature, nothing to book"
        );
        let mut failed = completed_swap(now - Duration::hours(window), "sig");
        failed.state = ActionState::Cancelled;
        assert!(
            !is_unrecorded(&failed, now),
            "only a completed action claims a swap"
        );
    }

    #[test]
    fn an_entry_reports_what_it_spent_and_bought() {
        let outcome = entry_outcome(&entry(false), ChainId::Solana, Some(6));
        assert_eq!(outcome.kind, OutcomeKind::Entry);
        assert_eq!(outcome.token_amount, Some(12_345.0));
        assert_eq!(outcome.native_amount, 0.005);
        assert_eq!(outcome.fees_native, Some(0.000_005));
        assert_eq!(outcome.realized_pnl, None);
        assert_eq!(
            entry_outcome(&entry(true), ChainId::Solana, Some(6)).kind,
            OutcomeKind::Dca
        );
    }

    #[test]
    fn unknown_decimals_leave_the_token_amount_out_rather_than_guess_it() {
        assert_eq!(
            entry_outcome(&entry(false), ChainId::Solana, None).token_amount,
            None
        );
        assert_eq!(
            exit_outcome(&exit(false), ChainId::Solana, None, None).token_amount,
            None
        );
    }

    #[test]
    fn only_the_exit_that_closed_the_position_carries_its_pnl() {
        let full = exit_outcome(&exit(false), ChainId::Solana, Some(6), Some(ROUND));
        assert_eq!(full.kind, OutcomeKind::Exit);
        assert_eq!(full.realized_pnl, Some(0.0011));
        assert_eq!(full.realized_pnl_percent, Some(22.0));
        assert_eq!(full.exit_percentage, Some(100.0));

        let partial = exit_outcome(&exit(true), ChainId::Solana, Some(6), Some(ROUND));
        assert_eq!(partial.kind, OutcomeKind::PartialExit);
        assert_eq!(partial.realized_pnl, None);
        assert_eq!(partial.realized_pnl_percent, None);
        assert_eq!(partial.exit_percentage, Some(50.0));
    }
}
