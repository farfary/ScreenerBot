/// Centralized RPC Client for Solana
///
/// This module provides a centralized RPC client that can be used throughout the application
/// for consistent RPC configuration and connection management.

use crate::logger::{ log, LogTag };
use crate::global::read_configs;
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    account::Account,
    pubkey::Pubkey,
    commitment_config::CommitmentConfig,
    client::SyncClient,
};
use std::sync::Arc;
use std::str::FromStr;

/// Default RPC URLs for different environments
pub const MAINNET_RPC_URL: &str = "https://api.mainnet-beta.solana.com";
pub const HELIUS_RPC_URL: &str = "https://rpc.helius.xyz/?api-key=your-api-key";
pub const QUICKNODE_RPC_URL: &str = "https://your-endpoint.solana.quiknode.pro/";

/// Centralized RPC client with connection pooling and error handling
pub struct SolanaRpcClient {
    client: Arc<RpcClient>,
    rpc_url: String,
    fallback_urls: Vec<String>,
    current_url_index: usize,
}

impl SolanaRpcClient {
    /// Create new RPC client with configuration from configs.json
    pub fn new() -> Self {
        Self::from_config().unwrap_or_else(|e| {
            log(LogTag::Rpc, "ERROR", &format!("Failed to load config, using fallback: {}", e));
            Self::new_with_url(MAINNET_RPC_URL)
        })
    }

    /// Create new RPC client from configs.json
    pub fn from_config() -> Result<Self, String> {
        let configs = read_configs("configs.json").map_err(|e|
            format!("Failed to read configs: {}", e)
        )?;

        let mut all_urls = vec![configs.rpc_url.clone()];
        all_urls.extend(configs.rpc_fallbacks.clone());

        log(
            LogTag::Rpc,
            "INIT",
            &format!(
                "Initializing RPC client with {} URLs (primary + {} fallbacks)",
                all_urls.len(),
                configs.rpc_fallbacks.len()
            )
        );

        if !configs.rpc_fallbacks.is_empty() {
            log(
                LogTag::Rpc,
                "FALLBACKS",
                &format!("Available fallback URLs: {}", configs.rpc_fallbacks.join(", "))
            );
        }

        Self::new_with_urls(&configs.rpc_url, configs.rpc_fallbacks)
    }

    /// Create new RPC client with primary URL and fallbacks
    pub fn new_with_urls(primary_url: &str, fallback_urls: Vec<String>) -> Result<Self, String> {
        log(LogTag::Rpc, "INIT", &format!("Initializing RPC client with primary: {}", primary_url));

        let client = RpcClient::new_with_commitment(
            primary_url.to_string(),
            CommitmentConfig::confirmed()
        );

        Ok(Self {
            client: Arc::new(client),
            rpc_url: primary_url.to_string(),
            fallback_urls,
            current_url_index: 0,
        })
    }

    /// Create new RPC client with custom URL (legacy method)
    pub fn new_with_url(rpc_url: &str) -> Self {
        log(LogTag::Rpc, "INIT", &format!("Initializing RPC client with URL: {}", rpc_url));

        let client = RpcClient::new_with_commitment(
            rpc_url.to_string(),
            CommitmentConfig::confirmed()
        );

        Self {
            client: Arc::new(client),
            rpc_url: rpc_url.to_string(),
            fallback_urls: Vec::new(),
            current_url_index: 0,
        }
    }

    /// Get the underlying RPC client
    pub fn client(&self) -> Arc<RpcClient> {
        self.client.clone()
    }

    /// Get RPC URL
    pub fn url(&self) -> &str {
        &self.rpc_url
    }

    /// Get all available URLs (primary + fallbacks)
    pub fn get_all_urls(&self) -> Vec<String> {
        let mut urls = vec![self.rpc_url.clone()];
        urls.extend(self.fallback_urls.clone());
        urls
    }

