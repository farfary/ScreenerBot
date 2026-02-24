//! AI automation scheduled tasks handlers

use axum::{extract::Path, http::StatusCode, response::Response, Json};

use crate::logger::{self, LogTag};
use crate::webserver::utils::{error_response, success_response};

use super::types::*;

/// GET /api/ai/automation — List all scheduled tasks
pub async fn list_automation_tasks() -> Response {
    let pool = match crate::ai::chat_db::get_chat_pool() {
        Some(p) => p,
        None => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "DB_ERROR",
                "Database not initialized",
                None,
            )
        }
    };

    match crate::ai::scheduled_db::list_tasks(&pool) {
        Ok(tasks) => success_response(serde_json::json!({ "tasks": tasks })),
        Err(e) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DB_ERROR",
            &format!("Failed to list tasks: {e}"),
            None,
        ),
    }
}

/// POST /api/ai/automation — Create a new scheduled task
pub async fn create_automation_task(Json(req): Json<CreateAutomationTaskRequest>) -> Response {
    let pool = match crate::ai::chat_db::get_chat_pool() {
        Some(p) => p,
        None => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "DB_ERROR",
                "Database not initialized",
                None,
            )
        }
    };

    // Validate name is not empty
    if req.name.trim().is_empty() {
        return error_response(
            StatusCode::BAD_REQUEST,
            "INVALID_NAME",
            "Task name cannot be empty",
            None,
        );
    }

    // Validate instruction is not empty
    if req.instruction.trim().is_empty() {
        return error_response(
            StatusCode::BAD_REQUEST,
            "INVALID_INSTRUCTION",
            "Task instruction cannot be empty",
            None,
        );
    }

    // Validate schedule type
    if !["interval", "daily", "weekly"].contains(&req.schedule_type.as_str()) {
        return error_response(
            StatusCode::BAD_REQUEST,
            "INVALID_SCHEDULE_TYPE",
            "Invalid schedule_type. Must be: interval, daily, or weekly",
            None,
        );
    }

    // Validate schedule value
    if let Err(e) =
        crate::ai::scheduled_db::calculate_next_run(&req.schedule_type, &req.schedule_value, None)
    {
        return error_response(
            StatusCode::BAD_REQUEST,
            "INVALID_SCHEDULE",
            &format!("Invalid schedule_value: {e}"),
            None,
        );
    }

    match crate::ai::scheduled_db::create_task(
        &pool,
        &req.name,
        &req.instruction,
        &req.schedule_type,
        &req.schedule_value,
        Some(&req.tool_permissions),
        Some(&req.priority),
    ) {
        Ok(id) => {
            // Update optional fields that aren't part of create_task
            if let Err(e) = crate::ai::scheduled_db::update_task(
                &pool,
                id,
                None,
                None,
                req.instruction_ids.as_ref().map(|s| Some(s.as_str())),
                None,
                None,
                None,
                None,
                Some(req.notify_telegram),
                Some(req.notify_on_success),
                Some(req.notify_on_failure),
                req.max_retries,
                req.timeout_seconds,
            ) {
                logger::warning(
                    LogTag::System,
                    &format!("Failed to update optional fields for task {}: {}", id, e),
                );
            }

            match crate::ai::scheduled_db::get_task(&pool, id) {
                Ok(Some(task)) => success_response(serde_json::json!({ "task": task })),
                _ => success_response(serde_json::json!({ "id": id })),
            }
        }
        Err(e) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DB_ERROR",
            &format!("Failed to create task: {e}"),
            None,
        ),
    }
}

/// GET /api/ai/automation/:id — Get a specific task
pub async fn get_automation_task(Path(id): Path<i64>) -> Response {
    let pool = match crate::ai::chat_db::get_chat_pool() {
        Some(p) => p,
        None => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "DB_ERROR",
                "Database not initialized",
                None,
            )
        }
    };

    match crate::ai::scheduled_db::get_task(&pool, id) {
        Ok(Some(task)) => success_response(serde_json::json!({ "task": task })),
        Ok(None) => error_response(StatusCode::NOT_FOUND, "NOT_FOUND", "Task not found", None),
        Err(e) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DB_ERROR",
            &format!("Failed to get task: {e}"),
            None,
        ),
    }
}

