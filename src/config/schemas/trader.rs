// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Trading system configuration.

use crate::config::metadata::{ConfigCategory, ConfigImpact};
use crate::config_struct;
use crate::field_metadata;
use serde::{Deserialize, Serialize};

/// Time unit for duration configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TimeUnit {
    Seconds,
    Minutes,
    Hours,
    Days,
}

impl Default for TimeUnit {
    fn default() -> Self {
        TimeUnit::Hours
    }
}

impl TimeUnit {
    /// Convert duration to seconds
    pub fn to_seconds(&self, value: f64) -> f64 {
        match self {
            TimeUnit::Seconds => value,
            TimeUnit::Minutes => value * 60.0,
            TimeUnit::Hours => value * 3600.0,
            TimeUnit::Days => value * 86400.0,
        }
    }

    /// Convert from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "seconds" | "s" | "sec" => Some(TimeUnit::Seconds),
            "minutes" | "m" | "min" => Some(TimeUnit::Minutes),
            "hours" | "h" | "hr" => Some(TimeUnit::Hours),
            "days" | "d" | "day" => Some(TimeUnit::Days),
            _ => None,
        }
    }

    /// Convert to string
    pub fn to_string(&self) -> String {
        match self {
            TimeUnit::Seconds => "seconds".to_owned(),
            TimeUnit::Minutes => "minutes".to_owned(),
            TimeUnit::Hours => "hours".to_owned(),
            TimeUnit::Days => "days".to_owned(),
        }
    }
}

