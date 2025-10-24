//! Strategy management and application for trading decisions

use crate::logger::{self, LogTag};
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
        logger::info(
        LogTag::Trader,
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

        // Call strategies module for evaluation with timeout
        let strategy_timeout = std::time::Duration::from_secs(5);
        let evaluation_result = tokio::time::timeout(
            strategy_timeout,
            strategies::evaluate_entry_strategies(
                token_mint,
                price_info.price_sol,
                Some(market_data),
                None, // OHLCV data could be added later
            )
        )
        .await;

        match evaluation_result {
            Ok(Ok(Some(strategy_id))) => {
                logger::info(
        LogTag::Trader,
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
            Ok(Ok(None)) => Ok(None),
            Ok(Err(e)) => {
                logger::info(
        LogTag::Trader,
                    &format!("Strategy evaluation error for {}: {}", token_mint, e),
                );
                Ok(None) // Don't fail trading on strategy errors
            }
            Err(_timeout) => {
                logger::info(
        LogTag::Trader,
                    &format!(
                        "⚠️ STRATEGY_TIMEOUT: Entry evaluation for {} exceeded {}s - Consider increasing timeout or optimizing strategies. This timeout is distinct from 'no signal' case.",
                        token_mint,
                        strategy_timeout.as_secs()
                    ),
                );
                
                // Record event for metrics tracking
                crate::events::record_safe(crate::events::Event::new(
                    crate::events::EventCategory::Entry,
                    Some("strategy_evaluation_timeout".to_string()),
                    crate::events::Severity::Error,
                    Some(token_mint.to_string()),
                    None,
                    serde_json::json!({
                        "timeout_seconds": strategy_timeout.as_secs(),
                        "evaluation_type": "entry",
                    }),
                ))
                .await;
                
                Ok(None) // Skip this token on timeout
            }
        }
    }

    /// Check if a position should be exited based on strategies
    pub async fn check_exit_strategies(
        position: &Position,
        current_price: f64,
    ) -> Result<Option<TradeDecision>, String> {
        logger::info(
        LogTag::Trader,
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

        // Call strategies module for evaluation with timeout
        let strategy_timeout = std::time::Duration::from_secs(5);
        let evaluation_result = tokio::time::timeout(
            strategy_timeout,
            strategies::evaluate_exit_strategies(
                &position.mint,
                current_price,
                position_data,
                Some(market_data),
                None, // OHLCV data could be added later
            )
        )
        .await;

        match evaluation_result {
            Ok(Ok(Some(strategy_id))) => {
                logger::info(
        LogTag::Trader,
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
            Ok(Ok(None)) => Ok(None),
            Ok(Err(e)) => {
                logger::info(
        LogTag::Trader,
                    &format!("Strategy evaluation error for position {:?}: {}", position.id, e),
                );
                Ok(None) // Don't fail trading on strategy errors
            }
            Err(_timeout) => {
                logger::info(
        LogTag::Trader,
                    &format!(
                        "⚠️ STRATEGY_TIMEOUT: Exit evaluation for position {:?} (mint={}) exceeded {}s - Consider increasing timeout or optimizing strategies. This timeout is distinct from 'no signal' case.",
                        position.id,
                        position.mint,
                        strategy_timeout.as_secs()
                    ),
                );
                
                // Record event for metrics tracking
                crate::events::record_safe(crate::events::Event::new(
                    crate::events::EventCategory::Position,
                    Some("strategy_evaluation_timeout".to_string()),
                    crate::events::Severity::Error,
                    Some(position.mint.clone()),
                    None,
                    serde_json::json!({
                        "timeout_seconds": strategy_timeout.as_secs(),
                        "evaluation_type": "exit",
                        "position_id": position.id,
                    }),
                ))
                .await;
                
                Ok(None) // Skip this position on timeout
            }
        }
    }
}
