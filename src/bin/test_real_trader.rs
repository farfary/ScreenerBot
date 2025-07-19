use screenerbot::utils::load_positions_from_file;
use screenerbot::global::read_configs;
use screenerbot::trader::{ SAVED_POSITIONS, monitor_new_entries, monitor_open_positions };
use std::time::Duration;
use tokio::time;
use std::sync::Arc;
use tokio::sync::Notify;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🤖 Testing Real Trading Bot with On-Chain Transactions");
    println!("========================================================");

    // Load configurations
    let _configs = read_configs("configs.json")?;
    println!("✅ Configurations loaded");

    // Load existing positions
    load_positions_from_file();
    println!("✅ Positions loaded");

    // Print current position count
    {
        if let Ok(positions) = SAVED_POSITIONS.lock() {
            println!("📊 Current positions: {}", positions.len());
        }
    }

    // Create shutdown notifier for monitoring functions
    let shutdown = Arc::new(Notify::new());

    // Run a few cycles of the trading bot
    println!("\n🔄 Starting trading bot cycles...");

    for cycle in 1..=3 {
        println!("\n--- Cycle {} ---", cycle);

        // Monitor for new positions to open (run for a short time)
        let shutdown_clone = shutdown.clone();
        let _monitor_task = tokio::spawn(async move {
            monitor_new_entries(shutdown_clone).await;
        });

        // Let it run for 10 seconds
        time::sleep(Duration::from_secs(10)).await;

        // Monitor existing positions for exits (run for a short time)
        let shutdown_clone2 = shutdown.clone();
        let _positions_task = tokio::spawn(async move {
            monitor_open_positions(shutdown_clone2).await;
        });

        // Let it run for 10 seconds
        time::sleep(Duration::from_secs(10)).await;

        // Wait between cycles
        println!("⏳ Waiting 30 seconds before next cycle...");
        time::sleep(Duration::from_secs(30)).await;
    }

    // Print final status
    println!("\n📈 Final Status:");
    {
        if let Ok(positions) = SAVED_POSITIONS.lock() {
            println!("Total positions: {}", positions.len());

            for (i, position) in positions.iter().enumerate() {
                println!("Position {}: {} - Status: {}", i + 1, position.mint, if
                    position.exit_time.is_none()
                {
                    "OPEN"
                } else {
                    "CLOSED"
                });
                if let Some(pnl) = position.pnl_percent {
                    println!("  P&L: {:.2}%", pnl);
                }
                if let Some(entry_sig) = &position.entry_transaction_signature {
                    println!("  Entry TX: {}", entry_sig);
                }
                if let Some(exit_sig) = &position.exit_transaction_signature {
                    println!("  Exit TX: {}", exit_sig);
                }
                if let Some(effective_entry) = position.effective_entry_price {
                    println!("  Effective Entry Price: {:.8} SOL", effective_entry);
                }
                if let Some(effective_exit) = position.effective_exit_price {
                    println!("  Effective Exit Price: {:.8} SOL", effective_exit);
                }
            }
        }
    }

    println!("\n✅ Real trading bot test completed!");
    Ok(())
}
