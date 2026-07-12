//! Position activity — builds the ONE merged swap timeline the detail route serves.
//!
//! A position owns more swaps than the two signatures on its struct: every DCA add and
//! every partial exit is its own transaction, and those live in the entry/exit RECORDS.
//! A swap that is submitted but not yet verified has no record at all — its signature sits
//! in the pending registries (`get_pending_partial_exits_for_mint` /
//! `get_pending_dca_swaps_for_mint`) until verification books it.
//!
//! This module gathers all three sources, joins each swap's record half to its on-chain
//! half, and walks the result chronologically to derive the running position state and the
//! realized P&L of every exit against the average entry price IN FORCE AT THAT MOMENT.

use futures::future::join_all;

use super::types::{
    EntryRecordResponse, ExitRecordResponse, PositionActivityEvent, PositionActivityTotals,
    TransactionTokenTransferSummary,
};
use crate::logger::{self, LogTag};
use crate::positions;
use crate::transactions::{
    get_transaction, TokenTransfer, Transaction, TransactionDirection, TransactionStatus,
    TransactionType,
};
use crate::utils::lamports_to_sol;

/// The record half of an event, before its on-chain half is fetched.
struct Draft {
    kind: &'static str,
    signature: Option<String>,
    /// Record time, else the pending registry's submit time, else the position's own
    /// entry/exit time. The chain's timestamp is only a last resort — a swap whose
    /// transaction is not in the cache still has a real time.
    timestamp: Option<i64>,
    recorded: bool,
    record_id: Option<i64>,
    token_amount: Option<u64>,
    price: Option<f64>,
    sol_amount: Option<f64>,
    exit_percentage: Option<f64>,
    record_fee_sol: Option<f64>,
    synthetic: bool,
}

impl Draft {
    fn new(kind: &'static str, signature: Option<String>, timestamp: Option<i64>) -> Self {
        Self {
            kind,
            signature,
            timestamp,
            recorded: false,
            record_id: None,
            token_amount: None,
            price: None,
            sol_amount: None,
            exit_percentage: None,
            record_fee_sol: None,
            synthetic: false,
        }
    }

    fn is_exit(&self) -> bool {
        matches!(self.kind, "exit" | "partial_exit")
    }
}

/// Build the position's full activity timeline, oldest first.
pub async fn build_activity(
    position: &positions::Position,
    entries: &[EntryRecordResponse],
    exits: &[ExitRecordResponse],
) -> (Vec<PositionActivityEvent>, PositionActivityTotals) {
    let drafts = collect_drafts(position, entries, exits).await;

    // Each event's on-chain half is an independent cache/RPC lookup.
    let fetched = join_all(drafts.iter().map(|draft| async {
        match &draft.signature {
            Some(signature) => match get_transaction(signature).await {
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
            },
            None => None,
        }
    }))
    .await;

    let mut events: Vec<PositionActivityEvent> = drafts
        .into_iter()
        .zip(fetched)
        .map(|(draft, tx)| merge_event(position, draft, tx.as_ref()))
        .collect();

    // Chronological. An event with no timestamp at all (a synthetic close on a position
    // whose exit time was never stamped) sorts last rather than to the epoch.
    events.sort_by_key(|event| event.timestamp.unwrap_or(i64::MAX));

    let totals = number_and_walk(&mut events);
    (events, totals)
}

