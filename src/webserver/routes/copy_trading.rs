//! Copy-trading task, guarded mode-transition, and activity API. A transport over
//! `trader::copy::control`, which owns every rule these endpoints enforce.

use axum::{
    extract::{Path, Query},
    http::StatusCode,
    response::Response,
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::errors::ErrorClass;
use crate::trader::copy::{control, CopyMode, CopySkip, CopyTask, CopyTaskInput};
use crate::webserver::state::AppState;
use crate::webserver::utils::{error_response, success_response};

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/overview", get(overview))
        .route("/status", get(status))
        .route("/tasks", get(list_tasks).post(create_task))
        .route(
            "/tasks/:id",
            get(get_task).patch(update_task).delete(delete_task),
        )
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

async fn status() -> Response {
    respond(control::status().await)
}

async fn overview() -> Response {
    // Return promotional fixtures only for owner-initiated media capture.
    if crate::webserver::promo::are_promo_fixtures_enabled() {
        return success_response(crate::webserver::promo::get_promo_copy_trading_overview());
    }
    respond(control::overview(50).await)
}

async fn list_tasks() -> Response {
    respond(control::list_tasks().await.map(|tasks| TaskList { tasks }))
}

async fn get_task(Path(id): Path<i64>) -> Response {
    respond(
        control::get_task(id)
            .await
            .map(|task| TaskResponse { task }),
    )
}

async fn create_task(Json(input): Json<CopyTaskInput>) -> Response {
    respond(
        control::create_task(input)
            .await
            .map(|task| TaskResponse { task }),
    )
}

async fn update_task(Path(id): Path<i64>, Json(patch): Json<serde_json::Value>) -> Response {
    respond(
        control::update_task(id, patch)
            .await
            .map(|task| TaskResponse { task }),
    )
}

async fn delete_task(Path(id): Path<i64>) -> Response {
    respond(
        control::delete_task(id)
            .await
            .map(|()| serde_json::json!({ "deleted": id })),
    )
}

async fn set_task_mode(Path(id): Path<i64>, Json(request): Json<ModeRequest>) -> Response {
    respond(
        control::set_task_mode(id, request.mode, request.confirmation)
            .await
            .map(|task| TaskResponse { task }),
    )
}

async fn list_activity(Query(query): Query<ActivityQuery>) -> Response {
    respond(
        control::list_activity(None, query.limit)
            .await
            .map(|activity| serde_json::json!({ "activity": activity })),
    )
}

async fn task_stats(Path(id): Path<i64>) -> Response {
    respond(control::task_stats(id).await)
}

fn respond<T: Serialize>(result: crate::trader::Result<T>) -> Response {
    match result {
        Ok(value) => success_response(value),
        Err(error) => copy_error(&error),
    }
}

fn copy_error(error: &crate::trader::Error) -> Response {
    use crate::trader::Error;
    let (code, message) = match error {
        Error::CopyTaskNotFound { .. } => ("NOT_FOUND", "Copy task not found"),
        Error::CopyTaskRejected {
            reason: CopySkip::LiveConfirmationRequired,
        } => (
            "LIVE_CONFIRMATION_REQUIRED",
            "Arming live copy trading requires explicit confirmation",
        ),
        Error::CopyTaskRejected { .. } | Error::CopyValidation { .. } => {
            ("INVALID_TASK", "Invalid copy task")
        }
        Error::CopyTaskLimit { .. } => ("TASK_LIMIT", "Maximum active copy tasks reached"),
        Error::CopyWatchRejected { .. } => ("WATCH_REJECTED", "Copy target could not be watched"),
        Error::CopyLiveUnavailable { .. } => {
            ("LIVE_UNAVAILABLE", "Live copy trading is unavailable")
        }
        Error::CopyTaskLive { .. } => ("TASK_LIVE", "Pause the live task before deleting it"),
        Error::CopyTaskOwnsPositions { .. } => {
            ("OPEN_POSITIONS", "Copy task still owns open positions")
        }
        _ => ("COPY_ERROR", "Copy trading request failed"),
    };
    let status =
        StatusCode::from_u16(error.http_status()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    error_response(status, code, message, Some(&error.to_string()))
}
