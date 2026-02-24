use axum::{extract::Path, http::StatusCode, response::Response};

use crate::{
    logger::{self, LogTag},
    strategies::db::{get_strategy, get_strategy_performance},
    webserver::utils::success_response,
};

use super::types::StrategyPerformanceResponse;
use super::utils::err;

/// GET /api/strategies/:id/performance - Get strategy performance stats
pub async fn get_strategy_performance_stats(Path(id): Path<String>) -> Response {
    logger::info(
        LogTag::Webserver,
        &format!("GET /api/strategies/{id}/performance"),
    );

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
                &format!("Failed to get performance stats: {e}"),
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
