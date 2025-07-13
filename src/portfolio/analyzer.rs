use crate::core::{ BotResult, Position, PortfolioHealth, RebalanceRecommendation, RebalanceAction };
use chrono::{ Utc, Duration };
use std::collections::HashMap;

/// Portfolio analyzer for performance metrics and recommendations
pub struct PortfolioAnalyzer {
    min_position_value: f64,
    max_position_percentage: f64,
    dca_threshold: f64,
    profit_taking_threshold: f64,
}

impl PortfolioAnalyzer {
    pub fn new() -> Self {
        Self {
            min_position_value: 0.01, // 0.01 SOL minimum position
            max_position_percentage: 20.0, // 20% max allocation to any single token
            dca_threshold: 15.0, // DCA when down 15%
            profit_taking_threshold: 50.0, // Take profits at 50% gain
        }
    }

    /// Analyze overall portfolio health
    pub fn analyze_portfolio_health(&self, positions: &[Position]) -> PortfolioHealth {
        if positions.is_empty() {
            return PortfolioHealth {
                total_value_sol: 0.0,
                total_invested_sol: 0.0,
                total_unrealized_pnl: 0.0,
                total_pnl_percentage: 0.0,
                positions_count: 0,
                profitable_positions: 0,
                losing_positions: 0,
                largest_position_percentage: 0.0,
                portfolio_concentration_risk: "Low".to_string(),
                health_score: 100,
                recommendations: Vec::new(),
            };
        }

        // Calculate totals
        let total_value_sol: f64 = positions
            .iter()
            .map(|p| p.current_value_sol)
            .sum();
        let total_invested_sol: f64 = positions
            .iter()
            .map(|p| p.total_invested_sol)
            .sum();
        let total_unrealized_pnl: f64 = positions
            .iter()
            .map(|p| p.unrealized_pnl)
            .sum();

        let total_pnl_percentage = if total_invested_sol > 0.0 {
            (total_unrealized_pnl / total_invested_sol) * 100.0
        } else {
            0.0
        };

        // Count profitable vs losing positions
        let profitable_positions = positions
            .iter()
            .filter(|p| p.unrealized_pnl > 0.0)
            .count();
        let losing_positions = positions
            .iter()
            .filter(|p| p.unrealized_pnl < 0.0)
            .count();

        // Calculate concentration risk
        let largest_position_value = positions
            .iter()
            .map(|p| p.current_value_sol)
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap_or(0.0);

        let largest_position_percentage = if total_value_sol > 0.0 {
            (largest_position_value / total_value_sol) * 100.0
        } else {
            0.0
        };

        // Determine concentration risk level
        let concentration_risk = if largest_position_percentage > 50.0 {
            "Very High".to_string()
        } else if largest_position_percentage > 30.0 {
            "High".to_string()
        } else if largest_position_percentage > 20.0 {
            "Medium".to_string()
        } else {
            "Low".to_string()
        };

        // Calculate health score (0-100)
        let health_score = self.calculate_health_score(
            total_pnl_percentage,
            largest_position_percentage,
            ((profitable_positions as f64) / (positions.len() as f64)) * 100.0
        );

        // Generate recommendations
        let recommendations = self.generate_portfolio_recommendations(
            positions,
            &concentration_risk,
            total_pnl_percentage
        );

        PortfolioHealth {
            total_value_sol,
            total_invested_sol,
            total_unrealized_pnl,
            total_pnl_percentage,
            positions_count: positions.len(),
            profitable_positions,
            losing_positions,
            largest_position_percentage,
            portfolio_concentration_risk: concentration_risk,
            health_score,
            recommendations,
        }
    }

    /// Calculate portfolio health score (0-100)
    fn calculate_health_score(&self, pnl_percentage: f64, concentration: f64, win_rate: f64) -> u8 {
        let mut score = 50.0; // Base score

        // PnL impact (±30 points)
        if pnl_percentage > 20.0 {
            score += 30.0;
        } else if pnl_percentage > 0.0 {
            score += (pnl_percentage / 20.0) * 30.0;
        } else if pnl_percentage > -20.0 {
            score += (pnl_percentage / 20.0) * 30.0; // This will be negative
        } else {
            score -= 30.0;
        }

        // Concentration risk impact (±15 points)
        if concentration < 20.0 {
            score += 15.0;
        } else if concentration < 50.0 {
            score += 15.0 - ((concentration - 20.0) / 30.0) * 15.0;
        } else {
            score -= 15.0;
        }

        // Win rate impact (±5 points)
        if win_rate > 60.0 {
            score += 5.0;
        } else if win_rate > 40.0 {
            score += ((win_rate - 40.0) / 20.0) * 5.0;
        } else {
            score -= 5.0;
        }

        // Clamp to 0-100
        score.max(0.0).min(100.0) as u8
    }

