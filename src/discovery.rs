use crate::logger::{ log, LogTag };
use crate::global::*;
use crate::decimal_cache::{ DecimalCache, fetch_or_cache_decimals };
use crate::positions::*;
use std::sync::Arc;
use tokio::sync::Notify;
use reqwest::StatusCode;
use tokio::sync::Semaphore;
use tokio::time::{ sleep, Duration };
use crate::utils::check_shutdown_or_delay;
use solana_client::rpc_client::RpcClient;
use std::path::Path;
use colored::Colorize;

static INFO_RATE_LIMITER: once_cell::sync::Lazy<Arc<Semaphore>> = once_cell::sync::Lazy::new(||
    Arc::new(Semaphore::new(200))
);
static DISCOVERY_RATE_LIMITER: once_cell::sync::Lazy<Arc<Semaphore>> = once_cell::sync::Lazy::new(||
    Arc::new(Semaphore::new(30))
);

/// Check if debug discovery mode is enabled via command line args
fn is_debug_discovery_enabled() -> bool {
    if let Ok(args) = CMD_ARGS.lock() {
        args.contains(&"--debug-discovery".to_string())
    } else {
        false
    }
}

/// For each mint in LIST_MINTS, fetch token info and update LIST_TOKENS
pub async fn update_tokens_from_mints(shutdown: Arc<Notify>) -> Result<(), String> {
    let debug_mode = is_debug_discovery_enabled();

    if debug_mode {
        log(LogTag::Monitor, "DEBUG", "Starting update_tokens_from_mints...");
    }

    // First, get all mint addresses from open positions to ensure we always update them
    let position_mints = {
        if let Ok(positions) = SAVED_POSITIONS.lock() {
            positions
                .iter()
                .filter(|p| p.exit_time.is_none()) // Only consider open positions
                .map(|p| p.mint.clone())
                .collect::<Vec<String>>()
        } else {
            Vec::new()
        }
    };

    // Log the position mints we're prioritizing
    if !position_mints.is_empty() {
        let log_level = if debug_mode { "DEBUG" } else { "INFO" };
        log(
            LogTag::Monitor,
            log_level,
            &format!("Prioritizing {} tokens from open positions", position_mints.len())
        );
        if debug_mode && !position_mints.is_empty() {
            log(
                LogTag::Monitor,
                "DEBUG",
                &format!(
                    "Position mints: {:?}",
                    &position_mints[..std::cmp::min(5, position_mints.len())]
                )
            );
        }
    }

    // Get all mints from the global list
    let mut mints: Vec<String> = match LIST_MINTS.read() {
        Ok(set) => set.iter().cloned().collect(),
        Err(e) => {
            log(LogTag::Monitor, "ERROR", &format!("Failed to read LIST_MINTS: {}", e));
            return Err(e.to_string());
        }
    };

    // Make sure all position mints are included in our list (even if they somehow got removed from LIST_MINTS)
    for mint in &position_mints {
        if !mints.contains(mint) {
            mints.push(mint.clone());
        }
    }

    if mints.is_empty() {
        return Ok(());
    }

    // Load configuration and create RPC client for decimal fetching
    let configs = crate::global::read_configs("configs.json").map_err(|e| e.to_string())?;
    let rpc_client = RpcClient::new(&configs.rpc_url);

    // Load decimal cache
    let cache_path = Path::new("decimal_cache.json");
    let mut decimal_cache = match DecimalCache::load_from_file(cache_path) {
        Ok(cache) => cache,
        Err(e) => {
            log(
                LogTag::Monitor,
                "WARN",
                &format!("Failed to load decimal cache: {}, using new cache", e)
            );
            DecimalCache::new()
        }
    };

    // Fetch decimals for all mints upfront
    let decimals_map = fetch_or_cache_decimals(
        &rpc_client,
        &mints,
        &mut decimal_cache,
        cache_path
    ).await.map_err(|e| e.to_string())?;

    let mut tokens = Vec::new();

    // Reorganize mints: prioritize position mints first
    let mut prioritized_mints = Vec::new();

    // First add all position mints
    for mint in &position_mints {
        prioritized_mints.push(mint.clone());
    }

    // Then add all other mints that aren't in position_mints
    for mint in mints {
        if !position_mints.contains(&mint) {
            prioritized_mints.push(mint);
        }
    }

    // Process in chunks of 30, but prioritize position mints
    for (chunk_index, chunk) in prioritized_mints.chunks(30).enumerate() {
        if check_shutdown_or_delay(&shutdown, Duration::from_millis(500)).await {
            log(LogTag::Monitor, "INFO", "update_tokens_from_mints task shutting down...");
            return Ok(());
        }

        // Check if this chunk contains any position mints
        let contains_positions = chunk.iter().any(|mint| position_mints.contains(mint));

        if debug_mode {
            log(
                LogTag::Monitor,
                "DEBUG",
                &format!(
                    "Processing chunk {} with {} tokens (contains positions: {})",
                    chunk_index + 1,
                    chunk.len(),
                    contains_positions
                )
            );
        }

        // Log priority info if this chunk contains position mints
        if contains_positions && !debug_mode {
            log(
                LogTag::Monitor,
                "INFO",
                &format!("Processing chunk with prioritized position tokens").dimmed().to_string()
            );
        }

        // Acquire permit for info rate limit (200 per minute)
        let permit = match INFO_RATE_LIMITER.clone().acquire_owned().await {
            Ok(p) => p,
            Err(e) => {
                log(
                    LogTag::Monitor,
                    "ERROR",
                    &format!("Failed to acquire info rate limiter: {}", e)
                );
                return Err(e.to_string());
            }
        };
        let chain_id = "solana";
        let token_addresses = chunk.join(",");
        let url = format!("https://api.dexscreener.com/tokens/v1/{}/{}", chain_id, token_addresses);

        // Create HTTP client with timeout for shutdown responsiveness
        let client = reqwest::Client
            ::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        let resp = match client.get(&url).send().await {
            Ok(r) => r,
            Err(e) => {
                log(LogTag::Monitor, "ERROR", &format!("Failed to send batch request: {}", e));
                drop(permit);
                continue;
            }
        };
        drop(permit); // Release permit immediately after request
        if resp.status() == StatusCode::OK {
            let arr: serde_json::Value = resp.json().await.unwrap_or_else(|e| {
                log(
                    LogTag::Monitor,
                    "ERROR",
                    &format!("Failed to parse batch response JSON: {}", e)
                );
                serde_json::json!([])
            });
            if let Some(arr) = arr.as_array() {
                for pair in arr {
                    if let Some(base_token) = pair.get("baseToken") {
                        let mint = base_token
                            .get("address")
                            .and_then(|a| a.as_str())
                            .unwrap_or("");
                        let created_at = pair
                            .get("pairCreatedAt")
                            .and_then(|v| v.as_i64())
                            .and_then(|ts| chrono::DateTime::from_timestamp_millis(ts));

                        // Parse transaction stats
                        let txns = pair.get("txns").map(|txns_obj| {
                            crate::global::TxnStats {
                                m5: txns_obj.get("m5").map(|m5| crate::global::TxnPeriod {
                                    buys: m5.get("buys").and_then(|v| v.as_i64()),
                                    sells: m5.get("sells").and_then(|v| v.as_i64()),
                                }),
                                h1: txns_obj.get("h1").map(|h1| crate::global::TxnPeriod {
                                    buys: h1.get("buys").and_then(|v| v.as_i64()),
                                    sells: h1.get("sells").and_then(|v| v.as_i64()),
                                }),
                                h6: txns_obj.get("h6").map(|h6| crate::global::TxnPeriod {
                                    buys: h6.get("buys").and_then(|v| v.as_i64()),
                                    sells: h6.get("sells").and_then(|v| v.as_i64()),
                                }),
                                h24: txns_obj.get("h24").map(|h24| crate::global::TxnPeriod {
                                    buys: h24.get("buys").and_then(|v| v.as_i64()),
                                    sells: h24.get("sells").and_then(|v| v.as_i64()),
                                }),
                            }
                        });

                        // Parse volume stats
                        let volume = pair.get("volume").map(|vol_obj| {
                            crate::global::VolumeStats {
                                m5: vol_obj.get("m5").and_then(|v| v.as_f64()),
                                h1: vol_obj.get("h1").and_then(|v| v.as_f64()),
                                h6: vol_obj.get("h6").and_then(|v| v.as_f64()),
                                h24: vol_obj.get("h24").and_then(|v| v.as_f64()),
                            }
                        });

                        // Parse price change stats
                        let price_change = pair.get("priceChange").map(|pc_obj| {
                            crate::global::PriceChangeStats {
                                m5: pc_obj.get("m5").and_then(|v| v.as_f64()),
                                h1: pc_obj.get("h1").and_then(|v| v.as_f64()),
                                h6: pc_obj.get("h6").and_then(|v| v.as_f64()),
                                h24: pc_obj.get("h24").and_then(|v| v.as_f64()),
                            }
                        });

                        // Parse liquidity info
                        let liquidity = pair.get("liquidity").map(|liq_obj| {
                            crate::global::LiquidityInfo {
                                usd: liq_obj.get("usd").and_then(|v| v.as_f64()),
                                base: liq_obj.get("base").and_then(|v| v.as_f64()),
                                quote: liq_obj.get("quote").and_then(|v| v.as_f64()),
                            }
                        });

                        // Log liquidity parsing for debugging
                        if let Some(symbol) = base_token.get("symbol").and_then(|v| v.as_str()) {
                            let liquidity_usd = liquidity
                                .as_ref()
                                .and_then(|l| l.usd)
                                .unwrap_or(0.0);
                            if liquidity_usd == 0.0 {
                                log(
                                    LogTag::Monitor,
                                    "DEBUG",
                                    &format!(
                                        "Token {} parsed with zero liquidity USD from API: {:?}",
                                        symbol,
                                        pair.get("liquidity")
                                    )
                                        .dimmed()
                                        .to_string()
                                );
                            }
                        }

                        // Parse token info
                        let info = pair.get("info").map(|info_obj| {
                            let websites = info_obj
                                .get("websites")
                                .and_then(|w| w.as_array())
                                .map(|arr| {
                                    arr.iter()
                                        .filter_map(|website| {
                                            website
                                                .get("url")
                                                .and_then(|url| url.as_str())
                                                .map(|url| {
                                                    crate::global::WebsiteLink {
                                                        label: website
                                                            .get("label")
                                                            .and_then(|l| l.as_str())
                                                            .map(|s| s.to_string()),
                                                        url: url.to_string(),
                                                    }
                                                })
                                        })
                                        .collect()
                                })
                                .unwrap_or_default();

                            let socials = info_obj
                                .get("socials")
                                .and_then(|s| s.as_array())
                                .map(|arr| {
                                    arr.iter()
                                        .filter_map(|social| {
                                            let url = social.get("url").and_then(|u| u.as_str())?;
                                            let link_type = social
                                                .get("type")
                                                .and_then(|t| t.as_str())?;
                                            Some(crate::global::SocialLink {
                                                link_type: link_type.to_string(),
                                                url: url.to_string(),
                                            })
                                        })
                                        .collect()
                                })
                                .unwrap_or_default();

                            crate::global::TokenInfo {
                                image_url: info_obj
                                    .get("imageUrl")
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string()),
                                header: info_obj
                                    .get("header")
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string()),
                                open_graph: info_obj
                                    .get("openGraph")
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string()),
                                websites,
                                socials,
                            }
                        });

                        // Parse boost info
                        let boosts = pair.get("boosts").map(|boost_obj| {
                            crate::global::BoostInfo {
                                active: boost_obj.get("active").and_then(|v| v.as_i64()),
                            }
                        });

                        // Parse labels
                        let labels = pair
                            .get("labels")
                            .and_then(|l| l.as_array())
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|v| v.as_str())
                                    .map(|s| s.to_string())
                                    .collect()
                            })
                            .unwrap_or_default();

                        // Parse price data
                        let price = pair
                            .get("priceNative")
                            .and_then(|v| v.as_str())
                            .and_then(|s| s.parse::<f64>().ok())
                            .unwrap_or(0.0);

                        let price_usd = pair
                            .get("priceUsd")
                            .and_then(|v| v.as_str())
                            .and_then(|s| s.parse::<f64>().ok())
                            .unwrap_or(0.0);

                        let token = Token {
                            mint: mint.to_string(),
                            symbol: base_token
                                .get("symbol")
                                .and_then(|s| s.as_str())
                                .unwrap_or("")
                                .to_string(),
                            name: base_token
                                .get("name")
                                .and_then(|s| s.as_str())
                                .unwrap_or("")
                                .to_string(),
                            decimals: decimals_map.get(mint).copied().unwrap_or(9),
                            chain: "solana".to_string(),

                            // Existing fields - keeping original logic but using info.image_url as primary
                            logo_url: info
                                .as_ref()
                                .and_then(|i| i.image_url.clone())
                                .or_else(|| {
                                    pair.get("info")
                                        .and_then(|i| i.get("imageUrl"))
                                        .and_then(|s| s.as_str())
                                        .map(|s| s.to_string())
                                }),
                            coingecko_id: None,
                            website: info
                                .as_ref()
                                .and_then(|i| i.websites.first())
                                .map(|w| w.url.clone())
                                .or_else(|| {
                                    pair.get("info")
                                        .and_then(|i| i.get("websites"))
                                        .and_then(|w| w.as_array())
                                        .and_then(|arr| arr.get(0))
                                        .and_then(|w| w.get("url"))
                                        .and_then(|s| s.as_str())
                                        .map(|s| s.to_string())
                                }),
                            description: None,
                            tags: vec![],
                            is_verified: false,
                            created_at,

                            // Price data
                            price_dexscreener_sol: Some(price),
                            price_dexscreener_usd: Some(price_usd),
                            price_pool_sol: None,
                            price_pool_usd: None,
                            pools: vec![],

                            // New DexScreener fields
                            dex_id: pair
                                .get("dexId")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string()),
                            pair_address: pair
                                .get("pairAddress")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string()),
                            pair_url: pair
                                .get("url")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string()),
                            labels,
                            fdv: pair.get("fdv").and_then(|v| v.as_f64()),
                            market_cap: pair.get("marketCap").and_then(|v| v.as_f64()),
                            txns,
                            volume,
                            price_change,
                            liquidity,
                            info,
                            boosts,
                        };
                        tokens.push(token);
                    }
                }
            }
        }
        // Sleep to enforce 200 requests per minute (max 3.0 req/sec)
        sleep(Duration::from_millis(310)).await;
    }

    // Cache all tokens to database before updating LIST_TOKENS
    let mut cached_count = 0;
    let mut new_tokens_count = 0;

    if debug_mode {
        log(LogTag::Monitor, "DEBUG", &format!("Caching {} tokens to database...", tokens.len()));
    }

    for token in &tokens {
        match crate::global::cache_token_to_db(token, "dexscreener") {
            Ok(is_new) => {
                cached_count += 1;
                if is_new {
                    new_tokens_count += 1;
                    if debug_mode {
                        log(
                            LogTag::Monitor,
                            "DEBUG",
                            &format!("New token cached: {} ({})", token.symbol, token.mint)
                        );
                    }
                }
            }
            Err(e) => {
                log(
                    LogTag::Monitor,
                    "WARN",
                    &format!("Failed to cache token {} to DB: {}", token.symbol, e)
                        .dimmed()
                        .to_string()
                );
            }
        }
    }

    if cached_count > 0 || debug_mode {
        let log_level = if debug_mode { "DEBUG" } else { "CACHE" };
        log(
            LogTag::Monitor,
            log_level,
            &format!("Cached {} tokens to DB ({} new)", cached_count, new_tokens_count)
                .dimmed()
                .to_string()
        );
    }

    // Update LIST_TOKENS
    let mints_count = match LIST_MINTS.read() {
        Ok(set) => set.len(),
        Err(_) => 0,
    };
    match LIST_TOKENS.write() {
        Ok(mut list) => {
            // Log liquidity breakdown before updating
            let total_tokens = tokens.len();
            let with_liquidity = tokens
                .iter()
                .filter(|token| {
                    token.liquidity
                        .as_ref()
                        .and_then(|l| l.usd)
                        .unwrap_or(0.0) > 0.0
                })
                .count();
            let zero_liquidity = total_tokens - with_liquidity;

            log(
                LogTag::Monitor,
                "DEBUG",
                &format!(
                    "Updating LIST_TOKENS: {} total tokens, {} with liquidity, {} with zero liquidity",
                    total_tokens,
                    with_liquidity,
                    zero_liquidity
                )
                    .dimmed()
                    .to_string()
            );

            *list = tokens;

            // Count how many position tokens were successfully updated
            let position_tokens_updated = if !position_mints.is_empty() {
                list.iter()
                    .filter(|token| position_mints.contains(&token.mint))
                    .count()
            } else {
                0
            };

            // Enhanced logging to show position tokens status
            if !position_mints.is_empty() {
                let log_level = if debug_mode { "DEBUG" } else { "INFO" };
                log(
                    LogTag::Monitor,
                    log_level,
                    &format!(
                        "Dexscreener Updated tokens: {}, mints: {}, position tokens: {}/{}",
                        list.len(),
                        mints_count,
                        position_tokens_updated,
                        position_mints.len()
                    )
                );

                // Check if any position tokens are missing and log a warning
                if position_tokens_updated < position_mints.len() {
                    log(
                        LogTag::Monitor,
                        "WARN",
                        &format!(
                            "Failed to update {}/{} position tokens! Will retry in next cycle.",
                            position_mints.len() - position_tokens_updated,
                            position_mints.len()
                        )
                    );
                }
            } else if debug_mode || list.len() > 0 {
                let log_level = if debug_mode { "DEBUG" } else { "INFO" };
                log(
                    LogTag::Monitor,
                    log_level,
                    &format!("Dexscreener Updated tokens: {}, mints: {}", list.len(), mints_count)
                        .dimmed()
                        .to_string()
                );
            }
        }
        Err(e) => {
            log(LogTag::Monitor, "ERROR", &format!("Failed to write LIST_TOKENS: {}", e));
            return Err(e.to_string());
        }
    }
    Ok(())
}

