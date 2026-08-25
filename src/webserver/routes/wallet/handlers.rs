use axum::{
    extract::Path,
    http::StatusCode,
    response::{Json, Response},
    Json as AxumJson,
};
use std::collections::HashMap;

use super::types::*;
use crate::logger::{self, LogTag};
use crate::wallet::{
    clear_dashboard_api_cache, get_current_wallet_status, get_dashboard_cache_metrics,
    get_flow_cache_stats, get_snapshot_token_balances, get_wallet_dashboard_data,
    refresh_dashboard_cache,
};
use crate::webserver::utils::{error_response, success_response};

/// Generate a QR code for the current main wallet address.
pub(super) async fn get_wallet_qr(Path(address): Path<String>) -> Response {
    let current_address = if crate::webserver::promo::are_promo_fixtures_enabled() {
        crate::webserver::promo::get_promo_wallet_address().to_owned()
    } else {
        match crate::wallets::get_main_address().await {
            Ok(address) => address,
            Err(err) => {
                return error_response(
                    StatusCode::NOT_FOUND,
                    "WALLET_NOT_FOUND",
                    "Main wallet is not available",
                    Some(&err.to_string()),
                );
            }
        }
    };

    if address != current_address {
        return error_response(
            StatusCode::BAD_REQUEST,
            "WALLET_CHANGED",
            "The main wallet changed; refresh and try again",
            None,
        );
    }

    match crate::webserver::totp::generate_qr_data_url_for_value(&address) {
        Ok(qr_data_url) => success_response(WalletQrResponse {
            address,
            qr_data_url,
        }),
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "QR_ERROR",
            "Could not generate the wallet QR code",
            Some(&err.to_string()),
        ),
    }
}

