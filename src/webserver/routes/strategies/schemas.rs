use axum::response::Response;
use axum::http::StatusCode;
use chrono::Utc;

use crate::{
    logger::{self, LogTag},
    strategies,
    webserver::utils::success_response,
};

use super::types::ConditionSchemasResponse;
use super::utils::err;

/// GET /api/strategies/conditions/schemas - Get all condition schemas
pub async fn get_condition_schemas() -> Response {
    logger::info(LogTag::Webserver, "GET /api/strategies/conditions/schemas");

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
