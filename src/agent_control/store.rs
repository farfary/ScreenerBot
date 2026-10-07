// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Durable storage for the agent-control boundary: client pairings, the
//! external-agent approval queue, agent trade submissions and the audit log.
//!
//! One pooled SQLite database (`agent_control.db`) built through the canonical
//! `database::configure_connection` init hook so the WAL PRAGMAs survive r2d2
//! connection recycling. Policy lives in `pairing`/`approvals`/`audit`; this
//! module owns the pool, the schema, the startup recovery sweep and the stored
//! answer of a request that ended without one (`interrupted_result`).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::LazyLock;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use r2d2::{Pool, PooledConnection};
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::OptionalExtension;
use serde_json::{json, Value};

use crate::agent_control::audit::{self, AuditContext, AuditKind};
use crate::agent_control::error::{Error, Result};
use crate::agent_control::permissions::{PermissionLevel, ToolPermissions};
use crate::agent_control::submissions;
use crate::database;
use crate::errors::DatabaseError;
use crate::logger::{self, LogTag};

/// v1 was the initial schema. v2 replaces a pairing's coarse `scope` with its
/// own per-category permission policy (`permissions`), so an owner can grant a
/// connection everything and then limit exactly the categories they want. v3
/// adds `submissions`, the trades an agent connection submitted.
const SCHEMA_VERSION: u32 = 3;

/// Audit rows older than this are pruned by the periodic sweep.
const AUDIT_RETENTION_SECS: i64 = 30 * 24 * 60 * 60;
/// Hard cap on retained audit rows regardless of age.
const AUDIT_MAX_ROWS: i64 = 5_000;
/// How often the background sweep prunes audit rows and expires stale
/// pending approvals.
const SWEEP_INTERVAL: Duration = Duration::from_secs(3_600);

static INITIALIZED: LazyLock<AtomicBool> = LazyLock::new(|| AtomicBool::new(false));

