// ═══════════════════════════════════════════════════════════════════════════════
// DUAL-TIER PRICING SYSTEM - OPTIMIZED FOR RPC COST & SPEED
// ═══════════════════════════════════════════════════════════════════════════════
//
// This module implements a smart pricing system that balances speed and RPC costs:
//
// 🚀 FAST TIER (for positions & known tokens):
//    - batch_prices_fast_tier() - Only fetches tokens with cached pool addresses
//    - batch_prices_for_positions() - Priority pricing for open positions
//    - Near-instant pricing since pool discovery is skipped
//    - Minimal RPC calls (1 batch call for all tokens)
//
// 🔍 DISCOVERY TIER (for new tokens):
//    - batch_prices_discovery_tier() - Handles pool discovery for new tokens
//    - batch_prices_for_discovery() - Optimized for token candidates
//    - More expensive but caches results for future fast access
//    - Processes in small batches to avoid timeouts
//
// 🧠 SMART ROUTING:
//    - batch_prices_smart() - Automatically chooses best strategy
//    - batch_prices_from_pools() - Balanced dual-tier approach
//    - Analyzes token mix and routes accordingly
//
// USAGE EXAMPLES:
//   - Position monitoring: batch_prices_for_positions(rpc, position_mints)
//   - Token discovery: batch_prices_for_discovery(rpc, candidate_mints)
//   - Mixed scenarios: batch_prices_smart(rpc, all_mints)
//
// This ensures:
// ✅ Open positions get instant price updates (critical for trading)
// ✅ New token discovery still works but doesn't slow down position monitoring
// ✅ RPC costs are minimized through intelligent batching and caching
// ✅ Pool cache grows over time, making the system faster
// ═══════════════════════════════════════════════════════════════════════════════

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

