//! Provider configuration

use crate::rpc::provider::detection::{detect_provider_kind, generate_provider_id};
use crate::rpc::types::ProviderKind;
use serde::{Deserialize, Serialize};

/// Configuration for a single RPC provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    /// Unique identifier (auto-generated if not specified)
    pub id: String,
    /// RPC endpoint URL
    pub url: String,
    /// Provider type (auto-detected if not specified)
    pub kind: ProviderKind,
    /// Priority (lower = higher priority, default 100)
    pub priority: u8,
    /// Rate limit per second (0 = use provider default)
    pub rate_limit: u32,
    /// Whether this provider is enabled
    pub enabled: bool,
    /// Request timeout in seconds
    pub timeout_secs: u64,
    /// Maximum retries for this provider
    pub max_retries: u32,
    /// Weight for load balancing (higher = more traffic)
    pub weight: u32,
}

impl ProviderConfig {
    /// Create from URL with auto-detection
    pub fn from_url(url: &str) -> Self {
        let kind = detect_provider_kind(url);
        let id = generate_provider_id(url);

        Self {
            id,
            url: url.to_string(),
            kind,
            priority: 100,
            rate_limit: 0, // Use provider default
            enabled: true,
            timeout_secs: 30,
            max_retries: 3,
            weight: 100,
        }
    }

    /// Create from URL with priority
    pub fn from_url_with_priority(url: &str, priority: u8) -> Self {
        let mut config = Self::from_url(url);
        config.priority = priority;
        config
    }

    /// Get effective rate limit (configured or default)
    pub fn effective_rate_limit(&self) -> u32 {
        if self.rate_limit > 0 {
            self.rate_limit
        } else {
            self.kind.default_rate_limit()
        }
    }
}

impl Default for ProviderConfig {
    fn default() -> Self {
        Self {
            id: "default".to_string(),
            url: String::new(),
            kind: ProviderKind::Unknown,
            priority: 100,
            rate_limit: 0,
            enabled: true,
            timeout_secs: 30,
            max_retries: 3,
            weight: 100,
        }
    }
}
