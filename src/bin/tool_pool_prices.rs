use screenerbot::pool_price::PoolDiscoveryAndPricing;
use screenerbot::global::read_configs;
use screenerbot::logger::{ log, LogTag };
use std::env;
use anyhow::Result;
use colored::Colorize;

/// Display pool prices for a token mint address
pub async fn display_token_pool_prices(token_mint: &str) -> Result<()> {
    // Load configuration
    let configs = read_configs("configs.json").map_err(|e|
        anyhow::anyhow!("Failed to read configs: {}", e)
    )?;

    // Create pool discovery instance
    let pool_discovery = PoolDiscoveryAndPricing::new(&configs.rpc_url);

    println!("{}", "=".repeat(80).bright_cyan());
    println!(
        "{} {}",
        "🔍 Pool Price Discovery for Token:".bright_green().bold(),
        token_mint.bright_yellow()
    );
    println!("{}", "=".repeat(80).bright_cyan());
    println!();

    // Get all pools and their prices sorted by liquidity (biggest first)
    match pool_discovery.get_token_pool_prices(token_mint).await {
        Ok(pool_results) => {
            if pool_results.is_empty() {
                println!("{}", "❌ No pools found for this token".bright_red().bold());
                return Ok(());
            }

            println!(
                "{} {} {}",
                "📊 Found".bright_green(),
                pool_results.len().to_string().bright_yellow().bold(),
                "pools (sorted by liquidity - biggest first):".bright_green()
            );
            println!();

            for (index, pool) in pool_results.iter().enumerate() {
                let rank = format!("#{}", index + 1);
                let pool_type_str = format!("{:?}", pool.pool_type);
                let pair = format!("{}/{}", pool.token_a_symbol, pool.token_b_symbol);

                println!(
                    "{} {} {} {}",
                    rank.bright_blue().bold(),
                    pool_type_str.bright_magenta(),
                    format!("({})", pool.dex_id).dimmed(),
                    pair.bright_white().bold()
                );

                println!("   {} {}", "📍 Pool Address:".dimmed(), pool.pool_address.bright_cyan());

                println!("   {} ${:.2}", "💧 Liquidity USD:".bright_blue(), pool.liquidity_usd);

                println!("   {} ${:.2}", "📈 24h Volume:".bright_green(), pool.volume_24h);

                // Display DexScreener API price
                if pool.dexscreener_price > 0.0 {
                    println!(
                        "   {} {} SOL",
                        "🏷️  API Price (DexScreener):".bright_yellow(),
                        format!("{:.12}", pool.dexscreener_price).bright_white()
                    );
                } else {
                    println!(
                        "   {} {}",
                        "🏷️  API Price (DexScreener):".bright_yellow(),
                        "N/A".dimmed()
                    );
                }

                // Display calculated price if available
                if pool.calculation_successful && pool.calculated_price > 0.0 {
                    println!(
                        "   {} {} SOL",
                        "🧮 Calculated Price (On-chain):".bright_green(),
                        format!("{:.12}", pool.calculated_price).bright_white()
                    );

                    if pool.dexscreener_price > 0.0 {
                        let diff_color = if pool.price_difference_percent > 5.0 {
                            "bright_red"
                        } else if pool.price_difference_percent > 1.0 {
                            "bright_yellow"
                        } else {
                            "bright_green"
                        };

                        println!("   {} {}%", "📊 Price Difference:".bright_cyan(), match
                            diff_color
                        {
                            "bright_red" =>
                                format!("{:.2}", pool.price_difference_percent).bright_red(),
                            "bright_yellow" =>
                                format!("{:.2}", pool.price_difference_percent).bright_yellow(),
                            _ => format!("{:.2}", pool.price_difference_percent).bright_green(),
                        });
                    }
                } else {
                    println!(
                        "   {} {}",
                        "🧮 Calculated Price (On-chain):".bright_green(),
                        "❌ Calculation failed".bright_red()
                    );

                    if let Some(error) = &pool.error_message {
                        println!("   {} {}", "⚠️  Error:".bright_red(), error.dimmed());
                    }
                }

                // Show if it's a SOL pair
                if pool.is_sol_pair {
                    println!("   {} {}", "🪙 SOL Pair:".bright_yellow(), "Yes".bright_green());
                } else {
                    println!("   {} {}", "🪙 SOL Pair:".bright_yellow(), "No".dimmed());
                }

                // Support status
                let support_status = if pool.calculation_successful {
                    "✅ Supported".bright_green()
                } else {
                    "⚠️  Limited Support (API price only)".bright_yellow()
                };

                println!("   {} {}", "🔧 Support Status:".bright_cyan(), support_status);

                println!(); // Empty line between pools
            }

            // Summary
            let supported_count = pool_results
                .iter()
                .filter(|p| p.calculation_successful)
                .count();
            let unsupported_count = pool_results.len() - supported_count;

            println!("{}", "=".repeat(80).bright_cyan());
            println!("{}", "📋 SUMMARY".bright_green().bold());
            println!("{}", "=".repeat(80).bright_cyan());
            println!(
                "   {} {} pools",
                "📊 Total:".bright_blue(),
                pool_results.len().to_string().bright_white().bold()
            );
            println!(
                "   {} {} pools",
                "✅ Fully Supported:".bright_green(),
                supported_count.to_string().bright_green().bold()
            );
            println!(
                "   {} {} pools",
                "⚠️  API Only:".bright_yellow(),
                unsupported_count.to_string().bright_yellow().bold()
            );

            if let Some(best_pool) = pool_results.first() {
                println!();
                println!(
                    "   {} {} ({}) with ${:.2} liquidity",
                    "🏆 Best Pool:".bright_yellow().bold(),
                    format!("{}/{}", best_pool.token_a_symbol, best_pool.token_b_symbol)
                        .bright_white()
                        .bold(),
                    best_pool.dex_id.bright_magenta(),
                    best_pool.liquidity_usd
                );
            }

            println!("{}", "=".repeat(80).bright_cyan());
        }
        Err(e) => {
            println!(
                "{} {}",
                "❌ Error fetching pool data:".bright_red().bold(),
                e.to_string().bright_red()
            );
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        println!(
            "{}",
            "❌ Usage: cargo run --bin tool_pool_prices <token_mint_address>".bright_red().bold()
        );
        println!();
        println!("{}", "Examples:".bright_green().bold());
        println!(
            "  {} {}",
            "cargo run --bin tool_pool_prices".bright_cyan(),
            "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".bright_yellow()
        );
        println!(
            "  {} {}",
            "cargo run --bin tool_pool_prices".bright_cyan(),
            "So11111111111111111111111111111111111111112".bright_yellow()
        );
        std::process::exit(1);
    }

    let token_mint = &args[1];

    // Validate mint address format (basic check)
    if token_mint.len() != 44 && token_mint.len() != 43 {
        println!(
            "{} {}",
            "❌ Invalid token mint address format.".bright_red().bold(),
            "Expected 43-44 characters.".bright_red()
        );
        std::process::exit(1);
    }

    match display_token_pool_prices(token_mint).await {
        Ok(_) => {}
        Err(e) => {
            log(LogTag::System, "ERROR", &format!("Failed to display pool prices: {}", e));
            println!("{} {}", "❌ Error:".bright_red().bold(), e.to_string().bright_red());
            std::process::exit(1);
        }
    }

    Ok(())
}
