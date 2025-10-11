use axum::{http::StatusCode, response::Response, routing::post, Router};
use chrono::Utc;
use serde::Serialize;
use std::sync::Arc;

use crate::{
    filtering,
    logger::{log, LogTag},
    webserver::state::AppState,
    webserver::utils::{error_response, success_response},
};

/// Filtering management routes
pub fn routes() -> Router<Arc<AppState>> {
    Router::new().route("/filtering/refresh", post(trigger_refresh))
}

#[derive(Debug, Serialize)]
struct RefreshResponse {
    message: String,
    timestamp: String,
}

/// POST /api/filtering/refresh
/// Force a synchronous rebuild of the filtering snapshot so downstream
/// consumers see the newly-saved configuration immediately.
async fn trigger_refresh() -> Response {
    match filtering::refresh().await {
        Ok(()) => {
            log(
                LogTag::Filtering,
                "REFRESH_TRIGGERED",
                "Filtering snapshot rebuilt via API request",
            );

            success_response(RefreshResponse {
                message: "Filtering snapshot rebuilt".to_string(),
                timestamp: Utc::now().to_rfc3339(),
            })
        }
        Err(err) => {
            log(
                LogTag::Filtering,
                "REFRESH_FAILED",
                &format!("Filtering refresh failed: {}", err),
            );

            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "FILTERING_REFRESH_FAILED",
                &format!("Failed to rebuild filtering snapshot: {}", err),
                None,
            )
        }
    }
}
