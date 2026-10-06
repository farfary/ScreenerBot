// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Tests for the position activity timeline - externally closed rounds report stored proceeds and P&L, and open rounds have no stored close.

use super::*;

const MINT: &str = "JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN";
const ENTRY_SIG: &str = "entry-signature";
const EXIT_SIG: &str = "exit-signature";

/// A bot entry whose token later left the wallet outside the bot: the wallet-history
/// ledger closed the row (`closed_externally`) and stored the round's proceeds and P&L
/// on it, with no exit record.
fn closed_externally_round() -> Position {
    serde_json::from_str(
        r#"{
        "id": 205,
        "mint": "JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN",
        "symbol": "JUP",
        "name": "Jupiter",
        "entry_price": 0.00602,
        "entry_time": "2026-09-15T00:00:00Z",
        "exit_price": 0.00235,
        "exit_time": "2026-09-18T12:00:00Z",
        "position_type": "buy",
        "entry_size_sol": 0.01304544,
        "total_size_sol": 0.01304544,
        "price_highest": 0.00602,
        "price_lowest": 0.00235,
        "entry_transaction_signature": "entry-signature",
        "exit_transaction_signature": "exit-signature",
        "token_amount": "2167137",
        "effective_entry_price": 0.00602,
        "effective_exit_price": 0.00235,
        "sol_received": 0.005090624,
        "profit_target_min": null,
        "profit_target_max": null,
        "liquidity_tier": null,
        "transaction_entry_verified": true,
        "transaction_exit_verified": true,
        "entry_fee_lamports": null,
        "exit_fee_lamports": null,
        "current_price": null,
        "current_price_updated": null,
        "phantom_remove": false,
        "phantom_confirmations": 0,
        "phantom_first_seen": null,
        "synthetic_exit": false,
        "closed_reason": "closed_externally",
        "pnl": -0.007954816,
        "pnl_percent": -60.98,
        "unrealized_pnl": null,
        "unrealized_pnl_percent": null,
        "remaining_token_amount": "0",
        "total_exited_amount": "2167137",
        "average_exit_price": 0.00235,
        "partial_exit_count": 0,
        "dca_count": 0,
        "average_entry_price": 0.00602,
        "last_dca_time": null,
        "origin": { "kind": "manual" },
        "management": "user_only"
        }"#,
    )
    .expect("position fixture deserializes")
}

fn summary_for(position: &Position) -> ActivityPositionSummary {
    ActivityPositionSummary {
        id: position.id.unwrap_or_default(),
        index: 1,
        opened_at: position.entry_time.timestamp(),
        closed_at: position.exit_time.map(|time| time.timestamp()),
        is_open: false,
        archived: false,
        swaps: 0,
        sol_invested: 0.0,
        sol_returned: 0.0,
        realized_pnl: 0.0,
    }
}

#[test]
fn closed_externally_round_reports_the_stored_proceeds_and_pnl() {
    let position = closed_externally_round();
    let entries = vec![EntryRecordResponse {
        id: Some(1),
        timestamp: position.entry_time.timestamp(),
        amount: RawAmount::from(2_167_137u64),
        price: 0.00602,
        sol_spent: 0.01304544,
        transaction_signature: ENTRY_SIG.to_owned(),
        is_dca: false,
        fees_sol: None,
    }];

    let mut events: Vec<ActivityEvent> =
        drafts::position_drafts(&position, 1, &entries, &[], |raw| raw.to_whole_units(6))
            .into_iter()
            .map(|draft| merge::merge_position_event(MINT, draft, None))
            .collect();
    events.sort_by_key(|event| event.timestamp.unwrap_or(i64::MAX));

    let mut summaries = vec![summary_for(&position)];
    let mut totals = walk(&mut events, &mut summaries);

    // The close has no record, so the walk alone books nothing for it.
    let exit = events
        .iter()
        .find(|event| event.side == "exit")
        .expect("the close is on the timeline");
    assert_eq!(exit.signature.as_deref(), Some(EXIT_SIG));
    assert!(!exit.recorded);
    assert_eq!(summaries[0].sol_returned, 0.0);

    let settled = HashMap::from([(205, StoredClose::of(&position).expect("closed row"))]);
    settle_closed_rounds(&mut summaries, &settled, &mut totals);

    let stored_received = position.sol_received.unwrap();
    let stored_pnl = position.pnl.unwrap();
    assert_eq!(summaries[0].swaps, 2);
    assert_eq!(summaries[0].sol_invested, position.total_size_sol);
    assert_eq!(summaries[0].sol_returned, stored_received);
    assert_eq!(summaries[0].realized_pnl, stored_pnl);
    assert_eq!(totals.sol_returned, stored_received);
    assert_eq!(totals.realized_pnl, stored_pnl);
}

#[test]
fn untrustworthy_pnl_is_not_taken_from_the_row() {
    let mut position = closed_externally_round();
    position.basis_complete = false;

    let close = StoredClose::of(&position).expect("closed row");
    assert_eq!(close.sol_received, position.sol_received);
    assert_eq!(close.pnl, None);
}

#[test]
fn open_round_has_no_stored_close() {
    let mut position = closed_externally_round();
    position.exit_time = None;

    assert!(StoredClose::of(&position).is_none());
}
