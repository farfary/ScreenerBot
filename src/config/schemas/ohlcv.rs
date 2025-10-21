use crate::config_struct;
use crate::field_metadata;

// ============================================================================
// OHLCV DATA MONITORING
// ============================================================================

config_struct! {
    /// OHLCV data monitoring configuration
    pub struct OhlcvConfig {
        /// Enable OHLCV data collection
        enabled: bool = true,
        /// Maximum number of tokens to monitor simultaneously
        max_monitored_tokens: usize = 100,
        /// Data retention period in days
        #[metadata(field_metadata! {
            label: "Retention Days",
            hint: "Days to retain historical OHLCV data",
            min: 1,
            max: 30,
            step: 1,
            unit: "days",
            impact: "critical",
            category: "Retention",
        })]
        retention_days: i64 = 7,
        /// Maximum consecutive empty fetches before throttling
        max_empty_fetches: u32 = 10,
        /// Enable automatic gap filling
        auto_fill_gaps: bool = true,
        /// Cache size (maximum number of tokens in hot cache)
        cache_size: usize = 100,
        /// Cache retention hours (for hot cache)
        #[metadata(field_metadata! {
            label: "Cache Retention",
            hint: "Hours to keep tokens in hot cache",
            min: 1,
            max: 168,
            step: 1,
            unit: "hours",
            impact: "critical",
            category: "Cache",
        })]
        cache_retention_hours: i64 = 24,

        /// Enable pool failover
        pool_failover_enabled: bool = true,
        /// Maximum pool failures before switching
        max_pool_failures: u32 = 5,
    }
}
