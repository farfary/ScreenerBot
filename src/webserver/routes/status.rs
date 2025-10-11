use axum::{response::Response, routing::get, Router};
use chrono::Utc;
use serde::Serialize;
use std::sync::Arc;

use crate::{
    arguments::is_debug_webserver_enabled,
    logger::{log, LogTag},
    webserver::{
        state::AppState,
        status_snapshot::{
            gather_status_snapshot, ServiceStatusSnapshot, StatusSnapshot, SystemMetricsSnapshot,
        },
        utils::success_response,
    },
};

/// Simple health check response
#[derive(Debug, Clone, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub version: String,
}

/// Create status routes
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/health", get(health_check))
        .route("/status", get(system_status))
        .route("/status/services", get(service_status))
        .route("/status/metrics", get(system_metrics))
}

/// GET /api/health
async fn health_check() -> Response {
    if is_debug_webserver_enabled() {
        log(LogTag::Webserver, "DEBUG", "Health check endpoint called");
    }

    let response = HealthResponse {
        status: "ok".to_string(),
        timestamp: Utc::now(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    };

    success_response(response)
}

/// GET /api/status
async fn system_status() -> Response {
    if is_debug_webserver_enabled() {
        log(
            LogTag::Webserver,
            "DEBUG",
            "Fetching system status snapshot",
        );
    }

    let snapshot: StatusSnapshot = gather_status_snapshot().await;

    if is_debug_webserver_enabled() {
        log(
            LogTag::Webserver,
            "DEBUG",
            &format!(
                "Status snapshot ready (uptime={}s, trading_enabled={}, open_positions={})",
                snapshot.uptime_seconds, snapshot.trading_enabled, snapshot.open_positions
            ),
        );
    }

    success_response(snapshot)
}

/// GET /api/status/services
async fn service_status() -> Response {
    if is_debug_webserver_enabled() {
        log(
            LogTag::Webserver,
            "DEBUG",
            "Fetching service status snapshot",
        );
    }

    let services: ServiceStatusSnapshot = gather_status_snapshot().await.services;
    success_response(services)
}

/// GET /api/status/metrics
async fn system_metrics() -> Response {
    if is_debug_webserver_enabled() {
        log(
            LogTag::Webserver,
            "DEBUG",
            "Fetching system metrics snapshot",
        );
    }

    let metrics: SystemMetricsSnapshot = gather_status_snapshot().await.metrics;
    success_response(metrics)
}
