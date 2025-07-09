#![allow(warnings)]
use crate::prelude::*;

use std::collections::HashMap;
use once_cell::sync::Lazy;
use tokio::sync::RwLock;
use chrono::{ DateTime, Utc };
use serde::{ Serialize, Deserialize };
use tokio::{ fs, time::{ sleep, Duration } };
use anyhow::Result;

pub const OPEN_POS_FILE: &str = "open_positions.json";
pub const CLOSED_POS_FILE: &str = "recent_closed.json";
pub const ALL_CLOSED_POS_FILE: &str = "all_closed_positions.json";
pub const TRADING_HISTORY_FILE: &str = "trading_history.json";

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

#[derive(Clone, Serialize, Deserialize)]
pub struct TradingSnapshot {
    pub timestamp: DateTime<Utc>,
    pub total_pnl: f64,
    pub total_invested: f64,
    pub active_positions: usize,
    pub closed_positions: usize,
    pub win_rate: f64,
}

// in-memory stores -----------------------------------------------------------
pub static OPEN_POSITIONS: Lazy<RwLock<HashMap<String, Position>>> = Lazy::new(||
    RwLock::new(HashMap::new())
);

pub static RECENT_CLOSED_POSITIONS: Lazy<RwLock<HashMap<String, Position>>> = Lazy::new(||
    RwLock::new(HashMap::new())
);

// New: Store ALL closed positions for complete history
pub static ALL_CLOSED_POSITIONS: Lazy<RwLock<HashMap<String, Position>>> = Lazy::new(||
    RwLock::new(HashMap::new())
);

// New: Store trading history snapshots for analytics
pub static TRADING_HISTORY: Lazy<RwLock<Vec<TradingSnapshot>>> = Lazy::new(||
    RwLock::new(Vec::new())
);

// load at start --------------------------------------------------------------
pub async fn load_cache() -> Result<()> {
    if let Ok(data) = fs::read(OPEN_POS_FILE).await {
        let map: HashMap<String, Position> = serde_json::from_slice(&data)?;
        *OPEN_POSITIONS.write().await = map;
    }
    if let Ok(data) = fs::read(CLOSED_POS_FILE).await {
        let map: HashMap<String, Position> = serde_json::from_slice(&data)?;
        *RECENT_CLOSED_POSITIONS.write().await = map;
    }

    // Load complete closed positions history
    if let Ok(data) = fs::read(ALL_CLOSED_POS_FILE).await {
        let map: HashMap<String, Position> = serde_json::from_slice(&data)?;
        *ALL_CLOSED_POSITIONS.write().await = map;
    }

    // Load trading history snapshots
    if let Ok(data) = fs::read(TRADING_HISTORY_FILE).await {
        let history: Vec<TradingSnapshot> = serde_json::from_slice(&data)?;
        *TRADING_HISTORY.write().await = history;
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
    let snapshot = RECENT_CLOSED_POSITIONS.read().await.clone();
    let _ = atomic_write(CLOSED_POS_FILE, &serde_json::to_vec_pretty(&snapshot).unwrap()).await;
}

pub async fn save_all_closed() {
    let snapshot = ALL_CLOSED_POSITIONS.read().await.clone();
    let _ = atomic_write(ALL_CLOSED_POS_FILE, &serde_json::to_vec_pretty(&snapshot).unwrap()).await;
}

pub async fn save_trading_history() {
    let snapshot = TRADING_HISTORY.read().await.clone();
    let _ = atomic_write(
        TRADING_HISTORY_FILE,
        &serde_json::to_vec_pretty(&snapshot).unwrap()
    ).await;
}

// background autosave --------------------------------------------------------
pub async fn autosave_loop() {
    use std::sync::atomic::Ordering;
    let mut snapshot_counter = 0;

    loop {
        if SHUTDOWN.load(Ordering::SeqCst) {
            break;
        }

        // run all writes concurrently
        futures::join!(save_open(), save_closed(), save_all_closed(), save_trading_history());

        // the heavy pool-cache write on a blocking worker
        let _ = tokio::task::spawn_blocking(|| flush_pool_cache_to_disk_nonblocking()).await;

        // Take trading snapshot every 5 minutes (150 cycles of 2 seconds)
        snapshot_counter += 1;
        if snapshot_counter >= 150 {
            let _ = take_trading_snapshot().await;
            snapshot_counter = 0;
        }

        sleep(Duration::from_secs(2)).await;
    }
}

// Helper function to move position from open to closed
pub async fn close_position(token_id: &str, sol_received: f64) -> Result<()> {
    let mut open_positions = OPEN_POSITIONS.write().await;
    let mut recent_closed = RECENT_CLOSED_POSITIONS.write().await;
    let mut all_closed = ALL_CLOSED_POSITIONS.write().await;

    if let Some(mut position) = open_positions.remove(token_id) {
        position.sol_received = sol_received;
        position.close_time = Some(chrono::Utc::now());

        // Add to recent closed (limited to last 100 for performance)
        recent_closed.insert(token_id.to_string(), position.clone());
        if recent_closed.len() > 100 {
            // Remove oldest entry
            if
                let Some(oldest_key) = recent_closed
                    .keys()
                    .min_by_key(|k| recent_closed.get(*k).unwrap().close_time.unwrap())
                    .cloned()
            {
                recent_closed.remove(&oldest_key);
            }
        }

        // Add to complete history
        all_closed.insert(token_id.to_string(), position);

        // Save changes
        futures::join!(save_open(), save_closed(), save_all_closed());
    }

    Ok(())
}

// Helper function to take trading snapshot
pub async fn take_trading_snapshot() -> Result<()> {
    let open_positions = OPEN_POSITIONS.read().await;
    let all_closed = ALL_CLOSED_POSITIONS.read().await;

    let total_invested: f64 =
        open_positions
            .values()
            .map(|p| p.sol_spent)
            .sum::<f64>() +
        all_closed
            .values()
            .map(|p| p.sol_spent)
            .sum::<f64>();

    let total_received: f64 = all_closed
        .values()
        .map(|p| p.sol_received)
        .sum::<f64>();

    let total_pnl =
        total_received -
        all_closed
            .values()
            .map(|p| p.sol_spent)
            .sum::<f64>();

    let profitable_trades = all_closed
        .values()
        .filter(|p| p.sol_received > p.sol_spent)
        .count();

    let win_rate = if !all_closed.is_empty() {
        ((profitable_trades as f64) / (all_closed.len() as f64)) * 100.0
    } else {
        0.0
    };

    let snapshot = TradingSnapshot {
        timestamp: chrono::Utc::now(),
        total_pnl,
        total_invested,
        active_positions: open_positions.len(),
        closed_positions: all_closed.len(),
        win_rate,
    };

    let mut history = TRADING_HISTORY.write().await;
    history.push(snapshot);

    // Keep only last 1000 snapshots
    if history.len() > 1000 {
        history.remove(0);
    }

    save_trading_history().await;

    Ok(())
}