/// PATCH /api/ai/automation/:id — Update a task
pub async fn update_automation_task(
    Path(id): Path<i64>,
    Json(req): Json<UpdateAutomationTaskRequest>,
) -> Response {
    let pool = match crate::ai::chat_db::get_chat_pool() {
        Some(p) => p,
        None => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "DB_ERROR",
                "Database not initialized",
                None,
            )
        }
    };

    // Validate schedule if provided
    if let (Some(st), Some(sv)) = (&req.schedule_type, &req.schedule_value) {
        if let Err(e) = crate::ai::scheduled_db::calculate_next_run(st, sv, None) {
            return error_response(
                StatusCode::BAD_REQUEST,
                "INVALID_SCHEDULE",
                &format!("Invalid schedule: {e}"),
                None,
            );
        }
    }

    // Validate tool_permissions if provided
    if let Some(tp) = &req.tool_permissions {
        if !["full", "readonly"].contains(&tp.as_str()) {
            return error_response(
                StatusCode::BAD_REQUEST,
                "INVALID_TOOL_PERMISSIONS",
                "tool_permissions must be 'full' or 'readonly'",
                None,
            );
        }
    }

    // Validate priority if provided
    if let Some(p) = &req.priority {
        if !["low", "medium", "high"].contains(&p.as_str()) {
            return error_response(
                StatusCode::BAD_REQUEST,
                "INVALID_PRIORITY",
                "priority must be 'low', 'medium', or 'high'",
                None,
            );
        }
    }

    match crate::ai::scheduled_db::update_task(
        &pool,
        id,
        req.name.as_deref(),
        req.instruction.as_deref(),
        req.instruction_ids.as_ref().map(|s| Some(s.as_str())),
        req.schedule_type.as_deref(),
        req.schedule_value.as_deref(),
        req.tool_permissions.as_deref(),
        req.priority.as_deref(),
        req.notify_telegram,
        req.notify_on_success,
        req.notify_on_failure,
        req.max_retries,
        req.timeout_seconds,
    ) {
        Ok(_) => match crate::ai::scheduled_db::get_task(&pool, id) {
            Ok(Some(task)) => success_response(serde_json::json!({ "task": task })),
            _ => success_response(serde_json::json!({ "updated": true })),
        },
        Err(e) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DB_ERROR",
            &format!("Failed to update task: {e}"),
            None,
        ),
    }
}

/// DELETE /api/ai/automation/:id — Delete a task
pub async fn delete_automation_task(Path(id): Path<i64>) -> Response {
    let pool = match crate::ai::chat_db::get_chat_pool() {
        Some(p) => p,
        None => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "DB_ERROR",
                "Database not initialized",
                None,
            )
        }
    };

    // Check if task has a running execution
    match crate::ai::scheduled_db::list_runs_for_task(&pool, id, 1) {
        Ok(runs) if !runs.is_empty() && runs[0].status == "running" => {
            return error_response(
                StatusCode::CONFLICT,
                "TASK_RUNNING",
                "Cannot delete task while it is running",
                None,
            );
        }
        _ => {}
    }

    match crate::ai::scheduled_db::delete_task(&pool, id) {
        Ok(_) => success_response(serde_json::json!({ "deleted": true })),
        Err(e) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DB_ERROR",
            &format!("Failed to delete task: {e}"),
            None,
        ),
    }
}

/// POST /api/ai/automation/:id/toggle — Enable/disable a task
pub async fn toggle_automation_task(
    Path(id): Path<i64>,
    Json(req): Json<ToggleTaskRequest>,
) -> Response {
    let pool = match crate::ai::chat_db::get_chat_pool() {
        Some(p) => p,
        None => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "DB_ERROR",
                "Database not initialized",
                None,
            )
        }
    };

    match crate::ai::scheduled_db::toggle_task(&pool, id, req.enabled) {
        Ok(_) => match crate::ai::scheduled_db::get_task(&pool, id) {
            Ok(Some(task)) => success_response(serde_json::json!({ "task": task })),
            _ => success_response(serde_json::json!({ "toggled": true })),
        },
        Err(e) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DB_ERROR",
            &format!("Failed to toggle task: {e}"),
            None,
        ),
    }
}

