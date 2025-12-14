//! Trade Monitor for Trade Watcher
//!
//! Monitors pools for trades and triggers actions based on watch configuration.

use once_cell::sync::Lazy;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;

use super::types::{DetectedTrade, PoolSource, TradeMonitorStatus, WatchType};
use crate::apis::manager::get_api_manager;
use crate::logger::{self, LogTag};
use crate::notifications::{queue_notification, Notification};
use crate::tools::database::{
    get_active_watched_tokens, get_watched_tokens, update_watched_token_tracking, WatchedToken,
};
use crate::wallets::list_wallets;

// =============================================================================
// CONSTANTS
// =============================================================================

/// Poll interval for trade monitoring (seconds)
const POLL_INTERVAL_SECS: u64 = 5;

/// Minimum volume filter for trades (USD)
const MIN_TRADE_VOLUME_USD: f64 = 10.0;

// =============================================================================
// GLOBAL STATE
// =============================================================================

static TRADE_MONITOR: Lazy<Arc<RwLock<TradeMonitor>>> =
    Lazy::new(|| Arc::new(RwLock::new(TradeMonitor::new())));

// =============================================================================
// TRADE MONITOR
// =============================================================================

/// Trade monitor state
pub struct TradeMonitor {
    /// Whether the monitor is currently running
    is_running: bool,
    /// Own wallet addresses to filter out
    own_wallets: HashSet<String>,
    /// Last processed signature per pool
    last_signatures: HashMap<String, String>,
    /// Total trades detected
    total_trades_detected: i32,
    /// Total actions triggered
    total_actions_triggered: i32,
}

impl TradeMonitor {
    /// Create a new trade monitor
    pub fn new() -> Self {
        Self {
            is_running: false,
            own_wallets: HashSet::new(),
            last_signatures: HashMap::new(),
            total_trades_detected: 0,
            total_actions_triggered: 0,
        }
    }

    /// Start the trade monitor
    pub async fn start(&mut self) {
        if self.is_running {
            logger::debug(LogTag::Tools, "[TRADE_WATCHER] Monitor already running");
            return;
        }

        // Load own wallets to filter out
        if let Ok(wallets) = list_wallets(true).await {
            self.own_wallets = wallets.into_iter().map(|w| w.address).collect();
            logger::debug(
                LogTag::Tools,
                &format!(
                    "[TRADE_WATCHER] Loaded {} own wallets for filtering",
                    self.own_wallets.len()
                ),
            );
        }

        self.is_running = true;
        logger::info(LogTag::Tools, "[TRADE_WATCHER] Trade monitor started");

        // Spawn monitoring task
        tokio::spawn(async move {
            trade_monitor_loop().await;
        });
    }

    /// Stop the trade monitor
    pub fn stop(&mut self) {
        if !self.is_running {
            return;
        }
        self.is_running = false;
        logger::info(LogTag::Tools, "[TRADE_WATCHER] Trade monitor stopped");
    }

    /// Check if monitor is running
    pub fn is_running(&self) -> bool {
        self.is_running
    }

    /// Get monitor status
    pub fn status(&self) -> TradeMonitorStatus {
        let (watched_count, active_count) = match get_watched_tokens() {
            Ok(tokens) => {
                let active = tokens.iter().filter(|t| t.is_active).count();
                (tokens.len(), active)
            }
            Err(_) => (0, 0),
        };

        TradeMonitorStatus {
            is_running: self.is_running,
            watched_count,
            active_count,
            total_trades_detected: self.total_trades_detected,
            total_actions_triggered: self.total_actions_triggered,
        }
    }
}

