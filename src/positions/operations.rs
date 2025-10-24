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
    logger::{self, LogTag},
    pools::get_pool_price,
    pools::PriceResult,
    rpc::get_rpc_client,
    swaps::{execute_best_swap, get_best_quote, get_best_quote_for_opening},
    utils::{get_token_balance, get_total_token_balance, get_wallet_address, sol_to_lamports},
};
use chrono::Utc;
use serde_json::json;

const SOLANA_BLOCKHASH_VALIDITY_SLOTS: u64 = 150;

/// Internal helper to open a new position with an explicit SOL size
async fn open_position_impl(token_mint: &str, trade_size_sol: f64) -> Result<String, String> {
    let api_token = crate::tokens::get_full_token_async(token_mint)
        .await
        .map_err(|e| format!("Failed to get token: {}", e))?
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
                logger::warning(
                    LogTag::Positions,
                    &format!(
                        "🚫 DB guard: mint {} already has open/unverified position (id: {:?}, entry_sig: {:?}, exit_sig: {:?})",
                        &api_token.mint,
                        db_pos.id,
                        db_pos.entry_transaction_signature,
                        db_pos.exit_transaction_signature
                    ),
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
        logger::info(
            LogTag::Positions,
            &format!(
                "🚫 DRY-RUN: Would open position for {} at {} SOL",
                api_token.symbol,
                crate::utils::format_price_adaptive(entry_price)
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

    let slippage_quote_default = with_config(|cfg| cfg.swaps.slippage.quote_default_pct);

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
        // Initialize partial exit and DCA fields
        remaining_token_amount: None, // Will be set after entry verification
        total_exited_amount: 0,
        average_exit_price: None,
        partial_exit_count: 0,
        dca_count: 0,
        average_entry_price: entry_price, // Initial entry price
        last_dca_time: None,
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

    logger::info(
        LogTag::Positions,
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

/// Open a new position using trade size from configuration
pub async fn open_position_direct(token_mint: &str) -> Result<String, String> {
    let trade_size_sol = with_config(|cfg| cfg.trader.trade_size_sol);
    open_position_impl(token_mint, trade_size_sol).await
}

/// Open a new position with an explicit SOL size (used by manual buys)
pub async fn open_position_with_size(
    token_mint: &str,
    trade_size_sol: f64,
) -> Result<String, String> {
    if !trade_size_sol.is_finite() || trade_size_sol <= 0.0 {
        return Err(format!("Invalid trade size: {}", trade_size_sol));
    }
    open_position_impl(token_mint, trade_size_sol).await
}

/// Close an existing position
pub async fn close_position_direct(
    token_mint: &str,
    exit_reason: String,
) -> Result<String, String> {
    let api_token = crate::tokens::get_full_token_async(token_mint)
        .await
        .map_err(|e| format!("Failed to get token: {}", e))?
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

    // RACE CONDITION PREVENTION: Only block if a FULL exit is pending.
    // Partial exits are tracked separately via PENDING_PARTIAL_EXITS and serialized.
    if let Some(existing_position) = super::state::get_position_by_mint(token_mint).await {
        if let Some(pending_sig) = &existing_position.exit_transaction_signature {
            logger::warning(
                LogTag::Positions,
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
        logger::info(
            LogTag::Positions,
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

    logger::info(
        LogTag::Positions,
        &format!(
            "🔄 Selling ALL tokens for {}: {} total units across all accounts",
            api_token.symbol, total_token_balance
        ),
    );

    if let Some(note) = &multi_account_note {
        logger::warning(
            LogTag::Positions,
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
        with_config(|cfg| cfg.swaps.slippage.exit_retry_steps_pct.clone());
    // Slippage retry loop for exit
    let mut last_err: Option<String> = None;
    let mut swap_result = None;
    for (i, slippage) in slippage_exit_retry_steps.iter().enumerate() {
        let quote = match get_best_quote(
            token_mint,
            SOL_MINT,
            sell_amount,
            &wallet_address,
            *slippage,
            "ExactIn",
        )
        .await
        {
            Ok(q) => q,
            Err(e) => {
                last_err = Some(format!(
                    "Quote failed at step {} ({}%): {}",
                    i + 1,
                    slippage,
                    e
                ));
                continue;
            }
        };

        match execute_best_swap(&api_token, token_mint, SOL_MINT, sell_amount, quote).await {
            Ok(res) => {
                swap_result = Some(res);
                last_err = None;
                break;
            }
            Err(e) => {
                // If we attempted to sell the aggregated total and failed with insufficient funds,
                // hint at likely multi-account cause for easier diagnosis.
                let msg = e.to_string();
                let enriched = if msg.to_lowercase().contains("insufficient funds")
                    && multi_account_note.is_none()
                    && total_token_balance > sell_amount
                {
                    format!("Swap failed (insufficient funds) - aggregated balance mismatch; consider consolidating ATAs: {}", msg)
                } else {
                    format!("Swap failed: {}", msg)
                };
                last_err = Some(format!(
                    "{} (step {} slippage {}%)",
                    enriched,
                    i + 1,
                    slippage
                ));
                continue;
            }
        }
    }

    let swap_result =
        swap_result.ok_or_else(|| last_err.unwrap_or_else(|| "Exit swap failed".to_string()))?;

    let transaction_signature = swap_result
        .transaction_signature
        .ok_or("No transaction signature")?;

    // CRITICAL: Log execution vs requested amounts to detect partial execution
    if let Ok(executed_amount) = swap_result.input_amount.parse::<u64>() {
        if executed_amount < sell_amount {
            logger::warning(
                LogTag::Positions,
                &format!(
                    "⚠️ PARTIAL SWAP DETECTED for {}: Requested {} tokens, executed {} tokens, shortfall: {}",
                    api_token.symbol,
                    sell_amount,
                    executed_amount,
                    sell_amount - executed_amount
                ),
            );
        } else {
            logger::info(
                LogTag::Positions,
                &format!(
                    "✅ Full swap executed for {}: {} tokens",
                    api_token.symbol, executed_amount
                ),
            );
        }
    } else {
        logger::warning(
            LogTag::Positions,
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

    logger::info(
        LogTag::Positions,
        &format!(
            "✅ Position closing: {} | TX: {} | Reason: {}",
            api_token.symbol, transaction_signature, exit_reason
        ),
    );

    Ok(transaction_signature)
}

// =============================================================================
// PARTIAL EXIT & DCA OPERATIONS
// =============================================================================

/// Partially close a position by selling a percentage of remaining tokens
/// CRITICAL: This does NOT release the semaphore permit - position stays open
pub async fn partial_close_position(
    token_mint: &str,
    exit_percentage: f64,
    exit_reason: &str,
) -> Result<String, String> {
    // Serialize per-mint operations to avoid overlapping partials/full exits
    let _lock = acquire_position_lock(token_mint).await;
    use crate::swaps::{calculate_partial_amount, ExitType};

    // Validate percentage
    if exit_percentage <= 0.0 || exit_percentage >= 100.0 {
        return Err(format!(
            "Invalid exit percentage: {}. Must be between 0 and 100 (exclusive)",
            exit_percentage
        ));
    }

    // Get position
    let position = super::state::get_position_by_mint(token_mint)
        .await
        .ok_or_else(|| format!("No open position found for token: {}", token_mint))?;

    let position_id = position
        .id
        .ok_or_else(|| "Position has no ID".to_string())?;

    // Get remaining token amount
    let remaining_amount = position
        .remaining_token_amount
        .or(position.token_amount)
        .ok_or_else(|| "Position has no token amount".to_string())?;

    // Calculate partial exit amount
    let exit_amount = calculate_partial_amount(remaining_amount, exit_percentage);

    if exit_amount == 0 {
        return Err("Calculated exit amount is zero".to_string());
    }

    logger::info(
        LogTag::Positions,
        &format!(
            "Partial exit initiated: {} | {}% ({} of {} tokens) | Reason: {}",
            position.symbol, exit_percentage, exit_amount, remaining_amount, exit_reason
        ),
    );

    // Get API token for swap
    let api_token = crate::tokens::get_full_token_async(token_mint)
        .await
        .map_err(|e| format!("Failed to get token: {}", e))?
        .ok_or_else(|| format!("Token not found: {}", token_mint))?;

    // Get quote for partial exit
    let wallet_address = get_wallet_address().map_err(|e| e.to_string())?;
    let slippage_exit_retry_steps =
        with_config(|cfg| cfg.swaps.slippage.exit_retry_steps_pct.clone());
    // Slippage retry loop for partial exit
    let mut last_err: Option<String> = None;
    let mut quote_opt = None;
    for (i, slippage) in slippage_exit_retry_steps.iter().enumerate() {
        match get_best_quote(
            token_mint,
            SOL_MINT,
            exit_amount,
            &wallet_address,
            *slippage,
            "ExactIn",
        )
        .await
        {
            Ok(q) => {
                quote_opt = Some(q);
                last_err = None;
                break;
            }
            Err(e) => {
                last_err = Some(format!(
                    "Quote failed at step {} ({}%): {}",
                    i + 1,
                    slippage,
                    e
                ));
                continue;
            }
        }
    }
    let quote = quote_opt
        .ok_or_else(|| last_err.unwrap_or_else(|| "Failed to get exit quote".to_string()))?;

    logger::info(
        LogTag::Positions,
        &format!(
            "Partial exit quote: {} tokens → {} SOL",
            exit_amount,
            quote.output_amount as f64 / 1_000_000_000.0
        ),
    );

    // Mark pending partial BEFORE executing swap to serialize concurrent attempts
    super::state::mark_partial_exit_pending(token_mint).await;

    // Execute swap
    // Try execute with the selected quote; if it fails, iterate remaining slippage steps
    let mut swap_result = execute_best_swap(&api_token, token_mint, SOL_MINT, exit_amount, quote)
        .await
        .map_err(|e| e.to_string());
    if swap_result.is_err() {
        for (i, slippage) in slippage_exit_retry_steps.iter().enumerate() {
            // We already tried first successful quote attempt; attempt new quotes with higher slippage
            let q = match get_best_quote(
                token_mint,
                SOL_MINT,
                exit_amount,
                &wallet_address,
                *slippage,
                "ExactIn",
            )
            .await
            {
                Ok(q) => q,
                Err(e) => {
                    last_err = Some(format!(
                        "Quote failed at step {} ({}%): {}",
                        i + 1,
                        slippage,
                        e
                    ));
                    continue;
                }
            };
            match execute_best_swap(&api_token, token_mint, SOL_MINT, exit_amount, q).await {
                Ok(res) => {
                    swap_result = Ok(res);
                    last_err = None;
                    break;
                }
                Err(e) => {
                    last_err = Some(format!(
                        "Partial exit swap failed at step {} ({}%): {}",
                        i + 1,
                        slippage,
                        e
                    ));
                }
            }
        }
    }
    let swap_result = match swap_result {
        Ok(res) => res,
        Err(e) => {
            super::state::clear_partial_exit_pending(token_mint).await;
            return Err(format!("Partial exit swap failed: {}", e));
        }
    };

    let transaction_signature = swap_result
        .transaction_signature
        .ok_or("No transaction signature")?;

    // Update position state (mark as partial exit pending)
    super::state::update_position_state(token_mint, |pos| {
        pos.exit_transaction_signature = Some(transaction_signature.clone());
        // Do NOT set exit_time - position is still open!
    })
    .await;

    // Save updated position to DB
    if let Some(updated_pos) = super::state::get_position_by_mint(token_mint).await {
        save_position(&updated_pos).await?;
    }

    // Add signature to index
    add_signature_to_index(&transaction_signature, token_mint).await;

    // Create partial exit transition
    let transition = super::transitions::PositionTransition::PartialExitSubmitted {
        position_id,
        exit_signature: transaction_signature.clone(),
        exit_amount,
        exit_percentage,
        market_price: position.current_price.unwrap_or(position.entry_price),
    };

    // Apply transition
    super::apply::apply_transition(transition)
        .await
        .map_err(|e| format!("Failed to apply partial exit transition: {}", e))?;

    // Enqueue for verification with partial exit flag
    let expiry_height =
        get_rpc_client().get_block_height().await.unwrap_or(0) + SOLANA_BLOCKHASH_VALIDITY_SLOTS;

    let verification_item = VerificationItem::new_partial_exit(
        transaction_signature.clone(),
        token_mint.to_string(),
        Some(position_id),
        exit_amount,
        Some(expiry_height),
    );

    enqueue_verification(verification_item).await;

    logger::info(
        LogTag::Positions,
        &format!(
            "✅ Partial exit submitted: {} | {}% | TX: {} | Reason: {}",
            api_token.symbol, exit_percentage, transaction_signature, exit_reason
        ),
    );

    // CRITICAL: Do NOT release semaphore permit - position still open!

    Ok(transaction_signature)
}

/// Add to an existing position (Dollar Cost Averaging)
/// CRITICAL: This does NOT consume a new semaphore permit - same position
pub async fn add_to_position(token_mint: &str, dca_amount_sol: f64) -> Result<String, String> {
    // Serialize per-mint DCA operations
    let _lock = acquire_position_lock(token_mint).await;
    // Get position
    let position = super::state::get_position_by_mint(token_mint)
        .await
        .ok_or_else(|| format!("No open position found for token: {}", token_mint))?;

    let position_id = position
        .id
        .ok_or_else(|| "Position has no ID".to_string())?;

    // Check DCA limits from config
    let dca_enabled = with_config(|cfg| cfg.trader.dca_enabled);
    if !dca_enabled {
        return Err("DCA is disabled in configuration".to_string());
    }

    let max_dca_count = with_config(|cfg| cfg.trader.dca_max_count);
    if position.dca_count >= max_dca_count as u32 {
        return Err(format!(
            "Maximum DCA count reached: {} (max: {})",
            position.dca_count, max_dca_count
        ));
    }

    // Check DCA cooldown
    if let Some(last_dca) = position.last_dca_time {
        let cooldown_minutes = with_config(|cfg| cfg.trader.dca_cooldown_minutes);
        let elapsed = Utc::now().signed_duration_since(last_dca).num_minutes();
        if elapsed < cooldown_minutes {
            return Err(format!(
                "DCA cooldown active: {} minutes remaining",
                cooldown_minutes - elapsed
            ));
        }
    }

    logger::info(
        LogTag::Positions,
        &format!(
            "DCA entry initiated: {} | {} SOL | DCA #{} ",
            position.symbol,
            dca_amount_sol,
            position.dca_count + 1
        ),
    );

    // Get API token for swap
    let api_token = crate::tokens::get_full_token_async(token_mint)
        .await
        .map_err(|e| format!("Failed to get token: {}", e))?
        .ok_or_else(|| format!("Token not found: {}", token_mint))?;

    // Get quote for DCA entry
    let wallet_address = get_wallet_address().map_err(|e| e.to_string())?;
    let slippage = with_config(|cfg| cfg.swaps.slippage.quote_default_pct);
    let quote = get_best_quote_for_opening(
        SOL_MINT,
        token_mint,
        sol_to_lamports(dca_amount_sol),
        &wallet_address,
        slippage,
        &api_token.symbol,
    )
    .await
    .map_err(|e| format!("Failed to get DCA quote: {}", e))?;

    logger::info(
        LogTag::Positions,
        &format!(
            "DCA quote: {} SOL → {} tokens",
            dca_amount_sol,
            quote.output_amount as f64 / 10_f64.powi(api_token.decimals as i32)
        ),
    );

    // Execute swap
    let swap_result = execute_best_swap(
        &api_token,
        SOL_MINT,
        token_mint,
        sol_to_lamports(dca_amount_sol),
        quote,
    )
    .await
    .map_err(|e| format!("DCA swap failed: {}", e))?;

    let transaction_signature = swap_result
        .transaction_signature
        .ok_or("No transaction signature")?;

    // Create DCA transition
    let price_info = get_pool_price(token_mint)
        .ok_or_else(|| format!("No price data for token: {}", token_mint))?;

    let transition = super::transitions::PositionTransition::DcaSubmitted {
        position_id,
        dca_signature: transaction_signature.clone(),
        dca_amount_sol,
        market_price: price_info.price_sol,
    };

    // Apply transition
    super::apply::apply_transition(transition)
        .await
        .map_err(|e| format!("Failed to apply DCA transition: {}", e))?;

    // Enqueue for verification
    let expiry_height =
        get_rpc_client().get_block_height().await.unwrap_or(0) + SOLANA_BLOCKHASH_VALIDITY_SLOTS;

    let verification_item = VerificationItem::new(
        transaction_signature.clone(),
        token_mint.to_string(),
        Some(position_id),
        VerificationKind::Entry, // DCA is another entry
        Some(expiry_height),
    );

    enqueue_verification(verification_item).await;

    logger::info(
        LogTag::Positions,
        &format!(
            "✅ DCA entry submitted: {} | {} SOL | TX: {} | DCA #{}",
            api_token.symbol,
            dca_amount_sol,
            transaction_signature,
            position.dca_count + 1
        ),
    );

    // CRITICAL: Do NOT consume a new semaphore permit - same position!

    Ok(transaction_signature)
}

/// Calculate weighted average entry price
pub fn calculate_average_entry_price(
    current_total_sol: f64,
    current_total_tokens: u64,
    new_sol: f64,
    new_tokens: u64,
    decimals: u8,
) -> f64 {
    let new_total_sol = current_total_sol + new_sol;
    let new_total_tokens_float =
        (current_total_tokens + new_tokens) as f64 / 10_f64.powi(decimals as i32);

    if new_total_tokens_float > 0.0 {
        new_total_sol / new_total_tokens_float
    } else {
        0.0
    }
}

/// Calculate weighted average exit price
pub fn calculate_average_exit_price(
    current_average: Option<f64>,
    current_total_exited: u64,
    new_exited: u64,
    new_price: f64,
) -> f64 {
    match current_average {
        Some(avg) => {
            let total_tokens = current_total_exited + new_exited;
            if total_tokens == 0 {
                return new_price;
            }
            let current_weight = current_total_exited as f64 / total_tokens as f64;
            let new_weight = new_exited as f64 / total_tokens as f64;
            (avg * current_weight) + (new_price * new_weight)
        }
        None => new_price,
    }
}

/// Update position's current price and track high/low for trailing stop
pub async fn update_position_price(token_mint: &str, current_price: f64) -> Result<(), String> {
    if !current_price.is_finite() || current_price <= 0.0 {
        return Err(format!("Invalid price: {}", current_price));
    }

    let updated = super::state::update_position_state(token_mint, |pos| {
        pos.current_price = Some(current_price);
        pos.current_price_updated = Some(Utc::now());

        // Update highest price for trailing stop
        if current_price > pos.price_highest {
            pos.price_highest = current_price;
        }

        // Update lowest price tracking
        if current_price < pos.price_lowest || pos.price_lowest == 0.0 {
            pos.price_lowest = current_price;
        }
    })
    .await;

    if !updated {
        return Err(format!("Position not found for mint: {}", token_mint));
    }

    // Save to database (async)
    if let Some(position) = super::state::get_position_by_mint(token_mint).await {
        save_position(&position).await?;
    }

    Ok(())
}
