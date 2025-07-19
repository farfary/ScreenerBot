use crate::logger::{ log, LogTag };
use crate::global::*;
use std::sync::Arc;
use tokio::sync::Notify;
use reqwest::StatusCode;
use tokio::sync::Semaphore;
use tokio::time::{ sleep, Duration };
use crate::utils::check_shutdown_or_delay;

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
    let mints: Vec<String> = match LIST_MINTS.read() {
        Ok(set) => set.iter().cloned().collect(),
        Err(e) => {
            log(LogTag::Monitor, "ERROR", &format!("Failed to read LIST_MINTS: {}", e));
            return Err(Box::new(e));
        }
    };
    let mut tokens = Vec::new();

    for chunk in mints.chunks(30) {
        if check_shutdown_or_delay(&shutdown, Duration::from_millis(500)).await {
            log(LogTag::Monitor, "INFO", "update_tokens_from_mints task shutting down...");
            return Ok(());
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
        let resp = match reqwest::Client::new().get(&url).send().await {
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
                            decimals: 9,
                            chain: "solana".to_string(),
                            logo_url: pair
                                .get("info")
                                .and_then(|i| i.get("imageUrl"))
                                .and_then(|s| s.as_str())
                                .map(|s| s.to_string()),
                            coingecko_id: None,
                            website: pair
                                .get("info")
                                .and_then(|i| i.get("websites"))
                                .and_then(|w| w.as_array())
                                .and_then(|arr| arr.get(0))
                                .and_then(|w| w.get("url"))
                                .and_then(|s| s.as_str())
                                .map(|s| s.to_string()),
                            description: None,
                            tags: vec![],
                            is_verified: false,
                            created_at,
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
            log(
                LogTag::Monitor,
                "INFO",
                &format!("[Dexscreener] Updated tokens: {}, mints: {}", list.len(), mints_count)
            );
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
    let resp = match reqwest::get(url).await {
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
    let resp = match reqwest::get(url).await {
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
    let resp = match reqwest::get(url).await {
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
