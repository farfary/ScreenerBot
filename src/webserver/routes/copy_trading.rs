//! Copy-trading task, guarded mode-transition, and activity API.

use axum::{
    extract::{Path, Query},
    http::StatusCode,
    response::Response,
    routing::get,
    Json, Router,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::trader::copy::{
    build_task_stats, confirm_mode_transition, CopyDatabase, CopyMode, CopyTask, CopyTaskInput,
};
use crate::wallets::watch;
use crate::webserver::state::AppState;
use crate::webserver::utils::{error_response, success_response};

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/overview", get(overview))
        .route("/status", get(status))
        .route("/tasks", get(list_tasks).post(create_task))
        .route("/tasks/:id", get(get_task).patch(update_task))
        .route("/tasks/:id/mode", axum::routing::post(set_task_mode))
        .route("/tasks/:id/stats", get(task_stats))
        .route("/activity", get(list_activity))
}

#[derive(Serialize)]
struct TaskList {
    tasks: Vec<CopyTask>,
}

#[derive(Serialize)]
struct TaskResponse {
    task: CopyTask,
}

// The overview shapes are `pub(crate)` so the promo fixtures can build the exact
// payload this route serves instead of a parallel copy that could drift from it.
#[derive(Clone, Serialize)]
pub(crate) struct StatusResponse {
    pub(crate) enabled: bool,
    pub(crate) live_available: bool,
    pub(crate) blocked_reason: Option<&'static str>,
    pub(crate) default_mode: String,
    pub(crate) default_slippage_pct: f64,
    pub(crate) force_stop_blocks: bool,
    pub(crate) total_tasks: usize,
    pub(crate) active_tasks: usize,
    pub(crate) paper_tasks: usize,
    pub(crate) live_tasks: usize,
}

#[derive(Serialize)]
pub(crate) struct TaskSummary {
    #[serde(flatten)]
    pub(crate) task: CopyTask,
    pub(crate) stats: crate::trader::copy::CopyTaskStats,
    pub(crate) spent_sol: f64,
    pub(crate) remaining_budget_sol: f64,
    pub(crate) effective_state: &'static str,
}

#[derive(Serialize)]
pub(crate) struct OverviewResponse {
    pub(crate) status: StatusResponse,
    pub(crate) tasks: Vec<TaskSummary>,
    pub(crate) activity: Vec<crate::trader::copy::CopyActivityRow>,
}

#[derive(Deserialize)]
struct ActivityQuery {
    #[serde(default = "default_limit")]
    limit: usize,
}

#[derive(Deserialize)]
struct ModeRequest {
    mode: CopyMode,
    confirmation: Option<String>,
}

fn default_limit() -> usize {
    50
}

async fn open_database() -> Result<CopyDatabase, String> {
    tokio::task::spawn_blocking(|| CopyDatabase::shared(crate::chains::active_chain()))
        .await
        .map_err(|e| format!("Copy database task failed: {e}"))?
}

async fn status() -> Response {
    match open_database().await {
        Ok(db) => match db.list_tasks().await {
            Ok(tasks) => success_response(build_status(&tasks)),
            Err(error) => internal_error(error),
        },
        Err(error) => internal_error(error),
    }
}

async fn overview() -> Response {
    // Return promotional fixtures only for owner-initiated media capture.
    if crate::webserver::promo::are_promo_fixtures_enabled() {
        return success_response(crate::webserver::promo::get_promo_copy_trading_overview());
    }

    let db = match open_database().await {
        Ok(db) => db,
        Err(error) => return internal_error(error),
    };
    let tasks = match db.list_tasks().await {
        Ok(tasks) => tasks,
        Err(error) => return internal_error(error),
    };
    let activity = match db.list_activity(50).await {
        Ok(activity) => activity,
        Err(error) => return internal_error(error),
    };
    let mut positions = crate::positions::get_open_positions().await;
    positions.extend(crate::positions::get_closed_positions().await);
    positions.extend(crate::positions::get_archived_positions().await);
    let status = build_status(&tasks);
    let mut summaries = Vec::with_capacity(tasks.len());
    for task in tasks {
        let task_activity = match db.list_task_activity(task.id, 10_000).await {
            Ok(activity) => activity,
            Err(error) => return internal_error(error),
        };
        let spent_sol = match db.task_total_spent(task.id).await {
            Ok(spent) => spent,
            Err(error) => return internal_error(error),
        };
        let effective_state = if !status.enabled {
            "system_paused"
        } else if status.blocked_reason == Some("force_stop") {
            "force_stopped"
        } else if !task.enabled {
            "paused"
        } else if status.blocked_reason.is_some() {
            "entries_blocked"
        } else if task.mode == CopyMode::Live {
            "live"
        } else {
            "paper"
        };
        summaries.push(TaskSummary {
            stats: build_task_stats(task.id, &task_activity, &positions),
            remaining_budget_sol: (task.total_budget_sol - spent_sol).max(0.0),
            spent_sol,
            effective_state,
            task,
        });
    }
    success_response(OverviewResponse {
        status,
        tasks: summaries,
        activity,
    })
}

