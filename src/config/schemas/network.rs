// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Network proxy configuration for routing all external HTTP, RPC, and
//! WebSocket traffic through a proxy. Essential for users behind national
//! firewalls or corporate proxies where direct connections are blocked.
//!
//! When `proxy` is set, it takes priority over env vars (`HTTPS_PROXY`, etc.)
//! and macOS system proxy detection. Supported formats:
//! - HTTP proxy:  `http://127.0.0.1:8080`
//! - SOCKS5 proxy: `socks5://127.0.0.1:1080`
//! - Bare `host:port` is treated as HTTP.
//!
//! Leave empty to use automatic detection (env vars → macOS system proxy).

use crate::config::metadata::ConfigCategory;
use crate::config_struct;
use crate::field_metadata;

// ============================================================================
// NETWORK PROXY
// ============================================================================

config_struct! {
    /// Network proxy configuration
    pub struct NetworkConfig {
        /// Proxy URL for all external connections (HTTP, RPC, WebSocket).
        /// Set to e.g. "socks5://127.0.0.1:1080" or "http://127.0.0.1:8080".
        /// Leave empty for automatic detection (env vars / system proxy).
        #[metadata(field_metadata! {
            category: ConfigCategory::General
        })]
        proxy: String = String::new(),
    }
}
