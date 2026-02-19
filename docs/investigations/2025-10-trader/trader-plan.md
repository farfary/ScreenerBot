# Trader Module Reorganization & Strategy Integration

## Proposed Architecture for `src/trader/`

I recommend reorganizing the trader code into a modular structure that leverages the strategy system while providing clear separation of concerns. Here's a comprehensive design:

```
src/trader/
├── mod.rs                # Main module exports and initialization
├── service.rs            # TraderService implementation
├── types.rs              # Core trader types and enums
├── config.rs             # Trading configuration and constants
├── controller.rs         # Control API (start/stop/status)
├── auto/
│   ├── mod.rs            # Auto trading system entry point
│   ├── strategy_manager.rs # Strategy application and tracking
│   ├── entry_monitor.rs  # Entry opportunity monitoring
│   ├── exit_monitor.rs   # Position monitoring and exit decisions
│   └── dca.rs            # Dollar cost averaging implementation
├── manual/
│   ├── mod.rs            # Manual trading entry point
│   ├── orders.rs         # Manual order processing
│   └── tracking.rs       # Manual trade tracking/history
├── execution/
│   ├── mod.rs            # Trade execution pipeline
│   ├── buy.rs            # Buy operation workflow
│   ├── sell.rs           # Sell operation workflow
│   ├── retry.rs          # Retry mechanism for failed operations
│   └── decision_cache.rs # Decision tracking and management
├── safety/
│   ├── mod.rs            # Safety systems entry point
│   ├── blacklist.rs      # Blacklist integration
│   ├── limits.rs         # Position/trade limits enforcement
│   └── risk.rs           # Risk management utilities
└── exit/
    ├── mod.rs            # Exit strategy coordination
    ├── trailing_stop.rs  # Trailing stop loss implementation
    ├── roi.rs            # Return on investment exit
    └── time_override.rs  # Time-based exit rules
```

## Core Components Design

### 1. Trader Module (mod.rs)

```rust
//! Trader module - Core trading functionality orchestration
//!
//! The trader module is responsible for:
//! 1. Automated trading via strategies
//! 2. Manual trading operations
//! 3. Position management and exit strategies
//! 4. Trade execution and retry mechanisms

mod service;
mod types;
mod config;
mod controller;
mod auto;
mod manual;
mod execution;
mod safety;
mod exit;

// Re-exports for common usage
pub use controller::{start_trader, stop_trader_gracefully, is_trader_running, TraderControlError};
pub use types::{TradeDecision, TradeReason, TradeResult, TradeSeverity};
pub use service::TraderService;

// Public API
pub async fn init_trader_system() -> Result<(), String> {
    // Initialize trader subsystems
}

// Other public functions...
```

### 2. Trader Types (types.rs)

```rust
//! Core trader types and structures

use chrono::{DateTime, Utc};

/// Represents a decision to trade
#[derive(Debug, Clone)]
pub struct TradeDecision {
    pub position_id: Option<String>,
    pub mint: String,
    pub action: TradeAction,
    pub reason: TradeReason,
    pub strategy_id: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub priority: TradePriority,
    pub price_sol: Option<f64>,
    pub size_sol: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TradeAction {
    Buy,
    Sell,
    DCA,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TradeReason {
    // Entry reasons
    StrategySignal,
    ManualEntry,
    ForceBuy,
    DCAScheduled,

    // Exit reasons
    TakeProfit,
    StopLoss,
    TrailingStop,
    TimeOverride,
    StrategyExit,
    ManualExit,
    RiskManagement,
    Blacklisted,
    ForceSell,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum TradePriority {
    Emergency,  // Immediate execution (stop loss, blacklist)
    High,       // Next available execution slot
    Normal,     // Standard execution
    Low,        // Can be delayed if needed
}

#[derive(Debug, Clone)]
pub struct TradeResult {
    pub decision: TradeDecision,
    pub success: bool,
    pub tx_signature: Option<String>,
    pub executed_price_sol: Option<f64>,
    pub executed_size_sol: Option<f64>,
    pub error: Option<String>,
    pub position_id: Option<String>,
    pub execution_timestamp: DateTime<Utc>,
    pub retry_count: u32,
}

// Additional types...
```

