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

/// fetch live price using the (cached) biggest pool
pub fn price_from_biggest_pool(rpc: &RpcClient, token_mint: &str) -> Result<f64> {
    use crate::pool_decoder::{ decode_any_pool, decode_any_pool_price };

    ensure_pool_cache_loaded()?; // load address cache

    let start = Instant::now();
    let maybe_pool = { POOL_CACHE.read().get(token_mint).copied() };

    // ---------- 1. best pool address -------------------------------------------------
    let best_pk = if let Some(pk) = maybe_pool {
        pk
    } else {
        // scan all pools and keep the biggest
        let pools = fetch_solana_pairs(token_mint)?;
        println!(
            "🔗 [POOL] {token_mint} → fetched {} pool(s) ({} ms)",
            pools.len(),
            start.elapsed().as_millis()
        );

        let (best_pk, max_liq) = pools
            .par_iter()
            .filter_map(|pk| {
                decode_any_pool(rpc, pk)
                    .ok()
                    .map(|(b, q, _, _)| (*pk, (b as u128) + (q as u128)))
            })
            .max_by_key(|&(_, liq)| liq)
            .ok_or_else(|| anyhow!("no valid pools for {token_mint}"))?;

        // remember address in RAM + disk
        POOL_CACHE.write().insert(token_mint.to_string(), best_pk);
        flush_pool_cache_to_disk();
        // println!("💾 [POOL] {token_mint} → new biggest pool {best_pk} | liq {max_liq}");
        best_pk
    };

    // ---------- 2. get fresh price ---------------------------------------------------
    let (_, _, price) = decode_any_pool_price(rpc, &best_pk)?;
    println!(
        "🏆 [POOL] {token_mint} → price {price:.12} SOL (total {} ms)",
        start.elapsed().as_millis()
    );

    // ---------- 3. save price in RAM for UI -----------------------------------------
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
