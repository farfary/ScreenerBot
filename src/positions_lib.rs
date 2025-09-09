use crate::{
    arguments::is_debug_positions_enabled,
    logger::{ log, LogTag },
    positions::{ acquire_position_lock, MINT_TO_POSITION_INDEX, POSITIONS, SIG_TO_MINT_INDEX },
    positions_db::{
        delete_position_by_id,
        save_position,
        save_token_snapshot,
        update_position,
        TokenSnapshot,
    },
    positions_types::Position,
    rpc::lamports_to_sol,
    // Pool priority functions removed - no longer needed
    tokens::{ get_token_decimals, get_global_dexscreener_api },
    trader::PROFIT_EXTRA_NEEDED_SOL,
    utils::safe_truncate,
};
use chrono::Utc;

// ==================== INDEX MAINTENANCE HELPERS ====================

/// Add signature to mint mapping for O(1) lookups
pub async fn add_signature_to_index(signature: &str, mint: &str) {
    let mut index = SIG_TO_MINT_INDEX.write().await;
    index.insert(signature.to_string(), mint.to_string());

    if is_debug_positions_enabled() {
        log(
            LogTag::Positions,
            "DEBUG",
            &format!("📋 Added signature {} -> mint {} to index", signature, mint)
        );
    }
}

/// Remove signature from mint mapping
async fn remove_signature_from_index(signature: &str) {
    let mut index = SIG_TO_MINT_INDEX.write().await;
    index.remove(signature);
}

/// Update mint to position index (must be called when positions vector changes)
pub async fn update_mint_position_index() {
    let positions = POSITIONS.read().await;
    let mut index = MINT_TO_POSITION_INDEX.write().await;

    index.clear();
    for (idx, position) in positions.iter().enumerate() {
        index.insert(position.mint.clone(), idx);
    }
}

/// Get position index by mint (O(1) lookup)
pub async fn get_position_index_by_mint(mint: &str) -> Option<usize> {
    let index = MINT_TO_POSITION_INDEX.read().await;
    index.get(mint).copied()
}

/// Get mint by signature (O(1) lookup)
async fn get_mint_by_signature(signature: &str) -> Option<String> {
    let index = SIG_TO_MINT_INDEX.read().await;
    index.get(signature).cloned()
}

/// Find position by signature using O(1) index lookup
async fn find_position_by_signature(signature: &str) -> Option<(String, usize)> {
    // Step 1: Get mint from signature index
    let mint = get_mint_by_signature(signature).await?;

    // Step 2: Get position index from mint index
    let position_idx = get_position_index_by_mint(&mint).await?;

    Some((mint, position_idx))
}

// ==================== P&L CALCULATION ====================