### 3. Auto Trading System (mod.rs)

```rust
//! Automated trading system using strategies
//!
//! This module handles strategy-based trading including:
//! 1. Entry monitoring based on strategies
//! 2. Exit decisions based on strategies and exit rules
//! 3. DCA implementation

mod strategy_manager;
mod entry_monitor;
mod exit_monitor;
mod dca;

use crate::strategies;
use crate::trader::types::{TradeDecision, TradeAction, TradeReason, TradePriority};
use crate::logger::{log, LogTag};

pub use strategy_manager::StrategyManager;

/// Monitor for new entry opportunities using strategies
pub async fn monitor_new_entries(shutdown: tokio::sync::watch::Receiver<bool>) -> Result<(), String> {
    entry_monitor::monitor_entries(shutdown).await
}

/// Monitor open positions for exit opportunities
pub async fn monitor_open_positions(shutdown: tokio::sync::watch::Receiver<bool>) -> Result<(), String> {
    exit_monitor::monitor_positions(shutdown).await
}

/// Process DCA for eligible positions
pub async fn process_dca() -> Result<Vec<TradeDecision>, String> {
    dca::process_dca_opportunities().await
}

// Additional functions...
```

### 4. Strategy Manager (`auto/strategy_manager.rs`)

```rust
//! Strategy management and application for trading decisions

use crate::strategies::{self, types::{EvaluationContext, MarketData, PositionData, OhlcvData}};
use crate::pools::PriceResult;
use crate::positions::Position;
use crate::trader::types::{TradeDecision, TradeAction, TradeReason, TradePriority};

/// Manager for applying strategies to trading decisions
pub struct StrategyManager;

impl StrategyManager {
    /// Check if a token meets entry criteria based on strategies
    pub async fn check_entry_strategies(
        token_mint: &str,
        price_info: &PriceResult,
    ) -> Result<Option<TradeDecision>, String> {
        // Build market data from price info
        let market_data = MarketData {
            liquidity_sol: Some(price_info.sol_reserves),
            volume_24h: None,
            market_cap: None,
            holder_count: None,
            token_age_hours: None,
        };

        // Try to get OHLCV data if available
        let ohlcv_data = crate::ohlcvs::get_candles_for_mint(token_mint, "5m", 24)
            .await
            .ok()
            .map(|candles| {
                // Convert to strategy system's OhlcvData format
                // ...
            });

        // Evaluate entry strategies
        if let Some(strategy_id) = strategies::evaluate_entry_strategies(
            token_mint,
            price_info.price_sol,
            Some(market_data),
            ohlcv_data,
        ).await? {
            // Create trade decision
            return Ok(Some(TradeDecision {
                position_id: None,
                mint: token_mint.to_string(),
                action: TradeAction::Buy,
                reason: TradeReason::StrategySignal,
                strategy_id: Some(strategy_id),
                timestamp: chrono::Utc::now(),
                priority: TradePriority::Normal,
                price_sol: Some(price_info.price_sol),
                size_sol: None, // Will be filled from config later
            }));
        }

        Ok(None)
    }

    /// Check if a position should be exited based on strategies
    pub async fn check_exit_strategies(
        position: &Position,
        current_price: f64,
    ) -> Result<Option<TradeDecision>, String> {
        // Build position data for strategy evaluation
        let position_data = PositionData {
            entry_price: position.entry_price,
            entry_time: position.entry_time,
            current_size_sol: position.amount_sol,
            unrealized_profit_pct: position.unrealized_profit_pct,
            position_age_hours: (chrono::Utc::now() - position.entry_time).num_seconds() as f64 / 3600.0,
        };

        // Market data (get from token database if possible)
        let market_data = get_market_data_for_mint(&position.mint).await;

        // OHLCV data
        let ohlcv_data = crate::ohlcvs::get_candles_for_mint(&position.mint, "5m", 24)
            .await
            .ok()
            .map(|candles| {
                // Convert to strategy system's OhlcvData format
                // ...
            });

        // Evaluate exit strategies
        if let Some(strategy_id) = strategies::evaluate_exit_strategies(
            &position.mint,
            current_price,
            position_data,
            market_data,
            ohlcv_data,
        ).await? {
            return Ok(Some(TradeDecision {
                position_id: Some(position.id.to_string()),
                mint: position.mint.clone(),
                action: TradeAction::Sell,
                reason: TradeReason::StrategyExit,
                strategy_id: Some(strategy_id),
                timestamp: chrono::Utc::now(),
                priority: TradePriority::Normal,
                price_sol: Some(current_price),
                size_sol: None, // Will sell all
            }));
        }

        Ok(None)
    }

    // Additional methods...
}

// Helper functions...
```

