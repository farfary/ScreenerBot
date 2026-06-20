//! Positions list route — serves paginated position listings with sort and filter.

use axum::{extract::Query, Json};

use super::types::*;
use crate::logger::{self, LogTag};
use crate::pools;
use crate::positions;
use crate::sol_price;
use crate::tokens;

pub async fn get_positions(Query(params): Query<PositionsQuery>) -> Json<Vec<PositionResponse>> {
    // Return demo data if demo mode is enabled
    if crate::webserver::demo::is_demo_mode() {
        let status = params.status.as_deref();
        return Json(crate::webserver::demo::get_demo_positions(status));
    }

    let status = params.status.as_deref().unwrap_or("all");
    let limit = params.limit.unwrap_or(100);
    let mint_filter = params.mint.as_deref();

    let responses = load_positions_with_filters(status, limit, mint_filter).await;
    Json(responses)
}

pub async fn load_positions_with_filters(
    status: &str,
    limit: usize,
    mint_filter: Option<&str>,
) -> Vec<PositionResponse> {
    let positions: Vec<positions::Position> = match status {
        "open" => positions::get_open_positions().await,
        "closed" => positions::get_closed_positions().await,
        "archived" => positions::get_archived_positions().await,
        _ => {
            // "all" excludes archived so they only appear in the Archived tab.
            let positions_guard = positions::POSITIONS.read().await;
            positions_guard.iter().filter(|p| !p.archived).cloned().collect()
        }
    };

    let mut filtered_positions: Vec<_> = if let Some(mint) = mint_filter {
        positions
            .into_iter()
            .filter(|p| p.mint.contains(mint))
            .collect()
    } else {
        positions
    };

    if limit > 0 {
        filtered_positions.truncate(limit);
    }

    // Batch fetch all logo URLs in a single query (performance optimization)
    let mints: Vec<String> = filtered_positions.iter().map(|p| p.mint.clone()).collect();
    let logo_map = tokens::database::get_token_images_batch_async(mints)
        .await
        .unwrap_or_default();

    // Map positions to responses using pre-fetched logos
    filtered_positions
        .iter()
        .map(|p| map_position_to_response_with_logo(p, logo_map.get(&p.mint).cloned()))
        .collect()
}

/// Map position to response with a pre-fetched logo URL (used for batch operations)
fn map_position_to_response_with_logo(
    p: &positions::Position,
    logo_url: Option<String>,
) -> PositionResponse {
    let entry_time_ts = p.entry_time.timestamp();
    let exit_time_ts = p.exit_time.map(|dt| dt.timestamp());
    let current_price_updated_ts = p.current_price_updated.map(|dt| dt.timestamp());

    PositionResponse {
        id: p.id,
        mint: p.mint.clone(),
        symbol: p.symbol.clone(),
        name: p.name.clone(),
        logo_url,
        entry_price: p.entry_price,
        entry_time: entry_time_ts,
        exit_price: p.exit_price,
        exit_time: exit_time_ts,
        position_type: p.position_type.clone(),
        entry_size_sol: p.entry_size_sol,
        total_size_sol: p.total_size_sol,
        price_highest: p.price_highest,
        price_lowest: p.price_lowest,
        entry_transaction_signature: p.entry_transaction_signature.clone(),
        exit_transaction_signature: p.exit_transaction_signature.clone(),
        token_amount: p.token_amount,
        effective_entry_price: p.effective_entry_price,
        effective_exit_price: p.effective_exit_price,
        sol_received: p.sol_received,
        profit_target_min: p.profit_target_min,
        profit_target_max: p.profit_target_max,
        liquidity_tier: p.liquidity_tier.clone(),
        transaction_entry_verified: p.transaction_entry_verified,
        transaction_exit_verified: p.transaction_exit_verified,
        entry_fee_lamports: p.entry_fee_lamports,
        exit_fee_lamports: p.exit_fee_lamports,
        current_price: p.current_price,
        current_price_updated: current_price_updated_ts,
        phantom_confirmations: p.phantom_confirmations,
        synthetic_exit: p.synthetic_exit,
        closed_reason: p.closed_reason.clone(),
        pnl: p.pnl,
        pnl_percent: p.pnl_percent,
        unrealized_pnl: p.unrealized_pnl,
        unrealized_pnl_percent: p.unrealized_pnl_percent,
        dca_count: p.dca_count,
        average_entry_price: p.average_entry_price,
        partial_exit_count: p.partial_exit_count,
        average_exit_price: p.average_exit_price,
        remaining_token_amount: p.remaining_token_amount,
        total_exited_amount: p.total_exited_amount,
        archived: p.archived,
        archived_at: p.archived_at.map(|dt| dt.timestamp()),
        manual_management: p.manual_management,
    }
}

/// Map position to response with async logo fetch (used for single position lookups)
pub async fn map_position_to_response_async(p: &positions::Position) -> PositionResponse {
    // Fetch logo from tokens database
    let logo_url = match tokens::database::get_full_token_async(&p.mint).await {
        Ok(Some(token)) => token.image_url.clone(),
        Ok(None) => None,
        Err(_) => None,
    };

    map_position_to_response_with_logo(p, logo_url)
}

pub async fn get_positions_stats() -> Json<PositionsStatsResponse> {
    // Return demo data if demo mode is enabled
    if crate::webserver::demo::is_demo_mode() {
        return Json(crate::webserver::demo::get_demo_positions_stats());
    }

    let open_positions = positions::get_open_positions().await;
    let closed_positions = positions::get_closed_positions().await;

    let total = open_positions.len() + closed_positions.len();
    let open = open_positions.len();
    let closed = closed_positions.len();

    let total_invested_sol: f64 = open_positions.iter().map(|p| p.entry_size_sol).sum();

    let total_pnl: f64 = closed_positions
        .iter()
        .filter_map(|p| {
            if let (Some(sol_received), entry_size) = (p.sol_received, p.entry_size_sol) {
                Some(sol_received - entry_size)
            } else {
                None
            }
        })
        .sum();

    Json(PositionsStatsResponse {
        total,
        open,
        closed,
        total_invested_sol,
        total_pnl,
    })
}