/// Unified profit/loss calculation for both open and closed positions
/// Uses effective prices and actual token amounts when available
/// For closed positions with sol_received, uses actual SOL invested vs SOL received
/// NOTE: sol_received should contain ONLY the SOL from token sale, excluding ATA rent reclaim
pub async fn calculate_position_pnl(position: &Position, current_price: Option<f64>) -> (f64, f64) {
    // Safety check: validate position has valid entry price
    let entry_price = position.effective_entry_price.unwrap_or(position.entry_price);
    if entry_price <= 0.0 || !entry_price.is_finite() {
        if is_debug_positions_enabled() {
            log(
                LogTag::Positions,
                "DEBUG",
                &format!("❌ Invalid entry price for {}: {}", position.symbol, entry_price)
            );
        }
        // Invalid entry price - return neutral P&L to avoid triggering emergency exits
        return (0.0, 0.0);
    }

    // For open positions, validate current price if provided
    if let Some(current) = current_price {
        if current <= 0.0 || !current.is_finite() {
            // Invalid current price - return neutral P&L to avoid false emergency signals
            return (0.0, 0.0);
        }
    }

    // For positions with pending exit transactions (closing in progress), use current price for estimation
    if position.exit_transaction_signature.is_some() && !position.transaction_exit_verified {
        if let Some(current) = current_price {
            let entry_price = position.effective_entry_price.unwrap_or(position.entry_price);
            let entry_cost = position.entry_size_sol;

            // Calculate estimated P&L based on current price (closing in progress)
            if let Some(token_amount) = position.token_amount {
                let token_decimals_opt = get_token_decimals(&position.mint).await;
                if let Some(token_decimals) = token_decimals_opt {
                    let ui_token_amount =
                        (token_amount as f64) / (10_f64).powi(token_decimals as i32);
                    let current_value = ui_token_amount * current;

                    // Account for fees (estimated)
                    let buy_fee = position.entry_fee_lamports.map_or(0.0, |fee|
                        lamports_to_sol(fee)
                    );
                    let estimated_sell_fee = buy_fee;
                    let total_fees = buy_fee + estimated_sell_fee + PROFIT_EXTRA_NEEDED_SOL;
                    let net_pnl_sol = current_value - entry_cost - total_fees;
                    let net_pnl_percent = (net_pnl_sol / entry_cost) * 100.0;

                    return (net_pnl_sol, net_pnl_percent);
                }
            }

            // Fallback calculation for closing positions
            let price_change = (current - entry_price) / entry_price;
            let buy_fee = position.entry_fee_lamports.map_or(0.0, |fee| lamports_to_sol(fee));
            let estimated_sell_fee = buy_fee;
            let total_fees = buy_fee + estimated_sell_fee + PROFIT_EXTRA_NEEDED_SOL;
            let fee_percent = (total_fees / entry_cost) * 100.0;
            let net_pnl_percent = price_change * 100.0 - fee_percent;
            let net_pnl_sol = (net_pnl_percent / 100.0) * entry_cost;

            return (net_pnl_sol, net_pnl_percent);
        }
    }

    // For closed positions, prioritize sol_received for most accurate P&L
    if let (Some(exit_price), Some(sol_received)) = (position.exit_price, position.sol_received) {
        // Use actual SOL invested vs SOL received for closed positions
        let sol_invested = position.entry_size_sol;

        // Use actual transaction fees plus profit buffer for P&L calculation
        let buy_fee = position.entry_fee_lamports.map_or(0.0, |fee| lamports_to_sol(fee));
        let sell_fee = position.exit_fee_lamports.map_or(0.0, |fee| lamports_to_sol(fee));
        let total_fees = buy_fee + sell_fee + PROFIT_EXTRA_NEEDED_SOL; // Include profit buffer in P&L calculation

        let net_pnl_sol = sol_received - sol_invested - total_fees;
        let safe_invested = if sol_invested < 0.00001 { 0.00001 } else { sol_invested };
        let net_pnl_percent = (net_pnl_sol / safe_invested) * 100.0;

        return (net_pnl_sol, net_pnl_percent);
    }

    // Fallback for closed positions without sol_received (backward compatibility)
    if let Some(exit_price) = position.exit_price {
        let entry_price = position.effective_entry_price.unwrap_or(position.entry_price);
        let effective_exit = position.effective_exit_price.unwrap_or(exit_price);

        // For closed positions: actual transaction-based calculation
        if let Some(token_amount) = position.token_amount {
            // Get token decimals from cache (async)
            let token_decimals_opt = get_token_decimals(&position.mint).await;

            // CRITICAL: Skip P&L calculation if decimals are not available
            let token_decimals = match token_decimals_opt {
                Some(decimals) => decimals,
                None => {
                    log(
                        LogTag::Positions,
                        "ERROR",
                        &format!(
                            "Cannot calculate P&L for {} - decimals not available, skipping calculation",
                            position.mint
                        )
                    );
                    return (0.0, 0.0); // Return zero P&L instead of wrong calculation
                }
            };

            let ui_token_amount = (token_amount as f64) / (10_f64).powi(token_decimals as i32);
            let entry_cost = position.entry_size_sol;
            let exit_value = ui_token_amount * effective_exit;

            // Account for actual buy + sell fees plus profit buffer
            let buy_fee = position.entry_fee_lamports.map_or(0.0, |fee| lamports_to_sol(fee));
            let sell_fee = position.exit_fee_lamports.map_or(0.0, |fee| lamports_to_sol(fee));
            let total_fees = buy_fee + sell_fee + PROFIT_EXTRA_NEEDED_SOL; // Include profit buffer
            let net_pnl_sol = exit_value - entry_cost - total_fees;
            let net_pnl_percent = (net_pnl_sol / entry_cost) * 100.0;

            return (net_pnl_sol, net_pnl_percent);
        }

        // Fallback for closed positions without token amount
        let price_change = (effective_exit - entry_price) / entry_price;
        let buy_fee = position.entry_fee_lamports.map_or(0.0, |fee| lamports_to_sol(fee));
        let sell_fee = position.exit_fee_lamports.map_or(0.0, |fee| lamports_to_sol(fee));
        let total_fees = buy_fee + sell_fee + PROFIT_EXTRA_NEEDED_SOL; // Include profit buffer
        let fee_percent = (total_fees / position.entry_size_sol) * 100.0;
        let net_pnl_percent = price_change * 100.0 - fee_percent;
        let net_pnl_sol = (net_pnl_percent / 100.0) * position.entry_size_sol;

        return (net_pnl_sol, net_pnl_percent);
    }

    // For open positions, use current price
    if let Some(current) = current_price {
        let entry_price = position.effective_entry_price.unwrap_or(position.entry_price);

        // For open positions: current value vs entry cost
        if let Some(token_amount) = position.token_amount {
            // Get token decimals from cache (async)
            let token_decimals_opt = get_token_decimals(&position.mint).await;

            // CRITICAL: Skip P&L calculation if decimals are not available
            let token_decimals = match token_decimals_opt {
                Some(decimals) => decimals,
                None => {
                    log(
                        LogTag::Positions,
                        "ERROR",
                        &format!(
                            "Cannot calculate P&L for {} - decimals not available, skipping calculation",
                            position.mint
                        )
                    );
                    return (0.0, 0.0); // Return zero P&L instead of wrong calculation
                }
            };

            let ui_token_amount = (token_amount as f64) / (10_f64).powi(token_decimals as i32);
            let current_value = ui_token_amount * current;
            let entry_cost = position.entry_size_sol;

            // Account for actual buy fee (already paid) + estimated sell fee + profit buffer
            let buy_fee = position.entry_fee_lamports.map_or(0.0, |fee| lamports_to_sol(fee));
            let estimated_sell_fee = buy_fee; // Estimate sell fee same as buy fee
            let total_fees = buy_fee + estimated_sell_fee + PROFIT_EXTRA_NEEDED_SOL; // Include profit buffer
            let net_pnl_sol = current_value - entry_cost - total_fees;
            let net_pnl_percent = (net_pnl_sol / entry_cost) * 100.0;

            return (net_pnl_sol, net_pnl_percent);
        }

        // Fallback for open positions without token amount
        let price_change = (current - entry_price) / entry_price;
        let buy_fee = position.entry_fee_lamports.map_or(0.0, |fee| lamports_to_sol(fee));
        let estimated_sell_fee = buy_fee; // Estimate sell fee same as buy fee
        let total_fees = buy_fee + estimated_sell_fee + PROFIT_EXTRA_NEEDED_SOL; // Include profit buffer
        let fee_percent = (total_fees / position.entry_size_sol) * 100.0;
        let net_pnl_percent = price_change * 100.0 - fee_percent;
        let net_pnl_sol = (net_pnl_percent / 100.0) * position.entry_size_sol;

        return (net_pnl_sol, net_pnl_percent);
    }

    // No price available
    (0.0, 0.0)
}

