use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

// Import tool implementations
mod analysis;
mod config;
mod portfolio;
mod system;
mod trading;

use analysis::{AnalyzeTokenTool, CheckSecurityTool, GetMarketDataTool};
use config::{GetConfigTool, UpdateConfigTool};
use portfolio::{GetBalanceTool, GetPnLTool, GetPositionTool, GetPositionsTool};
use system::{ForceStopTool, GetEventsTool, GetStatusTool};
use trading::{BuyTokenTool, ClosePositionTool, SellTokenTool};

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

/// Definition of a tool that can be called by the AI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub category: ToolCategory,
    /// JSON Schema for parameters (OpenAI/Claude format)
    pub parameters: serde_json::Value,
    /// Whether this tool requires user confirmation before execution
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
    pub fn success(data: serde_json::Value) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message.into()),
        }
    }
}

/// Trait for implementing a tool that can be called by the AI
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
        self.tools.values().map(|t| t.definition()).collect()
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
            grouped
                .entry(def.category.clone())
                .or_insert_with(Vec::new)
                .push(def);
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
    registry.register(Arc::new(SellTokenTool));
    registry.register(Arc::new(ClosePositionTool));

    // Config tools
    registry.register(Arc::new(GetConfigTool));
    registry.register(Arc::new(UpdateConfigTool));

    // System tools
    registry.register(Arc::new(GetStatusTool));
    registry.register(Arc::new(GetEventsTool));
    registry.register(Arc::new(ForceStopTool));

    registry
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_registry_creation() {
        let registry = create_tool_registry();
        let definitions = registry.list_definitions();

        // Should have all 16 tools
        assert_eq!(definitions.len(), 16);

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
        assert_eq!(tools.len(), 16);

        // Check format
        let first_tool = &tools[0];
        assert_eq!(first_tool["type"], "function");
        assert!(first_tool["function"]["name"].is_string());
        assert!(first_tool["function"]["description"].is_string());
        assert!(first_tool["function"]["parameters"].is_object());
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
