//! Assistant scheduled-conversation automation submodule.
//!
//! Organized into:
//! - `types` — Type definitions (ScheduledTask, TaskRun, etc.)
//! - `database` — Schema initialization, task CRUD, and scheduling logic
//! - `runs` — Run history recording and querying
//! - `worker` — Scheduler worker loop and task execution

pub mod database;
pub mod runs;
pub mod types;
pub mod worker;

// Re-export types
pub use types::{
    AutomationStats, RunStatus, ScheduleType, ScheduledTask, TaskRun, TaskToolPermissions,
};

// Re-export database functions
pub use database::{
    calculate_next_run, create_task, delete_task, get_due_tasks, get_task,
    initialize_scheduled_tables, list_tasks, toggle_task, update_task, update_task_after_run,
};

// Re-export run functions
pub use runs::{
    cleanup_old_runs, get_automation_stats, get_run, list_recent_runs, list_runs_for_task,
    record_run_complete, record_run_start,
};

// Re-export worker functions
pub use worker::{execute_task_public, scheduler_worker};
