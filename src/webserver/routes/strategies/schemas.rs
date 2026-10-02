// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Strategies schemas route — serves condition and action schema definitions for the UI.

use axum::response::Response;
use chrono::Utc;

use crate::{
    logger::{self, LogTag},
    strategies,
    webserver::utils::success_response,
};

use super::types::ConditionSchemasResponse;
use super::utils::err_cause;
use crate::i18n::ids;
use crate::webserver::api_error::ApiErrorCode;

/// GET /api/strategies/conditions/schemas - Get all condition schemas
pub async fn get_condition_schemas() -> Response {
    logger::info(LogTag::Webserver, "GET /api/strategies/conditions/schemas");

    let schemas = match strategies::get_condition_schemas().await {
        Ok(s) => s,
        Err(e) => {
            return err_cause(
                ApiErrorCode::Internal,
                ids::ERRORS_STRATEGIES_SCHEMAS_FAILED,
                &e,
            );
        }
    };

    let response = ConditionSchemasResponse {
        schemas,
        timestamp: Utc::now().to_rfc3339(),
    };

    success_response(response)
}
