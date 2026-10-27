// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Tests for the position activity timeline - closed rounds report the basis, proceeds and P&L their rows stored, and open rounds have no stored close.

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
        "entry_size_native": 0.01304544,
        "total_size_native": 0.01304544,
        "price_highest": 0.00602,
        "price_lowest": 0.00235,
        "basis_peak_price": 0.00602,
        "entry_transaction_signature": "entry-signature",
        "exit_transaction_signature": "exit-signature",
        "token_amount": "2167137",
        "effective_entry_price": 0.00602,
        "effective_exit_price": 0.00235,
        "native_received": 0.005090624,
        "profit_target_min": null,
        "profit_target_max": null,
        "liquidity_tier": null,
        "transaction_entry_verified": true,
        "transaction_exit_verified": true,
        "entry_fee_raw": null,
        "exit_fee_raw": null,
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
        native_invested: 0.0,
        native_returned: 0.0,
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
        native_spent: 0.01304544,
        transaction_signature: ENTRY_SIG.to_owned(),
        is_dca: false,
        fees_native: None,
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
    assert_eq!(summaries[0].native_returned, 0.0);

    let settled = HashMap::from([(205, StoredClose::of(&position).expect("closed row"))]);
    settle_closed_rounds(&mut summaries, &mut events, &settled, &mut totals);

    let stored_received = position.native_received.unwrap();
    let stored_pnl = position.pnl.unwrap();
    assert_eq!(summaries[0].swaps, 2);
    assert_eq!(summaries[0].native_invested, position.total_size_native);
    assert_eq!(summaries[0].native_returned, stored_received);
    assert_eq!(summaries[0].realized_pnl, stored_pnl);
    assert_eq!(totals.native_returned, stored_received);
    assert_eq!(totals.realized_pnl, stored_pnl);
}

/// One entry record and three DCA adds that wrote none: the walk sees a quarter of the
/// basis, and the full sell would show a large profit on a round that lost its fees.
#[test]
fn a_round_whose_dca_adds_have_no_record_is_priced_against_its_whole_basis() {
    let mut position = closed_externally_round();
    position.closed_reason = None;
    position.dca_count = 3;
    position.total_size_native = 0.02;
    position.native_received = Some(0.0198);
    position.pnl = Some(-0.000196);
    let entries = vec![EntryRecordResponse {
        id: Some(1),
        timestamp: position.entry_time.timestamp(),
        amount: RawAmount::from(1_000_000_000u64),
        price: 0.000005,
        native_spent: 0.005,
        transaction_signature: ENTRY_SIG.to_owned(),
        is_dca: false,
        fees_native: None,
    }];
    let exits = vec![ExitRecordResponse {
        id: Some(1),
        timestamp: position.exit_time.unwrap().timestamp(),
        amount: RawAmount::from(4_000_000_000u64),
        price: 0.00000495,
        native_received: 0.0198,
        transaction_signature: EXIT_SIG.to_owned(),
        is_partial: false,
        percentage: 100.0,
        fees_native: None,
    }];

    let mut events: Vec<ActivityEvent> =
        drafts::position_drafts(&position, 1, &entries, &exits, |raw| raw.to_whole_units(6))
            .into_iter()
            .map(|draft| merge::merge_position_event(MINT, draft, None))
            .collect();
    events.sort_by_key(|event| event.timestamp.unwrap_or(i64::MAX));
    let mut summaries = vec![summary_for(&position)];
    let mut totals = walk(&mut events, &mut summaries);

    let walked_exit = events.iter().find(|event| event.side == "exit").unwrap();
    assert!(walked_exit.recorded);
    assert!(
        walked_exit.realized_pnl.unwrap() > 0.01,
        "the walk alone prices the sell against the one recorded entry"
    );

    let settled = HashMap::from([(205, StoredClose::of(&position).expect("closed row"))]);
    settle_closed_rounds(&mut summaries, &mut events, &settled, &mut totals);

    let exit = events.iter().find(|event| event.side == "exit").unwrap();
    assert!((exit.cost_basis.unwrap() - 0.02).abs() < 1e-12);
    assert!((exit.realized_pnl.unwrap() - -0.000196).abs() < 1e-12);
    assert!((exit.realized_pnl_percent.unwrap() - -0.98).abs() < 1e-9);
    assert_eq!(summaries[0].native_invested, 0.02);
    assert_eq!(summaries[0].realized_pnl, -0.000196);
    assert_eq!(totals.native_invested, 0.02);
    assert_eq!(totals.realized_pnl, -0.000196);
}