impl Default for TradeMonitor {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// MONITORING LOOP
// =============================================================================

/// Main monitoring loop
async fn trade_monitor_loop() {
    let api_manager = get_api_manager();

    loop {
        // Check if still running
        {
            let monitor = TRADE_MONITOR.read().await;
            if !monitor.is_running {
                logger::debug(LogTag::Tools, "[TRADE_WATCHER] Monitor loop exiting");
                break;
            }
        }

        // Get active watched tokens
        let watched = match get_active_watched_tokens() {
            Ok(w) => w,
            Err(e) => {
                logger::debug(
                    LogTag::Tools,
                    &format!("[TRADE_WATCHER] Failed to get watched tokens: {}", e),
                );
                tokio::time::sleep(tokio::time::Duration::from_secs(POLL_INTERVAL_SECS)).await;
                continue;
            }
        };

        if watched.is_empty() {
            // No tokens to watch, sleep and retry
            tokio::time::sleep(tokio::time::Duration::from_secs(POLL_INTERVAL_SECS)).await;
            continue;
        }

        // Process each watched token
        for token in watched {
            // Only GeckoTerminal supports trade fetching currently
            if token.pool_source.to_lowercase() == "geckoterminal" {
                process_geckoterminal_trades(&api_manager.geckoterminal, &token).await;
            }
            // TODO: Add DexScreener trade fetching when available
        }

        // Poll interval
        tokio::time::sleep(tokio::time::Duration::from_secs(POLL_INTERVAL_SECS)).await;
    }
}

/// Process trades from GeckoTerminal for a watched token
async fn process_geckoterminal_trades(
    client: &crate::apis::geckoterminal::GeckoTerminalClient,
    token: &WatchedToken,
) {
    // Fetch trades
    let trades_response = match client
        .fetch_pool_trades(
            "solana",
            &token.pool_address,
            Some(MIN_TRADE_VOLUME_USD),
            None,
        )
        .await
    {
        Ok(t) => t,
        Err(e) => {
            logger::debug(
                LogTag::Tools,
                &format!(
                    "[TRADE_WATCHER] Failed to fetch trades for pool {}: {}",
                    token.pool_address, e
                ),
            );
            return;
        }
    };

    // Get own wallets for filtering
    let own_wallets = {
        let monitor = TRADE_MONITOR.read().await;
        monitor.own_wallets.clone()
    };

    // Get last processed signature
    let last_sig = {
        let monitor = TRADE_MONITOR.read().await;
        monitor.last_signatures.get(&token.pool_address).cloned()
    };

    let mut new_trades_count = 0;
    let mut latest_sig: Option<String> = None;

    for trade_data in trades_response.data {
        let attrs = &trade_data.attributes;

        // Skip if we've already processed this trade
        if let Some(ref last) = last_sig {
            if attrs.tx_hash == *last {
                break; // We've caught up to previously processed trades
            }
        }

        // Skip own wallet trades
        if own_wallets.contains(&attrs.tx_from_address) {
            continue;
        }

        // Track latest signature
        if latest_sig.is_none() {
            latest_sig = Some(attrs.tx_hash.clone());
        }

        new_trades_count += 1;

        // Create detected trade
        let detected_trade = DetectedTrade {
            signature: attrs.tx_hash.clone(),
            trade_type: attrs.kind.clone(),
            wallet: attrs.tx_from_address.clone(),
            amount_base: attrs
                .from_token_amount
                .as_ref()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0.0),
            amount_quote: attrs
                .to_token_amount
                .as_ref()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0.0),
            price: attrs
                .price_from_in_usd
                .as_ref()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0.0),
            volume_usd: attrs.volume_in_usd.parse().unwrap_or(0.0),
            timestamp: chrono::DateTime::parse_from_rfc3339(&attrs.block_timestamp)
                .map(|dt| dt.timestamp())
                .unwrap_or(0),
        };

        // Process the detected trade
        process_detected_trade(token, &detected_trade).await;
    }

    // Update tracking
    if new_trades_count > 0 {
        if let Some(sig) = latest_sig {
            // Update in-memory state
            {
                let mut monitor = TRADE_MONITOR.write().await;
                monitor
                    .last_signatures
                    .insert(token.pool_address.clone(), sig.clone());
                monitor.total_trades_detected += new_trades_count;
            }

            // Update database
            let _ = update_watched_token_tracking(
                token.id,
                Some(&chrono::Utc::now().to_rfc3339()),
                Some(&sig),
                Some(token.trades_detected + new_trades_count),
                None,
            );

            logger::debug(
                LogTag::Tools,
                &format!(
                    "[TRADE_WATCHER] Detected {} new trades for {} ({})",
                    new_trades_count,
                    token.symbol.as_deref().unwrap_or(&token.mint),
                    token.pool_address
                ),
            );
        }
    }
}

