//! The dashboard launch contract: what has to hold for a launched window to stop
//! showing its loading state.
//!
//! The regression this file exists for: the app opened, navigated to the
//! dashboard, and stayed in its skeleton forever. Nothing was wrong with the
//! dashboard. `tokens_new` failed to initialize while migrating the token
//! database onto the chain-scoped schema, the process exited, and the only
//! thing gating the loader — `GET /api/system/bootstrap` reporting `ui_ready` —
//! could therefore never flip.
//!
//! So the launch graph covered here is exactly the one a stuck window implicates:
//!
//! * the token database opens (a pre-chain-identity file included) — failing it
//!   kills the process that serves everything below;
//! * `/api/system/bootstrap` answers a parseable, stable contract in every
//!   readiness state, quickly, and without a service manager;
//! * that contract carries a terminal decision — `ui_ready` flips exactly when
//!   the frontend prerequisites are up, and `retry_after_ms` is present only
//!   while the loader still has to poll;
//! * an uninitialized wallet database is reported as absent, never as a failure;
//! * the webserver stays usable while core services are still starting.
//!
//! These are pure/local: no network, no wallet, isolated temp storage.

mod common;

use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};
use std::time::{Duration, Instant};

use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use rusqlite::Connection;
use screenerbot::chains::ChainId;
use screenerbot::global::{
    CONNECTIVITY_SYSTEM_READY, INITIALIZATION_COMPLETE, POOL_SERVICE_READY, POSITIONS_SYSTEM_READY,
    TOKENS_SYSTEM_READY, TRANSACTIONS_SYSTEM_READY,
};
use screenerbot::tokens::schema::CREATE_TABLES;
use screenerbot::tokens::TokenDatabase;
use screenerbot::webserver::routes;
use screenerbot::webserver::state::AppState;
use serde_json::Value;
use tower::ServiceExt;

/// A launch fetch the loader blocks on may not cost seconds. The loader polls at
/// 750ms and the dashboard's own request timeout is 10s, so anything near a
/// second here is already a boot defect — and a handler that waits on a service
/// or a synchronous database read lands orders of magnitude above it.
const LAUNCH_FETCH_BUDGET: Duration = Duration::from_secs(2);

/// The readiness flags are process-wide statics, and one test binary runs its
/// tests concurrently. Every test that reads or writes them takes this lock and
/// restores the flags before releasing it.
fn readiness_lock() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Restores every readiness flag it captured, whatever the test did or panicked on.
struct ReadinessFlags {
    _guard: MutexGuard<'static, ()>,
    saved: [(&'static std::sync::atomic::AtomicBool, bool); 6],
}

impl ReadinessFlags {
    fn capture() -> Self {
        let guard = readiness_lock();
        let flags: [&'static std::sync::atomic::AtomicBool; 6] = [
            &INITIALIZATION_COMPLETE,
            &CONNECTIVITY_SYSTEM_READY,
            &TOKENS_SYSTEM_READY,
            &POSITIONS_SYSTEM_READY,
            &POOL_SERVICE_READY,
            &TRANSACTIONS_SYSTEM_READY,
        ];
        let saved = flags.map(|flag| (flag, flag.load(Ordering::SeqCst)));
        Self {
            _guard: guard,
            saved,
        }
    }

    /// Full mode (setup complete) with nothing else started — the state a normal
    /// launch is in for as long as the services take to come up.
    fn full_mode_still_starting(&self) {
        INITIALIZATION_COMPLETE.store(true, Ordering::SeqCst);
        for flag in [
            &CONNECTIVITY_SYSTEM_READY,
            &TOKENS_SYSTEM_READY,
            &POSITIONS_SYSTEM_READY,
            &POOL_SERVICE_READY,
            &TRANSACTIONS_SYSTEM_READY,
        ] {
            flag.store(false, Ordering::SeqCst);
        }
    }
}

impl Drop for ReadinessFlags {
    fn drop(&mut self) {
        for (flag, value) in self.saved {
            flag.store(value, Ordering::SeqCst);
        }
    }
}

/// Issue one request against the REAL router (every middleware included) and
/// return the status, the decoded JSON body and how long it took.
async fn call_api(path: &str) -> (StatusCode, Value, Duration) {
    let state = Arc::new(AppState::new());
    let router = routes::create_router(state);
    let request = Request::builder()
        .uri(path)
        .body(Body::empty())
        .expect("build request");

    let started = Instant::now();
    let response = router.oneshot(request).await.expect("router responded");
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 4 * 1024 * 1024)
        .await
        .expect("read response body");
    let elapsed = started.elapsed();

    let body = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap_or_else(|error| {
            panic!(
                "{path} did not answer parseable JSON ({error}): {}",
                String::from_utf8_lossy(&bytes)
            )
        })
    };
    (status, body, elapsed)
}

