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
mod web_server;
mod transaction_manager;

use prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    // 1 ─ install signal handlers
    install_sigint_handler()?;

    // 2 ─ restore caches and initialize transaction manager
    persistence::load_cache().await?;

    // Initialize transaction manager
    let tx_manager = transaction_manager::init_transaction_manager(
        crate::configs::CONFIGS.rpc_url.clone()
    );

    // Load pending transactions
    TransactionManager::load_pending_transactions().await?;

    // Check for any pending transactions from previous session
    let pending_count = TransactionManager::get_all_pending_transactions().await.len();
    if pending_count > 0 {
        println!("⚠️  Found {} pending transactions from previous session - will monitor for completion", pending_count);
    }

    // Start transaction monitoring service
    tokio::spawn(async move {
        if let Err(e) = tx_manager.start_monitoring().await {
            eprintln!("Transaction monitoring error: {}", e);
        }
    });

    // Clean up old transactions periodically
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(3600)).await; // Every hour
            if let Err(e) = TransactionManager::cleanup_old_transactions().await {
                eprintln!("Transaction cleanup error: {}", e);
            }
        }
    });

    // 3 ─ start background services (each spawns its own task and returns)
    dexscreener::start_dexscreener_loop();
    trader::start_trader_loop();

    // 3.1 ─ start web server
    tokio::spawn(async move {
        if let Err(e) = web_server::start_web_server().await {
            eprintln!("Web server error: {}", e);
        }
    });

    // 4 ─ periodic autosave task
    let autosaver = task::spawn(persistence::autosave_loop());

    // 5 ─ block until the first shutdown signal arrives
    wait_for_shutdown_signal().await;

    println!("⏹  shutdown signal caught, flushing state …");

    // 6 ─ broadcast shutdown flag so every long‑running loop stops
    SHUTDOWN.store(true, Ordering::Release);
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    // 7 ─ abort autosaver if still pending
    autosaver.abort();
    let _ = autosaver.await;

    // 8 ─ final flush to disk
    persistence::save_open().await;
    persistence::save_closed().await;
    persistence::save_all_closed().await;
    persistence::save_trading_history().await;
    flush_pool_cache_to_disk_nonblocking();

    println!("✅ graceful shutdown complete.");
    process::exit(0);
}
