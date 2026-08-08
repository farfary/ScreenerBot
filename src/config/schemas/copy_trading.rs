//! Global copy-trading policy. Per-target limits and mode live in copy_trading.db.

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
    }
}

impl CopyTradingConfig {
    pub fn validate(&self) -> Result<(), String> {
        if !(1..=50).contains(&self.max_active_tasks) {
            return Err("Maximum active copy tasks must be between 1 and 50".to_owned());
        }
        if !self.default_slippage_pct.is_finite()
            || self.default_slippage_pct <= 0.0
            || self.default_slippage_pct > crate::trader::MAX_MANUAL_SLIPPAGE_PCT
        {
            return Err(format!(
                "Default copy slippage must be greater than 0 and at most {}%",
                crate::trader::MAX_MANUAL_SLIPPAGE_PCT
            ));
        }
        if self.default_mode != "paper" {
            return Err(
                "New copy tasks must default to paper; arm live per task with confirmation"
                    .to_owned(),
            );
        }
        if !self.block_on_force_stop {
            return Err("Copy trading cannot bypass the global force stop".to_owned());
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
