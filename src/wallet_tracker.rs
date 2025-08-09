/// Wallet Balance Tracker
///
/// This module provides comprehensive wallet tracking with historical data:
/// - SOL/WSOL balances tracking
/// - Token holdings valuation using pool prices
/// - ATA rent calculations and tracking
/// - Historical wallet value analysis
/// - 1-year data retention with periodic cleanup

use crate::logger::{log, LogTag};
use crate::global::{is_debug_wallet_tracker_enabled, DATA_DIR};
use crate::utils::{get_sol_balance, get_wallet_address};
use crate::tokens::pool::{get_pool_service};
use crate::rpc::{get_rpc_client, lamports_to_sol};
use std::collections::HashMap;
use std::str::FromStr;
use std::fs;
use std::path::Path;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc, Duration};
use tokio::time::{sleep, Duration as TokioDuration};
use std::sync::Arc;

// =============================================================================
// CONSTANTS
// =============================================================================

/// Wallet tracking interval (every 30 seconds after successful swaps)
const WALLET_TRACKING_INTERVAL_SECONDS: u64 = 30;

/// Historical data retention period (1 year)
const HISTORY_RETENTION_DAYS: i64 = 365;

/// Cleanup interval (daily cleanup of old history)
const CLEANUP_INTERVAL_HOURS: i64 = 24;

/// ATA rent cost (standard rent for token accounts)
const ATA_RENT_LAMPORTS: u64 = 2039280; // ~0.00203928 SOL

/// Wallet history file path
const WALLET_HISTORY_FILE: &str = "data/wallet_history.json";

// =============================================================================
// DATA STRUCTURES
// =============================================================================

/// Token holding information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenHolding {
    pub mint: String,
    pub symbol: Option<String>,
    pub balance: u64, // Raw token balance
    pub decimals: u8,
    pub balance_ui: f64, // UI balance (raw / 10^decimals)
    pub ata_address: String,
    pub price_sol: Option<f64>, // Price per token in SOL
    pub value_sol: f64, // Total value in SOL
    pub ata_rent_sol: f64, // ATA rent cost in SOL
}

/// Wallet snapshot at a specific time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletSnapshot {
    pub timestamp: DateTime<Utc>,
    pub sol_balance: f64,
    pub wsol_balance: f64,
    pub token_holdings: Vec<TokenHolding>,
    pub total_tokens_value_sol: f64,
    pub total_ata_rent_sol: f64,
    pub total_wallet_value_sol: f64, // SOL + WSOL + tokens + ATA rent
    pub token_count: usize,
    pub ata_count: usize,
}

/// Wallet value change analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletAnalysis {
    pub current_value: f64,
    pub start_value: f64,
    pub value_change: f64,
    pub value_change_percent: f64,
    pub period_days: i64,
    pub best_day_value: f64,
    pub worst_day_value: f64,
    pub avg_daily_change: f64,
}

/// Complete wallet history data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletHistory {
    pub wallet_address: String,
    pub created_at: DateTime<Utc>,
    pub last_updated: DateTime<Utc>,
    pub snapshots: Vec<WalletSnapshot>,
    pub start_value: f64, // Initial wallet value for comparison
}

impl WalletHistory {
    pub fn new(wallet_address: String) -> Self {
        Self {
            wallet_address,
            created_at: Utc::now(),
            last_updated: Utc::now(),
            snapshots: Vec::new(),
            start_value: 0.0,
        }
    }

    /// Add new snapshot and clean up old data
    pub fn add_snapshot(&mut self, snapshot: WalletSnapshot) {
        // Set start value from first snapshot
        if self.snapshots.is_empty() {
            self.start_value = snapshot.total_wallet_value_sol;
        }

        self.snapshots.push(snapshot);
        self.last_updated = Utc::now();

        // Clean up old snapshots (keep only last year)
        let cutoff_date = Utc::now() - Duration::days(HISTORY_RETENTION_DAYS);
        self.snapshots.retain(|s| s.timestamp > cutoff_date);

        if is_debug_wallet_tracker_enabled() {
            log(
                LogTag::Wallet,
                "HISTORY_UPDATE",
                &format!("Added snapshot, total: {} (cleaned old data)", self.snapshots.len())
            );
        }
    }