static POOL: LazyLock<Pool<SqliteConnectionManager>> = LazyLock::new(|| {
    // Tests point this at a temp file so they never touch a real install's DB.
    let path = std::env::var("SCREENERBOT_AGENT_CONTROL_DB")
        .ok()
        .filter(|s| !s.is_empty())
        .map(std::path::PathBuf::from)
        .unwrap_or_else(crate::paths::get_agent_control_db_path);
    let manager = SqliteConnectionManager::file(&path)
        .with_init(|c| database::configure_connection(c, database::AGENT_CONTROL_DB));
    Pool::builder()
        .max_size(3)
        .idle_timeout(None)
        .max_lifetime(None)
        .build(manager)
        .expect("failed to create agent_control database pool")
});

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS schema_version (
    version INTEGER PRIMARY KEY,
    applied_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS pairings (
    client_id     TEXT PRIMARY KEY,
    label         TEXT NOT NULL,
    agent_kind    TEXT NOT NULL,
    -- The connection's own per-category policy, as JSON
    -- ({"analysis":"allow",…}). Unreadable content fails closed in `pairing`.
    permissions   TEXT NOT NULL,
    verifier      BLOB NOT NULL,
    created_at    INTEGER NOT NULL,
    last_used_at  INTEGER,
    revoked_at    INTEGER
);

CREATE TABLE IF NOT EXISTS approvals (
    id             TEXT PRIMARY KEY,
    client_id      TEXT NOT NULL,
    tool           TEXT NOT NULL,
    args_digest    BLOB NOT NULL,
    canonical_args TEXT NOT NULL,
    args_summary   TEXT NOT NULL,
    correlation_id TEXT NOT NULL,
    state          TEXT NOT NULL,
    created_at     INTEGER NOT NULL,
    expires_at     INTEGER NOT NULL,
    resolved_at    INTEGER,
    decided_by     TEXT,
    result_json    TEXT
);
CREATE INDEX IF NOT EXISTS idx_approvals_client_state ON approvals(client_id, state);
CREATE INDEX IF NOT EXISTS idx_approvals_state_expiry ON approvals(state, expires_at);
-- One row per binding, enforced by the database. `create_or_reuse` relies on
-- this: a concurrent identical call hits the conflict and reuses the winner.
CREATE UNIQUE INDEX IF NOT EXISTS idx_approvals_binding
    ON approvals(client_id, tool, args_digest);

CREATE TABLE IF NOT EXISTS submissions (
    id             TEXT PRIMARY KEY,
    client_id      TEXT NOT NULL,
    tool           TEXT NOT NULL,
    args_digest    BLOB NOT NULL,
    args_summary   TEXT NOT NULL,
    correlation_id TEXT NOT NULL,
    state          TEXT NOT NULL,
    created_at     INTEGER NOT NULL,
    finished_at    INTEGER,
    result_json    TEXT
);
CREATE INDEX IF NOT EXISTS idx_submissions_binding
    ON submissions(client_id, tool, args_digest, created_at);

CREATE TABLE IF NOT EXISTS audit (
    id             INTEGER PRIMARY KEY AUTOINCREMENT,
    ts             INTEGER NOT NULL,
    kind           TEXT NOT NULL,
    client_id      TEXT,
    tool           TEXT,
    correlation_id TEXT,
    outcome        TEXT NOT NULL,
    detail         TEXT
);
CREATE INDEX IF NOT EXISTS idx_audit_ts ON audit(ts DESC);
"#;

/// Why a request has no answer: the process stopped while it ran.
pub(crate) const STOPPED_WHILE_RUNNING: &str = "the app stopped while this ran";
/// Why a request has no answer: the task running it ended without one.
pub(crate) const TASK_ENDED: &str = "the task running this ended without an answer";
/// Why a request has no answer: it finished, but its outcome could not be stored.
pub(crate) const OUTCOME_NOT_STORED: &str = "this finished but its outcome could not be stored";

/// The stored answer of an approval or a submission whose task ended without an
/// answer (`cause`). It may have taken effect, so it is never replayed, and the
/// answer says what to read instead: a trade's swap may have been sent, so the
/// positions come first. The one wording of that state for both tables.
pub(crate) fn interrupted_result(cause: &str, sends_transaction: bool) -> Value {
    let consequence = if sends_transaction {
        "its swap may have been sent, so read the positions before trading again"
    } else {
        "it may have taken effect, so read the current state before repeating it"
    };
    json!({ "success": false, "error": format!("{cause}; {consequence}") })
}

/// Current unix time in whole seconds.
pub(crate) fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Borrow a pooled connection. Callers run their own statements synchronously;
/// the web layer wraps calls in `spawn_blocking`. A raw `rusqlite::Error` from
/// any later statement converts into `Error::Database` through `?`.
pub(crate) fn conn() -> Result<PooledConnection<SqliteConnectionManager>> {
    POOL.get().map_err(|e| {
        Error::Database(DatabaseError::Connection {
            message: e.to_string(),
        })
    })
}

/// Create the schema (idempotent) and run the startup recovery sweep. Safe to
/// call in every boot state that has a webserver; repeat calls are no-ops.
pub fn init() -> Result<()> {
    if INITIALIZED.load(Ordering::Relaxed) {
        return Ok(());
    }

    let connection = conn()?;

    // Create the schema (idempotent) before anything reads it — this also
    // creates `schema_version` itself and the UNIQUE approval-binding index.
    connection.execute_batch(SCHEMA)?;

    // v1 -> v2: give an existing pairing its own permission policy, derived
    // from the scope it was created with, then drop the scope column.
    migrate_scope_to_permissions(&connection)?;

    // With `schema_version` guaranteed to exist, read the recorded version. An
    // empty table (a fresh database) is the only "no value" case; every other
    // SQLite error propagates.
    let stored_version: Option<u32> = connection
        .query_row(
            "SELECT version FROM schema_version ORDER BY version DESC LIMIT 1",
            [],
            |row| row.get(0),
        )
        .optional()?;

    if stored_version.unwrap_or(0) < SCHEMA_VERSION {
        connection.execute(
            "INSERT OR REPLACE INTO schema_version (version, applied_at) VALUES (?1, ?2)",
            rusqlite::params![SCHEMA_VERSION, chrono::Utc::now().to_rfc3339()],
        )?;
    }

    let recovered = recover_interrupted(&connection)?;
    if recovered > 0 {
        logger::warning(
            LogTag::Security,
            &format!(
                "agent-control: {recovered} approval(s) were claimed or executing at \
                 shutdown; marked failed or interrupted and NOT replayed"
            ),
        );
    }
    let interrupted = submissions::recover_interrupted(&connection)?;
    if interrupted > 0 {
        logger::warning(
            LogTag::Security,
            &format!(
                "agent-control: {interrupted} agent trade(s) were running at shutdown; \
                 marked interrupted and NOT replayed"
            ),
        );
        audit::record(
            AuditKind::Execution,
            &AuditContext::default(),
            "interrupted_recovery",
            Some(&format!(
                "{interrupted} running agent trade(s) marked interrupted on boot"
            )),
        );
    }

    // Enforce the audit retention window and row cap at startup, not only on the
    // hourly loop — a long-idle process should not carry a stale, unbounded log.
    if let Err(e) = prune_audit(&connection) {
        logger::debug(
            LogTag::Security,
            &format!("agent-control: startup audit prune failed: {e}"),
        );
    }

    INITIALIZED.store(true, Ordering::Relaxed);
    logger::info(LogTag::System, "agent-control store initialized");

    // The periodic maintenance loop needs a Tokio runtime. Production always
    // has one (bootstrap runs inside it); a bare unit/integration test may not,
    // and there the one-shot recovery above is enough.
    if tokio::runtime::Handle::try_current().is_ok() {
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(SWEEP_INTERVAL).await;
                if let Err(e) = sweep() {
                    logger::debug(
                        LogTag::Security,
                        &format!("agent-control sweep failed: {e}"),
                    );
                }
            }
        });
    }

    Ok(())
}

