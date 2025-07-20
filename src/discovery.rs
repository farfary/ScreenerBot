use crate::logger::{ log, LogTag };
use crate::global::*;
use crate::decimal_cache::{ DecimalCache, fetch_or_cache_decimals };
use std::sync::Arc;
use tokio::sync::Notify;
use reqwest::StatusCode;
use tokio::sync::Semaphore;
use tokio::time::{ sleep, Duration };
use crate::utils::check_shutdown_or_delay;
use solana_client::rpc_client::RpcClient;
use std::path::Path;
use crate::trader::SAVED_POSITIONS;

static INFO_RATE_LIMITER: once_cell::sync::Lazy<Arc<Semaphore>> = once_cell::sync::Lazy::new(||
    Arc::new(Semaphore::new(200))
);
static DISCOVERY_RATE_LIMITER: once_cell::sync::Lazy<Arc<Semaphore>> = once_cell::sync::Lazy::new(||
    Arc::new(Semaphore::new(30))
);

/// For each mint in LIST_MINTS, fetch token info and update LIST_TOKENS
pub async fn update_tokens_from_mints(
    shutdown: Arc<Notify>
) -> Result<(), Box<dyn std::error::Error>> {
    // First, get all mint addresses from open positions to ensure we always update them
    let position_mints = {
        if let Ok(positions) = crate::trader::SAVED_POSITIONS.lock() {
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
            return Err(Box::new(e));
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
    let configs = crate::global::read_configs("configs.json")?;
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
    ).await?;

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
    for chunk in prioritized_mints.chunks(30) {
        if check_shutdown_or_delay(&shutdown, Duration::from_millis(500)).await {
            log(LogTag::Monitor, "INFO", "update_tokens_from_mints task shutting down...");
            return Ok(());
        }

        // Check if this chunk contains any position mints
        let contains_positions = chunk.iter().any(|mint| position_mints.contains(mint));

        // Log priority info if this chunk contains position mints
        if contains_positions {
            log(
                LogTag::Monitor,
                "INFO",
                &format!("Processing chunk with prioritized position tokens")
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
                return Err(Box::new(e));
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
                            price_dexscreener_sol: pair
                                .get("priceNative")
                                .and_then(|v| v.as_str())
                                .and_then(|s| s.parse().ok()),
                            price_dexscreener_usd: pair
                                .get("priceUsd")
                                .and_then(|v| v.as_str())
                                .and_then(|s| s.parse().ok()),
                            price_geckoterminal_sol: None,
                            price_geckoterminal_usd: None,
                            price_raydium_sol: None,
                            price_raydium_usd: None,
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
    // Update LIST_TOKENS
    let mints_count = match LIST_MINTS.read() {
        Ok(set) => set.len(),
        Err(_) => 0,
    };
    match LIST_TOKENS.write() {
        Ok(mut list) => {
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
                log(
                    LogTag::Monitor,
                    "INFO",
                    &format!(
                        "[Dexscreener] Updated tokens: {}, mints: {}, position tokens: {}/{}",
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
                    "INFO",
                    &format!("[Dexscreener] Updated tokens: {}, mints: {}", list.len(), mints_count)
                );
            }
        }
        Err(e) => {
            log(LogTag::Monitor, "ERROR", &format!("Failed to write LIST_TOKENS: {}", e));
            return Err(Box::new(e));
        }
    }
    Ok(())
}

// End of update_tokens_from_mints
/// Fetch token boosts from Dexscreener API, only filling mint field
pub async fn discovery_dexscreener_fetch_token_boosts() -> Result<(), Box<dyn std::error::Error>> {
    // Acquire permit for discovery rate limit (30 per minute)
    let permit = match DISCOVERY_RATE_LIMITER.clone().acquire_owned().await {
        Ok(p) => p,
        Err(e) => {
            log(
                LogTag::Monitor,
                "ERROR",
                &format!("Failed to acquire discovery rate limiter: {}", e)
            );
            return Err(Box::new(e));
        }
    };
    let url = "https://api.dexscreener.com/token-boosts/latest/v1";

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
            return Err(Box::new(e));
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
            return Err(Box::new(e));
        }
    };
    let mints = arr
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter(|item| item.get("chainId").and_then(|v| v.as_str()) == Some("solana"))
        .filter_map(|item| item.get("tokenAddress").and_then(|v| v.as_str()))
        .map(|mint| mint.to_string())
        .collect::<Vec<_>>();
    let mut new_count = 0;
    if let Ok(mut set) = LIST_MINTS.write() {
        for mint in mints {
            if set.insert(mint) {
                new_count += 1;
            }
        }
    }
    if new_count > 0 {
        crate::logger::log(
            crate::logger::LogTag::Monitor,
            "INFO",
            &format!("[Dexscreener Boosts] New tokens seen: {}", new_count)
        );
    }
    // Sleep to enforce 30 requests per minute (max 2.0 req/sec)
    sleep(Duration::from_millis(2100)).await;
    Ok(())
}

/// Fetch token boosts from Dexscreener TOP API, only filling mint field
pub async fn discovery_dexscreener_fetch_token_boosts_top() -> Result<
    (),
    Box<dyn std::error::Error>
> {
    // Acquire permit for discovery rate limit (30 per minute)
    let permit = match DISCOVERY_RATE_LIMITER.clone().acquire_owned().await {
        Ok(p) => p,
        Err(e) => {
            log(
                LogTag::Monitor,
                "ERROR",
                &format!("Failed to acquire discovery rate limiter: {}", e)
            );
            return Err(Box::new(e));
        }
    };
    let url = "https://api.dexscreener.com/token-boosts/top/v1";

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
            return Err(Box::new(e));
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
            return Err(Box::new(e));
        }
    };
    let mints = arr
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter(|item| item.get("chainId").and_then(|v| v.as_str()) == Some("solana"))
        .filter_map(|item| item.get("tokenAddress").and_then(|v| v.as_str()))
        .map(|mint| mint.to_string())
        .collect::<Vec<_>>();
    let mut new_count = 0;
    if let Ok(mut set) = LIST_MINTS.write() {
        for mint in mints {
            if set.insert(mint) {
                new_count += 1;
            }
        }
    }
    if new_count > 0 {
        crate::logger::log(
            crate::logger::LogTag::Monitor,
            "INFO",
            &format!("[Dexscreener Boosts Top] New tokens seen: {}", new_count)
        );
    }
    // Sleep to enforce 30 requests per minute (max 2.0 req/sec)
    sleep(Duration::from_millis(2100)).await;
    Ok(())
}

