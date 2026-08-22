//! Token activity — everything that ever happened to a token in this wallet.
//!
//! `GET /api/positions/{key}/activity`. The key resolves a position (the dialog was opened
//! on one), but the timeline it returns is scoped to that position's MINT, all-time: every
//! swap of every position ever opened on it — a token can be entered, exited and re-entered
//! any number of times, and each round is its own position — plus every wallet transaction
//! that touched the mint without belonging to a position at all.
//!
//! It is a separate route on purpose. `/details` is hit by the trade dialog on every manual
//! trade, by the row context menu and by the token-details Positions tab; none of them want
//! to pay for a multi-position, whole-wallet scan. The Activity tab fetches this lazily,
//! only while it is open.

mod drafts;
mod merge;

use std::collections::HashMap;

use axum::{extract::Path, http::StatusCode, response::Response};
use chrono::Utc;
use futures::future::join_all;

use super::detail::resolve_position_by_key;
use super::types::{
    ActivityEvent, ActivityPositionSummary, ActivityStateChange, ActivityTotals,
    EntryRecordResponse, ExitRecordResponse, TokenActivityResponse,
};
use crate::logger::{self, LogTag};
use crate::positions::{self, Position};
use crate::sol_price;
use crate::tokens;
use crate::transactions::get_transaction;
use crate::utils::lamports_to_sol;
use crate::webserver::utils::{error_response, success_response};

use drafts::Draft;

/// Most SPL tokens; used only when the token's decimals are not known yet, which is also
/// what the dashboard defaulted to before decimals moved server-side.
const FALLBACK_DECIMALS: u8 = 9;

pub async fn get_token_activity(Path(key): Path<String>) -> Response {
    let position = match resolve_position_by_key(&key).await {
        Ok(Some(position)) => position,
        Ok(None) => {
            return error_response(
                StatusCode::NOT_FOUND,
                "POSITION_NOT_FOUND",
                "Position not found",
                Some(&format!("No position found for key {key}")),
            )
        }
        Err(err) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "POSITION_ACTIVITY_ERROR",
                "Failed to resolve position",
                Some(&err),
            )
        }
    };

    if !drafts::is_tradeable_mint(&position.mint) {
        return error_response(
            StatusCode::BAD_REQUEST,
            "INVALID_MINT",
            "Wrapped SOL has no token activity",
            None,
        );
    }

    success_response(build_token_activity(&position).await)
}

