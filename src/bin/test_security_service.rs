use screenerbot::{
    logger::{ init_file_logging, log, LogTag },
    tokens::security::{ start_security_monitoring, get_security_analyzer },
};

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Notify;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    init_file_logging();

    println!("=== Testing Security Background Service ===\n");

    // Initialize security analyzer
    let analyzer = get_security_analyzer();

    // Check how many tokens need security analysis
    match analyzer.database.count_tokens_without_security() {
        Ok(count) => {
            println!("✅ Found {} tokens without security info in database", count);
            log(LogTag::Security, "TEST_COUNT", &format!("Uncached tokens: {}", count));
        }
        Err(e) => {
            println!("❌ Failed to count uncached tokens: {}", e);
            return Err(e.into());
        }
    }

    // Get a few token examples
    match analyzer.database.get_tokens_without_security() {
        Ok(tokens) => {
            let sample_size = std::cmp::min(5, tokens.len());
            if sample_size > 0 {
                println!("\n📋 Sample tokens without security info:");
                for (i, token) in tokens.iter().take(sample_size).enumerate() {
                    println!("  {}. {}", i + 1, token);
                }
                if tokens.len() > sample_size {
                    println!("  ... and {} more", tokens.len() - sample_size);
                }
            } else {
                println!("✅ All tokens already have security info cached!");
            }
        }
        Err(e) => {
            println!("❌ Failed to get uncached tokens: {}", e);
            return Err(e.into());
        }
    }

    println!("\n🚀 Starting security monitoring service (will run for 15 seconds)...");

    // Create shutdown notification
    let shutdown = Arc::new(Notify::new());

    // Start the security monitoring service
    let service_handle = match start_security_monitoring(shutdown.clone()).await {
        Ok(handle) => handle,
        Err(e) => {
            println!("❌ Failed to start security monitoring: {}", e);
            return Err(e.into());
        }
    };

    // Let it run for 15 seconds to see some activity
    println!("⏳ Letting service run for 15 seconds...\n");
    tokio::time::sleep(Duration::from_secs(15)).await;

    // Check progress
    match analyzer.database.count_tokens_without_security() {
        Ok(remaining) => {
            println!("\n📊 Progress check: {} tokens still need security analysis", remaining);
        }
        Err(e) => {
            println!("\n⚠️  Failed to check progress: {}", e);
        }
    }

    // Shutdown the service
    println!("\n🛑 Shutting down security monitoring service...");
    shutdown.notify_waiters();

    // Wait for service to shut down (with timeout)
    match tokio::time::timeout(Duration::from_secs(5), service_handle).await {
        Ok(Ok(())) => println!("✅ Security service shut down cleanly"),
        Ok(Err(e)) => println!("⚠️  Security service ended with error: {}", e),
        Err(_) => println!("⚠️  Security service shutdown timed out"),
    }

    println!("\n=== Security Service Test Complete ===");
    Ok(())
}
