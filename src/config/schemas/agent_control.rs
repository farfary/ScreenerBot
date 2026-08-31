//! Agent-control configuration: whether the shared capability boundary is
//! available at all, and the per-category tool permission policy the dashboard
//! assistant and scheduled automation run under. A paired MCP connection is not
//! governed by this — it carries its own policy in the pairing store, editable
//! per connection under Settings -> Agent Connections.

use crate::config_struct;
use crate::field_metadata;

config_struct! {
    /// Capability registry availability, plus the per-category policy for the
    /// in-app assistant and scheduled automation. Paired agent connections do
    /// NOT read this: each carries its own policy in the pairing store.
    pub struct AgentControlConfig {
        /// Enable the loopback agent bridge and stdio MCP adapter.
        #[metadata(field_metadata! {
            label: "Enable Agent Control",
            hint: "Enable the authenticated loopback agent bridge and stdio MCP adapter. Unpaired clients receive zero tools",
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
            hint: "Permission level for trading tools (allow, ask_user, deny) for the in-app assistant and scheduled automation. Paired agent connections carry their own policy, set per connection under Settings -> Agent Connections",
            placeholder: "allow",
            category: "Tool Permissions",
        })]
        trading: String = "allow".to_owned(),

        /// Permission level for config-modification tools (allow, ask_user, deny).
        #[metadata(field_metadata! {
            label: "Config Tools",
            hint: "Permission level for configuration tools (allow, ask_user, deny). Every setting the app has is reachable, including RPC endpoints; wallet private-key material is never readable or writable by an agent at any level",
            placeholder: "allow",
            category: "Tool Permissions",
        })]
        config: String = "allow".to_owned(),

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
