use crate::config_struct;

config_struct! {
    /// Strategies configuration
    pub struct StrategiesConfig {
        /// Enable strategy system
        enabled: bool = true,

        /// Evaluation timeout in milliseconds
        evaluation_timeout_ms: u64 = 50,

        /// Cache TTL in seconds
        cache_ttl_seconds: u64 = 5,

        /// Maximum concurrent strategy evaluations
        max_concurrent_evaluations: usize = 10,
    }
}
