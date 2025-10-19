/// Rugcheck security data fetching and caching
/// 
/// Flow: API -> Parse -> Database -> Cache
/// Updates: Every 30 minutes (security data is relatively stable)

use crate::apis::rugcheck::RugcheckInfo;
use crate::cache::{CacheConfig, CacheManager};
use crate::tokens::database::TokenDatabase;
use crate::tokens::types::{RugcheckData, TokenError, TokenResult};
use chrono::Utc;
use once_cell::sync::OnceCell;
use std::sync::Arc;

// Global cache instance (TTL = 30min)
static RUGCHECK_CACHE: OnceCell<Arc<CacheManager<String, RugcheckData>>> = OnceCell::new();

/// Get or initialize Rugcheck cache
fn get_cache() -> Arc<CacheManager<String, RugcheckData>> {
    RUGCHECK_CACHE
        .get_or_init(|| {
            let config = CacheConfig::security_rugcheck(); // 30min TTL
            Arc::new(CacheManager::new(config))
        })
        .clone()
}

/// Convert API rugcheck info to our RugcheckData type
fn convert_rugcheck_to_data(info: &RugcheckInfo) -> RugcheckData {
    RugcheckData {
        mint: info.mint.clone(),
        
        token_program: info.token_program.clone(),
        token_type: info.token_type.clone(),
        
        mint_authority: info.mint_authority.clone(),
        freeze_authority: info.freeze_authority.clone(),
        
        score: info.score,
        score_normalised: info.score_normalised,
        rugged: info.rugged,
        
        // Serialize complex types to JSON for database storage
        risks: serde_json::to_string(&info.risks).ok(),
        top_holders: serde_json::to_string(&info.top_holders).ok(),
        
        total_markets: info.total_markets,
        total_market_liquidity: info.total_market_liquidity,
        total_stable_liquidity: info.total_stable_liquidity,
        total_lp_providers: info.total_lp_providers,
        
        total_holders: info.total_holders,
        
        transfer_fee_pct: info.transfer_fee_pct,
        transfer_fee_max_amount: info.transfer_fee_max_amount,
        transfer_fee_authority: info.transfer_fee_authority.clone(),
        
        updated_at: Utc::now(),
    }
}

/// Fetch Rugcheck security data for a token (with cache + database)
/// 
/// Flow:
/// 1. Check cache (if fresh, return immediately)
/// 2. Check database (if fresh, cache + return)
/// 3. Fetch from API (store in database + cache + return)
/// 
/// # Arguments
/// * `mint` - Token mint address
/// * `db` - Database instance
/// 
/// # Returns
/// RugcheckData if analysis available, None if token not analyzed
pub async fn fetch_rugcheck_data(
    mint: &str,
    db: &TokenDatabase,
) -> TokenResult<Option<RugcheckData>> {
    let cache = get_cache();
    
    // 1. Check cache
    if let Some(data) = cache.get(&mint.to_string()) {
        return Ok(Some(data));
    }
    
    // 2. Check database (if recently updated, use it)
    if let Some(db_data) = db.get_rugcheck_data(mint)? {
        // If data is fresh (< 30min old), use it
        let age = Utc::now()
            .signed_duration_since(db_data.updated_at)
            .num_seconds();
        
        if age < 1800 { // 30 minutes
            cache.insert(mint.to_string(), db_data.clone());
            return Ok(Some(db_data));
        }
    }
    
    // 3. Fetch from API
    let api_manager = crate::apis::manager::get_api_manager();
    let rugcheck_info = match api_manager.rugcheck.fetch_report(mint).await {
        Ok(info) => info,
        Err(e) => {
            // Check if it's a "not found" error (token not analyzed yet)
            let err_str = format!("{:?}", e);
            if err_str.contains("404") || err_str.contains("NotFound") {
                return Ok(None);
            }
            
            // Other errors
            return Err(TokenError::Api {
                source: "Rugcheck".to_string(),
                message: err_str,
            });
        }
    };
    
    let data = convert_rugcheck_to_data(&rugcheck_info);
    
    // Store in database
    db.upsert_rugcheck_data(mint, &data)?;
    
    // Cache it
    cache.insert(mint.to_string(), data.clone());
    
    Ok(Some(data))
}

/// Calculate security score from Rugcheck data
/// 
/// Returns a 0-100 score where:
/// - 80-100: Safe (green)
/// - 50-79: Medium (yellow)
/// - 20-49: Risky (orange)
/// - 0-19: Dangerous (red)
pub fn calculate_security_score(data: &RugcheckData) -> i32 {
    // Use normalized score if available (0-100 scale)
    if let Some(score) = data.score_normalised {
        return score.max(0).min(100);
    }
    
    // Fallback: use raw score (typically 0-1000) and normalize
    if let Some(score) = data.score {
        return ((score as f64 / 1000.0) * 100.0) as i32;
    }
    
    // No score available
    50 // Default to medium
}

/// Get cache metrics for monitoring
pub fn get_cache_metrics() -> String {
    let cache = get_cache();
    let metrics = cache.metrics();
    format!(
        "Rugcheck cache: {} entries, {:.2}% hit rate ({} hits, {} misses)",
        cache.len(),
        metrics.hit_rate() * 100.0,
        metrics.hits,
        metrics.misses
    )
}
