// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The wallet-relative direction names what moved and round-trips through storage.

use super::types::TransactionDirection;

#[test]
fn direction_names_the_token_flow_before_the_sol_flow() {
    // A buy: tokens arrive while SOL leaves. The label must say tokens came in, never
    // an unnamed "incoming" beside a negative SOL delta.
    assert!(matches!(
        TransactionDirection::from_wallet_flow(-500_000_000, 1_000),
        TransactionDirection::TokensIn
    ));
    // A sell: tokens leave while SOL arrives.
    assert!(matches!(
        TransactionDirection::from_wallet_flow(400_000_000, -1_000),
        TransactionDirection::TokensOut
    ));
    // No token moved: the SOL flow decides (a rent reclaim, a SOL transfer).
    assert!(matches!(
        TransactionDirection::from_wallet_flow(2_039_280, 0),
        TransactionDirection::SolIn
    ));
    assert!(matches!(
        TransactionDirection::from_wallet_flow(-250_000_000, 0),
        TransactionDirection::SolOut
    ));
    // Only the fee was paid.
    assert!(matches!(
        TransactionDirection::from_wallet_flow(0, 0),
        TransactionDirection::Internal
    ));
}

#[test]
fn every_direction_round_trips_through_its_stored_id() {
    for direction in [
        TransactionDirection::TokensIn,
        TransactionDirection::TokensOut,
        TransactionDirection::SolIn,
        TransactionDirection::SolOut,
        TransactionDirection::Internal,
        TransactionDirection::Unknown,
    ] {
        let id = direction.as_str();
        assert_eq!(TransactionDirection::from_stored(id).as_str(), id);
        // The API and the stored column carry the same id.
        assert_eq!(
            serde_json::to_value(&direction).expect("serialize direction"),
            serde_json::Value::String(id.to_owned())
        );
    }
}

#[test]
fn ids_that_do_not_name_a_subject_read_as_unknown() {
    for legacy in ["Incoming", "Outgoing", ""] {
        assert!(matches!(
            TransactionDirection::from_stored(legacy),
            TransactionDirection::Unknown
        ));
    }
}
