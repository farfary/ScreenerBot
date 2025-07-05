// Cargo.toml ──────────────────────────────────────────────────────────────────
// [dependencies]
// tokio       = { version = "1", features = ["full"] }
// once_cell   = "1"
// serde       = { version = "1", features = ["derive"] }
// serde_json  = "1"
// chrono      = { version = "0.4", features = ["serde"] }
// anyhow      = "1"
// ──────────────────────────────────────────────────────────────────────────────



// persistence.rs ──────────────────────────────────────────────────────────────
use std::collections::HashMap;
use once_cell::sync::Lazy;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};
use tokio::{fs, time::{sleep, Duration}};
use anyhow::Result;

pub const OPEN_POS_FILE:   &str = "open_positions.json";
pub const CLOSED_POS_FILE: &str = "recent_closed.json";

#[derive(Clone, Serialize, Deserialize)]
pub struct Position {
    pub entry_price:     f64,
    pub peak_price:      f64,
    pub dca_count:       u8,
    pub token_amount:    f64,
    pub sol_spent:       f64,
    pub sol_received:    f64,
    pub open_time:       DateTime<Utc>,
    pub close_time:      Option<DateTime<Utc>>,
    pub last_dca_price:  f64,
}

// in-memory stores ------------------------------------------------------------
pub static OPEN_POSITIONS: Lazy<RwLock<HashMap<String, Position>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

pub static RECENT_CLOSED_POSITIONS: Lazy<RwLock<Vec<Position>>> =
    Lazy::new(|| RwLock::new(Vec::new()));

// load at startup ------------------------------------------------------------
pub async fn load_cache() -> Result<()> {
    if let Ok(data) = fs::read(OPEN_POS_FILE).await {
        let map: HashMap<String, Position> = serde_json::from_slice(&data)?;
        *OPEN_POSITIONS.write().await = map;
    }
    if let Ok(data) = fs::read(CLOSED_POS_FILE).await {
        let vec: Vec<Position> = serde_json::from_slice(&data)?;
        *RECENT_CLOSED_POSITIONS.write().await = vec;
    }
    Ok(())
}

// save helpers ---------------------------------------------------------------
pub async fn save_open() -> Result<()> {
    let map = OPEN_POSITIONS.read().await.clone();
    fs::write(OPEN_POS_FILE, serde_json::to_vec_pretty(&map)?).await?;
    Ok(())
}

pub async fn save_closed() -> Result<()> {
    let vec = RECENT_CLOSED_POSITIONS.read().await.clone();
    fs::write(CLOSED_POS_FILE, serde_json::to_vec_pretty(&vec)?).await?;
    Ok(())
}

// background autosave --------------------------------------------------------
pub async fn autosave_loop() {
    loop {
        let _ = save_open().await;
        let _ = save_closed().await;
        sleep(Duration::from_secs(10)).await;   // every 10 s
    }
}
// ──────────────────────────────────────────────────────────────────────────────

