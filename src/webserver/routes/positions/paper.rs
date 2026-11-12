// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Paper book routes under `/positions/paper`: its rounds, stats, account and run reset,
//! a round's details, and the manual close of a paper round. They read and write the paper
//! store only; no live position route accepts a paper round.

use std::sync::Arc;

use axum::{
    extract::{Path, Query},
    middleware::from_fn_with_state,
    response::{IntoResponse as _, Response},
    routing::{get, post},
    Router,
};

use crate::chains::ChainId;
use crate::database::catalog;
use crate::i18n::ids;
use crate::logger::{self, LogTag};
use crate::positions::paper::{PaperAccount, PaperBook, PaperRoundHistory};
use crate::webserver::api_error::{ApiError, ApiErrorCode};
use crate::webserver::middleware::store_gate;
use crate::webserver::routes::trader::{trade_response, trader_failure};
use crate::webserver::state::AppState;
use crate::webserver::utils::success_response;

use super::detail::build_position_details;
use super::list::{position_responses, positions_stats};
use super::types::{EntryRecordResponse, ExitRecordResponse, PaperAccountResponse, PositionsQuery};

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
    if crate::webserver::promo::are_promo_fixtures_enabled() {
        return account_response(crate::webserver::promo::get_promo_paper_account());
    }
    match crate::trader::paper::paper_account().await {
        Ok(account) => account_response(account),
        Err(error) => trader_failure(&error),
    }
}

/// Settles every open paper round at its mark and starts the next paper run.
async fn reset_paper_run() -> Response {
    if crate::webserver::promo::are_promo_fixtures_enabled() {
        return crate::webserver::promo::promo_write_refused();
    }
    match crate::trader::paper::reset_paper_run().await {
        Ok(account) => account_response(account),
        Err(error) => trader_failure(&error),
    }
}

fn account_response(account: PaperAccount) -> Response {
    success_response(paper_account_response(account))
}

/// The starting balance and cash of the current run.
fn paper_account_response(account: PaperAccount) -> PaperAccountResponse {
    PaperAccountResponse {
        epoch: account.epoch,
        started_at: account.started_at.timestamp(),
        starting_balance_native: account.starting_balance_native,
        cash_native: account.cash_native,
    }
}

/// A paper round in the shape of a live one: its booked entries and exits, each with the
/// strategy that decided it and the quotes it filled at. A paper round has no swaps to
/// confirm, so no swap is pending.
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
        Ok(history) => {
            let (entries, exits) = paper_records(history, book.chain());
            success_response(build_position_details(&position, entries, exits, Vec::new()).await)
        }
        Err(error) => paper_load_failed(&error),
    }
}

/// The entry and exit records of a paper round on `chain`, oldest first, each joined to
/// the leg its transaction reference names.
fn paper_records(
    history: PaperRoundHistory,
    chain: ChainId,
) -> (Vec<EntryRecordResponse>, Vec<ExitRecordResponse>) {
    let PaperRoundHistory {
        entries,
        exits,
        legs,
    } = history;
    let leg_of = |reference: &str| legs.iter().find(|leg| leg.leg_ref == reference);
    let mut entries: Vec<EntryRecordResponse> = entries
        .into_iter()
        .map(|entry| {
            let leg = leg_of(&entry.transaction_signature);
            EntryRecordResponse::from_record(entry, chain).with_leg(leg)
        })
        .collect();
    let mut exits: Vec<ExitRecordResponse> = exits
        .into_iter()
        .map(|exit| {
            let leg = leg_of(&exit.transaction_signature);
            ExitRecordResponse::from_record(exit, chain).with_leg(leg)
        })
        .collect();
    entries.sort_by_key(|entry| entry.timestamp);
    exits.sort_by_key(|exit| exit.timestamp);
    (entries, exits)
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