/// Every field `scripts/core/bootstrap.js` reads to decide whether the loader may
/// stop. A rename on either side is what makes a working backend look stuck, so
/// they are asserted by name and by type.
fn assert_loader_contract(body: &Value) {
    for (field, kind) in [
        ("initialization_required", "bool"),
        ("initialization_complete", "bool"),
        ("explore_mode", "bool"),
        ("onboarding_complete", "bool"),
        ("core_services_ready", "bool"),
        ("ui_ready", "bool"),
        ("ready_for_requests", "bool"),
        ("wallet_snapshot_ready", "bool"),
    ] {
        assert!(
            body.get(field).is_some_and(Value::is_boolean),
            "/api/system/bootstrap no longer answers a {kind} `{field}` — the dashboard \
             loader reads it to decide whether to stop polling"
        );
    }
    assert!(
        body.get("phase").is_some_and(Value::is_string),
        "/api/system/bootstrap no longer reports a `phase` string"
    );
    assert!(
        body.get("message").is_some_and(Value::is_string),
        "/api/system/bootstrap no longer reports a `message` string"
    );
    assert!(
        body.get("pending_services").is_some_and(Value::is_array),
        "/api/system/bootstrap no longer reports `pending_services`"
    );
    assert!(
        body.get("boot_progress").is_some_and(Value::is_array),
        "/api/system/bootstrap no longer reports `boot_progress`"
    );
}

/// The one poll that decides whether the window keeps its loading state must
/// answer, fast, in the state a launch actually starts in: setup complete, no
/// service ready, no service manager registered in this process.
#[tokio::test]
async fn bootstrap_answers_a_stable_contract_while_services_are_still_starting() {
    let _dir = common::isolated_env();
    let flags = ReadinessFlags::capture();
    flags.full_mode_still_starting();

    let (status, body, elapsed) = call_api("/api/system/bootstrap").await;

    assert_eq!(
        status,
        StatusCode::OK,
        "the launch poll must answer 200 while services are still starting, got {status}"
    );
    assert!(
        elapsed < LAUNCH_FETCH_BUDGET,
        "the launch poll took {elapsed:?}, over the {LAUNCH_FETCH_BUDGET:?} budget — it is \
         waiting on a service or a synchronous database read instead of reporting state"
    );
    assert_loader_contract(&body);

    assert_eq!(
        body["initialization_required"], false,
        "setup is complete, so the loader must not be sent to the setup screen"
    );
    assert_eq!(
        body["ui_ready"], false,
        "no frontend prerequisite is ready, so the loader must be told to keep waiting"
    );
    assert_eq!(body["phase"], "ui_startup");
    assert!(
        body.get("retry_after_ms")
            .is_some_and(|value| value.as_u64().is_some_and(|ms| ms > 0)),
        "a non-ready answer must carry a positive retry_after_ms; without it the loader has \
         no cadence to poll on"
    );
}

