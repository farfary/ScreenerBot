use super::db as positions_db;
use super::{
    apply::apply_transition,
    queue::{enqueue_verification, VerificationItem, VerificationKind},
    state::{
        acquire_global_position_permit, acquire_position_lock, add_position,
        add_signature_to_index, release_global_position_permit, LAST_OPEN_TIME,
    },
    transitions::PositionTransition,
};
use super::{db::save_position, types::Position};
use crate::{
    arguments::{is_debug_positions_enabled, is_dry_run_enabled},
    config::with_config,
    constants::SOL_MINT,
    logger::{log, LogTag},
    pools::get_pool_price,
    pools::PriceResult,
    rpc::get_rpc_client,
    swaps::{execute_best_swap, get_best_quote, get_best_quote_for_opening},
    utils::{get_token_balance, get_total_token_balance, get_wallet_address, sol_to_lamports},
};
use chrono::Utc;
use serde_json::json;

const SOLANA_BLOCKHASH_VALIDITY_SLOTS: u64 = 150;

/// Open a new position
pub async fn open_position_direct(token_mint: &str) -> Result<String, String> {
    let api_token = crate::tokens::store::get_token(token_mint)
        .ok_or_else(|| format!("Token not found: {}", token_mint))?;

    let price_info = get_pool_price(token_mint)
        .ok_or_else(|| format!("No price data for token: {}", token_mint))?;

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
    let _lock = acquire_position_lock(&api_token.mint).await;

    // Re-check no existing open position for this mint (prevents duplicate concurrent entries)
    if super::state::is_open_position(&api_token.mint).await {
        // Record event for better post-mortem visibility
        crate::events::record_safe(crate::events::Event::new(
            crate::events::EventCategory::Position,
            Some("open_blocked_in_memory".to_string()),
            crate::events::Severity::Warn,
            Some(api_token.mint.clone()),
            None,
            json!({
                "reason": "is_open_position_guard",
                "mint": api_token.mint,
            }),
        ))
        .await;
        return Err("Already have open position for this token".to_string());
    }

    // Extra safety: consult database for any existing open or unverified position for this mint.
    // This covers edge cases across restarts or rare state desyncs where in-memory guards miss.
    if let Ok(db_pos_opt) = positions_db::get_position_by_mint(&api_token.mint).await {
        if let Some(db_pos) = db_pos_opt {
            let is_still_open = db_pos.position_type == "buy"
                && db_pos.exit_time.is_none()
                && (!db_pos.exit_transaction_signature.is_some()
                    || !db_pos.transaction_exit_verified);
            if is_still_open {
                log(
                    LogTag::Positions,
                    "DB_GUARD_OPEN_BLOCKED",
                    &format!(
                        "🚫 DB guard: mint {} already has open/unverified position (id: {:?}, entry_sig: {:?}, exit_sig: {:?})",
                        &api_token.mint,
                        db_pos.id,
                        db_pos.entry_transaction_signature,
                        db_pos.exit_transaction_signature
                    )
                );
                // Record event for DB guard block
                crate::events::record_safe(crate::events::Event::new(
                    crate::events::EventCategory::Position,
                    Some("open_blocked_db_guard".to_string()),
                    crate::events::Severity::Warn,
                    Some(api_token.mint.clone()),
                    db_pos.entry_transaction_signature.clone(),
                    json!({
                        "db_position_id": db_pos.id,
                        "entry_sig": db_pos.entry_transaction_signature,
                        "exit_sig": db_pos.exit_transaction_signature,
                        "has_exit_verified": db_pos.transaction_exit_verified,
                    }),
                ))
                .await;
                return Err("Open position already exists in DB".to_string());
            }
        }
    }

    // Note: No need to check MAX_OPEN_POSITIONS here anymore - the semaphore enforces it atomically

    // Lightweight global cooldown (prevents rapid duplicate openings across different tokens)
    {
        let cooldown_secs = with_config(|cfg| cfg.positions.position_open_cooldown_secs);
        let last_open_opt = LAST_OPEN_TIME.read().await.clone();
        if let Some(last_open) = last_open_opt {
            let elapsed = Utc::now().signed_duration_since(last_open).num_seconds();
            if elapsed < cooldown_secs {
                return Err(format!(
                    "Opening positions cooldown active: wait {}s",
                    cooldown_secs - elapsed
                ));
            }
        }
    }

    if is_dry_run_enabled() {
        log(
            LogTag::Positions,
            "DRY-RUN",
            &format!(
                "🚫 DRY-RUN: Would open position for {} at {:.6} SOL",
                api_token.symbol, entry_price
            ),
        );
        return Err("DRY-RUN: Position would be opened".to_string());
    }

    // Execute swap
    let wallet_address =
        get_wallet_address().map_err(|e| format!("Failed to get wallet address: {}", e))?;

    // Mark mint as pending-open BEFORE submitting the swap to avoid duplicate attempts
    super::state::set_pending_open(&api_token.mint, super::state::PENDING_OPEN_TTL_SECS).await;
    crate::events::record_safe(crate::events::Event::new(
        crate::events::EventCategory::Position,
        Some("pending_open_set".to_string()),
        crate::events::Severity::Debug,
        Some(api_token.mint.clone()),
        None,
        json!({
            "ttl_secs": super::state::PENDING_OPEN_TTL_SECS,
        }),
    ))
    .await;

    let trade_size_sol = with_config(|cfg| cfg.trader.trade_size_sol);
    let slippage_quote_default = with_config(|cfg| cfg.swaps.slippage_quote_default_pct);

    let quote = get_best_quote_for_opening(
        SOL_MINT,
        &api_token.mint,
        sol_to_lamports(trade_size_sol),
        &wallet_address,
        slippage_quote_default, // Use configured slippage for opening
        &api_token.symbol,
    )
    .await
    .map_err(|e| format!("Quote failed: {}", e))?;

    let swap_result = execute_best_swap(
        &api_token,
        SOL_MINT,
        &api_token.mint,
        sol_to_lamports(trade_size_sol),
        quote,
    )
    .await
    .map_err(|e| format!("Swap failed: {}", e))?;

    let transaction_signature = swap_result
        .transaction_signature
        .ok_or("No transaction signature")?;

    // Create position
    let position = Position {
        id: None,
        mint: api_token.mint.clone(),
        symbol: api_token.symbol.clone(),
        name: api_token.name.clone(),
        entry_price,
        entry_time: Utc::now(),
        exit_price: None,
        exit_time: None,
        position_type: "buy".to_string(),
        entry_size_sol: trade_size_sol,
        total_size_sol: trade_size_sol,
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
    let position_id = match save_position(&position).await {
        Ok(id) => id,
        Err(e) => {
            // Keep pending-open state for TTL so retries are blocked; propagate error
            return Err(format!("Failed to save position: {}", e));
        }
    };

    let mut position_with_id = position;
    position_with_id.id = Some(position_id);

    // Add to state
    add_position(position_with_id).await;

    // We intentionally keep the global slot occupied for the lifecycle of this position by
    // calling forget() so the permit is NOT returned on drop. Terminal transitions will
    // explicitly release it.
    _global_permit.forget();
    add_signature_to_index(&transaction_signature, &api_token.mint).await;

    // Record a position opened event for durability
    crate::events::record_position_event(
        &format!("{}", position_id),
        &api_token.mint,
        "opened",
        Some(&transaction_signature),
        None,
        trade_size_sol,
        swap_result.output_amount.parse().unwrap_or(0),
        None,
        None,
    )
    .await;

    // Get block height for expiration
    let expiry_height = get_rpc_client()
        .get_block_height()
        .await
        .map(|h| h + SOLANA_BLOCKHASH_VALIDITY_SLOTS)
        .ok();

    // Enqueue for verification
    let verification_item = VerificationItem::new(
        transaction_signature.clone(),
        api_token.mint.clone(),
        Some(position_id),
        VerificationKind::Entry,
        expiry_height,
    );

    enqueue_verification(verification_item).await;

    // We successfully created the position; clear pending-open now
    super::state::clear_pending_open(&api_token.mint).await;
    crate::events::record_safe(crate::events::Event::new(
        crate::events::EventCategory::Position,
        Some("pending_open_cleared".to_string()),
        crate::events::Severity::Debug,
        Some(api_token.mint.clone()),
        Some(transaction_signature.clone()),
        json!({}),
    ))
    .await;

    log(
        LogTag::Positions,
        "SUCCESS",
        &format!(
            "✅ Position opened: {} (ID: {}) | TX: {}",
            api_token.symbol, position_id, transaction_signature
        ),
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
    exit_reason: String,
) -> Result<String, String> {
    let api_token = crate::tokens::store::get_token(token_mint)
        .ok_or_else(|| format!("Token not found: {}", token_mint))?;

    let price_info = get_pool_price(token_mint)
        .ok_or_else(|| format!("No price data for token: {}", token_mint))?;

    let exit_price = match price_info.price_sol {
        price if price > 0.0 && price.is_finite() => price,
        _ => {
            return Err(format!("Invalid exit price for token: {}", token_mint));
        }
    };

    let _lock = acquire_position_lock(token_mint).await;

    // RACE CONDITION PREVENTION: Check if position already has pending exit
    if let Some(existing_position) = super::state::get_position_by_mint(token_mint).await {
        if existing_position.exit_transaction_signature.is_some() {
            let pending_sig = existing_position.exit_transaction_signature.unwrap();
            log(
                LogTag::Positions,
                "RACE_PREVENTION",
                &format!(
                    "🚫 Position {} already has pending exit transaction: {}",
                    api_token.symbol,
                    &pending_sig[..8]
                ),
            );
            crate::events::record_safe(crate::events::Event::new(
                crate::events::EventCategory::Position,
                Some("exit_blocked_pending_sig".to_string()),
                crate::events::Severity::Warn,
                Some(api_token.mint.clone()),
                Some(pending_sig.clone()),
                json!({
                    "reason": "pending_exit_tx_present"
                }),
            ))
            .await;
            return Err(format!(
                "Position already has pending exit transaction: {}",
                &pending_sig[..8]
            ));
        }
    }

    if is_dry_run_enabled() {
        log(
            LogTag::Positions,
            "DRY-RUN",
            &format!("🚫 DRY-RUN: Would close position for {}", api_token.symbol),
        );
        return Err("DRY-RUN: Position would be closed".to_string());
    }

    // Get TOTAL token balance across ALL accounts (CRITICAL FOR COMPLETE LIQUIDATION)
    let wallet_address =
        get_wallet_address().map_err(|e| format!("Failed to get wallet address: {}", e))?;

    let total_token_balance = get_total_token_balance(&wallet_address, token_mint)
        .await
        .map_err(|e| format!("Failed to get total token balance: {}", e))?;

    // Fetch primary (associated) token account balance separately. This is the balance most
    // swap routes will actually spend from. When multiple token accounts exist, passing the
    // aggregated total to a router that only sources a single ATA causes an "insufficient funds"
    // simulation failure (observed in logs). We therefore cap the sell amount to the primary
    // balance when it is lower than the aggregate, and log the discrepancy.
    let primary_token_balance = get_token_balance(&wallet_address, token_mint)
        .await
        .unwrap_or(0);

    let (sell_amount, multi_account_note) = if primary_token_balance == 0 && total_token_balance > 0
    {
        // We have tokens but not in the primary ATA (likely split or token-2022 alt). Use total but
        // expect potential router failure; still attempt but log.
        (
            total_token_balance,
            Some("primary_ata_empty_using_total".to_string()),
        )
    } else if total_token_balance > primary_token_balance && primary_token_balance > 0 {
        (
            primary_token_balance,
            Some(format!(
                "multi_account_total={} primary={} shortfall={}, limiting_to_primary",
                total_token_balance,
                primary_token_balance,
                total_token_balance - primary_token_balance
            )),
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
            api_token.symbol, total_token_balance
        ),
    );

    if let Some(note) = &multi_account_note {
        log(
            LogTag::Positions,
            "MULTI_ACCOUNT_SELL_ADJUSTMENT",
            &format!(
                "⚠️ Sell amount adjusted due to account distribution: {}",
                note
            ),
        );
    }

    // Execute swap
    // IMPORTANT: Use ExactIn here. For exits we want to spend the exact token amount we actually have
    // (often restricted to a single ATA). Using ExactOut with `sell_amount` (token units) makes routers
    // treat it as desired SOL out, causing them to require more tokens than reside in the spending ATA,
    // which leads to SPL Token "insufficient funds" during Transfer. ExactIn avoids that.
    let slippage_exit_retry_steps =
        with_config(|cfg| cfg.swaps.slippage_exit_retry_steps_pct.clone());
    let quote = get_best_quote(
        token_mint,
        SOL_MINT,
        sell_amount,
        &wallet_address,
        slippage_exit_retry_steps[0], // Use first step (3.0%) for initial exit attempt
        "ExactIn", // Spend exactly the available input tokens; router computes SOL out
    )
    .await
    .map_err(|e| format!("Quote failed: {}", e))?;

    let swap_result = execute_best_swap(
        &api_token,
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

    let transaction_signature = swap_result
        .transaction_signature
        .ok_or("No transaction signature")?;

    // CRITICAL: Log execution vs requested amounts to detect partial execution
    if let Ok(executed_amount) = swap_result.input_amount.parse::<u64>() {
        if executed_amount < sell_amount {
            log(
                LogTag::Positions,
                "PARTIAL_EXECUTION",
                &format!(
                    "⚠️ PARTIAL SWAP DETECTED for {}: Requested {} tokens, executed {} tokens, shortfall: {}",
                    api_token.symbol,
                    sell_amount,
                    executed_amount,
                    sell_amount - executed_amount
                )
            );
        } else {
            log(
                LogTag::Positions,
                "FULL_EXECUTION",
                &format!(
                    "✅ Full swap executed for {}: {} tokens",
                    api_token.symbol, executed_amount
                ),
            );
        }
    } else {
        log(
            LogTag::Positions,
            "EXECUTION_PARSE_ERROR",
            &format!(
                "⚠️ Could not parse executed amount '{}' for {}",
                swap_result.input_amount, api_token.symbol
            ),
        );
    }

    // Update position with exit signature and market exit price
    super::state::update_position_state(token_mint, |pos| {
        pos.exit_transaction_signature = Some(transaction_signature.clone());
        pos.exit_price = Some(exit_price); // Store pool/market price at exit decision time
        pos.closed_reason = Some(format!("{}_pending_verification", exit_reason));
    })
    .await;

    add_signature_to_index(&transaction_signature, token_mint).await;

    // Get position ID (needed for event recording)
    let position_id = super::state::get_position_by_mint(token_mint)
        .await
        .and_then(|p| p.id)
        .unwrap_or(0);

    // Record a position closing event (pending verification)
    crate::events::record_position_event(
        &format!("{}", position_id),
        token_mint,
        "closing_submitted",
        None,
        Some(&transaction_signature),
        0.0,
        sell_amount,
        None,
        None,
    )
    .await;

    // Get block height for expiration
    let expiry_height = get_rpc_client()
        .get_block_height()
        .await
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
        &format!(
            "✅ Position closing: {} | TX: {} | Reason: {}",
            api_token.symbol, transaction_signature, exit_reason
        ),
    );

    Ok(transaction_signature)
}