/// Calculate total fees for a position
pub fn calculate_position_total_fees(position: &Position) -> f64 {
    // Sum entry and exit fees in SOL (excluding ATA rent from trading costs)
    let entry_fees_sol = (position.entry_fee_lamports.unwrap_or(0) as f64) / 1_000_000_000.0;
    let exit_fees_sol = (position.exit_fee_lamports.unwrap_or(0) as f64) / 1_000_000_000.0;
    entry_fees_sol + exit_fees_sol
}

// ==================== TOKEN SNAPSHOT FUNCTIONS ====================

/// Fetch latest token data from APIs and create a snapshot
async fn fetch_and_create_token_snapshot(
    position_id: i64,
    mint: &str,
    snapshot_type: &str
) -> Result<TokenSnapshot, String> {
    let fetch_start = Utc::now();

    if is_debug_positions_enabled() {
        log(
            LogTag::Positions,
            "SNAPSHOT_FETCH",
            &format!(
                "Fetching latest token data for {} snapshot of {}",
                snapshot_type,
                safe_truncate(mint, 8)
            )
        );
    }

    // Fetch latest data from DexScreener API
    // Fetch latest data from DexScreener API (manual lock instead of global helper)
    let dex_token = match get_global_dexscreener_api().await {
        Ok(api) => {
            match tokio::time::timeout(std::time::Duration::from_secs(5), api.lock()).await {
                Ok(mut guard) =>
                    match guard.get_token_from_mint(mint).await {
                        Ok(Some(token)) => Some(token),
                        Ok(None) => {
                            log(
                                LogTag::Positions,
                                "SNAPSHOT_NO_DEX_DATA",
                                &format!("No DexScreener data found for {}", safe_truncate(mint, 8))
                            );
                            None
                        }
                        Err(e) => {
                            log(
                                LogTag::Positions,
                                "SNAPSHOT_DEX_ERROR",
                                &format!(
                                    "Error fetching DexScreener data for {}: {}",
                                    safe_truncate(mint, 8),
                                    e
                                )
                            );
                            None
                        }
                    }
                Err(_) => {
                    log(
                        LogTag::Positions,
                        "SNAPSHOT_DEX_ERROR",
                        &format!("DexScreener API lock timeout for {}", safe_truncate(mint, 8))
                    );
                    None
                }
            }
        }
        Err(e) => {
            log(
                LogTag::Positions,
                "SNAPSHOT_DEX_ERROR",
                &format!("Failed to acquire DexScreener API: {}", e)
            );
            None
        }
    };

    // Calculate data freshness score (0-100)
    let fetch_duration_ms = Utc::now().signed_duration_since(fetch_start).num_milliseconds();
    let freshness_score = if fetch_duration_ms < 1000 {
        100 // Very fresh, under 1 second
    } else if fetch_duration_ms < 5000 {
        80 // Good, under 5 seconds
    } else if fetch_duration_ms < 10000 {
        60 // OK, under 10 seconds
    } else if fetch_duration_ms < 30000 {
        40 // Slow, under 30 seconds
    } else {
        20 // Very slow, over 30 seconds
    };

    // Extract DexScreener data
    let (
        symbol,
        name,
        price_sol,
        price_usd,
        price_native,
        dex_id,
        pair_address,
        pair_url,
        fdv,
        market_cap,
        pair_created_at,
        liquidity_usd,
        liquidity_base,
        liquidity_quote,
        volume_h24,
        volume_h6,
        volume_h1,
        volume_m5,
        txns_h24_buys,
        txns_h24_sells,
        txns_h6_buys,
        txns_h6_sells,
        txns_h1_buys,
        txns_h1_sells,
        txns_m5_buys,
        txns_m5_sells,
        price_change_h24,
        price_change_h6,
        price_change_h1,
        price_change_m5,
    ) = if let Some(ref token) = dex_token {
        (
            Some(token.symbol.clone()),
            Some(token.name.clone()),
            token.price_dexscreener_sol,
            token.price_dexscreener_usd,
            token.price_dexscreener_sol, // Use SOL price as native
            token.dex_id.clone(),
            token.pair_address.clone(),
            token.pair_url.clone(),
            token.fdv,
            token.market_cap,
            token.created_at.map(|dt| dt.timestamp()),
            token.liquidity.as_ref().and_then(|l| l.usd),
            token.liquidity.as_ref().and_then(|l| l.base),
            token.liquidity.as_ref().and_then(|l| l.quote),
            token.volume.as_ref().and_then(|v| v.h24),
            token.volume.as_ref().and_then(|v| v.h6),
            token.volume.as_ref().and_then(|v| v.h1),
            token.volume.as_ref().and_then(|v| v.m5),
            token.txns.as_ref().and_then(|t| t.h24.as_ref().and_then(|h| h.buys)),
            token.txns.as_ref().and_then(|t| t.h24.as_ref().and_then(|h| h.sells)),
            token.txns.as_ref().and_then(|t| t.h6.as_ref().and_then(|h| h.buys)),
            token.txns.as_ref().and_then(|t| t.h6.as_ref().and_then(|h| h.sells)),
            token.txns.as_ref().and_then(|t| t.h1.as_ref().and_then(|h| h.buys)),
            token.txns.as_ref().and_then(|t| t.h1.as_ref().and_then(|h| h.sells)),
            token.txns.as_ref().and_then(|t| t.m5.as_ref().and_then(|h| h.buys)),
            token.txns.as_ref().and_then(|t| t.m5.as_ref().and_then(|h| h.sells)),
            token.price_change.as_ref().and_then(|pc| pc.h24),
            token.price_change.as_ref().and_then(|pc| pc.h6),
            token.price_change.as_ref().and_then(|pc| pc.h1),
            token.price_change.as_ref().and_then(|pc| pc.m5),
        )
    } else {
        (
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
    };

    // Create snapshot
    let snapshot = TokenSnapshot {
        id: None,
        position_id,
        snapshot_type: snapshot_type.to_string(),
        mint: mint.to_string(),
        symbol,
        name,
        price_sol,
        price_usd,
        price_native,
        dex_id,
        pair_address,
        pair_url,
        fdv,
        market_cap,
        pair_created_at,
        liquidity_usd,
        liquidity_base,
        liquidity_quote,
        volume_h24,
        volume_h6,
        volume_h1,
        volume_m5,
        txns_h24_buys,
        txns_h24_sells,
        txns_h6_buys,
        txns_h6_sells,
        txns_h1_buys,
        txns_h1_sells,
        txns_m5_buys,
        txns_m5_sells,
        price_change_h24,
        price_change_h6,
        price_change_h1,
        price_change_m5,
        rugcheck_score: None,
        rugcheck_score_normalised: None,
        rugcheck_rugged: None,
        rugcheck_risks_json: None,
        rugcheck_mint_authority: None,
        rugcheck_freeze_authority: None,
        rugcheck_creator: None,
        rugcheck_creator_balance: None,
        rugcheck_total_holders: None,
        rugcheck_total_market_liquidity: None,
        rugcheck_total_stable_liquidity: None,
        rugcheck_total_lp_providers: None,
        rugcheck_lp_locked_pct: None,
        rugcheck_lp_locked_usd: None,
        rugcheck_transfer_fee_pct: None,
        rugcheck_transfer_fee_max_amount: None,
        rugcheck_jup_verified: None,
        rugcheck_jup_strict: None,
        token_uri: None,
        token_description: None,
        token_image: None,
        token_website: None,
        token_twitter: None,
        token_telegram: None,
        snapshot_time: Utc::now(),
        api_fetch_time: fetch_start,
        data_freshness_score: freshness_score,
    };

    log(
        LogTag::Positions,
        "SNAPSHOT_CREATED",
        &format!(
            "Created {} snapshot for {} - freshness: {}/100, price: {:?} SOL",
            snapshot_type,
            safe_truncate(mint, 8),
            freshness_score,
            price_sol
        )
    );

    Ok(snapshot)
}

/// Save token snapshot for a position
pub async fn save_position_token_snapshot(
    position_id: i64,
    mint: &str,
    snapshot_type: &str
) -> Result<(), String> {
    let _lock = acquire_position_lock(mint).await;

    // Fetch and create snapshot
    let snapshot = fetch_and_create_token_snapshot(position_id, mint, snapshot_type).await?;

    // Save to database
    match save_token_snapshot(&snapshot).await {
        Ok(snapshot_id) => {
            log(
                LogTag::Positions,
                "SNAPSHOT_SAVED",
                &format!(
                    "Saved {} snapshot for {} with ID {}",
                    snapshot_type,
                    safe_truncate(mint, 8),
                    snapshot_id
                )
            );
            Ok(())
        }
        Err(e) => {
            log(
                LogTag::Positions,
                "SNAPSHOT_SAVE_ERROR",
                &format!(
                    "Failed to save {} snapshot for {}: {}",
                    snapshot_type,
                    safe_truncate(mint, 8),
                    e
                )
            );
            Err(e)
        }
    }
}

// ==================== DATABASE SYNC HELPERS ====================

/// Remove a position by its transaction signature (for cleanup of failed positions)
pub async fn remove_position_by_signature(signature: &str) -> Result<(), String> {
    log(
        LogTag::Positions,
        "CLEANUP_START",
        &format!("🗑️ Starting cleanup of position with signature {}", signature)
    );

    // Find mint first, then acquire lock
    let mint_for_lock = {
        let positions = POSITIONS.read().await;
        positions
            .iter()
            .find(|p| {
                p.entry_transaction_signature.as_ref().map(|s| s.as_str()) == Some(signature) ||
                    p.exit_transaction_signature.as_ref().map(|s| s.as_str()) == Some(signature)
            })
            .map(|p| p.mint.clone())
    };

    let mint_for_lock = match mint_for_lock {
        Some(mint) => mint,
        None => {
            log(
                LogTag::Positions,
                "CLEANUP_NOT_FOUND",
                &format!("⚠️ No position found with signature {}", signature)
            );
            return Ok(());
        }
    };

    let _lock = acquire_position_lock(&mint_for_lock).await;

    let position_to_remove = {
        let mut positions = POSITIONS.write().await;

        // Find position with matching entry or exit signature
        let position_index = positions
            .iter()
            .position(|p| {
                p.entry_transaction_signature.as_ref().map(|s| s.as_str()) == Some(signature) ||
                    p.exit_transaction_signature.as_ref().map(|s| s.as_str()) == Some(signature)
            });

        if let Some(index) = position_index {
            let position = positions.remove(index);

            // Update indexes after removal
            {
                let mut sig_to_mint = SIG_TO_MINT_INDEX.write().await;
                let mut mint_to_position = MINT_TO_POSITION_INDEX.write().await;

                // Remove signature mappings for this position
                if let Some(ref entry_sig) = position.entry_transaction_signature {
                    sig_to_mint.remove(entry_sig);
                }
                if let Some(ref exit_sig) = position.exit_transaction_signature {
                    sig_to_mint.remove(exit_sig);
                }
                mint_to_position.remove(&position.mint);

                // Rebuild position index mapping since positions shifted
                mint_to_position.clear();
                for (new_index, pos) in positions.iter().enumerate() {
                    mint_to_position.insert(pos.mint.clone(), new_index);
                }
            }

            log(
                LogTag::Positions,
                "CLEANUP_REMOVED",
                &format!(
                    "🗑️ Removed position {} from memory (signature: {})",
                    position.symbol,
                    signature
                )
            );

            // Remove token from priority pool service since position is being cleaned up
            let mint_for_cleanup = position.mint.clone();
            tokio::spawn(async move {
                // Priority token management removed - no longer needed
            });

            Some(position)
        } else {
            log(
                LogTag::Positions,
                "CLEANUP_NOT_FOUND",
                &format!("⚠️ No position found with signature {}", signature)
            );
            None
        }
    };

    // Remove from database if position had an ID
    if let Some(position) = position_to_remove {
        if let Some(position_id) = position.id {
            match crate::positions_db::delete_position_by_id(position_id).await {
                Ok(_) => {
                    log(
                        LogTag::Positions,
                        "CLEANUP_DB_SUCCESS",
                        &format!(
                            "🗑️ Removed position {} (ID: {}) from database",
                            position.symbol,
                            position_id
                        )
                    );
                }
                Err(e) => {
                    log(
                        LogTag::Positions,
                        "CLEANUP_DB_ERROR",
                        &format!(
                            "❌ Failed to remove position {} (ID: {}) from database: {}",
                            position.symbol,
                            position_id,
                            e
                        )
                    );
                    return Err(format!("Database cleanup failed: {}", e));
                }
            }
        }

        log(
            LogTag::Positions,
            "CLEANUP_COMPLETE",
            &format!(
                "✅ Successfully cleaned up failed position {} with signature {}",
                position.symbol,
                signature
            )
        );
    }

    Ok(())
}

/// Sync a position between memory and database
pub async fn sync_position_to_database(position: &Position) -> Result<(), String> {
    let _lock = acquire_position_lock(&position.mint).await;

    if let Some(_position_id) = position.id {
        // Update existing position
        update_position(position).await
    } else {
        // Insert new position
        let new_id = save_position(position).await?;
        log(
            LogTag::Positions,
            "DB_SYNC",
            &format!("Position synced to database with new ID {}", new_id)
        );
        Ok(())
    }
}
