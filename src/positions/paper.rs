// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The paper book owner: simulated rounds in `paper_positions.db`, booked with the live
//! booking math and never through a swap, a key or a send. The guard
//! `tests/paper_isolation.rs` keeps this module free of live execution symbols.

mod account;
mod book;
mod fills;
mod leg;

pub use account::{paper_cash, PaperAccount};
pub use book::{PaperBook, PaperBooked, PaperOpenTerms, PaperRoundHistory};
pub use fills::PaperSale;
pub use leg::{PaperLeg, PAPER_LEG_PREFIX};

use tokio::sync::OnceCell;

use crate::chains::ChainId;
use crate::logger::{self, LogTag};

use super::types::Book;
use super::{PositionsDatabase, Result};

static PAPER_BOOK: OnceCell<PaperBook> = OnceCell::const_new();

/// Opens the paper store of `chain` and loads the open paper rounds of `wallet_address`.
/// A second call keeps the book the first one loaded.
pub async fn initialize(chain: ChainId, wallet_address: &str) -> Result<&'static PaperBook> {
    PAPER_BOOK
        .get_or_try_init(|| async {
            let db = PositionsDatabase::open(Book::Paper, chain).await?;
            let open = db.open_positions_of(wallet_address)?;
            logger::info(
                LogTag::Positions,
                &format!("Paper book loaded with {} open rounds", open.len()),
            );
            Ok(PaperBook::new(db, open))
        })
        .await
}

/// The paper book, once [`initialize`] loaded it.
pub fn book() -> Option<&'static PaperBook> {
    PAPER_BOOK.get()
}

#[cfg(test)]
mod tests;