/// Two partial sells of a round with an incomplete record set: each keeps its walked share
/// of the basis, and together they carry the round's basis and P&L.
#[test]
fn a_closed_rounds_exits_add_up_to_its_basis_and_pnl() {
    let mut position = closed_externally_round();
    position.closed_reason = None;
    position.total_size_native = 0.03;
    position.native_received = Some(0.036);
    position.pnl = Some(0.0055);
    let entries = vec![EntryRecordResponse {
        id: Some(1),
        timestamp: position.entry_time.timestamp(),
        amount: RawAmount::from(3_000_000_000u64),
        price: 0.000005,
        native_spent: 0.015,
        transaction_signature: ENTRY_SIG.to_owned(),
        is_dca: false,
        fees_native: None,
    }];
    let exit_at = position.exit_time.unwrap().timestamp();
    let exits = vec![
        ExitRecordResponse {
            id: Some(1),
            timestamp: exit_at - 60,
            amount: RawAmount::from(1_000_000_000u64),
            price: 0.000012,
            native_received: 0.012,
            transaction_signature: "partial-signature".to_owned(),
            is_partial: true,
            percentage: 33.3,
            fees_native: None,
        },
        ExitRecordResponse {
            id: Some(2),
            timestamp: exit_at,
            amount: RawAmount::from(2_000_000_000u64),
            price: 0.000012,
            native_received: 0.024,
            transaction_signature: EXIT_SIG.to_owned(),
            is_partial: false,
            percentage: 100.0,
            fees_native: None,
        },
    ];

    let mut events: Vec<ActivityEvent> =
        drafts::position_drafts(&position, 1, &entries, &exits, |raw| raw.to_whole_units(6))
            .into_iter()
            .map(|draft| merge::merge_position_event(MINT, draft, None))
            .collect();
    events.sort_by_key(|event| event.timestamp.unwrap_or(i64::MAX));
    let mut summaries = vec![summary_for(&position)];
    let mut totals = walk(&mut events, &mut summaries);
    let settled = HashMap::from([(205, StoredClose::of(&position).expect("closed row"))]);
    settle_closed_rounds(&mut summaries, &mut events, &settled, &mut totals);

    let sells: Vec<&ActivityEvent> = events.iter().filter(|event| event.side == "exit").collect();
    assert_eq!(sells.len(), 2);
    let basis: f64 = sells.iter().filter_map(|event| event.cost_basis).sum();
    let pnl: f64 = sells.iter().filter_map(|event| event.realized_pnl).sum();
    assert!((basis - 0.03).abs() < 1e-12);
    assert!((pnl - 0.0055).abs() < 1e-12);
    // The first sell took a third of the walked basis, and keeps a third of the real one.
    assert!((sells[0].cost_basis.unwrap() - 0.01).abs() < 1e-12);
}

#[test]
fn untrustworthy_pnl_is_not_taken_from_the_row() {
    let mut position = closed_externally_round();
    position.basis_complete = false;

    let close = StoredClose::of(&position).expect("closed row");
    assert_eq!(close.sol_received, position.native_received);
    assert_eq!(close.pnl, None);
    assert_eq!(close.native_invested, None);
}

#[test]
fn open_round_has_no_stored_close() {
    let mut position = closed_externally_round();
    position.exit_time = None;

    assert!(StoredClose::of(&position).is_none());
}
