//! Shared capability boundary for every agent-facing control surface.
//!
//! This module owns the canonical tool registry (`tools`), the tool permission
//! policy and confirmation manager (`permissions`), and the single
//! authorization decision (`decide`) that the dashboard assistant, scheduled
//! automation and the MCP adapter must all pass a tool through before it can
//! reach a domain owner. Transport adapters never decide whether money-moving
//! work is allowed — they call `decide` and honour the result.

pub mod approvals;
pub mod audit;
pub mod bridge;
pub mod error;
pub mod pairing;
pub mod permissions;
pub mod store;
pub mod tools;

pub use error::{Error, Result};
pub use permissions::{
    check_tool_permission, get_confirmation_manager, get_tool_permissions, ConfirmationManager,
    PermissionLevel, ToolPermissions,
};
pub use tools::{
    create_tool_registry, Tool, ToolCategory, ToolDefinition, ToolRegistry, ToolResult,
};

/// Initialize the durable agent-control store (pairings, approval queue, audit
/// log). Called from dashboard-persistence bootstrap in every boot state that
/// serves the webserver; idempotent.
pub fn init_store() -> Result<()> {
    store::init()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvocationSource {
    Assistant,
    ScheduledReadOnly,
    ScheduledFull,
    Mcp { scope: ClientScope },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ClientScope {
    Read,
    Operate,
    Trade,
}

impl ClientScope {
    /// Parse a stored/requested scope. Only the three exact lowercase tokens
    /// are accepted; anything else returns `None` so scope handling fails
    /// closed rather than defaulting to a capability.
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "read" => Some(ClientScope::Read),
            "operate" => Some(ClientScope::Operate),
            "trade" => Some(ClientScope::Trade),
            _ => None,
        }
    }

    /// The canonical lowercase token for this scope.
    pub fn as_str(self) -> &'static str {
        match self {
            ClientScope::Read => "read",
            ClientScope::Operate => "operate",
            ClientScope::Trade => "trade",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    Execute,
    RequireApproval,
    Deny,
}

/// The canonical authorization decision for an agent-facing tool.
pub fn decide(definition: &ToolDefinition, source: InvocationSource) -> Decision {
    let category_permission = check_tool_permission(&definition.category);
    if category_permission == PermissionLevel::Deny {
        return Decision::Deny;
    }

    let is_trade = definition.category == ToolCategory::Trading;
    match source {
        InvocationSource::ScheduledReadOnly => {
            if is_read_only(definition) {
                Decision::Execute
            } else {
                Decision::Deny
            }
        }
        InvocationSource::ScheduledFull => {
            if category_permission == PermissionLevel::AskUser && !definition.requires_confirmation
            {
                Decision::RequireApproval
            } else {
                Decision::Execute
            }
        }
        InvocationSource::Mcp { scope } => {
            if required_scope(definition) > scope {
                return Decision::Deny;
            }
            // External trades are never auto-approved, even when a user has
            // enabled scheduled Full automation or category Allow.
            if is_trade
                || definition.requires_confirmation
                || category_permission == PermissionLevel::AskUser
            {
                Decision::RequireApproval
            } else {
                Decision::Execute
            }
        }
        InvocationSource::Assistant => {
            if definition.requires_confirmation || category_permission == PermissionLevel::AskUser {
                Decision::RequireApproval
            } else {
                Decision::Execute
            }
        }
    }
}

pub fn required_scope(definition: &ToolDefinition) -> ClientScope {
    match definition.category {
        ToolCategory::Analysis | ToolCategory::Portfolio => ClientScope::Read,
        ToolCategory::Trading => ClientScope::Trade,
        ToolCategory::Config | ToolCategory::System => {
            if definition.requires_confirmation {
                ClientScope::Operate
            } else {
                ClientScope::Read
            }
        }
    }
}

pub fn is_read_only(definition: &ToolDefinition) -> bool {
    !definition.requires_confirmation
        && matches!(
            definition.category,
            ToolCategory::Analysis | ToolCategory::Portfolio | ToolCategory::System
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    fn tool(category: ToolCategory, confirmation: bool) -> ToolDefinition {
        ToolDefinition {
            name: "test".into(),
            description: String::new(),
            category,
            parameters: serde_json::json!({}),
            requires_confirmation: confirmation,
        }
    }
    #[test]
    fn mcp_trade_is_always_approval_gated() {
        assert_eq!(
            decide(
                &tool(ToolCategory::Trading, true),
                InvocationSource::Mcp {
                    scope: ClientScope::Trade
                }
            ),
            Decision::RequireApproval
        );
    }
    #[test]
    fn read_only_schedule_rejects_mutations() {
        assert_eq!(
            decide(
                &tool(ToolCategory::Config, true),
                InvocationSource::ScheduledReadOnly
            ),
            Decision::Deny
        );
    }

    #[test]
    fn client_scope_parse_fails_closed() {
        assert_eq!(ClientScope::parse("read"), Some(ClientScope::Read));
        assert_eq!(ClientScope::parse("operate"), Some(ClientScope::Operate));
        assert_eq!(ClientScope::parse("trade"), Some(ClientScope::Trade));
        for bad in ["", "Read", "READ", "admin", "read ", "trade,read", "*"] {
            assert_eq!(ClientScope::parse(bad), None, "{bad:?}");
        }
        // Round-trips through the canonical token.
        for scope in [ClientScope::Read, ClientScope::Operate, ClientScope::Trade] {
            assert_eq!(ClientScope::parse(scope.as_str()), Some(scope));
        }
    }
}
