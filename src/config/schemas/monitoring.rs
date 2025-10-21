use crate::config_struct;
use crate::field_metadata;

// ============================================================================
// MONITORING CONFIGURATION
// ============================================================================

config_struct! {
    /// System monitoring configuration
    pub struct MonitoringConfig {
        #[metadata(field_metadata! {
            label: "Metrics Interval",
            hint: "Seconds between metrics sampling",
            min: 5,
            max: 300,
            step: 5,
            unit: "seconds",
            impact: "critical",
            category: "Monitoring",
        })]
        metrics_interval_secs: u64 = 30,
        #[metadata(field_metadata! {
            label: "Health Check Interval",
            hint: "Seconds between service health checks",
            min: 5,
            max: 600,
            step: 5,
            unit: "seconds",
            impact: "critical",
            category: "Monitoring",
        })]
        health_check_interval_secs: u64 = 60,
    }
}
