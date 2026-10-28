// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Strategies store async API — async wrappers that run the synchronous SQLite operations on the blocking pool.

use crate::strategies::types::{Strategy, StrategyType};
use crate::trader::paper::TradingMode;

/// Run one synchronous store operation on the blocking pool, so the runtime worker that
/// awaits it never holds a SQLite connection.
async fn on_blocking_pool<T, F>(operation: F) -> crate::Result<T>
where
    F: FnOnce() -> crate::Result<T> + Send + 'static,
    T: Send + 'static,
{
    tokio::task::spawn_blocking(operation)
        .await
        .map_err(crate::Error::from)
        .and_then(|result| result)
}

/// [`super::insert_strategy`] on the blocking pool.
pub async fn insert_strategy_async(strategy: &Strategy) -> crate::Result<()> {
    let strategy = strategy.clone();
    on_blocking_pool(move || super::insert_strategy(&strategy)).await
}

/// [`super::update_strategy`] on the blocking pool.
pub async fn update_strategy_async(strategy: &Strategy) -> crate::Result<bool> {
    let strategy = strategy.clone();
    on_blocking_pool(move || super::update_strategy(&strategy)).await
}

/// [`super::rules_would_change`] on the blocking pool.
pub async fn rules_would_change_async(strategy: &Strategy) -> crate::Result<Option<bool>> {
    let strategy = strategy.clone();
    on_blocking_pool(move || super::rules_would_change(&strategy)).await
}

/// [`super::set_strategy_mode`] on the blocking pool.
pub async fn set_strategy_mode_async(strategy_id: &str, mode: TradingMode) -> crate::Result<()> {
    let strategy_id = strategy_id.to_owned();
    on_blocking_pool(move || super::set_strategy_mode(&strategy_id, mode)).await
}

/// [`super::delete_strategy`] on the blocking pool.
pub async fn delete_strategy_async(strategy_id: &str) -> crate::Result<()> {
    let strategy_id = strategy_id.to_owned();
    on_blocking_pool(move || super::delete_strategy(&strategy_id)).await
}

/// [`super::get_strategy`] on the blocking pool.
pub async fn get_strategy_async(strategy_id: &str) -> crate::Result<Option<Strategy>> {
    let strategy_id = strategy_id.to_owned();
    on_blocking_pool(move || super::get_strategy(&strategy_id)).await
}

/// [`super::get_all_strategies`] on the blocking pool.
pub async fn get_all_strategies_async() -> crate::Result<Vec<Strategy>> {
    on_blocking_pool(super::get_all_strategies).await
}

/// [`super::get_strategies_with_totals`] on the blocking pool.
pub async fn get_strategies_with_totals_async() -> crate::Result<super::StrategiesWithTotals> {
    on_blocking_pool(super::get_strategies_with_totals).await
}

/// [`super::get_enabled_strategies`] on the blocking pool.
pub async fn get_enabled_strategies_async(
    strategy_type: StrategyType,
) -> crate::Result<Vec<Strategy>> {
    on_blocking_pool(move || super::get_enabled_strategies(strategy_type)).await
}
