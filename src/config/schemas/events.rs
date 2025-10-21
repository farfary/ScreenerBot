use crate::config_struct;
use crate::field_metadata;

// ============================================================================
// EVENTS SYSTEM
// ============================================================================

config_struct! {
    /// Events system configuration
    pub struct EventsConfig {
        /// Batch timeout (milliseconds)
        #[metadata(field_metadata! {
            label: "Batch Timeout",
            hint: "Milliseconds for event batch timeout",
            min: 10,
            max: 1000,
            step: 10,
            unit: "ms",
            impact: "critical",
            category: "Performance",
        })]
        batch_timeout_ms: u64 = 100,
    }
}
