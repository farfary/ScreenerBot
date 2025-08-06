/// Token blacklist system for managing problematic tokens
/// Automatically blacklists tokens with poor liquidity performance
use serde::{ Serialize, Deserialize };
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use chrono::{ DateTime, Utc, Duration as ChronoDuration };
use crate::logger::{ log, LogTag };
use crate::global::TOKEN_BLACKLIST as TOKEN_BLACKLIST_FILE;

// =============================================================================
// CONFIGURATION CONSTANTS
// =============================================================================

/// Low liquidity threshold in USD
pub const LOW_LIQUIDITY_THRESHOLD: f64 = 100.0;

/// Minimum token age in hours before tracking for blacklist
pub const MIN_AGE_HOURS: i64 = 2;

/// Maximum low liquidity occurrences before blacklisting
pub const MAX_LOW_LIQUIDITY_COUNT: u32 = 5;

/// Blacklist file path
pub const BLACKLIST_FILE: &str = TOKEN_BLACKLIST_FILE;

/// System and stable tokens that should always be excluded from trading
pub const SYSTEM_STABLE_TOKENS: &[&str] = &[
    "So11111111111111111111111111111111111111112", // SOL
    "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v", // USDC
    "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB", // USDT
    "7dHbWXmci3dT8UFYWYZweBLXgycu7Y3iL6trKn1Y7ARj", // stSOL
    "mSoLzYCxHdYgdzU16g5QSh3i5K3z3KZK7ytfqcJm7So", // mSOL
    "11111111111111111111111111111111", // System Program (invalid token)
    "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA", // Token Program
    "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb", // Token-2022 Program
];

// =============================================================================
// DATA STRUCTURES
// =============================================================================

/// Token blacklist entry with tracking information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlacklistEntry {
    pub mint: String,
    pub symbol: String,
    pub reason: BlacklistReason,
    pub first_occurrence: DateTime<Utc>,
    pub last_occurrence: DateTime<Utc>,
    pub occurrence_count: u32,
    pub liquidity_checks: Vec<LiquidityCheck>,
}

/// Reasons for blacklisting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BlacklistReason {
    LowLiquidity,
    PoorPerformance,
    ManualBlacklist,
    SystemToken, // System/program tokens
    StableToken, // Stable coins and major tokens
}

/// Individual liquidity check record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiquidityCheck {
    pub timestamp: DateTime<Utc>,
    pub liquidity_usd: f64,
    pub token_age_hours: i64,
}

/// Complete blacklist data structure
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TokenBlacklist {
    pub blacklisted_tokens: HashMap<String, BlacklistEntry>, // mint -> entry
    pub tracking_data: HashMap<String, Vec<LiquidityCheck>>, // mint -> checks
    pub last_updated: Option<DateTime<Utc>>,
}

// =============================================================================
// BLACKLIST MANAGER
// =============================================================================

impl TokenBlacklist {
    /// Create new empty blacklist
    pub fn new() -> Self {
        Self {
            blacklisted_tokens: HashMap::new(),
            tracking_data: HashMap::new(),
            last_updated: Some(Utc::now()),
        }
    }

    /// Load blacklist from file
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        if !Path::new(BLACKLIST_FILE).exists() {
            log(LogTag::Blacklist, "INFO", "No blacklist file found, creating new one");
            return Ok(Self::new());
        }

