use axum::{extract::Path, http::StatusCode, response::Response};
use std::collections::HashMap;

use crate::connectivity::{get_all_health, get_endpoint_health, get_unhealthy_critical_endpoints};
use crate::webserver::utils::{error_response, success_response};

use super::types::*;

/// GET /api/connectivity/status
/// Get overall connectivity status
pub(super) async fn get_connectivity_status() -> Response {
    let all_health = get_all_health().await;
    let unhealthy_critical = get_unhealthy_critical_endpoints().await;

    let mut endpoints = HashMap::new();
    let mut all_healthy = true;

    for (name, health) in &all_health {
        if !health.is_available() {
            all_healthy = false;
        }
        endpoints.insert(
            name.to_string(),
            EndpointHealthResponse::from(health.clone()),
        );
    }

    let response = ConnectivityStatusResponse {
        all_healthy,
        critical_healthy: unhealthy_critical.is_empty(),
        unhealthy_critical_endpoints: unhealthy_critical.iter().map(|s| s.to_string()).collect(),
        endpoints,
    };

    success_response(response)
}

/// GET /api/connectivity/status/:endpoint
/// Get status for a specific endpoint
pub(super) async fn get_endpoint_status(Path(endpoint): Path<String>) -> Response {
    match get_endpoint_health(&endpoint).await {
        Some(health) => {
            let response = EndpointHealthResponse::from(health);
            success_response(response)
        }
        None => error_response(
            StatusCode::NOT_FOUND,
            "NOT_FOUND",
            &format!("Endpoint '{endpoint}' not found or not monitored"),
            None,
        ),
    }
}
