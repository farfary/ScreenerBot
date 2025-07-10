use std::collections::HashMap;
use std::time::{ SystemTime, UNIX_EPOCH };
use crate::helpers::PRICE_CACHE;

// ═══════════════════════════════════════════════════════════════════════════════
// REAL-TIME PRICE CHANGE TRACKING SYSTEM
// ═══════════════════════════════════════════════════════════════════════════════

/// Price history entry with timestamp and price
#[derive(Debug, Clone)]
pub struct PriceHistoryEntry {
    pub timestamp: u64,
    pub price: f64,
}

/// Real-time price change calculator using pool prices
#[derive(Debug, Clone)]
pub struct PriceChangeTracker {
    pub history: Vec<PriceHistoryEntry>,
    pub max_history_size: usize,
    pub max_age_seconds: u64,
}

impl PriceChangeTracker {
    pub fn new() -> Self {
        Self {
            history: Vec::new(),
            max_history_size: 100, // Keep last 100 price points
            max_age_seconds: 3600, // Keep data for 1 hour
        }
    }

    /// Add a new price point to the history
    pub fn add_price(&mut self, price: f64) {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        self.history.push(PriceHistoryEntry {
            timestamp: now,
            price,
        });

        // Clean old entries
        self.cleanup_old_entries();

        // Limit history size
        if self.history.len() > self.max_history_size {
            self.history.remove(0);
        }
    }

    /// Remove entries older than max_age_seconds
    fn cleanup_old_entries(&mut self) {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        let cutoff = now.saturating_sub(self.max_age_seconds);
        self.history.retain(|entry| entry.timestamp >= cutoff);
    }

    /// Calculate price change percentage over the specified period (in minutes)
    pub fn get_price_change(&self, minutes: u64) -> Option<f64> {
        if self.history.is_empty() {
            return None;
        }

        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        let target_time = now.saturating_sub(minutes * 60);

        // Get current price (most recent)
        let current_price = self.history.last()?.price;

        // Find the closest historical price to target_time
        let historical_price = self.find_closest_price(target_time)?;

        // Calculate percentage change
        let change_pct = ((current_price - historical_price) / historical_price) * 100.0;
        Some(change_pct)
    }

    /// Find the price closest to the target timestamp
    fn find_closest_price(&self, target_time: u64) -> Option<f64> {
        if self.history.is_empty() {
            return None;
        }

        let mut closest_entry = &self.history[0];
        let mut closest_diff = ((target_time as i64) - (closest_entry.timestamp as i64)).abs();

        for entry in &self.history {
            let diff = ((target_time as i64) - (entry.timestamp as i64)).abs();
            if diff < closest_diff {
                closest_diff = diff;
                closest_entry = entry;
            }
        }

        Some(closest_entry.price)
    }

    /// Get the age of the oldest entry in minutes
    pub fn get_history_age_minutes(&self) -> Option<u64> {
        if self.history.is_empty() {
            return None;
        }

        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        let oldest_time = self.history.first()?.timestamp;
        Some(now.saturating_sub(oldest_time) / 60)
    }

    /// Check if we have enough history for reliable price change calculation
    pub fn has_sufficient_history(&self, minutes: u64) -> bool {
        if let Some(age) = self.get_history_age_minutes() {
            age >= minutes && self.history.len() >= 3 // At least 3 data points and enough time
        } else {
            false
        }
    }
}

// Global price history tracking for all tokens
use once_cell::sync::Lazy;
use std::sync::RwLock;

pub static PRICE_HISTORY: Lazy<RwLock<HashMap<String, PriceChangeTracker>>> = Lazy::new(||
    RwLock::new(HashMap::new())
);

/// Update price history for a token
pub fn update_price_history(mint: &str, price: f64) {
    if !is_price_valid(price) {
        return;
    }

    let mut history = PRICE_HISTORY.write().unwrap();
    let tracker = history.entry(mint.to_string()).or_insert_with(PriceChangeTracker::new);
    tracker.add_price(price);
}

/// Get real-time price change for a token over specified minutes
pub fn get_realtime_price_change(mint: &str, minutes: u64) -> Option<f64> {
    let history = PRICE_HISTORY.read().unwrap();
    if let Some(tracker) = history.get(mint) {
        tracker.get_price_change(minutes)
    } else {
        None
    }
}

/// Check if token has sufficient price history for reliable calculations
pub fn has_sufficient_price_history(mint: &str, minutes: u64) -> bool {
    let history = PRICE_HISTORY.read().unwrap();
    if let Some(tracker) = history.get(mint) {
        tracker.has_sufficient_history(minutes)
    } else {
        false
    }
}

/// Get real-time price changes for multiple timeframes
pub fn get_realtime_price_changes(mint: &str) -> (Option<f64>, Option<f64>, Option<f64>) {
    let price_5m = get_realtime_price_change(mint, 5);
    let price_15m = get_realtime_price_change(mint, 15);
    let price_1h = get_realtime_price_change(mint, 60);
    (price_5m, price_15m, price_1h)
}

// ═══════════════════════════════════════════════════════════════════════════════
// EXISTING PRICE VALIDATION CODE
// ═══════════════════════════════════════════════════════════════════════════════

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
        // Update price history when we get a new price
        update_price_history(mint, price);

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
        PriceState::Loaded(price) => {
            // Update price history when accessing trading price
            update_price_history(mint, price);
            Some(price)
        }
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
