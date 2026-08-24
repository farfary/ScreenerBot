//! Multi-Sell Operation
//!
//! Coordinate sell orders across multiple wallets with optional consolidation.

use std::collections::{HashMap, HashSet};
use std::sync::atomic::Ordering;

use tokio::time::{sleep, Duration};
use uuid::Uuid;

use crate::chains::adapter;
use crate::chains::solana::rpc::{get_rpc_client, RpcClientMethods};
use crate::logger::{self, LogTag};
use crate::tools::swap_executor::tool_sell;
use crate::tools::Error;
use crate::wallets::{self, Wallet, WalletRole};

use crate::chains::solana::assets::transfer::{close_ata_for_wallet, transfer_sol_from_main};

use super::transfer::collect_sol;
use super::types::{MultiSellConfig, SessionResult, WalletOpResult, WalletPlan};

/// Execute a multi-sell operation across multiple wallets
///
/// # Arguments
/// * `config` - Multi-sell configuration
///
/// # Returns
/// Session result with all operation outcomes
pub async fn execute_multi_sell(config: MultiSellConfig) -> Result<SessionResult, Error> {
    // Validate configuration
    config.validate()?;

    let session_id = Uuid::new_v4().to_string();
    let mut result = SessionResult::new(session_id.clone());

    logger::info(
        LogTag::Tools,
        &format!(
            "Starting multi-sell session {} for token {}, {}% sell",
            &session_id[..8],
            &config.token_mint[..8],
            config.sell_percentage
        ),
    );

    // Load wallets with token balance
    let wallets = load_wallets_for_sell(&config).await?;
    if wallets.is_empty() {
        return Err(Error::InvalidConfig {
            detail: "no wallets found with token balance".to_owned(),
        });
    }

    // Create sell plans
    let plans = create_sell_plans(&config, &wallets).await?;
    if plans.is_empty() {
        return Err(Error::InvalidConfig {
            detail: "no valid sell plans could be created".to_owned(),
        });
    }

    logger::info(
        LogTag::Tools,
        &format!(
            "Multi-sell plans: {} wallets with token balance",
            plans.len()
        ),
    );

    // Top-up wallets that need SOL for fees
    if config.auto_topup {
        let topup_needed: Vec<_> = plans
            .iter()
            .filter(|p| p.sol_balance < config.min_sol_for_fee)
            .collect();

        if !topup_needed.is_empty() {
            for plan in topup_needed {
                let topup_amount = config.min_sol_for_fee - plan.sol_balance + 0.001;
                if let Err(e) = transfer_sol_from_main(&plan.wallet_address, topup_amount).await {
                    logger::warning(
                        LogTag::Tools,
                        &format!(
                            "Failed to top-up wallet {}: {}",
                            &plan.wallet_address[..8],
                            e
                        ),
                    );
                }
            }

            // Small delay after top-ups
            sleep(Duration::from_millis(500)).await;
        }
    }

    // Execute sells - track successful sells for ATA closure
    let wallet_map: HashMap<String, &Wallet> =
        wallets.iter().map(|w| (w.address.clone(), w)).collect();

    // Track wallets that successfully sold 100% (for ATA closure)
    let mut successful_full_sells: HashSet<String> = HashSet::new();

    for plan in &plans {
        // Check abort flag before each operation
        if let Some(ref abort_flag) = config.abort_flag {
            if abort_flag.load(Ordering::SeqCst) {
                logger::info(
                    LogTag::Tools,
                    &format!("Multi-sell session {} aborted by user", &session_id[..8]),
                );
                result.error = Some("Operation aborted by user".to_owned());
                result.finalize();
                return Ok(result);
            }
        }

        if let Some(wallet) = wallet_map.get(&plan.wallet_address) {
            if let Some(token_balance) = plan.token_balance {
                let sell_amount = (token_balance * config.sell_percentage / 100.0) as u64;

                let op_result = execute_single_sell(
                    wallet,
                    &config.token_mint,
                    sell_amount,
                    config.slippage_bps,
                    config.router.as_deref(),
                )
                .await;

                // Track successful 100% sells for ATA closure
                if op_result.success && config.sell_percentage >= 99.9 {
                    successful_full_sells.insert(plan.wallet_address.clone());
                }

                result.add_operation(op_result);

                // Apply delay between operations
                let delay_ms = config.delay.get_delay_ms();
                if delay_ms > 0 {
                    sleep(Duration::from_millis(delay_ms)).await;
                }
            }
        }
    }

    // Close ATAs only for wallets that successfully sold 100%
    if config.close_atas_after && !successful_full_sells.is_empty() {
        let rpc_client = get_rpc_client();

        for wallet_address in &successful_full_sells {
            if let Some(wallet) = wallet_map.get(wallet_address) {
                // Verify balance is actually zero before closing
                let remaining_balance = rpc_client
                    .get_token_balance(&wallet.address, &config.token_mint)
                    .await
                    .unwrap_or_default();

                if remaining_balance == 0 {
                    if let Err(e) = close_ata_for_wallet(wallet.id, &config.token_mint, false).await
                    {
                        logger::debug(
                            LogTag::Tools,
                            &format!("Failed to close ATA for {}: {}", &wallet_address[..8], e),
                        );
                    } else {
                        logger::debug(
                            LogTag::Tools,
                            &format!("Closed ATA for wallet {}", &wallet_address[..8]),
                        );
                    }
                } else {
                    logger::debug(
                        LogTag::Tools,
                        &format!(
                            "Skipping ATA close for {} - remaining balance: {}",
                            &wallet_address[..8],
                            remaining_balance
                        ),
                    );
                }
            }
        }
    }

    // Consolidate SOL if configured
    if config.consolidate_after {
        let main_wallet =
            wallets::get_main_wallet()
                .await?
                .ok_or_else(|| Error::MainWalletUnavailable {
                    detail: "no main wallet configured".to_owned(),
                })?;

        let wallets_to_consolidate: Vec<Wallet> = wallets
            .into_iter()
            .filter(|w| w.role == WalletRole::Secondary)
            .collect();

        let collect_results =
            collect_sol(wallets_to_consolidate, &main_wallet.address, false).await;

        result.total_sol_recovered = collect_results
            .iter()
            .filter(|r| r.success)
            .filter_map(|r| r.amount_sol)
            .sum();
    }

    result.finalize();

    logger::info(
        LogTag::Tools,
        &format!(
            "Multi-sell session {} complete: {}/{} successful, {:.6} SOL recovered",
            &session_id[..8],
            result.successful_ops,
            result.total_wallets,
            result.total_sol_recovered
        ),
    );

    Ok(result)
}

