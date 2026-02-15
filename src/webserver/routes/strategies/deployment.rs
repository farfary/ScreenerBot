use axum::{extract::Path, http::StatusCode, response::Response};
use chrono::Utc;

use crate::{
    logger::{self, LogTag},
    strategies::{self, db::{get_strategy, update_strategy}},
    webserver::utils::success_response,
};

use super::utils::err;

/// POST /api/strategies/:id/deploy - Enable a strategy
pub async fn deploy_strategy_handler(Path(id): Path<String>) -> Response {
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
