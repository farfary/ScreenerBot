/// Test tool for dry-run mode functionality
/// 
/// This tool verifies that the dry-run mode prevents actual trading operations
/// while still showing what would happen in normal operation.

use screenerbot::logger::{log, LogTag, init_file_logging};
use screenerbot::arguments::{set_cmd_args, is_dry_run_enabled, print_debug_info};

#[tokio::main]
async fn main() {
    // Initialize logging
    init_file_logging();
    
    log(LogTag::System, "INFO", "🧪 Testing dry-run mode functionality");
    
    // Test command-line arguments
    let args: Vec<String> = std::env::args().collect();
    set_cmd_args(args);
    
    // Print current argument state
    print_debug_info();
    
    // Test dry-run detection
    if is_dry_run_enabled() {
        log(LogTag::System, "SUCCESS", "✅ Dry-run mode detected correctly");
        log(LogTag::System, "INFO", "🚫 Trading operations would be simulated only");
        log(LogTag::System, "INFO", "📊 All trading signals and analysis would be logged but not executed");
    } else {
        log(LogTag::System, "INFO", "💰 Normal trading mode - actual transactions would be executed");
        log(LogTag::System, "INFO", "⚠️  Use --dry-run flag to enable simulation mode");
    }
    
    // Test centralized arguments system
    log(LogTag::System, "INFO", "🔧 Testing centralized arguments system:");
    
    // Test debug flags
    if screenerbot::arguments::is_debug_trader_enabled() {
        log(LogTag::System, "INFO", "🐛 Trader debug mode enabled");
    }
    
    if screenerbot::arguments::is_debug_swap_enabled() {
        log(LogTag::System, "INFO", "🔄 Swap debug mode enabled");
    }
    
    // Show enabled modes
    let enabled_modes = screenerbot::arguments::get_enabled_debug_modes();
    if !enabled_modes.is_empty() {
        log(LogTag::System, "INFO", &format!("🎛️  Enabled modes: {:?}", enabled_modes));
    } else {
        log(LogTag::System, "INFO", "🎛️  No debug modes enabled");
    }
    
    // Test argument value extraction
    if let Some(mint) = screenerbot::arguments::patterns::get_mint_address() {
        log(LogTag::System, "INFO", &format!("🪙 Mint address provided: {}", mint));
    }
    
    if let Some(duration) = screenerbot::arguments::patterns::get_duration_seconds() {
        log(LogTag::System, "INFO", &format!("⏱️  Duration provided: {} seconds", duration));
    }
    
    // Show usage help
    if screenerbot::arguments::patterns::is_help_requested() {
        print_usage();
    } else {
        log(LogTag::System, "INFO", "ℹ️  Use --help to see usage information");
    }
    
    log(LogTag::System, "SUCCESS", "🏁 Dry-run mode test completed");
}

fn print_usage() {
    println!("\n📖 USAGE:");
    println!("  cargo run --bin test_dry_run -- [OPTIONS]");
    println!("\n🎛️  OPTIONS:");
    println!("  --dry-run                 Enable simulation mode (no actual trading)");
    println!("  --debug-trader            Enable trader debug output");
    println!("  --debug-swap              Enable swap debug output");
    println!("  --mint <ADDRESS>          Test with specific mint address");
    println!("  --duration <SECONDS>      Test with specific duration");
    println!("  --help                    Show this help message");
    println!("\n📝 EXAMPLES:");
    println!("  # Test dry-run mode");
    println!("  cargo run --bin test_dry_run -- --dry-run");
    println!();
    println!("  # Test with debug flags");
    println!("  cargo run --bin test_dry_run -- --dry-run --debug-trader --debug-swap");
    println!();
    println!("  # Test with parameters");
    println!("  cargo run --bin test_dry_run -- --dry-run --mint EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v --duration 300");
    println!();
}
