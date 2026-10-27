// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Position detail route — serves detailed data for a single position view.

use axum::{
    extract::Path,
    response::{IntoResponse as _, Response},
};
use chrono::Utc;

use super::list::map_position_to_response_async;
use super::types::*;
use crate::chains::adapter;
use crate::i18n::ids;
use crate::logger::{self, LogTag};
use crate::native_price;
use crate::pools;
use crate::positions;
use crate::tokens;
use crate::tokens::security::SecurityRiskBand;
use crate::webserver::api_error::{ApiError, ApiErrorCode};
use crate::webserver::utils::success_response;

pub async fn get_position_details(Path(key): Path<String>) -> Response {
    match resolve_position_by_key(&key).await {
        Ok(Some(position)) => {
            let mint = &position.mint;

            // Fetch all data concurrently for better performance
            let (detail, (entries, exits), pending_swaps) = tokio::join!(
                map_position_to_detail(&position),
                load_entry_exit_history(&position),
                load_pending_swaps(&position)
            );

            // Fetch token data from database
            let token_data =
                tokens::database::get_full_token_async(crate::chains::active_chain(), mint)
                    .await
                    .ok()
                    .flatten();

            // Build token info from token database
            let token_info = token_data.as_ref().map(|token| {
                let website = token.websites.first().map(|w| w.url.clone());
                let twitter = token
                    .socials
                    .iter()
                    .find(|s| s.link_type.to_lowercase().contains("twitter"))
                    .map(|s| s.url.clone());
                let telegram = token
                    .socials
                    .iter()
                    .find(|s| s.link_type.to_lowercase().contains("telegram"))
                    .map(|s| s.url.clone());

                PositionTokenInfo {
                    decimals: token.decimals,
                    description: token.description.clone(),
                    image_url: token.image_url.clone(),
                    website,
                    twitter,
                    telegram,
                }
            });

            // Build market data from token database
            let market_data = token_data.as_ref().map(|token| PositionMarketData {
                market_cap: token.market_cap,
                fdv: token.fdv,
                liquidity_usd: token.liquidity_usd,
                volume_24h: token.volume_h24,
                price_change_h1: token.price_change_h1,
                price_change_h24: token.price_change_h24,
                holder_count: token.holder_count(),
            });

            // Build security summary from token database
            let security = token_data.as_ref().map(|token| {
                let top_risks: Vec<String> = token
                    .security_risks
                    .iter()
                    .take(3)
                    .map(|r| r.name.clone())
                    .collect();

                PositionSecuritySummary {
                    score_normalized: token.security_score_normalised,
                    risk_level: SecurityRiskBand::of_optional(token.security_score_normalised),
                    has_mint_authority: token.mint_authority.is_some(),
                    has_freeze_authority: token.freeze_authority.is_some(),
                    top_risks,
                }
            });

            // Get pool info from pool service
            let pool_info =
                pools::get_pool_price(crate::chains::active_chain(), mint).map(|price_result| {
                    PositionPoolInfo {
                        pool_address: Some(price_result.pool_address.clone()),
                        pool_program: pools::pool_protocol(
                            crate::chains::active_chain(),
                            mint,
                            &price_result.pool_address,
                        )
                        .map(str::to_owned),
                        liquidity_native: Some(price_result.native_reserves),
                    }
                });

            // Build external links
            let external_links = ExternalLinks::for_mint(mint);

            // Calculate position age in seconds
            let position_age_seconds = Some(
                Utc::now()
                    .signed_duration_since(position.entry_time)
                    .num_seconds(),
            );

            // Get SOL price in USD
            let native_price_usd = {
                let price = native_price::get_native_price();
                if price > 0.0 {
                    Some(price)
                } else {
                    None
                }
            };

            success_response(PositionDetailResponse {
                position: Some(detail),
                entries,
                exits,
                token_info,
                market_data,
                security,
                pool_info,
                external_links,
                position_age_seconds,
                native_price_usd,
                pending_swaps,
                fetched_at: Utc::now().to_rfc3339(),
            })
        }
        Ok(None) => ApiError::new(ApiErrorCode::NotFound, ids::ERRORS_POSITIONS_NOT_FOUND)
            .details(key.clone())
            .into_response(),
        Err(err) => {
            logger::info(
                LogTag::Webserver,
                &format!("Failed to resolve position for key {key}: {err}"),
            );

            ApiError::new(ApiErrorCode::Internal, ids::ERRORS_POSITIONS_DETAIL_FAILED)
                .details(err.to_string())
                .into_response()
        }
    }
}

