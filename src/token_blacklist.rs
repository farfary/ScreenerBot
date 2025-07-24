// token_blacklist.rs - Token blacklist management for low liquidity tokens
use std::collections::HashMap;
use std::sync::{ RwLock, Arc };
use chrono::{ DateTime, Utc, Duration };
use serde::{ Serialize, Deserialize };
use std::fs;
use std::path::Path;
use crate::logger::{ log, LogTag };
use once_cell::sync::Lazy;

// Global blacklist instance
pub static TOKEN_BLACKLIST: Lazy<Arc<RwLock<TokenBlacklist>>> = Lazy::new(|| {
    Arc::new(RwLock::new(TokenBlacklist::load_or_create()))
});

/// Represents a blacklisted token with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlacklistedToken {
    pub mint: String,
    pub symbol: String,
    pub first_low_liquidity: DateTime<Utc>,
    pub last_low_liquidity: DateTime<Utc>,
    pub low_liquidity_count: u32,
    pub creation_time: Option<DateTime<Utc>>,
    pub reason: String,
}

/// Token blacklist manager
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenBlacklist {
    pub blacklisted_tokens: HashMap<String, BlacklistedToken>, // mint -> BlacklistedToken
    pub low_liquidity_tracking: HashMap<String, LowLiquidityTracker>, // mint -> tracker
    pub blacklist_file: String,
}

/// Tracks low liquidity occurrences for a token
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LowLiquidityTracker {
    pub mint: String,
    pub symbol: String,
    pub first_occurrence: DateTime<Utc>,
    pub last_occurrence: DateTime<Utc>,
    pub occurrence_count: u32,
    pub creation_time: Option<DateTime<Utc>>,
}

impl TokenBlacklist {
    const BLACKLIST_FILE: &'static str = "token_blacklist.json";
    const LOW_LIQUIDITY_THRESHOLD: f64 = 100.0; // $100 USD
    const MIN_AGE_HOURS: i64 = 2; // 2 hours minimum age
    const MAX_LOW_LIQUIDITY_COUNT: u32 = 5; // Blacklist after 5 occurrences

    /// Load blacklist from file or create new one
    pub fn load_or_create() -> Self {
        if Path::new(Self::BLACKLIST_FILE).exists() {
            match Self::load_from_file(Self::BLACKLIST_FILE) {
                Ok(blacklist) => {
                    log(
                        LogTag::System,
                        "SUCCESS",
                        &format!(
                            "Loaded token blacklist: {} blacklisted, {} tracking",
                            blacklist.blacklisted_tokens.len(),
                            blacklist.low_liquidity_tracking.len()
                        )
                    );
                    blacklist
                }
                Err(e) => {
                    log(
                        LogTag::System,
                        "WARN",
                        &format!("Failed to load blacklist, creating new: {}", e)
                    );
                    Self::new()
                }
            }
        } else {
            Self::new()
        }
    }

    /// Create new empty blacklist
    pub fn new() -> Self {
        Self {
            blacklisted_tokens: HashMap::new(),
            low_liquidity_tracking: HashMap::new(),
            blacklist_file: Self::BLACKLIST_FILE.to_string(),
        }
    }

    /// Load blacklist from JSON file
    pub fn load_from_file(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let data = fs::read_to_string(path)?;
        let blacklist: Self = serde_json::from_str(&data)?;
        Ok(blacklist)
    }

    /// Save blacklist to JSON file
    pub fn save_to_file(&self) -> Result<(), Box<dyn std::error::Error>> {
        let data = serde_json::to_string_pretty(self)?;
        fs::write(&self.blacklist_file, data)?;
        Ok(())
    }

    /// Check if a token is blacklisted
    pub fn is_blacklisted(&self, mint: &str) -> bool {
        self.blacklisted_tokens.contains_key(mint)
    }

    /// Track low liquidity occurrence for a token
    pub fn track_low_liquidity(
        &mut self,
        mint: &str,
        symbol: &str,
        liquidity_usd: f64,
        creation_time: Option<DateTime<Utc>>
    ) -> bool {
        // Don't track if already blacklisted
        if self.is_blacklisted(mint) {
            return false;
        }

        // Check if token is old enough (more than 2 hours)
        let now = Utc::now();
        let min_age = Duration::hours(Self::MIN_AGE_HOURS);

        if let Some(created) = creation_time {
            if now.signed_duration_since(created) < min_age {
                return false; // Too young, don't track
            }
        }

        // Only track if liquidity is below threshold
        if liquidity_usd >= Self::LOW_LIQUIDITY_THRESHOLD {
            return false;
        }

        let mut should_blacklist = false;

        // Update or create tracker
        if let Some(tracker) = self.low_liquidity_tracking.get_mut(mint) {
            tracker.last_occurrence = now;
            tracker.occurrence_count += 1;

            if tracker.occurrence_count >= Self::MAX_LOW_LIQUIDITY_COUNT {
                should_blacklist = true;
            }
        } else {
            let tracker = LowLiquidityTracker {
                mint: mint.to_string(),
                symbol: symbol.to_string(),
                first_occurrence: now,
                last_occurrence: now,
                occurrence_count: 1,
                creation_time,
            };
            self.low_liquidity_tracking.insert(mint.to_string(), tracker);
        }

        // Blacklist if threshold reached
        if should_blacklist {
            self.blacklist_token(mint, symbol, creation_time);
            return true;
        }

        false
    }

