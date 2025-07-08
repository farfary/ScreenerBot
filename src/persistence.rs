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

#[derive(Clone, Serialize, Deserialize)]
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
}

// in-memory stores -----------------------------------------------------------
pub static OPEN_POSITIONS: Lazy<RwLock<HashMap<String, Position>>> = Lazy::new(||
    RwLock::new(HashMap::new())
);

pub static RECENT_CLOSED_POSITIONS: Lazy<RwLock<HashMap<String, Position>>> = Lazy::new(||
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
        *RECENT_CLOSED_POSITIONS.write().await = map;
    }
    
    Ok(())
}

// save helpers ---------------------------------------------------------------
async fn atomic_write(path: &str, bytes: &[u8]) -> std::io::Result<()> {
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

// background autosave --------------------------------------------------------
pub async fn autosave_loop() {
    use std::sync::atomic::Ordering;
    loop {
        if SHUTDOWN.load(Ordering::SeqCst) {
            break;
        }

        // run both writes concurrently
        futures::join!(save_open(), save_closed());

        // the heavy pool-cache write on a blocking worker
        let _ = tokio::task::spawn_blocking(|| flush_pool_cache_to_disk_nonblocking()).await;

        sleep(Duration::from_secs(2)).await;
    }
}
