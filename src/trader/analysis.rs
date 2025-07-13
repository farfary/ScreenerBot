use crate::core::{
    BotResult,
    BotError,
    TokenOpportunity,
    TradeAnalysis,
    RiskAssessment,
    RiskLevel,
    Portfolio,
};
use std::collections::HashMap;

/// Trade analysis engine
pub struct TradeAnalyzer {}

impl TradeAnalyzer {
    pub fn new() -> Self {
        Self {}
    }

    /// Analyze a token opportunity for trading
    pub async fn analyze_opportunity(
        &self,
        opportunity: &TokenOpportunity,
        portfolio: &Portfolio
    ) -> BotResult<TradeAnalysis> {
        // Generate technical indicators
        let technical_indicators = self.generate_technical_indicators(opportunity, portfolio);

        // Calculate fundamental score
        let fundamental_score = self.calculate_fundamental_score(opportunity);

        // Assess risks
        let risk_assessment = self.assess_risks(opportunity, portfolio);

        // Calculate expected return
        let expected_return = self.calculate_expected_return(opportunity);

        // Determine time horizon
        let time_horizon = self.determine_time_horizon(opportunity);

        // Generate entry reason
        let entry_reason = self.generate_entry_reason(opportunity, &technical_indicators);

        Ok(TradeAnalysis {
            entry_reason,
            technical_indicators,
            fundamental_score,
            risk_assessment,
            expected_return,
            time_horizon,
        })
    }

    /// Generate technical indicators for analysis
    fn generate_technical_indicators(
        &self,
        opportunity: &TokenOpportunity,
        portfolio: &Portfolio
    ) -> HashMap<String, f64> {
        let mut indicators = HashMap::new();

        // Liquidity to Volume ratio
        let lv_ratio = if opportunity.metrics.volume_24h > 0.0 {
            opportunity.metrics.liquidity_usd / opportunity.metrics.volume_24h
        } else {
            0.0
        };
        indicators.insert("liquidity_volume_ratio".to_string(), lv_ratio);

        // Risk-adjusted potential
        let risk_adjusted = opportunity.confidence_score / (opportunity.risk_score + 0.1);
        indicators.insert("risk_adjusted_potential".to_string(), risk_adjusted);

        // Price momentum (if available)
        if let Some(change) = opportunity.metrics.price_change_24h {
            indicators.insert("price_momentum".to_string(), change);

            // Momentum strength
            let momentum_strength = if change.abs() > 20.0 {
                1.0
            } else if change.abs() > 10.0 {
                0.7
            } else if change.abs() > 5.0 {
                0.5
            } else {
                0.3
            };
            indicators.insert("momentum_strength".to_string(), momentum_strength);
        }

        // Market cap efficiency
        if let Some(mcap) = opportunity.metrics.market_cap {
            if mcap > 0.0 {
                let efficiency = opportunity.metrics.volume_24h / mcap;
                indicators.insert("market_cap_efficiency".to_string(), efficiency);
            }
        }

        // Liquidity depth score
        let liquidity_score = if opportunity.metrics.liquidity_usd > 500000.0 {
            1.0
        } else if opportunity.metrics.liquidity_usd > 200000.0 {
            0.8
        } else if opportunity.metrics.liquidity_usd > 100000.0 {
            0.6
        } else if opportunity.metrics.liquidity_usd > 50000.0 {
            0.4
        } else {
            0.2
        };
        indicators.insert("liquidity_depth_score".to_string(), liquidity_score);

        // Volume consistency (simplified)
        let volume_score = if opportunity.metrics.volume_24h > 100000.0 {
            1.0
        } else if opportunity.metrics.volume_24h > 50000.0 {
            0.8
        } else if opportunity.metrics.volume_24h > 20000.0 {
            0.6
        } else if opportunity.metrics.volume_24h > 5000.0 {
            0.4
        } else {
            0.2
        };
        indicators.insert("volume_consistency_score".to_string(), volume_score);

        // Portfolio correlation (avoid overconcentration)
        let correlation_risk = self.calculate_portfolio_correlation(opportunity, portfolio);
        indicators.insert("portfolio_correlation_risk".to_string(), correlation_risk);

        // Age factor
        let age_factor = if opportunity.metrics.age_hours > 72.0 {
            0.8 // Mature
        } else if opportunity.metrics.age_hours > 24.0 {
            0.9 // Established
        } else if opportunity.metrics.age_hours > 6.0 {
            0.6 // New but stable
        } else {
            0.3 // Very new
        };
        indicators.insert("age_stability_factor".to_string(), age_factor);

        indicators
    }

