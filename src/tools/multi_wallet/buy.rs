//! Multi-Buy Operation
//!
//! Distribute buy orders across multiple wallets for stealth accumulation.

use std::sync::atomic::Ordering;

use tokio::time::{sleep, Duration};
use uuid::Uuid;

use crate::logger::{self, LogTag};
use crate::rpc::{get_rpc_client, RpcClientMethods};
use crate::tools::swap_executor::tool_buy;
use crate::wallets::{self, WalletRole, WalletWithKey};

use super::transfer::fund_wallets;
use super::types::{MultiBuyConfig, SessionResult, SessionStatus, WalletOpResult, WalletPlan};

/// Execute a multi-buy operation across multiple wallets
///
/// # Arguments
/// * `config` - Multi-buy configuration
///
/// # Returns
/// Session result with all operation outcomes
pub async fn execute_multi_buy(config: MultiBuyConfig) -> Result<SessionResult, String> {
    // Validate configuration
    config.validate()?;

    let session_id = Uuid::new_v4().to_string();
    let mut result = SessionResult::new(session_id.clone());

    logger::info(
        LogTag::Tools,
        &format!(
            "Starting multi-buy session {} for token {}, {} wallets",
            &session_id[..8],
            &config.token_mint[..8],
            config.wallet_count
        ),
    );

    // Load and prepare wallets
    let wallets = load_wallets_for_buy(&config).await?;
    if wallets.is_empty() {
        return Err("No wallets available for multi-buy".to_string());
    }

    // Create execution plans
    let plans = create_buy_plans(&config, &wallets).await?;
    if plans.is_empty() {
        return Err("No valid buy plans could be created".to_string());
    }

    logger::info(
        LogTag::Tools,
        &format!(
            "Multi-buy plans: {} wallets, {:.6} SOL total",
            plans.len(),
            plans.iter().map(|p| p.planned_amount_sol).sum::<f64>()
        ),
    );

    // Fund wallets that need funding
    let funding_needed: Vec<(String, f64)> = plans
        .iter()
        .filter(|p| p.needs_funding)
        .map(|p| (p.wallet_address.clone(), p.funding_amount))
        .collect();

    if !funding_needed.is_empty() {
        let main_keypair = wallets::get_main_keypair()
            .await
            .map_err(|e| format!("Failed to get main wallet keypair: {}", e))?;

        logger::info(
            LogTag::Tools,
            &format!("Funding {} wallets before buy", funding_needed.len()),
        );

        let funding_results = fund_wallets(&main_keypair, funding_needed, 3).await;
        let failed_funding: Vec<_> = funding_results.iter().filter(|r| !r.success).collect();

        if !failed_funding.is_empty() {
            logger::warning(
                LogTag::Tools,
                &format!("{} wallets failed to fund", failed_funding.len()),
            );
        }

        // Small delay after funding
        sleep(Duration::from_millis(500)).await;
    }

    // Execute buys
    let wallet_map: std::collections::HashMap<String, &WalletWithKey> = wallets
        .iter()
        .map(|w| (w.wallet.address.clone(), w))
        .collect();

    for plan in plans {
        // Check abort flag before each operation
        if let Some(ref abort_flag) = config.abort_flag {
            if abort_flag.load(Ordering::SeqCst) {
                logger::info(
                    LogTag::Tools,
                    &format!("Multi-buy session {} aborted by user", &session_id[..8]),
                );
                result.error = Some("Operation aborted by user".to_string());
                result.finalize();
                return Ok(result);
            }
        }

        if let Some(wallet) = wallet_map.get(&plan.wallet_address) {
            let op_result = execute_single_buy(
                wallet,
                &config.token_mint,
                plan.planned_amount_sol,
                config.slippage_bps,
                config.router.as_deref(),
            )
            .await;

            result.add_operation(op_result);

            // Apply delay between operations
            let delay_ms = config.delay.get_delay_ms();
            if delay_ms > 0 {
                sleep(Duration::from_millis(delay_ms)).await;
            }
        }
    }

    result.finalize();

    logger::info(
        LogTag::Tools,
        &format!(
            "Multi-buy session {} complete: {}/{} successful, {:.6} SOL spent",
            &session_id[..8],
            result.successful_ops,
            result.total_wallets,
            result.total_sol_spent
        ),
    );

    Ok(result)
}

/// Load wallets for multi-buy operation
async fn load_wallets_for_buy(config: &MultiBuyConfig) -> Result<Vec<WalletWithKey>, String> {
    let all_wallets = wallets::get_wallets_with_keys().await?;

    // Filter to secondary wallets only
    let secondary_wallets: Vec<WalletWithKey> = all_wallets
        .into_iter()
        .filter(|w| w.wallet.role == WalletRole::Secondary && w.wallet.is_active)
        .take(config.wallet_count)
        .collect();

    if secondary_wallets.is_empty() {
        return Err("No secondary wallets available. Create sub-wallets first.".to_string());
    }

    Ok(secondary_wallets)
}

/// Create buy plans for each wallet
async fn create_buy_plans(
    config: &MultiBuyConfig,
    wallets: &[WalletWithKey],
) -> Result<Vec<WalletPlan>, String> {
    let rpc_client = get_rpc_client();
    let mut plans = Vec::new();

    for wallet in wallets {
        let balance = rpc_client
            .get_sol_balance(&wallet.wallet.address)
            .await
            .unwrap_or(0.0);

        // Calculate buy amount (random within range)
        let buy_amount = if config.min_amount_sol == config.max_amount_sol {
            config.min_amount_sol
        } else {
            use rand::Rng;
            let mut rng = rand::thread_rng();
            rng.gen_range(config.min_amount_sol..=config.max_amount_sol)
        };

        // Calculate required balance
        let required = buy_amount + config.sol_buffer;
        let needs_funding = balance < required;
        let funding_amount = if needs_funding {
            required - balance + 0.001 // Extra for safety
        } else {
            0.0
        };

        plans.push(WalletPlan {
            wallet_id: wallet.wallet.id,
            wallet_address: wallet.wallet.address.clone(),
            wallet_name: wallet.wallet.name.clone(),
            sol_balance: balance,
            token_balance: None,
            planned_amount_sol: buy_amount,
            needs_funding,
            funding_amount,
        });
    }

    // Apply total limit if set
    if let Some(limit) = config.total_sol_limit {
        let mut total = 0.0;
        plans.retain(|p| {
            if total + p.planned_amount_sol <= limit {
                total += p.planned_amount_sol;
                true
            } else {
                false
            }
        });
    }

    Ok(plans)
}

/// Execute a single buy operation
async fn execute_single_buy(
    wallet: &WalletWithKey,
    token_mint: &str,
    amount_sol: f64,
    slippage_bps: u64,
    _router: Option<&str>,
) -> WalletOpResult {
    let wallet_id = wallet.wallet.id;
    let wallet_address = wallet.wallet.address.clone();

    logger::debug(
        LogTag::Tools,
        &format!(
            "Executing buy: wallet={}, token={}, amount={:.6} SOL",
            &wallet_address[..8],
            &token_mint[..8],
            amount_sol
        ),
    );

    // Convert bps to percentage
    let slippage_pct = slippage_bps as f64 / 100.0;

    match tool_buy(wallet, token_mint, amount_sol, Some(slippage_pct)).await {
        Ok(swap_result) => WalletOpResult::success(
            wallet_id,
            wallet_address,
            swap_result.signature,
            amount_sol,
            Some(swap_result.output_amount as f64),
        ),
        Err(e) => WalletOpResult::failure(wallet_id, wallet_address, e),
    }
}
