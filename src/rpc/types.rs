//! Core types for RPC module

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

/// RPC provider identification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProviderKind {
    Helius,
    QuickNode,
    Triton,
    Alchemy,
    GetBlock,
    Shyft,
    Public,
    Unknown,
}

impl ProviderKind {
    /// Default rate limit per second for this provider type
    pub fn default_rate_limit(&self) -> u32 {
        match self {
            Self::Helius => 50,
            Self::QuickNode => 25,
            Self::Triton => 100,
            Self::Alchemy => 25,
            Self::GetBlock => 25,
            Self::Shyft => 25,
            Self::Public => 4,
            Self::Unknown => 10,
        }
    }

    /// Whether this provider supports getPriorityFeeEstimate
    pub fn supports_priority_fee_estimate(&self) -> bool {
        matches!(self, Self::Helius)
    }

    /// Whether this provider supports DAS API
    pub fn supports_das_api(&self) -> bool {
        matches!(self, Self::Helius)
    }

    /// Whether this is a premium provider
    pub fn is_premium(&self) -> bool {
        !matches!(self, Self::Public | Self::Unknown)
    }
}

impl fmt::Display for ProviderKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Helius => write!(f, "Helius"),
            Self::QuickNode => write!(f, "QuickNode"),
            Self::Triton => write!(f, "Triton"),
            Self::Alchemy => write!(f, "Alchemy"),
            Self::GetBlock => write!(f, "GetBlock"),
            Self::Shyft => write!(f, "Shyft"),
            Self::Public => write!(f, "Public"),
            Self::Unknown => write!(f, "Unknown"),
        }
    }
}

/// RPC method enumeration with cost weighting
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RpcMethod {
    // Health/Info
    GetHealth,
    GetVersion,
    GetSlot,
    GetBlockHeight,
    GetEpochInfo,

    // Account queries
    GetBalance,
    GetAccountInfo,
    GetMultipleAccounts,
    GetTokenAccountBalance,
    GetTokenAccountsByOwner,
    GetProgramAccounts,

    // Transaction queries
    GetTransaction,
    GetSignaturesForAddress,
    GetSignatureStatuses,

    // Blockhash
    GetLatestBlockhash,
    GetRecentBlockhash,
    IsBlockhashValid,
    GetFeeForMessage,

    // Rent
    GetMinimumBalanceForRentExemption,

    // Sending
    SendTransaction,
    SimulateTransaction,

    // Provider-specific
    GetPriorityFeeEstimate,
    GetAsset,
    GetAssetsByOwner,
    SearchAssets,

    // Unknown method
    Other(String),
}

impl RpcMethod {
    /// Parse method name string into enum
    pub fn from_str(method: &str) -> Self {
        match method {
            "getHealth" => Self::GetHealth,
            "getVersion" => Self::GetVersion,
            "getSlot" => Self::GetSlot,
            "getBlockHeight" => Self::GetBlockHeight,
            "getEpochInfo" => Self::GetEpochInfo,
            "getBalance" => Self::GetBalance,
            "getAccountInfo" => Self::GetAccountInfo,
            "getMultipleAccounts" => Self::GetMultipleAccounts,
            "getTokenAccountBalance" => Self::GetTokenAccountBalance,
            "getTokenAccountsByOwner" => Self::GetTokenAccountsByOwner,
            "getProgramAccounts" => Self::GetProgramAccounts,
            "getTransaction" => Self::GetTransaction,
            "getSignaturesForAddress" => Self::GetSignaturesForAddress,
            "getSignatureStatuses" => Self::GetSignatureStatuses,
            "getLatestBlockhash" => Self::GetLatestBlockhash,
            "getRecentBlockhash" => Self::GetRecentBlockhash,
            "isBlockhashValid" => Self::IsBlockhashValid,
            "getFeeForMessage" => Self::GetFeeForMessage,
            "getMinimumBalanceForRentExemption" => Self::GetMinimumBalanceForRentExemption,
            "sendTransaction" => Self::SendTransaction,
            "simulateTransaction" => Self::SimulateTransaction,
            "getPriorityFeeEstimate" => Self::GetPriorityFeeEstimate,
            "getAsset" => Self::GetAsset,
            "getAssetsByOwner" => Self::GetAssetsByOwner,
            "searchAssets" => Self::SearchAssets,
            other => Self::Other(other.to_string()),
        }
    }