/// Add the `permissions` column to a v1 `pairings` table and translate each
/// row's old scope into an equivalent policy, then drop `scope`. A database
/// created fresh at v2 already has the column and skips this entirely.
///
/// The scope translation is deliberately conservative — it preserves what the
/// pairing could already do rather than widening it to the new full-access
/// default, which applies to connections created from now on.
///
/// Public so `tests/agent_control_store.rs` can run it against a hand-built v1
/// database; the process pool only ever holds one schema version.
pub fn migrate_scope_to_permissions(connection: &rusqlite::Connection) -> Result<()> {
    let has_scope = connection
        .prepare("SELECT 1 FROM pragma_table_info('pairings') WHERE name = 'scope'")?
        .exists([])?;
    if !has_scope {
        return Ok(());
    }

    let has_permissions = connection
        .prepare("SELECT 1 FROM pragma_table_info('pairings') WHERE name = 'permissions'")?
        .exists([])?;
    if !has_permissions {
        connection.execute(
            "ALTER TABLE pairings ADD COLUMN permissions TEXT NOT NULL DEFAULT ''",
            [],
        )?;
    }

    let rows: Vec<(String, String)> = connection
        .prepare("SELECT client_id, scope FROM pairings WHERE permissions = ''")?
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<std::result::Result<Vec<_>, _>>()?;

    for (client_id, scope) in rows {
        let policy = permissions_for_legacy_scope(&scope);
        connection.execute(
            "UPDATE pairings SET permissions = ?1 WHERE client_id = ?2",
            rusqlite::params![policy.to_json(), client_id],
        )?;
    }

    // SQLite 3.35+ (bundled) supports DROP COLUMN; the column is not indexed.
    connection.execute("ALTER TABLE pairings DROP COLUMN scope", [])?;
    logger::info(
        LogTag::System,
        "agent-control store: migrated pairing scopes to per-connection permissions",
    );
    Ok(())
}

/// The v1 scopes, mapped onto the policy that grants the same capabilities.
fn permissions_for_legacy_scope(scope: &str) -> ToolPermissions {
    use PermissionLevel::{Allow, Deny};
    let (trading, operate) = match scope {
        "trade" => (Allow, Allow),
        "operate" => (Deny, Allow),
        // "read" and anything unrecognised: reads only.
        _ => (Deny, Deny),
    };
    ToolPermissions {
        analysis: Allow,
        portfolio: Allow,
        trading,
        config: operate,
        system: operate,
    }
}