fn build_status(tasks: &[CopyTask]) -> StatusResponse {
    let (enabled, default_mode, default_slippage_pct, force_stop_blocks) =
        crate::config::with_config(|config| {
            (
                config.copy_trading.enabled,
                config.copy_trading.default_mode.clone(),
                config.copy_trading.default_slippage_pct,
                config.copy_trading.block_on_force_stop,
            )
        });
    StatusResponse {
        enabled,
        live_available: crate::global::is_initialization_complete()
            && !crate::global::is_force_stopped(),
        blocked_reason: if crate::global::is_force_stopped() {
            Some("force_stop")
        } else if crate::trader::safety::loss_limit::is_entry_blocked_by_loss_limit() {
            Some("loss_limit")
        } else {
            None
        },
        default_mode,
        default_slippage_pct,
        force_stop_blocks,
        total_tasks: tasks.len(),
        active_tasks: tasks.iter().filter(|task| task.enabled).count(),
        paper_tasks: tasks
            .iter()
            .filter(|task| task.enabled && task.mode == CopyMode::Paper)
            .count(),
        live_tasks: tasks
            .iter()
            .filter(|task| task.enabled && task.mode == CopyMode::Live)
            .count(),
    }
}

async fn list_tasks() -> Response {
    match open_database().await {
        Ok(db) => match db.list_tasks().await {
            Ok(tasks) => success_response(TaskList { tasks }),
            Err(error) => internal_error(error),
        },
        Err(error) => internal_error(error),
    }
}

async fn get_task(Path(id): Path<i64>) -> Response {
    match open_database().await {
        Ok(db) => match db.get_task(id).await {
            Ok(Some(task)) => success_response(TaskResponse { task }),
            Ok(None) => not_found(id),
            Err(error) => internal_error(error),
        },
        Err(error) => internal_error(error),
    }
}

async fn create_task(Json(input): Json<CopyTaskInput>) -> Response {
    let task = match input.into_task(crate::chains::active_chain(), Utc::now()) {
        Ok(task) => task,
        Err(reason) => return invalid_task(reason),
    };
    if let Err(error) = crate::chains::adapter().validate_address(&task.target_address) {
        return error_response(
            StatusCode::BAD_REQUEST,
            "INVALID_ADDRESS",
            "Invalid target wallet",
            Some(&error.to_string()),
        );
    }
    let db = match open_database().await {
        Ok(db) => db,
        Err(error) => return internal_error(error),
    };
    if task.enabled {
        match active_task_limit_reached(&db).await {
            Ok(true) => {
                return error_response(
                    StatusCode::BAD_REQUEST,
                    "TASK_LIMIT",
                    "Maximum active copy tasks reached",
                    None,
                );
            }
            Ok(false) => {}
            Err(error) => return internal_error(error),
        }
    }
    let inserted = match db.insert_task(task).await {
        Ok(task) => task,
        Err(error) => return internal_error(error),
    };
    if inserted.enabled {
        if let Err(error) = watch::add_copy_source(
            inserted.id,
            &inserted.target_address,
            inserted.label.as_deref(),
        )
        .await
        {
            let _ = db.delete_task(inserted.id).await;
            return error_response(
                StatusCode::BAD_REQUEST,
                "WATCH_REJECTED",
                "Copy target could not be watched",
                Some(&error),
            );
        }
    }
    success_response(TaskResponse { task: inserted })
}