/// POST /api/ai/automation/:id/run — Trigger immediate execution
pub async fn run_automation_task(Path(id): Path<i64>) -> Response {
    let pool = match crate::ai::chat_db::get_chat_pool() {
        Some(p) => p,
        None => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "DB_ERROR",
                "Database not initialized",
                None,
            )
        }
    };

    let task = match crate::ai::scheduled_db::get_task(&pool, id) {
        Ok(Some(t)) => t,
        Ok(None) => {
            return error_response(StatusCode::NOT_FOUND, "NOT_FOUND", "Task not found", None)
        }
        Err(e) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "DB_ERROR",
                &format!("Failed to get task: {e}"),
                None,
            )
        }
    };

    // Don't allow running disabled tasks
    if !task.enabled {
        return error_response(
            StatusCode::BAD_REQUEST,
            "TASK_DISABLED",
            "Cannot run a disabled task",
            None,
        );
    }

    // Check if task is already running
    match crate::ai::scheduled_db::list_runs_for_task(&pool, id, 1) {
        Ok(runs) if !runs.is_empty() && runs[0].status == "running" => {
            return error_response(
                StatusCode::CONFLICT,
                "TASK_RUNNING",
                "Task is already running",
                None,
            );
        }
        _ => {}
    }

    // Execute in background
    tokio::spawn(async move {
        let pool = match crate::ai::chat_db::get_chat_pool() {
            Some(p) => p,
            None => {
                logger::warning(
                    LogTag::System,
                    "Failed to get DB pool for manual task execution",
                );
                return;
            }
        };
        let timeout = if task.timeout_seconds > 0 {
            task.timeout_seconds as u64
        } else {
            120
        };
        if let Err(e) =
            crate::ai::scheduled_worker::execute_task_public(&pool, &task, timeout).await
        {
            logger::warning(
                LogTag::System,
                &format!("Manual task execution failed for '{}': {}", task.name, e),
            );
        }
    });

    success_response(serde_json::json!({ "triggered": true, "task_id": id }))
}

/// GET /api/ai/automation/:id/runs — Get run history for a task
pub async fn get_automation_task_runs(Path(id): Path<i64>) -> Response {
    let pool = match crate::ai::chat_db::get_chat_pool() {
        Some(p) => p,
        None => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "DB_ERROR",
                "Database not initialized",
                None,
            )
        }
    };

    match crate::ai::scheduled_db::list_runs_for_task(&pool, id, 50) {
        Ok(runs) => success_response(serde_json::json!({ "runs": runs })),
        Err(e) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DB_ERROR",
            &format!("Failed to list runs: {e}"),
            None,
        ),
    }
}

/// GET /api/ai/automation/runs — Get all recent runs
pub async fn get_automation_recent_runs() -> Response {
    let pool = match crate::ai::chat_db::get_chat_pool() {
        Some(p) => p,
        None => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "DB_ERROR",
                "Database not initialized",
                None,
            )
        }
    };

    match crate::ai::scheduled_db::list_recent_runs(&pool, 100) {
        Ok(runs) => success_response(serde_json::json!({ "runs": runs })),
        Err(e) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DB_ERROR",
            &format!("Failed to list recent runs: {e}"),
            None,
        ),
    }
}

/// GET /api/ai/automation/runs/:id — Get a specific run
pub async fn get_automation_run_detail(Path(run_id): Path<i64>) -> Response {
    let pool = match crate::ai::chat_db::get_chat_pool() {
        Some(p) => p,
        None => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "DB_ERROR",
                "Database not initialized",
                None,
            )
        }
    };

    match crate::ai::scheduled_db::get_run(&pool, run_id) {
        Ok(Some(run)) => success_response(serde_json::json!({ "run": run })),
        Ok(None) => error_response(StatusCode::NOT_FOUND, "NOT_FOUND", "Run not found", None),
        Err(e) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DB_ERROR",
            &format!("Failed to get run: {e}"),
            None,
        ),
    }
}

/// GET /api/ai/automation/stats — Aggregated automation statistics
pub async fn get_automation_stats_handler() -> Response {
    let pool = match crate::ai::chat_db::get_chat_pool() {
        Some(p) => p,
        None => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "DB_ERROR",
                "Database not initialized",
                None,
            )
        }
    };

    match crate::ai::scheduled_db::get_automation_stats(&pool) {
        Ok(stats) => success_response(serde_json::json!({ "stats": stats })),
        Err(e) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DB_ERROR",
            &format!("Failed to get stats: {e}"),
            None,
        ),
    }
}
