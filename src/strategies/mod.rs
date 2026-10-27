// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Trading strategy engine — rule trees, conditions, and evaluation logic.
mod error;
mod error_text;

pub mod arm;
pub mod conditions;
pub mod database;
pub use database as db;
pub mod engine;
pub mod overview;
pub mod readiness;
pub(crate) mod recorder;
pub mod types;

pub use error::{Error, Result};

use crate::logger::{self, LogTag};
use crate::ohlcvs::TimeframeBundle;
use crate::positions::Book;
use crate::strategies::engine::{EngineConfig, StrategyEngine};
use crate::strategies::types::{EvaluationContext, MarketData, PositionData, Strategy};
use crate::trader::paper::TradingMode;
use chrono::Utc;
use std::sync::Arc;
use std::sync::LazyLock;
use tokio::sync::RwLock;

/// Global strategy engine instance
static STRATEGY_ENGINE: LazyLock<Arc<RwLock<Option<StrategyEngine>>>> =
    LazyLock::new(|| Arc::new(RwLock::new(None)));

/// Initialize the strategy system
pub async fn init_strategy_system(config: EngineConfig) -> crate::Result<()> {
    // The engine exists whatever the state of the store, so an evaluation reaches the
    // store's own error instead of an uninitialized engine.
    *STRATEGY_ENGINE.write().await = Some(StrategyEngine::new(config));
    recorder::start();

    db::init_strategies_db()?;
    logger::info(LogTag::System, "Strategy system initialized successfully");

    Ok(())
}

/// Get the global strategy engine
async fn get_engine() -> crate::Result<Arc<RwLock<Option<StrategyEngine>>>> {
    let engine = STRATEGY_ENGINE.read().await;
    if engine.is_none() {
        return Err(crate::Error::internal_error(
            "Strategy engine not initialized",
        ));
    }
    drop(engine);
    Ok(STRATEGY_ENGINE.clone())
}

/// The modes of the exit strategies that judge a position on `book`. A live exit strategy
/// judges both books, so a paper position faces every rule a live one would; a paper exit
/// strategy judges paper positions only.
pub fn exit_strategy_modes(book: Book) -> &'static [TradingMode] {
    match book {
        Book::Live => &[TradingMode::Live],
        Book::Paper => &[TradingMode::Live, TradingMode::Paper],
    }
}

/// Evaluate entry strategies for a token
///
/// This function is the main entry point for the trader module to check
/// if any entry strategies signal to open a position for a token.
///
/// # Arguments
/// * `strategies` - The enabled entry strategies of the entry cycle, in priority order
/// * `modes` - The modes admitted for this token: only strategies of these modes are
///   evaluated, in priority order across the modes
/// * `token_mint` - The token mint address
/// * `current_price` - Current token price in SOL
/// * `market_data` - Optional market data (liquidity, transaction counts)
/// * `timeframe_bundle` - Optional multi-timeframe OHLCV bundle
///
/// # Returns
/// * `Ok(Some((strategy_id, mode)))` - The first strategy that signals entry and its mode,
///   which decides the book the entry is made on
/// * `Ok(None)` - If no strategy signals entry
/// * `Err(e)` - If evaluation fails
pub async fn evaluate_entry_strategies(
    strategies: &[Strategy],
    modes: &[TradingMode],
    token_mint: &str,
    current_price: f64,
    market_data: Option<MarketData>,
    timeframe_bundle: Option<TimeframeBundle>,
) -> crate::Result<Option<(String, TradingMode)>> {
    let engine_lock = get_engine().await?;
    let engine_guard = engine_lock.read().await;
    let engine = engine_guard
        .as_ref()
        .ok_or_else(|| crate::Error::internal_error("Strategy engine not available"))?;

    // Evaluate the strategies of the admitted modes by priority (lower priority first)
    for strategy in strategies
        .iter()
        .filter(|strategy| modes.contains(&strategy.mode))
    {
        let context = EvaluationContext {
            token_mint: token_mint.to_string(),
            current_price: Some(current_price),
            position_data: None,
            market_data: market_data.clone(),
            timeframe_bundle: timeframe_bundle.clone(),
            strategy_timeframe: strategy.timeframe.clone(),
            evaluated_at: Utc::now(),
        };

        let result = engine.evaluate_strategy(strategy, &context).await;

        match result {
            Ok(eval_result) => {
                let signalled = eval_result.result;
                if let Err(e) = recorder::record(eval_result, token_mint).await {
                    logger::warning(LogTag::System, &format!("Failed to record evaluation: {e}"));
                }

                // If strategy signals entry, return it
                if signalled {
                    logger::info(
                        LogTag::System,
                        &format!(
                            "Entry strategy triggered: strategy={}, token={}, price={:.9}",
                            strategy.name, token_mint, current_price
                        ),
                    );
                    return Ok(Some((strategy.id.clone(), strategy.mode)));
                }
            }
            // A token without the data a condition needs (a fresh token short of
            // candles) answers Unknown: an expected state, not a fault.
            Err(e) if e.is_data_unavailable() => {
                logger::debug(
                    LogTag::System,
                    &format!(
                        "Entry strategy unknown: strategy={}, token={}, reason={}",
                        strategy.name, token_mint, e
                    ),
                );
            }
            Err(e) => {
                logger::error(
                    LogTag::System,
                    &format!(
                        "Entry strategy evaluation error: strategy={}, error={}",
                        strategy.name, e
                    ),
                );
            }
        }
    }

    Ok(None)
}

