//! AI Scheduled Tasks Database Module
//!
//! Manages scheduled AI task definitions and execution history.
//! Tasks run AI instructions on a schedule (interval/daily/weekly)
//! using the ChatEngine in headless mode.

use chrono::Datelike;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::ai::chat_db::get_chat_pool;

// ─── Types ───────────────────────────────────────────────────────────

/// Schedule type for a task
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ScheduleType {
    Interval,
    Daily,
    Weekly,
}

impl ScheduleType {
    pub fn as_str(&self) -> &str {
        match self {
            ScheduleType::Interval => "interval",
            ScheduleType::Daily => "daily",
            ScheduleType::Weekly => "weekly",
        }
    }

    pub fn from_str(s: &str) -> Result<Self, String> {
        match s {
            "interval" => Ok(ScheduleType::Interval),
            "daily" => Ok(ScheduleType::Daily),
            "weekly" => Ok(ScheduleType::Weekly),
            _ => Err(format!("Unknown schedule type: {}", s)),
        }
    }
}

/// Tool permission mode for scheduled tasks
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TaskToolPermissions {
    ReadOnly,
    Full,
}

impl TaskToolPermissions {
    pub fn as_str(&self) -> &str {
        match self {
            TaskToolPermissions::ReadOnly => "read_only",
            TaskToolPermissions::Full => "full",
        }
    }

    pub fn from_str(s: &str) -> Result<Self, String> {
        match s {
            "read_only" => Ok(TaskToolPermissions::ReadOnly),
            "full" => Ok(TaskToolPermissions::Full),
            _ => Err(format!("Unknown tool permission: {}", s)),
        }
    }
}

/// A scheduled AI task definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledTask {
    pub id: i64,
    pub name: String,
    pub instruction: String,
    pub instruction_ids: Option<String>,
    pub schedule_type: String,
    pub schedule_value: String,
    pub tool_permissions: String,
    pub priority: String,
    pub notify_telegram: bool,
    pub notify_on_success: bool,
    pub notify_on_failure: bool,
    pub enabled: bool,
    pub max_retries: i32,
    pub timeout_seconds: i64,
    pub last_run_at: Option<String>,
    pub next_run_at: Option<String>,
    pub run_count: i64,
    pub error_count: i64,
    pub created_at: String,
    pub updated_at: String,
}

/// Run status for a task execution
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    Running,
    Success,
    Failed,
    Timeout,
    Skipped,
}

impl RunStatus {
    pub fn as_str(&self) -> &str {
        match self {
            RunStatus::Running => "running",
            RunStatus::Success => "success",
            RunStatus::Failed => "failed",
            RunStatus::Timeout => "timeout",
            RunStatus::Skipped => "skipped",
        }
    }
}

/// A task execution record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRun {
    pub id: i64,
    pub task_id: i64,
    pub status: String,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub duration_ms: Option<f64>,
    pub ai_response: Option<String>,
    pub tool_calls: Option<String>,
    pub tokens_used: Option<i64>,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub error_message: Option<String>,
    pub session_id: Option<i64>,
}

// ─── Schema ──────────────────────────────────────────────────────────

