//! Token detail, analysis, and refresh handlers

use axum::{extract::Path, http::StatusCode, Json};
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

use super::types::*;
use crate::{
    logger::{self, LogTag},
    pools, positions,
    sol_price::get_sol_price,
    tokens::database::get_global_database,
};

/// GET /api/tokens/:mint
///
/// Get detailed information about a specific token
pub async fn get_token_detail(Path(mint): Path<String>) -> Json<TokenDetailResponse> {
    let request_start = std::time::Instant::now();

    logger::debug(LogTag::Webserver, &format!("mint={mint}"));

    // Fetch token from database (with market data)
    let lookup_start = std::time::Instant::now();
    let snapshot = match crate::tokens::get_full_token_async(&mint).await {
        Ok(Some(snap)) => Some(snap),
        // Not in our DB yet — fetch from external APIs (DexScreener/Jupiter) and
        // add it, so opening a token we have not tracked (e.g. from the billboard
        // or search) shows real data instead of an empty NOT_FOUND stub. Mirrors
        // get_token_analysis.
        Ok(None) => fetch_and_add_token_from_external(&mint).await,
        Err(_) => None,
    };
    logger::debug(
        LogTag::Webserver,
        &format!(
            "mint={} elapsed={}μs",
            mint,
            lookup_start.elapsed().as_micros()
        ),
    );

    let snapshot = match snapshot {
        Some(snap) => snap,
        None => {
            return Json(TokenDetailResponse {
                mint: mint.clone(),
                symbol: "NOT_FOUND".to_owned(),
                name: Some("Token not in cache".to_owned()),
                description: None,
                logo_url: None,
                header_image_url: None,
                open_graph_image: None,
                website: None,
                data_source: None,
                verified: false,
                tags: vec![],
                pair_labels: vec![],
                decimals: None,
                created_at: None,
                market_data_last_fetched_at: None,
                pool_price_last_calculated_at: None,
                pair_created_at: None,
                pair_url: None,
                boosts_active: None,
                price_sol: None,
                price_usd: None,
                price_confidence: None,
                price_change_h1: None,
                price_change_h24: None,
                price_change_periods: PeriodStats::empty(),
                liquidity_usd: None,
                liquidity_base: None,
                liquidity_quote: None,
                volume_24h: None,
                volume_periods: PeriodStats::empty(),
                fdv: None,
                market_cap: None,
                pool_address: None,
                pool_dex: None,
                pool_reserves_sol: None,
                pool_reserves_token: None,
                txn_periods: PeriodStats::empty(),
                buys_24h: None,
                sells_24h: None,
                net_flow_24h: None,
                buy_sell_ratio_24h: None,
                risk_score: None,
                safety_score: None,
                rugged: None,
                mint_authority: None,
                freeze_authority: None,
                total_holders: None,
                top_10_concentration: None,
                security_risks: vec![],
                security_summary: None,
                token_type: None,
                creator_balance_pct: None,
                lp_provider_count: None,
                graph_insiders_detected: None,
                transfer_fee_pct: None,
                transfer_fee_max_amount: None,
                transfer_fee_authority: None,
                top_holders: vec![],
                security_last_updated: None,
                websites: vec![],
                socials: vec![],
                pools: vec![],
                has_ohlcv: false,
                has_pool_price: false,
                has_open_position: false,
                blacklisted: false,
                source_status: build_source_status(false, false, false, false).await,
                timestamp: chrono::Utc::now().to_rfc3339(),
            });
        }
    };

    // Extract token from snapshot for processing
    let token = &snapshot;

    let pool_descriptors = pools::get_token_pools(&mint);
    let canonical_pool_id = pool_descriptors.first().map(|pool| pool.pool_id);
    let mint_pubkey = Pubkey::from_str(&mint).ok();
    let now_unix_opt = match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        Ok(duration) => Some(duration.as_secs() as i64),
        Err(_) => None,
    };
    let mut pool_infos: Vec<TokenPoolInfo> = pool_descriptors
        .into_iter()
        .map(|pool| {
            let token_role = if let Some(mint_key) = mint_pubkey {
                if pool.base_mint == mint_key {
                    "base"
                } else if pool.quote_mint == mint_key {
                    "quote"
                } else {
                    "unknown"
                }
            } else {
                "unknown"
            };

            let paired_mint = if token_role == "base" {
                pool.quote_mint.to_string()
            } else {
                pool.base_mint.to_string()
            };

            let age_secs = pool.last_updated.elapsed().as_secs();
            let age_i64 = if age_secs > i64::MAX as u64 {
                i64::MAX
            } else {
                age_secs as i64
            };

            let last_updated_unix = now_unix_opt.map(|now| now.saturating_sub(age_i64));

            TokenPoolInfo {
                pool_id: pool.pool_id.to_string(),
                program: pool.program_kind.display_name().to_string(),
                base_mint: pool.base_mint.to_string(),
                quote_mint: pool.quote_mint.to_string(),
                token_role: token_role.to_string(),
                paired_mint,
                liquidity_usd: if pool.liquidity_usd.is_finite() {
                    Some(pool.liquidity_usd)
                } else {
                    None
                },
                volume_h24_usd: if pool.volume_h24_usd.is_finite() {
                    Some(pool.volume_h24_usd)
                } else {
                    None
                },
                reserve_accounts: pool
                    .reserve_accounts
                    .iter()
                    .map(|account| account.to_string())
                    .collect(),
                is_canonical: canonical_pool_id
                    .map(|canonical| canonical == pool.pool_id)
                    .unwrap_or_default(),
                last_updated_unix,
            }
        })
        .collect();

    // Get enrichment data (all sync or from cache)
    let pool_start = std::time::Instant::now();
    let (
        price_sol,
        price_confidence,
        price_updated_at,
        pool_address,
        pool_dex,
        pool_reserves_sol,
        pool_reserves_token,
    ) = if let Some(price_result) = pools::get_pool_price(&mint) {
        let age_secs = price_result.timestamp.elapsed().as_secs();
        let now_unix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        (
            Some(price_result.price_sol),
            Some(price_result.confidence.to_string()),
            Some(now_unix - (age_secs as i64)),
            Some(price_result.pool_address),
            price_result.source_pool,
            Some(price_result.sol_reserves),
            Some(price_result.token_reserves),
        )
    } else {
        (None, None, None, None, None, None, None)
    };

    logger::debug(
        LogTag::Webserver,
        &format!(
            "mint={} elapsed={}ms has_price={}",
            mint,
            pool_start.elapsed().as_millis(),
            price_sol.is_some()
        ),
    );

    // Fallback pool entry: the on-chain pool analyzer registry only knows pools
    // for tokens it has actively discovered, so tokens opened from the billboard
    // or search (enriched purely from external APIs) have an empty pools[] even
    // though we DO have a real SOL pair used for pricing. When the registry has
    // nothing, synthesize a single descriptor from the pool the price calculator
    // used (or the last-used pool stored on the token) so the Pools tab shows the
    // canonical SOL pair instead of "No pools found". Pricing always uses a
    // single highest-liquidity SOL pair, so the quote is WSOL.
    if pool_infos.is_empty() {
        // Prefer the on-chain pool the price calculator used; fall back to the
        // last-used pool stored on the token, then to the DexScreener pair the
        // token was enriched from (available immediately for API/billboard tokens
        // even before the pool service has priced them on-chain).
        let mut fallback_pool = pool_address
            .clone()
            .or_else(|| token.pool_price_last_used_pool.clone());
        let mut fallback_dex = pool_dex.clone();

        if fallback_pool.is_none() {
            let mint_clone = mint.clone();
            if let Ok(Some(ds)) = tokio::task::spawn_blocking(move || {
                get_global_database().and_then(|db| db.get_dexscreener_data(&mint_clone).ok().flatten())
            })
            .await
            {
                if let Some(pair) = ds.pair_address.filter(|p| !p.trim().is_empty()) {
                    fallback_pool = Some(pair);
                    fallback_dex = fallback_dex.or(ds.dex_id);
                }
            }
        }

        if let Some(pool_id) = fallback_pool {
            pool_infos.push(TokenPoolInfo {
                pool_id,
                program: fallback_dex.unwrap_or_else(|| format!("{:?}", token.data_source)),
                base_mint: mint.clone(),
                quote_mint: crate::constants::SOL_MINT.to_string(),
                token_role: "base".to_string(),
                paired_mint: crate::constants::SOL_MINT.to_string(),
                liquidity_usd: token.liquidity_usd.filter(|v| v.is_finite()),
                volume_h24_usd: token.volume_h24.filter(|v| v.is_finite()),
                reserve_accounts: Vec::new(),
                is_canonical: true,
                last_updated_unix: now_unix_opt,
            });
        }
    }

    // Get security info from Token struct
    let security_start = std::time::Instant::now();
    let security_score = token.security_score; // Raw risk score (0-150000+, higher = riskier)
                                               // Normalized score from Rugcheck is 0-100 where HIGHER = MORE RISKY
    let security_score_normalised = token.security_score_normalised;
    // Invert it to create a safety_score where HIGHER = SAFER for the UI
    let safety_score = security_score_normalised.map(|s| (100 - s).clamp(0, 100));
    let rugged = Some(token.is_rugged);
    let mint_authority = token.mint_authority.clone();
    let freeze_authority = token.freeze_authority.clone();
    let total_holders = token.total_holders;
    let top_10_concentration = if !token.top_holders.is_empty() {
        // Calculate top 10 concentration from top_holders
        let top_10_pct: f64 = token.top_holders.iter().take(10).map(|h| h.pct).sum();
        if top_10_pct > 0.0 {
            Some(top_10_pct)
        } else {
            None
        }
    } else {
        None
    };
    let security_risks: Vec<crate::tokens::SecurityRisk> = token
        .security_risks
        .iter()
        .map(|r| crate::tokens::SecurityRisk {
            name: r.name.clone(),
            value: r.value.clone(),
            description: r.description.clone(),
            score: r.score,
            level: r.level.clone(),
        })
        .collect();

    logger::debug(
        LogTag::Webserver,
        &format!(
            "mint={} elapsed={}ms has_score={} risks_count={}",
            mint,
            security_start.elapsed().as_millis(),
            security_score.is_some(),
            security_risks.len()
        ),
    );

    // Get status flags (mix of sync and cache checks)
    let ohlcv_start = std::time::Instant::now();
    let has_ohlcv = match crate::ohlcvs::has_data(&mint).await {
        Ok(flag) => flag,
        Err(e) => {
            logger::info(
                LogTag::Webserver,
                &format!("Failed to determine OHLCV availability for {mint}: {e}"),
            );
            false
        }
    };

    logger::debug(
        LogTag::Webserver,
        &format!(
            "mint={} elapsed={}ms has_data={}",
            mint,
            ohlcv_start.elapsed().as_millis(),
            has_ohlcv
        ),
    );

    let has_pool_price = price_sol.is_some();
    let blacklisted = if let Some(db) = get_global_database() {
        let mint_clone = mint.clone();
        let db_clone = db.clone();
        match tokio::task::spawn_blocking(move || db_clone.is_blacklisted(&mint_clone)).await {
            Ok(Ok(flag)) => flag,
            Ok(Err(err)) => {
                logger::info(
                    LogTag::Webserver,
                    &format!("Failed to check blacklist for {mint}: {err}"),
                );
                false
            }
            Err(join_err) => {
                logger::info(
                    LogTag::Webserver,
                    &format!("Join error checking blacklist for {mint}: {join_err}"),
                );
                false
            }
        }
    } else {
        false
    };

    // Position check - this is the ONLY additional async, keep it last and simple
    let position_start = std::time::Instant::now();
    let has_open_position = positions::is_open_position(&mint).await;

    logger::debug(
        LogTag::Webserver,
        &format!(
            "mint={} elapsed={}ms has_position={}",
            mint,
            position_start.elapsed().as_millis(),
            has_open_position
        ),
    );

    // Add token to OHLCV monitoring with appropriate priority
    // This ensures chart data will be available when users view this token again
    let monitoring_start = std::time::Instant::now();
    let priority = if has_open_position {
        crate::ohlcvs::Priority::Critical
    } else {
        crate::ohlcvs::Priority::High // User is actively viewing, high priority for fast data
    };

    if let Err(e) = crate::ohlcvs::add_token_monitoring(&mint, priority).await {
        logger::info(
            LogTag::Webserver,
            &format!("Failed to add {mint} to OHLCV monitoring: {e}"),
        );
    }

    // Record view activity
    if let Err(e) =
        crate::ohlcvs::record_activity(&mint, crate::ohlcvs::ActivityType::TokenViewed).await
    {
        logger::info(
            LogTag::Webserver,
            &format!("Failed to record token view for {mint}: {e}"),
        );
    }

    logger::debug(
        LogTag::Webserver,
        &format!(
            "mint={} elapsed={}ms",
            mint,
            monitoring_start.elapsed().as_millis()
        ),
    );

    // Only log total elapsed time at INFO level if it's significant (>100ms)
    let total_ms = request_start.elapsed().as_millis();
    if total_ms > 100 {
        logger::info(
            LogTag::Webserver,
            &format!("mint={mint} total_elapsed={total_ms}ms (slow request)"),
        );
    } else {
        logger::debug(
            LogTag::Webserver,
            &format!("mint={mint} total_elapsed={total_ms}ms"),
        );
    }

    let created_at_ts = Some(token.first_discovered_at.timestamp());
    let token_birth_ts = token.blockchain_created_at.map(|dt| dt.timestamp());
    let market_data_last_fetched_at_ts = Some(token.market_data_last_fetched_at.timestamp());
    let pool_price_last_calculated_at_ts = Some(token.pool_price_last_calculated_at.timestamp());
    let pair_created_at = token_birth_ts.or(created_at_ts);

    // Prefer pool price (real-time on-chain) over token cached price
    let sol_price_usd = crate::sol_price::get_sol_price();
    let price_usd = if let Some(sol_p) = price_sol {
        if sol_p > 0.0 && sol_price_usd > 0.0 {
            Some(sol_p * sol_price_usd)
        } else {
            None
        }
    } else {
        Some(token.price_usd)
    };

    // Build price change periods from flat fields
    let price_change_periods = PeriodStats {
        m5: token.price_change_m5,
        h1: token.price_change_h1,
        h6: token.price_change_h6,
        h24: token.price_change_h24,
    };

    // Build volume periods from flat fields
    let volume_periods = PeriodStats {
        m5: token.volume_m5,
        h1: token.volume_h1,
        h6: token.volume_h6,
        h24: token.volume_h24,
    };

    // Build transaction periods from flat fields
    let txn_periods = PeriodStats {
        m5: Some(TxnPeriodSummary {
            buys: token.txns_m5_buys,
            sells: token.txns_m5_sells,
        }),
        h1: Some(TxnPeriodSummary {
            buys: token.txns_h1_buys,
            sells: token.txns_h1_sells,
        }),
        h6: Some(TxnPeriodSummary {
            buys: token.txns_h6_buys,
            sells: token.txns_h6_sells,
        }),
        h24: Some(TxnPeriodSummary {
            buys: token.txns_h24_buys,
            sells: token.txns_h24_sells,
        }),
    };

    let buys_24h = token.txns_h24_buys;
    let sells_24h = token.txns_h24_sells;

    let net_flow_24h = match (buys_24h, sells_24h) {
        (Some(buys), Some(sells)) => Some(buys - sells),
        _ => None,
    };

    let buy_sell_ratio_24h = match (buys_24h, sells_24h) {
        (Some(buys), Some(sells)) if sells != 0 => Some(buys as f64 / sells as f64),
        _ => None,
    };

    // Liquidity base/quote not available in new unified Token - use None
    let liquidity_base = None;
    let liquidity_quote = None;

    // Build websites from token.websites vec
    let mut websites: Vec<TokenWebsiteLink> = token
        .websites
        .iter()
        .filter(|w| !w.url.trim().is_empty())
        .map(|w| TokenWebsiteLink {
            label: w.label.clone(),
            url: w.url.clone(),
        })
        .collect();

    // Build socials from token.socials vec
    let socials: Vec<TokenSocialLink> = token
        .socials
        .iter()
        .filter(|s| !s.url.trim().is_empty())
        .map(|s| TokenSocialLink {
            platform: s.link_type.clone(),
            url: s.url.clone(),
        })
        .collect();

    // Tags - unified token doesn't have separate tags/labels, use empty vec
    let combined_tags: Vec<String> = Vec::new();

    let logo_url = token.image_url.clone();
    let primary_website = websites.first().map(|link| link.url.clone());

    // Security summary based on normalized score (0-100, LOWER = SAFER in Rugcheck)
    // We invert it for display: show as "safety score" where higher = safer
    let security_summary = match (rugged, security_score_normalised) {
        (Some(true), Some(score)) => {
            let safety = 100 - score;
            Some(format!(
                "Token flagged as rugged (safety score {}). Investigate before trading.",
                safety
            ))
        }
        (Some(true), None) => {
            Some("Token flagged as rugged. Investigate before trading.".to_owned())
        }
        (_, Some(score)) if score <= 30 => {
            let safety = 100 - score;
            Some(format!(
                "Strong security posture (safety score {}/100).",
                safety
            ))
        }
        (_, Some(score)) if score <= 50 => {
            let safety = 100 - score;
            Some(format!(
                "Moderate security (safety score {}/100). Monitor for changes.",
                safety
            ))
        }
        (_, Some(score)) if score <= 70 => {
            let safety = 100 - score;
            Some(format!(
                "Safety score {}/100 indicates elevated risk.",
                safety
            ))
        }
        (_, Some(score)) => {
            let safety = 100 - score;
            Some(format!(
                "Safety score {}/100 indicates critical risk.",
                safety
            ))
        }
        _ => None,
    };

    // Media fields
    let header_image_url = token.header_image_url.clone();
    let description = normalize_optional_text(token.description.clone());

    // Data source as string
    let data_source = Some(format!("{:?}", token.data_source));

    // Verified = normalized score <= 30 (low risk = safer tokens in Rugcheck)
    let verified = security_score_normalised.is_some_and(|s| s <= 30);

    // Per-source presence for the dialog "no data" row: check each market/security
    // table independently (a token may have DexScreener but not GeckoTerminal, or
    // vice versa) in one blocking hop.
    let (has_dexscreener, has_geckoterminal, has_rugcheck) =
        if let Some(db) = get_global_database() {
            let mint_owned = mint.clone();
            tokio::task::spawn_blocking(move || {
                (
                    matches!(db.get_dexscreener_data(&mint_owned), Ok(Some(_))),
                    matches!(db.get_geckoterminal_data(&mint_owned), Ok(Some(_))),
                    matches!(db.get_rugcheck_data(&mint_owned), Ok(Some(_))),
                )
            })
            .await
            .unwrap_or((false, false, false))
        } else {
            (false, false, false)
        };
    let source_status =
        build_source_status(has_dexscreener, has_geckoterminal, has_rugcheck, has_ohlcv).await;

    Json(TokenDetailResponse {
        mint: token.mint.clone(),
        symbol: token.symbol.clone(),
        name: Some(token.name.clone()),
        description,
        logo_url,
        header_image_url,
        open_graph_image: None, // Not currently stored in Token struct
        website: primary_website,
        data_source,
        verified,
        tags: combined_tags,
        pair_labels: Vec::new(), // Not available in unified Token
        decimals: Some(token.decimals),
        created_at: created_at_ts,
        market_data_last_fetched_at: market_data_last_fetched_at_ts,
        pool_price_last_calculated_at: pool_price_last_calculated_at_ts,
        pair_created_at,
        pair_url: None,      // Not available in unified Token
        boosts_active: None, // Not available in unified Token
        price_sol,
        price_usd,
        price_confidence,
        price_change_h1: token.price_change_h1,
        price_change_h24: token.price_change_h24,
        price_change_periods,
        liquidity_usd: token.liquidity_usd,
        liquidity_base,
        liquidity_quote,
        volume_24h: token.volume_h24,
        volume_periods,
        fdv: token.fdv,
        market_cap: token.market_cap,
        pool_address,
        pool_dex,
        pool_reserves_sol,
        pool_reserves_token,
        txn_periods,
        buys_24h,
        sells_24h,
        net_flow_24h,
        buy_sell_ratio_24h,
        risk_score: security_score,
        safety_score,
        rugged,
        mint_authority,
        freeze_authority,
        total_holders,
        top_10_concentration,
        security_risks,
        security_summary,
        token_type: token.token_type.clone(),
        creator_balance_pct: token.creator_balance_pct,
        lp_provider_count: token.lp_provider_count,
        graph_insiders_detected: token.graph_insiders_detected,
        transfer_fee_pct: token.transfer_fee_pct,
        transfer_fee_max_amount: token.transfer_fee_max_amount,
        transfer_fee_authority: token.transfer_fee_authority.clone(),
        top_holders: token
            .top_holders
            .iter()
            .take(10)
            .map(|h| TopHolderInfo {
                address: h.address.clone(),
                percentage: h.pct,
                is_insider: h.insider,
                owner_type: h.owner.clone(),
            })
            .collect(),
        security_last_updated: token.security_data_last_fetched_at.map(|dt| dt.timestamp()),
        websites,
        socials,
        pools: pool_infos,
        has_ohlcv,
        has_pool_price,
        has_open_position,
        blacklisted,
        source_status,
        timestamp: chrono::Utc::now().to_rfc3339(),
    })
}

