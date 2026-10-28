// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Copy-trading task, guarded mode-transition, and activity API. A transport over
//! `trader::copy::control`, which owns every rule these endpoints enforce.

use axum::{
    extract::{rejection::JsonRejection, Path, Query},
    middleware::{from_fn, from_fn_with_state},
    response::Response,
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::database::catalog;
use crate::errors::ErrorClass;
use crate::i18n::ids;
use crate::trader::copy::workspace::{self, ActivityFilter, CloneRequest, RangeQuery};
use crate::trader::copy::{control, CopySkip, CopyTask, CopyTaskInput};
use crate::trader::paper::TradingMode;
use crate::webserver::api_error::{ApiError, ApiErrorCode};
use crate::webserver::middleware::{full_setup_gate, store_gate};
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
        .route("/tasks/resume", post(resume_tasks))
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
        .route_layer(from_fn_with_state(&catalog::COPY_TRADING, store_gate))
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
    mode: TradingMode,
    confirmation: Option<String>,
}

/// The holding round a manual paper close was confirmed for: the exact
/// `opened_at` instant the workspace returned for that holding.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CloseHoldingRequest {
    opened_at: DateTime<Utc>,
}

/// A bulk resume of the paused tasks; live tasks are left paused unless included.
#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct ResumeRequest {
    include_live: bool,
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

