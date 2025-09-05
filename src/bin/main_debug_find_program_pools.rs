use screenerbot::arguments::{ get_arg_value, has_arg };
use screenerbot::logger::{ init_file_logging, log, LogTag };
use screenerbot::rpc::get_rpc_client;
// Pool constants moved to pool_interface module
const SOL_MINT: &str = "So11111111111111111111111111111111111111112";
const RAYDIUM_CLMM_PROGRAM_ID: &str = "CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK";
// Pool DB service moved to pool_service module
use rusqlite;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use tokio;

/// Debug tool to find pools by exact program ID
/// Usage:
/// cargo run --bin main_debug_find_program_pools -- --program-id CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK
/// cargo run --bin main_debug_find_program_pools -- --program-id CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK --wsol-pairs-only
/// cargo run --bin main_debug_find_program_pools -- --help

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    init_file_logging();

    if has_arg("help") || has_arg("h") {
        print_help();
        return Ok(());
    }

    // Get program ID to search for
    let target_program_id = get_arg_value("program-id").unwrap_or_else(||
        RAYDIUM_CLMM_PROGRAM_ID.to_string()
    );

    let wsol_pairs_only = has_arg("wsol-pairs-only");
    let max_pools = get_arg_value("max-pools")
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(100);

    log(
        LogTag::Pool,
        "DEBUG_START",
        &format!("🔍 Searching for pools with program ID: {}", target_program_id)
    );

    if wsol_pairs_only {
        log(LogTag::Pool, "DEBUG_FILTER", "🎯 Filtering for TOKEN/WSOL pairs only");
    }

    // Initialize database service
    // Pool DB service moved to pool_service module

    // Search for pools
    match find_pools_by_program_id(&target_program_id, wsol_pairs_only, max_pools).await {
        Ok(pools) => {
            if pools.is_empty() {
                log(
                    LogTag::Pool,
                    "DEBUG_RESULT",
                    &format!("❌ No pools found with program ID: {}", target_program_id)
                );
            } else {
                log(
                    LogTag::Pool,
                    "DEBUG_RESULT",
                    &format!(
                        "✅ Found {} pools with program ID: {}",
                        pools.len(),
                        target_program_id
                    )
                );

                // Print detailed results
                for (i, pool) in pools.iter().enumerate() {
                    println!("\n🏊 Pool #{}: {}", i + 1, pool.pool_address);
                    println!("   📊 Program ID: {}", target_program_id);
                    println!("   🪙 Token Mint: {}", pool.token_mint);
                    println!(
                        "   🏷️  Token Symbol: {}",
                        pool.token_symbol.as_deref().unwrap_or("❓ Unknown")
                    );
                    println!(
                        "   🏷️  Token Name: {}",
                        pool.token_name.as_deref().unwrap_or("❓ Unknown")
                    );
                    println!(
                        "   💰 Quote Token: {} ({})",
                        pool.quote_symbol.as_deref().unwrap_or("❓ Unknown"),
                        pool.quote_token_address
                    );
                    println!("   🏢 DEX: {}", pool.dex_id);
                    println!("   💵 Liquidity: ${:.2}", pool.liquidity_usd.unwrap_or(0.0));
                    println!("   📈 Price: {} SOL", pool.price_native.unwrap_or(0.0));

                    if let Some(price_usd) = pool.price_usd {
                        println!("   💲 Price USD: ${:.8}", price_usd);
                    }

                    if let Some(volume) = pool.volume_24h {
                        println!("   📊 Volume 24h: ${:.2}", volume);
                    }

                    if let Some(market_cap) = pool.market_cap {
                        println!("   🏦 Market Cap: ${:.2}", market_cap);
                    }

                    if let Some(fdv) = pool.fdv {
                        println!("   📊 FDV: ${:.2}", fdv);
                    }

                    // Transaction stats
                    if let Some(buys) = pool.txns_24h_buys {
                        println!("   📈 24h Buys: {}", buys);
                    }

                    if let Some(sells) = pool.txns_24h_sells {
                        println!("   📉 24h Sells: {}", sells);
                    }

                    // Price changes
                    if let Some(change) = pool.price_change_24h {
                        let emoji = if change >= 0.0 { "📈" } else { "📉" };
                        println!("   {} 24h Change: {:.2}%", emoji, change);
                    }

                    println!("   ⏰ Last Updated: {}", pool.last_updated);
                    println!("   🔗 Pool URL: https://solscan.io/account/{}", pool.pool_address);
                    println!("   🔗 Token URL: https://solscan.io/token/{}", pool.token_mint);
                }
            }
        }
        Err(e) => {
            log(LogTag::Pool, "DEBUG_ERROR", &format!("❌ Error searching pools: {}", e));
        }
    }

    Ok(())
}