/// Initialize scheduled tasks tables in the chat database
pub fn initialize_scheduled_tables(conn: &rusqlite::Connection) -> Result<(), String> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS ai_scheduled_tasks (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            instruction TEXT NOT NULL,
            instruction_ids TEXT,
            schedule_type TEXT NOT NULL,
            schedule_value TEXT NOT NULL,
            tool_permissions TEXT NOT NULL DEFAULT 'read_only',
            priority TEXT NOT NULL DEFAULT 'low',
            notify_telegram INTEGER NOT NULL DEFAULT 1,
            notify_on_success INTEGER NOT NULL DEFAULT 1,
            notify_on_failure INTEGER NOT NULL DEFAULT 1,
            enabled INTEGER NOT NULL DEFAULT 1,
            max_retries INTEGER NOT NULL DEFAULT 2,
            timeout_seconds INTEGER NOT NULL DEFAULT 120,
            last_run_at TEXT,
            next_run_at TEXT,
            run_count INTEGER NOT NULL DEFAULT 0,
            error_count INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        )",
        [],
    )
    .map_err(|e| format!("Failed to create ai_scheduled_tasks table: {}", e))?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS ai_task_runs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            task_id INTEGER NOT NULL,
            status TEXT NOT NULL,
            started_at TEXT NOT NULL,
            completed_at TEXT,
            duration_ms REAL,
            ai_response TEXT,
            tool_calls TEXT,
            tokens_used INTEGER,
            provider TEXT,
            model TEXT,
            error_message TEXT,
            session_id INTEGER,
            FOREIGN KEY (task_id) REFERENCES ai_scheduled_tasks(id) ON DELETE CASCADE
        )",
        [],
    )
    .map_err(|e| format!("Failed to create ai_task_runs table: {}", e))?;

    // Indexes
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_scheduled_tasks_enabled 
         ON ai_scheduled_tasks(enabled, next_run_at)",
        [],
    )
    .map_err(|e| format!("Failed to create scheduled tasks index: {}", e))?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_task_runs_task 
         ON ai_task_runs(task_id, started_at DESC)",
        [],
    )
    .map_err(|e| format!("Failed to create task runs index: {}", e))?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_task_runs_status 
         ON ai_task_runs(status, started_at DESC)",
        [],
    )
    .map_err(|e| format!("Failed to create task runs status index: {}", e))?;

    Ok(())
}

// ─── Task CRUD ───────────────────────────────────────────────────────

/// Create a new scheduled task
pub fn create_task(
    pool: &Pool<SqliteConnectionManager>,
    name: &str,
    instruction: &str,
    schedule_type: &str,
    schedule_value: &str,
    tool_permissions: Option<&str>,
    priority: Option<&str>,
) -> Result<i64, String> {
    let conn = pool
        .get()
        .map_err(|e| format!("Failed to get connection: {}", e))?;
    let now = chrono::Utc::now().to_rfc3339();
    let tool_perms = tool_permissions.unwrap_or("read_only");
    let prio = priority.unwrap_or("low");

    // Calculate initial next_run_at
    let next_run = calculate_next_run(schedule_type, schedule_value, None)?;

    conn.execute(
        "INSERT INTO ai_scheduled_tasks (name, instruction, schedule_type, schedule_value, 
         tool_permissions, priority, next_run_at, created_at, updated_at) 
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            name,
            instruction,
            schedule_type,
            schedule_value,
            tool_perms,
            prio,
            &next_run,
            &now,
            &now
        ],
    )
    .map_err(|e| format!("Failed to create scheduled task: {}", e))?;

    Ok(conn.last_insert_rowid())
}

/// Get all scheduled tasks
pub fn list_tasks(pool: &Pool<SqliteConnectionManager>) -> Result<Vec<ScheduledTask>, String> {
    let conn = pool
        .get()
        .map_err(|e| format!("Failed to get connection: {}", e))?;

    let mut stmt = conn
        .prepare(
            "SELECT id, name, instruction, instruction_ids, schedule_type, schedule_value,
                    tool_permissions, priority, notify_telegram, notify_on_success, notify_on_failure,
                    enabled, max_retries, timeout_seconds, last_run_at, next_run_at,
                    run_count, error_count, created_at, updated_at
             FROM ai_scheduled_tasks
             ORDER BY enabled DESC, created_at DESC",
        )
        .map_err(|e| format!("Failed to prepare statement: {}", e))?;

    let tasks = stmt
        .query_map([], |row| {
            Ok(ScheduledTask {
                id: row.get(0)?,
                name: row.get(1)?,
                instruction: row.get(2)?,
                instruction_ids: row.get(3)?,
                schedule_type: row.get(4)?,
                schedule_value: row.get(5)?,
                tool_permissions: row.get(6)?,
                priority: row.get(7)?,
                notify_telegram: row.get::<_, i32>(8)? != 0,
                notify_on_success: row.get::<_, i32>(9)? != 0,
                notify_on_failure: row.get::<_, i32>(10)? != 0,
                enabled: row.get::<_, i32>(11)? != 0,
                max_retries: row.get(12)?,
                timeout_seconds: row.get(13)?,
                last_run_at: row.get(14)?,
                next_run_at: row.get(15)?,
                run_count: row.get(16)?,
                error_count: row.get(17)?,
                created_at: row.get(18)?,
                updated_at: row.get(19)?,
            })
        })
        .map_err(|e| format!("Failed to query tasks: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to collect tasks: {}", e))?;

    Ok(tasks)
}

