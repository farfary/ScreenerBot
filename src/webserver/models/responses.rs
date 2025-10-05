/// API response type definitions
///
/// Standard response structures for REST API endpoints

use chrono::{ DateTime, Utc };
use serde::{ Deserialize, Serialize };

// ================================================================================================
// Phase 1: System Status Responses
// ================================================================================================

/// Complete system status response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemStatusResponse {
    pub timestamp: DateTime<Utc>,
    pub uptime_seconds: u64,
    pub uptime_formatted: String,
    pub services: ServiceStatusResponse,
    pub metrics: SystemMetricsResponse,
    pub trading_enabled: bool,
}

/// Service readiness status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceStatusResponse {
    pub tokens_system: ServiceState,
    pub positions_system: ServiceState,
    pub pool_service: ServiceState,
    pub security_analyzer: ServiceState,
    pub transactions_system: ServiceState,
    pub all_ready: bool,
}

/// Individual service state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceState {
    pub ready: bool,
    pub last_check: DateTime<Utc>,
    pub error: Option<String>,
}

/// System resource metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetricsResponse {
    pub memory_usage_mb: u64,
    pub cpu_usage_percent: f32,
    pub active_threads: usize,
    pub rpc_calls_total: u64,
    pub rpc_calls_failed: u64,
    pub rpc_success_rate: f32,
    pub ws_connections: usize,
}

/// Simple health check response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub timestamp: DateTime<Utc>,
    pub version: String,
}

// ================================================================================================
// Service Management Responses
// ================================================================================================

/// Complete service information for a single service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceDetailResponse {
    pub name: String,
    pub priority: i32,
    pub dependencies: Vec<String>,
    pub enabled: bool,
    pub health: crate::services::ServiceHealth,
    pub metrics: crate::services::ServiceMetrics,
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
    pub health: crate::services::ServiceHealth,
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

/// Generic error response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: ErrorDetails,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorDetails {
    pub code: String,
    pub message: String,
    pub details: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub request_id: Option<String>,
}

/// Generic success response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuccessResponse {
    pub success: bool,
    pub message: String,
    pub timestamp: DateTime<Utc>,
}

/// Paginated response wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub pagination: PaginationInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationInfo {
    pub total_count: usize,
    pub page: usize,
    pub page_size: usize,
    pub total_pages: usize,
    pub has_next: bool,
    pub has_prev: bool,
}
