// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Realized auto-trader performance over a window, plus open exposure, of one book. One
//! aggregation over one row set, shared by the dashboard Stats tab, the agent tools and
//! the promotional fixtures, so no surface can show a figure the other never computed.
//! The live and the paper book are aggregated apart and never mixed.
//!
//! The realized part is cached per book, wallet, window and UTC day behind the
//! fingerprint of the window's closed rounds: a poll of an unchanged book costs one
//! indexed aggregate statement, and the closed rows are read and aggregated again only
//! when that fingerprint moves. Open exposure is read on every call.

mod breakdowns;
mod realized;
mod types;

use chrono::{DateTime, NaiveDate, Utc};
use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex, PoisonError};

use crate::config::with_config;
use crate::positions::{self, Book, ClosedFingerprint, Position};

pub use realized::round_totals;
pub use types::{
    DailyPnlPoint, ExitBreakdown, Outcome, OutcomeBucket, PnlBucket, RealizedStats, RecentRound,
    RoundTotals, SourceBreakdown, StatsWindow, Streak, TraderStats,
};

/// Aggregate the closed rounds of `book` in `window` with its currently open exposure.
pub async fn trader_stats(book: Book, window: StatsWindow) -> TraderStats {
    // Open exposure. Not window-bound: what is at risk right now is independent of
    // how far back the realized window reaches. Only the rounds that take a slot count,
    // by the same rule admission counts them with.
    let slot_positions = positions::books::slot_positions(book).await;
    let max_open_positions = with_config(|cfg| cfg.trader.max_open_positions);
    let realized = cached_realized(&REALIZED_CACHE, book, window, Utc::now()).await;
    realized.with_exposure(&slot_positions, max_open_positions)
}

/// The realized part of `book` in `window` at `now`, from `cache` while the window's
/// closed rounds keep their fingerprint. A store that cannot be read yields the empty
/// window and is never cached, so the next poll reads it again.
async fn cached_realized(
    cache: &RealizedCache,
    book: Book,
    window: StatsWindow,
    now: DateTime<Utc>,
) -> Arc<RealizedStats> {
    let since = window.since(now);
    let (Ok(wallet), Some(fingerprint)) = (
        crate::utils::get_wallet_address(),
        closed_fingerprint(book, since).await,
    ) else {
        return Arc::new(RealizedStats::of(window, Vec::new(), now));
    };
    let key = StatsKey {
        book,
        wallet,
        window,
        day: now.date_naive(),
    };
    if let Some(realized) = cache.get(&key, &fingerprint) {
        return realized;
    }
    let Some(closed) = closed_since(book, since).await else {
        return Arc::new(RealizedStats::of(window, Vec::new(), now));
    };
    let realized = Arc::new(RealizedStats::of(window, closed, now));
    cache.insert(key, fingerprint, Arc::clone(&realized));
    realized
}

/// The fingerprint of the rounds of `book` closed since `since`; `None` when its store is
/// not open or cannot be read.
async fn closed_fingerprint(book: Book, since: DateTime<Utc>) -> Option<ClosedFingerprint> {
    match book {
        Book::Live => {
            let db_arc = positions::db::get_positions_database().await.ok()?;
            let db_guard = db_arc.lock().await;
            db_guard
                .as_ref()?
                .closed_fingerprint_since(since)
                .await
                .ok()
        }
        Book::Paper => positions::paper::book()?
            .closed_fingerprint_since(since)
            .await
            .ok(),
    }
}

/// The rounds of `book` closed since `since`; `None` when its store is not open or cannot
/// be read.
async fn closed_since(book: Book, since: DateTime<Utc>) -> Option<Vec<Position>> {
    match book {
        Book::Live => {
            let db_arc = positions::db::get_positions_database().await.ok()?;
            let db_guard = db_arc.lock().await;
            db_guard
                .as_ref()?
                .get_closed_positions_since(since)
                .await
                .ok()
        }
        Book::Paper => positions::paper::book()?.closed_since(since).await.ok(),
    }
}

/// What one cached realized part is of. The day is part of the key because the P&L
/// series and a day window's start move with the UTC date even when no round closes.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct StatsKey {
    book: Book,
    wallet: String,
    window: StatsWindow,
    day: NaiveDate,
}

/// The realized parts last computed, each with the fingerprint of the closed rounds it
/// was computed from.
#[derive(Default)]
struct RealizedCache {
    entries: Mutex<HashMap<StatsKey, (ClosedFingerprint, Arc<RealizedStats>)>>,
}

impl RealizedCache {
    /// The cached part of `key`, while it was computed from rounds of `fingerprint`.
    fn get(&self, key: &StatsKey, fingerprint: &ClosedFingerprint) -> Option<Arc<RealizedStats>> {
        let entries = self.entries.lock().unwrap_or_else(PoisonError::into_inner);
        entries
            .get(key)
            .filter(|(cached, _)| cached == fingerprint)
            .map(|(_, realized)| Arc::clone(realized))
    }

    /// Cache `realized` for `key`, dropping every entry of an earlier day.
    fn insert(&self, key: StatsKey, fingerprint: ClosedFingerprint, realized: Arc<RealizedStats>) {
        let mut entries = self.entries.lock().unwrap_or_else(PoisonError::into_inner);
        entries.retain(|cached, _| cached.day == key.day);
        entries.insert(key, (fingerprint, realized));
    }
}

static REALIZED_CACHE: LazyLock<RealizedCache> = LazyLock::new(RealizedCache::default);

#[cfg(test)]
mod tests;
