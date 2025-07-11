#![allow(warnings)]
use crate::prelude::*;
use crate::shutdown;

use std::collections::HashMap;
use once_cell::sync::Lazy;
use tokio::sync::RwLock;
use chrono::{ DateTime, Utc };
use serde::{ Serialize, Deserialize };
use tokio::{ fs, time::{ sleep, Duration } };
use anyhow::Result;

pub const OPEN_POS_FILE: &str = "open_positions.json";
pub const CLOSED_POS_FILE: &str = "closed_positions.json";
pub const WATCHLIST_FILE: &str = "watchlist_tokens.json";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Position {
    pub entry_price: f64,
    pub peak_price: f64,
    pub dca_count: u8,
    pub token_amount: f64,
    pub sol_spent: f64,
    pub sol_received: f64,
    pub open_time: DateTime<Utc>,
    pub close_time: Option<DateTime<Utc>>,
    pub last_dca_price: f64,
    pub last_dca_time: DateTime<Utc>, // Time of last DCA for cooldown tracking
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WatchlistEntry {
    pub mint: String,
    pub symbol: String,
    pub name: String,
    pub first_traded: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
    pub total_trades: u32,
    pub last_price: f64,
    pub priority_score: f64, // Higher score = higher priority for monitoring
}

// in-memory stores -----------------------------------------------------------
pub static OPEN_POSITIONS: Lazy<RwLock<HashMap<String, Position>>> = Lazy::new(||
    RwLock::new(HashMap::new())
);

pub static CLOSED_POSITIONS: Lazy<RwLock<HashMap<String, Position>>> = Lazy::new(||
    RwLock::new(HashMap::new())
);

// NEW: Persistent watchlist for tokens we've previously traded
pub static WATCHLIST_TOKENS: Lazy<RwLock<HashMap<String, WatchlistEntry>>> = Lazy::new(||
    RwLock::new(HashMap::new())
);

// load at start --------------------------------------------------------------
pub async fn load_cache() -> Result<()> {
    if let Ok(data) = fs::read(OPEN_POS_FILE).await {
        let map: HashMap<String, Position> = serde_json::from_slice(&data)?;
        *OPEN_POSITIONS.write().await = map;
    }
    if let Ok(data) = fs::read(CLOSED_POS_FILE).await {
        let map: HashMap<String, Position> = serde_json::from_slice(&data)?;
        *CLOSED_POSITIONS.write().await = map;
    }

    // Load watchlist
    if let Ok(data) = fs::read(WATCHLIST_FILE).await {
        let map: HashMap<String, WatchlistEntry> = serde_json::from_slice(&data)?;

        // Filter out excluded tokens from watchlist loading
        let blacklist = crate::configs::BLACKLIST.read().await;
        let filtered_count = map
            .iter()
            .filter(|(mint, _)| !blacklist.contains(*mint))
            .count();
        drop(blacklist);

        *WATCHLIST_TOKENS.write().await = map;
        println!("📋 Loaded {} tokens in watchlist (excluded tokens filtered)", filtered_count);
    }

    Ok(())
}

// save helpers ---------------------------------------------------------------
pub async fn atomic_write(path: &str, bytes: &[u8]) -> std::io::Result<()> {
    use std::path::Path;
    use tokio::fs;

    let tmp = format!("{path}.tmp");
    fs::write(&tmp, bytes).await?;
    fs::rename(&tmp, Path::new(path)).await
}

pub async fn save_open() {
    let snapshot = OPEN_POSITIONS.read().await.clone();
    let _ = atomic_write(OPEN_POS_FILE, &serde_json::to_vec_pretty(&snapshot).unwrap()).await;
}

pub async fn save_closed() {
    let snapshot = CLOSED_POSITIONS.read().await.clone();
    let _ = atomic_write(CLOSED_POS_FILE, &serde_json::to_vec_pretty(&snapshot).unwrap()).await;
}

pub async fn save_watchlist() {
    let snapshot = WATCHLIST_TOKENS.read().await.clone();
    let _ = atomic_write(WATCHLIST_FILE, &serde_json::to_vec_pretty(&snapshot).unwrap()).await;
}

// background autosave --------------------------------------------------------
pub async fn autosave_loop() {
    use std::sync::atomic::Ordering;

    loop {
        if shutdown::is_shutdown_requested() {
            break;
        }

        // run all writes concurrently
        futures::join!(save_open(), save_closed(), save_watchlist());

        // the heavy pool-cache write on a blocking worker
        let _ = tokio::task::spawn_blocking(|| flush_pool_cache_to_disk_nonblocking()).await;

        sleep(Duration::from_secs(2)).await;
    }
}

// Helper function to move position from open to closed
pub async fn close_position(token_id: &str, sol_received: f64) -> Result<()> {
    let mut open_positions = OPEN_POSITIONS.write().await;
    let mut closed_positions = CLOSED_POSITIONS.write().await;

    if let Some(mut position) = open_positions.remove(token_id) {
        position.sol_received = sol_received;
        position.close_time = Some(chrono::Utc::now());

        // Add to closed positions (keep only last 100 for performance)
        closed_positions.insert(token_id.to_string(), position.clone());

        // Add to watchlist for future monitoring
        // Try to get token details from TOKENS
        {
            let tokens = TOKENS.read().await;
            if let Some(token) = tokens.iter().find(|t| t.mint == token_id) {
                add_to_watchlist(
                    &token.mint,
                    &token.symbol,
                    &token.name,
                    position.entry_price
                ).await;
                println!(
                    "📋 Added {} ({}) to watchlist for continuous monitoring",
                    token.symbol,
                    token.mint
                );
            } else {
                // Fallback: add with minimal info
                add_to_watchlist(token_id, "UNKNOWN", "UNKNOWN", position.entry_price).await;
                println!("📋 Added {} to watchlist (unknown symbol)", token_id);
            }
        }

        // Keep only the most recent 100 positions (by close_time)
        if closed_positions.len() > 100 {
            if
                let Some((oldest_mint, _)) = closed_positions
                    .iter()
                    .min_by_key(|(_, pos)| pos.close_time)
                    .map(|(mint, _)| (mint.clone(), ()))
            {
                closed_positions.remove(&oldest_mint);
            }
        }

        // Save changes
        futures::join!(save_open(), save_closed());
    }

    Ok(())
}

// Watchlist management functions ---------------------------------------------

/// Add a token to the watchlist when we first trade it
pub async fn add_to_watchlist(mint: &str, symbol: &str, name: &str, price: f64) {
    // Check if token is excluded from trading
    if crate::configs::BLACKLIST.read().await.contains(mint) {
        println!("🚫 [WATCHLIST] Skipping excluded token: {} ({}) - {}", symbol, name, mint);
        return;
    }

    let mut watchlist = WATCHLIST_TOKENS.write().await;

    let now = chrono::Utc::now();

    if let Some(entry) = watchlist.get_mut(mint) {
        // Update existing entry
        entry.last_seen = now;
        entry.total_trades += 1;
        entry.last_price = price;
        entry.priority_score += 1.0; // Increase priority each time we trade
    } else {
        // Add new entry
        let entry = WatchlistEntry {
            mint: mint.to_string(),
            symbol: symbol.to_string(),
            name: name.to_string(),
            first_traded: now,
            last_seen: now,
            total_trades: 1,
            last_price: price,
            priority_score: 10.0, // Start with base priority
        };
        watchlist.insert(mint.to_string(), entry);
    }
}

/// Get all watchlist tokens sorted by priority
pub async fn get_watchlist_tokens() -> Vec<WatchlistEntry> {
    let watchlist = WATCHLIST_TOKENS.read().await;
    let blacklist = crate::configs::BLACKLIST.read().await;

    let mut tokens: Vec<WatchlistEntry> = watchlist
        .values()
        .filter(|entry| !blacklist.contains(&entry.mint))
        .cloned()
        .collect();

    drop(blacklist);

    // Sort by priority score (highest first)
    tokens.sort_by(|a, b|
        b.priority_score.partial_cmp(&a.priority_score).unwrap_or(std::cmp::Ordering::Equal)
    );

    tokens
}

/// Update last seen time for a watchlist token
pub async fn update_watchlist_token_seen(mint: &str, price: f64) {
    let mut watchlist = WATCHLIST_TOKENS.write().await;
    if let Some(entry) = watchlist.get_mut(mint) {
        entry.last_seen = chrono::Utc::now();
        entry.last_price = price;
    }
}

/// Check if a token is in our watchlist
pub async fn is_watchlist_token(mint: &str) -> bool {
    let watchlist = WATCHLIST_TOKENS.read().await;
    watchlist.contains_key(mint)
}

/// Get priority watchlist tokens (top 50 by priority score)
pub async fn get_priority_watchlist_tokens(limit: usize) -> Vec<String> {
    let tokens = get_watchlist_tokens().await;
    tokens
        .into_iter()
        .take(limit)
        .map(|entry| entry.mint)
        .collect()
}