    /// Get value analysis for the wallet
    pub fn get_analysis(&self) -> Option<WalletAnalysis> {
        if self.snapshots.is_empty() {
            return None;
        }

        let current = self.snapshots.last()?;
        let current_value = current.total_wallet_value_sol;
        let start_value = self.start_value;
        
        let value_change = current_value - start_value;
        let value_change_percent = if start_value != 0.0 {
            (value_change / start_value) * 100.0
        } else {
            0.0
        };

        let period_days = if let Some(first) = self.snapshots.first() {
            (current.timestamp - first.timestamp).num_days()
        } else {
            0
        };

        // Find best and worst days
        let values: Vec<f64> = self.snapshots.iter().map(|s| s.total_wallet_value_sol).collect();
        let best_day_value = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let worst_day_value = values.iter().cloned().fold(f64::INFINITY, f64::min);

        let avg_daily_change = if period_days > 0 {
            value_change / period_days as f64
        } else {
            0.0
        };

        Some(WalletAnalysis {
            current_value,
            start_value,
            value_change,
            value_change_percent,
            period_days,
            best_day_value,
            worst_day_value,
            avg_daily_change,
        })
    }

    /// Load from file
    pub fn load_from_file() -> Result<Self, String> {
        if !Path::new(WALLET_HISTORY_FILE).exists() {
            return Err("Wallet history file does not exist".to_string());
        }

        let content = fs::read_to_string(WALLET_HISTORY_FILE)
            .map_err(|e| format!("Failed to read wallet history file: {}", e))?;

        let history: WalletHistory = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse wallet history: {}", e))?;

        Ok(history)
    }

    /// Save to file
    pub fn save_to_file(&self) -> Result<(), String> {
        // Ensure data directory exists
        if let Some(parent) = Path::new(WALLET_HISTORY_FILE).parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create data directory: {}", e))?;
        }

        let content = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize wallet history: {}", e))?;

        fs::write(WALLET_HISTORY_FILE, content)
            .map_err(|e| format!("Failed to write wallet history file: {}", e))?;

        Ok(())
    }
}

// =============================================================================
// WALLET TRACKER SERVICE
// =============================================================================

pub struct WalletTracker {
    wallet_address: String,
    history: WalletHistory,
    last_cleanup: DateTime<Utc>,
    tracking_active: bool,
}

impl WalletTracker {
    /// Create new wallet tracker
    pub fn new() -> Result<Self, String> {
        let wallet_address = get_wallet_address()
            .map_err(|e| format!("Failed to get wallet address: {}", e))?;

        // Try to load existing history
        let history = match WalletHistory::load_from_file() {
            Ok(mut existing_history) => {
                // Verify wallet address matches
                if existing_history.wallet_address != wallet_address {
                    log(
                        LogTag::Wallet,
                        "WARNING",
                        &format!(
                            "Wallet address changed from {} to {}, starting fresh",
                            existing_history.wallet_address, wallet_address
                        )
                    );
                    WalletHistory::new(wallet_address.clone())
                } else {
                    log(
                        LogTag::Wallet,
                        "LOADED",
                        &format!("Loaded wallet history with {} snapshots", existing_history.snapshots.len())
                    );
                    existing_history
                }
            }
            Err(_) => {
                log(LogTag::Wallet, "NEW", "Creating new wallet history");
                WalletHistory::new(wallet_address.clone())
            }
        };

        Ok(Self {
            wallet_address,
            history,
            last_cleanup: Utc::now(),
            tracking_active: false,
        })
    }