### 5. Entry Monitor (`auto/entry_monitor.rs`)

```rust
//! Entry opportunity monitoring based on strategies

use crate::trader::types::{TradeDecision, TradeAction, TradeReason};
use crate::trader::execution::execute_trade;
use crate::trader::safety::is_blacklisted;
use crate::trader::auto::strategy_manager::StrategyManager;
use crate::config::with_config;
use crate::logger::{log, LogTag};
use crate::pools;
use tokio::time::{Duration, Instant};

/// Constants for entry monitoring
const ENTRY_MONITOR_INTERVAL_SECS: u64 = 3;
const ENTRY_CYCLE_MIN_WAIT_MS: u64 = 100;
const ENTRY_CHECK_ACQUIRE_TIMEOUT_SECS: u64 = 30;

/// Monitor for new entry opportunities
pub async fn monitor_entries(mut shutdown: tokio::sync::watch::Receiver<bool>) -> Result<(), String> {
    log(LogTag::Trader, "INFO", "Starting entry opportunity monitor");

    // Create semaphore for concurrent entry checks
    let entry_check_concurrency = with_config(|cfg| cfg.trader.entry_check_concurrency);
    let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(entry_check_concurrency));

    loop {
        // Check if we should shutdown
        if *shutdown.borrow() {
            log(LogTag::Trader, "INFO", "Entry monitor shutting down");
            break;
        }

        // Check if trader is enabled
        let trader_enabled = with_config(|cfg| cfg.trader.enabled);
        if !trader_enabled {
            log(LogTag::Trader, "INFO", "Entry monitor paused - trader disabled");
            tokio::time::sleep(Duration::from_secs(5)).await;
            continue;
        }

        // Start cycle timing
        let cycle_start = Instant::now();

        // Get available tokens from pools
        let available_tokens = pools::get_available_tokens().await;

        // Apply re-entry cooldown filter
        let filtered_tokens = apply_reentry_cooldown_filter(available_tokens).await?;

        // Process tokens with concurrency control
        let mut futures = Vec::new();

        for token in filtered_tokens {
            // Get latest price info
            if let Ok(price_info) = pools::get_token_price(&token).await {
                // Check if token is blacklisted
                if is_blacklisted(&token).await? {
                    continue;
                }

                // Acquire semaphore permit with timeout
                let sem_clone = semaphore.clone();
                let token_clone = token.clone();

                let future = tokio::spawn(async move {
                    let _permit = match tokio::time::timeout(
                        Duration::from_secs(ENTRY_CHECK_ACQUIRE_TIMEOUT_SECS),
                        sem_clone.acquire(),
                    ).await {
                        Ok(Ok(permit)) => permit,
                        Ok(Err(e)) => {
                            log(LogTag::Trader, "ERROR",
                                &format!("Failed to acquire semaphore for entry check: {}", e));
                            return None;
                        },
                        Err(_) => {
                            log(LogTag::Trader, "WARN",
                                &format!("Timeout waiting for entry check semaphore for {}", token_clone));
                            return None;
                        }
                    };

                    // Check entry strategies
                    match StrategyManager::check_entry_strategies(&token_clone, &price_info).await {
                        Ok(Some(decision)) => Some(decision),
                        Ok(None) => None,
                        Err(e) => {
                            log(LogTag::Trader, "ERROR",
                                &format!("Entry strategy check failed for {}: {}", token_clone, e));
                            None
                        }
                    }
                });

                futures.push(future);
            }
        }

        // Collect results and process trade decisions
        for future in futures {
            if let Ok(Some(decision)) = future.await {
                // Execute the trade
                match execute_trade(&decision).await {
                    Ok(result) => {
                        if result.success {
                            log(LogTag::Trader, "SUCCESS",
                                &format!("Entry executed for {}: tx={}",
                                    decision.mint, result.tx_signature.unwrap_or_default()));
                        } else {
                            log(LogTag::Trader, "ERROR",
                                &format!("Entry failed for {}: {}",
                                    decision.mint, result.error.unwrap_or_default()));
                        }
                    },
                    Err(e) => {
                        log(LogTag::Trader, "ERROR",
                            &format!("Failed to execute entry for {}: {}", decision.mint, e));
                    }
                }
            }
        }

        // Calculate wait time for next cycle
        let cycle_duration = cycle_start.elapsed();
        let wait_time = if cycle_duration >= Duration::from_secs(ENTRY_MONITOR_INTERVAL_SECS) {
            Duration::from_millis(ENTRY_CYCLE_MIN_WAIT_MS)
        } else {
            Duration::from_secs(ENTRY_MONITOR_INTERVAL_SECS) - cycle_duration
        };

        // Wait for next cycle or shutdown
        tokio::select! {
            _ = tokio::time::sleep(wait_time) => {},
            _ = shutdown.changed() => {
                if *shutdown.borrow() {
                    log(LogTag::Trader, "INFO", "Entry monitor shutting down");
                    break;
                }
            }
        }
    }

    Ok(())
}

// Helper functions...
```

