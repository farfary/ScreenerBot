//! Runtime performance tuning — thread pools, batch sizes, and concurrency limits.

use crate::config::metadata::{ConfigCategory, ConfigImpact};
use crate::config_struct;
use crate::field_metadata;

// ============================================================================
// PERFORMANCE CONFIGURATION
// ============================================================================

config_struct! {
    /// Performance tuning configuration.
    ///
    /// Controls memory profile selection and SQLite/cache sizing.
    /// The `memory_profile` field selects a preset; individual fields
    /// override the preset when non-zero.
    pub struct PerformanceConfig {
        /// Memory profile: "auto" detects from available RAM,
        /// or choose "low" (<4 GB), "medium" (4-8 GB), "high" (>8 GB).
        #[metadata(field_metadata! {
            impact: ConfigImpact::High,
            category: ConfigCategory::Performance,
        })]
        memory_profile: String = "auto".to_owned(),

        /// SQLite cache size multiplier (0 = use profile default).
        /// Applied to the per-database cache_size preset.
        #[metadata(field_metadata! {
            impact: ConfigImpact::Medium,
            category: ConfigCategory::Performance,
            min: 0.0,
            step: 0.1,
        })]
        sqlite_cache_multiplier: f64 = 0.0,

        /// Maximum tokens held in filtering snapshot (0 = unlimited).
        #[metadata(field_metadata! {
            impact: ConfigImpact::High,
            category: ConfigCategory::Performance,
            min: 0.0,
            step: 1000.0,
        })]
        max_filter_tokens: usize = 0,

        /// Filtering refresh interval in seconds (0 = use profile default).
        /// Profile defaults: low=300, medium=180, high=120.
        #[metadata(field_metadata! {
            impact: ConfigImpact::Medium,
            category: ConfigCategory::Performance,
            min: 0.0,
            step: 10.0,
        })]
        filtering_refresh_secs: u64 = 0,

        /// Dashboard poll interval in seconds (0 = use profile default).
        /// Profile defaults: low=15, medium=10, high=5.
        #[metadata(field_metadata! {
            impact: ConfigImpact::Low,
            category: ConfigCategory::Performance,
            min: 0.0,
            step: 1.0,
        })]
        dashboard_poll_secs: u64 = 0,
    }
}