/// Evaluate exit strategies for a position
///
/// This function is the main entry point for the trader module to check
/// if any exit strategies signal to close a position.
///
/// # Arguments
/// * `strategies` - The enabled exit strategies of the exit cycle, in priority order
/// * `book` - The book of the position: only strategies of the modes
///   [`exit_strategy_modes`] gives it are evaluated
/// * `token_mint` - The token mint address
/// * `current_price` - Current token price in SOL
/// * `position_data` - Position data (entry price, age, etc.)
/// * `market_data` - Optional market data (liquidity, transaction counts)
/// * `timeframe_bundle` - Optional multi-timeframe OHLCV bundle
/// * `entry_strategy_id` - The entry strategy that opened the position, if any; an exit
///   strategy bound to other entry strategies is not evaluated
///
/// # Returns
/// * `Ok(Some(strategy_id))` - If a strategy signals exit
/// * `Ok(None)` - If no strategy signals exit
/// * `Err(e)` - If evaluation fails
pub async fn evaluate_exit_strategies(
    strategies: &[Strategy],
    book: Book,
    token_mint: &str,
    current_price: f64,
    position_data: PositionData,
    market_data: Option<MarketData>,
    timeframe_bundle: Option<TimeframeBundle>,
    entry_strategy_id: Option<&str>,
) -> crate::Result<Option<String>> {
    let engine_lock = get_engine().await?;
    let engine_guard = engine_lock.read().await;
    let engine = engine_guard
        .as_ref()
        .ok_or_else(|| crate::Error::internal_error("Strategy engine not available"))?;

    // Evaluate, by priority (lower priority first), the exit strategies that judge the
    // position's book and manage positions of this entry strategy
    let modes = exit_strategy_modes(book);
    for strategy in strategies
        .iter()
        .filter(|strategy| modes.contains(&strategy.mode) && strategy.manages(entry_strategy_id))
    {
        let context = EvaluationContext {
            token_mint: token_mint.to_string(),
            current_price: Some(current_price),
            position_data: Some(position_data.clone()),
            market_data: market_data.clone(),
            timeframe_bundle: timeframe_bundle.clone(),
            strategy_timeframe: strategy.timeframe.clone(),
            evaluated_at: Utc::now(),
        };

        let result = engine.evaluate_strategy(strategy, &context).await;

        match result {
            Ok(eval_result) => {
                let signalled = eval_result.result;
                if let Err(e) = recorder::record(eval_result, token_mint).await {
                    logger::warning(LogTag::System, &format!("Failed to record evaluation: {e}"));
                }

                // If strategy signals exit, return it
                if signalled {
                    logger::info(LogTag::System, &format!("Exit strategy triggered: strategy={}, token={}, price={:.9}, entry_price={:.9}, profit_pct={:.2}%",
                            strategy.name, token_mint, current_price, position_data.entry_price,
                            position_data.unrealized_profit_pct.unwrap_or_default()),
                    );
                    return Ok(Some(strategy.id.clone()));
                }
            }
            Err(e) if e.is_data_unavailable() => {
                logger::debug(
                    LogTag::System,
                    &format!(
                        "Exit strategy unknown: strategy={}, token={}, reason={}",
                        strategy.name, token_mint, e
                    ),
                );
            }
            Err(e) => {
                logger::error(
                    LogTag::System,
                    &format!(
                        "Exit strategy evaluation error: strategy={}, error={}",
                        strategy.name, e
                    ),
                );
            }
        }
    }

    Ok(None)
}

/// Validate a strategy without evaluation
pub async fn validate_strategy(strategy: &Strategy) -> crate::Result<()> {
    let engine_lock = get_engine().await?;
    let engine_guard = engine_lock.read().await;
    let engine = engine_guard
        .as_ref()
        .ok_or_else(|| crate::Error::internal_error("Strategy engine not available"))?;

    Ok(engine.validate_strategy(strategy)?)
}

/// Clear the evaluation cache
pub async fn clear_evaluation_cache() -> crate::Result<()> {
    let engine_lock = get_engine().await?;
    let engine_guard = engine_lock.read().await;
    let engine = engine_guard
        .as_ref()
        .ok_or_else(|| crate::Error::internal_error("Strategy engine not available"))?;

    engine.clear_cache().await;
    Ok(())
}

/// Get all condition schemas for UI
pub async fn get_condition_schemas() -> crate::Result<serde_json::Value> {
    let engine_lock = get_engine().await?;
    let engine_guard = engine_lock.read().await;
    let engine = engine_guard
        .as_ref()
        .ok_or_else(|| crate::Error::internal_error("Strategy engine not available"))?;

    let registry = engine.get_condition_registry();
    Ok(registry.get_all_schemas())
}

// Re-export commonly used types for convenience
pub use types::{
    Condition, LogicalOperator, Parameter, ParameterConstraints, RiskLevel, RuleTree,
    StrategyPerformance, StrategyTemplate,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_live_exit_strategy_judges_both_books_and_a_paper_one_only_paper() {
        for book in Book::ALL {
            assert!(
                exit_strategy_modes(book).contains(&TradingMode::Live),
                "{book:?}"
            );
            assert_eq!(
                exit_strategy_modes(book).contains(&TradingMode::Paper),
                book == Book::Paper,
                "{book:?}"
            );
        }
    }
}
