// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Neutral-facing feeds over the Solana-only Jupiter client.
//!
//! A-02 moved this provider under the Solana adapter; the neutral consumers
//! (tokens discovery, featured boards, the SOL-price fallback) reach it through
//! fn seams installed by `crate::run::services::register_all_services`.
//! A-12a/A-12c delete the discovery and OHLCV sides when those domains thread
//! the chain through.

use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, LazyLock};
use std::time::Duration;

use serde::{Deserialize, Serialize};

use super::JupiterClient;
use crate::errors::{DataError, NetworkError};
use crate::tokens::discovery::DiscoveryRecord;
use crate::tokens::Error;
use crate::webserver::routes::featured::ExternalToken;
// The discovery feeds keep their origin `Error` (crate::tokens::Error); the
// featured boards' origin `Error` (crate::webserver::Error) is aliased because
// both moved bodies meet in this file.
use crate::webserver::Error as WebError;

/// Client singleton (construction moved from `apis::manager.rs`; identical
/// config logic, identical warning text).
static CLIENT: LazyLock<Arc<JupiterClient>> = LazyLock::new(|| {
    let cfg = crate::config::get_config_clone();
    let jup_enabled = cfg.tokens.discovery.enabled && cfg.tokens.discovery.jupiter.enabled;
    Arc::new(JupiterClient::new(jup_enabled).unwrap_or_else(|e| {
        crate::logger::warning(
            crate::logger::LogTag::Api,
            &format!(
                "Failed to initialize Jupiter client: {} - using disabled client",
                e
            ),
        );
        JupiterClient::new(false).expect("Failed to create disabled Jupiter client")
    }))
});

/// The process-wide Jupiter client (one rate limiter, one stats tracker).
pub fn client() -> Arc<JupiterClient> {
    Arc::clone(&CLIENT)
}

// ── tokens discovery feeds ───────────────────────────────────────────────────

pub fn recent() -> Pin<Box<dyn Future<Output = crate::tokens::Result<Vec<DiscoveryRecord>>> + Send>>
{
    Box::pin(fetch_recent())
}

async fn fetch_recent() -> crate::tokens::Result<Vec<DiscoveryRecord>> {
    let tokens = client()
        .fetch_recent_tokens()
        .await
        .map_err(|e| Error::Api {
            provider: "Jupiter".to_owned(),
            message: e.to_string(),
        })?;

    Ok(tokens
        .into_iter()
        .map(|token| DiscoveryRecord {
            mint: token.id,
            symbol: Some(token.symbol),
            name: Some(token.name),
            decimals: Some(token.decimals),
        })
        .collect())
}

pub fn top_organic(
) -> Pin<Box<dyn Future<Output = crate::tokens::Result<Vec<DiscoveryRecord>>> + Send>> {
    Box::pin(fetch_top_organic())
}

async fn fetch_top_organic() -> crate::tokens::Result<Vec<DiscoveryRecord>> {
    let tokens = client()
        .fetch_top_organic_score("1h", None)
        .await
        .map_err(|e| Error::Api {
            provider: "Jupiter".to_owned(),
            message: e.to_string(),
        })?;

    Ok(tokens
        .into_iter()
        .map(|token| DiscoveryRecord {
            mint: token.id,
            symbol: Some(token.symbol),
            name: Some(token.name),
            decimals: Some(token.decimals),
        })
        .collect())
}

pub fn top_traded(
) -> Pin<Box<dyn Future<Output = crate::tokens::Result<Vec<DiscoveryRecord>>> + Send>> {
    Box::pin(fetch_top_traded())
}

async fn fetch_top_traded() -> crate::tokens::Result<Vec<DiscoveryRecord>> {
    let tokens = client()
        .fetch_top_traded("1h", None)
        .await
        .map_err(|e| Error::Api {
            provider: "Jupiter".to_owned(),
            message: e.to_string(),
        })?;

    Ok(tokens
        .into_iter()
        .map(|token| DiscoveryRecord {
            mint: token.id,
            symbol: Some(token.symbol),
            name: Some(token.name),
            decimals: Some(token.decimals),
        })
        .collect())
}

pub fn top_trending(
) -> Pin<Box<dyn Future<Output = crate::tokens::Result<Vec<DiscoveryRecord>>> + Send>> {
    Box::pin(fetch_top_trending())
}

async fn fetch_top_trending() -> crate::tokens::Result<Vec<DiscoveryRecord>> {
    let tokens = client()
        .fetch_top_trending("1h", None)
        .await
        .map_err(|e| Error::Api {
            provider: "Jupiter".to_owned(),
            message: e.to_string(),
        })?;

    Ok(tokens
        .into_iter()
        .map(|token| DiscoveryRecord {
            mint: token.id,
            symbol: Some(token.symbol),
            name: Some(token.name),
            decimals: Some(token.decimals),
        })
        .collect())
}

// ── featured boards ──────────────────────────────────────────────────────────

pub fn featured_organic(
) -> Pin<Box<dyn Future<Output = crate::webserver::Result<Vec<ExternalToken>>> + Send>> {
    Box::pin(featured_organic_impl())
}

