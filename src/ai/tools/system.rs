use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::{Tool, ToolCategory, ToolDefinition, ToolResult};
use crate::events::{self, EventCategory, Severity};
use crate::global;
use crate::services;

// ============================================================================
// GetStatusTool - System status
// ============================================================================

pub struct GetStatusTool;

#[derive(Serialize)]
struct SystemStatus {
    uptime_seconds: u64,
    initialization_complete: bool,
    core_services_ready: bool,
    force_stopped: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    force_stop_reason: Option<String>,
    open_positions: usize,
    services: Vec<ServiceStatus>,
}

#[derive(Serialize)]
struct ServiceStatus {
    name: String,
    status: String,
    health: String,
}

#[async_trait]
impl Tool for GetStatusTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "get_status".to_string(),
            description:
                "Get current system status including running services, uptime, and health checks."
                    .to_string(),
            category: ToolCategory::System,
            parameters: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
            requires_confirmation: false,
        }
    }

    async fn execute(&self, _params: serde_json::Value) -> ToolResult {
        // Calculate uptime
        let uptime = (chrono::Utc::now() - *global::STARTUP_TIME).num_seconds() as u64;

        // Get force stop info
        let force_stopped = global::is_force_stopped();
        let force_stop_reason = if force_stopped {
            let status = global::get_force_stop_status();
            if !status.reason.is_empty() {
                Some(status.reason)
            } else {
                None
            }
        } else {
            None
        };

        // Get open positions count
        let open_positions = crate::positions::get_open_positions_count().await;

        // Get service health if service manager is available
        let mut service_statuses = Vec::new();

        if let Some(manager_lock) = services::get_service_manager().await {
            let manager_guard = manager_lock.read().await;
            if let Some(manager) = manager_guard.as_ref() {
                let health_map = manager.get_health_cached().await;

                for (name, health) in health_map {
                    service_statuses.push(ServiceStatus {
                        name: name.to_string(),
                        status: "Running".to_string(),
                        health: format!("{:?}", health),
                    });
                }
            }
        }

        let status = SystemStatus {
            uptime_seconds: uptime,
            initialization_complete: global::is_initialization_complete(),
            core_services_ready: global::are_core_services_ready(),
            force_stopped,
            force_stop_reason,
            open_positions,
            services: service_statuses,
        };

        match serde_json::to_value(status) {
            Ok(v) => ToolResult::success(v),
            Err(e) => ToolResult::error(format!("Serialization error: {}", e)),
        }
    }
}

// ============================================================================
// GetEventsTool - Recent system events
// ============================================================================

pub struct GetEventsTool;

#[derive(Deserialize)]
struct GetEventsParams {
    #[serde(default = "default_limit")]
    limit: usize,
    #[serde(default)]
    event_type: Option<String>,
}

fn default_limit() -> usize {
    50
}

#[derive(Serialize)]
struct EventInfo {
    category: String,
    message: String,
    timestamp: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    mint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reference_id: Option<String>,
}

#[async_trait]
impl Tool for GetEventsTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "get_events".to_string(),
            description:
                "Get recent system events and logs including trades, errors, and notifications."
                    .to_string(),
            category: ToolCategory::System,
            parameters: json!({
                "type": "object",
                "properties": {
                    "limit": {
                        "type": "integer",
                        "description": "Maximum number of events to return (default: 50)",
                        "minimum": 1,
                        "maximum": 500
                    },
                    "event_type": {
                        "type": "string",
                        "description": "Filter by event type",
                        "enum": ["trade", "error", "notification", "all"]
                    }
                },
                "required": []
            }),
            requires_confirmation: false,
        }
    }

    async fn execute(&self, params: serde_json::Value) -> ToolResult {
        let params: GetEventsParams = match serde_json::from_value(params) {
            Ok(p) => p,
            Err(e) => return ToolResult::error(format!("Invalid parameters: {}", e)),
        };

        // Validate limit
        let limit = if params.limit > 500 {
            500
        } else if params.limit < 1 {
            50
        } else {
            params.limit
        };

        // Get events based on type filter
        let events = match params.event_type.as_deref() {
            Some("trade") => events::recent(events::EventCategory::Position, limit).await,
            Some("error") => {
                // Filter by severity instead
                let all_events = match events::recent_all(limit).await {
                    Ok(e) => e,
                    Err(err) => return ToolResult::error(format!("Failed to get events: {}", err)),
                };
                Ok(all_events
                    .into_iter()
                    .filter(|e| matches!(e.severity, events::Severity::Error))
                    .collect())
            }
            Some("notification") => events::recent(events::EventCategory::System, limit).await,
            _ => events::recent_all(limit).await,
        };

        let events = match events {
            Ok(e) => e,
            Err(err) => return ToolResult::error(format!("Failed to get events: {}", err)),
        };

        // Convert to serializable format
        let event_infos: Vec<EventInfo> = events
            .iter()
            .map(|e| EventInfo {
                category: e.category.to_string(),
                message: e.payload.to_string(),
                timestamp: e.event_time.to_rfc3339(),
                mint: e.mint.clone(),
                reference_id: e.reference_id.clone(),
            })
            .collect();

        ToolResult::success(json!({
            "events": event_infos,
            "count": event_infos.len()
        }))
    }
}

// ============================================================================
// ForceStopTool - Emergency stop
// ============================================================================

pub struct ForceStopTool;

#[derive(Deserialize)]
struct ForceStopParams {
    reason: String,
}

#[async_trait]
impl Tool for ForceStopTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "force_stop".to_string(),
            description: "Emergency stop all bot services. This will halt trading and monitoring. REQUIRES USER CONFIRMATION.".to_string(),
            category: ToolCategory::System,
            parameters: json!({
                "type": "object",
                "properties": {
                    "reason": {
                        "type": "string",
                        "description": "Reason for emergency stop"
                    }
                },
                "required": ["reason"]
            }),
            requires_confirmation: true,
        }
    }

    async fn execute(&self, params: serde_json::Value) -> ToolResult {
        let params: ForceStopParams = match serde_json::from_value(params) {
            Ok(p) => p,
            Err(e) => return ToolResult::error(format!("Invalid parameters: {}", e)),
        };

        // Set force stop flag
        global::set_force_stopped(true, Some(&params.reason));

        // Log the event
        let _ = events::record(events::Event {
            id: None,
            event_time: chrono::Utc::now(),
            category: events::EventCategory::System,
            subtype: Some("ForceStop".to_string()),
            severity: events::Severity::Warn,
            mint: None,
            reference_id: None,
            payload: serde_json::json!({
                "reason": params.reason,
            }),
            created_at: None,
        })
        .await;

        ToolResult::success(json!({
            "message": "Bot force stopped successfully",
            "reason": params.reason,
            "timestamp": chrono::Utc::now().to_rfc3339()
        }))
    }
}
