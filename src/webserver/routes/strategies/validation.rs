//! Strategies validation route — validates strategy configurations before saving.

use axum::{extract::Path, response::Response, Json};
use chrono::Utc;

use crate::{
    logger::{self, LogTag},
    strategies::{self, db::get_strategy, types::*},
    webserver::utils::success_response,
};

use super::types::StrategyRequest;
use super::utils::{err, err_cause};
use crate::i18n::{ids, UiArg, UiText};
use crate::webserver::api_error::ApiErrorCode;

/// Validation outcome: `errors` are catalog texts the dashboard renders.
fn invalid(errors: Vec<UiText>) -> Response {
    success_response(serde_json::json!({"valid": false, "errors": errors}))
}

/// Run the engine validation and map its result to the response. A rule
/// problem is a validation result; any other failure is an API error.
async fn validation_response(strategy: &Strategy) -> Response {
    match strategies::validate_strategy(strategy).await {
        Ok(_) => success_response(serde_json::json!({"valid": true})),
        Err(crate::Error::Strategies(e)) => invalid(vec![e.ui_text()]),
        Err(e) => err_cause(
            ApiErrorCode::ServiceUnavailable,
            ids::ERRORS_STRATEGIES_VALIDATION_FAILED,
            &e,
        ),
    }
}

/// POST /api/strategies/:id/validate - Validate a strategy by id
pub async fn validate_strategy_handler(Path(id): Path<String>) -> Response {
    let strategy = match get_strategy(&id) {
        Ok(Some(s)) => s,
        Ok(None) => return err(ApiErrorCode::NotFound, ids::ERRORS_STRATEGIES_NOT_FOUND),
        Err(e) => {
            return err_cause(
                ApiErrorCode::Internal,
                ids::ERRORS_STRATEGIES_GET_FAILED,
                &e,
            )
        }
    };

    validation_response(&strategy).await
}

/// POST /api/strategies/validate - Validate a strategy from JSON body (for unsaved strategies)
pub async fn validate_strategy_inline_handler(Json(request): Json<StrategyRequest>) -> Response {
    logger::info(
        LogTag::Webserver,
        &format!("POST /api/strategies/validate - name={}", request.name),
    );

    // Parse strategy type
    let strategy_type = match request.strategy_type.to_uppercase().as_str() {
        "ENTRY" => StrategyType::Entry,
        "EXIT" => StrategyType::Exit,
        _ => {
            return invalid(vec![UiText::new(ids::ERRORS_STRATEGIES_INVALID_TYPE)]);
        }
    };

    // Parse rules
    let rules: RuleTree = match serde_json::from_value(request.rules) {
        Ok(rules) => rules,
        Err(e) => {
            return invalid(vec![UiText::new(ids::STRATEGIES_ERROR_INVALID_RULES)
                .arg("reason", UiArg::Text(e.to_string()))]);
        }
    };

    let now = Utc::now();
    let strategy = Strategy {
        id: "validation-check".to_owned(),
        name: request.name,
        description: request.description,
        strategy_type,
        enabled: request.enabled,
        priority: request.priority,
        timeframe: request.timeframe,
        rules,
        parameters: request.parameters,
        created_at: now,
        updated_at: now,
        author: request.author,
        version: 1,
    };

    validation_response(&strategy).await
}
