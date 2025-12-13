//! Auto-detect RPC provider type from URL

use crate::rpc::types::ProviderKind;

/// Detect provider kind from URL
pub fn detect_provider_kind(url: &str) -> ProviderKind {
    let lower = url.to_lowercase();

    if lower.contains("helius") {
        ProviderKind::Helius
    } else if lower.contains("quiknode") || lower.contains("quicknode") {
        ProviderKind::QuickNode
    } else if lower.contains("triton") || lower.contains("rpcpool") {
        ProviderKind::Triton
    } else if lower.contains("alchemy") {
        ProviderKind::Alchemy
    } else if lower.contains("getblock") {
        ProviderKind::GetBlock
    } else if lower.contains("shyft") {
        ProviderKind::Shyft
    } else if lower.contains("api.mainnet-beta.solana.com")
        || lower.contains("api.devnet.solana.com")
        || lower.contains("api.testnet.solana.com")
    {
        ProviderKind::Public
    } else {
        ProviderKind::Unknown
    }
}

/// Generate a unique provider ID from URL
pub fn generate_provider_id(url: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    url.hash(&mut hasher);
    let hash = hasher.finish();

    let kind = detect_provider_kind(url);
    format!("{}_{:x}", kind.to_string().to_lowercase(), hash & 0xFFFF)
}

/// Extract WebSocket URL from HTTP URL
pub fn derive_websocket_url(http_url: &str) -> Option<String> {
    let url = http_url.trim();

    // Handle different URL patterns
    if url.starts_with("wss://") || url.starts_with("ws://") {
        return Some(url.to_string());
    }

    let ws_url = if url.starts_with("https://") {
        url.replacen("https://", "wss://", 1)
    } else if url.starts_with("http://") {
        url.replacen("http://", "ws://", 1)
    } else {
        return None;
    };

    Some(ws_url)
}
