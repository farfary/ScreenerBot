use crate::prelude::*;
use crate::performance::PerformanceMetrics;

/// Smart position sizing calculator for high-success rate trading
pub struct PositionSizer {
    pub min_size_sol: f64,
    pub max_size_sol: f64,
    pub target_success_rate: f64,
}

impl Default for PositionSizer {
    fn default() -> Self {
        Self {
            min_size_sol: 0.002,
            max_size_sol: 0.02,
            target_success_rate: 0.85, // 85% target success rate
        }
    }
}

impl PositionSizer {
    /// Calculate optimal position size for maximum success rate
    pub fn calculate_optimal_size(&self, token: &Token, opportunity_score: f64) -> f64 {
        let liquidity_sol = token.liquidity.base + token.liquidity.quote;
        // Parse market cap from fdv_usd field
        let market_cap = token.fdv_usd.parse::<f64>().unwrap_or(0.0);
        let volume_24h = token.volume.h24;

        // Base size from liquidity (conservative approach)
        let liquidity_size = self.calculate_liquidity_based_size(liquidity_sol);

        // Opportunity adjustment (higher score = larger size)
        let opportunity_multiplier = 0.5 + opportunity_score * 1.5; // 0.5x to 2.0x range

        // Volume factor (ensure sufficient activity)
        let volume_factor = self.calculate_volume_factor(volume_24h);

        // Market cap stability factor
        let stability_factor = self.calculate_stability_factor(market_cap);

        // Calculate final size
        let calculated_size =
            liquidity_size * opportunity_multiplier * volume_factor * stability_factor;

        // Apply safety bounds
        calculated_size.max(self.min_size_sol).min(self.max_size_sol)
    }

    fn calculate_liquidity_based_size(&self, liquidity_sol: f64) -> f64 {
        if liquidity_sol <= 10.0 {
            0.001 // Ultra small for very low liquidity
        } else if liquidity_sol <= 50.0 {
            0.002 // Small for low liquidity
        } else if liquidity_sol <= 200.0 {
            0.003 // Small-medium for moderate liquidity
        } else if liquidity_sol <= 1000.0 {
            0.005 // Medium for good liquidity
        } else if liquidity_sol <= 5000.0 {
            0.008 // Medium-large for high liquidity
        } else {
            0.012 // Large for very high liquidity
        }
    }

    fn calculate_volume_factor(&self, volume_24h: f64) -> f64 {
        if volume_24h < 1000.0 {
            0.5 // Very conservative for low volume
        } else if volume_24h < 10000.0 {
            0.7 // Conservative for moderate volume
        } else if volume_24h < 100000.0 {
            1.0 // Normal for good volume
        } else {
            1.2 // Slightly aggressive for high volume
        }
    }

    fn calculate_stability_factor(&self, market_cap: f64) -> f64 {
        if market_cap <= 0.0 {
            0.6 // Very conservative for unknown market cap
        } else if market_cap < 100000.0 {
            0.7 // Conservative for micro caps
        } else if market_cap < 1000000.0 {
            0.9 // Slightly conservative for small caps
        } else if market_cap < 10000000.0 {
            1.0 // Normal for medium caps
        } else {
            1.1 // Slightly aggressive for large caps
        }
    }

    /// Calculate risk-adjusted position size based on historical performance
    pub fn calculate_risk_adjusted_size(
        &self,
        token: &Token,
        base_size: f64,
        recent_performance: &PerformanceMetrics
    ) -> f64 {
        let mut adjustment_factor = 1.0;

        // Adjust based on recent win rate
        if recent_performance.win_rate < 0.6 {
            adjustment_factor *= 0.7; // Reduce size for poor performance
        } else if recent_performance.win_rate > 0.8 {
            adjustment_factor *= 1.2; // Increase size for good performance
        }

        // Adjust based on recent performance (7 days)
        if recent_performance.recent_performance_7d < -0.05 {
            adjustment_factor *= 0.8; // Reduce size after poor recent performance
        } else if recent_performance.recent_performance_7d > 0.1 {
            adjustment_factor *= 1.1; // Slight increase for good recent performance
        }

        // Apply adjustment
        (base_size * adjustment_factor).max(self.min_size_sol).min(self.max_size_sol)
    }

    /// Special sizing for MOONCAT (famous token with lots of data)
    pub fn calculate_mooncat_size(&self, token: &Token, opportunity_score: f64) -> f64 {
        // MOONCAT gets special treatment due to fame and data availability
        let base_size = self.calculate_optimal_size(token, opportunity_score);

        // Slight increase for MOONCAT due to:
        // 1. More data available for analysis
        // 2. Higher confidence in patterns
        // 3. Fame factor (more predictable behavior)
        let mooncat_multiplier = 1.3;

        (base_size * mooncat_multiplier).min(self.max_size_sol)
    }
}
