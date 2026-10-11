// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Data retention configuration: how long history is kept and how much disk the app's data may use.

use crate::config::metadata::{ConfigCategory, ConfigImpact};
use crate::field_metadata;

// ============================================================================
// MAINTENANCE CONFIGURATION
// ============================================================================

config_struct! {
    /// Data retention configuration.
    ///
    /// `storage_limit_mb` is the budget storage housekeeping holds the app's data to;
    /// zero adapts it to the disk.
    pub struct MaintenanceConfig {
        #[metadata(field_metadata! {
            impact: ConfigImpact::Medium,
            category: ConfigCategory::Retention,
            min: 0.0,
            step: 1.0,
        })]
        events_retention_days: u32 = 30,

        #[metadata(field_metadata! {
            impact: ConfigImpact::Medium,
            category: ConfigCategory::Retention,
            min: 0.0,
            step: 1.0,
        })]
        actions_retention_days: u32 = 30,

        #[metadata(field_metadata! {
            impact: ConfigImpact::Low,
            category: ConfigCategory::Retention,
            min: 0.0,
            step: 1.0,
        })]
        rpc_stats_retention_hours: u64 = 72,

        #[metadata(field_metadata! {
            impact: ConfigImpact::High,
            category: ConfigCategory::Optimization,
            min: 0.0,
            step: 1.0,
        })]
        stale_token_days: u32 = 7,

        #[metadata(field_metadata! {
            impact: ConfigImpact::Medium,
            category: ConfigCategory::Retention,
            min: 0.0,
            step: 1024.0,
        })]
        storage_limit_mb: u64 = 0,
    }
}
