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
            label: "Memory Profile",
            hint: "Memory profile preset: 'auto' detects from available RAM, or choose 'low' (<4 GB), 'medium' (4-8 GB), 'high' (>8 GB)",
            impact: "high",
            category: "Performance",
        })]
        memory_profile: String = String::from("auto"),

        /// SQLite cache size multiplier (0 = use profile default).
        /// Applied to the per-database cache_size preset.
        #[metadata(field_metadata! {
            label: "SQLite Cache Multiplier",
            hint: "Multiplier for SQLite cache size (0 = use profile default, 1.0 = normal, >1.0 = larger cache)",
            impact: "medium",
            category: "Performance",
            min: 0.0,
            step: 0.1,
        })]
        sqlite_cache_multiplier: f64 = 0.0,

        /// Maximum tokens held in filtering snapshot (0 = unlimited).
        #[metadata(field_metadata! {
            label: "Max Filter Tokens",
            hint: "Maximum number of tokens held in filtering snapshot (0 = unlimited)",
            impact: "high",
            category: "Performance",
            min: 0.0,
            step: 1000.0,
            unit: "tokens",
        })]
        max_filter_tokens: usize = 0,

        /// Filtering refresh interval in seconds (0 = use profile default).
        /// Profile defaults: low=300, medium=180, high=120.
        #[metadata(field_metadata! {
            label: "Filtering Refresh Interval",
            hint: "How often to refresh the filtering snapshot in seconds (0 = use profile default: low=300, medium=180, high=120)",
            impact: "medium",
            category: "Performance",
            min: 0.0,
            step: 10.0,
            unit: "seconds",
        })]
        filtering_refresh_secs: u64 = 0,

        /// Dashboard poll interval in seconds (0 = use profile default).
        /// Profile defaults: low=15, medium=10, high=5.
        #[metadata(field_metadata! {
            label: "Dashboard Poll Interval",
            hint: "How often the dashboard polls for updates in seconds (0 = use profile default: low=15, medium=10, high=5)",
            impact: "low",
            category: "Performance",
            min: 0.0,
            step: 1.0,
            unit: "seconds",
        })]
        dashboard_poll_secs: u64 = 0,
    }
}
