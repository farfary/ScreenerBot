use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct WalletQrResponse {
    pub address: String,
    pub qr_data_url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WalletCurrentResponse {
    pub sol_balance: f64,
    pub sol_balance_lamports: u64,
    pub total_tokens_count: u32,
    pub token_balances: Vec<TokenBalanceInfo>,
    pub snapshot_time: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TokenBalanceInfo {
    pub mint: String,
    pub balance: u64,
    pub balance_ui: f64,
    pub decimals: u8,
    pub is_token_2022: bool,
}

#[derive(Debug, Serialize)]
pub struct WalletTokensResponse {
    pub tokens: Vec<WalletTokenHolding>,
}

#[derive(Debug, Serialize)]
pub struct WalletTokenHolding {
    pub mint: String,
    pub symbol: Option<String>,
    pub name: Option<String>,
    pub logo_url: Option<String>,
    pub balance: u64,
    pub ui_amount: f64,
    pub decimals: u8,
    pub is_token_2022: bool,
    /// Latest known token price in SOL (None when no market data is available).
    pub price_sol: Option<f64>,
    /// Holding value in SOL (ui_amount * price_sol; None when price is unknown).
    pub value_sol: Option<f64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WalletDashboardRequest {
    #[serde(default = "default_window_hours")]
    pub window_hours: i64,
    #[serde(default = "default_snapshot_limit")]
    pub snapshot_limit: usize,
    #[serde(default = "default_token_limit")]
    pub max_tokens: usize,
}

fn default_window_hours() -> i64 {
    24
}

fn default_snapshot_limit() -> usize {
    600
}

fn default_token_limit() -> usize {
    250
}

#[derive(Debug, Serialize)]
pub struct WalletDashboardResponse {
    pub data: Option<crate::wallet::WalletDashboardData>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct WalletFlowCacheResponse {
    pub data: Option<crate::wallet::WalletFlowCacheStats>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct WalletCacheMetricsResponse {
    pub data: crate::wallet::CachePerformanceMetrics,
}
