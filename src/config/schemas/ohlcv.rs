use crate::config_struct;
use crate::field_metadata;

// ============================================================================
// OHLCV DATA MONITORING
// ============================================================================

config_struct! {
    /// OHLCV data monitoring configuration
    pub struct OhlcvConfig {
        /// Enable OHLCV data collection
        #[metadata(field_metadata! {
            label: "Enabled",
            hint: "Enable OHLCV candlestick data collection for technical analysis",
            impact: "high",
            category: "General",
        })]
        enabled: bool = true,
        /// Maximum number of tokens to monitor simultaneously
        #[metadata(field_metadata! {
            label: "Max Monitored Tokens",
            hint: "Maximum tokens to track OHLCV data for (higher uses more memory)",
            min: 10,
            max: 500,
            step: 10,
            unit: "tokens",
            impact: "medium",
            category: "General",
        })]
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
        #[metadata(field_metadata! {
            label: "Empty Response Threshold",
            hint: "Consecutive empty API responses before throttling requests",
            min: 1,
            max: 50,
            step: 1,
            impact: "low",
            category: "General",
        })]
        max_empty_fetches: u32 = 10,
        /// Enable automatic gap filling
        #[metadata(field_metadata! {
            label: "Auto Fetch Gaps",
            hint: "Automatically fetch missing candles when gaps are detected",
            impact: "medium",
            category: "General",
        })]
        auto_fill_gaps: bool = true,
        /// Cache size (maximum number of tokens in hot cache)
        #[metadata(field_metadata! {
            label: "Cache Max Tokens",
            hint: "Maximum tokens to keep in hot memory cache",
            min: 10,
            max: 500,
            step: 10,
            unit: "tokens",
            impact: "medium",
            category: "Cache",
        })]
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
        #[metadata(field_metadata! {
            label: "Enable Fallback",
            hint: "Switch to alternative data source when primary fails",
            impact: "medium",
            category: "Fallback",
        })]
        pool_failover_enabled: bool = true,
        /// Maximum pool failures before switching
        #[metadata(field_metadata! {
            label: "Fallback Threshold",
            hint: "Consecutive failures before switching to backup source",
            min: 1,
            max: 20,
            step: 1,
            impact: "low",
            category: "Fallback",
        })]
        max_pool_failures: u32 = 5,
    }
}
