// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Trades an agent connection submitted, and the outcome each one reached.
//!
//! A tool that sends an on-chain transaction answers an agent connection at
//! once with a trade id; the trade runs on its own task in the live process and
//! its result is stored here for `get_trade_status`. An agent call has a short
//! transport deadline and a swap can settle well past it, so the answer cannot
//! wait for the outcome.
//!
//! Identity: a submission is identified by the connection, the tool and the
//! digest of its canonical arguments. A repeat of the same call returns the
//! existing trade id instead of trading again while that trade still runs, and
//! for `REUSE_WINDOW` after it was submitted. The MCP call schema carries no
//! idempotency key, so a retry after a dropped answer can only be recognized by
//! what it asks for.
//!
//! Lifecycle: `submitted → done | failed`. A row still `submitted` at startup
//! belonged to a process that stopped mid-trade and becomes `interrupted`: its
//! swap may have been sent, so it is never replayed and its outcome is read
//! from the positions.

use std::time::Duration;

use rusqlite::{OptionalExtension, TransactionBehavior};
use serde::Serialize;
use serde_json::{json, Value};

use crate::agent_control::approvals::{canonicalize, digest_of};
use crate::agent_control::audit;
use crate::agent_control::error::{Error, Result};
use crate::agent_control::store::{self, now_unix};
use crate::errors::DatabaseError;

/// How long after a submission an identical call returns that trade instead of
/// trading again. A submission that is still running is reused at any age.
pub const REUSE_WINDOW: Duration = Duration::from_secs(10 * 60);
/// Finished submissions older than this are pruned.
const RETENTION_SECS: i64 = 24 * 60 * 60;
/// Hard cap on retained submissions regardless of age. A running one is never
/// pruned, so its outcome always has a row to land in.
const MAX_ROWS: i64 = 1_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SubmissionState {
    /// The trade is running.
    Submitted,
    /// The trade finished and reported success.
    Done,
    /// The trade finished and reported a failure.
    Failed,
    /// The process stopped while the trade ran. Its swap may have been sent.
    Interrupted,
}

impl SubmissionState {
    pub fn as_str(self) -> &'static str {
        match self {
            SubmissionState::Submitted => "submitted",
            SubmissionState::Done => "done",
            SubmissionState::Failed => "failed",
            SubmissionState::Interrupted => "interrupted",
        }
    }

    fn parse(value: &str) -> Option<SubmissionState> {
        Some(match value {
            "submitted" => SubmissionState::Submitted,
            "done" => SubmissionState::Done,
            "failed" => SubmissionState::Failed,
            "interrupted" => SubmissionState::Interrupted,
            _ => return None,
        })
    }
}

/// One submission as the bridge and the status tool report it.
#[derive(Debug, Clone, Serialize)]
pub struct SubmissionHandle {
    pub trade_id: String,
    pub tool: String,
    pub state: SubmissionState,
    pub submitted_at: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finished_at: Option<i64>,
    /// The tool result, present once the trade finished.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    /// True only when this call inserted the row, so the caller must start the
    /// trade. A reused submission is already running or finished.
    #[serde(skip)]
    pub created: bool,
}

type SubmissionRow = (String, String, String, i64, Option<i64>, Option<String>);

fn handle_from_row(row: SubmissionRow, created: bool) -> Result<SubmissionHandle> {
    let (trade_id, tool, state, submitted_at, finished_at, result_json) = row;
    let state = SubmissionState::parse(&state).ok_or_else(|| {
        Error::Database(DatabaseError::Sqlite {
            message: format!("stored submission state {state:?} is not recognised"),
        })
    })?;
    let result = result_json.map(|raw| {
        serde_json::from_str::<Value>(&raw).unwrap_or_else(
            |_| json!({ "success": false, "error": "stored result could not be read" }),
        )
    });
    Ok(SubmissionHandle {
        trade_id,
        tool,
        state,
        submitted_at,
        finished_at,
        result,
        created,
    })
}

const SELECT_COLUMNS: &str = "id, tool, state, created_at, finished_at, result_json";

fn read_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<SubmissionRow> {
    Ok((
        row.get(0)?,
        row.get(1)?,
        row.get(2)?,
        row.get(3)?,
        row.get(4)?,
        row.get(5)?,
    ))
}

