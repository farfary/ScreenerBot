//! src/pool_price.rs
//! -------------------------------------------------------------
//! • *NO* price-ttl caching anymore – price is fetched every call
//! • Biggest-pool `Pubkey` **is** cached (RAM + `pool_cache.json`)
//! -------------------------------------------------------------

use std::{
    collections::HashMap,
    fs::{ File, OpenOptions },
    io::{ BufReader, BufWriter },
    time::{ Instant, SystemTime, UNIX_EPOCH },
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

/// in-memory cache: token-mint → biggest-pool address
pub static POOL_CACHE: Lazy<RwLock<HashMap<String, Pubkey>>> = Lazy::new(||
    RwLock::new(HashMap::new())
);

/// on first use try to load the on-disk cache into `POOL_CACHE`
fn ensure_pool_cache_loaded() -> Result<()> {
    let mut cache = POOL_CACHE.write();
    if !cache.is_empty() {
        return Ok(());
    }
    if let Ok(file) = File::open(POOL_CACHE_FILE) {
        #[derive(Deserialize)]
        struct DiskEntry {
            token: String,
            pool: String,
        }
        let entries: Vec<DiskEntry> = serde_json::from_reader(BufReader::new(file))?;
        for DiskEntry { token, pool } in entries {
            if let Ok(pk) = pool.parse::<Pubkey>() {
                cache.insert(token, pk);
            }
        }
    }
    Ok(())
}

/// flush current `POOL_CACHE` to disk (best-effort)
fn flush_pool_cache_to_disk() {
    #[derive(Serialize)]
    struct DiskEntry<'a> {
        token: &'a str,
        pool: String,
    }
    let cache = POOL_CACHE.read();
    let entries: Vec<_> = cache
        .iter()
        .map(|(t, p)| DiskEntry {
            token: t,
            pool: p.to_string(),
        })
        .collect();
    if
        let Ok(file) = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(POOL_CACHE_FILE)
    {
        let _ = serde_json::to_writer_pretty(BufWriter::new(file), &entries);
    }
}

pub fn price_from_biggest_pool(rpc: &RpcClient, token_mint: &str) -> Result<f64> {
    use crate::pool_decoder::{ decode_any_pool, decode_any_pool_price };
    use std::time::Instant;

    ensure_pool_cache_loaded()?;

    let start = Instant::now();
    let maybe_pool = { POOL_CACHE.read().get(token_mint).copied() };

    // ---------- 1. best pool address -------------------------------------------------
    let best_pk = if let Some(pk) = maybe_pool {
        pk
    } else {
        let pools = fetch_solana_pairs(token_mint)?;
        println!(
            "🔗 [POOL] {token_mint} → fetched {} pool(s) ({} ms)",
            pools.len(),
            start.elapsed().as_millis()
        );
        let (best_pk, _max_liq) = pools
            .par_iter()
            .filter_map(|pk| {
                decode_any_pool(rpc, pk)
                    .ok()
                    .map(|(b, q, _, _)| (*pk, (b as u128) + (q as u128)))
            })
            .max_by_key(|&(_, liq)| liq)
            .ok_or_else(|| anyhow!("no valid pools for {token_mint}"))?;
        POOL_CACHE.write().insert(token_mint.to_string(), best_pk);
        flush_pool_cache_to_disk();
        best_pk
    };

    // ---------- 2. get fresh price ---------------------------------------------------
    let (_, _, price) = decode_any_pool_price(rpc, &best_pk)?;

    // ---------- compute percent change from last price ------------------------------
    let prev_price_opt = PRICE_CACHE.read()
        .unwrap()
        .get(token_mint)
        .map(|&(_ts, p)| p);

    let pct_str = if let Some(prev) = prev_price_opt {
        let pct = ((price - prev) / prev) * 100.0;
        if pct > 0.0 {
            // green, bold
            format!("\x1b[32m\x1b[1m+{:.2}%\x1b[0m", pct)
        } else if pct < 0.0 {
            // red, bold
            format!("\x1b[31m\x1b[1m{:.2}%\x1b[0m", pct)
        } else {
            // no change, dark gray
            format!("\x1b[90m+0.00%\x1b[0m")
        }
    } else {
        // first time, treat as no change
        format!("\x1b[90m+0.00%\x1b[0m")
    };

    // ---------- 3. log with percent before price, price bold -------------------------
    println!(
        "🏆 [POOL] {} → {} \x1b[1m{:.12}\x1b[0m SOL (total {} ms)",
        token_mint,
        pct_str,
        price,
        start.elapsed().as_millis()
    );

    // ---------- 4. save price in RAM for next tick ---------------------------------
    let ts = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_secs();
    PRICE_CACHE.write().unwrap().insert(token_mint.to_string(), (ts, price));

    Ok(price)
}

pub fn flush_pool_cache_to_disk_nonblocking() {
    use serde::Serialize;

    #[derive(Serialize)]
    struct DiskEntry<'a> {
        token: &'a str,
        pool: String,
    }

    // If someone is writing, skip – abandoning a cache write is fine.
    let guard = match POOL_CACHE.try_read() {
        Some(g) => g,
        None => {
            eprintln!("⚠️  pool cache busy; skip flush");
            return;
        }
    };

    let entries: Vec<_> = guard
        .iter()
        .map(|(t, p)| DiskEntry { token: t, pool: p.to_string() })
        .collect();

    if
        let Ok(file) = std::fs::OpenOptions
            ::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(POOL_CACHE_FILE)
    {
        let _ = serde_json::to_writer_pretty(std::io::BufWriter::new(file), &entries);
    }
}
