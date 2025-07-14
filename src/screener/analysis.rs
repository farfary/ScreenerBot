use crate::core::{ BotResult, BotError, TokenOpportunity, RiskLevel, RiskAssessment };
use std::collections::HashMap;

/// Analyzer for token opportunities
#[derive(Debug)]
pub struct OpportunityAnalyzer {}

impl OpportunityAnalyzer {
    pub fn new() -> Self {
        Self {}
    }

    /// Analyze a list of opportunities
    pub async fn analyze_opportunities(
        &self,
        opportunities: Vec<TokenOpportunity>
    ) -> BotResult<Vec<TokenOpportunity>> {
        let mut analyzed = Vec::new();

        for mut opportunity in opportunities {
            self.enhance_opportunity_analysis(&mut opportunity).await?;
            analyzed.push(opportunity);
        }

        // Sort by overall score
        analyzed.sort_by(|a, b| {
            let score_a = self.calculate_overall_score(a);
            let score_b = self.calculate_overall_score(b);
            score_b.partial_cmp(&score_a).unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(analyzed)
    }

    /// Enhance opportunity with additional analysis
    async fn enhance_opportunity_analysis(
        &self,
        opportunity: &mut TokenOpportunity
    ) -> BotResult<()> {
        // Recalculate risk score with more detailed analysis
        opportunity.risk_score = self.calculate_detailed_risk_score(opportunity);

        // Recalculate confidence score
        opportunity.confidence_score = self.calculate_detailed_confidence_score(opportunity);

        Ok(())
    }

    /// Calculate detailed risk score
    fn calculate_detailed_risk_score(&self, opportunity: &TokenOpportunity) -> f64 {
        let mut risk_factors = Vec::new();

        // Liquidity risk
        let liquidity_risk = if opportunity.metrics.liquidity_usd < 5000.0 {
            RiskLevel::Critical
        } else if opportunity.metrics.liquidity_usd < 20000.0 {
            RiskLevel::High
        } else if opportunity.metrics.liquidity_usd < 100000.0 {
            RiskLevel::Medium
        } else {
            RiskLevel::Low
        };
        risk_factors.push(self.risk_level_to_score(&liquidity_risk));

        // Volume risk
        let volume_risk = if opportunity.metrics.volume_24h < 1000.0 {
            RiskLevel::High
        } else if opportunity.metrics.volume_24h < 10000.0 {
            RiskLevel::Medium
        } else {
            RiskLevel::Low
        };
        risk_factors.push(self.risk_level_to_score(&volume_risk));

        // Age risk
        let age_risk = if opportunity.metrics.age_hours < 1.0 {
            RiskLevel::Critical
        } else if opportunity.metrics.age_hours < 6.0 {
            RiskLevel::High
        } else if opportunity.metrics.age_hours < 24.0 {
            RiskLevel::Medium
        } else {
            RiskLevel::Low
        };
        risk_factors.push(self.risk_level_to_score(&age_risk));

        // Market cap risk
        let mcap_risk = if let Some(mcap) = opportunity.metrics.market_cap {
            if mcap < 50000.0 {
                RiskLevel::High
            } else if mcap > 100000000.0 {
                RiskLevel::Medium // Very high mcap might be overvalued
            } else {
                RiskLevel::Low
            }
        } else {
            RiskLevel::Medium // Unknown market cap
        };
        risk_factors.push(self.risk_level_to_score(&mcap_risk));

        // Price volatility risk
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
        risk_factors.push(self.risk_level_to_score(&volatility_risk));

        // Verification risk
        let verification_risk = if opportunity.verification_status.security_flags.len() > 0 {
            RiskLevel::High
        } else if
            !opportunity.verification_status.is_verified &&
            !opportunity.verification_status.has_profile
        {
            RiskLevel::Medium
        } else {
            RiskLevel::Low
        };
        risk_factors.push(self.risk_level_to_score(&verification_risk));

        // Calculate weighted average risk
        let total_risk: f64 = risk_factors.iter().sum();
        let average_risk = total_risk / (risk_factors.len() as f64);

        average_risk.clamp(0.0, 1.0)
    }

    /// Calculate detailed confidence score
    fn calculate_detailed_confidence_score(&self, opportunity: &TokenOpportunity) -> f64 {
        let mut confidence_factors = Vec::new();

        // Volume confidence
        let volume_confidence = if opportunity.metrics.volume_24h > 100000.0 {
            0.9
        } else if opportunity.metrics.volume_24h > 50000.0 {
            0.8
        } else if opportunity.metrics.volume_24h > 10000.0 {
            0.6
        } else if opportunity.metrics.volume_24h > 1000.0 {
            0.4
        } else {
            0.1
        };
        confidence_factors.push(volume_confidence);

        // Liquidity confidence
        let liquidity_confidence = if opportunity.metrics.liquidity_usd > 500000.0 {
            0.9
        } else if opportunity.metrics.liquidity_usd > 100000.0 {
            0.8
        } else if opportunity.metrics.liquidity_usd > 50000.0 {
            0.6
        } else if opportunity.metrics.liquidity_usd > 10000.0 {
            0.4
        } else {
            0.2
        };
        confidence_factors.push(liquidity_confidence);

        // Verification confidence
        let verification_confidence = if opportunity.verification_status.is_verified {
            0.8
        } else if opportunity.verification_status.has_profile {
            0.6
        } else {
            0.3
        };
        confidence_factors.push(verification_confidence);

        // Stability confidence (lower volatility = higher confidence for trading)
        let stability_confidence = if let Some(change) = opportunity.metrics.price_change_24h {
            let abs_change = change.abs();
            if abs_change < 5.0 {
                0.9 // Very stable
            } else if abs_change < 20.0 {
                0.7 // Moderately stable
            } else if abs_change < 50.0 {
                0.5 // Volatile but manageable
            } else {
                0.2 // Highly volatile
            }
        } else {
            0.5 // Unknown stability
        };
        confidence_factors.push(stability_confidence);

        // Market presence confidence
        let market_confidence = if let Some(mcap) = opportunity.metrics.market_cap {
            if mcap > 1000000.0 {
                0.8
            } else if mcap > 100000.0 {
                0.6
            } else if mcap > 50000.0 {
                0.4
            } else {
                0.2
            }
        } else {
            0.3
        };
        confidence_factors.push(market_confidence);

        // Calculate weighted average confidence
        let total_confidence: f64 = confidence_factors.iter().sum();
        let average_confidence = total_confidence / (confidence_factors.len() as f64);

        average_confidence.clamp(0.0, 1.0)
    }

    /// Convert risk level to numeric score
    fn risk_level_to_score(&self, risk: &RiskLevel) -> f64 {
        match risk {
            RiskLevel::Low => 0.2,
            RiskLevel::Medium => 0.5,
            RiskLevel::High => 0.8,
            RiskLevel::Critical => 1.0,
        }
    }

    /// Calculate overall opportunity score
    fn calculate_overall_score(&self, opportunity: &TokenOpportunity) -> f64 {
        // Score = Confidence - Risk + Source bonus
        let base_score = opportunity.confidence_score - opportunity.risk_score;

        // Add source reliability bonus
        let source_bonus = match opportunity.source {
            crate::core::ScreenerSource::DexScreener => 0.1,
            crate::core::ScreenerSource::GeckoTerminal => 0.15,
            crate::core::ScreenerSource::Raydium => 0.2,
            crate::core::ScreenerSource::RugCheck => 0.05,
            crate::core::ScreenerSource::Manual => 0.25,
        };

        (base_score + source_bonus).clamp(-1.0, 1.0)
    }

    /// Generate technical indicators for an opportunity
    pub fn generate_technical_indicators(
        &self,
        opportunity: &TokenOpportunity
    ) -> HashMap<String, f64> {
        let mut indicators = HashMap::new();

        // Liquidity-to-Volume ratio
        let lv_ratio = if opportunity.metrics.volume_24h > 0.0 {
            opportunity.metrics.liquidity_usd / opportunity.metrics.volume_24h
        } else {
            0.0
        };
        indicators.insert("liquidity_volume_ratio".to_string(), lv_ratio);

        // Market cap efficiency (price stability relative to market cap)
        if let Some(mcap) = opportunity.metrics.market_cap {
            let efficiency = if mcap > 0.0 { opportunity.metrics.volume_24h / mcap } else { 0.0 };
            indicators.insert("market_cap_efficiency".to_string(), efficiency);
        }

        // Price momentum (if available)
        if let Some(change) = opportunity.metrics.price_change_24h {
            indicators.insert("price_momentum".to_string(), change);
            indicators.insert("price_momentum_abs".to_string(), change.abs());
        }

        // Risk-adjusted return potential
        let risk_adjusted_potential = opportunity.confidence_score / (opportunity.risk_score + 0.1);
        indicators.insert("risk_adjusted_potential".to_string(), risk_adjusted_potential);

        indicators
    }
}