fn print_help() {
    println!("Debug Tool: Find Pools by Program ID");
    println!();
    println!("USAGE:");
    println!("  cargo run --bin main_debug_find_program_pools [OPTIONS]");
    println!();
    println!("OPTIONS:");
    println!("  --program-id <PROGRAM_ID>   Program ID to search for (default: Raydium CLMM)");
    println!("  --wsol-pairs-only          Only show TOKEN/WSOL pairs");
    println!("  --max-pools <NUMBER>       Maximum pools to check (default: 100)");
    println!("  --help                     Show this help message");
    println!();
    println!("EXAMPLES:");
    println!("  # Find Raydium CLMM pools");
    println!(
        "  cargo run --bin main_debug_find_program_pools -- --program-id CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK"
    );
    println!();
    println!("  # Find TOKEN/WSOL pairs only");
    println!(
        "  cargo run --bin main_debug_find_program_pools -- --program-id CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK --wsol-pairs-only"
    );
    println!();
    println!("  # Check more pools");
    println!(
        "  cargo run --bin main_debug_find_program_pools -- --program-id CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK --max-pools 500"
    );
}

#[derive(Debug)]
struct PoolResult {
    pub pool_address: String,
    pub token_mint: String,
    pub token_symbol: Option<String>,
    pub token_name: Option<String>,
    pub quote_token_address: String,
    pub quote_symbol: Option<String>,
    pub dex_id: String,
    pub liquidity_usd: Option<f64>,
    pub price_native: Option<f64>,
    pub price_usd: Option<f64>,
    pub volume_24h: Option<f64>,
    pub market_cap: Option<f64>,
    pub fdv: Option<f64>,
    pub txns_24h_buys: Option<i64>,
    pub txns_24h_sells: Option<i64>,
    pub price_change_24h: Option<f64>,
    pub last_updated: String,
}

async fn find_pools_by_program_id(
    target_program_id: &str,
    wsol_pairs_only: bool,
    max_pools: usize
) -> Result<Vec<PoolResult>, String> {
    // Get all pool addresses from database
    let pool_addresses = get_all_pool_addresses_from_db(wsol_pairs_only, max_pools)?;

    log(
        LogTag::Pool,
        "DEBUG_SEARCH",
        &format!("🔍 Checking {} pool addresses for program ID match", pool_addresses.len())
    );

    let mut matching_pools = Vec::new();
    let rpc_client = get_rpc_client();

    // Parse target program ID
    let target_pubkey = Pubkey::from_str(target_program_id).map_err(|e|
        format!("Invalid program ID: {}", e)
    )?;

    // Check each pool address on-chain
    for (i, pool_data) in pool_addresses.iter().enumerate() {
        if i % 50 == 0 && i > 0 {
            log(
                LogTag::Pool,
                "DEBUG_PROGRESS",
                &format!(
                    "🔄 Checked {}/{} pools, found {} matches",
                    i,
                    pool_addresses.len(),
                    matching_pools.len()
                )
            );
        }

        // Parse pool address
        let pool_pubkey = match Pubkey::from_str(&pool_data.pool_address) {
            Ok(pubkey) => pubkey,
            Err(_) => {
                log(
                    LogTag::Pool,
                    "DEBUG_SKIP",
                    &format!("⚠️ Invalid pool address: {}", pool_data.pool_address)
                );
                continue;
            }
        };

        // Get account info to check owner (program ID)
        match rpc_client.get_account(&pool_pubkey).await {
            Ok(account) => {
                if account.owner == target_pubkey {
                    log(
                        LogTag::Pool,
                        "DEBUG_MATCH",
                        &format!(
                            "✅ Found matching pool: {} (Token: {})",
                            pool_data.pool_address,
                            pool_data.token_mint
                        )
                    );

                    matching_pools.push(PoolResult {
                        pool_address: pool_data.pool_address.clone(),
                        token_mint: pool_data.token_mint.clone(),
                        token_symbol: pool_data.token_symbol.clone(),
                        token_name: pool_data.token_name.clone(),
                        quote_token_address: pool_data.quote_token_address.clone(),
                        quote_symbol: pool_data.quote_symbol.clone(),
                        dex_id: pool_data.dex_id.clone(),
                        liquidity_usd: pool_data.liquidity_usd,
                        price_native: pool_data.price_native,
                        price_usd: pool_data.price_usd,
                        volume_24h: pool_data.volume_24h,
                        market_cap: pool_data.market_cap,
                        fdv: pool_data.fdv,
                        txns_24h_buys: pool_data.txns_24h_buys,
                        txns_24h_sells: pool_data.txns_24h_sells,
                        price_change_24h: pool_data.price_change_24h,
                        last_updated: pool_data.last_updated.clone(),
                    });
                }
            }
            Err(e) => {
                // Pool might not exist or RPC error - skip silently unless debug enabled
                if matching_pools.len() < 5 {
                    // Only log first few errors
                    log(
                        LogTag::Pool,
                        "DEBUG_RPC_ERROR",
                        &format!("⚠️ RPC error for pool {}: {}", &pool_data.pool_address[..8], e)
                    );
                }
            }
        }
    }

    log(
        LogTag::Pool,
        "DEBUG_COMPLETE",
        &format!(
            "🎯 Search complete: found {} matching pools out of {} checked",
            matching_pools.len(),
            pool_addresses.len()
        )
    );

    Ok(matching_pools)
}

