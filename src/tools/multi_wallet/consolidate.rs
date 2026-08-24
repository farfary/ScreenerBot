//! Consolidation Operation
//!
//! Manage and cleanup sub-wallets by consolidating funds back to main wallet.

use tokio::time::{sleep, Duration};
use uuid::Uuid;

use crate::chains::solana::rpc::{get_rpc_client, RpcClientMethods};
use crate::logger::{self, LogTag};
use crate::tools::Error;
use crate::wallets::{self, Wallet, WalletRole};

use crate::chains::solana::assets::transfer::{close_ata_for_wallet, transfer_token_for_wallet};

use super::transfer::collect_sol;
use super::types::{ConsolidateConfig, SessionResult, WalletOpResult};

/// Execute a consolidation operation
///
/// Collects SOL and/or tokens from sub-wallets back to main wallet.
/// Optionally closes empty ATAs to reclaim rent.
///
/// # Arguments
/// * `config` - Consolidation configuration
///
/// # Returns
/// Session result with all operation outcomes
pub async fn execute_consolidation(config: ConsolidateConfig) -> Result<SessionResult, Error> {
    // Validate configuration
    config.validate()?;

    let session_id = Uuid::new_v4().to_string();
    let mut result = SessionResult::new(session_id.clone());

    logger::info(
        LogTag::Tools,
        &format!(
            "Starting consolidation session {}: sol={}, tokens={:?}, close_atas={}",
            &session_id[..8],
            config.transfer_sol,
            config.transfer_tokens.as_ref().map(|t| t.len()),
            config.close_atas
        ),
    );

    // Get main wallet as destination
    let main_wallet =
        wallets::get_main_wallet()
            .await?
            .ok_or_else(|| Error::MainWalletUnavailable {
                detail: "no main wallet configured".to_owned(),
            })?;

    // Load sub-wallets to consolidate
    let wallets = load_wallets_for_consolidation(&config).await?;
    if wallets.is_empty() {
        logger::info(LogTag::Tools, "No sub-wallets found for consolidation");
        result.finalize();
        return Ok(result);
    }

    logger::info(
        LogTag::Tools,
        &format!("Consolidating {} sub-wallets", wallets.len()),
    );

    // Transfer tokens first (if configured)
    if let Some(ref token_mints) = config.transfer_tokens {
        for mint in token_mints {
            for wallet in &wallets {
                let op_result = transfer_token_to_main(
                    wallet,
                    &main_wallet.address,
                    mint,
                    config.include_token_2022,
                )
                .await;

                if let Some(r) = op_result {
                    result.add_operation(r);
                }

                // Small delay between transfers
                sleep(Duration::from_millis(100)).await;
            }
        }
    }

    // Close ATAs (if configured)
    if config.close_atas {
        for wallet in &wallets {
            let ata_results = close_wallet_atas(wallet, config.include_token_2022).await;
            for r in ata_results {
                result.add_operation(r);
            }
        }
    }

    // Transfer SOL last (if configured)
    if config.transfer_sol {
        let sol_results =
            collect_sol(wallets, &main_wallet.address, config.leave_rent_exempt).await;

        result.total_sol_recovered = sol_results
            .iter()
            .filter(|r| r.success)
            .filter_map(|r| r.amount_sol)
            .sum();

        for r in sol_results {
            result.add_operation(r);
        }
    }

    result.finalize();

    logger::info(
        LogTag::Tools,
        &format!(
            "Consolidation session {} complete: {}/{} successful, {:.6} SOL recovered",
            &session_id[..8],
            result.successful_ops,
            result.total_wallets,
            result.total_sol_recovered
        ),
    );

    Ok(result)
}

/// Load wallets for consolidation
async fn load_wallets_for_consolidation(config: &ConsolidateConfig) -> Result<Vec<Wallet>, Error> {
    let all_wallets = wallets::list_active_wallets().await?;

    let selected: Vec<Wallet> = all_wallets
        .into_iter()
        .filter(|w| {
            // Only secondary wallets
            if w.role != WalletRole::Secondary {
                return false;
            }
            // Filter by ID if specified
            if let Some(ref ids) = config.wallet_ids {
                return ids.contains(&w.id);
            }
            true
        })
        .collect();

    Ok(selected)
}

/// Transfer all tokens of a specific mint from wallet to main
async fn transfer_token_to_main(
    wallet: &Wallet,
    main_address: &str,
    mint: &str,
    include_token_2022: bool,
) -> Option<WalletOpResult> {
    let rpc_client = get_rpc_client();
    let wallet_id = wallet.id;
    let wallet_address = wallet.address.clone();

    // Get token balance
    let balance = match rpc_client.get_token_balance(&wallet_address, mint).await {
        Ok(b) => b,
        Err(_) => return None,
    };

    if balance == 0 {
        return None;
    }

    if crate::chains::adapter().validate_address(mint).is_err() {
        return Some(WalletOpResult::failure(
            wallet_id,
            wallet_address,
            "Invalid mint address".to_owned(),
        ));
    }

    let is_token_2022 = if include_token_2022 {
        rpc_client
            .is_token_2022_mint_str(mint)
            .await
            .unwrap_or_default()
    } else {
        false
    };

    // transfer_token now fetches decimals directly from the mint account
    match transfer_token_for_wallet(wallet_id, main_address, mint, balance, is_token_2022).await {
        Ok(sig) => Some(WalletOpResult::success(
            wallet_id,
            wallet_address,
            sig,
            0.0, // No SOL spent
            Some(balance as f64),
        )),
        Err(e) => Some(WalletOpResult::failure(wallet_id, wallet_address, e)),
    }
}

/// Close all empty ATAs for a wallet
async fn close_wallet_atas(wallet: &Wallet, include_token_2022: bool) -> Vec<WalletOpResult> {
    let rpc_client = get_rpc_client();
    let wallet_id = wallet.id;
    let wallet_address = wallet.address.clone();
    let mut results = Vec::new();

    // Get all token accounts
    let token_accounts = match rpc_client.get_all_token_accounts_str(&wallet_address).await {
        Ok(accounts) => accounts,
        Err(e) => {
            logger::debug(
                LogTag::Tools,
                &format!(
                    "Failed to get token accounts for {}: {}",
                    &wallet_address[..8],
                    e
                ),
            );
            return results;
        }
    };

    for account_info in token_accounts {
        // Skip non-empty accounts
        if account_info.balance > 0 {
            continue;
        }

        // Skip Token-2022 if not included
        if !include_token_2022 && account_info.is_token_2022 {
            continue;
        }

        match close_ata_for_wallet(wallet_id, &account_info.mint, account_info.is_token_2022).await
        {
            Ok(sig) => {
                // Rent reclaimed is approximately 0.00203 SOL
                results.push(WalletOpResult::success(
                    wallet_id,
                    wallet_address.clone(),
                    sig,
                    0.00203, // Approximate rent reclaimed
                    None,
                ));
            }
            Err(e) => {
                logger::debug(
                    LogTag::Tools,
                    &format!(
                        "Failed to close ATA for mint {} in wallet {}: {}",
                        &account_info.mint[..8],
                        &wallet_address[..8],
                        e
                    ),
                );
            }
        }

        // Small delay between closes
        sleep(Duration::from_millis(50)).await;
    }

    results
}
