// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! WebSocket URL derivation and JSON-RPC subscription payloads.

use super::types::Topic;
use crate::config;
use crate::rpc::provider::derive_websocket_url;
use crate::{Error, Result};

/// Get every usable WebSocket URL in configured provider order.
pub fn get_websocket_urls() -> Result<Vec<String>> {
    let rpc_urls = config::with_config(|cfg| cfg.chains.solana.rpc.urls.clone());
    let urls = rpc_urls
        .iter()
        .filter_map(|url| derive_websocket_url(url))
        .collect::<Vec<_>>();
    if urls.is_empty() {
        return Err(Error::Configuration(
            crate::errors::ConfigurationError::Generic {
                message: "No usable RPC URLs configured".to_owned(),
            },
        ));
    }
    Ok(urls)
}

/// Backward-compatible primary URL accessor.
pub fn get_websocket_url() -> Result<String> {
    get_websocket_urls()?.into_iter().next().ok_or_else(|| {
        Error::Configuration(crate::errors::ConfigurationError::Generic {
            message: "No usable RPC URLs configured".to_owned(),
        })
    })
}

pub fn websocket_url_for_attempt(urls: &[String], attempt: u32) -> Option<&str> {
    (!urls.is_empty()).then(|| urls[attempt as usize % urls.len()].as_str())
}

/// Convert an HTTP/HTTPS RPC URL to its WebSocket equivalent.
///
/// # Examples
/// - `https://api.mainnet-beta.solana.com` -> `wss://api.mainnet-beta.solana.com`
/// - `http://localhost:8899` -> `ws://localhost:8899`
pub fn get_websocket_url_from_http(http_url: &str) -> Result<String> {
    derive_websocket_url(http_url).ok_or_else(|| {
        Error::Configuration(crate::errors::ConfigurationError::Generic {
            message: format!(
                "Failed to convert HTTP URL to WebSocket: {}",
                crate::logger::redact_url(http_url)
            ),
        })
    })
}

/// The subscribe request for `topic`, at `confirmed` commitment. An account change is
/// asked for without its data: only the slot of each change is used.
pub(super) fn subscribe_payload(topic: &Topic, request_id: u64) -> String {
    let params = match topic {
        Topic::Mentions(address) => serde_json::json!([
            { "mentions": [address] },
            { "commitment": "confirmed" }
        ]),
        Topic::Account(address) => serde_json::json!([
            address,
            {
                "commitment": "confirmed",
                "encoding": "base64",
                "dataSlice": { "offset": 0, "length": 0 }
            }
        ]),
    };
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": request_id,
        "method": topic.method(),
        "params": params
    })
    .to_string()
}

/// The unsubscribe request for a server-assigned subscription id of `topic`.
pub(super) fn unsubscribe_payload(topic: &Topic, subscription_id: u64, request_id: u64) -> String {
    let method = match topic {
        Topic::Mentions(_) => "logsUnsubscribe",
        Topic::Account(_) => "accountUnsubscribe",
    };
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": request_id,
        "method": method,
        "params": [subscription_id]
    })
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_topic_asks_for_its_own_method() {
        let mentions: serde_json::Value =
            serde_json::from_str(&subscribe_payload(&Topic::Mentions("A".to_owned()), 1)).unwrap();
        assert_eq!(mentions["method"], "logsSubscribe");
        assert_eq!(mentions["params"][0]["mentions"][0], "A");

        let account: serde_json::Value =
            serde_json::from_str(&subscribe_payload(&Topic::Account("A".to_owned()), 2)).unwrap();
        assert_eq!(account["method"], "accountSubscribe");
        assert_eq!(account["params"][0], "A");
        assert_eq!(account["params"][1]["dataSlice"]["length"], 0);

        let unsubscribe: serde_json::Value =
            serde_json::from_str(&unsubscribe_payload(&Topic::Account("A".to_owned()), 9, 3))
                .unwrap();
        assert_eq!(unsubscribe["method"], "accountUnsubscribe");
        assert_eq!(unsubscribe["params"][0], 9);
    }
}