/// Get a single scheduled task
pub fn get_task(
    pool: &Pool<SqliteConnectionManager>,
    id: i64,
) -> Result<Option<ScheduledTask>, String> {
    let conn = pool
        .get()
        .map_err(|e| format!("Failed to get connection: {}", e))?;

    let mut stmt = conn
        .prepare(
            "SELECT id, name, instruction, instruction_ids, schedule_type, schedule_value,
                    tool_permissions, priority, notify_telegram, notify_on_success, notify_on_failure,
                    enabled, max_retries, timeout_seconds, last_run_at, next_run_at,
                    run_count, error_count, created_at, updated_at
             FROM ai_scheduled_tasks WHERE id = ?1",
        )
        .map_err(|e| format!("Failed to prepare statement: {}", e))?;

    let task = stmt
        .query_row(params![id], |row| {
            Ok(ScheduledTask {
                id: row.get(0)?,
                name: row.get(1)?,
                instruction: row.get(2)?,
                instruction_ids: row.get(3)?,
                schedule_type: row.get(4)?,
                schedule_value: row.get(5)?,
                tool_permissions: row.get(6)?,
                priority: row.get(7)?,
                notify_telegram: row.get::<_, i32>(8)? != 0,
                notify_on_success: row.get::<_, i32>(9)? != 0,
                notify_on_failure: row.get::<_, i32>(10)? != 0,
                enabled: row.get::<_, i32>(11)? != 0,
                max_retries: row.get(12)?,
                timeout_seconds: row.get(13)?,
                last_run_at: row.get(14)?,
                next_run_at: row.get(15)?,
                run_count: row.get(16)?,
                error_count: row.get(17)?,
                created_at: row.get(18)?,
                updated_at: row.get(19)?,
            })
        })
        .optional()
        .map_err(|e| format!("Failed to query task: {}", e))?;

    Ok(task)
}

/// Update a scheduled task
pub fn update_task(
    pool: &Pool<SqliteConnectionManager>,
    id: i64,
    name: Option<&str>,
    instruction: Option<&str>,
    instruction_ids: Option<Option<&str>>,
    schedule_type: Option<&str>,
    schedule_value: Option<&str>,
    tool_permissions: Option<&str>,
    priority: Option<&str>,
    notify_telegram: Option<bool>,
    notify_on_success: Option<bool>,
    notify_on_failure: Option<bool>,
    max_retries: Option<i32>,
    timeout_seconds: Option<i64>,
) -> Result<(), String> {
    let conn = pool
        .get()
        .map_err(|e| format!("Failed to get connection: {}", e))?;
    let now = chrono::Utc::now().to_rfc3339();

    let mut updates = Vec::new();
    let mut param_values: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    if let Some(v) = name {
        updates.push("name = ?");
        param_values.push(Box::new(v.to_string()));
    }
    if let Some(v) = instruction {
        updates.push("instruction = ?");
        param_values.push(Box::new(v.to_string()));
    }
    if let Some(v) = instruction_ids {
        updates.push("instruction_ids = ?");
        param_values.push(Box::new(v.map(|s| s.to_string())));
    }
    if let Some(v) = schedule_type {
        updates.push("schedule_type = ?");
        param_values.push(Box::new(v.to_string()));
    }
    if let Some(v) = schedule_value {
        updates.push("schedule_value = ?");
        param_values.push(Box::new(v.to_string()));
    }
    if let Some(v) = tool_permissions {
        updates.push("tool_permissions = ?");
        param_values.push(Box::new(v.to_string()));
    }
    if let Some(v) = priority {
        updates.push("priority = ?");
        param_values.push(Box::new(v.to_string()));
    }
    if let Some(v) = notify_telegram {
        updates.push("notify_telegram = ?");
        param_values.push(Box::new(v as i32));
    }
    if let Some(v) = notify_on_success {
        updates.push("notify_on_success = ?");
        param_values.push(Box::new(v as i32));
    }
    if let Some(v) = notify_on_failure {
        updates.push("notify_on_failure = ?");
        param_values.push(Box::new(v as i32));
    }
    if let Some(v) = max_retries {
        updates.push("max_retries = ?");
        param_values.push(Box::new(v));
    }
    if let Some(v) = timeout_seconds {
        updates.push("timeout_seconds = ?");
        param_values.push(Box::new(v));
    }

    if updates.is_empty() {
        return Ok(());
    }

    updates.push("updated_at = ?");
    param_values.push(Box::new(now));

    let sql = format!(
        "UPDATE ai_scheduled_tasks SET {} WHERE id = ?",
        updates.join(", ")
    );
    param_values.push(Box::new(id));

    let params_refs: Vec<&dyn rusqlite::ToSql> = param_values.iter().map(|b| b.as_ref()).collect();
    conn.execute(&sql, params_refs.as_slice())
        .map_err(|e| format!("Failed to update task: {}", e))?;

    // Recalculate next_run_at if schedule changed
    if schedule_type.is_some() || schedule_value.is_some() {
        if let Ok(Some(task)) = get_task(pool, id) {
            match calculate_next_run(&task.schedule_type, &task.schedule_value, None) {
                Ok(next) => {
                    conn.execute(
                        "UPDATE ai_scheduled_tasks SET next_run_at = ? WHERE id = ?",
                        params![&next, id],
                    )
                    .map_err(|e| format!("Failed to update next_run_at: {}", e))?;
                }
                Err(e) => {
                    // Log warning but keep task - next_run_at stays at old value
                    crate::logger::warning(
                        crate::logger::LogTag::Api,
                        &format!(
                            "Failed to calculate next_run_at for task {}: {} - keeping old schedule",
                            id, e
                        ),
                    );
                }
            }
        }
    }

    Ok(())
}

