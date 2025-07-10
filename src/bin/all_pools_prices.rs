use screenerbot::prelude::*;
use screenerbot::helpers::RPC;

use anyhow::Result;
use colored::Colorize;
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use std::str::FromStr;

#[derive(Debug)]
struct PoolAnalysis {
    address: String,
    source: String,
    name: Option<String>,
    pool_type: String,
    price_sol: Option<f64>,
    liquidity_usd: Option<f64>,
    volume_24h_usd: Option<f64>,
    tx_count_24h: Option<u64>,
    base_reserve: Option<u64>,
    quote_reserve: Option<u64>,
    error: Option<String>,
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    
    if args.len() != 2 {
        eprintln!("Usage: {} <token_mint>", args[0]);
        eprintln!("Example: {} 4mu1ig6ML6ZQm5sVkWHVjuCttYADn9wguMsyvXsCbonk", args[0]);
        std::process::exit(1);
    }

    let token_mint = &args[1];
    
    println!("🔍 {} {}", "Analyzing all pools for token:".cyan().bold(), token_mint.yellow());
    println!();

    // Initialize RPC client
    let rpc = &*RPC;
    
    // Fetch all pools for this token
    let pools = match fetch_combined_pools(token_mint) {
        Ok(pools) => pools,
        Err(e) => {
            eprintln!("❌ Failed to fetch pools: {}", e);
            std::process::exit(1);
        }
    };

    if pools.is_empty() {
        println!("⚠️ No pools found for token {}", token_mint);
        return Ok(());
    }

    println!("📊 {} {}", "Found".green().bold(), format!("{} pools", pools.len()).cyan().bold());
    println!();

    // Analyze each pool
    let mut analyses = Vec::new();
    let mut successful_prices = Vec::new();
    
    for (i, pool) in pools.iter().enumerate() {
        print!("⏳ Analyzing pool {}/{}: {} ... ", i + 1, pools.len(), pool.address.dimmed());
        
        let pool_pk = match Pubkey::from_str(&pool.address) {
            Ok(pk) => pk,
            Err(e) => {
                println!("{}", "❌ Invalid address".red());
                analyses.push(PoolAnalysis {
                    address: pool.address.clone(),
                    source: pool.source.clone(),
                    name: pool.name.clone(),
                    pool_type: "Unknown".to_string(),
                    price_sol: None,
                    liquidity_usd: pool.liquidity_usd,
                    volume_24h_usd: pool.volume_24h_usd,
                    tx_count_24h: pool.tx_count_24h,
                    base_reserve: None,
                    quote_reserve: None,
                    error: Some(format!("Invalid address: {}", e)),
                });
                continue;
            }
        };

        // Get pool type by checking owner
        let pool_type = match rpc.get_account(&pool_pk) {
            Ok(account) => {
                match account.owner.to_string().as_str() {
                    "pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA" => "PumpFun v1".to_string(),
                    "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P" => "PumpFun v2 CPMM".to_string(),
                    "CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK" => "Raydium CLMM v2".to_string(),
                    "RVKd61ztZW9g2VZgPZrFYuXJcZ1t7xvaUo1NkL6MZ5w" => "Raydium AMM v4".to_string(),
                    "CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C" => "Raydium CPMM".to_string(),
                    "whirLb9FtDwZ2Bi4FXe65aaPaJqmCj7QSfUeCrpuHgx" => "Orca Whirlpool".to_string(),
                    "LanMV9sAd7wArD4vJFi2qDdfnVhFxYSUg6eADduJ3uj" => "Raydium Launchpad".to_string(),
                    "LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo" => "Meteora DLMM".to_string(),
                    "cpamdpZCGKUy5JxQXB4dcpGPiikHawvSWAd6mEn1sGG" => "Meteora DYN2".to_string(),
                    owner => format!("Unknown ({})", &owner[..8]),
                }
            }
            Err(_) => "Unknown".to_string(),
        };

        // Try to decode the pool and get price
        match decode_any_pool(rpc, &pool_pk) {
            Ok((base_amt, quote_amt, base_mint, quote_mint)) => {
                // Calculate price
                let price = if base_amt > 0 {
                    match (get_token_decimals(rpc, &base_mint), get_token_decimals(rpc, &quote_mint)) {
                        (Ok(base_dec), Ok(quote_dec)) => {
                            let price = ((quote_amt as f64) / (base_amt as f64)) * (10f64).powi(base_dec as i32 - quote_dec as i32);
                            Some(price)
                        }
                        _ => None,
                    }
                } else {
                    None
                };

                if let Some(p) = price {
                    successful_prices.push(p);
                    println!("{} {:.12} SOL", "✅".green(), format!("{}", p).cyan().bold());
                } else {
                    println!("{}", "⚠️ Price calculation failed".yellow());
                }

                analyses.push(PoolAnalysis {
                    address: pool.address.clone(),
                    source: pool.source.clone(),
                    name: pool.name.clone(),
                    pool_type,
                    price_sol: price,
                    liquidity_usd: pool.liquidity_usd,
                    volume_24h_usd: pool.volume_24h_usd,
                    tx_count_24h: pool.tx_count_24h,
                    base_reserve: Some(base_amt),
                    quote_reserve: Some(quote_amt),
                    error: None,
                });
            }
            Err(e) => {
                println!("{} {}", "❌".red(), e.to_string().red());
                analyses.push(PoolAnalysis {
                    address: pool.address.clone(),
                    source: pool.source.clone(),
                    name: pool.name.clone(),
                    pool_type,
                    price_sol: None,
                    liquidity_usd: pool.liquidity_usd,
                    volume_24h_usd: pool.volume_24h_usd,
                    tx_count_24h: pool.tx_count_24h,
                    base_reserve: None,
                    quote_reserve: None,
                    error: Some(e.to_string()),
                });
            }
        }
    }