/// After a crash, an `executing` approval is indeterminate: its tool started and
/// the live mutation may have landed, so it becomes `interrupted` with the same
/// answer as an interrupted submission and is never replayed. A `claimed` row
/// never started its tool (it runs only once `executing`), so it fails closed as
/// not run. Overdue `pending` rows become `expired`. Only valid at startup
/// (`init`): called later, it would discard the result of a request still
/// running. Returns how many claimed or executing rows were closed.
pub(crate) fn recover_interrupted(connection: &rusqlite::Connection) -> Result<usize> {
    let now = now_unix();
    let executing: Vec<(String, String)> = connection
        .prepare("SELECT id, tool FROM approvals WHERE state = 'executing'")?
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let registry = crate::agent_control::create_tool_registry();
    let mut failed = 0;
    for (id, tool) in executing {
        let sends_transaction = registry
            .get(&tool)
            .is_some_and(|tool| tool.sends_transaction());
        failed += connection.execute(
            "UPDATE approvals SET state = 'interrupted', resolved_at = ?1, result_json = ?2
              WHERE id = ?3 AND state = 'executing'",
            rusqlite::params![
                now,
                interrupted_result(STOPPED_WHILE_RUNNING, sends_transaction).to_string(),
                id
            ],
        )?;
    }
    failed += connection.execute(
        "UPDATE approvals
                SET state = 'failed',
                    resolved_at = ?1,
                    result_json = '{\"success\":false,\"error\":\"the app stopped before this request ran; it did not run and is not retried\"}'
              WHERE state = 'claimed'",
        rusqlite::params![now],
    )?;

    let expired = connection.execute(
        "UPDATE approvals SET state = 'expired', resolved_at = ?1
              WHERE state = 'pending' AND expires_at <= ?1",
        rusqlite::params![now],
    )?;

    if failed > 0 {
        audit::record(
            AuditKind::Execution,
            &AuditContext::default(),
            "interrupted_recovery",
            Some(&format!("{failed} in-flight approval(s) closed on boot")),
        );
    }
    if expired > 0 {
        audit::record(
            AuditKind::ApprovalExpired,
            &AuditContext::default(),
            "boot_sweep",
            Some(&format!("{expired} pending approval(s) expired on boot")),
        );
    }

    Ok(failed)
}

/// Periodic maintenance: expire overdue pending approvals, prune finished
/// agent trade submissions, and prune the audit log to the retention window and
/// row cap.
pub fn sweep() -> Result<()> {
    let connection = conn()?;
    let now = now_unix();

    let expired = connection.execute(
        "UPDATE approvals SET state = 'expired', resolved_at = ?1
              WHERE state = 'pending' AND expires_at <= ?1",
        rusqlite::params![now],
    )?;
    if expired > 0 {
        audit::record(
            AuditKind::ApprovalExpired,
            &AuditContext::default(),
            "sweep",
            Some(&format!("{expired} pending approval(s) expired")),
        );
    }

    submissions::prune(&connection)?;
    prune_audit(&connection)
}

/// Trim the audit log to the retention window and the hard row cap, on the
/// caller's connection. Called after every audit insert (see `audit::try_record`)
/// as well as at startup and on the hourly sweep, so the table is bounded at all
/// times rather than only "eventually". Issues only DELETEs — no audit write, so
/// it cannot recurse.
pub(crate) fn prune_audit(connection: &rusqlite::Connection) -> Result<()> {
    connection.execute(
        "DELETE FROM audit WHERE ts < ?1",
        rusqlite::params![now_unix() - AUDIT_RETENTION_SECS],
    )?;
    connection.execute(
        "DELETE FROM audit
              WHERE id <= COALESCE((
                    SELECT id FROM audit ORDER BY id DESC LIMIT 1 OFFSET ?1
              ), 0)",
        rusqlite::params![AUDIT_MAX_ROWS],
    )?;
    Ok(())
}

/// A throwaway store for the crate's own tests, which reach the startup-only
/// recovery that integration tests cannot.
#[cfg(test)]
pub(crate) mod test_support {
    use std::sync::{LazyLock, Mutex, MutexGuard, PoisonError};

    static SETUP: LazyLock<()> = LazyLock::new(|| {
        let dir = tempfile::tempdir().expect("tempdir");
        std::env::set_var(
            "SCREENERBOT_AGENT_CONTROL_DB",
            dir.path().join("agent_control.db"),
        );
        // Keep the temp dir alive for the whole test process.
        std::mem::forget(dir);
        super::init().expect("store init");
    });

    static TEST_LOCK: Mutex<()> = Mutex::new(());