        match fs::read_to_string(BLACKLIST_FILE) {
            Ok(content) => {
                match serde_json::from_str::<Self>(&content) {
                    Ok(blacklist) => {
                        if blacklist.blacklisted_tokens.len() > 0 {
                            log(
                                LogTag::Blacklist,
                                "LOADED",
                                &format!(
                                    "Loaded blacklist with {} entries",
                                    blacklist.blacklisted_tokens.len()
                                )
                            );
                        }
                        Ok(blacklist)
                    }
                    Err(e) => {
                        log(
                            LogTag::Blacklist,
                            "WARN",
                            &format!("Failed to parse blacklist file: {}", e)
                        );
                        Ok(Self::new())
                    }
                }
            }
            Err(e) => {
                log(LogTag::Blacklist, "WARN", &format!("Failed to read blacklist file: {}", e));
                Ok(Self::new())
            }
        }
    }

    /// Save blacklist to file
    pub fn save(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.last_updated = Some(Utc::now());

        let json = serde_json::to_string_pretty(self)?;
        fs::write(BLACKLIST_FILE, json)?;

        // Only log if there are actually blacklisted entries
        if self.blacklisted_tokens.len() > 0 {
            log(
                LogTag::Blacklist,
                "SAVED",
                &format!("Saved blacklist with {} entries", self.blacklisted_tokens.len())
            );
        }
        Ok(())
    }

    /// Check if token is blacklisted
    pub fn is_blacklisted(&self, mint: &str) -> bool {
        self.blacklisted_tokens.contains_key(mint)
    }

    /// Add token to blacklist
    pub fn add_to_blacklist(&mut self, mint: &str, symbol: &str, reason: BlacklistReason) {
        let now = Utc::now();

        let reason_description = match &reason {
            BlacklistReason::LowLiquidity => "Low Liquidity",
            BlacklistReason::PoorPerformance => "Poor Performance",
            BlacklistReason::ManualBlacklist => "Manual",
            BlacklistReason::SystemToken => "System Token",
            BlacklistReason::StableToken => "Stable Token",
        };

        let entry = BlacklistEntry {
            mint: mint.to_string(),
            symbol: symbol.to_string(),
            reason,
            first_occurrence: now,
            last_occurrence: now,
            occurrence_count: 1,
            liquidity_checks: self.tracking_data.get(mint).cloned().unwrap_or_default(),
        };

        self.blacklisted_tokens.insert(mint.to_string(), entry);

        log(
            LogTag::Blacklist,
            "ADDED",
            &format!("Blacklisted {} ({}) - {}", symbol, mint, reason_description)
        );
    }

    /// Check and track token liquidity for potential blacklisting
    pub fn check_and_track_liquidity(
        &mut self,
        mint: &str,
        symbol: &str,
        liquidity_usd: f64,
        token_age_hours: i64
    ) -> bool {
        // Skip if already blacklisted
        if self.is_blacklisted(mint) {
            return false;
        }

        // Only track tokens older than minimum age
        if token_age_hours < MIN_AGE_HOURS {
            return true;
        }

        let now = Utc::now();
        let check = LiquidityCheck {
            timestamp: now,
            liquidity_usd,
            token_age_hours,
        };

        // Add to tracking data
        self.tracking_data.entry(mint.to_string()).or_insert_with(Vec::new).push(check);

        // Check if liquidity is below threshold
        if liquidity_usd < LOW_LIQUIDITY_THRESHOLD {
            let low_liquidity_count = self.tracking_data
                .get(mint)
                .map(
                    |checks|
                        checks
                            .iter()
                            .filter(|c| c.liquidity_usd < LOW_LIQUIDITY_THRESHOLD)
                            .count() as u32
                )
                .unwrap_or(0);

            log(
                LogTag::Blacklist,
                "TRACK",
                &format!(
                    "Low liquidity for {} ({}): ${:.2} USD (count: {})",
                    symbol,
                    mint,
                    liquidity_usd,
                    low_liquidity_count
                )
            );

            // Blacklist if threshold exceeded
            if low_liquidity_count >= MAX_LOW_LIQUIDITY_COUNT {
                self.add_to_blacklist(mint, symbol, BlacklistReason::LowLiquidity);
                return false;
            }
        }

        true
    }

    /// Remove token from blacklist
    pub fn remove_from_blacklist(&mut self, mint: &str) -> bool {
        if let Some(entry) = self.blacklisted_tokens.remove(mint) {
            log(
                LogTag::Blacklist,
                "REMOVED",
                &format!("Removed {} ({}) from blacklist", entry.symbol, mint)
            );
            true
        } else {
            false
        }
    }

    /// Get blacklist statistics
    pub fn get_stats(&self) -> BlacklistStats {
        let mut reason_counts = HashMap::new();

        for entry in self.blacklisted_tokens.values() {
            let reason_str = match entry.reason {
                BlacklistReason::LowLiquidity => "LowLiquidity",
                BlacklistReason::PoorPerformance => "PoorPerformance",
                BlacklistReason::ManualBlacklist => "ManualBlacklist",
                BlacklistReason::SystemToken => "SystemToken",
                BlacklistReason::StableToken => "StableToken",
            };
            *reason_counts.entry(reason_str.to_string()).or_insert(0) += 1;
        }

        BlacklistStats {
            total_blacklisted: self.blacklisted_tokens.len(),
            total_tracked: self.tracking_data.len(),
            reason_breakdown: reason_counts,
        }
    }

    /// Clean old tracking data (older than 7 days)
    pub fn cleanup_old_data(&mut self) {
        let cutoff = Utc::now() - ChronoDuration::days(7);

        for (mint, checks) in self.tracking_data.iter_mut() {
            checks.retain(|check| check.timestamp > cutoff);
        }

        // Remove empty tracking entries
        self.tracking_data.retain(|_, checks| !checks.is_empty());

        log(LogTag::Blacklist, "CLEANUP", "Cleaned old blacklist tracking data");
    }
}