// End of update_tokens_from_mints
/// Fetch token boosts from Dexscreener API, only filling mint field
pub async fn discovery_dexscreener_fetch_token_boosts() -> Result<(), String> {
    let debug_mode = is_debug_discovery_enabled();

    if debug_mode {
        log(LogTag::Monitor, "DEBUG", "Starting Dexscreener token boosts fetch...");
    }

    // Acquire permit for discovery rate limit (30 per minute)
    let permit = match DISCOVERY_RATE_LIMITER.clone().acquire_owned().await {
        Ok(p) => p,
        Err(e) => {
            log(
                LogTag::Monitor,
                "ERROR",
                &format!("Failed to acquire discovery rate limiter: {}", e)
            );
            return Err(e.to_string());
        }
    };
    let url = "https://api.dexscreener.com/token-boosts/latest/v1";

    if debug_mode {
        log(LogTag::Monitor, "DEBUG", &format!("Fetching from URL: {}", url));
    }

    // Create HTTP client with timeout for shutdown responsiveness
    let client = reqwest::Client
        ::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());

    let resp = match client.get(url).send().await {
        Ok(r) => r,
        Err(e) => {
            log(LogTag::Monitor, "ERROR", &format!("Failed to fetch token boosts: {}", e));
            drop(permit);
            return Err(e.to_string());
        }
    };
    drop(permit);
    if resp.status() != StatusCode::OK {
        log(LogTag::Monitor, "ERROR", &format!("API returned status {}", resp.status()));
        return Err(format!("API returned status {}", resp.status()).into());
    }
    let arr: serde_json::Value = match resp.json().await {
        Ok(a) => a,
        Err(e) => {
            log(LogTag::Monitor, "ERROR", &format!("Failed to parse token boosts JSON: {}", e));
            return Err(e.to_string());
        }
    };

    let all_items_count = arr
        .as_array()
        .map(|a| a.len())
        .unwrap_or(0);
    let mints = arr
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter(|item| item.get("chainId").and_then(|v| v.as_str()) == Some("solana"))
        .filter_map(|item| item.get("tokenAddress").and_then(|v| v.as_str()))
        .map(|mint| mint.to_string())
        .collect::<Vec<_>>();

    if debug_mode {
        log(
            LogTag::Monitor,
            "DEBUG",
            &format!("Found {} total items, {} Solana tokens", all_items_count, mints.len())
        );
        if !mints.is_empty() {
            log(
                LogTag::Monitor,
                "DEBUG",
                &format!("Sample mints: {:?}", &mints[..std::cmp::min(3, mints.len())])
            );
        }
    }

    let mut new_count = 0;
    if let Ok(mut set) = LIST_MINTS.write() {
        for mint in mints {
            if set.insert(mint.clone()) {
                new_count += 1;
                if debug_mode {
                    log(LogTag::Monitor, "DEBUG", &format!("Added new mint: {}", mint));
                }
            }
        }
    }

    if new_count > 0 || debug_mode {
        let log_level = if debug_mode { "DEBUG" } else { "INFO" };
        crate::logger::log(
            crate::logger::LogTag::Monitor,
            log_level,
            &format!("Dexscreener Boosts New tokens seen: {}", new_count)
        );
    }

    // Sleep to enforce 30 requests per minute (max 2.0 req/sec)
    sleep(Duration::from_millis(2100)).await;
    Ok(())
}