/// Record a new submission, or return the one an identical call already made.
///
/// Race-safe: the lookup and the insert run in one immediate transaction, so
/// concurrent identical calls all land on one row and exactly one of them sees
/// `created`.
pub fn submit_or_reuse(
    client_id: &str,
    tool: &str,
    args: &Value,
    correlation_id: &str,
) -> Result<SubmissionHandle> {
    let digest = digest_of(&canonicalize(args));
    let now = now_unix();
    let mut connection = store::conn()?;
    let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;

    let existing = transaction
        .query_row(
            &format!(
                "SELECT {SELECT_COLUMNS} FROM submissions
                  WHERE client_id = ?1 AND tool = ?2 AND args_digest = ?3
                    AND (state = 'submitted' OR created_at > ?4)
                  ORDER BY created_at DESC LIMIT 1"
            ),
            rusqlite::params![client_id, tool, digest, now - REUSE_WINDOW.as_secs() as i64],
            read_row,
        )
        .optional()?;
    if let Some(row) = existing {
        transaction.commit()?;
        return handle_from_row(row, false);
    }

    let trade_id = uuid::Uuid::new_v4().to_string();
    transaction.execute(
        "INSERT INTO submissions
               (id, client_id, tool, args_digest, args_summary, correlation_id, state, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'submitted', ?7)",
        rusqlite::params![
            trade_id,
            client_id,
            tool,
            digest,
            audit::sanitize(args),
            correlation_id,
            now
        ],
    )?;
    prune(&transaction)?;
    transaction.commit()?;

    Ok(SubmissionHandle {
        trade_id,
        tool: tool.to_owned(),
        state: SubmissionState::Submitted,
        submitted_at: now,
        finished_at: None,
        result: None,
        created: true,
    })
}

/// Record the outcome of a running submission. Returns false when the row is
/// not running any more (an interrupted row recovered at startup keeps its
/// state). The result is stored through `audit::sanitize_value`, so it is
/// redacted, bounded and always valid JSON.
pub fn finish(trade_id: &str, ok: bool, result: &Value) -> Result<bool> {
    let connection = store::conn()?;
    let state = if ok {
        SubmissionState::Done
    } else {
        SubmissionState::Failed
    };
    let stored = serde_json::to_string(&audit::sanitize_value(result)).unwrap_or_else(|_| {
        "{\"success\":false,\"error\":\"result could not be serialized\"}".to_owned()
    });
    let changed = connection.execute(
        "UPDATE submissions SET state = ?1, result_json = ?2, finished_at = ?3
          WHERE id = ?4 AND state = 'submitted'",
        rusqlite::params![state.as_str(), stored, now_unix(), trade_id],
    )?;
    Ok(changed == 1)
}

/// One submission by its trade id, or `None` when no such trade is retained.
pub fn view(trade_id: &str) -> Result<Option<SubmissionHandle>> {
    let connection = store::conn()?;
    let row = connection
        .query_row(
            &format!("SELECT {SELECT_COLUMNS} FROM submissions WHERE id = ?1"),
            rusqlite::params![trade_id],
            read_row,
        )
        .optional()?;
    row.map(|row| handle_from_row(row, false)).transpose()
}

/// Mark every submission still running as interrupted. Only valid at startup,
/// before any trade of this process could have been submitted.
pub(crate) fn recover_interrupted(connection: &rusqlite::Connection) -> Result<usize> {
    Ok(connection.execute(
        "UPDATE submissions
            SET state = 'interrupted', finished_at = ?1,
                result_json = '{\"success\":false,\"error\":\"the app stopped while this trade ran; its swap may have been sent, read the positions for its outcome\"}'
          WHERE state = 'submitted'",
        rusqlite::params![now_unix()],
    )?)
}

/// Drop finished submissions past the retention window, then trim finished
/// rows to the hard cap, oldest first. Running submissions are kept.
pub(crate) fn prune(connection: &rusqlite::Connection) -> Result<()> {
    connection.execute(
        "DELETE FROM submissions WHERE state != 'submitted' AND created_at < ?1",
        rusqlite::params![now_unix() - RETENTION_SECS],
    )?;
    connection.execute(
        "DELETE FROM submissions
          WHERE state != 'submitted'
            AND id IN (
                SELECT id FROM submissions
                 ORDER BY created_at DESC, rowid DESC
                 LIMIT -1 OFFSET ?1
            )",
        rusqlite::params![MAX_ROWS],
    )?;
    Ok(())
}
