// Webserver configuration schema

use crate::config_struct;
use crate::field_metadata;

// ============================================================================
// WEBSERVER CONFIGURATION
// ============================================================================

config_struct! {
    /// Webserver configuration for dashboard access
    ///
    /// Note: These settings only apply to headless/CLI mode.
    /// In GUI mode, the webserver uses a dynamic port with security token
    /// and binds only to localhost for security.
    pub struct WebserverConfig {
        /// Port to bind the webserver (1024-65535)
        #[metadata(field_metadata! {
            label: "Port",
            hint: "Port for dashboard access (headless mode only). Requires restart to take effect.",
            category: "General",
            min: 1024,
            max: 65535,
            step: 1,
        })]
        port: u16 = 8080,

        /// Host/IP address to bind the webserver
        #[metadata(field_metadata! {
            label: "Host",
            hint: "IP to bind: 127.0.0.1 = localhost only, 0.0.0.0 = all interfaces (VPS/remote). Requires restart.",
            category: "General",
            placeholder: "127.0.0.1",
        })]
        host: String = "127.0.0.1".to_string(),
    }
}
