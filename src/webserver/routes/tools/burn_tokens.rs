//! Burn tokens handlers

use axum::response::Response;
use axum::Json;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signer::Signer;
use solana_sdk::transaction::Transaction;
use spl_token::instruction as spl_instruction;
use std::collections::HashMap;
use std::str::FromStr;

use crate::constants::SOL_MINT;
use crate::logger::{self, LogTag};
use crate::pools;
use crate::positions;
use crate::rpc::{get_rpc_client, RpcClientMethods};
use crate::tokens::decimals;
use crate::utils::{get_all_token_accounts, get_wallet_address};
use crate::webserver::utils::{error_response, success_response};

use super::types::*;

// =============================================================================
// Burn Tokens Handlers
// =============================================================================

/// Scan wallet for tokens that can be burned, with categorization
pub async fn scan_burnable_tokens() -> Response {
    // ATA rent is approximately 0.00203928 SOL
    const ATA_RENT_SOL: f64 = 0.00203928;

    // Get wallet address
    let wallet_address = match get_wallet_address() {
        Ok(addr) => addr,
        Err(e) => {
            logger::error(
                LogTag::Tools,
                &format!("Failed to get wallet address: {}", e),
            );
            return error_response(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "WALLET_ERROR",
                "Failed to get wallet address",
                Some(&e.to_string()),
            );
        }
    };

    // Get all token accounts
    let all_accounts = match get_all_token_accounts(&wallet_address).await {
        Ok(accounts) => accounts,
        Err(e) => {
            logger::error(
                LogTag::Tools,
                &format!("Failed to get token accounts: {}", e),
            );
            return error_response(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "SCAN_ERROR",
                "Failed to scan token accounts",
                Some(&e.to_string()),
            );
        }
    };

    // Get open and closed positions for categorization
    let open_positions = positions::get_open_positions().await;
    let closed_positions = positions::get_closed_positions().await;

    let open_position_mints: std::collections::HashSet<String> =
        open_positions.iter().map(|p| p.mint.clone()).collect();
    let closed_position_mints: std::collections::HashSet<String> =
        closed_positions.iter().map(|p| p.mint.clone()).collect();

    // Get token metadata in batch
    let mints: Vec<String> = all_accounts.iter().map(|acc| acc.mint.clone()).collect();
    let mut metadata_map: HashMap<String, (Option<String>, Option<String>)> = HashMap::new();

    if let Some(db) = crate::tokens::database::get_global_database() {
        for mint in &mints {
            if let Ok(Some(meta)) = db.get_token(mint) {
                metadata_map.insert(mint.clone(), (meta.symbol.clone(), meta.name.clone()));
            }
        }
    }

    // Build token list with categorization
    let mut tokens: Vec<BurnableTokenInfo> = Vec::new();
    let mut categories = BurnTokensCategories {
        open_positions: 0,
        closed_positions: 0,
        has_value: 0,
        zero_liquidity: 0,
    };
    let mut total_rent_reclaimable = 0.0f64;

    for account in &all_accounts {
        // Skip SOL itself and NFTs
        if account.mint == SOL_MINT || account.is_nft {
            continue;
        }

        // Skip empty accounts (they should use ATA cleanup instead)
        if account.balance == 0 {
            continue;
        }

        let (symbol, name) = metadata_map
            .get(&account.mint)
            .cloned()
            .unwrap_or((None, None));

        // Get price from pools module
        let price_result = pools::get_pool_price(&account.mint);
        let price_sol = price_result.as_ref().map(|p| p.price_sol);
        let has_liquidity = price_result
            .as_ref()
            .map(|p| p.sol_reserves > 0.0)
            .unwrap_or_default();

        // Calculate UI amount and value
        let ui_amount = account.balance as f64 / 10f64.powi(account.decimals as i32);
        let value_sol = price_sol.map(|p| p * ui_amount);

        // Determine category
        let (category, category_label, can_burn, burn_warning) =
            if open_position_mints.contains(&account.mint) {
                categories.open_positions += 1;
                (
                    TokenCategory::OpenPosition,
                    "Open Position".to_string(),
                    false,
                    Some("Cannot burn tokens from open positions".to_string()),
                )
            } else if closed_position_mints.contains(&account.mint) {
                categories.closed_positions += 1;
                (
                    TokenCategory::ClosedPosition,
                    "Closed Position".to_string(),
                    true,
                    Some("Leftover from closed position".to_string()),
                )
            } else if has_liquidity && value_sol.is_some_and(|v| v > 0.0001) {
                categories.has_value += 1;
                (
                    TokenCategory::HasValue,
                    "Has Value".to_string(),
                    true,
                    Some(format!("Worth ~{:.6} SOL", value_sol.unwrap_or_default())),
                )
            } else {
                categories.zero_liquidity += 1;
                (
                    TokenCategory::ZeroLiquidity,
                    "Zero Liquidity".to_string(),
                    true,
                    None,
                )
            };

        // Only count rent reclaimable for tokens we can burn
        if can_burn {
            total_rent_reclaimable += ATA_RENT_SOL;
        }

        tokens.push(BurnableTokenInfo {
            mint: account.mint.clone(),
            symbol,
            name,
            balance: account.balance,
            ui_amount,
            decimals: account.decimals,
            is_token_2022: account.is_token_2022,
            category,
            category_label,
            price_sol,
            value_sol,
            has_liquidity,
            can_burn,
            burn_warning,
            rent_reclaimable_sol: if can_burn { ATA_RENT_SOL } else { 0.0 },
        });
    }

    // Sort tokens: Open positions first (can't burn), then by category
    tokens.sort_by(|a, b| {
        // Sort by category priority (open positions first as warning, then value, then zero liquidity)
        let priority = |t: &BurnableTokenInfo| match t.category {
            TokenCategory::OpenPosition => 0,
            TokenCategory::HasValue => 1,
            TokenCategory::ClosedPosition => 2,
            TokenCategory::ZeroLiquidity => 3,
        };

        let p1 = priority(a);
        let p2 = priority(b);

        if p1 != p2 {
            return p1.cmp(&p2);
        }

        // Within same category, sort by value (highest first)
        b.value_sol
            .unwrap_or_default()
            .partial_cmp(&a.value_sol.unwrap_or_default())
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    logger::info(
        LogTag::Tools,
        &format!(
            "Burn tokens scan: {} tokens found (open={}, closed={}, value={}, zero={})",
            tokens.len(),
            categories.open_positions,
            categories.closed_positions,
            categories.has_value,
            categories.zero_liquidity
        ),
    );

    success_response(BurnTokensScanResponse {
        tokens,
        categories,
        total_rent_reclaimable_sol: total_rent_reclaimable,
    })
}

/// Burn selected tokens
pub async fn burn_selected_tokens(Json(request): Json<BurnTokensRequest>) -> Response {
    const ATA_RENT_SOL: f64 = 0.00203928;

    if request.mints.is_empty() {
        return error_response(
            axum::http::StatusCode::BAD_REQUEST,
            "NO_TOKENS",
            "No tokens selected for burning",
            None,
        );
    }

    // Get wallet address and keypair
    let wallet_address = match get_wallet_address() {
        Ok(addr) => addr,
        Err(e) => {
            return error_response(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "WALLET_ERROR",
                "Failed to get wallet address",
                Some(&e.to_string()),
            );
        }
    };

    let wallet_keypair = match crate::config::get_wallet_keypair() {
        Ok(kp) => kp,
        Err(e) => {
            return error_response(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "KEYPAIR_ERROR",
                "Failed to load wallet keypair",
                Some(&e.to_string()),
            );
        }
    };

    // Check for open positions - prevent burning these
    let open_positions = positions::get_open_positions().await;
    let open_position_mints: std::collections::HashSet<String> =
        open_positions.iter().map(|p| p.mint.clone()).collect();

    // Get all token accounts
    let all_accounts = match get_all_token_accounts(&wallet_address).await {
        Ok(accounts) => accounts,
        Err(e) => {
            return error_response(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "SCAN_ERROR",
                "Failed to get token accounts",
                Some(&e.to_string()),
            );
        }
    };

    // Build account map for quick lookup
    let account_map: HashMap<String, _> = all_accounts
        .iter()
        .map(|acc| (acc.mint.clone(), acc))
        .collect();

    let rpc_client = get_rpc_client();
    let mut results: Vec<BurnResult> = Vec::new();
    let mut successful = 0;
    let mut failed = 0;
    let mut sol_reclaimed = 0.0f64;

    for mint in &request.mints {
        // Skip SOL
        if mint == SOL_MINT {
            results.push(BurnResult {
                mint: mint.clone(),
                success: false,
                signature: None,
                error: Some("Cannot burn SOL".to_string()),
            });
            failed += 1;
            continue;
        }

        // Prevent burning open position tokens
        if open_position_mints.contains(mint) {
            results.push(BurnResult {
                mint: mint.clone(),
                success: false,
                signature: None,
                error: Some("Cannot burn tokens from open positions".to_string()),
            });
            failed += 1;
            continue;
        }

        // Find the account for this mint
        let account = match account_map.get(mint) {
            Some(acc) => acc,
            None => {
                results.push(BurnResult {
                    mint: mint.clone(),
                    success: false,
                    signature: None,
                    error: Some("Token account not found".to_string()),
                });
                failed += 1;
                continue;
            }
        };

        // Skip if balance is 0
        if account.balance == 0 {
            results.push(BurnResult {
                mint: mint.clone(),
                success: false,
                signature: None,
                error: Some("Token balance is already zero".to_string()),
            });
            failed += 1;
            continue;
        }

        // Parse addresses
        let wallet_pubkey = match Pubkey::from_str(&wallet_address) {
            Ok(pk) => pk,
            Err(e) => {
                results.push(BurnResult {
                    mint: mint.clone(),
                    success: false,
                    signature: None,
                    error: Some(format!("Invalid wallet address: {}", e)),
                });
                failed += 1;
                continue;
            }
        };

        let mint_pubkey = match Pubkey::from_str(mint) {
            Ok(pk) => pk,
            Err(e) => {
                results.push(BurnResult {
                    mint: mint.clone(),
                    success: false,
                    signature: None,
                    error: Some(format!("Invalid mint address: {}", e)),
                });
                failed += 1;
                continue;
            }
        };

        let ata_pubkey = match Pubkey::from_str(&account.account) {
            Ok(pk) => pk,
            Err(e) => {
                results.push(BurnResult {
                    mint: mint.clone(),
                    success: false,
                    signature: None,
                    error: Some(format!("Invalid ATA address: {}", e)),
                });
                failed += 1;
                continue;
            }
        };

        // Determine token program
        let token_program_id = if account.is_token_2022 {
            spl_token_2022::id()
        } else {
            spl_token::id()
        };

        // Create burn instruction
        let burn_instruction = match spl_instruction::burn(
            &token_program_id,
            &ata_pubkey,
            &mint_pubkey,
            &wallet_pubkey,
            &[&wallet_pubkey],
            account.balance,
        ) {
            Ok(ix) => ix,
            Err(e) => {
                results.push(BurnResult {
                    mint: mint.clone(),
                    success: false,
                    signature: None,
                    error: Some(format!("Failed to create burn instruction: {}", e)),
                });
                failed += 1;
                continue;
            }
        };

        // Get recent blockhash
        let recent_blockhash = match rpc_client.get_latest_blockhash().await {
            Ok(bh) => bh,
            Err(e) => {
                results.push(BurnResult {
                    mint: mint.clone(),
                    success: false,
                    signature: None,
                    error: Some(format!("Failed to get blockhash: {}", e)),
                });
                failed += 1;
                continue;
            }
        };

        // Create and sign transaction
        let transaction = Transaction::new_signed_with_payer(
            &[burn_instruction],
            Some(&wallet_pubkey),
            &[&wallet_keypair],
            recent_blockhash,
        );

        // Send and confirm transaction
        match rpc_client
            .send_and_confirm_signed_transaction(&transaction)
            .await
        {
            Ok(signature) => {
                logger::info(
                    LogTag::Tools,
                    &format!(
                        "Burned {} tokens of {}. TX: {}",
                        account.balance, mint, signature
                    ),
                );
                results.push(BurnResult {
                    mint: mint.clone(),
                    success: true,
                    signature: Some(signature.to_string()),
                    error: None,
                });
                successful += 1;
                sol_reclaimed += ATA_RENT_SOL; // Will be reclaimed when ATA is closed
            }
            Err(e) => {
                logger::error(
                    LogTag::Tools,
                    &format!("Failed to burn tokens for {}: {}", mint, e),
                );
                results.push(BurnResult {
                    mint: mint.clone(),
                    success: false,
                    signature: None,
                    error: Some(format!("Transaction failed: {}", e)),
                });
                failed += 1;
            }
        }

        // Small delay between burns to avoid rate limiting
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
    }

    logger::info(
        LogTag::Tools,
        &format!(
            "Burn tokens complete: {}/{} successful, ~{:.6} SOL to reclaim via ATA cleanup",
            successful,
            request.mints.len(),
            sol_reclaimed
        ),
    );

    success_response(BurnTokensResponse {
        total: request.mints.len(),
        successful,
        failed,
        results,
        sol_reclaimed,
    })
}
