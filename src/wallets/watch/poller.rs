// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Resumable HTTP signature paging for baseline polls and gap-fill.
//!
//! Solana returns each page newest-first. A durable cursor may advance only after
//! the complete range back to the previous cursor has been fetched and replayed
//! oldest-first. `CatchUpState` therefore retains bounded pagination progress in
//! memory across ticks; a restart may re-page from the durable cursor, but cannot
//! skip signatures.

use chrono::{DateTime, Utc};

use super::runtime::WalletWatchRuntime;
use super::types::{ReferenceVerdict, SignaturePageItem};
use crate::wallets::Error;

/// Signatures per RPC page: the most `getSignaturesForAddress` returns. A busy
/// address is referenced by thousands of other wallets' failed transactions a
/// minute; those are skipped without a detail fetch, so a full page costs one call.
pub const PAGE_SIZE: usize = 1000;

/// Signatures read on a target's first observation, which establishes its
/// cursor from recent history instead of replaying the address's lifetime.
pub const INITIAL_WINDOW: usize = 100;

/// Maximum RPC pages fetched for one target in one poll.
pub const MAX_PAGES: usize = 5;

/// Pages per poll for a target that names no budget of its own.
pub const DEFAULT_PAGE_BUDGET: usize = MAX_PAGES;

/// Upper bound for a user-selected per-wallet budget.
pub const MAX_PAGE_BUDGET: usize = 50;

/// Polls of a full budget an unfinished range may span before the target
/// pauses. An unfinished range is read on, poll after poll; only a backlog this
/// large (a long outage on an address referenced hundreds of times a second)
/// stops the target instead of holding an unbounded range in memory.
pub const BACKLOG_POLLS: usize = 20;

/// The most references an unfinished range of `page_budget` holds before the
/// target pauses.
pub fn backlog_cap(page_budget: usize) -> usize {
    page_budget.clamp(1, MAX_PAGE_BUDGET) * PAGE_SIZE * BACKLOG_POLLS
}

/// Resolve baseline versus escalated polling without duplicating the state rule in
/// service code or tests. `None` while a connected transport needs no baseline poll
/// (`WalletWatchRuntime::polls_while_connected`).
pub fn cadence_secs(
    connected: bool,
    polls_while_connected: bool,
    baseline_secs: u64,
    fallback_secs: u64,
) -> Option<u64> {
    match (connected, polls_while_connected) {
        (true, true) => Some(baseline_secs.max(1)),
        (true, false) => None,
        (false, _) => Some(fallback_secs.max(1)),
    }
}

/// Gap-fill once at startup and on a disconnected-to-connected transition.
pub fn needs_gap_fill(startup: bool, was_connected: bool, is_connected: bool) -> bool {
    startup || (!was_connected && is_connected)
}

/// One signature and the moment the page carrying it reached us. Replay can run
/// long after the fetch (each signature is decoded in turn), so the replay clock
/// would charge that queueing to the target's arrival latency.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeenSignature {
    pub signature: String,
    pub failed: bool,
    pub block_time: Option<i64>,
    pub verdict: ReferenceVerdict,
    pub detected_at: DateTime<Utc>,
}

/// A fully fetched range, ordered for replay.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletedCatchUp {
    /// Every signature after the old durable cursor, oldest first.
    pub signatures: Vec<SeenSignature>,
    /// The newest signature in the range; this becomes durable after replay.
    pub newest_signature: Option<String>,
}

impl CompletedCatchUp {
    /// References per minute across the range, from its oldest and newest block
    /// times. `None` when fewer than two references carry a block time.
    pub fn references_per_minute(&self) -> Option<f64> {
        let mut times = self.signatures.iter().filter_map(|seen| seen.block_time);
        let oldest = times.next()?;
        let newest = times.last()?;
        // Block times have one-second resolution: a burst inside one second spans one.
        let span_secs = (newest - oldest).max(1) as f64;
        Some(self.signatures.len() as f64 * 60.0 / span_secs)
    }
}

/// In-memory progress through one range ending at a durable cursor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatchUpState {
    durable_until: Option<String>,
    initial_window: bool,
    before: Option<String>,
    newest_signature: Option<String>,
    newest_first: Vec<SeenSignature>,
    complete: bool,
}

impl CatchUpState {
    pub fn new(durable_until: Option<String>) -> Self {
        let initial_window = durable_until.is_none();
        Self {
            durable_until,
            initial_window,
            before: None,
            newest_signature: None,
            newest_first: Vec::new(),
            complete: false,
        }
    }

    /// Absorb one newest-first RPC page. A short or empty page proves that the
    /// requested range is complete; a full page requires another request.
    pub fn ingest_page(&mut self, page: Vec<SignaturePageItem>, page_size: usize) {
        if self.complete {
            return;
        }

        if page.is_empty() {
            self.complete = true;
            return;
        }

        let page_len = page.len();
        if self.newest_signature.is_none() {
            self.newest_signature = page.first().map(|item| item.signature.clone());
        }
        self.before = page.last().map(|item| item.signature.clone());
        let detected_at = Utc::now();
        self.newest_first
            .extend(page.into_iter().map(|item| SeenSignature {
                signature: item.signature,
                failed: item.failed,
                block_time: item.block_time,
                verdict: item.verdict,
                detected_at,
            }));
        // With no durable cursor there is no previously observed boundary to fill.
        // Establish observation from one bounded recent page; replaying an address's
        // entire lifetime on first add is neither gap-fill nor safe for memory.
        self.complete = self.initial_window || page_len < page_size;
    }

