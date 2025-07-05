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

pub static RPC: Lazy<RpcClient> = Lazy::new(|| {
    RpcClient::new_with_timeout(CONFIGS.rpc_url.clone(), std::time::Duration::from_secs(10))
});

use std::collections::HashSet;
use tokio::sync::RwLock;

/// Mints that produced “Unsupported program id …” or similar hard
/// errors while decoding pools.


use std::{fs};
use serde_json::json;


const BLACKLIST_FILE: &str = ".blacklist.json";

/// In-memory set, initially populated from disk
pub static BLACKLIST: Lazy<RwLock<HashSet<String>>> = Lazy::new(|| {
    let mut set = HashSet::new();
    if let Ok(s) = fs::read_to_string(BLACKLIST_FILE) {
        if let Ok(v) = serde_json::from_str::<Vec<String>>(&s) {
            set.extend(v);
        }
    }
    RwLock::new(set)
});

pub async fn add_to_blacklist(mint: &str) {
    // 1) Insert under lock, but don't write file yet
    let need_write = {
        let mut bl = BLACKLIST.write().await;
        bl.insert(mint.to_string())
    };

    // 2) If it was new, persist _after_ dropping the lock
    if need_write {
        // capture the current set
        let vec: Vec<String> = {
            let bl = BLACKLIST.read().await;
            bl.iter().cloned().collect()
        };
        let data = serde_json::to_string(&vec).unwrap();

        // async write without blocking the executor
        tokio::fs::write(BLACKLIST_FILE, data).await.ok();
    }
}


use std::env;
/// Cached command-line arguments
pub static ARGS: Lazy<Vec<String>> = Lazy::new(|| env::args().collect());
