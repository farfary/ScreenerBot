//! The external-agent approval queue and its state machine.
//!
//! An MCP tool call that resolves to `RequireApproval` creates a durable,
//! expiring request bound to the pairing/client, the exact tool, the canonical
//! arguments (and their digest) and an audit correlation id. A human approves
//! or denies it *inside ScreenerBot*; the external caller can never approve its
//! own request. Resolution is exactly-once, execution is at-most-once, and the
//! stored canonical arguments are the only thing ever executed — a later call
//! that changes the arguments produces a different digest and therefore a
//! different, separately-approved request.
//!
//! Lifecycle: `pending → claimed → executing → done | failed`, plus terminal
//! `denied` and `expired`. No DB transaction is held across the async tool
//! execution: each transition is a single guarded `UPDATE`, and a crash between
//! `claimed`/`executing` and a terminal state fails closed (see `store`).

use std::time::Duration;

use rusqlite::OptionalExtension;
use serde::Serialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::agent_control::audit::{self, AuditContext, AuditKind};
use crate::agent_control::error::{Error, Result};
use crate::agent_control::store::{self, now_unix};
use crate::errors::DatabaseError;

/// How long a pending approval stays actionable.
pub const APPROVAL_TTL: Duration = Duration::from_secs(5 * 60);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApprovalState {
    Pending,
    Claimed,
    Executing,
    Done,
    Failed,
    Denied,
    Expired,
}

impl ApprovalState {
    pub fn as_str(self) -> &'static str {
        match self {
            ApprovalState::Pending => "pending",
            ApprovalState::Claimed => "claimed",
            ApprovalState::Executing => "executing",
            ApprovalState::Done => "done",
            ApprovalState::Failed => "failed",
            ApprovalState::Denied => "denied",
            ApprovalState::Expired => "expired",
        }
    }
    /// Parse a stored `state`. An unrecognised value returns `None` so the
    /// caller fails closed rather than treating it as `expired`.
    fn parse(s: &str) -> Option<ApprovalState> {
        Some(match s {
            "pending" => ApprovalState::Pending,
            "claimed" => ApprovalState::Claimed,
            "executing" => ApprovalState::Executing,
            "done" => ApprovalState::Done,
            "failed" => ApprovalState::Failed,
            "denied" => ApprovalState::Denied,
            "expired" => ApprovalState::Expired,
            _ => return None,
        })
    }
}

/// What `create_or_reuse` returns to the bridge.
#[derive(Debug, Clone, Serialize)]
pub struct ApprovalHandle {
    pub id: String,
    pub state: String,
    pub expires_at: i64,
    /// Present once the request reached a terminal state with a stored result.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    /// True only when this call inserted a brand-new pending row. Internal —
    /// never serialized to the bridge/MCP client; the bridge uses it to avoid
    /// logging `approval_created` on every reuse.
    #[serde(skip)]
    pub created: bool,
}

/// The client/tool/correlation context of a resolved approval, for audit.
pub struct DeniedContext {
    pub client_id: String,
    pub tool: String,
    pub correlation_id: String,
}

/// A pending approval as shown on the dashboard review surface.
#[derive(Debug, Clone, Serialize)]
pub struct PendingApproval {
    pub id: String,
    pub client_id: String,
    pub client_label: String,
    pub tool: String,
    pub args_summary: String,
    pub correlation_id: String,
    pub created_at: i64,
    pub expires_at: i64,
}

/// The row the executor needs after a successful claim.
pub struct ClaimedApproval {
    pub id: String,
    pub client_id: String,
    pub tool: String,
    pub correlation_id: String,
    pub canonical_args: Value,
}

