#![allow(warnings)]

use once_cell::sync::Lazy;
use serde::Deserialize;
use solana_client::rpc_client::RpcClient;

#[derive(Debug, Deserialize)]
pub struct Configs {
    pub main_wallet_private: String,
    pub rpc_url: String,
}

pub static CONFIGS: Lazy<Configs> = Lazy::new(|| {
    let raw = std::fs::read_to_string("configs.json").expect("Failed to read configs.json");
    serde_json::from_str(&raw).expect("Failed to parse configs.json")
});

pub static RPC: Lazy<RpcClient> = Lazy::new(|| RpcClient::new(CONFIGS.rpc_url.clone()));

use std::collections::HashSet;
use tokio::sync::RwLock;

/// Mints that produced “Unsupported program id …” or similar hard
/// errors while decoding pools.
pub static BLACKLIST: Lazy<RwLock<HashSet<String>>> =
    Lazy::new(|| RwLock::new(HashSet::new()));

