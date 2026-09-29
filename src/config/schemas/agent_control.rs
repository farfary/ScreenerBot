//! Agent-control configuration: whether the shared capability boundary is
//! available at all, and the per-category tool permission policy the dashboard
//! assistant and scheduled automation run under. A paired MCP connection is not
//! governed by this — it carries its own policy in the pairing store, editable
//! per connection under Settings -> Agent Connections.

use crate::config::metadata::{ConfigCategory, ConfigImpact};
use crate::config_struct;
use crate::field_metadata;

config_struct! {
    /// Capability registry availability, plus the per-category policy for the
    /// in-app assistant and scheduled automation. Paired agent connections do
    /// NOT read this: each carries its own policy in the pairing store.
    pub struct AgentControlConfig {
        /// Enable the loopback agent bridge and stdio MCP adapter.
        #[metadata(field_metadata! {
            category: ConfigCategory::MasterControl,
            impact: ConfigImpact::Critical,
        })]
        enabled: bool = true,

        /// Permission level for analysis tools (allow, ask_user, deny).
        #[metadata(field_metadata! {
            category: ConfigCategory::ToolPermissions,
        })]
        analysis: String = "allow".to_owned(),

        /// Permission level for portfolio tools (allow, ask_user, deny).
        #[metadata(field_metadata! {
            category: ConfigCategory::ToolPermissions,
        })]
        portfolio: String = "allow".to_owned(),

        /// Permission level for trading tools (allow, ask_user, deny).
        #[metadata(field_metadata! {
            category: ConfigCategory::ToolPermissions,
        })]
        trading: String = "allow".to_owned(),

        /// Permission level for config-modification tools (allow, ask_user, deny).
        #[metadata(field_metadata! {
            category: ConfigCategory::ToolPermissions,
        })]
        config: String = "allow".to_owned(),

        /// Permission level for system tools (allow, ask_user, deny).
        #[metadata(field_metadata! {
            category: ConfigCategory::ToolPermissions,
        })]
        system: String = "allow".to_owned(),
    }
}