/// Deterministic JSON: object keys sorted recursively, no insignificant
/// whitespace. Two equal argument sets always produce the same bytes and the
/// same digest regardless of key order on the wire.
pub fn canonicalize(value: &Value) -> String {
    fn norm(v: &Value) -> Value {
        match v {
            Value::Array(items) => Value::Array(items.iter().map(norm).collect()),
            Value::Object(map) => {
                let mut keys: Vec<&String> = map.keys().collect();
                keys.sort();
                let mut out = serde_json::Map::new();
                for k in keys {
                    out.insert(k.clone(), norm(&map[k]));
                }
                Value::Object(out)
            }
            other => other.clone(),
        }
    }
    norm(value).to_string()
}

fn digest_of(canonical: &str) -> Vec<u8> {
    Sha256::digest(canonical.as_bytes()).to_vec()
}

/// Parse a stored `result_json`. We only ever write valid JSON (`sanitize_value`
/// guarantees it), so a parse failure means the row is corrupt — surface a
/// structured failure value rather than `null`, so a poll/retry sees a definite
/// error instead of "done with no result".
fn stored_result(raw: Option<String>) -> Option<Value> {
    raw.map(|r| {
        serde_json::from_str::<Value>(&r).unwrap_or_else(
            |_| json!({ "success": false, "error": "stored result could not be read" }),
        )
    })
}

/// Turn one stored approval row into an `ApprovalHandle`, lazily surfacing
/// expiry and failing closed on an unrecognised `state`.
fn row_to_handle(
    connection: &rusqlite::Connection,
    id: &str,
    state_str: &str,
    expires_at: i64,
    result_json: Option<String>,
    created: bool,
) -> Result<ApprovalHandle> {
    let mut state = ApprovalState::parse(state_str).ok_or_else(|| {
        Error::Database(DatabaseError::Sqlite {
            message: format!("stored approval state {state_str:?} is not recognised"),
        })
    })?;

    if state == ApprovalState::Pending && expires_at <= now_unix() {
        let moved = connection.execute(
            "UPDATE approvals SET state='expired', resolved_at=?1
               WHERE id=?2 AND state='pending'",
            rusqlite::params![now_unix(), id],
        )?;
        state = ApprovalState::Expired;
        if moved > 0 {
            audit::record(
                AuditKind::ApprovalExpired,
                &AuditContext {
                    correlation_id: Some(id.to_owned()),
                    ..Default::default()
                },
                "lazy",
                None,
            );
        }
    }

    Ok(ApprovalHandle {
        id: id.to_owned(),
        state: state.as_str().to_owned(),
        expires_at,
        result: stored_result(result_json),
        created,
    })
}

/// Create a pending approval, or recover the existing one for the same
/// `(client_id, tool, canonical-args digest)`.
///
/// Race-safe: the insert is `ON CONFLICT DO NOTHING` against the UNIQUE binding
/// index, then the row is read back — concurrent identical calls all land on
/// the one winning row. EVERY state is reused, terminal ones included: a
/// `done`/`failed` request returns its stored result and never re-executes, a
/// `denied` one keeps returning `denied`, an `expired` one keeps returning
/// `expired`. A binding therefore has exactly one row for its whole life.
pub fn create_or_reuse(
    client_id: &str,
    tool: &str,
    args: &Value,
    correlation_id: &str,
) -> Result<ApprovalHandle> {
    let canonical = canonicalize(args);
    let digest = digest_of(&canonical);
    let connection = store::conn()?;

    let now = now_unix();
    let expires_at = now + APPROVAL_TTL.as_secs() as i64;
    let inserted = connection.execute(
        "INSERT INTO approvals
               (id, client_id, tool, args_digest, canonical_args, args_summary,
                correlation_id, state, created_at, expires_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'pending', ?8, ?9)
         ON CONFLICT(client_id, tool, args_digest) DO NOTHING",
        rusqlite::params![
            uuid::Uuid::new_v4().to_string(),
            client_id,
            tool,
            digest,
            canonical,
            audit::sanitize(args),
            correlation_id,
            now,
            expires_at,
        ],
    )?;

    let (row_id, state_str, row_expires_at, result_json): (String, String, i64, Option<String>) =
        connection.query_row(
            "SELECT id, state, expires_at, result_json
               FROM approvals
              WHERE client_id = ?1 AND tool = ?2 AND args_digest = ?3",
            rusqlite::params![client_id, tool, digest],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )?;

    row_to_handle(
        &connection,
        &row_id,
        &state_str,
        row_expires_at,
        result_json,
        inserted == 1,
    )
}

