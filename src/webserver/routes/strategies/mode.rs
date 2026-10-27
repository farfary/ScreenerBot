// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Strategy trading-mode routes: the confirmation-gated switch between paper and live,
//! and the live readiness read from the strategy's paper rounds.

use axum::{
    extract::Path,
    response::{IntoResponse, Response},
    Json,
};

use super::types::{StrategyModeRequest, StrategyReadinessResponse};
use super::utils::{err, err_cause};
use crate::config::with_config;
use crate::i18n::ids;
use crate::logger::{self, LogTag};
use crate::positions::db::PaperExitOwner;
use crate::positions::Position;
use crate::strategies::{
    self, arm,
    db::get_strategy_async,
    readiness::{live_block_text, strategy_readiness},
    types::Strategy,
};
use crate::trader::paper::STRATEGY_LIVE_CONFIRMATION;
use crate::webserver::api_error::{ApiError, ApiErrorCode};
use crate::webserver::utils::success_response;

/// The strategy `id`, or the response that explains why it cannot be read.
async fn find_strategy(id: &str) -> Result<Strategy, Response> {
    match get_strategy_async(id).await {
        Ok(Some(strategy)) => Ok(strategy),
        Ok(None) => Err(err(
            ApiErrorCode::NotFound,
            ids::ERRORS_STRATEGIES_NOT_FOUND,
        )),
        Err(e) => Err(err_cause(
            ApiErrorCode::Internal,
            ids::ERRORS_STRATEGIES_GET_FAILED,
            &e,
        )),
    }
}

/// PATCH /api/strategies/:id/mode - Return a strategy to paper, or arm it live with the
/// exact confirmation phrase while the Auto Trader can trade live. Open positions keep the
/// book they were opened on; only new entries follow the mode.
pub async fn set_strategy_mode_handler(
    Path(id): Path<String>,
    Json(request): Json<StrategyModeRequest>,
) -> Response {
    logger::info(
        LogTag::Webserver,
        &format!(
            "PATCH /api/strategies/{id}/mode - mode={}",
            request.mode.as_str()
        ),
    );

    let strategy = match find_strategy(&id).await {
        Ok(strategy) => strategy,
        Err(response) => return response,
    };
    let mode = match arm::set_mode(&strategy, request.mode, request.confirmation.as_deref()).await {
        Ok(mode) => mode,
        Err(crate::Error::Strategies(strategies::Error::LiveConfirmationRequired)) => {
            return err(
                ApiErrorCode::LiveConfirmationRequired,
                ids::ERRORS_STRATEGIES_LIVE_CONFIRMATION_REQUIRED,
            );
        }
        Err(crate::Error::Strategies(strategies::Error::LiveUnavailable { reason })) => {
            return ApiError::with_text(ApiErrorCode::LiveUnavailable, live_block_text(reason))
                .into_response();
        }
        Err(e) => {
            return err_cause(
                ApiErrorCode::Internal,
                ids::ERRORS_STRATEGIES_UPDATE_MODE_FAILED,
                &e,
            );
        }
    };

    success_response(serde_json::json!({
        "id": strategy.id,
        "mode": mode,
    }))
}

/// GET /api/strategies/:id/readiness - The live readiness of a strategy from the paper
/// rounds it took part in during the current paper run, with the phrase that arms it
/// live. Without a loaded paper book the strategy has no rounds yet.
pub async fn get_strategy_readiness(Path(id): Path<String>) -> Response {
    let strategy = match find_strategy(&id).await {
        Ok(strategy) => strategy,
        Err(response) => return response,
    };
    let evidence = match paper_evidence(&strategy.id).await {
        Ok(evidence) => evidence,
        Err(e) => {
            return err_cause(
                ApiErrorCode::Internal,
                ids::ERRORS_STRATEGIES_READINESS_FAILED,
                &e,
            );
        }
    };
    let min_closed_rounds = with_config(|config| config.trader.paper.readiness_min_closed_rounds);
    success_response(StrategyReadinessResponse {
        readiness: strategy_readiness(
            strategy.strategy_type,
            &evidence.rounds,
            min_closed_rounds,
            evidence.failed_exit_streak,
            crate::trader::live_block(),
        ),
        live_confirmation: STRATEGY_LIVE_CONFIRMATION,
    })
}

/// What the paper book holds about one strategy.
#[derive(Default)]
struct PaperEvidence {
    /// The paper rounds of the current paper run the strategy took part in.
    rounds: Vec<Position>,
    /// The latest paper exits of its rounds in a row that found no fill.
    failed_exit_streak: u32,
}

/// The paper evidence of `strategy_id`; empty without a loaded paper book or a wallet.
async fn paper_evidence(strategy_id: &str) -> crate::positions::Result<PaperEvidence> {
    let Some(book) = crate::positions::paper::book() else {
        return Ok(PaperEvidence::default());
    };
    let Ok(wallet_address) = crate::utils::get_wallet_address() else {
        return Ok(PaperEvidence::default());
    };
    Ok(PaperEvidence {
        rounds: book.strategy_rounds(&wallet_address, strategy_id).await?,
        failed_exit_streak: book
            .failed_exit_streak(
                &wallet_address,
                &PaperExitOwner::Strategy(strategy_id.to_owned()),
            )
            .await?,
    })
}
