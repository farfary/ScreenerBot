// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Consolidation Operation
//!
//! Manage and cleanup sub-wallets by consolidating funds back to main wallet.

use tokio::time::{sleep, Duration};
use uuid::Uuid;

use crate::actions::ActionFailure;
use crate::chains::adapter;
use crate::chains::solana::rpc::{get_rpc_client, RpcClientMethods, TokenAccountInfo};
use crate::i18n::ids;
use crate::logger::{self, LogTag};
use crate::tools::Error;
use crate::wallets::{self, Wallet, WalletRole};

use crate::chains::solana::assets::transfer::{close_ata_for_wallet, transfer_token_for_wallet};

use super::transfer::collect_sol;
use super::types::{ConsolidateConfig, ConsolidationResult, SessionResult, WalletOpResult};

/// What a wallet's token accounts offer a consolidation: the mints holding a
/// transferable balance and the empty accounts a cleanup can close. Frozen
/// accounts are neither: the token program refuses to move or close them.
#[derive(Debug, Default, PartialEq)]
pub struct TokenHoldings {
    pub mints: Vec<String>,
    pub empty_accounts: usize,
    pub reclaimable_rent: f64,
}

impl TokenHoldings {
    pub fn from_accounts(accounts: &[TokenAccountInfo]) -> Self {
        let mut holdings = Self::default();
        let mut reclaimable_lamports: u64 = 0;
        for account in accounts.iter().filter(|a| !a.is_frozen) {
            if account.balance > 0 {
                if !holdings.mints.contains(&account.mint) {
                    holdings.mints.push(account.mint.clone());
                }
            } else {
                holdings.empty_accounts += 1;
                reclaimable_lamports = reclaimable_lamports.saturating_add(account.lamports);
            }
        }
        holdings.reclaimable_rent = adapter().raw_to_native(reclaimable_lamports);
        holdings
    }
}