    /// Switch to next fallback URL
    pub fn switch_to_fallback(&mut self) -> Result<(), String> {
        if self.fallback_urls.is_empty() {
            return Err("No fallback URLs available".to_string());
        }

        self.current_url_index = (self.current_url_index + 1) % (self.fallback_urls.len() + 1);

        let new_url = if self.current_url_index == 0 {
            self.rpc_url.clone()
        } else {
            self.fallback_urls[self.current_url_index - 1].clone()
        };

        log(LogTag::Rpc, "FALLBACK", &format!("Switching to URL: {}", new_url));

        let new_client = RpcClient::new_with_commitment(
            new_url.clone(),
            CommitmentConfig::confirmed()
        );

        self.client = Arc::new(new_client);
        Ok(())
    }

    /// Get single account data
    pub async fn get_account(&self, pubkey: &Pubkey) -> Result<Account, String> {
        tokio::task
            ::spawn_blocking({
                let client = self.client.clone();
                let pubkey = *pubkey;
                move || {
                    client
                        .get_account(&pubkey)
                        .map_err(|e| format!("Failed to get account {}: {}", pubkey, e))
                }
            }).await
            .map_err(|e| format!("Task error: {}", e))?
    }

    /// Get multiple accounts data (batch request for efficiency)
    pub async fn get_multiple_accounts(
        &self,
        pubkeys: &[Pubkey]
    ) -> Result<Vec<Option<Account>>, String> {
        if pubkeys.is_empty() {
            return Ok(Vec::new());
        }

        tokio::task
            ::spawn_blocking({
                let client = self.client.clone();
                let pubkeys = pubkeys.to_vec();
                move || {
                    client
                        .get_multiple_accounts(&pubkeys)
                        .map_err(|e| format!("Failed to get multiple accounts: {}", e))
                }
            }).await
            .map_err(|e| format!("Task error: {}", e))?
    }

    /// Get account data with automatic fallback support
    pub async fn get_account_with_fallback(&mut self, pubkey: &Pubkey) -> Result<Account, String> {
        let max_attempts = self.get_all_urls().len();
        let mut last_error = String::new();

        for attempt in 0..max_attempts {
            match self.get_account(pubkey).await {
                Ok(account) => {
                    return Ok(account);
                }
                Err(e) => {
                    last_error = e.clone();
                    log(LogTag::Rpc, "ERROR", &format!("RPC call failed on {}: {}", self.url(), e));

                    if attempt < max_attempts - 1 {
                        if let Err(switch_err) = self.switch_to_fallback() {
                            log(
                                LogTag::Rpc,
                                "ERROR",
                                &format!("Failed to switch fallback: {}", switch_err)
                            );
                            break;
                        }
                    }
                }
            }
        }

        Err(format!("Failed on all {} RPC endpoints: {}", max_attempts, last_error))
    }

    /// Get multiple accounts with automatic fallback support
    pub async fn get_multiple_accounts_with_fallback(
        &mut self,
        pubkeys: &[Pubkey]
    ) -> Result<Vec<Option<Account>>, String> {
        if pubkeys.is_empty() {
            return Ok(Vec::new());
        }

        let max_attempts = self.get_all_urls().len();
        let mut last_error = String::new();

        for attempt in 0..max_attempts {
            match self.get_multiple_accounts(pubkeys).await {
                Ok(accounts) => {
                    return Ok(accounts);
                }
                Err(e) => {
                    last_error = e.clone();
                    log(
                        LogTag::Rpc,
                        "ERROR",
                        &format!("RPC batch call failed on {}: {}", self.url(), e)
                    );

                    if attempt < max_attempts - 1 {
                        if let Err(switch_err) = self.switch_to_fallback() {
                            log(
                                LogTag::Rpc,
                                "ERROR",
                                &format!("Failed to switch fallback: {}", switch_err)
                            );
                            break;
                        }
                    }
                }
            }
        }

        Err(format!("Failed batch request on all {} RPC endpoints: {}", max_attempts, last_error))
    }