/// Get current wallet balance.
///
/// Served from the LIVE snapshot — the same source `get_wallet_worth()` reads for the
/// header card and the home hero. Reading the database instead made this endpoint a
/// second, lagging source of the wallet balance: the trade dialog sized orders against
/// a figure that could disagree with the one on screen, and it cost two locked SQLite
/// round-trips per call. The database is only consulted when the monitor has not
/// published a snapshot yet (first boot on an empty database).
pub(super) async fn get_wallet_current() -> Json<Option<WalletCurrentResponse>> {
    // Return promotional fixtures only for owner-initiated media capture.
    if crate::webserver::promo::are_promo_fixtures_enabled() {
        return Json(Some(crate::webserver::promo::get_promo_wallet_current()));
    }

    if let Some(snapshot) = crate::wallet::live_wallet_snapshot() {
        // The live snapshot already carries its token balances — no second query.
        return Json(Some(WalletCurrentResponse {
            sol_balance: snapshot.sol_balance,
            sol_balance_lamports: snapshot.sol_balance_lamports,
            total_tokens_count: snapshot.total_tokens_count,
            token_balances: snapshot
                .token_balances
                .iter()
                .map(|tb| TokenBalanceInfo {
                    mint: tb.mint.clone(),
                    balance: tb.balance,
                    balance_ui: tb.balance_ui,
                    decimals: tb.decimals,
                    is_token_2022: tb.is_token_2022,
                })
                .collect(),
            snapshot_time: snapshot.snapshot_time.to_rfc3339(),
        }));
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

/// Get wallet token holdings with enriched metadata.
///
/// Reads the live snapshot for the same reason `get_wallet_current` does: the holdings
/// list and the balance beside it must come from one source. Falls back to the database
/// only before the monitor has published anything.
pub(super) async fn get_wallet_tokens() -> Json<WalletTokensResponse> {
    // Return promotional fixtures only for owner-initiated media capture.
    if crate::webserver::promo::are_promo_fixtures_enabled() {
        return Json(crate::webserver::promo::get_promo_wallet_tokens());
    }

    let snapshot = match crate::wallet::live_wallet_snapshot() {
        Some(live) => (*live).clone(),
        None => match get_current_wallet_status().await {
            Ok(Some(s)) => s,
            Ok(None) => return Json(WalletTokensResponse { tokens: vec![] }),
            Err(err) => {
                logger::warning(
                    LogTag::Webserver,
                    &format!("Failed to get wallet status for tokens: {err}"),
                );
                return Json(WalletTokensResponse { tokens: vec![] });
            }
        },
    };

    Json(WalletTokensResponse {
        tokens: enrich_token_holdings(&snapshot).await,
    })
}

/// Force a fresh on-chain wallet snapshot, then return the enriched holdings.
///
/// Drives the dashboard "refresh" button: the SOL balance and token balances are
/// re-fetched from RPC (always), while token metadata is cache-first — only
/// never-before-seen mints trigger a metadata fetch (see [`enrich_token_holdings`]).
pub(super) async fn refresh_wallet_tokens() -> Json<WalletTokensResponse> {
    if crate::webserver::promo::are_promo_fixtures_enabled() {
        return Json(crate::webserver::promo::get_promo_wallet_tokens());
    }

    let snapshot = match crate::wallet::force_wallet_snapshot().await {
        Ok(s) => s,
        Err(err) => {
            logger::warning(
                LogTag::Wallet,
                &format!("Forced wallet snapshot failed, falling back to latest: {err}"),
            );
            match get_current_wallet_status().await {
                Ok(Some(s)) => s,
                _ => return Json(WalletTokensResponse { tokens: vec![] }),
            }
        }
    };

    Json(WalletTokensResponse {
        tokens: enrich_token_holdings(&snapshot).await,
    })
}

/// Enrich a snapshot's token balances with metadata (symbol/name/logo) and decimals.
///
/// Cache-first by design: known mints (already a metadata row in the token DB) are
/// served straight from cache. Mints the bot has never seen are bootstrapped once
/// via `ensure_token_available` (fetches decimals + market data and stores them);
/// after that first fetch the row exists and subsequent loads hit cache only.
async fn enrich_token_holdings(
    snapshot: &crate::wallet::WalletSnapshot,
) -> Vec<WalletTokenHolding> {
    // token_balances is not populated by get_recent_snapshots — load separately
    let token_balances = if let Some(id) = snapshot.id {
        get_snapshot_token_balances(id).await.unwrap_or_default()
    } else {
        snapshot.token_balances.clone()
    };

    let mints: Vec<String> = token_balances.iter().map(|tb| tb.mint.clone()).collect();

    // Identify never-seen mints (no metadata row at all) and bootstrap them once.
    // A mint already in the DB — even one whose market fetch previously failed and
    // only has a stamped decimals row — is treated as cached and skipped.
    if let Some(db) = crate::tokens::database::get_global_database() {
        let unknown: Vec<String> = mints
            .iter()
            .filter(|m| !matches!(db.get_token(m), Ok(Some(_))))
            .cloned()
            .collect();

        if !unknown.is_empty() {
            logger::info(
                LogTag::Wallet,
                &format!("Bootstrapping metadata for {} held token(s)", unknown.len()),
            );
            let fetches = unknown
                .iter()
                .map(|mint| crate::tokens::ensure_token_available(mint));
            // Failures are non-fatal (no market data yet) — the row is still stamped.
            let _ = futures::future::join_all(fetches).await;
        }
    }

    // Batch-read metadata (symbol/name) and logos from the token DB cache.
    let mut metadata_map: HashMap<String, (Option<String>, Option<String>)> = HashMap::new();
    if let Some(db) = crate::tokens::database::get_global_database() {
        for mint in &mints {
            if let Ok(Some(meta)) = db.get_token(mint) {
                metadata_map.insert(mint.clone(), (meta.symbol.clone(), meta.name.clone()));
            }
        }
    }
    let logo_map = crate::tokens::database::get_token_images_batch_async(mints.clone())
        .await
        .unwrap_or_default();

    // Latest price in SOL per mint, sourced from the assembled token's market data
    // (None for mints without market data — they simply show no value).
    let price_fetches = mints.iter().map(|mint| async move {
        let price = crate::tokens::database::get_full_token_async(mint)
            .await
            .ok()
            .flatten()
            .map(|t| t.price_sol)
            .filter(|p| *p > 0.0);
        (mint.clone(), price)
    });
    let price_map: HashMap<String, Option<f64>> = futures::future::join_all(price_fetches)
        .await
        .into_iter()
        .collect();

    token_balances
        .iter()
        .map(|tb| {
            let (symbol, name) = metadata_map.get(&tb.mint).cloned().unwrap_or((None, None));
            let price_sol = price_map.get(&tb.mint).copied().flatten();
            let value_sol = price_sol.map(|p| p * tb.balance_ui);
            WalletTokenHolding {
                mint: tb.mint.clone(),
                symbol,
                name,
                logo_url: logo_map.get(&tb.mint).cloned(),
                balance: tb.balance,
                ui_amount: tb.balance_ui,
                decimals: tb.decimals,
                is_token_2022: tb.is_token_2022,
                price_sol,
                value_sol,
            }
        })
        .collect()
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
            error: Some(err.to_string()),
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
            error: Some(err.to_string()),
        }),
    }
}

pub(super) async fn get_wallet_cache_metrics() -> Json<WalletCacheMetricsResponse> {
    let data = get_dashboard_cache_metrics().await;
    Json(WalletCacheMetricsResponse { data })
}
