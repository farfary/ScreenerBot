//! Strategy management and application for trading decisions

use crate::logger::{log, LogTag};
use crate::pools::PriceResult;
use crate::positions::Position;
use crate::strategies;
use crate::strategies::types::{MarketData, PositionData};
use crate::trader::types::{TradeAction, TradeDecision, TradePriority, TradeReason};
use chrono::Utc;

/// Manager for applying strategies to trading decisions
pub struct StrategyManager;

impl StrategyManager {
    /// Check if a token meets entry criteria based on strategies
    pub async fn check_entry_strategies(
        token_mint: &str,
        price_info: &PriceResult,
    ) -> Result<Option<TradeDecision>, String> {
        log(
            LogTag::Trader,
            "DEBUG",
            &format!(
                "Checking entry strategies for token {} (price={:.9} SOL, liquidity={:.2} SOL)",
                token_mint, price_info.price_sol, price_info.sol_reserves
            ),
        );

        // Build market data from price info
        let market_data = MarketData {
            liquidity_sol: Some(price_info.sol_reserves),
            volume_24h: None, // Could be enriched from tokens module if needed
            market_cap: None,
            holder_count: None,
            token_age_hours: None,
        };

        // Call strategies module for evaluation
        match strategies::evaluate_entry_strategies(
            token_mint,
            price_info.price_sol,
            Some(market_data),
            None, // OHLCV data could be added later
        )
        .await
        {
            Ok(Some(strategy_id)) => {
                log(
                    LogTag::Trader,
                    "SUCCESS",
                    &format!(
                        "✅ Entry strategy signal: token={}, strategy={}, price={:.9} SOL",
                        token_mint, strategy_id, price_info.price_sol
                    ),
                );

                Ok(Some(TradeDecision {
                    position_id: None,
                    mint: token_mint.to_string(),
                    action: TradeAction::Buy,
                    reason: TradeReason::StrategySignal,
                    strategy_id: Some(strategy_id),
                    timestamp: Utc::now(),
                    priority: TradePriority::Normal,
                    price_sol: Some(price_info.price_sol),
                    size_sol: None, // Will use config default
                }))
            }
            Ok(None) => Ok(None),
            Err(e) => {
                log(
                    LogTag::Trader,
                    "ERROR",
                    &format!("Strategy evaluation error for {}: {}", token_mint, e),
                );
                Ok(None) // Don't fail trading on strategy errors
            }
        }
    }

    /// Check if a position should be exited based on strategies
    pub async fn check_exit_strategies(
        position: &Position,
        current_price: f64,
    ) -> Result<Option<TradeDecision>, String> {
        log(
            LogTag::Trader,
            "DEBUG",
            &format!(
                "Checking exit strategies for position {:?} token {} (current_price={:.9} SOL)",
                position.id, position.mint, current_price
            ),
        );

        // Build position data
        let unrealized_profit_pct = if position.average_entry_price > 0.0 {
            Some(((current_price - position.average_entry_price) / position.average_entry_price) * 100.0)
        } else {
            None
        };

        let position_data = PositionData {
            entry_price: position.average_entry_price,
            entry_time: position.entry_time,
            current_size_sol: position.total_size_sol,
            unrealized_profit_pct,
            position_age_hours: (Utc::now() - position.entry_time).num_hours() as f64,
        };

        // Build market data (could be enriched from pools/tokens if needed)
        let market_data = MarketData {
            liquidity_sol: None,
            volume_24h: None,
            market_cap: None,
            holder_count: None,
            token_age_hours: None,
        };

        // Call strategies module for evaluation
        match strategies::evaluate_exit_strategies(
            &position.mint,
            current_price,
            position_data,
            Some(market_data),
            None, // OHLCV data could be added later
        )
        .await
        {
            Ok(Some(strategy_id)) => {
                log(
                    LogTag::Trader,
                    "SUCCESS",
                    &format!(
                        "✅ Exit strategy signal: position={:?}, strategy={}, price={:.9} SOL",
                        position.id, strategy_id, current_price
                    ),
                );

                Ok(Some(TradeDecision {
                    position_id: position.id.map(|id| id.to_string()),
                    mint: position.mint.clone(),
                    action: TradeAction::Sell,
                    reason: TradeReason::StrategySignal,
                    strategy_id: Some(strategy_id),
                    timestamp: Utc::now(),
                    priority: TradePriority::Normal,
                    price_sol: Some(current_price),
                    size_sol: None, // Will sell full position or use config
                }))
            }
            Ok(None) => Ok(None),
            Err(e) => {
                log(
                    LogTag::Trader,
                    "ERROR",
                    &format!("Strategy evaluation error for position {:?}: {}", position.id, e),
                );
                Ok(None) // Don't fail trading on strategy errors
            }
        }
    }
}