    /// Generate portfolio-level recommendations
    fn generate_portfolio_recommendations(
        &self,
        positions: &[Position],
        concentration_risk: &str,
        pnl_percentage: f64
    ) -> Vec<String> {
        let mut recommendations = Vec::new();

        // Concentration risk recommendations
        if concentration_risk == "Very High" || concentration_risk == "High" {
            recommendations.push(
                "🚨 Consider reducing position sizes to decrease concentration risk".to_string()
            );
        }

        // Overall performance recommendations
        if pnl_percentage < -20.0 {
            recommendations.push(
                "📉 Portfolio is significantly down. Consider reviewing trading strategy".to_string()
            );
        } else if pnl_percentage > 50.0 {
            recommendations.push(
                "💰 Strong portfolio performance! Consider taking some profits".to_string()
            );
        }

        // Position count recommendations
        if positions.len() > 10 {
            recommendations.push(
                "📊 Large number of positions. Consider consolidating for better management".to_string()
            );
        } else if positions.len() < 3 {
            recommendations.push("🎯 Consider diversifying with additional positions".to_string());
        }

        recommendations
    }

    /// Generate rebalancing recommendations
    pub async fn generate_rebalance_recommendations(
        &self,
        positions: &[Position]
    ) -> BotResult<Vec<RebalanceRecommendation>> {
        let mut recommendations = Vec::new();

        if positions.is_empty() {
            return Ok(recommendations);
        }

        let total_value: f64 = positions
            .iter()
            .map(|p| p.current_value_sol)
            .sum();

        for position in positions {
            let position_percentage = if total_value > 0.0 {
                (position.current_value_sol / total_value) * 100.0
            } else {
                0.0
            };

            // Check for position sizing issues
            if position.current_value_sol < self.min_position_value {
                recommendations.push(RebalanceRecommendation {
                    token: position.token,
                    symbol: position.symbol.clone(),
                    action: RebalanceAction::Close,
                    reason: "Position too small to manage effectively".to_string(),
                    current_percentage: position_percentage,
                    target_percentage: 0.0,
                    amount_sol: position.current_value_sol,
                    priority: (
                        if position.unrealized_pnl < 0.0 {
                            "High"
                        } else {
                            "Medium"
                        }
                    ).to_string(),
                });
                continue;
            }

            // Check for overconcentration
            if position_percentage > self.max_position_percentage {
                let target_percentage = self.max_position_percentage;
                let excess_percentage = position_percentage - target_percentage;
                let reduce_amount = (excess_percentage / 100.0) * total_value;

                recommendations.push(RebalanceRecommendation {
                    token: position.token,
                    symbol: position.symbol.clone(),
                    action: RebalanceAction::Reduce,
                    reason: format!(
                        "Position exceeds maximum allocation of {}%",
                        self.max_position_percentage
                    ),
                    current_percentage: position_percentage,
                    target_percentage,
                    amount_sol: reduce_amount,
                    priority: "High".to_string(),
                });
                continue;
            }

            // Check for DCA opportunities
            if position.unrealized_pnl_percentage < -self.dca_threshold {
                let dca_amount = position.total_invested_sol * 0.2; // 20% of original investment

                recommendations.push(RebalanceRecommendation {
                    token: position.token,
                    symbol: position.symbol.clone(),
                    action: RebalanceAction::DCA,
                    reason: format!(
                        "Position down {:.1}% - DCA opportunity",
                        position.unrealized_pnl_percentage.abs()
                    ),
                    current_percentage: position_percentage,
                    target_percentage: position_percentage,
                    amount_sol: dca_amount,
                    priority: (
                        if position.unrealized_pnl_percentage < -30.0 {
                            "High"
                        } else {
                            "Medium"
                        }
                    ).to_string(),
                });
            }

            // Check for profit taking opportunities
            if position.unrealized_pnl_percentage > self.profit_taking_threshold {
                let profit_take_amount = position.unrealized_pnl * 0.5; // Take 50% of profits

                recommendations.push(RebalanceRecommendation {
                    token: position.token,
                    symbol: position.symbol.clone(),
                    action: RebalanceAction::TakeProfit,
                    reason: format!(
                        "Position up {:.1}% - profit taking opportunity",
                        position.unrealized_pnl_percentage
                    ),
                    current_percentage: position_percentage,
                    target_percentage: position_percentage,
                    amount_sol: profit_take_amount,
                    priority: (
                        if position.unrealized_pnl_percentage > 100.0 {
                            "High"
                        } else {
                            "Medium"
                        }
                    ).to_string(),
                });
            }
        }

        // Sort by priority (High first)
        recommendations.sort_by(|a, b| {
            match (a.priority.as_str(), b.priority.as_str()) {
                ("High", "Medium") => std::cmp::Ordering::Less,
                ("Medium", "High") => std::cmp::Ordering::Greater,
                _ => std::cmp::Ordering::Equal,
            }
        });

        Ok(recommendations)
    }

