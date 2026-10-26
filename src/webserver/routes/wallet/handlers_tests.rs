// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The buy funding the trade dialog reads carries the network fee reserve of the swap funds check itself.

use super::handlers::buy_funding;
use crate::chains::solana::swaps::funds::NETWORK_FEE_RESERVE_LAMPORTS;
use crate::chains::{adapter_for, ChainId};

#[test]
fn the_served_fee_reserve_is_the_funds_check_reserve() {
    let adapter = adapter_for(ChainId::Solana);
    let funding = buy_funding(adapter, 1_461_600);

    assert_eq!(
        adapter.native_to_raw(funding.fee_reserve_sol),
        NETWORK_FEE_RESERVE_LAMPORTS
    );
    assert_eq!(adapter.native_to_raw(funding.rent_sol), 1_461_600);
}
