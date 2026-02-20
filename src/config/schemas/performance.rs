use crate::config_struct;

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
        memory_profile: String = String::from("auto"),

        /// SQLite cache size multiplier (0 = use profile default).
        /// Applied to the per-database cache_size preset.
        sqlite_cache_multiplier: f64 = 0.0,

        /// Maximum tokens held in filtering snapshot (0 = unlimited).
        max_filter_tokens: usize = 0,

        /// Filtering refresh interval in seconds (0 = use profile default).
        /// Profile defaults: low=300, medium=180, high=120.
        filtering_refresh_secs: u64 = 0,

        /// Dashboard poll interval in seconds (0 = use profile default).
        /// Profile defaults: low=15, medium=10, high=5.
        dashboard_poll_secs: u64 = 0,
    }
}