async fn build_token_activity(current: &Position) -> TokenActivityResponse {
    let mint = current.mint.clone();
    let decimals = load_decimals(&mint).await;
    let scale = 10f64.powi(decimals as i32);
    let to_ui = move |raw: u64| raw as f64 / scale;

    // Every round of trading this token, oldest first. The position the dialog was opened
    // on is in here too — falling back to it alone keeps the tab working if the DB read
    // fails rather than showing nothing.
    let positions = match positions::get_all_positions_for_mint(&mint).await {
        Ok(found) if !found.is_empty() => found,
        Ok(_) => vec![current.clone()],
        Err(err) => {
            logger::info(
                LogTag::Webserver,
                &format!("Failed to load positions for {mint}: {err}"),
            );
            vec![current.clone()]
        }
    };

    let mut drafts: Vec<Draft> = Vec::new();
    let mut state_history: Vec<ActivityStateChange> = Vec::new();
    let mut summaries: Vec<ActivityPositionSummary> = Vec::new();

    for (offset, position) in positions.iter().enumerate() {
        let index = offset as u32 + 1;
        let (entries, exits) = load_records(position).await;

        drafts.extend(drafts::position_drafts(
            position, index, &entries, &exits, to_ui,
        ));
        drafts.extend(drafts::pending_drafts(position, index, to_ui).await);
        state_history.extend(load_state_history(position, index).await);

        summaries.push(ActivityPositionSummary {
            id: position.id.unwrap_or_default(),
            index,
            opened_at: position.entry_time.timestamp(),
            closed_at: position.exit_time.map(|time| time.timestamp()),
            is_open: position.exit_time.is_none() && !position.synthetic_exit,
            archived: position.archived,
            swaps: 0,
            sol_invested: 0.0,
            sol_returned: 0.0,
            realized_pnl: 0.0,
        });
    }

    // Anything else in the wallet that ever touched this mint: transfers, airdrops, ATA
    // rent, swaps made in another app. Whatever the positions already claimed is excluded.
    let claimed: Vec<String> = drafts
        .iter()
        .filter_map(|draft| draft.signature.clone())
        .collect();
    drafts.extend(drafts::wallet_drafts(&mint, &claimed).await);

    // Only position swaps need their transaction looked up; a wallet event already carries
    // its own chain data.
    let fetched = join_all(drafts.iter().map(|draft| async {
        if draft.wallet.is_some() {
            return None;
        }
        let signature = draft.signature.as_deref()?;
        match get_transaction(signature).await {
            Ok(tx) => tx,
            Err(err) => {
                logger::debug(
                    LogTag::Webserver,
                    &format!(
                        "Failed to load {} transaction {signature}: {err}",
                        draft.kind
                    ),
                );
                None
            }
        }
    }))
    .await;

    let mut events: Vec<ActivityEvent> = drafts
        .into_iter()
        .zip(fetched)
        .map(|(draft, tx)| {
            if draft.wallet.is_some() {
                merge::wallet_event(draft)
            } else {
                merge::merge_position_event(&mint, draft, tx.as_ref())
            }
        })
        .collect();

    // Chronological. An event with no timestamp at all (a synthetic close on a position
    // whose exit time was never stamped) sorts last rather than to the epoch.
    events.sort_by_key(|event| event.timestamp.unwrap_or(i64::MAX));
    state_history.sort_by_key(|change| change.changed_at);

    let totals = walk(&mut events, &mut summaries);

    TokenActivityResponse {
        mint,
        symbol: current.symbol.clone(),
        token_decimals: decimals,
        positions: summaries,
        events,
        totals,
        state_history,
        sol_price_usd: Some(sol_price::get_sol_price()).filter(|price| *price > 0.0),
        fetched_at: Utc::now().to_rfc3339(),
    }
}

/// Number each event and walk the (already sorted) timeline to derive every position's
/// running state, each exit's realized P&L, the per-position summaries and the totals.
///
/// The cost basis is tracked PER POSITION: a token entered, exited and re-entered starts a
/// fresh basis each round, and rolling one basis across all of them would price the second
/// round's exits against the first round's buys. Wallet events belong to no position and so
/// move nothing — the tokens an airdrop delivered were never bought.
///
/// Only a BOOKED (`recorded`) event moves the state. A swap still confirming has not
/// changed the position; reporting otherwise is exactly the trap the pending registries
/// exist to avoid.
///
/// An exit's cost basis is a proportional slice of the basis still on the books
/// (`invested * sold / held`) — the same thing as "sold at the average entry price in force
/// right now", and, unlike the position's final `effective_entry_price`, correct for a
/// position that DCA'd between two exits.
fn walk(events: &mut [ActivityEvent], summaries: &mut [ActivityPositionSummary]) -> ActivityTotals {
    let mut totals = ActivityTotals {
        positions: summaries.len(),
        ..Default::default()
    };

    // position id -> (tokens held, cost basis on the books)
    let mut books: HashMap<i64, (f64, f64)> = HashMap::new();
    // (position id, side) -> how many of that side we have seen
    let mut sequences: HashMap<(Option<i64>, String), u32> = HashMap::new();
    let by_id: HashMap<i64, usize> = summaries
        .iter()
        .enumerate()
        .map(|(slot, summary)| (summary.id, slot))
        .collect();

    for event in events.iter_mut() {
        let counter = sequences
            .entry((event.position_id, event.side.clone()))
            .or_insert(0);
        *counter += 1;
        event.sequence = *counter;

        totals.events += 1;
        match event.kind.as_str() {
            "entry" => totals.entries += 1,
            "dca" => {
                totals.entries += 1;
                totals.dca_entries += 1;
            }
            "partial_exit" => {
                totals.exits += 1;
                totals.partial_exits += 1;
            }
            "exit" => totals.exits += 1,
            _ => totals.wallet_events += 1,
        }
        match event.state.as_str() {
            "pending" => totals.pending += 1,
            "failed" => totals.failed += 1,
            _ => {}
        }
        if let Some(fee) = event.fee_sol {
            totals.network_fees_sol += fee;
        }

        let summary = event
            .position_id
            .and_then(|id| by_id.get(&id).copied())
            .and_then(|slot| summaries.get_mut(slot));
        if let Some(summary) = summary {
            summary.swaps += 1;
        }

        // A wallet event has no basis to move, and an unbooked swap has moved nothing yet.
        let Some(position_id) = event.position_id else {
            continue;
        };
        if !event.recorded {
            continue;
        }

        let amount = event.token_amount.unwrap_or(0.0);
        let sol = event.sol_amount.unwrap_or(0.0);
        let (held, invested) = books.entry(position_id).or_insert((0.0, 0.0));

        if event.side == "exit" {
            let sold = amount.min(*held);
            let basis = if *held > 0.0 {
                *invested * (sold / *held)
            } else {
                0.0
            };
            let pnl = sol - basis;

            event.cost_basis = Some(basis);
            event.realized_pnl = Some(pnl);
            event.realized_pnl_percent = if basis > 0.0 {
                Some(pnl / basis * 100.0)
            } else {
                None
            };

            *held -= sold;
            *invested = (*invested - basis).max(0.0);

            totals.tokens_sold += amount;
            totals.sol_returned += sol;
            totals.realized_pnl += pnl;

            if let Some(slot) = by_id.get(&position_id).copied() {
                if let Some(summary) = summaries.get_mut(slot) {
                    summary.sol_returned += sol;
                    summary.realized_pnl += pnl;
                }
            }
        } else {
            *held += amount;
            *invested += sol;

            totals.tokens_bought += amount;
            totals.sol_invested += sol;

            if let Some(slot) = by_id.get(&position_id).copied() {
                if let Some(summary) = summaries.get_mut(slot) {
                    summary.sol_invested += sol;
                }
            }
        }

        event.tokens_after = Some(*held);
        event.invested_after = Some(*invested);
    }

    totals
}

