//! Paper copy-trading task and activity API. Live mode is rejected by the
//! domain input validator and has no route to the trader submission path.

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

use crate::trader::copy::{CopyDatabase, CopyTask, CopyTaskInput};
use crate::wallets::watch;
use crate::webserver::state::AppState;
use crate::webserver::utils::{error_response, success_response};

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/status", get(status))
        .route("/tasks", get(list_tasks).post(create_task))
        .route(
            "/tasks/:id",
            get(get_task).patch(update_task).delete(delete_task),
        )
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

#[derive(Serialize)]
struct StatusResponse {
    enabled: bool,
    paper_only: bool,
    default_mode: String,
    default_slippage_pct: f64,
    force_stop_blocks: bool,
    total_tasks: usize,
    active_tasks: usize,
}

#[derive(Deserialize)]
struct ActivityQuery {
    #[serde(default = "default_limit")]
    limit: usize,
}

fn default_limit() -> usize {
    50
}

async fn open_database() -> Result<CopyDatabase, String> {
    tokio::task::spawn_blocking(CopyDatabase::new)
        .await
        .map_err(|e| format!("Copy database task failed: {e}"))?
}

async fn status() -> Response {
    match open_database().await {
        Ok(db) => match db.list_tasks().await {
            Ok(tasks) => {
                let (enabled, default_mode, default_slippage_pct, force_stop_blocks) =
                    crate::config::with_config(|config| {
                        (
                            config.copy_trading.enabled,
                            config.copy_trading.default_mode.clone(),
                            config.copy_trading.default_slippage_pct,
                            config.copy_trading.block_on_force_stop,
                        )
                    });
                success_response(StatusResponse {
                    enabled,
                    paper_only: true,
                    default_mode,
                    default_slippage_pct,
                    force_stop_blocks,
                    total_tasks: tasks.len(),
                    active_tasks: tasks.iter().filter(|task| task.enabled).count(),
                })
            }
            Err(error) => internal_error(error),
        },
        Err(error) => internal_error(error),
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
    let task = match input.into_task(Utc::now()) {
        Ok(task) => task,
        Err(reason) => return invalid_task(reason),
    };
    if let Err(error) = crate::wallets::validate_address(&task.target_address) {
        return error_response(
            StatusCode::BAD_REQUEST,
            "INVALID_ADDRESS",
            "Invalid target wallet",
            Some(&error),
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
    let mut task = match input.into_task(Utc::now()) {
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

async fn delete_task(Path(id): Path<i64>) -> Response {
    let db = match open_database().await {
        Ok(db) => db,
        Err(error) => return internal_error(error),
    };
    let task = match db.get_task(id).await {
        Ok(Some(task)) => task,
        Ok(None) => return not_found(id),
        Err(error) => return internal_error(error),
    };
    if let Err(error) = watch::remove_copy_source(id, &task.target_address).await {
        return internal_error(format!(
            "Copy task was retained because its watch source could not be removed: {error}"
        ));
    }
    match db.delete_task(id).await {
        Ok(true) => success_response(serde_json::json!({ "message": "Copy task removed" })),
        Ok(false) => not_found(id),
        Err(error) => {
            let restored = if task.enabled {
                watch::add_copy_source(id, &task.target_address, task.label.as_deref())
                    .await
                    .map(|_| ())
            } else {
                Ok(())
            };
            internal_error(match restored {
                Ok(()) => format!("Copy task deletion failed; watch source was restored: {error}"),
                Err(restore) => format!(
                    "Copy task deletion failed ({error}) and its watch source could not be restored ({restore})"
                ),
            })
        }
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

fn invalid_task(reason: impl std::fmt::Debug) -> Response {
    error_response(
        StatusCode::BAD_REQUEST,
        "INVALID_TASK",
        "Invalid paper copy task",
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
