use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct HeaderMetricsResponse {
    pub trader: TraderHeaderInfo,
    pub wallet: WalletHeaderInfo,
    pub positions: PositionsHeaderInfo,
    pub rpc: RpcHeaderInfo,
    pub filtering: FilteringHeaderInfo,
    pub system: SystemHeaderInfo,
    pub sol: SolHeaderInfo,
    pub timestamp: String,
}

/// SOL/USD price for the header price card (click opens the SOL chart dialog).
#[derive(Debug, Serialize)]
pub struct SolHeaderInfo {
    pub price_usd: f64,
    pub change_24h_percent: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct TraderHeaderInfo {
    pub running: bool,
    pub enabled: bool,
    pub today_pnl_sol: f64,
    pub today_pnl_percent: f64,
    pub uptime_seconds: u64,
}

#[derive(Debug, Serialize)]
pub struct WalletHeaderInfo {
    pub sol_balance: f64,
    pub change_24h_sol: f64,
    pub change_24h_percent: f64,
    pub token_count: usize,
    pub tokens_worth_sol: f64,
    pub last_updated: String,
}

#[derive(Debug, Serialize)]
pub struct PositionsHeaderInfo {
    pub open_count: i64,
    pub unrealized_pnl_sol: f64,
    pub unrealized_pnl_percent: f64,
    pub total_invested_sol: f64,
}

#[derive(Debug, Serialize)]
pub struct RpcHeaderInfo {
    pub success_rate_percent: f32,
    pub avg_latency_ms: u64,
    pub calls_per_minute: f64,
    pub healthy: bool,
}

#[derive(Debug, Serialize)]
pub struct FilteringHeaderInfo {
    pub monitoring_count: usize,
    pub passed_count: usize,
    pub rejected_count: usize,
    pub last_refresh: String,
}

#[derive(Debug, Serialize)]
pub struct SystemHeaderInfo {
    pub all_services_healthy: bool,
    pub unhealthy_services: Vec<String>,
    pub critical_degraded: bool,
}