/// `id:<n>` resolves ANY position (open, closed or archived) — that is how the Closed and
/// Archived tabs open their rows. A MINT key resolves only the OPEN position: every caller
/// that looks a position up by mint (the trade dialog, the token-details Positions tab, the
/// context menu) is asking "is this token held right now", and answering with a closed
/// position made the dashboard turn a Buy into an Add that the trade route then rejected
/// with "no open position for this token".
pub(super) async fn resolve_position_by_key(
    key: &str,
) -> crate::webserver::Result<Option<positions::Position>> {
    if let Some(id_part) = key.strip_prefix("id:") {
        let id: i64 = id_part
            .parse()
            .map_err(|_| crate::webserver::Error::InvalidPositionKey {
                detail: format!("Invalid position id: {id_part}"),
            })?;
        return Ok(positions::get_position_by_id(id).await);
    }

    if let Some(mint_part) = key.strip_prefix("mint:") {
        return Ok(positions::get_position_by_mint(mint_part).await);
    }

    Ok(positions::get_position_by_mint(key).await)
}

async fn map_position_to_detail(position: &positions::Position) -> PositionDetail {
    let trailing_stop_active = runs_policy_exits(position)
        && crate::trader::policy::resolve_exit_policy(position)
            .await
            .trailing
            .enabled;
    PositionDetail {
        summary: map_position_to_response_async(position).await,
        phantom_remove: position.phantom_remove,
        phantom_first_seen: position.phantom_first_seen.map(|dt| dt.timestamp()),
        peak_since_last_buy: peak_since_last_buy(position),
        trailing_stop_active,
    }
}

/// The basis peak of `position` as the peak since its last buy, when a buy restarted it
/// and it differs from the round's peak. A wallet-derived round restarts it at the round's
/// average, which no buy paid, so it has none.
fn peak_since_last_buy(position: &positions::Position) -> Option<f64> {
    (!position.is_wallet_derived()
        && position.basis_peak_price > 0.0
        && position.basis_peak_price != position.price_highest)
        .then_some(position.basis_peak_price)
}

/// Whether the exit monitor evaluates the exit policy of `position`: it is open and not
/// archived, and its management runs policy exits.
fn runs_policy_exits(position: &positions::Position) -> bool {
    positions::state::is_position_open(position) && position.management.allows_policy_exits()
}

/// Swaps this position has submitted but not booked yet.
///
/// Filtered by position id, not just mint: the registries are per-mint, and handing one
/// position the other's in-flight add would misreport both. A position with no id cannot
/// own a pending swap.
async fn load_pending_swaps(position: &positions::Position) -> Vec<PendingSwapResponse> {
    let Some(id) = position.id else {
        return Vec::new();
    };

    let mut pending: Vec<PendingSwapResponse> =
        positions::get_pending_dca_swaps_for_mint(&position.mint)
            .await
            .into_iter()
            .filter(|entry| entry.position_id == id)
            .map(|entry| PendingSwapResponse {
                kind: "dca".to_owned(),
                signature: entry.signature,
                submitted_at: entry.created_at.timestamp(),
                size_sol: (entry.size_sol > 0.0).then_some(entry.size_sol),
                exit_percentage: None,
            })
            .collect();

    pending.extend(
        positions::get_pending_partial_exits_for_mint(&position.mint)
            .await
            .into_iter()
            .filter(|entry| entry.position_id == id)
            .map(|entry| PendingSwapResponse {
                kind: "partial_exit".to_owned(),
                signature: entry.signature,
                submitted_at: entry.created_at.timestamp(),
                size_sol: None,
                exit_percentage: Some(entry.requested_exit_percentage),
            }),
    );

    pending.sort_by_key(|entry| entry.submitted_at);
    pending
}

