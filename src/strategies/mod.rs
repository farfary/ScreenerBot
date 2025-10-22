pub mod conditions;
pub mod db;
pub mod engine;
pub mod types;

use crate::arguments::is_debug_system_enabled;
use crate::logger::{log, LogTag};
use crate::strategies::db::{get_enabled_strategies, get_strategy, record_evaluation};
use crate::strategies::engine::{EngineConfig, StrategyEngine};
use crate::strategies::types::{
    Candle, EvaluationContext, EvaluationResult, MarketData, OhlcvData, PositionData, Strategy,
    StrategyType,
};
use chrono::Utc;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Global strategy engine instance
static STRATEGY_ENGINE: Lazy<Arc<RwLock<Option<StrategyEngine>>>> =
    Lazy::new(|| Arc::new(RwLock::new(None)));

/// Initialize the strategy system
pub async fn init_strategy_system(config: EngineConfig) -> Result<(), String> {
    // Initialize database
    db::init_strategies_db()?;

    // Create and store engine
    let engine = StrategyEngine::new(config);
    let mut global_engine = STRATEGY_ENGINE.write().await;
    *global_engine = Some(engine);

    log(
        LogTag::System,
        "SUCCESS",
        "Strategy system initialized successfully",
    );

    Ok(())
}

/// Get the global strategy engine
async fn get_engine() -> Result<Arc<RwLock<Option<StrategyEngine>>>, String> {
    let engine = STRATEGY_ENGINE.read().await;
    if engine.is_none() {
        return Err("Strategy engine not initialized".to_string());
    }
    drop(engine);
    Ok(STRATEGY_ENGINE.clone())
}

