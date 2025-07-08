mod prelude;
mod dexscreener;
mod trader;
mod configs;
mod helpers;
mod swap_gmgn;
mod pool_decoder;
mod pool_cpmm;
mod pool_meteora_dlmm;
mod pool_orca_whirlpool;
mod pool_pumpfun;
mod pool_raydium_amm;
mod pool_raydium_clmm;
mod pool_raydium_cpmm;
mod pool_raydium_launchpad;
mod pool_pumpfun2;
mod persistence;
mod pool_price;

use prelude::*;





#[tokio::main]
async fn main() -> Result<()> {
    // 1 ─ install signal handlers
    install_sigint_handler()?;

    // 2 ─ restore caches
    persistence::load_cache().await?;

    // 3 ─ start background services (each spawns its own task and returns)
    dexscreener::start_dexscreener_loop();
    trader::start_trader_loop();

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
    flush_pool_cache_to_disk_nonblocking();

    println!("✅ graceful shutdown complete.");
    process::exit(0);
}

