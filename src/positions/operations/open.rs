// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Open position operations — new position entry with swap execution and verification.

use crate::chains::adapter;
use crate::chains::RawAmount;
use crate::config::with_config;
use crate::logger::{self, LogTag};
use crate::positions::db as positions_db;
use crate::positions::price_resolution::get_price_with_api_fallback;
use crate::positions::queue::{enqueue_verification, VerificationItem};
use crate::positions::state::{
    acquire_global_position_permit, acquire_position_lock, add_position, add_signature_to_index,
    mark_swap_in_flight, LAST_OPEN_TIME,
};
use crate::positions::types::{
    EntrySubmission, Position, PositionManagement, PositionOrigin, VerificationKind,
};
use crate::positions::{Error, Result};
use crate::swaps::{
    execute_swap_with_fallback, get_best_quote_for_opening, QuoteRequest, SwapMode,
};
use crate::utils::get_wallet_address;
use chrono::Utc;
use serde_json::json;

/// Open a new position using trade size from configuration (auto-trader path)
pub async fn open_position_direct(token_mint: &str) -> Result<EntrySubmission> {
    let trade_size_native = with_config(|cfg| cfg.trader.trade_size_sol);
    // Auto-trader entry: slippage always follows config.
    open_position_impl(
        token_mint,
        trade_size_native,
        PositionOrigin::Auto { strategy_id: None },
        PositionManagement::AutoTrader,
        None,
    )
    .await
}

/// Open a new position with an explicit SOL size (used by manual buys).
///
pub async fn open_position_with_size(
    token_mint: &str,
    trade_size_native: f64,
    origin: PositionOrigin,
    management: PositionManagement,
    slippage_pct: Option<f64>,
) -> Result<EntrySubmission> {
    if !trade_size_native.is_finite() || trade_size_native <= 0.0 {
        return Err(Error::InvalidTradeSize {
            amount_native: trade_size_native,
            reason: "must be a positive, finite amount".to_owned(),
        });
    }
    open_position_impl(
        token_mint,
        trade_size_native,
        origin,
        management,
        slippage_pct,
    )
    .await
}

