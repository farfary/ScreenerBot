// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Database maintenance, cleanup, and retention policy configuration.

use crate::config::metadata::{ConfigCategory, ConfigImpact};
use crate::config_struct;
use crate::field_metadata;

// ============================================================================
// MAINTENANCE CONFIGURATION
// ============================================================================

config_struct! {
    /// Automatic maintenance and data retention configuration.
    ///
    /// Controls how long historical data is kept and when heavy
    /// maintenance operations (VACUUM, WAL checkpoint) run.
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
            impact: ConfigImpact::Medium,
            category: ConfigCategory::Retention,
            min: 0.0,
            step: 1.0,
        })]
        ohlcv_retention_days: u32 = 90,

        #[metadata(field_metadata! {
            impact: ConfigImpact::High,
            category: ConfigCategory::Optimization,
            min: 0.0,
            step: 1.0,
        })]
        stale_token_days: u32 = 7,

        #[metadata(field_metadata! {
            impact: ConfigImpact::Low,
            category: ConfigCategory::Database,
            min: 300.0,
            step: 300.0,
        })]
        wal_checkpoint_interval_secs: u64 = 3600,

        #[metadata(field_metadata! {
            impact: ConfigImpact::Low,
            category: ConfigCategory::Database,
            min: 3600.0,
            step: 3600.0,
        })]
        vacuum_interval_secs: u64 = 86400,

        #[metadata(field_metadata! {
            impact: ConfigImpact::Low,
            category: ConfigCategory::Scheduling,
        })]
        maintenance_window_start: String = String::new(),

        #[metadata(field_metadata! {
            impact: ConfigImpact::Medium,
            category: ConfigCategory::Scheduling,
        })]
        skip_during_active_trades: bool = true,
    }
}
