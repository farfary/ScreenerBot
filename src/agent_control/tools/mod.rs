//! Agent-facing function-calling tool registry: analysis, portfolio, config, system and trading tools.
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

// Import tool implementations
mod analysis;
mod config;
mod copy_trading;
mod copy_trading_workspace;
mod portfolio;
mod system;
mod trader;
mod trading;

use analysis::{AnalyzeTokenTool, CheckSecurityTool, GetMarketDataTool};
use config::{DescribeConfigTool, GetConfigTool, UpdateConfigTool};
use copy_trading::{
    CreateCopyTaskTool, DeleteCopyTaskTool, GetCopyActivityTool, GetCopyTaskTool,
    GetCopyTradingOverviewTool, SetCopyTaskModeTool, UpdateCopyTaskTool,
};
use copy_trading_workspace::{
    CloneCopyTaskTool, CloseCopyPaperHoldingTool, GetCopyInsightsTool, GetCopyWalletProfileTool,
    ResetCopyPaperBookTool,
};
use portfolio::{GetBalanceTool, GetPnLTool, GetPositionTool, GetPositionsTool};
use system::{ClearForceStopTool, ForceStopTool, GetEventsTool, GetStatusTool};
use trader::{
    ApplyTraderTemplateTool, GetTraderStatsTool, GetTraderStatusTool, ListTraderTemplatesTool,
    ManageLossLimitTool, SetTraderEnabledTool, SetTraderMonitorTool,
};
use trading::{AddToPositionTool, BuyTokenTool, ClosePositionTool, SellTokenTool};

/// Category of tool for organization and UI display
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ToolCategory {
    /// Token analysis and market data
    Analysis,
    /// Position info, balance, P&L
    Portfolio,
    /// Buy/sell operations (requires confirmation)
    Trading,
    /// Bot settings and configuration
    Config,
    /// System status and logs
    System,
}

/// Definition of a tool that can be called by an authorized agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub category: ToolCategory,
    /// JSON Schema for parameters (OpenAI/Claude format)
    pub parameters: serde_json::Value,
    /// Whether this tool changes bot state (config, positions, services). Drives
    /// the client scope a paired agent needs and the MCP read-only annotation —
    /// it is a property of the tool, never of the current permission policy.
    pub mutating: bool,
    /// Whether an interactive caller (the dashboard assistant) must confirm this
    /// tool before it runs. Distinct from `mutating`: it is a UX gate, not the
    /// capability boundary.
    pub requires_confirmation: bool,
}

/// Result of a tool execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl ToolResult {
    /// Build a successful tool result with the given JSON data
    pub fn success(data: serde_json::Value) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    /// Build a failed tool result with an error message
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message.into()),
        }
    }
}

/// Trait for implementing a tool that can be called by an authorized agent
#[async_trait]
pub trait Tool: Send + Sync {
    /// Get the tool's definition (name, description, parameters schema)
    fn definition(&self) -> ToolDefinition;

    /// Execute the tool with the given parameters
    async fn execute(&self, params: serde_json::Value) -> ToolResult;
}

/// Registry of all available tools
pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
}

impl ToolRegistry {
    /// Create a new tool registry
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    /// Register a tool in the registry
    pub fn register(&mut self, tool: Arc<dyn Tool>) {
        let name = tool.definition().name.clone();
        self.tools.insert(name, tool);
    }

