//! Agent-facing config tools — read, describe and change any bot setting.
//!
//! These tools are deliberately schema-driven rather than key-by-key: they
//! operate on dotted paths into the serialized `Config`, so every setting the
//! app has (RPC endpoints, trading parameters, filters, providers) is reachable
//! without a hand-maintained allowlist. Wallet private-key material is the one
//! exception and is enforced in `agent_control::config_access`.

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{json, Value};

use super::{Tool, ToolCategory, ToolDefinition, ToolResult};
use crate::agent_control::config_access;
use crate::agent_control::error::Error;

// ============================================================================
// GetConfigTool - Read configuration
// ============================================================================

pub struct GetConfigTool;

#[derive(Deserialize)]
struct GetConfigParams {
    #[serde(default)]
    path: Option<String>,
}

#[async_trait]
impl Tool for GetConfigTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "get_config".to_owned(),
            description: format!(
                "Read bot configuration. With no arguments returns the entire configuration; \
                 pass a dotted path such as 'rpc', 'rpc.urls', 'rpc.urls.0' or \
                 'trader.trade_size_sol' to read one section or value. Wallet private-key \
                 material is always returned as '{}'.",
                config_access::REDACTED
            ),
            category: ToolCategory::Config,
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Dotted config path to read (omit for the whole configuration)"
                    }
                },
                "required": []
            }),
            mutating: false,
            requires_confirmation: false,
        }
    }

    async fn execute(&self, params: Value) -> ToolResult {
        let params: GetConfigParams = match serde_json::from_value(params) {
            Ok(p) => p,
            Err(e) => return ToolResult::error(format!("Invalid parameters: {e}")),
        };

        match config_access::read(params.path.as_deref()) {
            Ok(value) => ToolResult::success(json!({
                "path": params.path,
                "value": value,
            })),
            Err(e) => ToolResult::error(e.to_string()),
        }
    }
}

// ============================================================================
// DescribeConfigTool - Field metadata for discovery
// ============================================================================

pub struct DescribeConfigTool;

#[derive(Deserialize)]
struct DescribeConfigParams {
    #[serde(default)]
    section: Option<String>,
}

#[async_trait]
impl Tool for DescribeConfigTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "describe_config".to_owned(),
            description: "Describe the configuration schema: every section and field with its \
                          type, label, unit, allowed range and default. Use this to discover \
                          which paths update_config accepts and what a valid value looks like."
                .to_owned(),
            category: ToolCategory::Config,
            parameters: json!({
                "type": "object",
                "properties": {
                    "section": {
                        "type": "string",
                        "description": "Config section to describe (omit for every section)"
                    }
                },
                "required": []
            }),
            mutating: false,
            requires_confirmation: false,
        }
    }

    async fn execute(&self, params: Value) -> ToolResult {
        let params: DescribeConfigParams = match serde_json::from_value(params) {
            Ok(p) => p,
            Err(e) => return ToolResult::error(format!("Invalid parameters: {e}")),
        };

        match config_access::schema(params.section.as_deref()) {
            Ok(value) => ToolResult::success(json!({
                "section": params.section,
                "schema": value,
            })),
            Err(e) => ToolResult::error(e.to_string()),
        }
    }
}

// ============================================================================
// UpdateConfigTool - Change configuration
// ============================================================================

pub struct UpdateConfigTool;

#[derive(Deserialize)]
struct UpdateConfigParams {
    #[serde(default)]
    path: Option<String>,
    #[serde(default)]
    value: Option<Value>,
    #[serde(default)]
    updates: Option<Value>,
}

#[async_trait]
impl Tool for UpdateConfigTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "update_config".to_owned(),
            description: "Change bot configuration. Either pass a single 'path' plus 'value', or \
                          an 'updates' object mapping several dotted paths to values, which are \
                          applied as one atomic, schema-validated change and saved to disk. \
                          Examples: path 'rpc.urls' value ['https://…'], or path \
                          'trader.trade_size_sol' value 0.05. Wallet private-key material cannot \
                          be read or written here. Settings read once at startup (RPC endpoint \
                          list, webserver binding) take effect on the next launch."
                .to_owned(),
            category: ToolCategory::Config,
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Dotted config path to set, e.g. 'rpc.urls' or 'trader.trade_size_sol'"
                    },
                    "value": {
                        "description": "New value for 'path'; must match the schema type of that field"
                    },
                    "updates": {
                        "type": "object",
                        "description": "Map of dotted config path to new value, applied atomically"
                    }
                },
                "required": []
            }),
            mutating: true,
            requires_confirmation: true,
        }
    }

    async fn execute(&self, params: Value) -> ToolResult {
        let params: UpdateConfigParams = match serde_json::from_value(params) {
            Ok(p) => p,
            Err(e) => return ToolResult::error(format!("Invalid parameters: {e}")),
        };

        let updates = match collect_updates(params) {
            Ok(updates) => updates,
            Err(e) => return ToolResult::error(e.to_string()),
        };

        match config_access::apply(&updates) {
            Ok(applied) => ToolResult::success(json!({
                "message": format!("Applied {} configuration change(s)", applied.len()),
                "changes": applied,
                "saved_to_disk": true,
            })),
            Err(e) => ToolResult::error(e.to_string()),
        }
    }
}

/// Accept either the single `path`/`value` form or the batch `updates` object,
/// and reject the ambiguous or half-specified combinations outright.
fn collect_updates(
    params: UpdateConfigParams,
) -> crate::agent_control::error::Result<Vec<(String, Value)>> {
    let invalid = |detail: &str| Error::InvalidParameters {
        detail: detail.to_owned(),
    };

    let single = match (params.path, params.value) {
        (Some(path), Some(value)) => Some((path, value)),
        (Some(_), None) => return Err(invalid("'path' was supplied without a 'value'")),
        (None, Some(_)) => return Err(invalid("'value' was supplied without a 'path'")),
        (None, None) => None,
    };

    let batch = match params.updates {
        Some(Value::Object(object)) if !object.is_empty() => {
            Some(config_access::updates_from_object(&object))
        }
        Some(Value::Object(_)) | None => None,
        Some(_) => return Err(invalid("'updates' must be an object of path -> value")),
    };

    match (single, batch) {
        (Some(_), Some(_)) => Err(invalid(
            "supply either 'path' and 'value', or 'updates' — not both",
        )),
        (Some(single), None) => Ok(vec![single]),
        (None, Some(batch)) => Ok(batch),
        (None, None) => Err(invalid(
            "supply 'path' and 'value', or an 'updates' object of path -> value",
        )),
    }
}
