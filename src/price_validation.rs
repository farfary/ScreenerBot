use std::collections::HashMap;
use std::time::{ SystemTime, UNIX_EPOCH };
use crate::helpers::PRICE_CACHE;

/// Represents the state of price data for a token
#[derive(Debug, Clone, PartialEq)]
pub enum PriceState {
    /// Price is loaded and valid
    Loaded(f64),
    /// Price is currently being fetched
    Loading,
    /// Price failed to load or is stale
    NotAvailable,
    /// Price is invalid (zero or negative)
    Invalid,
}

/// Check if a price is valid for trading decisions
pub fn is_price_valid(price: f64) -> bool {
    price > 0.0 && price.is_finite()
}

/// Get the current price state for a token
pub fn get_price_state(mint: &str) -> PriceState {
    let cache = PRICE_CACHE.read().unwrap();

    if let Some(&(timestamp, price)) = cache.get(mint) {
        // Check if price is valid
        if !is_price_valid(price) {
            return PriceState::Invalid;
        }

        // Check if price is stale (older than 5 minutes)
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();

        if now.saturating_sub(timestamp) > 300 {
            return PriceState::NotAvailable;
        }

        PriceState::Loaded(price)
    } else {
        PriceState::NotAvailable
    }
}

/// Get price for trading if it's safe to use
pub fn get_trading_price(mint: &str) -> Option<f64> {
    match get_price_state(mint) {
        PriceState::Loaded(price) => Some(price),
        _ => None,
    }
}

/// Check if trading should be blocked due to missing prices
pub fn should_block_trading(required_mints: &[String]) -> bool {
    for mint in required_mints {
        match get_price_state(mint) {
            PriceState::Loaded(_) => {
                continue;
            }
            _ => {
                println!("🚫 [TRADING] Blocked due to missing price for {}", mint);
                return true;
            }
        }
    }
    false
}

/// Format price state for display in summaries
pub fn format_price_state(mint: &str) -> String {
    match get_price_state(mint) {
        PriceState::Loaded(price) => format!("{:.12}", price),
        PriceState::Loading => "Loading...".to_string(),
        PriceState::NotAvailable => "Price not loaded".to_string(),
        PriceState::Invalid => "Invalid price".to_string(),
    }
}

/// Batch check price states for multiple tokens
pub fn batch_price_states(mints: &[String]) -> HashMap<String, PriceState> {
    let mut states = HashMap::new();
    for mint in mints {
        states.insert(mint.clone(), get_price_state(mint));
    }
    states
}

/// Check if any token in a list has missing/invalid prices
pub fn has_missing_prices(mints: &[String]) -> bool {
    mints.iter().any(|mint| { !matches!(get_price_state(mint), PriceState::Loaded(_)) })
}
