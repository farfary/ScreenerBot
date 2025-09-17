use crate::{
    tokens::{ get_token_from_db, PriceResult },
    pools::get_pool_price,
    swaps::{ execute_best_swap, get_best_quote, get_best_quote_for_opening, config::SOL_MINT },
    rpc::{ get_rpc_client, sol_to_lamports },
    utils::{ get_wallet_address, get_token_balance, get_total_token_balance },
    arguments::{ is_dry_run_enabled, is_debug_positions_enabled },
    logger::{ log, LogTag },
    trader::{
        TRADE_SIZE_SOL,
        MAX_OPEN_POSITIONS,
        SLIPPAGE_QUOTE_DEFAULT_PCT,
        SLIPPAGE_EXIT_RETRY_STEPS_PCT,
    },
};
use super::{ db::save_position, types::Position };
use super::{
    state::{
        acquire_position_lock,
        acquire_global_position_permit,
        release_global_position_permit,
        add_position,
        add_signature_to_index,
        LAST_OPEN_TIME,
        POSITION_OPEN_COOLDOWN_SECS,
    },
    queue::{ enqueue_verification, VerificationItem, VerificationKind },
    transitions::PositionTransition,
    apply::apply_transition,
};
use chrono::Utc;

const SOLANA_BLOCKHASH_VALIDITY_SLOTS: u64 = 150;

/// Open a new position
pub async fn open_position_direct(token_mint: &str) -> Result<String, String> {
    let token = get_token_from_db(token_mint).await.ok_or_else(||
        format!("Token not found: {}", token_mint)
    )?;

    let price_info = get_pool_price(token_mint).ok_or_else(||
        format!("No price data for token: {}", token_mint)
    )?;

    let entry_price = match price_info.price_sol {
        price if price > 0.0 && price.is_finite() => price,
        _ => {
            return Err(format!("Invalid price for token: {}", token_mint));
        }
    };

    // CRITICAL: Acquire global position permit FIRST to enforce MAX_OPEN_POSITIONS atomically.
    // IMPORTANT: We will "forget" this permit ONLY after the position is successfully
    // created & added to in‑memory state so that the semaphore capacity remains reduced
    // for the lifetime of the open position. All terminal close paths MUST call
    // release_global_position_permit() (verified exit, synthetic exit, orphan removal).
    // Any early error returns (before we forget the permit) will automatically drop it
    // and thus NOT consume a slot.
    let mut _global_permit = acquire_global_position_permit().await?;

    // Acquire per-mint lock SECOND to serialize opens for same token
    let _lock = acquire_position_lock(&token.mint).await;

    // Re-check no existing open position for this mint (prevents duplicate concurrent entries)
    if super::state::is_open_position(&token.mint).await {
        return Err("Already have open position for this token".to_string());
    }

    // Note: No need to check MAX_OPEN_POSITIONS here anymore - the semaphore enforces it atomically

    // Lightweight global cooldown (prevents rapid duplicate openings across different tokens)
    {
        let last_open_opt = LAST_OPEN_TIME.read().await.clone();
        if let Some(last_open) = last_open_opt {
            let elapsed = Utc::now().signed_duration_since(last_open).num_seconds();
            if elapsed < POSITION_OPEN_COOLDOWN_SECS {
                return Err(
                    format!(
                        "Opening positions cooldown active: wait {}s",
                        POSITION_OPEN_COOLDOWN_SECS - elapsed
                    )
                );
            }
        }
    }

    if is_dry_run_enabled() {
        log(
            LogTag::Positions,
            "DRY-RUN",
            &format!(
                "🚫 DRY-RUN: Would open position for {} at {:.6} SOL",
                token.symbol,
                entry_price
            )
        );
        return Err("DRY-RUN: Position would be opened".to_string());
    }

    // Execute swap
    let wallet_address = get_wallet_address().map_err(|e|
        format!("Failed to get wallet address: {}", e)
    )?;

    let quote = get_best_quote_for_opening(
        SOL_MINT,
        &token.mint,
        sol_to_lamports(TRADE_SIZE_SOL),
        &wallet_address,
        SLIPPAGE_QUOTE_DEFAULT_PCT, // Use configured slippage for opening
        &token.symbol
    ).await.map_err(|e| format!("Quote failed: {}", e))?;

    let swap_result = execute_best_swap(
        &token,
        SOL_MINT,
        &token.mint,
        sol_to_lamports(TRADE_SIZE_SOL),
        quote
    ).await.map_err(|e| format!("Swap failed: {}", e))?;

    let transaction_signature = swap_result.transaction_signature.ok_or(
        "No transaction signature"
    )?;

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
    let position_id = save_position(&position).await.map_err(|e|
        format!("Failed to save position: {}", e)
    )?;

    let mut position_with_id = position;
    position_with_id.id = Some(position_id);

    // Add to state
    add_position(position_with_id).await;

    // We intentionally keep the global slot occupied for the lifecycle of this position by
    // calling forget() so the permit is NOT returned on drop. Terminal transitions will
    // explicitly release it.
    _global_permit.forget();
    add_signature_to_index(&transaction_signature, &token.mint).await;

    // Get block height for expiration
    let expiry_height = get_rpc_client()
        .get_block_height().await
        .map(|h| h + SOLANA_BLOCKHASH_VALIDITY_SLOTS)
        .ok();

    // Enqueue for verification
    let verification_item = VerificationItem::new(
        transaction_signature.clone(),
        token.mint.clone(),
        Some(position_id),
        VerificationKind::Entry,
        expiry_height
    );

    enqueue_verification(verification_item).await;

    log(
        LogTag::Positions,
        "SUCCESS",
        &format!(
            "✅ Position opened: {} (ID: {}) | TX: {}",
            token.symbol,
            position_id,
            transaction_signature
        )
    );

    // Update global last open time
    {
        let mut last = LAST_OPEN_TIME.write().await;
        *last = Some(Utc::now());
    }

    Ok(transaction_signature)
}