/// Internal helper to open a new position with an explicit SOL size
async fn open_position_impl(
    token_mint: &str,
    trade_size_native: f64,
    origin: PositionOrigin,
    management: PositionManagement,
    slippage_pct: Option<f64>,
) -> Result<EntrySubmission> {
    // Ensure the token exists in the local DB. For manual/force buys this lets the user
    // trade tokens that were never tracked by the pool service or that failed filtering
    // (decimals fetched from chain + metadata fetched from APIs on demand).
    let api_token =
        crate::tokens::ensure_token_available(crate::chains::active_chain(), token_mint)
            .await
            .map_err(|_| Error::TokenNotFound {
                mint: token_mint.to_owned(),
            })?;

    // Get price with fallback to API when pool price unavailable
    // This enables trading for tokens not yet tracked by pool service
    let (price_info, price_source) =
        get_price_with_api_fallback(token_mint)
            .await
            .ok_or_else(|| Error::InvalidPrice {
                mint: token_mint.to_owned(),
                price: 0.0,
            })?;

    let entry_price = match price_info.price_native {
        price if price > 0.0 && price.is_finite() => price,
        price => {
            return Err(Error::InvalidPrice {
                mint: token_mint.to_owned(),
                price,
            });
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
    // release_position_slot(id) (verified exit, synthetic exit, orphan removal, archive,
    // delete, force close) — which is idempotent, so only the first of them frees the slot.
    // Any early error returns (before we forget the permit) will automatically drop it
    // and thus NOT consume a slot.
    let mut _global_permit = acquire_global_position_permit().await?;

    // Acquire per-mint lock SECOND to serialize opens for same token
    let _lock = acquire_position_lock(&api_token.mint).await;

    // A mint has one open position, archived or not, and every later buy of it is an add on
    // that position: an open is refused while memory or storage holds one, or while another
    // open of the mint is pending. A storage read that fails refuses the open too, since
    // nothing has been sent yet.
    let in_memory = crate::positions::state::get_open_round_by_mint(&api_token.mint)
        .await
        .and_then(|position| position.id);
    let open_position_id = match in_memory {
        Some(id) => Some(id),
        None => positions_db::get_open_round_id(&api_token.mint).await?,
    };
    if let Some(open_position_id) = open_position_id {
        crate::events::record_position_event(
            &open_position_id.to_string(),
            &api_token.mint,
            "open_blocked",
            None,
            None,
            trade_size_native,
            RawAmount::ZERO,
            None,
            None,
        )
        .await;
        crate::events::record_position_event_flexible(
            "open_blocked_by_open_round",
            crate::events::Severity::Warn,
            Some(&api_token.mint),
            None,
            json!({
              "mint": api_token.mint,
              "open_position_id": open_position_id,
              "source": if in_memory.is_some() { "memory" } else { "storage" },
            }),
        )
        .await;
        return Err(Error::AlreadyOpen {
            mint: api_token.mint.clone(),
            open_position_id,
        });
    }
    if crate::positions::state::holds_open_round(&api_token.mint).await {
        // No open position of the mint exists yet, but another open of it may still land.
        return Err(Error::OpenPending {
            mint: api_token.mint.clone(),
        });
    }

    // Note: No need to check MAX_OPEN_POSITIONS here anymore - the semaphore enforces it atomically

    // Lightweight global cooldown (prevents rapid duplicate openings across different tokens)
    {
        let cooldown_secs = with_config(|cfg| cfg.positions.position_open_cooldown_secs);
        let last_open_opt = LAST_OPEN_TIME.read().await.clone();
        if let Some(last_open) = last_open_opt {
            let elapsed = Utc::now().signed_duration_since(last_open).num_seconds();
            if elapsed < cooldown_secs {
                return Err(Error::TransitionFailed {
                    transition: "open",
                    mint: token_mint.to_owned(),
                    detail: format!(
                        "opening positions cooldown active: wait {}s",
                        cooldown_secs - elapsed
                    ),
                });
            }
        }
    }

    // Execute swap
    let wallet_address = get_wallet_address().map_err(|e| Error::WalletUnavailable {
        detail: e.to_string(),
    })?;

    // Mark mint as pending-open BEFORE submitting the swap to avoid duplicate attempts.
    // The guard clears the mark on every return that proves no swap can land; it is
    // kept only once the swap may be in flight.
    let mut pending_open = crate::positions::state::hold_pending_open(
        &api_token.mint,
        crate::positions::state::PENDING_OPEN_TTL_SECS,
    );
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

    // Manual override when the user set one in the trade dialog; config default otherwise.
    let slippage_quote_default = super::slippage::entry_slippage(slippage_pct);

    let quote_request = QuoteRequest {
        chain: crate::chains::active_chain(),
        input_mint: adapter().native_asset_address().to_string(),
        output_mint: api_token.mint.clone(),
        input_amount: adapter().native_to_raw(trade_size_native).into(),
        wallet_address: wallet_address.clone(),
        slippage_pct: slippage_quote_default,
        swap_mode: SwapMode::ExactIn,
        exclude_dexes: None,
    };

    let quote = get_best_quote_for_opening(quote_request, &api_token.symbol)
        .await
        .map_err(|e| Error::QuoteFailed {
            mint: api_token.mint.clone(),
            detail: e.to_string(),
        })?;

    let expected_output_amount = quote.output_amount;
    // From here the swap may reach the chain, and a buy that may land must not be
    // opened twice: the mark outlives a cancelled call and is released only on a
    // failure that proves nothing was sent.
    pending_open.keep();
    // From before the submission until the row is in memory, the wallet-history sync
    // leaves the mint to the trader: a confirmed buy shows in the wallet before then, and
    // the sync would otherwise import it as a second, wallet-derived row of the round.
    let _in_flight = mark_swap_in_flight(positions_db::get_store_chain().await?, &api_token.mint);
    let (transaction_signature, output_amount, confirmation_pending, effective_entry_price) =
        match execute_swap_with_fallback(
            &api_token,
            quote,
            crate::swaps::SwapAmountLimit::Unrestricted,
        )
        .await
        {
            Ok(result) => {
                let effective_price = effective_entry_price_native(
                    trade_size_native,
                    result.output_amount,
                    api_token.decimals,
                )
                .unwrap_or(entry_price);
                (
                    result.transaction_signature,
                    result.output_amount,
                    false,
                    effective_price,
                )
            }
            Err(error) => match crate::swaps::failed_swap(&error) {
                crate::swaps::FailedSwap::Reconcile { signature } => {
                    logger::warning(
                        LogTag::Positions,
                        &format!(
                            "Entry swap {signature} was submitted but confirmation timed out; creating the pending position and handing it to verification"
                        ),
                    );
                    let effective_price = effective_entry_price_native(
                        trade_size_native,
                        expected_output_amount,
                        api_token.decimals,
                    )
                    .unwrap_or(entry_price);
                    (signature, expected_output_amount, true, effective_price)
                }
                failure => {
                    pending_open.after_failed_swap(&failure);
                    return Err(Error::SwapFailed {
                        mint: api_token.mint.clone(),
                        detail: error.to_string(),
                        not_submitted: crate::swaps::not_submitted_reason(&error),
                    });
                }
            },
        };

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
        entry_size_native: trade_size_native,
        total_size_native: trade_size_native,
        price_highest: entry_price,
        price_lowest: entry_price,
        entry_transaction_signature: Some(transaction_signature.clone()),
        exit_transaction_signature: None,
        token_amount: None,
        effective_entry_price: None,
        effective_exit_price: None,
        native_received: None,
        profit_target_min: Some(5.0),
        profit_target_max: Some(20.0),
        liquidity_tier: Some("UNKNOWN".to_owned()),
        transaction_entry_verified: false,
        transaction_exit_verified: false,
        entry_fee_raw: None,
        exit_fee_raw: None,
        current_price: Some(entry_price),
        current_price_updated: Some(Utc::now()),
        current_price_source: None,
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
        total_exited_amount: Default::default(),
        average_exit_price: None,
        partial_exit_count: 0,
        dca_count: 0,
        average_entry_price: entry_price, // Initial entry price
        last_dca_time: None,
        // New positions are never archived
        archived: false,
        archived_at: None,
        origin,
        management,
        round_key: None,
        basis_complete: true,
        history_complete: true,
        holding_state: None,
    };

    // Save to database (with retry) and get ID
    let position_id = match super::persist_position_with_retry(&position).await {
        super::Persisted::Opened(position_id) => position_id,
        super::Persisted::Added(open_position_id) => {
            return Ok(book_entry_as_add(
                &api_token.mint,
                open_position_id,
                &transaction_signature,
                trade_size_native,
                confirmation_pending,
                effective_entry_price,
            )
            .await);
        }
        // The swap was sent, so the open reports it as submitted: an error would let the
        // caller send the buy again. No row means no verification; the slot permit is
        // returned when it drops, and the pending-open mark expires on its own.
        super::Persisted::Unsaved => {
            return Ok(EntrySubmission {
                transaction_signature,
                confirmation_pending,
                entry_price_native: effective_entry_price,
            });
        }
    };

    let mut position_with_id = position;
    position_with_id.id = Some(position_id);

    // Add to state
    add_position(position_with_id).await;
    drop(_in_flight);

    // An open position's token takes the OpenPosition priority, the fastest update cadence,
    // so its price tracking stays responsive while it is traded.
    if let Some(db) = crate::tokens::database::database(crate::chains::active_chain()) {
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

    // The slot is now held by THIS position. Registering it makes the release idempotent:
    // archive / force-close / exit-verification can all run for the same position, and only
    // the first of them may hand the permit back.
    crate::positions::state::register_position_slot(position_id).await;
    add_signature_to_index(&transaction_signature, &api_token.mint).await;

    // Record a position opened event for durability
    crate::events::record_position_event(
        &position_id.to_string(),
        &api_token.mint,
        "opened",
        Some(&transaction_signature),
        None,
        trade_size_native,
        output_amount,
        None,
        None,
    )
    .await;

    let expiry_height = crate::positions::settle::submission_expiry_bound().await;

    // Enqueue for verification
    let verification_item = VerificationItem::new(
        transaction_signature.clone(),
        api_token.mint.clone(),
        Some(position_id),
        VerificationKind::Entry,
        expiry_height,
    );

    enqueue_verification(verification_item).await;

    if confirmation_pending {
        crate::events::record_position_event_flexible(
            "entry_submitted_unconfirmed",
            crate::events::Severity::Warn,
            Some(&api_token.mint),
            Some(&transaction_signature),
            json!({ "position_id": position_id }),
        )
        .await;
    }

    // We successfully created the position; clear pending-open now
    crate::positions::state::clear_pending_open(&api_token.mint);
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

    Ok(EntrySubmission {
        transaction_signature,
        confirmation_pending,
        entry_price_native: effective_entry_price,
    })
}

/// Books a sent entry swap as an add on `position_id`, the open position its mint gained
/// while the swap was in flight, as storage named it when it refused the second row: the
/// swap is registered as that position's pending DCA and verified as one, so its tokens and
/// cost land on the one open position. Nothing here can fail after the send: a target that
/// closed meanwhile is handled when the add is verified, as a late fill on a closed
/// position. The global slot this open took is returned when its permit drops, since no
/// position was created.
async fn book_entry_as_add(
    mint: &str,
    position_id: i64,
    signature: &str,
    trade_size_native: f64,
    confirmation_pending: bool,
    entry_price_native: f64,
) -> EntrySubmission {
    logger::warning(
        LogTag::Positions,
        &format!(
            "Entry swap {signature} for {mint} is booked as an add on open position {position_id}, which the mint gained while the swap was in flight"
        ),
    );
    let expiry_height = crate::positions::settle::submission_expiry_bound().await;
    let pending = crate::positions::types::PendingDcaSwap {
        signature: signature.to_owned(),
        mint: mint.to_owned(),
        position_id,
        expiry_height,
        created_at: Utc::now(),
        size_sol: trade_size_native,
    };
    if let Err(e) = crate::positions::state::register_pending_dca_swap(pending).await {
        logger::error(
            LogTag::Positions,
            &format!(
                "Pending add {signature} for position {position_id} (mint {mint}) is held in memory only, not persisted: {e}"
            ),
        );
    }
    enqueue_verification(VerificationItem::new_dca(
        signature.to_owned(),
        mint.to_owned(),
        Some(position_id),
        expiry_height,
    ))
    .await;
    crate::events::record_position_event_flexible(
        "entry_booked_as_add",
        crate::events::Severity::Warn,
        Some(mint),
        Some(signature),
        json!({ "position_id": position_id, "size_native": trade_size_native }),
    )
    .await;
    crate::positions::state::clear_pending_open(mint);
    EntrySubmission {
        transaction_signature: signature.to_owned(),
        confirmation_pending,
        entry_price_native,
    }
}

fn effective_entry_price_native(
    input_native: f64,
    output_amount: RawAmount,
    decimals: Option<u8>,
) -> Option<f64> {
    let token_amount = output_amount.to_whole_units(decimals?);
    let price = input_native / token_amount;
    (input_native.is_finite() && input_native > 0.0 && price.is_finite() && price > 0.0)
        .then_some(price)
}

#[cfg(test)]
mod submission_price_tests {
    use super::effective_entry_price_native;
    use crate::chains::RawAmount;

    #[test]
    fn entry_submission_price_uses_the_quoted_token_amount() {
        assert_eq!(
            effective_entry_price_native(0.2, RawAmount::from(50_000_000u64), Some(6)),
            Some(0.004)
        );
        assert_eq!(
            effective_entry_price_native(0.2, RawAmount::ZERO, Some(6)),
            None
        );
        assert_eq!(
            effective_entry_price_native(0.2, RawAmount::from(50_000_000u64), None),
            None
        );
        assert_eq!(
            effective_entry_price_native(1.0, RawAmount::new(1u128 << 70), Some(0)),
            Some(1.0 / (1u128 << 70) as f64)
        );
    }
}
