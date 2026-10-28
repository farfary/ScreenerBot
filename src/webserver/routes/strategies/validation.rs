// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Strategies validation route — validates strategy configurations before saving.

use axum::{extract::Query, response::Response, Json};
use chrono::Utc;

use crate::{
    logger::{self, LogTag},
    strategies::{
        self,
        db::{get_strategy_async, rules_would_change_async},
        types::*,
    },
    webserver::utils::success_response,
};

use super::crud::edited_strategy;
use super::types::{StrategyRequest, StrategyValidateQuery};
use super::utils::{err, err_cause};
use crate::i18n::{ids, UiArg, UiText};
use crate::trader::paper::TradingMode;
use crate::webserver::api_error::ApiErrorCode;

/// Validation outcome: `errors` are catalog texts the dashboard renders.
fn invalid(errors: Vec<UiText>) -> Response {
    success_response(serde_json::json!({"valid": false, "errors": errors}))
}

/// Run the engine validation and map its result to the response. A rule
/// problem is a validation result; any other failure is an API error. A valid draft of a
/// stored strategy also answers whether saving it would change the rules that strategy
/// trades on, by the store's own fingerprint comparison.
async fn validation_response(strategy: &Strategy, stored: bool) -> Response {
    match strategies::validate_strategy(strategy).await {
        Ok(_) if stored => match rules_would_change_async(strategy).await {
            Ok(Some(rules_changed)) => {
                success_response(serde_json::json!({"valid": true, "rules_changed": rules_changed}))
            }
            Ok(None) => err(ApiErrorCode::NotFound, ids::ERRORS_STRATEGIES_NOT_FOUND),
            Err(e) => err_cause(
                ApiErrorCode::Internal,
                ids::ERRORS_STRATEGIES_GET_FAILED,
                &e,
            ),
        },
        Ok(_) => success_response(serde_json::json!({"valid": true})),
        Err(crate::Error::Strategies(e)) => invalid(vec![e.ui_text()]),
        Err(e) => err_cause(
            ApiErrorCode::ServiceUnavailable,
            ids::ERRORS_STRATEGIES_VALIDATION_FAILED,
            &e,
        ),
    }
}

/// POST /api/strategies/validate - Validate a strategy draft from the JSON body. With
/// `?id=`, the draft is an edit of that stored strategy and is built the way the update
/// route would store it.
pub async fn validate_strategy_inline_handler(
    Query(query): Query<StrategyValidateQuery>,
    Json(mut request): Json<StrategyRequest>,
) -> Response {
    logger::info(
        LogTag::Webserver,
        &format!(
            "POST /api/strategies/validate - name={}, id={:?}",
            request.name, query.id
        ),
    );

    let existing = match query.id.as_deref() {
        None => None,
        Some(id) => match get_strategy_async(id).await {
            Ok(Some(existing)) => Some(existing),
            Ok(None) => return err(ApiErrorCode::NotFound, ids::ERRORS_STRATEGIES_NOT_FOUND),
            Err(e) => {
                return err_cause(
                    ApiErrorCode::Internal,
                    ids::ERRORS_STRATEGIES_GET_FAILED,
                    &e,
                )
            }
        },
    };

    // Parse strategy type
    let strategy_type = match request.strategy_type.to_uppercase().as_str() {
        "ENTRY" => StrategyType::Entry,
        "EXIT" => StrategyType::Exit,
        _ => {
            return invalid(vec![UiText::new(ids::ERRORS_STRATEGIES_INVALID_TYPE)]);
        }
    };

    if let Err(error) = Strategy::check_priority(request.priority) {
        return invalid(vec![error.ui_text()]);
    }

    // Parse rules
    let rules: RuleTree = match serde_json::from_value(std::mem::take(&mut request.rules)) {
        Ok(rules) => rules,
        Err(e) => {
            return invalid(vec![UiText::new(ids::STRATEGIES_ERROR_INVALID_RULES)
                .arg("reason", UiArg::Text(e.to_string()))]);
        }
    };

    if let Some(existing) = existing {
        let strategy = edited_strategy(&existing, request, strategy_type, rules);
        return validation_response(&strategy, true).await;
    }

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
        entry_strategy_ids: Strategy::entry_binding(
            strategy_type,
            request.entry_strategy_ids.unwrap_or_default(),
        ),
        mode: TradingMode::Paper,
        rules_changed_at: now,
    };

    validation_response(&strategy, false).await
}
