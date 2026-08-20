//! Swap and action resilience — the guards that keep a trade from getting STUCK.
//!
//! Every test here exists because of a real stuck sell: a manual "sell all" that
//! reached `Executing swap via Jupiter` and then produced no further log line, no
//! error and no completion for as long as the app stayed up. The position sat in
//! "Selling", its slot permit stayed consumed, and the exit action never resolved.
//!
//! These are pure/local tests — no chain, no provider, no network egress. The HTTP
//! ones bind a loopback listener so the "server that never answers" case is exact
//! and deterministic rather than something we hope to observe against a real host.

mod common;

use std::time::{Duration, Instant};
use tokio::io::AsyncReadExt;
use tokio::net::TcpListener;

/// Accept connections and NEVER reply. Returns the bound address.
///
/// This is the failure mode that hung the sell: the socket connects fine (so a
/// connect timeout does not help) and then nothing comes back, forever.
async fn spawn_silent_server() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("addr").to_string();
    tokio::spawn(async move {
        loop {
            let Ok((mut socket, _)) = listener.accept().await else {
                return;
            };
            // Hold the connection open, read the request, answer nothing.
            tokio::spawn(async move {
                let mut buf = [0u8; 1024];
                while socket.read(&mut buf).await.unwrap_or(0) > 0 {}
                std::future::pending::<()>().await;
            });
        }
    });
    addr
}

/// The regression itself: `net::client()` had NO request timeout, so reqwest waited
/// forever on a server that accepted the connection and went silent. Any HTTP call in
/// the app could park its task permanently — including the Jupiter swap-build call
/// that sits between "we decided to sell" and "we have a signed transaction".
#[tokio::test]
async fn a_silent_server_times_out_instead_of_hanging_forever() {
    let addr = spawn_silent_server().await;
    let client = screenerbot::net::client();

    let started = Instant::now();
    let result = tokio::time::timeout(
        Duration::from_secs(70),
        client.get(format!("http://{addr}/")).send(),
    )
    .await;

    let inner = result.expect("request must not outlive the default timeout — it hung");
    assert!(
        inner.is_err(),
        "a server that never answers must produce an error, not a response"
    );
    assert!(
        inner.unwrap_err().is_timeout(),
        "the failure must be reported as a timeout"
    );
    assert!(
        started.elapsed() < Duration::from_secs(70),
        "took {:?}",
        started.elapsed()
    );
}

/// Swap execution sets a TIGHTER per-request timeout than the shared default, because
/// a quote that is slow is also stale. Per-request timeouts must win over the client
/// default — if they silently did not, the swap path would fall back to the 45s
/// default and a trade decision would act on a price a minute old.
#[tokio::test]
async fn a_per_request_timeout_overrides_the_client_default() {
    let addr = spawn_silent_server().await;
    let client = screenerbot::net::client();

    let started = Instant::now();
    let result = client
        .get(format!("http://{addr}/"))
        .timeout(Duration::from_secs(2))
        .send()
        .await;

    assert!(result.is_err(), "silent server must not yield a response");
    assert!(
        started.elapsed() < Duration::from_secs(20),
        "per-request timeout was ignored; waited {:?}",
        started.elapsed()
    );
}

/// A submitted-but-unconfirmed swap is recognised even when a router wraps the RPC
/// error in its own prose.
///
/// This decision is the difference between "hand the signature to verification" and
/// "send the sell again through the next router" — the second is a real double sell.
/// The old parser matched the FIRST "Transaction " in the message, so a wrapper that
/// began "Transaction send failed: ..." made it return a sentence fragment. That
/// fragment was truthy, so the no-retry guard held by luck, but it was then written
/// to `exit_transaction_signature` and enqueued for verification — an exit pinned to
/// a signature that does not exist on chain can never settle.
#[test]
fn a_wrapped_unconfirmed_swap_error_yields_the_real_signature_only() {
    use screenerbot::swaps::unconfirmed_swap_signature_from_message;

    const SIGNATURE: &str =
        "5VERv8NMvzbJMEkV8xnrLkEaWRtSz9CosKDYjCJjBRnbJLgp8uirBgmQpjKhoR4tjF3ZpRzrFmBV6UjKdiSZkQUW";

    assert_eq!(
        unconfirmed_swap_signature_from_message(&format!(
            "Transaction send failed: RPC error: Transaction {SIGNATURE} not confirmed within timeout"
        )),
        Some(SIGNATURE.to_owned()),
    );

    // A pre-submission failure must NOT look submitted, or a recoverable trade is
    // abandoned with a phantom exit signature.
    assert_eq!(
        unconfirmed_swap_signature_from_message("No swap route available for this token"),
        None,
    );

    // Prose can never masquerade as a signature.
    assert_eq!(
        unconfirmed_swap_signature_from_message(
            "Transaction send failed: not confirmed within timeout"
        ),
        None,
    );
}

/// Completed actions must actually be deletable.
///
/// `action_steps.action_id` is a FOREIGN KEY into `actions(id)` with no ON DELETE
/// CASCADE, and the cleanup deleted the PARENT first — so every run aborted with
/// "FOREIGN KEY constraint failed" and removed nothing. Both tables therefore grew
/// for the life of an install, and every trade adds rows to both.
#[tokio::test(flavor = "multi_thread")]
async fn completed_actions_are_deleted_together_with_their_steps() {
    use screenerbot::actions::{Action, ActionState, ActionType, ActionsDatabase};

    let _dir = common::isolated_env();
    screenerbot::paths::ensure_all_directories().expect("create isolated data dirs");
    common::configure_own_wallet();

    let db = ActionsDatabase::new().await.expect("actions db");

    let action = Action::new(
        "stuck-sell-cleanup".to_owned(),
        ActionType::SwapSell,
        "6p6xgHyF7AeE6TZkSmFsko444wqoP15icUSqi2jfGiPN".to_owned(),
        vec![
            "Validating".to_owned(),
            "Getting Quote".to_owned(),
            "Executing Swap".to_owned(),
            "Verifying".to_owned(),
        ],
        serde_json::json!({ "symbol": "TRUMP" }),
    );
    db.insert_action(&action).await.expect("insert action");

    let completed_at = chrono::Utc::now();
    db.update_action_state(
        &action.id,
        &ActionState::Completed,
        Some(completed_at),
        action.started_at,
    )
    .await
    .expect("complete action");

    // A negative retention puts the cutoff in the future, so every completed action
    // qualifies — the point under test is that the delete SUCCEEDS, not the cutoff math.
    let deleted = db
        .cleanup_old_actions(-1)
        .await
        .expect("cleanup must not fail on the steps foreign key");

    assert_eq!(deleted, 1, "the completed action must be deleted");
    assert!(
        db.get_action(&action.id)
            .await
            .expect("read back")
            .is_none(),
        "the action must be gone after cleanup"
    );
}
