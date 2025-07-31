/// ATA Cleanup Service Test Tool
///
/// This tool allows testing the background ATA cleanup service functionality
/// without running the full trading bot.

use screenerbot::ata_cleanup::{ trigger_immediate_ata_cleanup, get_ata_cleanup_stats };
use screenerbot::logger::{ log, LogTag, init_file_logging };

#[tokio::main]
async fn main() {
    init_file_logging();

    log(LogTag::System, "INFO", "🧹 ATA Cleanup Service Test Tool");

    // Get current ATA stats
    log(LogTag::System, "INFO", "📊 Checking current ATA status...");
    match get_ata_cleanup_stats().await {
        Ok(stats) => {
            println!("📊 {}", stats);
        }
        Err(e) => {
            log(LogTag::System, "ERROR", &format!("Failed to get ATA stats: {}", e));
            println!("❌ Failed to get ATA stats: {}", e);
            return;
        }
    }

    // Ask user if they want to proceed with cleanup
    println!("\n🤔 Do you want to trigger immediate ATA cleanup? (y/N): ");
    let mut input = String::new();
    if std::io::stdin().read_line(&mut input).is_ok() {
        let input = input.trim().to_lowercase();
        if input == "y" || input == "yes" {
            log(LogTag::System, "INFO", "🚀 Triggering immediate ATA cleanup...");

            match trigger_immediate_ata_cleanup().await {
                Ok((closed_count, signatures)) => {
                    if closed_count > 0 {
                        let rent_reclaimed = (closed_count as f64) * 0.00203928;
                        println!("🎉 Successfully cleaned up {} empty ATAs", closed_count);
                        println!("💰 Reclaimed approximately {:.6} SOL in rent", rent_reclaimed);
                        println!("📝 Transaction signatures:");
                        for (i, sig) in signatures.iter().enumerate() {
                            println!("  {}. {}", i + 1, sig);
                        }
                        log(
                            LogTag::System,
                            "SUCCESS",
                            &format!("ATA cleanup completed: {} accounts closed", closed_count)
                        );
                    } else {
                        println!("✅ No empty ATAs found - wallet is already optimized");
                        log(LogTag::System, "INFO", "No empty ATAs found");
                    }
                }
                Err(e) => {
                    println!("❌ ATA cleanup failed: {}", e);
                    log(LogTag::System, "ERROR", &format!("ATA cleanup failed: {}", e));
                }
            }
        } else {
            println!("🚫 ATA cleanup cancelled");
            log(LogTag::System, "INFO", "ATA cleanup cancelled by user");
        }
    }

    // Show final stats
    println!("\n📊 Final ATA status:");
    match get_ata_cleanup_stats().await {
        Ok(stats) => {
            println!("📊 {}", stats);
        }
        Err(e) => {
            println!("❌ Failed to get final ATA stats: {}", e);
        }
    }

    println!("✅ ATA cleanup test tool completed");
}
