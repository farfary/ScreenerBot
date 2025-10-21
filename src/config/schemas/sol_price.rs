use crate::config_struct;
use crate::field_metadata;

// ============================================================================
// SOL PRICE SERVICE
// ============================================================================

config_struct! {
    /// SOL price tracking configuration
    pub struct SolPriceConfig {
        /// Price refresh interval (seconds)
        #[metadata(field_metadata! {
            label: "Price Refresh Interval",
            hint: "Seconds between SOL price updates",
            min: 10,
            max: 300,
            step: 10,
            unit: "seconds",
            impact: "critical",
            category: "Timing",
        })]
        price_refresh_interval_secs: u64 = 30,
    }
}