/// GET /api/tokens/:mint/analysis
///
/// Comprehensive token analysis endpoint for the Token Analyzer feature.
/// Aggregates data from multiple sources: token database, pool service, security data.
///
/// If the token is not in the local database, attempts to fetch it from external
/// APIs (DexScreener, GeckoTerminal) and adds it to the database before proceeding.
pub async fn get_token_analysis(
    Path(mint): Path<String>,
) -> Result<Json<TokenAnalysisResponse>, (StatusCode, Json<serde_json::Value>)> {
    let request_start = std::time::Instant::now();

    logger::debug(
        LogTag::Webserver,
        &format!("Token analysis requested: mint={mint}"),
    );

    // Fetch token from database, or try external APIs if not found
    let token = match crate::tokens::get_full_token_async(&mint).await {
        Ok(Some(t)) => t,
        Ok(None) => {
            // Token not in database - try to fetch from external APIs
            logger::debug(
                LogTag::Webserver,
                &format!("Token not in DB, trying external APIs: mint={mint}"),
            );

            match fetch_and_add_token_from_external(&mint).await {
                Some(t) => {
                    logger::info(
                        LogTag::Webserver,
                        &format!(
                            "Token fetched from external API and added to DB: mint={} symbol={}",
                            mint, t.symbol
                        ),
                    );
                    t
                }
                None => {
                    logger::debug(
                        LogTag::Webserver,
                        &format!("Token not found in DB or external APIs: mint={mint}"),
                    );
                    return Err((
                        StatusCode::NOT_FOUND,
                        Json(serde_json::json!({
                          "success": false,
                          "error": "Token not found in database or external sources"
                        })),
                    ));
                }
            }
        }
        Err(e) => {
            logger::warning(
                LogTag::Webserver,
                &format!("Failed to fetch token: mint={mint} error={e}"),
            );
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                  "success": false,
                  "error": format!("Failed to fetch token: {e}")
                })),
            ));
        }
    };

    // Get pool data for liquidity analysis
    let pool_descriptors = pools::get_token_pools(&mint);
    let canonical_pool_id = pool_descriptors.first().map(|p| p.pool_id);

    // Get real-time pool price
    let pool_price = pools::get_pool_price(&mint);

    // Get SOL/USD price for conversions (get_sol_price returns f64 directly)
    let sol_price_usd = get_sol_price();

    // Calculate effective prices (prefer pool price over cached token price)
    let effective_price_sol = pool_price
        .as_ref()
        .map(|p| p.price_sol)
        .unwrap_or(token.price_sol);
    let effective_price_usd = effective_price_sol * sol_price_usd;

    // Build overview
    let overview = TokenOverview {
        mint: token.mint.clone(),
        symbol: Some(token.symbol.clone()),
        name: Some(token.name.clone()),
        description: normalize_optional_text(token.description.clone()),
        logo_url: token.image_url.clone(),
        decimals: token.decimals,
        supply: token.supply.clone(),
        price_sol: if effective_price_sol > 0.0 {
            Some(effective_price_sol)
        } else {
            None
        },
        price_usd: if effective_price_usd > 0.0 {
            Some(effective_price_usd)
        } else {
            None
        },
        total_holders: token.total_holders,
        website: token.websites.first().map(|w| w.url.clone()),
        twitter: token
            .socials
            .iter()
            .find(|s| s.link_type.eq_ignore_ascii_case("twitter"))
            .map(|s| s.url.clone()),
        telegram: token
            .socials
            .iter()
            .find(|s| s.link_type.eq_ignore_ascii_case("telegram"))
            .map(|s| s.url.clone()),
    };

    // Build security analysis
    let security = if token.security_score.is_some()
        || token.security_score_normalised.is_some()
        || !token.security_risks.is_empty()
    {
        let has_transfer_fee = token.transfer_fee_pct.is_some_and(|f| f > 0.0);
        // is_mutable approximated by presence of mint_authority
        let is_mutable = token.mint_authority.is_some();

        Some(SecurityAnalysis {
            score: token.security_score,
            normalized_score: token.security_score_normalised,
            mint_authority: token.mint_authority.clone(),
            freeze_authority: token.freeze_authority.clone(),
            has_transfer_fee,
            is_mutable,
            top_holders_pct: if !token.top_holders.is_empty() {
                Some(token.top_holders.iter().take(10).map(|h| h.pct).sum())
            } else {
                None
            },
            risks: token
                .security_risks
                .iter()
                .map(|r| AnalysisSecurityRisk {
                    name: r.name.clone(),
                    level: r.level.clone(),
                    description: r.description.clone(),
                })
                .collect(),
        })
    } else {
        None
    };

    // Build market analysis using the effective_price_sol computed earlier
    let has_market_data = effective_price_sol > 0.0 || token.volume_h24.is_some();

    let market = if has_market_data {
        Some(MarketAnalysis {
            price_sol: effective_price_sol,
            price_usd: Some(effective_price_usd),
            volume_h24: token.volume_h24,
            volume_h6: token.volume_h6,
            volume_h1: token.volume_h1,
            price_change_h24: token.price_change_h24,
            price_change_h6: token.price_change_h6,
            price_change_h1: token.price_change_h1,
            txns_buys_h24: token.txns_h24_buys,
            txns_sells_h24: token.txns_h24_sells,
            fdv: token.fdv,
            market_cap: token.market_cap,
        })
    } else {
        None
    };

    // Build liquidity analysis from pool descriptors
    let liquidity = if !pool_descriptors.is_empty() {
        let total_liquidity_sol: f64 = pool_descriptors
            .iter()
            .map(|p| {
                // liquidity_usd / sol_price gives approximate SOL liquidity
                if p.liquidity_usd > 0.0 && sol_price_usd > 0.0 {
                    p.liquidity_usd / sol_price_usd
                } else {
                    0.0
                }
            })
            .sum();

        let total_liquidity_usd: f64 = pool_descriptors.iter().map(|p| p.liquidity_usd).sum();

        let pools: Vec<AnalysisPoolInfo> = pool_descriptors
            .iter()
            .map(|p| {
                let liquidity_sol = if p.liquidity_usd > 0.0 && sol_price_usd > 0.0 {
                    p.liquidity_usd / sol_price_usd
                } else {
                    0.0
                };
                AnalysisPoolInfo {
                    address: p.pool_id.to_string(),
                    dex: p.program_kind.display_name().to_string(),
                    liquidity_sol,
                    is_canonical: canonical_pool_id.is_some_and(|c| c == p.pool_id),
                }
            })
            .collect();

        Some(LiquidityAnalysis {
            total_liquidity_sol,
            total_liquidity_usd: if total_liquidity_usd > 0.0 {
                Some(total_liquidity_usd)
            } else {
                None
            },
            pool_count: pool_descriptors.len() as i32,
            pools,
        })
    } else {
        None
    };

    let elapsed_ms = request_start.elapsed().as_millis();
    logger::debug(
        LogTag::Webserver,
        &format!(
            "Token analysis completed: mint={} elapsed={}ms",
            mint, elapsed_ms
        ),
    );

    Ok(Json(TokenAnalysisResponse {
        success: true,
        overview,
        security,
        market,
        liquidity,
        fetched_at: chrono::Utc::now().to_rfc3339(),
    }))
}