/// Fetch token boosts from Dexscreener TOP API, only filling mint field
pub async fn discovery_dexscreener_fetch_token_boosts_top() -> Result<(), String> {
    let debug_mode = is_debug_discovery_enabled();

    if debug_mode {
        log(LogTag::Monitor, "DEBUG", "Starting Dexscreener token boosts TOP fetch...");
    }

    // Acquire permit for discovery rate limit (30 per minute)
    let permit = match DISCOVERY_RATE_LIMITER.clone().acquire_owned().await {
        Ok(p) => p,
        Err(e) => {
            log(
                LogTag::Monitor,
                "ERROR",
                &format!("Failed to acquire discovery rate limiter: {}", e)
            );
            return Err(e.to_string());
        }
    };
    let url = "https://api.dexscreener.com/token-boosts/top/v1";

    if debug_mode {
        log(LogTag::Monitor, "DEBUG", &format!("Fetching from URL: {}", url));
    }

    // Create HTTP client with timeout for shutdown responsiveness
    let client = reqwest::Client
        ::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());

    let resp = match client.get(url).send().await {
        Ok(r) => r,
        Err(e) => {
            log(LogTag::Monitor, "ERROR", &format!("Failed to fetch token boosts top: {}", e));
            drop(permit);
            return Err(e.to_string());
        }
    };
    drop(permit);
    if resp.status() != StatusCode::OK {
        log(LogTag::Monitor, "ERROR", &format!("API returned status {}", resp.status()));
        return Err(format!("API returned status {}", resp.status()).into());
    }
    let arr: serde_json::Value = match resp.json().await {
        Ok(a) => a,
        Err(e) => {
            log(LogTag::Monitor, "ERROR", &format!("Failed to parse token boosts top JSON: {}", e));
            return Err(e.to_string());
        }
    };

    let all_items_count = arr
        .as_array()
        .map(|a| a.len())
        .unwrap_or(0);
    let mints = arr
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter(|item| item.get("chainId").and_then(|v| v.as_str()) == Some("solana"))
        .filter_map(|item| item.get("tokenAddress").and_then(|v| v.as_str()))
        .map(|mint| mint.to_string())
        .collect::<Vec<_>>();

    if debug_mode {
        log(
            LogTag::Monitor,
            "DEBUG",
            &format!("Found {} total items, {} Solana tokens", all_items_count, mints.len())
        );
        if !mints.is_empty() {
            log(
                LogTag::Monitor,
                "DEBUG",
                &format!("Sample mints: {:?}", &mints[..std::cmp::min(3, mints.len())])
            );
        }
    }

    let mut new_count = 0;
    if let Ok(mut set) = LIST_MINTS.write() {
        for mint in mints {
            if set.insert(mint.clone()) {
                new_count += 1;
                if debug_mode {
                    log(LogTag::Monitor, "DEBUG", &format!("Added new mint: {}", mint));
                }
            }
        }
    }

    if new_count > 0 || debug_mode {
        let log_level = if debug_mode { "DEBUG" } else { "INFO" };
        crate::logger::log(
            crate::logger::LogTag::Monitor,
            log_level,
            &format!("Dexscreener Boosts Top New tokens seen: {}", new_count)
        );
    }

    // Sleep to enforce 30 requests per minute (max 2.0 req/sec)
    sleep(Duration::from_millis(2100)).await;
    Ok(())
}

