/// Rugcheck security data fetching and caching
///
/// Flow: API -> Parse -> Database -> Cache
/// Updates: Every 30 minutes (security data is relatively stable)
use crate::apis::rugcheck::RugcheckInfo;
use crate::cache::manager::CacheMetrics;
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
    let top_pct = if info.top_holders.is_empty() {
        None
    } else {
        Some(
            info.top_holders
                .iter()
                .take(10)
                .map(|holder| holder.pct)
                .sum(),
        )
    };

    RugcheckData {
        token_type: info.token_type.clone(),
        score: info.score,
        score_description: None,
        mint_authority: info.mint_authority.clone(),
        freeze_authority: info.freeze_authority.clone(),
        top_10_holders_pct: top_pct,
        total_supply: info.token_supply.clone(),
        risks: info.risks.clone(),
        top_holders: info.top_holders.clone(),
        markets: None,
        fetched_at: Utc::now(),
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
            .signed_duration_since(db_data.fetched_at)
            .num_seconds();

        if age < 1800 {
            // 30 minutes
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
    data.score.unwrap_or(50).clamp(0, 100)
}

/// Get cache metrics for monitoring
pub fn get_cache_metrics() -> CacheMetrics {
    get_cache().metrics()
}

/// Return current cache size
pub fn get_cache_size() -> usize {
    get_cache().len()
}