/// Delete a scheduled task and its run history
pub fn delete_task(pool: &Pool<SqliteConnectionManager>, id: i64) -> Result<(), String> {
    let conn = pool
        .get()
        .map_err(|e| format!("Failed to get connection: {}", e))?;

    conn.execute_batch("BEGIN TRANSACTION")
        .map_err(|e| format!("Failed to begin transaction: {}", e))?;
    if let Err(e) = conn.execute("DELETE FROM ai_task_runs WHERE task_id = ?1", params![id]) {
        let _ = conn.execute_batch("ROLLBACK");
        return Err(format!("Failed to delete task runs: {}", e));
    }
    if let Err(e) = conn.execute("DELETE FROM ai_scheduled_tasks WHERE id = ?1", params![id]) {
        let _ = conn.execute_batch("ROLLBACK");
        return Err(format!("Failed to delete task: {}", e));
    }
    conn.execute_batch("COMMIT")
        .map_err(|e| format!("Failed to commit transaction: {}", e))?;

    Ok(())
}

/// Toggle task enabled/disabled
pub fn toggle_task(
    pool: &Pool<SqliteConnectionManager>,
    id: i64,
    enabled: bool,
) -> Result<(), String> {
    let conn = pool
        .get()
        .map_err(|e| format!("Failed to get connection: {}", e))?;
    let now = chrono::Utc::now().to_rfc3339();

    let next_run = if enabled {
        // Recalculate next_run when re-enabling
        if let Ok(Some(task)) = get_task(pool, id) {
            calculate_next_run(&task.schedule_type, &task.schedule_value, None).ok()
        } else {
            None
        }
    } else {
        None
    };

    conn.execute(
        "UPDATE ai_scheduled_tasks SET enabled = ?1, next_run_at = ?2, updated_at = ?3 WHERE id = ?4",
        params![enabled as i32, next_run, &now, id],
    )
    .map_err(|e| format!("Failed to toggle task: {}", e))?;

    Ok(())
}

// ─── Task Scheduling ─────────────────────────────────────────────────