/// Blacklist statistics
#[derive(Debug, Clone)]
pub struct BlacklistStats {
    pub total_blacklisted: usize,
    pub total_tracked: usize,
    pub reason_breakdown: HashMap<String, usize>,
}

// =============================================================================
// GLOBAL BLACKLIST INSTANCE
// =============================================================================

use std::sync::Mutex;
use once_cell::sync::Lazy;

/// Global blacklist instance
pub static TOKEN_BLACKLIST: Lazy<Mutex<TokenBlacklist>> = Lazy::new(|| {
    match TokenBlacklist::load() {
        Ok(blacklist) => Mutex::new(blacklist),
        Err(e) => {
            log(
                LogTag::Blacklist,
                "ERROR",
                &format!("Failed to load blacklist, using empty: {}", e)
            );
            Mutex::new(TokenBlacklist::new())
        }
    }
});

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/// Check if token is blacklisted (thread-safe)
pub fn is_token_blacklisted(mint: &str) -> bool {
    match TOKEN_BLACKLIST.try_lock() {
        Ok(blacklist) => blacklist.is_blacklisted(mint),
        Err(_) => {
            log(LogTag::Blacklist, "WARN", "Could not acquire blacklist lock for check");
            false // Assume not blacklisted if can't check
        }
    }
}

/// Track token liquidity for blacklisting (thread-safe)
pub fn check_and_track_liquidity(
    mint: &str,
    symbol: &str,
    liquidity_usd: f64,
    token_age_hours: i64
) -> bool {
    match TOKEN_BLACKLIST.try_lock() {
        Ok(mut blacklist) => {
            let result = blacklist.check_and_track_liquidity(
                mint,
                symbol,
                liquidity_usd,
                token_age_hours
            );

            // Only save if something was actually blacklisted (result is false)
            if !result {
                if let Err(e) = blacklist.save() {
                    log(
                        LogTag::Blacklist,
                        "WARN",
                        &format!("Failed to save blacklist after adding entry: {}", e)
                    );
                }
            }

            result
        }
        Err(_) => {
            log(LogTag::Blacklist, "WARN", "Could not acquire blacklist lock for tracking");
            true // Assume allowed if can't track
        }
    }
}

/// Get blacklist statistics (thread-safe)
pub fn get_blacklist_stats() -> Option<BlacklistStats> {
    match TOKEN_BLACKLIST.try_lock() {
        Ok(blacklist) => Some(blacklist.get_stats()),
        Err(_) => {
            log(LogTag::Blacklist, "WARN", "Could not acquire blacklist lock for stats");
            None
        }
    }
}

