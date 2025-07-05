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
use std::process;
use tokio::{signal, task};

use pool_price::flush_pool_cache_to_disk_nonblocking;
use utilitis::{install_sigint_handler, SHUTDOWN};

// --- New imports ---
use once_cell::sync::Lazy;
use std::env;



#[tokio::main]
async fn main() -> Result<()> {
    // You can now refer to ARGS anywhere, e.g.:
    // println!("All args: {:?}", *ARGS);

    // 1 ─ install lightweight Ctrl-C handler (sets SHUTDOWN = true)
    install_sigint_handler()?;

    // 2 ─ restore JSON caches
    persistence::load_cache().await?;

    // 3 ─ start long-running services (they spawn their own tasks)
    dexscreener::start_dexscreener_loop(); // returns immediately
    trader::start_trader_loop();           // returns immediately

    // 4 ─ background autosaver (returns a JoinHandle we can abort later)
    let autosaver = task::spawn(persistence::autosave_loop());

    // 5 ─ wait for the **first** async Ctrl-C delivered by Tokio
    signal::ctrl_c().await?;
    println!("⏹  Ctrl-C received, shutting down …");

    // 6 ─ let every loop see the flag and stop by itself
    SHUTDOWN.store(true, std::sync::atomic::Ordering::SeqCst);
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    // 7 ─ abort the autosaver if it is still sleeping
    autosaver.abort();
    let _ = autosaver.await;

    // 8 ─ final flush
    persistence::save_open().await;
    persistence::save_closed().await;
    flush_pool_cache_to_disk_nonblocking();

    println!("✅ graceful shutdown complete.");
    process::exit(0);
}
