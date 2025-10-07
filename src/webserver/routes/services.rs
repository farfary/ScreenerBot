use axum::{
    extract::{ Path, State },
    http::StatusCode,
    response::{ IntoResponse, Response },
    routing::get,
    Router,
};
use chrono::{ DateTime, Utc };
use serde::{ Deserialize, Serialize };
use std::sync::Arc;

use crate::{
    arguments::is_debug_webserver_enabled,
    logger::{ log, LogTag },
    services::{ ServiceHealth, ServiceMetrics },
    webserver::{ state::AppState, utils::success_response },
};

// ================================================================================================
// Response Types
// ================================================================================================

/// Complete service information for a single service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceDetailResponse {
    pub name: String,
    pub priority: i32,
    pub dependencies: Vec<String>,
    pub enabled: bool,
    pub health: ServiceHealth,
    pub metrics: ServiceMetrics,
    pub uptime_seconds: u64,
}

/// List of all services with their status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServicesListResponse {
    pub services: Vec<ServiceDetailResponse>,
    pub total_count: usize,
    pub healthy_count: usize,
    pub unhealthy_count: usize,
    pub starting_count: usize,
    pub timestamp: DateTime<Utc>,
}

/// Service dependency graph node for visualization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceDependencyNode {
    pub name: String,
    pub priority: i32,
    pub dependencies: Vec<String>,
    pub health: ServiceHealth,
}

/// Complete services overview for dashboard
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServicesOverviewResponse {
    pub services: Vec<ServiceDetailResponse>,
    pub dependency_graph: Vec<ServiceDependencyNode>,
    pub summary: ServicesSummary,
    pub timestamp: DateTime<Utc>,
}

/// Summary statistics for all services
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServicesSummary {
    pub total_services: usize,
    pub enabled_services: usize,
    pub healthy_services: usize,
    pub degraded_services: usize,
    pub unhealthy_services: usize,
    pub starting_services: usize,
    pub all_healthy: bool,
}

// ================================================================================================
// Route Handlers
// ================================================================================================

/// Create services management routes
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/services", get(list_services))
        .route("/services/:name", get(get_service))
        .route("/services/overview", get(services_overview))
}

/// GET /api/services
/// List all services with their current status
async fn list_services(State(state): State<Arc<AppState>>) -> Response {
    if is_debug_webserver_enabled() {
        log(LogTag::Webserver, "DEBUG", "Fetching all services list");
    }

    let service_names = state.get_all_services().await;
    let health_map = state.get_all_services_health().await;
    let metrics_map = state.get_service_metrics().await;

    let mut services = Vec::new();
    let mut healthy_count = 0;
    let mut unhealthy_count = 0;
    let mut starting_count = 0;

    for name in service_names {
        if let Some(details) = state.get_service_details(name).await {
            let health = health_map
                .get(name)
                .cloned()
                .unwrap_or(ServiceHealth::Unhealthy("Health status unavailable".to_string()));
            let metrics = metrics_map.get(name).cloned().unwrap_or_default();

            // Count health statuses
            match &health {
                ServiceHealth::Healthy => {
                    healthy_count += 1;
                }
                ServiceHealth::Unhealthy(_) | ServiceHealth::Degraded(_) => {
                    unhealthy_count += 1;
                }
                ServiceHealth::Starting => {
                    starting_count += 1;
                }
                _ => {}
            }

            let uptime = metrics.uptime_seconds;
            services.push(ServiceDetailResponse {
                name: name.to_string(),
                priority: details.priority,
                dependencies: details.dependencies
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                enabled: details.enabled,
                health,
                metrics,
                uptime_seconds: uptime,
            });
        }
    }

    // Sort by priority
    services.sort_by_key(|s| s.priority);

    let response = ServicesListResponse {
        total_count: services.len(),
        healthy_count,
        unhealthy_count,
        starting_count,
        services,
        timestamp: Utc::now(),
    };

    success_response(response)
}

/// GET /api/services/:name
/// Get detailed information about a specific service
async fn get_service(Path(name): Path<String>, State(state): State<Arc<AppState>>) -> Response {
    log(LogTag::Webserver, "DEBUG", &format!("Fetching service details for: {}", name));

    // Get service details
    let details = match state.get_service_details(&name).await {
        Some(d) => d,
        None => {
            return (StatusCode::NOT_FOUND, format!("Service '{}' not found", name)).into_response();
        }
    };

    // Get health and metrics
    let health = state
        .get_service_health(&name).await
        .unwrap_or(ServiceHealth::Unhealthy("Health status unavailable".to_string()));

    let metrics_map = state.get_service_metrics().await;
    let metrics = metrics_map.get(name.as_str()).cloned().unwrap_or_default();
    let uptime = metrics.uptime_seconds;

    let response = ServiceDetailResponse {
        name: name.clone(),
        priority: details.priority,
        dependencies: details.dependencies
            .iter()
            .map(|s| s.to_string())
            .collect(),
        enabled: details.enabled,
        health,
        metrics,
        uptime_seconds: uptime,
    };

    success_response(response)
}

/// GET /api/services/overview
/// Complete services overview with dependency graph and summary
async fn services_overview(State(state): State<Arc<AppState>>) -> Response {
    log(LogTag::Webserver, "DEBUG", "Fetching complete services overview");

    let service_names = state.get_all_services().await;
    let health_map = state.get_all_services_health().await;
    let metrics_map = state.get_service_metrics().await;

    let mut services = Vec::new();
    let mut dependency_graph = Vec::new();
    let mut enabled_count = 0;
    let mut healthy_count = 0;
    let mut degraded_count = 0;
    let mut unhealthy_count = 0;
    let mut starting_count = 0;

    for name in service_names {
        if let Some(details) = state.get_service_details(name).await {
            let health = health_map
                .get(name)
                .cloned()
                .unwrap_or(ServiceHealth::Unhealthy("Health status unavailable".to_string()));
            let metrics = metrics_map.get(name).cloned().unwrap_or_default();

            if details.enabled {
                enabled_count += 1;
            }

            // Count health statuses
            match &health {
                ServiceHealth::Healthy => {
                    healthy_count += 1;
                }
                ServiceHealth::Degraded(_) => {
                    degraded_count += 1;
                }
                ServiceHealth::Unhealthy(_) => {
                    unhealthy_count += 1;
                }
                ServiceHealth::Starting => {
                    starting_count += 1;
                }
                _ => {}
            }

            // Build dependency graph node
            dependency_graph.push(ServiceDependencyNode {
                name: name.to_string(),
                priority: details.priority,
                dependencies: details.dependencies
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                health: health.clone(),
            });

            // Build full service detail
            let uptime = metrics.uptime_seconds;
            services.push(ServiceDetailResponse {
                name: name.to_string(),
                priority: details.priority,
                dependencies: details.dependencies
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                enabled: details.enabled,
                health,
                metrics,
                uptime_seconds: uptime,
            });
        }
    }

    // Sort by priority
    services.sort_by_key(|s| s.priority);
    dependency_graph.sort_by_key(|s| s.priority);

    let summary = ServicesSummary {
        total_services: services.len(),
        enabled_services: enabled_count,
        healthy_services: healthy_count,
        degraded_services: degraded_count,
        unhealthy_services: unhealthy_count,
        starting_services: starting_count,
        all_healthy: unhealthy_count == 0 && degraded_count == 0 && starting_count == 0,
    };

    let response = ServicesOverviewResponse {
        services,
        dependency_graph,
        summary,
        timestamp: Utc::now(),
    };

    success_response(response)
}
