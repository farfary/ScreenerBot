//! Open position operations — new position entry with swap execution and verification.

use crate::config::with_config;
use crate::constants::SOL_MINT;
use crate::logger::{self, LogTag};
use crate::positions::db as positions_db;
use crate::positions::price_resolution::get_price_with_api_fallback;
use crate::positions::queue::{enqueue_verification, VerificationItem};
use crate::positions::state::{
    acquire_global_position_permit, acquire_position_lock, add_position, add_signature_to_index,
    LAST_OPEN_TIME,
};
use crate::positions::types::{Position, VerificationKind};
use crate::rpc::{get_rpc_client, RpcClientMethods};
use crate::swaps::{
    execute_swap_with_fallback, get_best_quote_for_opening, QuoteRequest, SwapMode,
};
use crate::utils::{get_wallet_address, sol_to_lamports};
use chrono::Utc;
use serde_json::json;

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
        return Err(format!("Invalid trade size: {trade_size_sol}"));
    }
    open_position_impl(token_mint, trade_size_sol).await
}

/// Internal helper to open a new position with an explicit SOL size
async fn open_position_impl(token_mint: &str, trade_size_sol: f64) -> Result<String, String> {
    let api_token = crate::tokens::get_full_token_async(token_mint)
        .await
        .map_err(|e| format!("Failed to get token: {e}"))?
        .ok_or_else(|| format!("Token not found: {token_mint}"))?;

    // Get price with fallback to API when pool price unavailable
    // This enables trading for tokens not yet tracked by pool service
    let (price_info, price_source) =
        get_price_with_api_fallback(token_mint)
            .await
            .ok_or_else(|| {
                format!(
                    "No price data for token: {} (checked pool + API)",
                    token_mint
                )
            })?;

    let entry_price = match price_info.price_sol {
        price if price > 0.0 && price.is_finite() => price,
        _ => {
            return Err(format!("Invalid price for token: {token_mint}"));
        }
    };

    // Log price source for observability
    logger::info(
        LogTag::Positions,
        &format!(
            "Opening position for {} at {} SOL (source: {:?})",
            api_token.symbol, entry_price, price_source
        ),
    );

    // CRITICAL: Acquire global position permit FIRST to enforce MAX_OPEN_POSITIONS atomically.
    // IMPORTANT: We will "forget"this permit ONLY after the position is successfully
    // created & added to in‑memory state so that the semaphore capacity remains reduced
    // for the lifetime of the open position. All terminal close paths MUST call
    // release_global_position_permit() (verified exit, synthetic exit, orphan removal).
    // Any early error returns (before we forget the permit) will automatically drop it
    // and thus NOT consume a slot.
    let mut _global_permit = acquire_global_position_permit().await?;

    // Acquire per-mint lock SECOND to serialize opens for same token
    let _lock = acquire_position_lock(&api_token.mint).await;

    // Re-check no existing open position for this mint (prevents duplicate concurrent entries)
    if crate::positions::state::is_open_position(&api_token.mint).await {
        // Record event for better post-mortem visibility
        crate::events::record_position_event(
            "0",
            &api_token.mint,
            "open_blocked",
            None,
            None,
            trade_size_sol,
            0,
            None,
            None,
        )
        .await;

        crate::events::record_position_event_flexible(
            "open_blocked_in_memory",
            crate::events::Severity::Warn,
            Some(&api_token.mint),
            None,
            json!({
              "reason": "is_open_position_guard",
              "mint": api_token.mint,
            }),
        )
        .await;
        return Err("Already have open position for this token".to_owned());
    }

    // Extra safety: consult database for any existing open or unverified position for this mint.
    // This covers edge cases across restarts or rare state desyncs where in-memory guards miss.
    if let Ok(db_pos_opt) = positions_db::get_position_by_mint(&api_token.mint).await {
        if let Some(db_pos) = db_pos_opt {
            let is_still_open = db_pos.position_type == "buy"
                && db_pos.exit_time.is_none()
                && (db_pos.exit_transaction_signature.is_none()
                    || !db_pos.transaction_exit_verified);
            if is_still_open {
                logger::warning(
          LogTag::Positions,
          &format!(
 "DB guard: mint {} already has open/unverified position (id: {:?}, entry_sig: {:?}, exit_sig: {:?})",
            &api_token.mint,
            db_pos.id,
            db_pos.entry_transaction_signature,
            db_pos.exit_transaction_signature
          ),
        );
                // Record event for DB guard block
                crate::events::record_position_event(
                    &db_pos.id.unwrap_or_default().to_string(),
                    &api_token.mint,
                    "open_blocked_db",
                    db_pos.entry_transaction_signature.as_deref(),
                    db_pos.exit_transaction_signature.as_deref(),
                    trade_size_sol,
                    0,
                    None,
                    None,
                )
                .await;

                crate::events::record_position_event_flexible(
                    "open_blocked_db_guard",
                    crate::events::Severity::Warn,
                    Some(&api_token.mint),
                    db_pos.entry_transaction_signature.as_deref(),
                    json!({
                      "db_position_id": db_pos.id,
                      "entry_sig": db_pos.entry_transaction_signature,
                      "exit_sig": db_pos.exit_transaction_signature,
                      "has_exit_verified": db_pos.transaction_exit_verified,
                    }),
                )
                .await;
                return Err("Open position already exists in DB".to_owned());
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

    // Execute swap
    let wallet_address =
        get_wallet_address().map_err(|e| format!("Failed to get wallet address: {e}"))?;

    // Mark mint as pending-open BEFORE submitting the swap to avoid duplicate attempts
    crate::positions::state::set_pending_open(
        &api_token.mint,
        crate::positions::state::PENDING_OPEN_TTL_SECS,
    )
    .await;
    crate::events::record_position_event_flexible(
        "pending_open_set",
        crate::events::Severity::Debug,
        Some(&api_token.mint),
        None,
        json!({
          "ttl_secs": crate::positions::state::PENDING_OPEN_TTL_SECS,
        }),
    )
    .await;

    let slippage_quote_default = with_config(|cfg| cfg.swaps.slippage.quote_default_pct);

    let quote_request = QuoteRequest {
        input_mint: SOL_MINT.to_string(),
        output_mint: api_token.mint.clone(),
        input_amount: sol_to_lamports(trade_size_sol),
        wallet_address: wallet_address.clone(),
        slippage_pct: slippage_quote_default,
        swap_mode: SwapMode::ExactIn,
    };

    let quote = get_best_quote_for_opening(quote_request, &api_token.symbol)
        .await
        .map_err(|e| format!("Quote failed: {e}"))?;

    let swap_result = execute_swap_with_fallback(&api_token, quote)
        .await
        .map_err(|e| format!("Swap failed: {e}"))?;

    let transaction_signature = swap_result.transaction_signature;

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
        position_type: "buy".to_owned(),
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
        liquidity_tier: Some("UNKNOWN".to_owned()),
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
        // P&L fields - initialized as None, will be calculated by positions system
        pnl: None,
        pnl_percent: None,
        unrealized_pnl: None,
        unrealized_pnl_percent: None,
        // Initialize partial exit and DCA fields
        remaining_token_amount: None, // Will be set after entry verification
        total_exited_amount: 0,
        average_exit_price: None,
        partial_exit_count: 0,
        dca_count: 0,
        average_entry_price: entry_price, // Initial entry price
        last_dca_time: None,
    };

    // Save to database (with retry) and get ID
    let position_id = super::persist_position_with_retry(&position).await;

    let mut position_with_id = position;
    position_with_id.id = Some(position_id);

    // Add to state
    add_position(position_with_id).await;

    // Bug #25 fix: Set token priority to OpenPosition (100) for fastest updates (5s interval)
    // This ensures price tracking is responsive during active trading
    if let Some(db) = crate::tokens::database::get_global_database() {
        let _ = db.update_priority(
            &api_token.mint,
            crate::tokens::priorities::Priority::OpenPosition.to_value(),
        );
        logger::debug(
            LogTag::Positions,
            &format!("Set token {} to OpenPosition priority", api_token.symbol),
        );
    }

    // We intentionally keep the global slot occupied for the lifecycle of this position by
    // calling forget() so the permit is NOT returned on drop. Terminal transitions will
    // explicitly release it.
    _global_permit.forget();
    add_signature_to_index(&transaction_signature, &api_token.mint).await;

    // Record a position opened event for durability
    crate::events::record_position_event(
        &position_id.to_string(),
        &api_token.mint,
        "opened",
        Some(&transaction_signature),
        None,
        trade_size_sol,
        swap_result.output_amount,
        None,
        None,
    )
    .await;

    // Get block height for expiration
    let expiry_height = get_rpc_client()
        .get_block_height()
        .await
        .map(|h| h + super::SOLANA_BLOCKHASH_VALIDITY_SLOTS)
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
    crate::positions::state::clear_pending_open(&api_token.mint).await;
    crate::events::record_position_event_flexible(
        "pending_open_cleared",
        crate::events::Severity::Debug,
        Some(&api_token.mint),
        Some(&transaction_signature),
        json!({}),
    )
    .await;

    logger::info(
        LogTag::Positions,
        &format!(
            "Position opened: {} (ID: {}) | TX: {}",
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