### 6. Exit Monitor (`auto/exit_monitor.rs`)

```rust
//! Position monitoring and exit strategy application

use crate::trader::types::{TradeDecision, TradeAction, TradeReason, TradePriority};
use crate::trader::execution::execute_trade;
use crate::trader::auto::strategy_manager::StrategyManager;
use crate::trader::exit::{check_trailing_stop, check_roi_exit, check_time_override};
use crate::trader::safety::{check_blacklist_exit, check_risk_limits};
use crate::config::with_config;
use crate::logger::{log, LogTag};
use crate::positions;
use crate::pools;
use tokio::time::{Duration, Instant};

/// Constants for position monitoring
const POSITION_MONITOR_INTERVAL_SECS: u64 = 5;
const POSITION_CYCLE_MIN_WAIT_MS: u64 = 200;

/// Monitor open positions for exit opportunities
pub async fn monitor_positions(mut shutdown: tokio::sync::watch::Receiver<bool>) -> Result<(), String> {
    log(LogTag::Trader, "INFO", "Starting position monitor");

    loop {
        // Check if we should shutdown
        if *shutdown.borrow() {
            log(LogTag::Trader, "INFO", "Position monitor shutting down");
            break;
        }

        // Check if trader is enabled
        let trader_enabled = with_config(|cfg| cfg.trader.enabled);
        if !trader_enabled {
            log(LogTag::Trader, "INFO", "Position monitor paused - trader disabled");
            tokio::time::sleep(Duration::from_secs(5)).await;
            continue;
        }

        // Start cycle timing
        let cycle_start = Instant::now();

        // Get open positions
        let open_positions = match positions::db::get_open_positions() {
            Ok(positions) => positions,
            Err(e) => {
                log(LogTag::Trader, "ERROR", &format!("Failed to get open positions: {}", e));
                tokio::time::sleep(Duration::from_secs(5)).await;
                continue;
            }
        };

        // Check if we have any open positions
        if open_positions.is_empty() {
            tokio::time::sleep(Duration::from_secs(POSITION_MONITOR_INTERVAL_SECS)).await;
            continue;
        }

        // Process each position
        for position in &open_positions {
            // Skip positions with exit in progress
            if position.exit_in_progress {
                continue;
            }

            // Get current price
            let current_price = match pools::get_token_price(&position.mint).await {
                Ok(price_info) => price_info.price_sol,
                Err(e) => {
                    log(LogTag::Trader, "ERROR",
                        &format!("Failed to get price for {}: {}", position.mint, e));
                    continue;
                }
            };

            // Check exit conditions in priority order

            // 1. Check if token is blacklisted (highest priority)
            if let Some(exit_decision) = check_blacklist_exit(position, current_price).await? {
                execute_exit(exit_decision).await?;
                continue;
            }

            // 2. Check risk management limits
            if let Some(exit_decision) = check_risk_limits(position, current_price).await? {
                execute_exit(exit_decision).await?;
                continue;
            }

            // 3. Check strategy-based exit
            if let Some(exit_decision) = StrategyManager::check_exit_strategies(position, current_price).await? {
                execute_exit(exit_decision).await?;
                continue;
            }

            // 4. Check trailing stop
            if let Some(exit_decision) = check_trailing_stop(position, current_price).await? {
                execute_exit(exit_decision).await?;
                continue;
            }

            // 5. Check ROI exit
            if let Some(exit_decision) = check_roi_exit(position, current_price).await? {
                execute_exit(exit_decision).await?;
                continue;
            }

            // 6. Check time-based override
            if let Some(exit_decision) = check_time_override(position, current_price).await? {
                execute_exit(exit_decision).await?;
                continue;
            }
        }

        // Calculate wait time for next cycle
        let cycle_duration = cycle_start.elapsed();
        let wait_time = if cycle_duration >= Duration::from_secs(POSITION_MONITOR_INTERVAL_SECS) {
            Duration::from_millis(POSITION_CYCLE_MIN_WAIT_MS)
        } else {
            Duration::from_secs(POSITION_MONITOR_INTERVAL_SECS) - cycle_duration
        };

        // Wait for next cycle or shutdown
        tokio::select! {
            _ = tokio::time::sleep(wait_time) => {},
            _ = shutdown.changed() => {
                if *shutdown.borrow() {
                    log(LogTag::Trader, "INFO", "Position monitor shutting down");
                    break;
                }
            }
        }
    }

    Ok(())
}

/// Execute exit decision
async fn execute_exit(decision: TradeDecision) -> Result<(), String> {
    match execute_trade(&decision).await {
        Ok(result) => {
            if result.success {
                log(LogTag::Trader, "SUCCESS",
                    &format!("Exit executed for {}: reason={:?}, tx={}",
                        decision.mint, decision.reason, result.tx_signature.unwrap_or_default()));
            } else {
                log(LogTag::Trader, "ERROR",
                    &format!("Exit failed for {}: reason={:?}, error={}",
                        decision.mint, decision.reason, result.error.unwrap_or_default()));
            }
            Ok(())
        },
        Err(e) => {
            log(LogTag::Trader, "ERROR",
                &format!("Failed to execute exit for {}: {}", decision.mint, e));
            Err(e)
        }
    }
}
```

