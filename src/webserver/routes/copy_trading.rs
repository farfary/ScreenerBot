// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Copy-trading task, guarded mode-transition, and activity API. A transport over
//! `trader::copy::control`, which owns every rule these endpoints enforce.

use axum::{
    extract::{Path, Query},
    middleware::from_fn,
    response::Response,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::errors::ErrorClass;
use crate::i18n::ids;
use crate::trader::copy::workspace::{self, ActivityFilter, CloneRequest, RangeQuery};
use crate::trader::copy::{control, CopyMode, CopySkip, CopyTask, CopyTaskInput};
use crate::webserver::api_error::{ApiError, ApiErrorCode};
use crate::webserver::middleware::full_setup_gate;
use crate::webserver::promo;
use crate::webserver::state::AppState;
use crate::webserver::utils::success_response;
use axum::response::IntoResponse as _;

/// Decisions the overview carries.
const OVERVIEW_ACTIVITY: usize = 50;

/// Every read answers from the promotional fixtures during owner-initiated media
/// capture, so no page of a capture shows the operator's own copy tasks.
fn fixtures() -> bool {
    promo::are_promo_fixtures_enabled()
}

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/overview", get(overview))
        .route("/status", get(status))
        .route(
            "/tasks",
            get(list_tasks).merge(post(create_task).route_layer(from_fn(full_setup_gate))),
        )
        .route(
            "/tasks/{id}",
            get(get_task).patch(update_task).delete(delete_task),
        )
        .route("/tasks/{id}/mode", post(set_task_mode))
        .route("/tasks/{id}/stats", get(task_stats))
        .route("/tasks/{id}/workspace", get(task_workspace))
        .route("/tasks/{id}/activity", get(task_activity))
        .route("/tasks/{id}/insights", get(task_insights))
        .route(
            "/tasks/{id}/clone",
            post(clone_task).route_layer(from_fn(full_setup_gate)),
        )
        .route("/tasks/{id}/reset", post(reset_paper_book))
        .route("/tasks/{id}/holdings/{mint}/close", post(close_holding))
        .route("/insights", get(compare_tasks))
        .route("/defaults", get(defaults))
        .route("/wallets/{address}", get(wallet_profile))
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
    if fixtures() {
        return success_response(promo::get_promo_copy_status());
    }
    respond(control::status().await)
}

async fn overview() -> Response {
    if fixtures() {
        return success_response(promo::get_promo_copy_trading_overview(OVERVIEW_ACTIVITY));
    }
    respond(control::overview(OVERVIEW_ACTIVITY).await)
}

async fn list_tasks() -> Response {
    if fixtures() {
        return success_response(TaskList {
            tasks: promo::get_promo_copy_tasks(),
        });
    }
    respond(control::list_tasks().await.map(|tasks| TaskList { tasks }))
}

async fn get_task(Path(id): Path<i64>) -> Response {
    let task = if fixtures() {
        promo::get_promo_copy_task(id)
    } else {
        control::get_task(id).await
    };
    respond(task.map(|task| TaskResponse { task }))
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
    let activity = if fixtures() {
        Ok(promo::get_promo_copy_recent_activity(query.limit))
    } else {
        control::list_activity(None, query.limit).await
    };
    respond(activity.map(|activity| serde_json::json!({ "activity": activity })))
}

async fn task_stats(Path(id): Path<i64>) -> Response {
    if fixtures() {
        return respond(promo::get_promo_copy_task_stats(id));
    }
    respond(control::task_stats(id).await)
}

async fn task_workspace(Path(id): Path<i64>) -> Response {
    if fixtures() {
        return respond(promo::get_promo_copy_workspace(id));
    }
    respond(workspace::task_workspace(id).await)
}

async fn task_activity(Path(id): Path<i64>, Query(filter): Query<ActivityFilter>) -> Response {
    if fixtures() {
        return respond(promo::get_promo_copy_activity(Some(id), filter));
    }
    respond(workspace::activity_page(Some(id), filter).await)
}

