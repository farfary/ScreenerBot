//! AI Scheduled Tasks Run History
//!
//! Manages execution history for scheduled AI tasks:
//! recording run start/completion, querying run history, stats, and cleanup.

use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{params, OptionalExtension};

use super::scheduled_types::{AutomationStats, RunStatus, TaskRun};

// ─── Run History ─────────────────────────────────────────────────────

/// Record the start of a task run
pub fn record_run_start(
    pool: &Pool<SqliteConnectionManager>,
    task_id: i64,
    session_id: Option<i64>,
) -> Result<i64, String> {
    let conn = pool
        .get()
        .map_err(|e| format!("Failed to get connection: {e}"))?;
    let now = chrono::Utc::now().to_rfc3339();

    conn.execute(
        "INSERT INTO ai_task_runs (task_id, status, started_at, session_id) VALUES (?1, ?2, ?3, ?4)",
        params![task_id, RunStatus::Running.as_str(), &now, session_id],
    )
    .map_err(|e| format!("Failed to record run start: {e}"))?;

    Ok(conn.last_insert_rowid())
}

/// Record the completion of a task run
pub fn record_run_complete(
    pool: &Pool<SqliteConnectionManager>,
    run_id: i64,
    status: &str,
    ai_response: Option<&str>,
    tool_calls: Option<&str>,
    tokens_used: Option<i64>,
    provider: Option<&str>,
    model: Option<&str>,
    error_message: Option<&str>,
    duration_ms: f64,
) -> Result<(), String> {
    let conn = pool
        .get()
        .map_err(|e| format!("Failed to get connection: {e}"))?;
    let now = chrono::Utc::now().to_rfc3339();

    conn.execute(
        "UPDATE ai_task_runs SET status = ?1, completed_at = ?2, duration_ms = ?3, 
         ai_response = ?4, tool_calls = ?5, tokens_used = ?6, provider = ?7, 
         model = ?8, error_message = ?9 WHERE id = ?10",
        params![
            status,
            &now,
            duration_ms,
            ai_response,
            tool_calls,
            tokens_used,
            provider,
            model,
            error_message,
            run_id
        ],
    )
    .map_err(|e| format!("Failed to record run completion: {e}"))?;

    Ok(())
}

/// Get run history for a specific task
pub fn list_runs_for_task(
    pool: &Pool<SqliteConnectionManager>,
    task_id: i64,
    limit: i64,
) -> Result<Vec<TaskRun>, String> {
    let conn = pool
        .get()
        .map_err(|e| format!("Failed to get connection: {e}"))?;

    // Clamp limit to reasonable bounds
    let limit = limit.clamp(1, 100);

    let mut stmt = conn
        .prepare(
            "SELECT id, task_id, status, started_at, completed_at, duration_ms,
                    ai_response, tool_calls, tokens_used, provider, model, error_message, session_id
             FROM ai_task_runs
             WHERE task_id = ?1
             ORDER BY started_at DESC
             LIMIT ?2",
        )
        .map_err(|e| format!("Failed to prepare statement: {e}"))?;

    let runs = stmt
        .query_map(params![task_id, limit], |row| {
            Ok(TaskRun {
                id: row.get(0)?,
                task_id: row.get(1)?,
                status: row.get(2)?,
                started_at: row.get(3)?,
                completed_at: row.get(4)?,
                duration_ms: row.get(5)?,
                ai_response: row.get(6)?,
                tool_calls: row.get(7)?,
                tokens_used: row.get(8)?,
                provider: row.get(9)?,
                model: row.get(10)?,
                error_message: row.get(11)?,
                session_id: row.get(12)?,
            })
        })
        .map_err(|e| format!("Failed to query runs: {e}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to collect runs: {e}"))?;

    Ok(runs)
}

/// Get all recent runs across all tasks
pub fn list_recent_runs(
    pool: &Pool<SqliteConnectionManager>,
    limit: i64,
) -> Result<Vec<TaskRun>, String> {
    let conn = pool
        .get()
        .map_err(|e| format!("Failed to get connection: {e}"))?;

    // Clamp limit to reasonable bounds
    let limit = limit.clamp(1, 100);

    let mut stmt = conn
        .prepare(
            "SELECT id, task_id, status, started_at, completed_at, duration_ms,
                    ai_response, tool_calls, tokens_used, provider, model, error_message, session_id
             FROM ai_task_runs
             ORDER BY started_at DESC
             LIMIT ?1",
        )
        .map_err(|e| format!("Failed to prepare statement: {e}"))?;

    let runs = stmt
        .query_map(params![limit], |row| {
            Ok(TaskRun {
                id: row.get(0)?,
                task_id: row.get(1)?,
                status: row.get(2)?,
                started_at: row.get(3)?,
                completed_at: row.get(4)?,
                duration_ms: row.get(5)?,
                ai_response: row.get(6)?,
                tool_calls: row.get(7)?,
                tokens_used: row.get(8)?,
                provider: row.get(9)?,
                model: row.get(10)?,
                error_message: row.get(11)?,
                session_id: row.get(12)?,
            })
        })
        .map_err(|e| format!("Failed to query recent runs: {e}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to collect recent runs: {e}"))?;

    Ok(runs)
}

