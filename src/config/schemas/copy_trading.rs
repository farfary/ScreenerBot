// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Global copy-trading policy. Per-target limits and mode live in copy_trading.db.

use crate::config::metadata::{ConfigCategory, ConfigImpact};
use crate::config::{Error, Result};
use crate::errors::ConfigurationError;
use crate::{config_struct, field_metadata};

config_struct! {
    pub struct CopyTradingConfig {
        #[metadata(field_metadata! { impact: ConfigImpact::High, category: ConfigCategory::CopyTrading, })]
        enabled: bool = false,
        #[metadata(field_metadata! { min: 1, max: 50, step: 1, impact: ConfigImpact::High, category: ConfigCategory::CopyTrading, })]
        max_active_tasks: usize = 10,
        #[metadata(field_metadata! { min: 0.1, max: 50.0, step: 0.1, impact: ConfigImpact::High, category: ConfigCategory::CopyTrading, })]
        default_slippage_pct: f64 = 2.0,
        #[metadata(field_metadata! { impact: ConfigImpact::Low, category: ConfigCategory::CopyTrading, hidden: true, })]
        default_mode: String = "paper".to_owned(),
        #[metadata(field_metadata! { impact: ConfigImpact::High, category: ConfigCategory::CopyTrading, })]
        require_filter_pass: bool = true,
        #[metadata(field_metadata! { impact: ConfigImpact::High, category: ConfigCategory::CopyTrading, hidden: true, })]
        block_on_force_stop: bool = true,
        #[metadata(field_metadata! { impact: ConfigImpact::High, category: ConfigCategory::CopyTrading, })]
        latency_kill_switch_enabled: bool = true,
        #[metadata(field_metadata! { min: 250, max: 30000, step: 250, impact: ConfigImpact::High, category: ConfigCategory::CopyTrading, })]
        max_arrival_distance_ms: u64 = 4000,
        #[metadata(field_metadata! { min: 3, max: 100, step: 1, impact: ConfigImpact::Medium, category: ConfigCategory::CopyTrading, })]
        latency_window_size: usize = 10,
        #[metadata(field_metadata! { min: 1, max: 500, step: 1, impact: ConfigImpact::Low, category: ConfigCategory::CopyTrading, })]
        readiness_min_closed_rounds: usize = 10,
    }
}

impl CopyTradingConfig {
    pub fn validate(&self) -> Result<()> {
        if !(1..=50).contains(&self.max_active_tasks) {
            return Err(ConfigurationError::Generic {
                message: "Maximum active copy tasks must be between 1 and 50".to_owned(),
            }
            .into());
        }
        let minimum_slippage = crate::trader::copy::MIN_COPY_SLIPPAGE_PCT;
        if !self.default_slippage_pct.is_finite()
            || self.default_slippage_pct < minimum_slippage
            || self.default_slippage_pct > crate::trader::MAX_MANUAL_SLIPPAGE_PCT
        {
            return Err(Error::Configuration(ConfigurationError::Generic {
                message: format!(
                    "Default copy slippage must be between {minimum_slippage}% and {}%",
                    crate::trader::MAX_MANUAL_SLIPPAGE_PCT
                ),
            }));
        }
        if self.default_mode != "paper" {
            return Err(ConfigurationError::Generic {
                message:
                    "New copy tasks must default to paper; arm live per task with confirmation"
                        .to_owned(),
            }
            .into());
        }
        if !self.block_on_force_stop {
            return Err(ConfigurationError::Generic {
                message: "Copy trading cannot bypass the global force stop".to_owned(),
            }
            .into());
        }
        if !(250..=30_000).contains(&self.max_arrival_distance_ms) {
            return Err(ConfigurationError::Generic {
                message: "Maximum copy arrival delay must be between 250 and 30000 ms".to_owned(),
            }
            .into());
        }
        if !(3..=100).contains(&self.latency_window_size) {
            return Err(ConfigurationError::Generic {
                message: "Copy latency sample window must be between 3 and 100".to_owned(),
            }
            .into());
        }
        if !(1..=500).contains(&self.readiness_min_closed_rounds) {
            return Err(ConfigurationError::Generic {
                message: "Live readiness rounds must be between 1 and 500".to_owned(),
            }
            .into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::CopyTradingConfig;

    #[test]
    fn defaults_are_safe_and_live_or_force_stop_bypass_are_rejected() {
        assert!(CopyTradingConfig::default().validate().is_ok());

        let mut live = CopyTradingConfig::default();
        live.default_mode = "live".to_owned();
        assert!(live.validate().is_err());

        let mut bypass = CopyTradingConfig::default();
        bypass.block_on_force_stop = false;
        assert!(bypass.validate().is_err());
    }
}