    println!();
    
    // Calculate price statistics
    if !successful_prices.is_empty() {
        successful_prices.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let min_price = successful_prices[0];
        let max_price = successful_prices[successful_prices.len() - 1];
        let avg_price = successful_prices.iter().sum::<f64>() / successful_prices.len() as f64;
        let median_price = if successful_prices.len() % 2 == 0 {
            (successful_prices[successful_prices.len() / 2 - 1] + successful_prices[successful_prices.len() / 2]) / 2.0
        } else {
            successful_prices[successful_prices.len() / 2]
        };

        println!("📈 {} {}", "PRICE ANALYSIS".green().bold(), format!("({} successful pools)", successful_prices.len()).dimmed());
        println!("   {} {:.12} SOL", "Min Price:".blue().bold(), format!("{}", min_price).cyan());
        println!("   {} {:.12} SOL", "Max Price:".blue().bold(), format!("{}", max_price).cyan());
        println!("   {} {:.12} SOL", "Avg Price:".blue().bold(), format!("{}", avg_price).cyan());
        println!("   {} {:.12} SOL", "Median:  ".blue().bold(), format!("{}", median_price).cyan());
        
        if max_price > min_price {
            let price_spread = ((max_price - min_price) / avg_price) * 100.0;
            println!("   {} {:.2}%", "Spread:   ".blue().bold(), format!("{}", price_spread).yellow());
            
            // Show arbitrage opportunities
            if price_spread > 5.0 {
                println!("   {} Potential arbitrage opportunity detected!", "⚡".yellow().bold());
                let profit_percent = ((max_price - min_price) / min_price) * 100.0;
                println!("   {} {:.2}% profit potential", "💰".green().bold(), profit_percent.to_string().green().bold());
            }
        }
        
        // Show price deviations for each pool
        println!();
        println!("📊 {} ", "PRICE DEVIATIONS FROM AVERAGE".green().bold());
        for analysis in &analyses {
            if let Some(price) = analysis.price_sol {
                let deviation = ((price - avg_price) / avg_price) * 100.0;
                let deviation_str = if deviation > 0.0 {
                    format!("+{:.2}%", deviation).green()
                } else {
                    format!("{:.2}%", deviation).red()
                };
                let address_short = format!("{}...{}", 
                    &analysis.address[..8], 
                    &analysis.address[analysis.address.len()-8..]
                );
                println!("   {} {} {} ({})", 
                    analysis.pool_type.blue(),
                    address_short.dimmed(),
                    deviation_str,
                    format!("{:.12} SOL", price).cyan()
                );
            }
        }
        println!();
    }

    // Detailed table
    println!("📋 {} ", "DETAILED POOL ANALYSIS".green().bold());
    println!();
    
