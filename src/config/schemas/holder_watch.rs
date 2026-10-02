// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Holder Watch tool configuration for tracking token holder changes.

use crate::config::metadata::ConfigCategory;
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
            category: ConfigCategory::General,
        })]
        enabled: bool = false,

        /// Check interval in seconds for holder updates
        #[metadata(field_metadata! {
            category: ConfigCategory::Timing,
            min: 10.0,
            max: 3600.0,
            step: 10.0,
        })]
        check_interval_secs: i32 = 60,

        /// Notify via Telegram when new holders are detected
        #[metadata(field_metadata! {
            category: ConfigCategory::Notifications,
        })]
        notify_new_holders: bool = true,

        /// Notify via Telegram when holder count drops significantly
        #[metadata(field_metadata! {
            category: ConfigCategory::Notifications,
        })]
        notify_holder_drop: bool = true,

        /// Minimum holder count change to trigger notification
        #[metadata(field_metadata! {
            category: ConfigCategory::Thresholds,
            min: 1.0,
            max: 1000.0,
            step: 1.0,
        })]
        min_holder_change: i32 = 5,

        /// Percentage drop in holders to trigger drop alert
        #[metadata(field_metadata! {
            category: ConfigCategory::Thresholds,
            min: 1.0,
            max: 100.0,
            step: 0.5,
        })]
        holder_drop_percent: f64 = 10.0,

        /// Maximum tokens to watch simultaneously
        #[metadata(field_metadata! {
            category: ConfigCategory::Limits,
            min: 1.0,
            max: 100.0,
            step: 1.0,
        })]
        max_watched_tokens: i32 = 20,
    }
}
