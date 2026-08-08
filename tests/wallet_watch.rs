//! Pure wallet-watch pagination and cadence contracts.

use screenerbot::wallets::watch::{cadence_secs, needs_gap_fill, CatchUpState};

fn full_page(prefix: &str) -> Vec<String> {
    (0..100).map(|n| format!("{prefix}-{n:03}")).collect()
}

#[test]
fn a_complete_range_is_replayed_oldest_first() {
    let mut state = CatchUpState::new(Some("durable".to_owned()));
    state.ingest_page(
        vec![
            "newest".to_owned(),
            "middle".to_owned(),
            "oldest".to_owned(),
        ],
        100,
    );

    let completed = state.completed().expect("range complete");
    assert_eq!(completed.signatures, ["oldest", "middle", "newest"]);
    assert_eq!(completed.newest_signature.as_deref(), Some("newest"));
}

#[test]
fn a_capped_range_resumes_without_exposing_a_cursor() {
    let mut state = CatchUpState::new(Some("durable".to_owned()));
    for page in 0..5 {
        state.ingest_page(full_page(&format!("page-{page}")), 100);
    }

    assert!(!state.is_complete());
    assert!(state.completed().is_none());

    state.ingest_page(vec!["tail".to_owned()], 100);
    let completed = state.completed().expect("range completed on next tick");
    assert_eq!(completed.signatures.len(), 501);
    assert_eq!(completed.signatures.first().unwrap(), "tail");
    assert_eq!(completed.signatures.last().unwrap(), "page-0-000");
}

#[test]
fn first_observation_is_bounded_to_one_recent_page() {
    let mut state = CatchUpState::new(None);
    state.ingest_page(full_page("recent"), 100);

    let completed = state.completed().expect("initial window completed");
    assert_eq!(completed.signatures.len(), 100);
    assert_eq!(completed.newest_signature.as_deref(), Some("recent-000"));
}

#[test]
fn multiple_pages_keep_global_oldest_first_order() {
    let mut state = CatchUpState::new(Some("durable".to_owned()));
    state.ingest_page(full_page("new"), 100);
    state.ingest_page(vec!["old-1".to_owned(), "old-2".to_owned()], 100);

    let completed = state.completed().expect("range complete");
    assert_eq!(completed.signatures.first().unwrap(), "old-2");
    assert_eq!(completed.signatures.last().unwrap(), "new-000");
}

#[test]
fn polling_escalates_offline_and_gap_fill_is_edge_triggered() {
    assert_eq!(cadence_secs(true, 30, 3), 30);
    assert_eq!(cadence_secs(false, 30, 3), 3);
    assert!(needs_gap_fill(true, false, false));
    assert!(needs_gap_fill(false, false, true));
    assert!(!needs_gap_fill(false, true, true));
}
