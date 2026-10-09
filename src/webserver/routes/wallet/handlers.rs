// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Axum handlers for wallet routes - QR code generation, current balance and enriched token holdings, and dashboard data.

use crate::i18n::ids;
use crate::webserver::api_error::{ApiError, ApiErrorCode};
use axum::response::IntoResponse as _;
use axum::{
    extract::Path,
    response::{Json, Response},
    Json as AxumJson,
};
use std::collections::HashMap;

use crate::chains::ChainId;

use super::types::*;
use crate::logger::{self, LogTag};
use crate::wallet::{
    clear_dashboard_api_cache, get_current_wallet_status, get_dashboard_cache_metrics,
    get_flow_cache_stats, get_snapshot_token_balances, get_wallet_dashboard_data,
    refresh_dashboard_cache,
};
use crate::webserver::utils::success_response;

/// Generate a QR code for the current main wallet address.
pub(super) async fn get_wallet_qr(Path(address): Path<String>) -> Response {
    let current_address = if crate::webserver::promo::are_promo_fixtures_enabled() {
        crate::webserver::promo::get_promo_wallet_address().to_owned()
    } else {
        match crate::wallets::get_main_address().await {
            Ok(address) => address,
            Err(err) => {
                return ApiError::new(ApiErrorCode::NotFound, ids::ERRORS_WALLET_UNAVAILABLE)
                    .details(err.to_string())
                    .into_response();
            }
        }
    };

    if address != current_address {
        return ApiError::new(ApiErrorCode::StaleRequest, ids::ERRORS_WALLET_CHANGED)
            .into_response();
    }

    match crate::webserver::totp::generate_qr_data_url_for_value(&address) {
        Ok(qr_data_url) => success_response(WalletQrResponse {
            address,
            qr_data_url,
        }),
        Err(err) => ApiError::new(ApiErrorCode::Internal, ids::ERRORS_WALLET_QR_FAILED)
            .details(err.to_string())
            .into_response(),
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
pub(super) async fn get_wallet_current() -> Result<Json<Option<WalletCurrentResponse>>, ApiError> {
    // Return promotional fixtures only for owner-initiated media capture.
    if crate::webserver::promo::are_promo_fixtures_enabled() {
        return Ok(Json(Some(
            crate::webserver::promo::get_promo_wallet_current(),
        )));
    }

    if let Some(snapshot) = crate::wallet::live_wallet_snapshot() {
        // The live snapshot already carries its token balances — no second query.
        return Ok(Json(Some(WalletCurrentResponse {
            sol_balance: snapshot.native_balance,
            native_balance_raw: snapshot.native_balance_raw,
            total_tokens_count: snapshot.total_tokens_count,
            token_balances: snapshot
                .token_balances
                .iter()
                .map(token_balance_info)
                .collect::<crate::wallets::Result<Vec<_>>>()
                .map_err(|err| {
                    ApiError::new(ApiErrorCode::Internal, ids::ERRORS_WALLET_UNAVAILABLE)
                        .details(err.to_string())
                })?,
            snapshot_time: snapshot.snapshot_time.to_rfc3339(),
        })));
    }

    match get_current_wallet_status().await {
        Ok(Some(snapshot)) => {
            // token_balances is not populated by get_recent_snapshots — load separately
            let raw_balances = if let Some(id) = snapshot.id {
                get_snapshot_token_balances(id).await.map_err(|err| {
                    ApiError::new(ApiErrorCode::Internal, ids::ERRORS_WALLET_UNAVAILABLE)
                        .details(err.to_string())
                })?
            } else {
                vec![]
            };

            let token_balances = raw_balances
                .iter()
                .map(token_balance_info)
                .collect::<crate::wallets::Result<Vec<_>>>()
                .map_err(|err| {
                    ApiError::new(ApiErrorCode::Internal, ids::ERRORS_WALLET_UNAVAILABLE)
                        .details(err.to_string())
                })?;

            Ok(Json(Some(WalletCurrentResponse {
                sol_balance: snapshot.native_balance,
                native_balance_raw: snapshot.native_balance_raw,
                total_tokens_count: snapshot.total_tokens_count,
                token_balances,
                snapshot_time: snapshot.snapshot_time.to_rfc3339(),
            })))
        }
        _ => Ok(Json(None)),
    }
}

/// Get wallet balance (alias for get_wallet_current)
pub(super) async fn get_wallet_balance() -> Result<Json<Option<WalletCurrentResponse>>, ApiError> {
    get_wallet_current().await
}

/// Get wallet token holdings with enriched metadata.
///
/// Reads the live snapshot for the same reason `get_wallet_current` does: the holdings
/// list and the balance beside it must come from one source. Falls back to the database
/// only before the monitor has published anything.
pub(super) async fn get_wallet_tokens() -> Result<Json<WalletTokensResponse>, ApiError> {
    // Return promotional fixtures only for owner-initiated media capture.
    if crate::webserver::promo::are_promo_fixtures_enabled() {
        return Ok(Json(crate::webserver::promo::get_promo_wallet_tokens()));
    }

    let snapshot = match crate::wallet::live_wallet_snapshot() {
        Some(live) => (*live).clone(),
        None => match get_current_wallet_status().await {
            Ok(Some(s)) => s,
            Ok(None) => return Ok(Json(WalletTokensResponse { tokens: vec![] })),
            Err(err) => {
                logger::warning(
                    LogTag::Webserver,
                    &format!("Failed to get wallet status for tokens: {err}"),
                );
                return Ok(Json(WalletTokensResponse { tokens: vec![] }));
            }
        },
    };

    Ok(Json(WalletTokensResponse {
        tokens: enrich_token_holdings(&snapshot).await.map_err(|err| {
            ApiError::new(ApiErrorCode::Internal, ids::ERRORS_WALLET_UNAVAILABLE)
                .details(err.to_string())
        })?,
    }))
}

/// Force a fresh on-chain wallet snapshot, then return the enriched holdings.
///
/// Drives the dashboard "refresh" button: the SOL balance and token balances are
/// re-fetched from RPC (always), while token metadata is cache-first — only
/// never-before-seen mints trigger a metadata fetch (see [`enrich_token_holdings`]).
pub(super) async fn refresh_wallet_tokens() -> Result<Json<WalletTokensResponse>, ApiError> {
    if crate::webserver::promo::are_promo_fixtures_enabled() {
        return Ok(Json(crate::webserver::promo::get_promo_wallet_tokens()));
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
                _ => return Ok(Json(WalletTokensResponse { tokens: vec![] })),
            }
        }
    };

    Ok(Json(WalletTokensResponse {
        tokens: enrich_token_holdings(&snapshot).await.map_err(|err| {
            ApiError::new(ApiErrorCode::Internal, ids::ERRORS_WALLET_UNAVAILABLE)
                .details(err.to_string())
        })?,
    }))
}

