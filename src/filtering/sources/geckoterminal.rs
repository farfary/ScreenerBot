//! GeckoTerminal filter source — validates market data from GeckoTerminal API.

use crate::config::schemas::GeckoTerminalFilters;
use crate::filtering::sources::{bounds, FilterRejectionReason};
use crate::tokens::types::{DataSource, Token};

/// Evaluate a token against GeckoTerminal market data filter criteria.
pub fn evaluate(token: &Token, config: &GeckoTerminalFilters) -> Result<(), FilterRejectionReason> {
    if !config.enabled {
        return Ok(());
    }

    if token.data_source != DataSource::GeckoTerminal {
        return Ok(());
    }

    if let Some(reason) = check_liquidity(token, config) {
        return Err(reason);
    }

    if let Some(reason) = check_market_cap(token, config) {
        return Err(reason);
    }

    if let Some(reason) = check_volume(token, config) {
        return Err(reason);
    }

    if let Some(reason) = check_price_change(token, config) {
        return Err(reason);
    }

    if let Some(reason) = check_pool_metrics(token, config) {
        return Err(reason);
    }

    Ok(())
}

fn check_liquidity(token: &Token, config: &GeckoTerminalFilters) -> Option<FilterRejectionReason> {
    if !config.liquidity_enabled {
        return None;
    }

    match bounds::check_range(
        token.liquidity_usd,
        config.min_liquidity_usd,
        bounds::optional_ceiling(config.max_liquidity_usd),
    ) {
        bounds::Range::TooLow => Some(FilterRejectionReason::GeckoTerminalLiquidityTooLow),
        bounds::Range::TooHigh => Some(FilterRejectionReason::GeckoTerminalLiquidityTooHigh),
        bounds::Range::Ok => None,
    }
}

fn check_market_cap(token: &Token, config: &GeckoTerminalFilters) -> Option<FilterRejectionReason> {
    if !config.market_cap_enabled {
        return None;
    }

    match bounds::check_range(
        token.market_cap,
        config.min_market_cap_usd,
        bounds::optional_ceiling(config.max_market_cap_usd),
    ) {
        bounds::Range::TooLow => Some(FilterRejectionReason::GeckoTerminalMarketCapTooLow),
        bounds::Range::TooHigh => Some(FilterRejectionReason::GeckoTerminalMarketCapTooHigh),
        bounds::Range::Ok => None,
    }
}

fn check_volume(token: &Token, config: &GeckoTerminalFilters) -> Option<FilterRejectionReason> {
    if !config.volume_enabled {
        return None;
    }

    if let Some(reason) = enforce_floor(
        token.volume_m5,
        config.min_volume_5m,
        FilterRejectionReason::GeckoTerminalVolume5mTooLow,
        FilterRejectionReason::GeckoTerminalVolume5mMissing,
    ) {
        return Some(reason);
    }

    if let Some(reason) = enforce_floor(
        token.volume_h1,
        config.min_volume_1h,
        FilterRejectionReason::GeckoTerminalVolume1hTooLow,
        FilterRejectionReason::GeckoTerminalVolume1hMissing,
    ) {
        return Some(reason);
    }

    enforce_floor(
        token.volume_h24,
        config.min_volume_24h,
        FilterRejectionReason::GeckoTerminalVolume24hTooLow,
        FilterRejectionReason::GeckoTerminalVolume24hMissing,
    )
}

fn check_price_change(
    token: &Token,
    config: &GeckoTerminalFilters,
) -> Option<FilterRejectionReason> {
    if !config.price_change_enabled {
        return None;
    }

    if let Some(reason) = enforce_price_change(
        token.price_change_m5,
        config.min_price_change_m5,
        config.max_price_change_m5,
        FilterRejectionReason::GeckoTerminalPriceChange5mTooLow,
        FilterRejectionReason::GeckoTerminalPriceChange5mTooHigh,
    ) {
        return Some(reason);
    }

    if let Some(reason) = enforce_price_change(
        token.price_change_h1,
        config.min_price_change_h1,
        config.max_price_change_h1,
        FilterRejectionReason::GeckoTerminalPriceChange1hTooLow,
        FilterRejectionReason::GeckoTerminalPriceChange1hTooHigh,
    ) {
        return Some(reason);
    }

    enforce_price_change(
        token.price_change_h24,
        config.min_price_change_h24,
        config.max_price_change_h24,
        FilterRejectionReason::GeckoTerminalPriceChange24hTooLow,
        FilterRejectionReason::GeckoTerminalPriceChange24hTooHigh,
    )
}

fn check_pool_metrics(
    token: &Token,
    config: &GeckoTerminalFilters,
) -> Option<FilterRejectionReason> {
    if !config.pool_metrics_enabled {
        return None;
    }

    if config.min_pool_count > 0 {
        match token.pool_count {
            Some(count) => {
                if count < config.min_pool_count {
                    return Some(FilterRejectionReason::GeckoTerminalPoolCountTooLow);
                }
            }
            None => return Some(FilterRejectionReason::GeckoTerminalPoolCountMissing),
        }
    }

    if config.max_pool_count > 0 {
        if let Some(count) = token.pool_count {
            if count > config.max_pool_count {
                return Some(FilterRejectionReason::GeckoTerminalPoolCountTooHigh);
            }
        }
    }

    enforce_floor(
        token.reserve_in_usd,
        config.min_reserve_usd,
        FilterRejectionReason::GeckoTerminalReserveTooLow,
        FilterRejectionReason::GeckoTerminalReserveMissing,
    )
}

/// Both the volume windows and the pool reserve are floors, so they share one shape.
fn enforce_floor(
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
