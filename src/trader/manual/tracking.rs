//! Manual trade tracking and history

use crate::logger::{self, LogTag};
use crate::trader::constants::MANUAL_TRADE_HISTORY_LIMIT;
use crate::trader::types::TradeResult;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;
use tokio::sync::RwLock;

static MANUAL_TRADE_HISTORY: OnceLock<RwLock<Vec<ManualTradeRecord>>> = OnceLock::new();

fn get_history_storage() -> &'static RwLock<Vec<ManualTradeRecord>> {
    MANUAL_TRADE_HISTORY.get_or_init(|| RwLock::new(Vec::new()))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManualTradeRecord {
    pub timestamp: DateTime<Utc>,
    pub mint: String,
    pub action: String,
    pub reason: String,
    pub success: bool,
    pub tx_signature: Option<String>,
    pub size_sol: Option<f64>,
    pub price_sol: Option<f64>,
    pub error: Option<String>,
}

/// Record a manual trade
pub async fn record_manual_trade(result: &TradeResult) -> Result<(), String> {
    let record = ManualTradeRecord {
        timestamp: result.execution_timestamp,
        mint: result.decision.mint.clone(),
        action: format!("{:?}", result.decision.action),
        reason: format!("{:?}", result.decision.reason),
        success: result.success,
        tx_signature: result.tx_signature.clone(),
        size_sol: result.executed_size_sol,
        price_sol: result.executed_price_sol,
        error: result.error.clone(),
    };

    let mut history = get_history_storage().write().await;
    history.push(record.clone());

    // Keep only last N records
    if history.len() > MANUAL_TRADE_HISTORY_LIMIT {
        let excess = history.len() - MANUAL_TRADE_HISTORY_LIMIT;
        history.drain(0..excess);
    }

    logger::info(
        LogTag::Trader,
        &format!(
            "Recorded manual trade: action={}, mint={}, success={}",
            record.action, record.mint, record.success
        ),
    );

    Ok(())
}

/// Get manual trade history (most recent first)
pub async fn get_manual_trade_history(limit: usize) -> Vec<ManualTradeRecord> {
    let history = get_history_storage().read().await;
    let start = if history.len() > limit {
        history.len() - limit
    } else {
        0
    };

    history[start..].iter().rev().cloned().collect()
}

/// Clear manual trade history
pub async fn clear_history() {
    let mut history = get_history_storage().write().await;
    history.clear();
    logger::info(LogTag::Trader, "Cleared manual trade history");
}
