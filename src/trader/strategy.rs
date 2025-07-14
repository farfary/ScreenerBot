use crate::core::{ BotResult, TraderConfig, TradeSignal, SignalType, TokenOpportunity, RiskLevel };
use std::collections::HashMap;
use chrono::{ Utc, Duration };

/// Trading strategy implementation
#[derive(Debug)]
pub struct TradingStrategy {
    config: TraderConfig,
}

impl TradingStrategy {
    pub fn new(config: &TraderConfig) -> Self {
        Self {
            config: config.clone(),
        }
    }

    /// Generate a trade signal based on opportunity analysis
    pub async fn generate_signal(
        &self,
        opportunity: &TokenOpportunity,
        analysis: &crate::trader::TradeAnalysis
    ) -> BotResult<Option<TradeSignal>> {
        // Calculate signal strength based on multiple factors
        let signal_strength = self.calculate_signal_strength(opportunity, analysis);

        // Only generate signals for strong enough opportunities
        if signal_strength < 0.5 {
            log::debug!(
                "🔸 Signal strength too low for {}: {:.2}",
                opportunity.symbol,
                signal_strength
            );
            return Ok(None);
        }

        // Determine signal type (for new positions, it's always Buy)
        let signal_type = SignalType::Buy;

        // Calculate recommended amount
        let recommended_amount = self.calculate_position_size(opportunity, signal_strength);

        // Create the trade signal
        let signal = TradeSignal {
            token: opportunity.mint,
            signal_type,
            strength: signal_strength,
            recommended_amount,
            max_slippage: self.config.max_slippage,
            generated_at: Utc::now(),
            valid_until: Utc::now() + Duration::minutes(30), // Signal valid for 30 minutes
            analysis_data: analysis.clone(),
        };

        log::info!(
            "📊 Generated {} signal for {}: strength {:.2}, amount {} SOL",
            match signal_type {
                SignalType::Buy => "BUY",
                SignalType::Sell => "SELL",
                SignalType::DCA => "DCA",
                SignalType::Hold => "HOLD",
            },
            opportunity.symbol,
            signal_strength,
            recommended_amount
        );

        Ok(Some(signal))
    }

    /// Calculate signal strength based on opportunity metrics and analysis
    fn calculate_signal_strength(
        &self,
        opportunity: &TokenOpportunity,
        analysis: &crate::trader::TradeAnalysis
    ) -> f64 {
        let mut strength_factors = Vec::new();

        // Base confidence and risk scores
        strength_factors.push(opportunity.confidence_score);
        strength_factors.push(1.0 - opportunity.risk_score); // Invert risk score

        // Fundamental analysis score
        strength_factors.push(analysis.fundamental_score);

        // Technical indicators contribution
        let technical_score = self.evaluate_technical_indicators(&analysis.technical_indicators);
        strength_factors.push(technical_score);

        // Liquidity factor
        let liquidity_factor = if opportunity.metrics.liquidity_usd > 100000.0 {
            0.9
        } else if opportunity.metrics.liquidity_usd > 50000.0 {
            0.8
        } else if opportunity.metrics.liquidity_usd > 20000.0 {
            0.6
        } else {
            0.4
        };
        strength_factors.push(liquidity_factor);

        // Volume factor
        let volume_factor = if opportunity.metrics.volume_24h > 50000.0 {
            0.9
        } else if opportunity.metrics.volume_24h > 20000.0 {
            0.8
        } else if opportunity.metrics.volume_24h > 5000.0 {
            0.6
        } else {
            0.4
        };
        strength_factors.push(volume_factor);

        // Age factor (newer tokens get slightly lower strength)
        let age_factor = if opportunity.metrics.age_hours > 24.0 {
            0.8
        } else if opportunity.metrics.age_hours > 6.0 {
            0.7
        } else {
            0.6
        };
        strength_factors.push(age_factor);

        // Calculate weighted average
        let total_strength: f64 = strength_factors.iter().sum();
        let average_strength = total_strength / (strength_factors.len() as f64);

        average_strength.clamp(0.0, 1.0)
    }

