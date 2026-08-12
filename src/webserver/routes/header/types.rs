use serde::Serialize;

use crate::filtering::SnapshotState;

#[derive(Debug, Serialize)]
pub struct HeaderMetricsResponse {
    pub trader: TraderHeaderInfo,
    pub wallet: WalletHeaderInfo,
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
    pub enabled: bool,
    pub state: TraderHeaderState,
    pub today_pnl_sol: f64,
    pub today_pnl_percent: f64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TraderHeaderState {
    Preview,
    ForceStopped,
    Stopped,
    Waiting,
    Idle,
    EntryPaused,
    Running,
}

/// The header's wallet card. `total_equity_sol` is the headline and MUST be the same
/// number the home hero shows — both come from `wallet::get_wallet_worth()`.
#[derive(Debug, Serialize)]
pub struct WalletHeaderInfo {
    /// Free (uninvested) SOL.
    pub sol_balance: f64,
    /// SOL value of the held tokens.
    pub tokens_worth_sol: f64,
    /// Full wallet worth: cash + holdings. The card's headline.
    pub total_equity_sol: f64,
    /// Change vs the start-of-day WORTH (same quantity as the headline, never cash).
    pub change_today_sol: Option<f64>,
    pub change_today_percent: Option<f64>,
    pub token_count: usize,
    pub last_updated: String,
}

#[derive(Debug, Serialize)]
pub struct RpcHeaderInfo {
    pub success_rate_percent: f32,
    pub avg_latency_ms: u64,
    pub calls_per_minute: f64,
    pub healthy: bool,
}

/// The header's filtering counters, all of them snapshot-derived and therefore NULL with
/// `snapshot_state: "building"` until the first snapshot exists — see
/// [`crate::filtering::SnapshotState`].
#[derive(Debug, Serialize)]
pub struct FilteringHeaderInfo {
    pub snapshot_state: SnapshotState,
    pub monitoring_count: Option<usize>,
    pub passed_count: Option<usize>,
    pub rejected_count: Option<usize>,
    pub last_refresh: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SystemHeaderInfo {
    pub all_services_healthy: bool,
    pub unhealthy_services: Vec<String>,
    pub critical_degraded: bool,
}