/// Load wallets for multi-sell operation
async fn load_wallets_for_sell(config: &MultiSellConfig) -> Result<Vec<Wallet>, Error> {
    let all_wallets = wallets::list_active_wallets().await?;
    let rpc_client = get_rpc_client();

    let mut wallets_with_balance = Vec::new();

    for wallet in all_wallets {
        // Skip main wallet unless explicitly included
        if wallet.role == WalletRole::Main {
            continue;
        }

        // Filter by wallet IDs if specified
        if let Some(ref ids) = config.wallet_ids {
            if !ids.contains(&wallet.id) {
                continue;
            }
        }

        // Check token balance
        let token_balance = rpc_client
            .get_token_balance(&wallet.address, &config.token_mint)
            .await
            .unwrap_or_default();

        if token_balance > 0 {
            wallets_with_balance.push(wallet);
        }
    }

    Ok(wallets_with_balance)
}

/// Create sell plans for each wallet
async fn create_sell_plans(
    config: &MultiSellConfig,
    wallets: &[Wallet],
) -> Result<Vec<WalletPlan>, Error> {
    let rpc_client = get_rpc_client();
    let mut plans = Vec::new();

    for wallet in wallets {
        let sol_balance = rpc_client
            .get_sol_balance(&wallet.address)
            .await
            .unwrap_or_default();

        let token_balance = rpc_client
            .get_token_balance(&wallet.address, &config.token_mint)
            .await
            .unwrap_or_default();

        if token_balance == 0 {
            continue;
        }

        plans.push(WalletPlan {
            wallet_id: wallet.id,
            wallet_address: wallet.address.clone(),
            wallet_name: wallet.name.clone(),
            sol_balance,
            token_balance: Some(token_balance as f64),
            planned_amount_sol: 0.0, // Will be determined by swap output
            needs_funding: sol_balance < config.min_sol_for_fee,
            funding_amount: if sol_balance < config.min_sol_for_fee {
                config.min_sol_for_fee - sol_balance + 0.001
            } else {
                0.0
            },
        });
    }

    Ok(plans)
}

/// Execute a single sell operation
async fn execute_single_sell(
    wallet: &Wallet,
    token_mint: &str,
    amount: u64,
    slippage_bps: u64,
    _router: Option<&str>,
) -> WalletOpResult {
    let wallet_id = wallet.id;
    let wallet_address = wallet.address.clone();

    logger::debug(
        LogTag::Tools,
        &format!(
            "Executing sell: wallet={}, token={}, amount={}",
            &wallet_address[..8],
            &token_mint[..8],
            amount
        ),
    );

    // Convert bps to percentage
    let slippage_pct = slippage_bps as f64 / 100.0;

    match tool_sell(wallet, token_mint, amount, Some(slippage_pct)).await {
        Ok(swap_result) => {
            // output_amount is in lamports, convert to SOL
            let sol_received = adapter().raw_to_native(swap_result.output_amount);
            WalletOpResult::success(
                wallet_id,
                wallet_address,
                swap_result.signature,
                sol_received,
                Some(amount as f64),
            )
        }
        Err(e) => WalletOpResult::failure(wallet_id, wallet_address, e.to_string()),
    }
}