/// Evaluate entry strategies for a token
///
/// This function is the main entry point for the trader module to check
/// if any entry strategies signal to open a position for a token.
///
/// # Arguments
/// * `token_mint` - The token mint address
/// * `current_price` - Current token price in SOL
/// * `market_data` - Optional market data (liquidity, volume, etc.)
/// * `ohlcv_data` - Optional OHLCV candle data
///
/// # Returns
/// * `Ok(Some(strategy_id))` - If a strategy signals entry
/// * `Ok(None)` - If no strategy signals entry
/// * `Err(e)` - If evaluation fails
pub async fn evaluate_entry_strategies(
    token_mint: &str,
    current_price: f64,
    market_data: Option<MarketData>,
    ohlcv_data: Option<OhlcvData>,
) -> Result<Option<String>, String> {
    let engine_lock = get_engine().await?;
    let engine_guard = engine_lock.read().await;
    let engine = engine_guard
        .as_ref()
        .ok_or_else(|| "Strategy engine not available".to_string())?;

    // Get enabled entry strategies
    let strategies = get_enabled_strategies(StrategyType::Entry)?;

    if strategies.is_empty() {
        return Ok(None);
    }

    // Build evaluation context
    let context = EvaluationContext {
        token_mint: token_mint.to_string(),
        current_price: Some(current_price),
        position_data: None,
        market_data,
        ohlcv_data,
    };

    // Evaluate strategies by priority (lower priority first)
    for strategy in strategies {
        let result = engine.evaluate_strategy(&strategy, &context).await;

        match result {
            Ok(eval_result) => {
                // Record evaluation
                if let Err(e) = record_evaluation(&eval_result, token_mint) {
                    if is_debug_system_enabled() {
                        log(
                            LogTag::System,
                            "WARN",
                            &format!("Failed to record evaluation: {}", e),
                        );
                    }
                }

                // If strategy signals entry, return it
                if eval_result.result {
                    log(
                        LogTag::System,
                        "SUCCESS",
                        &format!(
                            "Entry strategy triggered: strategy={}, token={}, price={:.9}",
                            strategy.name, token_mint, current_price
                        ),
                    );
                    return Ok(Some(strategy.id.clone()));
                }
            }
            Err(e) => {
                if is_debug_system_enabled() {
                    log(
                        LogTag::System,
                        "ERROR",
                        &format!(
                            "Entry strategy evaluation error: strategy={}, error={}",
                            strategy.name, e
                        ),
                    );
                }
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
/// * `token_mint` - The token mint address
/// * `current_price` - Current token price in SOL
/// * `position_data` - Position data (entry price, age, etc.)
/// * `market_data` - Optional market data (liquidity, volume, etc.)
/// * `ohlcv_data` - Optional OHLCV candle data
///
/// # Returns
/// * `Ok(Some(strategy_id))` - If a strategy signals exit
/// * `Ok(None)` - If no strategy signals exit
/// * `Err(e)` - If evaluation fails
pub async fn evaluate_exit_strategies(
    token_mint: &str,
    current_price: f64,
    position_data: PositionData,
    market_data: Option<MarketData>,
    ohlcv_data: Option<OhlcvData>,
) -> Result<Option<String>, String> {
    let engine_lock = get_engine().await?;
    let engine_guard = engine_lock.read().await;
    let engine = engine_guard
        .as_ref()
        .ok_or_else(|| "Strategy engine not available".to_string())?;

    // Get enabled exit strategies
    let strategies = get_enabled_strategies(StrategyType::Exit)?;

    if strategies.is_empty() {
        return Ok(None);
    }

    // Build evaluation context
    let context = EvaluationContext {
        token_mint: token_mint.to_string(),
        current_price: Some(current_price),
        position_data: Some(position_data.clone()),
        market_data,
        ohlcv_data,
    };

    // Evaluate strategies by priority (lower priority first)
    for strategy in strategies {
        let result = engine.evaluate_strategy(&strategy, &context).await;

        match result {
            Ok(eval_result) => {
                // Record evaluation
                if let Err(e) = record_evaluation(&eval_result, token_mint) {
                    if is_debug_system_enabled() {
                        log(
                            LogTag::System,
                            "WARN",
                            &format!("Failed to record evaluation: {}", e),
                        );
                    }
                }

                // If strategy signals exit, return it
                if eval_result.result {
                    log(
                        LogTag::System,
                        "SUCCESS",
                        &format!(
                            "Exit strategy triggered: strategy={}, token={}, price={:.9}, entry_price={:.9}, profit_pct={:.2}%",
                            strategy.name, token_mint, current_price, position_data.entry_price,
                            position_data.unrealized_profit_pct.unwrap_or(0.0)
                        ),
                    );
                    return Ok(Some(strategy.id.clone()));
                }
            }
            Err(e) => {
                if is_debug_system_enabled() {
                    log(
                        LogTag::System,
                        "ERROR",
                        &format!(
                            "Exit strategy evaluation error: strategy={}, error={}",
                            strategy.name, e
                        ),
                    );
                }
            }
        }
    }

    Ok(None)
}

/// Validate a strategy without evaluation
pub async fn validate_strategy(strategy: &Strategy) -> Result<(), String> {
    let engine_lock = get_engine().await?;
    let engine_guard = engine_lock.read().await;
    let engine = engine_guard
        .as_ref()
        .ok_or_else(|| "Strategy engine not available".to_string())?;

    engine.validate_strategy(strategy)
}

/// Clear the evaluation cache
pub async fn clear_evaluation_cache() -> Result<(), String> {
    let engine_lock = get_engine().await?;
    let engine_guard = engine_lock.read().await;
    let engine = engine_guard
        .as_ref()
        .ok_or_else(|| "Strategy engine not available".to_string())?;

    engine.clear_cache().await;
    Ok(())
}

/// Get all condition schemas for UI
pub async fn get_condition_schemas() -> Result<serde_json::Value, String> {
    let engine_lock = get_engine().await?;
    let engine_guard = engine_lock.read().await;
    let engine = engine_guard
        .as_ref()
        .ok_or_else(|| "Strategy engine not available".to_string())?;

    let registry = engine.get_condition_registry();
    Ok(registry.get_all_schemas())
}

// Re-export commonly used types for convenience
pub use types::{
    Condition, LogicalOperator, Parameter, ParameterConstraints, RiskLevel, RuleTree,
    StrategyPerformance, StrategyTemplate,
};