/// Close an existing position
pub async fn close_position_direct(
    token_mint: &str,
    exit_reason: String
) -> Result<String, String> {
    let token = get_token_from_db(token_mint).await.ok_or_else(||
        format!("Token not found: {}", token_mint)
    )?;

    let price_info = get_pool_price(token_mint).ok_or_else(||
        format!("No price data for token: {}", token_mint)
    )?;

    let exit_price = match price_info.price_sol {
        price if price > 0.0 && price.is_finite() => price,
        _ => {
            return Err(format!("Invalid exit price for token: {}", token_mint));
        }
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

    // Get TOTAL token balance across ALL accounts (CRITICAL FOR COMPLETE LIQUIDATION)
    let wallet_address = get_wallet_address().map_err(|e|
        format!("Failed to get wallet address: {}", e)
    )?;

    let total_token_balance = get_total_token_balance(&wallet_address, token_mint).await.map_err(|e|
        format!("Failed to get total token balance: {}", e)
    )?;

    // Fetch primary (associated) token account balance separately. This is the balance most
    // swap routes will actually spend from. When multiple token accounts exist, passing the
    // aggregated total to a router that only sources a single ATA causes an "insufficient funds"
    // simulation failure (observed in logs). We therefore cap the sell amount to the primary
    // balance when it is lower than the aggregate, and log the discrepancy.
    let primary_token_balance = get_token_balance(&wallet_address, token_mint).await.unwrap_or(0);

    let (sell_amount, multi_account_note) = if
        primary_token_balance == 0 &&
        total_token_balance > 0
    {
        // We have tokens but not in the primary ATA (likely split or token-2022 alt). Use total but
        // expect potential router failure; still attempt but log.
        (total_token_balance, Some("primary_ata_empty_using_total".to_string()))
    } else if total_token_balance > primary_token_balance && primary_token_balance > 0 {
        (
            primary_token_balance,
            Some(
                format!(
                    "multi_account_total={} primary={} shortfall={}, limiting_to_primary",
                    total_token_balance,
                    primary_token_balance,
                    total_token_balance - primary_token_balance
                )
            ),
        )
    } else {
        (total_token_balance, None)
    };

    if sell_amount == 0 {
        return Err("No tokens to sell".to_string());
    }

    log(
        LogTag::Positions,
        "SELL_ALL",
        &format!(
            "🔄 Selling ALL tokens for {}: {} total units across all accounts",
            token.symbol,
            total_token_balance
        )
    );

    if let Some(note) = &multi_account_note {
        log(
            LogTag::Positions,
            "MULTI_ACCOUNT_SELL_ADJUSTMENT",
            &format!("⚠️ Sell amount adjusted due to account distribution: {}", note)
        );
    }

    // Execute swap
    let quote = get_best_quote(
        token_mint,
        SOL_MINT,
        sell_amount,
        &wallet_address,
        SLIPPAGE_EXIT_RETRY_STEPS_PCT[0] // Use first step (3.0%) for initial exit attempt
    ).await.map_err(|e| format!("Quote failed: {}", e))?;

    let swap_result = execute_best_swap(
        &token,
        token_mint,
        SOL_MINT,
        sell_amount,
        quote
    ).await.map_err(|e| {
        // If we attempted to sell the aggregated total and failed with insufficient funds,
        // hint at likely multi-account cause for easier diagnosis.
        let msg = e.to_string();
        if
            msg.to_lowercase().contains("insufficient funds") &&
            multi_account_note.is_none() &&
            total_token_balance > sell_amount
        {
            format!("Swap failed (insufficient funds) - aggregated balance mismatch; consider consolidating ATAs: {}", msg)
        } else {
            format!("Swap failed: {}", msg)
        }
    })?;

    let transaction_signature = swap_result.transaction_signature.ok_or(
        "No transaction signature"
    )?;

    // CRITICAL: Log execution vs requested amounts to detect partial execution
    if let Ok(executed_amount) = swap_result.input_amount.parse::<u64>() {
        if executed_amount < sell_amount {
            log(
                LogTag::Positions,
                "PARTIAL_EXECUTION",
                &format!(
                    "⚠️ PARTIAL SWAP DETECTED for {}: Requested {} tokens, executed {} tokens, shortfall: {}",
                    token.symbol,
                    sell_amount,
                    executed_amount,
                    sell_amount - executed_amount
                )
            );
        } else {
            log(
                LogTag::Positions,
                "FULL_EXECUTION",
                &format!("✅ Full swap executed for {}: {} tokens", token.symbol, executed_amount)
            );
        }
    } else {
        log(
            LogTag::Positions,
            "EXECUTION_PARSE_ERROR",
            &format!(
                "⚠️ Could not parse executed amount '{}' for {}",
                swap_result.input_amount,
                token.symbol
            )
        );
    }

    // Update position with exit signature
    super::state::update_position_state(token_mint, |pos| {
        pos.exit_transaction_signature = Some(transaction_signature.clone());
        pos.closed_reason = Some(format!("{}_pending_verification", exit_reason));
    }).await;

    add_signature_to_index(&transaction_signature, token_mint).await;

    // Get position ID
    let position_id = super::state
        ::get_position_by_mint(token_mint).await
        .and_then(|p| p.id)
        .unwrap_or(0);

    // Get block height for expiration
    let expiry_height = get_rpc_client()
        .get_block_height().await
        .map(|h| h + SOLANA_BLOCKHASH_VALIDITY_SLOTS)
        .ok();

    // Enqueue for verification
    let verification_item = VerificationItem::new(
        transaction_signature.clone(),
        token_mint.to_string(),
        Some(position_id),
        VerificationKind::Exit,
        expiry_height
    );

    enqueue_verification(verification_item).await;

    log(
        LogTag::Positions,
        "SUCCESS",
        &format!(
            "✅ Position closing: {} | TX: {} | Reason: {}",
            token.symbol,
            transaction_signature,
            exit_reason
        )
    );

    Ok(transaction_signature)
}
