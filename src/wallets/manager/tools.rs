//! Wallet tools — utility functions for wallet diagnostics and repair.

use super::super::crypto::decrypt_to_keypair;
use super::super::types::{WalletWithKey, WalletsSummary};
use crate::logger::{self, LogTag};

/// Get all wallets with their keypairs for tools
pub async fn get_wallets_with_keys() -> Result<Vec<WalletWithKey>, String> {
    let db_guard = super::WALLETS_DB.read().await;
    let db = db_guard.as_ref().ok_or("Wallet database not initialized")?;

    let wallets = db.list_active_wallets()?;
    let mut result = Vec::with_capacity(wallets.len());

    for wallet in wallets {
        let (encrypted, nonce) = match db.get_wallet_encrypted_key(wallet.id)? {
            Some(data) => data,
            None => {
                logger::warning(
                    LogTag::Wallet,
                    &format!(
                        "Skipping wallet id={} ({}): encrypted key not found",
                        wallet.id, wallet.address
                    ),
                );
                continue;
            }
        };

        let keypair = match decrypt_to_keypair(&encrypted, &nonce) {
            Ok(kp) => kp,
            Err(e) => {
                logger::warning(
                    LogTag::Wallet,
                    &format!(
                        "Skipping wallet id={} ({}): decryption failed - {}",
                        wallet.id, wallet.address, e
                    ),
                );
                continue;
            }
        };

        result.push(WalletWithKey { wallet, keypair });
    }

    Ok(result)
}

/// Update last used timestamp for a wallet
pub async fn update_last_used(wallet_id: i64) -> Result<(), String> {
    let db_guard = super::WALLETS_DB.read().await;
    let db = db_guard.as_ref().ok_or("Wallet database not initialized")?;

    db.update_last_used(wallet_id)
}

/// Get wallets summary for dashboard
pub async fn get_wallets_summary() -> Result<WalletsSummary, String> {
    let db_guard = super::WALLETS_DB.read().await;
    let db = db_guard.as_ref().ok_or("Wallet database not initialized")?;

    let (total, active) = db.get_wallet_counts()?;

    let main_wallet = db.get_main_wallet()?;

    Ok(WalletsSummary {
        total_count: total,
        active_count: active,
        main_wallet: main_wallet.as_ref().map(|w| w.address.clone()),
        main_wallet_name: main_wallet.as_ref().map(|w| w.name.clone()),
        total_sol: 0.0, // Will be updated by balance fetching
    })
}
