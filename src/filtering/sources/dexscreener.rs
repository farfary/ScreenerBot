use crate::config::schemas::DexScreenerFilters;
use crate::filtering::sources::FilterRejectionReason;
use crate::tokens::types::{DataSource, Token};

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

    let m5_totals = match (token.txns_m5_buys, token.txns_m5_sells) {
        (Some(buys), Some(sells)) => buys.saturating_add(sells),
        _ => return None,
    };

    if m5_totals < config.min_transactions_5min {
        return Some(FilterRejectionReason::DexScreenerInsufficientTransactions5Min);
    }

    let h1_totals = match (token.txns_h1_buys, token.txns_h1_sells) {
        (Some(buys), Some(sells)) => buys.saturating_add(sells),
        _ => return None,
    };

    if h1_totals < config.min_transactions_1h {
        return Some(FilterRejectionReason::DexScreenerInsufficientTransactions1H);
    }

    None
}

fn check_liquidity(token: &Token, config: &DexScreenerFilters) -> Option<FilterRejectionReason> {
    if !config.liquidity_enabled {
        return None;
    }

    let liquidity = token.liquidity_usd?;
    if liquidity <= 0.0 {
        return Some(FilterRejectionReason::DexScreenerZeroLiquidity);
    }

    if liquidity < config.min_liquidity_usd {
        return Some(FilterRejectionReason::DexScreenerInsufficientLiquidity);
    }

    if liquidity > config.max_liquidity_usd {
        return Some(FilterRejectionReason::DexScreenerLiquidityTooHigh);
    }

    None
}

fn check_market_cap(token: &Token, config: &DexScreenerFilters) -> Option<FilterRejectionReason> {
    if !config.market_cap_enabled {
        return None;
    }

    let market_cap = token.market_cap?;

    if market_cap < config.min_market_cap_usd {
        return Some(FilterRejectionReason::DexScreenerMarketCapTooLow);
    }

    if market_cap > config.max_market_cap_usd {
        return Some(FilterRejectionReason::DexScreenerMarketCapTooHigh);
    }

    None
}

fn check_volume(token: &Token, config: &DexScreenerFilters) -> Option<FilterRejectionReason> {
    if !config.volume_enabled {
        return None;
    }

    match token.volume_h24 {
        Some(volume) => {
            if volume < config.min_volume_24h {
                Some(FilterRejectionReason::DexScreenerVolumeTooLow)
            } else {
                None
            }
        }
        None => Some(FilterRejectionReason::DexScreenerVolumeMissing),
    }
}

fn check_price_change(token: &Token, config: &DexScreenerFilters) -> Option<FilterRejectionReason> {
    if !config.price_change_enabled {
        return None;
    }

    let change = match token.price_change_h1 {
        Some(value) => value,
        None => return Some(FilterRejectionReason::DexScreenerPriceChangeMissing),
    };

    if change < config.min_price_change_h1 {
        return Some(FilterRejectionReason::DexScreenerPriceChangeTooLow);
    }

    if change > config.max_price_change_h1 {
        return Some(FilterRejectionReason::DexScreenerPriceChangeTooHigh);
    }

    None
}
