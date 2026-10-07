// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Burn tokens handlers.

use axum::{response::Response, Json};
use std::collections::HashMap;
use std::time::Duration;

use crate::actions::ActionFailure;
use crate::chains::solana::assets::ata::get_all_token_accounts;
use crate::chains::solana::assets::burn_configured_wallet_token;
use crate::chains::solana::constants::ATA_RENT_COST_SOL;
use crate::i18n::{ids, UiArg, UiText};
use crate::logger::{self, LogTag};
use crate::pools;
use crate::positions;
use crate::utils::get_wallet_address;
use crate::webserver::api_error::{ApiError, ApiErrorCode};
use crate::webserver::utils::success_response;
use axum::response::IntoResponse as _;

use super::types::*;

// =============================================================================
// Burn Tokens Handlers
// =============================================================================

/// Warning shown for a token with market value: the value keeps the six
/// decimals of the scan and is passed as text so it is never regrouped.
fn worth_warning(value_sol: f64) -> UiText {
    UiText::new(ids::TOOLS_BURN_WARNING_WORTH).arg("amount", UiArg::Text(format!("{value_sol:.6}")))
}

/// Scan wallet for tokens that can be burned, with categorization
pub async fn scan_burnable_tokens() -> Response {
    // Get wallet address
    let wallet_address = match get_wallet_address() {
        Ok(addr) => addr,
        Err(e) => {
            logger::error(LogTag::Tools, &format!("Failed to get wallet address: {e}"));
            return ApiError::new(
                ApiErrorCode::Internal,
                ids::ERRORS_TOOLS_WALLET_ADDRESS_FAILED,
            )
            .details(e.to_string())
            .into_response();
        }
    };

    // Get all token accounts
    let all_accounts = match get_all_token_accounts(&wallet_address).await {
        Ok(accounts) => accounts,
        Err(e) => {
            logger::error(LogTag::Tools, &format!("Failed to get token accounts: {e}"));
            return ApiError::new(
                ApiErrorCode::Internal,
                ids::ERRORS_TOOLS_TOKEN_ACCOUNTS_SCAN_FAILED,
            )
            .details(e.to_string())
            .into_response();
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

    if let Some(db) = crate::tokens::database::database(crate::chains::active_chain()) {
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
        if crate::chains::adapter().is_native_asset(&account.mint) || account.is_nft {
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
        let price_result = pools::get_pool_price(crate::chains::active_chain(), &account.mint);
        let price_sol = price_result.as_ref().map(|p| p.price_native);
        let has_liquidity = price_result
            .as_ref()
            .map(|p| p.native_reserves > 0.0)
            .unwrap_or_default();

        // Calculate UI amount and value
        let ui_amount = account.balance as f64 / 10f64.powi(account.decimals as i32);
        let value_sol = price_sol.map(|p| p * ui_amount);

        // Determine category
        let (category, can_burn, burn_warning) = if open_position_mints.contains(&account.mint) {
            categories.open_positions += 1;
            (
                TokenCategory::OpenPosition,
                false,
                Some(UiText::new(ids::TOOLS_BURN_WARNING_OPEN_POSITION)),
            )
        } else if closed_position_mints.contains(&account.mint) {
            categories.closed_positions += 1;
            (
                TokenCategory::ClosedPosition,
                true,
                Some(UiText::new(ids::TOOLS_BURN_WARNING_CLOSED_POSITION)),
            )
        } else if has_liquidity && value_sol.is_some_and(|v| v > 0.0001) {
            categories.has_value += 1;
            (
                TokenCategory::HasValue,
                true,
                Some(worth_warning(value_sol.unwrap_or_default())),
            )
        } else {
            categories.zero_liquidity += 1;
            (TokenCategory::ZeroLiquidity, true, None)
        };

        // Only count rent reclaimable for tokens we can burn
        if can_burn {
            total_rent_reclaimable += ATA_RENT_COST_SOL;
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
            price_sol,
            value_sol,
            has_liquidity,
            can_burn,
            burn_warning,
            rent_reclaimable_native: if can_burn { ATA_RENT_COST_SOL } else { 0.0 },
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
        total_rent_reclaimable_native: total_rent_reclaimable,
    })
}

/// Burn selected tokens
pub async fn burn_selected_tokens(Json(request): Json<BurnTokensRequest>) -> Response {
    if request.mints.is_empty() {
        return ApiError::new(ApiErrorCode::InvalidInput, ids::ERRORS_TOOLS_NO_TOKENS)
            .into_response();
    }

    // Get wallet address
    let wallet_address = match get_wallet_address() {
        Ok(addr) => addr,
        Err(e) => {
            return ApiError::new(
                ApiErrorCode::Internal,
                ids::ERRORS_TOOLS_WALLET_ADDRESS_FAILED,
            )
            .details(e.to_string())
            .into_response();
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
            return ApiError::new(
                ApiErrorCode::Internal,
                ids::ERRORS_TOOLS_TOKEN_ACCOUNTS_GET_FAILED,
            )
            .details(e.to_string())
            .into_response();
        }
    };

    // The burns run on their own task: a dropped request must not stop the
    // loop between two sends, nor lose the signature of one already sent.
    let mints = request.mints;
    let burns = tokio::spawn(async move {
        // Build account map for quick lookup
        let account_map: HashMap<String, _> = all_accounts
            .iter()
            .map(|acc| (acc.mint.clone(), acc))
            .collect();

        let mut results: Vec<BurnResult> = Vec::new();
        let mut successful = 0;
        let mut failed = 0;
        let mut native_reclaimed = 0.0f64;

        for mint in &mints {
            // Skip SOL
            if crate::chains::adapter().is_native_asset(mint) {
                results.push(BurnResult {
                    mint: mint.clone(),
                    success: false,
                    signature: None,
                    error: Some(ActionFailure::new(ids::TOOLS_BURN_FAILURE_NATIVE_ASSET)),
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
                    error: Some(ActionFailure::new(ids::TOOLS_BURN_FAILURE_OPEN_POSITION)),
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
                        error: Some(ActionFailure::new(
                            ids::TOOLS_BURN_FAILURE_ACCOUNT_NOT_FOUND,
                        )),
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
                    error: Some(ActionFailure::new(ids::TOOLS_BURN_FAILURE_ZERO_BALANCE)),
                });
                failed += 1;
                continue;
            }

            // Build, sign and submit the burn — resolving and using the
            // configured wallet's keypair happens inside crate::chains::solana;
            // this handler never sees it.
            match burn_configured_wallet_token(
                &wallet_address,
                &account.account,
                mint,
                account.balance,
                account.is_token_2022,
            )
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
                        signature: Some(signature),
                        error: None,
                    });
                    successful += 1;
                    native_reclaimed += ATA_RENT_COST_SOL; // Will be reclaimed when ATA is closed
                }
                Err(e) => {
                    logger::error(
                        LogTag::Tools,
                        &format!("Failed to burn tokens for {mint}: {e}"),
                    );
                    results.push(failed_burn(mint, &e));
                    failed += 1;
                }
            }

            // Small delay between burns to avoid rate limiting
            tokio::time::sleep(Duration::from_millis(200)).await;
        }

        logger::info(
            LogTag::Tools,
            &format!(
                "Burn tokens complete: {}/{} successful, ~{:.6} SOL to reclaim via ATA cleanup",
                successful,
                mints.len(),
                native_reclaimed
            ),
        );

        BurnTokensResponse {
            total: mints.len(),
            successful,
            failed,
            results,
            native_reclaimed,
        }
    });
    match burns.await {
        Ok(response) => success_response(response),
        Err(e) => {
            logger::error(LogTag::Tools, &format!("Burn task failed: {e}"));
            ApiError::new(ApiErrorCode::Internal, ids::ERRORS_TOOLS_BURN_FAILED)
                .details(e.to_string())
                .into_response()
        }
    }
}