### 7. Execution System (mod.rs)

```rust
//! Trade execution system

mod buy;
mod sell;
mod retry;
mod decision_cache;

use crate::trader::types::{TradeDecision, TradeResult, TradeAction};

/// Execute a trade decision
pub async fn execute_trade(decision: &TradeDecision) -> Result<TradeResult, String> {
    match decision.action {
        TradeAction::Buy => buy::execute_buy(decision).await,
        TradeAction::Sell => sell::execute_sell(decision).await,
        TradeAction::DCA => buy::execute_dca(decision).await,
    }
}

/// Retry a failed trade
pub async fn retry_trade(result: &TradeResult) -> Result<TradeResult, String> {
    retry::retry_trade(result).await
}

/// Cache a sell decision for retry
pub fn cache_sell_decision(decision: &TradeDecision) -> Result<(), String> {
    decision_cache::cache_decision(decision)
}

/// Get pending sell decisions ready for retry
pub fn get_pending_sell_decisions() -> Vec<TradeDecision> {
    decision_cache::get_retry_ready_decisions()
}

/// Mark a sell decision as complete
pub fn mark_sell_complete(position_id: &str) -> bool {
    decision_cache::remove_decision(position_id)
}

// Additional functions...
```

### 8. Exit Strategies (`exit/trailing_stop.rs`)