    /// Analyze individual position performance
    pub fn analyze_position_performance(&self, position: &Position) -> PositionAnalysis {
        let days_held = (Utc::now() - position.first_buy_time).num_days() as f64;
        let annualized_return = if days_held > 0.0 {
            (position.unrealized_pnl_percentage / days_held) * 365.0
        } else {
            0.0
        };

        // Determine position status
        let status = if position.unrealized_pnl_percentage > 20.0 {
            "Strong Winner".to_string()
        } else if position.unrealized_pnl_percentage > 0.0 {
            "Winner".to_string()
        } else if position.unrealized_pnl_percentage > -20.0 {
            "Underperforming".to_string()
        } else {
            "Significant Loss".to_string()
        };

        // Risk assessment
        let risk_level = if position.current_value_sol > 1.0 {
            "High"
        } else if position.current_value_sol > 0.1 {
            "Medium"
        } else {
            "Low"
        };

        PositionAnalysis {
            days_held,
            annualized_return,
            status,
            risk_level: risk_level.to_string(),
            should_dca: position.unrealized_pnl_percentage < -self.dca_threshold,
            should_take_profit: position.unrealized_pnl_percentage > self.profit_taking_threshold,
            trade_frequency_score: self.calculate_trade_frequency_score(position),
        }
    }

    /// Calculate trade frequency score (how active trading has been)
    fn calculate_trade_frequency_score(&self, position: &Position) -> f64 {
        let days_held = (Utc::now() - position.first_buy_time).num_days() as f64;
        if days_held <= 0.0 {
            return 0.0;
        }

        // Trades per week
        ((position.trade_count as f64) / days_held) * 7.0
    }

    /// Find positions that complement each other (diversification analysis)
    pub fn analyze_diversification(&self, positions: &[Position]) -> DiversificationAnalysis {
        // This would analyze correlation between positions
        // For now, return basic metrics
        let unique_tokens = positions.len();
        let total_value: f64 = positions
            .iter()
            .map(|p| p.current_value_sol)
            .sum();

        // Calculate Herfindahl-Hirschman Index for concentration
        let hhi: f64 = positions
            .iter()
            .map(|p| {
                let percentage = if total_value > 0.0 {
                    p.current_value_sol / total_value
                } else {
                    0.0
                };
                percentage * percentage
            })
            .sum();

        let diversification_score = ((1.0 - hhi) * 100.0).max(0.0).min(100.0);

        DiversificationAnalysis {
            unique_positions: unique_tokens,
            herfindahl_index: hhi,
            diversification_score: diversification_score as u8,
            concentration_risk: (
                if hhi > 0.25 {
                    "High"
                } else if hhi > 0.15 {
                    "Medium"
                } else {
                    "Low"
                }
            ).to_string(),
        }
    }
}

/// Individual position analysis result
#[derive(Debug, Clone)]
pub struct PositionAnalysis {
    pub days_held: f64,
    pub annualized_return: f64,
    pub status: String,
    pub risk_level: String,
    pub should_dca: bool,
    pub should_take_profit: bool,
    pub trade_frequency_score: f64,
}

/// Portfolio diversification analysis
#[derive(Debug, Clone)]
pub struct DiversificationAnalysis {
    pub unique_positions: usize,
    pub herfindahl_index: f64,
    pub diversification_score: u8,
    pub concentration_risk: String,
}
