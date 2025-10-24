//! Blacklist integration for safety checks

use crate::logger::{self, LogTag};
use crate::positions::Position;
use crate::trader::types::{TradeAction, TradeDecision, TradePriority, TradeReason};
use chrono::Utc;
use std::collections::HashSet;
use std::sync::OnceLock;
use tokio::sync::RwLock;

static BLACKLIST_CACHE: OnceLock<RwLock<HashSet<String>>> = OnceLock::new();

fn get_blacklist_cache() -> &'static RwLock<HashSet<String>> {
    BLACKLIST_CACHE.get_or_init(|| RwLock::new(HashSet::new()))
}

/// Initialize blacklist cache
pub async fn init_blacklist() -> Result<(), String> {
    logger::info(LogTag::Trader, "Initializing blacklist cache...");
    update_blacklist_cache().await?;
    Ok(())
}

/// Update the blacklist cache from the filtering system
async fn update_blacklist_cache() -> Result<(), String> {
    // Get blacklisted tokens from tokens module
    let blacklist = crate::tokens::get_blacklisted_tokens();
    
    let mut cache = get_blacklist_cache().write().await;
    let previous_count = cache.len();
    cache.clear();
    cache.extend(blacklist.iter().cloned());
    
    if cache.len() != previous_count {
        logger::info(
            LogTag::Trader,
            &format!(
                "🚫 Blacklist cache updated: {} tokens (was {})",
                cache.len(),
                previous_count
            ),
        );
    }
    
    Ok(())
}

/// Check if a token is blacklisted
pub async fn is_blacklisted(mint: &str) -> Result<bool, String> {
    // First check cache
    {
        let cache = get_blacklist_cache().read().await;
        if cache.contains(mint) {
            return Ok(true);
        }
    }

    // If not in cache, refresh cache and check again
    update_blacklist_cache().await?;

    let cache = get_blacklist_cache().read().await;
    Ok(cache.contains(mint))
}

/// Check if a position should be exited due to blacklist
///
/// Returns an immediate exit decision if the position's token is blacklisted.
/// This is a critical safety check that overrides all other exit conditions.
pub async fn check_blacklist_exit(
    position: &Position,
    current_price: f64,
) -> Result<Option<TradeDecision>, String> {
    // Check if token is in blacklist cache
    let is_blacklisted = {
        let cache = get_blacklist_cache().read().await;
        cache.contains(&position.mint)
    };
    
    if is_blacklisted {
        logger::info(
        LogTag::Trader,
            &format!(
                "⛔ BLACKLISTED: {} (mint={}) - Triggering emergency exit at {:.9} SOL",
                position.symbol,
                position.mint,
                current_price
            ),
        );

        return Ok(Some(TradeDecision {
            position_id: position.id.map(|id| id.to_string()),
            mint: position.mint.clone(),
            action: TradeAction::Sell,
            reason: TradeReason::Blacklisted,
            strategy_id: None,
            timestamp: Utc::now(),
            priority: TradePriority::Emergency, // Highest priority
            price_sol: Some(current_price),
            size_sol: None, // Sell entire position
        }));
    }

    Ok(None)
}

/// Refresh blacklist cache periodically
/// Should be called by the trader controller on a schedule (e.g., every 60s)
pub async fn refresh_blacklist() -> Result<(), String> {
    update_blacklist_cache().await
}
