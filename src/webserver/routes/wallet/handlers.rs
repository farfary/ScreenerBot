use axum::response::Json;
use axum::Json as AxumJson;
use std::collections::HashMap;

use crate::logger::{self, LogTag};
use crate::wallet::{
    clear_dashboard_api_cache, get_current_wallet_status, get_dashboard_cache_metrics,
    get_flow_cache_stats, get_snapshot_token_balances, get_wallet_dashboard_data,
    refresh_dashboard_cache,
};
use super::types::*;

/// Get current wallet balance
pub(super) async fn get_wallet_current() -> Json<Option<WalletCurrentResponse>> {
    // Return demo data if demo mode is enabled
    if crate::webserver::demo::is_demo_mode() {
        return Json(Some(crate::webserver::demo::get_demo_wallet_current()));
    }

    match get_current_wallet_status().await {
        Ok(Some(snapshot)) => {
            // token_balances is not populated by get_recent_snapshots — load separately
            let raw_balances = if let Some(id) = snapshot.id {
                get_snapshot_token_balances(id).await.unwrap_or_default()
            } else {
                vec![]
            };

            let token_balances = raw_balances
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

/// Get wallet balance (alias for get_wallet_current)
pub(super) async fn get_wallet_balance() -> Json<Option<WalletCurrentResponse>> {
    get_wallet_current().await
}

/// Get wallet token holdings with enriched metadata
pub(super) async fn get_wallet_tokens() -> Json<WalletTokensResponse> {
    // Return demo data if demo mode is enabled
    if crate::webserver::demo::is_demo_mode() {
        return Json(crate::webserver::demo::get_demo_wallet_tokens());
    }

    // Get current wallet snapshot
    let snapshot = match get_current_wallet_status().await {
        Ok(Some(s)) => s,
        Ok(None) => {
            return Json(WalletTokensResponse { tokens: vec![] });
        }
        Err(err) => {
            logger::warning(
                LogTag::Webserver,
                &format!("Failed to get wallet status for tokens: {err}"),
            );
            return Json(WalletTokensResponse { tokens: vec![] });
        }
    };

    // token_balances is not populated by get_recent_snapshots — load separately
    let token_balances = if let Some(id) = snapshot.id {
        get_snapshot_token_balances(id).await.unwrap_or_default()
    } else {
        vec![]
    };

    // Collect mints from token balances
    let mints: Vec<String> = token_balances.iter().map(|tb| tb.mint.clone()).collect();

    // Fetch token metadata in batch
    let mut metadata_map: HashMap<String, (Option<String>, Option<String>)> = HashMap::new();

    // Try to get metadata for each token from token database
    if let Some(db) = crate::tokens::database::get_global_database() {
        for mint in &mints {
            if let Ok(Some(meta)) = db.get_token(mint) {
                metadata_map.insert(mint.clone(), (meta.symbol.clone(), meta.name.clone()));
            }
        }
    }

    // Build response with enriched data
    let tokens: Vec<WalletTokenHolding> = token_balances
        .iter()
        .map(|tb| {
            let (symbol, name) = metadata_map.get(&tb.mint).cloned().unwrap_or((None, None));

            WalletTokenHolding {
                mint: tb.mint.clone(),
                symbol,
                name,
                balance: tb.balance,
                ui_amount: tb.balance_ui,
                decimals: tb.decimals,
                is_token_2022: tb.is_token_2022,
            }
        })
        .collect();

    Json(WalletTokensResponse { tokens })
}

pub(super) async fn get_wallet_dashboard(
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

pub(super) async fn refresh_wallet_dashboard(
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

pub(super) async fn get_wallet_flow_cache_stats() -> Json<WalletFlowCacheResponse> {
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

pub(super) async fn get_wallet_cache_metrics() -> Json<WalletCacheMetricsResponse> {
    let data = get_dashboard_cache_metrics().await;
    Json(WalletCacheMetricsResponse { data })
}
