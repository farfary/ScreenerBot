use std::sync::{ Arc, Mutex };
use std::sync::atomic::{ AtomicBool, Ordering };
use once_cell::sync::Lazy;
use crate::configs::{ SharedConfig };
use crate::trader::TradingStats;
use crate::pools::PoolStats;

// Global shutdown signal
pub static SHUTDOWN: AtomicBool = AtomicBool::new(false);

// Global bot state
pub static BOT_STATE: Lazy<Arc<Mutex<BotState>>> = Lazy::new(|| {
    Arc::new(Mutex::new(BotState::new()))
});

#[derive(Debug)]
pub struct BotState {
    pub config: Option<SharedConfig>,
    pub rpc_stats: RpcStats,
    pub wallet_balance: f64,
    pub task_delays: TaskDelays,
    pub trading_stats: TradingStats,
    pub pool_stats: PoolStats,
}

#[derive(Debug, Clone)]
pub struct RpcStats {
    pub main_rpc_calls: u64,
    pub fallback_rpc_calls: u64,
    pub failed_calls: u64,
    pub total_calls: u64,
    pub rate_limited_calls: u64,
}

#[derive(Debug, Clone)]
pub struct TaskDelays {
    pub monitor_delay: u64,
    pub wallet_delay: u64,
    pub trader_delay: u64,
    pub pools_delay: u64,
    pub logger_delay: u64,
    pub rpc_delay: u64,
}

impl Default for RpcStats {
    fn default() -> Self {
        Self {
            main_rpc_calls: 0,
            fallback_rpc_calls: 0,
            failed_calls: 0,
            total_calls: 0,
            rate_limited_calls: 0,
        }
    }
}

impl Default for TaskDelays {
    fn default() -> Self {
        Self {
            monitor_delay: 5,
            wallet_delay: 10,
            trader_delay: 3,
            pools_delay: 15,
            logger_delay: 1,
            rpc_delay: 1,
        }
    }
}

impl BotState {
    pub fn new() -> Self {
        Self {
            config: None,
            rpc_stats: RpcStats::default(),
            wallet_balance: 0.0,
            task_delays: TaskDelays::default(),
            trading_stats: TradingStats::default(),
            pool_stats: PoolStats::default(),
        }
    }

    pub fn set_config(&mut self, config: SharedConfig) {
        self.config = Some(config);
    }

    pub fn get_config(&self) -> Option<SharedConfig> {
        self.config.clone()
    }

    pub fn update_rpc_stats<F>(&mut self, update_fn: F) where F: FnOnce(&mut RpcStats) {
        update_fn(&mut self.rpc_stats);
    }

    pub fn set_wallet_balance(&mut self, balance: f64) {
        self.wallet_balance = balance;
    }

    pub fn get_wallet_balance(&self) -> f64 {
        self.wallet_balance
    }
}

// Helper functions for global state access
pub fn is_shutdown() -> bool {
    SHUTDOWN.load(Ordering::Relaxed)
}

pub fn trigger_shutdown() {
    SHUTDOWN.store(true, Ordering::Relaxed);
}

pub fn get_bot_state() -> Arc<Mutex<BotState>> {
    BOT_STATE.clone()
}

pub fn update_config(config: SharedConfig) {
    if let Ok(mut state) = BOT_STATE.lock() {
        state.set_config(config);
    }
}

pub fn get_config() -> Option<SharedConfig> {
    BOT_STATE.lock()
        .ok()
        .and_then(|state| state.get_config())
}

pub fn update_wallet_balance(balance: f64) {
    if let Ok(mut state) = BOT_STATE.lock() {
        state.set_wallet_balance(balance);
    }
}

pub fn get_wallet_balance() -> f64 {
    BOT_STATE.lock()
        .map(|state| state.get_wallet_balance())
        .unwrap_or(0.0)
}

pub fn update_rpc_stats<F>(update_fn: F) where F: FnOnce(&mut RpcStats) {
    if let Ok(mut state) = BOT_STATE.lock() {
        state.update_rpc_stats(update_fn);
    }
}

pub fn get_rpc_stats() -> RpcStats {
    BOT_STATE.lock()
        .map(|state| state.rpc_stats.clone())
        .unwrap_or_default()
}

pub fn get_task_delays() -> TaskDelays {
    BOT_STATE.lock()
        .map(|state| state.task_delays.clone())
        .unwrap_or_default()
}

pub fn get_trading_stats() -> TradingStats {
    BOT_STATE.lock()
        .map(|state| state.trading_stats.clone())
        .unwrap_or_default()
}

pub fn update_trading_stats(trading_stats: TradingStats) {
    if let Ok(mut state) = BOT_STATE.lock() {
        state.trading_stats = trading_stats;
    }
}

pub fn get_pool_stats() -> PoolStats {
    BOT_STATE.lock()
        .map(|state| state.pool_stats.clone())
        .unwrap_or_default()
}

pub fn update_pool_stats(pool_stats: PoolStats) {
    if let Ok(mut state) = BOT_STATE.lock() {
        state.pool_stats = pool_stats;
    }
}
