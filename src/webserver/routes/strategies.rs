use axum::{
    extract::{Path, Query},
    http::StatusCode,
    response::Response,
    routing::{delete, get, post, put},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

use crate::{
    logger::{self, LogTag},
    strategies::{
        self, db,
        db::{
            delete_strategy, get_all_strategies, get_enabled_strategies, get_strategy,
            get_strategy_performance, insert_strategy, update_strategy,
        },
        engine::{EngineConfig, StrategyEngine},
        types::*,
    },
    webserver::{
        state::AppState,
        utils::{error_response, success_response},
    },
};

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/// Helper to create error response with standard format
fn err(status: StatusCode, message: &str) -> Response {
    error_response(
        status,
        match status {
            StatusCode::BAD_REQUEST => "BAD_REQUEST",
            StatusCode::NOT_FOUND => "NOT_FOUND",
            StatusCode::CONFLICT => "CONFLICT",
            StatusCode::INTERNAL_SERVER_ERROR => "INTERNAL_SERVER_ERROR",
            _ => "ERROR",
        },
        message,
        None,
    )
}

// =============================================================================
// RESPONSE TYPES (Inline with routes as per ScreenerBot patterns)
// =============================================================================

/// Strategy list response
#[derive(Debug, Serialize)]
pub struct StrategyListResponse {
    pub items: Vec<StrategyItem>,
    pub total: usize,
    pub timestamp: String,
}

/// Strategy item in list
#[derive(Debug, Serialize)]
pub struct StrategyItem {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub strategy_type: String,
    pub enabled: bool,
    pub priority: i32,
    pub created_at: String,
    pub updated_at: String,
    pub author: Option<String>,
    pub version: i32,
}

/// Strategy detail response
#[derive(Debug, Serialize)]
pub struct StrategyDetailResponse {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub strategy_type: String,
    pub enabled: bool,
    pub priority: i32,
    pub rules: serde_json::Value,
    pub parameters: HashMap<String, serde_json::Value>,
    pub created_at: String,
    pub updated_at: String,
    pub author: Option<String>,
    pub version: i32,
}

/// Strategy performance response
#[derive(Debug, Serialize)]
pub struct StrategyPerformanceResponse {
    pub strategy_id: String,
    pub total_evaluations: u64,
    pub successful_signals: u64,
    pub success_rate: f64,
    pub avg_execution_time_ms: f64,
    pub last_evaluation: String,
}

/// Strategy test request
#[derive(Debug, Deserialize)]
pub struct StrategyTestRequest {
    pub token_mint: String,
    pub current_price: f64,
    #[serde(default)]
    pub market_data: Option<TestMarketData>,
    #[serde(default)]
    pub position_data: Option<TestPositionData>,
    #[serde(default)]
    pub ohlcv_data: Option<TestOhlcvData>,
}

#[derive(Debug, Deserialize)]
pub struct TestMarketData {
    pub liquidity_sol: Option<f64>,
    pub volume_24h: Option<f64>,
    pub market_cap: Option<f64>,
    pub holder_count: Option<u32>,
    pub token_age_hours: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct TestPositionData {
    pub entry_price: f64,
    pub entry_time: String,
    pub current_size_sol: f64,
    pub unrealized_profit_pct: Option<f64>,
    pub position_age_hours: f64,
}

#[derive(Debug, Deserialize)]
pub struct TestOhlcvData {
    pub candles: Vec<TestCandle>,
    pub timeframe: String,
}

#[derive(Debug, Deserialize)]
pub struct TestCandle {
    pub timestamp: String,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

/// Strategy test response
#[derive(Debug, Serialize)]
pub struct StrategyTestResponse {
    pub strategy_id: String,
    pub strategy_name: String,
    pub result: bool,
    pub confidence: f64,
    pub execution_time_ms: u64,
    pub details: HashMap<String, serde_json::Value>,
}

/// Strategy create/update request
#[derive(Debug, Deserialize)]
pub struct StrategyRequest {
    pub name: String,
    pub description: Option<String>,
    pub strategy_type: String,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default = "default_priority")]
    pub priority: i32,
    pub rules: serde_json::Value,
    #[serde(default)]
    pub parameters: HashMap<String, serde_json::Value>,
    pub author: Option<String>,
}

fn default_enabled() -> bool {
    true
}

fn default_priority() -> i32 {
    10
}

/// Query parameters for strategy list
#[derive(Debug, Deserialize)]
pub struct StrategyListQuery {
    #[serde(rename = "type")]
    pub strategy_type: Option<String>,
    pub enabled: Option<bool>,
}

/// Condition schemas response
#[derive(Debug, Serialize)]
pub struct ConditionSchemasResponse {
    pub schemas: serde_json::Value,
    pub timestamp: String,
}

/// Strategy templates list response
#[derive(Debug, Serialize)]
pub struct StrategyTemplatesResponse {
    pub items: Vec<StrategyTemplateItem>,
    pub total: usize,
    pub timestamp: String,
}

/// Strategy template item
#[derive(Debug, Serialize)]
pub struct StrategyTemplateItem {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub category: String,
    pub risk_level: String,
    pub rules: serde_json::Value,
    pub parameters: HashMap<String, serde_json::Value>,
    pub created_at: String,
    pub author: Option<String>,
}

// =============================================================================
// ROUTE HANDLERS
// =============================================================================

/// GET /api/strategies - List all strategies
async fn list_strategies(Query(query): Query<StrategyListQuery>) -> Response {
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
                            &format!("Failed to get strategies: {}", e),
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
                            &format!("Failed to get strategies: {}", e),
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
                        &format!("Failed to get strategies: {}", e),
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
                    &format!("Failed to get strategies: {}", e),
                );
            }
        }
    } else {
        match get_all_strategies() {
            Ok(list) => list,
            Err(e) => {
                return err(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    &format!("Failed to get strategies: {}", e),
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
async fn get_strategy_detail(Path(id): Path<String>) -> Response {
    logger::info(
        LogTag::Webserver,
            &format!("GET /api/strategies/{}", id),
        );

    let strategy = match get_strategy(&id) {
        Ok(Some(s)) => s,
        Ok(None) => return err(StatusCode::NOT_FOUND, "Strategy not found"),
        Err(e) => {
            return err(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("Failed to get strategy: {}", e),
            );
        }
    };

    let rules_json = match serde_json::to_value(&strategy.rules) {
        Ok(json) => json,
        Err(e) => {
            return err(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("Failed to serialize rules: {}", e),
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
async fn create_strategy(Json(request): Json<StrategyRequest>) -> Response {
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
                &format!("Invalid rules JSON: {}", e),
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
            &format!("Strategy with ID '{}' already exists", id),
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
            &format!("Strategy validation failed: {}", e),
        );
    }

    // Insert into database
    if let Err(e) = insert_strategy(&strategy) {
        return err(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("Failed to create strategy: {}", e),
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
async fn update_strategy_handler(
    Path(id): Path<String>,
    Json(request): Json<StrategyRequest>,
) -> Response {
    logger::info(
        LogTag::Webserver,
            &format!("PUT /api/strategies/{}", id),
        );

    // Check if strategy exists
    let existing = match get_strategy(&id) {
        Ok(Some(s)) => s,
        Ok(None) => return err(StatusCode::NOT_FOUND, "Strategy not found"),
        Err(e) => {
            return err(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("Failed to get strategy: {}", e),
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
                &format!("Invalid rules JSON: {}", e),
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
            &format!("Strategy validation failed: {}", e),
        );
    }

    // Update in database
    if let Err(e) = update_strategy(&strategy) {
        return err(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("Failed to update strategy: {}", e),
        );
    }

    // Clear evaluation cache after update
    if let Err(e) = strategies::clear_evaluation_cache().await {
        logger::info(
        LogTag::Webserver,
            &format!("Failed to clear evaluation cache: {}", e),
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
async fn delete_strategy_handler(Path(id): Path<String>) -> Response {
    logger::info(
        LogTag::Webserver,
            &format!("DELETE /api/strategies/{}", id),
        );

    // Check if strategy exists
    match get_strategy(&id) {
        Ok(Some(_)) => {}
        Ok(None) => return err(StatusCode::NOT_FOUND, "Strategy not found"),
        Err(e) => {
            return err(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("Failed to get strategy: {}", e),
            );
        }
    }

    // Delete from database
    if let Err(e) = delete_strategy(&id) {
        return err(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("Failed to delete strategy: {}", e),
        );
    }

    // Clear evaluation cache after deletion
    if let Err(e) = strategies::clear_evaluation_cache().await {
        logger::info(
        LogTag::Webserver,
            &format!("Failed to clear evaluation cache: {}", e),
        );
    }

    logger::info(
        LogTag::Webserver,
        &format!("Strategy deleted: id={}", id),
    );

    success_response(serde_json::json!({
        "id": id,
        "message": "Strategy deleted successfully"
    }))
}

/// GET /api/strategies/:id/performance - Get strategy performance stats
async fn get_strategy_performance_stats(Path(id): Path<String>) -> Response {
    logger::info(
        LogTag::Webserver,
            &format!("GET /api/strategies/{}/performance", id),
        );

    // Check if strategy exists
    match get_strategy(&id) {
        Ok(Some(_)) => {}
        Ok(None) => return err(StatusCode::NOT_FOUND, "Strategy not found"),
        Err(e) => {
            return err(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("Failed to get strategy: {}", e),
            );
        }
    }

    // Get performance stats
    let performance = match get_strategy_performance(&id) {
        Ok(Some(p)) => p,
        Ok(None) => {
            return err(
                StatusCode::NOT_FOUND,
                "No performance data available for this strategy",
            );
        }
        Err(e) => {
            return err(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("Failed to get performance stats: {}", e),
            );
        }
    };

    let success_rate = if performance.total_evaluations > 0 {
        (performance.successful_signals as f64 / performance.total_evaluations as f64) * 100.0
    } else {
        0.0
    };

    let response = StrategyPerformanceResponse {
        strategy_id: performance.strategy_id,
        total_evaluations: performance.total_evaluations,
        successful_signals: performance.successful_signals,
        success_rate,
        avg_execution_time_ms: performance.avg_execution_time_ms,
        last_evaluation: performance.last_evaluation.to_rfc3339(),
    };

    success_response(response)
}

/// POST /api/strategies/:id/test - Test strategy evaluation
async fn test_strategy(
    Path(id): Path<String>,
    Json(request): Json<StrategyTestRequest>,
) -> Response {
    logger::info(
        LogTag::Webserver,
            &format!(
                "POST /api/strategies/{}/test - token={}",
                id, request.token_mint
            ),
        );

    // Get strategy
    let strategy = match get_strategy(&id) {
        Ok(Some(s)) => s,
        Ok(None) => return err(StatusCode::NOT_FOUND, "Strategy not found"),
        Err(e) => {
            return err(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("Failed to get strategy: {}", e),
            );
        }
    };

    // Convert test data to evaluation context
    let market_data = request.market_data.map(|md| MarketData {
        liquidity_sol: md.liquidity_sol,
        volume_24h: md.volume_24h,
        market_cap: md.market_cap,
        holder_count: md.holder_count,
        token_age_hours: md.token_age_hours,
    });

    let position_data = request.position_data.and_then(|pd| {
        DateTime::parse_from_rfc3339(&pd.entry_time)
            .ok()
            .map(|entry_time| PositionData {
                entry_price: pd.entry_price,
                entry_time: entry_time.with_timezone(&Utc),
                current_size_sol: pd.current_size_sol,
                unrealized_profit_pct: pd.unrealized_profit_pct,
                position_age_hours: pd.position_age_hours,
            })
    });

    let ohlcv_data = request.ohlcv_data.and_then(|od| {
        let candles: Result<Vec<Candle>, _> = od
            .candles
            .into_iter()
            .map(|c| {
                DateTime::parse_from_rfc3339(&c.timestamp)
                    .map(|ts| Candle {
                        timestamp: ts.with_timezone(&Utc),
                        open: c.open,
                        high: c.high,
                        low: c.low,
                        close: c.close,
                        volume: c.volume,
                    })
                    .map_err(|_| ())
            })
            .collect();

        candles.ok().map(|candles| OhlcvData {
            candles,
            timeframe: od.timeframe,
        })
    });

    let context = EvaluationContext {
        token_mint: request.token_mint.clone(),
        current_price: Some(request.current_price),
        position_data,
        market_data,
        ohlcv_data,
    };

    // Create engine and evaluate
    let engine = StrategyEngine::new(EngineConfig::default());
    let eval_result = match engine.evaluate_strategy(&strategy, &context).await {
        Ok(result) => result,
        Err(e) => {
            return err(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("Strategy evaluation failed: {}", e),
            );
        }
    };

    let response = StrategyTestResponse {
        strategy_id: strategy.id,
        strategy_name: strategy.name,
        result: eval_result.result,
        confidence: eval_result.confidence,
        execution_time_ms: eval_result.execution_time_ms,
        details: eval_result.details,
    };

    success_response(response)
}

/// GET /api/strategies/conditions/schemas - Get all condition schemas
async fn get_condition_schemas() -> Response {
    logger::info(
        LogTag::Webserver,
            "GET /api/strategies/conditions/schemas",
        );

    let schemas = match strategies::get_condition_schemas().await {
        Ok(s) => s,
        Err(e) => {
            return err(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("Failed to get condition schemas: {}", e),
            );
        }
    };

    let response = ConditionSchemasResponse {
        schemas,
        timestamp: Utc::now().to_rfc3339(),
    };

    success_response(response)
}

// =============================================================================
// ROUTER SETUP
// =============================================================================

/// Create the strategies router with all routes
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        // Strategy CRUD
        .route("/", get(list_strategies).post(create_strategy))
        .route(
            "/:id",
            get(get_strategy_detail)
                .put(update_strategy_handler)
                .delete(delete_strategy_handler),
        )
        // Performance and testing
        .route("/:id/performance", get(get_strategy_performance_stats))
        .route("/:id/test", post(test_strategy))
        // Validate / Deploy
        .route("/:id/validate", post(validate_strategy_handler))
        .route("/:id/deploy", post(deploy_strategy_handler))
        // Condition schemas
        .route("/conditions/schemas", get(get_condition_schemas))
        // Templates
        .route("/templates", get(list_templates))
}

/// GET /api/strategies/templates - List available strategy templates
async fn list_templates() -> Response {
    // For now, load templates from DB if available; otherwise return empty list.
    // The DB schema exists; add a simple read using rusqlite directly here to avoid exposing internals.
    // We provide an empty response if not implemented in db module.
    let mut items: Vec<StrategyTemplateItem> = Vec::new();

    // Attempt to query templates table
    let result: Result<Vec<StrategyTemplateItem>, String> = (|| {
        // Open a read-only connection to strategies DB directly for listing templates
        let db_path = "data/strategies.db";
        let conn = rusqlite::Connection::open(db_path)
            .map_err(|e| format!("Failed to open strategies db: {}", e))?;
        let mut stmt = conn
            .prepare(
                "SELECT id, name, description, category, risk_level, rules_json, parameters_json, created_at, author FROM strategy_templates ORDER BY created_at DESC",
            )
            .map_err(|e| format!("Failed to prepare templates query: {}", e))?;
        let rows = stmt
            .query_map([], |row| {
                let rules_json: String = row.get(5)?;
                let params_json: String = row.get(6)?;
                let rules_val: serde_json::Value =
                    serde_json::from_str(&rules_json).unwrap_or(serde_json::Value::Null);
                let params_val: HashMap<String, serde_json::Value> =
                    serde_json::from_str(&params_json).unwrap_or_default();
                Ok(StrategyTemplateItem {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    category: row.get(3)?,
                    risk_level: row.get::<_, String>(4)?,
                    rules: rules_val,
                    parameters: params_val,
                    created_at: row.get::<_, String>(7)?,
                    author: row.get(8)?,
                })
            })
            .map_err(|e| format!("Failed to query templates: {}", e))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Failed to collect templates: {}", e))?;
        Ok(rows)
    })();

    if let Ok(rows) = result {
        items = rows;
    }

    let total = items.len();
    let response = StrategyTemplatesResponse {
        items,
        total,
        timestamp: Utc::now().to_rfc3339(),
    };

    success_response(response)
}

/// POST /api/strategies/:id/validate - Validate a strategy by id
async fn validate_strategy_handler(Path(id): Path<String>) -> Response {
    let strategy = match get_strategy(&id) {
        Ok(Some(s)) => s,
        Ok(None) => return err(StatusCode::NOT_FOUND, "Strategy not found"),
        Err(e) => {
            return err(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("Failed to get strategy: {}", e),
            )
        }
    };

    match strategies::validate_strategy(&strategy).await {
        Ok(_) => success_response(serde_json::json!({"valid": true})),
        Err(e) => success_response(serde_json::json!({"valid": false, "errors": [e]})),
    }
}

/// POST /api/strategies/:id/deploy - Enable a strategy
async fn deploy_strategy_handler(Path(id): Path<String>) -> Response {
    let mut strategy = match get_strategy(&id) {
        Ok(Some(s)) => s,
        Ok(None) => return err(StatusCode::NOT_FOUND, "Strategy not found"),
        Err(e) => {
            return err(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("Failed to get strategy: {}", e),
            )
        }
    };

    // Enable and bump version
    strategy.enabled = true;
    strategy.version += 1;
    strategy.updated_at = Utc::now();

    if let Err(e) = update_strategy(&strategy) {
        return err(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("Failed to deploy strategy: {}", e),
        );
    }

    // Clear evaluation cache after deployment
    if let Err(e) = strategies::clear_evaluation_cache().await {
        logger::info(
        LogTag::Webserver,
            &format!("Failed to clear evaluation cache: {}", e),
        );
    }

    success_response(serde_json::json!({"id": strategy.id, "message": "Strategy deployed"}))
}
