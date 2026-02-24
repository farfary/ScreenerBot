//! Provider abstraction for RPC endpoints

pub mod config;
pub mod detection;

pub use config::ProviderConfig;
pub use detection::{derive_websocket_url, detect_provider_kind, generate_provider_id};

use crate::rpc::errors::RpcError;
use crate::rpc::types::{ProviderKind, ProviderState, RpcMethod};
use async_trait::async_trait;
use std::sync::Arc;
use std::time::Duration;

/// Trait for RPC provider implementations
#[async_trait]
pub trait RpcProvider: Send + Sync {
    /// Get provider ID
    fn id(&self) -> &str;

    /// Get provider URL
    fn url(&self) -> &str;

    /// Get provider kind
    fn kind(&self) -> ProviderKind;

    /// Get current provider state
    fn state(&self) -> ProviderState;

    /// Check if method is supported
    fn supports_method(&self, method: &RpcMethod) -> bool;

    /// Get configured rate limit
    fn rate_limit(&self) -> u32;

    /// Get request timeout
    fn timeout(&self) -> Duration;

    /// Execute raw JSON-RPC request
    async fn execute_raw(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, RpcError>;

    /// Health probe - returns latency in ms
    async fn probe_health(&self) -> Result<u64, RpcError>;
}

/// Type alias for provider references
pub type ProviderRef = Arc<dyn RpcProvider>;