/// Fetch token profiles from Dexscreener API, only filling mint field
pub async fn discovery_dexscreener_fetch_token_profiles() -> Result<(), String> {
    let debug_mode = is_debug_discovery_enabled();

    if debug_mode {
        log(LogTag::Monitor, "DEBUG", "Starting Dexscreener token profiles fetch...");
    }

    // Acquire permit for discovery rate limit (30 per minute)
    let permit = match DISCOVERY_RATE_LIMITER.clone().acquire_owned().await {
        Ok(p) => p,
        Err(e) => {
            log(
                LogTag::Monitor,
                "ERROR",
                &format!("Failed to acquire discovery rate limiter: {}", e)
            );
            return Err(e.to_string());
        }
    };
    let url = "https://api.dexscreener.com/token-profiles/latest/v1";

    if debug_mode {
        log(LogTag::Monitor, "DEBUG", &format!("Fetching from URL: {}", url));
    }

    // Create HTTP client with timeout for shutdown responsiveness
    let client = reqwest::Client
        ::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());

    let resp = match client.get(url).send().await {
        Ok(r) => r,
        Err(e) => {
            log(LogTag::Monitor, "ERROR", &format!("Failed to fetch token profiles: {}", e));
            drop(permit);
            return Err(e.to_string());
        }
    };
    drop(permit);
    if resp.status() != StatusCode::OK {
        log(LogTag::Monitor, "ERROR", &format!("API returned status {}", resp.status()));
        return Err(format!("API returned status {}", resp.status()).into());
    }
    let arr: serde_json::Value = match resp.json().await {
        Ok(a) => a,
        Err(e) => {
            log(LogTag::Monitor, "ERROR", &format!("Failed to parse token profiles JSON: {}", e));
            return Err(e.to_string());
        }
    };

    let all_items_count = arr
        .as_array()
        .map(|a| a.len())
        .unwrap_or(0);
    let mints = arr
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter(|item| item.get("chainId").and_then(|v| v.as_str()) == Some("solana"))
        .filter_map(|item| item.get("tokenAddress").and_then(|v| v.as_str()))
        .map(|mint| mint.to_string())
        .collect::<Vec<_>>();

    if debug_mode {
        log(
            LogTag::Monitor,
            "DEBUG",
            &format!("Found {} total items, {} Solana tokens", all_items_count, mints.len())
        );
        if !mints.is_empty() {
            log(
                LogTag::Monitor,
                "DEBUG",
                &format!("Sample mints: {:?}", &mints[..std::cmp::min(3, mints.len())])
            );
        }
    }

    let mut new_count = 0;
    if let Ok(mut set) = LIST_MINTS.write() {
        for mint in mints {
            if set.insert(mint.clone()) {
                new_count += 1;
                if debug_mode {
                    log(LogTag::Monitor, "DEBUG", &format!("Added new mint: {}", mint));
                }
            }
        }
    }

    if new_count > 0 || debug_mode {
        let log_level = if debug_mode { "DEBUG" } else { "INFO" };
        crate::logger::log(
            crate::logger::LogTag::Monitor,
            log_level,
            &format!("Dexscreener Profiles New tokens seen: {}", new_count)
        );
    }

    // Sleep to enforce 30 requests per minute (max 2.0 req/sec)
    sleep(Duration::from_millis(2100)).await;
    Ok(())
}

/// Fetch verified tokens from RugCheck API, only filling mint field
pub async fn discovery_rugcheck_fetch_verified() -> Result<(), String> {
    let debug_mode = is_debug_discovery_enabled();

    if debug_mode {
        log(LogTag::Monitor, "DEBUG", "Starting RugCheck verified tokens fetch...");
    }

    // Acquire permit for discovery rate limit (30 per minute)
    let permit = match DISCOVERY_RATE_LIMITER.clone().acquire_owned().await {
        Ok(p) => p,
        Err(e) => {
            log(
                LogTag::Monitor,
                "ERROR",
                &format!("Failed to acquire discovery rate limiter: {}", e)
            );
            return Err(e.to_string());
        }
    };
    let url = "https://api.rugcheck.xyz/v1/stats/verified";

    if debug_mode {
        log(LogTag::Monitor, "DEBUG", &format!("Fetching from URL: {}", url));
    }

    // Create HTTP client with timeout for shutdown responsiveness
    let client = reqwest::Client
        ::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());

    let resp = match client.get(url).send().await {
        Ok(r) => r,
        Err(e) => {
            log(LogTag::Monitor, "ERROR", &format!("Failed to fetch RugCheck verified: {}", e));
            drop(permit);
            return Err(e.to_string());
        }
    };
    drop(permit);

    if resp.status() != StatusCode::OK {
        log(LogTag::Monitor, "ERROR", &format!("RugCheck API returned status {}", resp.status()));
        return Err(format!("RugCheck API returned status {}", resp.status()).into());
    }

    let arr: serde_json::Value = match resp.json().await {
        Ok(a) => a,
        Err(e) => {
            log(
                LogTag::Monitor,
                "ERROR",
                &format!("Failed to parse RugCheck verified JSON: {}", e)
            );
            return Err(e.to_string());
        }
    };

    let all_items_count = arr
        .as_array()
        .map(|a| a.len())
        .unwrap_or(0);
    let mints = arr
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|item| item.get("mint").and_then(|v| v.as_str()))
        .map(|mint| mint.to_string())
        .collect::<Vec<_>>();

    if debug_mode {
        log(
            LogTag::Monitor,
            "DEBUG",
            &format!("Found {} total items, {} valid mints", all_items_count, mints.len())
        );
        if !mints.is_empty() {
            log(
                LogTag::Monitor,
                "DEBUG",
                &format!("Sample mints: {:?}", &mints[..std::cmp::min(3, mints.len())])
            );
        }
    }

    let mut new_count = 0;
    if let Ok(mut set) = LIST_MINTS.write() {
        for mint in mints {
            if set.insert(mint.clone()) {
                new_count += 1;
                if debug_mode {
                    log(LogTag::Monitor, "DEBUG", &format!("Added new mint: {}", mint));
                }
            }
        }
    }

    if new_count > 0 || debug_mode {
        let log_level = if debug_mode { "DEBUG" } else { "INFO" };
        log(
            LogTag::Monitor,
            log_level,
            &format!("RugCheck Verified New tokens seen: {}", new_count)
        );
    }

    // Sleep to enforce 30 requests per minute (max 2.0 req/sec)
    sleep(Duration::from_millis(2100)).await;
    Ok(())
}

/// Fetch trending tokens from RugCheck API, only filling mint field
pub async fn discovery_rugcheck_fetch_trending() -> Result<(), String> {
    let debug_mode = is_debug_discovery_enabled();

    if debug_mode {
        log(LogTag::Monitor, "DEBUG", "Starting RugCheck trending tokens fetch...");
    }

    // Acquire permit for discovery rate limit (30 per minute)
    let permit = match DISCOVERY_RATE_LIMITER.clone().acquire_owned().await {
        Ok(p) => p,
        Err(e) => {
            log(
                LogTag::Monitor,
                "ERROR",
                &format!("Failed to acquire discovery rate limiter: {}", e)
            );
            return Err(e.to_string());
        }
    };
    let url = "https://api.rugcheck.xyz/v1/stats/trending";

    if debug_mode {
        log(LogTag::Monitor, "DEBUG", &format!("Fetching from URL: {}", url));
    }

    // Create HTTP client with timeout for shutdown responsiveness
    let client = reqwest::Client
        ::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());

    let resp = match client.get(url).send().await {
        Ok(r) => r,
        Err(e) => {
            log(LogTag::Monitor, "ERROR", &format!("Failed to fetch RugCheck trending: {}", e));
            drop(permit);
            return Err(e.to_string());
        }
    };
    drop(permit);

    if resp.status() != StatusCode::OK {
        log(LogTag::Monitor, "ERROR", &format!("RugCheck API returned status {}", resp.status()));
        return Err(format!("RugCheck API returned status {}", resp.status()).into());
    }

    let arr: serde_json::Value = match resp.json().await {
        Ok(a) => a,
        Err(e) => {
            log(
                LogTag::Monitor,
                "ERROR",
                &format!("Failed to parse RugCheck trending JSON: {}", e)
            );
            return Err(e.to_string());
        }
    };

    let all_items_count = arr
        .as_array()
        .map(|a| a.len())
        .unwrap_or(0);
    let mints = arr
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|item| item.get("mint").and_then(|v| v.as_str()))
        .map(|mint| mint.to_string())
        .collect::<Vec<_>>();

    if debug_mode {
        log(
            LogTag::Monitor,
            "DEBUG",
            &format!("Found {} total items, {} valid mints", all_items_count, mints.len())
        );
        if !mints.is_empty() {
            log(
                LogTag::Monitor,
                "DEBUG",
                &format!("Sample mints: {:?}", &mints[..std::cmp::min(3, mints.len())])
            );
        }
    }

    let mut new_count = 0;
    if let Ok(mut set) = LIST_MINTS.write() {
        for mint in mints {
            if set.insert(mint.clone()) {
                new_count += 1;
                if debug_mode {
                    log(LogTag::Monitor, "DEBUG", &format!("Added new mint: {}", mint));
                }
            }
        }
    }

    if new_count > 0 || debug_mode {
        let log_level = if debug_mode { "DEBUG" } else { "INFO" };
        log(
            LogTag::Monitor,
            log_level,
            &format!("RugCheck Trending New tokens seen: {}", new_count)
        );
    }

    // Sleep to enforce 30 requests per minute (max 2.0 req/sec)
    sleep(Duration::from_millis(2100)).await;
    Ok(())
}

