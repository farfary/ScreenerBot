// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Paper book routes under `/positions/paper`: its rounds, stats, account and run reset,
//! a round's fills, and the manual close of a paper round. They read and write the paper
//! store only; no live position route accepts a paper round.

use std::sync::Arc;

use axum::{
    extract::{Path, Query},
    middleware::from_fn_with_state,
    response::{IntoResponse as _, Response},
    routing::{get, post},
    Router,
};

use crate::database::catalog;
use crate::i18n::ids;
use crate::logger::{self, LogTag};
use crate::positions::db::PaperLegRecord;
use crate::positions::paper::{PaperAccount, PaperBook, PaperRoundHistory};
use crate::positions::Position;
use crate::webserver::api_error::{ApiError, ApiErrorCode};
use crate::webserver::middleware::store_gate;
use crate::webserver::routes::trader::{trade_response, trader_failure};
use crate::webserver::state::AppState;
use crate::webserver::utils::success_response;

use super::list::{map_position_to_response_async, position_responses, positions_stats};
use super::types::{
    PaperAccountResponse, PaperFillResponse, PaperPositionDetailResponse, PositionsQuery,
};

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/positions/paper", get(list_paper_positions))
        .route("/positions/paper/stats", get(paper_positions_stats))
        .route(
            "/positions/paper/account",
            get(paper_account).post(reset_paper_run),
        )
        .route(
            "/positions/paper/{position_id}/details",
            get(paper_position_details),
        )
        .route(
            "/positions/paper/{position_id}/close",
            post(close_paper_position),
        )
        .route_layer(from_fn_with_state(&catalog::PAPER_POSITIONS, store_gate))
}

fn paper_book() -> Result<&'static PaperBook, Response> {
    crate::positions::paper::book()
        .ok_or_else(|| trader_failure(&crate::trader::Error::PaperBookUnavailable))
}

fn paper_load_failed(error: &crate::positions::Error) -> Response {
    logger::warning(
        LogTag::Webserver,
        &format!("Failed to read the paper book: {error}"),
    );
    ApiError::new(
        ApiErrorCode::DatabaseError,
        ids::ERRORS_POSITIONS_PAPER_LOAD_FAILED,
    )
    .details(error.to_string())
    .into_response()
}

/// The paper rounds of `status` (`open`, `closed`, or both), newest activity first.
async fn list_paper_positions(Query(params): Query<PositionsQuery>) -> Response {
    let book = match paper_book() {
        Ok(book) => book,
        Err(response) => return response,
    };
    let status = params.status.as_deref().unwrap_or("all");
    let mut rounds = Vec::new();
    if status != "closed" {
        rounds.extend(book.open_positions().await);
    }
    if status != "open" {
        match book.closed_positions().await {
            Ok(closed) => rounds.extend(closed),
            Err(error) => return paper_load_failed(&error),
        }
    }
    success_response(
        position_responses(rounds, params.limit.unwrap_or(0), params.mint.as_deref()).await,
    )
}

async fn paper_positions_stats() -> Response {
    let book = match paper_book() {
        Ok(book) => book,
        Err(response) => return response,
    };
    let open = book.open_positions().await;
    match book.closed_positions().await {
        Ok(closed) => success_response(positions_stats(&open, &closed)),
        Err(error) => paper_load_failed(&error),
    }
}

async fn paper_account() -> Response {
    match crate::trader::paper::paper_account().await {
        Ok(account) => account_response(account).await,
        Err(error) => trader_failure(&error),
    }
}

/// Settles every open paper round at its mark and starts the next paper run.
async fn reset_paper_run() -> Response {
    match crate::trader::paper::reset_paper_run().await {
        Ok(account) => account_response(account).await,
        Err(error) => trader_failure(&error),
    }
}

async fn account_response(account: PaperAccount) -> Response {
    let open = match crate::positions::paper::book() {
        Some(book) => book.open_positions().await,
        None => Vec::new(),
    };
    success_response(paper_account_response(account, &open))
}