/// Get tasks that are due for execution
pub fn get_due_tasks(pool: &Pool<SqliteConnectionManager>) -> Result<Vec<ScheduledTask>, String> {
    let conn = pool
        .get()
        .map_err(|e| format!("Failed to get connection: {}", e))?;
    let now = chrono::Utc::now().to_rfc3339();

    // Use a transaction to atomically select and mark tasks as picked up
    let tx = conn
        .unchecked_transaction()
        .map_err(|e| format!("Failed to start transaction: {}", e))?;

    let mut stmt = tx
        .prepare(
            "SELECT id, name, instruction, instruction_ids, schedule_type, schedule_value,
                    tool_permissions, priority, notify_telegram, notify_on_success, notify_on_failure,
                    enabled, max_retries, timeout_seconds, last_run_at, next_run_at,
                    run_count, error_count, created_at, updated_at
             FROM ai_scheduled_tasks
             WHERE enabled = 1 AND next_run_at IS NOT NULL AND next_run_at <= ?1
             ORDER BY next_run_at ASC",
        )
        .map_err(|e| format!("Failed to prepare statement: {}", e))?;

    let tasks = stmt
        .query_map(params![&now], |row| {
            Ok(ScheduledTask {
                id: row.get(0)?,
                name: row.get(1)?,
                instruction: row.get(2)?,
                instruction_ids: row.get(3)?,
                schedule_type: row.get(4)?,
                schedule_value: row.get(5)?,
                tool_permissions: row.get(6)?,
                priority: row.get(7)?,
                notify_telegram: row.get::<_, i32>(8)? != 0,
                notify_on_success: row.get::<_, i32>(9)? != 0,
                notify_on_failure: row.get::<_, i32>(10)? != 0,
                enabled: row.get::<_, i32>(11)? != 0,
                max_retries: row.get(12)?,
                timeout_seconds: row.get(13)?,
                last_run_at: row.get(14)?,
                next_run_at: row.get(15)?,
                run_count: row.get(16)?,
                error_count: row.get(17)?,
                created_at: row.get(18)?,
                updated_at: row.get(19)?,
            })
        })
        .map_err(|e| format!("Failed to query due tasks: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to collect due tasks: {}", e))?;

    // Mark picked-up tasks so another cycle won't grab them
    let task_ids: Vec<i64> = tasks.iter().map(|t| t.id).collect();
    for task_id in &task_ids {
        tx.execute(
            "UPDATE ai_scheduled_tasks SET next_run_at = NULL WHERE id = ?1",
            params![task_id],
        )
        .map_err(|e| format!("Failed to mark task as picked: {}", e))?;
    }

    // Need to drop the statement before committing
    drop(stmt);

    tx.commit()
        .map_err(|e| format!("Failed to commit transaction: {}", e))?;

    Ok(tasks)
}

/// Update task after a run completes
pub fn update_task_after_run(
    pool: &Pool<SqliteConnectionManager>,
    id: i64,
    success: bool,
) -> Result<(), String> {
    let conn = pool
        .get()
        .map_err(|e| format!("Failed to get connection: {}", e))?;
    let now = chrono::Utc::now().to_rfc3339();

    // Get the task to calculate next run
    let task = get_task(pool, id)?.ok_or_else(|| format!("Task {} not found", id))?;

    let next_run = calculate_next_run(&task.schedule_type, &task.schedule_value, None)?;

    if success {
        conn.execute(
            "UPDATE ai_scheduled_tasks SET last_run_at = ?1, next_run_at = ?2, 
             run_count = run_count + 1, updated_at = ?3 WHERE id = ?4",
            params![&now, &next_run, &now, id],
        )
        .map_err(|e| format!("Failed to update task after success: {}", e))?;
    } else {
        conn.execute(
            "UPDATE ai_scheduled_tasks SET last_run_at = ?1, next_run_at = ?2, 
             run_count = run_count + 1, error_count = error_count + 1, updated_at = ?3 WHERE id = ?4",
            params![&now, &next_run, &now, id],
        )
        .map_err(|e| format!("Failed to update task after failure: {}", e))?;
    }

    Ok(())
}