    // Sort by liquidity (highest first), then by price
    analyses.sort_by(|a, b| {
        let a_liq = a.liquidity_usd.unwrap_or(0.0);
        let b_liq = b.liquidity_usd.unwrap_or(0.0);
        b_liq.partial_cmp(&a_liq).unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                match (a.price_sol, b.price_sol) {
                    (Some(a_price), Some(b_price)) => b_price.partial_cmp(&a_price).unwrap_or(std::cmp::Ordering::Equal),
                    (Some(_), None) => std::cmp::Ordering::Less,
                    (None, Some(_)) => std::cmp::Ordering::Greater,
                    (None, None) => std::cmp::Ordering::Equal,
                }
            })
    });

    // Header
    println!("{:<4} {:<15} {:<50} {:<20} {:<15} {:<12} {:<10} {:<8}", 
        "#".bold(), 
        "Type".bold(), 
        "Address".bold(), 
        "Name".bold(), 
        "Price (SOL)".bold(), 
        "Liquidity".bold(), 
        "Volume 24h".bold(), 
        "Txs 24h".bold()
    );
    println!("{}", "─".repeat(140).dimmed());

    for (i, analysis) in analyses.iter().enumerate() {
        let rank = format!("{}", i + 1);
        let pool_type = if analysis.pool_type.len() > 14 {
            format!("{}...", &analysis.pool_type[..11])
        } else {
            analysis.pool_type.clone()
        };
        
        let address_short = format!("{}...{}", 
            &analysis.address[..8], 
            &analysis.address[analysis.address.len()-8..]
        );
        
        let name = analysis.name.as_ref()
            .map(|n| if n.len() > 18 { format!("{}...", &n[..15]) } else { n.clone() })
            .unwrap_or_else(|| "Unknown".dimmed().to_string());

        let price_str = if let Some(price) = analysis.price_sol {
            if price > 0.0 {
                format!("{:.12}", price).cyan().to_string()
            } else {
                "0".dimmed().to_string()
            }
        } else if let Some(error) = &analysis.error {
            "ERROR".red().to_string()
        } else {
            "N/A".dimmed().to_string()
        };

        let liquidity_str = analysis.liquidity_usd
            .map(|l| if l >= 1000.0 { format!("${:.0}k", l / 1000.0) } else { format!("${:.0}", l) })
            .unwrap_or_else(|| "N/A".dimmed().to_string());

        let volume_str = analysis.volume_24h_usd
            .map(|v| if v >= 1000.0 { format!("${:.0}k", v / 1000.0) } else { format!("${:.0}", v) })
            .unwrap_or_else(|| "N/A".dimmed().to_string());

        let tx_str = analysis.tx_count_24h
            .map(|t| t.to_string())
            .unwrap_or_else(|| "N/A".dimmed().to_string());

        println!("{:<4} {:<15} {:<50} {:<20} {:<15} {:<12} {:<10} {:<8}", 
            rank.yellow(),
            pool_type.blue(),
            address_short.white(),
            name,
            price_str,
            liquidity_str.green(),
            volume_str.magenta(),
            tx_str.cyan()
        );

        // Show reserves if available
        if let (Some(base), Some(quote)) = (analysis.base_reserve, analysis.quote_reserve) {
            println!("     {} Base: {}, Quote: {}", 
                "Reserves:".dimmed(), 
                format!("{}", base).dimmed(), 
                format!("{}", quote).dimmed()
            );
        }

        // Show error if any
        if let Some(error) = &analysis.error {
            println!("     {} {}", "Error:".red(), error.red());
        }
        
        println!();
    }

    // Summary statistics
    let working_pools = analyses.iter().filter(|a| a.price_sol.is_some()).count();
    let total_liquidity: f64 = analyses.iter()
        .filter_map(|a| a.liquidity_usd)
        .sum();
    let total_volume: f64 = analyses.iter()
        .filter_map(|a| a.volume_24h_usd)
        .sum();
    let total_txs: u64 = analyses.iter()
        .filter_map(|a| a.tx_count_24h)
        .sum();

    println!("📊 {} ", "SUMMARY STATISTICS".green().bold());
    println!("   {} {} out of {}", "Working Pools:".blue().bold(), working_pools.to_string().green(), analyses.len());
    println!("   {} ${:.0}", "Total Liquidity:".blue().bold(), total_liquidity);
    println!("   {} ${:.0}", "Total Volume 24h:".blue().bold(), total_volume);
    println!("   {} {}", "Total Transactions 24h:".blue().bold(), total_txs.to_string().green());

    // Pool type breakdown
    let mut type_counts: HashMap<String, usize> = HashMap::new();
    for analysis in &analyses {
        *type_counts.entry(analysis.pool_type.clone()).or_insert(0) += 1;
    }

    println!();
    println!("🏗️ {} ", "POOL TYPE BREAKDOWN".green().bold());
    for (pool_type, count) in type_counts.iter() {
        println!("   {} {}", format!("{}:", pool_type).blue().bold(), count.to_string().cyan());
    }

    Ok(())
}
