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
                holder_count: token.total_holders,
            });

            // Build security summary from token database
            let security = token_data.as_ref().map(|token| {
                // Rugcheck normalized score: 0-100, LOWER = SAFER, HIGHER = RISKIER
                let risk_level = match token.security_score_normalised {
                    Some(score) if score <= 20 => "low".to_owned(),
                    Some(score) if score <= 50 => "medium".to_owned(),
                    Some(_) => "high".to_owned(),
                    None => "unknown".to_owned(),
                };

                let top_risks: Vec<String> = token
                    .security_risks
                    .iter()
                    .take(3)
                    .map(|r| r.name.clone())
                    .collect();

                PositionSecuritySummary {
                    score_normalized: token.security_score_normalised,
                    risk_level,
                    has_mint_authority: token.mint_authority.is_some(),
                    has_freeze_authority: token.freeze_authority.is_some(),
                    top_risks,
                }
            });

            // Get pool info from pool service
            let pool_info = pools::get_pool_price(mint).map(|price_result| PositionPoolInfo {
                pool_address: Some(price_result.pool_address.clone()),
                pool_program: crate::chains::solana::pools::service::get_pool_program(
                    mint,
                    &price_result.pool_address,
                )
                .map(str::to_owned),
                liquidity_native: Some(price_result.native_reserves),
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
    PositionDetail {
        summary: map_position_to_response_async(position).await,
        phantom_remove: position.phantom_remove,
        phantom_first_seen: position.phantom_first_seen.map(|dt| dt.timestamp()),
    }
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