/// Calculate the next run time based on schedule
pub fn calculate_next_run(
    schedule_type: &str,
    schedule_value: &str,
    _from: Option<&str>,
) -> Result<String, String> {
    let now = chrono::Utc::now();

    match schedule_type {
        "interval" => {
            let seconds: u64 = schedule_value
                .parse()
                .map_err(|e| format!("Invalid interval value '{}': {}", schedule_value, e))?;
            if seconds < 60 {
                return Err(format!("Interval must be at least 60 seconds, got {}", seconds));
            }
            let next = now + chrono::Duration::seconds(seconds as i64);
            Ok(next.to_rfc3339())
        }
        "daily" => {
            // schedule_value format: "HH:MM"
            // NOTE: All times are treated as UTC. No timezone conversion is performed.
            // Users should specify times in UTC format.
            let parts: Vec<&str> = schedule_value.split(':').collect();
            if parts.len() != 2 {
                return Err(format!("Invalid daily schedule format: {}", schedule_value));
            }
            let hour: u32 = parts[0].parse().map_err(|_| "Invalid hour")?;
            let minute: u32 = parts[1].parse().map_err(|_| "Invalid minute")?;

            let today = now
                .date_naive()
                .and_hms_opt(hour, minute, 0)
                .ok_or("Invalid time")?;
            let today_utc =
                chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(today, chrono::Utc);

            let next = if today_utc > now {
                today_utc
            } else {
                today_utc + chrono::Duration::days(1)
            };
            Ok(next.to_rfc3339())
        }
        "weekly" => {
            // schedule_value format: "mon,wed,fri:HH:MM"
            let parts: Vec<&str> = schedule_value.splitn(2, ':').collect();
            if parts.len() < 2 {
                return Err(format!(
                    "Invalid weekly schedule format: {}",
                    schedule_value
                ));
            }

            let days_str = parts[0];
            let time_str = if parts.len() > 1 { parts[1] } else { "00:00" };
            let time_parts: Vec<&str> = time_str.splitn(2, ':').collect();
            let hour: u32 = time_parts.first().unwrap_or(&"0").parse().unwrap_or(0);
            let minute: u32 = time_parts.get(1).unwrap_or(&"0").parse().unwrap_or(0);

            let day_names: Vec<&str> = days_str.split(',').map(|s| s.trim()).collect();
            let target_weekdays: Vec<chrono::Weekday> = day_names
                .iter()
                .filter_map(|d| match d.to_lowercase().as_str() {
                    "mon" => Some(chrono::Weekday::Mon),
                    "tue" => Some(chrono::Weekday::Tue),
                    "wed" => Some(chrono::Weekday::Wed),
                    "thu" => Some(chrono::Weekday::Thu),
                    "fri" => Some(chrono::Weekday::Fri),
                    "sat" => Some(chrono::Weekday::Sat),
                    "sun" => Some(chrono::Weekday::Sun),
                    _ => None,
                })
                .collect();

            if target_weekdays.is_empty() {
                return Err("No valid days specified".to_string());
            }

            // Find the next matching day
            for offset in 0..=7 {
                let candidate_date = (now + chrono::Duration::days(offset)).date_naive();
                let candidate_time = candidate_date
                    .and_hms_opt(hour, minute, 0)
                    .ok_or("Invalid time")?;
                let candidate_utc = chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(
                    candidate_time,
                    chrono::Utc,
                );

                if target_weekdays.contains(&candidate_date.weekday()) && candidate_utc > now {
                    return Ok(candidate_utc.to_rfc3339());
                }
            }

            // Should not be reachable if target_weekdays is non-empty
            Err("Could not calculate next weekly run time".to_string())
        }
        _ => Err(format!("Unknown schedule type: {}", schedule_type)),
    }
}

// ─── Run History ─────────────────────────────────────────────────────

/// Record the start of a task run
pub fn record_run_start(
    pool: &Pool<SqliteConnectionManager>,
    task_id: i64,
    session_id: Option<i64>,
) -> Result<i64, String> {
    let conn = pool
        .get()
        .map_err(|e| format!("Failed to get connection: {}", e))?;
    let now = chrono::Utc::now().to_rfc3339();

    conn.execute(
        "INSERT INTO ai_task_runs (task_id, status, started_at, session_id) VALUES (?1, ?2, ?3, ?4)",
        params![task_id, RunStatus::Running.as_str(), &now, session_id],
    )
    .map_err(|e| format!("Failed to record run start: {}", e))?;

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
        .map_err(|e| format!("Failed to get connection: {}", e))?;
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
    .map_err(|e| format!("Failed to record run completion: {}", e))?;

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
        .map_err(|e| format!("Failed to get connection: {}", e))?;

    // Clamp limit to reasonable bounds
    let limit = limit.min(100).max(1);

    let mut stmt = conn
        .prepare(
            "SELECT id, task_id, status, started_at, completed_at, duration_ms,
                    ai_response, tool_calls, tokens_used, provider, model, error_message, session_id
             FROM ai_task_runs
             WHERE task_id = ?1
             ORDER BY started_at DESC
             LIMIT ?2",
        )
        .map_err(|e| format!("Failed to prepare statement: {}", e))?;

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
        .map_err(|e| format!("Failed to query runs: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to collect runs: {}", e))?;

    Ok(runs)
}

