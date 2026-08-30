//! Shared capability boundary for every agent-facing control surface.
//!
//! This module owns the canonical tool registry (`tools`), the tool permission
//! policy and confirmation manager (`permissions`), and the single
//! authorization decision (`decide`) that the dashboard assistant, scheduled
//! automation and the MCP adapter must all pass a tool through before it can
//! reach a domain owner. Transport adapters never decide whether money-moving
//! work is allowed — they call `decide` and honour the result.
//!
//! Who is bounded by what:
//! - A **paired agent connection** carries its own per-category policy, stored
//!   with the pairing. A new connection starts at full access and the owner
//!   limits it per connection in Settings → Agent Connections. Nothing else
//!   narrows it, so what the dashboard shows for a connection is exactly what
//!   that connection can do.
//! - The **in-app assistant and scheduled automation** are bounded by the
//!   `agent_control` config policy, edited on the config page.
//! - Wallet private-key material is outside this policy entirely: no level and
//!   no connection can read or write it (`config_access`).

pub mod approvals;
pub mod audit;
pub mod bridge;
pub mod config_access;
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
    /// A paired external client, carrying that pairing's own policy.
    Mcp {
        permissions: ToolPermissions,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    Execute,
    RequireApproval,
    Deny,
}

/// The policy that governs this invocation: a paired connection brings its own,
/// everything else is the in-app `agent_control` policy.
pub fn effective_permission(category: &ToolCategory, source: InvocationSource) -> PermissionLevel {
    match source {
        InvocationSource::Mcp { permissions } => permissions.get_permission(category),
        _ => check_tool_permission(category),
    }
}

/// The canonical authorization decision for an agent-facing tool.
///
/// `Allow` executes, `AskUser` parks the call for a human decision, `Deny`
/// refuses. `requires_confirmation` is an extra gate for the interactive
/// assistant only — applying it to the other sources would make an `Allow`
/// category impossible to actually automate.
pub fn decide(definition: &ToolDefinition, source: InvocationSource) -> Decision {
    let permission = effective_permission(&definition.category, source);
    if permission == PermissionLevel::Deny {
        return Decision::Deny;
    }

    match source {
        InvocationSource::ScheduledReadOnly => {
            if is_read_only(definition) {
                Decision::Execute
            } else {
                Decision::Deny
            }
        }
        InvocationSource::ScheduledFull | InvocationSource::Mcp { .. } => {
            if permission == PermissionLevel::Allow {
                Decision::Execute
            } else {
                Decision::RequireApproval
            }
        }
        InvocationSource::Assistant => {
            if definition.requires_confirmation || permission == PermissionLevel::AskUser {
                Decision::RequireApproval
            } else {
                Decision::Execute
            }
        }
    }
}

/// True when the tool only observes state. Drives the MCP read-only annotation
/// and what read-only scheduled automation may run. A property of the tool, not
/// of anyone's policy.
pub fn is_read_only(definition: &ToolDefinition) -> bool {
    !definition.mutating
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tool(category: ToolCategory, mutating: bool) -> ToolDefinition {
        ToolDefinition {
            name: "test".into(),
            description: String::new(),
            category,
            parameters: serde_json::json!({}),
            mutating,
            requires_confirmation: mutating,
        }
    }

    fn client(permissions: ToolPermissions) -> InvocationSource {
        InvocationSource::Mcp { permissions }
    }

    fn with_trading(level: PermissionLevel) -> ToolPermissions {
        ToolPermissions {
            trading: level,
            ..ToolPermissions::full_access()
        }
    }

    /// A new connection starts at full access, so it acts without an approval
    /// round-trip — including money-moving work. This is the owner-facing
    /// default the settings dialog then narrows per connection.
    #[test]
    fn a_full_access_connection_executes_every_category() {
        for category in [
            ToolCategory::Analysis,
            ToolCategory::Portfolio,
            ToolCategory::Trading,
            ToolCategory::Config,
            ToolCategory::System,
        ] {
            assert_eq!(
                decide(
                    &tool(category.clone(), true),
                    client(ToolPermissions::full_access())
                ),
                Decision::Execute,
                "{category:?} must execute for a full-access connection"
            );
        }
    }

    /// Limiting one category limits only that category — the point of the
    /// per-connection policy.
    #[test]
    fn limiting_one_category_leaves_the_others_alone() {
        let source = client(with_trading(PermissionLevel::AskUser));
        assert_eq!(
            decide(&tool(ToolCategory::Trading, true), source),
            Decision::RequireApproval
        );
        assert_eq!(
            decide(&tool(ToolCategory::Config, true), source),
            Decision::Execute
        );

        let source = client(with_trading(PermissionLevel::Deny));
        assert_eq!(
            decide(&tool(ToolCategory::Trading, true), source),
            Decision::Deny
        );
        assert_eq!(
            decide(&tool(ToolCategory::Config, true), source),
            Decision::Execute
        );
    }

    /// A connection restricted to reads cannot mutate anything, in any
    /// category, however the in-app policy is set.
    #[test]
    fn a_read_only_connection_cannot_mutate() {
        let read_only = ToolPermissions {
            analysis: PermissionLevel::Allow,
            portfolio: PermissionLevel::Allow,
            trading: PermissionLevel::Deny,
            config: PermissionLevel::Deny,
            system: PermissionLevel::Deny,
        };
        assert_eq!(
            decide(&tool(ToolCategory::Analysis, false), client(read_only)),
            Decision::Execute
        );
        for category in [
            ToolCategory::Trading,
            ToolCategory::Config,
            ToolCategory::System,
        ] {
            assert_eq!(
                decide(&tool(category, true), client(read_only)),
                Decision::Deny
            );
        }
    }

    /// An unreadable stored policy must fail closed.
    #[test]
    fn an_unparseable_policy_denies_everything() {
        assert_eq!(ToolPermissions::from_json("not json"), None);
        assert_eq!(
            decide(
                &tool(ToolCategory::Analysis, false),
                client(ToolPermissions::denied())
            ),
            Decision::Deny
        );
    }

    /// The policy survives the store round-trip it is written through.
    #[test]
    fn policy_round_trips_through_its_stored_form() {
        let policy = with_trading(PermissionLevel::AskUser);
        let restored = ToolPermissions::from_json(&policy.to_json()).expect("valid stored policy");
        assert_eq!(restored, policy);
        assert!(policy.to_json().contains("ask_user"));
    }

    /// Read-only automation is decided by what a tool does, not by its
    /// category: reading config is a read, force-stopping is not.
    #[test]
    fn read_only_automation_follows_the_mutation_flag() {
        assert!(is_read_only(&tool(ToolCategory::Config, false)));
        assert!(!is_read_only(&tool(ToolCategory::System, true)));
        assert_eq!(
            decide(
                &tool(ToolCategory::Config, false),
                InvocationSource::ScheduledReadOnly
            ),
            Decision::Execute
        );
        assert_eq!(
            decide(
                &tool(ToolCategory::Config, true),
                InvocationSource::ScheduledReadOnly
            ),
            Decision::Deny
        );
    }
}
