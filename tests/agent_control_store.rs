//! Agent-control durable store: pairing credentials, the approval state
//! machine, crash recovery and audit bounds — exercised against a real
//! throwaway SQLite database.
//!
//! `store::POOL` is a process-global `LazyLock`. Every test goes through
//! `setup()`, whose one-time initializer points the pool at a temp file (via
//! `SCREENERBOT_AGENT_CONTROL_DB`) *before* the pool is first built, then runs
//! the schema. Rows are namespaced per test by unique client ids.
//!
//! `setup()` also takes a process-wide lock so the test bodies run one at a
//! time: several run 8 threads against one `max_size(3)` pool, and letting a
//! dozen such tests overlap turns a bounded stress test into `SQLITE_BUSY`
//! flakiness. The intra-test concurrency (the actual race under test) is
//! unaffected.

use std::sync::{LazyLock, Mutex, MutexGuard, PoisonError};

use screenerbot::agent_control::{
    approvals, audit, pairing, store, Error, PermissionLevel, ToolPermissions,
};

static SETUP: LazyLock<()> = LazyLock::new(|| {
    let dir = tempfile::tempdir().expect("tempdir");
    let db = dir.path().join("agent_control.db");
    std::env::set_var("SCREENERBOT_AGENT_CONTROL_DB", &db);
    // Keep the temp dir alive for the whole test binary.
    std::mem::forget(dir);
    store::init().expect("store init");
});

static TEST_LOCK: Mutex<()> = Mutex::new(());

fn setup() -> MutexGuard<'static, ()> {
    LazyLock::force(&SETUP);
    TEST_LOCK.lock().unwrap_or_else(PoisonError::into_inner)
}

#[test]
fn init_is_idempotent() {
    let _guard = setup();
    // Re-running init after the schema exists must be a no-op, not an error.
    store::init().expect("second init");
    store::sweep().expect("sweep on an initialized store");
}

#[test]
fn pairing_secret_is_shown_once_and_only_the_verifier_is_kept() {
    let _guard = setup();
    let limited = ToolPermissions {
        trading: PermissionLevel::Deny,
        ..ToolPermissions::full_access()
    };
    let created = pairing::create("Roundtrip agent", "roundtrip-kind", Some(limited)).unwrap();
    assert!(created.pairing_secret.len() >= 43);

    // The secret never comes back through the list surface.
    let listed = pairing::list().unwrap();
    let row = listed
        .iter()
        .find(|p| p.client_id == created.client_id)
        .expect("created pairing is listed");
    assert_eq!(row.permissions, limited);
    assert!(!row.revoked);
    let json = serde_json::to_string(&listed).unwrap();
    assert!(
        !json.contains(&created.pairing_secret),
        "secret leaked into list"
    );
    assert!(!json.to_lowercase().contains("verifier"));

    // The right secret authenticates and resolves the stored policy.
    let authed = pairing::authenticate(&created.client_id, &created.pairing_secret).unwrap();
    assert_eq!(authed.permissions, limited);
}

/// A connection created without an explicit policy can do everything, and the
/// owner can narrow (or re-widen) it afterwards without recreating it.
#[test]
fn a_new_pairing_is_full_access_and_can_be_limited_afterwards() {
    let _guard = setup();
    let created = pairing::create("Default agent", "default-kind", None).unwrap();

    let authed = pairing::authenticate(&created.client_id, &created.pairing_secret).unwrap();
    assert_eq!(authed.permissions, ToolPermissions::full_access());

    let limited = ToolPermissions {
        trading: PermissionLevel::AskUser,
        config: PermissionLevel::Deny,
        ..ToolPermissions::full_access()
    };
    assert!(pairing::set_permissions(&created.client_id, limited).unwrap());

    // The next authentication — i.e. the connection's next request — sees it.
    let authed = pairing::authenticate(&created.client_id, &created.pairing_secret).unwrap();
    assert_eq!(authed.permissions, limited);
    assert_eq!(
        pairing::active_permissions(&created.client_id).unwrap(),
        Some(limited)
    );

    // And it survives a widening edit, so a limit is never a one-way door.
    assert!(pairing::set_permissions(&created.client_id, ToolPermissions::full_access()).unwrap());
    assert_eq!(
        pairing::active_permissions(&created.client_id).unwrap(),
        Some(ToolPermissions::full_access())
    );

    // A revoked pairing accepts no further policy edits.
    assert!(pairing::revoke(&created.client_id).unwrap());
    assert!(!pairing::set_permissions(&created.client_id, limited).unwrap());
}

