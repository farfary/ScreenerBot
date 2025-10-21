use crate::config::schemas::GeckoTerminalFilters;
use crate::filtering::sources::FilterRejectionReason;
use crate::tokens::types::{DataSource, Token};

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

    let liquidity = match token.liquidity_usd {
        Some(value) => value,
        None => return Some(FilterRejectionReason::GeckoTerminalLiquidityMissing),
    };

    if liquidity < config.min_liquidity_usd {
        return Some(FilterRejectionReason::GeckoTerminalLiquidityTooLow);
    }

    if config.max_liquidity_usd > 0.0 && liquidity > config.max_liquidity_usd {
        return Some(FilterRejectionReason::GeckoTerminalLiquidityTooHigh);
    }

    None
}

fn check_market_cap(token: &Token, config: &GeckoTerminalFilters) -> Option<FilterRejectionReason> {
    if !config.market_cap_enabled {
        return None;
    }

    let market_cap = match token.market_cap {
        Some(value) => value,
        None => return Some(FilterRejectionReason::GeckoTerminalMarketCapMissing),
    };

    if market_cap < config.min_market_cap_usd {
        return Some(FilterRejectionReason::GeckoTerminalMarketCapTooLow);
    }

    if config.max_market_cap_usd > 0.0 && market_cap > config.max_market_cap_usd {
        return Some(FilterRejectionReason::GeckoTerminalMarketCapTooHigh);
    }

    None
}

fn check_volume(token: &Token, config: &GeckoTerminalFilters) -> Option<FilterRejectionReason> {
    if !config.volume_enabled {
        return None;
    }

    if let Some(reason) = enforce_volume_threshold(
        token.volume_m5,
        config.min_volume_5m,
        FilterRejectionReason::GeckoTerminalVolume5mTooLow,
        FilterRejectionReason::GeckoTerminalVolume5mMissing,
    ) {
        return Some(reason);
    }

    if let Some(reason) = enforce_volume_threshold(
        token.volume_h1,
        config.min_volume_1h,
        FilterRejectionReason::GeckoTerminalVolume1hTooLow,
        FilterRejectionReason::GeckoTerminalVolume1hMissing,
    ) {
        return Some(reason);
    }

    enforce_volume_threshold(
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
        FilterRejectionReason::GeckoTerminalPriceChange5mMissing,
    ) {
        return Some(reason);
    }

    if let Some(reason) = enforce_price_change(
        token.price_change_h1,
        config.min_price_change_h1,
        config.max_price_change_h1,
        FilterRejectionReason::GeckoTerminalPriceChange1hTooLow,
        FilterRejectionReason::GeckoTerminalPriceChange1hTooHigh,
        FilterRejectionReason::GeckoTerminalPriceChange1hMissing,
    ) {
        return Some(reason);
    }

    enforce_price_change(
        token.price_change_h24,
        config.min_price_change_h24,
        config.max_price_change_h24,
        FilterRejectionReason::GeckoTerminalPriceChange24hTooLow,
        FilterRejectionReason::GeckoTerminalPriceChange24hTooHigh,
        FilterRejectionReason::GeckoTerminalPriceChange24hMissing,
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

    enforce_reserve_threshold(
        token.reserve_in_usd,
        config.min_reserve_usd,
        FilterRejectionReason::GeckoTerminalReserveTooLow,
        FilterRejectionReason::GeckoTerminalReserveMissing,
    )
}

fn enforce_volume_threshold(
    value: Option<f64>,
    threshold: f64,
    too_low_reason: FilterRejectionReason,
    missing_reason: FilterRejectionReason,
) -> Option<FilterRejectionReason> {
    if threshold <= 0.0 {
        return None;
    }

    match value {
        Some(volume) if volume < threshold => Some(too_low_reason),
        Some(_) => None,
        None => Some(missing_reason),
    }
}

fn enforce_price_change(
    value: Option<f64>,
    min_threshold: f64,
    max_threshold: f64,
    too_low_reason: FilterRejectionReason,
    too_high_reason: FilterRejectionReason,
    missing_reason: FilterRejectionReason,
) -> Option<FilterRejectionReason> {
    let change = match value {
        Some(value) => value,
        None => return Some(missing_reason),
    };

    if change < min_threshold {
        return Some(too_low_reason);
    }

    if change > max_threshold {
        return Some(too_high_reason);
    }

    None
}

fn enforce_reserve_threshold(
    value: Option<f64>,
    min_threshold: f64,
    too_low_reason: FilterRejectionReason,
    missing_reason: FilterRejectionReason,
) -> Option<FilterRejectionReason> {
    if min_threshold <= 0.0 {
        return None;
    }

    match value {
        Some(reserve) if reserve < min_threshold => Some(too_low_reason),
        Some(_) => None,
        None => Some(missing_reason),
    }
}