config_struct! {
    /// Trading system configuration
    pub struct TraderConfig {
        // Trader control
        #[metadata(field_metadata! {
            impact: ConfigImpact::Critical,
            category: ConfigCategory::CoreTrading,
        })]
        enabled: bool = false,

        // Core trading parameters
        #[metadata(field_metadata! {
            min: 1,
            max: 100,
            impact: ConfigImpact::Critical,
            category: ConfigCategory::CoreTrading,
        })]
        max_open_positions: usize = 2,
        #[metadata(field_metadata! {
            min: 0.001,
            max: 10,
            step: 0.001,
            impact: ConfigImpact::Critical,
            category: ConfigCategory::CoreTrading,
        })]
        trade_size_sol: f64 = 0.005,
        #[metadata(field_metadata! {
            impact: ConfigImpact::Medium,
            category: ConfigCategory::CoreTrading,
        })]
        entry_sizes: Vec<f64> = vec![0.005, 0.01, 0.02, 0.05],

        // ==================== ROI EXIT CONFIGURATION ====================
        #[metadata(field_metadata! {
            impact: ConfigImpact::High,
            category: ConfigCategory::RoiExit,
        })]
        roi_exit_enabled: bool = true,
        #[metadata(field_metadata! {
            min: 1,
            max: 1000,
            step: 1,
            impact: ConfigImpact::High,
            category: ConfigCategory::RoiExit,
        })]
        roi_target_percent: f64 = 20.0,

        // ==================== TIME OVERRIDE CONFIGURATION ====================
        #[metadata(field_metadata! {
            impact: ConfigImpact::High,
            category: ConfigCategory::TimeOverride,
        })]
        time_override_enabled: bool = true,
        #[metadata(field_metadata! {
            min: 1,
            max: 43200,
            step: 1,
            impact: ConfigImpact::Critical,
            category: ConfigCategory::TimeOverride,
        })]
        time_override_duration: f64 = 168.0,
        #[metadata(field_metadata! {
            impact: ConfigImpact::Critical,
            category: ConfigCategory::TimeOverride,
        })]
        time_override_unit: String = "hours".to_owned(),
        #[metadata(field_metadata! {
            min: -100,
            max: 0,
            step: 1,
            impact: ConfigImpact::Medium,
            category: ConfigCategory::TimeOverride,
        })]
        time_override_loss_threshold_percent: f64 = -40.0,

        // ==================== STOP LOSS CONFIGURATION ====================
        #[metadata(field_metadata! {
            impact: ConfigImpact::High,
            category: ConfigCategory::StopLoss,
        })]
        stop_loss_enabled: bool = false,
        #[metadata(field_metadata! {
            min: 1,
            max: 100,
            step: 1,
            impact: ConfigImpact::Critical,
            category: ConfigCategory::StopLoss,
        })]
        stop_loss_threshold_pct: f64 = 50.0,
        #[metadata(field_metadata! {
            impact: ConfigImpact::Medium,
            category: ConfigCategory::StopLoss,
        })]
        stop_loss_allow_partial: bool = false,
        #[metadata(field_metadata! {
            min: 0,
            max: 86400,
            step: 1,
            impact: ConfigImpact::Medium,
            category: ConfigCategory::StopLoss,
        })]
        stop_loss_min_hold_seconds: u64 = 0,

        // Position timing
        #[metadata(field_metadata! {
            min: 0,
            max: 1440,
            step: 5,
            impact: ConfigImpact::Critical,
            category: ConfigCategory::Timing,
        })]
        position_close_cooldown_minutes: i64 = 15,

        // Performance settings
        #[metadata(field_metadata! {
            min: 1,
            max: 50,
            step: 1,
            impact: ConfigImpact::Medium,
            category: ConfigCategory::Performance,
        })]
        entry_check_concurrency: usize = 10,

        // Sell concurrency
        #[metadata(field_metadata! {
            min: 1,
            max: 20,
            step: 1,
            impact: ConfigImpact::Medium,
            category: ConfigCategory::Performance,
        })]
        sell_concurrency: usize = 5,

        // ==================== DCA CONFIGURATION ====================
        #[metadata(field_metadata! {
            impact: ConfigImpact::High,
            category: ConfigCategory::Dca,
        })]
        dca_enabled: bool = false,
        #[metadata(field_metadata! {
            min: -100,
            max: 0,
            step: 1,
            impact: ConfigImpact::High,
            category: ConfigCategory::Dca,
        })]
        dca_threshold_pct: f64 = -10.0,
        #[metadata(field_metadata! {
            min: 1,
            max: 5,
            step: 1,
            impact: ConfigImpact::Critical,
            category: ConfigCategory::Dca,
        })]
        dca_max_count: usize = 2,
        #[metadata(field_metadata! {
            min: 10,
            max: 200,
            step: 10,
            impact: ConfigImpact::High,
            category: ConfigCategory::Dca,
        })]
        dca_size_percentage: f64 = 50.0,
        #[metadata(field_metadata! {
            min: 1,
            max: 1440,
            step: 5,
            impact: ConfigImpact::Medium,
            category: ConfigCategory::Dca,
        })]
        dca_cooldown_minutes: i64 = 30,

        // ==================== ENTRY/EXIT MONITOR CONTROL ====================
        /// Enable entry monitor (scans for new entry opportunities)
        /// When false, entry monitor pauses but exit monitor continues
        /// Default: true
        #[metadata(field_metadata! {
            impact: ConfigImpact::Critical,
            category: ConfigCategory::MonitorControl,
        })]
        entry_monitor_enabled: bool = true,

        /// Enable exit monitor (manages open positions, stop losses, exits)
        /// When false, exit monitor pauses - USE WITH CAUTION
        /// Default: true
        #[metadata(field_metadata! {
            impact: ConfigImpact::Critical,
            category: ConfigCategory::MonitorControl,
        })]
        exit_monitor_enabled: bool = true,

        // ==================== PERIOD LOSS LIMIT PROTECTION ====================
        /// Enable period-based loss limit protection
        /// When cumulative realized losses exceed the limit, entry monitor pauses
        /// Default: false
        #[metadata(field_metadata! {
            impact: ConfigImpact::High,
            category: ConfigCategory::LossLimit,
        })]
        loss_limit_enabled: bool = false,

        /// Maximum realized loss allowed in the period (in SOL)
        /// When cumulative losses reach this amount, new entries are blocked
        /// Default: 0.1 SOL
        #[metadata(field_metadata! {
            min: 0.001,
            max: 100.0,
            step: 0.01,
            impact: ConfigImpact::Critical,
            category: ConfigCategory::LossLimit,
        })]
        loss_limit_sol: f64 = 0.1,

        /// Loss limit period in hours (1, 6, 12, 24, etc.)
        /// After this period, cumulative loss tracking resets
        /// Default: 24 (daily limit)
        #[metadata(field_metadata! {
            min: 1,
            max: 168,
            step: 1,
            impact: ConfigImpact::High,
            category: ConfigCategory::LossLimit,
        })]
        loss_limit_period_hours: u64 = 24,

        /// Auto-resume when period resets
        /// If true, entry monitor automatically resumes after period reset
        /// If false, manual resume required via dashboard
        /// Default: true
        #[metadata(field_metadata! {
            impact: ConfigImpact::Medium,
            category: ConfigCategory::LossLimit,
        })]
        loss_limit_auto_resume: bool = true,
    }
}

#[cfg(test)]
mod tests {
    use super::TraderConfig;

    #[test]
    fn auto_trading_is_opt_in_by_default() {
        assert!(!TraderConfig::default().enabled);
    }
}
