//! Local agent-control configuration. Authorization remains per paired client;
//! this section only controls whether the embedded MCP surface is available.

use crate::config_struct;

config_struct! {
    pub struct AgentsConfig {
        /// Enable the native local MCP endpoint and stdio adapter.
        enabled: bool = true,
    }
}
