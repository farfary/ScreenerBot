//! Features API route
//!
//! Exposes feature availability to the dashboard.

use axum::{extract::Path, response::Response, routing::get, Router};
use serde::Serialize;
use std::sync::Arc;

use crate::{
    features::{get_features, get_tool_status, get_trading_feature_status, FeatureStatus},
    webserver::{state::AppState, utils::success_response},
};

/// Response for checking a specific feature
#[derive(Serialize)]
struct FeatureCheckResponse {
    id: String,
    status: FeatureStatus,
    available: bool,
    visible: bool,
}

/// Create features routes
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(get_all_features))
        .route("/tool/{tool_id}", get(check_tool))
        .route("/trading/{feature_id}", get(check_trading_feature))
}

/// GET /api/features
/// Returns all feature flags
async fn get_all_features() -> Response {
    success_response(get_features())
}

/// GET /api/features/tool/{tool_id}
/// Check if a specific tool is available
async fn check_tool(Path(tool_id): Path<String>) -> Response {
    let status = get_tool_status(&tool_id);
    success_response(FeatureCheckResponse {
        id: tool_id,
        status,
        available: status.is_usable(),
        visible: status.is_visible(),
    })
}

/// GET /api/features/trading/{feature_id}
/// Check if a specific trading feature is available
async fn check_trading_feature(Path(feature_id): Path<String>) -> Response {
    let status = get_trading_feature_status(&feature_id);
    success_response(FeatureCheckResponse {
        id: feature_id,
        status,
        available: status.is_usable(),
        visible: status.is_visible(),
    })
}
