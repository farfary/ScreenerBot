// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Trader statistics, exit templates and the time exit watching each open position.

use axum::{
    extract::Query,
    response::{IntoResponse as _, Response},
    Json,
};

use crate::i18n::ids;
use crate::positions::Book;
use crate::webserver::api_error::{ApiError, ApiErrorCode};
use crate::webserver::utils::success_response;

use super::control::trader_failure;
use super::types::*;

// =============================================================================
// TRADER STATS HANDLER
// =============================================================================

/// GET /api/trader/stats - realized trading performance of one book (`book=live`, the
/// default, or `book=paper`) over a selectable window. The books are never mixed.
pub async fn get_trader_stats(Query(query): Query<TraderStatsQuery>) -> Response {
    let book = query.book.unwrap_or_default();
    // Return promotional fixtures only for owner-initiated media capture.
    if crate::webserver::promo::are_promo_fixtures_enabled() && book == Book::Live {
        return success_response(crate::webserver::promo::get_promo_trader_stats());
    }
    success_response(crate::trader::stats::trader_stats(book, query.window.into()).await)
}

// =============================================================================
// TEMPLATE ENDPOINTS
// =============================================================================

pub async fn get_templates() -> Response {
    success_response(TemplateListResponse {
        templates: crate::trader::templates::all_templates(),
    })
}

/// POST /api/trader/apply-template - Apply a preset exit template
pub async fn apply_template(Json(request): Json<ApplyTemplateRequest>) -> Response {
    match crate::trader::templates::apply_template(&request.template_id) {
        Ok(template) => success_response(serde_json::json!({
            "template": template,
        })),
        Err(error @ crate::trader::Error::TemplateNotFound { .. }) => trader_failure(&error),
        Err(error) => ApiError::new(
            ApiErrorCode::ConfigError,
            ids::ERRORS_TRADE_CONFIG_UPDATE_FAILED,
        )
        .details(error.to_string())
        .into_response(),
    }
}

// =============================================================================
// TIME EXIT WATCH
// =============================================================================

/// GET /api/trader/time-exits - for each open position, the time exit that applies to it
/// under the exit policy the exit monitor resolves for it, and when that exit is due.
pub async fn get_time_exits() -> Response {
    let now = chrono::Utc::now();
    let mut watches = Vec::new();
    for position in crate::positions::get_open_positions().await {
        let Some(position_id) = position.id else {
            continue;
        };
        let policy = crate::trader::policy::resolve_exit_policy(&position).await;
        watches.push(PositionTimeExit {
            position_id,
            time_exit: crate::trader::evaluators::exit_time_watch::time_exit_watch(
                &position, &policy, now,
            ),
        });
    }
    success_response(watches)
}