```rust
//! Trailing stop loss implementation

use crate::positions::Position;
use crate::trader::types::{TradeDecision, TradeAction, TradeReason, TradePriority};
use crate::config::with_config;
use chrono::Utc;

/// Check if a position should be exited based on trailing stop
pub async fn check_trailing_stop(
    position: &Position,
    current_price: f64
) -> Result<Option<TradeDecision>, String> {
    // Skip if position doesn't have highest price recorded
    if position.price_highest <= 0.0 {
        return Ok(None);
    }

    // Get trailing stop configuration
    let trailing_enabled = with_config(|cfg| cfg.positions.trailing_stop_enabled);
    if !trailing_enabled {
        return Ok(None);
    }

    // Get trailing percentages
    let activation_pct = with_config(|cfg| cfg.positions.trailing_stop_activation_pct);
    let trailing_pct = with_config(|cfg| cfg.positions.trailing_stop_pct);

    // Calculate unrealized profit percentage
    let profit_pct = (current_price / position.entry_price - 1.0) * 100.0;

    // Check if profit exceeds activation threshold
    if profit_pct >= activation_pct {
        // Calculate stop price based on highest recorded price
        let stop_price = position.price_highest * (1.0 - trailing_pct / 100.0);

        // Check if current price fell below stop price
        if current_price <= stop_price {
            return Ok(Some(TradeDecision {
                position_id: Some(position.id.to_string()),
                mint: position.mint.clone(),
                action: TradeAction::Sell,
                reason: TradeReason::TrailingStop,
                strategy_id: None,
                timestamp: Utc::now(),
                priority: TradePriority::High, // High priority for trailing stops
                price_sol: Some(current_price),
                size_sol: None, // Will sell entire position
            }));
        }
    }

    Ok(None)
}
```

### 9. ROI Exit (`exit/roi.rs`)

```rust
//! Return on Investment (ROI) based exit strategy

use crate::positions::Position;
use crate::trader::types::{TradeDecision, TradeAction, TradeReason, TradePriority};
use crate::config::with_config;
use chrono::Utc;

/// Check if a position should be exited based on ROI target
pub async fn check_roi_exit(
    position: &Position,
    current_price: f64
) -> Result<Option<TradeDecision>, String> {
    // Check if ROI-based exit is enabled
    let roi_enabled = with_config(|cfg| cfg.trader.min_profit_threshold_enabled);
    if !roi_enabled {
        return Ok(None);
    }

    // Get target ROI percentage
    let target_profit_pct = with_config(|cfg| cfg.trader.min_profit_threshold_percent);

    // Get position's unrealized profit percentage
    let profit_pct = match position.unrealized_profit_pct {
        Some(pct) => pct,
        None => (current_price / position.entry_price - 1.0) * 100.0,
    };

    // Check if profit exceeds target
    if profit_pct >= target_profit_pct {
        return Ok(Some(TradeDecision {
            position_id: Some(position.id.to_string()),
            mint: position.mint.clone(),
            action: TradeAction::Sell,
            reason: TradeReason::TakeProfit,
            strategy_id: None,
            timestamp: Utc::now(),
            priority: TradePriority::Normal,
            price_sol: Some(current_price),
            size_sol: None, // Will sell entire position
        }));
    }

    Ok(None)
}
```

