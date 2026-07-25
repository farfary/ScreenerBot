//! DexScreener filter source — validates market data from DexScreener API.

use crate::config::schemas::DexScreenerFilters;
use crate::filtering::sources::{bounds, FilterRejectionReason};
use crate::tokens::types::{DataSource, Token};

/// Evaluate a token against DexScreener market data filter criteria.
pub fn evaluate(token: &Token, config: &DexScreenerFilters) -> Result<(), FilterRejectionReason> {
    if !config.enabled {
        return Ok(());
    }

    if let Some(reason) = check_token_info(token, config) {
        return Err(reason);
    }

    if let Some(reason) = check_transaction_activity(token, config) {
        return Err(reason);
    }

    if let Some(reason) = check_liquidity(token, config) {
        return Err(reason);
    }

    if let Some(reason) = check_market_cap(token, config) {
        return Err(reason);
    }

    if let Some(reason) = check_fdv(token, config) {
        return Err(reason);
    }

    if let Some(reason) = check_volume(token, config) {
        return Err(reason);
    }

    if let Some(reason) = check_price_change(token, config) {
        return Err(reason);
    }

    Ok(())
}

fn check_token_info(token: &Token, config: &DexScreenerFilters) -> Option<FilterRejectionReason> {
    if !config.token_info_enabled {
        return None;
    }

    if config.require_name_and_symbol {
        if token.name.trim().is_empty() {
            return Some(FilterRejectionReason::DexScreenerEmptyName);
        }
        if token.symbol.trim().is_empty() {
            return Some(FilterRejectionReason::DexScreenerEmptySymbol);
        }
    }

    if config.require_logo_url {
        let missing_logo = token
            .image_url
            .as_ref()
            .map(|value| value.trim().is_empty())
            .unwrap_or(true);
        if missing_logo {
            return Some(FilterRejectionReason::DexScreenerEmptyLogoUrl);
        }
    }

    if config.require_website_url {
        let has_website = token
            .websites
            .iter()
            .any(|link| !link.url.trim().is_empty());
        if !has_website {
            return Some(FilterRejectionReason::DexScreenerEmptyWebsiteUrl);
        }
    }

    None
}

fn check_transaction_activity(
    token: &Token,
    config: &DexScreenerFilters,
) -> Option<FilterRejectionReason> {
    if !config.transactions_enabled {
        return None;
    }

    if token.data_source != DataSource::DexScreener {
        return None;
    }

    // `txns_*_total` is the shared definition also used for sorting, so a window the
    // provider reported one-sided counts as what it reported instead of being discarded.
    // Each window is judged on its own: an early `return` for an absent 5m reading used to
    // waive the 1h minimum entirely, which is the check that actually gates dead tokens.
    if let Some(reason) = enforce_transaction_floor(
        token.txns_5m_total(),
        config.min_transactions_5min,
        FilterRejectionReason::DexScreenerInsufficientTransactions5Min,
    ) {
        return Some(reason);
    }

    enforce_transaction_floor(
        token.txns_1h_total(),
        config.min_transactions_1h,
        FilterRejectionReason::DexScreenerInsufficientTransactions1H,
    )
}

/// A transaction floor of zero constrains nothing, so an absent count is only a rejection
/// when the configuration actually demands activity in that window.
fn enforce_transaction_floor(
    total: Option<i64>,
    minimum: i64,
    too_low_reason: FilterRejectionReason,
) -> Option<FilterRejectionReason> {
    if minimum <= 0 {
        return None;
    }

    match total {
        Some(count) if count < minimum => Some(too_low_reason),
        Some(_) => None,
        None => Some(too_low_reason),
    }
}

fn check_liquidity(token: &Token, config: &DexScreenerFilters) -> Option<FilterRejectionReason> {
    if !config.liquidity_enabled {
        return None;
    }

    // A reported zero is its own rejection: unlike an absent reading it is a measurement,
    // and it says the pool cannot be traded at all.
    if let Some(liquidity) = bounds::reading(token.liquidity_usd) {
        if liquidity <= 0.0 {
            return Some(FilterRejectionReason::DexScreenerZeroLiquidity);
        }
    }

    match bounds::check_range(
        token.liquidity_usd,
        config.min_liquidity_usd,
        Some(config.max_liquidity_usd),
    ) {
        bounds::Range::TooLow => Some(FilterRejectionReason::DexScreenerInsufficientLiquidity),
        bounds::Range::TooHigh => Some(FilterRejectionReason::DexScreenerLiquidityTooHigh),
        bounds::Range::Ok => None,
    }
}

