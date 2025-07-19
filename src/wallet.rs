use std::time::Duration;
use solana_sdk::{ pubkey::Pubkey, signature::{ Keypair, Signer } };
use bs58;
use crate::global::{ is_shutdown, get_config, update_wallet_balance, get_wallet_balance };
use crate::logger::{ log, LogLevel };
use crate::rpc::RPC_MANAGER;

#[derive(Debug)]
pub struct WalletManager {
    pub keypair: Keypair,
    pub pubkey: Pubkey,
}

impl WalletManager {
    pub fn new() -> anyhow::Result<Self> {
        let config = get_config().ok_or_else(|| anyhow::anyhow!("Config not available"))?;
        let config_guard = config.lock().unwrap();

        // Decode the private key from base58
        let private_key_bytes = bs58
            ::decode(&config_guard.main_wallet_private)
            .into_vec()
            .map_err(|e| anyhow::anyhow!("Failed to decode private key: {}", e))?;

        let keypair = Keypair::try_from(&private_key_bytes[..]).map_err(|e|
            anyhow::anyhow!("Failed to create keypair: {}", e)
        )?;

        let pubkey = keypair.pubkey();

        log("WALLET", LogLevel::Info, &format!("Wallet initialized: {}", pubkey));

        Ok(WalletManager {
            keypair,
            pubkey,
        })
    }

    pub async fn check_balance(&self) -> anyhow::Result<f64> {
        // Get balance using a simple RPC call without holding mutex across await
        let balance_lamports = {
            let pubkey_str = self.pubkey.to_string();

            // Try to get balance via simple approach for now
            // In production, would use proper RPC client
            log("WALLET", LogLevel::Info, &format!("Checking balance for {}", pubkey_str));
            1000000000u64 // Mock 1 SOL for now
        };

        let balance_sol = (balance_lamports as f64) / 1_000_000_000.0;
        Ok(balance_sol)
    }

    pub async fn get_token_accounts(&self) -> anyhow::Result<serde_json::Value> {
        let rpc_manager_guard = RPC_MANAGER.lock().unwrap();
        if let Some(rpc_manager) = rpc_manager_guard.as_ref() {
            rpc_manager.get_token_accounts_by_owner(&self.pubkey.to_string(), None).await
        } else {
            Err(anyhow::anyhow!("RPC Manager not initialized"))
        }
    }

    pub async fn get_account_info(&self) -> anyhow::Result<serde_json::Value> {
        let rpc_manager_guard = RPC_MANAGER.lock().unwrap();
        if let Some(rpc_manager) = rpc_manager_guard.as_ref() {
            rpc_manager.get_account_info(&self.pubkey.to_string()).await
        } else {
            Err(anyhow::anyhow!("RPC Manager not initialized"))
        }
    }

    pub fn get_public_key(&self) -> String {
        self.pubkey.to_string()
    }

    pub fn get_keypair(&self) -> &Keypair {
        &self.keypair
    }
}

// Global wallet manager instance
use std::sync::{ Arc, Mutex };
use once_cell::sync::Lazy;

pub static WALLET_MANAGER: Lazy<Arc<Mutex<Option<WalletManager>>>> = Lazy::new(|| {
    Arc::new(Mutex::new(None))
});

pub async fn initialize_wallet_manager() -> anyhow::Result<()> {
    let manager = WalletManager::new()?;
    let mut global_manager = WALLET_MANAGER.lock().unwrap();
    *global_manager = Some(manager);
    Ok(())
}

pub async fn get_wallet_manager() -> anyhow::Result<Arc<Mutex<Option<WalletManager>>>> {
    Ok(WALLET_MANAGER.clone())
}

pub fn start_wallet_manager() {
    tokio::task::spawn(async move {
        tokio::time::sleep(Duration::from_secs(2)).await;

        if let Err(e) = initialize_wallet_manager().await {
            log("WALLET", LogLevel::Error, &format!("Failed to initialize wallet manager: {}", e));
            return;
        }

        log("WALLET", LogLevel::Info, "Wallet Manager initialized successfully");

        let delays = crate::global::get_task_delays();

        loop {
            if is_shutdown() {
                log("WALLET", LogLevel::Info, "Wallet Manager shutting down...");
                break;
            }

            // Check wallet balance periodically
            let balance_result = {
                let manager_guard = WALLET_MANAGER.lock().unwrap();
                if let Some(manager) = manager_guard.as_ref() {
                    let pubkey = manager.pubkey;
                    drop(manager_guard); // Release lock
                    Some(WalletManager {
                        keypair: Keypair::new(), // Generate a new keypair
                        pubkey,
                    })
                } else {
                    None
                }
            };

            if let Some(manager) = balance_result {
                match manager.check_balance().await {
                    Ok(balance) => {
                        let previous_balance = get_wallet_balance();
                        update_wallet_balance(balance);

                        if (balance - previous_balance).abs() > 0.001 {
                            log(
                                "WALLET",
                                LogLevel::Info,
                                &format!(
                                    "Balance updated: {:.6} SOL (change: {:.6})",
                                    balance,
                                    balance - previous_balance
                                )
                            );
                        } else {
                            log(
                                "WALLET",
                                LogLevel::Info,
                                &format!("Current balance: {:.6} SOL", balance)
                            );
                        }
                    }
                    Err(e) => {
                        log("WALLET", LogLevel::Error, &format!("Failed to check balance: {}", e));
                    }
                }
            }

            tokio::time::sleep(Duration::from_secs(delays.wallet_delay)).await;
        }
    });
}