/// Fetch recent tokens from RugCheck API, only filling mint field
pub async fn discovery_rugcheck_fetch_recent() -> Result<(), String> {
    let debug_mode = is_debug_discovery_enabled();

    if debug_mode {
        log(LogTag::Monitor, "DEBUG", "Starting RugCheck recent tokens fetch...");
    }

    // Acquire permit for discovery rate limit (30 per minute)
    let permit = match DISCOVERY_RATE_LIMITER.clone().acquire_owned().await {
        Ok(p) => p,
        Err(e) => {
            log(
                LogTag::Monitor,
                "ERROR",
                &format!("Failed to acquire discovery rate limiter: {}", e)
            );
            return Err(e.to_string());
        }
    };
    let url = "https://api.rugcheck.xyz/v1/stats/recent";

    if debug_mode {
        log(LogTag::Monitor, "DEBUG", &format!("Fetching from URL: {}", url));
    }

    // Create HTTP client with timeout for shutdown responsiveness
    let client = reqwest::Client
        ::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());

    let resp = match client.get(url).send().await {
        Ok(r) => r,
        Err(e) => {
            log(LogTag::Monitor, "ERROR", &format!("Failed to fetch RugCheck recent: {}", e));
            drop(permit);
            return Err(e.to_string());
        }
    };
    drop(permit);

    if resp.status() != StatusCode::OK {
        log(LogTag::Monitor, "ERROR", &format!("RugCheck API returned status {}", resp.status()));
        return Err(format!("RugCheck API returned status {}", resp.status()).into());
    }

    let arr: serde_json::Value = match resp.json().await {
        Ok(a) => a,
        Err(e) => {
            log(LogTag::Monitor, "ERROR", &format!("Failed to parse RugCheck recent JSON: {}", e));
            return Err(e.to_string());
        }
    };

    let all_items_count = arr
        .as_array()
        .map(|a| a.len())
        .unwrap_or(0);
    let mints = arr
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|item| item.get("mint").and_then(|v| v.as_str()))
        .map(|mint| mint.to_string())
        .collect::<Vec<_>>();

    if debug_mode {
        log(
            LogTag::Monitor,
            "DEBUG",
            &format!("Found {} total items, {} valid mints", all_items_count, mints.len())
        );
        if !mints.is_empty() {
            log(
                LogTag::Monitor,
                "DEBUG",
                &format!("Sample mints: {:?}", &mints[..std::cmp::min(3, mints.len())])
            );
        }
    }

    let mut new_count = 0;
    if let Ok(mut set) = LIST_MINTS.write() {
        for mint in mints {
            if set.insert(mint.clone()) {
                new_count += 1;
                if debug_mode {
                    log(LogTag::Monitor, "DEBUG", &format!("Added new mint: {}", mint));
                }
            }
        }
    }

    if new_count > 0 || debug_mode {
        let log_level = if debug_mode { "DEBUG" } else { "INFO" };
        log(LogTag::Monitor, log_level, &format!("RugCheck Recent New tokens seen: {}", new_count));
    }

    // Sleep to enforce 30 requests per minute (max 2.0 req/sec)
    sleep(Duration::from_millis(2100)).await;
    Ok(())
}

/// Fetch new tokens from RugCheck API, only filling mint field
pub async fn discovery_rugcheck_fetch_new_tokens() -> Result<(), String> {
    let debug_mode = is_debug_discovery_enabled();

    if debug_mode {
        log(LogTag::Monitor, "DEBUG", "Starting RugCheck new tokens fetch...");
    }

    // Acquire permit for discovery rate limit (30 per minute)
    let permit = match DISCOVERY_RATE_LIMITER.clone().acquire_owned().await {
        Ok(p) => p,
        Err(e) => {
            log(
                LogTag::Monitor,
                "ERROR",
                &format!("Failed to acquire discovery rate limiter: {}", e)
            );
            return Err(e.to_string());
        }
    };
    let url = "https://api.rugcheck.xyz/v1/stats/new_tokens";

    if debug_mode {
        log(LogTag::Monitor, "DEBUG", &format!("Fetching from URL: {}", url));
    }

    // Create HTTP client with timeout for shutdown responsiveness
    let client = reqwest::Client
        ::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());

    let resp = match client.get(url).send().await {
        Ok(r) => r,
        Err(e) => {
            log(LogTag::Monitor, "ERROR", &format!("Failed to fetch RugCheck new tokens: {}", e));
            drop(permit);
            return Err(e.to_string());
        }
    };
    drop(permit);

    if resp.status() != StatusCode::OK {
        log(LogTag::Monitor, "ERROR", &format!("RugCheck API returned status {}", resp.status()));
        return Err(format!("RugCheck API returned status {}", resp.status()).into());
    }

    let arr: serde_json::Value = match resp.json().await {
        Ok(a) => a,
        Err(e) => {
            log(
                LogTag::Monitor,
                "ERROR",
                &format!("Failed to parse RugCheck new tokens JSON: {}", e)
            );
            return Err(e.to_string());
        }
    };

    let all_items_count = arr
        .as_array()
        .map(|a| a.len())
        .unwrap_or(0);
    let mints = arr
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|item| item.get("mint").and_then(|v| v.as_str()))
        .map(|mint| mint.to_string())
        .collect::<Vec<_>>();

    if debug_mode {
        log(
            LogTag::Monitor,
            "DEBUG",
            &format!("Found {} total items, {} valid mints", all_items_count, mints.len())
        );
        if !mints.is_empty() {
            log(
                LogTag::Monitor,
                "DEBUG",
                &format!("Sample mints: {:?}", &mints[..std::cmp::min(3, mints.len())])
            );
        }
    }

    let mut new_count = 0;
    if let Ok(mut set) = LIST_MINTS.write() {
        for mint in mints {
            if set.insert(mint.clone()) {
                new_count += 1;
                if debug_mode {
                    log(LogTag::Monitor, "DEBUG", &format!("Added new mint: {}", mint));
                }
            }
        }
    }

    if new_count > 0 || debug_mode {
        let log_level = if debug_mode { "DEBUG" } else { "INFO" };
        log(
            LogTag::Monitor,
            log_level,
            &format!("RugCheck New Tokens New tokens seen: {}", new_count)
        );
    }

    // Sleep to enforce 30 requests per minute (max 2.0 req/sec)
    sleep(Duration::from_millis(2100)).await;
    Ok(())
}