    pub fn is_complete(&self) -> bool {
        self.complete
    }

    /// Newest signature seen so far in this range, if any page arrived.
    pub fn newest_signature(&self) -> Option<&str> {
        self.newest_signature.as_deref()
    }

    /// Signatures held in memory for a range that is not complete yet.
    pub fn pending_len(&self) -> usize {
        self.newest_first.len()
    }

    pub fn completed(&self) -> Option<CompletedCatchUp> {
        self.complete.then(|| {
            let mut signatures = self.newest_first.clone();
            signatures.reverse();
            CompletedCatchUp {
                signatures,
                newest_signature: self.newest_signature.clone(),
            }
        })
    }
}

/// Advance one target by at most `page_budget` pages. Returns a replay batch only after
/// the complete range back to the durable cursor is known. Paging mechanics for
/// the chain's wire format live behind the injected `runtime`; this function
/// stays chain-neutral.
pub(super) async fn advance_catch_up(
    runtime: &dyn WalletWatchRuntime,
    address: &str,
    state: &mut CatchUpState,
    page_budget: usize,
) -> Result<Option<CompletedCatchUp>, Error> {
    if state.is_complete() {
        return Ok(state.completed());
    }

    for _ in 0..page_budget.clamp(1, MAX_PAGE_BUDGET) {
        let limit = if state.initial_window {
            INITIAL_WINDOW
        } else {
            PAGE_SIZE
        };
        let page = runtime
            .fetch_signatures_page(
                address,
                limit,
                state.before.as_deref(),
                state.durable_until.as_deref(),
            )
            .await?;
        state.ingest_page(page, limit);
        if state.is_complete() {
            break;
        }
    }

    Ok(state.completed())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(completed: &CompletedCatchUp) -> Vec<&str> {
        completed
            .signatures
            .iter()
            .map(|seen| seen.signature.as_str())
            .collect()
    }

    fn signatures(prefix: &str, count: usize) -> Vec<SignaturePageItem> {
        (0..count)
            .map(|n| SignaturePageItem {
                signature: format!("{prefix}-{n:03}"),
                ..SignaturePageItem::default()
            })
            .collect()
    }

    fn page(items: &[(&str, bool)]) -> Vec<SignaturePageItem> {
        items
            .iter()
            .map(|(signature, failed)| SignaturePageItem {
                signature: (*signature).to_owned(),
                failed: *failed,
                ..SignaturePageItem::default()
            })
            .collect()
    }

    #[test]
    fn complete_range_replays_oldest_first_and_advances_to_newest() {
        let mut state = CatchUpState::new(Some("old-cursor".to_owned()));
        state.ingest_page(page(&[("newest", false), ("older", false)]), PAGE_SIZE);

        let completed = state.completed().expect("complete range");
        assert_eq!(names(&completed), ["older", "newest"]);
        assert_eq!(completed.newest_signature.as_deref(), Some("newest"));
    }

    #[test]
    fn multiple_pages_preserve_global_oldest_first_order() {
        let mut state = CatchUpState::new(Some("old-cursor".to_owned()));
        state.ingest_page(signatures("new", PAGE_SIZE), PAGE_SIZE);
        state.ingest_page(page(&[("old-1", false), ("old-2", false)]), PAGE_SIZE);

        let completed = state.completed().expect("complete range");
        assert_eq!(completed.signatures.first().unwrap().signature, "old-2");
        assert_eq!(completed.signatures.last().unwrap().signature, "new-000");
        assert_eq!(completed.signatures.len(), PAGE_SIZE + 2);
    }

    #[test]
    fn capped_full_pages_remain_incomplete_without_cursor_advance() {
        let mut state = CatchUpState::new(Some("durable".to_owned()));
        for page in 0..MAX_PAGES {
            state.ingest_page(signatures(&format!("page-{page}"), PAGE_SIZE), PAGE_SIZE);
        }

        assert!(!state.is_complete());
        assert_eq!(state.completed(), None);

        state.ingest_page(page(&[("tail", false)]), PAGE_SIZE);
        let completed = state.completed().expect("resumed range completes");
        assert_eq!(completed.signatures.first().unwrap().signature, "tail");
        assert_eq!(completed.newest_signature.as_deref(), Some("page-0-000"));
    }

    #[test]
    fn first_observation_is_one_bounded_recent_window() {
        let mut state = CatchUpState::new(None);
        state.ingest_page(signatures("recent", PAGE_SIZE), PAGE_SIZE);

        let completed = state.completed().expect("initial window completes");
        assert_eq!(completed.signatures.len(), PAGE_SIZE);
        assert_eq!(completed.newest_signature.as_deref(), Some("recent-000"));
    }

    #[test]
    fn completed_range_keeps_failure_metadata_in_oldest_first_order() {
        let mut state = CatchUpState::new(Some("old-cursor".to_owned()));
        state.ingest_page(
            page(&[
                ("new-success", false),
                ("middle-failed", true),
                ("old-success", false),
            ]),
            PAGE_SIZE,
        );

        let completed = state.completed().expect("complete range");
        assert_eq!(
            names(&completed),
            ["old-success", "middle-failed", "new-success"]
        );
        assert_eq!(
            completed
                .signatures
                .iter()
                .map(|seen| seen.failed)
                .collect::<Vec<_>>(),
            [false, true, false]
        );
        assert_eq!(completed.newest_signature.as_deref(), Some("new-success"));
    }

    #[test]
    fn a_completed_range_reports_its_reference_rate_from_block_times() {
        let mut state = CatchUpState::new(Some("old-cursor".to_owned()));
        state.ingest_page(
            (0..30)
                .map(|n| SignaturePageItem {
                    signature: format!("s{n}"),
                    block_time: Some(1_000 - n / 10),
                    ..SignaturePageItem::default()
                })
                .collect(),
            PAGE_SIZE,
        );
        let completed = state.completed().expect("short page completes");
        assert_eq!(completed.references_per_minute(), Some(30.0 * 60.0 / 2.0));

        let mut sparse = CatchUpState::new(Some("old-cursor".to_owned()));
        sparse.ingest_page(signatures("one", 1), PAGE_SIZE);
        assert_eq!(sparse.completed().unwrap().references_per_minute(), None);
    }

    #[test]
    fn cadence_and_gap_fill_follow_connection_state() {
        assert_eq!(cadence_secs(true, true, 30, 3), Some(30));
        assert_eq!(cadence_secs(false, true, 30, 3), Some(3));
        assert_eq!(
            cadence_secs(true, false, 30, 3),
            None,
            "a connected stream that gap-fills on every notification is never polled"
        );
        assert_eq!(cadence_secs(false, false, 30, 3), Some(3));
        assert!(needs_gap_fill(true, false, false));
        assert!(needs_gap_fill(false, false, true));
        assert!(!needs_gap_fill(false, true, true));
        assert!(!needs_gap_fill(false, false, false));
    }

    #[tokio::test]
    async fn advance_catch_up_pages_through_the_injected_runtime() {
        use super::super::runtime::test_support::FakeRuntime;

        let runtime = FakeRuntime::new(vec!["Addr1111".to_owned()]);
        runtime.queue_page("Addr1111", vec!["newest".to_owned(), "older".to_owned()]);

        let mut state = CatchUpState::new(Some("old-cursor".to_owned()));
        let completed = advance_catch_up(
            runtime.as_ref(),
            "Addr1111",
            &mut state,
            DEFAULT_PAGE_BUDGET,
        )
        .await
        .expect("fetch succeeds")
        .expect("range completes in one short page");

        assert_eq!(names(&completed), ["older", "newest"]);
        assert_eq!(completed.newest_signature.as_deref(), Some("newest"));
    }

    #[tokio::test]
    async fn the_first_observation_reads_a_small_window_and_later_ranges_full_pages() {
        use super::super::runtime::test_support::FakeRuntime;

        let runtime = FakeRuntime::new(vec!["Addr1111".to_owned()]);
        runtime.queue_page("Addr1111", vec!["recent".to_owned()]);
        let mut first = CatchUpState::new(None);
        advance_catch_up(
            runtime.as_ref(),
            "Addr1111",
            &mut first,
            DEFAULT_PAGE_BUDGET,
        )
        .await
        .expect("fetch succeeds")
        .expect("the first window completes");

        runtime.queue_page("Addr1111", vec!["newer".to_owned()]);
        let mut later = CatchUpState::new(Some("recent".to_owned()));
        advance_catch_up(
            runtime.as_ref(),
            "Addr1111",
            &mut later,
            DEFAULT_PAGE_BUDGET,
        )
        .await
        .expect("fetch succeeds")
        .expect("a short page completes the range");

        assert_eq!(
            runtime.calls.lock().unwrap().page_sizes,
            [INITIAL_WINDOW, PAGE_SIZE]
        );
    }

    #[tokio::test]
    async fn advance_catch_up_stops_at_max_pages_without_advancing_cursor() {
        use super::super::runtime::test_support::FakeRuntime;

        let runtime = FakeRuntime::new(vec!["Addr1111".to_owned()]);
        for page in 0..MAX_PAGES {
            runtime.queue_page_items("Addr1111", signatures(&format!("page-{page}"), PAGE_SIZE));
        }

        let mut state = CatchUpState::new(Some("durable".to_owned()));
        let completed = advance_catch_up(
            runtime.as_ref(),
            "Addr1111",
            &mut state,
            DEFAULT_PAGE_BUDGET,
        )
        .await
        .expect("fetch succeeds");

        assert!(completed.is_none(), "range must stay open past MAX_PAGES");
        assert!(!state.is_complete());
    }
}