    /// Get method name as string
    pub fn as_str(&self) -> &str {
        match self {
            Self::GetHealth => "getHealth",
            Self::GetVersion => "getVersion",
            Self::GetSlot => "getSlot",
            Self::GetBlockHeight => "getBlockHeight",
            Self::GetEpochInfo => "getEpochInfo",
            Self::GetBalance => "getBalance",
            Self::GetAccountInfo => "getAccountInfo",
            Self::GetMultipleAccounts => "getMultipleAccounts",
            Self::GetTokenAccountBalance => "getTokenAccountBalance",
            Self::GetTokenAccountsByOwner => "getTokenAccountsByOwner",
            Self::GetProgramAccounts => "getProgramAccounts",
            Self::GetTransaction => "getTransaction",
            Self::GetSignaturesForAddress => "getSignaturesForAddress",
            Self::GetSignatureStatuses => "getSignatureStatuses",
            Self::GetLatestBlockhash => "getLatestBlockhash",
            Self::GetRecentBlockhash => "getRecentBlockhash",
            Self::IsBlockhashValid => "isBlockhashValid",
            Self::GetFeeForMessage => "getFeeForMessage",
            Self::GetMinimumBalanceForRentExemption => "getMinimumBalanceForRentExemption",
            Self::SendTransaction => "sendTransaction",
            Self::SimulateTransaction => "simulateTransaction",
            Self::GetPriorityFeeEstimate => "getPriorityFeeEstimate",
            Self::GetAsset => "getAsset",
            Self::GetAssetsByOwner => "getAssetsByOwner",
            Self::SearchAssets => "searchAssets",
            Self::Other(s) => s.as_str(),
        }
    }

    /// Cost weight for rate limiting (higher = consumes more quota)
    pub fn cost(&self) -> u32 {
        match self {
            // Lightweight operations
            Self::GetHealth | Self::GetVersion => 1,
            Self::GetSlot | Self::GetBlockHeight | Self::GetEpochInfo => 1,
            Self::GetBalance | Self::GetTokenAccountBalance => 1,
            Self::GetLatestBlockhash | Self::GetRecentBlockhash | Self::IsBlockhashValid => 1,
            Self::GetMinimumBalanceForRentExemption | Self::GetFeeForMessage => 1,

            // Standard operations
            Self::GetAccountInfo => 1,
            Self::GetMultipleAccounts => 2,
            Self::GetTransaction => 2,
            Self::GetSignatureStatuses => 2,
            Self::SendTransaction | Self::SimulateTransaction => 2,
            Self::GetPriorityFeeEstimate => 1,
            Self::GetAsset => 1,

            // Heavy operations
            Self::GetSignaturesForAddress => 3,
            Self::GetTokenAccountsByOwner => 3,
            Self::GetAssetsByOwner => 3,
            Self::SearchAssets => 4,
            Self::GetProgramAccounts => 5,

            // Unknown - assume standard cost
            Self::Other(_) => 2,
        }
    }

    /// Whether this method requires a premium provider
    pub fn requires_premium_provider(&self) -> bool {
        matches!(
            self,
            Self::GetPriorityFeeEstimate
                | Self::GetAsset
                | Self::GetAssetsByOwner
                | Self::SearchAssets
        )
    }
}

impl fmt::Display for RpcMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Circuit breaker state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CircuitState {
    /// Normal operation - requests allowed
    Closed,
    /// Circuit tripped - requests rejected
    Open,
    /// Testing recovery - limited requests allowed
    HalfOpen,
}

impl Default for CircuitState {
    fn default() -> Self {
        Self::Closed
    }
}

impl fmt::Display for CircuitState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Closed => write!(f, "closed"),
            Self::Open => write!(f, "open"),
            Self::HalfOpen => write!(f, "half-open"),
        }
    }
}

/// Provider runtime state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderState {
    /// Unique provider identifier
    pub id: String,
    /// RPC URL (masked for logging)
    pub url_masked: String,
    /// Provider type
    pub kind: ProviderKind,
    /// Priority (lower = higher priority)
    pub priority: u8,
    /// Whether provider is enabled
    pub enabled: bool,
    /// Circuit breaker state
    pub circuit_state: CircuitState,
    /// Consecutive failures
    pub consecutive_failures: u32,
    /// Consecutive successes (in half-open state)
    pub consecutive_successes: u32,
    /// Last successful call
    pub last_success: Option<DateTime<Utc>>,
    /// Last failed call
    pub last_failure: Option<DateTime<Utc>>,
    /// Last error message
    pub last_error: Option<String>,
    /// Average latency in milliseconds
    pub avg_latency_ms: f64,
    /// Total calls made
    pub total_calls: u64,
    /// Total errors
    pub total_errors: u64,
    /// Current rate limit (may be reduced due to backoff)
    pub current_rate_limit: u32,
    /// Base rate limit
    pub base_rate_limit: u32,
}