#[test]
fn unknown_wrong_and_revoked_credentials_fail_identically() {
    let _guard = setup();
    let created = pairing::create("Reject agent", "reject-kind", None).unwrap();

    let wrong = pairing::authenticate(&created.client_id, "definitely-not-the-secret");
    let unknown = pairing::authenticate("00000000-0000-0000-0000-000000000000", "x");
    assert!(matches!(wrong, Err(Error::PairingRejected)));
    assert!(matches!(unknown, Err(Error::PairingRejected)));

    assert!(pairing::revoke(&created.client_id).unwrap());
    let revoked = pairing::authenticate(&created.client_id, &created.pairing_secret);
    assert!(matches!(revoked, Err(Error::PairingRejected)));
    // Revoking again changes nothing.
    assert!(!pairing::revoke(&created.client_id).unwrap());
    assert!(pairing::active_permissions(&created.client_id)
        .unwrap()
        .is_none());
}

#[test]
fn create_rejects_out_of_bounds_input() {
    let _guard = setup();
    assert!(matches!(
        pairing::create("ok", "Bad Kind With Spaces", None),
        Err(Error::InvalidPairingRequest { .. })
    ));
    assert!(matches!(
        pairing::create("", "kind", None),
        Err(Error::InvalidPairingRequest { .. })
    ));
}

#[test]
fn approval_claim_is_exactly_once_under_concurrency() {
    let _guard = setup();
    let client = "concurrency-client";
    let handle = approvals::create_or_reuse(
        client,
        "buy_token",
        &serde_json::json!({ "mint": "M", "sol": 1 }),
        "corr-conc",
    )
    .unwrap();
    assert_eq!(handle.state, "pending");

    let id = handle.id.clone();
    let wins: usize = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..8)
            .map(|_| {
                let id = id.clone();
                scope.spawn(move || approvals::claim(&id).is_ok())
            })
            .collect();
        handles
            .into_iter()
            .map(|h| h.join().unwrap())
            .filter(|won| *won)
            .count()
    });
    assert_eq!(wins, 1, "exactly one claimer may win");

    // A denial now also loses — the row already left `pending`.
    assert!(matches!(
        approvals::deny(&id),
        Err(Error::ApprovalNotPending)
    ));
}

#[test]
fn concurrent_create_or_reuse_yields_exactly_one_row() {
    let _guard = setup();
    let client = "create-race-client";
    let args = serde_json::json!({ "mint": "RACE", "sol": 1 });

    let ids: Vec<String> = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..8)
            .map(|n| {
                let args = args.clone();
                scope.spawn(move || {
                    approvals::create_or_reuse(client, "buy_token", &args, &format!("corr-{n}"))
                        .unwrap()
                        .id
                })
            })
            .collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    });

    // Every racing caller resolved to the one winning row.
    let first = &ids[0];
    assert!(ids.iter().all(|id| id == first), "ids: {ids:?}");

    // And the database holds exactly one approval row for this client+tool
    // (each test uses a unique client, so this is the binding).
    assert_eq!(
        approval_row_count(client, "buy_token"),
        1,
        "one row per (client, tool, digest)"
    );
}

/// Count `approvals` rows for a client+tool by opening the temp DB directly.
fn approval_row_count(client_id: &str, tool: &str) -> i64 {
    let path = std::env::var("SCREENERBOT_AGENT_CONTROL_DB").expect("db path set by setup()");
    let conn = rusqlite::Connection::open(path).expect("open temp agent_control.db");
    conn.query_row(
        "SELECT COUNT(*) FROM approvals WHERE client_id = ?1 AND tool = ?2",
        rusqlite::params![client_id, tool],
        |r| r.get(0),
    )
    .expect("count approvals")
}

