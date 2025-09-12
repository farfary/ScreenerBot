use crate::{
    tokens::{get_token_from_db, PriceResult},
    pools::get_pool_price,
    swaps::{execute_best_swap, get_best_quote, config::SOL_MINT},
    rpc::{get_rpc_client, sol_to_lamports},
    utils::{get_wallet_address, get_token_balance},
    arguments::{is_dry_run_enabled, is_debug_positions_enabled},
    logger::{log, LogTag},
    trader::TRADE_SIZE_SOL,
    positions_db::save_position,
    positions_types::Position,
};
use super::{
    state::{acquire_position_lock, add_position, add_signature_to_index},
    queue::{enqueue_verification, VerificationItem, VerificationKind},
    transitions::PositionTransition,
    apply::apply_transition,
};
use chrono::Utc;

const SOLANA_BLOCKHASH_VALIDITY_SLOTS: u64 = 150;

/// Open a new position
pub async fn open_position_direct(token_mint: &str) -> Result<String, String> {
    let token = get_token_from_db(token_mint).await
        .ok_or_else(|| format!("Token not found: {}", token_mint))?;

    let price_info = get_pool_price(token_mint)
        .ok_or_else(|| format!("No price data for token: {}", token_mint))?;

    let entry_price = match price_info.price_sol {
        price if price > 0.0 && price.is_finite() => price,
        _ => return Err(format!("Invalid price for token: {}", token_mint)),
    };

    let _lock = acquire_position_lock(&token.mint).await;

    if is_dry_run_enabled() {
        log(
            LogTag::Positions,
            "DRY-RUN",
            &format!("🚫 DRY-RUN: Would open position for {} at {:.6} SOL", token.symbol, entry_price)
        );
        return Err("DRY-RUN: Position would be opened".to_string());
    }

    // Execute swap
    let wallet_address = get_wallet_address()
        .map_err(|e| format!("Failed to get wallet address: {}", e))?;

    let quote = get_best_quote(
        SOL_MINT,
        &token.mint,
        sol_to_lamports(TRADE_SIZE_SOL),
        &wallet_address,
        2.0, // 2% slippage
    ).await.map_err(|e| format!("Quote failed: {}", e))?;

    let swap_result = execute_best_swap(
        &token,
        SOL_MINT,
        &token.mint,
        sol_to_lamports(TRADE_SIZE_SOL),
        quote,
    ).await.map_err(|e| format!("Swap failed: {}", e))?;

    let transaction_signature = swap_result.transaction_signature
        .ok_or("No transaction signature")?;

    // Create position
    let position = Position {
        id: None,
        mint: token.mint.clone(),
        symbol: token.symbol.clone(),
        name: token.name.clone(),
        entry_price,
        entry_time: Utc::now(),
        exit_price: None,
        exit_time: None,
        position_type: "buy".to_string(),
        entry_size_sol: TRADE_SIZE_SOL,
        total_size_sol: TRADE_SIZE_SOL,
        price_highest: entry_price,
        price_lowest: entry_price,
        entry_transaction_signature: Some(transaction_signature.clone()),
        exit_transaction_signature: None,
        token_amount: None,
        effective_entry_price: None,
        effective_exit_price: None,
        sol_received: None,
        profit_target_min: Some(5.0),
        profit_target_max: Some(20.0),
        liquidity_tier: Some("UNKNOWN".to_string()),
        transaction_entry_verified: false,
        transaction_exit_verified: false,
        entry_fee_lamports: None,
        exit_fee_lamports: None,
        current_price: Some(entry_price),
        current_price_updated: Some(Utc::now()),
        phantom_remove: false,
        phantom_confirmations: 0,
        phantom_first_seen: None,
        synthetic_exit: false,
        closed_reason: None,
    };

    // Save to database and get ID
    let position_id = save_position(&position).await
        .map_err(|e| format!("Failed to save position: {}", e))?;

    let mut position_with_id = position;
    position_with_id.id = Some(position_id);

    // Add to state
    add_position(position_with_id).await;
    add_signature_to_index(&transaction_signature, &token.mint).await;

    // Get block height for expiration
    let expiry_height = get_rpc_client().get_block_height().await
        .map(|h| h + SOLANA_BLOCKHASH_VALIDITY_SLOTS)
        .ok();

    // Enqueue for verification
    let verification_item = VerificationItem::new(
        transaction_signature.clone(),
        token.mint.clone(),
        Some(position_id),
        VerificationKind::Entry,
        expiry_height,
    );

    enqueue_verification(verification_item).await;

    log(
        LogTag::Positions,
        "SUCCESS",
        &format!("✅ Position opened: {} (ID: {}) | TX: {}", token.symbol, position_id, transaction_signature)
    );

    Ok(transaction_signature)
}

/// Close an existing position
pub async fn close_position_direct(token_mint: &str, exit_reason: String) -> Result<String, String> {
    let token = get_token_from_db(token_mint).await
        .ok_or_else(|| format!("Token not found: {}", token_mint))?;

    let price_info = get_pool_price(token_mint)
        .ok_or_else(|| format!("No price data for token: {}", token_mint))?;

    let exit_price = match price_info.price_sol {
        price if price > 0.0 && price.is_finite() => price,
        _ => return Err(format!("Invalid exit price for token: {}", token_mint)),
    };

    let _lock = acquire_position_lock(token_mint).await;

    if is_dry_run_enabled() {
        log(
            LogTag::Positions,
            "DRY-RUN",
            &format!("🚫 DRY-RUN: Would close position for {}", token.symbol)
        );
        return Err("DRY-RUN: Position would be closed".to_string());
    }

    // Get token balance
    let wallet_address = get_wallet_address()
        .map_err(|e| format!("Failed to get wallet address: {}", e))?;

    let token_balance = get_token_balance(&wallet_address, token_mint).await
        .map_err(|e| format!("Failed to get token balance: {}", e))?;

    if token_balance == 0 {
        return Err("No tokens to sell".to_string());
    }

    // Execute swap
    let quote = get_best_quote(
        token_mint,
        SOL_MINT,
        token_balance,
        &wallet_address,
        5.0, // 5% slippage for exits
    ).await.map_err(|e| format!("Quote failed: {}", e))?;

    let swap_result = execute_best_swap(
        &token,
        token_mint,
        SOL_MINT,
        token_balance,
        quote,
    ).await.map_err(|e| format!("Swap failed: {}", e))?;

    let transaction_signature = swap_result.transaction_signature
        .ok_or("No transaction signature")?;

    // Update position with exit signature
    super::state::update_position_state(token_mint, |pos| {
        pos.exit_transaction_signature = Some(transaction_signature.clone());
        pos.closed_reason = Some(format!("{}_pending_verification", exit_reason));
    }).await;

    add_signature_to_index(&transaction_signature, token_mint).await;

    // Get position ID
    let position_id = super::state::get_position_by_mint(token_mint).await
        .and_then(|p| p.id)
        .unwrap_or(0);

    // Get block height for expiration
    let expiry_height = get_rpc_client().get_block_height().await
        .map(|h| h + SOLANA_BLOCKHASH_VALIDITY_SLOTS)
        .ok();

    // Enqueue for verification
    let verification_item = VerificationItem::new(
        transaction_signature.clone(),
        token_mint.to_string(),
        Some(position_id),
        VerificationKind::Exit,
        expiry_height,
    );

    enqueue_verification(verification_item).await;

    log(
        LogTag::Positions,
        "SUCCESS",
        &format!("✅ Position closing: {} | TX: {} | Reason: {}", token.symbol, transaction_signature, exit_reason)
    );

    Ok(transaction_signature)
}