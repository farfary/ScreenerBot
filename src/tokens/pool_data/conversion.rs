// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! API conversion functions - transform external API types to TokenPoolInfo.

use crate::apis::dexscreener::types::DexScreenerPool;
use crate::apis::geckoterminal::types::GeckoTerminalPool;
use crate::chains::{adapter_for, ChainId};
use crate::tokens::types::{TokenPoolInfo, TokenPoolSources};

use super::utils::{parse_f64, parse_gecko_token_id};

/// Convert a DexScreener pair on `chain` to TokenPoolInfo oriented to the requested `mint`:
/// `base_mint` is `mint` and `quote_mint` its counter asset, the shape the data server's rows
/// have. DexScreener sizes and prices a pair from its own base (`priceNative` is the base priced
/// in the quote, `priceUsd` the base in USD), so the token's side is read from whichever side
/// `mint` is on. A native price is set only when it is the token's price in the chain's native
/// coin; a token listed as the quote has no USD price in the row. `None` when `mint` is on
/// neither side of the pair.
pub fn from_dexscreener(
    chain: ChainId,
    mint: &str,
    pool: &DexScreenerPool,
) -> Option<TokenPoolInfo> {
    if pool.pair_address.trim().is_empty() {
        return None;
    }

    let pair_base = pool.base_token_address.trim();
    let pair_quote = pool.quote_token_address.trim();

    if pair_base.is_empty() || pair_quote.is_empty() {
        return None;
    }

    let (counter_mint, token_is_base, liquidity_token, liquidity_counter) = if pair_base == mint {
        (pair_quote, true, pool.liquidity_base, pool.liquidity_quote)
    } else if pair_quote == mint {
        (pair_base, false, pool.liquidity_quote, pool.liquidity_base)
    } else {
        return None;
    };

    let adapter = adapter_for(chain);
    let counter_is_native = adapter.is_native_asset(counter_mint);
    let pair_price = parse_f64(&pool.price_native).filter(|p| p.is_finite() && *p > 0.0);

    let (price_sol, price_native, price_usd) = match (token_is_base, counter_is_native) {
        (true, true) => (
            pair_price,
            pair_price.map(|_| pool.price_native.trim().to_string()),
            parse_f64(&pool.price_usd),
        ),
        (true, false) => (None, None, parse_f64(&pool.price_usd)),
        (false, true) => {
            let price = pair_price.map(|p| 1.0 / p);
            (price, price.map(|p| p.to_string()), None)
        }
        (false, false) => (None, None, None),
    };

    let liquidity_native = if counter_is_native {
        liquidity_counter
    } else if adapter.is_native_asset(mint) {
        liquidity_token
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
        base_mint: mint.to_string(),
        quote_mint: counter_mint.to_string(),
        is_native_pair: counter_is_native || adapter.is_native_asset(mint),
        liquidity_usd: pool.liquidity_usd,
        liquidity_token,
        liquidity_native,
        volume_h24: pool.volume_h24,
        price_usd,
        price_sol,
        price_native,
        sources,
        pool_data_last_fetched_at: chrono::Utc::now(),
        pool_data_first_seen_at: chrono::Utc::now(),
    })
}