fn check_market_cap(token: &Token, config: &DexScreenerFilters) -> Option<FilterRejectionReason> {
    if !config.market_cap_enabled {
        return None;
    }

    match bounds::check_range(
        token.market_cap,
        config.min_market_cap_usd,
        Some(config.max_market_cap_usd),
    ) {
        bounds::Range::TooLow => Some(FilterRejectionReason::DexScreenerMarketCapTooLow),
        bounds::Range::TooHigh => Some(FilterRejectionReason::DexScreenerMarketCapTooHigh),
        bounds::Range::Ok => None,
    }
}

fn check_fdv(token: &Token, config: &DexScreenerFilters) -> Option<FilterRejectionReason> {
    if !config.fdv_enabled {
        return None;
    }

    match bounds::check_range(token.fdv, config.min_fdv_usd, Some(config.max_fdv_usd)) {
        bounds::Range::TooLow => Some(FilterRejectionReason::DexScreenerFdvTooLow),
        bounds::Range::TooHigh => Some(FilterRejectionReason::DexScreenerFdvTooHigh),
        bounds::Range::Ok => None,
    }
}

fn check_volume(token: &Token, config: &DexScreenerFilters) -> Option<FilterRejectionReason> {
    if !config.volume_enabled {
        return None;
    }

    if let Some(reason) = enforce_volume_threshold(
        token.volume_m5,
        config.min_volume_5m,
        FilterRejectionReason::DexScreenerVolume5mTooLow,
        FilterRejectionReason::DexScreenerVolume5mMissing,
    ) {
        return Some(reason);
    }

    if let Some(reason) = enforce_volume_threshold(
        token.volume_h1,
        config.min_volume_1h,
        FilterRejectionReason::DexScreenerVolume1hTooLow,
        FilterRejectionReason::DexScreenerVolume1hMissing,
    ) {
        return Some(reason);
    }

    if let Some(reason) = enforce_volume_threshold(
        token.volume_h6,
        config.min_volume_6h,
        FilterRejectionReason::DexScreenerVolume6hTooLow,
        FilterRejectionReason::DexScreenerVolume6hMissing,
    ) {
        return Some(reason);
    }

    enforce_volume_threshold(
        token.volume_h24,
        config.min_volume_24h,
        FilterRejectionReason::DexScreenerVolumeTooLow,
        FilterRejectionReason::DexScreenerVolumeMissing,
    )
}

fn check_price_change(token: &Token, config: &DexScreenerFilters) -> Option<FilterRejectionReason> {
    if !config.price_change_enabled {
        return None;
    }

    if let Some(reason) = enforce_price_change(
        token.price_change_m5,
        config.min_price_change_m5,
        config.max_price_change_m5,
        FilterRejectionReason::DexScreenerPriceChange5mTooLow,
        FilterRejectionReason::DexScreenerPriceChange5mTooHigh,
    ) {
        return Some(reason);
    }

    if let Some(reason) = enforce_price_change(
        token.price_change_h1,
        config.min_price_change_h1,
        config.max_price_change_h1,
        FilterRejectionReason::DexScreenerPriceChangeTooLow,
        FilterRejectionReason::DexScreenerPriceChangeTooHigh,
    ) {
        return Some(reason);
    }

    if let Some(reason) = enforce_price_change(
        token.price_change_h6,
        config.min_price_change_h6,
        config.max_price_change_h6,
        FilterRejectionReason::DexScreenerPriceChange6hTooLow,
        FilterRejectionReason::DexScreenerPriceChange6hTooHigh,
    ) {
        return Some(reason);
    }

    enforce_price_change(
        token.price_change_h24,
        config.min_price_change_h24,
        config.max_price_change_h24,
        FilterRejectionReason::DexScreenerPriceChange24hTooLow,
        FilterRejectionReason::DexScreenerPriceChange24hTooHigh,
    )
}

fn enforce_volume_threshold(
    value: Option<f64>,
    threshold: f64,
    too_low_reason: FilterRejectionReason,
    missing_reason: FilterRejectionReason,
) -> Option<FilterRejectionReason> {
    match bounds::check_floor(value, threshold) {
        bounds::Floor::TooLow => Some(too_low_reason),
        bounds::Floor::Missing => Some(missing_reason),
        bounds::Floor::Ok => None,
    }
}

fn enforce_price_change(
    value: Option<f64>,
    min_threshold: f64,
    max_threshold: f64,
    too_low_reason: FilterRejectionReason,
    too_high_reason: FilterRejectionReason,
) -> Option<FilterRejectionReason> {
    match bounds::check_range(value, min_threshold, Some(max_threshold)) {
        bounds::Range::TooLow => Some(too_low_reason),
        bounds::Range::TooHigh => Some(too_high_reason),
        bounds::Range::Ok => None,
    }
}
