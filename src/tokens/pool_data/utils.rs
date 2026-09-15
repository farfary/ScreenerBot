//! Utility functions for pool operations

use crate::tokens::types::TokenPoolInfo;

/// Parse f64 from string (handles empty/invalid strings)
pub fn parse_f64(value: &str) -> Option<f64> {
    value.trim().parse::<f64>().ok()
}

/// Parse GeckoTerminal token ID (extracts address from chain:address format)
pub fn parse_gecko_token_id(value: &str) -> Option<String> {
    let candidate = value
        .trim()
        .rsplit(|c| c == ':' || c == '_')
        .next()
        .unwrap_or(value)
        .trim();

    if candidate.is_empty() {
        None
    } else {
        Some(candidate.to_string())
    }
}

/// Liquidity metric for pool selection: the pool's SOL-side reserve, in SOL.
///
/// Every pool must be ranked in ONE unit. This used to fall through `liquidity_sol` →
/// `liquidity_usd` → `volume_h24`, so a server-sourced pool that carries only USD liquidity
/// ($476 scored 476) outranked a DexScreener pool with a real $61k market (3.2 SOL scored 3.2),
/// became the canonical pool, and the chart charted a pool with almost no trades.
pub fn calculate_pool_metric(pool: &TokenPoolInfo) -> f64 {
    sol_side_liquidity(pool, crate::apis::sol_price::get_sol_price())
}

/// SOL-side reserve in SOL: `liquidity_sol` when a source reports it, else total USD liquidity
/// halved and converted at `sol_price_usd` — the same conversion `from_geckoterminal` applies.
/// Zero when neither is known (including an unknown SOL price); callers break ties by volume.
pub fn sol_side_liquidity(pool: &TokenPoolInfo, sol_price_usd: f64) -> f64 {
    let usable = |v: &f64| v.is_finite() && *v > 0.0;
    if let Some(sol) = pool.liquidity_sol.filter(usable) {
        return sol;
    }
    match pool.liquidity_usd.filter(usable) {
        Some(usd) if usable(&sol_price_usd) => (usd / 2.0) / sol_price_usd,
        _ => 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pool(liquidity_sol: Option<f64>, liquidity_usd: Option<f64>) -> TokenPoolInfo {
        TokenPoolInfo {
            liquidity_sol,
            liquidity_usd,
            volume_h24: Some(1_000_000.0),
            ..TokenPoolInfo::default()
        }
    }

    #[test]
    fn usd_only_and_sol_reported_pools_rank_in_the_same_unit() {
        // BUTT: a server pool with only $476 against a DexScreener pool holding 3.23 SOL.
        let usd_only = pool(None, Some(476.0));
        let reported = pool(Some(3.2298), Some(61_364.12));
        let price = 99.65;
        assert!(sol_side_liquidity(&reported, price) > sol_side_liquidity(&usd_only, price));
        assert!((sol_side_liquidity(&usd_only, price) - 476.0 / 2.0 / price).abs() < 1e-9);
    }

    #[test]
    fn volume_and_unusable_values_never_count_as_liquidity() {
        assert_eq!(sol_side_liquidity(&pool(None, None), 100.0), 0.0);
        assert_eq!(
            sol_side_liquidity(&pool(Some(f64::NAN), Some(200.0)), 100.0),
            1.0
        );
        assert_eq!(sol_side_liquidity(&pool(Some(0.0), Some(-5.0)), 100.0), 0.0);
        // Without a SOL price a USD-only pool cannot be converted, so it does not guess.
        assert_eq!(sol_side_liquidity(&pool(None, Some(476.0)), 0.0), 0.0);
    }
}

/// Extract DEX label from pool (with fallback to source)
pub fn extract_dex_label(pool: &TokenPoolInfo) -> String {
    if let Some(dex) = &pool.dex {
        let trimmed = dex.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }

    if pool.sources.dexscreener.is_some() {
        "dexscreener".to_owned()
    } else if pool.sources.geckoterminal.is_some() {
        "geckoterminal".to_owned()
    } else {
        "unknown".to_owned()
    }
}

/// Liquidity stored on an OHLCV pool row, in the same unit the canonical ranking uses (see
/// [`calculate_pool_metric`]). It was USD, else SOL, else 24h volume, so the OHLCV series-pool
/// fallback compared numbers in three different units.
pub fn extract_pool_liquidity(pool: &TokenPoolInfo) -> f64 {
    calculate_pool_metric(pool)
}