    /// Test connection with automatic fallback
    pub async fn test_connection_with_fallback(&mut self) -> Result<(), String> {
        let max_attempts = self.get_all_urls().len();
        let mut last_error = String::new();

        for attempt in 0..max_attempts {
            match self.test_connection().await {
                Ok(()) => {
                    log(
                        LogTag::Rpc,
                        "SUCCESS",
                        &format!("RPC connection test passed on {}", self.url())
                    );
                    return Ok(());
                }
                Err(e) => {
                    last_error = e.clone();
                    log(
                        LogTag::Rpc,
                        "ERROR",
                        &format!("RPC connection test failed on {}: {}", self.url(), e)
                    );

                    if attempt < max_attempts - 1 {
                        if let Err(switch_err) = self.switch_to_fallback() {
                            log(
                                LogTag::Rpc,
                                "ERROR",
                                &format!("Failed to switch fallback: {}", switch_err)
                            );
                            break;
                        }
                    }
                }
            }
        }

        Err(format!("Connection test failed on all {} RPC endpoints: {}", max_attempts, last_error))
    }

    /// Test RPC connection
    pub async fn test_connection(&self) -> Result<(), String> {
        tokio::task
            ::spawn_blocking({
                let client = self.client.clone();
                move || {
                    client
                        .get_slot()
                        .map_err(|e| format!("RPC connection test failed: {}", e))
                        .map(|_| ())
                }
            }).await
            .map_err(|e| format!("Task error: {}", e))?
    }

    /// Get current slot
    pub async fn get_slot(&self) -> Result<u64, String> {
        tokio::task
            ::spawn_blocking({
                let client = self.client.clone();
                move || { client.get_slot().map_err(|e| format!("Failed to get slot: {}", e)) }
            }).await
            .map_err(|e| format!("Task error: {}", e))?
    }
}

/// Global RPC client instance
static mut GLOBAL_RPC_CLIENT: Option<SolanaRpcClient> = None;
static RPC_INIT: std::sync::Once = std::sync::Once::new();

/// Initialize global RPC client from configuration
pub fn init_rpc_client() -> Result<&'static SolanaRpcClient, String> {
    unsafe {
        let mut init_error: Option<String> = None;

        RPC_INIT.call_once(|| {
            match SolanaRpcClient::from_config() {
                Ok(client) => {
                    log(LogTag::Rpc, "SUCCESS", "Global RPC client initialized from configuration");
                    GLOBAL_RPC_CLIENT = Some(client);
                }
                Err(e) => {
                    init_error = Some(e.clone());
                    log(
                        LogTag::Rpc,
                        "ERROR",
                        &format!("Failed to init from config, using fallback: {}", e)
                    );
                    GLOBAL_RPC_CLIENT = Some(SolanaRpcClient::new_with_url(MAINNET_RPC_URL));
                }
            }
        });

        if let Some(error) = init_error {
            Err(error)
        } else {
            Ok(GLOBAL_RPC_CLIENT.as_ref().unwrap())
        }
    }
}

/// Initialize global RPC client with custom URL (legacy method)
pub fn init_rpc_client_with_url(rpc_url: Option<&str>) -> &'static SolanaRpcClient {
    unsafe {
        RPC_INIT.call_once(|| {
            let url = rpc_url.unwrap_or(MAINNET_RPC_URL);
            GLOBAL_RPC_CLIENT = Some(SolanaRpcClient::new_with_url(url));
        });
        GLOBAL_RPC_CLIENT.as_ref().unwrap()
    }
}

/// Get global RPC client instance
pub fn get_rpc_client() -> &'static SolanaRpcClient {
    unsafe {
        if GLOBAL_RPC_CLIENT.is_none() {
            let _ = init_rpc_client(); // Initialize if not already done
        }
        GLOBAL_RPC_CLIENT.as_ref().unwrap()
    }
}

/// Parse string to Pubkey
pub fn parse_pubkey(address: &str) -> Result<Pubkey, String> {
    Pubkey::from_str(address).map_err(|e| format!("Invalid pubkey '{}': {}", address, e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rpc_client_creation() {
        let client = SolanaRpcClient::new();
        assert!(!client.url().is_empty());
    }

    #[tokio::test]
    async fn test_parse_pubkey() {
        let valid_pubkey = "So11111111111111111111111111111111111111112";
        assert!(parse_pubkey(valid_pubkey).is_ok());

        let invalid_pubkey = "invalid";
        assert!(parse_pubkey(invalid_pubkey).is_err());
    }
}
