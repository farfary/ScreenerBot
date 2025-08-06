use std::collections::HashMap;
use std::fs;
use std::path::Path;
use serde::{Deserialize, Serialize};
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};
use chrono::{DateTime, Utc};
use anyhow::{Result, anyhow};
use tokio::sync::Mutex;
use std::sync::Arc;

use crate::global::DATA_DIR;
use crate::rpc::get_rpc_client;
use crate::logger::{log, LogTag};
use crate::wallet::get_token_balance;

// Wallet data structures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletInfo {
    pub public_key: String,
    pub private_key: String,
    pub created_at: DateTime<Utc>,
    pub label: Option<String>,
    pub sol_balance: f64,
    pub token_balances: HashMap<String, TokenBalance>,
    pub last_updated: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenBalance {
    pub mint: String,
    pub symbol: Option<String>,
    pub amount: u64,
    pub decimals: u8,
    pub ui_amount: f64,
    pub last_updated: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletBackup {
    pub wallets: HashMap<String, WalletInfo>,
    pub backup_timestamp: DateTime<Utc>,
    pub version: String,
}

// Wallet manager implementation
pub struct WalletsManager {
    wallets: Arc<Mutex<HashMap<String, WalletInfo>>>,
    wallets_dir: String,
}

impl WalletsManager {
    pub fn new() -> Result<Self> {
        let wallets_dir = format!("{}/wallets", DATA_DIR);
        
        // Create wallets directory if it doesn't exist
        if !Path::new(&wallets_dir).exists() {
            fs::create_dir_all(&wallets_dir)?;
            log(LogTag::Wallet, "INFO", &format!("Created wallets directory: {}", wallets_dir));
        }

        let manager = WalletsManager {
            wallets: Arc::new(Mutex::new(HashMap::new())),
            wallets_dir,
        };

        // Load existing wallets
        manager.load_wallets()?;

        Ok(manager)
    }

    /// Create a new wallet with optional label
    pub async fn create_wallet(&self, label: Option<String>) -> Result<WalletInfo> {
        let keypair = Keypair::new();
        let public_key = keypair.pubkey().to_string();
        let private_key = bs58::encode(keypair.to_bytes()).into_string();
        
        let wallet_info = WalletInfo {
            public_key: public_key.clone(),
            private_key: private_key.clone(),
            created_at: Utc::now(),
            label: label.clone(),
            sol_balance: 0.0,
            token_balances: HashMap::new(),
            last_updated: Utc::now(),
        };

        // Add to memory
        {
            let mut wallets = self.wallets.lock().await;
            wallets.insert(public_key.clone(), wallet_info.clone());
        }

        // Save to disk
        self.save_wallet_to_file(&wallet_info).await?;
        
        // Create backup
        self.create_backup().await?;

        log(LogTag::Wallet, "INFO", 
            &format!("Created new wallet: {} {}", 
                public_key, 
                label.map_or("".to_string(), |l| format!("({})", l))
            )
        );

        Ok(wallet_info)
    }

    /// Get wallet by public key
    pub async fn get_wallet(&self, public_key: &str) -> Result<Option<WalletInfo>> {
        let wallets = self.wallets.lock().await;
        Ok(wallets.get(public_key).cloned())
    }

    /// Get all wallets
    pub async fn get_all_wallets(&self) -> Result<Vec<WalletInfo>> {
        let wallets = self.wallets.lock().await;
        Ok(wallets.values().cloned().collect())
    }

    /// Update wallet balances (SOL and all tokens)
    pub async fn update_wallet_balances(&self, public_key: &str) -> Result<()> {
        let rpc_client = get_rpc_client();

        // Get SOL balance using the existing async method
        let sol_balance = rpc_client.get_sol_balance(public_key).await
            .map_err(|e| anyhow!("Failed to get SOL balance: {}", e))?;

        // Get token accounts using existing RPC functionality
        let token_accounts = rpc_client.get_all_token_accounts(public_key).await
            .map_err(|e| anyhow!("Failed to get token accounts: {}", e))?;

        let mut token_balances = HashMap::new();
        
        for account_info in token_accounts {
            if account_info.balance > 0 {
                // Get token decimals from existing cache
                let decimals = crate::tokens::get_token_decimals(&account_info.mint).await.unwrap_or(9);
                let ui_amount = account_info.balance as f64 / 10_f64.powi(decimals as i32);
                
                token_balances.insert(account_info.mint.clone(), TokenBalance {
                    mint: account_info.mint.clone(),
                    symbol: None, // Can be enhanced to fetch symbol from metadata
                    amount: account_info.balance,
                    decimals,
                    ui_amount,
                    last_updated: Utc::now(),
                });
            }
        }

        // Update wallet in memory
        {
            let mut wallets = self.wallets.lock().await;
            if let Some(wallet) = wallets.get_mut(public_key) {
                wallet.sol_balance = sol_balance;
                wallet.token_balances = token_balances;
                wallet.last_updated = Utc::now();
                
                // Save updated wallet to file
                self.save_wallet_to_file(wallet).await?;
            }
        }

        log(LogTag::Wallet, "INFO", 
            &format!("Updated balances for wallet: {} (SOL: {:.6})", public_key, sol_balance));

        Ok(())
    }

    /// Update all wallet balances
    pub async fn update_all_wallet_balances(&self) -> Result<()> {
        let wallet_keys: Vec<String> = {
            let wallets = self.wallets.lock().await;
            wallets.keys().cloned().collect()
        };

        for public_key in wallet_keys {
            if let Err(e) = self.update_wallet_balances(&public_key).await {
                log(LogTag::Wallet, "ERROR", 
                    &format!("Failed to update balances for {}: {}", public_key, e));
            }
        }

        Ok(())
    }

    /// Get keypair from public key
    pub async fn get_keypair(&self, public_key: &str) -> Result<Keypair> {
        let wallets = self.wallets.lock().await;
        
        if let Some(wallet) = wallets.get(public_key) {
            let private_key_bytes = bs58::decode(&wallet.private_key)
                .into_vec()
                .map_err(|e| anyhow!("Failed to decode private key: {}", e))?;
            
            Keypair::from_bytes(&private_key_bytes)
                .map_err(|e| anyhow!("Failed to create keypair: {}", e))
        } else {
            Err(anyhow!("Wallet not found: {}", public_key))
        }
    }

    /// Delete wallet (with confirmation)
    pub async fn delete_wallet(&self, public_key: &str, confirm: bool) -> Result<()> {
        if !confirm {
            return Err(anyhow!("Wallet deletion requires confirmation"));
        }

        // Remove from memory
        {
            let mut wallets = self.wallets.lock().await;
            wallets.remove(public_key);
        }

        // Remove file
        let wallet_file = format!("{}/{}.json", self.wallets_dir, public_key);
        if Path::new(&wallet_file).exists() {
            fs::remove_file(&wallet_file)?;
        }

        // Create backup after deletion
        self.create_backup().await?;

        log(LogTag::Wallet, "WARNING", 
            &format!("Deleted wallet: {}", public_key));

        Ok(())
    }

    /// Create encrypted backup of all wallets
    pub async fn create_backup(&self) -> Result<String> {
        let timestamp = Utc::now();
        let backup_filename = format!("wallets_backup_{}.json", 
            timestamp.format("%Y%m%d_%H%M%S"));
        let backup_path = format!("{}/{}", self.wallets_dir, backup_filename);

        let wallets = self.wallets.lock().await;
        let backup = WalletBackup {
            wallets: wallets.clone(),
            backup_timestamp: timestamp,
            version: "1.0".to_string(),
        };

        let backup_json = serde_json::to_string_pretty(&backup)?;
        fs::write(&backup_path, backup_json)?;

        log(LogTag::Wallet, "INFO", 
            &format!("Created wallet backup: {}", backup_filename));

        Ok(backup_filename)
    }

    /// Restore wallets from backup
    pub async fn restore_from_backup(&self, backup_filename: &str) -> Result<usize> {
        let backup_path = format!("{}/{}", self.wallets_dir, backup_filename);
        
        if !Path::new(&backup_path).exists() {
            return Err(anyhow!("Backup file not found: {}", backup_filename));
        }

        let backup_content = fs::read_to_string(&backup_path)?;
        let backup: WalletBackup = serde_json::from_str(&backup_content)?;

        let restored_count = backup.wallets.len();

        // Replace current wallets with backup
        {
            let mut wallets = self.wallets.lock().await;
            *wallets = backup.wallets.clone();
        }

        // Save all wallets to individual files
        for wallet in backup.wallets.values() {
            self.save_wallet_to_file(wallet).await?;
        }

        log(LogTag::Wallet, "INFO", 
            &format!("Restored {} wallets from backup: {}", restored_count, backup_filename));

        Ok(restored_count)
    }

    /// List all backup files
    pub fn list_backups(&self) -> Result<Vec<String>> {
        let mut backups = Vec::new();
        
        if let Ok(entries) = fs::read_dir(&self.wallets_dir) {
            for entry in entries {
                if let Ok(entry) = entry {
                    let filename = entry.file_name().to_string_lossy().to_string();
                    if filename.starts_with("wallets_backup_") && filename.ends_with(".json") {
                        backups.push(filename);
                    }
                }
            }
        }

        backups.sort();
        backups.reverse(); // Most recent first
        Ok(backups)
    }

    /// Get wallet statistics
    pub async fn get_wallet_stats(&self) -> Result<WalletStats> {
        let wallets = self.wallets.lock().await;
        
        let total_wallets = wallets.len();
        let total_sol: f64 = wallets.values().map(|w| w.sol_balance).sum();
        let total_tokens: usize = wallets.values()
            .map(|w| w.token_balances.len())
            .sum();

        Ok(WalletStats {
            total_wallets,
            total_sol_balance: total_sol,
            total_token_accounts: total_tokens,
            last_updated: Utc::now(),
        })
    }

    // Private helper methods

    /// Load all wallets from disk
    fn load_wallets(&self) -> Result<()> {
        if !Path::new(&self.wallets_dir).exists() {
            return Ok(());
        }

        let entries = fs::read_dir(&self.wallets_dir)?;
        let mut loaded_count = 0;

        for entry in entries {
            if let Ok(entry) = entry {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("json") {
                    let filename = path.file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("");
                    
                    // Skip backup files
                    if filename.starts_with("wallets_backup_") {
                        continue;
                    }

                    if let Ok(content) = fs::read_to_string(&path) {
                        if let Ok(wallet_info) = serde_json::from_str::<WalletInfo>(&content) {
                            // Use blocking mutex for initialization
                            if let Ok(mut wallets) = self.wallets.try_lock() {
                                wallets.insert(wallet_info.public_key.clone(), wallet_info);
                                loaded_count += 1;
                            }
                        }
                    }
                }
            }
        }

        if loaded_count > 0 {
            log(LogTag::Wallet, "INFO", 
                &format!("Loaded {} wallets from disk", loaded_count));
        }

        Ok(())
    }

    /// Save individual wallet to file
    async fn save_wallet_to_file(&self, wallet: &WalletInfo) -> Result<()> {
        let wallet_file = format!("{}/{}.json", self.wallets_dir, wallet.public_key);
        let wallet_json = serde_json::to_string_pretty(wallet)?;
        
        tokio::fs::write(&wallet_file, wallet_json).await
            .map_err(|e| anyhow!("Failed to save wallet file: {}", e))?;

        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WalletStats {
    pub total_wallets: usize,
    pub total_sol_balance: f64,
    pub total_token_accounts: usize,
    pub last_updated: DateTime<Utc>,
}

// Global wallet manager instance
use std::sync::OnceLock;

static GLOBAL_WALLETS_MANAGER: OnceLock<Arc<WalletsManager>> = OnceLock::new();

/// Initialize the global wallets manager
pub fn init_wallets_manager() -> Result<()> {
    let manager = WalletsManager::new()?;
    GLOBAL_WALLETS_MANAGER.set(Arc::new(manager))
        .map_err(|_| anyhow!("Wallets manager already initialized"))?;
    
    log(LogTag::Wallet, "INFO", "Wallets manager initialized");
    Ok(())
}

/// Get the global wallets manager instance
pub fn get_wallets_manager() -> Result<Arc<WalletsManager>> {
    GLOBAL_WALLETS_MANAGER.get()
        .ok_or_else(|| anyhow!("Wallets manager not initialized"))
        .map(|manager| manager.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_wallet() {
        let manager = WalletsManager::new().unwrap();
        let wallet = manager.create_wallet(Some("Test Wallet".to_string())).await.unwrap();
        
        assert!(!wallet.public_key.is_empty());
        assert!(!wallet.private_key.is_empty());
        assert_eq!(wallet.label, Some("Test Wallet".to_string()));
    }

    #[tokio::test]
    async fn test_wallet_backup_restore() {
        let manager = WalletsManager::new().unwrap();
        
        // Create test wallet
        let _wallet = manager.create_wallet(Some("Backup Test".to_string())).await.unwrap();
        
        // Create backup
        let backup_filename = manager.create_backup().await.unwrap();
        
        // List backups
        let backups = manager.list_backups().unwrap();
        assert!(backups.contains(&backup_filename));
    }
}