/// Fetch token profiles from Dexscreener API, only filling mint field
pub async fn discovery_dexscreener_fetch_token_profiles() -> Result<
    (),
    Box<dyn std::error::Error>
> {
    // Acquire permit for discovery rate limit (30 per minute)
    let permit = match DISCOVERY_RATE_LIMITER.clone().acquire_owned().await {
        Ok(p) => p,
        Err(e) => {
            log(
                LogTag::Monitor,
                "ERROR",
                &format!("Failed to acquire discovery rate limiter: {}", e)
            );
            return Err(Box::new(e));
        }
    };
    let url = "https://api.dexscreener.com/token-profiles/latest/v1";

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
            return Err(Box::new(e));
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
            return Err(Box::new(e));
        }
    };
    let mints = arr
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter(|item| item.get("chainId").and_then(|v| v.as_str()) == Some("solana"))
        .filter_map(|item| item.get("tokenAddress").and_then(|v| v.as_str()))
        .map(|mint| mint.to_string())
        .collect::<Vec<_>>();
    let mut new_count = 0;
    if let Ok(mut set) = LIST_MINTS.write() {
        for mint in mints {
            if set.insert(mint) {
                new_count += 1;
            }
        }
    }
    if new_count > 0 {
        crate::logger::log(
            crate::logger::LogTag::Monitor,
            "INFO",
            &format!("[Dexscreener Profiles] New tokens seen: {}", new_count)
        );
    }
    // Sleep to enforce 30 requests per minute (max 2.0 req/sec)
    sleep(Duration::from_millis(2100)).await;
    Ok(())
}
