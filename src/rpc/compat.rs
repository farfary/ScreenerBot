//! RPC Client Access Layer
//!
//! Provides the global `RpcClient` instance that uses `RpcManager` internally.
//! This is the main entry point for RPC operations throughout the codebase.
//!
//! # Usage
//!
//! ```rust,ignore
//! use crate::rpc::{get_rpc_client, RpcClientMethods};
//!
//! let client = get_rpc_client();
//! let balance = client.get_sol_balance("wallet_address").await?;
//! ```

use std::sync::Arc;
use std::sync::OnceLock;

use crate::rpc::client::RpcClient;
use crate::rpc::manager::{get_rpc_manager, init_rpc_manager, RpcManager};

/// Global RPC client instance (uses RpcManager internally)
static RPC_CLIENT: OnceLock<RpcClient> = OnceLock::new();

/// Initialize the RPC client
///
/// This initializes the RpcManager-based client. Called automatically
/// on first access via `get_rpc_client()`.
pub fn init_rpc_client() -> Result<&'static RpcClient, String> {
    // Check if already initialized
    if let Some(client) = RPC_CLIENT.get() {
        return Ok(client);
    }

    // Use block_in_place for sync-to-async bridge
    // This works correctly in multi-threaded Tokio runtime
    let manager = tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(init_rpc_manager())
    })?;

    Ok(RPC_CLIENT.get_or_init(|| RpcClient::new(manager)))
}

/// Get the global RPC client
///
/// Returns the RpcManager-based client. Auto-initializes if not done.
///
/// # Example
/// ```ignore
/// use crate::rpc::{get_rpc_client, RpcClientMethods};
///
/// let client = get_rpc_client();
/// let balance = client.get_sol_balance("...").await?;
/// ```
pub fn get_rpc_client() -> &'static RpcClient {
    RPC_CLIENT.get().unwrap_or_else(|| {
        // Auto-initialize if not done
        match init_rpc_client() {
            Ok(client) => client,
            Err(e) => panic!("Failed to initialize RPC client: {}", e),
        }
    })
}

/// Try to get the RPC client without panicking
pub fn try_get_rpc_client() -> Option<&'static RpcClient> {
    RPC_CLIENT.get()
}

/// Check if RPC client is initialized
pub fn is_rpc_initialized() -> bool {
    RPC_CLIENT.get().is_some()
}

/// Get the underlying RpcManager (for advanced usage)
pub fn get_manager() -> Option<Arc<RpcManager>> {
    get_rpc_manager()
}