/// Execute a consolidation operation
///
/// Moves tokens and/or SOL from the selected sub-wallets back to the main
/// wallet, and optionally closes their empty token accounts to reclaim rent.
/// Tokens move first, accounts close next and SOL moves last, so the rent a
/// close returns is swept with the SOL when both are requested.
///
/// # Arguments
/// * `config` - Consolidation configuration
///
/// # Returns
/// Every operation's outcome, with what was transferred and closed
pub async fn execute_consolidation(
    config: ConsolidateConfig,
) -> Result<ConsolidationResult, Error> {
    // Validate configuration
    config.validate()?;

    let session_id = Uuid::new_v4().to_string();
    let mut session = SessionResult::new(session_id.clone());

    logger::info(
        LogTag::Tools,
        &format!(
            "Starting consolidation session {}: wallets={}, sol={}, tokens={}, close_atas={}",
            &session_id[..8],
            config.wallet_ids.len(),
            config.transfer_sol,
            config.transfer_tokens.len(),
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

    let wallets = load_wallets_for_consolidation(&config.wallet_ids).await?;
    let wallet_count = wallets.len();
    let mut tokens_transferred = 0;
    let mut accounts_closed = 0;
    let mut rent_reclaimed_sol = 0.0;
    let mut sol_transferred = 0.0;

    for mint in &config.transfer_tokens {
        for wallet in &wallets {
            let op_result = transfer_token_to_main(
                wallet,
                &main_wallet.address,
                mint,
                config.include_token_2022,
            )
            .await;

            if let Some(r) = op_result {
                if r.success {
                    tokens_transferred += 1;
                }
                session.add_operation(r);
            }

            // Small delay between transfers
            sleep(Duration::from_millis(100)).await;
        }
    }

    if config.close_atas {
        for wallet in &wallets {
            for r in close_wallet_atas(wallet, config.include_token_2022).await {
                if r.success {
                    accounts_closed += 1;
                    rent_reclaimed_sol += r.amount_sol.unwrap_or_default();
                }
                session.add_operation(r);
            }
        }
    }

    // Transfer SOL last (if configured)
    if config.transfer_sol {
        let sol_results =
            collect_sol(wallets, &main_wallet.address, config.leave_rent_exempt).await;

        sol_transferred = sol_results
            .iter()
            .filter(|r| r.success)
            .filter_map(|r| r.amount_sol)
            .sum();

        for r in sol_results {
            session.add_operation(r);
        }
    }

    session.total_sol_recovered = sol_transferred;
    session.finalize();

    logger::info(
        LogTag::Tools,
        &format!(
            "Consolidation session {} complete: {}/{} operations successful, {} token transfers, {} accounts closed ({:.9} SOL rent), {:.9} SOL transferred",
            &session_id[..8],
            session.successful_ops,
            session.operations.len(),
            tokens_transferred,
            accounts_closed,
            rent_reclaimed_sol,
            sol_transferred
        ),
    );

    Ok(ConsolidationResult {
        session,
        wallets: wallet_count,
        sol_transferred,
        tokens_transferred,
        accounts_closed,
        rent_reclaimed_sol,
    })
}

/// The active secondary wallets named by `wallet_ids`, in that order. An id
/// that is not an active secondary wallet rejects the whole request: the main
/// wallet is the destination, and a stale selection must not move funds.
async fn load_wallets_for_consolidation(wallet_ids: &[i64]) -> Result<Vec<Wallet>, Error> {
    let secondary: Vec<Wallet> = wallets::list_active_wallets()
        .await?
        .into_iter()
        .filter(|w| w.role == WalletRole::Secondary)
        .collect();
    select_sub_wallets(secondary, wallet_ids)
}

/// Pick `wallet_ids` out of the active secondary wallets, refusing an id that
/// is not one of them.
fn select_sub_wallets(secondary: Vec<Wallet>, wallet_ids: &[i64]) -> Result<Vec<Wallet>, Error> {
    let mut by_id: std::collections::HashMap<i64, Wallet> =
        secondary.into_iter().map(|w| (w.id, w)).collect();
    let mut selected = Vec::with_capacity(wallet_ids.len());
    for id in wallet_ids {
        let wallet = by_id.remove(id).ok_or_else(|| Error::InvalidConfig {
            detail: format!("wallet {id} is not an active sub-wallet"),
        })?;
        selected.push(wallet);
    }
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
            ActionFailure::new(ids::TOOLS_MULTI_OP_MINT_INVALID),
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
            None,
            None,
        )),
        Err(e) => Some(WalletOpResult::failure(
            wallet_id,
            wallet_address,
            ActionFailure::with_details(ids::TOOLS_MULTI_OP_TRANSFER_FAILED, e.to_string()),
        )),
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
            logger::warning(
                LogTag::Tools,
                &format!("Failed to get token accounts for {wallet_address}: {e}"),
            );
            results.push(WalletOpResult::failure(
                wallet_id,
                wallet_address,
                ActionFailure::with_details(ids::TOOLS_MULTI_OP_BALANCE_FAILED, e.to_string()),
            ));
            return results;
        }
    };

    for account_info in token_accounts {
        // Skip non-empty accounts, and frozen ones the token program refuses to close
        if account_info.balance > 0 || account_info.is_frozen {
            continue;
        }

        // Skip Token-2022 if not included
        if !include_token_2022 && account_info.is_token_2022 {
            continue;
        }

        match close_ata_for_wallet(wallet_id, &account_info.mint, account_info.is_token_2022).await
        {
            // The close returns the account's own deposit, whatever rent rate it
            // was opened at.
            Ok(sig) => results.push(WalletOpResult::success(
                wallet_id,
                wallet_address.clone(),
                sig,
                adapter().raw_to_native(account_info.lamports),
                None,
                None,
                None,
            )),
            Err(e) => {
                logger::warning(
                    LogTag::Tools,
                    &format!(
                        "Failed to close token account {} (mint {}) in wallet {}: {}",
                        account_info.account, account_info.mint, wallet_address, e
                    ),
                );
                results.push(WalletOpResult::failure(
                    wallet_id,
                    wallet_address.clone(),
                    ActionFailure::with_details(ids::TOOLS_MULTI_OP_CLOSE_FAILED, e.to_string()),
                ));
            }
        }

        // Small delay between closes
        sleep(Duration::from_millis(50)).await;
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wallets::WalletType;

    fn sub_wallet(id: i64) -> Wallet {
        Wallet {
            id,
            name: format!("sub {id}"),
            address: format!("address-{id}"),
            role: WalletRole::Secondary,
            wallet_type: WalletType::Generated,
            created_at: chrono::Utc::now(),
            last_used_at: None,
            notes: None,
            is_active: true,
        }
    }

    #[test]
    fn only_the_selected_sub_wallets_are_consolidated() {
        let selected =
            select_sub_wallets(vec![sub_wallet(1), sub_wallet(2), sub_wallet(3)], &[3, 1])
                .expect("both ids are sub-wallets");
        let ids: Vec<i64> = selected.iter().map(|w| w.id).collect();
        assert_eq!(ids, vec![3, 1]);
    }

    #[test]
    fn a_selected_id_that_is_not_an_active_sub_wallet_rejects_the_request() {
        let error = select_sub_wallets(vec![sub_wallet(1)], &[1, 7]).unwrap_err();
        assert!(matches!(error, Error::InvalidConfig { .. }), "{error:?}");
    }

    #[test]
    fn a_request_without_a_selection_or_an_operation_is_rejected() {
        let config = ConsolidateConfig {
            wallet_ids: vec![1],
            transfer_sol: true,
            transfer_tokens: Vec::new(),
            close_atas: false,
            include_token_2022: true,
            leave_rent_exempt: false,
        };
        assert!(config.validate().is_ok());

        let no_wallets = ConsolidateConfig {
            wallet_ids: Vec::new(),
            ..config.clone()
        };
        assert!(matches!(
            no_wallets.validate(),
            Err(Error::InvalidConfig { .. })
        ));

        let no_operation = ConsolidateConfig {
            transfer_sol: false,
            ..config
        };
        assert!(matches!(
            no_operation.validate(),
            Err(Error::InvalidConfig { .. })
        ));
    }

    fn account(mint: &str, balance: u64, lamports: u64, is_frozen: bool) -> TokenAccountInfo {
        TokenAccountInfo {
            account: format!("{mint}-account-{balance}"),
            mint: mint.to_owned(),
            balance,
            decimals: 6,
            is_token_2022: false,
            is_nft: false,
            is_frozen,
            lamports,
        }
    }

    #[test]
    fn holdings_name_each_transferable_mint_and_each_closable_account() {
        let holdings = TokenHoldings::from_accounts(&[
            account("A", 10, 2_039_280, false),
            account("A", 5, 2_039_280, false),
            account("B", 0, 2_039_280, false),
            // Opened at a later, lower rent rate: its own deposit is reclaimed.
            account("C", 0, 1_000_000, false),
            account("D", 7, 2_039_280, true),
            account("E", 0, 2_039_280, true),
        ]);
        assert_eq!(holdings.mints, vec!["A".to_owned()]);
        assert_eq!(holdings.empty_accounts, 2);
        assert!((holdings.reclaimable_rent - 0.00303928).abs() < 1e-12);
    }
}