/// Get a specific run by ID
pub fn get_run(
    pool: &Pool<SqliteConnectionManager>,
    run_id: i64,
) -> Result<Option<TaskRun>, String> {
    let conn = pool
        .get()
        .map_err(|e| format!("Failed to get connection: {e}"))?;

    let mut stmt = conn
        .prepare(
            "SELECT id, task_id, status, started_at, completed_at, duration_ms,
                    ai_response, tool_calls, tokens_used, provider, model, error_message, session_id
             FROM ai_task_runs WHERE id = ?1",
        )
        .map_err(|e| format!("Failed to prepare statement: {e}"))?;

    let run = stmt
        .query_row(params![run_id], |row| {
            Ok(TaskRun {
                id: row.get(0)?,
                task_id: row.get(1)?,
                status: row.get(2)?,
                started_at: row.get(3)?,
                completed_at: row.get(4)?,
                duration_ms: row.get(5)?,
                ai_response: row.get(6)?,
                tool_calls: row.get(7)?,
                tokens_used: row.get(8)?,
                provider: row.get(9)?,
                model: row.get(10)?,
                error_message: row.get(11)?,
                session_id: row.get(12)?,
            })
        })
        .optional()
        .map_err(|e| format!("Failed to query run: {e}"))?;

    Ok(run)
}

/// Get aggregated stats for automation
pub fn get_automation_stats(
    pool: &Pool<SqliteConnectionManager>,
) -> Result<AutomationStats, String> {
    let conn = pool
        .get()
        .map_err(|e| format!("Failed to get connection: {e}"))?;

    let total_tasks: i64 = conn
        .query_row("SELECT COUNT(*) FROM ai_scheduled_tasks", [], |row| {
            row.get(0)
        })
        .unwrap_or_default();

    let active_tasks: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM ai_scheduled_tasks WHERE enabled = 1",
            [],
            |row| row.get(0),
        )
        .unwrap_or_default();

    let total_runs: i64 = conn
        .query_row("SELECT COUNT(*) FROM ai_task_runs", [], |row| row.get(0))
        .unwrap_or_default();

    let successful_runs: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM ai_task_runs WHERE status = 'success'",
            [],
            |row| row.get(0),
        )
        .unwrap_or_default();

    let failed_runs: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM ai_task_runs WHERE status = 'failed' OR status = 'timeout'",
            [],
            |row| row.get(0),
        )
        .unwrap_or_default();

    let avg_duration: f64 = conn
        .query_row(
            "SELECT COALESCE(AVG(duration_ms), 0) FROM ai_task_runs WHERE status = 'success'",
            [],
            |row| row.get(0),
        )
        .unwrap_or_default();

    // Runs in last 24 hours
    let runs_today: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM ai_task_runs WHERE started_at >= datetime('now', '-1 day')",
            [],
            |row| row.get(0),
        )
        .unwrap_or_default();

    let success_rate = if total_runs > 0 {
        (successful_runs as f64 / total_runs as f64) * 100.0
    } else {
        0.0
    };

    Ok(AutomationStats {
        total_tasks,
        active_tasks,
        total_runs,
        successful_runs,
        failed_runs,
        success_rate,
        avg_duration_ms: avg_duration,
        runs_today,
    })
}

/// Clean up old run history
pub fn cleanup_old_runs(
    pool: &Pool<SqliteConnectionManager>,
    keep_days: i64,
) -> Result<usize, String> {
    let conn = pool
        .get()
        .map_err(|e| format!("Failed to get connection: {e}"))?;

    let deleted = conn
        .execute(
            "DELETE FROM ai_task_runs WHERE started_at < datetime('now', ?1)",
            params![format!("-{keep_days} days")],
        )
        .map_err(|e| format!("Failed to cleanup old runs: {e}"))?;

    Ok(deleted)
}