async fn task_insights(Path(id): Path<i64>, Query(range): Query<RangeQuery>) -> Response {
    if fixtures() {
        return respond(promo::get_promo_copy_insights(id, range.into()));
    }
    respond(workspace::task_insights(id, range.into()).await)
}

async fn compare_tasks(Query(range): Query<RangeQuery>) -> Response {
    let tasks = if fixtures() {
        Ok(promo::get_promo_copy_comparison(range.into()))
    } else {
        workspace::compare_tasks(range.into()).await
    };
    respond(tasks.map(|tasks| serde_json::json!({ "tasks": tasks })))
}

async fn clone_task(Path(id): Path<i64>, body: Option<Json<CloneRequest>>) -> Response {
    let request = body.map(|Json(request)| request).unwrap_or_default();
    respond(
        workspace::clone_task(id, request)
            .await
            .map(|task| TaskResponse { task }),
    )
}

async fn reset_paper_book(Path(id): Path<i64>) -> Response {
    respond(workspace::reset_paper_book(id).await)
}

async fn close_holding(Path((id, mint)): Path<(i64, String)>) -> Response {
    respond(workspace::close_paper_holding(id, &mint).await)
}

async fn defaults() -> Response {
    success_response(workspace::defaults())
}

async fn wallet_profile(Path(address): Path<String>) -> Response {
    if fixtures() {
        return respond(promo::get_promo_copy_wallet_profile(&address));
    }
    respond(workspace::wallet_profile(&address).await)
}

fn respond<T: Serialize>(result: crate::trader::Result<T>) -> Response {
    match result {
        Ok(value) => success_response(value),
        Err(error) => copy_error(&error),
    }
}

fn copy_error(error: &crate::trader::Error) -> Response {
    use crate::trader::Error;
    let (code, id) = match error {
        Error::CopyTaskNotFound { .. } => (ApiErrorCode::NotFound, ids::ERRORS_COPY_TASK_NOT_FOUND),
        Error::CopyHoldingNotFound { .. } => {
            (ApiErrorCode::NotFound, ids::ERRORS_COPY_HOLDING_NOT_FOUND)
        }
        Error::CopyTaskRejected {
            reason: CopySkip::LiveConfirmationRequired,
        } => (
            ApiErrorCode::LiveConfirmationRequired,
            ids::ERRORS_COPY_LIVE_CONFIRMATION_REQUIRED,
        ),
        Error::CopyTaskRejected { .. } => {
            (ApiErrorCode::InvalidTask, ids::ERRORS_COPY_TASK_INVALID)
        }
        Error::CopyValidation { .. } => (
            ApiErrorCode::InvalidInput,
            ids::ERRORS_COPY_REQUEST_REJECTED,
        ),
        Error::CopyTaskLimit { .. } => (ApiErrorCode::TaskLimit, ids::ERRORS_COPY_TASK_LIMIT),
        Error::CopyWatchRejected { .. } => {
            (ApiErrorCode::WatchRejected, ids::ERRORS_COPY_WATCH_REJECTED)
        }
        Error::CopyLiveUnavailable { .. } => (
            ApiErrorCode::LiveUnavailable,
            ids::ERRORS_COPY_LIVE_UNAVAILABLE,
        ),
        Error::CopyTaskLive { .. } => (ApiErrorCode::TaskLive, ids::ERRORS_COPY_TASK_LIVE),
        Error::CopyTaskOwnsPositions { .. } => (
            ApiErrorCode::OpenPositions,
            ids::ERRORS_COPY_TASK_OWNS_POSITIONS,
        ),
        _ => (
            match error.http_status() {
                500 => ApiErrorCode::CopyError,
                status => ApiErrorCode::for_status(status),
            },
            ids::ERRORS_COPY_REQUEST_FAILED,
        ),
    };
    ApiError::new(code, id)
        .details(error.to_string())
        .into_response()
}