/// Every swap the position owns, de-duplicated by signature (first source wins).
///
/// Records come first because they are authoritative: they carry the booked amount, price
/// and SOL. The position's own signatures only add a swap that has no record yet (an entry
/// or close still confirming) and the pending registries only add one that is not even
/// stamped on the position (a DCA add or partial exit still confirming).
async fn collect_drafts(
    position: &positions::Position,
    entries: &[EntryRecordResponse],
    exits: &[ExitRecordResponse],
) -> Vec<Draft> {
    let mut drafts: Vec<Draft> = Vec::with_capacity(entries.len() + exits.len() + 2);

    // A swap without a signature is not a swap. An OPEN position has no
    // `exit_transaction_signature`, and pushing that `None` anyway would give every open
    // position a phantom "Exit" row. The ONLY signature-less event is the synthetic close
    // below, which is appended directly.
    let mut push = |draft: Draft, drafts: &mut Vec<Draft>| {
        let Some(signature) = draft.signature.as_deref() else {
            return;
        };
        if signature.is_empty() {
            return;
        }
        if drafts
            .iter()
            .any(|existing| existing.signature.as_deref() == Some(signature))
        {
            return;
        }
        drafts.push(draft);
    };

    for entry in entries {
        let kind = if entry.is_dca { "dca" } else { "entry" };
        let mut draft = Draft::new(
            kind,
            Some(entry.transaction_signature.clone()),
            Some(entry.timestamp),
        );
        draft.recorded = true;
        draft.record_id = entry.id;
        draft.token_amount = Some(entry.amount);
        draft.price = Some(entry.price);
        draft.sol_amount = Some(entry.sol_spent);
        draft.record_fee_sol = entry.fees_sol;
        push(draft, &mut drafts);
    }

    for exit in exits {
        let kind = if exit.is_partial {
            "partial_exit"
        } else {
            "exit"
        };
        let mut draft = Draft::new(
            kind,
            Some(exit.transaction_signature.clone()),
            Some(exit.timestamp),
        );
        draft.recorded = true;
        draft.record_id = exit.id;
        draft.token_amount = Some(exit.amount);
        draft.price = Some(exit.price);
        draft.sol_amount = Some(exit.sol_received);
        draft.exit_percentage = Some(exit.percentage);
        draft.record_fee_sol = exit.fees_sol;
        push(draft, &mut drafts);
    }

    // Entry still confirming (or a legacy position that predates the records).
    push(
        Draft::new(
            "entry",
            position.entry_transaction_signature.clone(),
            Some(position.entry_time.timestamp()),
        ),
        &mut drafts,
    );

    // DCA adds in flight: their tokens and SOL land on the position only on verification,
    // so they have no record and are not stamped on the position either.
    for pending in positions::get_pending_dca_swaps_for_mint(&position.mint).await {
        push(
            Draft::new(
                "dca",
                Some(pending.signature),
                Some(pending.created_at.timestamp()),
            ),
            &mut drafts,
        );
    }

    // Partial exits in flight. The registry knows what was submitted, so the event can show
    // the EXPECTED amount and percentage — flagged `recorded: false`, since the position has
    // not booked them and must not be reported as if it had.
    for pending in positions::get_pending_partial_exits_for_mint(&position.mint).await {
        let mut draft = Draft::new(
            "partial_exit",
            Some(pending.signature),
            Some(pending.created_at.timestamp()),
        );
        draft.token_amount = Some(pending.expected_exit_amount);
        draft.exit_percentage = Some(pending.requested_exit_percentage);
        push(draft, &mut drafts);
    }

    // Close still confirming.
    push(
        Draft::new(
            "exit",
            position.exit_transaction_signature.clone(),
            position.exit_time.map(|time| time.timestamp()),
        ),
        &mut drafts,
    );

    // A position that exited without a usable signature (synthetic / force close) still owes
    // the user an exit row. An OPEN position that merely took partial profits has exit
    // records but has NOT exited — it must not get an empty "Exit" placeholder.
    let has_exited = position.exit_time.is_some() || position.synthetic_exit;
    if has_exited && !drafts.iter().any(|draft| draft.kind == "exit") {
        let mut draft = Draft::new("exit", None, position.exit_time.map(|t| t.timestamp()));
        draft.synthetic = true;
        drafts.push(draft);
    }

    drafts
}

