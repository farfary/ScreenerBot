// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Wallet watch service tests - cursor durability, baseline establishment and target reload isolation.

use super::*;
use crate::wallets::watch::runtime::test_support::FakeRuntime;
use crate::wallets::watch::WatchDisableReason;
use chrono::Utc;

#[test]
fn high_activity_failures_preserve_their_source() {
    assert_eq!(
        HighActivityPollResult::ProviderFailed.pause_reason(),
        WatchDisableReason::HeliusUnavailable
    );
    assert_eq!(
        HighActivityPollResult::ProcessingFailed.pause_reason(),
        WatchDisableReason::ProcessingFailed
    );
}

fn own_watch_target(address: &str) -> WatchTarget {
    WatchTarget {
        id: None,
        address: address.to_owned(),
        label: None,
        sources: vec![WatchSource::OwnWallet],
        enabled: true,
        page_budget: poller::DEFAULT_PAGE_BUDGET,
        high_activity_approved: false,
        disable_reason: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

fn alert_watch_target(address: &str, rule_id: i64) -> WatchTarget {
    WatchTarget {
        id: Some(rule_id),
        address: address.to_owned(),
        label: None,
        sources: vec![WatchSource::Alert { rule_id }],
        enabled: true,
        page_budget: poller::DEFAULT_PAGE_BUDGET,
        high_activity_approved: false,
        disable_reason: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

fn idle_target_runtime(target: WatchTarget) -> TargetRuntime {
    TargetRuntime {
        target,
        last_poll: Instant::now(),
        catch_up: None,
        range_open: false,
        stream_mode: crate::wallets::watch::StreamMode::References,
        baseline_only: false,
        backfill: false,
        high_activity: false,
        high_activity_catching_up: false,
        high_activity_dirty: false,
        high_activity_failures: 0,
        high_activity_failure_reason: None,
    }
}

#[test]
fn high_activity_ws_burst_respects_minimum_interval_and_idle_safety_poll() {
    assert!(!high_activity_poll_due(Duration::from_secs(4), true, 5, 30));
    assert!(high_activity_poll_due(Duration::from_secs(5), true, 5, 30));
    assert!(!high_activity_poll_due(
        Duration::from_secs(29),
        false,
        5,
        30
    ));
    assert!(high_activity_poll_due(
        Duration::from_secs(30),
        false,
        5,
        30
    ));
}

#[tokio::test]
async fn raw_overflow_switches_to_helius_and_preserves_the_prior_cursor() {
    use crate::wallets::watch::{
        SignaturePageItem, SuccessfulTransactionPageItem, SuccessfulTransactionsPage,
    };
    let address = "HighModeTarget1111";
    let runtime = FakeRuntime::new(vec![address.to_owned()]);
    runtime.set_high_activity_supported(true);
    for page in 0..poller::DEFAULT_PAGE_BUDGET {
        runtime.queue_page_items(
            address,
            (0..poller::PAGE_SIZE)
                .map(|item| SignaturePageItem {
                    signature: format!("raw-{page}-{item}"),
                    failed: false,
                    ..SignaturePageItem::default()
                })
                .collect(),
        );
    }
    runtime.queue_successful_page(
        address,
        SuccessfulTransactionsPage {
            items: vec![SuccessfulTransactionPageItem {
                signature: "helius-next".to_owned(),
                transaction: Ok(None),
            }],
            has_more: false,
        },
    );
    let chain_runtime: Arc<dyn WalletWatchRuntime> = runtime;
    let (watch_db, _dir) = temp_watch_db();
    watch_db
        .set_cursor(address, "durable-before")
        .await
        .unwrap();
    let own = Subject::from_account(
        crate::chains::AccountId::new(crate::chains::ChainId::Solana, "OwnWallet1111").unwrap(),
    );
    let mut target = idle_target_runtime(alert_watch_target(address, 9));
    target.target.high_activity_approved = true;
    poll_target(&mut target, &chain_runtime, &watch_db, own, false).await;
    assert!(target.high_activity);
    assert_eq!(
        watch_db.get_cursor(address).await.unwrap().as_deref(),
        Some("helius-next")
    );
}

#[tokio::test]
async fn helius_bad_middle_item_commits_only_the_prior_item() {
    use crate::wallets::watch::{SuccessfulTransactionPageItem, SuccessfulTransactionsPage};
    let address = "HighMiddleTarget1111";
    let runtime = FakeRuntime::new(vec![address.to_owned()]);
    runtime.set_high_activity_supported(true);
    runtime.queue_successful_page(
        address,
        SuccessfulTransactionsPage {
            items: vec![
                SuccessfulTransactionPageItem {
                    signature: "first".to_owned(),
                    transaction: Ok(None),
                },
                SuccessfulTransactionPageItem {
                    signature: "bad".to_owned(),
                    transaction: Err(crate::wallets::Error::ChainRuntime {
                        operation: "test",
                        detail: "bad".to_owned(),
                    }),
                },
                SuccessfulTransactionPageItem {
                    signature: "later".to_owned(),
                    transaction: Ok(None),
                },
            ],
            has_more: false,
        },
    );
    let chain_runtime: Arc<dyn WalletWatchRuntime> = runtime;
    let (watch_db, _dir) = temp_watch_db();
    let own = Subject::from_account(
        crate::chains::AccountId::new(crate::chains::ChainId::Solana, "OwnWallet1111").unwrap(),
    );
    let mut target = idle_target_runtime(alert_watch_target(address, 10));
    target.target.high_activity_approved = true;
    target.high_activity = true;
    poll_target(&mut target, &chain_runtime, &watch_db, own, false).await;
    assert_eq!(
        watch_db.get_cursor(address).await.unwrap().as_deref(),
        Some("first")
    );
}

fn temp_watch_db() -> (WatchDatabase, tempfile::TempDir) {
    let dir = tempfile::tempdir().expect("temp dir");
    let db = WatchDatabase::new_with_path(
        dir.path().join("wallets.db"),
        crate::chains::ChainId::Solana,
    )
    .expect("create watch database");
    (db, dir)
}

#[tokio::test]
async fn poll_target_rejects_an_invalid_address_before_any_observation_starts() {
    // Nothing is registered as valid on this fake runtime -- every address is
    // rejected at `resolve_subject`, mirroring an adapter boundary rejection
    // for a wrong-chain or malformed target.
    let chain_runtime: Arc<dyn WalletWatchRuntime> = FakeRuntime::new(vec![]);
    let (watch_db, _dir) = temp_watch_db();
    let own = Subject::from_account(
        crate::chains::AccountId::new(crate::chains::ChainId::Solana, "OwnWallet1111").unwrap(),
    );

    let mut target_runtime = idle_target_runtime(own_watch_target("NotAValidTarget1111"));
    poll_target(&mut target_runtime, &chain_runtime, &watch_db, own, false).await;

    assert!(
        target_runtime.catch_up.is_none(),
        "an invalid target must never enter catch-up state"
    );
    assert_eq!(
        watch_db.get_cursor("NotAValidTarget1111").await.unwrap(),
        None,
        "an invalid target must never get a cursor row"
    );
}

#[tokio::test]
async fn first_observation_establishes_a_bounded_baseline_without_replaying_history() {
    let address = "FreshTarget1111";
    let chain_runtime = FakeRuntime::new(vec![address.to_owned()]);
    // A short (< PAGE_SIZE) page proves the range complete on the first call.
    chain_runtime.queue_page(
        address,
        vec!["newest-sig".to_owned(), "older-sig".to_owned()],
    );
    let chain_runtime: Arc<dyn WalletWatchRuntime> = chain_runtime;

    let (watch_db, _dir) = temp_watch_db();
    let own = Subject::from_account(
        crate::chains::AccountId::new(crate::chains::ChainId::Solana, "OwnWallet1111").unwrap(),
    );

    // No cursor row yet and not the own wallet -- this is the baseline-only path.
    let mut target_runtime = idle_target_runtime(alert_watch_target(address, 1));
    poll_target(&mut target_runtime, &chain_runtime, &watch_db, own, false).await;

    assert_eq!(
        watch_db.get_cursor(address).await.unwrap().as_deref(),
        Some("newest-sig"),
        "baseline must adopt the newest signature as the cursor"
    );
    assert!(
        target_runtime.catch_up.is_none() && !target_runtime.baseline_only,
        "baseline establishment must clear catch-up state without processing anything"
    );
}

#[tokio::test]
async fn a_processing_failure_never_advances_the_durable_cursor() {
    let address = "EscalatedTarget1111";
    let chain_runtime = FakeRuntime::new(vec![address.to_owned()]);
    chain_runtime.queue_page(address, vec!["pending-sig".to_owned()]);
    let chain_runtime: Arc<dyn WalletWatchRuntime> = chain_runtime;

    let (watch_db, _dir) = temp_watch_db();
    // A cursor row already exists (an established target), so this is NOT the
    // baseline path -- the queued signature goes through `process_signature`.
    watch_db.mark_cursor_initialized(address).await.unwrap();
    let own = Subject::from_account(
        crate::chains::AccountId::new(crate::chains::ChainId::Solana, "OwnWallet1111").unwrap(),
    );

    let mut target_runtime = idle_target_runtime(alert_watch_target(address, 2));
    poll_target(&mut target_runtime, &chain_runtime, &watch_db, own, false).await;

    // No global transaction database is installed in this unit test, so dedupe
    // admission fails and `process_signature` returns `Retryable` -- exactly the
    // path a real transient failure takes. The cursor must stay put either way.
    assert_eq!(
        watch_db.get_cursor(address).await.unwrap(),
        None,
        "a retryable processing outcome must not advance the cursor"
    );
    assert!(
        target_runtime.catch_up.is_some(),
        "an incomplete replay must keep its catch-up state for the next tick"
    );
}

#[tokio::test]
async fn a_range_resumed_after_a_failed_poll_is_backfill() {
    let address = "OutageTarget1111";
    let fake_runtime = FakeRuntime::new(vec![address.to_owned()]);
    fake_runtime.fail_fetches(address, 1);
    fake_runtime.queue_page(address, vec!["late-sig".to_owned()]);
    let chain_runtime: Arc<dyn WalletWatchRuntime> = fake_runtime;
    let (watch_db, _dir) = temp_watch_db();
    watch_db.mark_cursor_initialized(address).await.unwrap();
    let own = Subject::from_account(
        crate::chains::AccountId::new(crate::chains::ChainId::Solana, "OwnWallet1111").unwrap(),
    );

    let mut target_runtime = idle_target_runtime(alert_watch_target(address, 4));
    poll_target(
        &mut target_runtime,
        &chain_runtime,
        &watch_db,
        own.clone(),
        false,
    )
    .await;
    assert!(
        target_runtime.backfill,
        "a cadence poll that failed leaves its range to be resumed as history"
    );

    // The next cadence tick resumes the same range and keeps it a backfill.
    poll_target(&mut target_runtime, &chain_runtime, &watch_db, own, false).await;
    assert!(target_runtime.backfill);
    assert!(target_runtime.catch_up.is_some());
}

#[tokio::test]
async fn failed_external_page_advances_raw_cursor_without_decoding() {
    use crate::wallets::watch::SignaturePageItem;

    let address = "FailedExternal1111";
    let chain_runtime = FakeRuntime::new(vec![address.to_owned()]);
    chain_runtime.queue_page_items(
        address,
        vec![SignaturePageItem {
            signature: "failed-sig".to_owned(),
            failed: true,
            ..SignaturePageItem::default()
        }],
    );
    let fake_runtime = chain_runtime.clone();
    let chain_runtime: Arc<dyn WalletWatchRuntime> = chain_runtime;
    let (watch_db, _dir) = temp_watch_db();
    watch_db.mark_cursor_initialized(address).await.unwrap();
    let own = Subject::from_account(
        crate::chains::AccountId::new(crate::chains::ChainId::Solana, "OwnWallet1111").unwrap(),
    );

    let mut target_runtime = idle_target_runtime(alert_watch_target(address, 3));
    poll_target(&mut target_runtime, &chain_runtime, &watch_db, own, false).await;

    assert_eq!(
        watch_db.get_cursor(address).await.unwrap().as_deref(),
        Some("failed-sig")
    );
    assert!(fake_runtime.calls.lock().unwrap().decoded.is_empty());
}

#[test]
fn own_wallet_failed_items_retain_the_decode_path() {
    let target = own_watch_target("OwnFailed1111");
    assert!(!skip_known_failed_external(&target, true));
}

#[test]
fn failed_external_items_skip_but_successful_items_replay() {
    let target = alert_watch_target("External1111", 4);
    assert!(skip_known_failed_external(&target, true));
    assert!(!skip_known_failed_external(&target, false));
}

#[tokio::test]
async fn process_signature_resolves_the_exact_target_identity_before_dedupe() {
    let address = "IdentityTarget1111";
    let chain_runtime = FakeRuntime::new(vec![address.to_owned()]);

    let outcome = process_signature(
        &(Arc::clone(&chain_runtime) as Arc<dyn WalletWatchRuntime>),
        &own_watch_target(address),
        Subject::from_account(
            crate::chains::AccountId::new(crate::chains::ChainId::Solana, address).unwrap(),
        ),
        "some-signature",
        Utc::now(),
        false,
        None,
    )
    .await;

    assert_eq!(
        chain_runtime.calls.lock().unwrap().resolved,
        vec![address.to_owned()],
        "the funnel must resolve the exact address the target carries"
    );
    // No global transaction database in this unit test -- dedupe admission
    // fails closed (retryable), never panics, and never reaches decode.
    assert_eq!(outcome, ProcessOutcome::Retryable);
    assert!(chain_runtime.calls.lock().unwrap().decoded.is_empty());
}

#[tokio::test]
async fn process_signature_rejects_a_wrong_chain_target_before_any_call() {
    let chain_runtime: Arc<dyn WalletWatchRuntime> = FakeRuntime::new(vec![]);

    let outcome = process_signature(
        &chain_runtime,
        &own_watch_target("WrongChainTarget1111"),
        Subject::from_account(
            crate::chains::AccountId::new(crate::chains::ChainId::Solana, "WrongChainTarget1111")
                .unwrap(),
        ),
        "some-signature",
        Utc::now(),
        false,
        None,
    )
    .await;

    assert_eq!(outcome, ProcessOutcome::Terminal);
}

#[tokio::test]
async fn an_unfinished_range_stays_open_and_completes_on_the_next_poll() {
    use crate::wallets::watch::SignaturePageItem;
    let address = "BusyTarget1111";
    let fake = FakeRuntime::new(vec![address.to_owned()]);
    // Other wallets' failed transactions, as a busy address is referenced by.
    let failed = |prefix: &str, count: usize| -> Vec<SignaturePageItem> {
        (0..count)
            .map(|n| SignaturePageItem {
                signature: format!("{prefix}-{n:04}"),
                failed: true,
                ..SignaturePageItem::default()
            })
            .collect()
    };
    for page in 0..poller::MAX_PAGES {
        fake.queue_page_items(address, failed(&format!("first-{page}"), poller::PAGE_SIZE));
    }
    fake.queue_page_items(address, failed("tail", 1));
    let chain_runtime: Arc<dyn WalletWatchRuntime> = fake.clone();

    let (watch_db, _dir) = temp_watch_db();
    let target = watch_db.insert_alert_target(address, None).await.unwrap();
    let id = target.id.expect("persisted target id");
    watch_db.set_cursor(address, "old-head").await.unwrap();
    let own = Subject::from_account(
        crate::chains::AccountId::new(crate::chains::ChainId::Solana, "OwnWallet1111").unwrap(),
    );

    let mut target_runtime = idle_target_runtime(target);
    poll_target(
        &mut target_runtime,
        &chain_runtime,
        &watch_db,
        own.clone(),
        false,
    )
    .await;
    assert!(target_runtime.range_open, "the next tick reads on");
    assert!(target_runtime.catch_up.is_some());
    assert_eq!(
        watch_db.get_cursor(address).await.unwrap().as_deref(),
        Some("old-head"),
        "an unfinished range never moves the durable cursor"
    );
    assert!(watch_db.get_target(id).await.unwrap().unwrap().enabled);

    poll_target(&mut target_runtime, &chain_runtime, &watch_db, own, false).await;
    assert!(!target_runtime.range_open);
    assert!(target_runtime.catch_up.is_none());
    assert_eq!(
        watch_db.get_cursor(address).await.unwrap().as_deref(),
        Some("first-0-0000")
    );
}

#[test]
fn only_a_sustained_flood_moves_a_target_to_its_own_changes() {
    use crate::wallets::watch::StreamMode::{OwnChanges, References};
    assert_eq!(
        stream_mode_for(References, 1_000, Some(6_000.0)),
        OwnChanges
    );
    assert_eq!(
        stream_mode_for(References, 20, Some(1_200.0)),
        References,
        "one burst of a quiet wallet stays on every reference"
    );
    assert_eq!(stream_mode_for(References, 1_000, Some(500.0)), References);
    assert_eq!(stream_mode_for(OwnChanges, 1_000, Some(500.0)), OwnChanges);
    assert_eq!(stream_mode_for(OwnChanges, 2, Some(4.0)), References);
    assert_eq!(stream_mode_for(OwnChanges, 1, None), OwnChanges);
}

#[tokio::test]
async fn quiet_references_are_skipped_and_the_range_ends_before_an_unsettled_one() {
    use crate::wallets::watch::{ReferenceVerdict, SignaturePageItem};
    let address = "GatedTarget1111";
    let fake = FakeRuntime::new(vec![address.to_owned()]);
    let item = |signature: &str, failed: bool, verdict: ReferenceVerdict| SignaturePageItem {
        signature: signature.to_owned(),
        failed,
        verdict,
        ..SignaturePageItem::default()
    };
    // Newest first, as a page arrives.
    fake.queue_page_items(
        address,
        vec![
            item("unsettled-new", false, ReferenceVerdict::Unsettled),
            item("quiet-mid", false, ReferenceVerdict::Quiet),
            item("failed-old", true, ReferenceVerdict::Unknown),
        ],
    );
    let chain_runtime: Arc<dyn WalletWatchRuntime> = fake.clone();
    let (watch_db, _dir) = temp_watch_db();
    let target = watch_db.insert_alert_target(address, None).await.unwrap();
    watch_db.set_cursor(address, "old-head").await.unwrap();
    let own = Subject::from_account(
        crate::chains::AccountId::new(crate::chains::ChainId::Solana, "OwnWallet1111").unwrap(),
    );

    let mut target_runtime = idle_target_runtime(target);
    poll_target(&mut target_runtime, &chain_runtime, &watch_db, own, false).await;

    assert!(
        fake.calls.lock().unwrap().decoded.is_empty(),
        "a quiet reference needs no transaction fetch"
    );
    assert_eq!(
        watch_db.get_cursor(address).await.unwrap().as_deref(),
        Some("quiet-mid"),
        "the unsettled reference is read again by the next poll"
    );
    assert!(target_runtime.catch_up.is_none());
}

#[tokio::test]
async fn a_flood_range_moves_its_target_to_own_changes_and_a_calm_one_back() {
    use crate::wallets::watch::{SignaturePageItem, StreamMode};
    let address = "RateTarget1111";
    let fake = FakeRuntime::new(vec![address.to_owned()]);
    let reference = |signature: String, block_time: i64| SignaturePageItem {
        signature,
        failed: true,
        block_time: Some(block_time),
        ..SignaturePageItem::default()
    };
    // 999 references inside ten seconds (a short page, so the range completes), then
    // two references a minute apart.
    fake.queue_page_items(
        address,
        (0..999)
            .map(|n| reference(format!("flood-{n}"), 10_000 - n as i64 / 100))
            .collect(),
    );
    fake.queue_page_items(
        address,
        vec![
            reference("calm-0".to_owned(), 20_060),
            reference("calm-1".to_owned(), 20_000),
        ],
    );
    let chain_runtime: Arc<dyn WalletWatchRuntime> = fake.clone();
    let (watch_db, _dir) = temp_watch_db();
    let target = watch_db.insert_alert_target(address, None).await.unwrap();
    watch_db.set_cursor(address, "old-head").await.unwrap();
    let own = Subject::from_account(
        crate::chains::AccountId::new(crate::chains::ChainId::Solana, "OwnWallet1111").unwrap(),
    );

    let mut target_runtime = idle_target_runtime(target);
    poll_target(
        &mut target_runtime,
        &chain_runtime,
        &watch_db,
        own.clone(),
        false,
    )
    .await;
    assert_eq!(target_runtime.stream_mode, StreamMode::OwnChanges);
    poll_target(&mut target_runtime, &chain_runtime, &watch_db, own, false).await;
    assert_eq!(target_runtime.stream_mode, StreamMode::References);
}

#[tokio::test]
async fn a_new_stream_mode_resubscribes_the_target_in_that_mode() {
    use crate::wallets::watch::StreamMode;
    let address = "ModeTarget1111";
    let fake = FakeRuntime::new(vec![address.to_owned()]);
    let chain_runtime: Arc<dyn WalletWatchRuntime> = fake.clone();
    let (events, _events_rx) = tokio::sync::mpsc::channel(4);
    let (mode, mode_rx) = tokio::sync::watch::channel(StreamMode::References);
    let cancel = Arc::new(tokio::sync::Notify::new());
    let active = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let forwarder = super::super::service_targets::spawn_ws_forwarder(
        address.to_owned(),
        false,
        events,
        chain_runtime,
        Arc::clone(&cancel),
        Arc::clone(&active),
        Arc::new(std::sync::atomic::AtomicBool::new(false)),
        mode_rx,
    );
    let subscriptions = |count: usize| {
        let fake = fake.clone();
        async move {
            while fake.calls.lock().unwrap().stream_modes.len() < count {
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        }
    };
    tokio::time::timeout(Duration::from_secs(5), subscriptions(1))
        .await
        .expect("first subscribe");
    mode.send_replace(StreamMode::OwnChanges);
    tokio::time::timeout(Duration::from_secs(5), subscriptions(2))
        .await
        .expect("resubscribe in the new mode");
    assert_eq!(
        fake.calls.lock().unwrap().stream_modes,
        [StreamMode::References, StreamMode::OwnChanges]
    );
    cancel.notify_waiters();
    forwarder.abort();
}

#[tokio::test]
async fn a_backlog_past_its_cap_pauses_without_advancing_the_cursor() {
    let address = "FloodTarget1111";
    let fake = FakeRuntime::new(vec![address.to_owned()]);
    let cap = poller::backlog_cap(poller::DEFAULT_PAGE_BUDGET);
    for page in 0..cap / poller::PAGE_SIZE {
        fake.queue_page(
            address,
            (0..poller::PAGE_SIZE)
                .map(|n| format!("flood-{page}-{n}"))
                .collect(),
        );
    }
    let chain_runtime: Arc<dyn WalletWatchRuntime> = fake.clone();

    let (watch_db, _dir) = temp_watch_db();
    let target = watch_db.insert_alert_target(address, None).await.unwrap();
    let id = target.id.expect("persisted target id");
    watch_db.set_cursor(address, "old-head").await.unwrap();
    let own = Subject::from_account(
        crate::chains::AccountId::new(crate::chains::ChainId::Solana, "OwnWallet1111").unwrap(),
    );

    let mut target_runtime = idle_target_runtime(target);
    for _ in 0..poller::BACKLOG_POLLS - 1 {
        poll_target(
            &mut target_runtime,
            &chain_runtime,
            &watch_db,
            own.clone(),
            false,
        )
        .await;
        assert!(
            target_runtime.target.enabled,
            "below the cap the range stays open"
        );
    }
    poll_target(&mut target_runtime, &chain_runtime, &watch_db, own, false).await;

    let paused = watch_db.get_target(id).await.unwrap().unwrap();
    assert!(!paused.enabled);
    assert_eq!(
        paused.disable_reason,
        Some(crate::wallets::watch::WatchDisableReason::SignatureBudget {
            page_budget: poller::DEFAULT_PAGE_BUDGET,
            signatures_checked: cap,
        })
    );
    assert_eq!(
        watch_db.get_cursor(address).await.unwrap().as_deref(),
        Some("old-head")
    );
}

#[tokio::test]
async fn changing_one_target_keeps_other_subscription_forwarders() {
    let (watch_db, _dir) = temp_watch_db();
    let own_address = "OwnReload1111";
    let stable_address = "StableReload1111";
    let changed_address = "ChangedReload1111";
    watch_db
        .insert_alert_target(stable_address, None)
        .await
        .unwrap();
    let changed = watch_db
        .insert_alert_target(changed_address, None)
        .await
        .unwrap();
    let fake = FakeRuntime::new(vec![
        own_address.to_owned(),
        stable_address.to_owned(),
        changed_address.to_owned(),
    ]);
    let chain_runtime: Arc<dyn WalletWatchRuntime> = fake;
    let own = Subject::from_account(
        crate::chains::AccountId::new(crate::chains::ChainId::Solana, own_address).unwrap(),
    );
    let mut runtimes = HashMap::new();

    super::super::service_targets::reload_targets(
        &mut runtimes,
        &chain_runtime,
        &watch_db,
        own.clone(),
        true,
    )
    .await;
    let own_forwarder = runtimes[own_address].forwarder_id();
    let stable_forwarder = runtimes[stable_address].forwarder_id();
    assert!(runtimes.contains_key(changed_address));

    watch_db
        .set_enabled(changed.id.unwrap(), false)
        .await
        .unwrap();
    super::super::service_targets::reload_targets(
        &mut runtimes,
        &chain_runtime,
        &watch_db,
        own,
        true,
    )
    .await;

    assert_eq!(runtimes[own_address].forwarder_id(), own_forwarder);
    assert_eq!(runtimes[stable_address].forwarder_id(), stable_forwarder);
    assert!(!runtimes.contains_key(changed_address));
    super::super::service_targets::stop_all(runtimes).await;
}

#[tokio::test]
async fn stalled_wallet_does_not_block_six_other_watchers_or_reload() {
    let addresses = [
        "BlockedTarget1111",
        "ReadyTargetTwo1111",
        "ReadyTargetThree1111",
        "ReadyTargetFour1111",
        "ReadyTargetFive1111",
        "ReadyTargetSix1111",
        "ReadyTargetSeven1111",
    ];
    let own_address = "OwnWorkerTest1111";
    let (watch_db, _dir) = temp_watch_db();
    let fake = FakeRuntime::new(
        std::iter::once(own_address.to_owned())
            .chain(addresses.iter().map(|address| (*address).to_owned()))
            .collect(),
    );
    let blocked = fake.block_fetch(addresses[0]);
    for (index, address) in addresses.iter().enumerate() {
        watch_db.insert_alert_target(address, None).await.unwrap();
        fake.queue_page(address, vec![format!("baseline-{index}")]);
    }
    let chain_runtime: Arc<dyn WalletWatchRuntime> = fake;
    let own = Subject::from_account(
        crate::chains::AccountId::new(crate::chains::ChainId::Solana, own_address).unwrap(),
    );
    let mut workers = HashMap::new();
    super::super::service_targets::reload_targets(
        &mut workers,
        &chain_runtime,
        &watch_db,
        own.clone(),
        true,
    )
    .await;

    tokio::time::timeout(Duration::from_secs(5), blocked.wait_started())
        .await
        .expect("busy wallet reaches the blocked RPC");
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let mut ready = true;
            for (index, address) in addresses.iter().enumerate().skip(1) {
                let expected = format!("baseline-{index}");
                ready &= watch_db.get_cursor(address).await.unwrap().as_deref()
                    == Some(expected.as_str());
            }
            if ready {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("six other wallets establish cursors while one RPC is blocked");
    assert_eq!(watch_db.get_cursor(addresses[0]).await.unwrap(), None);

    tokio::time::timeout(
        Duration::from_secs(5),
        super::super::service_targets::reload_targets(
            &mut workers,
            &chain_runtime,
            &watch_db,
            own,
            true,
        ),
    )
    .await
    .expect("target reload does not wait for the blocked wallet");
    blocked.release();
    super::super::service_targets::stop_all(workers).await;
}

#[tokio::test]
async fn seven_targets_read_their_ranges_independently() {
    let addresses = [
        "TargetOne1111",
        "TargetTwo1111",
        "TargetThree1111",
        "TargetFour1111",
        "TargetFive1111",
        "TargetSix1111",
        "TargetSeven1111",
    ];
    let busy = [false, true, false, true, false, true, false];
    let raised_budget = [false, false, false, true, false, false, false];
    let fake = FakeRuntime::new(
        addresses
            .iter()
            .map(|address| (*address).to_owned())
            .collect(),
    );
    let (watch_db, _dir) = temp_watch_db();
    let own = Subject::from_account(
        crate::chains::AccountId::new(crate::chains::ChainId::Solana, "OwnWallet1111").unwrap(),
    );
    let mut targets = Vec::new();

    for ((address, is_busy), raised) in addresses.into_iter().zip(busy).zip(raised_budget) {
        let inserted = watch_db.insert_alert_target(address, None).await.unwrap();
        if raised {
            watch_db
                .set_page_budget(inserted.id.unwrap(), poller::DEFAULT_PAGE_BUDGET + 1)
                .await
                .unwrap();
        }
        let target = watch_db
            .get_target(inserted.id.unwrap())
            .await
            .unwrap()
            .unwrap();
        watch_db.set_cursor(address, "old-head").await.unwrap();
        if is_busy {
            for page in 0..poller::DEFAULT_PAGE_BUDGET {
                fake.queue_page(
                    address,
                    (0..poller::PAGE_SIZE)
                        .map(|item| format!("{address}-{page}-{item}"))
                        .collect(),
                );
            }
            if raised {
                fake.queue_page(address, vec![format!("{address}-tail")]);
            }
        } else {
            fake.queue_page(address, Vec::new());
        }
        targets.push(idle_target_runtime(target));
    }

    let chain_runtime: Arc<dyn WalletWatchRuntime> = fake;
    for target in &mut targets {
        poll_target(target, &chain_runtime, &watch_db, own.clone(), false).await;
    }

    for ((target, is_busy), raised) in targets.iter().zip(busy).zip(raised_budget) {
        let id = target.target.id.unwrap();
        let persisted = watch_db.get_target(id).await.unwrap().unwrap();
        assert!(persisted.enabled, "{}", persisted.address);
        assert_eq!(
            target.range_open,
            is_busy && !raised,
            "only a busy target's own range stays open: {}",
            persisted.address
        );
        if target.range_open {
            assert_eq!(
                watch_db
                    .get_cursor(&persisted.address)
                    .await
                    .unwrap()
                    .as_deref(),
                Some("old-head")
            );
        }
    }
}