impl ProviderState {
    /// Create new provider state
    pub fn new(id: String, url: &str, kind: ProviderKind, priority: u8) -> Self {
        let rate_limit = kind.default_rate_limit();
        Self {
            id,
            url_masked: mask_url(url),
            kind,
            priority,
            enabled: true,
            circuit_state: CircuitState::Closed,
            consecutive_failures: 0,
            consecutive_successes: 0,
            last_success: None,
            last_failure: None,
            last_error: None,
            avg_latency_ms: 0.0,
            total_calls: 0,
            total_errors: 0,
            current_rate_limit: rate_limit,
            base_rate_limit: rate_limit,
        }
    }

    /// Success rate as percentage
    pub fn success_rate(&self) -> f64 {
        if self.total_calls == 0 {
            100.0
        } else {
            100.0 * (self.total_calls - self.total_errors) as f64 / self.total_calls as f64
        }
    }

    /// Whether provider is healthy
    pub fn is_healthy(&self) -> bool {
        self.enabled && self.circuit_state == CircuitState::Closed
    }
}

/// RPC call result for statistics
#[derive(Debug, Clone)]
pub struct RpcCallResult {
    /// Provider that handled the call
    pub provider_id: String,
    /// RPC method called
    pub method: RpcMethod,
    /// Whether call succeeded
    pub success: bool,
    /// Latency in milliseconds
    pub latency_ms: u64,
    /// Error message if failed
    pub error: Option<String>,
    /// Timestamp of the call
    pub timestamp: DateTime<Utc>,
    /// Number of retries before this attempt
    pub retry_count: u32,
    /// Whether this was a rate limit error
    pub was_rate_limited: bool,
}

/// Provider selection strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SelectionStrategy {
    /// Round-robin across healthy providers
    RoundRobin,
    /// Use highest priority provider, fallback on failure
    Priority,
    /// Route to lowest latency provider
    LatencyBased,
    /// Combine health, latency, and error rate
    #[default]
    Adaptive,
}

impl SelectionStrategy {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "round_robin" | "roundrobin" => Self::RoundRobin,
            "priority" => Self::Priority,
            "latency" | "latency_based" => Self::LatencyBased,
            "adaptive" | _ => Self::Adaptive,
        }
    }
}

impl fmt::Display for SelectionStrategy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RoundRobin => write!(f, "round_robin"),
            Self::Priority => write!(f, "priority"),
            Self::LatencyBased => write!(f, "latency_based"),
            Self::Adaptive => write!(f, "adaptive"),
        }
    }
}

/// Mask API keys in URLs for safe logging
pub fn mask_url(url: &str) -> String {
    // Common patterns for API keys in URLs
    let patterns = [
        ("api-key=", '&'),
        ("api_key=", '&'),
        ("apikey=", '&'),
        ("x-api-key=", '&'),
        ("token=", '&'),
        ("access_token=", '&'),
    ];

    let mut masked = url.to_string();
    let lower = url.to_lowercase();

    for (pattern, delimiter) in patterns {
        if let Some(start) = lower.find(pattern) {
            let key_start = start + pattern.len();
            let key_end = url[key_start..]
                .find(delimiter)
                .map(|i| key_start + i)
                .unwrap_or(url.len());

            if key_end > key_start {
                let key_len = key_end - key_start;
                let mask = if key_len > 8 {
                    format!(
                        "{}...{}",
                        &url[key_start..key_start + 4],
                        &url[key_end - 4..key_end]
                    )
                } else {
                    "***".to_string()
                };
                masked = format!("{}{}{}", &url[..key_start], mask, &url[key_end..]);
            }
        }
    }

    // Also mask common subdomain patterns (e.g., quiknode URLs)
    // https://xxx-yyy-zzz.solana-mainnet.quiknode.pro/API_KEY/
    if masked.contains("quiknode.pro/") || masked.contains("quicknode.pro/") {
        if let Some(idx) = masked.rfind('/') {
            let after_slash = &masked[idx + 1..];
            if !after_slash.is_empty() && !after_slash.starts_with('?') {
                let end = after_slash.find('/').unwrap_or(after_slash.len());
                if end > 8 {
                    let key = &after_slash[..end];
                    let mask = format!("{}...{}", &key[..4], &key[key.len() - 4..]);
                    masked = format!("{}/{}", &masked[..idx], mask);
                }
            }
        }
    }

    masked
}
