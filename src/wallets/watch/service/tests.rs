use super::*;
use crate::wallets::watch::runtime::test_support::FakeRuntime;
use chrono::Utc;

fn own_watch_target(address: &str) -> WatchTarget {
    WatchTarget {
        id: None,
        address: address.to_owned(),
        label: None,
        sources: vec![WatchSource::OwnWallet],
        enabled: true,
        page_budget: poller::DEFAULT_PAGE_BUDGET,
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
        disable_reason: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

fn idle_target_runtime(target: WatchTarget) -> TargetRuntime {
    TargetRuntime {
        target,
        ws_task: tokio::spawn(async {}),
        last_poll: Instant::now(),
        catch_up: None,
        baseline_only: false,
        backfill: false,
    }
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
    )
    .await;

    assert_eq!(outcome, ProcessOutcome::Terminal);
}

#[tokio::test]
async fn the_first_incomplete_range_pauses_without_advancing_the_cursor() {
    let address = "BusyTarget1111";
    let fake = FakeRuntime::new(vec![address.to_owned()]);
    let full = |prefix: &str| -> Vec<String> {
        (0..poller::PAGE_SIZE)
            .map(|n| format!("{prefix}-{n:03}"))
            .collect()
    };
    for page in 0..poller::MAX_PAGES {
        fake.queue_page(address, full(&format!("first-{page}")));
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
    poll_target(
        &mut target_runtime,
        &chain_runtime,
        &watch_db,
        own.clone(),
        false,
    )
    .await;
    assert_eq!(
        watch_db.get_cursor(address).await.unwrap().as_deref(),
        Some("old-head")
    );
    assert!(target_runtime.catch_up.is_none());
    let paused = watch_db.get_target(id).await.unwrap().unwrap();
    assert!(
        !paused.enabled,
        "the first incomplete range pauses the target"
    );
    assert_eq!(
        paused.disable_reason,
        Some(crate::wallets::watch::WatchDisableReason::SignatureBudget {
            page_budget: poller::DEFAULT_PAGE_BUDGET,
            signatures_checked: poller::DEFAULT_PAGE_BUDGET * poller::PAGE_SIZE,
        })
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
    let (ws_tx, _ws_rx) = mpsc::unbounded_channel();
    let own = Subject::from_account(
        crate::chains::AccountId::new(crate::chains::ChainId::Solana, own_address).unwrap(),
    );
    let mut runtimes = HashMap::new();

    super::super::service_targets::reload_targets(
        &mut runtimes,
        &ws_tx,
        &chain_runtime,
        &watch_db,
        own.clone(),
        true,
    )
    .await;
    let own_forwarder = runtimes[own_address].ws_task.id();
    let stable_forwarder = runtimes[stable_address].ws_task.id();
    assert!(runtimes.contains_key(changed_address));

    watch_db
        .set_enabled(changed.id.unwrap(), false)
        .await
        .unwrap();
    super::super::service_targets::reload_targets(
        &mut runtimes,
        &ws_tx,
        &chain_runtime,
        &watch_db,
        own,
        true,
    )
    .await;

    assert_eq!(runtimes[own_address].ws_task.id(), own_forwarder);
    assert_eq!(runtimes[stable_address].ws_task.id(), stable_forwarder);
    assert!(!runtimes.contains_key(changed_address));
    for runtime in runtimes.values() {
        runtime.ws_task.abort();
    }
}

#[tokio::test]
async fn seven_targets_keep_independent_watch_budgets() {
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
        let expected_enabled = !is_busy || raised;
        assert_eq!(persisted.enabled, expected_enabled, "{}", persisted.address);
        assert_eq!(
            matches!(
                persisted.disable_reason,
                Some(crate::wallets::watch::WatchDisableReason::SignatureBudget { .. })
            ),
            is_busy && !raised,
            "{}",
            persisted.address
        );
        if !expected_enabled {
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
