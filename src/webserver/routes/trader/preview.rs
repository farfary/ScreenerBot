//! Trader statistics and exit templates

use axum::{
    extract::Query,
    response::{IntoResponse as _, Response},
    Json,
};

use crate::i18n::ids;
use crate::webserver::api_error::{ApiError, ApiErrorCode};
use crate::webserver::utils::success_response;

use super::control::trader_failure;
use super::types::*;

// =============================================================================
// TRADER STATS HANDLER
// =============================================================================

/// GET /api/trader/stats - realized trading performance over a selectable window.
pub async fn get_trader_stats(Query(query): Query<TraderStatsQuery>) -> Response {
    // Return promotional fixtures only for owner-initiated media capture.
    if crate::webserver::promo::are_promo_fixtures_enabled() {
        return success_response(crate::webserver::promo::get_promo_trader_stats());
    }
    success_response(crate::trader::stats::trader_stats(query.days.unwrap_or(30)).await)
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
