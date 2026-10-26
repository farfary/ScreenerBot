// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Status API handlers - health check, system and service status snapshots, and cached system metrics.

use axum::{
    extract::{ConnectInfo, Request},
    response::{IntoResponse as _, Response},
};
use chrono::Utc;
use std::net::SocketAddr;

use crate::{
    i18n::ids,
    logger::{self, LogTag},
    webserver::{
        api_error::{ApiError, ApiErrorCode},
        promo,
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
        instance_id: crate::global::instance_id().to_owned(),
    };

    success_response(response)
}

/// Whether the connection's peer is this machine. A request without a recorded peer
/// address is not.
pub(super) fn is_loopback_peer(peer: Option<&ConnectInfo<SocketAddr>>) -> bool {
    peer.is_some_and(|ConnectInfo(addr)| addr.ip().to_canonical().is_loopback())
}

/// GET /api/health/restart-safety
///
/// Answers only `safe_to_restart`, and only to a loopback peer: the service manager on
/// the same machine reads it to time an automatic update without a dashboard login.
pub(super) async fn restart_safety(request: Request) -> Response {
    let peer = request.extensions().get::<ConnectInfo<SocketAddr>>();
    if !is_loopback_peer(peer) {
        logger::warning(
            LogTag::Webserver,
            &format!(
                "Refused the restart check from a non-loopback peer: {}",
                peer.map_or_else(
                    || "unknown".to_string(),
                    |ConnectInfo(addr)| addr.to_string()
                )
            ),
        );
        return ApiError::new(
            ApiErrorCode::InvalidLocalRequest,
            ids::ERRORS_SECURITY_INVALID_LOCAL_REQUEST,
        )
        .into_response();
    }
    success_response(RestartSafetyResponse {
        safe_to_restart: crate::version::is_restart_safe().await,
    })
}

/// GET /api/status
pub(super) async fn system_status() -> Response {
    logger::debug(LogTag::Webserver, "Fetching system status snapshot");

    let mut snapshot = gather_status_snapshot().await;
    if promo::are_promo_fixtures_enabled() {
        promo::apply_promo_status(&mut snapshot);
    }

    logger::debug(
        LogTag::Webserver,
        &format!(
            "Status snapshot ready (uptime={}s, trading_enabled={}, open_positions={:?})",
            snapshot.uptime_seconds, snapshot.trading_enabled, snapshot.open_positions
        ),
    );

    success_response(snapshot)
}

/// GET /api/status/services
pub(super) async fn service_status() -> Response {
    logger::debug(LogTag::Webserver, "Fetching service status snapshot");

    let services = collect_service_status_snapshot();
    success_response(services)
}

/// GET /api/status/metrics
pub(super) async fn system_metrics() -> Response {
    logger::debug(LogTag::Webserver, "Fetching system metrics snapshot");

    let metrics = get_cached_system_metrics().await;
    success_response(metrics)
}
