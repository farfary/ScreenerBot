//! Position helpers — utility functions for index maintenance, snapshots, and database sync.

use crate::{
    logger::{self, LogTag},
    positions::{
        acquire_position_lock, delete_position_by_id, save_position, save_token_snapshot,
        update_position, Position, TokenSnapshot, MINT_TO_POSITION_INDEX, POSITIONS,
        SIG_TO_MINT_INDEX,
    },
};
use chrono::Utc;

pub use super::pnl::{
    calculate_position_pnl, calculate_position_pnl_safe, calculate_position_total_fees,
    calculate_split_pnl,
};

// ==================== INDEX MAINTENANCE HELPERS ====================

/// Add signature to mint mapping for O(1) lookups
pub async fn add_signature_to_index(signature: &str, mint: &str) {
    let mut index = SIG_TO_MINT_INDEX.write().await;
    index.insert(signature.to_string(), mint.to_string());

    logger::debug(
        LogTag::Positions,
        &format!("Added signature {signature} -> mint {mint} to index"),
    );
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

// ==================== TOKEN SNAPSHOT FUNCTIONS ====================

/// Fetch latest token data from store and create a snapshot (no direct API calls)
async fn fetch_and_create_token_snapshot(
    position_id: i64,
    mint: &str,
    snapshot_type: &str,
) -> Result<TokenSnapshot, String> {
    // Read token from the unified tokens store
    let token = crate::tokens::get_full_token_async(mint)
        .await
        .map_err(|e| format!("Failed to get token: {e}"))?
        .ok_or_else(|| format!("Token not found in store: {mint}"))?;

    // Compute freshness based on last price update vs now
    let now = Utc::now();
    let last_update = token.pool_price_last_calculated_at;
    let age_ms = now.signed_duration_since(last_update).num_milliseconds();
    let freshness_score = if age_ms < 30_000 {
        100 // < 30s
    } else if age_ms < 60_000 {
        80 // < 60s
    } else if age_ms < 300_000 {
        60 // < 5m
    } else if age_ms < 900_000 {
        40 // < 15m
    } else {
        20 // stale
    };

    // Map Token -> TokenSnapshot (Dex-like fields best-effort)
    let symbol = Some(token.symbol.clone());
    let name = Some(token.name.clone());
    let price_sol = Some(token.price_sol);
    let price_usd = Some(token.price_usd);
    let price_native = token.price_native.parse::<f64>().ok();
    let dex_id = Some(token.data_source.as_str().to_owned());
    let pair_address = None;
    let pair_url = None;
    let fdv = token.fdv;
    let market_cap = token.market_cap;
    let pair_created_at = None;
    let liquidity_usd = token.liquidity_usd;
    let liquidity_base = None;
    let liquidity_quote = None;
    let volume_h24 = token.volume_h24;
    let volume_h6 = token.volume_h6;
    let volume_h1 = token.volume_h1;
    let volume_m5 = token.volume_m5;
    let txns_h24_buys = token.txns_h24_buys;
    let txns_h24_sells = token.txns_h24_sells;
    let txns_h6_buys = token.txns_h6_buys;
    let txns_h6_sells = token.txns_h6_sells;
    let txns_h1_buys = token.txns_h1_buys;
    let txns_h1_sells = token.txns_h1_sells;
    let txns_m5_buys = token.txns_m5_buys;
    let txns_m5_sells = token.txns_m5_sells;
    let price_change_h24 = token.price_change_h24;
    let price_change_h6 = token.price_change_h6;
    let price_change_h1 = token.price_change_h1;
    let price_change_m5 = token.price_change_m5;

    // Token meta links
    let token_description = token.description.clone();
    let token_image = token.image_url.clone();
    let token_website = token.websites.first().map(|w| w.url.clone());
    let token_twitter = token
        .socials
        .iter()
        .find(|s| s.link_type.to_lowercase().contains("twitter"))
        .map(|s| s.url.clone());
    let token_telegram = token
        .socials
        .iter()
        .find(|s| s.link_type.to_lowercase().contains("telegram"))
        .map(|s| s.url.clone());

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
        token_uri: None,
        token_description,
        token_image,
        token_website,
        token_twitter,
        token_telegram,
        snapshot_time: now,
        api_fetch_time: token.market_data_last_fetched_at,
        data_freshness_score: freshness_score,
    };

    logger::info(
        LogTag::Positions,
        &format!(
            "Created {} snapshot for {} from store (freshness: {}/100, price_sol: {:?})",
            snapshot_type, mint, freshness_score, price_sol
        ),
    );

    Ok(snapshot)
}

