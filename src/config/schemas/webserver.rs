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

        /// Enable password authentication for headless mode
        #[metadata(field_metadata! {
            label: "Enable Authentication",
            hint: "Require password to access the dashboard in headless mode. Set password via CLI or API.",
            category: "Authentication",
        })]
        auth_enabled: bool = false,

        /// Password hash (BLAKE3) - do not edit directly
        #[metadata(field_metadata! {
            label: "Password Hash",
            hint: "Hashed password for authentication. Set via API, not directly.",
            category: "Authentication",
            hidden: true,
        })]
        auth_password_hash: String = String::new(),

        /// Password salt - do not edit directly
        #[metadata(field_metadata! {
            label: "Password Salt",
            hint: "Salt for password hashing. Set via API, not directly.",
            category: "Authentication",
            hidden: true,
        })]
        auth_password_salt: String = String::new(),

        /// Session timeout in seconds (0 = never expires)
        #[metadata(field_metadata! {
            label: "Session Timeout",
            hint: "How long before a session expires and requires re-login. 0 = never expires.",
            category: "Authentication",
            min: 0,
            max: 604800,
            step: 3600,
        })]
        auth_session_timeout_secs: u64 = 86400,

        /// Show logo on login page
        #[metadata(field_metadata! {
            label: "Show Logo",
            hint: "Display the ScreenerBot logo on the login page.",
            category: "Authentication",
        })]
        auth_show_logo: bool = true,

        /// Show app name on login page
        #[metadata(field_metadata! {
            label: "Show App Name",
            hint: "Display 'ScreenerBot' on the login page.",
            category: "Authentication",
        })]
        auth_show_name: bool = true,

        /// Custom title for login page (empty = use default)
        #[metadata(field_metadata! {
            label: "Custom Login Title",
            hint: "Custom title displayed on login page. Leave empty for default.",
            category: "Authentication",
            placeholder: "",
        })]
        auth_custom_title: String = String::new(),

        /// Enable TOTP two-factor authentication
        #[metadata(field_metadata! {
            label: "Enable 2FA (TOTP)",
            hint: "Require TOTP code in addition to password. Set up via dashboard settings.",
            category: "Authentication",
        })]
        auth_totp_enabled: bool = false,

        /// TOTP secret key (base32 encoded) - do not edit directly
        #[metadata(field_metadata! {
            label: "TOTP Secret",
            hint: "Secret key for TOTP generation. Set via API, not directly.",
            category: "Authentication",
            hidden: true,
        })]
        auth_totp_secret: String = String::new(),
    }
}
