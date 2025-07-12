use crate::prelude::*;

/// Dynamic DCA calculator that adapts to token characteristics
pub struct DynamicDcaCalculator {
    pub base_dca_percentage: f64,
    pub max_dca_count: u8,
    pub liquidity_adjustment_factor: f64,
}

impl Default for DynamicDcaCalculator {
    fn default() -> Self {
        Self {
            base_dca_percentage: -8.0, // Start DCA at -8% drop
            max_dca_count: 5, // Maximum 5 DCA entries
            liquidity_adjustment_factor: 0.1,
        }
    }
}

impl DynamicDcaCalculator {
    /// Calculate dynamic DCA thresholds based on token characteristics
    pub fn calculate_dca_levels(
        &self,
        token: &Token,
        current_position: &Position
    ) -> Vec<DcaLevel> {
        let liquidity_sol = token.liquidity.base + token.liquidity.quote;
        // Parse market cap from fdv_usd field
        let market_cap = token.fdv_usd.parse::<f64>().unwrap_or(0.0);
        let volume_24h = token.volume.h24;

        let mut levels = Vec::new();

        // Adjust DCA spacing based on liquidity and volatility
        let base_spacing = self.calculate_base_spacing(liquidity_sol, market_cap, volume_24h);
        let position_size_multiplier = self.calculate_position_multiplier(
            current_position.dca_count
        );

        for i in 1..=self.max_dca_count {
            let drop_threshold =
                self.base_dca_percentage * (1.0 + ((i as f64) - 1.0) * base_spacing);
            let position_size = self.calculate_dynamic_position_size(
                token,
                liquidity_sol,
                i,
                position_size_multiplier
            );

            levels.push(DcaLevel {
                level: i,
                drop_threshold_pct: drop_threshold,
                position_size_sol: position_size,
                confidence_required: self.calculate_confidence_requirement(i, liquidity_sol),
                max_age_seconds: self.calculate_max_signal_age(i),
            });
        }

        levels
    }

    /// Calculate base spacing between DCA levels
    fn calculate_base_spacing(&self, liquidity_sol: f64, market_cap: f64, volume_24h: f64) -> f64 {
        let mut spacing = 0.5f64; // Base 50% spacing between levels

        // Higher liquidity = wider spacing (more stable)
        if liquidity_sol > 1000.0 {
            spacing += 0.2;
        } else if liquidity_sol < 100.0 {
            spacing -= 0.1; // Tighter spacing for low liquidity
        }

        // Higher volume = tighter spacing (more volatile)
        if volume_24h > 100000.0 {
            spacing -= 0.1;
        }

        // Higher market cap = wider spacing (more stable)
        if market_cap > 1000000.0 {
            spacing += 0.1;
        }

        spacing.max(0.2).min(1.0)
    }

    /// Calculate position size multiplier based on DCA count
    fn calculate_position_multiplier(&self, current_dca_count: u8) -> f64 {
        match current_dca_count {
            0 => 1.0, // First position - normal size
            1 => 1.2, // First DCA - slightly larger
            2 => 1.5, // Second DCA - larger
            3 => 2.0, // Third DCA - much larger
            4 => 2.5, // Fourth DCA - largest
            _ => 3.0, // Final DCA - maximum size
        }
    }

    /// Calculate dynamic position size for each DCA level
    fn calculate_dynamic_position_size(
        &self,
        token: &Token,
        liquidity_sol: f64,
        level: u8,
        multiplier: f64
    ) -> f64 {
        // Base size calculation
        let base_size = if liquidity_sol <= 50.0 {
            0.003 // Very small for low liquidity
        } else if liquidity_sol <= 500.0 {
            0.005 // Small for moderate liquidity
        } else if liquidity_sol <= 5000.0 {
            0.008 // Medium for good liquidity
        } else {
            0.012 // Larger for high liquidity
        };

        // Apply level multiplier
        let level_multiplier = match level {
            1 => 1.0,
            2 => 1.3,
            3 => 1.6,
            4 => 2.0,
            5 => 2.5,
            _ => 3.0,
        };

        // Safety check - never exceed 0.5% of liquidity
        let max_safe_size = liquidity_sol * 0.005;
        let calculated_size = base_size * level_multiplier * multiplier;

        calculated_size.min(max_safe_size).max(0.001)
    }

    /// Calculate confidence requirement for each DCA level
    fn calculate_confidence_requirement(&self, level: u8, liquidity_sol: f64) -> f64 {
        let base_confidence = match level {
            1 => 0.4, // Lower confidence for first DCA
            2 => 0.5,
            3 => 0.6,
            4 => 0.7,
            5 => 0.8, // High confidence for final DCA
            _ => 0.9,
        };

        // Adjust based on liquidity
        if liquidity_sol < 100.0 {
            base_confidence + 0.1 // Higher confidence required for low liquidity
        } else {
            base_confidence
        }
    }

    /// Calculate maximum signal age for each DCA level
    fn calculate_max_signal_age(&self, level: u8) -> u64 {
        match level {
            1 => 300, // 5 minutes for first DCA
            2 => 600, // 10 minutes for second DCA
            3 => 900, // 15 minutes for third DCA
            4 => 1200, // 20 minutes for fourth DCA
            5 => 1800, // 30 minutes for final DCA
            _ => 3600, // 1 hour maximum
        }
    }

    /// Check if DCA should be executed based on current conditions
    pub fn should_execute_dca(
        &self,
        token: &Token,
        position: &Position,
        current_price: f64,
        drop_signal: Option<&crate::strategy::helpers::drop_detector::DropSignal>
    ) -> Option<DcaDecision> {
        let levels = self.calculate_dca_levels(token, position);
        let current_drop = ((current_price - position.entry_price) / position.entry_price) * 100.0;

        // Find applicable DCA level
        for level in levels {
            if position.dca_count < level.level && current_drop <= level.drop_threshold_pct {
                // Check if we have a valid drop signal with sufficient confidence
                if let Some(signal) = drop_signal {
                    if
                        signal.confidence >= level.confidence_required &&
                        signal.is_fresh(level.max_age_seconds)
                    {
                        return Some(DcaDecision {
                            should_dca: true,
                            level: level.level,
                            size_sol: level.position_size_sol,
                            reason: format!(
                                "DCA Level {} triggered at {:.1}% drop (threshold: {:.1}%)",
                                level.level,
                                current_drop,
                                level.drop_threshold_pct
                            ),
                            confidence: signal.confidence,
                        });
                    }
                } else {
                    // No signal available, use basic threshold check with higher confidence requirement
                    if current_drop <= level.drop_threshold_pct * 1.5 {
                        // 50% more strict without signal
                        return Some(DcaDecision {
                            should_dca: true,
                            level: level.level,
                            size_sol: level.position_size_sol * 0.7, // Smaller size without signal
                            reason: format!(
                                "DCA Level {} triggered at {:.1}% drop (no signal, conservative)",
                                level.level,
                                current_drop
                            ),
                            confidence: 0.3, // Lower confidence without signal
                        });
                    }
                }
            }
        }

        None
    }
}

#[derive(Debug, Clone)]
pub struct DcaLevel {
    pub level: u8,
    pub drop_threshold_pct: f64,
    pub position_size_sol: f64,
    pub confidence_required: f64,
    pub max_age_seconds: u64,
}

#[derive(Debug, Clone)]
pub struct DcaDecision {
    pub should_dca: bool,
    pub level: u8,
    pub size_sol: f64,
    pub reason: String,
    pub confidence: f64,
}