/// Process a detected trade and trigger actions if configured
async fn process_detected_trade(token: &WatchedToken, trade: &DetectedTrade) {
    let watch_type = WatchType::from_str(&token.watch_type);
    let symbol = token.symbol.as_deref().unwrap_or("???");

    // Send notification
    queue_notification(Notification::trade_alert(
        symbol.to_string(),
        token.mint.clone(),
        &trade.trade_type,
        trade.volume_usd / 200.0, // Rough SOL conversion (assuming ~$200/SOL)
        trade.wallet.clone(),
    ));

    // Determine if action should be triggered
    let should_trigger = match watch_type {
        WatchType::BuyOnSell => trade.trade_type == "sell",
        WatchType::SellOnBuy => trade.trade_type == "buy",
        WatchType::NotifyOnly => false,
    };

    if !should_trigger {
        return;
    }

    // Check trigger amount threshold
    if let Some(trigger_sol) = token.trigger_amount_sol {
        let trade_sol = trade.volume_usd / 200.0; // Rough conversion
        if trade_sol < trigger_sol {
            logger::debug(
                LogTag::Tools,
                &format!(
                    "[TRADE_WATCHER] Trade below trigger threshold for {}: {:.4} < {:.4} SOL",
                    symbol, trade_sol, trigger_sol
                ),
            );
            return;
        }
    }

    // Log action intent
    if let Some(amount) = token.action_amount_sol {
        let action = match watch_type {
            WatchType::BuyOnSell => "BUY",
            WatchType::SellOnBuy => "SELL",
            WatchType::NotifyOnly => return,
        };

        logger::info(
            LogTag::Tools,
            &format!(
                "[TRADE_WATCHER] Triggering {} for {} ({:.4} SOL) - detected external {}",
                action, symbol, amount, trade.trade_type
            ),
        );

        // Update action triggered count
        {
            let mut monitor = TRADE_MONITOR.write().await;
            monitor.total_actions_triggered += 1;
        }

        let _ = update_watched_token_tracking(
            token.id,
            None,
            None,
            None,
            Some(token.actions_triggered + 1),
        );

        // TODO: Execute swap via manual swap module
        // For now, just log the intent
        // crate::tools::swap_executor::tool_buy(...)
        // crate::tools::swap_executor::tool_sell(...)
    }
}

// =============================================================================
// PUBLIC API
// =============================================================================

/// Start the trade monitor
pub async fn start_trade_monitor() {
    let mut monitor = TRADE_MONITOR.write().await;
    monitor.start().await;
}

/// Stop the trade monitor
pub async fn stop_trade_monitor() {
    let mut monitor = TRADE_MONITOR.write().await;
    monitor.stop();
}

/// Check if trade monitor is running
pub async fn is_trade_monitor_running() -> bool {
    let monitor = TRADE_MONITOR.read().await;
    monitor.is_running()
}

/// Get trade monitor status
pub async fn get_trade_monitor_status() -> TradeMonitorStatus {
    let monitor = TRADE_MONITOR.read().await;
    monitor.status()
}

/// Refresh own wallets list (call after adding/removing wallets)
pub async fn refresh_own_wallets() {
    let mut monitor = TRADE_MONITOR.write().await;
    if let Ok(wallets) = list_wallets(true).await {
        monitor.own_wallets = wallets.into_iter().map(|w| w.address).collect();
        logger::debug(
            LogTag::Tools,
            &format!(
                "[TRADE_WATCHER] Refreshed {} own wallets for filtering",
                monitor.own_wallets.len()
            ),
        );
    }
}

/// Clear all tracked signatures (for testing/reset)
pub async fn clear_tracked_signatures() {
    let mut monitor = TRADE_MONITOR.write().await;
    monitor.last_signatures.clear();
    logger::debug(
        LogTag::Tools,
        "[TRADE_WATCHER] Cleared all tracked signatures",
    );
}