/// The token's decimals. Read from the stable on-chain `tokens.decimals` column, which
/// survives the market-data loss that empties an assembled token for a delisted or rugged
/// mint — the exact case where the activity view still has to render correct amounts.
async fn load_decimals(mint: &str) -> u8 {
    tokens::database::get_token_decimals_batch_async(vec![mint.to_owned()])
        .await
        .ok()
        .and_then(|map| map.get(mint).copied())
        .unwrap_or(FALLBACK_DECIMALS)
}

async fn load_records(position: &Position) -> (Vec<EntryRecordResponse>, Vec<ExitRecordResponse>) {
    let Some(id) = position.id else {
        return (Vec::new(), Vec::new());
    };

    let entries = positions::get_entry_history(id)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|record| EntryRecordResponse {
            id: record.id,
            timestamp: record.timestamp.timestamp(),
            amount: record.amount,
            price: record.price,
            sol_spent: record.sol_spent,
            transaction_signature: record.transaction_signature,
            is_dca: record.is_dca,
            fees_sol: record.fees_lamports.map(lamports_to_sol),
        })
        .collect();

    let exits = positions::get_exit_history(id)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|record| ExitRecordResponse {
            id: record.id,
            timestamp: record.timestamp.timestamp(),
            amount: record.amount,
            price: record.price,
            sol_received: record.sol_received,
            transaction_signature: record.transaction_signature,
            is_partial: record.is_partial,
            percentage: record.percentage,
            fees_sol: record.fees_lamports.map(lamports_to_sol),
        })
        .collect();

    (entries, exits)
}

async fn load_state_history(position: &Position, index: u32) -> Vec<ActivityStateChange> {
    let Some(id) = position.id else {
        return Vec::new();
    };

    let Ok(db_arc) = positions::get_positions_database().await else {
        return Vec::new();
    };
    let db = {
        let guard = db_arc.lock().await;
        guard.clone()
    };
    let Some(db) = db else {
        return Vec::new();
    };

    db.get_position_state_history(id)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|entry| ActivityStateChange {
            position_id: id,
            position_index: index,
            state: entry.state.to_string(),
            changed_at: entry.changed_at.timestamp(),
            reason: entry.reason,
        })
        .collect()
}
