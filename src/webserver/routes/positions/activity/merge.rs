//! Joining an event's record half to its on-chain half.

use crate::chains::adapter;
use crate::transactions::{
    TokenTransfer, Transaction, TransactionDirection, TransactionStatus, TransactionType,
};

use super::super::types::{ActivityEvent, TransactionTokenTransferSummary};
use super::drafts::Draft;

/// A position swap: the record (what the position booked) joined to the transaction (what
/// the chain says).
pub(super) fn merge_position_event(
    mint: &str,
    draft: Draft,
    tx: Option<&Transaction>,
) -> ActivityEvent {
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

    // The synthetic close is the only event that can lack a signature — `push` drops every
    // other signature-less draft.
    let notes = match tx {
        Some(tx) => tx.error_message.clone(),
        None if draft.synthetic => Some("Synthetic exit — no on-chain signature".to_owned()),
        None => Some("Transaction not available in the local cache".to_owned()),
    };

    // Prefer the record's own time; a transaction's timestamp is when WE saw it.
    let timestamp = draft
        .timestamp
        .or_else(|| tx.map(|tx| tx.timestamp.timestamp()));

    ActivityEvent {
        kind: draft.kind,
        side: draft.side.as_str().to_owned(),
        sequence: 0, // assigned once the timeline is sorted
        state: state.to_owned(),
        signature: draft.signature,
        timestamp,

        position_id: draft.position_id,
        position_index: draft.position_index,

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
            .map(|tx| map_token_transfers(mint, &tx.token_transfers))
            .unwrap_or_default(),

        tokens_after: None,
        invested_after: None,
        cost_basis: None,
        realized_pnl: None,
        realized_pnl_percent: None,
    }
}

/// A wallet transaction that touched the token outside any position. It carries its own
/// chain data (that is what the transaction list returns), so there is nothing to join and
/// nothing to look up — and nothing it can move, since it belongs to no cost basis.
pub(super) fn wallet_event(draft: Draft) -> ActivityEvent {
    let row = draft
        .wallet
        .as_ref()
        .expect("wallet_event called on a draft with no wallet row");

    let state = if !row.success {
        "failed"
    } else if row.status == "Pending" {
        "pending"
    } else {
        "confirmed"
    };

    ActivityEvent {
        kind: draft.kind.clone(),
        side: draft.side.as_str().to_owned(),
        sequence: 0,
        state: state.to_owned(),
        signature: Some(row.signature.clone()),
        timestamp: Some(row.timestamp.timestamp()),

        position_id: None,
        position_index: None,

        recorded: false,
        record_id: None,
        token_amount: row.token_amount,
        price: None,
        sol_amount: None,
        exit_percentage: None,
        record_fee_sol: None,

        available: true,
        status: Some(row.status.clone()),
        success: Some(row.success),
        slot: row.slot,
        block_time: None,
        fee_sol: if row.fee_sol > 0.0 {
            Some(row.fee_sol)
        } else {
            row.fee_lamports.map(|l| adapter().raw_to_native(l))
        },
        direction: row.direction.clone(),
        transaction_type: row.transaction_type.clone(),
        router: row.router.clone(),
        sol_change: Some(row.sol_delta),
        instructions_count: Some(row.instructions_count),
        compute_units: None,
        accounts_count: None,
        notes: None,
        token_transfers: Vec::new(),

        tokens_after: None,
        invested_after: None,
        cost_basis: None,
        realized_pnl: None,
        realized_pnl_percent: None,
    }
}

fn transaction_fee_sol(tx: &Transaction) -> Option<f64> {
    if let Some(lamports) = tx.fee_lamports {
        Some(adapter().raw_to_native(lamports))
    } else if tx.fee_sol > 0.0 {
        Some(tx.fee_sol)
    } else {
        None
    }
}

fn map_token_transfers(
    mint: &str,
    transfers: &[TokenTransfer],
) -> Vec<TransactionTokenTransferSummary> {
    let mut relevant: Vec<&TokenTransfer> = transfers
        .iter()
        .filter(|transfer| transfer.mint == mint)
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
