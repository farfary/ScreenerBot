use axum::{ extract::Query, routing::get, Json, Router };
use serde::{ Deserialize, Serialize };
use std::sync::Arc;

use crate::positions;
use crate::webserver::state::AppState;

#[derive(Debug, Deserialize)]
pub struct PositionsQuery {
    pub status: Option<String>, // "open", "closed", "all"
    pub limit: Option<usize>,
    pub mint: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PositionResponse {
    pub id: Option<i64>,
    pub mint: String,
    pub symbol: String,
    pub name: String,
    pub entry_price: f64,
    pub entry_time: i64, // Unix timestamp
    pub exit_price: Option<f64>,
    pub exit_time: Option<i64>,
    pub position_type: String,
    pub entry_size_sol: f64,
    pub total_size_sol: f64,
    pub price_highest: f64,
    pub price_lowest: f64,
    pub entry_transaction_signature: Option<String>,
    pub exit_transaction_signature: Option<String>,
    pub token_amount: Option<u64>,
    pub effective_entry_price: Option<f64>,
    pub effective_exit_price: Option<f64>,
    pub sol_received: Option<f64>,
    pub profit_target_min: Option<f64>,
    pub profit_target_max: Option<f64>,
    pub liquidity_tier: Option<String>,
    pub transaction_entry_verified: bool,
    pub transaction_exit_verified: bool,
    pub entry_fee_lamports: Option<u64>,
    pub exit_fee_lamports: Option<u64>,
    pub current_price: Option<f64>,
    pub current_price_updated: Option<i64>,
    pub phantom_confirmations: u32,
    pub synthetic_exit: bool,
    pub closed_reason: Option<String>,
    // Calculated fields
    pub pnl: Option<f64>,
    pub pnl_percent: Option<f64>,
    pub unrealized_pnl: Option<f64>,
    pub unrealized_pnl_percent: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct PositionsStatsResponse {
    pub total: usize,
    pub open: usize,
    pub closed: usize,
    pub total_invested_sol: f64,
    pub total_pnl: f64,
}

pub fn routes() -> Router<Arc<AppState>> {
    Router::new().route("/", get(get_positions)).route("/stats", get(get_positions_stats))
}

async fn get_positions(Query(params): Query<PositionsQuery>) -> Json<Vec<PositionResponse>> {
    let status = params.status.as_deref().unwrap_or("all");
    let limit = params.limit.unwrap_or(100);

    // Get positions from database based on status
    let positions_result = match status {
        "open" => positions::db::get_open_positions().await,
        "closed" => positions::db::get_closed_positions().await,
        _ => positions::db::load_all_positions().await,
    };

    let positions = match positions_result {
        Ok(pos) => pos,
        Err(e) => {
            eprintln!("Failed to load positions: {}", e);
            return Json(vec![]);
        }
    };

    // Filter by mint if provided
    let mut filtered_positions = if let Some(mint) = &params.mint {
        positions
            .into_iter()
            .filter(|p| p.mint.contains(mint))
            .collect::<Vec<_>>()
    } else {
        positions
    };

    // Apply limit
    filtered_positions.truncate(limit);

    // Convert to response format
    let responses: Vec<PositionResponse> = filtered_positions
        .iter()
        .map(|p| {
            let entry_time_ts = p.entry_time.timestamp();
            let exit_time_ts = p.exit_time.map(|dt| dt.timestamp());
            let current_price_updated_ts = p.current_price_updated.map(|dt| dt.timestamp());

            // Calculate P&L for closed positions
            let (pnl, pnl_percent) = if p.transaction_exit_verified {
                if
                    let (Some(exit_price), Some(sol_received)) = (
                        p.effective_exit_price,
                        p.sol_received,
                    )
                {
                    let invested = p.entry_size_sol;
                    let pnl_value = sol_received - invested;
                    let pnl_pct = if invested > 0.0 { (pnl_value / invested) * 100.0 } else { 0.0 };
                    (Some(pnl_value), Some(pnl_pct))
                } else if let (Some(exit_price), entry_price) = (p.exit_price, p.entry_price) {
                    let pnl_pct = ((exit_price - entry_price) / entry_price) * 100.0;
                    let pnl_value = p.entry_size_sol * (pnl_pct / 100.0);
                    (Some(pnl_value), Some(pnl_pct))
                } else {
                    (None, None)
                }
            } else {
                (None, None)
            };

            // Calculate unrealized P&L for open positions
            let (unrealized_pnl, unrealized_pnl_percent) = if !p.transaction_exit_verified {
                if let Some(current_price) = p.current_price {
                    let entry_price = p.effective_entry_price.unwrap_or(p.entry_price);
                    let pnl_pct = ((current_price - entry_price) / entry_price) * 100.0;
                    let pnl_value = p.entry_size_sol * (pnl_pct / 100.0);
                    (Some(pnl_value), Some(pnl_pct))
                } else {
                    (None, None)
                }
            } else {
                (None, None)
            };

            PositionResponse {
                id: p.id,
                mint: p.mint.clone(),
                symbol: p.symbol.clone(),
                name: p.name.clone(),
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
                pnl,
                pnl_percent,
                unrealized_pnl,
                unrealized_pnl_percent,
            }
        })
        .collect();

    Json(responses)
}

async fn get_positions_stats() -> Json<PositionsStatsResponse> {
    let open_positions = positions::db::get_open_positions().await.unwrap_or_default();
    let closed_positions = positions::db::get_closed_positions().await.unwrap_or_default();

    let total = open_positions.len() + closed_positions.len();
    let open = open_positions.len();
    let closed = closed_positions.len();

    let total_invested_sol: f64 = open_positions
        .iter()
        .map(|p| p.entry_size_sol)
        .sum();

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