/// Join a draft's record half to its on-chain half.
fn merge_event(
    position: &positions::Position,
    draft: Draft,
    tx: Option<&Transaction>,
) -> PositionActivityEvent {
    let side = if draft.is_exit() { "exit" } else { "entry" };

    let state = if draft.synthetic {
        "synthetic"
    } else if let Some(tx) = tx {
        if !tx.success {
            "failed"
        } else if matches!(tx.status, TransactionStatus::Pending) && !draft.recorded {
            "pending"
        } else {
            "confirmed"
        }
    } else if draft.recorded {
        // The record is only written on verification, so the swap did land — the
        // transaction simply is not in our cache.
        "confirmed"
    } else {
        "pending"
    };

    // The synthetic close is the only event that can lack a signature — `collect_drafts`
    // drops every other signature-less draft.
    let notes = match tx {
        Some(tx) => tx.error_message.clone(),
        None if draft.synthetic => Some("Synthetic exit — no on-chain signature".to_owned()),
        None => Some("Transaction not available in the local cache".to_owned()),
    };

    // Prefer the record's own time; a transaction's timestamp is when we saw it.
    let timestamp = draft
        .timestamp
        .or_else(|| tx.map(|tx| tx.timestamp.timestamp()));

    PositionActivityEvent {
        kind: draft.kind.to_owned(),
        side: side.to_owned(),
        sequence: 0, // assigned once the timeline is sorted
        state: state.to_owned(),
        signature: draft.signature,
        timestamp,

        recorded: draft.recorded,
        record_id: draft.record_id,
        token_amount: draft.token_amount,
        price: draft.price,
        sol_amount: draft.sol_amount,
        exit_percentage: draft.exit_percentage,
        record_fee_sol: draft.record_fee_sol,

        available: tx.is_some(),
        status: tx.map(|tx| describe_status(&tx.status)),
        success: tx.map(|tx| tx.success),
        slot: tx.and_then(|tx| tx.slot),
        block_time: tx.and_then(|tx| tx.block_time),
        fee_sol: tx.and_then(transaction_fee_sol),
        direction: tx.map(|tx| describe_direction(&tx.direction)),
        transaction_type: tx.map(|tx| describe_type(&tx.transaction_type)),
        router: tx.and_then(|tx| tx.token_swap_info.as_ref().map(|info| info.router.clone())),
        sol_change: tx.map(|tx| tx.sol_balance_change),
        instructions_count: tx.map(|tx| tx.instructions_count),
        compute_units: tx.and_then(|tx| tx.compute_units_consumed),
        accounts_count: tx.map(|tx| tx.accounts_count),
        notes,
        token_transfers: tx
            .map(|tx| map_token_transfers(position, &tx.token_transfers))
            .unwrap_or_default(),

        tokens_after: None,
        invested_after: None,
        cost_basis: None,
        realized_pnl: None,
        realized_pnl_percent: None,
    }
}

/// Number each side's events and walk the (already sorted) timeline to derive the running
/// position state, each exit's realized P&L, and the totals.
///
/// Only BOOKED events move the running state. A swap still confirming has not changed the
/// position — reporting otherwise is exactly the trap the pending registries exist to avoid.
///
/// An exit's cost basis is a proportional slice of the basis still on the books
/// (`invested * sold / held`), which is the same thing as "sold at the average entry price
/// in force right now" but needs no decimals to compute — and, unlike the position's final
/// `effective_entry_price`, it is correct for a position that DCA'd between two exits.
fn number_and_walk(events: &mut [PositionActivityEvent]) -> PositionActivityTotals {
    let mut totals = PositionActivityTotals::default();
    let mut entry_seq: u32 = 0;
    let mut exit_seq: u32 = 0;
    let mut tokens_held: u64 = 0;
    let mut invested: f64 = 0.0;

    for event in events.iter_mut() {
        let is_exit = event.side == "exit";
        if is_exit {
            exit_seq += 1;
            event.sequence = exit_seq;
        } else {
            entry_seq += 1;
            event.sequence = entry_seq;
        }

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
            _ => totals.exits += 1,
        }
        match event.state.as_str() {
            "pending" => totals.pending += 1,
            "failed" => totals.failed += 1,
            _ => {}
        }
        if let Some(fee) = event.fee_sol {
            totals.network_fees_sol += fee;
        }

        if !event.recorded {
            continue;
        }

        let amount = event.token_amount.unwrap_or(0);
        let sol = event.sol_amount.unwrap_or(0.0);

        if is_exit {
            let sold = amount.min(tokens_held);
            let basis = if tokens_held > 0 {
                invested * (sold as f64 / tokens_held as f64)
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

            tokens_held -= sold;
            invested = (invested - basis).max(0.0);

            totals.tokens_sold += amount;
            totals.sol_returned += sol;
            totals.realized_pnl += pnl;
        } else {
            tokens_held = tokens_held.saturating_add(amount);
            invested += sol;

            totals.tokens_bought += amount;
            totals.sol_invested += sol;
        }

        event.tokens_after = Some(tokens_held);
        event.invested_after = Some(invested);
    }

    totals
}