/// Fetch token info for a single mint address from DexScreener API
pub async fn get_single_token_info(
    mint: &str,
    shutdown: Arc<Notify>
) -> Result<Option<Token>, String> {
    if check_shutdown_or_delay(&shutdown, Duration::from_millis(0)).await {
        return Ok(None);
    }

    // Load configuration and create RPC client for decimal fetching
    let configs = crate::global::read_configs("configs.json").map_err(|e| e.to_string())?;
    let rpc_client = RpcClient::new(&configs.rpc_url);

    // Load decimal cache
    let cache_path = Path::new("decimal_cache.json");
    let mut decimal_cache = match DecimalCache::load_from_file(cache_path) {
        Ok(cache) => cache,
        Err(e) => {
            log(
                LogTag::Monitor,
                "WARN",
                &format!("Failed to load decimal cache: {}, using new cache", e)
            );
            DecimalCache::new()
        }
    };

    // Fetch decimals for the mint
    let decimals_map = fetch_or_cache_decimals(
        &rpc_client,
        &vec![mint.to_string()],
        &mut decimal_cache,
        cache_path
    ).await.map_err(|e| e.to_string())?;

    // Acquire permit for info rate limit (200 per minute)
    let permit = match INFO_RATE_LIMITER.clone().acquire_owned().await {
        Ok(p) => p,
        Err(e) => {
            log(LogTag::Monitor, "ERROR", &format!("Failed to acquire info rate limiter: {}", e));
            return Err(e.to_string());
        }
    };

    let chain_id = "solana";
    let url = format!("https://api.dexscreener.com/tokens/v1/{}/{}", chain_id, mint);

    // Create HTTP client with timeout
    let client = reqwest::Client
        ::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());

    let resp = match client.get(&url).send().await {
        Ok(r) => r,
        Err(e) => {
            log(LogTag::Monitor, "ERROR", &format!("Failed to send single token request: {}", e));
            drop(permit);
            return Err(e.to_string());
        }
    };
    drop(permit); // Release permit immediately after request

    if resp.status() != StatusCode::OK {
        log(
            LogTag::Monitor,
            "WARN",
            &format!("Token {} not found or API error: {}", mint, resp.status())
        );
        return Ok(None);
    }

    let arr: serde_json::Value = resp.json().await.unwrap_or_else(|e| {
        log(
            LogTag::Monitor,
            "ERROR",
            &format!("Failed to parse single token response JSON: {}", e)
        );
        serde_json::json!([])
    });

    if let Some(arr) = arr.as_array() {
        if let Some(pair) = arr.first() {
            if let Some(base_token) = pair.get("baseToken") {
                let token_mint = base_token
                    .get("address")
                    .and_then(|a| a.as_str())
                    .unwrap_or("");

                if token_mint != mint {
                    log(
                        LogTag::Monitor,
                        "WARN",
                        &format!("Requested mint {} but got {}", mint, token_mint)
                    );
                    return Ok(None);
                }

                let created_at = pair
                    .get("pairCreatedAt")
                    .and_then(|v| v.as_i64())
                    .and_then(|ts| chrono::DateTime::from_timestamp_millis(ts));

                // Parse transaction stats
                let txns = pair.get("txns").map(|txns_obj| {
                    crate::global::TxnStats {
                        m5: txns_obj.get("m5").map(|m5| crate::global::TxnPeriod {
                            buys: m5.get("buys").and_then(|v| v.as_i64()),
                            sells: m5.get("sells").and_then(|v| v.as_i64()),
                        }),
                        h1: txns_obj.get("h1").map(|h1| crate::global::TxnPeriod {
                            buys: h1.get("buys").and_then(|v| v.as_i64()),
                            sells: h1.get("sells").and_then(|v| v.as_i64()),
                        }),
                        h6: txns_obj.get("h6").map(|h6| crate::global::TxnPeriod {
                            buys: h6.get("buys").and_then(|v| v.as_i64()),
                            sells: h6.get("sells").and_then(|v| v.as_i64()),
                        }),
                        h24: txns_obj.get("h24").map(|h24| crate::global::TxnPeriod {
                            buys: h24.get("buys").and_then(|v| v.as_i64()),
                            sells: h24.get("sells").and_then(|v| v.as_i64()),
                        }),
                    }
                });

                // Parse volume stats
                let volume = pair.get("volume").map(|vol_obj| {
                    crate::global::VolumeStats {
                        m5: vol_obj.get("m5").and_then(|v| v.as_f64()),
                        h1: vol_obj.get("h1").and_then(|v| v.as_f64()),
                        h6: vol_obj.get("h6").and_then(|v| v.as_f64()),
                        h24: vol_obj.get("h24").and_then(|v| v.as_f64()),
                    }
                });

                // Parse price change stats
                let price_change = pair.get("priceChange").map(|pc_obj| {
                    crate::global::PriceChangeStats {
                        m5: pc_obj.get("m5").and_then(|v| v.as_f64()),
                        h1: pc_obj.get("h1").and_then(|v| v.as_f64()),
                        h6: pc_obj.get("h6").and_then(|v| v.as_f64()),
                        h24: pc_obj.get("h24").and_then(|v| v.as_f64()),
                    }
                });

                // Parse liquidity info
                let liquidity = pair.get("liquidity").map(|liq_obj| {
                    crate::global::LiquidityInfo {
                        usd: liq_obj.get("usd").and_then(|v| v.as_f64()),
                        base: liq_obj.get("base").and_then(|v| v.as_f64()),
                        quote: liq_obj.get("quote").and_then(|v| v.as_f64()),
                    }
                });

                // Parse token info
                let info = pair.get("info").map(|info_obj| {
                    let websites = info_obj
                        .get("websites")
                        .and_then(|w| w.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|website| {
                                    website
                                        .get("url")
                                        .and_then(|url| url.as_str())
                                        .map(|url| {
                                            crate::global::WebsiteLink {
                                                label: website
                                                    .get("label")
                                                    .and_then(|l| l.as_str())
                                                    .map(|s| s.to_string()),
                                                url: url.to_string(),
                                            }
                                        })
                                })
                                .collect()
                        })
                        .unwrap_or_default();

                    let socials = info_obj
                        .get("socials")
                        .and_then(|s| s.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|social| {
                                    let url = social.get("url").and_then(|u| u.as_str())?;
                                    let link_type = social.get("type").and_then(|t| t.as_str())?;
                                    Some(crate::global::SocialLink {
                                        link_type: link_type.to_string(),
                                        url: url.to_string(),
                                    })
                                })
                                .collect()
                        })
                        .unwrap_or_default();

                    crate::global::TokenInfo {
                        image_url: info_obj
                            .get("imageUrl")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string()),
                        header: info_obj
                            .get("header")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string()),
                        open_graph: info_obj
                            .get("openGraph")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string()),
                        websites,
                        socials,
                    }
                });

                // Parse boost info
                let boosts = pair.get("boosts").map(|boost_obj| {
                    crate::global::BoostInfo {
                        active: boost_obj.get("active").and_then(|v| v.as_i64()),
                    }
                });

                // Parse labels
                let labels = pair
                    .get("labels")
                    .and_then(|l| l.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str())
                            .map(|s| s.to_string())
                            .collect()
                    })
                    .unwrap_or_default();

                let token = Token {
                    mint: token_mint.to_string(),
                    symbol: base_token
                        .get("symbol")
                        .and_then(|s| s.as_str())
                        .unwrap_or("")
                        .to_string(),
                    name: base_token
                        .get("name")
                        .and_then(|s| s.as_str())
                        .unwrap_or("")
                        .to_string(),
                    decimals: decimals_map.get(token_mint).copied().unwrap_or(9),
                    chain: "solana".to_string(),

                    // Existing fields - keeping original logic but using info.image_url as primary
                    logo_url: info
                        .as_ref()
                        .and_then(|i| i.image_url.clone())
                        .or_else(|| {
                            pair.get("info")
                                .and_then(|i| i.get("imageUrl"))
                                .and_then(|s| s.as_str())
                                .map(|s| s.to_string())
                        }),
                    coingecko_id: None,
                    website: info
                        .as_ref()
                        .and_then(|i| i.websites.first())
                        .map(|w| w.url.clone())
                        .or_else(|| {
                            pair.get("info")
                                .and_then(|i| i.get("websites"))
                                .and_then(|w| w.as_array())
                                .and_then(|arr| arr.get(0))
                                .and_then(|w| w.get("url"))
                                .and_then(|s| s.as_str())
                                .map(|s| s.to_string())
                        }),
                    description: None,
                    tags: vec![],
                    is_verified: false,
                    created_at,

                    // Price data
                    price_dexscreener_sol: pair
                        .get("priceNative")
                        .and_then(|v| v.as_str())
                        .and_then(|s| s.parse().ok()),
                    price_dexscreener_usd: pair
                        .get("priceUsd")
                        .and_then(|v| v.as_str())
                        .and_then(|s| s.parse().ok()),

                    price_pool_sol: None,
                    price_pool_usd: None,
                    pools: vec![],

                    // New DexScreener fields
                    dex_id: pair
                        .get("dexId")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    pair_address: pair
                        .get("pairAddress")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    pair_url: pair
                        .get("url")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    labels,
                    fdv: pair.get("fdv").and_then(|v| v.as_f64()),
                    market_cap: pair.get("marketCap").and_then(|v| v.as_f64()),
                    txns,
                    volume,
                    price_change,
                    liquidity,
                    info,
                    boosts,
                };

                // Cache the token to database
                match crate::global::cache_token_to_db(&token, "dexscreener") {
                    Ok(is_new) => {
                        log(
                            LogTag::Monitor,
                            "CACHE",
                            &format!("Cached single token {} to DB ({})", token.symbol, if is_new {
                                "new"
                            } else {
                                "updated"
                            })
                                .dimmed()
                                .to_string()
                        );
                    }
                    Err(e) => {
                        log(
                            LogTag::Monitor,
                            "WARN",
                            &format!("Failed to cache single token {} to DB: {}", token.symbol, e)
                                .dimmed()
                                .to_string()
                        );
                    }
                }

                log(
                    LogTag::Monitor,
                    "INFO",
                    &format!("Fetched single token info: {} ({})", token.symbol, token.mint)
                        .dimmed()
                        .to_string()
                );

                return Ok(Some(token));
            }
        }
    }

    log(LogTag::Monitor, "WARN", &format!("No token data found for mint: {}", mint));
    Ok(None)
}

