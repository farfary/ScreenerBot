// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Force close — an operator write-off of a position stuck open, booked without a swap.

use chrono::Utc;

use crate::logger::{self, LogTag};
use crate::positions::apply::book_position;
use crate::positions::booking::ForceCloseFill;
use crate::positions::db::{Booking, Committed};
use crate::positions::state::{get_position_by_id, release_position_slot};
use crate::positions::{Error, Result, FORCE_CLOSED_PREFIX};

/// A committed force close.
#[derive(Debug, Clone)]
pub struct ForceClosed {
    pub position_id: i64,
    pub symbol: String,
    pub closed_reason: String,
}

/// Writes a position off: whatever is still held is booked as exited with no proceeds,
/// while the proceeds of earlier partial exits stay on the books.
///
/// The booking is made on the row read inside its own transaction, and refused when that
/// row's exit is already verified, so a close verified concurrently and this write-off
/// cannot both book. Memory adopts the committed row. The slot release and the loss
/// limiter's recompute from the books run once, after the commit. On any error the row and memory
/// are unchanged.
pub async fn force_close_position(position_id: i64, note: &str) -> Result<ForceClosed> {
    let closed_reason = format!("{FORCE_CLOSED_PREFIX} {note}");

    // The snapshot supplies the mint and the last known price; the booking itself reads the
    // stored row.
    let (snapshot, in_memory) = match get_position_by_id(position_id).await {
        Some(position) => (position, true),
        None => match crate::positions::db::get_position_by_id(position_id).await? {
            Some(position) => (position, false),
            None => return Err(Error::NotFoundById { position_id }),
        },
    };

    if snapshot.exit_time.is_some() && snapshot.transaction_exit_verified {
        return Err(Error::AlreadyClosed { position_id });
    }

    // The exit price is informational: a live pool price when there is one, else the last
    // known price.
    let exit_price = crate::positions::price_resolution::live_pool_price(&snapshot.mint)
        .map(|price| price.price_native)
        .filter(|price| *price > 0.0 && price.is_finite())
        .or(snapshot.current_price)
        .unwrap_or(0.0);
    let fill = ForceCloseFill {
        exit_time: Utc::now(),
        exit_price,
        closed_reason: closed_reason.clone(),
    };

    let committed = book_position(position_id, |row, _| {
        if row.transaction_exit_verified {
            return Ok(Booking::Skip(false));
        }
        row.book_force_close(&fill)?;
        Ok(Booking::Write {
            record: None,
            outcome: true,
        })
    })
    .await?;
    let candidate = match committed {
        Committed::Written { row, outcome: true } => row,
        _ => return Err(Error::AlreadyClosed { position_id }),
    };

    // Idempotent: a queued exit verification for this position does not hand the slot
    // back a second time.
    release_position_slot(position_id).await;

    // The loss limiter follows the books, which now hold the write-off. A wallet-derived
    // round is not counted there: it is a pre-existing holding, not risk the bot took.
    crate::trader::safety::loss_limit::sync_from_books().await;

    logger::info(
        LogTag::Positions,
        &format!(
            "Force-closed position {position_id} ({}) - reason: {closed_reason}, in_memory: {in_memory}",
            candidate.symbol
        ),
    );

    Ok(ForceClosed {
        position_id,
        symbol: candidate.symbol,
        closed_reason,
    })
}