    /// Start wallet tracking service
    pub async fn start_tracking(&mut self, shutdown: Arc<tokio::sync::Notify>) {
        self.tracking_active = true;
        log(LogTag::Wallet, "START", "Wallet tracker started");

        // Take initial snapshot
        if let Err(e) = self.take_snapshot().await {
            log(LogTag::Wallet, "ERROR", &format!("Initial snapshot failed: {}", e));
        }

        loop {
            tokio::select! {
                _ = shutdown.notified() => {
                    log(LogTag::Wallet, "SHUTDOWN", "Wallet tracker stopping");
                    break;
                }
                
                _ = sleep(TokioDuration::from_secs(WALLET_TRACKING_INTERVAL_SECONDS)) => {
                    if let Err(e) = self.take_snapshot().await {
                        log(LogTag::Wallet, "ERROR", &format!("Snapshot failed: {}", e));
                    }

                    // Periodic cleanup
                    if (Utc::now() - self.last_cleanup).num_hours() >= CLEANUP_INTERVAL_HOURS {
                        self.cleanup_old_data().await;
                        self.last_cleanup = Utc::now();
                    }
                }
            }
        }

        self.tracking_active = false;
        log(LogTag::Wallet, "STOP", "Wallet tracker stopped");
    }

    /// Take a wallet snapshot
    pub async fn take_snapshot(&mut self) -> Result<(), String> {
        if is_debug_wallet_tracker_enabled() {
            log(LogTag::Wallet, "SNAPSHOT_START", "Taking wallet snapshot...");
        }

        // Get SOL balance
        let sol_balance = get_sol_balance(&self.wallet_address).await
            .map_err(|e| format!("Failed to get SOL balance: {}", e))?;

        // Get all token accounts
        let token_holdings = self.get_token_holdings().await?;

        // Calculate WSOL balance (if any)
        let wsol_balance = token_holdings.iter()
            .find(|h| h.mint == "So11111111111111111111111111111111111111112")
            .map(|h| h.balance_ui)
            .unwrap_or(0.0);

        // Calculate totals
        let total_tokens_value_sol: f64 = token_holdings.iter()
            .map(|h| h.value_sol)
            .sum();

        let total_ata_rent_sol: f64 = token_holdings.iter()
            .map(|h| h.ata_rent_sol)
            .sum();

        let total_wallet_value_sol = sol_balance + wsol_balance + total_tokens_value_sol + total_ata_rent_sol;

        // Create snapshot
        let snapshot = WalletSnapshot {
            timestamp: Utc::now(),
            sol_balance,
            wsol_balance,
            token_count: token_holdings.len(),
            ata_count: token_holdings.len(), // Each token holding has an ATA
            token_holdings,
            total_tokens_value_sol,
            total_ata_rent_sol,
            total_wallet_value_sol,
        };

        if is_debug_wallet_tracker_enabled() {
            log(
                LogTag::Wallet,
                "SNAPSHOT_COMPLETE",
                &format!(
                    "Wallet snapshot: SOL={:.6}, Tokens={:.6} SOL, ATA_Rent={:.6} SOL, Total={:.6} SOL ({} tokens)",
                    sol_balance,
                    total_tokens_value_sol,
                    total_ata_rent_sol,
                    total_wallet_value_sol,
                    snapshot.token_count
                )
            );
        }

        // Add to history and save
        self.history.add_snapshot(snapshot);
        self.history.save_to_file()?;

        log(
            LogTag::Wallet,
            "UPDATED",
            &format!("Wallet value: {:.6} SOL ({} tokens)", total_wallet_value_sol, self.history.snapshots.last().unwrap().token_count)
        );

        Ok(())
    }

