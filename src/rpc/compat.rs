//! Backward compatibility layer
//!
//! Provides the new `RpcClient` wrapper that uses `RpcManager` internally,
//! while maintaining compatibility with the legacy RpcClient API.
//!
//! # Migration Strategy
//!
//! During migration:
//! 1. The legacy `get_rpc_client()` from `rpc_legacy` continues to work
//! 2. The new `RpcClient` wrapper is available via `rpc::client::RpcClient`
//! 3. Code can be migrated incrementally by importing `RpcClientMethods` trait
//!
//! After full migration:
//! 1. The compat layer's `get_rpc_client()` will replace the legacy one
//! 2. The legacy module can be removed

use std::sync::Arc;
use std::sync::OnceLock;

use crate::rpc::client::RpcClient as NewRpcClient;
use crate::rpc::manager::{get_rpc_manager, init_rpc_manager, RpcManager};

/// Global new RPC client instance (uses RpcManager internally)
static NEW_RPC_CLIENT: OnceLock<NewRpcClient> = OnceLock::new();

/// Initialize the new RPC client (uses RpcManager)
///
/// This initializes the new RpcManager-based client. The legacy client
/// is initialized separately via `rpc_legacy::init_rpc_client()`.
pub fn init_new_rpc_client() -> Result<&'static NewRpcClient, String> {
    // Check if already initialized
    if let Some(client) = NEW_RPC_CLIENT.get() {
        return Ok(client);
    }

    // Use block_in_place for sync-to-async bridge
    // This works correctly in multi-threaded Tokio runtime
    let manager = tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(init_rpc_manager())
    })?;

    Ok(NEW_RPC_CLIENT.get_or_init(|| NewRpcClient::new(manager)))
}

/// Get the new RPC client (uses RpcManager)
///
/// This returns the new RpcManager-based client. For the legacy client,
/// use `crate::rpc::get_rpc_client()` which comes from `rpc_legacy`.
///
/// # Example
/// ```ignore
/// use crate::rpc::compat::get_new_rpc_client;
/// use crate::rpc::RpcClientMethods;
///
/// let client = get_new_rpc_client();
/// let balance = client.get_sol_balance("...").await?;
/// ```
pub fn get_new_rpc_client() -> &'static NewRpcClient {
    NEW_RPC_CLIENT.get().unwrap_or_else(|| {
        // Auto-initialize if not done
        match init_new_rpc_client() {
            Ok(client) => client,
            Err(e) => panic!("Failed to initialize new RPC client: {}", e),
        }
    })
}

/// Try to get the new RPC client without panicking
pub fn try_get_new_rpc_client() -> Option<&'static NewRpcClient> {
    NEW_RPC_CLIENT.get()
}

/// Check if new RPC client is initialized
pub fn is_new_rpc_initialized() -> bool {
    NEW_RPC_CLIENT.get().is_some()
}

/// Get the underlying RpcManager (for advanced usage)
pub fn get_manager() -> Option<Arc<RpcManager>> {
    get_rpc_manager()
}