/// Get all recent runs across all tasks
pub fn list_recent_runs(
    pool: &Pool<SqliteConnectionManager>,
    limit: i64,
) -> Result<Vec<TaskRun>, String> {
    let conn = pool
        .get()
        .map_err(|e| format!("Failed to get connection: {}", e))?;

    // Clamp limit to reasonable bounds
    let limit = limit.min(100).max(1);

    let mut stmt = conn
        .prepare(
            "SELECT id, task_id, status, started_at, completed_at, duration_ms,
                    ai_response, tool_calls, tokens_used, provider, model, error_message, session_id
             FROM ai_task_runs
             ORDER BY started_at DESC
             LIMIT ?1",
        )
        .map_err(|e| format!("Failed to prepare statement: {}", e))?;

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
        .map_err(|e| format!("Failed to query recent runs: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to collect recent runs: {}", e))?;

    Ok(runs)
}

/// Get a specific run by ID
pub fn get_run(
    pool: &Pool<SqliteConnectionManager>,
    run_id: i64,
) -> Result<Option<TaskRun>, String> {
    let conn = pool
        .get()
        .map_err(|e| format!("Failed to get connection: {}", e))?;

    let mut stmt = conn
        .prepare(
            "SELECT id, task_id, status, started_at, completed_at, duration_ms,
                    ai_response, tool_calls, tokens_used, provider, model, error_message, session_id
             FROM ai_task_runs WHERE id = ?1",
        )
        .map_err(|e| format!("Failed to prepare statement: {}", e))?;

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
        .map_err(|e| format!("Failed to query run: {}", e))?;

    Ok(run)
}

/// Get aggregated stats for automation
pub fn get_automation_stats(
    pool: &Pool<SqliteConnectionManager>,
) -> Result<AutomationStats, String> {
    let conn = pool
        .get()
        .map_err(|e| format!("Failed to get connection: {}", e))?;

    let total_tasks: i64 = conn
        .query_row("SELECT COUNT(*) FROM ai_scheduled_tasks", [], |row| {
            row.get(0)
        })
        .unwrap_or(0);

    let active_tasks: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM ai_scheduled_tasks WHERE enabled = 1",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    let total_runs: i64 = conn
        .query_row("SELECT COUNT(*) FROM ai_task_runs", [], |row| row.get(0))
        .unwrap_or(0);

    let successful_runs: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM ai_task_runs WHERE status = 'success'",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    let failed_runs: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM ai_task_runs WHERE status = 'failed' OR status = 'timeout'",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    let avg_duration: f64 = conn
        .query_row(
            "SELECT COALESCE(AVG(duration_ms), 0) FROM ai_task_runs WHERE status = 'success'",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0.0);

    // Runs in last 24 hours
    let runs_today: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM ai_task_runs WHERE started_at >= datetime('now', '-1 day')",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

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

/// Aggregated automation statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomationStats {
    pub total_tasks: i64,
    pub active_tasks: i64,
    pub total_runs: i64,
    pub successful_runs: i64,
    pub failed_runs: i64,
    pub success_rate: f64,
    pub avg_duration_ms: f64,
    pub runs_today: i64,
}

/// Clean up old run history
pub fn cleanup_old_runs(
    pool: &Pool<SqliteConnectionManager>,
    keep_days: i64,
) -> Result<usize, String> {
    let conn = pool
        .get()
        .map_err(|e| format!("Failed to get connection: {}", e))?;

    let deleted = conn
        .execute(
            "DELETE FROM ai_task_runs WHERE started_at < datetime('now', ?1)",
            params![format!("-{} days", keep_days)],
        )
        .map_err(|e| format!("Failed to cleanup old runs: {}", e))?;

    Ok(deleted)
}
