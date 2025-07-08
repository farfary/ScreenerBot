#![allow(warnings)]
use crate::prelude::*;

use std::{ collections::HashMap, fs::File, io::BufReader, time::Instant };

use anyhow::{ anyhow, Result };
use once_cell::sync::Lazy;
use parking_lot::RwLock;
use rayon::prelude::*;
use serde::Deserialize;
use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;

const POOL_CACHE_FILE: &str = ".pool_cache.json";

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

    // ---------- log change (skip zero) -----------------------------------
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

    if pct != 0.0 {
        let pct_str = if pct > 0.0 {
            format!("\x1b[32;1m+{pct:.2}%\x1b[0m")
        } else {
            format!("\x1b[31;1m{pct:.2}%\x1b[0m")
        };

        println!(
            "🏆 [POOL] {mint} → {pct_str} \x1b[1m{price:.12}\x1b[0m SOL ({} ms)",
            t0.elapsed().as_millis()
        );
    }

    // ---------- cache price ----------------------------------------------
    let ts = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_secs();
    PRICE_CACHE.write().unwrap().insert(mint.to_string(), (ts, price));

    Ok(price)
}

/// Batch fetch prices for multiple tokens to reduce RPC costs
/// Returns HashMap<mint, price> for successful fetches
/// Logs important info including RPC savings
pub fn batch_prices_from_pools(rpc: &RpcClient, mints: &[String]) -> HashMap<String, f64> {
    if mints.is_empty() {
        return HashMap::new();
    }

    println!("💰 [BATCH] Fetching prices for {} tokens to save RPC costs...", mints.len());
    let batch_start = Instant::now();

    ensure_pool_cache_loaded().unwrap_or_else(|e| {
        eprintln!("⚠️ Failed to load pool cache: {}", e);
    });

    // 1. Collect all pool addresses that we need to fetch
    let mut mint_to_pool: HashMap<String, Pubkey> = HashMap::new();
    let mut missing_pools: Vec<String> = Vec::new();

    {
        let cache = POOL_CACHE.read();
        for mint in mints {
            if let Some(&pool_pk) = cache.get(mint) {
                mint_to_pool.insert(mint.clone(), pool_pk);
            } else {
                missing_pools.push(mint.clone());
            }
        }
    }

    // 2. For missing pools, find biggest pools (this still requires individual calls)
    for mint in &missing_pools {
        match find_biggest_pool_for_mint(rpc, mint) {
            Ok(pool_pk) => {
                mint_to_pool.insert(mint.clone(), pool_pk);
                // Cache it
                POOL_CACHE.write().insert(mint.clone(), pool_pk);
            }
            Err(e) => {
                eprintln!("❌ Failed to find pool for {}: {}", mint, e);
            }
        }
    }

    if !missing_pools.is_empty() {
        println!("🔍 [BATCH] Found {} new pools, cached for future use", missing_pools.len());
        tokio::task::spawn_blocking(flush_pool_cache_to_disk);
    }

    // 3. Batch fetch all pool accounts
    let pool_addresses: Vec<Pubkey> = mint_to_pool.values().copied().collect();
    let mint_order: Vec<String> = mint_to_pool.keys().cloned().collect();

    if pool_addresses.is_empty() {
        println!("⚠️ [BATCH] No valid pools found for any tokens");
        return HashMap::new();
    }

    println!(
        "📡 [BATCH] Fetching {} pool accounts in single RPC call (vs {} individual calls)",
        pool_addresses.len(),
        pool_addresses.len()
    );

    let accounts_result = rpc.get_multiple_accounts(&pool_addresses);
    let accounts = match accounts_result {
        Ok(accounts) => accounts,
        Err(e) => {
            eprintln!("❌ [BATCH] Failed to fetch multiple accounts: {}", e);
            return HashMap::new();
        }
    };

    // 4. Process each account to get prices
    let mut prices = HashMap::new();
    let mut successful_prices = 0;

    for (i, account_opt) in accounts.iter().enumerate() {
        let mint = &mint_order[i];
        let pool_pk = pool_addresses[i];

        if let Some(account) = account_opt {
            match decode_pool_account_to_price(rpc, &pool_pk, account) {
                Ok(price) => {
                    let prev_price = {
                        PRICE_CACHE.read()
                            .unwrap()
                            .get(mint)
                            .map(|&(_, p)| p)
                    };

                    // Calculate price change
                    let pct = match prev_price {
                        Some(p) if p != 0.0 => ((price - p) / p) * 100.0,
                        _ => 0.0,
                    };

                    // Log significant price changes
                    if pct.abs() > 1.0 {
                        // Only log changes > 1%
                        let pct_str = if pct > 0.0 {
                            format!("\x1b[32;1m+{:.2}%\x1b[0m", pct)
                        } else {
                            format!("\x1b[31;1m{:.2}%\x1b[0m", pct)
                        };

                        let symbol = mint.chars().take(4).collect::<String>();
                        println!(
                            "📊 [BATCH] {} → {} \x1b[1m{:.12}\x1b[0m SOL",
                            symbol,
                            pct_str,
                            price
                        );
                    }

                    // Update price cache
                    let ts = std::time::SystemTime
                        ::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs();
                    PRICE_CACHE.write().unwrap().insert(mint.clone(), (ts, price));

                    prices.insert(mint.clone(), price);
                    successful_prices += 1;
                }
                Err(e) => {
                    eprintln!("❌ [BATCH] Failed to decode price for {}: {}", mint, e);
                }
            }
        } else {
            eprintln!("❌ [BATCH] Account not found for {}", mint);
        }
    }

    let batch_duration = batch_start.elapsed();
    let rpc_savings = pool_addresses.len().saturating_sub(1); // We saved this many RPC calls

    println!(
        "✅ [BATCH] Completed in {} ms - Success: {}/{} - Saved {} RPC calls 💰",
        batch_duration.as_millis(),
        successful_prices,
        mints.len(),
        rpc_savings
    );

    prices
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

/// Helper: Find biggest pool for a single mint (used for cache misses)
fn find_biggest_pool_for_mint(rpc: &RpcClient, mint: &str) -> Result<Pubkey> {
    let pools = fetch_solana_pairs(mint)?;
    let (best, _liq) = pools
        .par_iter()
        .filter_map(|pk| {
            decode_any_pool(rpc, pk)
                .ok()
                .map(|(b, q, _, _)| (*pk, (b as u128) + (q as u128)))
        })
        .max_by_key(|&(_, liq)| liq)
        .ok_or_else(|| anyhow!("no valid pools for {}", mint))?;

    Ok(best)
}

/// Helper: Decode a pool account (already fetched) to get price
fn decode_pool_account_to_price(
    rpc: &RpcClient,
    pool_pk: &Pubkey,
    account: &solana_sdk::account::Account
) -> Result<f64> {
    let owner = account.owner.to_string();

    let (base_amt, quote_amt, base_mint, quote_mint) = match owner.as_str() {
        // Pump.fun (Raydium-CLMM v1)
        "pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA" =>
            decode_pumpfun_pool_from_account(rpc, pool_pk, account)?,
        // PumpFun v2 CPMM
        "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P" =>
            decode_pumpfun2_pool_from_account(rpc, pool_pk, account)?,
        // Raydium CLMM v2
        "CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK" =>
            decode_raydium_clmm_from_account(rpc, pool_pk, account)?,
        // Raydium AMM v4
        "RVKd61ztZW9g2VZgPZrFYuXJcZ1t7xvaUo1NkL6MZ5w" =>
            decode_raydium_amm_from_account(rpc, pool_pk, account)?,
        // Raydium CPMM
        "CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C" =>
            decode_raydium_cpmm_from_account(rpc, pool_pk, account)?,
        // Orca Whirlpool
        "whirLb9FtDwZ2Bi4FXe65aaPaJqmCj7QSfUeCrpuHgx" =>
            decode_orca_whirlpool_from_account(rpc, pool_pk, account)?,
        // Meteora DLMM (uncomment if needed)
        // "LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo" |
        // "cpamdpZCGKUy5JxQXB4dcpGPiikHawvSWAd6mEn1sGG" =>
        //     decode_meteora_dlmm_from_account(rpc, pool_pk, account)?,

        // Raydium Launchpad
        "LanMV9sAd7wArD4vJFi2qDdfnVhFxYSUg6eADduJ3uj" =>
            decode_raydium_launchpad_from_account(rpc, pool_pk, account)?,

        _ => {
            return Err(anyhow!("Unsupported program id {} for pool {}", owner, pool_pk));
        }
    };

    if base_amt == 0 {
        return Err(anyhow!("base reserve is zero – cannot calculate price"));
    }

    let base_dec = get_token_decimals(rpc, &base_mint)? as i32;
    let quote_dec = get_token_decimals(rpc, &quote_mint)? as i32;

    // price of **one whole base token** expressed in quote tokens
    let price = ((quote_amt as f64) / (base_amt as f64)) * (10f64).powi(base_dec - quote_dec);

    Ok(price)
}