    /// Get a tool by name
    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools.get(name).cloned()
    }

    /// List all tool definitions
    pub fn list_definitions(&self) -> Vec<ToolDefinition> {
        let mut definitions: Vec<_> = self.tools.values().map(|t| t.definition()).collect();
        definitions.sort_by(|left, right| left.name.cmp(&right.name));
        definitions
    }

    /// Get tools in OpenAI/Claude function calling format
    pub fn get_tools_json_schema(&self) -> serde_json::Value {
        let tools: Vec<serde_json::Value> = self
            .tools
            .values()
            .map(|tool| {
                let def = tool.definition();
                serde_json::json!({
                    "type": "function",
                    "function": {
                        "name": def.name,
                        "description": def.description,
                        "parameters": def.parameters
                    }
                })
            })
            .collect();

        serde_json::json!(tools)
    }

    /// Get tools grouped by category for UI display
    pub fn get_tools_by_category(&self) -> HashMap<ToolCategory, Vec<ToolDefinition>> {
        let mut grouped: HashMap<ToolCategory, Vec<ToolDefinition>> = HashMap::new();

        for tool in self.tools.values() {
            let def = tool.definition();
            grouped.entry(def.category.clone()).or_default().push(def);
        }

        grouped
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Registry Builder
// ============================================================================

/// Create and populate the tool registry with all available tools
pub fn create_tool_registry() -> ToolRegistry {
    let mut registry = ToolRegistry::new();

    // Analysis tools
    registry.register(Arc::new(AnalyzeTokenTool));
    registry.register(Arc::new(GetMarketDataTool));
    registry.register(Arc::new(CheckSecurityTool));

    // Portfolio tools
    registry.register(Arc::new(GetPositionsTool));
    registry.register(Arc::new(GetPositionTool));
    registry.register(Arc::new(GetBalanceTool));
    registry.register(Arc::new(GetPnLTool));

    // Trading tools
    registry.register(Arc::new(BuyTokenTool));
    registry.register(Arc::new(AddToPositionTool));
    registry.register(Arc::new(SellTokenTool));
    registry.register(Arc::new(ClosePositionTool));

    // Config tools
    registry.register(Arc::new(GetConfigTool));
    registry.register(Arc::new(DescribeConfigTool));
    registry.register(Arc::new(UpdateConfigTool));

    // System tools
    registry.register(Arc::new(GetStatusTool));
    registry.register(Arc::new(GetEventsTool));
    registry.register(Arc::new(ForceStopTool));
    registry.register(Arc::new(ClearForceStopTool));

    // Auto-trader tools
    registry.register(Arc::new(GetTraderStatusTool));
    registry.register(Arc::new(GetTraderStatsTool));
    registry.register(Arc::new(SetTraderEnabledTool));
    registry.register(Arc::new(SetTraderMonitorTool));
    registry.register(Arc::new(ManageLossLimitTool));
    registry.register(Arc::new(ListTraderTemplatesTool));
    registry.register(Arc::new(ApplyTraderTemplateTool));

    // Copy-trading tools
    registry.register(Arc::new(GetCopyTradingOverviewTool));
    registry.register(Arc::new(GetCopyTaskTool));
    registry.register(Arc::new(GetCopyActivityTool));
    registry.register(Arc::new(CreateCopyTaskTool));
    registry.register(Arc::new(UpdateCopyTaskTool));
    registry.register(Arc::new(DeleteCopyTaskTool));
    registry.register(Arc::new(SetCopyTaskModeTool));
    registry.register(Arc::new(GetCopyInsightsTool));
    registry.register(Arc::new(GetCopyWalletProfileTool));
    registry.register(Arc::new(CloneCopyTaskTool));
    registry.register(Arc::new(ResetCopyPaperBookTool));
    registry.register(Arc::new(CloseCopyPaperHoldingTool));

    registry
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_registry_creation() {
        let registry = create_tool_registry();
        let definitions = registry.list_definitions();

        // Should have all registered tools
        assert_eq!(definitions.len(), 37);

        // Check that we have tools in each category
        let by_category = registry.get_tools_by_category();
        assert!(by_category.contains_key(&ToolCategory::Analysis));
        assert!(by_category.contains_key(&ToolCategory::Portfolio));
        assert!(by_category.contains_key(&ToolCategory::Trading));
        assert!(by_category.contains_key(&ToolCategory::Config));
        assert!(by_category.contains_key(&ToolCategory::System));
    }

    #[test]
    fn test_tool_retrieval() {
        let registry = create_tool_registry();

        // Should be able to get a tool by name
        let tool = registry.get("analyze_token");
        assert!(tool.is_some());

        for name in [
            "get_copy_trading_overview",
            "get_copy_task",
            "get_copy_activity",
            "get_copy_insights",
            "get_copy_wallet_profile",
            "get_trader_status",
            "get_trader_stats",
            "list_trader_templates",
        ] {
            let def = registry
                .get(name)
                .unwrap_or_else(|| panic!("{name} is registered"))
                .definition();
            assert!(!def.mutating, "{name} only reads");
        }

        let tool = registry.get("nonexistent_tool");
        assert!(tool.is_none());
    }

    #[test]
    fn test_tools_json_schema() {
        let registry = create_tool_registry();
        let schema = registry.get_tools_json_schema();

        // Should be an array
        assert!(schema.is_array());
        let tools = schema.as_array().unwrap();
        assert_eq!(tools.len(), 37);

        // Check format
        let first_tool = &tools[0];
        assert_eq!(first_tool["type"], "function");
        assert!(first_tool["function"]["name"].is_string());
        assert!(first_tool["function"]["description"].is_string());
        assert!(first_tool["function"]["parameters"].is_object());
    }

    /// `mutating` is the capability boundary: every tool that changes state
    /// must declare it, or `required_scope` would hand a read-only pairing a
    /// write. Everything else must declare the opposite, or read-only
    /// automation would refuse plain queries.
    #[test]
    fn mutation_flags_match_what_each_tool_does() {
        let registry = create_tool_registry();
        let mutating: Vec<String> = registry
            .list_definitions()
            .into_iter()
            .filter(|def| def.mutating)
            .map(|def| def.name)
            .collect();
        assert_eq!(
            mutating,
            vec![
                "add_to_position",
                "apply_trader_template",
                "buy_token",
                "clear_force_stop",
                "clone_copy_task",
                "close_copy_paper_holding",
                "close_position",
                "create_copy_task",
                "delete_copy_task",
                "force_stop",
                "manage_loss_limit",
                "reset_copy_paper_book",
                "sell_token",
                "set_copy_task_mode",
                "set_trader_enabled",
                "set_trader_monitor",
                "update_config",
                "update_copy_task",
            ]
        );
    }

    /// Reading configuration is a read: a paired client with `read` scope must
    /// be able to inspect settings without being able to change them.
    #[test]
    fn config_reads_are_not_mutations() {
        let registry = create_tool_registry();
        for name in ["get_config", "describe_config"] {
            let def = registry
                .get(name)
                .unwrap_or_else(|| panic!("{name} is registered"))
                .definition();
            assert!(!def.mutating, "{name} must not be marked mutating");
            assert!(!def.requires_confirmation, "{name} must not need approval");
        }
    }

    #[test]
    fn test_confirmation_requirements() {
        let registry = create_tool_registry();

        // Trading tools should require confirmation
        let buy_tool = registry.get("buy_token").unwrap();
        assert!(buy_tool.definition().requires_confirmation);

        let sell_tool = registry.get("sell_token").unwrap();
        assert!(sell_tool.definition().requires_confirmation);

        // Analysis tools should not require confirmation
        let analyze_tool = registry.get("analyze_token").unwrap();
        assert!(!analyze_tool.definition().requires_confirmation);
    }
}