/// The client-scoped poll view. Only the owning client may read its request.
pub fn view_for_client(id: &str, client_id: &str) -> Result<ApprovalHandle> {
    let connection = store::conn()?;
    let row: Option<(String, i64, Option<String>)> = connection
        .query_row(
            "SELECT state, expires_at, result_json
               FROM approvals WHERE id = ?1 AND client_id = ?2",
            rusqlite::params![id, client_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()?;
    let Some((state_str, expires_at, result_json)) = row else {
        return Err(Error::ApprovalNotFound);
    };

    row_to_handle(&connection, id, &state_str, expires_at, result_json, false)
}

/// Pending approvals for the dashboard review surface, newest first.
pub fn list_pending() -> Result<Vec<PendingApproval>> {
    let connection = store::conn()?;
    let mut stmt = connection.prepare(
        "SELECT a.id, a.client_id, COALESCE(p.label, a.client_id), a.tool,
                    a.args_summary, a.correlation_id, a.created_at, a.expires_at
               FROM approvals a
               LEFT JOIN pairings p ON p.client_id = a.client_id
              WHERE a.state = 'pending' AND a.expires_at > ?1
              ORDER BY a.created_at DESC",
    )?;
    let rows = stmt
        .query_map(rusqlite::params![now_unix()], |row| {
            Ok(PendingApproval {
                id: row.get(0)?,
                client_id: row.get(1)?,
                client_label: row.get(2)?,
                tool: row.get(3)?,
                args_summary: row.get(4)?,
                correlation_id: row.get(5)?,
                created_at: row.get(6)?,
                expires_at: row.get(7)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(rows)
}

/// Deny a pending approval. Exactly-once: a lost race returns
/// `ApprovalNotPending`. Returns the request's client/tool/correlation context
/// so the caller can write a complete audit row.
pub fn deny(id: &str) -> Result<DeniedContext> {
    let connection = store::conn()?;
    let changed = connection.execute(
        "UPDATE approvals SET state='denied', resolved_at=?1, decided_by='dashboard'
               WHERE id=?2 AND state='pending'",
        rusqlite::params![now_unix(), id],
    )?;
    if changed == 0 {
        return Err(Error::ApprovalNotPending);
    }
    let context = connection.query_row(
        "SELECT client_id, tool, correlation_id FROM approvals WHERE id = ?1",
        rusqlite::params![id],
        |r| {
            Ok(DeniedContext {
                client_id: r.get(0)?,
                tool: r.get(1)?,
                correlation_id: r.get(2)?,
            })
        },
    )?;
    Ok(context)
}

/// Claim a pending, non-expired approval for execution. This is the durable
/// exactly-once gate: only one caller can move a row out of `pending`.
pub fn claim(id: &str) -> Result<ClaimedApproval> {
    let connection = store::conn()?;
    let now = now_unix();
    let changed = connection.execute(
        "UPDATE approvals SET state='claimed', resolved_at=?1, decided_by='dashboard'
               WHERE id=?2 AND state='pending' AND expires_at > ?1",
        rusqlite::params![now, id],
    )?;
    if changed == 0 {
        return Err(Error::ApprovalNotPending);
    }

    let (client_id, tool, correlation_id, canonical_args, digest): (
        String,
        String,
        String,
        String,
        Vec<u8>,
    ) = connection.query_row(
        "SELECT client_id, tool, correlation_id, canonical_args, args_digest
               FROM approvals WHERE id = ?1",
        rusqlite::params![id],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
    )?;

    // Integrity: the stored canonical args must still hash to the stored
    // digest. A mismatch means the row was tampered with; fail closed with a
    // valid structured result.
    if digest_of(&canonical_args) != digest {
        let _ = connection.execute(
            "UPDATE approvals SET state='failed', resolved_at=?1,
                    result_json='{\"success\":false,\"error\":\"argument integrity check failed\"}'
               WHERE id=?2",
            rusqlite::params![now_unix(), id],
        );
        return Err(Error::ApprovalNotPending);
    }

    // `canonicalize` only ever produces valid JSON, so this failing means the
    // stored column is corrupt. The row already moved to `claimed` above, so it
    // must be driven to a terminal state here — otherwise it stays `claimed`
    // until a restart sweep. Fail it closed with a valid structured result,
    // then surface the typed error.
    let parsed: Value = match serde_json::from_str(&canonical_args) {
        Ok(value) => value,
        Err(e) => {
            let _ = connection.execute(
                "UPDATE approvals SET state='failed', resolved_at=?1,
                        result_json='{\"success\":false,\"error\":\"stored canonical args are not valid JSON\"}'
                   WHERE id=?2 AND state='claimed'",
                rusqlite::params![now_unix(), id],
            );
            return Err(Error::Database(DatabaseError::Sqlite {
                message: format!("stored canonical args for approval {id} are not valid JSON: {e}"),
            }));
        }
    };
    Ok(ClaimedApproval {
        id: id.to_owned(),
        client_id,
        tool,
        correlation_id,
        canonical_args: parsed,
    })
}

/// Move a claimed approval to `executing`. Guards against a second executor.
pub fn mark_executing(id: &str) -> Result<()> {
    let connection = store::conn()?;
    let changed = connection.execute(
        "UPDATE approvals SET state='executing' WHERE id=?1 AND state='claimed'",
        rusqlite::params![id],
    )?;
    if changed == 0 {
        return Err(Error::ApprovalNotPending);
    }
    Ok(())
}

/// Record the terminal outcome of an execution. The result goes through
/// `sanitize_value` — redacted, structurally bounded, and guaranteed to be
/// valid JSON under the size cap — so a retry/poll can always parse it back and
/// a tool result that echoes configuration cannot leak a secret.
pub fn finish(id: &str, ok: bool, result: &Value) -> Result<()> {
    let connection = store::conn()?;
    let state = if ok { "done" } else { "failed" };
    let stored = serde_json::to_string(&audit::sanitize_value(result)).unwrap_or_else(|_| {
        "{\"success\":false,\"error\":\"result could not be serialized\"}".to_owned()
    });
    let changed = connection.execute(
        "UPDATE approvals SET state=?1, result_json=?2, resolved_at=?3
               WHERE id=?4 AND state='executing'",
        rusqlite::params![state, stored, now_unix(), id],
    )?;
    // Exactly one `executing` row must transition. Zero means the row is not in
    // `executing` (never claimed, already terminal, or recovered after a crash);
    // the outcome must not be silently dropped.
    if changed != 1 {
        return Err(Error::ApprovalNotPending);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn canonicalize_is_key_order_independent() {
        let a = json!({ "b": 1, "a": { "y": 2, "x": 3 } });
        let b = json!({ "a": { "x": 3, "y": 2 }, "b": 1 });
        assert_eq!(canonicalize(&a), canonicalize(&b));
    }

    #[test]
    fn canonicalize_differs_when_a_value_changes() {
        assert_ne!(
            canonicalize(&json!({ "sol": 1 })),
            canonicalize(&json!({ "sol": 9 }))
        );
    }

    // The state machine (exactly-once claim, digest binding, retry recovery,
    // denial persistence, crash recovery) is exercised against a real temp
    // database in `tests/agent_control_store.rs`.
}
