#!/usr/bin/env cargo -Zscript
//! Pool Watch List Management Tool
//!
//! This tool helps manage and monitor the pool price service watch list,
//! including the new 5-minute automatic cleanup feature.

use screenerbot::logger::{ log, LogTag, init_file_logging };
use screenerbot::global::{ read_configs, is_debug_pool_prices_enabled };
use screenerbot::tokens::pool::{ get_pool_service, init_pool_service };
use std::env;
use tokio::time::{ sleep, Duration };

/// Print comprehensive help menu for the Pool Watch List Management Tool
fn print_help() {
    println!("🔧 Pool Watch List Management Tool");
    println!("=====================================");
    println!("Management and monitoring tool for the pool price service watch list");
    println!("with automatic cleanup features and priority tracking.");
    println!("");
    println!("USAGE:");
    println!("    cargo run --bin tool_pool_watch_list -- <COMMAND> [ARGS] [OPTIONS]");
    println!("");
    println!("COMMANDS:");
    println!("    add <token> [priority]     Add token to watch list with optional priority");
    println!("    remove <token>             Remove specific token from watch list");
    println!("    list                      Show current watch list with details");
    println!("    stats                     Display watch list statistics and performance");
    println!("    cleanup                   Manually cleanup expired watch list entries");
    println!("    monitor [duration]        Monitor watch list changes for specified seconds");
    println!("");
    println!("OPTIONS:");
    println!("    --help, -h                Show this help message");
    println!("");
    println!("EXAMPLES:");
    println!("    # Add SOL with high priority");
    println!(
        "    cargo run --bin tool_pool_watch_list -- add So11111111111111111111111111111111111111112 10"
    );
    println!("");
    println!("    # Add USDC with default priority");
    println!(
        "    cargo run --bin tool_pool_watch_list -- add EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"
    );
    println!("");
    println!("    # Monitor watch list for 60 seconds");
    println!("    cargo run --bin tool_pool_watch_list -- monitor 60");
    println!("");
    println!("    # Show current watch list status");
    println!("    cargo run --bin tool_pool_watch_list -- list");
    println!("");
    println!("    # Manual cleanup of expired entries");
    println!("    cargo run --bin tool_pool_watch_list -- cleanup");
    println!("");
    println!("WATCH LIST FEATURES:");
    println!("    • Priority-based token monitoring (1-10 scale)");
    println!("    • Automatic 5-minute expiry for inactive tokens");
    println!("    • Background cleanup service integration");
    println!("    • Real-time pool price tracking for watched tokens");
    println!("    • Request frequency tracking and optimization");
    println!("");
    println!("MONITORING OUTPUT:");
    println!("    • Current watch list size and token count");
    println!("    • Priority distribution and average priority");
    println!("    • Last update timestamps for each token");
    println!("    • Success/failure rates for price updates");
    println!("    • Cleanup statistics and expired entry counts");
    println!("");
    println!("AUTOMATIC FEATURES:");
    println!("    • Tokens auto-removed after 5 minutes without price updates");
    println!("    • Background monitoring service integration");
    println!("    • Priority-based update frequency optimization");
    println!("    • Failed request tracking and retry logic");
    println!("");
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    init_file_logging();

    let args: Vec<String> = env::args().collect();

    // Check for help flag
    if args.len() < 2 || args.contains(&"--help".to_string()) || args.contains(&"-h".to_string()) {
        print_help();
        if args.len() < 2 {
            return Ok(());
        } else {
            std::process::exit(0);
        }
    }

    // Load configuration
    let _configs = read_configs()?;

    // Initialize pool service
    init_pool_service();
    let pool_service = get_pool_service();

    let command = &args[1];

    match command.as_str() {
        "add" => {
            if args.len() < 3 {
                eprintln!("❌ Error: Token address required for add command");
                return Ok(());
            }

            let token_address = &args[2];
            let priority = args
                .get(3)
                .and_then(|s| s.parse::<i32>().ok())
                .unwrap_or(1);

            pool_service.add_to_watch_list(token_address, priority).await;

            log(
                LogTag::Pool,
                "ADD_SUCCESS",
                &format!("✅ Added {} to watch list with priority {}", token_address, priority)
            );

            println!("✅ Added {} to watch list with priority {}", token_address, priority);
        }

        "remove" => {
            if args.len() < 3 {
                eprintln!("❌ Error: Token address required for remove command");
                return Ok(());
            }

            let token_address = &args[2];
            pool_service.remove_from_watch_list(token_address).await;

            log(
                LogTag::Pool,
                "REMOVE_SUCCESS",
                &format!("✅ Removed {} from watch list", token_address)
            );

            println!("✅ Removed {} from watch list", token_address);
        }

        "list" => {
            let watch_list = pool_service.get_watch_list().await;

            if watch_list.is_empty() {
                println!("📝 Watch list is empty");
                return Ok(());
            }

            println!("📝 Current Watch List ({} tokens):", watch_list.len());
            println!(
                "┌─────────────────────────────────────────────────┬──────────┬─────────────────────┬─────────────────────┐"
            );
            println!(
                "│ Token Address                                   │ Priority │ Added At            │ Last Price Check    │"
            );
            println!(
                "├─────────────────────────────────────────────────┼──────────┼─────────────────────┼─────────────────────┤"
            );

            for entry in &watch_list {
                let added_at = entry.added_at.format("%Y-%m-%d %H:%M:%S").to_string();
                let last_check = entry.last_price_check
                    .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                    .unwrap_or_else(|| "Never".to_string());

                let expired_marker = if entry.is_expired() { " ⏰ EXPIRED" } else { "" };

                println!(
                    "│ {:<47} │ {:>8} │ {} │ {}{} │",
                    if entry.token_address.len() > 47 {
                        format!("{}...", &entry.token_address[..44])
                    } else {
                        entry.token_address.clone()
                    },
                    entry.priority,
                    added_at,
                    last_check,
                    expired_marker
                );
            }

            println!(
                "└─────────────────────────────────────────────────┴──────────┴─────────────────────┴─────────────────────┘"
            );
        }

        "stats" => {
            let (total, expired, never_checked) = pool_service.get_watch_list_stats().await;
            let (pool_cache, price_cache, availability_cache) =
                pool_service.get_cache_stats().await;

            println!("📊 Watch List Statistics:");
            println!("  • Total tokens: {}", total);
            println!("  • Expired tokens: {} (will be auto-removed)", expired);
            println!("  • Never checked: {} (no successful price yet)", never_checked);
            println!("  • Active tokens: {}", total - expired);
            println!();
            println!("💾 Cache Statistics:");
            println!("  • Pool cache entries: {}", pool_cache);
            println!("  • Price cache entries: {}", price_cache);
            println!("  • Availability cache entries: {}", availability_cache);
        }

        "cleanup" => {
            let removed_count = pool_service.cleanup_expired_watch_list().await;

            if removed_count == 0 {
                println!("🧹 No expired tokens found to clean up");
            } else {
                println!("🧹 Cleaned up {} expired tokens from watch list", removed_count);
            }
        }

        "monitor" => {
            let duration_seconds = args
                .get(2)
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(30);

            println!("👀 Monitoring watch list for {} seconds...", duration_seconds);
            println!(
                "⏰ Tokens are automatically removed after 5 minutes without successful price updates"
            );
            println!("🔍 Press Ctrl+C to stop monitoring early");
            println!();

            let start_time = std::time::Instant::now();
            let mut last_stats = pool_service.get_watch_list_stats().await;

            println!(
                "📊 Initial stats: {} total, {} expired, {} never checked",
                last_stats.0,
                last_stats.1,
                last_stats.2
            );

            while start_time.elapsed().as_secs() < duration_seconds {
                sleep(Duration::from_secs(5)).await;

                let current_stats = pool_service.get_watch_list_stats().await;

                if current_stats != last_stats {
                    let elapsed = start_time.elapsed().as_secs();
                    println!(
                        "[{:3}s] 📊 Stats changed: {} total, {} expired, {} never checked",
                        elapsed,
                        current_stats.0,
                        current_stats.1,
                        current_stats.2
                    );

                    if current_stats.0 < last_stats.0 {
                        let removed = last_stats.0 - current_stats.0;
                        println!("      🗑️  {} tokens were automatically removed", removed);
                    }

                    last_stats = current_stats;
                } else if is_debug_pool_prices_enabled() {
                    let elapsed = start_time.elapsed().as_secs();
                    println!(
                        "[{:3}s] 📊 No changes: {} total, {} expired, {} never checked",
                        elapsed,
                        current_stats.0,
                        current_stats.1,
                        current_stats.2
                    );
                }
            }

            println!("👀 Monitoring complete after {} seconds", duration_seconds);
        }

        _ => {
            eprintln!("❌ Unknown command: {}", command);
            eprintln!("Use '{}' without arguments to see usage", args[0]);
            return Ok(());
        }
    }

    Ok(())
}
