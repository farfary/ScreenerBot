/// Service health status
#[derive(Debug, Clone, PartialEq)]
pub enum ServiceHealth {
    /// Service is operating normally
    Healthy,

    /// Service is operating but with degraded performance
    Degraded(String),

    /// Service has failed
    Unhealthy(String),

    /// Service is starting up
    Starting,

    /// Service is shutting down
    Stopping,
}

impl ServiceHealth {
    pub fn is_healthy(&self) -> bool {
        matches!(self, ServiceHealth::Healthy)
    }

    pub fn is_degraded(&self) -> bool {
        matches!(self, ServiceHealth::Degraded(_))
    }

    pub fn is_unhealthy(&self) -> bool {
        matches!(self, ServiceHealth::Unhealthy(_))
    }
}