/// POST /api/tokens/:mint/refresh
///
/// Force refresh token data (immediate update outside scheduled loops)
pub async fn refresh_token_data(
    Path(mint): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    logger::debug(
        LogTag::Webserver,
        &format!("Force refresh requested for mint={mint}"),
    );

    match crate::tokens::request_immediate_update(&mint).await {
        Ok(result) => {
            if result.is_success() {
                logger::info(
                    LogTag::Webserver,
                    &format!(
                        "mint={} refresh_success sources={:?}",
                        mint, result.successes
                    ),
                );
                Ok(Json(serde_json::json!({
                  "success": true,
                  "mint": mint,
                  "sources_updated": result.successes,
                  "partial_failures": result.failures,
                })))
            } else {
                logger::debug(
                    LogTag::Webserver,
                    &format!(
                        "mint={} refresh_failed failures={:?}",
                        mint, result.failures
                    ),
                );
                Err((
                    StatusCode::SERVICE_UNAVAILABLE,
                    Json(serde_json::json!({
                      "success": false,
                      "mint": mint,
                      "error": "All data sources failed",
                      "failures": result.failures,
                    })),
                ))
            }
        }
        Err(e) => {
            logger::warning(
                LogTag::Webserver,
                &format!("mint={mint} refresh_error error={e}"),
            );
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                  "success": false,
                  "mint": mint,
                  "error": format!("Failed to refresh token: {e}"),
                })),
            ))
        }
    }
}
