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
// Snapshot Helpers
// ================================================================================================

/// Build a complete services overview snapshot directly from the global ServiceManager
pub async fn gather_services_overview_snapshot() -> ServicesOverviewResponse {
    use crate::services::get_service_manager;

    let mut services = Vec::new();
    let mut dependency_graph = Vec::new();
    let mut summary = ServicesSummary {
        total_services: 0,
        enabled_services: 0,
        healthy_services: 0,
        degraded_services: 0,
        unhealthy_services: 0,
        starting_services: 0,
        all_healthy: true,
    };

    if let Some(manager_ref) = get_service_manager().await {
        if let Some(manager) = manager_ref.read().await.as_ref() {
            let service_names = manager.get_all_service_names();
            let health_map = manager.get_health().await;
            let metrics_map = manager.get_metrics().await;

            for name in service_names {
                if let Some(service) = manager.get_service(name) {
                    let priority = service.priority();
                    let dependencies = service
                        .dependencies()
                        .iter()
                        .map(|dep| dep.to_string())
                        .collect::<Vec<_>>();
                    let enabled = manager.is_service_enabled(name);
                    let health = health_map
                        .get(name)
                        .cloned()
                        .unwrap_or(
                            ServiceHealth::Unhealthy("Health status unavailable".to_string())
                        );
                    let metrics = metrics_map
                        .get(name)
                        .cloned()
                        .unwrap_or_else(ServiceMetrics::default)
                        .sanitized();
                    let uptime_seconds = metrics.uptime_seconds;

                    if enabled {
                        summary.enabled_services += 1;
                    }

                    match &health {
                        ServiceHealth::Healthy => {
                            summary.healthy_services += 1;
                        }
                        ServiceHealth::Degraded(_) => {
                            summary.degraded_services += 1;
                        }
                        ServiceHealth::Unhealthy(_) => {
                            summary.unhealthy_services += 1;
                        }
                        ServiceHealth::Starting => {
                            summary.starting_services += 1;
                        }
                        ServiceHealth::Stopping => {
                            summary.unhealthy_services += 1;
                        }
                    }

                    dependency_graph.push(ServiceDependencyNode {
                        name: name.to_string(),
                        priority,
                        dependencies: dependencies.clone(),
                        health: health.clone(),
                    });

                    services.push(ServiceDetailResponse {
                        name: name.to_string(),
                        priority,
                        dependencies,
                        enabled,
                        health,
                        metrics,
                        uptime_seconds,
                    });
                }
            }
        }
    }

    services.sort_by_key(|service| service.priority);
    dependency_graph.sort_by_key(|service| service.priority);

    summary.total_services = services.len();
    summary.all_healthy =
        summary.unhealthy_services == 0 &&
        summary.degraded_services == 0 &&
        summary.starting_services == 0;

    ServicesOverviewResponse {
        services,
        dependency_graph,
        summary,
        timestamp: Utc::now(),
    }
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
async fn list_services(State(_state): State<Arc<AppState>>) -> Response {
    if is_debug_webserver_enabled() {
        log(LogTag::Webserver, "DEBUG", "Fetching all services list");
    }

    let overview = gather_services_overview_snapshot().await;
    let unhealthy_count = overview.summary.unhealthy_services + overview.summary.degraded_services;
    let response = ServicesListResponse {
        services: overview.services.clone(),
        total_count: overview.summary.total_services,
        healthy_count: overview.summary.healthy_services,
        unhealthy_count,
        starting_count: overview.summary.starting_services,
        timestamp: overview.timestamp,
    };

    success_response(response)
}

/// GET /api/services/:name
/// Get detailed information about a specific service
async fn get_service(Path(name): Path<String>, State(_state): State<Arc<AppState>>) -> Response {
    log(LogTag::Webserver, "DEBUG", &format!("Fetching service details for: {}", name));

    let overview = gather_services_overview_snapshot().await;

    match overview.services.into_iter().find(|svc| svc.name == name) {
        Some(service) => success_response(service),
        None => (StatusCode::NOT_FOUND, format!("Service '{}' not found", name)).into_response(),
    }
}

/// GET /api/services/overview
/// Complete services overview with dependency graph and summary
async fn services_overview(State(_state): State<Arc<AppState>>) -> Response {
    log(LogTag::Webserver, "DEBUG", "Fetching complete services overview");
    success_response(gather_services_overview_snapshot().await)
}
