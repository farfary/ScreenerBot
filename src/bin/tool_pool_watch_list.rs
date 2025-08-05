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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    init_file_logging();

    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        println!("🔧 Pool Watch List Management Tool");
        println!();
        println!("Usage:");
        println!("  {} add <token_address> [priority]     - Add token to watch list", args[0]);
        println!("  {} remove <token_address>             - Remove token from watch list", args[0]);
        println!("  {} list                               - Show current watch list", args[0]);
        println!("  {} stats                              - Show watch list statistics", args[0]);
        println!(
            "  {} cleanup                            - Manually cleanup expired entries",
            args[0]
        );
        println!(
            "  {} monitor [duration_seconds]         - Monitor watch list for changes",
            args[0]
        );
        println!();
        println!("Examples:");
        println!("  {} add So11111111111111111111111111111111111111112 10", args[0]);
        println!("  {} monitor 60  # Monitor for 60 seconds", args[0]);
        println!();
        println!(
            "Note: Tokens are automatically removed after 5 minutes without successful price updates"
        );
        return Ok(());
    }

    // Load configuration
    let _configs = read_configs("configs.json")?;

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