### 10. DCA Implementation (`auto/dca.rs`)

```rust
//! Dollar Cost Averaging implementation

use crate::trader::types::{TradeDecision, TradeAction, TradeReason, TradePriority};
use crate::config::with_config;
use crate::positions;
use chrono::Utc;

/// Process DCA opportunities for eligible positions
pub async fn process_dca_opportunities() -> Result<Vec<TradeDecision>, String> {
    // Check if DCA is enabled
    let dca_enabled = with_config(|cfg| cfg.trader.dca_enabled);
    if !dca_enabled {
        return Ok(Vec::new());
    }

    // Get open positions
    let open_positions = positions::db::get_open_positions()?;
    let mut decisions = Vec::new();

    for position in &open_positions {
        // Check if position is eligible for DCA
        if let Some(decision) = check_position_for_dca(position).await? {
            decisions.push(decision);
        }
    }

    Ok(decisions)
}

/// Check if a position is eligible for DCA
async fn check_position_for_dca(position: &positions::Position) -> Result<Option<TradeDecision>, String> {
    // Get DCA configuration
    let dca_threshold_pct = with_config(|cfg| cfg.trader.dca_threshold_pct);
    let dca_max_count = with_config(|cfg| cfg.trader.dca_max_count);

    // Skip if position already has max DCA count
    if position.dca_count >= dca_max_count {
        return Ok(None);
    }

    // Get current price
    let current_price = match crate::pools::get_token_price(&position.mint).await {
        Ok(price_info) => price_info.price_sol,
        Err(_) => return Ok(None), // Skip if price can't be retrieved
    };

    // Calculate unrealized loss percentage
    let loss_pct = (position.entry_price / current_price - 1.0) * 100.0;

    // Check if loss exceeds DCA threshold
    if loss_pct >= dca_threshold_pct {
        // Calculate DCA size based on initial position size
        let dca_size_sol = with_config(|cfg| cfg.trader.dca_size_percentage) * position.entry_size_sol / 100.0;

        return Ok(Some(TradeDecision {
            position_id: Some(position.id.to_string()),
            mint: position.mint.clone(),
            action: TradeAction::DCA,
            reason: TradeReason::DCAScheduled,
            strategy_id: None,
            timestamp: Utc::now(),
            priority: TradePriority::Low, // Lower priority than new entries
            price_sol: Some(current_price),
            size_sol: Some(dca_size_sol),
        }));
    }

    Ok(None)
}
```

### 11. Controller (`controller.rs`)