    /// Get all token holdings with values
    async fn get_token_holdings(&self) -> Result<Vec<TokenHolding>, String> {
        let rpc_client = get_rpc_client();
        
        // Get all token accounts for this wallet
        let token_accounts = rpc_client
            .get_all_token_accounts(&self.wallet_address).await
            .map_err(|e| format!("Failed to get token accounts: {}", e))?;

        let mut holdings = Vec::new();
        let pool_service = get_pool_service();

        for account_info in token_accounts {
            // Skip if balance is zero
            if account_info.balance == 0 {
                continue;
            }

            let mint = account_info.mint;
            let balance = account_info.balance;
            let ata_address = account_info.account;

            // Get token decimals
            let decimals = self.get_token_decimals(&mint).await.unwrap_or(9);
            let balance_ui = balance as f64 / 10_f64.powi(decimals as i32);

            // Get price from pool service only
            let price_sol = if pool_service.check_token_availability(&mint).await {
                match pool_service.get_pool_price(&mint, None).await {
                    Some(result) => result.price_sol,
                    None => None,
                }
            } else {
                None
            };

            let value_sol = price_sol.map(|p| p * balance_ui).unwrap_or(0.0);
            let ata_rent_sol = lamports_to_sol(ATA_RENT_LAMPORTS);

            let holding = TokenHolding {
                mint: mint.clone(),
                symbol: None, // We'll get this from token database if needed
                balance,
                decimals,
                balance_ui,
                ata_address,
                price_sol,
                value_sol,
                ata_rent_sol,
            };

            holdings.push(holding);

            if is_debug_wallet_tracker_enabled() {
                log(
                    LogTag::Wallet,
                    "TOKEN_HOLDING",
                    &format!(
                        "Token {}: {:.6} tokens, price={:.10} SOL, value={:.6} SOL",
                        &mint[..8],
                        balance_ui,
                        price_sol.unwrap_or(0.0),
                        value_sol
                    )
                );
            }
        }

        // Sort by value (highest first)
        holdings.sort_by(|a, b| b.value_sol.partial_cmp(&a.value_sol).unwrap_or(std::cmp::Ordering::Equal));

        Ok(holdings)
    }

    /// Get token decimals (simple implementation)
    async fn get_token_decimals(&self, mint: &str) -> Option<u8> {
        // Use decimals service if available
        match crate::tokens::get_token_decimals(mint).await {
            Some(decimals) => Some(decimals),
            None => {
                // Fallback to 9 for SOL, 6 for most tokens
                if mint == "So11111111111111111111111111111111111111112" {
                    Some(9)
                } else {
                    Some(6)
                }
            }
        }
    }

    /// Clean up old data
    async fn cleanup_old_data(&mut self) {
        let before_count = self.history.snapshots.len();
        let cutoff_date = Utc::now() - Duration::days(HISTORY_RETENTION_DAYS);
        
        self.history.snapshots.retain(|s| s.timestamp > cutoff_date);
        
        let after_count = self.history.snapshots.len();
        let removed = before_count - after_count;

        if removed > 0 {
            log(
                LogTag::Wallet,
                "CLEANUP",
                &format!("Cleaned up {} old snapshots, {} remaining", removed, after_count)
            );

            // Save after cleanup
            if let Err(e) = self.history.save_to_file() {
                log(LogTag::Wallet, "ERROR", &format!("Failed to save after cleanup: {}", e));
            }
        }
    }

    /// Force snapshot update (called after successful swaps)
    pub async fn update_after_swap(&mut self) -> Result<(), String> {
        log(LogTag::Wallet, "SWAP_UPDATE", "Updating wallet snapshot after successful swap");
        self.take_snapshot().await
    }

    /// Get current wallet analysis
    pub fn get_analysis(&self) -> Option<WalletAnalysis> {
        self.history.get_analysis()
    }

    /// Get summary for logging
    pub fn get_summary(&self) -> String {
        if let Some(analysis) = self.get_analysis() {
            format!(
                "Wallet: {:.6} SOL ({}{}% from start, {} days tracked)",
                analysis.current_value,
                if analysis.value_change >= 0.0 { "+" } else { "" },
                analysis.value_change_percent,
                analysis.period_days
            )
        } else {
            "Wallet: No data available".to_string()
        }
    }
}

// =============================================================================
// GLOBAL WALLET TRACKER
// =============================================================================

use tokio::sync::Mutex;
use once_cell::sync::Lazy;

static GLOBAL_WALLET_TRACKER: Lazy<Arc<Mutex<Option<WalletTracker>>>> = 
    Lazy::new(|| Arc::new(Mutex::new(None)));

/// Initialize global wallet tracker
pub async fn init_wallet_tracker() -> Result<(), String> {
    let tracker = WalletTracker::new()?;
    let mut global_tracker = GLOBAL_WALLET_TRACKER.lock().await;
    *global_tracker = Some(tracker);
    
    log(LogTag::Wallet, "INIT", "Wallet tracker initialized");
    Ok(())
}

