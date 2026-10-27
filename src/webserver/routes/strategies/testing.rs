// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Strategies testing route — evaluates a stored strategy once, rule by rule, in a token's
//! live context or against supplied market and position data.

use std::time::{Duration, Instant};

use axum::{extract::Path, response::Response, Json};
use chrono::{DateTime, Utc};

use crate::{
    chains::ChainId,
    logger::{self, LogTag},
    ohlcvs::{OhlcvError, Priority, Timeframe},
    strategies::{
        db::get_strategy,
        engine::{EngineConfig, StrategyEngine},
        types::*,
    },
    trader::evaluators::strategies::live_strategy_context,
    webserver::utils::success_response,
};

use super::types::{StrategyTestRequest, StrategyTestResponse};
use super::utils::{err, err_cause};
use crate::i18n::ids;
use crate::webserver::api_error::ApiErrorCode;

/// Longest a live test waits for the candles its rules read: pool discovery for a token the
/// app has not seen, then one chart read-through per timeframe. Past it the test evaluates
/// what is stored.
const TEST_CANDLE_WAIT: Duration = Duration::from_secs(10);

/// Pause between read-through attempts while pool discovery has not yet named the token's
/// series pool.
const TEST_CANDLE_RETRY: Duration = Duration::from_millis(500);

/// Name of the OHLCV service in the service manager.
const OHLCV_SERVICE: &str = "ohlcv";

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
        Ok(None) => return err(ApiErrorCode::NotFound, ids::ERRORS_STRATEGIES_NOT_FOUND),
        Err(e) => {
            return err_cause(
                ApiErrorCode::Internal,
                ids::ERRORS_STRATEGIES_GET_FAILED,
                &e,
            );
        }
    };

    let engine = StrategyEngine::new(EngineConfig::default());
    let context = match request.current_price {
        Some(current_price) => supplied_context(&request, current_price, &strategy),
        None if request.market_data.is_some() || request.position_data.is_some() => {
            return err(
                ApiErrorCode::InvalidInput,
                ids::ERRORS_STRATEGIES_TEST_DATA_WITHOUT_PRICE,
            );
        }
        None => {
            let chain = crate::chains::active_chain();
            let timeframes = engine
                .get_condition_registry()
                .candle_timeframes(&strategy.rules, &strategy.timeframe);
            read_through_candles(chain, &request.token_mint, &timeframes, TEST_CANDLE_WAIT).await;
            live_strategy_context(chain, &request.token_mint, &strategy).await
        }
    };

    // Every rule reports its own verdict. A rule that had no usable data is unknown,
    // which is an answer of its own, not a failure of the test.
    let started = Instant::now();
    let rules = match engine.explain_strategy(&strategy, &context).await {
        Ok(rules) => rules,
        Err(e) => {
            return err_cause(
                ApiErrorCode::Internal,
                ids::ERRORS_STRATEGIES_EVALUATION_FAILED,
                &e,
            );
        }
    };
    let result = match rules.verdict {
        RuleVerdict::Pass => Some(true),
        RuleVerdict::Fail => Some(false),
        RuleVerdict::Unknown | RuleVerdict::Off => None,
    };

    success_response(StrategyTestResponse {
        strategy_id: strategy.id,
        strategy_name: strategy.name,
        result,
        unknown_reason: rules.reason.clone(),
        execution_time_ms: started.elapsed().as_millis() as u64,
        rules,
    })
}

/// Reads the `timeframes` of `mint` through the chart's on-demand path
/// (`ohlcvs::get_chart_ohlcv`) ahead of a live test, so a token the app holds no candles for
/// is tested on the Data Server's series. The token is monitored at chart-view priority,
/// which starts its pool discovery; a timeframe is read again while discovery has not named
/// the series pool. Everything ends within `wait`, and nothing runs while the OHLCV service
/// is not running. A timeframe still missing or stale afterwards stays unknown in the result.
async fn read_through_candles(
    chain: ChainId,
    mint: &str,
    timeframes: &[Timeframe],
    wait: Duration,
) {
    if timeframes.is_empty() {
        return;
    }
    let read = async {
        if !ohlcv_service_running().await {
            return;
        }
        let priority = if crate::positions::is_open_position(mint).await {
            Priority::Critical
        } else {
            Priority::High
        };
        if let Err(e) = crate::ohlcvs::add_token_monitoring(chain, mint, priority).await {
            logger::info(
                LogTag::Webserver,
                &format!("mint={mint} strategy_test_monitoring_failed error={e}"),
            );
            return;
        }
        let mut pending = timeframes.to_vec();
        loop {
            let reads = futures::future::join_all(
                pending
                    .iter()
                    .map(|timeframe| crate::ohlcvs::get_chart_ohlcv(chain, mint, *timeframe, 1)),
            )
            .await;
            pending = pending
                .into_iter()
                .zip(reads)
                .filter(|(_, read)| matches!(read, Err(OhlcvError::PoolNotFound(_))))
                .map(|(timeframe, _)| timeframe)
                .collect();
            if pending.is_empty() {
                return;
            }
            tokio::time::sleep(TEST_CANDLE_RETRY).await;
        }
    };
    if tokio::time::timeout(wait, read).await.is_err() {
        logger::debug(
            LogTag::Webserver,
            &format!(
                "mint={mint} strategy_test_candles_wait_elapsed wait_ms={}",
                wait.as_millis()
            ),
        );
    }
}

/// Whether the service manager runs the OHLCV service.
async fn ohlcv_service_running() -> bool {
    let Some(manager) = crate::services::get_service_manager().await else {
        return false;
    };
    let running = manager
        .read()
        .await
        .as_ref()
        .is_some_and(|manager| manager.is_service_running(OHLCV_SERVICE));
    running
}

/// The evaluation context of a test that supplies its own price, market and position data.
fn supplied_context(
    request: &StrategyTestRequest,
    current_price: f64,
    strategy: &Strategy,
) -> EvaluationContext {
    let market_data = request.market_data.as_ref().map(|md| MarketData {
        liquidity_sol: md.liquidity_native,
        liquidity_pool: None,
        transactions: None,
        snapshot_at: None,
        token_created_at: None,
        reference_size_native: None,
    });

    let position_data = request.position_data.as_ref().and_then(|pd| {
        DateTime::parse_from_rfc3339(&pd.entry_time)
            .ok()
            .map(|entry_time| PositionData {
                entry_price: pd.entry_price,
                entry_time: entry_time.with_timezone(&Utc),
                current_size_sol: pd.current_size_native,
                unrealized_profit_pct: pd.unrealized_profit_pct,
                position_age_hours: pd.position_age_hours,
                peak_price: pd.peak_price,
                dca_count: pd.dca_count,
                partial_exit_count: pd.partial_exit_count,
                remaining_value_native: pd.remaining_value_native,
                entry_liquidity_native: pd.entry_liquidity_native,
                entry_liquidity_pool: pd.entry_liquidity_pool.clone(),
            })
    });

    EvaluationContext {
        token_mint: request.token_mint.clone(),
        current_price: Some(current_price),
        position_data,
        market_data,
        timeframe_bundle: None,
        strategy_timeframe: strategy.timeframe.clone(),
        evaluated_at: Utc::now(),
    }
}
