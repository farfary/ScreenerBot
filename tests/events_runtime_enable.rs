// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Events switched on in settings while the app runs: the store starts without a restart, the
//! first recorded event is persisted, and the service reports healthy. The writer inserts each
//! batch on the blocking pool, never on a runtime worker.

use crate::common;

use screenerbot::events::{self, Event, EventCategory, Severity};
use screenerbot::services::implementations::EventsService;
use screenerbot::services::Service;
use serde_json::json;
use std::time::{Duration, Instant};

const MINT: &str = "EventsRuntimeEnableMint1111111111111111111";

fn event() -> Event {
    Event::new(
        EventCategory::System,
        Some("runtime_enable".to_owned()),
        Severity::Info,
        Some(MINT.to_owned()),
        None,
        json!({}),
    )
}

/// Polls until the writer flushed the event to the store, or the deadline passes.
async fn persisted_events() -> usize {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let found = events::by_mint(MINT, 10).await.map_or(0, |rows| rows.len());
        if found > 0 || Instant::now() >= deadline {
            return found;
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
}

#[test]
fn an_event_recorded_after_events_were_switched_on_is_persisted() {
    common::run_isolated(
        module_path!(),
        "an_event_recorded_after_events_were_switched_on_is_persisted",
        || async {
            let _dir = common::isolated_env();
            let _cfg = common::config_guard();
            screenerbot::paths::ensure_all_directories().expect("create data directories");
            screenerbot::global::set_explore_mode(true);

            assert!(
                events::EVENTS_DB.get().is_none(),
                "no store before events are switched on"
            );
            common::set_config(|cfg| cfg.events.enabled = true);

            events::record(event())
                .await
                .expect("the first event starts the store and is queued");

            assert!(events::EVENTS_DB.get().is_some(), "the store was started");
            assert_eq!(persisted_events().await, 1, "the event reached the store");
            assert!(
                EventsService.health().await.is_healthy(),
                "the service is healthy once the store runs"
            );
        },
    );
}

#[test]
fn the_health_check_starts_the_store_when_events_were_switched_on_without_an_event() {
    common::run_isolated(
        module_path!(),
        "the_health_check_starts_the_store_when_events_were_switched_on_without_an_event",
        || async {
            let _dir = common::isolated_env();
            let _cfg = common::config_guard();
            screenerbot::paths::ensure_all_directories().expect("create data directories");
            screenerbot::global::set_explore_mode(true);
            common::set_config(|cfg| cfg.events.enabled = true);

            assert!(events::EVENTS_DB.get().is_none());
            assert!(
                EventsService.health().await.is_healthy(),
                "enabled at runtime reports healthy, not an uninitialized store"
            );
            assert!(
                events::EVENTS_DB.get().is_some(),
                "the check started the store"
            );

            events::record_safe(event()).await;
            assert_eq!(
                persisted_events().await,
                1,
                "recording works after the check"
            );
        },
    );
}

#[test]
fn recording_with_events_switched_off_starts_nothing() {
    common::run_isolated(
        module_path!(),
        "recording_with_events_switched_off_starts_nothing",
        || async {
            let _dir = common::isolated_env();
            let _cfg = common::config_guard();
            screenerbot::paths::ensure_all_directories().expect("create data directories");

            events::record(event())
                .await
                .expect("a switched-off record is a no-op");
            events::record_safe(event()).await;

            assert!(events::EVENTS_DB.get().is_none(), "no store was started");
        },
    );
}

/// The batch insert is synchronous and the event writer runs it inside `spawn_blocking`, so a
/// slow events store never parks a runtime worker.
#[test]
fn the_event_writer_inserts_batches_on_the_blocking_pool() {
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/events");
    let store = std::fs::read_to_string(src.join("database/mod.rs")).expect("read events store");
    let writer = std::fs::read_to_string(src.join("mod.rs")).expect("read events writer");

    assert!(
        store.contains("pub fn insert_events(") && !store.contains("async fn insert_events("),
        "the batch insert runs SQLite and must be synchronous"
    );
    let calls: Vec<usize> = writer
        .match_indices(".insert_events(")
        .map(|(at, _)| at)
        .collect();
    assert!(!calls.is_empty(), "the event writer inserts its batches");
    for at in calls {
        let before = &writer[..at];
        let blocking = before
            .rfind("spawn_blocking(")
            .expect("the batch insert runs inside spawn_blocking");
        assert!(
            !before[blocking..].contains("fn "),
            "the batch insert at byte {at} is outside the spawn_blocking closure"
        );
    }
}
