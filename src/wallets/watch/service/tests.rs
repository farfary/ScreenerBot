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
        overflow_streak: 0,
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
    poll_target(&mut target_runtime, &chain_runtime, &watch_db, own).await;

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
    poll_target(&mut target_runtime, &chain_runtime, &watch_db, own).await;

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
    poll_target(&mut target_runtime, &chain_runtime, &watch_db, own).await;

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
    )
    .await;

    assert_eq!(outcome, ProcessOutcome::Terminal);
}

#[tokio::test]
async fn an_overflowing_target_skips_to_the_head_then_is_disabled_when_it_repeats() {
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
    poll_target(&mut target_runtime, &chain_runtime, &watch_db, own.clone()).await;
    assert_eq!(
        watch_db.get_cursor(address).await.unwrap().as_deref(),
        Some("first-0-000"),
        "the first overflow re-baselines to the newest signature seen"
    );
    assert!(target_runtime.catch_up.is_none());
    assert!(watch_db.get_target(id).await.unwrap().unwrap().enabled);

    for page in 0..poller::MAX_PAGES {
        fake.queue_page(address, full(&format!("second-{page}")));
    }
    poll_target(&mut target_runtime, &chain_runtime, &watch_db, own).await;
    assert!(
        !watch_db.get_target(id).await.unwrap().unwrap().enabled,
        "a second consecutive overflow disables the target"
    );
    assert!(super::service_state::saturation_reason(address).is_some());
    super::service_state::clear_saturation(address);
}
