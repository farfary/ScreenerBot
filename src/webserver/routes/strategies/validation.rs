use axum::{extract::Path, http::StatusCode, response::Response, Json};
use chrono::Utc;

use crate::{
    logger::{self, LogTag},
    strategies::{self, db::get_strategy, types::*},
    webserver::utils::success_response,
};

use super::types::StrategyRequest;
use super::utils::err;

/// POST /api/strategies/:id/validate - Validate a strategy by id
pub async fn validate_strategy_handler(Path(id): Path<String>) -> Response {
    let strategy = match get_strategy(&id) {
        Ok(Some(s)) => s,
        Ok(None) => return err(StatusCode::NOT_FOUND, "Strategy not found"),
        Err(e) => {
            return err(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("Failed to get strategy: {e}"),
            )
        }
    };

    match strategies::validate_strategy(&strategy).await {
        Ok(_) => success_response(serde_json::json!({"valid": true})),
        Err(e) => success_response(serde_json::json!({"valid": false, "errors": [e.to_string()]})),
    }
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
            return success_response(serde_json::json!({
                "valid": false,
                "errors": ["Invalid strategy type. Must be ENTRY or EXIT"]
            }));
        }
    };

    // Parse rules
    let rules: RuleTree = match serde_json::from_value(request.rules) {
        Ok(rules) => rules,
        Err(e) => {
            return success_response(serde_json::json!({
                "valid": false,
                "errors": [format!("Invalid rules JSON: {e}")]
            }));
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

    match strategies::validate_strategy(&strategy).await {
        Ok(_) => success_response(serde_json::json!({"valid": true})),
        Err(e) => success_response(serde_json::json!({"valid": false, "errors": [e.to_string()]})),
    }
}
