//! Blacklist integration for safety checks

use crate::logger::{log, LogTag};
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
    log(LogTag::Trader, "INFO", "Initializing blacklist cache...");
    update_blacklist_cache().await?;
    Ok(())
}

/// Update the blacklist cache from the filtering system
async fn update_blacklist_cache() -> Result<(), String> {
    // TODO: CRITICAL - Integrate with filtering module
    // 
    // This function should call:
    //   let blacklist = crate::filtering::get_blacklisted_tokens().await?;
    //
    // CURRENT STATUS: Returns empty list - blacklist checks are NON-FUNCTIONAL
    // IMPACT: Dangerous tokens are never force-exited, emergency exit logic never triggers
    // PRIORITY: P0 - Implement before production use
    // 
    // Implementation steps:
    // 1. Add get_blacklisted_tokens() to filtering module
    // 2. Return Vec<String> of blacklisted mint addresses
    // 3. Replace empty Vec below with actual filtering call
    // 4. Add periodic cache refresh (every 60s recommended)
    
    let blacklist: Vec<String> = Vec::new(); // ⚠️ STUB - Always empty!
    
    let mut cache = get_blacklist_cache().write().await;
    cache.clear();
    cache.extend(blacklist.into_iter());
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
pub async fn check_blacklist_exit(
    position: &Position,
    current_price: f64,
) -> Result<Option<TradeDecision>, String> {
    if is_blacklisted(&position.mint).await? {
        log(
            LogTag::Trader,
            "WARN",
            &format!(
                "Position {:?} for token {} is blacklisted - triggering emergency exit",
                position.id, position.mint
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
