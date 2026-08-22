//! Positions list route — serves paginated position listings with sort and filter.

use axum::{extract::Query, Json};

use super::types::*;
use crate::positions;
use crate::tokens;

pub async fn get_positions(Query(params): Query<PositionsQuery>) -> Json<Vec<PositionResponse>> {
    // Return promotional fixtures only for owner-initiated media capture.
    if crate::webserver::promo::are_promo_fixtures_enabled() {
        let status = params.status.as_deref();
        return Json(crate::webserver::promo::get_promo_positions(status));
    }

    let status = params.status.as_deref().unwrap_or("all");
    // No implicit cap: these lists are the wallet's own history, and a default that
    // silently dropped everything past the hundredth row made the tabs look like an
    // incomplete record of the wallet. A caller that wants a page asks for one.
    let limit = params.limit.unwrap_or(0);
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
            positions_guard
                .iter()
                .filter(|p| !p.archived)
                .cloned()
                .collect()
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

    // Batch fetch all logo URLs and decimals in single queries (performance optimization)
    let mints: Vec<String> = filtered_positions.iter().map(|p| p.mint.clone()).collect();
    let logo_map = tokens::database::get_token_images_batch_async(mints.clone())
        .await
        .unwrap_or_default();
    let decimals_map = tokens::database::get_token_decimals_batch_async(mints)
        .await
        .unwrap_or_default();

    // Map positions to responses using pre-fetched logos and decimals
    filtered_positions
        .iter()
        .map(|p| {
            map_position_to_response_with_logo(
                p,
                logo_map.get(&p.mint).cloned(),
                decimals_map.get(&p.mint).copied(),
            )
        })
        .collect()
}

/// Map position to response with a pre-fetched logo URL (used for batch operations)
fn map_position_to_response_with_logo(
    p: &positions::Position,
    logo_url: Option<String>,
    token_decimals: Option<u8>,
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
        token_decimals,
        archived: p.archived,
        archived_at: p.archived_at.map(|dt| dt.timestamp()),
        origin: p.origin.clone(),
        management: p.management,
        round_key: p.round_key.clone(),
        basis_complete: p.basis_complete,
        history_complete: p.history_complete,
        holding_state: p.holding_state.clone(),
    }
}

/// Map position to response with async logo fetch (used for single position lookups)
pub async fn map_position_to_response_async(p: &positions::Position) -> PositionResponse {
    // Logo comes from the assembled token (best-effort). Decimals come from the
    // stable on-chain `tokens.decimals` column, NOT the assembled token: full-token
    // assembly returns None once a token loses market data (e.g. a delisted/rugged
    // token), which would drop decimals and make the UI render raw amounts (3.08B
    // instead of 3,075). The decimals column survives that.
    let logo_url = match tokens::database::get_full_token_async(&p.mint).await {
        Ok(Some(token)) => token.image_url.clone(),
        _ => None,
    };
    let token_decimals = tokens::database::get_token_decimals_batch_async(vec![p.mint.clone()])
        .await
        .ok()
        .and_then(|m| m.get(&p.mint).copied());

    map_position_to_response_with_logo(p, logo_url, token_decimals)
}

pub async fn get_positions_stats() -> Json<PositionsStatsResponse> {
    // Return promotional fixtures only for owner-initiated media capture.
    if crate::webserver::promo::are_promo_fixtures_enabled() {
        return Json(crate::webserver::promo::get_promo_positions_stats());
    }

    let open_positions = positions::get_open_positions().await;
    let closed_positions = positions::get_closed_positions().await;

    let total = open_positions.len() + closed_positions.len();
    let open = open_positions.len();
    let closed = closed_positions.len();

    // Capital currently at work is the CUMULATIVE cost basis, not the first entry:
    // `entry_size_sol` never grows on a DCA, so a position averaged into three times
    // reported only its first buy and the card understated the portfolio.
    //
    // A round with no established cost basis (an imported airdrop, a USD-quoted fill)
    // contributes nothing rather than a zero that would silently read as "free".
    let total_invested_sol: f64 = open_positions
        .iter()
        .filter(|p| p.has_trustworthy_pnl())
        .map(|p| p.total_size_sol)
        .sum();

    // Realized P&L is the stored, fee-aware `pnl` — the one the position itself booked
    // at close. Recomputing it as `sol_received - entry_size_sol` double-counted DCA
    // adds as pure profit and ignored fees entirely. `pnl` is None exactly when there is
    // no honest number, so those rounds drop out instead of being guessed at.
    let total_pnl: f64 = closed_positions
        .iter()
        .filter(|p| p.has_trustworthy_pnl())
        .filter_map(|p| p.pnl)
        .sum();

    Json(PositionsStatsResponse {
        total,
        open,
        closed,
        total_invested_sol,
        total_pnl,
    })
}
