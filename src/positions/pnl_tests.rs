// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The unrealized P&L of open rounds: priced rounds are summed, unpriced ones are counted,
//! and a set with no priced round has no figure.

use super::*;
use crate::positions::{Book, Opening, PositionManagement, PositionOrigin};

fn open_round(unrealized_pnl: Option<f64>) -> Position {
    let mut position = Position::opening(Opening {
        mint: "Mint".to_owned(),
        symbol: "T".to_owned(),
        name: "Token".to_owned(),
        entry_price: 0.001,
        entry_time: chrono::Utc::now(),
        size_native: 0.1,
        entry_signature: None,
        origin: PositionOrigin::Auto { strategy_id: None },
        management: PositionManagement::AutoTrader,
        entry_liquidity: None,
        book: Book::Live,
    });
    position.unrealized_pnl = unrealized_pnl;
    position
}

#[test]
fn open_unrealized_sums_priced_rounds_and_counts_the_rest() {
    let rounds = [
        open_round(Some(0.02)),
        open_round(None),
        open_round(Some(-0.005)),
        open_round(None),
    ];
    let unrealized = open_unrealized(&rounds);
    assert_eq!(unrealized.unpriced, 2);
    let native = unrealized.native.expect("two rounds are priced");
    assert!((native - 0.015).abs() < 1e-12, "{native}");
}

#[test]
fn open_unrealized_is_absent_when_nothing_is_priced() {
    let rounds = [open_round(None), open_round(None)];
    assert_eq!(
        open_unrealized(&rounds),
        OpenUnrealized {
            native: None,
            unpriced: 2,
        }
    );
    assert_eq!(
        open_unrealized(std::iter::empty()),
        OpenUnrealized {
            native: None,
            unpriced: 0,
        }
    );
}
