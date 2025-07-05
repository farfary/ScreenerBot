//! src/pool_price.rs
//! -------------------------------------------------------------
//! • Every call fetches a fresh price (no TTL).
//! • Biggest pool address is cached in RAM + pool_cache.json.
//!   – Locks are held only long enough to copy data.
//! -------------------------------------------------------------

use std::{
    collections::HashMap,
    fs::{ File, OpenOptions },
    io::{ BufReader, BufWriter },
    time::Instant,
};

use anyhow::{ anyhow, Result };
use once_cell::sync::Lazy;
use parking_lot::RwLock;
use rayon::prelude::*;
use serde::{ Deserialize, Serialize };
use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;

use crate::utilitis::*;

const POOL_CACHE_FILE: &str = "pool_cache.json";

/// token-mint → biggest-pool address
pub static POOL_CACHE: Lazy<RwLock<HashMap<String, Pubkey>>> = Lazy::new(||
    RwLock::new(HashMap::new())
);

/// one-time lazy load
fn ensure_pool_cache_loaded() -> Result<()> {
    // try quick read
    if !POOL_CACHE.read().is_empty() {
        return Ok(());
    }

    // escalate to write only if still empty
    let mut w = POOL_CACHE.write();
    if !w.is_empty() {
        return Ok(());
    }

    if let Ok(f) = File::open(POOL_CACHE_FILE) {
        #[derive(Deserialize)]
        struct DiskEntry {
            token: String,
            pool: String,
        }
        for DiskEntry { token, pool } in serde_json::from_reader::<_, Vec<DiskEntry>>(
            BufReader::new(f)
        )? {
            if let Ok(pk) = pool.parse::<Pubkey>() {
                w.insert(token, pk);
            }
        }
    }
    Ok(())
}

pub fn price_from_biggest_pool(rpc: &RpcClient, mint: &str) -> Result<f64> {
    use crate::pool_decoder::{ decode_any_pool, decode_any_pool_price };

    ensure_pool_cache_loaded()?;

    let t0 = Instant::now();
    let maybe = { POOL_CACHE.read().get(mint).copied() };

    // ---------- biggest pool ---------------------------------------------
    let pool_pk = if let Some(pk) = maybe {
        pk
    } else {
        let pools = fetch_solana_pairs(mint)?;
        let (best, _liq) = pools
            .par_iter()
            .filter_map(|pk| {
                decode_any_pool(rpc, pk)
                    .ok()
                    .map(|(b, q, _, _)| (*pk, (b as u128) + (q as u128)))
            })
            .max_by_key(|&(_, liq)| liq)
            .ok_or_else(|| anyhow!("no valid pools for {mint}"))?;

        {
            POOL_CACHE.write().insert(mint.to_string(), best);
        } // drop before spawn_blocking

        tokio::task::spawn_blocking(flush_pool_cache_to_disk);
        best
    };

    // ---------- fresh price ----------------------------------------------
    let (_, _, price) = decode_any_pool_price(rpc, &pool_pk)?;

    // ---------- log change ------------------------------------------------
    let prev = {
        PRICE_CACHE.read()
            .unwrap()
            .get(mint)
            .map(|&(_, p)| p)
    };
    let pct = match prev {
        Some(p) if p != 0.0 => ((price - p) / p) * 100.0,
        _ => 0.0,
    };
    let pct_str = if pct > 0.0 {
        format!("\x1b[32;1m+{pct:.2}%\x1b[0m")
    } else if pct < 0.0 {
        format!("\x1b[31;1m{pct:.2}%\x1b[0m")
    } else {
        "\x1b[90m+0.00%\x1b[0m".into()
    };

    println!(
        "🏆 [POOL] {mint} → {pct_str} \x1b[1m{price:.12}\x1b[0m SOL ({} ms)",
        t0.elapsed().as_millis()
    );

    // ---------- cache price ----------------------------------------------
    let ts = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_secs();
    PRICE_CACHE.write().unwrap().insert(mint.to_string(), (ts, price));

    Ok(price)
}

// --- replace the two helpers -----------------------------------------------

// ---------------------------------------------------------------------------
// 1.  Write the RAM cache to disk (runs on a blocking worker thread)
// ---------------------------------------------------------------------------
fn flush_pool_cache_to_disk() {
    #[derive(serde::Serialize)]
    struct DiskEntry {
        token: String,
        pool: String,
    }

    // ---- snapshot under read-lock ----
    let snapshot: Vec<DiskEntry> = {
        let guard = POOL_CACHE.read(); // <–– lock
        guard
            .iter()
            .map(|(t, p)| DiskEntry {
                token: t.clone(), // own the key
                pool: p.to_string(), // own the value
            })
            .collect()
    }; // <–– guard dropped here

    // ---- write the file without any lock ----
    if
        let Ok(f) = std::fs::OpenOptions
            ::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(POOL_CACHE_FILE)
    {
        let _ = serde_json::to_writer_pretty(std::io::BufWriter::new(f), &snapshot);
    }
}

// ---------------------------------------------------------------------------
// 2.  Public, non-blocking entry point used by the autosave task
// ---------------------------------------------------------------------------
pub fn flush_pool_cache_to_disk_nonblocking() {
    // Fire-and-forget on a new OS thread; returns immediately.
    std::thread::spawn(flush_pool_cache_to_disk);
}