```rust
//! Trader controller for starting/stopping trading

use crate::config::{with_config, Config, ConfigUpdate};
use crate::logger::{log, LogTag};
use crate::global::CRITICAL_OPERATIONS_IN_PROGRESS;
use std::time::Duration;
use tokio::time::timeout;

/// Trader control error types
#[derive(Debug)]
pub enum TraderControlError {
    AlreadyRunning,
    AlreadyStopped,
    ConfigUpdate(String),
}

impl std::fmt::Display for TraderControlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TraderControlError::AlreadyRunning => write!(f, "Trader is already running"),
            TraderControlError::AlreadyStopped => write!(f, "Trader is already stopped"),
            TraderControlError::ConfigUpdate(err) => write!(f, "Config update failed: {}", err),
        }
    }
}

impl std::error::Error for TraderControlError {}

/// Check if the trader is currently running
pub fn is_trader_running() -> bool {
    with_config(|cfg| cfg.trader.enabled)
}

/// Start the trader by enabling trader operations
pub async fn start_trader() -> Result<(), TraderControlError> {
    if with_config(|cfg| cfg.trader.enabled) {
        return Err(TraderControlError::AlreadyRunning);
    }

    log(LogTag::Trader, "INFO", "Enabling trader operations...");

    // Update config to enable trader
    ConfigUpdate::new()
        .with_change(|cfg| {
            cfg.trader.enabled = true;
        })
        .apply()
        .map_err(TraderControlError::ConfigUpdate)?;

    log(LogTag::Trader, "INFO", "Trader operations enabled");
    Ok(())
}

/// Stop the trader gracefully by signaling shutdown and waiting for tasks to complete
pub async fn stop_trader_gracefully() -> Result<(), TraderControlError> {
    if !with_config(|cfg| cfg.trader.enabled) {
        return Err(TraderControlError::AlreadyStopped);
    }

    log(LogTag::Trader, "INFO", "Disabling trader operations...");

    // Update config to disable trader
    ConfigUpdate::new()
        .with_change(|cfg| {
            cfg.trader.enabled = false;
        })
        .apply()
        .map_err(TraderControlError::ConfigUpdate)?;

    // Wait for critical operations to complete
    let wait_result = timeout(
        Duration::from_secs(30),
        wait_for_critical_operations()
    ).await;

    if wait_result.is_err() {
        log(
            LogTag::Trader,
            "WARN",
            "Timeout waiting for critical operations to complete during trader stop",
        );
    }

    log(LogTag::Trader, "INFO", "Trader operations disabled");
    Ok(())
}

/// Wait for critical operations to complete
async fn wait_for_critical_operations() {
    while CRITICAL_OPERATIONS_IN_PROGRESS.load(std::sync::atomic::Ordering::Acquire) > 0 {
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}
```

## Configuration Example

Add to `config.toml`:

```toml
[trader]
enabled = false
max_open_positions = 10
trade_size_sol = 0.5
entry_check_concurrency = 5
position_close_cooldown_minutes = 30
min_profit_threshold_enabled = true
min_profit_threshold_percent = 15.0
time_override_loss_threshold_percent = 50.0
time_override_duration_hours = 24.0

# New DCA settings
dca_enabled = true
dca_threshold_pct = 20.0
dca_max_count = 3
dca_size_percentage = 50.0

# Trailing stop settings
trailing_stop_enabled = true
trailing_stop_activation_pct = 15.0
trailing_stop_pct = 5.0
```

## Integration Points

The key points of integration are:

1. **ServiceManager**: Register `TraderService` with the service manager in run.rs
2. **Strategies**: Interface with strategy system for entry/exit decisions
3. **Positions**: Track and update positions throughout their lifecycle
4. **Events**: Record trade events for dashboard monitoring
5. **Pools**: Get price and liquidity data for trading decisions
6. **Blacklist**: Check before entry and verify during position monitoring

## Key Features

This architecture supports:

1. **Strategy-based Trading**: Leverage the strategy system for entry/exit decisions
2. **Manual Trading**: Clean API for manual buy/sell operations
3. **DCA**: Automatic averaging down on positions meeting criteria
4. **Multiple Exit Types**: Strategy-based, trailing stop, ROI, time-based
5. **Blacklist Integration**: Prevent trading blacklisted tokens
6. **Position Tracking**: Complete lifecycle management
7. **Reason Tracking**: Fully documented reasons for all trade actions
8. **Retry System**: Resilient execution with retries for transient failures

## Implementation Steps

1. Create the folder structure and base files
2. Implement the core types and interfaces
3. Port existing functionality from trader.rs to the new structure
4. Integrate strategy system for decision-making
5. Implement trailing stop and ROI exit strategies
6. Add DCA functionality
7. Update the service implementation
8. Build controller API for webserver integration
9. Implement safety systems

This architecture is modular, maintainable, and follows a clean separation of concerns. It can be extended with new features while maintaining performance and reliability.
