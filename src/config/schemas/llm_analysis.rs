//! Model-scored token/trading analysis configuration.
//!
//! Owns the filtering/entry/exit analysis feature flags, the auto-blacklist
//! and background-check settings, and the analysis rate limit and cache TTL.
//! Provider credentials live in `llm`.

use crate::config_struct;
use crate::field_metadata;

config_struct! {
    /// Model-scored analysis for filtering and trading decisions.
    pub struct LlmAnalysisConfig {
        // === Filtering ===
        /// Score tokens during filtering.
        #[metadata(field_metadata! {
            label: "Filtering Analysis",
            hint: "Use model analysis to score tokens on metadata, social signals and risk factors during filtering",
            category: "Filtering",
        })]
        filtering_enabled: bool = false,

        /// Minimum confidence to pass filtering (0-100%).
        #[metadata(field_metadata! {
            label: "Min Confidence",
            hint: "Minimum confidence score (0-100%) to pass filtering. Higher = stricter",
            min: 0,
            max: 100,
            step: 5,
            unit: "%",
            category: "Filtering",
        })]
        min_confidence: u8 = 70,

        /// Pass tokens when analysis fails or is unavailable.
        #[metadata(field_metadata! {
            label: "Fallback Pass on Failure",
            hint: "If true, tokens pass filtering when analysis fails/errors. If false, tokens fail when analysis is unavailable",
            category: "Filtering",
        })]
        fallback_pass: bool = false,

        /// Cache filtering evaluations.
        #[metadata(field_metadata! {
            label: "Cache Filtering Results",
            hint: "Cache filtering evaluations to reduce API calls and cost",
            category: "Filtering",
        })]
        use_cache: bool = true,

        // === Trading ===
        /// Analyze tokens before opening positions.
        #[metadata(field_metadata! {
            label: "Entry Analysis",
            hint: "Analyze tokens before opening positions",
            category: "Trading",
        })]
        entry_analysis_enabled: bool = false,

        /// Analyze open positions when deciding to exit.
        #[metadata(field_metadata! {
            label: "Exit Analysis",
            hint: "Use analysis to help decide when to exit positions",
            category: "Trading",
        })]
        exit_analysis_enabled: bool = false,

        /// Adjust trailing stop levels dynamically from analysis.
        #[metadata(field_metadata! {
            label: "Dynamic Trailing Stop",
            hint: "Adjust trailing stop levels dynamically based on analysis of market conditions",
            category: "Trading",
        })]
        trailing_stop_enabled: bool = false,

        /// Always get fresh analysis for trading decisions.
        #[metadata(field_metadata! {
            label: "Trading Bypass Cache",
            hint: "Always get fresh analysis for trading decisions (recommended for accuracy)",
            category: "Trading",
        })]
        trading_bypass_cache: bool = true,

        // === Auto Blacklist ===
        /// Blacklist tokens analysis identifies as high-risk scams.
        #[metadata(field_metadata! {
            label: "Auto Blacklist",
            hint: "Automatically blacklist tokens that analysis identifies as high-risk scams",
            category: "Auto Blacklist",
        })]
        auto_blacklist_enabled: bool = false,

        /// Minimum confidence to auto-blacklist (0-100%).
        #[metadata(field_metadata! {
            label: "Auto Blacklist Min Confidence",
            hint: "Minimum confidence (0-100%) that a token is a scam before auto-blacklisting. Higher = fewer false positives",
            min: 0,
            max: 100,
            step: 5,
            unit: "%",
            category: "Auto Blacklist",
        })]
        auto_blacklist_min_confidence: u8 = 90,

        // === Background Check ===
        /// Periodically re-evaluate open positions.
        #[metadata(field_metadata! {
            label: "Background Check",
            hint: "Periodically re-evaluate open positions in the background",
            category: "Background Check",
        })]
        background_check_enabled: bool = false,

        /// Interval between background checks.
        #[metadata(field_metadata! {
            label: "Background Check Interval",
            hint: "How often to re-check open positions",
            min: 60,
            max: 3600,
            step: 60,
            unit: "seconds",
            category: "Background Check",
        })]
        background_check_interval_seconds: u64 = 300,

        /// Positions to check per batch.
        #[metadata(field_metadata! {
            label: "Background Batch Size",
            hint: "How many positions to check in each background batch",
            min: 1,
            max: 20,
            step: 1,
            category: "Background Check",
        })]
        background_batch_size: u32 = 5,

        // === Rate Limits ===
        /// Global evaluations-per-minute limit.
        #[metadata(field_metadata! {
            label: "Max Evaluations/Minute",
            hint: "Global rate limit for analysis evaluations across all features",
            min: 1,
            max: 100,
            step: 5,
            unit: "requests/min",
            category: "Rate Limits",
        })]
        max_evaluations_per_minute: u32 = 10,

        // === Performance ===
        /// Cache TTL for analysis results.
        #[metadata(field_metadata! {
            label: "Cache TTL",
            hint: "How long to cache analysis results before re-evaluating",
            min: 60,
            max: 3600,
            step: 60,
            unit: "seconds",
            category: "Performance",
        })]
        cache_ttl_seconds: u64 = 300,
    }
}