/// Start wallet tracking service
/// Start wallet tracking loop without holding the global lock continuously
async fn start_wallet_tracking_loop(shutdown: Arc<tokio::sync::Notify>) {
    log(LogTag::Wallet, "START", "Wallet tracker started");

    // Take initial snapshot
    {
        let mut tracker_guard = GLOBAL_WALLET_TRACKER.lock().await;
        if let Some(ref mut tracker) = *tracker_guard {
            if let Err(e) = tracker.take_snapshot().await {
                log(LogTag::Wallet, "ERROR", &format!("Initial snapshot failed: {}", e));
            }
            tracker.tracking_active = true;
        } else {
            log(LogTag::Wallet, "ERROR", "Wallet tracker not initialized");
            return;
        }
    } // Lock released here

    loop {
        tokio::select! {
            _ = shutdown.notified() => {
                log(LogTag::Wallet, "SHUTDOWN", "Wallet tracker stopping");
                break;
            }
            
            _ = sleep(TokioDuration::from_secs(WALLET_TRACKING_INTERVAL_SECONDS)) => {
                // Acquire lock only for the duration of the snapshot
                let mut should_cleanup = false;
                {
                    let mut tracker_guard = GLOBAL_WALLET_TRACKER.lock().await;
                    if let Some(ref mut tracker) = *tracker_guard {
                        if let Err(e) = tracker.take_snapshot().await {
                            log(LogTag::Wallet, "ERROR", &format!("Snapshot failed: {}", e));
                        }

                        // Check if cleanup is needed
                        if (Utc::now() - tracker.last_cleanup).num_hours() >= CLEANUP_INTERVAL_HOURS {
                            should_cleanup = true;
                        }
                    }
                } // Lock released here

                // Perform cleanup if needed (acquire lock briefly again)
                if should_cleanup {
                    let mut tracker_guard = GLOBAL_WALLET_TRACKER.lock().await;
                    if let Some(ref mut tracker) = *tracker_guard {
                        tracker.cleanup_old_data().await;
                        tracker.last_cleanup = Utc::now();
                    }
                } // Lock released here
            }
        }
    }

    // Mark as stopped
    {
        let mut tracker_guard = GLOBAL_WALLET_TRACKER.lock().await;
        if let Some(ref mut tracker) = *tracker_guard {
            tracker.tracking_active = false;
        }
    }
    log(LogTag::Wallet, "STOP", "Wallet tracker stopped");
}

pub async fn start_wallet_tracking(shutdown: Arc<tokio::sync::Notify>) -> Result<tokio::task::JoinHandle<()>, String> {
    log(LogTag::Wallet, "START", "Starting wallet tracking service");

    let handle = tokio::spawn(async move {
        // Instead of holding the lock for the entire tracking duration,
        // we'll periodically acquire and release it for each operation
        start_wallet_tracking_loop(shutdown).await;
    });

    Ok(handle)
}

/// Update wallet after successful swap
pub async fn update_wallet_after_swap() {
    let mut tracker_guard = GLOBAL_WALLET_TRACKER.lock().await;
    if let Some(ref mut tracker) = *tracker_guard {
        if let Err(e) = tracker.update_after_swap().await {
            log(LogTag::Wallet, "ERROR", &format!("Failed to update wallet after swap: {}", e));
        }
    }
}

/// Get wallet summary for display
pub async fn get_wallet_summary() -> String {
    let tracker_guard = GLOBAL_WALLET_TRACKER.lock().await;
    if let Some(ref tracker) = *tracker_guard {
        tracker.get_summary()
    } else {
        "Wallet tracker not initialized".to_string()
    }
}

/// Get detailed wallet analysis
pub async fn get_wallet_analysis() -> Option<WalletAnalysis> {
    let tracker_guard = GLOBAL_WALLET_TRACKER.lock().await;
    if let Some(ref tracker) = *tracker_guard {
        tracker.get_analysis()
    } else {
        None
    }
}
