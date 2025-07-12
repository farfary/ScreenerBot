use crate::prelude::*;

/// Smart profit targeting system for maximum success rate
pub struct ProfitTargetCalculator {
    pub min_profit_pct: f64,
    pub max_profit_pct: f64,
    pub quick_profit_threshold: f64,
}

impl Default for ProfitTargetCalculator {
    fn default() -> Self {
        Self {
            min_profit_pct: 0.3, // Minimum 0.3% profit to consider
            max_profit_pct: 100.0, // Maximum 100% profit target
            quick_profit_threshold: 2.0, // Quick profit at 2%
        }
    }
}

impl ProfitTargetCalculator {
    /// Calculate dynamic profit targets based on market conditions
    pub fn calculate_profit_targets(
        &self,
        token: &Token,
        entry_price: f64,
        current_price: f64
    ) -> Vec<ProfitTarget> {
        let current_profit = ((current_price - entry_price) / entry_price) * 100.0;
        let liquidity_sol = token.liquidity.base + token.liquidity.quote;
        let volume_24h = token.volume.h24;

        let mut targets = Vec::new();

        // Micro profit (always available for quick wins)
        targets.push(ProfitTarget {
            percentage: 0.5,
            urgency: ProfitUrgency::Low,
            size_to_sell: 0.2, // Sell 20% of position
            reason: "Micro profit - quick win".to_string(),
        });

        // Small profit (conservative target)
        targets.push(ProfitTarget {
            percentage: 1.5,
            urgency: ProfitUrgency::Medium,
            size_to_sell: 0.3, // Sell 30% of position
            reason: "Small profit - conservative exit".to_string(),
        });

        // Medium profit (main target)
        targets.push(ProfitTarget {
            percentage: 4.0,
            urgency: ProfitUrgency::Medium,
            size_to_sell: 0.4, // Sell 40% of position
            reason: "Medium profit - main target".to_string(),
        });

        // Good profit (let winners run)
        targets.push(ProfitTarget {
            percentage: 8.0,
            urgency: ProfitUrgency::Low,
            size_to_sell: 0.5, // Sell 50% of position
            reason: "Good profit - partial exit".to_string(),
        });

        // Large profit (moon shot protection)
        targets.push(ProfitTarget {
            percentage: 20.0,
            urgency: ProfitUrgency::High,
            size_to_sell: 0.8, // Sell 80% of position
            reason: "Large profit - secure gains".to_string(),
        });

        // Adjust targets based on liquidity and volume
        self.adjust_targets_for_conditions(&mut targets, liquidity_sol, volume_24h);

        targets
    }

    fn adjust_targets_for_conditions(
        &self,
        targets: &mut Vec<ProfitTarget>,
        liquidity_sol: f64,
        volume_24h: f64
    ) {
        let liquidity_factor = if liquidity_sol < 100.0 {
            0.8 // Lower targets for low liquidity
        } else if liquidity_sol > 1000.0 {
            1.2 // Higher targets for high liquidity
        } else {
            1.0
        };

        let volume_factor = if volume_24h < 10000.0 {
            0.9 // Lower targets for low volume
        } else if volume_24h > 100000.0 {
            1.1 // Higher targets for high volume
        } else {
            1.0
        };

        let adjustment = liquidity_factor * volume_factor;

        for target in targets.iter_mut() {
            target.percentage *= adjustment;

            // Increase urgency for low liquidity/volume
            if adjustment < 0.9 {
                target.urgency = match target.urgency {
                    ProfitUrgency::Low => ProfitUrgency::Medium,
                    ProfitUrgency::Medium => ProfitUrgency::High,
                    ProfitUrgency::High => ProfitUrgency::Critical,
                    ProfitUrgency::Critical => ProfitUrgency::Critical,
                };
            }
        }
    }

    /// Check if should take profit immediately based on current conditions
    pub fn should_take_immediate_profit(
        &self,
        token: &Token,
        position: &Position,
        current_price: f64
    ) -> Option<ImmediateProfitDecision> {
        let current_profit =
            ((current_price - position.entry_price) / position.entry_price) * 100.0;

        if current_profit < self.min_profit_pct {
            return None; // Not profitable enough
        }

        let targets = self.calculate_profit_targets(token, position.entry_price, current_price);

        // Find the most appropriate target
        for target in targets {
            if current_profit >= target.percentage {
                return Some(ImmediateProfitDecision {
                    should_sell: true,
                    target: target,
                    current_profit_pct: current_profit,
                    confidence: self.calculate_exit_confidence(token, current_profit),
                });
            }
        }

        None
    }

    fn calculate_exit_confidence(&self, token: &Token, profit_pct: f64) -> f64 {
        let mut confidence = 0.5f64; // Base confidence

        // Higher confidence for smaller, more certain profits
        if profit_pct <= 2.0 {
            confidence += 0.3; // High confidence for small profits
        } else if profit_pct <= 5.0 {
            confidence += 0.2; // Good confidence for medium profits
        } else if profit_pct <= 10.0 {
            confidence += 0.1; // Lower confidence for large profits
        }

        // Liquidity factor
        let liquidity_sol = token.liquidity.base + token.liquidity.quote;
        if liquidity_sol > 500.0 {
            confidence += 0.1;
        }

        // Volume factor
        if token.volume.h24 > 50000.0 {
            confidence += 0.1;
        }

        confidence.min(1.0)
    }
}

#[derive(Debug, Clone)]
pub struct ProfitTarget {
    pub percentage: f64,
    pub urgency: ProfitUrgency,
    pub size_to_sell: f64, // Fraction of position to sell (0.0 to 1.0)
    pub reason: String,
}

#[derive(Debug, Clone)]
pub enum ProfitUrgency {
    Low, // Can wait for better conditions
    Medium, // Should sell if conditions are right
    High, // Sell quickly
    Critical, // Sell immediately
}

#[derive(Debug, Clone)]
pub struct ImmediateProfitDecision {
    pub should_sell: bool,
    pub target: ProfitTarget,
    pub current_profit_pct: f64,
    pub confidence: f64,
}
