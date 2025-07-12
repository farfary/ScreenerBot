#![allow(warnings)]

mod prelude;
mod dexscreener;
mod trader;
mod configs;
mod helpers;
mod swap_gmgn;
mod pools;
mod persistence;
mod pool_price;
mod strategy;
mod performance;
mod trades;
mod ohlcv;
mod rate_limiter;
mod price_validation;
mod shutdown;

use prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    // 1 ─ install NEW shutdown handlers with transaction safety
    shutdown::install_shutdown_handlers()?;

    // 2 ─ restore caches
    persistence::load_cache().await?;
    performance::load_performance_history().await?;

    // 3 ─ start background services (each spawns its own task and returns)
    dexscreener::start_dexscreener_loop();
    trader::start_trader_loop();
    trades::start_trades_cache_task();
    ohlcv::start_ohlcv_cache_task();

    // 4 ─ periodic autosave task
    let autosaver = task::spawn(persistence::autosave_loop());

    // 5 ─ run until shutdown (the shutdown system handles all cleanup internally)
    println!("🚀 [MAIN] All systems started. Use Ctrl+C for graceful shutdown.");
    println!(
        "⏱️  [TIMING] Position Monitor: profitable {}s | all {}s",
        crate::strategy::config::POSITIONS_FREQUENT_CHECK_TIME_SEC,
        crate::strategy::config::POSITIONS_CHECK_TIME_SEC
    );
    println!(
        "⏱️  [TIMING] Price Monitor: {}s | Token Discovery: {}s",
        crate::strategy::config::PRICE_MONITORING_CHECK_TIME_SEC,
        crate::strategy::config::TOKEN_DISCOVERY_CHECK_TIME_SEC
    );

    // Keep the main thread alive until shutdown
    loop {
        if shutdown::is_shutdown_requested() {
            break;
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }

    // Abort autosaver if still running
    autosaver.abort();
    let _ = autosaver.await;

    // Note: All other cleanup is handled by the shutdown system
    println!("✅ [MAIN] Main loop exited, shutdown system handling cleanup.");

    // Give shutdown system time to complete
    tokio::time::sleep(Duration::from_secs(5)).await;
    Ok(())
}