pub async fn price_from_biggest_pool_async(rpc: &RpcClient, mint: &str) -> Result<f64> {
    ensure_pool_cache_loaded()?;

    let t0 = Instant::now();
    let maybe = { POOL_CACHE.read().get(mint).copied() };

    // ---------- biggest pool ---------------------------------------------
    let pool_pk = if let Some(pk) = maybe {
        pk
    } else {
        let pools = fetch_solana_pairs(mint).await?;
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

pub fn price_from_biggest_pool(rpc: &RpcClient, mint: &str) -> Result<f64> {
    // For now, use blocking runtime to call the async version
    let rt = tokio::runtime::Handle
        ::try_current()
        .map_err(|_| anyhow!("No async runtime available"))?;

    rt.block_on(price_from_biggest_pool_async(rpc, mint))
}

/// Fast batch pricing for tokens with known pools (priority tokens like open positions)
/// Returns HashMap<mint, price> for successful fetches
/// This function only processes tokens that already have cached pool addresses
pub fn batch_prices_fast_tier(rpc: &RpcClient, mints: &[String]) -> HashMap<String, f64> {
    if mints.is_empty() {
        return HashMap::new();
    }

    let debug_prices = crate::configs::ARGS.iter().any(|a| a == "--debug-prices");

    ensure_pool_cache_loaded().unwrap_or_else(|e| {
        eprintln!("⚠️ Failed to load pool cache: {}", e);
    });

    // Only process tokens with known pool addresses
    let mut mint_to_pool: HashMap<String, Pubkey> = HashMap::new();
    {
        let cache = POOL_CACHE.read();
        for mint in mints {
            if let Some(&pool_pk) = cache.get(mint) {
                mint_to_pool.insert(mint.clone(), pool_pk);
                if debug_prices {
                    println!("⚡ [DEBUG-PRICES] Found cached pool for {}: {}", mint, pool_pk);
                }
            } else if debug_prices {
                println!("⚠️ [DEBUG-PRICES] No cached pool found for {}", mint);
            }
        }
    }

    if mint_to_pool.is_empty() {
        if debug_prices {
            println!("⚠️ [DEBUG-PRICES] No tokens have cached pools for fast pricing");
        }
        return HashMap::new();
    }

    if debug_prices {
        println!(
            "⚡ [DEBUG-PRICES] Fast pricing {} tokens with known pools...",
            mint_to_pool.len()
        );
    }

    let fast_start = Instant::now();
    let pool_addresses: Vec<Pubkey> = mint_to_pool.values().copied().collect();
    let mint_order: Vec<String> = mint_to_pool.keys().cloned().collect();

    let accounts_result = rpc.get_multiple_accounts(&pool_addresses);
    let accounts = match accounts_result {
        Ok(accounts) => accounts,
        Err(e) => {
            if debug_prices {
                eprintln!("❌ [DEBUG-PRICES] Failed to fetch multiple accounts: {}", e);
            } else {
                eprintln!("❌ [FAST] Failed to fetch multiple accounts: {}", e);
            }
            return HashMap::new();
        }
    };

    let mut prices = HashMap::new();
    let mut successful_prices = 0;

    for (i, account_opt) in accounts.iter().enumerate() {
        let mint = &mint_order[i];
        let pool_pk = pool_addresses[i];

        if let Some(account) = account_opt {
            if debug_prices {
                println!(
                    "⚡ [DEBUG-PRICES] Decoding fast price for {} from pool {}",
                    mint,
                    pool_pk
                );
            }

            match decode_pool_account_to_price(rpc, &pool_pk, account) {
                Ok(price) => {
                    update_price_cache_with_change_log(mint, price);
                    prices.insert(mint.clone(), price);
                    successful_prices += 1;

                    if debug_prices {
                        println!("✅ [DEBUG-PRICES] Fast price for {}: {:.9}", mint, price);
                    }
                }
                Err(e) => {
                    // CRITICAL FIX: Remove failed pools from cache
                    POOL_CACHE.write().remove(mint);

                    if debug_prices {
                        eprintln!(
                            "❌ [DEBUG-PRICES] Failed to decode fast price for {}: {} - REMOVED FROM CACHE",
                            mint,
                            e
                        );
                    } else {
                        eprintln!(
                            "❌ [FAST] Failed to decode price for {}: {} - REMOVED FROM CACHE",
                            mint,
                            e
                        );
                    }
                }
            }
        } else {
            // ALSO FIX: Remove pools with no account data
            POOL_CACHE.write().remove(mint);

            if debug_prices {
                println!(
                    "⚠️ [DEBUG-PRICES] No account data for {} (pool: {}) - REMOVED FROM CACHE",
                    mint,
                    pool_pk
                );
            }
        }
    }

    if debug_prices {
        println!(
            "⚡ [DEBUG-PRICES] Fast tier completed in {}ms - Priced: {}/{} tokens - RPC calls saved: {}",
            fast_start.elapsed().as_millis(),
            successful_prices,
            mint_order.len(),
            pool_addresses.len().saturating_sub(1)
        );
    } else {
        println!(
            "⚡ [FAST] {} known pools priced in {} ms - RPC saved: {}",
            successful_prices,
            fast_start.elapsed().as_millis(),
            pool_addresses.len().saturating_sub(1)
        );
    }

    // Save the updated cache to disk (async, non-blocking)
    tokio::task::spawn_blocking(flush_pool_cache_to_disk);

    prices
}

/// Slower batch pricing for tokens that need pool discovery
/// This function handles the expensive pool discovery process
pub fn batch_prices_discovery_tier(rpc: &RpcClient, mints: &[String]) -> HashMap<String, f64> {
    if mints.is_empty() {
        return HashMap::new();
    }

    let debug_prices = crate::configs::ARGS.iter().any(|a| a == "--debug-prices");

    ensure_pool_cache_loaded().unwrap_or_else(|e| {
        eprintln!("⚠️ Failed to load pool cache: {}", e);
    });

    // Only process tokens that need pool discovery
    let mut missing_pools: Vec<String> = Vec::new();
    {
        let cache = POOL_CACHE.read();
        for mint in mints {
            if !cache.contains_key(mint) {
                missing_pools.push(mint.clone());
            }
        }
    }

    if missing_pools.is_empty() {
        if debug_prices {
            println!("🔍 [DEBUG-PRICES] No tokens need pool discovery");
        }
        return HashMap::new();
    }

    let discovery_start = Instant::now();
    println!("🔍 [DISCOVERY] Finding pools for {} new tokens...", missing_pools.len());

    let mut mint_to_pool: HashMap<String, Pubkey> = HashMap::new();
    let mut successful_discoveries = 0;
    let mut failed_discoveries = 0;

    // Process discovery in smaller batches to avoid timeouts
    let batch_size = 5; // Process 5 tokens at a time
    for (batch_idx, chunk) in missing_pools.chunks(batch_size).enumerate() {
        if debug_prices {
            println!(
                "🔍 [DEBUG-PRICES] Processing batch {} ({} tokens)...",
                batch_idx + 1,
                chunk.len()
            );
        }

        for mint in chunk {
            if debug_prices {
                println!("🔍 [DEBUG-PRICES] Finding pool for token: {}", mint);
            }

            match find_biggest_pool_for_mint(rpc, mint) {
                Ok(pool_pk) => {
                    mint_to_pool.insert(mint.clone(), pool_pk);
                    // Cache immediately for future fast access
                    POOL_CACHE.write().insert(mint.clone(), pool_pk);
                    successful_discoveries += 1;

                    if debug_prices {
                        println!("✅ [DEBUG-PRICES] Found pool for {}: {}", mint, pool_pk);
                    }
                }
                Err(e) => {
                    failed_discoveries += 1;
                    if debug_prices {
                        eprintln!("❌ [DEBUG-PRICES] Failed to find pool for {}: {}", mint, e);
                    } else {
                        eprintln!("❌ [DISCOVERY] Failed to find pool for {}: {}", mint, e);
                    }
                }
            }
        }

        // Small delay between batches to avoid overwhelming the system
        if chunk.len() == batch_size {
            if debug_prices {
                println!("🔍 [DEBUG-PRICES] Batch {} completed, pausing 100ms...", batch_idx + 1);
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    }

    if !mint_to_pool.is_empty() {
        if debug_prices {
            println!(
                "🔍 [DEBUG-PRICES] Pool discovery completed - Success: {}, Failed: {}",
                successful_discoveries,
                failed_discoveries
            );
            println!(
                "🔍 [DEBUG-PRICES] Fetching prices for {} discovered pools...",
                mint_to_pool.len()
            );
        }

        // Save discovered pools to disk
        tokio::task::spawn_blocking(flush_pool_cache_to_disk);

        // Now get prices for discovered pools
        let pool_addresses: Vec<Pubkey> = mint_to_pool.values().copied().collect();
        let mint_order: Vec<String> = mint_to_pool.keys().cloned().collect();

        let accounts_result = rpc.get_multiple_accounts(&pool_addresses);
        let accounts = match accounts_result {
            Ok(accounts) => accounts,
            Err(e) => {
                if debug_prices {
                    eprintln!("❌ [DEBUG-PRICES] Failed to fetch pool accounts: {}", e);
                } else {
                    eprintln!("❌ [DISCOVERY] Failed to fetch pool accounts: {}", e);
                }
                return HashMap::new();
            }
        };

        let mut prices = HashMap::new();
        let mut successful_prices = 0;

        for (i, account_opt) in accounts.iter().enumerate() {
            let mint = &mint_order[i];
            let pool_pk = pool_addresses[i];

            if let Some(account) = account_opt {
                if debug_prices {
                    println!("🔍 [DEBUG-PRICES] Decoding price for {} from pool {}", mint, pool_pk);
                }

                match decode_pool_account_to_price(rpc, &pool_pk, account) {
                    Ok(price) => {
                        update_price_cache_with_change_log(mint, price);
                        prices.insert(mint.clone(), price);
                        successful_prices += 1;

                        if debug_prices {
                            println!("✅ [DEBUG-PRICES] Got price for {}: {:.9}", mint, price);
                        }
                    }
                    Err(e) => {
                        // CRITICAL FIX: Remove failed pools from cache
                        POOL_CACHE.write().remove(mint);

                        if debug_prices {
                            eprintln!(
                                "❌ [DEBUG-PRICES] Failed to decode price for {}: {} - REMOVED FROM CACHE",
                                mint,
                                e
                            );
                        } else {
                            eprintln!(
                                "❌ [DISCOVERY] Failed to decode price for {}: {} - REMOVED FROM CACHE",
                                mint,
                                e
                            );
                        }
                    }
                }
            } else {
                // Remove pools with no account data
                POOL_CACHE.write().remove(mint);

                if debug_prices {
                    println!(
                        "⚠️ [DEBUG-PRICES] No account data for {} (pool: {}) - REMOVED FROM CACHE",
                        mint,
                        pool_pk
                    );
                }
            }
        }

        if debug_prices {
            println!(
                "🔍 [DEBUG-PRICES] Discovery pricing completed in {}ms - Discoveries: {}/{}, Prices: {}/{}",
                discovery_start.elapsed().as_millis(),
                successful_discoveries,
                missing_pools.len(),
                successful_prices,
                mint_to_pool.len()
            );
        } else {
            println!(
                "🔍 [DISCOVERY] Completed in {} ms - Found: {}/{} pools - Priced: {} tokens",
                discovery_start.elapsed().as_millis(),
                mint_to_pool.len(),
                missing_pools.len(),
                successful_prices
            );
        }

        prices
    } else {
        if debug_prices {
            println!(
                "❌ [DEBUG-PRICES] No pools found for any new tokens after {} attempts",
                missing_pools.len()
            );
        } else {
            println!("❌ [DISCOVERY] No pools found for any new tokens");
        }
        HashMap::new()
    }
}

/// Dual-tier pricing system: Fast for known pools, slower for discovery
/// This is the main entry point that combines both tiers optimally
pub fn batch_prices_from_pools(rpc: &RpcClient, mints: &[String]) -> HashMap<String, f64> {
    if mints.is_empty() {
        return HashMap::new();
    }

    println!("💰 [DUAL-TIER] Processing {} tokens with optimized pricing...", mints.len());

    // Separate tokens into fast and discovery tiers
    let (known_tokens, unknown_tokens) = {
        ensure_pool_cache_loaded().unwrap_or_else(|e| {
            eprintln!("⚠️ Failed to load pool cache: {}", e);
        });

        let cache = POOL_CACHE.read();
        let mut known = Vec::new();
        let mut unknown = Vec::new();

        for mint in mints {
            if cache.contains_key(mint) {
                known.push(mint.clone());
            } else {
                unknown.push(mint.clone());
            }
        }
        (known, unknown)
    };

    let mut all_prices = HashMap::new();

    // Fast tier: Process known tokens immediately
    if !known_tokens.is_empty() {
        let fast_prices = batch_prices_fast_tier(rpc, &known_tokens);
        all_prices.extend(fast_prices);
    }

    // Discovery tier: Process unknown tokens (slower)
    if !unknown_tokens.is_empty() {
        let discovery_prices = batch_prices_discovery_tier(rpc, &unknown_tokens);
        all_prices.extend(discovery_prices);
    }

    let total_success = all_prices.len();
    let total_requested = mints.len();

    println!(
        "✅ [DUAL-TIER] Completed - Fast: {}, Discovery: {}, Total: {}/{} 💰",
        known_tokens.len(),
        unknown_tokens.len(),
        total_success,
        total_requested
    );

    all_prices
}

/// Priority pricing for open positions (highest priority - must be fast)
/// Uses only fast tier since position tokens should have known pools
pub fn batch_prices_for_positions(
    rpc: &RpcClient,
    position_mints: &[String]
) -> HashMap<String, f64> {
    if position_mints.is_empty() {
        return HashMap::new();
    }

    println!("⚡ [POSITIONS] Fast pricing for {} open positions...", position_mints.len());
    let start = Instant::now();

    // Use only fast tier for positions - they should all have known pools
    let prices = batch_prices_fast_tier(rpc, position_mints);

    // If any position tokens are missing from fast tier, this is unusual
    let missing_count = position_mints.len() - prices.len();
    if missing_count > 0 {
        eprintln!("⚠️ [POSITIONS] {} position tokens missing from cache - this shouldn't happen!", missing_count);

        // For positions, we need prices immediately, so do emergency discovery
        let missing_mints: Vec<String> = position_mints
            .iter()
            .filter(|mint| !prices.contains_key(*mint))
            .cloned()
            .collect();

        if !missing_mints.is_empty() {
            println!(
                "🚨 [POSITIONS] Emergency discovery for {} missing position tokens",
                missing_mints.len()
            );
            let emergency_prices = batch_prices_discovery_tier(rpc, &missing_mints);
            let total_prices = prices.into_iter().chain(emergency_prices).collect();

            println!(
                "⚡ [POSITIONS] Completed with emergency discovery in {} ms",
                start.elapsed().as_millis()
            );
            return total_prices;
        }
    }

    println!("⚡ [POSITIONS] Completed fast pricing in {} ms", start.elapsed().as_millis());
    prices
}

/// Discovery pricing for new token candidates (lower priority - can be slower)
/// Uses discovery tier since these are new tokens
pub fn batch_prices_for_discovery(
    rpc: &RpcClient,
    candidate_mints: &[String]
) -> HashMap<String, f64> {
    if candidate_mints.is_empty() {
        return HashMap::new();
    }

    println!("🔍 [CANDIDATES] Discovery pricing for {} new tokens...", candidate_mints.len());

    // Use dual-tier but prioritize discovery since these are likely new
    batch_prices_from_pools(rpc, candidate_mints)
}

/// Separated discovery pricing for token discovery - processes known and unknown pools separately
/// This prevents slow pool discovery from blocking fast pricing of known tokens
pub fn batch_prices_for_discovery_separated(
    rpc: &RpcClient,
    candidate_mints: &[String],
    debug: bool
) -> HashMap<String, f64> {
    if candidate_mints.is_empty() {
        return HashMap::new();
    }

    if debug {
        println!(
            "🔍 [DEBUG-PRICES] Starting separated discovery pricing for {} tokens",
            candidate_mints.len()
        );
    }

    ensure_pool_cache_loaded().unwrap_or_else(|e| {
        if debug {
            eprintln!("⚠️ [DEBUG-PRICES] Failed to load pool cache: {}", e);
        }
    });

    // Separate tokens into known and unknown pools
    let (known_tokens, unknown_tokens) = {
        let cache = POOL_CACHE.read();
        let mut known = Vec::new();
        let mut unknown = Vec::new();

        for mint in candidate_mints {
            if cache.contains_key(mint) {
                known.push(mint.clone());
            } else {
                unknown.push(mint.clone());
            }
        }

        if debug {
            println!(
                "🔍 [DEBUG-PRICES] Token separation - Known: {}, Unknown: {}",
                known.len(),
                unknown.len()
            );
        }

        (known, unknown)
    };

    let mut all_prices = HashMap::new();
    let discovery_start = std::time::Instant::now();

    // STEP 1: Process known tokens first (fast)
    if !known_tokens.is_empty() {
        if debug {
            println!(
                "⚡ [DEBUG-PRICES] Processing {} known tokens (fast tier)...",
                known_tokens.len()
            );
        }

        let fast_start = std::time::Instant::now();
        let fast_prices = batch_prices_fast_tier(rpc, &known_tokens);

        if debug {
            println!(
                "⚡ [DEBUG-PRICES] Fast tier completed in {}ms - Got {}/{} prices",
                fast_start.elapsed().as_millis(),
                fast_prices.len(),
                known_tokens.len()
            );
        }

        all_prices.extend(fast_prices);
    }

    // STEP 2: Process unknown tokens (slower, but limited)
    if !unknown_tokens.is_empty() {
        if debug {
            println!(
                "🔍 [DEBUG-PRICES] Processing {} unknown tokens (discovery tier)...",
                unknown_tokens.len()
            );
        }

        // Limit unknown tokens processing to avoid blocking
        let max_unknown_per_cycle = 10; // Process max 10 unknown tokens per discovery cycle
        let limited_unknown: Vec<String> = unknown_tokens
            .iter()
            .take(max_unknown_per_cycle)
            .cloned()
            .collect();

        if limited_unknown.len() < unknown_tokens.len() && debug {
            println!(
                "🔍 [DEBUG-PRICES] Limited unknown token processing to {} tokens (from {})",
                limited_unknown.len(),
                unknown_tokens.len()
            );
        }

        let discovery_tier_start = std::time::Instant::now();
        let discovery_prices = batch_prices_discovery_tier(rpc, &limited_unknown);

        if debug {
            println!(
                "🔍 [DEBUG-PRICES] Discovery tier completed in {}ms - Got {}/{} prices",
                discovery_tier_start.elapsed().as_millis(),
                discovery_prices.len(),
                limited_unknown.len()
            );
        }

        all_prices.extend(discovery_prices);
    }

    let total_duration = discovery_start.elapsed();
    if debug {
        println!(
            "✅ [DEBUG-PRICES] Separated discovery completed in {}ms - Total: {}/{} prices",
            total_duration.as_millis(),
            all_prices.len(),
            candidate_mints.len()
        );
    } else {
        println!(
            "🔍 [DISCOVERY-SEPARATED] Fast: {} | Discovery: {} | Total: {}/{} in {}ms",
            known_tokens.len(),
            std::cmp::min(unknown_tokens.len(), 10),
            all_prices.len(),
            candidate_mints.len(),
            total_duration.as_millis()
        );
    }

    all_prices
}

/// Smart pricing that automatically chooses the best strategy based on token mix
/// This analyzes the token list and uses the most efficient pricing approach
pub fn batch_prices_smart(rpc: &RpcClient, mints: &[String]) -> HashMap<String, f64> {
    if mints.is_empty() {
        return HashMap::new();
    }

    ensure_pool_cache_loaded().unwrap_or_else(|e| {
        eprintln!("⚠️ Failed to load pool cache: {}", e);
    });

    // Analyze the token mix
    let (known_count, unknown_count) = {
        let cache = POOL_CACHE.read();
        let known = mints
            .iter()
            .filter(|mint| cache.contains_key(*mint))
            .count();
        let unknown = mints.len() - known;
        (known, unknown)
    };

    let known_ratio = (known_count as f64) / (mints.len() as f64);

    // Choose strategy based on token mix
    if known_ratio >= 0.8 {
        // Mostly known tokens - prioritize speed
        println!(
            "⚡ [SMART] Using fast-priority strategy ({:.0}% known tokens)",
            known_ratio * 100.0
        );

        let mut all_prices = HashMap::new();

        // Fast tier first
        let known_tokens: Vec<String> = {
            let cache = POOL_CACHE.read();
            mints
                .iter()
                .filter(|mint| cache.contains_key(*mint))
                .cloned()
                .collect()
        };

        if !known_tokens.is_empty() {
            all_prices.extend(batch_prices_fast_tier(rpc, &known_tokens));
        }

        // Quick discovery for remaining
        let remaining: Vec<String> = mints
            .iter()
            .filter(|mint| !all_prices.contains_key(*mint))
            .cloned()
            .collect();

        if !remaining.is_empty() {
            all_prices.extend(batch_prices_discovery_tier(rpc, &remaining));
        }

        all_prices
    } else {
        // Many unknown tokens - use balanced approach
        println!("🔍 [SMART] Using balanced strategy ({:.0}% known tokens)", known_ratio * 100.0);
        batch_prices_from_pools(rpc, mints)
    }
}

/// Helper function to update price cache and log significant changes
fn update_price_cache_with_change_log(mint: &str, price: f64) {
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
        let pct_str = if pct > 0.0 {
            format!("\x1b[32;1m+{:.2}%\x1b[0m", pct)
        } else {
            format!("\x1b[31;1m{:.2}%\x1b[0m", pct)
        };

        let symbol = mint.chars().take(4).collect::<String>();
        println!("📊 {} → {} \x1b[1m{:.12}\x1b[0m SOL", symbol, pct_str, price);
    }

    // Update price cache
    let ts = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
    PRICE_CACHE.write().unwrap().insert(mint.to_string(), (ts, price));
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
    // Use blocking runtime to call the async version
    let rt = tokio::runtime::Handle
        ::try_current()
        .map_err(|_| anyhow!("No async runtime available"))?;

    let pools = rt.block_on(fetch_solana_pairs(mint))?;
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