/// The terminal transition. `ui_ready` is the ONLY thing that lets the dashboard
/// leave its loading state in full mode, and it must flip on exactly the
/// frontend prerequisites — connectivity, tokens and pools — not on the whole
/// trading core, which starts later.
#[tokio::test]
async fn bootstrap_turns_ui_ready_on_the_frontend_prerequisites_alone() {
    let _dir = common::isolated_env();
    let flags = ReadinessFlags::capture();
    flags.full_mode_still_starting();

    // One prerequisite short: still not ready.
    CONNECTIVITY_SYSTEM_READY.store(true, Ordering::SeqCst);
    TOKENS_SYSTEM_READY.store(true, Ordering::SeqCst);
    let (_, body, _) = call_api("/api/system/bootstrap").await;
    assert_eq!(
        body["ui_ready"], false,
        "pools is a frontend prerequisite in full mode and it is not ready"
    );

    // All three up: the loader must be released even though positions and
    // transactions — the trading core — are still starting.
    POOL_SERVICE_READY.store(true, Ordering::SeqCst);
    let (status, body, elapsed) = call_api("/api/system/bootstrap").await;

    assert_eq!(status, StatusCode::OK);
    assert!(
        elapsed < LAUNCH_FETCH_BUDGET,
        "the ready poll took {elapsed:?}"
    );
    assert_eq!(
        body["ui_ready"], true,
        "connectivity, tokens and pools are up — the dashboard is usable and the loader must \
         terminate; waiting for the trading core here is what keeps a window in its skeleton"
    );
    assert_eq!(
        body["core_services_ready"], false,
        "positions and transactions are still starting, so the trading core is not ready — \
         `ui_ready` and `core_services_ready` must stay separate answers"
    );
    assert!(
        body.get("retry_after_ms").is_none_or(Value::is_null),
        "a ready answer must not ask the loader to poll again"
    );
    assert_eq!(body["phase"], "service_startup");
}

/// The launch poll reads the cached wallet snapshot, and on a cold process the
/// wallet database has not been initialized yet — that is ordinary startup
/// ordering, not a failure. It must be reported as `wallet_snapshot_ready:
/// false` with the rest of the contract intact, never as an error status that
/// the loader would treat as an unreachable backend.
#[tokio::test]
async fn an_uninitialized_wallet_database_is_reported_absent_not_fatal() {
    let _dir = common::isolated_env();
    let flags = ReadinessFlags::capture();
    flags.full_mode_still_starting();

    let (status, body, elapsed) = call_api("/api/system/bootstrap").await;

    assert_eq!(
        status,
        StatusCode::OK,
        "no wallet database has been initialized in this process; the launch poll must still \
         answer 200"
    );
    assert!(
        elapsed < LAUNCH_FETCH_BUDGET,
        "the launch poll took {elapsed:?}"
    );
    assert_loader_contract(&body);
    assert_eq!(
        body["wallet_snapshot_ready"], false,
        "an uninitialized wallet database is an absent snapshot"
    );
    assert!(
        body.get("wallet_last_updated").is_none_or(Value::is_null),
        "there is no snapshot, so there is no timestamp to report"
    );
}

/// The webserver comes up before the services it reports on and must stay usable
/// while they are starting — that separation is what lets the dashboard render a
/// starting state instead of failing to load at all.
#[tokio::test]
async fn the_webserver_serves_the_dashboard_while_core_services_are_unready() {
    let _dir = common::isolated_env();
    let flags = ReadinessFlags::capture();
    flags.full_mode_still_starting();

    let (status, body, elapsed) = call_api("/api/health").await;

    assert_eq!(
        status,
        StatusCode::OK,
        "the liveness probe must answer while services are still starting — Electron treats a \
         failing probe as a dead backend"
    );
    assert!(
        elapsed < LAUNCH_FETCH_BUDGET,
        "the liveness probe took {elapsed:?}"
    );
    assert_eq!(body["status"], "ok");
}

