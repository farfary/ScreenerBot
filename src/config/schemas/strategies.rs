// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Trading strategy conditions and template configuration.

use crate::config::metadata::{ConfigCategory, ConfigImpact};
use crate::config_struct;
use crate::field_metadata;

config_struct! {
    /// Strategies configuration
    pub struct StrategiesConfig {
        /// Enable strategy system
        #[metadata(field_metadata! {
            impact: ConfigImpact::High,
            category: ConfigCategory::General,
        })]
        enabled: bool = true,

        /// Evaluation timeout in milliseconds
        #[metadata(field_metadata! {
            min: 10,
            max: 1000,
            step: 10,
            impact: ConfigImpact::Medium,
            category: ConfigCategory::Performance,
        })]
        evaluation_timeout_ms: u64 = 50,

        /// Cache TTL in seconds
        #[metadata(field_metadata! {
            min: 1,
            max: 60,
            step: 1,
            impact: ConfigImpact::Low,
            category: ConfigCategory::Performance,
        })]
        cache_ttl_seconds: u64 = 5,

        /// Maximum concurrent strategy evaluations
        #[metadata(field_metadata! {
            min: 1,
            max: 50,
            step: 1,
            impact: ConfigImpact::Medium,
            category: ConfigCategory::Performance,
        })]
        max_concurrent_evaluations: usize = 10,
    }
}
