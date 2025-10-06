use serde::{Deserialize, Serialize};

/// Service health status
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", content = "message")]
pub enum ServiceHealth {
    /// Service is operating normally
    #[serde(rename = "healthy")]
    Healthy,

    /// Service is operating but with degraded performance
    #[serde(rename = "degraded")]
    Degraded(String),

    /// Service has failed
    #[serde(rename = "unhealthy")]
    Unhealthy(String),

    /// Service is starting up
    #[serde(rename = "starting")]
    Starting,

    /// Service is shutting down
    #[serde(rename = "stopping")]
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
