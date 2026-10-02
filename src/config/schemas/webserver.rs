// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Web dashboard server and API endpoint configuration.

// Webserver configuration schema

use crate::config::metadata::ConfigCategory;
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
            category: ConfigCategory::General,
            min: 1024,
            max: 65535,
            step: 1,
        })]
        port: u16 = 8080,

        /// Host/IP address to bind the webserver
        #[metadata(field_metadata! {
            category: ConfigCategory::General,
        })]
        host: String = "127.0.0.1".to_owned(),

        /// Enable password authentication for headless mode
        #[metadata(field_metadata! {
            category: ConfigCategory::Authentication,
        })]
        auth_enabled: bool = false,

        /// Password hash (BLAKE3) - do not edit directly
        #[metadata(field_metadata! {
            category: ConfigCategory::Authentication,
            hidden: true,
        })]
        auth_password_hash: String = String::new(),

        /// Password salt - do not edit directly
        #[metadata(field_metadata! {
            category: ConfigCategory::Authentication,
            hidden: true,
        })]
        auth_password_salt: String = String::new(),

        /// Session timeout in seconds (0 = never expires)
        #[metadata(field_metadata! {
            category: ConfigCategory::Authentication,
            min: 0,
            max: 604800,
            step: 3600,
        })]
        auth_session_timeout_secs: u64 = 86400,

        /// Show logo on login page
        #[metadata(field_metadata! {
            category: ConfigCategory::Authentication,
        })]
        auth_show_logo: bool = true,

        /// Show app name on login page
        #[metadata(field_metadata! {
            category: ConfigCategory::Authentication,
        })]
        auth_show_name: bool = true,

        /// Custom title for login page (empty = use default)
        #[metadata(field_metadata! {
            category: ConfigCategory::Authentication,
        })]
        auth_custom_title: String = String::new(),

        /// Enable TOTP two-factor authentication
        #[metadata(field_metadata! {
            category: ConfigCategory::Authentication,
        })]
        auth_totp_enabled: bool = false,

        /// TOTP secret key (base32 encoded) - do not edit directly
        #[metadata(field_metadata! {
            category: ConfigCategory::Authentication,
            hidden: true,
        })]
        auth_totp_secret: String = String::new(),
    }
}