    /// Point the pool at a temp database before it is first built, initialise
    /// it, and run the calling test alone against it.
    pub(crate) fn setup() -> MutexGuard<'static, ()> {
        LazyLock::force(&SETUP);
        TEST_LOCK.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Simulate the next startup's recovery sweep over both tables.
    pub(crate) fn restart() {
        let connection = super::conn().expect("connection");
        super::recover_interrupted(&connection).expect("approval recovery");
        crate::agent_control::submissions::recover_interrupted(&connection)
            .expect("submission recovery");
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::test_support::{restart, setup};
    use super::*;
    use crate::agent_control::approvals;
    use crate::agent_control::submissions::{self, SubmissionState};

    /// An approved trade and a submitted trade that were running when the app
    /// stopped give the same state and the same answer: the swap may have been
    /// sent, nothing is replayed, and an identical call returns that answer.
    #[test]
    fn a_trade_running_at_shutdown_reads_interrupted_in_both_tables() {
        let _guard = setup();
        let client = "restart-client";
        let args = json!({ "mint_address": "X", "amount_native": 0.1 });

        let approval = approvals::create_or_reuse(client, "buy_token", &args, "c1").unwrap();
        approvals::claim(&approval.id).unwrap();
        approvals::mark_executing(&approval.id).unwrap();
        let submission = submissions::submit_or_reuse(client, "buy_token", &args, "c1").unwrap();

        restart();

        let approved = approvals::view_for_client(&approval.id, client).unwrap();
        let submitted = submissions::view_for_client(&submission.trade_id, client)
            .unwrap()
            .expect("retained");
        assert_eq!(approved.state, "interrupted");
        assert_eq!(submitted.state, SubmissionState::Interrupted);
        assert_eq!(approved.state, submitted.state.as_str());
        let expected = interrupted_result(STOPPED_WHILE_RUNNING, true);
        assert_eq!(approved.result.as_ref(), Some(&expected));
        assert_eq!(submitted.result.as_ref(), Some(&expected));
        assert!(expected["error"]
            .as_str()
            .is_some_and(|error| error.contains("may have been sent")
                && error.contains("read the positions before trading again")));

        // Neither is picked up again, a late outcome cannot rewrite either, and
        // an identical call answers the interruption instead of trading.
        assert!(matches!(
            approvals::claim(&approval.id),
            Err(Error::ApprovalNotPending)
        ));
        assert!(matches!(
            approvals::finish(&approval.id, true, &json!({})),
            Err(Error::ApprovalNotPending)
        ));
        assert!(!submissions::finish(&submission.trade_id, true, &json!({})).unwrap());
        let retry = approvals::create_or_reuse(client, "buy_token", &args, "c2").unwrap();
        assert_eq!(retry.id, approval.id);
        assert_eq!(retry.state, "interrupted");
        let retry = submissions::submit_or_reuse(client, "buy_token", &args, "c2").unwrap();
        assert_eq!(retry.trade_id, submission.trade_id);
        assert_eq!(retry.state, SubmissionState::Interrupted);
    }

    /// A request that was not a trade says it may have taken effect, not that a
    /// swap may have been sent; one that was only claimed never ran.
    #[test]
    fn recovery_answers_what_each_interrupted_request_could_have_done() {
        let _guard = setup();
        let client = "restart-kinds-client";
        let config =
            approvals::create_or_reuse(client, "update_config", &json!({ "changes": [] }), "c1")
                .unwrap();
        approvals::claim(&config.id).unwrap();
        approvals::mark_executing(&config.id).unwrap();
        let claimed =
            approvals::create_or_reuse(client, "sell_token", &json!({ "mint_address": "Y" }), "c1")
                .unwrap();
        approvals::claim(&claimed.id).unwrap();

        restart();

        let config = approvals::view_for_client(&config.id, client).unwrap();
        assert_eq!(config.state, "interrupted");
        assert_eq!(
            config.result,
            Some(interrupted_result(STOPPED_WHILE_RUNNING, false))
        );
        let claimed = approvals::view_for_client(&claimed.id, client).unwrap();
        assert_eq!(claimed.state, "failed");
        let error = claimed
            .result
            .as_ref()
            .and_then(|result| result["error"].as_str())
            .expect("a claimed request explains itself");
        assert!(error.contains("did not run"), "{error}");
    }
}