    /// Calculate fundamental score based on token metrics
    fn calculate_fundamental_score(&self, opportunity: &TokenOpportunity) -> f64 {
        let mut score_factors = Vec::new();

        // Liquidity factor
        let liquidity_factor = if opportunity.metrics.liquidity_usd > 1000000.0 {
            1.0
        } else if opportunity.metrics.liquidity_usd > 500000.0 {
            0.9
        } else if opportunity.metrics.liquidity_usd > 200000.0 {
            0.8
        } else if opportunity.metrics.liquidity_usd > 100000.0 {
            0.6
        } else if opportunity.metrics.liquidity_usd > 50000.0 {
            0.4
        } else {
            0.2
        };
        score_factors.push(liquidity_factor);

        // Volume factor
        let volume_factor = if opportunity.metrics.volume_24h > 200000.0 {
            1.0
        } else if opportunity.metrics.volume_24h > 100000.0 {
            0.9
        } else if opportunity.metrics.volume_24h > 50000.0 {
            0.8
        } else if opportunity.metrics.volume_24h > 20000.0 {
            0.6
        } else if opportunity.metrics.volume_24h > 5000.0 {
            0.4
        } else {
            0.2
        };
        score_factors.push(volume_factor);

        // Verification factor
        let verification_factor = if opportunity.verification_status.is_verified {
            1.0
        } else if opportunity.verification_status.has_profile {
            0.7
        } else {
            0.4
        };
        score_factors.push(verification_factor);

        // Market cap factor (prefer mid-cap)
        let mcap_factor = if let Some(mcap) = opportunity.metrics.market_cap {
            if mcap > 1000000.0 && mcap < 100000000.0 {
                1.0 // Sweet spot
            } else if mcap > 100000.0 && mcap < 1000000000.0 {
                0.8
            } else {
                0.5
            }
        } else {
            0.6 // Unknown market cap
        };
        score_factors.push(mcap_factor);

        // Security factor
        let security_factor = if opportunity.verification_status.security_flags.is_empty() {
            1.0
        } else if opportunity.verification_status.security_flags.len() <= 2 {
            0.6
        } else {
            0.2
        };
        score_factors.push(security_factor);

        // Calculate weighted average
        let total_score: f64 = score_factors.iter().sum();
        (total_score / (score_factors.len() as f64)).clamp(0.0, 1.0)
    }

    /// Assess various risk factors
    fn assess_risks(
        &self,
        opportunity: &TokenOpportunity,
        portfolio: &Portfolio
    ) -> RiskAssessment {
        // Liquidity risk
        let liquidity_risk = if opportunity.metrics.liquidity_usd < 50000.0 {
            RiskLevel::Critical
        } else if opportunity.metrics.liquidity_usd < 200000.0 {
            RiskLevel::High
        } else if opportunity.metrics.liquidity_usd < 500000.0 {
            RiskLevel::Medium
        } else {
            RiskLevel::Low
        };

        // Volatility risk
        let volatility_risk = if let Some(change) = opportunity.metrics.price_change_24h {
            if change.abs() > 100.0 {
                RiskLevel::Critical
            } else if change.abs() > 50.0 {
                RiskLevel::High
            } else if change.abs() > 20.0 {
                RiskLevel::Medium
            } else {
                RiskLevel::Low
            }
        } else {
            RiskLevel::Medium
        };

        // Concentration risk
        let concentration_risk = if portfolio.positions.len() >= 40 {
            RiskLevel::High
        } else if portfolio.positions.len() >= 30 {
            RiskLevel::Medium
        } else {
            RiskLevel::Low
        };

        // Smart money risk (simplified)
        let smart_money_risk = if opportunity.verification_status.security_flags.len() > 2 {
            RiskLevel::High
        } else if opportunity.verification_status.security_flags.len() > 0 {
            RiskLevel::Medium
        } else {
            RiskLevel::Low
        };

        // Overall risk
        let overall_risk = match (liquidity_risk, volatility_risk) {
            (RiskLevel::Critical, _) | (_, RiskLevel::Critical) => RiskLevel::Critical,
            (RiskLevel::High, RiskLevel::High) => RiskLevel::Critical,
            (RiskLevel::High, _) | (_, RiskLevel::High) => RiskLevel::High,
            (RiskLevel::Medium, RiskLevel::Medium) => RiskLevel::Medium,
            _ => RiskLevel::Low,
        };

        RiskAssessment {
            overall_risk,
            liquidity_risk,
            volatility_risk,
            concentration_risk,
            smart_money_risk,
        }
    }

