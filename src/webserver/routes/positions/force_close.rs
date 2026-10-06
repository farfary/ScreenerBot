// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Force-close route — allows manual closure of ghost positions stuck in open state.

use axum::{
    extract::Path,
    response::{IntoResponse as _, Response},
    Json,
};
use serde::{Deserialize, Serialize};

use crate::errors::ErrorClass;
use crate::i18n::ids;
use crate::logger::{self, LogTag};
use crate::positions;
use crate::webserver::api_error::{ApiError, ApiErrorCode};
use crate::webserver::utils::success_response;

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
}

pub(super) async fn force_close_position(
    Path(position_id): Path<i64>,
    body: Option<Json<ForceCloseRequest>>,
) -> Response {
    let note = body
        .and_then(|b| b.reason.clone())
        .unwrap_or_else(|| "manual force close".to_owned());

    match positions::operations::force_close_position(position_id, &note).await {
        Ok(closed) => success_response(ForceCloseResponse {
            success: true,
            position_id: closed.position_id,
            symbol: closed.symbol,
            reason: closed.closed_reason,
        }),
        Err(positions::Error::NotFoundById { .. }) => {
            ApiError::new(ApiErrorCode::NotFound, ids::ERRORS_POSITIONS_NOT_FOUND)
                .details(position_id.to_string())
                .into_response()
        }
        Err(positions::Error::AlreadyClosed { .. }) => ApiError::new(
            ApiErrorCode::InvalidInput,
            ids::ERRORS_POSITIONS_ALREADY_CLOSED,
        )
        .details(position_id.to_string())
        .into_response(),
        Err(error) => {
            logger::error(
                LogTag::Positions,
                &format!("Force-close of position {position_id} failed: {error}"),
            );
            ApiError::new(
                ApiErrorCode::for_status(error.http_status()),
                ids::ERRORS_POSITIONS_FORCE_CLOSE_FAILED,
            )
            .details(error.to_string())
            .into_response()
        }
    }
}
