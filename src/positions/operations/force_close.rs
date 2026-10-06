// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Force close — an operator write-off of a position stuck open, booked without a swap.

use chrono::Utc;

use crate::logger::{self, LogTag};
use crate::positions::apply::publish_booking;
use crate::positions::booking::ForceCloseFill;
use crate::positions::db::{commit_booking, BookingCommit, BookingGuard};
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
/// The booking is committed under the stored exit-verified guard before memory changes,
/// so a close verified concurrently and this write-off cannot both book. Slot release and
/// realized-loss accounting run once, after the commit. On any error the row and memory
/// are unchanged.
pub async fn force_close_position(position_id: i64, note: &str) -> Result<ForceClosed> {
    let closed_reason = format!("{FORCE_CLOSED_PREFIX} {note}");

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
    let exit_price = crate::pools::get_pool_price(&snapshot.mint)
        .map(|price| price.price_native)
        .filter(|price| *price > 0.0 && price.is_finite())
        .or(snapshot.current_price)
        .unwrap_or(0.0);
    let fill = ForceCloseFill {
        exit_time: Utc::now(),
        exit_price,
        closed_reason: closed_reason.clone(),
    };

    let mut candidate = snapshot;
    let realized_pnl = candidate.book_force_close(&fill)?;

    match commit_booking(&candidate, BookingGuard::ExitNotVerified, None).await? {
        BookingCommit::AlreadyBooked => return Err(Error::AlreadyClosed { position_id }),
        BookingCommit::Committed => {}
    }
    if in_memory {
        publish_booking(position_id, &candidate, |live| {
            live.book_force_close(&fill).map(|_| ())
        })
        .await;
    }

    // Idempotent: a queued exit verification for this position does not hand the slot
    // back a second time.
    release_position_slot(position_id).await;

    // A force close realizes the loss on everything still held. A wallet-derived round is
    // excluded: it is a pre-existing holding, not risk the bot took, and counting it could
    // pause the trader over money it never risked.
    if realized_pnl < 0.0 && !candidate.is_wallet_derived() {
        crate::trader::safety::loss_limit::record_realized_loss(realized_pnl.abs());
    }

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
