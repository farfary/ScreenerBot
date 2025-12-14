//! Holder Watch tool configuration for tracking token holder changes

use crate::config_struct;
use crate::field_metadata;

// ============================================================================
// HOLDER WATCH CONFIGURATION
// ============================================================================

config_struct! {
    /// Configuration for the Holder Watch tool
    pub struct HolderWatchConfig {
        /// Enable holder watching functionality
        #[metadata(field_metadata! {
            label: "Enable Holder Watch",
            hint: "Enable holder tracking for watched tokens",
            category: "General",
        })]
        enabled: bool = false,

        /// Check interval in seconds for holder updates
        #[metadata(field_metadata! {
            label: "Check Interval",
            hint: "How often to check for new holders (in seconds)",
            category: "Timing",
            min: 10.0,
            max: 3600.0,
            step: 10.0,
            unit: "seconds",
        })]
        check_interval_secs: i32 = 60,

        /// Notify via Telegram when new holders are detected
        #[metadata(field_metadata! {
            label: "Notify New Holders",
            hint: "Send Telegram notification when new holders are detected",
            category: "Notifications",
        })]
        notify_new_holders: bool = true,

        /// Notify via Telegram when holder count drops significantly
        #[metadata(field_metadata! {
            label: "Notify Holder Drop",
            hint: "Alert when holder count drops below threshold",
            category: "Notifications",
        })]
        notify_holder_drop: bool = true,

        /// Minimum holder count change to trigger notification
        #[metadata(field_metadata! {
            label: "Min Holder Change",
            hint: "Minimum change in holder count to trigger alert",
            category: "Thresholds",
            min: 1.0,
            max: 1000.0,
            step: 1.0,
        })]
        min_holder_change: i32 = 5,

        /// Percentage drop in holders to trigger drop alert
        #[metadata(field_metadata! {
            label: "Holder Drop Threshold",
            hint: "Percentage drop in holders to trigger alert (e.g., 10.0 = 10%)",
            category: "Thresholds",
            min: 1.0,
            max: 100.0,
            step: 0.5,
            unit: "%",
        })]
        holder_drop_percent: f64 = 10.0,

        /// Maximum tokens to watch simultaneously
        #[metadata(field_metadata! {
            label: "Max Watched Tokens",
            hint: "Maximum number of tokens that can be watched at once",
            category: "Limits",
            min: 1.0,
            max: 100.0,
            step: 1.0,
        })]
        max_watched_tokens: i32 = 20,
    }
}
