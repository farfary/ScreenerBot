//! Structured DCA evaluation for better observability and testing

use crate::positions::Position;
use chrono::Utc;
use serde::{Deserialize, Serialize};

/// Configuration snapshot for DCA evaluation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DcaConfigSnapshot {
    pub enabled: bool,
    pub max_count: u32,
    pub cooldown_minutes: i64,
    pub threshold_pct: f64,
    pub size_percentage: f64,
}

/// Calculated metrics for DCA evaluation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DcaCalculations {
    pub current_dca_count: u32,
    pub minutes_since_last: Option<i64>,
    pub pnl_pct: f64,
    pub required_drop_pct: f64,
    pub dca_amount_sol: f64,
    pub entry_price: f64,
    pub current_price: f64,
}

/// Structured DCA evaluation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DcaEvaluation {
    pub should_trigger: bool,
    pub reasons: Vec<String>,
    pub config: DcaConfigSnapshot,
    pub calculations: DcaCalculations,
}

impl DcaEvaluation {
    /// Evaluate whether DCA should trigger for a position
    pub fn evaluate(position: &Position, config: DcaConfigSnapshot) -> Result<Self, String> {
        let mut reasons = Vec::new();
        let mut should_trigger = true;

        // Extract position data
        let current_price = match position.current_price {
            Some(price) if price > 0.0 && price.is_finite() => price,
            _ => {
                return Ok(Self {
                    should_trigger: false,
                    reasons: vec!["No valid current price".to_string()],
                    config: config.clone(),
                    calculations: DcaCalculations {
                        current_dca_count: position.dca_count,
                        minutes_since_last: None,
                        pnl_pct: 0.0,
                        required_drop_pct: config.threshold_pct.abs(),
                        dca_amount_sol: 0.0,
                        entry_price: position.average_entry_price,
                        current_price: 0.0,
                    },
                });
            }
        };

        let entry_price = position.average_entry_price;
        if entry_price <= 0.0 || !entry_price.is_finite() {
            return Ok(Self {
                should_trigger: false,
                reasons: vec!["No valid entry price".to_string()],
                config: config.clone(),
                calculations: DcaCalculations {
                    current_dca_count: position.dca_count,
                    minutes_since_last: None,
                    pnl_pct: 0.0,
                    required_drop_pct: config.threshold_pct.abs(),
                    dca_amount_sol: 0.0,
                    entry_price,
                    current_price,
                },
            });
        }

        // Calculate metrics
        let pnl_pct = ((current_price - entry_price) / entry_price) * 100.0;
        let required_drop_pct = config.threshold_pct.abs(); // Config value is negative, we need positive for comparison
        let dca_amount_sol = position.entry_size_sol * (config.size_percentage / 100.0);

        let minutes_since_last = position
            .last_dca_time
            .map(|t| (Utc::now() - t).num_minutes());

        let calculations = DcaCalculations {
            current_dca_count: position.dca_count,
            minutes_since_last,
            pnl_pct,
            required_drop_pct,
            dca_amount_sol,
            entry_price,
            current_price,
        };

        // Evaluate conditions
        if !config.enabled {
            should_trigger = false;
            reasons.push("DCA disabled in config".to_string());
        }

        if calculations.current_dca_count >= config.max_count {
            should_trigger = false;
            reasons.push(format!(
                "DCA count limit reached ({}/{})",
                calculations.current_dca_count, config.max_count
            ));
        }

        if let Some(minutes) = calculations.minutes_since_last {
            if minutes < config.cooldown_minutes {
                should_trigger = false;
                reasons.push(format!(
                    "DCA cooldown active ({}/{} minutes)",
                    minutes, config.cooldown_minutes
                ));
            }
        }

        // Check if price has dropped enough (pnl_pct is negative when losing)
        // If pnl_pct = -15% and required_drop_pct = 10%, then abs(-15) = 15 >= 10 → trigger
        if pnl_pct >= config.threshold_pct {
            // Not losing enough (threshold is negative, so if pnl_pct >= threshold, not dropped enough)
            should_trigger = false;
            reasons.push(format!(
                "Price drop insufficient: {:.2}% P&L (need < {:.2}%)",
                pnl_pct, config.threshold_pct
            ));
        }

        if should_trigger {
            reasons.push(format!(
                "DCA triggered: {:.2}% loss exceeds {:.2}% threshold (price: {:.9} → {:.9} SOL)",
                pnl_pct, config.threshold_pct, entry_price, current_price
            ));
        }

        Ok(Self {
            should_trigger,
            reasons,
            config,
            calculations,
        })
    }

    /// Get a human-readable summary of the evaluation
    pub fn summary(&self) -> String {
        if self.should_trigger {
            format!(
                "DCA #{}: {:.2}% loss, amount: {:.4} SOL",
                self.calculations.current_dca_count + 1,
                self.calculations.pnl_pct,
                self.calculations.dca_amount_sol
            )
        } else {
            self.reasons.join(", ")
        }
    }
}
