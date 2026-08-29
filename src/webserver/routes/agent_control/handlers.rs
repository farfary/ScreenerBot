//! Shared agent-control handlers (`/api/agent-control`): the tool list and the
//! per-category permission policy that the dashboard assistant, scheduled
//! automation and the MCP adapter all read.

use axum::{extract::State, http::StatusCode, response::Response, Json};
use std::sync::Arc;

use crate::agent_control::permissions::ToolPermissions;
use crate::assistant::try_get_chat_engine;
use crate::config::{update_config_section, with_config};
use crate::logger::{self, LogTag};
use crate::webserver::state::AppState;
use crate::webserver::utils::{error_response, success_response};

/// GET /api/agent-control/tools - List available tools
pub async fn list_tools(State(_state): State<Arc<AppState>>) -> Response {
    // The registry is the same regardless of transport; requiring the chat
    // engine keeps this endpoint honest about the assistant being available.
    if try_get_chat_engine().is_none() {
        return error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "CHAT_NOT_INITIALIZED",
            "Chat engine not initialized",
            None,
        );
    }

    let registry = crate::agent_control::create_tool_registry();
    success_response(registry.list_definitions())
}

/// GET /api/agent-control/permissions - Get the tool permission policy
pub async fn get_permissions(State(_state): State<Arc<AppState>>) -> Response {
    let permissions = with_config(|cfg| ToolPermissions {
        analysis: crate::agent_control::PermissionLevel::from_str(&cfg.agent_control.analysis),
        portfolio: crate::agent_control::PermissionLevel::from_str(&cfg.agent_control.portfolio),
        trading: crate::agent_control::PermissionLevel::from_str(&cfg.agent_control.trading),
        config: crate::agent_control::PermissionLevel::from_str(&cfg.agent_control.config),
        system: crate::agent_control::PermissionLevel::from_str(&cfg.agent_control.system),
    });

    success_response(permissions)
}

/// PATCH /api/agent-control/permissions - Update the tool permission policy
pub async fn update_permissions(
    State(_state): State<Arc<AppState>>,
    Json(req): Json<ToolPermissions>,
) -> Response {
    match update_config_section(
        |cfg| {
            cfg.agent_control.analysis = req.analysis.to_str().to_string();
            cfg.agent_control.portfolio = req.portfolio.to_str().to_string();
            cfg.agent_control.trading = req.trading.to_str().to_string();
            cfg.agent_control.config = req.config.to_str().to_string();
            cfg.agent_control.system = req.system.to_str().to_string();
        },
        true,
    ) {
        Ok(()) => {
            logger::info(LogTag::Api, "Updated agent-control tool permissions");
            success_response(serde_json::json!({
                "message": "Tool permissions updated successfully"
            }))
        }
        Err(e) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "CONFIG_ERROR",
            &format!("Failed to update permissions: {e}"),
            None,
        ),
    }
}
