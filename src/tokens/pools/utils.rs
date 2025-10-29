/// Utility functions for pool operations
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

/// Calculate primary liquidity metric for pool selection
/// Priority: liquidity_sol > liquidity_usd > volume_h24
pub fn calculate_pool_metric(pool: &TokenPoolInfo) -> f64 {
    pool.liquidity_sol
        .or(pool.liquidity_usd)
        .or(pool.volume_h24)
        .unwrap_or(0.0)
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
        "dexscreener".to_string()
    } else if pool.sources.geckoterminal.is_some() {
        "geckoterminal".to_string()
    } else {
        "unknown".to_string()
    }
}

/// Extract liquidity value with validation
pub fn extract_pool_liquidity(pool: &TokenPoolInfo) -> f64 {
    if let Some(liquidity) = pool.liquidity_usd {
        if liquidity.is_finite() && liquidity > 0.0 {
            return liquidity;
        }
    }

    if let Some(liquidity) = pool.liquidity_sol {
        if liquidity.is_finite() && liquidity > 0.0 {
            return liquidity;
        }
    }

    if let Some(volume) = pool.volume_h24 {
        if volume.is_finite() && volume > 0.0 {
            return volume;
        }
    }

    0.0
}