/// Resume every paused task a single resume would restart; an absent body resumes no
/// live task.
async fn resume_tasks(body: Result<Option<Json<ResumeRequest>>, JsonRejection>) -> Response {
    let request = match body {
        Ok(body) => body.map(|Json(request)| request).unwrap_or_default(),
        Err(rejection) => return rejected_body(rejection),
    };
    respond(control::resume_paused_tasks(request.include_live).await)
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

async fn close_holding(
    Path((id, mint)): Path<(i64, String)>,
    body: Result<Json<CloseHoldingRequest>, JsonRejection>,
) -> Response {
    let request = match body {
        Ok(Json(request)) => request,
        Err(rejection) => return rejected_body(rejection),
    };
    respond(workspace::close_paper_holding(id, &mint, request.opened_at).await)
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

/// A request body that is not the endpoint's JSON shape, refused before any task is
/// touched.
fn rejected_body(rejection: JsonRejection) -> Response {
    ApiError::new(
        ApiErrorCode::InvalidInput,
        ids::ERRORS_COPY_REQUEST_REJECTED,
    )
    .details(rejection.body_text())
    .into_response()
}

fn respond<T: Serialize>(result: crate::trader::Result<T>) -> Response {
    match result {
        Ok(value) => success_response(value),
        Err(error) => copy_error(&error),
    }
}

fn copy_error(error: &crate::trader::Error) -> Response {
    use crate::trader::Error;
    if let Error::ChainAccess(refusal) = error {
        return ApiError::chain_access(refusal).into_response();
    }
    let code = match error {
        Error::CopyTaskNotFound { .. } | Error::CopyHoldingNotFound { .. } => {
            ApiErrorCode::NotFound
        }
        Error::CopyTaskRejected {
            reason: CopySkip::LiveConfirmationRequired,
        } => ApiErrorCode::LiveConfirmationRequired,
        Error::CopyTaskRejected { .. } => ApiErrorCode::InvalidTask,
        Error::CopyValidation { .. } => ApiErrorCode::InvalidInput,
        Error::CopyTaskLimit { .. } => ApiErrorCode::TaskLimit,
        Error::CopyWatchRejected { .. } => ApiErrorCode::WatchRejected,
        Error::CopyLiveUnavailable { .. } => ApiErrorCode::LiveUnavailable,
        Error::CopyTaskLive { .. } => ApiErrorCode::TaskLive,
        Error::CopyTaskOwnsPositions { .. } => ApiErrorCode::OpenPositions,
        _ => match error.http_status() {
            500 => ApiErrorCode::CopyError,
            status => ApiErrorCode::for_status(status),
        },
    };
    ApiError::with_text(code, error.copy_ui_text())
        .details(error.to_string())
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::{close_holding, copy_error, resume_tasks, CloseHoldingRequest, ResumeRequest};
    use crate::trader::copy::control::CopyLiveBlock;
    use crate::trader::copy::CopySkip;
    use crate::trader::Error;
    use crate::webserver::state::AppState;
    use axum::{
        body::Body,
        http::{header, Request, StatusCode},
        routing::post,
        Router,
    };
    use chrono::{TimeZone, Timelike, Utc};
    use std::sync::Arc;
    use tower::ServiceExt;

    async fn post_close(body: Option<&str>) -> (StatusCode, serde_json::Value) {
        let router = Router::new().route("/tasks/{id}/holdings/{mint}/close", post(close_holding));
        let mut request = Request::post("/tasks/1/holdings/Mint111/close");
        let payload = match body {
            Some(text) => {
                request = request.header(header::CONTENT_TYPE, "application/json");
                Body::from(text.to_owned())
            }
            None => Body::empty(),
        };
        let response = router
            .oneshot(request.body(payload).expect("request"))
            .await
            .expect("response");
        let status = response.status();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body");
        (status, serde_json::from_slice(&bytes).unwrap_or_default())
    }

    #[tokio::test]
    async fn a_close_without_the_confirmed_round_is_a_structured_rejection() {
        for body in [
            None,
            Some("{}"),
            Some(r#"{"opened_at":null}"#),
            Some(r#"{"opened_at":"not-a-date"}"#),
            Some(r#"{"opened_at":"2026-10-07T03:00:00Z","mint":"x"}"#),
        ] {
            let (status, json) = post_close(body).await;
            assert_eq!(status, StatusCode::BAD_REQUEST, "body {body:?}");
            assert_eq!(json["error"]["code"], "INVALID_INPUT", "body {body:?}");
        }
    }

    #[test]
    fn the_confirmed_round_keeps_sub_millisecond_precision() {
        let request: CloseHoldingRequest =
            serde_json::from_str(r#"{"opened_at":"2026-10-07T03:00:00.123456789Z"}"#)
                .expect("request");
        assert_eq!(request.opened_at.nanosecond(), 123_456_789);
        assert_eq!(
            request.opened_at.date_naive(),
            Utc.with_ymd_and_hms(2026, 10, 7, 0, 0, 0)
                .unwrap()
                .date_naive()
        );
    }

    async fn send(
        router: Router,
        method: &str,
        uri: &str,
        body: Option<&str>,
    ) -> (StatusCode, serde_json::Value) {
        let mut request = Request::builder().method(method).uri(uri);
        let payload = match body {
            Some(text) => {
                request = request.header(header::CONTENT_TYPE, "application/json");
                Body::from(text.to_owned())
            }
            None => Body::empty(),
        };
        let response = router
            .oneshot(request.body(payload).expect("request"))
            .await
            .expect("response");
        let status = response.status();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body");
        (status, serde_json::from_slice(&bytes).unwrap_or_default())
    }

    /// The bulk resume reaches its own handler, never the task route: an unknown field is
    /// refused by the resume handler before any store is opened, while a POST to a task id
    /// falls through to the task route's 405.
    #[tokio::test]
    async fn the_bulk_resume_is_not_captured_by_the_task_route() {
        let router = super::routes().with_state(Arc::new(AppState::new()));
        let (status, json) = send(
            router.clone(),
            "POST",
            "/tasks/resume",
            Some(r#"{"includeLive":true}"#),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(json["error"]["code"], "INVALID_INPUT");
        assert_eq!(json["error"]["text"]["id"], "errors-copy-request-rejected");
        let (status, _) = send(router, "POST", "/tasks/7", Some("{}")).await;
        assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED);
    }

    /// An empty object and an absent field leave live tasks out; only an explicit
    /// `true` includes them.
    #[test]
    fn the_resume_body_defaults_to_leaving_live_tasks_out() {
        let parse =
            |text: &str| serde_json::from_str::<ResumeRequest>(text).map(|r| r.include_live);
        assert!(!ResumeRequest::default().include_live);
        assert!(!parse("{}").expect("empty object"));
        assert!(!parse(r#"{"include_live":false}"#).expect("false"));
        assert!(parse(r#"{"include_live":true}"#).expect("true"));
    }

    /// A body that is not the resume request is refused before any task is touched, so
    /// a misspelled or mistyped flag never resumes or skips tasks silently.
    #[tokio::test]
    async fn a_malformed_resume_body_is_a_structured_rejection() {
        for body in [
            r#"{"includeLive":true}"#,
            r#"{"include_live":"yes"}"#,
            r#"{"include_live":null}"#,
            "true",
            "[1]",
            "not json",
        ] {
            let router = Router::new().route("/tasks/resume", post(resume_tasks));
            let (status, json) = send(router, "POST", "/tasks/resume", Some(body)).await;
            assert_eq!(status, StatusCode::BAD_REQUEST, "body {body}");
            assert_eq!(json["error"]["code"], "INVALID_INPUT", "body {body}");
        }
    }

    /// Every copy error answers with its own API code and catalog message, so the copy
    /// API and the per-task result of a bulk resume name the same failure.
    #[tokio::test]
    async fn every_copy_error_has_its_code_and_message() {
        let cases = [
            (
                Error::CopyTaskNotFound { task_id: 7 },
                "NOT_FOUND",
                "errors-copy-task-not-found",
            ),
            (
                Error::CopyHoldingNotFound {
                    task_id: 7,
                    mint: "Mint111".to_owned(),
                },
                "NOT_FOUND",
                "errors-copy-holding-not-found",
            ),
            (
                Error::CopyTaskRejected {
                    reason: CopySkip::LiveConfirmationRequired,
                },
                "LIVE_CONFIRMATION_REQUIRED",
                "errors-copy-live-confirmation-required",
            ),
            (
                Error::CopyTaskRejected {
                    reason: CopySkip::ModeTransitionRequired,
                },
                "INVALID_TASK",
                "errors-copy-task-invalid",
            ),
            (
                Error::CopyValidation {
                    detail: "unknown field".to_owned(),
                },
                "INVALID_INPUT",
                "errors-copy-request-rejected",
            ),
            (
                Error::CopyTaskLimit { maximum: 2 },
                "TASK_LIMIT",
                "errors-copy-task-limit",
            ),
            (
                Error::CopyWatchRejected {
                    detail: "refused".to_owned(),
                },
                "WATCH_REJECTED",
                "errors-copy-watch-rejected",
            ),
            (
                Error::CopyLiveUnavailable {
                    reason: CopyLiveBlock::CopyTradingDisabled,
                },
                "LIVE_UNAVAILABLE",
                "errors-copy-live-unavailable",
            ),
            (
                Error::CopyTaskLive { task_id: 7 },
                "TASK_LIVE",
                "errors-copy-task-live",
            ),
            (
                Error::CopyTaskOwnsPositions {
                    task_id: 7,
                    open_positions: 1,
                },
                "OPEN_POSITIONS",
                "errors-copy-task-owns-positions",
            ),
            (
                Error::CopyReconciliation {
                    detail: "rollback failed".to_owned(),
                },
                "COPY_ERROR",
                "errors-copy-request-failed",
            ),
            (
                Error::ChainAccess(crate::chains::Error::CapabilityUnavailable {
                    chain: crate::chains::ChainId::Solana,
                    capability: crate::chains::access::Capability::WatchOtherWallets,
                }),
                "CAPABILITY_UNAVAILABLE",
                "errors-chain-capability-unavailable",
            ),
        ];
        for (error, code, message) in cases {
            assert_eq!(error.copy_ui_text().id, message, "{error:?}");
            let response = copy_error(&error);
            let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
                .await
                .expect("body");
            let json: serde_json::Value = serde_json::from_slice(&bytes).expect("json");
            assert_eq!(json["error"]["code"], code, "{error:?}");
            assert_eq!(json["error"]["text"]["id"], message, "{error:?}");
        }
    }
}
