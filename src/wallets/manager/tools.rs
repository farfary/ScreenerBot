//! Wallet tools — utility functions for wallet diagnostics and repair.

use super::super::types::WalletsSummary;

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
