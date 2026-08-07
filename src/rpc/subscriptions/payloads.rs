//! WebSocket URL derivation and JSON-RPC subscription payloads.

use crate::config;
use crate::rpc::provider::derive_websocket_url;
use crate::{Error, Result};

/// Get the WebSocket URL from the primary configured RPC endpoint.
///
/// Converts the first RPC URL from config to a WebSocket URL.
/// Returns an error if no RPC URLs are configured.
pub fn get_websocket_url() -> Result<String> {
    let rpc_urls = config::with_config(|cfg| cfg.rpc.urls.clone());

    if rpc_urls.is_empty() {
        return Err(Error::Configuration(
            crate::errors::ConfigurationError::Generic {
                message: "No RPC URLs configured".to_owned(),
            },
        ));
    }

    let http_url = &rpc_urls[0];
    get_websocket_url_from_http(http_url)
}

/// Convert an HTTP/HTTPS RPC URL to its WebSocket equivalent.
///
/// # Examples
/// - `https://api.mainnet-beta.solana.com` -> `wss://api.mainnet-beta.solana.com`
/// - `http://localhost:8899` -> `ws://localhost:8899`
pub fn get_websocket_url_from_http(http_url: &str) -> Result<String> {
    derive_websocket_url(http_url).ok_or_else(|| {
        Error::Configuration(crate::errors::ConfigurationError::Generic {
            message: format!("Failed to convert HTTP URL to WebSocket: {http_url}"),
        })
    })
}

/// `logsSubscribe` for one address, filtered by `mentions`, at `confirmed` commitment.
pub(super) fn logs_subscribe_payload(address: &str, request_id: u64) -> String {
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": request_id,
        "method": "logsSubscribe",
        "params": [
            { "mentions": [address] },
            { "commitment": "confirmed" }
        ]
    })
    .to_string()
}

/// `logsUnsubscribe` for a server-assigned subscription id.
pub(super) fn logs_unsubscribe_payload(subscription_id: u64, request_id: u64) -> String {
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": request_id,
        "method": "logsUnsubscribe",
        "params": [subscription_id]
    })
    .to_string()
}
