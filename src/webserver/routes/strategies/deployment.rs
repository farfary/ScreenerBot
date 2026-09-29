//! Strategies deployment route — activates and deactivates trading strategies.

use axum::{extract::Path, response::Response};
use chrono::Utc;

use crate::{
    logger::{self, LogTag},
    strategies::{
        self,
        db::{get_strategy, update_strategy},
    },
    webserver::utils::success_response,
};

use super::utils::{err, err_cause};
use crate::i18n::ids;
use crate::webserver::api_error::ApiErrorCode;

/// POST /api/strategies/:id/deploy - Enable a strategy
pub async fn deploy_strategy_handler(Path(id): Path<String>) -> Response {
    let mut strategy = match get_strategy(&id) {
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

    // Enable and bump version
    strategy.enabled = true;
    strategy.version += 1;
    strategy.updated_at = Utc::now();

    if let Err(e) = update_strategy(&strategy) {
        return err_cause(
            ApiErrorCode::Internal,
            ids::ERRORS_STRATEGIES_DEPLOY_FAILED,
            &e,
        );
    }

    // Clear evaluation cache after deployment
    if let Err(e) = strategies::clear_evaluation_cache().await {
        logger::info(
            LogTag::Webserver,
            &format!("Failed to clear evaluation cache: {e}"),
        );
    }

    success_response(serde_json::json!({"id": strategy.id}))
}
