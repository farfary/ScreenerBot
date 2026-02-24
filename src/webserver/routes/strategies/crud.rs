use axum::{
    extract::{Path, Query},
    http::StatusCode,
    response::Response,
    Json,
};
use chrono::Utc;

use crate::{
    logger::{self, LogTag},
    strategies::{
        self,
        db::{
            delete_strategy, get_all_strategies, get_enabled_strategies, get_strategy,
            insert_strategy, update_strategy,
        },
        types::*,
    },
    webserver::utils::success_response,
};

use super::types::*;
use super::utils::err;

/// GET /api/strategies - List all strategies
pub async fn list_strategies(Query(query): Query<StrategyListQuery>) -> Response {
    logger::info(
        LogTag::Webserver,
        &format!(
            "GET /api/strategies - type={:?}, enabled={:?}",
            query.strategy_type, query.enabled
        ),
    );

    // Get strategies based on filters
    let strategies = if let Some(type_str) = query.strategy_type {
        let strategy_type = match type_str.to_uppercase().as_str() {
            "ENTRY" => StrategyType::Entry,
            "EXIT" => StrategyType::Exit,
            _ => {
                return err(
                    StatusCode::BAD_REQUEST,
                    "Invalid strategy type. Must be ENTRY or EXIT",
                );
            }
        };

        if let Some(enabled) = query.enabled {
            if enabled {
                match get_enabled_strategies(strategy_type) {
                    Ok(list) => list,
                    Err(e) => {
                        return err(
                            StatusCode::INTERNAL_SERVER_ERROR,
                            &format!("Failed to get strategies: {e}"),
                        );
                    }
                }
            } else {
                match get_all_strategies() {
                    Ok(list) => list
                        .into_iter()
                        .filter(|s| s.strategy_type == strategy_type && !s.enabled)
                        .collect(),
                    Err(e) => {
                        return err(
                            StatusCode::INTERNAL_SERVER_ERROR,
                            &format!("Failed to get strategies: {e}"),
                        );
                    }
                }
            }
        } else {
            match get_all_strategies() {
                Ok(list) => list
                    .into_iter()
                    .filter(|s| s.strategy_type == strategy_type)
                    .collect(),
                Err(e) => {
                    return err(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        &format!("Failed to get strategies: {e}"),
                    );
                }
            }
        }
    } else if let Some(enabled) = query.enabled {
        match get_all_strategies() {
            Ok(list) => list.into_iter().filter(|s| s.enabled == enabled).collect(),
            Err(e) => {
                return err(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    &format!("Failed to get strategies: {e}"),
                );
            }
        }
    } else {
        match get_all_strategies() {
            Ok(list) => list,
            Err(e) => {
                return err(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    &format!("Failed to get strategies: {e}"),
                );
            }
        }
    };

    let items: Vec<StrategyItem> = strategies
        .into_iter()
        .map(|s| StrategyItem {
            id: s.id,
            name: s.name,
            description: s.description,
            strategy_type: s.strategy_type.to_string(),
            enabled: s.enabled,
            priority: s.priority,
            created_at: s.created_at.to_rfc3339(),
            updated_at: s.updated_at.to_rfc3339(),
            author: s.author,
            version: s.version,
        })
        .collect();

    let total = items.len();

    let response = StrategyListResponse {
        items,
        total,
        timestamp: Utc::now().to_rfc3339(),
    };

    success_response(response)
}

/// GET /api/strategies/:id - Get strategy details
pub async fn get_strategy_detail(Path(id): Path<String>) -> Response {
    logger::info(LogTag::Webserver, &format!("GET /api/strategies/{id}"));

    let strategy = match get_strategy(&id) {
        Ok(Some(s)) => s,
        Ok(None) => return err(StatusCode::NOT_FOUND, "Strategy not found"),
        Err(e) => {
            return err(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("Failed to get strategy: {e}"),
            );
        }
    };

    let rules_json = match serde_json::to_value(&strategy.rules) {
        Ok(json) => json,
        Err(e) => {
            return err(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("Failed to serialize rules: {e}"),
            );
        }
    };

    let response = StrategyDetailResponse {
        id: strategy.id,
        name: strategy.name,
        description: strategy.description,
        strategy_type: strategy.strategy_type.to_string(),
        enabled: strategy.enabled,
        priority: strategy.priority,
        rules: rules_json,
        parameters: strategy.parameters,
        created_at: strategy.created_at.to_rfc3339(),
        updated_at: strategy.updated_at.to_rfc3339(),
        author: strategy.author,
        version: strategy.version,
    };

    success_response(response)
}