#[test]
fn retry_recovers_the_same_request_but_changed_arguments_do_not() {
    let _guard = setup();
    let client = "binding-client";
    let args = serde_json::json!({ "mint": "N", "sol": 2 });

    let first = approvals::create_or_reuse(client, "buy_token", &args, "c1").unwrap();
    let again = approvals::create_or_reuse(client, "buy_token", &args, "c2").unwrap();
    assert_eq!(first.id, again.id, "same digest recovers the same request");

    let substituted = serde_json::json!({ "mint": "N", "sol": 999 });
    let other = approvals::create_or_reuse(client, "buy_token", &substituted, "c3").unwrap();
    assert_ne!(first.id, other.id, "changed args open a separate request");
}

#[test]
fn denied_request_stays_denied_on_retry() {
    let _guard = setup();
    let client = "deny-client";
    let args = serde_json::json!({ "mint": "D" });
    let handle = approvals::create_or_reuse(client, "sell_token", &args, "c1").unwrap();
    approvals::deny(&handle.id).unwrap();

    let retry = approvals::create_or_reuse(client, "sell_token", &args, "c2").unwrap();
    assert_eq!(retry.state, "denied");
    assert_eq!(retry.id, handle.id);
}

#[test]
fn view_is_scoped_to_the_owning_client() {
    let _guard = setup();
    let handle =
        approvals::create_or_reuse("owner-client", "get_status", &serde_json::json!({}), "c1")
            .unwrap();
    assert!(approvals::view_for_client(&handle.id, "owner-client").is_ok());
    assert!(matches!(
        approvals::view_for_client(&handle.id, "someone-else"),
        Err(Error::ApprovalNotFound)
    ));
}

#[test]
fn interrupted_execution_recovers_to_failed_and_is_never_replayed() {
    let _guard = setup();
    let client = "crash-client";
    let handle = approvals::create_or_reuse(
        client,
        "buy_token",
        &serde_json::json!({ "mint": "X", "sol": 3 }),
        "c1",
    )
    .unwrap();
    let claimed = approvals::claim(&handle.id).unwrap();
    approvals::mark_executing(&claimed.id).unwrap();

    // Simulate a crash between `executing` and a terminal state.
    let failed = store::recover_interrupted_approvals().unwrap();
    assert!(failed >= 1);

    let view = approvals::view_for_client(&handle.id, client).unwrap();
    assert_eq!(view.state, "failed");
    // The stored failure result is valid structured JSON, not null.
    let result = view
        .result
        .expect("a structured failure result is preserved");
    assert_eq!(result.get("success"), Some(&serde_json::json!(false)));
    assert!(result.get("error").and_then(|e| e.as_str()).is_some());

    // It cannot be picked up again.
    assert!(matches!(
        approvals::claim(&handle.id),
        Err(Error::ApprovalNotPending)
    ));

    // A retry of the SAME binding recovers the same failed request and its
    // result — it never opens a fresh pending row that could be re-approved.
    let retry = approvals::create_or_reuse(
        client,
        "buy_token",
        &serde_json::json!({ "mint": "X", "sol": 3 }),
        "c2",
    )
    .unwrap();
    assert_eq!(retry.id, handle.id);
    assert_eq!(retry.state, "failed");
    assert_eq!(
        retry.result.and_then(|r| r.get("success").cloned()),
        Some(serde_json::json!(false))
    );
    assert_eq!(approval_row_count(client, "buy_token"), 1);
}

#[test]
fn audit_records_are_redacted_bounded_and_paginated() {
    let _guard = setup();
    let ctx = audit::AuditContext {
        client_id: Some("audit-client".to_owned()),
        tool: Some("update_config".to_owned()),
        correlation_id: Some("corr-a".to_owned()),
    };
    // A detail carrying a secret-looking blob must be redacted at rest.
    let payload = serde_json::json!({
        "pairing_secret": "TOP-SECRET-VALUE",
        "rpc_url": "https://example.test/rpc",
    });
    for _ in 0..3 {
        audit::record(
            audit::AuditKind::ToolRequest,
            &ctx,
            "received",
            Some(&audit::sanitize(&payload)),
        );
    }

    let (rows, total) = audit::list(1, 1000).unwrap();
    assert!(total >= 3);
    assert!(rows.len() as i64 <= 200, "per_page is clamped to 200");
    let dump = serde_json::to_string(&rows).unwrap();
    assert!(
        !dump.contains("TOP-SECRET-VALUE"),
        "secret survived into audit"
    );

    // A page past the end is empty, not an error.
    let (empty, _) = audit::list(9_999, 50).unwrap();
    assert!(empty.is_empty());
}

