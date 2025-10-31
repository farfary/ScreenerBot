/// API conversion functions - transform external API types to TokenPoolInfo
use crate::apis::dexscreener::types::DexScreenerPool;
use crate::apis::geckoterminal::types::GeckoTerminalPool;
use crate::pools::utils::is_sol_mint;
use crate::tokens::types::{TokenPoolInfo, TokenPoolSources};

use super::utils::{parse_f64, parse_gecko_token_id};

/// Convert DexScreener pool to TokenPoolInfo
pub fn from_dexscreener(pool: &DexScreenerPool) -> Option<TokenPoolInfo> {
    if pool.pair_address.trim().is_empty() {
        return None;
    }

    let base_mint = pool.base_token_address.trim();
    let quote_mint = pool.quote_token_address.trim();

    if base_mint.is_empty() || quote_mint.is_empty() {
        return None;
    }

    let price_usd = parse_f64(&pool.price_usd);
    let price_sol = parse_f64(&pool.price_native);
    let price_native = if pool.price_native.trim().is_empty() {
        None
    } else {
        Some(pool.price_native.clone())
    };

    let liquidity_token = pool.liquidity_base;
    let liquidity_sol = if is_sol_mint(quote_mint) {
        pool.liquidity_quote
    } else if is_sol_mint(base_mint) {
        pool.liquidity_base
    } else {
        None
    };

    let sources = TokenPoolSources {
        dexscreener: serde_json::to_value(pool).ok(),
        ..TokenPoolSources::default()
    };

    Some(TokenPoolInfo {
        pool_address: pool.pair_address.clone(),
        dex: if pool.dex_id.trim().is_empty() {
            None
        } else {
            Some(pool.dex_id.clone())
        },
        base_mint: base_mint.to_string(),
        quote_mint: quote_mint.to_string(),
        is_sol_pair: is_sol_mint(base_mint) || is_sol_mint(quote_mint),
        liquidity_usd: pool.liquidity_usd,
        liquidity_token,
        liquidity_sol,
        volume_h24: pool.volume_h24,
        price_usd,
        price_sol,
        price_native,
        sources,
        pool_data_last_fetched_at: chrono::Utc::now(),
        pool_data_first_seen_at: chrono::Utc::now(),
    })
}

/// Convert GeckoTerminal pool to TokenPoolInfo
pub fn from_geckoterminal(pool: &GeckoTerminalPool, sol_price_usd: f64) -> Option<TokenPoolInfo> {
    if pool.pool_address.trim().is_empty() {
        return None;
    }

    let base_mint =
        parse_gecko_token_id(&pool.base_token_id).unwrap_or_else(|| pool.base_token_id.clone());
    let quote_mint =
        parse_gecko_token_id(&pool.quote_token_id).unwrap_or_else(|| pool.quote_token_id.clone());

    if base_mint.is_empty() || quote_mint.is_empty() {
        return None;
    }

    let is_sol_pair = is_sol_mint(&base_mint) || is_sol_mint(&quote_mint);

    let (price_native_str, price_usd) = if pool.mint == base_mint {
        (
            if pool.base_token_price_native.trim().is_empty() {
                None
            } else {
                Some(pool.base_token_price_native.clone())
            },
            parse_f64(&pool.base_token_price_usd).or_else(|| parse_f64(&pool.token_price_usd)),
        )
    } else if pool.mint == quote_mint {
        (
            if pool.quote_token_price_native.trim().is_empty() {
                None
            } else {
                Some(pool.quote_token_price_native.clone())
            },
            parse_f64(&pool.quote_token_price_usd).or_else(|| parse_f64(&pool.token_price_usd)),
        )
    } else {
        (
            if pool.base_token_price_native.trim().is_empty() {
                None
            } else {
                Some(pool.base_token_price_native.clone())
            },
            parse_f64(&pool.token_price_usd),
        )
    };

    let price_sol = price_native_str
        .as_ref()
        .and_then(|value| parse_f64(value))
        .or_else(|| {
            price_usd.and_then(|usd| {
                if sol_price_usd > 0.0 {
                    Some(usd / sol_price_usd)
                } else {
                    None
                }
            })
        });

    let liquidity_usd = pool.reserve_usd;
    let liquidity_sol = if is_sol_pair && sol_price_usd > 0.0 {
        liquidity_usd.map(|usd| (usd / 2.0) / sol_price_usd)
    } else {
        None
    };

    let sources = TokenPoolSources {
        geckoterminal: serde_json::to_value(pool).ok(),
        ..TokenPoolSources::default()
    };

    Some(TokenPoolInfo {
        pool_address: pool.pool_address.clone(),
        dex: if pool.dex_id.trim().is_empty() {
            None
        } else {
            Some(pool.dex_id.clone())
        },
        base_mint,
        quote_mint,
        is_sol_pair,
        liquidity_usd,
        liquidity_token: None,
        liquidity_sol,
        volume_h24: pool.volume_h24,
        price_usd,
        price_sol,
        price_native: price_native_str,
        sources,
        pool_data_last_fetched_at: chrono::Utc::now(),
        pool_data_first_seen_at: chrono::Utc::now(),
    })
}
