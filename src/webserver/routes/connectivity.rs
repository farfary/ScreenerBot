use axum::{extract::Path, http::StatusCode, response::Response, routing::get, Router};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

use crate::{
    connectivity::{
        get_all_health, get_endpoint_health, get_unhealthy_critical_endpoints, EndpointHealth,
    },
    webserver::{
        state::AppState,
        utils::{error_response, success_response},
    },
};

/// Response for connectivity status overview
#[derive(Debug, Serialize, Deserialize)]
pub struct ConnectivityStatusResponse {
    pub all_healthy: bool,
    pub critical_healthy: bool,
    pub unhealthy_critical_endpoints: Vec<String>,
    pub endpoints: HashMap<String, EndpointHealthResponse>,
}

/// Serializable endpoint health response
#[derive(Debug, Serialize, Deserialize)]
pub struct EndpointHealthResponse {
    pub status: String,
    pub latency_ms: Option<u64>,
    pub message: Option<String>,
    pub last_check: Option<String>,
    pub last_success: Option<String>,
    pub consecutive_failures: Option<u32>,
}

impl From<EndpointHealth> for EndpointHealthResponse {
    fn from(health: EndpointHealth) -> Self {
        match health {
            EndpointHealth::Healthy {
                latency_ms,
                last_check,
            } => Self {
                status: "healthy".to_string(),
                latency_ms: Some(latency_ms),
                message: None,
                last_check: Some(last_check.to_rfc3339()),
                last_success: Some(last_check.to_rfc3339()),
                consecutive_failures: None,
            },
            EndpointHealth::Degraded {
                latency_ms,
                reason,
                last_check,
            } => Self {
                status: "degraded".to_string(),
                latency_ms: Some(latency_ms),
                message: Some(reason),
                last_check: Some(last_check.to_rfc3339()),
                last_success: Some(last_check.to_rfc3339()),
                consecutive_failures: None,
            },
            EndpointHealth::Unhealthy {
                reason,
                last_check,
                last_success,
                consecutive_failures,
            } => Self {
                status: "unhealthy".to_string(),
                latency_ms: None,
                message: Some(reason),
                last_check: Some(last_check.to_rfc3339()),
                last_success: last_success.map(|t| t.to_rfc3339()),
                consecutive_failures: Some(consecutive_failures),
            },
            EndpointHealth::Unknown => Self {
                status: "unknown".to_string(),
                latency_ms: None,
                message: Some("Not checked yet".to_string()),
                last_check: None,
                last_success: None,
                consecutive_failures: None,
            },
        }
    }
}

/// Create connectivity routes
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/status", get(get_connectivity_status))
        .route("/status/:endpoint", get(get_endpoint_status))
}

/// GET /api/connectivity/status
/// Get overall connectivity status
async fn get_connectivity_status() -> Response {
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
async fn get_endpoint_status(Path(endpoint): Path<String>) -> Response {
    match get_endpoint_health(&endpoint).await {
        Some(health) => {
            let response = EndpointHealthResponse::from(health);
            success_response(response)
        }
        None => error_response(
            StatusCode::NOT_FOUND,
            "NOT_FOUND",
            &format!("Endpoint '{}' not found or not monitored", endpoint),
            None,
        ),
    }
}