async fn featured_organic_impl() -> crate::webserver::Result<Vec<ExternalToken>> {
    if !client().is_enabled() {
        return Ok(vec![]);
    }

    let tokens = client()
        .fetch_top_organic_score("24h", Some(20))
        .await
        .map_err(|source| WebError::Api {
            operation: "Jupiter organic fetch",
            source,
        })?;

    Ok(tokens
        .into_iter()
        .map(|t| ExternalToken {
            mint: t.id,
            name: t.name,
            symbol: t.symbol,
            logo: t.icon,
            website: None,
            twitter: None,
            telegram: None,
            discord: None,
            price_usd: t.usd_price,
            volume_24h: t.stats24h.as_ref().map(|s| {
                let buy = s.buy_volume.unwrap_or_default();
                let sell = s.sell_volume.unwrap_or_default();
                buy + sell
            }),
            liquidity: t.liquidity,
            organic_score: t.organic_score,
        })
        .collect())
}

pub fn featured_traded(
) -> Pin<Box<dyn Future<Output = crate::webserver::Result<Vec<ExternalToken>>> + Send>> {
    Box::pin(featured_traded_impl())
}

async fn featured_traded_impl() -> crate::webserver::Result<Vec<ExternalToken>> {
    if !client().is_enabled() {
        return Ok(vec![]);
    }

    let tokens = client()
        .fetch_top_traded("24h", Some(20))
        .await
        .map_err(|source| WebError::Api {
            operation: "Jupiter traded fetch",
            source,
        })?;

    Ok(tokens
        .into_iter()
        .map(|t| ExternalToken {
            mint: t.id,
            name: t.name,
            symbol: t.symbol,
            logo: t.icon,
            website: None,
            twitter: None,
            telegram: None,
            discord: None,
            price_usd: t.usd_price,
            volume_24h: t.stats24h.as_ref().map(|s| {
                let buy = s.buy_volume.unwrap_or_default();
                let sell = s.sell_volume.unwrap_or_default();
                buy + sell
            }),
            liquidity: t.liquidity,
            organic_score: t.organic_score,
        })
        .collect())
}

// ── SOL-price fallback ───────────────────────────────────────────────────────

/// Jupiter price endpoint — LAST-RESORT fallback (shares the swap rate budget).
fn jupiter_price_api() -> String {
    format!(
        "https://lite-api.jup.ag/price/v3?ids={}",
        crate::chains::adapter().native_asset_address()
    )
}

/// Request timeout in seconds
const REQUEST_TIMEOUT_SECS: u64 = 10;

/// Jupiter API price response structure — keyed by mint address. Serde cannot
/// compute a `rename` at runtime, so the native asset is looked up by key at
/// the call site instead of being a named field.
pub type JupiterPriceResponse = std::collections::HashMap<String, JupiterTokenPrice>;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct JupiterTokenPrice {
    #[serde(rename = "usdPrice")]
    pub usd_price: f64,
    #[serde(rename = "blockId")]
    pub block_id: u64,
    pub decimals: u8,
    #[serde(rename = "priceChange24h")]
    pub price_change_24h: f64,
}

pub fn price_fallback() -> Pin<Box<dyn Future<Output = Result<f64, crate::apis::Error>> + Send>> {
    Box::pin(fetch_sol_price_from_jupiter())
}

/// LAST-RESORT: fetch SOL price from Jupiter API
async fn fetch_sol_price_from_jupiter() -> Result<f64, crate::apis::Error> {
    // Yield to in-flight swaps and space against other background Jupiter calls
    // so price polling never starves the swap rate budget (lite-api is per-IP).
    crate::chains::solana::apis::jupiter::throttle::acquire_background().await;

    let client = crate::net::client();

    let response = client
        .get(jupiter_price_api())
        .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .send()
        .await
        .map_err(|e| NetworkError::RequestFailed {
            endpoint: "jupiter sol price".to_owned(),
            detail: e.to_string(),
        })?;

    if !response.status().is_success() {
        return Err(NetworkError::HttpStatus {
            endpoint: "jupiter sol price".to_owned(),
            status: response.status().as_u16(),
            body: None,
        }
        .into());
    }

    let price_response: JupiterPriceResponse =
        response.json().await.map_err(|e| DataError::ParseError {
            data_type: "jupiter sol price".to_owned(),
            error: e.to_string(),
        })?;

    // Extract the native asset's price from the response, keyed by mint. A
    // missing key gets the same error-propagation treatment as any other parse
    // failure on this path — no unwrap/panic and no silent zero substitution.
    let sol_price = price_response
        .get(crate::chains::adapter().native_asset_address())
        .ok_or_else(|| DataError::InvalidFormat {
            expected: "native asset price".to_owned(),
            received: "missing from Jupiter response".to_owned(),
        })?
        .usd_price;

    if sol_price > 0.0 && sol_price.is_finite() {
        Ok(sol_price)
    } else {
        Err(DataError::ValidationError {
            field: "sol_price".to_owned(),
            value: sol_price.to_string(),
            reason: "not a positive finite value".to_owned(),
        }
        .into())
    }
}
