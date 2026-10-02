// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Model-scored token/trading analysis configuration.
//!
//! Owns the filtering/entry/exit analysis feature flags, the auto-blacklist
//! and background-check settings, and the analysis rate limit and cache TTL.
//! Provider credentials live in `llm`.

use crate::config::metadata::ConfigCategory;
use crate::config_struct;
use crate::field_metadata;

config_struct! {
    /// Model-scored analysis for filtering and trading decisions.
    pub struct LlmAnalysisConfig {
        // === Filtering ===
        /// Score tokens during filtering.
        #[metadata(field_metadata! {
            category: ConfigCategory::Filtering,
        })]
        filtering_enabled: bool = false,

        /// Minimum confidence to pass filtering (0-100%).
        #[metadata(field_metadata! {
            min: 0,
            max: 100,
            step: 5,
            category: ConfigCategory::Filtering,
        })]
        min_confidence: u8 = 70,

        /// Pass tokens when analysis fails or is unavailable.
        #[metadata(field_metadata! {
            category: ConfigCategory::Filtering,
        })]
        fallback_pass: bool = false,

        /// Cache filtering evaluations.
        #[metadata(field_metadata! {
            category: ConfigCategory::Filtering,
        })]
        use_cache: bool = true,

        // === Trading ===
        /// Analyze tokens before opening positions.
        #[metadata(field_metadata! {
            category: ConfigCategory::Trading,
        })]
        entry_analysis_enabled: bool = false,

        /// Analyze open positions when deciding to exit.
        #[metadata(field_metadata! {
            category: ConfigCategory::Trading,
        })]
        exit_analysis_enabled: bool = false,

        /// Adjust trailing stop levels dynamically from analysis.
        #[metadata(field_metadata! {
            category: ConfigCategory::Trading,
        })]
        trailing_stop_enabled: bool = false,

        /// Always get fresh analysis for trading decisions.
        #[metadata(field_metadata! {
            category: ConfigCategory::Trading,
        })]
        trading_bypass_cache: bool = true,

        // === Auto Blacklist ===
        /// Blacklist tokens analysis identifies as high-risk scams.
        #[metadata(field_metadata! {
            category: ConfigCategory::AutoBlacklist,
        })]
        auto_blacklist_enabled: bool = false,

        /// Minimum confidence to auto-blacklist (0-100%).
        #[metadata(field_metadata! {
            min: 0,
            max: 100,
            step: 5,
            category: ConfigCategory::AutoBlacklist,
        })]
        auto_blacklist_min_confidence: u8 = 90,

        // === Background Check ===
        /// Periodically re-evaluate open positions.
        #[metadata(field_metadata! {
            category: ConfigCategory::BackgroundCheck,
        })]
        background_check_enabled: bool = false,

        /// Interval between background checks.
        #[metadata(field_metadata! {
            min: 60,
            max: 3600,
            step: 60,
            category: ConfigCategory::BackgroundCheck,
        })]
        background_check_interval_seconds: u64 = 300,

        /// Positions to check per batch.
        #[metadata(field_metadata! {
            min: 1,
            max: 20,
            step: 1,
            category: ConfigCategory::BackgroundCheck,
        })]
        background_batch_size: u32 = 5,

        // === Rate Limits ===
        /// Global evaluations-per-minute limit.
        #[metadata(field_metadata! {
            min: 1,
            max: 100,
            step: 5,
            category: ConfigCategory::RateLimits,
        })]
        max_evaluations_per_minute: u32 = 10,

        // === Performance ===
        /// Cache TTL for analysis results.
        #[metadata(field_metadata! {
            min: 60,
            max: 3600,
            step: 60,
            category: ConfigCategory::Performance,
        })]
        cache_ttl_seconds: u64 = 300,
    }
}
