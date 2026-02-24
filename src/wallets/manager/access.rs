use solana_sdk::signature::Keypair;

use super::super::types::Wallet;

/// Get a wallet by ID
pub async fn get_wallet(wallet_id: i64) -> Result<Option<Wallet>, String> {
    let db_guard = super::WALLETS_DB.read().await;
    let db = db_guard.as_ref().ok_or("Wallet database not initialized")?;

    db.get_wallet(wallet_id)
}

/// Get a wallet by address
pub async fn get_wallet_by_address(address: &str) -> Result<Option<Wallet>, String> {
    let db_guard = super::WALLETS_DB.read().await;
    let db = db_guard.as_ref().ok_or("Wallet database not initialized")?;

    db.get_wallet_by_address(address)
}

/// Get a wallet's keypair by ID
pub async fn get_wallet_keypair(wallet_id: i64) -> Result<Keypair, String> {
    let db_guard = super::WALLETS_DB.read().await;
    let db = db_guard.as_ref().ok_or("Wallet database not initialized")?;

    let (encrypted, nonce) = db
        .get_wallet_encrypted_key(wallet_id)?
        .ok_or("Wallet not found")?;

    super::super::crypto::decrypt_to_keypair(&encrypted, &nonce)
}

/// List all wallets
pub async fn list_wallets(include_inactive: bool) -> Result<Vec<Wallet>, String> {
    let db_guard = super::WALLETS_DB.read().await;
    let db = db_guard.as_ref().ok_or("Wallet database not initialized")?;

    db.list_wallets(include_inactive)
}

/// List active wallets (usable for operations)
pub async fn list_active_wallets() -> Result<Vec<Wallet>, String> {
    let db_guard = super::WALLETS_DB.read().await;
    let db = db_guard.as_ref().ok_or("Wallet database not initialized")?;

    db.list_active_wallets()
}
