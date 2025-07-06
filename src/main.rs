#![allow(warnings)]

mod utilitis;
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
mod pool_pumpfun2;
mod persistence;
mod pool_price;

use anyhow::Result;
use tokio::task;
use pool_price::flush_pool_cache_to_disk_nonblocking;
use utilitis::{install_sigint_handler, SHUTDOWN};

use once_cell::sync::Lazy;
use std::{env, process, sync::atomic::Ordering};

/// All command‑line arguments captured at startup.
pub static ARGS: Lazy<Vec<String>> = Lazy::new(|| env::args().collect());

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

/// Waits until either Ctrl‑C (SIGINT) or SIGTERM (from systemd) is received.
async fn wait_for_shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};
        let mut term = signal(SignalKind::terminate())
            .expect("failed to install SIGTERM handler");
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {},
            _ = term.recv()             => {},
        }
    }

    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}