/// Manual blacklist addition (thread-safe)
pub fn add_to_blacklist_manual(mint: &str, symbol: &str) -> bool {
    match TOKEN_BLACKLIST.try_lock() {
        Ok(mut blacklist) => {
            blacklist.add_to_blacklist(mint, symbol, BlacklistReason::ManualBlacklist);

            if let Err(e) = blacklist.save() {
                log(
                    LogTag::Blacklist,
                    "WARN",
                    &format!("Failed to save blacklist after manual addition: {}", e)
                );
                false
            } else {
                true
            }
        }
        Err(_) => {
            log(LogTag::Blacklist, "WARN", "Could not acquire blacklist lock for manual addition");
            false
        }
    }
}

// =============================================================================
// CENTRALIZED TOKEN EXCLUSION SYSTEM
// =============================================================================

/// Check if token is a system or stable token that should be excluded from trading
pub fn is_system_or_stable_token(mint: &str) -> bool {
    SYSTEM_STABLE_TOKENS.contains(&mint)
}

/// Check if token should be excluded from trading (blacklisted OR system/stable)
/// This is the main function that should be used everywhere for token exclusion checks
pub fn is_token_excluded_from_trading(mint: &str) -> bool {
    // Check system/stable tokens first (fastest)
    if is_system_or_stable_token(mint) {
        return true;
    }

    // Check dynamic blacklist
    is_token_blacklisted(mint)
}

/// Add system/stable token to blacklist for permanent exclusion
pub fn add_system_token_to_blacklist(mint: &str, symbol: &str) -> bool {
    match TOKEN_BLACKLIST.try_lock() {
        Ok(mut blacklist) => {
            blacklist.add_to_blacklist(mint, symbol, BlacklistReason::SystemToken);

            if let Err(e) = blacklist.save() {
                log(
                    LogTag::Blacklist,
                    "WARN",
                    &format!("Failed to save blacklist after adding system token: {}", e)
                );
                false
            } else {
                true
            }
        }
        Err(_) => {
            log(LogTag::Blacklist, "WARN", "Could not acquire blacklist lock for system token");
            false
        }
    }
}

/// Add stable token to blacklist for permanent exclusion
pub fn add_stable_token_to_blacklist(mint: &str, symbol: &str) -> bool {
    match TOKEN_BLACKLIST.try_lock() {
        Ok(mut blacklist) => {
            blacklist.add_to_blacklist(mint, symbol, BlacklistReason::StableToken);

            if let Err(e) = blacklist.save() {
                log(
                    LogTag::Blacklist,
                    "WARN",
                    &format!("Failed to save blacklist after adding stable token: {}", e)
                );
                false
            } else {
                true
            }
        }
        Err(_) => {
            log(LogTag::Blacklist, "WARN", "Could not acquire blacklist lock for stable token");
            false
        }
    }
}

/// Initialize system and stable tokens in blacklist (run at startup)
pub fn initialize_system_stable_blacklist() {
    for &mint in SYSTEM_STABLE_TOKENS {
        if !is_token_blacklisted(mint) {
            let symbol = match mint {
                "So11111111111111111111111111111111111111112" => "SOL",
                "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v" => "USDC",
                "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB" => "USDT",
                "7dHbWXmci3dT8UFYWYZweBLXgycu7Y3iL6trKn1Y7ARj" => "stSOL",
                "mSoLzYCxHdYgdzU16g5QSh3i5K3z3KZK7ytfqcJm7So" => "mSOL",
                "11111111111111111111111111111111" => "SYSTEM",
                "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA" => "TOKEN_PROGRAM",
                "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb" => "TOKEN_2022",
                _ => "UNKNOWN",
            };

            let reason = match mint {
                | "11111111111111111111111111111111"
                | "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
                | "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb" => BlacklistReason::SystemToken,
                _ => BlacklistReason::StableToken,
            };

            if let Ok(mut blacklist) = TOKEN_BLACKLIST.try_lock() {
                blacklist.add_to_blacklist(mint, symbol, reason);
                let _ = blacklist.save(); // Ignore save errors during initialization
            }
        }
    }

    log(LogTag::Blacklist, "INIT", "System and stable tokens initialized in blacklist");
}
