use axum::{
    response::Json,
    routing::{get, post},
    Json as AxumJson, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::logger::{self, LogTag};
use crate::wallet::{
    clear_dashboard_api_cache, get_current_wallet_status, get_dashboard_cache_metrics,
    get_flow_cache_stats, get_wallet_dashboard_data, refresh_dashboard_cache,
    CachePerformanceMetrics, WalletDashboardData, WalletFlowCacheStats,
};
use crate::webserver::state::AppState;

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
    pub decimals: Option<u8>,
    pub is_token_2022: bool,
}

/// Create wallet routes
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/wallet/current", get(get_wallet_current))
        .route("/wallet/dashboard", post(get_wallet_dashboard))
        .route("/wallet/dashboard/refresh", post(refresh_wallet_dashboard))
        .route("/wallet/flow-cache", get(get_wallet_flow_cache_stats))
        .route("/wallet/cache-metrics", get(get_wallet_cache_metrics))
}

/// Get current wallet balance
async fn get_wallet_current() -> Json<Option<WalletCurrentResponse>> {
    match get_current_wallet_status().await {
        Ok(Some(snapshot)) => {
            let token_balances = snapshot
                .token_balances
                .iter()
                .map(|tb| TokenBalanceInfo {
                    mint: tb.mint.clone(),
                    balance: tb.balance,
                    balance_ui: tb.balance_ui,
                    decimals: tb.decimals,
                    is_token_2022: tb.is_token_2022,
                })
                .collect();

            Json(Some(WalletCurrentResponse {
                sol_balance: snapshot.sol_balance,
                sol_balance_lamports: snapshot.sol_balance_lamports,
                total_tokens_count: snapshot.total_tokens_count,
                token_balances,
                snapshot_time: snapshot.snapshot_time.to_rfc3339(),
            }))
        }
        _ => Json(None),
    }
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
    pub data: Option<WalletDashboardData>,
    pub error: Option<String>,
}

async fn get_wallet_dashboard(
    AxumJson(request): AxumJson<WalletDashboardRequest>,
) -> Json<WalletDashboardResponse> {
    match get_wallet_dashboard_data(
        request.window_hours,
        request.snapshot_limit,
        request.max_tokens,
    )
    .await
    {
        Ok(payload) => Json(WalletDashboardResponse {
            data: Some(payload),
            error: None,
        }),
        Err(err) => Json(WalletDashboardResponse {
            data: None,
            error: Some(err),
        }),
    }
}

async fn refresh_wallet_dashboard(
    AxumJson(request): AxumJson<WalletDashboardRequest>,
) -> Json<WalletDashboardResponse> {
    match refresh_dashboard_cache(request.window_hours).await {
        Ok(_) => {
            clear_dashboard_api_cache().await;
        }
        Err(err) => {
            logger::warning(
                LogTag::Wallet,
                &format!(
                    "Failed to refresh dashboard cache for {}h: {}",
                    request.window_hours, err
                ),
            );
        }
    }

    get_wallet_dashboard(AxumJson(request)).await
}

#[derive(Debug, Serialize)]
pub struct WalletFlowCacheResponse {
    pub data: Option<WalletFlowCacheStats>,
    pub error: Option<String>,
}

async fn get_wallet_flow_cache_stats() -> Json<WalletFlowCacheResponse> {
    let stats = get_flow_cache_stats().await;
    match stats {
        Ok(data) => Json(WalletFlowCacheResponse {
            data: Some(data),
            error: None,
        }),
        Err(err) => Json(WalletFlowCacheResponse {
            data: None,
            error: Some(err),
        }),
    }
}

#[derive(Debug, Serialize)]
pub struct WalletCacheMetricsResponse {
    pub data: CachePerformanceMetrics,
}

async fn get_wallet_cache_metrics() -> Json<WalletCacheMetricsResponse> {
    let data = get_dashboard_cache_metrics().await;
    Json(WalletCacheMetricsResponse { data })
}
