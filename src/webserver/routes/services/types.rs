use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::services::{ServiceHealth, ServiceMetrics};

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
    pub disabled_services: usize,
    pub all_healthy: bool,
}