fn transaction_fee_sol(tx: &Transaction) -> Option<f64> {
    if let Some(lamports) = tx.fee_lamports {
        Some(lamports_to_sol(lamports))
    } else if tx.fee_sol > 0.0 {
        Some(tx.fee_sol)
    } else {
        None
    }
}

fn map_token_transfers(
    position: &positions::Position,
    transfers: &[TokenTransfer],
) -> Vec<TransactionTokenTransferSummary> {
    let mut relevant: Vec<&TokenTransfer> = transfers
        .iter()
        .filter(|transfer| transfer.mint == position.mint)
        .collect();

    if relevant.is_empty() {
        relevant = transfers.iter().collect();
    }

    relevant
        .into_iter()
        .take(8)
        .map(|transfer| TransactionTokenTransferSummary {
            mint: transfer.mint.clone(),
            amount: transfer.amount,
            from: transfer.from.clone(),
            to: transfer.to.clone(),
            program_id: transfer.program_id.clone(),
        })
        .collect()
}

fn describe_status(status: &TransactionStatus) -> String {
    match status {
        TransactionStatus::Pending => "Pending".to_owned(),
        TransactionStatus::Confirmed => "Confirmed".to_owned(),
        TransactionStatus::Finalized => "Finalized".to_owned(),
        TransactionStatus::Failed(err) => format!("Failed: {err}"),
    }
}

fn describe_direction(direction: &TransactionDirection) -> String {
    match direction {
        TransactionDirection::Incoming => "Incoming".to_owned(),
        TransactionDirection::Outgoing => "Outgoing".to_owned(),
        TransactionDirection::Internal => "Internal".to_owned(),
        TransactionDirection::Unknown => "Unknown".to_owned(),
    }
}

fn describe_type(transaction_type: &TransactionType) -> String {
    match transaction_type {
        TransactionType::Buy => "Buy".to_owned(),
        TransactionType::Sell => "Sell".to_owned(),
        TransactionType::Transfer => "Transfer".to_owned(),
        TransactionType::Compute => "Compute".to_owned(),
        TransactionType::AtaOperation => "ATA Operation".to_owned(),
        TransactionType::Failed => "Failed".to_owned(),
        TransactionType::Unknown => "Unknown".to_owned(),
        TransactionType::SwapSolToToken { router, .. } => format!("Swap SOL→Token ({router})"),
        TransactionType::SwapTokenToSol { router, .. } => format!("Swap Token→SOL ({router})"),
        TransactionType::SwapTokenToToken { router, .. } => format!("Swap Token→Token ({router})"),
        TransactionType::SolTransfer { .. } => "SOL Transfer".to_owned(),
        TransactionType::TokenTransfer { mint, amount, .. } => {
            format!("Token Transfer {mint} ({amount:.4})")
        }
        TransactionType::AtaClose { token_mint, .. } => format!("ATA Close ({token_mint})"),
        TransactionType::Other { description, .. } => description.clone(),
    }
}
