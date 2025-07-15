use crate::config::TimeProfitConfig;
use crate::logger::Logger;
use crate::types::{ TradingPosition, TimeCategory, SignalType };
use anyhow::Result;
use chrono::Utc;

/// Manages time-based profit strategies and position evaluation
pub struct ProfitStrategy {
    config: TimeProfitConfig,
}

impl ProfitStrategy {
    pub fn new(config: TimeProfitConfig) -> Self {
        Self { config }
    }

    /// Evaluates a position and determines if it should be closed based on time and profit targets
    pub async fn evaluate_position(
        &self,
        position: &TradingPosition
    ) -> Result<Option<SignalType>> {
        if !self.config.enabled {
            return Ok(None);
        }

        let elapsed = Utc::now().signed_duration_since(position.opened_at);
        let pnl_percentage = position.pnl_percentage;

        // Never sell at a loss greater than -70%
        if pnl_percentage <= -70.0 {
            Logger::trader(
                &format!(
                    "⚠️  Position {} at -70% loss, but HODLING as per strategy",
                    position.token_mint
                )
            );
            return Ok(Some(SignalType::Hold));
        }

        // Time-based profit targets
        let target_profit = match position.time_category {
            TimeCategory::Quick => {
                if elapsed.num_minutes() >= (self.config.quick_profit_threshold_mins as i64) {
                    Some(self.config.quick_profit_target)
                } else {
                    None
                }
            }
            TimeCategory::Medium => {
                if elapsed.num_hours() >= (self.config.medium_profit_threshold_hours as i64) {
                    Some(self.config.medium_profit_target)
                } else {
                    None
                }
            }
            TimeCategory::Long => {
                if elapsed.num_hours() >= (self.config.long_profit_threshold_hours as i64) {
                    Some(self.config.long_profit_target)
                } else {
                    None
                }
            }
            TimeCategory::Extended => {
                // For extended positions, use minimum profit target
                Some(3.0)
            }
        };

        if let Some(target) = target_profit {
            if pnl_percentage >= target {
                Logger::trader(
                    &format!(
                        "🎯 Position {} reached target profit {:.2}% (current: {:.2}%)",
                        position.token_mint,
                        target,
                        pnl_percentage
                    )
                );
                return Ok(Some(SignalType::Sell));
            }
        }

        // Check for exceptional profits (100x in seconds/minutes)
        if elapsed.num_minutes() < 5 && pnl_percentage >= 10000.0 {
            Logger::trader(
                &format!(
                    "🚀 MEGA PROFIT! Position {} gained {:.2}% in {} minutes",
                    position.token_mint,
                    pnl_percentage,
                    elapsed.num_minutes()
                )
            );
            return Ok(Some(SignalType::Sell));
        }

        // Check for quick 1x (100%) profits
        if elapsed.num_minutes() < 60 && pnl_percentage >= 100.0 {
            Logger::trader(
                &format!(
                    "💰 Quick 100%+ profit! Position {} gained {:.2}% in {} minutes",
                    position.token_mint,
                    pnl_percentage,
                    elapsed.num_minutes()
                )
            );
            return Ok(Some(SignalType::Sell));
        }

        // Always take profit at minimum 3% if we're past the long threshold
        if elapsed.num_hours() >= 24 && pnl_percentage >= 3.0 {
            Logger::trader(
                &format!(
                    "⏰ Time-based exit: Position {} held for 24+ hours with {:.2}% profit",
                    position.token_mint,
                    pnl_percentage
                )
            );
            return Ok(Some(SignalType::Sell));
        }

        // Default: hold the position
        Ok(Some(SignalType::Hold))
    }

    /// Get the appropriate profit target for a position based on its age
    pub fn get_profit_target(&self, position: &TradingPosition) -> f64 {
        let elapsed = Utc::now().signed_duration_since(position.opened_at);

        if elapsed.num_minutes() < (self.config.quick_profit_threshold_mins as i64) {
            self.config.quick_profit_target
        } else if elapsed.num_hours() < (self.config.medium_profit_threshold_hours as i64) {
            self.config.medium_profit_target
        } else if elapsed.num_hours() < (self.config.long_profit_threshold_hours as i64) {
            self.config.long_profit_target
        } else {
            3.0 // Minimum profit for extended positions
        }
    }

    /// Check if a position qualifies for "fast money" - quick high profits
    pub fn is_fast_money_opportunity(&self, position: &TradingPosition) -> bool {
        let elapsed = Utc::now().signed_duration_since(position.opened_at);
        let pnl_percentage = position.pnl_percentage;

        // 100x in seconds to minutes
        if elapsed.num_minutes() < 10 && pnl_percentage >= 10000.0 {
            return true;
        }

        // 10x in minutes
        if elapsed.num_minutes() < 30 && pnl_percentage >= 1000.0 {
            return true;
        }

        // 1x (100%) in under an hour
        if elapsed.num_hours() < 1 && pnl_percentage >= 100.0 {
            return true;
        }

        false
    }

    /// Calculate dynamic profit target based on market conditions and position performance
    pub fn calculate_dynamic_target(&self, position: &TradingPosition) -> f64 {
        let base_target = self.get_profit_target(position);
        let elapsed = Utc::now().signed_duration_since(position.opened_at);
        let pnl_percentage = position.pnl_percentage;

        // If we're already in significant profit, be more aggressive
        if pnl_percentage > 500.0 {
            // If we're up 500%+, take some profit but let it run
            return base_target * 0.5;
        } else if pnl_percentage > 100.0 {
            // If we're up 100%+, be more conservative
            return base_target * 0.8;
        } else if pnl_percentage > 50.0 {
            // If we're up 50%+, stick to plan
            return base_target;
        } else {
            // If we're not in significant profit yet, be more patient
            return base_target * 1.5;
        }
    }

    /// Log current strategy status for a position
    pub fn log_strategy_status(&self, position: &TradingPosition) {
        let elapsed = Utc::now().signed_duration_since(position.opened_at);
        let target = self.get_profit_target(position);
        let dynamic_target = self.calculate_dynamic_target(position);

        Logger::trader(
            &format!(
                "📊 Strategy Status - {} | Age: {}h {}m | P&L: {:.2}% | Target: {:.1}% | Dynamic: {:.1}%",
                position.token_mint,
                elapsed.num_hours(),
                elapsed.num_minutes() % 60,
                position.pnl_percentage,
                target,
                dynamic_target
            )
        );
    }
}