    /// Calculate expected return based on opportunity metrics
    fn calculate_expected_return(&self, opportunity: &TokenOpportunity) -> f64 {
        let mut return_factors = Vec::new();

        // Base return expectation based on confidence
        return_factors.push(opportunity.confidence_score * 30.0); // Up to 30% base return

        // Momentum contribution
        if let Some(momentum) = opportunity.metrics.price_change_24h {
            if momentum > 0.0 && momentum < 50.0 {
                return_factors.push(10.0); // Positive momentum bonus
            } else if momentum < 0.0 && momentum > -20.0 {
                return_factors.push(15.0); // Potential reversal opportunity
            }
        }

        // Liquidity contribution
        if opportunity.metrics.liquidity_usd > 200000.0 {
            return_factors.push(5.0); // Lower slippage = better returns
        }

        // Risk adjustment
        let risk_adjustment = (1.0 - opportunity.risk_score) * 10.0;
        return_factors.push(risk_adjustment);

        let total_return: f64 = return_factors.iter().sum();
        (total_return / (return_factors.len() as f64)).clamp(5.0, 50.0) // 5% to 50% expected return
    }

    /// Determine appropriate time horizon for the trade
    fn determine_time_horizon(&self, opportunity: &TokenOpportunity) -> String {
        if opportunity.metrics.age_hours < 6.0 {
            "short".to_string() // Very new tokens - quick trades
        } else if
            opportunity.metrics.liquidity_usd > 500000.0 &&
            opportunity.verification_status.is_verified
        {
            "long".to_string() // Established tokens - longer holds
        } else {
            "medium".to_string() // Most tokens - medium term
        }
    }

    /// Generate human-readable entry reason
    fn generate_entry_reason(
        &self,
        opportunity: &TokenOpportunity,
        indicators: &HashMap<String, f64>
    ) -> String {
        let mut reasons = Vec::new();

        // Main confidence reason
        if opportunity.confidence_score > 0.8 {
            reasons.push("High confidence score".to_string());
        } else if opportunity.confidence_score > 0.6 {
            reasons.push("Good confidence score".to_string());
        }

        // Risk reason
        if opportunity.risk_score < 0.3 {
            reasons.push("Low risk profile".to_string());
        } else if opportunity.risk_score < 0.5 {
            reasons.push("Acceptable risk level".to_string());
        }

        // Technical reasons
        if let Some(&lv_ratio) = indicators.get("liquidity_volume_ratio") {
            if lv_ratio > 2.0 && lv_ratio < 8.0 {
                reasons.push("Good liquidity/volume balance".to_string());
            }
        }

        if let Some(&momentum) = indicators.get("price_momentum") {
            if momentum > 0.0 && momentum < 30.0 {
                reasons.push("Positive price momentum".to_string());
            } else if momentum < 0.0 && momentum > -15.0 {
                reasons.push("Potential reversal opportunity".to_string());
            }
        }

        // Verification reasons
        if opportunity.verification_status.is_verified {
            reasons.push("Verified token".to_string());
        }

        if opportunity.verification_status.has_profile {
            reasons.push("Has project profile".to_string());
        }

        // Default reason if no specific reasons
        if reasons.is_empty() {
            reasons.push("Meets minimum criteria".to_string());
        }

        reasons.join("; ")
    }

    /// Calculate portfolio correlation risk
    fn calculate_portfolio_correlation(
        &self,
        opportunity: &TokenOpportunity,
        portfolio: &Portfolio
    ) -> f64 {
        // Simplified correlation calculation
        // In reality, this would analyze token sector, technology, etc.

        let portfolio_size = portfolio.positions.len();

        if portfolio_size == 0 {
            return 0.0; // No correlation risk with empty portfolio
        }

        // If we already have many positions, correlation risk increases
        if portfolio_size > 30 {
            0.8
        } else if portfolio_size > 20 {
            0.6
        } else if portfolio_size > 10 {
            0.4
        } else {
            0.2
        }
    }
}