/// Convert a GeckoTerminal pool on `chain` to TokenPoolInfo
pub fn from_geckoterminal(
    chain: ChainId,
    pool: &GeckoTerminalPool,
    native_price_usd: f64,
) -> Option<TokenPoolInfo> {
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

    let adapter = adapter_for(chain);
    let is_native_pair =
        adapter.is_native_asset(&base_mint) || adapter.is_native_asset(&quote_mint);

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
                if native_price_usd > 0.0 {
                    Some(usd / native_price_usd)
                } else {
                    None
                }
            })
        });

    let liquidity_usd = pool.reserve_usd;
    let liquidity_native = if is_native_pair && native_price_usd > 0.0 {
        liquidity_usd.map(|usd| (usd / 2.0) / native_price_usd)
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
        is_native_pair,
        liquidity_usd,
        liquidity_token: None,
        liquidity_native,
        volume_h24: pool.volume_h24,
        price_usd,
        price_sol,
        price_native: price_native_str,
        sources,
        pool_data_last_fetched_at: chrono::Utc::now(),
        pool_data_first_seen_at: chrono::Utc::now(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SOL: &str = "So11111111111111111111111111111111111111112";
    const USDC: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
    const MEME: &str = "MemeMint1111111111111111111111111111111111";

    /// A DexScreener pair as `token-pairs/v1` reports it: sizes and prices read from the base.
    fn pair(base: &str, quote: &str, price_native: &str, price_usd: &str) -> DexScreenerPool {
        DexScreenerPool {
            pair_address: "PairAddr111".to_string(),
            base_token_address: base.to_string(),
            quote_token_address: quote.to_string(),
            dex_id: "raydium".to_string(),
            price_native: price_native.to_string(),
            price_usd: price_usd.to_string(),
            liquidity_usd: Some(20_000.0),
            liquidity_base: Some(1_000_000.0),
            liquidity_quote: Some(50.0),
            ..DexScreenerPool::default()
        }
    }

    /// `TokenPoolInfo.dex` (persisted as `token_pools.dex`) is a third-party
    /// diagnostic label sourced verbatim from DexScreener's own `dex_id` —
    /// it is never routed through `ProgramKind`/`ProtocolId`. This pins that
    /// boundary: an arbitrary/unknown upstream label must survive unchanged,
    /// not get normalized, replaced or dropped by protocol-identity logic.
    #[test]
    fn dex_field_preserves_third_party_label_verbatim() {
        let pool = DexScreenerPool {
            pair_address: "PoolAddr111".to_string(),
            base_token_address: "BaseMint111".to_string(),
            quote_token_address: "QuoteMint111".to_string(),
            dex_id: "some_future_unlisted_dex".to_string(),
            ..DexScreenerPool::default()
        };

        let info =
            from_dexscreener(ChainId::Solana, "BaseMint111", &pool).expect("valid pool converts");
        assert_eq!(info.dex.as_deref(), Some("some_future_unlisted_dex"));
    }

    #[test]
    fn a_token_on_the_base_side_keeps_the_pair_figures() {
        let info = from_dexscreener(ChainId::Solana, MEME, &pair(MEME, SOL, "0.00005", "0.01"))
            .expect("token is the base");
        assert_eq!(
            (info.base_mint.as_str(), info.quote_mint.as_str()),
            (MEME, SOL)
        );
        assert!(info.is_native_pair);
        assert_eq!(info.liquidity_token, Some(1_000_000.0));
        assert_eq!(info.liquidity_native, Some(50.0));
        assert_eq!(info.price_sol, Some(0.00005));
        assert_eq!(info.price_native.as_deref(), Some("0.00005"));
        assert_eq!(info.price_usd, Some(0.01));
    }

    #[test]
    fn a_token_quoted_against_the_native_coin_is_read_from_the_quote_side() {
        // SOL/MEME: DexScreener prices SOL in MEME (20 000 MEME per SOL) and in USD.
        let mut row = pair(SOL, MEME, "20000", "200");
        row.liquidity_base = Some(50.0);
        row.liquidity_quote = Some(1_000_000.0);
        let info = from_dexscreener(ChainId::Solana, MEME, &row).expect("token is the quote");
        assert_eq!(
            (info.base_mint.as_str(), info.quote_mint.as_str()),
            (MEME, SOL)
        );
        assert!(info.is_native_pair);
        assert_eq!(info.liquidity_token, Some(1_000_000.0));
        assert_eq!(info.liquidity_native, Some(50.0));
        assert_eq!(info.price_sol, Some(1.0 / 20_000.0));
        assert_eq!(info.price_native, Some((1.0f64 / 20_000.0).to_string()));
        assert_eq!(info.price_usd, None, "priceUsd is the base's price");
    }

    #[test]
    fn a_token_quoted_against_another_token_carries_no_price() {
        // MEME/USDC listed under USDC: both prices describe MEME, not USDC.
        let info = from_dexscreener(ChainId::Solana, USDC, &pair(MEME, USDC, "0.01", "0.01"))
            .expect("token is the quote");
        assert_eq!(
            (info.base_mint.as_str(), info.quote_mint.as_str()),
            (USDC, MEME)
        );
        assert!(!info.is_native_pair);
        assert_eq!(info.liquidity_token, Some(50.0));
        assert_eq!(info.liquidity_native, None);
        assert_eq!((info.price_sol, info.price_usd), (None, None));
        assert_eq!(info.price_native, None);
    }

    #[test]
    fn a_base_token_against_a_stablecoin_has_no_native_price() {
        // priceNative of MEME/USDC is MEME in USDC, not in SOL.
        let info = from_dexscreener(ChainId::Solana, MEME, &pair(MEME, USDC, "0.01", "0.01"))
            .expect("token is the base");
        assert_eq!((info.price_sol, info.price_native.as_deref()), (None, None));
        assert_eq!(info.price_usd, Some(0.01));
    }

    #[test]
    fn a_pair_without_the_token_on_either_side_is_refused() {
        assert!(from_dexscreener(ChainId::Solana, MEME, &pair(SOL, USDC, "200", "200")).is_none());
    }
}