/// Save token snapshot for a position
pub async fn save_position_token_snapshot(
    position_id: i64,
    mint: &str,
    snapshot_type: &str,
) -> Result<(), String> {
    let _lock = acquire_position_lock(mint).await;

    // Fetch and create snapshot
    let snapshot = fetch_and_create_token_snapshot(position_id, mint, snapshot_type).await?;

    // Save to database
    match save_token_snapshot(&snapshot).await {
        Ok(snapshot_id) => {
            logger::info(
                LogTag::Positions,
                &format!(
                    "Saved {} snapshot for {} with ID {}",
                    snapshot_type, mint, snapshot_id
                ),
            );
            Ok(())
        }
        Err(e) => {
            logger::error(
                LogTag::Positions,
                &format!(
                    "Failed to save {} snapshot for {}: {}",
                    snapshot_type, mint, e
                ),
            );
            Err(e)
        }
    }
}

// ==================== DATABASE SYNC HELPERS ====================

/// Remove a position by its transaction signature (for cleanup of failed positions)
pub async fn remove_position_by_signature(signature: &str) -> Result<(), String> {
    logger::info(
        LogTag::Positions,
        &format!("Starting cleanup of position with signature {signature}"),
    );

    // Find mint first, then acquire lock
    let mint_for_lock = {
        let positions = POSITIONS.read().await;
        positions
            .iter()
            .find(|p| {
                p.entry_transaction_signature.as_deref() == Some(signature)
                    || p.exit_transaction_signature.as_deref() == Some(signature)
            })
            .map(|p| p.mint.clone())
    };

    let mint_for_lock = match mint_for_lock {
        Some(mint) => mint.clone(),
        None => {
            logger::warning(
                LogTag::Positions,
                &format!("No position found with signature {signature}"),
            );
            return Ok(());
        }
    };

    let _lock = acquire_position_lock(&mint_for_lock).await;

    let position_to_remove = {
        let mut positions = POSITIONS.write().await;

        // Find position with matching entry or exit signature
        let position_index = positions.iter().position(|p| {
            p.entry_transaction_signature.as_deref() == Some(signature)
                || p.exit_transaction_signature.as_deref() == Some(signature)
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

            logger::info(
                LogTag::Positions,
                &format!(
                    "Removed position {} from memory (signature: {})",
                    position.symbol, signature
                ),
            );

            Some(position)
        } else {
            logger::warning(
                LogTag::Positions,
                &format!("No position found with signature {signature}"),
            );
            None
        }
    };

    // Remove from database if position had an ID
    if let Some(position) = position_to_remove {
        if let Some(position_id) = position.id {
            match delete_position_by_id(position_id).await {
                Ok(_) => {
                    logger::info(
                        LogTag::Positions,
                        &format!(
                            "Removed position {} (ID: {}) from database",
                            position.symbol, position_id
                        ),
                    );
                }
                Err(e) => {
                    logger::error(
                        LogTag::Positions,
                        &format!(
                            "Failed to remove position {} (ID: {}) from database: {}",
                            position.symbol, position_id, e
                        ),
                    );
                    return Err(format!("Database cleanup failed: {e}"));
                }
            }
        }

        logger::info(
            LogTag::Positions,
            &format!(
                "Successfully cleaned up failed position {} with signature {}",
                position.symbol, signature
            ),
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
        logger::info(
            LogTag::Positions,
            &format!("Position synced to database with new ID {new_id}"),
        );
        Ok(())
    }
}
