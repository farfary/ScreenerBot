//! AI Scheduled Tasks Type Definitions
//!
//! Contains all type definitions for scheduled AI tasks:
//! ScheduleType, TaskToolPermissions, ScheduledTask, RunStatus, TaskRun, AutomationStats.

use serde::{Deserialize, Serialize};

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
            _ => Err(format!("Unknown schedule type: {s}")),
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
            _ => Err(format!("Unknown tool permission: {s}")),
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