/// Concurrent batched token update function - processes chunks of 30 tokens simultaneously
pub async fn update_tokens_from_mints_concurrent(shutdown: Arc<Notify>) -> Result<(), String> {
    use tokio::sync::Semaphore;
    use std::sync::Arc as StdArc;

    // First, get all mint addresses from open positions to ensure we always update them
    let position_mints = {
        if let Ok(positions) = SAVED_POSITIONS.lock() {
            positions
                .iter()
                .filter(|p| p.exit_time.is_none()) // Only consider open positions
                .map(|p| p.mint.clone())
                .collect::<Vec<String>>()
        } else {
            Vec::new()
        }
    };

    // Log the position mints we're prioritizing
    if !position_mints.is_empty() {
        log(
            LogTag::Monitor,
            "INFO",
            &format!("Prioritizing {} tokens from open positions", position_mints.len())
        );
    }

    // Get all mints from the global list
    let mut mints: Vec<String> = match LIST_MINTS.read() {
        Ok(set) => set.iter().cloned().collect(),
        Err(e) => {
            log(LogTag::Monitor, "ERROR", &format!("Failed to read LIST_MINTS: {}", e));
            return Err(format!("Failed to read LIST_MINTS: {}", e));
        }
    };

    // Make sure all position mints are included in our list
    for mint in &position_mints {
        if !mints.contains(mint) {
            mints.push(mint.clone());
        }
    }

    if mints.is_empty() {
        return Ok(());
    }

    // Load configuration and create RPC client for decimal fetching
    let configs = crate::global::read_configs("configs.json").map_err(|e| e.to_string())?;
    let rpc_client = RpcClient::new(&configs.rpc_url);

    // Load decimal cache
    let cache_path = Path::new("decimal_cache.json");
    let mut decimal_cache = match DecimalCache::load_from_file(cache_path) {
        Ok(cache) => cache,
        Err(e) => {
            log(
                LogTag::Monitor,
                "WARN",
                &format!("Failed to load decimal cache: {}, using new cache", e)
            );
            DecimalCache::new()
        }
    };

    // Fetch decimals for all mints upfront
    let decimals_map = fetch_or_cache_decimals(
        &rpc_client,
        &mints,
        &mut decimal_cache,
        cache_path
    ).await.map_err(|e| e.to_string())?;

    // Reorganize mints: prioritize position mints first
    let mut prioritized_mints = Vec::new();

    // First add all position mints
    for mint in &position_mints {
        prioritized_mints.push(mint.clone());
    }

    // Then add all other mints that aren't in position_mints
    for mint in mints {
        if !position_mints.contains(&mint) {
            prioritized_mints.push(mint);
        }
    }

    // Split into chunks of 30 for concurrent processing
    let chunks: Vec<Vec<String>> = prioritized_mints
        .chunks(30)
        .map(|chunk| chunk.to_vec())
        .collect();

    if chunks.is_empty() {
        return Ok(());
    }

    log(
        LogTag::Monitor,
        "INFO",
        &format!("Processing {} token chunks concurrently (30 tokens per chunk)", chunks.len())
    );

    // Create semaphore to limit concurrent chunk processing (max 5 concurrent chunks)
    let semaphore = StdArc::new(Semaphore::new(5));
    let decimals_map = StdArc::new(decimals_map);
    let position_mints = StdArc::new(position_mints);

    // Process all chunks concurrently
    let chunk_tasks: Vec<_> = chunks
        .into_iter()
        .enumerate()
        .map(|(chunk_index, chunk)| {
            let semaphore = semaphore.clone();
            let decimals_map = decimals_map.clone();
            let position_mints = position_mints.clone();
            let shutdown = shutdown.clone();

            tokio::spawn(async move {
                // Acquire semaphore permit
                let _permit = semaphore
                    .acquire().await
                    .map_err(|e| { format!("Failed to acquire semaphore permit: {}", e) })?;

                if check_shutdown_or_delay(&shutdown, Duration::from_millis(0)).await {
                    return Ok(Vec::new());
                }

                // Check if this chunk contains any position mints
                let contains_positions = chunk.iter().any(|mint| position_mints.contains(mint));

                if contains_positions {
                    log(
                        LogTag::Monitor,
                        "INFO",
                        &format!(
                            "Processing chunk {} with prioritized position tokens",
                            chunk_index + 1
                        )
                            .dimmed()
                            .to_string()
                    );
                }

                // Acquire permit for info rate limit (200 per minute)
                let permit = INFO_RATE_LIMITER.clone()
                    .acquire_owned().await
                    .map_err(|e| { format!("Failed to acquire info rate limiter: {}", e) })?;

                let chain_id = "solana";
                let token_addresses = chunk.join(",");
                let url = format!(
                    "https://api.dexscreener.com/tokens/v1/{}/{}",
                    chain_id,
                    token_addresses
                );

                // Create HTTP client with timeout
                let client = reqwest::Client
                    ::builder()
                    .timeout(Duration::from_secs(5))
                    .build()
                    .unwrap_or_else(|_| reqwest::Client::new());

                let resp = client
                    .get(&url)
                    .send().await
                    .map_err(|e| {
                        format!("Failed to send batch request for chunk {}: {}", chunk_index + 1, e)
                    })?;

                drop(permit); // Release permit immediately after request

                if resp.status() != reqwest::StatusCode::OK {
                    return Err(
                        format!(
                            "API returned status {} for chunk {}",
                            resp.status(),
                            chunk_index + 1
                        )
                    );
                }

                let arr: serde_json::Value = resp
                    .json().await
                    .map_err(|e| {
                        format!(
                            "Failed to parse response JSON for chunk {}: {}",
                            chunk_index + 1,
                            e
                        )
                    })?;

                let mut chunk_tokens = Vec::new();

                if let Some(arr) = arr.as_array() {
                    for pair in arr {
                        if let Some(base_token) = pair.get("baseToken") {
                            let mint = base_token
                                .get("address")
                                .and_then(|a| a.as_str())
                                .unwrap_or("");

                            // Parse all token data similar to the original function
                            let created_at = pair
                                .get("pairCreatedAt")
                                .and_then(|v| v.as_i64())
                                .and_then(|ts| chrono::DateTime::from_timestamp_millis(ts));

                            // Parse transaction stats, volume, price change, liquidity, info, boosts, etc.
                            // (Using the same parsing logic as the original function)
                            let txns = pair.get("txns").map(|txns_obj| {
                                crate::global::TxnStats {
                                    m5: txns_obj.get("m5").map(|m5| crate::global::TxnPeriod {
                                        buys: m5.get("buys").and_then(|v| v.as_i64()),
                                        sells: m5.get("sells").and_then(|v| v.as_i64()),
                                    }),
                                    h1: txns_obj.get("h1").map(|h1| crate::global::TxnPeriod {
                                        buys: h1.get("buys").and_then(|v| v.as_i64()),
                                        sells: h1.get("sells").and_then(|v| v.as_i64()),
                                    }),
                                    h6: txns_obj.get("h6").map(|h6| crate::global::TxnPeriod {
                                        buys: h6.get("buys").and_then(|v| v.as_i64()),
                                        sells: h6.get("sells").and_then(|v| v.as_i64()),
                                    }),
                                    h24: txns_obj.get("h24").map(|h24| crate::global::TxnPeriod {
                                        buys: h24.get("buys").and_then(|v| v.as_i64()),
                                        sells: h24.get("sells").and_then(|v| v.as_i64()),
                                    }),
                                }
                            });

                            let volume = pair.get("volume").map(|vol_obj| {
                                crate::global::VolumeStats {
                                    m5: vol_obj.get("m5").and_then(|v| v.as_f64()),
                                    h1: vol_obj.get("h1").and_then(|v| v.as_f64()),
                                    h6: vol_obj.get("h6").and_then(|v| v.as_f64()),
                                    h24: vol_obj.get("h24").and_then(|v| v.as_f64()),
                                }
                            });

                            let price_change = pair.get("priceChange").map(|pc_obj| {
                                crate::global::PriceChangeStats {
                                    m5: pc_obj.get("m5").and_then(|v| v.as_f64()),
                                    h1: pc_obj.get("h1").and_then(|v| v.as_f64()),
                                    h6: pc_obj.get("h6").and_then(|v| v.as_f64()),
                                    h24: pc_obj.get("h24").and_then(|v| v.as_f64()),
                                }
                            });

                            let liquidity = pair.get("liquidity").map(|liq_obj| {
                                crate::global::LiquidityInfo {
                                    usd: liq_obj.get("usd").and_then(|v| v.as_f64()),
                                    base: liq_obj.get("base").and_then(|v| v.as_f64()),
                                    quote: liq_obj.get("quote").and_then(|v| v.as_f64()),
                                }
                            });

                            let info = pair.get("info").map(|info_obj| {
                                let websites = info_obj
                                    .get("websites")
                                    .and_then(|w| w.as_array())
                                    .map(|arr| {
                                        arr.iter()
                                            .filter_map(|website| {
                                                website
                                                    .get("url")
                                                    .and_then(|url| url.as_str())
                                                    .map(|url| {
                                                        crate::global::WebsiteLink {
                                                            label: website
                                                                .get("label")
                                                                .and_then(|l| l.as_str())
                                                                .map(|s| s.to_string()),
                                                            url: url.to_string(),
                                                        }
                                                    })
                                            })
                                            .collect()
                                    })
                                    .unwrap_or_default();

                                let socials = info_obj
                                    .get("socials")
                                    .and_then(|s| s.as_array())
                                    .map(|arr| {
                                        arr.iter()
                                            .filter_map(|social| {
                                                let url = social
                                                    .get("url")
                                                    .and_then(|u| u.as_str())?;
                                                let link_type = social
                                                    .get("type")
                                                    .and_then(|t| t.as_str())?;
                                                Some(crate::global::SocialLink {
                                                    link_type: link_type.to_string(),
                                                    url: url.to_string(),
                                                })
                                            })
                                            .collect()
                                    })
                                    .unwrap_or_default();

                                crate::global::TokenInfo {
                                    image_url: info_obj
                                        .get("imageUrl")
                                        .and_then(|v| v.as_str())
                                        .map(|s| s.to_string()),
                                    header: info_obj
                                        .get("header")
                                        .and_then(|v| v.as_str())
                                        .map(|s| s.to_string()),
                                    open_graph: info_obj
                                        .get("openGraph")
                                        .and_then(|v| v.as_str())
                                        .map(|s| s.to_string()),
                                    websites,
                                    socials,
                                }
                            });

                            let boosts = pair.get("boosts").map(|boost_obj| {
                                crate::global::BoostInfo {
                                    active: boost_obj.get("active").and_then(|v| v.as_i64()),
                                }
                            });

                            let labels = pair
                                .get("labels")
                                .and_then(|l| l.as_array())
                                .map(|arr| {
                                    arr.iter()
                                        .filter_map(|v| v.as_str())
                                        .map(|s| s.to_string())
                                        .collect()
                                })
                                .unwrap_or_default();

                            let price = pair
                                .get("priceNative")
                                .and_then(|v| v.as_str())
                                .and_then(|s| s.parse::<f64>().ok())
                                .unwrap_or(0.0);

                            let price_usd = pair
                                .get("priceUsd")
                                .and_then(|v| v.as_str())
                                .and_then(|s| s.parse::<f64>().ok())
                                .unwrap_or(0.0);

                            let token = Token {
                                mint: mint.to_string(),
                                symbol: base_token
                                    .get("symbol")
                                    .and_then(|s| s.as_str())
                                    .unwrap_or("")
                                    .to_string(),
                                name: base_token
                                    .get("name")
                                    .and_then(|s| s.as_str())
                                    .unwrap_or("")
                                    .to_string(),
                                decimals: decimals_map.get(mint).copied().unwrap_or(9),
                                chain: "solana".to_string(),

                                logo_url: info.as_ref().and_then(|i| i.image_url.clone()),
                                coingecko_id: None,
                                website: info
                                    .as_ref()
                                    .and_then(|i| i.websites.first())
                                    .map(|w| w.url.clone()),
                                description: None,
                                tags: vec![],
                                is_verified: false,
                                created_at,

                                price_dexscreener_sol: Some(price),
                                price_dexscreener_usd: Some(price_usd),
                                price_pool_sol: None,
                                price_pool_usd: None,
                                pools: vec![],

                                dex_id: pair
                                    .get("dexId")
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string()),
                                pair_address: pair
                                    .get("pairAddress")
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string()),
                                pair_url: pair
                                    .get("url")
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string()),
                                labels,
                                fdv: pair.get("fdv").and_then(|v| v.as_f64()),
                                market_cap: pair.get("marketCap").and_then(|v| v.as_f64()),
                                txns,
                                volume,
                                price_change,
                                liquidity,
                                info,
                                boosts,
                            };

                            chunk_tokens.push(token);
                        }
                    }
                }

                log(
                    LogTag::Monitor,
                    "DEBUG",
                    &format!("Chunk {} processed {} tokens", chunk_index + 1, chunk_tokens.len())
                        .dimmed()
                        .to_string()
                );

                // Sleep to enforce rate limiting (staggered per chunk)
                tokio::time::sleep(Duration::from_millis(300 + ((chunk_index * 50) as u64))).await;

                Ok::<Vec<Token>, String>(chunk_tokens)
            })
        })
        .collect();

    // Wait for all chunks to complete and collect results
    let mut all_tokens = Vec::new();
    let mut successful_chunks = 0;
    let mut failed_chunks = 0;

    for (chunk_index, task) in chunk_tasks.into_iter().enumerate() {
        match task.await {
            Ok(Ok(chunk_tokens)) => {
                all_tokens.extend(chunk_tokens);
                successful_chunks += 1;
            }
            Ok(Err(e)) => {
                log(LogTag::Monitor, "ERROR", &format!("Chunk {} failed: {}", chunk_index + 1, e));
                failed_chunks += 1;
            }
            Err(e) => {
                log(
                    LogTag::Monitor,
                    "ERROR",
                    &format!("Chunk {} task failed: {}", chunk_index + 1, e)
                );
                failed_chunks += 1;
            }
        }
    }

    log(
        LogTag::Monitor,
        "INFO",
        &format!(
            "Concurrent token update completed: {} successful chunks, {} failed chunks, {} total tokens",
            successful_chunks,
            failed_chunks,
            all_tokens.len()
        )
    );

    // Cache all tokens to database before updating LIST_TOKENS
    let mut cached_count = 0;
    let mut new_tokens_count = 0;
    for token in &all_tokens {
        match crate::global::cache_token_to_db(token, "dexscreener") {
            Ok(is_new) => {
                cached_count += 1;
                if is_new {
                    new_tokens_count += 1;
                }
            }
            Err(e) => {
                log(
                    LogTag::Monitor,
                    "WARN",
                    &format!("Failed to cache token {} to DB: {}", token.symbol, e)
                        .dimmed()
                        .to_string()
                );
            }
        }
    }

    if cached_count > 0 {
        log(
            LogTag::Monitor,
            "CACHE",
            &format!("Cached {} tokens to DB ({} new)", cached_count, new_tokens_count)
                .dimmed()
                .to_string()
        );
    }

    // Update LIST_TOKENS
    let mints_count = match LIST_MINTS.read() {
        Ok(set) => set.len(),
        Err(_) => 0,
    };

    match LIST_TOKENS.write() {
        Ok(mut list) => {
            let total_tokens = all_tokens.len();
            let with_liquidity = all_tokens
                .iter()
                .filter(|token| {
                    token.liquidity
                        .as_ref()
                        .and_then(|l| l.usd)
                        .unwrap_or(0.0) > 0.0
                })
                .count();
            let zero_liquidity = total_tokens - with_liquidity;

            log(
                LogTag::Monitor,
                "DEBUG",
                &format!(
                    "Concurrent update LIST_TOKENS: {} total tokens, {} with liquidity, {} with zero liquidity",
                    total_tokens,
                    with_liquidity,
                    zero_liquidity
                )
                    .dimmed()
                    .to_string()
            );

            *list = all_tokens;

            // Count how many position tokens were successfully updated
            let position_tokens_updated = if !position_mints.is_empty() {
                list.iter()
                    .filter(|token| position_mints.contains(&token.mint))
                    .count()
            } else {
                0
            };

            // Enhanced logging to show position tokens status
            if !position_mints.is_empty() {
                log(
                    LogTag::Monitor,
                    "SUCCESS",
                    &format!(
                        "Concurrent Dexscreener Updated tokens: {}, mints: {}, position tokens: {}/{}",
                        list.len(),
                        mints_count,
                        position_tokens_updated,
                        position_mints.len()
                    )
                );

                // Check if any position tokens are missing and log a warning
                if position_tokens_updated < position_mints.len() {
                    log(
                        LogTag::Monitor,
                        "WARN",
                        &format!(
                            "Failed to update {}/{} position tokens! Will retry in next cycle.",
                            position_mints.len() - position_tokens_updated,
                            position_mints.len()
                        )
                    );
                }
            } else {
                log(
                    LogTag::Monitor,
                    "SUCCESS",
                    &format!(
                        "Concurrent Dexscreener Updated tokens: {}, mints: {}",
                        list.len(),
                        mints_count
                    )
                        .dimmed()
                        .to_string()
                );
            }
        }
        Err(e) => {
            log(LogTag::Monitor, "ERROR", &format!("Failed to write LIST_TOKENS: {}", e));
            return Err(format!("Failed to write LIST_TOKENS: {}", e));
        }
    }

    Ok(())
}
