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

// ================================================================================================
// Phase 2: Position Responses (Future)
// ================================================================================================

// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub struct PositionResponse {
//     pub id: i64,
//     pub token_mint: String,
//     pub token_symbol: Option<String>,
//     pub entry_price_sol: f64,
//     pub current_price_sol: Option<f64>,
//     pub amount: f64,
//     pub invested_sol: f64,
//     pub current_value_sol: Option<f64>,
//     pub pnl_sol: Option<f64>,
//     pub pnl_percent: Option<f64>,
//     pub entry_time: DateTime<Utc>,
//     pub exit_time: Option<DateTime<Utc>>,
//     pub status: String,
// }

// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub struct PositionListResponse {
//     pub positions: Vec<PositionResponse>,
//     pub total_count: usize,
//     pub open_count: usize,
//     pub closed_count: usize,
// }

// ================================================================================================
// Phase 2: Token Responses (Future)
// ================================================================================================

// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub struct TokenResponse {
//     pub mint: String,
//     pub symbol: Option<String>,
//     pub name: Option<String>,
//     pub decimals: u8,
//     pub total_supply: Option<String>,
//     pub liquidity_usd: Option<f64>,
//     pub market_cap_usd: Option<f64>,
//     pub price_sol: Option<f64>,
//     pub price_usd: Option<f64>,
//     pub security_score: Option<f64>,
//     pub is_blacklisted: bool,
//     pub last_updated: DateTime<Utc>,
// }

// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub struct TokenSearchResponse {
//     pub tokens: Vec<TokenResponse>,
//     pub total_count: usize,
//     pub page: usize,
//     pub page_size: usize,
// }

// ================================================================================================
// Phase 2: Transaction Responses (Future)
// ================================================================================================

// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub struct TransactionResponse {
//     pub signature: String,
//     pub block_time: DateTime<Utc>,
//     pub transaction_type: String,
//     pub status: String,
//     pub fee_sol: f64,
//     pub swap_details: Option<SwapDetails>,
// }

// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub struct SwapDetails {
//     pub from_mint: String,
//     pub to_mint: String,
//     pub from_amount: f64,
//     pub to_amount: f64,
//     pub price_impact: Option<f64>,
// }

// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub struct TransactionListResponse {
//     pub transactions: Vec<TransactionResponse>,
//     pub total_count: usize,
//     pub page: usize,
//     pub page_size: usize,
// }

// ================================================================================================
// Phase 3: Analytics Responses (Future)
// ================================================================================================

// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub struct PerformanceResponse {
//     pub total_trades: usize,
//     pub winning_trades: usize,
//     pub losing_trades: usize,
//     pub win_rate: f32,
//     pub total_pnl_sol: f64,
//     pub total_pnl_percent: f64,
//     pub average_trade_duration_seconds: u64,
//     pub best_trade_pnl_sol: f64,
//     pub worst_trade_pnl_sol: f64,
// }

// ================================================================================================
// Common Response Types
// ================================================================================================

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