    /// Evaluate technical indicators and return a score
    fn evaluate_technical_indicators(&self, indicators: &HashMap<String, f64>) -> f64 {
        let mut score: f64 = 0.5; // Base score

        // Risk-adjusted potential
        if let Some(&potential) = indicators.get("risk_adjusted_potential") {
            if potential > 2.0 {
                score += 0.3;
            } else if potential > 1.5 {
                score += 0.2;
            } else if potential > 1.0 {
                score += 0.1;
            }
        }

        // Liquidity to volume ratio
        if let Some(&lv_ratio) = indicators.get("liquidity_volume_ratio") {
            if lv_ratio > 1.0 && lv_ratio < 10.0 {
                score += 0.1; // Good liquidity relative to volume
            }
        }

        // Price momentum
        if let Some(&momentum) = indicators.get("price_momentum") {
            if momentum > 0.0 && momentum < 50.0 {
                score += 0.1; // Positive but not excessive momentum
            } else if momentum < -20.0 {
                score -= 0.1; // Negative momentum
            }
        }

        score.clamp(0.0, 1.0)
    }

    /// Calculate position size based on signal strength and risk management
    fn calculate_position_size(&self, opportunity: &TokenOpportunity, signal_strength: f64) -> f64 {
        let base_amount = self.config.entry_amount_sol;

        // Adjust size based on signal strength
        let strength_multiplier = 0.5 + signal_strength; // Range: 0.5 to 1.5

        // Adjust size based on risk level
        let risk_multiplier = match self.determine_risk_level(opportunity) {
            RiskLevel::Low => 1.2,
            RiskLevel::Medium => 1.0,
            RiskLevel::High => 0.7,
            RiskLevel::Critical => 0.5,
        };

        // Adjust size based on liquidity
        let liquidity_multiplier = if opportunity.metrics.liquidity_usd > 200000.0 {
            1.2
        } else if opportunity.metrics.liquidity_usd > 100000.0 {
            1.0
        } else if opportunity.metrics.liquidity_usd > 50000.0 {
            0.8
        } else {
            0.6
        };

        let final_amount =
            base_amount * strength_multiplier * risk_multiplier * liquidity_multiplier;

        // Ensure we don't exceed maximum per trade
        final_amount.min(self.config.entry_amount_sol * 2.0)
    }

    /// Determine overall risk level for an opportunity
    fn determine_risk_level(&self, opportunity: &TokenOpportunity) -> RiskLevel {
        if opportunity.risk_score > 0.8 {
            RiskLevel::Critical
        } else if opportunity.risk_score > 0.6 {
            RiskLevel::High
        } else if opportunity.risk_score > 0.4 {
            RiskLevel::Medium
        } else {
            RiskLevel::Low
        }
    }

    /// Check if conditions are favorable for trading
    pub fn is_market_favorable(&self, _market_conditions: &MarketConditions) -> bool {
        // Simple implementation - can be enhanced with more sophisticated market analysis
        true
    }

    /// Generate exit strategy for a position
    pub fn generate_exit_strategy(&self, _position: &crate::core::Position) -> ExitStrategy {
        ExitStrategy {
            take_profit_percentage: self.config.take_profit_percentage,
            stop_loss_percentage: if self.config.stop_loss_enabled {
                Some(self.config.stop_loss_percentage)
            } else {
                None
            },
            trailing_stop: false, // Can be enhanced later
            time_based_exit: None,
        }
    }
}

/// Market conditions for strategy decisions
#[derive(Debug)]
pub struct MarketConditions {
    pub overall_trend: MarketTrend,
    pub volatility_level: f64,
    pub volume_trend: f64,
}

#[derive(Debug)]
pub enum MarketTrend {
    Bullish,
    Bearish,
    Sideways,
}

/// Exit strategy for positions
#[derive(Debug)]
pub struct ExitStrategy {
    pub take_profit_percentage: f64,
    pub stop_loss_percentage: Option<f64>,
    pub trailing_stop: bool,
    pub time_based_exit: Option<chrono::Duration>,
}
