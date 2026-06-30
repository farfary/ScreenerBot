//! Force-close route — allows manual closure of ghost positions stuck in open state.

use axum::{extract::Path, http::StatusCode, response::Response, Json};
use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::logger::{self, LogTag};
use crate::pools;
use crate::positions;
use crate::webserver::utils::{error_response, success_response};

#[derive(Debug, Deserialize)]
pub struct ForceCloseRequest {
    pub reason: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ForceCloseResponse {
    pub success: bool,
    pub position_id: i64,
    pub symbol: String,
    pub reason: String,
    pub message: String,
}

pub(super) async fn force_close_position(
    Path(position_id): Path<i64>,
    body: Option<Json<ForceCloseRequest>>,
) -> Response {
    let reason_text = body
        .and_then(|b| b.reason.clone())
        .unwrap_or_else(|| "manual force close".to_owned());

    let closed_reason = format!("force_closed: {reason_text}");

    // 1. Look up the position in memory by ID
    let position = match positions::get_position_by_id(position_id).await {
        Some(p) => p,
        None => {
            // Fall back to database lookup in case it's not in memory
            match positions::get_db_position_by_id(position_id).await {
                Ok(Some(p)) => p,
                _ => {
                    return error_response(
                        StatusCode::NOT_FOUND,
                        "POSITION_NOT_FOUND",
                        "Position not found",
                        Some(&format!("No position found with ID {position_id}")),
                    );
                }
            }
        }
    };

    // 2. Validate it's actually open
    if position.exit_time.is_some() && position.transaction_exit_verified {
        return error_response(
            StatusCode::BAD_REQUEST,
            "POSITION_ALREADY_CLOSED",
            "Position is already closed",
            Some(&format!(
                "Position {position_id} ({}) is already closed",
                position.symbol
            )),
        );
    }

    let symbol = position.symbol.clone();
    let mint = position.mint.clone();
    let now = Utc::now();

    // 3. Try to get current price for the exit_price field (best-effort)
    let exit_price = pools::get_pool_price(&mint)
        .map(|pr| pr.price_sol)
        .filter(|p| *p > 0.0 && p.is_finite())
        .or(position.current_price)
        .unwrap_or(0.0);

    // 4. Calculate P&L for the force-closed position
    let (pnl, pnl_percent) = if exit_price > 0.0 {
        // Use exit_price=0 for effective (no SOL received), so P&L is a total loss
        let pnl_sol = -position.total_size_sol;
        let pnl_pct = if position.total_size_sol > 0.0 {
            (pnl_sol / position.total_size_sol) * 100.0
        } else {
            0.0
        };
        (pnl_sol, pnl_pct)
    } else {
        let pnl_sol = -position.total_size_sol;
        let pnl_pct = -100.0;
        (pnl_sol, pnl_pct)
    };

    // 5. Update in-memory state
    let updated_in_memory = positions::state::update_position_state_by_id(position_id, |pos| {
        pos.exit_time = Some(now);
        pos.exit_price = Some(exit_price);
        pos.effective_exit_price = Some(0.0);
        pos.transaction_exit_verified = true;
        pos.closed_reason = Some(closed_reason.clone());
        pos.sol_received = Some(0.0);
        pos.synthetic_exit = true;
        pos.pnl = Some(pnl);
        pos.pnl_percent = Some(pnl_percent);
        pos.unrealized_pnl = None;
        pos.unrealized_pnl_percent = None;
    })
    .await;

    // 6. Build updated position for database persistence
    let mut db_position = position.clone();
    db_position.exit_time = Some(now);
    db_position.exit_price = Some(exit_price);
    db_position.effective_exit_price = Some(0.0);
    db_position.transaction_exit_verified = true;
    db_position.closed_reason = Some(closed_reason.clone());
    db_position.sol_received = Some(0.0);
    db_position.synthetic_exit = true;
    db_position.pnl = Some(pnl);
    db_position.pnl_percent = Some(pnl_percent);
    db_position.unrealized_pnl = None;
    db_position.unrealized_pnl_percent = None;

    // 7. Persist to database
    if let Err(e) = positions::update_position(&db_position).await {
        logger::error(
            LogTag::Positions,
            &format!("Force-close: failed to persist position {position_id} to database: {e}"),
        );
        // Continue anyway — in-memory state is already updated and semaphore should be released
    }

    // 8. Release the semaphore permit
    positions::state::release_global_position_permit();

    logger::info(
        LogTag::Positions,
        &format!(
            "Force-closed position {position_id} ({symbol}) — reason: {closed_reason}, \
             in_memory_updated: {updated_in_memory}"
        ),
    );

    success_response(ForceCloseResponse {
        success: true,
        position_id,
        symbol,
        reason: closed_reason,
        message: "Position force-closed successfully".to_owned(),
    })
}
