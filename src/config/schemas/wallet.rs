// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Wallet monitoring and caching configuration.

use crate::config::metadata::{ConfigCategory, ConfigImpact};
use crate::config_struct;
use crate::field_metadata;

config_struct! {
    /// Wallet monitoring and caching configuration
    pub struct WalletConfig {
        #[metadata(field_metadata! {
            min: 10,
            max: 600,
            step: 5,
            impact: ConfigImpact::Medium,
            category: ConfigCategory::Wallet,
        })]
        snapshot_interval_secs: u64 = 15,

        #[metadata(field_metadata! {
            min: 1,
            max: 60,
            step: 1,
            impact: ConfigImpact::High,
            category: ConfigCategory::Wallet,
        })]
        flow_cache_update_secs: u64 = 5,

        #[metadata(field_metadata! {
            min: 100,
            max: 20000,
            step: 100,
            impact: ConfigImpact::Medium,
            category: ConfigCategory::Wallet,
        })]
        flow_cache_backfill_batch: usize = 2000,

        #[metadata(field_metadata! {
            min: 0,
            max: 86400,
            step: 60,
            impact: ConfigImpact::Medium,
            category: ConfigCategory::Wallet,
        })]
        flow_cache_lookback_secs: u64 = 3600,

        #[metadata(field_metadata! {
            min: 30,
            max: 1825,
            step: 30,
            impact: ConfigImpact::Medium,
            category: ConfigCategory::Wallet,
        })]
        max_daily_flow_days: usize = 730,

        #[metadata(field_metadata! {
            min: 30,
            max: 730,
            step: 30,
            impact: ConfigImpact::Low,
            category: ConfigCategory::Wallet,
        })]
        daily_flow_decimate_threshold_days: usize = 365,

        #[metadata(field_metadata! {
            min: 30,
            max: 300,
            step: 10,
            impact: ConfigImpact::High,
            category: ConfigCategory::Wallet,
        })]
        dashboard_metrics_24h_interval_secs: u64 = 60,

        #[metadata(field_metadata! {
            min: 60,
            max: 600,
            step: 30,
            impact: ConfigImpact::Medium,
            category: ConfigCategory::Wallet,
        })]
        dashboard_metrics_7d_interval_secs: u64 = 300,

        #[metadata(field_metadata! {
            min: 300,
            max: 1800,
            step: 60,
            impact: ConfigImpact::Medium,
            category: ConfigCategory::Wallet,
        })]
        dashboard_metrics_30d_interval_secs: u64 = 900,

        #[metadata(field_metadata! {
            min: 600,
            max: 3600,
            step: 60,
            impact: ConfigImpact::Low,
            category: ConfigCategory::Wallet,
        })]
        dashboard_metrics_alltime_interval_secs: u64 = 1800,

        #[metadata(field_metadata! {
            min: 10,
            max: 300,
            step: 10,
            impact: ConfigImpact::Low,
            category: ConfigCategory::Wallet,
        })]
        api_response_cache_ttl_secs: u64 = 30,

        #[metadata(field_metadata! {
            min: 0.001,
            max: 1.0,
            step: 0.001,
            impact: ConfigImpact::High,
            category: ConfigCategory::Safety,
        })]
        min_balance_sol: f64 = 0.01,

        #[metadata(field_metadata! {
            impact: ConfigImpact::High,
            category: ConfigCategory::Watch,
        })]
        watch_enabled: bool = true,

        #[metadata(field_metadata! {
            min: 1,
            max: 50,
            step: 1,
            impact: ConfigImpact::Medium,
            category: ConfigCategory::Watch,
        })]
        watch_max_targets: usize = 10,

        #[metadata(field_metadata! {
            min: 5,
            max: 300,
            step: 5,
            impact: ConfigImpact::Medium,
            category: ConfigCategory::Watch,
        })]
        watch_poll_interval_secs: u64 = 30,

        #[metadata(field_metadata! {
            min: 1,
            max: 60,
            step: 1,
            impact: ConfigImpact::Medium,
            category: ConfigCategory::Watch,
        })]
        watch_poll_fallback_secs: u64 = 3,

        #[metadata(field_metadata! {
            min: 1,
            max: 60,
            step: 1,
            impact: ConfigImpact::Medium,
            category: ConfigCategory::Watch,
        })]
        watch_high_activity_interval_secs: u64 = 5,

        #[metadata(field_metadata! {
            min: 1,
            max: 365,
            step: 1,
            impact: ConfigImpact::Low,
            category: ConfigCategory::Watch,
        })]
        watch_retention_days: u32 = 30,
    }
}