/// The launch dependency that actually broke: `tokens_new` opens the token
/// database first, and a database written before chain identity is rebuilt onto
/// the chain-scoped schema at that moment. `token_pools` is a foreign-key child
/// of `tokens` with `ON DELETE CASCADE`, so the rebuild's teardown could empty
/// the staged copy it was about to verify — the migration aborted, the service
/// failed to initialize, the process exited, and the already-loaded dashboard
/// waited on a backend that was gone.
///
/// This is the smallest fixture that reproduces it end to end: a legacy database
/// with a parent row and a cascade child, opened exactly the way the service
/// opens it.
#[tokio::test]
async fn a_legacy_token_database_opens_instead_of_killing_the_launch() {
    let _dir = common::isolated_env();
    let fixture = tempfile::tempdir().expect("create fixture directory");
    let db_path = fixture.path().join("tokens.db");
    let db_path_str = db_path.to_string_lossy().to_string();

    {
        let conn = Connection::open(&db_path).expect("create legacy fixture");
        for statement in CREATE_TABLES {
            conn.execute(&legacy_statement(statement), [])
                .expect("create pre-chain-identity table");
        }
        conn.execute(
            "INSERT INTO tokens (mint, symbol, name, decimals, first_discovered_at, \
             metadata_last_fetched_at, decimals_last_fetched_at) \
             VALUES ('LEGACY_MINT', 'OLD', 'Legacy', 9, 1, 1, 1)",
            [],
        )
        .expect("seed the foreign-key parent");
        conn.execute(
            "INSERT INTO token_pools (mint, pool_address, base_mint, quote_mint, is_sol_pair, \
             pool_data_last_fetched_at, pool_data_first_seen_at) \
             VALUES ('LEGACY_MINT', 'POOL', 'LEGACY_MINT', \
             'So11111111111111111111111111111111111111112', 1, 1, 1)",
            [],
        )
        .expect("seed the ON DELETE CASCADE child");
        conn.pragma_update(None, "user_version", 1)
            .expect("stamp the legacy schema version");
    }

    let opened = TokenDatabase::new(&db_path_str, ChainId::Solana);
    let database = opened.unwrap_or_else(|error| {
        panic!(
            "opening a pre-chain-identity token database failed: {error}. The tokens service \
             initializes with this call, so the whole process dies here and a dashboard that \
             already loaded waits on a backend that is gone."
        )
    });
    drop(database);

    let conn = Connection::open(&db_path).expect("reopen the migrated database");
    let pools: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM token_pools WHERE chain_id = 'solana' AND mint = 'LEGACY_MINT'",
            [],
            |row| row.get(0),
        )
        .expect("count migrated pool rows");
    assert_eq!(
        pools, 1,
        "the cascade child lost its rows during the rebuild — the migration's own row-count \
         verification is what then fails the launch"
    );
}

/// Strip chain identity out of a live `CREATE TABLE` statement to produce the
/// pre-chain-identity shape, including the single-column foreign key the rebuild
/// has to carry across.
fn legacy_statement(statement: &str) -> String {
    statement
        .replace("chain_id TEXT NOT NULL DEFAULT 'solana',", "")
        .replace(
            "PRIMARY KEY (chain_id, mint, pool_address)",
            "PRIMARY KEY (mint, pool_address)",
        )
        .replace("PRIMARY KEY (chain_id, mint)", "PRIMARY KEY (mint)")
        .replace("UNIQUE (chain_id, mint)", "UNIQUE (mint)")
        .replace(
            "PRIMARY KEY (chain_id, bucket_hour, reason, source)",
            "PRIMARY KEY (bucket_hour, reason, source)",
        )
        .replace("PRIMARY KEY (chain_id, address)", "PRIMARY KEY (address)")
        .replace(
            "FOREIGN KEY (chain_id, mint) REFERENCES tokens(chain_id, mint)",
            "FOREIGN KEY (mint) REFERENCES tokens(mint)",
        )
}