/// Load entry and exit history for a position
async fn load_entry_exit_history(
    position: &positions::Position,
) -> (Vec<EntryRecordResponse>, Vec<ExitRecordResponse>) {
    let Some(id) = position.id else {
        return (Vec::new(), Vec::new());
    };

    // Load entries
    let entries = match positions::get_entry_history(id).await {
        Ok(records) => records
            .into_iter()
            .map(|r| EntryRecordResponse {
                id: r.id,
                timestamp: r.timestamp.timestamp(),
                amount: r.amount,
                price: r.price,
                native_spent: r.native_spent,
                transaction_signature: r.transaction_signature,
                is_dca: r.is_dca,
                fees_native: r.fees_raw.map(|l| adapter().raw_to_native(l)),
            })
            .collect(),
        Err(err) => {
            logger::debug(
                LogTag::Webserver,
                &format!("Failed to load entry history for position {id}: {err}"),
            );
            Vec::new()
        }
    };

    // Load exits
    let exits = match positions::get_exit_history(id).await {
        Ok(records) => records
            .into_iter()
            .map(|r| ExitRecordResponse {
                id: r.id,
                timestamp: r.timestamp.timestamp(),
                amount: r.amount,
                price: r.price,
                native_received: r.native_received,
                transaction_signature: r.transaction_signature,
                is_partial: r.is_partial,
                percentage: r.percentage,
                fees_native: r.fees_raw.map(|l| adapter().raw_to_native(l)),
            })
            .collect(),
        Err(err) => {
            logger::debug(
                LogTag::Webserver,
                &format!("Failed to load exit history for position {id}: {err}"),
            );
            Vec::new()
        }
    };

    (entries, exits)
}

#[cfg(test)]
mod tests {
    use super::{peak_since_last_buy, runs_policy_exits};
    use crate::positions::{Position, PositionManagement, PositionOrigin};

    /// An open bot position that took an add: its basis peak restarted below the round's
    /// peak.
    fn averaged_position() -> Position {
        serde_json::from_str(
            r#"{
            "id": 7,
            "mint": "So11111111111111111111111111111111111111112",
            "symbol": "TEST",
            "name": "Test",
            "entry_price": 1.0,
            "entry_time": "2026-10-01T00:00:00Z",
            "exit_price": null,
            "exit_time": null,
            "position_type": "buy",
            "entry_size_native": 1.0,
            "total_size_native": 2.0,
            "price_highest": 1.4,
            "price_lowest": 0.8,
            "basis_peak_price": 0.95,
            "entry_transaction_signature": "entry",
            "exit_transaction_signature": null,
            "token_amount": "2000000",
            "effective_entry_price": 1.0,
            "effective_exit_price": null,
            "native_received": null,
            "profit_target_min": null,
            "profit_target_max": null,
            "liquidity_tier": null,
            "transaction_entry_verified": true,
            "transaction_exit_verified": false,
            "entry_fee_raw": null,
            "exit_fee_raw": null,
            "current_price": 0.9,
            "current_price_updated": null,
            "phantom_remove": false,
            "phantom_confirmations": 0,
            "phantom_first_seen": null,
            "synthetic_exit": false,
            "closed_reason": null,
            "pnl": null,
            "pnl_percent": null,
            "unrealized_pnl": null,
            "unrealized_pnl_percent": null,
            "remaining_token_amount": "2000000",
            "total_exited_amount": "0",
            "average_exit_price": null,
            "partial_exit_count": 0,
            "dca_count": 1,
            "average_entry_price": 0.9,
            "last_dca_time": null,
            "origin": { "kind": "auto", "strategy_id": null },
            "management": "auto_trader"
        }"#,
        )
        .expect("position fixture deserializes")
    }

    #[test]
    fn the_peak_since_the_last_buy_is_reported_only_where_a_buy_restarted_it() {
        let averaged = averaged_position();
        assert_eq!(peak_since_last_buy(&averaged), Some(0.95));

        let mut never_averaged = averaged_position();
        never_averaged.basis_peak_price = never_averaged.price_highest;
        assert_eq!(peak_since_last_buy(&never_averaged), None);

        let mut unseeded = averaged_position();
        unseeded.basis_peak_price = 0.0;
        assert_eq!(peak_since_last_buy(&unseeded), None);

        let mut derived = averaged_position();
        derived.origin = PositionOrigin::External;
        assert_eq!(
            peak_since_last_buy(&derived),
            None,
            "a wallet-derived round restarts its peak at the round's average, not at a buy"
        );
    }

    #[test]
    fn policy_exits_run_only_on_open_positions_whose_management_runs_them() {
        for (management, runs) in [
            (PositionManagement::AutoTrader, true),
            (PositionManagement::Hybrid, true),
            (PositionManagement::UserOnly, false),
            (PositionManagement::CopyTask, false),
        ] {
            let mut position = averaged_position();
            position.management = management;
            assert_eq!(runs_policy_exits(&position), runs, "{management:?}");
        }

        let mut closed = averaged_position();
        closed.exit_time = Some(closed.entry_time);
        closed.transaction_exit_verified = true;
        assert!(!runs_policy_exits(&closed), "a closed position");

        let mut archived = averaged_position();
        archived.archived = true;
        assert!(!runs_policy_exits(&archived), "an archived position");
    }
}