/// POST /api/strategies - Create new strategy
pub async fn create_strategy(Json(request): Json<StrategyRequest>) -> Response {
    logger::info(
        LogTag::Webserver,
        &format!("POST /api/strategies - name={}", request.name),
    );

    // Parse strategy type
    let strategy_type = match request.strategy_type.to_uppercase().as_str() {
        "ENTRY" => StrategyType::Entry,
        "EXIT" => StrategyType::Exit,
        _ => {
            return err(
                StatusCode::BAD_REQUEST,
                "Invalid strategy type. Must be ENTRY or EXIT",
            );
        }
    };

    // Parse rules
    let rules: RuleTree = match serde_json::from_value(request.rules) {
        Ok(rules) => rules,
        Err(e) => {
            return err(
                StatusCode::BAD_REQUEST,
                &format!("Invalid rules JSON: {e}"),
            );
        }
    };

    // Generate ID from name
    let id = request
        .name
        .to_lowercase()
        .replace(" ", "-")
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '-')
        .collect::<String>();

    // Check if strategy with this ID already exists
    if let Ok(Some(_)) = get_strategy(&id) {
        return err(
            StatusCode::CONFLICT,
            &format!("Strategy with ID '{id}' already exists"),
        );
    }

    let now = Utc::now();
    let strategy = Strategy {
        id: id.clone(),
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

    // Validate strategy before saving
    if let Err(e) = strategies::validate_strategy(&strategy).await {
        return err(
            StatusCode::BAD_REQUEST,
            &format!("Strategy validation failed: {e}"),
        );
    }

    // Insert into database
    if let Err(e) = insert_strategy(&strategy) {
        return err(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("Failed to create strategy: {e}"),
        );
    }

    logger::info(
        LogTag::Webserver,
        &format!(
            "Strategy created: id={}, name={}",
            strategy.id, strategy.name
        ),
    );

    success_response(serde_json::json!({
        "id": strategy.id,
        "message": "Strategy created successfully"
    }))
}

/// PUT /api/strategies/:id - Update strategy
pub async fn update_strategy_handler(
    Path(id): Path<String>,
    Json(request): Json<StrategyRequest>,
) -> Response {
    logger::info(LogTag::Webserver, &format!("PUT /api/strategies/{id}"));

    // Check if strategy exists
    let existing = match get_strategy(&id) {
        Ok(Some(s)) => s,
        Ok(None) => return err(StatusCode::NOT_FOUND, "Strategy not found"),
        Err(e) => {
            return err(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("Failed to get strategy: {e}"),
            );
        }
    };

    // Parse strategy type
    let strategy_type = match request.strategy_type.to_uppercase().as_str() {
        "ENTRY" => StrategyType::Entry,
        "EXIT" => StrategyType::Exit,
        _ => {
            return err(
                StatusCode::BAD_REQUEST,
                "Invalid strategy type. Must be ENTRY or EXIT",
            );
        }
    };

    // Parse rules
    let rules: RuleTree = match serde_json::from_value(request.rules) {
        Ok(rules) => rules,
        Err(e) => {
            return err(
                StatusCode::BAD_REQUEST,
                &format!("Invalid rules JSON: {e}"),
            );
        }
    };

    let strategy = Strategy {
        id: id.clone(),
        name: request.name,
        description: request.description,
        strategy_type,
        enabled: request.enabled,
        priority: request.priority,
        timeframe: request.timeframe,
        rules,
        parameters: request.parameters,
        created_at: existing.created_at,
        updated_at: Utc::now(),
        author: request.author,
        version: existing.version + 1,
    };

    // Validate strategy before saving
    if let Err(e) = strategies::validate_strategy(&strategy).await {
        return err(
            StatusCode::BAD_REQUEST,
            &format!("Strategy validation failed: {e}"),
        );
    }

    // Update in database
    if let Err(e) = update_strategy(&strategy) {
        return err(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("Failed to update strategy: {e}"),
        );
    }

    // Clear evaluation cache after update
    if let Err(e) = strategies::clear_evaluation_cache().await {
        logger::info(
            LogTag::Webserver,
            &format!("Failed to clear evaluation cache: {e}"),
        );
    }

    logger::info(
        LogTag::Webserver,
        &format!(
            "Strategy updated: id={}, version={}",
            strategy.id, strategy.version
        ),
    );

    success_response(serde_json::json!({
        "id": strategy.id,
        "version": strategy.version,
        "message": "Strategy updated successfully"
    }))
}

/// DELETE /api/strategies/:id - Delete strategy
pub async fn delete_strategy_handler(Path(id): Path<String>) -> Response {
    logger::info(LogTag::Webserver, &format!("DELETE /api/strategies/{id}"));

    // Check if strategy exists
    match get_strategy(&id) {
        Ok(Some(_)) => {}
        Ok(None) => return err(StatusCode::NOT_FOUND, "Strategy not found"),
        Err(e) => {
            return err(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("Failed to get strategy: {e}"),
            );
        }
    }

    // Delete from database
    if let Err(e) = delete_strategy(&id) {
        return err(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("Failed to delete strategy: {e}"),
        );
    }

    // Clear evaluation cache after deletion
    if let Err(e) = strategies::clear_evaluation_cache().await {
        logger::info(
            LogTag::Webserver,
            &format!("Failed to clear evaluation cache: {e}"),
        );
    }

    logger::info(LogTag::Webserver, &format!("Strategy deleted: id={id}"));

    success_response(serde_json::json!({
        "id": id,
        "message": "Strategy deleted successfully"
    }))
}
