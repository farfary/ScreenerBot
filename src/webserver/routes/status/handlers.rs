use axum::response::Response;
use chrono::Utc;

use crate::{
    logger::{self, LogTag},
    webserver::{
        snapshot::{
            collect_service_status_snapshot, gather_status_snapshot, get_cached_system_metrics,
        },
        utils::success_response,
    },
};

use super::types::*;

/// GET /api/health
pub(super) async fn health_check() -> Response {
    logger::debug(LogTag::Webserver, "Health check endpoint called");

    let response = HealthResponse {
        status: "ok".to_owned(),
        timestamp: Utc::now(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    };

    success_response(response)
}

/// GET /api/status
pub(super) async fn system_status() -> Response {
    logger::info(LogTag::Webserver, "Fetching system status snapshot");

    let snapshot = gather_status_snapshot().await;

    logger::info(
        LogTag::Webserver,
        &format!(
            "Status snapshot ready (uptime={}s, trading_enabled={}, open_positions={})",
            snapshot.uptime_seconds, snapshot.trading_enabled, snapshot.open_positions
        ),
    );

    success_response(snapshot)
}

/// GET /api/status/services
pub(super) async fn service_status() -> Response {
    logger::info(LogTag::Webserver, "Fetching service status snapshot");

    let services = collect_service_status_snapshot();
    success_response(services)
}

/// GET /api/status/metrics
pub(super) async fn system_metrics() -> Response {
    logger::info(LogTag::Webserver, "Fetching system metrics snapshot");

    let metrics = get_cached_system_metrics().await;
    success_response(metrics)
}
