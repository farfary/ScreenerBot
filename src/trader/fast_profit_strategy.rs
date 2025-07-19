use anyhow::Result;
use chrono::{ DateTime, Utc };
use std::collections::HashMap;

use crate::trader::types::*;
use crate::trader::position::Position;
use crate::config::TraderConfig;

/// Fast profit-taking strategy for capturing quick gains (10%-500% in seconds)
#[derive(Debug, Clone)]
pub struct FastProfitStrategy {
    config: TraderConfig,
    // Track price momentum for each position
    price_history: HashMap<String, Vec<PricePoint>>,
}

#[derive(Debug, Clone)]
struct PricePoint {
    price: f64,
    timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct ProfitTarget {
    pub percentage: f64,
    pub sell_portion: f64, // What percentage of position to sell (0.0-1.0)
    pub time_threshold_seconds: u64, // Max time to hold before selling
}

impl FastProfitStrategy {
    pub fn new(config: TraderConfig) -> Self {
        Self {
            config,
            price_history: HashMap::new(),
        }
    }

    /// Define multi-tier profit targets for fast profit-taking
    pub fn get_profit_targets() -> Vec<ProfitTarget> {
        vec![
            // Tier 1: Quick scalp profits
            ProfitTarget {
                percentage: 10.0, // 10% profit
                sell_portion: 0.25, // Sell 25% of position
                time_threshold_seconds: 30, // Within 30 seconds
            },
            ProfitTarget {
                percentage: 25.0, // 25% profit
                sell_portion: 0.25, // Sell another 25%
                time_threshold_seconds: 60, // Within 1 minute
            },
            // Tier 2: Medium profits
            ProfitTarget {
                percentage: 50.0, // 50% profit
                sell_portion: 0.25, // Sell another 25%
                time_threshold_seconds: 120, // Within 2 minutes
            },
            ProfitTarget {
                percentage: 100.0, // 100% profit
                sell_portion: 0.25, // Sell final 25%
                time_threshold_seconds: 300, // Within 5 minutes
            },
            // Tier 3: High profits - sell everything quickly
            ProfitTarget {
                percentage: 200.0, // 200% profit
                sell_portion: 1.0, // Sell remaining position
                time_threshold_seconds: 10, // Immediately
            },
            ProfitTarget {
                percentage: 500.0, // 500% profit
                sell_portion: 1.0, // Sell everything
                time_threshold_seconds: 5, // Immediately
            }
        ]
    }

    /// Update price history and detect momentum
    pub fn update_price_history(&mut self, token_address: &str, current_price: f64) {
        let now = Utc::now();

        let history = self.price_history.entry(token_address.to_string()).or_insert_with(Vec::new);

        // Add new price point
        history.push(PricePoint {
            price: current_price,
            timestamp: now,
        });

        // Keep only last 100 price points (last ~10 minutes if updated every 6 seconds)
        if history.len() > 100 {
            history.remove(0);
        }
    }

    /// Analyze position for fast profit-taking opportunities
    pub fn analyze_position_fast(
        &mut self,
        position: &Position,
        current_price: f64
    ) -> Vec<TradeSignal> {
        let mut signals = Vec::new();

        // Skip if position is not active
        if !matches!(position.status, PositionStatus::Active) {
            return signals;
        }

        // Update price history
        self.update_price_history(&position.token_address, current_price);

        let profit_targets = Self::get_profit_targets();
        let position_age_seconds = (Utc::now() - position.created_at).num_seconds() as u64;

        for target in profit_targets {
            // Check if we've hit the profit percentage
            if position.unrealized_pnl_percent >= target.percentage {
                // Check time constraints
                let should_sell_by_time = position_age_seconds <= target.time_threshold_seconds;
                let should_sell_by_momentum = self.detect_momentum_reversal(
                    &position.token_address
                );

                if should_sell_by_time || should_sell_by_momentum {
                    signals.push(TradeSignal {
                        token_address: position.token_address.clone(),
                        signal_type: TradeSignalType::FastProfit {
                            profit_percentage: target.percentage,
                            sell_portion: target.sell_portion,
                            reason: if should_sell_by_time {
                                "Time-based".to_string()
                            } else {
                                "Momentum reversal".to_string()
                            },
                        },
                        current_price,
                        trigger_price: position.average_buy_price *
                        (1.0 + target.percentage / 100.0),
                        timestamp: Utc::now(),
                        volume_24h: 0.0,
                        liquidity: 0.0,
                    });

                    // Only create one signal per analysis to avoid multiple sells
                    break;
                }
            }
        }

        // Emergency stop loss for very fast drops
        if position.unrealized_pnl_percent <= -15.0 && position_age_seconds <= 60 {
            signals.push(TradeSignal {
                token_address: position.token_address.clone(),
                signal_type: TradeSignalType::EmergencyStopLoss,
                current_price,
                trigger_price: position.average_buy_price * 0.85, // -15%
                timestamp: Utc::now(),
                volume_24h: 0.0,
                liquidity: 0.0,
            });
        }

        signals
    }

