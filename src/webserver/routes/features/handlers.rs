use axum::extract::Path;
use axum::response::Response;

use crate::{
    features::{get_features, get_tool_status, get_trading_feature_status},
    webserver::utils::success_response,
};

use super::types::*;

/// GET /api/features
/// Returns all feature flags
pub(super) async fn get_all_features() -> Response {
    success_response(get_features())
}

/// GET /api/features/tool/{tool_id}
/// Check if a specific tool is available
pub(super) async fn check_tool(Path(tool_id): Path<String>) -> Response {
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
pub(super) async fn check_trading_feature(Path(feature_id): Path<String>) -> Response {
    let status = get_trading_feature_status(&feature_id);
    success_response(FeatureCheckResponse {
        id: feature_id,
        status,
        available: status.is_usable(),
        visible: status.is_visible(),
    })
}
