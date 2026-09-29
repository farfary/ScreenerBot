//! Position management configuration

use crate::config::metadata::{ConfigCategory, ConfigImpact};
use crate::config_struct;
use crate::field_metadata;

config_struct! {
    /// Position management configuration
    pub struct PositionsConfig {
        /// Extra SOL needed for profit calculations (accounts for priority fees, etc.)
        #[metadata(field_metadata! {
            min: 0,
            max: 0.01,
            step: 0.0001,
            impact: ConfigImpact::High,
            category: ConfigCategory::Profit,
        })]
        profit_extra_needed_sol: f64 = 0.0002,

        /// Global cooldown between opening ANY positions (prevents rapid bursts)
        #[metadata(field_metadata! {
            min: 1,
            max: 30,
            step: 1,
            impact: ConfigImpact::Medium,
            category: ConfigCategory::Timing,
        })]
        position_open_cooldown_secs: i64 = 5,

        // ==================== LOSS DETECTION CONFIGURATION ====================
        #[metadata(field_metadata! {
            impact: ConfigImpact::High,
            category: ConfigCategory::LossDetection,
        })]
        loss_blacklist_enabled: bool = true,
        #[metadata(field_metadata! {
            min: -100,
            max: 0,
            step: 1,
            impact: ConfigImpact::High,
            category: ConfigCategory::LossDetection,
        })]
        loss_blacklist_threshold_pct: f64 = -15.0,
        #[metadata(field_metadata! {
            impact: ConfigImpact::High,
            category: ConfigCategory::LossDetection,
        })]
        loss_blacklist_copy_origins: bool = false,

        // ==================== PARTIAL EXIT CONFIGURATION ====================
        #[metadata(field_metadata! {
            impact: ConfigImpact::High,
            category: ConfigCategory::PartialExit,
        })]
        partial_exit_enabled: bool = false,
        #[metadata(field_metadata! {
            min: 10,
            max: 90,
            step: 5,
            impact: ConfigImpact::Medium,
            category: ConfigCategory::PartialExit,
        })]
        partial_exit_default_pct: f64 = 50.0,
        #[metadata(field_metadata! {
            min: 1,
            max: 50,
            step: 1,
            impact: ConfigImpact::Low,
            category: ConfigCategory::PartialExit,
        })]
        partial_exit_min_pct: f64 = 10.0,
        #[metadata(field_metadata! {
            min: 50,
            max: 99,
            step: 1,
            impact: ConfigImpact::Low,
            category: ConfigCategory::PartialExit,
        })]
        partial_exit_max_pct: f64 = 90.0,

        // ==================== TRAILING STOP CONFIGURATION ====================
        #[metadata(field_metadata! {
            impact: ConfigImpact::High,
            category: ConfigCategory::TrailingStop,
        })]
        trailing_stop_enabled: bool = false,
        #[metadata(field_metadata! {
            min: 0,
            max: 100,
            step: 1,
            impact: ConfigImpact::High,
            category: ConfigCategory::TrailingStop,
        })]
        trailing_stop_activation_pct: f64 = 10.0,
        #[metadata(field_metadata! {
            min: 1,
            max: 50,
            step: 1,
            impact: ConfigImpact::Critical,
            category: ConfigCategory::TrailingStop,
        })]
        trailing_stop_distance_pct: f64 = 5.0,
    }
}
