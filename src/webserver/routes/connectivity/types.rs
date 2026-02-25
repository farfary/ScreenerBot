use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::connectivity::EndpointHealth;

/// Response for connectivity status overview
#[derive(Debug, Serialize, Deserialize)]
pub struct ConnectivityStatusResponse {
    pub all_healthy: bool,
    pub critical_healthy: bool,
    pub unhealthy_critical_endpoints: Vec<String>,
    pub endpoints: HashMap<String, EndpointHealthResponse>,
}

/// Serializable endpoint health response
#[derive(Debug, Serialize, Deserialize)]
pub struct EndpointHealthResponse {
    pub status: String,
    pub latency_ms: Option<u64>,
    pub message: Option<String>,
    pub last_check: Option<String>,
    pub last_success: Option<String>,
    pub consecutive_failures: Option<u32>,
}

impl From<EndpointHealth> for EndpointHealthResponse {
    fn from(health: EndpointHealth) -> Self {
        match health {
            EndpointHealth::Healthy {
                latency_ms,
                last_check,
            } => Self {
                status: "healthy".to_owned(),
                latency_ms: Some(latency_ms),
                message: None,
                last_check: Some(last_check.to_rfc3339()),
                last_success: Some(last_check.to_rfc3339()),
                consecutive_failures: None,
            },
            EndpointHealth::Degraded {
                latency_ms,
                reason,
                last_check,
            } => Self {
                status: "degraded".to_owned(),
                latency_ms: Some(latency_ms),
                message: Some(reason),
                last_check: Some(last_check.to_rfc3339()),
                last_success: Some(last_check.to_rfc3339()),
                consecutive_failures: None,
            },
            EndpointHealth::Unhealthy {
                reason,
                last_check,
                last_success,
                consecutive_failures,
            } => Self {
                status: "unhealthy".to_owned(),
                latency_ms: None,
                message: Some(reason),
                last_check: Some(last_check.to_rfc3339()),
                last_success: last_success.map(|t| t.to_rfc3339()),
                consecutive_failures: Some(consecutive_failures),
            },
            EndpointHealth::Unknown => Self {
                status: "unknown".to_owned(),
                latency_ms: None,
                message: Some("Not checked yet".to_owned()),
                last_check: None,
                last_success: None,
                consecutive_failures: None,
            },
        }
    }
}
