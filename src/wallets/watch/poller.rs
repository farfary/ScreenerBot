//! Resumable HTTP signature paging for baseline polls and gap-fill.
//!
//! Solana returns each page newest-first. A durable cursor may advance only after
//! the complete range back to the previous cursor has been fetched and replayed
//! oldest-first. `CatchUpState` therefore retains bounded pagination progress in
//! memory across ticks; a restart may re-page from the durable cursor, but cannot
//! skip signatures.

use chrono::{DateTime, Utc};

use super::runtime::WalletWatchRuntime;
use crate::wallets::Error;

/// Signatures per RPC page.
pub const PAGE_SIZE: usize = 100;

/// Maximum RPC pages fetched for one target in one service tick.
pub const MAX_PAGES: usize = 5;

/// Resolve baseline versus escalated polling without duplicating the state rule in
/// service code or tests.
pub fn cadence_secs(connected: bool, baseline_secs: u64, fallback_secs: u64) -> u64 {
    if connected {
        baseline_secs.max(1)
    } else {
        fallback_secs.max(1)
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
    pub fn ingest_page(&mut self, page: Vec<String>, page_size: usize) {
        if self.complete {
            return;
        }

        if page.is_empty() {
            self.complete = true;
            return;
        }

        let page_len = page.len();
        if self.newest_signature.is_none() {
            self.newest_signature = page.first().cloned();
        }
        self.before = page.last().cloned();
        let detected_at = Utc::now();
        self.newest_first
            .extend(page.into_iter().map(|signature| SeenSignature {
                signature,
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

/// Advance one target by at most `MAX_PAGES`. Returns a replay batch only after
/// the complete range back to the durable cursor is known. Paging mechanics for
/// the chain's wire format live behind the injected `runtime`; this function
/// stays chain-neutral.
pub(super) async fn advance_catch_up(
    runtime: &dyn WalletWatchRuntime,
    address: &str,
    state: &mut CatchUpState,
) -> Result<Option<CompletedCatchUp>, Error> {
    if state.is_complete() {
        return Ok(state.completed());
    }

    for _ in 0..MAX_PAGES {
        let page = runtime
            .fetch_signatures_page(
                address,
                PAGE_SIZE,
                state.before.as_deref(),
                state.durable_until.as_deref(),
            )
            .await?;
        state.ingest_page(page, PAGE_SIZE);
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

    fn signatures(prefix: &str, count: usize) -> Vec<String> {
        (0..count).map(|n| format!("{prefix}-{n:03}")).collect()
    }

    #[test]
    fn complete_range_replays_oldest_first_and_advances_to_newest() {
        let mut state = CatchUpState::new(Some("old-cursor".to_owned()));
        state.ingest_page(vec!["newest".to_owned(), "older".to_owned()], PAGE_SIZE);

        let completed = state.completed().expect("complete range");
        assert_eq!(names(&completed), ["older", "newest"]);
        assert_eq!(completed.newest_signature.as_deref(), Some("newest"));
    }

    #[test]
    fn multiple_pages_preserve_global_oldest_first_order() {
        let mut state = CatchUpState::new(Some("old-cursor".to_owned()));
        state.ingest_page(signatures("new", PAGE_SIZE), PAGE_SIZE);
        state.ingest_page(vec!["old-1".to_owned(), "old-2".to_owned()], PAGE_SIZE);

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

        state.ingest_page(vec!["tail".to_owned()], PAGE_SIZE);
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
    fn cadence_and_gap_fill_follow_connection_state() {
        assert_eq!(cadence_secs(true, 30, 3), 30);
        assert_eq!(cadence_secs(false, 30, 3), 3);
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
        let completed = advance_catch_up(runtime.as_ref(), "Addr1111", &mut state)
            .await
            .expect("fetch succeeds")
            .expect("range completes in one short page");

        assert_eq!(names(&completed), ["older", "newest"]);
        assert_eq!(completed.newest_signature.as_deref(), Some("newest"));
    }

    #[tokio::test]
    async fn advance_catch_up_stops_at_max_pages_without_advancing_cursor() {
        use super::super::runtime::test_support::FakeRuntime;

        let runtime = FakeRuntime::new(vec!["Addr1111".to_owned()]);
        for page in 0..MAX_PAGES {
            runtime.queue_page("Addr1111", signatures(&format!("page-{page}"), PAGE_SIZE));
        }

        let mut state = CatchUpState::new(Some("durable".to_owned()));
        let completed = advance_catch_up(runtime.as_ref(), "Addr1111", &mut state)
            .await
            .expect("fetch succeeds");

        assert!(completed.is_none(), "range must stay open past MAX_PAGES");
        assert!(!state.is_complete());
    }
}
