/// Rugcheck security data fetching and caching
///
/// Flow: API -> Parse -> Database -> Store cache
/// Updates: Every 30 minutes (security data is relatively stable)
use crate::apis::rugcheck::RugcheckInfo;
use crate::events::record_security_event;
use crate::logger::{self, LogTag};
use crate::tokens::database::TokenDatabase;
use crate::tokens::store::{self, CacheMetrics};
use crate::tokens::types::{RugcheckData, TokenError, TokenResult};
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
        security_data_last_fetched_at: Utc::now(),
        security_data_first_fetched_at: Utc::now(),
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
    // Check connectivity before API call - fallback to cache/DB if unhealthy
    let connectivity_ok = crate::connectivity::check_endpoints_healthy(&["rugcheck"])
        .await
        .is_none();

    if !connectivity_ok {
        logger::debug(
            LogTag::Tokens,
            &format!(
                "Rugcheck endpoint unhealthy for {} - using cached/DB data only",
                mint
            ),
        );
    }

    // 1. Check cache first (fastest path)
    if let Some(data) = store::get_cached_rugcheck(mint) {
        return Ok(Some(data));
    }

    // 2. Check database (if recently updated, use it)
    if let Some(db_data) = db.get_rugcheck_data(mint)? {
        // If data is fresh (< 30min old), use it
        let age = Utc::now()
            .signed_duration_since(db_data.security_data_last_fetched_at)
            .num_seconds();

        if age < 1800 {
            // 30 minutes
            store::store_rugcheck(mint, &db_data);
            if let Err(err) = store::refresh_token_snapshot(mint).await {
                logger::error(
                    LogTag::Tokens,
                    &format!(
                        "[TOKENS][STORE] Failed to refresh token snapshot after Rugcheck DB hit mint={} err={:?}",
                        mint,
                        err
                    ),
                );
            }
            return Ok(Some(db_data));
        }
    }

    // Skip API fetch if connectivity is down - return what we have from DB or None
    if !connectivity_ok {
        logger::debug(
            LogTag::Tokens,
            &format!(
                "Skipping Rugcheck API fetch for {} - connectivity issue",
                mint
            ),
        );
        // Return stale DB data if available, otherwise None
        return Ok(db.get_rugcheck_data(mint)?);
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

    // Cache it in store and refresh token snapshot
    store::store_rugcheck(mint, &data);
    if let Err(err) = store::refresh_token_snapshot(mint).await {
        logger::error(
            LogTag::Tokens,
            &format!(
                "[TOKENS][STORE] Failed to refresh token snapshot after Rugcheck API mint={} err={:?}",
                mint, err
            ),
        );
    }

    // Record security analysis event (sampled - only for high-risk tokens or every 20th)
    let score = calculate_security_score(&data);
    let is_high_risk = score < 50 || data.rugged;
    let hash = mint.chars().fold(0u32, |acc, c| acc.wrapping_add(c as u32));
    if is_high_risk || hash % 20 == 0 {
        tokio::spawn({
            let mint = mint.to_string();
            let risks = data.risks.clone();
            let rugged = data.rugged;
            async move {
                let risk_level = if rugged {
                    "critical"
                } else if is_high_risk {
                    "high"
                } else {
                    "low"
                };
                record_security_event(
                    &mint,
                    "rugcheck_analysis",
                    risk_level,
                    serde_json::json!({
                        "score": score,
                        "rugged": rugged,
                        "risk_count": risks.len(),
                        "high_risk": is_high_risk,
                    }),
                )
                .await;
            }
        });
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
