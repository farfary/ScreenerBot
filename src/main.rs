#![allow(warnings)]

// use crate::helpers::get_all_tokens;

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

use anyhow::Result;
use tokio::signal;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<()> {
    // ── 1. restore cache from disk ───────────────────────────────────────────
    persistence::load_cache().await?;

    // ── 2. start the background auto-saver ───────────────────────────────────
    tokio::spawn(async { persistence::autosave_loop().await; });

    // ── 3. kick off the two main services (they spawn their own tasks) ───────
    dexscreener::start_dexscreener_loop().await;
    trader::start_trader_loop().await;

    // ── 4. keep the program alive and shut down gracefully on Ctrl-C ─────────
    signal::ctrl_c().await?;
    println!("⏹  Ctrl-C received, saving cache …");
    persistence::save_open().await?;
    persistence::save_closed().await?;
    Ok(())
}