#[test]
fn audit_log_is_hard_capped_and_context_is_bounded() {
    let _guard = setup();

    // Context fields far longer than the per-column cap, to prove they are
    // clamped at rest and not stored verbatim.
    let ctx = audit::AuditContext {
        client_id: Some("c".repeat(4_000)),
        tool: Some("cap_probe".to_owned()),
        correlation_id: Some("k".repeat(4_000)),
    };
    for _ in 0..5_200 {
        audit::record(audit::AuditKind::ToolRequest, &ctx, "received", None);
    }

    // The table itself never exceeds the hard cap — enforced on every insert,
    // not "eventually" on the hourly sweep.
    let (_page, total) = audit::list(1, 1).unwrap();
    assert!(
        total <= 5_000,
        "audit table exceeded the hard row cap: {total}"
    );

    let (page, _) = audit::list(1, 5).unwrap();
    let row = page
        .iter()
        .find(|r| r.tool.as_deref() == Some("cap_probe"))
        .expect("a cap_probe row is retained");
    assert!(
        row.client_id.as_deref().unwrap_or_default().chars().count() <= 129,
        "client_id was not clamped at rest"
    );
    assert!(
        row.correlation_id
            .as_deref()
            .unwrap_or_default()
            .chars()
            .count()
            <= 129,
        "correlation_id was not clamped at rest"
    );

    // A rejected bridge auth is recorded with the identity-free `Default`
    // context (see `bridge::authenticate`) so a pairing secret mistakenly sent
    // in the client-id header can never land in the audit log.
    audit::record(
        audit::AuditKind::BridgeAuth,
        &audit::AuditContext::default(),
        "rejected",
        None,
    );
    let (page, _) = audit::list(1, 5).unwrap();
    let rejected = page
        .iter()
        .find(|r| r.kind == "bridge_auth" && r.outcome == "rejected")
        .expect("a rejected bridge_auth row");
    assert!(
        rejected.client_id.is_none(),
        "rejected auth must not persist a caller-supplied identity"
    );
    assert!(rejected.detail.is_none());
}

#[test]
fn finish_requires_an_executing_row() {
    let _guard = setup();
    let client = "finish-guard-client";
    let handle = approvals::create_or_reuse(
        client,
        "buy_token",
        &serde_json::json!({ "mint": "F" }),
        "c1",
    )
    .unwrap();

    // Not yet claimed / executing: `finish` refuses rather than silently no-op.
    assert!(matches!(
        approvals::finish(&handle.id, true, &serde_json::json!({ "success": true })),
        Err(Error::ApprovalNotPending)
    ));

    let claimed = approvals::claim(&handle.id).unwrap();
    // Claimed but not executing: still refused.
    assert!(matches!(
        approvals::finish(&claimed.id, true, &serde_json::json!({ "success": true })),
        Err(Error::ApprovalNotPending)
    ));

    approvals::mark_executing(&claimed.id).unwrap();
    // The one legitimate transition succeeds.
    approvals::finish(&claimed.id, true, &serde_json::json!({ "success": true })).unwrap();
    assert_eq!(
        approvals::view_for_client(&handle.id, client)
            .unwrap()
            .state,
        "done"
    );

    // A second finish now finds no executing row.
    assert!(matches!(
        approvals::finish(&claimed.id, true, &serde_json::json!({ "success": true })),
        Err(Error::ApprovalNotPending)
    ));
}

