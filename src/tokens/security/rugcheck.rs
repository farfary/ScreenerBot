/// Rugcheck security data fetching and caching
///
/// Flow: API -> Parse -> Database -> Store cache
/// Updates: Every 30 minutes (security data is relatively stable)
use crate::apis::rugcheck::RugcheckInfo;
use crate::tokens::database::TokenDatabase;
use crate::tokens::store::{self, CacheMetrics};
use crate::tokens::types::{ApiError as TokenApiError, RugcheckData, TokenError, TokenResult};
use chrono::Utc;

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

    let creator_pct_from_holders = info.top_holders.iter().find_map(|holder| {
        if holder
            .owner
            .as_ref()
            .map(|owner| owner.eq_ignore_ascii_case("creator"))
            .unwrap_or(false)
        {
            return Some(holder.pct);
        }

        if let Some(creator_address) = info.creator.as_ref() {
            if creator_address == &holder.address {
                return Some(holder.pct);
            }
        }

        None
    });

    let creator_pct = creator_pct_from_holders.or_else(|| {
        let balance = info.creator_balance? as f64;
        let supply = info
            .token_supply
            .as_ref()
            .and_then(|value| value.parse::<f64>().ok())?;
        if supply > 0.0 {
            Some((balance / supply) * 100.0)
        } else {
            None
        }
    });

    RugcheckData {
        token_type: info.token_type.clone(),
        token_decimals: info.token_decimals,
        score: info.score,
        score_description: None,
        mint_authority: info.mint_authority.clone(),
        freeze_authority: info.freeze_authority.clone(),
        top_10_holders_pct: top_pct,
        total_holders: info.total_holders,
        total_lp_providers: info.total_lp_providers,
        graph_insiders_detected: info.graph_insiders_detected,
        total_market_liquidity: info.total_market_liquidity,
        total_stable_liquidity: info.total_stable_liquidity,
        total_supply: info.token_supply.clone(),
        creator_balance_pct: creator_pct,
        transfer_fee_pct: info.transfer_fee_pct,
        transfer_fee_max_amount: info.transfer_fee_max_amount,
        transfer_fee_authority: info.transfer_fee_authority.clone(),
        rugged: info.rugged,
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
    // Short-circuit if this mint is marked as security skip (e.g., 400/404 not found)
    if db.is_security_skip(mint)? {
        return Ok(None);
    }
    // 1. Check in-memory store cache
    if let Some(data) = store::get_cached_rugcheck(mint) {
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
            store::store_rugcheck(mint, &db_data);
            if let Err(err) = store::refresh_token_snapshot(mint).await {
                eprintln!(
                    "[TOKENS][STORE] Failed to refresh token snapshot after Rugcheck DB hit mint={} err={:?}",
                    mint,
                    err
                );
            }
            return Ok(Some(db_data));
        }
    }

    // 3. Fetch from API
    let api_manager = crate::apis::manager::get_api_manager();
    let rugcheck_info = match api_manager.rugcheck.fetch_report(mint).await {
        Ok(info) => info,
        Err(e) => {
            // Handle terminal not-found cases: 404 NotFound or 400 with a not-found style body
            match e {
                TokenApiError::NotFound => {
                    if let Err(perr) = db.mark_security_skip(mint, "rugcheck_not_found_404", "Rugcheck") {
                        eprintln!(
                            "[TOKENS][SECURITY] Failed to mark security skip mint={} reason={} err={}",
                            mint, "rugcheck_not_found_404", perr
                        );
                    }
                    return Ok(None);
                }
                TokenApiError::InvalidResponse(msg) => {
                    let lower = msg.to_ascii_lowercase();
                    let is_400 = lower.contains("http 400");
                    let mentions_not_found = lower.contains("not found")
                        || lower.contains("no analysis")
                        || lower.contains("no token analysis")
                        || lower.contains("unknown token")
                        || lower.contains("does not exist")
                        || lower.contains("not analyzed");
                    if is_400 && mentions_not_found {
                        if let Err(perr) = db.mark_security_skip(mint, "rugcheck_not_found_400", "Rugcheck") {
                            eprintln!(
                                "[TOKENS][SECURITY] Failed to mark security skip mint={} reason={} err={}",
                                mint, "rugcheck_not_found_400", perr
                            );
                        }
                        return Ok(None);
                    }
                    return Err(TokenError::Api { source: "Rugcheck".to_string(), message: msg });
                }
                other => {
                    let err_str = other.to_string();
                    return Err(TokenError::Api { source: "Rugcheck".to_string(), message: err_str });
                }
            }
        }
    };

    let data = convert_rugcheck_to_data(&rugcheck_info);

    // Store in database
    db.upsert_rugcheck_data(mint, &data)?;

    // Record last security update in tracking (best-effort)
    if let Err(perr) = db.record_security_update(mint) {
        eprintln!(
            "[TOKENS][SECURITY] Failed to record security update mint={} err={}",
            mint, perr
        );
    }

    // Cache it in store and refresh token snapshot
    store::store_rugcheck(mint, &data);
    if let Err(err) = store::refresh_token_snapshot(mint).await {
        eprintln!(
            "[TOKENS][STORE] Failed to refresh token snapshot after Rugcheck API mint={} err={:?}",
            mint, err
        );
    }

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
    store::rugcheck_cache_metrics()
}

/// Return current cache size
pub fn get_cache_size() -> usize {
    store::rugcheck_cache_size()
}
