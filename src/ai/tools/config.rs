use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::{Tool, ToolCategory, ToolDefinition, ToolResult};
use crate::config::{get_config_clone, with_config};

// ============================================================================
// GetConfigTool - Read configuration
// ============================================================================

pub struct GetConfigTool;

#[derive(Deserialize)]
struct GetConfigParams {
    #[serde(default)]
    section: Option<String>,
}

#[async_trait]
impl Tool for GetConfigTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "get_config".to_string(),
            description: "Get current bot configuration settings including trading parameters, risk settings, and filters.".to_string(),
            category: ToolCategory::Config,
            parameters: json!({
                "type": "object",
                "properties": {
                    "section": {
                        "type": "string",
                        "description": "Specific config section to retrieve (if not provided, returns all)",
                        "enum": ["trader", "screener", "filters", "telegram", "ai", "services"]
                    }
                },
                "required": []
            }),
            requires_confirmation: false,
        }
    }

    async fn execute(&self, params: serde_json::Value) -> ToolResult {
        let params: GetConfigParams = match serde_json::from_value(params) {
            Ok(p) => p,
            Err(e) => return ToolResult::error(format!("Invalid parameters: {}", e)),
        };

        let config = get_config_clone();

        let result = match params.section.as_deref() {
            Some("trader") => json!({
                "trader": {
                    "enabled": config.trader.enabled,
                    "max_open_positions": config.trader.max_open_positions,
                    "trade_size_sol": config.trader.trade_size_sol,
                }
            }),
            Some("filters") => json!({
                "filters": {
                    "age_enabled": config.filtering.age_enabled,
                    "cooldown_enabled": config.filtering.cooldown_enabled,
                }
            }),
            Some("telegram") => json!({
                "telegram": {
                    "enabled": config.telegram.enabled,
                }
            }),
            Some("ai") => json!({
                "ai": {
                    "enabled": config.ai.enabled,
                    "default_provider": config.ai.default_provider,
                }
            }),
            Some("services") => json!({
                "services": {
                    "note": "Services config is currently empty - use individual service enabled flags"
                }
            }),
            Some(section) => {
                return ToolResult::error(format!("Unknown section: {}", section));
            }
            None => {
                // Return all important sections
                json!({
                    "trader": {
                        "enabled": config.trader.enabled,
                        "max_open_positions": config.trader.max_open_positions,
                        "trade_size_sol": config.trader.trade_size_sol,
                    },
                    "telegram": {
                        "enabled": config.telegram.enabled,
                    },
                    "ai": {
                        "enabled": config.ai.enabled,
                    }
                })
            }
        };

        ToolResult::success(result)
    }
}

// ============================================================================
// UpdateConfigTool - Update configuration
// ============================================================================

pub struct UpdateConfigTool;

#[derive(Deserialize)]
struct UpdateConfigParams {
    section: String,
    key: String,
    value: serde_json::Value,
}

#[async_trait]
impl Tool for UpdateConfigTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "update_config".to_string(),
            description: "Update bot configuration settings. REQUIRES USER CONFIRMATION."
                .to_string(),
            category: ToolCategory::Config,
            parameters: json!({
                "type": "object",
                "properties": {
                    "section": {
                        "type": "string",
                        "description": "Config section to update",
                        "enum": ["trader", "screener", "filters", "telegram", "ai"]
                    },
                    "key": {
                        "type": "string",
                        "description": "Configuration key to update (e.g., 'max_open_positions', 'auto_buy')"
                    },
                    "value": {
                        "description": "New value for the configuration key"
                    }
                },
                "required": ["section", "key", "value"]
            }),
            requires_confirmation: true,
        }
    }

    async fn execute(&self, params: serde_json::Value) -> ToolResult {
        let params: UpdateConfigParams = match serde_json::from_value(params) {
            Ok(p) => p,
            Err(e) => return ToolResult::error(format!("Invalid parameters: {}", e)),
        };

        // Update config based on section and key
        let result = match params.section.as_str() {
            "trader" => update_trader_config(&params.key, params.value),
            "filters" => update_filters_config(&params.key, params.value),
            "telegram" => update_telegram_config(&params.key, params.value),
            "ai" => update_ai_config(&params.key, params.value),
            section => {
                return ToolResult::error(format!("Cannot update section: {}", section));
            }
        };

        match result {
            Ok(msg) => ToolResult::success(json!({
                "message": msg,
                "section": params.section,
                "key": params.key,
            })),
            Err(e) => ToolResult::error(e),
        }
    }
}

// ============================================================================
// Helper functions for config updates
// ============================================================================

fn update_trader_config(key: &str, value: serde_json::Value) -> Result<String, String> {
    match key {
        "enabled" => {
            let val = value.as_bool().ok_or("Value must be true or false")?;
            crate::config::update_config_section(
                |cfg| {
                    cfg.trader.enabled = val;
                },
                true,
            )?;
            Ok(format!("Updated trader enabled to {}", val))
        }
        "max_open_positions" => {
            let val = value.as_u64().ok_or("Value must be a number")?;
            crate::config::update_config_section(
                |cfg| {
                    cfg.trader.max_open_positions = val as usize;
                },
                true,
            )?;
            Ok(format!("Updated max_open_positions to {}", val))
        }
        "trade_size_sol" => {
            let val = value.as_f64().ok_or("Value must be a number")?;
            crate::config::update_config_section(
                |cfg| {
                    cfg.trader.trade_size_sol = val;
                },
                true,
            )?;
            Ok(format!("Updated trade_size_sol to {}", val))
        }
        _ => Err(format!("Unknown trader config key: {}", key)),
    }
}

fn update_filters_config(key: &str, value: serde_json::Value) -> Result<String, String> {
    match key {
        "age_enabled" => {
            let val = value.as_bool().ok_or("Value must be true or false")?;
            crate::config::update_config_section(
                |cfg| {
                    cfg.filtering.age_enabled = val;
                },
                true,
            )?;
            Ok(format!("Updated age filtering enabled to {}", val))
        }
        "cooldown_enabled" => {
            let val = value.as_bool().ok_or("Value must be true or false")?;
            crate::config::update_config_section(
                |cfg| {
                    cfg.filtering.cooldown_enabled = val;
                },
                true,
            )?;
            Ok(format!("Updated cooldown filtering enabled to {}", val))
        }
        _ => Err(format!("Unknown filters config key: {}", key)),
    }
}

fn update_telegram_config(key: &str, value: serde_json::Value) -> Result<String, String> {
    match key {
        "enabled" => {
            let val = value.as_bool().ok_or("Value must be true or false")?;
            crate::config::update_config_section(
                |cfg| {
                    cfg.telegram.enabled = val;
                },
                true,
            )?;
            Ok(format!("Updated telegram enabled to {}", val))
        }
        _ => Err(format!("Unknown telegram config key: {}", key)),
    }
}

fn update_ai_config(key: &str, value: serde_json::Value) -> Result<String, String> {
    match key {
        "enabled" => {
            let val = value.as_bool().ok_or("Value must be true or false")?;
            crate::config::update_config_section(
                |cfg| {
                    cfg.ai.enabled = val;
                },
                true,
            )?;
            Ok(format!("Updated AI enabled to {}", val))
        }
        _ => Err(format!("Unknown AI config key: {}", key)),
    }
}
