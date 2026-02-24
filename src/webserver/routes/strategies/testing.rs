//! Strategies testing route — runs strategy backtests and simulations.

use axum::{extract::Path, http::StatusCode, response::Response, Json};
use chrono::{DateTime, Utc};

use crate::{
    logger::{self, LogTag},
    strategies::{
        db::get_strategy,
        engine::{EngineConfig, StrategyEngine},
        types::*,
    },
    webserver::utils::success_response,
};

use super::types::{StrategyTestRequest, StrategyTestResponse};
use super::utils::err;

/// POST /api/strategies/:id/test - Test strategy evaluation
pub async fn test_strategy(
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
                &format!("Failed to get strategy: {e}"),
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

    // TODO Phase 4: Re-enable OHLCV bundle creation for test endpoint
    // For now, strategies will work without OHLCV data (non-OHLCV conditions only)
    let timeframe_bundle = None; // request.ohlcv_data conversion will be implemented in Phase 4

    let context = EvaluationContext {
        token_mint: request.token_mint.clone(),
        current_price: Some(request.current_price),
        position_data,
        market_data,
        timeframe_bundle,
        strategy_timeframe: strategy.timeframe.clone(),
    };

    // Create engine and evaluate
    let engine = StrategyEngine::new(EngineConfig::default());
    let eval_result = match engine.evaluate_strategy(&strategy, &context).await {
        Ok(result) => result,
        Err(e) => {
            return err(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("Strategy evaluation failed: {e}"),
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