#[derive(Debug)]
struct PoolDbData {
    pub pool_address: String,
    pub token_mint: String,
    pub token_symbol: Option<String>,
    pub token_name: Option<String>,
    pub quote_token_address: String,
    pub quote_symbol: Option<String>,
    pub dex_id: String,
    pub liquidity_usd: Option<f64>,
    pub price_native: Option<f64>,
    pub price_usd: Option<f64>,
    pub volume_24h: Option<f64>,
    pub market_cap: Option<f64>,
    pub fdv: Option<f64>,
    pub txns_24h_buys: Option<i64>,
    pub txns_24h_sells: Option<i64>,
    pub price_change_24h: Option<f64>,
    pub last_updated: String,
}

fn get_all_pool_addresses_from_db(
    wsol_pairs_only: bool,
    max_pools: usize
) -> Result<Vec<PoolDbData>, String> {
    let conn = rusqlite::Connection
        ::open("data/pools.db")
        .map_err(|e| format!("Failed to open database: {}", e))?;

    let mut query =
        "SELECT pool_address, token_mint, base_token_symbol, base_token_name, quote_token_address, quote_token_symbol, dex_id, liquidity_usd, price_native, price_usd, volume_24h, market_cap, fdv, txns_24h_buys, txns_24h_sells, price_change_24h, last_updated FROM pool_metadata WHERE is_active = 1".to_string();

    if wsol_pairs_only {
        query.push_str(&format!(" AND quote_token_address = '{}'", SOL_MINT));
        log(LogTag::Pool, "DEBUG_FILTER", "🎯 Filtering for TOKEN/WSOL pairs only");
    }

    query.push_str(" ORDER BY liquidity_usd DESC");
    query.push_str(&format!(" LIMIT {}", max_pools));

    let mut stmt = conn.prepare(&query).map_err(|e| format!("Failed to prepare query: {}", e))?;

    let rows = stmt
        .query_map([], |row| {
            Ok(PoolDbData {
                pool_address: row.get(0)?,
                token_mint: row.get(1)?,
                token_symbol: row.get(2)?,
                token_name: row.get(3)?,
                quote_token_address: row.get(4)?,
                quote_symbol: row.get(5)?,
                dex_id: row.get(6)?,
                liquidity_usd: row.get(7)?,
                price_native: row.get(8)?,
                price_usd: row.get(9)?,
                volume_24h: row.get(10)?,
                market_cap: row.get(11)?,
                fdv: row.get(12)?,
                txns_24h_buys: row.get(13)?,
                txns_24h_sells: row.get(14)?,
                price_change_24h: row.get(15)?,
                last_updated: row.get(16)?,
            })
        })
        .map_err(|e| format!("Failed to execute query: {}", e))?;

    let mut pools = Vec::new();
    for row in rows {
        pools.push(row.map_err(|e| format!("Failed to read row: {}", e))?);
    }

    log(
        LogTag::Pool,
        "DEBUG_DB_QUERY",
        &format!("📊 Retrieved {} pools from database", pools.len())
    );

    Ok(pools)
}