fn token_balance_info(
    tb: &crate::wallet::SnapshotTokenBalance,
) -> crate::wallets::Result<TokenBalanceInfo> {
    Ok(TokenBalanceInfo {
        mint: tb.mint.clone(),
        balance: crate::wallets::balance_for_numeric_wire(&tb.mint, tb.balance)?,
        balance_ui: tb.balance_ui,
        decimals: tb.decimals,
        is_token_2022: tb.is_token_2022,
    })
}

/// Enrich a snapshot's token balances with metadata (symbol/name/logo), decimals and the
/// live-worth price of each holding.
///
/// Cache-first by design: known mints (already a metadata row in the token DB) are
/// served straight from cache. Mints the bot has never seen are bootstrapped once
/// via `ensure_token_available` (fetches decimals + market data and stores them);
/// after that first fetch the row exists and subsequent loads hit cache only.
async fn enrich_token_holdings(
    snapshot: &crate::wallet::WalletSnapshot,
) -> crate::wallets::Result<Vec<WalletTokenHolding>> {
    // token_balances is not populated by get_recent_snapshots — load separately
    let token_balances = if let Some(id) = snapshot.id {
        get_snapshot_token_balances(id).await?
    } else {
        snapshot.token_balances.clone()
    };

    let mints: Vec<String> = token_balances.iter().map(|tb| tb.mint.clone()).collect();

    // Each held mint is read from its own chain's token database; a mint no enabled
    // chain accepts has no stored metadata, logo or price.
    let mut mints_by_chain: HashMap<ChainId, Vec<String>> = HashMap::new();
    for mint in &mints {
        if let Ok(chain) = crate::chains::chain_for_address(mint) {
            mints_by_chain.entry(chain).or_default().push(mint.clone());
        }
    }

    // Identify never-seen mints (no metadata row at all) and bootstrap them once.
    // A mint already in the DB — even one whose market fetch previously failed and
    // only has a stamped decimals row — is treated as cached and skipped.
    for (&chain, chain_mints) in &mints_by_chain {
        let Some(db) = crate::tokens::database::database(chain) else {
            continue;
        };
        let unknown: Vec<String> = chain_mints
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
                .map(|mint| crate::tokens::ensure_token_available(chain, mint));
            // Failures are non-fatal (no market data yet) — the row is still stamped.
            let _ = futures::future::join_all(fetches).await;
        }
    }

    // Batch-read metadata (symbol/name) and logos from the token DB cache.
    let mut metadata_map: HashMap<String, (Option<String>, Option<String>)> = HashMap::new();
    let mut logo_map: HashMap<String, String> = HashMap::new();
    for (&chain, chain_mints) in &mints_by_chain {
        if let Some(db) = crate::tokens::database::database(chain) {
            for mint in chain_mints {
                if let Ok(Some(meta)) = db.get_token(mint) {
                    metadata_map.insert(mint.clone(), (meta.symbol.clone(), meta.name.clone()));
                }
            }
        }
        logo_map.extend(
            crate::tokens::database::get_token_images_batch_async(chain, chain_mints.clone())
                .await
                .unwrap_or_default(),
        );
    }

    token_balances
        .iter()
        .map(|tb| {
            let (symbol, name) = metadata_map.get(&tb.mint).cloned().unwrap_or((None, None));
            // Priced by the worth owner, so the rows sum to the header and Home figure.
            let price_sol = crate::wallet::held_token_price_native(&tb.mint);
            let value_sol = price_sol.map(|p| p * tb.balance_ui);
            Ok(WalletTokenHolding {
                mint: tb.mint.clone(),
                symbol,
                name,
                logo_url: logo_map.get(&tb.mint).cloned(),
                balance: crate::wallets::balance_for_numeric_wire(&tb.mint, tb.balance)?,
                ui_amount: tb.balance_ui,
                decimals: tb.decimals,
                is_token_2022: tb.is_token_2022,
                price_sol,
                value_sol,
            })
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

#[cfg(test)]
mod raw_balance_wire_tests {
    use super::*;
    use crate::chains::RawAmount;

    #[test]
    fn current_wallet_balance_remains_numeric_through_u64_max() {
        for raw in [0, i64::MAX as u64, i64::MAX as u64 + 1, u64::MAX] {
            let token = crate::wallet::SnapshotTokenBalance {
                id: None,
                snapshot_id: None,
                mint: "mint".to_owned(),
                balance: RawAmount::from(raw),
                balance_ui: 1.0,
                decimals: 0,
                is_token_2022: false,
            };
            let response = WalletCurrentResponse {
                sol_balance: 1.0,
                native_balance_raw: 1_000_000_000,
                total_tokens_count: 1,
                token_balances: vec![token_balance_info(&token).unwrap()],
                snapshot_time: "2026-10-05T00:00:00Z".to_owned(),
            };
            let json = serde_json::to_value(response).unwrap();
            assert_eq!(json["token_balances"][0]["balance"], serde_json::json!(raw));
            assert!(json["token_balances"][0]["balance"].is_number());
        }
    }

    #[test]
    fn wide_wallet_balance_fails_with_typed_range_error() {
        let token = crate::wallet::SnapshotTokenBalance {
            id: None,
            snapshot_id: None,
            mint: "wide".to_owned(),
            balance: RawAmount::new(u128::from(u64::MAX) + 1),
            balance_ui: 1.0,
            decimals: 0,
            is_token_2022: false,
        };
        assert!(
            matches!(token_balance_info(&token), Err(crate::wallets::Error::BalanceOutOfRange { mint }) if mint == "wide")
        );
        assert!(matches!(
            crate::wallets::balance_for_numeric_wire("wide", RawAmount::MAX),
            Err(crate::wallets::Error::BalanceOutOfRange { .. })
        ));
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
