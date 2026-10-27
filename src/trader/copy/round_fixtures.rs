// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Position rows of copy rounds for the copy module's read-side tests.

use chrono::{DateTime, Duration, Utc};

use crate::positions::types::Opening;
use crate::positions::{Book, Position, PositionManagement, PositionOrigin};
use crate::trader::types::TradeReason;

/// An open round of copy task `task_id` in `book`, bought for `size` at `entry`,
/// entered `minutes_ago` and marked at its entry price.
pub(super) fn open_round(
    task_id: i64,
    book: Book,
    mint: &str,
    entry: f64,
    size: f64,
    minutes_ago: i64,
) -> Position {
    let entry_time = Utc::now() - Duration::minutes(minutes_ago);
    let mut position = Position::opening(Opening {
        mint: mint.to_owned(),
        symbol: mint.to_uppercase(),
        name: mint.to_owned(),
        entry_price: entry,
        entry_time,
        size_native: size,
        entry_signature: None,
        origin: PositionOrigin::Copy {
            task_id,
            source_wallet: "target".to_owned(),
        },
        management: PositionManagement::Hybrid,
        entry_liquidity: None,
        book,
    });
    position.id = Some(task_id * 1_000 + minutes_ago);
    position.transaction_entry_verified = true;
    position.unrealized_pnl = Some(0.0);
    position.unrealized_pnl_percent = Some(0.0);
    position
}

/// `position` marked at `price`, with the unrealized result of that mark.
pub(super) fn marked(mut position: Position, price: Option<f64>) -> Position {
    position.current_price = price;
    position.unrealized_pnl = price.map(|price| {
        (price - position.average_entry_price) / position.average_entry_price
            * position.total_size_native
    });
    position
}

/// `position` closed at `at` for `reason` with the realized result `pnl`.
pub(super) fn closed(
    mut position: Position,
    pnl: f64,
    reason: TradeReason,
    at: DateTime<Utc>,
) -> Position {
    position.exit_time = Some(at);
    position.exit_price = Some(position.entry_price * (1.0 + pnl / position.total_size_native));
    position.native_received = Some(position.total_size_native + pnl);
    position.pnl = Some(pnl);
    position.pnl_percent = Some(pnl / position.total_size_native * 100.0);
    position.closed_reason = Some(format!("{reason:?}"));
    position.transaction_exit_verified = true;
    position.unrealized_pnl = None;
    position.unrealized_pnl_percent = None;
    position
}