/// The account of the current run with the cost and unrealized P&L of `open`.
fn paper_account_response(account: PaperAccount, open: &[Position]) -> PaperAccountResponse {
    let open_unrealized_pnl_native = open
        .iter()
        .map(|position| position.unrealized_pnl)
        .sum::<Option<f64>>();
    PaperAccountResponse {
        epoch: account.epoch,
        started_at: account.started_at.timestamp(),
        starting_balance_native: account.starting_balance_native,
        cash_native: account.cash_native,
        open_rounds: open.len(),
        open_cost_native: open.iter().map(|position| position.total_size_native).sum(),
        open_unrealized_pnl_native,
    }
}

async fn paper_position_details(Path(position_id): Path<i64>) -> Response {
    let book = match paper_book() {
        Ok(book) => book,
        Err(response) => return response,
    };
    let position = match book.stored(position_id).await {
        Ok(Some(position)) => position,
        Ok(None) => {
            return ApiError::new(ApiErrorCode::NotFound, ids::ERRORS_POSITIONS_NOT_FOUND)
                .details(position_id.to_string())
                .into_response()
        }
        Err(error) => return paper_load_failed(&error),
    };
    let wallet = match crate::utils::get_wallet_address() {
        Ok(wallet) => wallet,
        Err(_) => return trader_failure(&crate::trader::Error::PaperBookUnavailable),
    };
    match book.round_history(&wallet, position_id).await {
        Ok(history) => success_response(PaperPositionDetailResponse {
            position: map_position_to_response_async(&position).await,
            fills: paper_fills(history),
        }),
        Err(error) => paper_load_failed(&error),
    }
}

/// Every fill of a paper round, oldest first, each joined to the leg its record carries.
fn paper_fills(history: PaperRoundHistory) -> Vec<PaperFillResponse> {
    let adapter = crate::chains::adapter();
    let leg_of = |reference: &str| history.legs.iter().find(|leg| leg.leg_ref == reference);
    let provenance = |leg: Option<&PaperLegRecord>| {
        (
            leg.and_then(|leg| leg.strategy_id.clone()),
            leg.and_then(|leg| serde_json::from_str(&leg.quote_json).ok()),
        )
    };
    let mut fills: Vec<PaperFillResponse> = history
        .entries
        .iter()
        .map(|entry| {
            let leg = leg_of(&entry.transaction_signature);
            let (strategy_id, quote) = provenance(leg);
            PaperFillResponse {
                kind: if entry.is_dca { "dca" } else { "entry" }.to_owned(),
                timestamp: entry.timestamp.timestamp(),
                amount: entry.amount,
                price: entry.price,
                native_amount: entry.native_spent,
                percentage: None,
                fees_native: entry.fees_raw.map(|raw| adapter.raw_to_native(raw)),
                strategy_id,
                quote,
            }
        })
        .collect();
    fills.extend(history.exits.iter().map(|exit| {
        let leg = leg_of(&exit.transaction_signature);
        let (strategy_id, quote) = provenance(leg);
        PaperFillResponse {
            kind: if exit.is_partial { "partial" } else { "exit" }.to_owned(),
            timestamp: exit.timestamp.timestamp(),
            amount: exit.amount,
            price: exit.price,
            native_amount: exit.native_received,
            percentage: Some(exit.percentage),
            fees_native: exit.fees_raw.map(|raw| adapter.raw_to_native(raw)),
            strategy_id,
            quote,
        }
    }));
    fills.sort_by_key(|fill| fill.timestamp);
    fills
}

/// Closes the open paper round `position_id` in full at a paper fill. A round of the
/// live book with the same id is never read: the id is resolved in the paper book only.
async fn close_paper_position(Path(position_id): Path<i64>) -> Response {
    let book = match paper_book() {
        Ok(book) => book,
        Err(response) => return response,
    };
    let Some(position) = book.position(position_id).await else {
        return match book.stored(position_id).await {
            Ok(Some(_)) => ApiError::new(
                ApiErrorCode::InvalidInput,
                ids::ERRORS_POSITIONS_ALREADY_CLOSED,
            )
            .details(position_id.to_string())
            .into_response(),
            Ok(None) => ApiError::new(ApiErrorCode::NotFound, ids::ERRORS_POSITIONS_NOT_FOUND)
                .details(position_id.to_string())
                .into_response(),
            Err(error) => paper_load_failed(&error),
        };
    };
    let mint = position.mint.clone();
    trade_response(
        crate::trader::manual::manual_paper_close(position).await,
        mint,
    )
}

#[cfg(test)]
#[path = "paper_tests.rs"]
mod tests;