async fn update_task(Path(id): Path<i64>, Json(input): Json<CopyTaskInput>) -> Response {
    let db = match open_database().await {
        Ok(db) => db,
        Err(error) => return internal_error(error),
    };
    let original = match db.get_task(id).await {
        Ok(Some(task)) => task,
        Ok(None) => return not_found(id),
        Err(error) => return internal_error(error),
    };
    let mut task = match input.into_task_for_update(
        crate::chains::active_chain(),
        Utc::now(),
        original.mode,
    ) {
        Ok(task) => task,
        Err(reason) => return invalid_task(reason),
    };
    task.id = id;
    task.created_at = original.created_at;
    if task.enabled && !original.enabled {
        match active_task_limit_reached(&db).await {
            Ok(true) => {
                return error_response(
                    StatusCode::BAD_REQUEST,
                    "TASK_LIMIT",
                    "Maximum active copy tasks reached",
                    None,
                );
            }
            Ok(false) => {}
            Err(error) => return internal_error(error),
        }
    }
    if task.enabled {
        if let Err(error) =
            watch::add_copy_source(id, &task.target_address, task.label.as_deref()).await
        {
            return error_response(
                StatusCode::BAD_REQUEST,
                "WATCH_REJECTED",
                "Copy target could not be watched",
                Some(&error),
            );
        }
    }
    let new_address = task.target_address.clone();
    let new_enabled = task.enabled;
    let source_was_added =
        new_enabled && (!original.enabled || original.target_address != new_address);
    match db.update_task(task).await {
        Ok(updated) => {
            if original.enabled
                && (original.target_address != updated.target_address || !updated.enabled)
            {
                if let Err(error) = watch::remove_copy_source(id, &original.target_address).await {
                    let database_rollback = db.update_task(original.clone()).await;
                    if source_was_added {
                        let _ = watch::remove_copy_source(id, &new_address).await;
                    }
                    return internal_error(match database_rollback {
                        Ok(_) => format!(
                            "Failed to detach the previous copy target; task update was rolled back: {error}"
                        ),
                        Err(rollback) => format!(
                            "Failed to detach the previous copy target ({error}) and roll back the task ({rollback})"
                        ),
                    });
                }
            }
            if let Err(error) =
                crate::trader::copy::sync_open_position_management(id, updated.exit_mode).await
            {
                let rollback = db.update_task(original.clone()).await;
                let position_rollback =
                    crate::trader::copy::sync_open_position_management(id, original.exit_mode)
                        .await;
                return internal_error(match (rollback, position_rollback) {
                    (Ok(_), Ok(())) => format!(
                        "Failed to update copy position ownership; task update was rolled back: {error}"
                    ),
                    (task_result, position_result) => format!(
                        "Failed to update copy position ownership ({error}); rollback results: task={task_result:?}, positions={position_result:?}"
                    ),
                });
            }
            success_response(TaskResponse { task: updated })
        }
        Err(error) => {
            if source_was_added {
                let _ = watch::remove_copy_source(id, &new_address).await;
            }
            internal_error(error)
        }
    }
}

async fn set_task_mode(Path(id): Path<i64>, Json(request): Json<ModeRequest>) -> Response {
    let db = match open_database().await {
        Ok(db) => db,
        Err(error) => return internal_error(error),
    };
    let current = match db.get_task(id).await {
        Ok(Some(task)) => task,
        Ok(None) => return not_found(id),
        Err(error) => return internal_error(error),
    };
    let mode = match confirm_mode_transition(
        current.mode,
        request.mode,
        request.confirmation.as_deref(),
    ) {
        Ok(mode) => mode,
        Err(reason) => {
            return error_response(
                StatusCode::BAD_REQUEST,
                "LIVE_CONFIRMATION_REQUIRED",
                "Arming live copy trading requires explicit confirmation",
                Some(&format!("{reason:?}")),
            )
        }
    };
    match db.set_task_mode(id, mode, request.confirmation).await {
        Ok(task) => success_response(TaskResponse { task }),
        Err(error) => internal_error(error),
    }
}

async fn list_activity(Query(query): Query<ActivityQuery>) -> Response {
    match open_database().await {
        Ok(db) => match db.list_activity(query.limit).await {
            Ok(activity) => success_response(serde_json::json!({ "activity": activity })),
            Err(error) => internal_error(error),
        },
        Err(error) => internal_error(error),
    }
}

async fn task_stats(Path(id): Path<i64>) -> Response {
    let db = match open_database().await {
        Ok(db) => db,
        Err(error) => return internal_error(error),
    };
    match db.get_task(id).await {
        Ok(Some(_)) => {}
        Ok(None) => return not_found(id),
        Err(error) => return internal_error(error),
    }
    let activity = match db.list_task_activity(id, 10_000).await {
        Ok(activity) => activity,
        Err(error) => return internal_error(error),
    };
    let mut positions = crate::positions::get_open_positions().await;
    positions.extend(crate::positions::get_closed_positions().await);
    positions.extend(crate::positions::get_archived_positions().await);
    success_response(build_task_stats(id, &activity, &positions))
}

fn invalid_task(reason: impl std::fmt::Debug) -> Response {
    error_response(
        StatusCode::BAD_REQUEST,
        "INVALID_TASK",
        "Invalid copy task",
        Some(&format!("{reason:?}")),
    )
}

fn not_found(id: i64) -> Response {
    error_response(
        StatusCode::NOT_FOUND,
        "NOT_FOUND",
        "Copy task not found",
        Some(&format!("Task {id}")),
    )
}

fn internal_error(error: String) -> Response {
    error_response(
        StatusCode::INTERNAL_SERVER_ERROR,
        "COPY_ERROR",
        "Copy trading request failed",
        Some(&error),
    )
}

async fn active_task_limit_reached(database: &CopyDatabase) -> Result<bool, String> {
    let active = database
        .list_tasks()
        .await?
        .into_iter()
        .filter(|task| task.enabled)
        .count();
    let maximum = crate::config::with_config(|config| config.copy_trading.max_active_tasks);
    Ok(active >= maximum)
}