    /// Detect momentum reversal using price history
    pub fn detect_momentum_reversal(&self, token_address: &str) -> bool {
        if let Some(history) = self.price_history.get(token_address) {
            if history.len() < 5 {
                return false; // Not enough data
            }

            let recent_points = &history[history.len().saturating_sub(5)..];

            // Check if price is declining after reaching a peak
            let mut declining_count = 0;
            for i in 1..recent_points.len() {
                if recent_points[i].price < recent_points[i - 1].price {
                    declining_count += 1;
                }
            }

            // If 3 out of 4 recent price movements are declining, consider it a reversal
            declining_count >= 3
        } else {
            false
        }
    }

    /// Calculate optimal sell amount based on profit target
    pub fn calculate_fast_sell_amount(&self, position: &Position, sell_portion: f64) -> f64 {
        (position.total_tokens * sell_portion).max(0.0)
    }

    /// Check if position should use fast profit strategy
    pub fn should_use_fast_strategy(&self, position: &Position) -> bool {
        let position_age_minutes = (Utc::now() - position.created_at).num_minutes();

        // Use fast strategy for positions younger than 10 minutes
        position_age_minutes <= 10 && position.unrealized_pnl_percent >= 5.0 // Only if we're in profit
    }

    /// Get recommended price check interval based on position profit
    pub fn get_price_check_interval(&self, max_profit_percent: f64) -> u64 {
        if max_profit_percent >= 100.0 {
            1 // Check every second for high profit positions
        } else if max_profit_percent >= 50.0 {
            3 // Check every 3 seconds for medium profit
        } else if max_profit_percent >= 10.0 {
            6 // Check every 6 seconds for low profit
        } else {
            10 // Standard interval for break-even positions
        }
    }

    /// Check if position should trigger fast profit signals
    pub fn check_profit_targets(
        &self,
        position: &Position,
        current_price: f64
    ) -> Option<TradeSignal> {
        // Only process active positions
        if position.status != crate::trader::types::PositionStatus::Active {
            return None;
        }

        // Calculate current profit percentage
        let profit_percent = if position.average_buy_price > 0.0 {
            ((current_price - position.average_buy_price) / position.average_buy_price) * 100.0
        } else {
            return None;
        };

        // Check if we hit any profit targets
        for target in Self::get_profit_targets() {
            if profit_percent >= target.percentage {
                // Check time constraint
                let position_age = chrono::Utc
                    ::now()
                    .signed_duration_since(position.created_at)
                    .num_seconds() as u64;

                if position_age <= target.time_threshold_seconds {
                    return Some(TradeSignal {
                        token_address: position.token_address.clone(),
                        signal_type: TradeSignalType::FastProfit {
                            profit_percentage: profit_percent,
                            sell_portion: target.sell_portion,
                            reason: format!(
                                "Fast profit {}% reached in {} seconds",
                                target.percentage,
                                position_age
                            ),
                        },
                        current_price,
                        trigger_price: position.average_buy_price *
                        (1.0 + target.percentage / 100.0),
                        timestamp: chrono::Utc::now(),
                        volume_24h: 0.0, // Will be updated by calling system
                        liquidity: 0.0, // Will be updated by calling system
                    });
                }
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::TraderConfig;

    fn create_test_config() -> TraderConfig {
        TraderConfig {
            enabled: true,
            dry_run: false,
            trade_size_sol: 0.01,
            buy_trigger_percent: -5.0,
            sell_trigger_percent: 10.0,
            stop_loss_percent: -50.0,
            dca_enabled: true,
            dca_min_loss_percent: -20.0,
            dca_max_loss_percent: -50.0,
            dca_levels: 3,
            max_positions: 10,
            position_check_interval_seconds: 10,
            price_check_interval_seconds: 5,
            database_path: "test.db".to_string(),
            rug_detection: Default::default(),
            fast_profit_enabled: true,
            profit_targets: vec![],
            momentum_check_seconds: 5,
        }
    }

    #[test]
    fn test_profit_targets() {
        let targets = FastProfitStrategy::get_profit_targets();
        assert_eq!(targets.len(), 6);
        assert_eq!(targets[0].percentage, 10.0);
        assert_eq!(targets.last().unwrap().percentage, 500.0);
    }

    #[test]
    fn test_fast_strategy_detection() {
        let config = create_test_config();
        let strategy = FastProfitStrategy::new(config);

        let mut position = Position::new("test_token".to_string(), "TEST".to_string());
        position.unrealized_pnl_percent = 15.0;

        assert!(strategy.should_use_fast_strategy(&position));
    }
}
