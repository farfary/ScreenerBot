use std::time::Duration;
use crate::global::{ is_shutdown, get_bot_state, get_rpc_stats, get_wallet_balance };
use crate::logger::{ log, LogLevel, get_recent_logs };

pub fn start_monitoring() {
    tokio::task::spawn(async move {
        log("MONITOR", LogLevel::Info, "Monitor Manager initialized successfully");

        let delays = crate::global::get_task_delays();

        loop {
            if is_shutdown() {
                log("MONITOR", LogLevel::Info, "Monitor Manager shutting down...");
                break;
            }

            // Monitor system health
            monitor_system_health().await;

            // Monitor trading activities
            monitor_trading_activities().await;

            // Monitor pool activities
            monitor_pool_activities().await;

            // Monitor logs for errors
            monitor_error_logs().await;

            tokio::time::sleep(Duration::from_secs(delays.monitor_delay)).await;
        }
    });
}

async fn monitor_system_health() {
    let bot_state = get_bot_state();

    let (config_loaded, balance, rpc_stats) = {
        if let Ok(state) = bot_state.lock() {
            (state.get_config().is_some(), state.get_wallet_balance(), state.rpc_stats.clone())
        } else {
            return;
        }
    }; // MutexGuard is dropped here

    // Check if config is loaded
    if !config_loaded {
        log("MONITOR", LogLevel::Warn, "Configuration not loaded");
    }

    // Check wallet balance
    if balance < 0.01 {
        log("MONITOR", LogLevel::Warn, &format!("Low wallet balance: {:.6} SOL", balance));
    }

    // Check RPC health
    if rpc_stats.total_calls > 0 {
        let success_rate =
            (((rpc_stats.total_calls - rpc_stats.failed_calls) as f64) /
                (rpc_stats.total_calls as f64)) *
            100.0;

        if success_rate < 90.0 {
            log("MONITOR", LogLevel::Warn, &format!("Low RPC success rate: {:.1}%", success_rate));
        }

        if rpc_stats.rate_limited_calls > rpc_stats.total_calls / 10 {
            log("MONITOR", LogLevel::Warn, "High rate limiting detected");
        }
    }
}

async fn monitor_trading_activities() {
    use crate::trader::TRADER_MANAGER;

    if let Ok(trader_guard) = TRADER_MANAGER.lock() {
        if let Some(trader) = trader_guard.as_ref() {
            let stats = trader.get_trading_stats();

            if stats.total_trades > 0 {
                if stats.win_rate < 30.0 {
                    log(
                        "MONITOR",
                        LogLevel::Warn,
                        &format!("Low win rate: {:.1}%", stats.win_rate)
                    );
                }

                if stats.total_pnl < -0.1 {
                    log(
                        "MONITOR",
                        LogLevel::Warn,
                        &format!("Significant losses: {:.6} SOL", stats.total_pnl)
                    );
                }

                if stats.active_positions > 10 {
                    log(
                        "MONITOR",
                        LogLevel::Info,
                        &format!("High number of active positions: {}", stats.active_positions)
                    );
                }
            }
        }
    }
}

async fn monitor_pool_activities() {
    use crate::pools::POOL_MANAGER;

    if let Ok(pool_guard) = POOL_MANAGER.lock() {
        if let Some(pool_manager) = pool_guard.as_ref() {
            let stats = pool_manager.get_pool_stats();

            if stats.total_pools == 0 {
                log("MONITOR", LogLevel::Warn, "No pools discovered yet");
            }

            if stats.total_pools > 1000 {
                log(
                    "MONITOR",
                    LogLevel::Info,
                    &format!("Large number of pools: {}", stats.total_pools)
                );
            }

            // Check for unusual liquidity patterns
            if stats.total_liquidity > 0.0 {
                let avg_liquidity = stats.total_liquidity / (stats.total_pools as f64);
                if avg_liquidity < 1000.0 {
                    log("MONITOR", LogLevel::Warn, "Low average pool liquidity detected");
                }
            }
        }
    }
}

async fn monitor_error_logs() {
    let recent_errors = get_recent_logs(50)
        .into_iter()
        .filter(|entry| matches!(entry.level, crate::logger::LogLevel::Error))
        .count();

    if recent_errors > 10 {
        log(
            "MONITOR",
            LogLevel::Warn,
            &format!("High error rate: {} errors in recent logs", recent_errors)
        );
    }
}

pub async fn get_system_status() -> SystemStatus {
    use crate::trader::TRADER_MANAGER;
    use crate::pools::POOL_MANAGER;

    let bot_state = get_bot_state();
    let rpc_stats = get_rpc_stats();
    let wallet_balance = get_wallet_balance();

    let trading_stats = if let Ok(trader_guard) = TRADER_MANAGER.lock() {
        trader_guard.as_ref().map(|t| t.get_trading_stats())
    } else {
        None
    };

    let pool_stats = if let Ok(pool_guard) = POOL_MANAGER.lock() {
        pool_guard.as_ref().map(|p| p.get_pool_stats())
    } else {
        None
    };

    SystemStatus {
        config_loaded: bot_state
            .lock()
            .map(|s| s.get_config().is_some())
            .unwrap_or(false),
        wallet_balance,
        rpc_health: if rpc_stats.total_calls > 0 {
            (((rpc_stats.total_calls - rpc_stats.failed_calls) as f64) /
                (rpc_stats.total_calls as f64)) *
                100.0
        } else {
            100.0
        },
        total_pools: pool_stats.map(|s| s.total_pools).unwrap_or(0),
        active_positions: trading_stats
            .as_ref()
            .map(|s| s.active_positions)
            .unwrap_or(0),
        total_trades: trading_stats
            .as_ref()
            .map(|s| s.total_trades)
            .unwrap_or(0),
        uptime_seconds: std::time::SystemTime
            ::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
    }
}

#[derive(Debug)]
pub struct SystemStatus {
    pub config_loaded: bool,
    pub wallet_balance: f64,
    pub rpc_health: f64,
    pub total_pools: usize,
    pub active_positions: usize,
    pub total_trades: u64,
    pub uptime_seconds: u64,
}
