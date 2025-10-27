use super::types::{EndpointCriticality, FallbackStrategy, HealthCheckResult};
use async_trait::async_trait;

/// Trait for endpoint health monitoring
///
/// Implement this trait for each endpoint that needs health monitoring.
/// The ConnectivityService will periodically call check_health() and
/// update the global health state accordingly.
#[async_trait]
pub trait EndpointMonitor: Send + Sync {
    /// Unique identifier for this endpoint
    fn name(&self) -> &'static str;

    /// Criticality level determines system behavior when endpoint fails
    fn criticality(&self) -> EndpointCriticality;

    /// Fallback strategy when endpoint is unavailable
    fn fallback_strategy(&self) -> Option<FallbackStrategy>;

    /// Check if monitoring is enabled for this endpoint
    fn is_enabled(&self) -> bool {
        true
    }

    /// Perform health check for this endpoint
    ///
    /// This method should:
    /// - Be fast (complete within timeout_secs from config)
    /// - Return HealthCheckResult with success/failure status
    /// - Measure latency for healthy responses
    /// - Provide detailed error messages for failures
    async fn check_health(&self) -> HealthCheckResult;

    /// Get human-readable description of this endpoint
    fn description(&self) -> &'static str {
        self.name()
    }
}
