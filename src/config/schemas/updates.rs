// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Automatic update configuration.

use crate::config::metadata::{ConfigCategory, ConfigImpact};
use crate::config_struct;
use crate::field_metadata;

// ============================================================================
// UPDATES CONFIGURATION
// ============================================================================

config_struct! {
    /// How ScreenerBot keeps itself current.
    ///
    /// A release ships two components: the core binary (which also carries the
    /// dashboard) and the Electron desktop shell. Core-only releases install
    /// silently with a backend restart; a release that also changes the shell
    /// needs the operating-system installer to run once.
    pub struct UpdatesConfig {
        #[metadata(field_metadata! {
            impact: ConfigImpact::Medium,
            category: ConfigCategory::Checking,
        })]
        auto_check: bool = true,

        #[metadata(field_metadata! {
            impact: ConfigImpact::Low,
            category: ConfigCategory::Checking,
            min: 1.0,
            max: 168.0,
            step: 1.0,
        })]
        check_interval_hours: u64 = 6,

        #[metadata(field_metadata! {
            impact: ConfigImpact::Medium,
            category: ConfigCategory::Installing,
        })]
        auto_download: bool = true,

        #[metadata(field_metadata! {
            impact: ConfigImpact::High,
            category: ConfigCategory::Installing,
        })]
        auto_install: bool = true,

        #[metadata(field_metadata! {
            impact: ConfigImpact::High,
            category: ConfigCategory::Installing,
        })]
        defer_while_trading: bool = true,

        #[metadata(field_metadata! {
            impact: ConfigImpact::Low,
            category: ConfigCategory::Notifications,
        })]
        notify_telegram: bool = true,
    }
}
