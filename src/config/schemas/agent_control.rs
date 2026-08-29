//! Agent-control configuration: whether the shared capability boundary is
//! available at all, and the per-category tool permission policy that the
//! dashboard assistant, scheduled automation and the MCP adapter all read.

use crate::config_struct;
use crate::field_metadata;

config_struct! {
    /// Capability registry availability and per-category tool permissions.
    pub struct AgentControlConfig {
        /// Enable the native local MCP endpoint and stdio adapter.
        #[metadata(field_metadata! {
            label: "Enable Agent Control",
            hint: "Enable the native local MCP endpoint and stdio adapter. Unknown clients still receive zero tools until a pairing store exists",
            category: "Master Control",
            impact: "critical",
        })]
        enabled: bool = true,

        /// Permission level for analysis tools (allow, ask_user, deny).
        #[metadata(field_metadata! {
            label: "Analysis Tools",
            hint: "Permission level for analysis tools (allow, ask_user, deny)",
            placeholder: "allow",
            category: "Tool Permissions",
        })]
        analysis: String = "allow".to_owned(),

        /// Permission level for portfolio tools (allow, ask_user, deny).
        #[metadata(field_metadata! {
            label: "Portfolio Tools",
            hint: "Permission level for portfolio tools (allow, ask_user, deny)",
            placeholder: "allow",
            category: "Tool Permissions",
        })]
        portfolio: String = "allow".to_owned(),

        /// Permission level for trading tools (allow, ask_user, deny).
        #[metadata(field_metadata! {
            label: "Trading Tools",
            hint: "Permission level for trading tools (allow, ask_user, deny)",
            placeholder: "ask_user",
            category: "Tool Permissions",
        })]
        trading: String = "ask_user".to_owned(),

        /// Permission level for config-modification tools (allow, ask_user, deny).
        #[metadata(field_metadata! {
            label: "Config Tools",
            hint: "Permission level for config-modification tools (allow, ask_user, deny)",
            placeholder: "ask_user",
            category: "Tool Permissions",
        })]
        config: String = "ask_user".to_owned(),

        /// Permission level for system tools (allow, ask_user, deny).
        #[metadata(field_metadata! {
            label: "System Tools",
            hint: "Permission level for system tools (allow, ask_user, deny)",
            placeholder: "allow",
            category: "Tool Permissions",
        })]
        system: String = "allow".to_owned(),
    }
}