#[test]
fn claim_with_corrupt_canonical_args_fails_the_row_closed() {
    use sha2::{Digest, Sha256};

    let _guard = setup();
    let client = "corrupt-args-client";
    let handle = approvals::create_or_reuse(
        client,
        "buy_token",
        &serde_json::json!({ "mint": "C" }),
        "c1",
    )
    .unwrap();

    // Rewrite the stored canonical args to a non-JSON string, keeping the digest
    // consistent so the integrity check passes and `claim` reaches the JSON
    // parse that then fails.
    let bad = "this is not json";
    let digest = Sha256::digest(bad.as_bytes()).to_vec();
    let path = std::env::var("SCREENERBOT_AGENT_CONTROL_DB").expect("db path set by setup()");
    let conn = rusqlite::Connection::open(&path).expect("open temp agent_control.db");
    conn.execute(
        "UPDATE approvals SET canonical_args = ?1, args_digest = ?2 WHERE id = ?3",
        rusqlite::params![bad, digest, handle.id],
    )
    .expect("rewrite canonical args");
    drop(conn);

    match approvals::claim(&handle.id) {
        Err(Error::Database(_)) => {}
        Err(other) => panic!("expected a Database error, got {other:?}"),
        Ok(_) => panic!("claim must fail on corrupt canonical args"),
    }

    // The row must not be stuck in `claimed` until a restart; it is now
    // terminally failed with a valid structured result.
    let view = approvals::view_for_client(&handle.id, client).unwrap();
    assert_eq!(view.state, "failed");
    let result = view.result.expect("a structured failure result is stored");
    assert_eq!(result.get("success"), Some(&serde_json::json!(false)));
    assert!(result.get("error").and_then(|e| e.as_str()).is_some());

    // And it can never be picked up again.
    assert!(matches!(
        approvals::claim(&handle.id),
        Err(Error::ApprovalNotPending)
    ));
}

/// The v1 -> v2 upgrade an existing install goes through on its next launch: a
/// pairing keeps exactly the capabilities its old scope granted, rather than
/// silently inheriting the new full-access default.
#[test]
fn legacy_scopes_migrate_into_equivalent_per_connection_permissions() {
    let dir = tempfile::tempdir().expect("tempdir");
    let connection = rusqlite::Connection::open(dir.path().join("v1.db")).expect("open v1 db");
    connection
        .execute_batch(
            "CREATE TABLE pairings (
                 client_id     TEXT PRIMARY KEY,
                 label         TEXT NOT NULL,
                 agent_kind    TEXT NOT NULL,
                 scope         TEXT NOT NULL,
                 verifier      BLOB NOT NULL,
                 created_at    INTEGER NOT NULL,
                 last_used_at  INTEGER,
                 revoked_at    INTEGER
             );
             INSERT INTO pairings VALUES ('r', 'reader', 'k', 'read', x'00', 1, NULL, NULL);
             INSERT INTO pairings VALUES ('o', 'operator', 'k', 'operate', x'00', 1, NULL, NULL);
             INSERT INTO pairings VALUES ('t', 'trader', 'k', 'trade', x'00', 1, NULL, NULL);
             INSERT INTO pairings VALUES ('x', 'broken', 'k', 'bogus', x'00', 1, NULL, NULL);",
        )
        .expect("seed a v1 pairings table");

    store::migrate_scope_to_permissions(&connection).expect("migrate");

    let policy = |client_id: &str| -> ToolPermissions {
        let raw: String = connection
            .query_row(
                "SELECT permissions FROM pairings WHERE client_id = ?1",
                [client_id],
                |row| row.get(0),
            )
            .expect("row");
        ToolPermissions::from_json(&raw).expect("migrated policy parses")
    };

    for client_id in ["r", "o", "t", "x"] {
        let permissions = policy(client_id);
        assert_eq!(permissions.analysis, PermissionLevel::Allow);
        assert_eq!(permissions.portfolio, PermissionLevel::Allow);
    }
    // read: nothing beyond reads. operate: config/system but no trading.
    assert_eq!(policy("r").config, PermissionLevel::Deny);
    assert_eq!(policy("r").trading, PermissionLevel::Deny);
    assert_eq!(policy("o").config, PermissionLevel::Allow);
    assert_eq!(policy("o").system, PermissionLevel::Allow);
    assert_eq!(policy("o").trading, PermissionLevel::Deny);
    assert_eq!(policy("t"), ToolPermissions::full_access());
    // An unrecognised stored scope must not be read as a capability.
    assert_eq!(policy("x").trading, PermissionLevel::Deny);
    assert_eq!(policy("x").config, PermissionLevel::Deny);

    // The old column is gone, and re-running the migration is a no-op.
    assert!(!connection
        .prepare("SELECT 1 FROM pragma_table_info('pairings') WHERE name = 'scope'")
        .expect("pragma")
        .exists([])
        .expect("query"));
    store::migrate_scope_to_permissions(&connection).expect("second migrate is a no-op");
}