/// A burn that failed after its transaction was sent keeps that transaction's
/// signature: a burn whose confirmation timed out may still land, and a
/// reverted or expired one is proven on chain by it.
fn failed_burn(mint: &str, error: &crate::chains::solana::Error) -> BurnResult {
    BurnResult {
        mint: mint.to_owned(),
        success: false,
        signature: error
            .classify()
            .map(|failure| failure.reference().to_owned()),
        error: Some(ActionFailure::with_details(
            ids::TOOLS_BURN_FAILURE_TRANSACTION,
            error.to_string(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn english(id: crate::i18n::MessageId, details: &str) -> String {
        ActionFailure::with_details(id, details).log_text()
    }

    #[test]
    fn burn_failures_and_warnings_render_their_english() {
        assert_eq!(
            english(ids::TOOLS_BURN_FAILURE_NATIVE_ASSET, ""),
            "Cannot burn SOL"
        );
        assert_eq!(
            english(ids::TOOLS_BURN_FAILURE_OPEN_POSITION, ""),
            "Cannot burn tokens from open positions"
        );
        assert_eq!(
            english(ids::TOOLS_BURN_FAILURE_ACCOUNT_NOT_FOUND, ""),
            "Token account not found"
        );
        assert_eq!(
            english(ids::TOOLS_BURN_FAILURE_ZERO_BALANCE, ""),
            "Token balance is already zero"
        );
        assert_eq!(
            english(ids::TOOLS_BURN_FAILURE_TRANSACTION, "rpc timeout"),
            "Transaction failed: rpc timeout"
        );
        assert_eq!(
            UiText::new(ids::TOOLS_BURN_WARNING_OPEN_POSITION).render_source_plain(),
            "Cannot burn tokens from open positions"
        );
        assert_eq!(
            UiText::new(ids::TOOLS_BURN_WARNING_CLOSED_POSITION).render_source_plain(),
            "Leftover from closed position"
        );
        assert_eq!(
            worth_warning(0.00123).render_source_plain(),
            "Worth ~0.001230 SOL"
        );
    }

    /// Every failure of a sent burn names its transaction; only a burn that
    /// provably never reached the chain, or never got that far, has none.
    #[test]
    fn a_failed_burn_keeps_the_signature_of_its_sent_transaction() {
        use crate::chains::solana::Error;
        use crate::chains::ExecutionFailure;

        let sent = [
            ExecutionFailure::ConfirmationTimeout {
                reference: "SIG".to_owned(),
                waited_ms: 60_000,
            },
            ExecutionFailure::Reverted {
                reference: "SIG".to_owned(),
                detail: "custom program error".to_owned(),
            },
            ExecutionFailure::Expired {
                reference: "SIG".to_owned(),
                last_valid_block_height: 10,
                current_block_height: 20,
            },
            ExecutionFailure::NotFound {
                reference: "SIG".to_owned(),
            },
            ExecutionFailure::IndexingDelay {
                reference: "SIG".to_owned(),
            },
        ];
        for failure in sent {
            let item = failed_burn("Mint1111", &Error::Execution(failure.clone()));
            assert_eq!(item.signature.as_deref(), Some("SIG"), "{failure:?}");
            assert!(!item.success);
        }

        let never_sent = [
            Error::NotSent(crate::swaps::NotSubmittedReason::SimulationFailed {
                detail: "insufficient funds".to_owned(),
            }),
            Error::Rpc {
                operation: "get_latest_blockhash",
                detail: "unreachable".to_owned(),
            },
        ];
        for error in never_sent {
            assert_eq!(failed_burn("Mint1111", &error).signature, None, "{error:?}");
        }
    }

    #[test]
    fn failed_burn_item_serializes_text_and_details() {
        let item = BurnResult {
            mint: "Mint1111".to_owned(),
            success: false,
            signature: None,
            error: Some(ActionFailure::with_details(
                ids::TOOLS_BURN_FAILURE_TRANSACTION,
                "rpc timeout",
            )),
        };
        let json = serde_json::to_value(&item).unwrap();
        assert_eq!(json["mint"], "Mint1111");
        assert_eq!(json["success"], false);
        assert_eq!(json["signature"], serde_json::Value::Null);
        assert_eq!(
            json["error"]["text"]["id"],
            "tools-burn-failure-transaction"
        );
        assert_eq!(json["error"]["details"], "rpc timeout");
    }
}
