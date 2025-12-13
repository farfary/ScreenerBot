//! WebSocket utilities for RPC connections
//!
//! This module provides helpers for WebSocket-based Solana RPC subscriptions.

use crate::config;
use crate::errors::ScreenerBotError;
use crate::rpc::provider::derive_websocket_url;

/// Get the WebSocket URL from the primary configured RPC endpoint
///
/// Converts the first RPC URL from config to a WebSocket URL.
/// Returns an error if no RPC URLs are configured.
pub fn get_websocket_url() -> Result<String, ScreenerBotError> {
    let rpc_urls = config::with_config(|cfg| cfg.rpc.urls.clone());

    if rpc_urls.is_empty() {
        return Err(ScreenerBotError::Configuration(
            crate::errors::ConfigurationError::Generic {
                message: "No RPC URLs configured".to_string(),
            },
        ));
    }

    let http_url = &rpc_urls[0];
    get_websocket_url_from_http(http_url)
}

/// Convert an HTTP/HTTPS RPC URL to its WebSocket equivalent
///
/// # Examples
/// - `https://api.mainnet-beta.solana.com` -> `wss://api.mainnet-beta.solana.com`
/// - `http://localhost:8899` -> `ws://localhost:8899`
pub fn get_websocket_url_from_http(http_url: &str) -> Result<String, ScreenerBotError> {
    derive_websocket_url(http_url).ok_or_else(|| {
        ScreenerBotError::Configuration(crate::errors::ConfigurationError::Generic {
            message: format!("Failed to convert HTTP URL to WebSocket: {}", http_url),
        })
    })
}

/// Create WebSocket subscription payload for account monitoring
///
/// Creates a JSON-RPC payload for subscribing to account updates.
/// Uses jsonParsed encoding and confirmed commitment.
///
/// # Arguments
/// * `pubkey` - The account public key to subscribe to
/// * `id` - The JSON-RPC request ID
pub fn create_account_subscribe_payload(pubkey: &str, id: u64) -> String {
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": "accountSubscribe",
        "params": [
            pubkey,
            {
                "encoding": "jsonParsed",
                "commitment": "confirmed"
            }
        ]
    })
    .to_string()
}

/// Create WebSocket subscription payload for log monitoring
///
/// Creates a JSON-RPC payload for subscribing to program logs.
/// Filters by the specified program mentions.
///
/// # Arguments
/// * `mentions` - List of program IDs to filter logs for
pub fn build_logs_subscribe_payload(mentions: &[&str]) -> serde_json::Value {
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "logsSubscribe",
        "params": [
            { "mentions": mentions },
            { "commitment": "confirmed" }
        ]
    })
}

/// Check if log messages contain "InitializeMint" instruction
pub fn logs_contains_initialize_mint(logs: &[String]) -> bool {
    logs.iter().any(|log| log.contains("InitializeMint"))
}

/// Check if log messages contain "InitializeAccount" instruction
pub fn logs_contains_initialize_account(logs: &[String]) -> bool {
    logs.iter()
        .any(|log| log.contains("InitializeAccount") || log.contains("InitializeAccount3"))
}