    /// Blacklist a token
    fn blacklist_token(&mut self, mint: &str, symbol: &str, creation_time: Option<DateTime<Utc>>) {
        let now = Utc::now();
        let tracker = self.low_liquidity_tracking.get(mint);

        let blacklisted = BlacklistedToken {
            mint: mint.to_string(),
            symbol: symbol.to_string(),
            first_low_liquidity: tracker.map(|t| t.first_occurrence).unwrap_or(now),
            last_low_liquidity: now,
            low_liquidity_count: tracker
                .map(|t| t.occurrence_count)
                .unwrap_or(Self::MAX_LOW_LIQUIDITY_COUNT),
            creation_time,
            reason: format!(
                "Low liquidity (<${}) {} times over {} hours",
                Self::LOW_LIQUIDITY_THRESHOLD,
                Self::MAX_LOW_LIQUIDITY_COUNT,
                Self::MIN_AGE_HOURS
            ),
        };

        self.blacklisted_tokens.insert(mint.to_string(), blacklisted.clone());
        self.low_liquidity_tracking.remove(mint);

        log(
            LogTag::System,
            "BLACKLIST",
            &format!("Blacklisted token {} ({}) - {}", symbol, mint, blacklisted.reason)
        );

        // Save to file
        if let Err(e) = self.save_to_file() {
            log(LogTag::System, "ERROR", &format!("Failed to save blacklist: {}", e));
        }
    }

    /// Remove a token from blacklist (manual intervention)
    pub fn remove_from_blacklist(&mut self, mint: &str) -> bool {
        if let Some(removed) = self.blacklisted_tokens.remove(mint) {
            log(
                LogTag::System,
                "INFO",
                &format!("Removed {} ({}) from blacklist", removed.symbol, mint)
            );

            if let Err(e) = self.save_to_file() {
                log(LogTag::System, "ERROR", &format!("Failed to save blacklist: {}", e));
            }
            return true;
        }
        false
    }

    /// Get blacklist statistics
    pub fn get_stats(&self) -> (usize, usize) {
        (self.blacklisted_tokens.len(), self.low_liquidity_tracking.len())
    }

    /// Clean up old tracking entries (older than 30 days)
    pub fn cleanup_old_entries(&mut self) {
        let now = Utc::now();
        let cleanup_age = Duration::days(30);

        let old_tracking: Vec<String> = self.low_liquidity_tracking
            .iter()
            .filter(|(_, tracker)| {
                now.signed_duration_since(tracker.first_occurrence) > cleanup_age
            })
            .map(|(mint, _)| mint.clone())
            .collect();

        for mint in old_tracking {
            self.low_liquidity_tracking.remove(&mint);
        }

        if !self.low_liquidity_tracking.is_empty() {
            log(
                LogTag::System,
                "INFO",
                &format!("Cleaned up {} old tracking entries", self.low_liquidity_tracking.len())
            );

            if let Err(e) = self.save_to_file() {
                log(
                    LogTag::System,
                    "ERROR",
                    &format!("Failed to save blacklist after cleanup: {}", e)
                );
            }
        }
    }
}

/// Check if a token should be tracked for low liquidity
pub fn check_and_track_liquidity(
    mint: &str,
    symbol: &str,
    liquidity_usd: Option<f64>,
    creation_time: Option<DateTime<Utc>>
) -> bool {
    if let Some(liquidity) = liquidity_usd {
        if let Ok(mut blacklist) = TOKEN_BLACKLIST.write() {
            return blacklist.track_low_liquidity(mint, symbol, liquidity, creation_time);
        }
    }
    false
}

/// Check if a token is blacklisted
pub fn is_token_blacklisted(mint: &str) -> bool {
    if let Ok(blacklist) = TOKEN_BLACKLIST.read() { blacklist.is_blacklisted(mint) } else { false }
}

/// Get blacklist statistics
pub fn get_blacklist_stats() -> (usize, usize) {
    if let Ok(blacklist) = TOKEN_BLACKLIST.read() { blacklist.get_stats() } else { (0, 0) }
}

/// Remove token from blacklist (for manual intervention)
pub fn remove_token_from_blacklist(mint: &str) -> bool {
    if let Ok(mut blacklist) = TOKEN_BLACKLIST.write() {
        blacklist.remove_from_blacklist(mint)
    } else {
        false
    }
}

/// Periodic cleanup of old entries
pub fn cleanup_blacklist() {
    if let Ok(mut blacklist) = TOKEN_BLACKLIST.write() {
        blacklist.cleanup_old_entries();
    }
}
