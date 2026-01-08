use crate::config_struct;
use crate::field_metadata;

config_struct! {
    /// Strategies configuration
    pub struct StrategiesConfig {
        /// Enable strategy system
        #[metadata(field_metadata! {
            label: "Enabled",
            hint: "Enable custom trading strategy evaluation",
            impact: "high",
            category: "General",
        })]
        enabled: bool = true,

        /// Evaluation timeout in milliseconds
        #[metadata(field_metadata! {
            label: "Evaluation Timeout",
            hint: "Timeout for strategy evaluation (milliseconds)",
            min: 10,
            max: 1000,
            step: 10,
            unit: "ms",
            impact: "medium",
            category: "Performance",
        })]
        evaluation_timeout_ms: u64 = 50,

        /// Cache TTL in seconds
        #[metadata(field_metadata! {
            label: "Cache Duration",
            hint: "Seconds to cache strategy evaluation results",
            min: 1,
            max: 60,
            step: 1,
            unit: "seconds",
            impact: "low",
            category: "Performance",
        })]
        cache_ttl_seconds: u64 = 5,

        /// Maximum concurrent strategy evaluations
        #[metadata(field_metadata! {
            label: "Max Concurrent Evaluations",
            hint: "Maximum strategies to evaluate simultaneously",
            min: 1,
            max: 50,
            step: 1,
            impact: "medium",
            category: "Performance",
        })]
        max_concurrent_evaluations: usize = 10,
    }
}
