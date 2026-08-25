//! Global copy-trading policy. Per-target limits and mode live in copy_trading.db.

use crate::config::{Error, Result};
use crate::errors::ConfigurationError;
use crate::{config_struct, field_metadata};

config_struct! {
    pub struct CopyTradingConfig {
        #[metadata(field_metadata! { label: "Wallet Copy", hint: "Enable copy task processing", impact: "high", category: "Copy Trading", })]
        enabled: bool = false,
        #[metadata(field_metadata! { label: "Maximum Active Tasks", hint: "Maximum simultaneously enabled copy tasks", min: 1, max: 50, step: 1, impact: "high", category: "Copy Trading", })]
        max_active_tasks: usize = 10,
        #[metadata(field_metadata! { label: "Default Slippage", hint: "Default paper and future live copy slippage", min: 0.1, max: 50.0, step: 0.1, unit: "%", impact: "high", category: "Copy Trading", })]
        default_slippage_pct: f64 = 2.0,
        #[metadata(field_metadata! { label: "Default Mode", hint: "New copy tasks always begin in paper mode", impact: "low", category: "Copy Trading", hidden: true, })]
        default_mode: String = "paper".to_owned(),
        #[metadata(field_metadata! { label: "Require Filter Pass", hint: "Only copy tokens accepted by the filtering pipeline", impact: "high", category: "Copy Trading", })]
        require_filter_pass: bool = true,
        #[metadata(field_metadata! { label: "Block On Force Stop", hint: "Copy entries always obey the global force stop", impact: "high", category: "Copy Trading", hidden: true, })]
        block_on_force_stop: bool = true,
        #[metadata(field_metadata! { label: "Latency Kill Switch", hint: "Pause a copy task when its recent target-to-detection delay stays above the configured limit", impact: "high", category: "Copy Trading", })]
        latency_kill_switch_enabled: bool = true,
        #[metadata(field_metadata! { label: "Maximum Arrival Delay", hint: "Pause after the trailing sample window exceeds this average target-to-detection delay", min: 250, max: 30000, step: 250, unit: "ms", impact: "high", category: "Copy Trading", })]
        max_arrival_distance_ms: u64 = 4000,
        #[metadata(field_metadata! { label: "Latency Sample Window", hint: "Number of recent observations used by the latency kill switch", min: 3, max: 100, step: 1, impact: "medium", category: "Copy Trading", })]
        latency_window_size: usize = 10,
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
        if !self.default_slippage_pct.is_finite()
            || self.default_slippage_pct <= 0.0
            || self.default_slippage_pct > crate::trader::MAX_MANUAL_SLIPPAGE_PCT
        {
            return Err(Error::Configuration(ConfigurationError::Generic {
                message: format!(
                    "Default copy slippage must be greater than 0 and at most {}%",
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
